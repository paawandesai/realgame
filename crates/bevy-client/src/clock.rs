//! The game clock: how much game time one frame may advance, and how a
//! long frame is split into short physics steps. No Bevy types, so the unit
//! tests run with plain `rustc`:
//! `rustc --edition 2021 --test src/clock.rs -o /tmp/clock && /tmp/clock`
//!
//! Every per-frame system takes its dt from `frame_dt`, and `main()` gives
//! Bevy's virtual clock (which drives the fixed-step chemistry) the same
//! ceiling, so the player, the spells and the fire all keep wall-clock
//! speed down to 4 fps. Below that the whole game slows evenly.

/// The most game time (seconds) one frame may advance.
pub const MAX_FRAME_DT: f32 = 0.25;
/// The longest single player-physics step (seconds).
pub const MAX_STEP: f32 = 1.0 / 60.0;
/// A guard on the number of substeps in one frame.
const MAX_SUBSTEPS: u32 = 64;

/// This frame's game dt: `raw` clamped to `[0, MAX_FRAME_DT]`. NaN,
/// infinities and negative values give 0.
pub fn frame_dt(raw: f32) -> f32 {
    if raw.is_finite() && raw > 0.0 {
        raw.min(MAX_FRAME_DT)
    } else {
        0.0
    }
}

/// Splits `dt` into `n` equal steps of `h <= max_step`, so `n * h == dt`.
/// dt 0 (or anything not positive and finite) gives `(0, 0.0)`; n is capped
/// at 64.
pub fn substeps(dt: f32, max_step: f32) -> (u32, f32) {
    if !(dt.is_finite() && dt > 0.0 && max_step > 0.0) {
        return (0, 0.0);
    }
    let n = ((dt / max_step).ceil() as u32).clamp(1, MAX_SUBSTEPS);
    (n, dt / n as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_dt_clamps() {
        assert_eq!(frame_dt(0.016), 0.016);
        assert_eq!(frame_dt(0.1), 0.1);
        assert_eq!(frame_dt(0.25), 0.25);
        assert_eq!(frame_dt(1.5), 0.25);
        assert_eq!(frame_dt(0.0), 0.0);
        assert_eq!(frame_dt(-0.1), 0.0);
        assert_eq!(frame_dt(f32::NAN), 0.0);
        assert_eq!(frame_dt(f32::INFINITY), 0.0);
        assert_eq!(frame_dt(f32::NEG_INFINITY), 0.0);
    }

    #[test]
    fn substeps_split_evenly() {
        let (n, h) = substeps(0.25, 1.0 / 60.0);
        assert_eq!(n, 15);
        assert!((n as f32 * h - 0.25).abs() < 1e-6, "n*h = {}", n as f32 * h);
        assert!(h <= 1.0 / 60.0 + 1e-7, "h = {h}");
        assert_eq!(substeps(0.01, 1.0 / 60.0), (1, 0.01));
        assert_eq!(substeps(0.0, 1.0 / 60.0), (0, 0.0));
        assert_eq!(substeps(1e9, 1.0 / 60.0).0, 64);
        assert_eq!(MAX_FRAME_DT, 0.25);
        assert_eq!(MAX_STEP, 1.0 / 60.0);
    }

    #[test]
    fn one_substep_at_or_below_max_step() {
        // At 60 fps and above the loop runs once with the frame's own dt.
        for dt in [1.0 / 60.0, 1.0 / 120.0, 0.001] {
            assert_eq!(substeps(dt, MAX_STEP), (1, dt));
        }
    }
}
