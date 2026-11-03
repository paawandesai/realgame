//! The objective beacon (Wednesday polish): a soft column of light standing
//! on the current quest step's target, so a new player can see where to go
//! from anywhere in the valley, not only from the HUD arrow. It fades out as
//! the player walks up to it (it would fill the screen), hides during the
//! intro and after the ending, and is drawn unlit, without fog or shadows.
//! Registered only in `Mode::Voxel` (by `story::plugin`).

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use crate::player::Player;
use crate::story::{water_known, Story, TARGETS};
use crate::voxel_view::VoxWorld;

/// How tall the beam stands above the ground (m).
const HEIGHT: f32 = 70.0;
/// Inner core and outer halo radii (m): a core about half a metre across
/// (B4 judges: the first beam read as a wide searchlight).
const CORE_R: f32 = 0.25;
const HALO_R: f32 = 0.6;
/// Closer than `NEAR` (horizontal m) the beam is gone; past `FULL` it is at
/// full strength, with a smooth ramp between.
const NEAR: f32 = 3.0;
const FULL: f32 = 14.0;
/// Peak alpha of the core and the halo (additive, so "brightness").
const CORE_A: f32 = 0.32;
const HALO_A: f32 = 0.09;

#[derive(Component)]
struct Beam;

#[derive(Resource)]
struct BeamMats {
    core: Handle<StandardMaterial>,
    halo: Handle<StandardMaterial>,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup).add_systems(Update, place);
}

/// The beam's strength (0-1) for quest `step` with the player `d` metres
/// (horizontally) from the target: none before the intro closes (step 0)
/// or after the last step, and fading out up close.
pub fn strength(step: u8, d: f32) -> f32 {
    if !(1..=TARGETS.len() as u8).contains(&step) {
        return 0.0;
    }
    let t = ((d - NEAR) / (FULL - NEAR)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The top of the ground at column (x, z): the highest cell that is not
/// air or leaves (a tree crown would lift the beam off the ground).
fn ground_top(w: &realgame_voxel::World, x: f32, z: f32) -> f32 {
    use realgame_voxel::mat;
    let (x, z) = (x.floor() as i32, z.floor() as i32);
    for y in (0..realgame_voxel::WORLD[1]).rev() {
        let m = w.get([x, y, z]).mat;
        if m != mat::AIR && m != mat::LEAVES && m != mat::WOOD {
            return y as f32 + 1.0;
        }
    }
    0.0
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let mat = |a: f32| StandardMaterial {
        base_color: Color::srgba(1.0, 0.86, 0.45, a),
        unlit: true,
        fog_enabled: false,
        alpha_mode: AlphaMode::Add,
        double_sided: true,
        cull_mode: None,
        ..default()
    };
    let core = mats.add(mat(CORE_A));
    let halo = mats.add(mat(HALO_A));
    for (r, m) in [(CORE_R, core.clone()), (HALO_R, halo.clone())] {
        commands.spawn((
            Beam,
            Mesh3d(meshes.add(Cylinder::new(r, HEIGHT).mesh().resolution(16))),
            MeshMaterial3d(m),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
    commands.insert_resource(BeamMats { core, halo });
}

#[allow(clippy::too_many_arguments)]
fn place(
    time: Res<Time>,
    story: Res<Story>,
    progress: Res<crate::Progress>,
    player: Res<Player>,
    world: Option<Res<VoxWorld>>,
    beams_mats: Option<Res<BeamMats>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut beams: Query<(&mut Transform, &mut Visibility), With<Beam>>,
    mut top: Local<Option<((f32, f32), f32)>>,
) {
    let step = story.quest.step();
    let Some((tx, tz)) = crate::story::aim(&story, water_known(&progress)) else {
        for (_, mut v) in &mut beams {
            *v = Visibility::Hidden;
        }
        return;
    };
    // The ground under the target, found once per target.
    let y = match *top {
        Some((at, y)) if at == (tx, tz) => y,
        _ => {
            let Some(w) = world else { return };
            let y = ground_top(&w.0, tx, tz);
            *top = Some(((tx, tz), y));
            y
        }
    };
    let d = Vec2::new(player.pos.x - tx, player.pos.z - tz).length();
    let k = strength(step, d);
    // A slow breath so it reads as alive, not a pole.
    let breathe = 0.85 + 0.15 * (time.elapsed_secs() * 1.4).sin();
    if let Some(bm) = beams_mats {
        for (h, a) in [(&bm.core, CORE_A), (&bm.halo, HALO_A)] {
            if let Some(mut m) = mats.get_mut(h) {
                m.base_color.set_alpha(a * k * breathe);
            }
        }
    }
    for (mut t, mut v) in &mut beams {
        *v = if k > 0.01 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        t.translation = Vec3::new(tx, y + HEIGHT / 2.0, tz);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_beam_shows_only_for_a_live_step_and_fades_up_close() {
        assert_eq!(strength(0, 100.0), 0.0, "not during the intro");
        assert_eq!(
            strength(TARGETS.len() as u8 + 1, 100.0),
            0.0,
            "not after the ending"
        );
        assert_eq!(strength(1, 100.0), 1.0, "full from afar");
        assert_eq!(strength(3, FULL), 1.0);
        assert_eq!(strength(3, NEAR), 0.0, "gone when you are there");
        assert_eq!(strength(3, 1.0), 0.0);
        let mid = strength(3, (NEAR + FULL) / 2.0);
        assert!(mid > 0.3 && mid < 0.7, "{mid}");
        // Monotonic as you walk away.
        let mut last = 0.0;
        for i in 0..40 {
            let s = strength(2, i as f32);
            assert!(s >= last);
            last = s;
        }
    }

    #[test]
    fn the_beam_stands_on_the_ground_not_on_a_tree() {
        use realgame_voxel::{mat, Cell, World};
        let mut w = World::new();
        for y in 0..30 {
            w.set([5, y, 5], Cell::new(mat::STONE));
        }
        for y in 30..34 {
            w.set([5, y, 5], Cell::new(mat::WOOD));
        }
        w.set([5, 34, 5], Cell::new(mat::LEAVES));
        assert_eq!(ground_top(&w, 5.5, 5.5), 30.0);
    }
}
