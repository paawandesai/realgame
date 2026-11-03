//! The ending scene (submission P9): when the Great Kiln is relit, the
//! camera leaves the player and circles the Kiln slowly, high enough to see
//! the valley around it, while the ending card and credits show. It hands
//! the view back when the player picks Keep exploring (or after
//! `ORBIT_S`). The camera only: the player does not move. Voxel mode only
//! (registered by `story::plugin`).

use bevy::prelude::*;

use crate::late::Late;
use crate::player::Player;

/// What the camera circles: the Kiln's hearth.
pub const KILN: Vec3 = Vec3::new(128.5, 41.0, 169.5);
/// The circle's radius and how high above the hearth it flies (m).
pub const RADIUS: f32 = 34.0;
pub const HEIGHT: f32 = 22.0;
/// Radians per second around the Kiln (a full turn in about 50 s).
pub const SPIN: f32 = 0.125;
/// The longest the scene holds the camera (s).
pub const ORBIT_S: f32 = 45.0;
/// How long the camera takes to glide from the player's eye to the circle.
pub const EASE_S: f32 = 2.5;

#[derive(Resource, Default, Debug)]
pub struct Scene {
    /// Seconds since the scene began; `None` before the Kiln is relit or
    /// after it handed the view back.
    pub t: Option<f32>,
    /// The angle round the Kiln it starts at (from where the player stood).
    start: f32,
    /// The camera's place and rotation when it began (the eye).
    from: Option<Transform>,
    /// Done: never takes the camera again this run.
    done: bool,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Scene>()
        .add_systems(Update, orbit.after(crate::control).before(crate::publish));
}

/// The camera on the circle at time `t` (s) from start angle `a0`, looking
/// at the Kiln.
pub fn on_circle(a0: f32, t: f32) -> Transform {
    let a = a0 + SPIN * t;
    let at = KILN + Vec3::new(a.cos() * RADIUS, HEIGHT, a.sin() * RADIUS);
    Transform::from_translation(at).looking_at(KILN + Vec3::Y * 2.0, Vec3::Y)
}

/// Smoothstep ease 0..1 over `EASE_S`.
fn ease(t: f32) -> f32 {
    let x = (t / EASE_S).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn orbit(
    time: Res<Time>,
    late: Option<Res<Late>>,
    player: Res<Player>,
    mut scene: ResMut<Scene>,
    mut cam: Query<&mut Transform, With<crate::FpsCam>>,
) {
    let Some(late) = late else { return };
    let Ok(mut tf) = cam.single_mut() else { return };
    if scene.done {
        return;
    }
    match scene.t {
        None if late.card => {
            let d = player.pos - KILN;
            scene.start = d.z.atan2(d.x);
            scene.from = Some(*tf);
            scene.t = Some(0.0);
        }
        None => return,
        Some(t) if !late.card || t >= ORBIT_S => {
            scene.t = None;
            scene.done = true;
            return;
        }
        Some(_) => {}
    }
    let t = scene.t.unwrap_or(0.0) + time.delta_secs().min(0.1);
    scene.t = Some(t);
    let goal = on_circle(scene.start, t);
    let k = ease(t);
    let from = scene.from.unwrap_or(goal);
    tf.translation = from.translation.lerp(goal.translation, k);
    tf.rotation = from.rotation.slerp(goal.rotation, k);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ending_camera_circles_the_kiln_looking_at_it() {
        for t in [0.0, 5.0, 20.0] {
            let tf = on_circle(0.3, t);
            let flat = (tf.translation - KILN).xz().length();
            assert!((flat - RADIUS).abs() < 1e-3, "{flat}");
            assert!((tf.translation.y - KILN.y - HEIGHT).abs() < 1e-3);
            // Looking at the Kiln: forward points at it.
            let to = (KILN + Vec3::Y * 2.0 - tf.translation).normalize();
            assert!(tf.forward().dot(to) > 0.999);
        }
        // It moves round.
        assert!(
            on_circle(0.0, 0.0)
                .translation
                .distance(on_circle(0.0, 4.0).translation)
                > 5.0
        );
    }

    #[test]
    fn the_glide_in_is_smooth_and_ends_on_the_circle() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(EASE_S), 1.0);
        assert!(ease(EASE_S * 0.5) > 0.4 && ease(EASE_S * 0.5) < 0.6);
    }
}
