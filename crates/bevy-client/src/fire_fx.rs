//! Fire you can see in the block world (I-052 round 2, P1): flames on
//! burning blocks and lit gas vents, smoke rising from them, and a puff of
//! steam where water puts a fire out. Billboards from small fixed pools, so
//! a big fire never costs more than `FLAMES + SMOKE` quads; the nearest
//! burning cells get them. Voxel mode only (registered by `blockchem`).

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use realgame_matter::{By, MatterEvent as M};

use crate::blockchem::Matter;
use crate::player::Player;
use crate::voxel_view::VoxWorld;

/// Flame quads at most (the nearest burning cells get one each).
pub const FLAMES: usize = 48;
/// Smoke puffs at most (shared out over the nearest burning cells).
pub const SMOKE: usize = 24;
/// Steam puffs alive at most.
pub const STEAM: usize = 32;
/// How long a steam puff lives, seconds.
const STEAM_S: f32 = 1.6;
/// How far smoke rises before it starts again, metres.
const SMOKE_RISE: f32 = 6.0;

#[derive(Component)]
struct Flame(usize);

#[derive(Component)]
struct Smoke(usize);

#[derive(Component)]
struct Steam {
    age: f32,
    vel: Vec3,
}

/// The quad mesh and the three materials.
#[derive(Resource)]
struct FireKit {
    quad: Handle<Mesh>,
    flame: Handle<StandardMaterial>,
    core: Handle<StandardMaterial>,
    smoke: Handle<StandardMaterial>,
    steam: Handle<StandardMaterial>,
}

/// The cells drawn burning this frame (nearest first), for tests and logs.
#[derive(Resource, Default)]
pub struct Shown {
    pub flames: usize,
    pub steam: usize,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Shown>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (draw, steam)
                .after(crate::blockchem::drive)
                .before(crate::publish),
        );
}

fn glow(mats: &mut Assets<StandardMaterial>, c: Color, e: LinearRgba) -> Handle<StandardMaterial> {
    mats.add(StandardMaterial {
        base_color: c,
        emissive: e,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    })
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let kit = FireKit {
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        flame: glow(
            &mut mats,
            Color::srgba(1.0, 0.45, 0.08, 0.85),
            LinearRgba::rgb(3.0, 0.9, 0.1),
        ),
        core: glow(
            &mut mats,
            Color::srgba(1.0, 0.85, 0.3, 0.9),
            LinearRgba::rgb(4.0, 2.6, 0.6),
        ),
        smoke: glow(
            &mut mats,
            Color::srgba(0.25, 0.24, 0.23, 0.35),
            LinearRgba::BLACK,
        ),
        steam: glow(
            &mut mats,
            Color::srgba(0.95, 0.97, 1.0, 0.55),
            LinearRgba::rgb(0.3, 0.3, 0.32),
        ),
    };
    for i in 0..FLAMES {
        let m = if i % 3 == 0 { &kit.core } else { &kit.flame };
        commands.spawn((
            Mesh3d(kit.quad.clone()),
            MeshMaterial3d(m.clone()),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            Flame(i),
        ));
    }
    for i in 0..SMOKE {
        commands.spawn((
            Mesh3d(kit.quad.clone()),
            MeshMaterial3d(kit.smoke.clone()),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            Smoke(i),
        ));
    }
    commands.insert_resource(kit);
}

/// A small hash for flicker phases.
fn jitter(c: [i32; 3], k: u32) -> f32 {
    let h = (c[0] as u32).wrapping_mul(73_856_093)
        ^ (c[1] as u32).wrapping_mul(19_349_663)
        ^ (c[2] as u32).wrapping_mul(83_492_791)
        ^ k.wrapping_mul(2_654_435_761);
    (h % 1000) as f32 / 1000.0
}

/// The burning cells (blocks and lit vents), nearest to `eye` first.
pub fn nearest(mut cells: Vec<[i32; 3]>, eye: Vec3, n: usize) -> Vec<[i32; 3]> {
    let d = |c: &[i32; 3]| {
        (Vec3::new(c[0] as f32 + 0.5, c[1] as f32 + 0.5, c[2] as f32 + 0.5) - eye).length_squared()
    };
    cells.sort_by(|a, b| d(a).total_cmp(&d(b)));
    cells.truncate(n);
    cells
}

/// Places the flame and smoke quads on the nearest burning cells, facing
/// the player, flickering and rising.
#[allow(clippy::type_complexity)]
fn draw(
    time: Res<Time>,
    player: Res<Player>,
    matter: Option<Res<Matter>>,
    vox: Option<Res<VoxWorld>>,
    mut shown: ResMut<Shown>,
    mut flames: Query<(&Flame, &mut Transform, &mut Visibility), Without<Smoke>>,
    mut smoke: Query<(&Smoke, &mut Transform, &mut Visibility), Without<Flame>>,
) {
    let (Some(matter), Some(vox)) = (matter, vox) else {
        return;
    };
    let eye = player.eye();
    let mut cells: Vec<[i32; 3]> = matter.world.burning_cells().collect();
    cells.extend(matter.world.lit_vents(&vox.0));
    let cells = nearest(cells, eye, FLAMES);
    shown.flames = cells.len();
    let t = time.elapsed_secs();
    let face = Quat::from_rotation_y(player.yaw);
    for (f, mut tf, mut vis) in &mut flames {
        let Some(&c) = cells.get(f.0) else {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        };
        let j = jitter(c, 1);
        let flick = 0.8 + 0.25 * (t * (7.0 + 3.0 * j) + j * 6.28).sin();
        let core = f.0 % 3 == 0;
        let (w, h) = if core { (0.55, 0.8) } else { (0.95, 1.35) };
        let base = Vec3::new(c[0] as f32 + 0.5, c[1] as f32 + 1.0, c[2] as f32 + 0.5);
        let sway = 0.08 * (t * 3.0 + j * 9.0).sin();
        tf.translation = base + Vec3::new(sway, h * flick * 0.5 - 0.05, sway * 0.5);
        tf.rotation = face;
        tf.scale = Vec3::new(w * (0.9 + 0.2 * j), h * flick, 1.0);
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
    }
    for (s, mut tf, mut vis) in &mut smoke {
        if cells.is_empty() {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        }
        let c = cells[s.0 % cells.len().min(8)];
        let j = jitter(c, 7 + s.0 as u32);
        let phase = (t * 0.35 + j + s.0 as f32 * 0.41).fract();
        let base = Vec3::new(c[0] as f32 + 0.5, c[1] as f32 + 1.8, c[2] as f32 + 0.5);
        let drift = Vec3::new(
            0.6 * phase * (j - 0.5) * 4.0,
            phase * SMOKE_RISE,
            0.4 * phase,
        );
        tf.translation = base + drift;
        tf.rotation = face;
        let size = 0.6 + 1.8 * phase;
        tf.scale = Vec3::new(size, size, 1.0);
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
    }
}

/// A puff of steam where water puts a fire out (matter's `Extinguished`
/// by water), rising and growing for `STEAM_S`.
fn steam(
    mut commands: Commands,
    time: Res<Time>,
    kit: Option<Res<FireKit>>,
    player: Res<Player>,
    mut shown: ResMut<Shown>,
    mut events: MessageReader<crate::MatterEvent>,
    mut puffs: Query<(Entity, &mut Steam, &mut Transform)>,
) {
    let Some(kit) = kit else {
        return;
    };
    let dt = time.delta_secs().min(0.1);
    let face = Quat::from_rotation_y(player.yaw);
    let mut alive = puffs.iter().count();
    for e in events.read() {
        let M::Extinguished {
            cell,
            by: By::Water,
        } = e.0
        else {
            continue;
        };
        for k in 0..3u32 {
            if alive >= STEAM {
                break;
            }
            alive += 1;
            let j = jitter(cell, 31 + k);
            let p = Vec3::new(
                cell[0] as f32 + 0.2 + 0.6 * j,
                cell[1] as f32 + 1.0,
                cell[2] as f32 + 0.8 - 0.6 * j,
            );
            commands.spawn((
                Mesh3d(kit.quad.clone()),
                MeshMaterial3d(kit.steam.clone()),
                Transform::from_translation(p).with_scale(Vec3::splat(0.5)),
                NotShadowCaster,
                NotShadowReceiver,
                Steam {
                    age: 0.0,
                    vel: Vec3::new(0.4 * (j - 0.5), 1.6 + j, 0.3 * (0.5 - j)),
                },
            ));
        }
    }
    for (e, mut s, mut tf) in &mut puffs {
        s.age += dt;
        if s.age >= STEAM_S {
            commands.entity(e).despawn();
            continue;
        }
        tf.translation += s.vel * dt;
        tf.rotation = face;
        let k = s.age / STEAM_S;
        tf.scale = Vec3::splat(0.5 + 1.6 * k);
    }
    shown.steam = alive;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_cells_get_the_flames() {
        let cells = vec![[50, 0, 0], [1, 0, 0], [10, 0, 0], [3, 0, 0]];
        let n = nearest(cells, Vec3::ZERO, 2);
        assert_eq!(n, vec![[1, 0, 0], [3, 0, 0]]);
    }
}
