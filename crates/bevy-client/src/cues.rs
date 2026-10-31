//! Cues that make a cast legible (I-019): how many fires are left, where the
//! nearest one is relative to the view, how long a spell flies, and how
//! strongly each object look glows. No Bevy types, so the unit tests run
//! with plain `rustc`:
//! `rustc --edition 2021 --test src/cues.rs -o /tmp/cues && /tmp/cues`

/// Fires the player still has to put out: the fires the objective still
/// needs, or every fire burning now if that is more. It is 0 exactly when
/// the level is won (`extinguished >= goal` and nothing burns), and never 0
/// while anything burns.
pub fn fires_left(goal: u32, ext: u32, burning: u32) -> u32 {
    goal.saturating_sub(ext).max(burning)
}

/// Signed degrees from the view's forward direction to a point, in the
/// horizontal plane: positive to the right, in (-180, 180]. Forward is
/// `(-sin yaw, -cos yaw)` and right is `(cos yaw, -sin yaw)` (x, z), as in
/// `player.rs`.
pub fn bearing_deg(px: f32, pz: f32, yaw: f32, tx: f32, tz: f32) -> f32 {
    let (dx, dz) = (tx - px, tz - pz);
    let fwd = dx * -yaw.sin() + dz * -yaw.cos();
    let right = dx * yaw.cos() + dz * -yaw.sin();
    let deg = right.atan2(fwd).to_degrees();
    if deg <= -180.0 {
        180.0
    } else {
        deg
    }
}

/// True when a point at `bearing`/`elev` degrees from the view centre is
/// outside the view (with a 5° margin) for the given fields of view
/// (degrees). Any non-finite input gives false.
pub fn off_screen(bearing: f32, elev: f32, hfov: f32, vfov: f32) -> bool {
    if !(bearing.is_finite() && elev.is_finite() && hfov.is_finite() && vfov.is_finite()) {
        return false;
    }
    bearing.abs() > hfov / 2.0 - 5.0 || elev.abs() > vfov / 2.0 - 5.0
}

/// Spell flight speeds (m/s) in `Spell::ALL` order: Fire, Water, Frost, CO2.
const SPEEDS: [f32; 4] = [32.0, 30.0, 120.0, 20.0];
/// Shortest and longest flight, game seconds.
pub const MIN_FLIGHT: f32 = 0.03;
pub const MAX_FLIGHT: f32 = 0.25;

/// Game seconds a spell takes to fly `dist` metres. `spell` is the index in
/// `Spell::ALL` (0..4); an unknown index, or a distance that is not positive
/// and finite, gives the shortest flight.
pub fn flight_secs(spell: u8, dist: f32) -> f32 {
    match SPEEDS.get(spell as usize) {
        Some(&speed) if dist.is_finite() && dist > 0.0 => {
            (dist / speed).clamp(MIN_FLIGHT, MAX_FLIGHT)
        }
        _ => MIN_FLIGHT,
    }
}

/// What an object looks like right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Normal,
    Burning,
    Charred,
    Wet,
    Frosty,
    Foamed,
    Hot,
}

/// Emissive strength of each look's material (0 = no glow). A heating bush
/// (`Hot`, above 120 °C but below its ignition temperature) does not glow:
/// wood that hot gives off no visible light. A fire stays the brightest
/// thing in the courtyard.
pub fn look_emissive(look: Look) -> f32 {
    match look {
        Look::Burning => 1.2,
        Look::Frosty => 0.25,
        Look::Normal | Look::Charred | Look::Wet | Look::Foamed | Look::Hot => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    fn near(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    // 1.
    #[test]
    fn fires_left_counts_what_is_left() {
        assert_eq!(fires_left(3, 0, 3), 3);
        assert_eq!(fires_left(3, 0, 5), 5);
        assert_eq!(fires_left(3, 1, 2), 2);
        assert_eq!(fires_left(3, 2, 0), 1);
        assert_eq!(fires_left(3, 3, 0), 0);
        assert_eq!(fires_left(3, 5, 1), 1);
        for g in 0..=8 {
            for e in 0..=8 {
                for b in 0..=8 {
                    let n = fires_left(g, e, b);
                    if b > 0 {
                        assert!(n > 0, "fires_left({g}, {e}, {b}) = 0 while fires burn");
                    }
                    assert_eq!(n == 0, e >= g && b == 0, "fires_left({g}, {e}, {b}) = {n}");
                }
            }
        }
    }

    // 2.
    #[test]
    fn bearing_matches_the_player_axes() {
        assert!(near(bearing_deg(0.0, 0.0, 0.0, 0.0, -10.0), 0.0, 0.01));
        assert!(near(bearing_deg(0.0, 0.0, 0.0, 10.0, -10.0), 45.0, 0.01));
        assert!(near(bearing_deg(0.0, 0.0, 0.0, -10.0, -10.0), -45.0, 0.01));
        assert!(near(
            bearing_deg(0.0, 0.0, 0.0, 0.0, 10.0).abs(),
            180.0,
            0.01
        ));
        assert!(near(
            bearing_deg(0.0, 0.0, FRAC_PI_2, -10.0, 0.0),
            0.0,
            0.01
        ));
        assert!(near(
            bearing_deg(0.0, 0.0, FRAC_PI_2, 0.0, -10.0),
            90.0,
            0.01
        ));
        // Agreement with player.rs: a point straight along right = (cos yaw,
        // -sin yaw) is at +90 for any yaw.
        for k in 0..12 {
            let yaw = k as f32 * 0.5;
            let (rx, rz) = (yaw.cos(), -yaw.sin());
            let (fx, fz) = (-yaw.sin(), -yaw.cos());
            assert!(near(
                bearing_deg(1.0, 2.0, yaw, 1.0 + 7.0 * rx, 2.0 + 7.0 * rz),
                90.0,
                0.01
            ));
            assert!(near(
                bearing_deg(1.0, 2.0, yaw, 1.0 + 7.0 * fx, 2.0 + 7.0 * fz),
                0.0,
                0.01
            ));
            let b = bearing_deg(1.0, 2.0, yaw, 1.0 - 7.0 * fx, 2.0 - 7.0 * fz);
            assert!(
                b > -180.0 && b <= 180.0 && near(b.abs(), 180.0, 0.01),
                "behind: {b}"
            );
        }
    }

    // 3.
    #[test]
    fn off_screen_uses_both_fovs_with_a_margin() {
        let (h, v) = (90.0, 58.72);
        assert!(!off_screen(0.0, 0.0, h, v));
        assert!(!off_screen(39.0, 0.0, h, v));
        assert!(off_screen(41.0, 0.0, h, v));
        assert!(off_screen(-170.0, 0.0, h, v));
        assert!(off_screen(0.0, 25.0, h, v));
        assert!(!off_screen(0.0, 24.0, h, v));
        let n = f32::NAN;
        assert!(!off_screen(n, 0.0, h, v));
        assert!(!off_screen(0.0, n, h, v));
        assert!(!off_screen(170.0, 0.0, n, v));
        assert!(!off_screen(0.0, 50.0, h, n));
    }

    // 4.
    #[test]
    fn spells_land_within_a_quarter_second() {
        for spell in 0..4u8 {
            let mut prev = 0.0;
            for d in [0.5, 1.0, 18.0, 60.0, 120.0] {
                let t = flight_secs(spell, d);
                assert!((0.03..=0.25).contains(&t), "spell {spell} d {d}: {t}");
                assert!(t >= prev, "spell {spell} d {d}: {t} < {prev}");
                prev = t;
            }
            for d in [f32::NAN, 0.0, -5.0] {
                assert_eq!(flight_secs(spell, d), 0.03, "spell {spell} d {d}");
            }
        }
        // Water at 18 m: 0.6 s before the cap.
        assert!(flight_secs(1, 18.0) <= 0.25);
        assert!(flight_secs(1, 18.0) > 0.03);
        assert_eq!(flight_secs(7, 18.0), 0.03);
    }

    // 5.
    #[test]
    fn hot_does_not_glow_and_fire_keeps_its_glow() {
        assert_eq!(look_emissive(Look::Hot), 0.0);
        assert_eq!(look_emissive(Look::Burning), 1.2);
        for l in [
            Look::Normal,
            Look::Charred,
            Look::Wet,
            Look::Frosty,
            Look::Foamed,
            Look::Hot,
        ] {
            assert!(look_emissive(l) < look_emissive(Look::Burning), "{l:?}");
        }
    }
}

// ---- I-025: streams you cannot miss, an honest arrow, one cast per click.

/// How one spell's stream is drawn (I-025): a constant-width rod `thick`
/// metres across, a core colour and a darker outline shell `outline_scale`
/// times as wide (an inverted hull), both sRGB in 0..1. `mist` is the most
/// cold-fog particles one cast may emit along the stream.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamStyle {
    pub thick: f32,
    pub core: [f32; 3],
    pub outline: [f32; 3],
    pub outline_scale: f32,
    pub mist: u32,
}

/// The stream look of `spell` (index in `Spell::ALL`: Fire, Water, Frost,
/// CO2). Indices 4 and above give the Water style.
pub fn stream_style(spell: u8) -> StreamStyle {
    // Dark outlines (an inverted hull) make every stream read against the
    // pale sky and the green ground alike (relative luminance <= 0.025).
    const NAVY: [f32; 3] = [0.03, 0.06, 0.16];
    const SLATE: [f32; 3] = [0.06, 0.08, 0.11];
    match spell {
        // Fire keeps its glow material and gets only the shared thickness.
        0 => StreamStyle {
            thick: 0.22,
            core: [1.0, 0.45, 0.1],
            outline: [1.0, 0.45, 0.1],
            outline_scale: 1.0,
            mist: 0,
        },
        // A liquid-nitrogen jet in white fog (condensed water vapour).
        2 => StreamStyle {
            thick: 0.22,
            core: [0.86, 0.96, 1.0],
            outline: SLATE,
            outline_scale: 1.3,
            mist: 10,
        },
        // A CO2 discharge: a white cloud of dry-ice snow and vapour.
        3 => StreamStyle {
            thick: 0.22,
            core: [0.95, 0.96, 0.97],
            outline: SLATE,
            outline_scale: 1.3,
            mist: 10,
        },
        // Water: a coherent jet in a clearly blue colour code.
        _ => StreamStyle {
            thick: 0.22,
            core: [0.1, 0.45, 1.0],
            outline: NAVY,
            outline_scale: 1.3,
            mist: 0,
        },
    }
}

/// WCAG 2 relative luminance of an sRGB colour (channels 0..1). Used by the
/// unit tests and for tuning the stream colours, not by the game.
#[cfg_attr(not(test), allow(dead_code))]
pub fn rel_luminance(rgb: [f32; 3]) -> f32 {
    let lin = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2])
}

/// WCAG 2 contrast ratio of two sRGB colours, lighter over darker (1..21).
#[cfg_attr(not(test), allow(dead_code))]
pub fn contrast_ratio(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (la, lb) = (rel_luminance(a), rel_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// True when the aimed object burns, or some burning object's
/// `(bearing, elev)` (degrees from the view centre) is on screen by
/// `off_screen`'s 5° margin. The HUD's arrow hides while this holds.
pub fn fire_in_view(fires: &[(f32, f32)], hfov: f32, vfov: f32, aim_burning: bool) -> bool {
    aim_burning
        || fires
            .iter()
            .any(|&(b, e)| b.is_finite() && e.is_finite() && !off_screen(b, e, hfov, vfov))
}

/// Game seconds a press must stay down before held fire repeats.
pub const HOLD_DELAY: f32 = 0.5;

/// How long (game seconds) the button has been held since the press this
/// frame belongs to: 0 when up, on the first frame down, or on a frame with
/// a new click; otherwise `prev + dt` (a non-finite or negative dt adds 0).
pub fn held_secs(prev: f32, down: bool, was_down: bool, clicked: bool, dt: f32) -> f32 {
    if !down || !was_down || clicked {
        return 0.0;
    }
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    prev + dt
}

/// True once a press has been held long enough to repeat.
pub fn hold_repeats(held: f32) -> bool {
    held >= HOLD_DELAY
}

#[cfg(test)]
mod i025_tests {
    use super::*;

    const SKY: [f32; 3] = [0.55, 0.78, 0.98];
    const GROUND: [f32; 3] = [0.2, 0.4, 0.2];

    fn near(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    // 1.
    #[test]
    fn stream_styles_stand_out_and_tell_the_truth() {
        assert!(near(rel_luminance([1.0, 1.0, 1.0]), 1.0, 1e-4));
        assert_eq!(rel_luminance([0.0, 0.0, 0.0]), 0.0);
        assert!(near(
            contrast_ratio([1.0, 1.0, 1.0], [0.0, 0.0, 0.0]),
            21.0,
            0.01
        ));
        for spell in 0..4u8 {
            let s = stream_style(spell);
            assert!(
                (0.18..=0.25).contains(&s.thick),
                "spell {spell}: thick {}",
                s.thick
            );
        }
        for spell in 1..4u8 {
            let s = stream_style(spell);
            assert!(
                (1.15..=1.35).contains(&s.outline_scale),
                "spell {spell}: outline_scale {}",
                s.outline_scale
            );
            let lo = rel_luminance(s.outline);
            assert!(lo <= 0.025, "spell {spell}: outline L {lo}");
            let c = contrast_ratio(s.outline, SKY);
            assert!(c >= 3.0, "spell {spell}: outline vs sky {c}");
            let c = contrast_ratio(s.outline, s.core);
            assert!(c >= 3.0, "spell {spell}: outline vs core {c}");
            let c = contrast_ratio(s.outline, GROUND);
            assert!(c >= 2.0, "spell {spell}: outline vs ground {c}");
        }
        let w = stream_style(1);
        let [r, _, b] = w.core;
        assert!(b >= 0.8 && b - r >= 0.5, "water core {:?}", w.core);
        let lw = rel_luminance(w.core);
        assert!((0.175..=0.24).contains(&lw), "water core L {lw}");
        let c = contrast_ratio(w.core, SKY);
        assert!(c >= 2.0, "water core vs sky {c}");
        let f = stream_style(2);
        assert!(f.core.iter().all(|&v| v >= 0.75), "frost core {:?}", f.core);
        assert!(f.core[2] >= f.core[0], "frost core {:?}", f.core);
        let co2 = stream_style(3);
        let mx = co2.core.iter().cloned().fold(f32::MIN, f32::max);
        let mn = co2.core.iter().cloned().fold(f32::MAX, f32::min);
        assert!(mn >= 0.85 && mx - mn <= 0.10, "co2 core {:?}", co2.core);
        assert_eq!(stream_style(0).mist, 0);
        assert_eq!(stream_style(1).mist, 0);
        assert!((6..=12).contains(&stream_style(2).mist));
        assert!((6..=12).contains(&stream_style(3).mist));
        assert_eq!(stream_style(9), stream_style(1));
    }

    /// Steps frames of `dt` over a press from `t_down` to `t_up` (sampled at
    /// each frame start; `clicked` on the first frame that sees it) and
    /// counts the frames where `hold_repeats` holds while the button is down.
    fn repeat_frames(dt: f32, t_down: f32, t_up: f32) -> u32 {
        let (mut held, mut was_down, mut n) = (0.0, false, 0);
        let mut seen = false;
        for k in 0..400 {
            let t = k as f32 * dt;
            let down = t >= t_down && t < t_up;
            let clicked = down && !seen;
            if down {
                seen = true;
            }
            held = held_secs(held, down, was_down, clicked, dt);
            was_down = down;
            if down && hold_repeats(held) {
                n += 1;
            }
        }
        n
    }

    // 2.
    #[test]
    fn a_click_casts_once_and_a_hold_repeats() {
        assert_eq!(HOLD_DELAY, 0.5);
        let dt = 1.0 / 60.0;
        let t0 = 10.0 * dt + 0.001;
        assert_eq!(repeat_frames(dt, t0, t0 + 0.30), 0);
        assert_eq!(repeat_frames(dt, t0, t0 + 0.45), 0);
        assert!(repeat_frames(dt, t0, t0 + 0.60) >= 1);
        // 4 fps: samples at 0, 0.25, 0.5, ...
        let dt = 0.25;
        assert_eq!(
            repeat_frames(dt, 0.9, 1.4),
            0,
            "seen on 2 samples (1.0, 1.25)"
        );
        assert!(repeat_frames(dt, 0.9, 1.6) >= 1, "seen on 3 samples");
        // Resets.
        assert_eq!(held_secs(0.4, true, true, true, 0.1), 0.0);
        assert_eq!(held_secs(0.4, false, true, false, 0.1), 0.0);
        assert_eq!(held_secs(0.4, true, false, false, 0.1), 0.0);
        assert!(near(held_secs(0.4, true, true, false, 0.1), 0.5, 1e-6));
        for bad in [f32::NAN, -1.0, f32::INFINITY] {
            assert_eq!(held_secs(0.3, true, true, false, bad), 0.3, "dt {bad}");
        }
        assert!(!hold_repeats(0.49));
        assert!(hold_repeats(0.5));
    }

    // 3.
    #[test]
    fn fire_in_view_sees_any_fire_on_screen() {
        let (h, v) = (90.0, 58.72);
        assert!(!fire_in_view(&[], h, v, false));
        assert!(fire_in_view(&[], h, v, true));
        assert!(fire_in_view(&[(0.0, 0.0)], h, v, false));
        assert!(fire_in_view(&[(39.0, 0.0)], h, v, false));
        assert!(!fire_in_view(&[(41.0, 0.0)], h, v, false));
        assert!(fire_in_view(&[(41.0, 0.0), (-10.0, 3.0)], h, v, false));
        assert!(!fire_in_view(&[(f32::NAN, 0.0)], h, v, false));
    }
}
