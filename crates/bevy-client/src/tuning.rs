//! The block world's feel numbers, in one place (Paawan's tuning,
//! 2026-11-01: "increase the side to side movement speed for looking, and
//! make the spells shoot out faster and further, same mechanics just
//! faster"). Voxel mode only: the classic courtyard keeps `feel` and
//! `ballistics` as they were.

/// Side-to-side look speed over the comfort default (`feel::LOOK_PER_PX`).
/// Applies to the mouse (locked or drag-to-look) and the `turn` command;
/// up-down look is unchanged, and `?sens` still scales both.
pub const YAW_SCALE: f32 = 1.6;

/// Water's reach on flat ground (m) for a tap and for a full charge. The
/// launch speed is the one that reaches that far at 45 degrees
/// (`ballistics::launch_speed`, v = sqrt(g R)), so a longer reach is also a
/// visibly faster jet; the flight then follows real gravity.
pub const WATER_REACH: (f32, f32) = (10.0, 25.0);
/// CO2 is a heavier, shorter lob.
pub const CO2_REACH: (f32, f32) = (8.0, 14.0);
/// Seconds of holding for the full charge.
pub const FULL_HOLD_S: f32 = 1.0;

/// Fire's and Frost's sprays: range (m) and cone half-angle (degrees).
pub const FIRE_CONE: (f32, f32) = (3.5, 18.0);
pub const FROST_CONE: (f32, f32) = (4.0, 20.0);
/// How fast a spray reaches its end (m/s).
pub const CONE_SPEED: f32 = 32.0;

/// The reach after holding `held` seconds, from `lo` (a tap) to `hi`.
pub fn reach(lo: f32, hi: f32, held: f32) -> f32 {
    let f = if held.is_finite() {
        (held / FULL_HOLD_S).clamp(0.0, 1.0)
    } else {
        0.0
    };
    lo + (hi - lo) * f
}

/// How spell `spell` (0 Fire, 1 Water, 2 Frost, 3 CO2) flies after a hold
/// of `held` seconds.
pub fn motion(spell: u8, held: f32) -> crate::ballistics::Motion {
    use crate::ballistics::Motion;
    match spell {
        1 => Motion::Arc {
            reach: reach(WATER_REACH.0, WATER_REACH.1, held),
        },
        3 => Motion::Arc {
            reach: reach(CO2_REACH.0, CO2_REACH.1, held),
        },
        0 => Motion::Cone {
            range: FIRE_CONE.0,
            half: FIRE_CONE.1.to_radians(),
        },
        _ => Motion::Cone {
            range: FROST_CONE.0,
            half: FROST_CONE.1.to_radians(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ballistics::Motion;

    #[test]
    fn a_tap_reaches_10_m_and_a_1_s_hold_25_m() {
        assert_eq!(motion(1, 0.0), Motion::Arc { reach: 10.0 });
        assert_eq!(motion(1, 0.5), Motion::Arc { reach: 17.5 });
        assert_eq!(motion(1, 1.0), Motion::Arc { reach: 25.0 });
        assert_eq!(motion(1, 3.0), Motion::Arc { reach: 25.0 });
        assert_eq!(motion(3, 1.0), Motion::Arc { reach: 14.0 });
        assert!(matches!(motion(0, 0.0), Motion::Cone { range, .. } if range == 3.5));
        assert!(matches!(motion(2, 0.0), Motion::Cone { range, .. } if range == 4.0));
    }

    #[test]
    fn the_jet_is_faster_than_the_old_one() {
        // I-051's tap launched at sqrt(g * 6.5) = 8.0 m/s; now 9.9 m/s, and
        // a full charge 15.7 m/s (was 13.1).
        let v = crate::ballistics::launch_speed(10.0);
        assert!((v - 9.9).abs() < 0.05, "{v}");
        assert!(crate::ballistics::launch_speed(25.0) > 15.6);
    }
}
