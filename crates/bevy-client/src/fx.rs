//! A fixed pool of particles: one shared faceted-sphere mesh and one shared
//! material per kind, so they batch. Spawning reuses the oldest slot.

use crate::clock;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum P {
    Fire,
    Smoke,
    Steam,
    Frost,
    Foam,
    Water,
    Spark,
    /// Level 2 (I-035): a liquid nitrogen bead skating on the stream on its
    /// own vapour (the Leidenfrost effect), shrinking as it boils away.
    Bead,
}

const KINDS: [P; 8] = [
    P::Fire,
    P::Smoke,
    P::Steam,
    P::Frost,
    P::Foam,
    P::Water,
    P::Spark,
    P::Bead,
];

#[derive(Component)]
pub struct Particle {
    pub vel: Vec3,
    pub life: f32,
    pub max: f32,
    pub size: f32,
    pub grow: f32,
    pub gravity: f32,
    pub kind: P,
}

#[derive(Resource)]
pub struct Fx {
    pub slots: Vec<Entity>,
    pub next: usize,
    pub mats: Vec<Handle<StandardMaterial>>,
    /// Cheap deterministic noise for spread.
    pub seed: u32,
    pub pending: Vec<(P, Vec3, Vec3, f32, f32)>,
    /// Set to hide every live particle on the next update (after a reset).
    pub clear: bool,
    /// The game's reset counter when the particles were last cleared.
    pub epoch: u32,
    /// Live `Water` particles after this frame's update (`fxWater`).
    pub water_live: u32,
    /// Live `Bead` particles after this frame's update (`fxBeads`, I-035).
    pub bead_live: u32,
}

impl Fx {
    pub fn rand(&mut self) -> f32 {
        // xorshift32
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn rand3(&mut self) -> Vec3 {
        Vec3::new(self.rand() - 0.5, self.rand() - 0.5, self.rand() - 0.5) * 2.0
    }

    /// Queues one particle: kind, position, velocity, size, lifetime.
    pub fn emit(&mut self, k: P, pos: Vec3, vel: Vec3, size: f32, life: f32) {
        if self.pending.len() < 256 {
            self.pending.push((k, pos, vel, size, life));
        }
    }

    /// `n` liquid nitrogen beads skating out across the water from `pos`
    /// for under a second (I-035: 4 at `?low`, 6 otherwise), in white mist.
    pub fn spawn_beads(&mut self, pos: Vec3, n: usize) {
        for k in 0..n {
            let a = std::f32::consts::TAU * (k as f32 + self.rand() * 0.6) / n as f32;
            let speed = 2.2 + self.rand() * 1.4;
            let v = Vec3::new(a.cos() * speed, 0.0, a.sin() * speed);
            let life = 0.6 + self.rand() * 0.2;
            self.emit(P::Bead, pos, v, 0.16, life);
        }
        self.burst(P::Frost, pos + Vec3::Y * 0.2, 10, 1.0, 0.45);
    }

    /// A burst at an impact point.
    pub fn burst(&mut self, k: P, pos: Vec3, n: usize, speed: f32, size: f32) {
        for _ in 0..n {
            let mut v = self.rand3() * speed;
            v.y = v.y.abs() * 0.8 + speed * 0.3;
            let life = 0.5 + self.rand() * 0.6;
            let at = pos + self.rand3() * 0.2;
            let sz = size * (0.6 + self.rand() * 0.6);
            self.emit(k, at, v, sz, life);
        }
    }
}

pub fn setup_fx(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pool: usize,
) {
    let mut mesh = Sphere::new(0.5).mesh().ico(0).expect("ico");
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    let mesh = meshes.add(mesh);
    let mats: Vec<Handle<StandardMaterial>> = KINDS
        .iter()
        .map(|k| {
            let (c, e, blend) = match k {
                P::Fire => (Color::srgb(1.0, 0.55, 0.1), 7.0, false),
                P::Smoke => (Color::srgba(0.35, 0.33, 0.33, 0.6), 0.0, true),
                P::Steam => (Color::srgba(0.95, 0.97, 1.0, 0.5), 0.3, true),
                P::Frost => (Color::srgba(0.7, 0.92, 1.0, 0.75), 1.6, true),
                P::Foam => (Color::srgb(0.98, 0.98, 1.0), 0.15, false),
                P::Water => (Color::srgb(0.3, 0.6, 1.0), 0.4, false),
                P::Spark => (Color::srgb(1.0, 0.9, 0.5), 10.0, false),
                P::Bead => (Color::srgb(0.97, 0.99, 1.0), 1.2, false),
            };
            let l = c.to_linear();
            materials.add(StandardMaterial {
                base_color: c,
                emissive: LinearRgba::rgb(l.red * e, l.green * e, l.blue * e),
                unlit: matches!(k, P::Fire | P::Spark | P::Steam | P::Frost | P::Bead),
                alpha_mode: if blend {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                },
                perceptual_roughness: 0.8,
                ..default()
            })
        })
        .collect();
    let slots = (0..pool)
        .map(|_| {
            commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(mats[0].clone()),
                    Transform::from_scale(Vec3::ZERO),
                    Visibility::Hidden,
                    NotShadowCaster,
                    NotShadowReceiver,
                    Particle {
                        vel: Vec3::ZERO,
                        life: 0.0,
                        max: 1.0,
                        size: 0.0,
                        grow: 1.0,
                        gravity: 0.0,
                        kind: P::Fire,
                    },
                ))
                .id()
        })
        .collect();
    commands.insert_resource(Fx {
        slots,
        next: 0,
        mats,
        seed: 0x9E37_79B9,
        pending: Vec::new(),
        clear: false,
        epoch: 0,
        water_live: 0,
        bead_live: 0,
    });
}

/// Starts queued particles and moves live ones.
pub fn update_fx(
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    mut q: Query<(
        &mut Particle,
        &mut Transform,
        &mut Visibility,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let dt = clock::frame_dt(time.delta_secs());
    if std::mem::take(&mut fx.clear) {
        fx.pending.clear();
        for (mut p, _, mut vis, _) in &mut q {
            p.life = 0.0;
            *vis = Visibility::Hidden;
        }
    }
    let pending = std::mem::take(&mut fx.pending);
    for (k, pos, vel, size, life) in pending {
        let e = fx.slots[fx.next];
        fx.next = (fx.next + 1) % fx.slots.len();
        if let Ok((mut p, mut t, mut vis, mut mat)) = q.get_mut(e) {
            let ki = KINDS.iter().position(|x| *x == k).unwrap_or(0);
            if p.kind != k || *vis == Visibility::Hidden {
                mat.0 = fx.mats[ki].clone();
            }
            *p = Particle {
                vel,
                life,
                max: life,
                size,
                grow: match k {
                    P::Smoke | P::Steam | P::Frost => 2.2,
                    P::Fire | P::Bead => 0.2,
                    _ => 0.6,
                },
                gravity: match k {
                    P::Water | P::Foam | P::Spark => -9.0,
                    P::Fire => 2.5,
                    P::Smoke | P::Steam => 1.2,
                    P::Frost => -0.6,
                    P::Bead => 0.0,
                },
                kind: k,
            };
            t.translation = pos;
            t.scale = Vec3::splat(size);
            *vis = Visibility::Visible;
        }
    }
    let mut water = 0;
    let mut beads = 0;
    for (mut p, mut t, mut vis, _) in &mut q {
        if p.life <= 0.0 {
            continue;
        }
        p.life -= dt;
        if p.life <= 0.0 {
            *vis = Visibility::Hidden;
            continue;
        }
        if p.kind == P::Water {
            water += 1;
        }
        if p.kind == P::Bead {
            beads += 1;
        }
        let g = p.gravity;
        p.vel.y += g * dt;
        p.vel *= 1.0 - 1.5 * dt;
        t.translation += p.vel * dt;
        if t.translation.y < 0.02 {
            t.translation.y = 0.02;
            p.vel.y = 0.0;
        }
        let a = 1.0 - p.life / p.max;
        t.scale = Vec3::splat(p.size * (1.0 + (p.grow - 1.0) * a) * (1.0 - a * a).max(0.05));
    }
    fx.water_live = water;
    fx.bead_live = beads;
}
