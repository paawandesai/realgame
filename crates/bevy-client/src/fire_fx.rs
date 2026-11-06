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
const SMOKE_RISE: f32 = 10.0;

#[derive(Component)]
struct Flame(usize);

#[derive(Component)]
struct Smoke(usize);

/// A soft heat plume over a lit Marsh vent (methane burns clean: no
/// grey smoke there).
#[derive(Component)]
struct Plume(usize);

/// Plume puffs at most.
const PLUMES: usize = 12;

/// A warm point light at a fire (B4 round 3: the burning workshop and the
/// lit vents threw no light on the ground). Full quality only.
#[derive(Component)]
struct FireGlow(usize);

/// Fire lights at most: the nearest fires, at least `GLOW_APART` m apart.
const GLOWS: usize = 3;
const GLOW_APART: f32 = 4.0;

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
    plume: Handle<StandardMaterial>,
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

/// P10: soft sprite shapes made in code, so flames and smoke read as fire
/// from across the plaza instead of as square tiles. `flame`: a teardrop,
/// wide and solid at the bottom, tapering to a point; else a soft disc.
pub fn sprite_alpha(flame: bool, u: f32, v: f32) -> f32 {
    // u, v in 0..1; v = 0 is the top of the quad.
    let x = (u - 0.5) * 2.0;
    if flame {
        let up = 1.0 - v; // 0 at the bottom, 1 at the top
                          // Half-width: round at the base, narrowing to a tip.
        let w = (1.0 - up).powf(0.8) * (0.55 + 0.45 * (up * 3.0).min(1.0));
        let edge = if w <= 0.0 { 1.0 } else { x.abs() / w };
        let base = ((1.0 - up) * 6.0).min(1.0) * (up * 8.0).clamp(0.6, 1.0);
        (1.0 - edge.powf(1.6)).clamp(0.0, 1.0) * base
    } else {
        let y = (v - 0.5) * 2.0;
        let r = (x * x + y * y).sqrt();
        (1.0 - r).clamp(0.0, 1.0).powf(1.4)
    }
}

pub(crate) fn sprite(flame: bool) -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const N: u32 = 64;
    let mut data = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let a = sprite_alpha(
                flame,
                (x as f32 + 0.5) / N as f32,
                (y as f32 + 0.5) / N as f32,
            );
            data.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: N,
            height: N,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Value noise for the realistic sprites (tileable enough at this size).
fn vnoise(x: f32, y: f32, seed: u32) -> f32 {
    let h = |i: i32, j: i32| {
        let mut n =
            (i as u32).wrapping_mul(0x27d4_eb2d) ^ (j as u32).wrapping_mul(0x1656_67b1) ^ seed;
        n ^= n >> 15;
        n = n.wrapping_mul(0x2c1b_3c6d);
        n ^= n >> 12;
        (n & 0xffff) as f32 / 65535.0
    };
    let (i, j) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = h(i, j) + (h(i + 1, j) - h(i, j)) * sx;
    let b = h(i, j + 1) + (h(i + 1, j + 1) - h(i, j + 1)) * sx;
    a + (b - a) * sy
}

fn fbm(x: f32, y: f32, seed: u32) -> f32 {
    let (mut v, mut amp, mut f) = (0.0, 0.5, 1.0);
    for o in 0..4 {
        v += amp * vnoise(x * f, y * f, seed.wrapping_add(o));
        amp *= 0.5;
        f *= 2.0;
    }
    v
}

/// Blackbody-ish colour of a flame at heat `t` (0 edge, 1 core), sRGB:
/// deep red at the cool edge, orange, yellow, near-white at the core.
fn flame_colour(t: f32) -> [f32; 3] {
    let stops = [
        (0.0, [0.45, 0.04, 0.0]),
        (0.35, [0.95, 0.30, 0.03]),
        (0.65, [1.0, 0.62, 0.12]),
        (1.0, [1.0, 0.93, 0.70]),
    ];
    for k in 0..stops.len() - 1 {
        let (a, ca) = stops[k];
        let (b, cb) = stops[k + 1];
        if t <= b {
            let u = ((t - a) / (b - a)).clamp(0.0, 1.0);
            return [0, 1, 2].map(|c| ca[c] + (cb[c] - ca[c]) * u);
        }
    }
    stops[3].1
}

/// The realistic look's sprites (2026-11-05, Paawan: one consistent real
/// look): a turbulent flame tongue coloured by temperature, drawn
/// additively; or a billowy smoke puff with darker folds.
pub(crate) fn sprite_real(flame: bool, seed: u32) -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const N: u32 = 128;
    let mut data = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let (u, v) = ((x as f32 + 0.5) / N as f32, (y as f32 + 0.5) / N as f32);
            if flame {
                // Turbulence bends the tongue sideways more towards the tip.
                let up = 1.0 - v;
                let n = fbm(u * 4.0, v * 3.0, seed);
                let uu = u + (n - 0.5) * 0.35 * up;
                let a0 = sprite_alpha(true, uu, v);
                let breakup =
                    (fbm(u * 6.0, v * 5.0 + 3.1, seed ^ 0x55) * 1.6 - 0.25).clamp(0.0, 1.0);
                let a = (a0 * mix(1.0, breakup, up * 0.8)).clamp(0.0, 1.0);
                let heat = (a0.powf(0.7) * (1.0 - up * 0.45)).clamp(0.0, 1.0);
                let c = flame_colour(heat);
                // Additive: brightness in the colour, alpha as the mask.
                data.extend_from_slice(&[
                    (c[0] * a * 255.0) as u8,
                    (c[1] * a * 255.0) as u8,
                    (c[2] * a * 255.0) as u8,
                    (a * 255.0) as u8,
                ]);
            } else {
                let d = sprite_alpha(false, u, v);
                let n = fbm(u * 5.0, v * 5.0, seed);
                let a = (d * (0.55 + 0.9 * n) - 0.12).clamp(0.0, 1.0);
                let shade = 0.75 + 0.5 * fbm(u * 3.0 + 7.0, v * 3.0, seed ^ 0x99);
                let g = (shade.min(1.0) * 255.0) as u8;
                data.extend_from_slice(&[g, g, g, (a * 255.0) as u8]);
            }
        }
    }
    Image::new(
        Extent3d {
            width: N,
            height: N,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
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
    mut images: ResMut<Assets<Image>>,
    settings: Option<Res<crate::Settings>>,
) {
    if !settings.is_some_and(|s| s.low) {
        for i in 0..GLOWS {
            commands.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.55, 0.2),
                    intensity: 0.0,
                    range: 18.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::default(),
                FireGlow(i),
            ));
        }
    }
    let real = crate::look::on();
    let flame_tex = images.add(if real {
        sprite_real(true, 7)
    } else {
        sprite(true)
    });
    let puff_tex = images.add(if real {
        sprite_real(false, 11)
    } else {
        sprite(false)
    });
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
            Color::srgba(0.22, 0.21, 0.2, 0.55),
            LinearRgba::BLACK,
        ),
        steam: glow(
            &mut mats,
            Color::srgba(0.95, 0.97, 1.0, 0.55),
            LinearRgba::rgb(0.3, 0.3, 0.32),
        ),
        plume: glow(
            &mut mats,
            Color::srgba(0.95, 0.85, 0.6, 0.16),
            LinearRgba::rgb(0.12, 0.08, 0.02),
        ),
    };
    for (h, tex) in [
        (&kit.flame, &flame_tex),
        (&kit.core, &flame_tex),
        (&kit.smoke, &puff_tex),
        (&kit.steam, &puff_tex),
        (&kit.plume, &puff_tex),
    ] {
        if let Some(mut m) = mats.get_mut(h) {
            m.base_color_texture = Some(tex.clone());
        }
    }
    if real {
        // Real fire is light: flames add to what is behind them, white
        // tinted by the sprite's temperature colours; smoke is a dark,
        // slightly translucent soot grey.
        for h in [&kit.flame, &kit.core] {
            if let Some(mut m) = mats.get_mut(h) {
                m.base_color = Color::srgba(1.0, 1.0, 1.0, 1.0);
                m.emissive = LinearRgba::rgb(2.2, 2.2, 2.2);
                m.emissive_texture = Some(flame_tex.clone());
                m.alpha_mode = AlphaMode::Add;
            }
        }
        if let Some(mut m) = mats.get_mut(&kit.smoke) {
            m.base_color = Color::srgba(0.16, 0.15, 0.14, 0.7);
        }
    }
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
    for i in 0..PLUMES {
        commands.spawn((
            Mesh3d(kit.quad.clone()),
            MeshMaterial3d(kit.plume.clone()),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            Plume(i),
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
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn draw(
    time: Res<Time>,
    player: Res<Player>,
    matter: Option<Res<Matter>>,
    vox: Option<Res<VoxWorld>>,
    mut shown: ResMut<Shown>,
    mut flames: Query<(&Flame, &mut Transform, &mut Visibility), (Without<Smoke>, Without<Plume>)>,
    mut smoke: Query<(&Smoke, &mut Transform, &mut Visibility), (Without<Flame>, Without<Plume>)>,
    mut plumes: Query<(&Plume, &mut Transform, &mut Visibility), (Without<Flame>, Without<Smoke>)>,
    mut glows: Query<
        (&FireGlow, &mut PointLight, &mut Transform),
        (Without<Flame>, Without<Smoke>, Without<Plume>),
    >,
) {
    let (Some(matter), Some(vox)) = (matter, vox) else {
        return;
    };
    let eye = player.eye();
    let burning: Vec<[i32; 3]> = matter.world.burning_cells().collect();
    let vents: Vec<[i32; 3]> = matter.world.lit_vents(&vox.0).collect();
    let mut cells = burning.clone();
    cells.extend(vents.iter().copied());
    let cells = nearest(cells, eye, FLAMES);
    // Spare flame quads go to the burning blocks again, as taller tongues
    // round each one (B4 round 3: the workshop's dozen burning cells read
    // as a small fire from the plaza).
    let mut drawn: Vec<([i32; 3], u32)> = cells.iter().map(|&c| (c, 0)).collect();
    let fuel: Vec<[i32; 3]> = cells
        .iter()
        .copied()
        .filter(|c| !vents.contains(c))
        .collect();
    let mut k = 0usize;
    while !fuel.is_empty() && drawn.len() < FLAMES && k < FLAMES * 3 {
        drawn.push((fuel[k % fuel.len()], 1 + (k / fuel.len()) as u32));
        k += 1;
    }
    let smoky = nearest(burning, eye, 8);
    let vents = nearest(vents, eye, PLUMES);
    shown.flames = cells.len();
    let t = time.elapsed_secs();
    // Fire light: the nearest fires, spread out.
    let mut at: Vec<(Vec3, bool)> = Vec::new();
    for &c in &cells {
        let p = Vec3::new(c[0] as f32 + 0.5, c[1] as f32 + 1.6, c[2] as f32 + 0.5);
        if at.len() < GLOWS && at.iter().all(|(q, _)| q.distance(p) >= GLOW_APART) {
            at.push((p, vents.contains(&c)));
        }
    }
    for (g, mut l, mut tf) in &mut glows {
        match at.get(g.0) {
            Some(&(p, vent)) if p.distance(eye) < 60.0 => {
                let j = g.0 as f32 * 1.7;
                let flick = 1.0 + 0.18 * (t * 9.0 + j).sin() + 0.1 * (t * 23.0 + j * 2.0).sin();
                l.intensity = if vent { 180_000.0 } else { 1_300_000.0 } * flick;
                tf.translation = p;
            }
            _ => l.intensity = 0.0,
        }
    }
    let face = Quat::from_rotation_y(player.yaw);
    for (f, mut tf, mut vis) in &mut flames {
        let Some(&(c, extra)) = drawn.get(f.0) else {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        };
        let j = jitter(c, 1 + extra * 13);
        let flick = 0.8 + 0.25 * (t * (7.0 + 3.0 * j) + j * std::f32::consts::TAU).sin();
        let core = f.0 % 3 == 0;
        // A vent's flame is a smaller tongue out of its mound's throat.
        let vent = vents.contains(&c);
        let (w, h) = match (core, vent) {
            (true, false) => (1.0, 1.7),
            (false, false) => (1.6, 2.8),
            (true, true) => (0.45, 0.8),
            (false, true) => (0.7, 1.25),
        };
        let lift = if vent { 1.9 } else { 1.0 };
        let base = Vec3::new(c[0] as f32 + 0.5, c[1] as f32 + lift, c[2] as f32 + 0.5);
        let sway = 0.08 * (t * 3.0 + j * 9.0).sin();
        // A repeat tongue stands a little off the block and higher.
        let (h, spread) = if extra > 0 {
            (
                h * (1.15 + 0.25 * j),
                Vec3::new(j - 0.5, 0.25 * extra as f32, jitter(c, 5 + extra) - 0.5) * 0.9,
            )
        } else {
            (h, Vec3::ZERO)
        };
        tf.translation = base + spread + Vec3::new(sway, h * flick * 0.5 - 0.05, sway * 0.5);
        tf.rotation = face;
        tf.scale = Vec3::new(w * (0.9 + 0.2 * j), h * flick, 1.0);
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
    }
    for (s, mut tf, mut vis) in &mut smoke {
        if smoky.is_empty() {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        }
        let c = smoky[s.0 % smoky.len()];
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
        let size = 1.0 + 3.2 * phase;
        tf.scale = Vec3::new(size, size, 1.0);
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
    }
    for (pl, mut tf, mut vis) in &mut plumes {
        if vents.is_empty() {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        }
        let c = vents[pl.0 % vents.len()];
        let j = jitter(c, 31 + pl.0 as u32);
        let phase = (t * 0.3 + j + pl.0 as f32 * 0.37).fract();
        let base = Vec3::new(c[0] as f32 + 0.5, c[1] as f32 + 2.7, c[2] as f32 + 0.5);
        tf.translation = base + Vec3::new(0.3 * (j - 0.5) * phase, phase * 4.0, 0.25 * phase);
        tf.rotation = face;
        let size = 0.6 + 1.6 * phase;
        tf.scale = Vec3::new(size, size * 1.2, 1.0);
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
    #[test]
    fn the_flame_sprite_is_a_soft_teardrop_and_smoke_a_soft_disc() {
        use super::sprite_alpha;
        // Solid low in the middle, clear at the corners and the tip's sides.
        assert!(sprite_alpha(true, 0.5, 0.75) > 0.7);
        assert!(sprite_alpha(true, 0.02, 0.98) < 0.1);
        assert!(sprite_alpha(true, 0.1, 0.1) < 0.05);
        assert!(sprite_alpha(true, 0.5, 0.75) > sprite_alpha(true, 0.5, 0.15));
        // Smoke: solid centre, clear corners, no hard edge.
        assert!(sprite_alpha(false, 0.5, 0.5) > 0.9);
        assert_eq!(sprite_alpha(false, 0.0, 0.0), 0.0);
        assert!(sprite_alpha(false, 0.5, 0.06) < 0.2);
    }

    use super::*;

    #[test]
    fn the_nearest_cells_get_the_flames() {
        let cells = vec![[50, 0, 0], [1, 0, 0], [10, 0, 0], [3, 0, 0]];
        let n = nearest(cells, Vec3::ZERO, 2);
        assert_eq!(n, vec![[1, 0, 0], [3, 0, 0]]);
    }
}
