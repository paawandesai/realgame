//! How each spell moves (I-051): a water jet and a CO2 puff fly under
//! gravity, Fire and Frost are short cones from the hand. Pure maths on
//! `Vec3` (no world, no Bevy systems), so it is unit tested here; `main.rs`
//! traces the paths through the level and draws them.
//!
//! The jet's launch speed is √(g·R): R is how far it would carry on flat
//! ground at the best angle (45°). R grows with how long the button has been
//! held: a click reaches about 6.5 m, a 0.8 s charge about 12 m and a full
//! 1.5 s charge 17.5 m. Water and CO2 charge while the button is down and
//! fly when it is released.

use bevy::prelude::*;

/// Gravity, m/s² (standard gravity is 9.80665 m/s²).
pub const G: f32 = 9.81;

/// Water's flat-ground reach for a click and for a full charge, metres
/// (gameplay tuning).
pub const WATER_REACH: (f32, f32) = (6.5, 17.5);
/// CO2's lob: reach for a click and for a full charge, metres.
pub const CO2_REACH: (f32, f32) = (5.0, 9.0);
/// The charge (seconds held) that gives the full reach.
pub const FULL_HOLD_S: f32 = 1.5;
/// Fire's flame cone: range (m) and half-angle (degrees) (gameplay tuning).
pub const FIRE_CONE: (f32, f32) = (2.5, 18.0);
/// Frost's spray cone: range (m) and half-angle (degrees).
pub const FROST_CONE: (f32, f32) = (3.0, 20.0);
/// A water splash on bare ground still wets an object this close (m).
pub const SPLASH_R: f32 = 0.5;
/// Settled CO2 spreads along the ground this far (m).
pub const CO2_SPREAD: f32 = 1.5;

/// How a spell moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    /// Flies under gravity with a launch speed for this flat-ground reach.
    Arc { reach: f32 },
    /// A cone from the hand: range (m) and half-angle (radians).
    Cone { range: f32, half: f32 },
}

/// The motion of spell `spell` (0 Fire, 1 Water, 2 Frost, 3 CO2) when the
/// button has been held `held` seconds.
pub fn motion(spell: u8, held: f32) -> Motion {
    match spell {
        1 => Motion::Arc {
            reach: reach_for_hold(WATER_REACH.0, WATER_REACH.1, held),
        },
        3 => Motion::Arc {
            reach: reach_for_hold(CO2_REACH.0, CO2_REACH.1, held),
        },
        2 => Motion::Cone {
            range: FROST_CONE.0,
            half: FROST_CONE.1.to_radians(),
        },
        _ => Motion::Cone {
            range: FIRE_CONE.0,
            half: FIRE_CONE.1.to_radians(),
        },
    }
}

/// Reach between `min` (a tap) and `max` (held [`FULL_HOLD_S`] or longer),
/// linear in between; a negative or non-finite hold is a tap.
pub fn reach_for_hold(min: f32, max: f32, held: f32) -> f32 {
    let f = if held.is_finite() {
        (held / FULL_HOLD_S).clamp(0.0, 1.0)
    } else {
        0.0
    };
    min + (max - min) * f
}

/// Launch speed for a flat-ground reach at 45°: R = v²/g.
pub fn launch_speed(reach: f32) -> f32 {
    (G * reach.max(0.0)).sqrt()
}

/// Position at time `t` after leaving `p0` with velocity `v0`.
pub fn at(p0: Vec3, v0: Vec3, t: f32) -> Vec3 {
    p0 + v0 * t - Vec3::Y * (0.5 * G * t * t)
}

/// The top of the path: when, and where (at the start if it goes down).
#[cfg_attr(not(test), allow(dead_code))]
pub fn apex(p0: Vec3, v0: Vec3) -> (f32, Vec3) {
    let t = (v0.y / G).max(0.0);
    (t, at(p0, v0, t))
}

/// When the path comes down to height `y` (after leaving `p0`), if it does.
#[cfg_attr(not(test), allow(dead_code))]
pub fn landing_time(p0: Vec3, v0: Vec3, y: f32) -> Option<f32> {
    // p0.y + v0.y t - g t²/2 = y  ->  (g/2) t² - v0.y t + (y - p0.y) = 0
    let (a, b, c) = (0.5 * G, -v0.y, y - p0.y);
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return None;
    }
    let t = (-b + disc.sqrt()) / (2.0 * a);
    (t >= 0.0).then_some(t)
}

/// The launch velocity of speed `speed` from `p0` whose path passes through
/// `target`: the lower of the two arcs, or `None` if `speed` cannot reach it.
pub fn aim_velocity(p0: Vec3, target: Vec3, speed: f32) -> Option<Vec3> {
    let dv = target - p0;
    let d = dv.xz().length();
    let h = dv.y;
    let v2 = speed * speed;
    if d < 1e-4 {
        // Straight up or down.
        return if h <= 0.0 {
            Some(Vec3::NEG_Y * speed)
        } else if v2 >= 2.0 * G * h {
            Some(Vec3::Y * speed)
        } else {
            None
        };
    }
    let disc = v2 * v2 - G * (G * d * d + 2.0 * h * v2);
    if disc < 0.0 {
        return None;
    }
    let tan = (v2 - disc.sqrt()) / (G * d);
    let up = tan.atan();
    let flat = Vec3::new(dv.x, 0.0, dv.z) / d;
    Some(flat * (speed * up.cos()) + Vec3::Y * (speed * up.sin()))
}

/// Whether `p` lies inside the cone from `apex` along unit `dir`, out to
/// `range`, within `half` radians of the axis.
pub fn in_cone(apex: Vec3, dir: Vec3, range: f32, half: f32, p: Vec3) -> bool {
    let v = p - apex;
    let d = v.length();
    if d < 1e-6 {
        return true;
    }
    if d > range {
        return false;
    }
    let c = (v.dot(dir) / d).clamp(-1.0, 1.0);
    c.acos() <= half
}

/// Traces the path from `p0` with `v0` in steps of `dt` until `hit(a, b)`
/// says the segment a→b hits something (returning how far along, 0..1), or
/// `max_t` runs out. Returns the points so far (the last is the landing
/// point) and the flight time.
pub fn path_hits(
    p0: Vec3,
    v0: Vec3,
    dt: f32,
    max_t: f32,
    mut hit: impl FnMut(Vec3, Vec3) -> Option<f32>,
) -> (Vec<Vec3>, f32) {
    let mut pts = vec![p0];
    let mut t = 0.0f32;
    while t < max_t {
        let step = dt.min(max_t - t);
        let (a, b) = (at(p0, v0, t), at(p0, v0, t + step));
        if let Some(f) = hit(a, b) {
            let f = f.clamp(0.0, 1.0);
            pts.push(a.lerp(b, f));
            return (pts, t + f * step);
        }
        pts.push(b);
        t += step;
    }
    (pts, max_t)
}

/// The point at fraction `a` (0..1) of a path traced in equal time steps.
pub fn along(path: &[Vec3], a: f32) -> Vec3 {
    match path.len() {
        0 => Vec3::ZERO,
        1 => path[0],
        n => {
            let x = a.clamp(0.0, 1.0) * (n - 1) as f32;
            let i = (x.floor() as usize).min(n - 2);
            path[i].lerp(path[i + 1], x - i as f32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(speed: f32, deg: f32) -> Vec3 {
        let a = deg.to_radians();
        Vec3::new(0.0, a.sin(), -a.cos()) * speed
    }

    #[test]
    fn i051_apex_height_and_time_match_the_textbook() {
        for (v, deg) in [(7.0f32, 45.0f32), (10.0, 30.0), (9.9, 60.0)] {
            let v0 = launch(v, deg);
            let s = deg.to_radians().sin();
            let (t, p) = apex(Vec3::ZERO, v0);
            assert!((t - v * s / G).abs() < 1e-4, "t {t}");
            assert!((p.y - v * v * s * s / (2.0 * G)).abs() < 1e-4, "h {}", p.y);
            assert!((at(Vec3::ZERO, v0, t) - p).length() < 1e-4);
            // Gravity: the path is a parabola, not a line.
            let mid = at(Vec3::ZERO, v0, t * 0.5);
            assert!(mid.y < (v0 * t * 0.5).y - 1e-3);
        }
    }

    #[test]
    fn i051_flat_ground_range_is_v2_sin2_theta_over_g() {
        let v = launch_speed(10.0);
        assert!((v * v / G - 10.0).abs() < 1e-3, "speed for 10 m reach {v}");
        for deg in [15.0f32, 30.0, 45.0, 60.0, 75.0] {
            let v0 = launch(v, deg);
            let t = landing_time(Vec3::ZERO, v0, 0.0).expect("lands");
            let land = at(Vec3::ZERO, v0, t);
            let want = v * v * (2.0 * deg.to_radians()).sin() / G;
            assert!(
                (land.xz().length() - want).abs() < 1e-3,
                "{deg}°: {} vs {want}",
                land.xz().length()
            );
            assert!(land.y.abs() < 1e-3);
        }
        // From a raised hand the jet lands later and farther.
        let v0 = launch(v, 30.0);
        let t0 = landing_time(Vec3::ZERO, v0, 0.0).unwrap();
        let t1 = landing_time(Vec3::Y * 1.45, v0, 0.0).unwrap();
        assert!(t1 > t0);
        assert!(at(Vec3::Y * 1.45, v0, t1).xz().length() > at(Vec3::ZERO, v0, t0).xz().length());
        // Thrown straight down from 0, already below: no landing above.
        assert!(landing_time(Vec3::ZERO, Vec3::new(0.0, -1.0, 0.0), 1.0).is_none());
    }

    #[test]
    fn i051_reach_grows_with_hold_time() {
        let (a, b) = WATER_REACH;
        assert_eq!(reach_for_hold(a, b, 0.0), a);
        assert_eq!(reach_for_hold(a, b, FULL_HOLD_S), b);
        assert_eq!(reach_for_hold(a, b, 5.0), b);
        assert_eq!(reach_for_hold(a, b, -1.0), a);
        assert_eq!(reach_for_hold(a, b, f32::NAN), a);
        let mut last = 0.0;
        for k in 0..=16 {
            let r = reach_for_hold(a, b, k as f32 * 0.05);
            assert!(r >= last);
            last = r;
        }
        assert_eq!(motion(1, 0.0), Motion::Arc { reach: 6.5 });
        assert_eq!(motion(1, FULL_HOLD_S), Motion::Arc { reach: 17.5 });
        match motion(1, 0.8) {
            Motion::Arc { reach } => assert!((11.5..=13.0).contains(&reach), "0.8 s: {reach}"),
            m => panic!("water is an arc, got {m:?}"),
        }
        assert_eq!(motion(3, 0.0), Motion::Arc { reach: 5.0 });
        assert_eq!(motion(3, FULL_HOLD_S), Motion::Arc { reach: 9.0 });
        match motion(0, 0.8) {
            Motion::Cone { range, half } => {
                assert_eq!(range, 2.5);
                assert!((half - 18f32.to_radians()).abs() < 1e-6);
            }
            m => panic!("fire is a cone, got {m:?}"),
        }
        match motion(2, 0.0) {
            Motion::Cone { range, half } => {
                assert_eq!(range, 3.0);
                assert!((half - 20f32.to_radians()).abs() < 1e-6);
            }
            m => panic!("frost is a cone, got {m:?}"),
        }
    }

    #[test]
    fn i051_aim_velocity_lands_on_the_target_with_the_low_arc() {
        let hand = Vec3::new(0.2, 1.45, 0.0);
        let speed = launch_speed(8.0);
        for target in [
            Vec3::new(0.0, 0.5, -5.0),
            Vec3::new(2.0, 0.0, -6.0),
            Vec3::new(-1.0, 2.0, -3.0),
            Vec3::new(0.0, 0.0, -1.0),
        ] {
            let v = aim_velocity(hand, target, speed).expect("reachable");
            assert!((v.length() - speed).abs() < 1e-3);
            // Its path passes through the target.
            let d = (target - hand).xz().length();
            let t = d / v.xz().length();
            assert!(
                (at(hand, v, t) - target).length() < 1e-2,
                "{target}: {}",
                at(hand, v, t)
            );
            // The low arc: less than 45° up whenever the 45° shot overshoots.
            let up = v.y.atan2(v.xz().length()).to_degrees();
            assert!(up < 45.0 + 1e-3, "{target}: {up}°");
        }
        assert!(aim_velocity(hand, Vec3::new(0.0, 0.0, -30.0), speed).is_none());
        assert!(aim_velocity(hand, Vec3::new(0.0, 12.0, -1.0), speed).is_none());
    }

    #[test]
    fn i051_cone_membership() {
        let (o, d) = (Vec3::ZERO, Vec3::NEG_Z);
        let half = 18f32.to_radians();
        assert!(in_cone(o, d, 2.5, half, Vec3::new(0.0, 0.0, -2.0)));
        assert!(in_cone(o, d, 2.5, half, Vec3::new(0.5, 0.0, -2.0)));
        assert!(
            !in_cone(o, d, 2.5, half, Vec3::new(0.0, 0.0, -2.6)),
            "beyond range"
        );
        assert!(
            !in_cone(o, d, 2.5, half, Vec3::new(0.0, 0.0, 1.0)),
            "behind"
        );
        assert!(
            !in_cone(o, d, 2.5, half, Vec3::new(1.5, 0.0, -1.5)),
            "outside the angle"
        );
        assert!(in_cone(o, d, 2.5, half, o), "at the hand");
    }

    #[test]
    fn i051_path_ends_where_it_meets_the_ground() {
        let p0 = Vec3::new(0.0, 1.45, 0.0);
        let v0 = launch(launch_speed(6.0), 40.0);
        let ground = |a: Vec3, b: Vec3| (b.y <= 0.0).then(|| a.y / (a.y - b.y));
        let (pts, t) = path_hits(p0, v0, 1.0 / 60.0, 5.0, ground);
        let want_t = landing_time(p0, v0, 0.0).unwrap();
        assert!((t - want_t).abs() < 1e-3, "{t} vs {want_t}");
        let last = *pts.last().unwrap();
        assert!(last.y.abs() < 1e-3);
        assert!((last - at(p0, v0, want_t)).length() < 1e-2);
        assert!(pts.len() > 10, "a curve, not a line: {} points", pts.len());
        // Nothing hit: it stops at max_t.
        let (pts, t) = path_hits(p0, v0, 1.0 / 60.0, 0.5, |_, _| None);
        assert!((t - 0.5).abs() < 1e-4 && (pts.last().unwrap() - at(p0, v0, 0.5)).length() < 1e-3);
    }
}
