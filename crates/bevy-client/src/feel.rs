//! Feel and comfort maths for the first-person view: field of view, look
//! speed, the aim-assist falloff and the cursor ray. No Bevy types, so the
//! unit tests run with plain `rustc`:
//! `rustc --edition 2021 --test src/feel.rs -o /tmp/feel && /tmp/feel`
//!
//! Angles are in degrees unless a name says otherwise; `assist_cone` works in
//! whatever unit its `base` is given in.

/// Radians of view turn per CSS pixel of mouse movement at sensitivity 1:
/// about 21 cm per full turn at 800 CPI (gameplay-standards GS-8).
pub const LOOK_PER_PX: f32 = 0.00095;

/// Default horizontal field of view at 16:9 (GS-1).
pub const DEFAULT_HFOV: f32 = 90.0;
/// The settings slider's range for the horizontal field of view (GS-2).
pub const HFOV_MIN: f32 = 70.0;
pub const HFOV_MAX: f32 = 110.0;
/// The settings slider's range for the look-speed multiplier.
pub const SENS_MIN: f32 = 0.25;
pub const SENS_MAX: f32 = 3.0;
/// Aim assist is full strength up to this distance (metres)...
pub const ASSIST_FULL_M: f32 = 20.0;
/// ...and falls linearly to nothing at this one.
pub const ASSIST_ZERO_M: f32 = 60.0;

/// Vertical field of view (degrees) for a horizontal one at 16:9.
pub fn vfov_from_h(h_deg: f32) -> f32 {
    (2.0 * ((h_deg.to_radians() * 0.5).tan() * 9.0 / 16.0).atan()).to_degrees()
}

/// A horizontal field of view from a setting or command: clamped to the
/// slider's range, or `None` when it is not a number.
pub fn clamp_hfov(h_deg: f32) -> Option<f32> {
    h_deg.is_finite().then(|| h_deg.clamp(HFOV_MIN, HFOV_MAX))
}

/// A look-speed multiplier: clamped to the slider's range, or `None` when it
/// is not a positive number.
pub fn clamp_sens(s: f32) -> Option<f32> {
    (s.is_finite() && s > 0.0).then(|| s.clamp(SENS_MIN, SENS_MAX))
}

/// Mouse travel (cm) for a full turn, at `cpi` counts per inch with one
/// count per CSS pixel.
pub fn cm_per_360(look_per_px: f32, sens: f32, cpi: f32) -> f32 {
    std::f32::consts::TAU / (look_per_px * sens) / (cpi / 2.54)
}

/// The aim-assist cone at a distance: all of `base` up to
/// [`ASSIST_FULL_M`], falling linearly to 0 at [`ASSIST_ZERO_M`] and beyond.
pub fn assist_cone(base: f32, dist: f32) -> f32 {
    if !dist.is_finite() || dist >= ASSIST_ZERO_M {
        0.0
    } else if dist <= ASSIST_FULL_M {
        base
    } else {
        base * (ASSIST_ZERO_M - dist) / (ASSIST_ZERO_M - ASSIST_FULL_M)
    }
}

/// The camera-space unit ray (x right, y up, looking down -z) through a
/// point on the screen given as fractions of its width and height (0..1,
/// from the top left), for a vertical field of view and aspect ratio. The
/// same maths as the perspective projection, so a click lands where the
/// cursor is at any field of view. `None` off the screen.
pub fn ray_dir(fx: f32, fy: f32, fov_y_deg: f32, aspect: f32) -> Option<[f32; 3]> {
    if !(0.0..=1.0).contains(&fx) || !(0.0..=1.0).contains(&fy) {
        return None;
    }
    let t = (fov_y_deg.to_radians() * 0.5).tan();
    let v = [(fx * 2.0 - 1.0) * t * aspect, (1.0 - fy * 2.0) * t, -1.0];
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    Some([v[0] / len, v[1] / len, v[2] / len])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn fov_conversion_and_clamp() {
        assert!(
            near(vfov_from_h(90.0), 58.72, 0.05),
            "{}",
            vfov_from_h(90.0)
        );
        assert!(near(vfov_from_h(70.0), 43.0, 0.1), "{}", vfov_from_h(70.0));
        assert!(
            near(vfov_from_h(110.0), 77.6, 0.1),
            "{}",
            vfov_from_h(110.0)
        );
        assert_eq!(clamp_hfov(f32::NAN), None);
        assert_eq!(clamp_hfov(f32::INFINITY), None);
        assert_eq!(clamp_hfov(f32::NEG_INFINITY), None);
        assert_eq!(clamp_hfov(50.0), Some(70.0));
        assert_eq!(clamp_hfov(130.0), Some(110.0));
        assert_eq!(clamp_hfov(95.0), Some(95.0));
        assert_eq!(DEFAULT_HFOV, 90.0);
    }

    #[test]
    fn look_speed_is_comfortable() {
        let cm = cm_per_360(LOOK_PER_PX, 1.0, 800.0);
        assert!((20.0..=80.0).contains(&cm), "{cm} cm/360");
        assert_eq!(clamp_sens(f32::NAN), None);
        assert_eq!(clamp_sens(f32::INFINITY), None);
        assert_eq!(clamp_sens(f32::NEG_INFINITY), None);
        assert_eq!(clamp_sens(0.0), None);
        assert_eq!(clamp_sens(-1.0), None);
        assert_eq!(clamp_sens(5.0), Some(3.0));
        assert_eq!(clamp_sens(0.1), Some(0.25));
        assert_eq!(clamp_sens(1.5), Some(1.5));
    }

    #[test]
    fn assist_falls_off_with_distance() {
        let base = 4f32.to_radians();
        assert_eq!(assist_cone(base, 5.0), base);
        assert_eq!(assist_cone(base, 20.0), base);
        assert!(near(assist_cone(base, 40.0).to_degrees(), 2.0, 0.01));
        assert_eq!(assist_cone(base, 60.0), 0.0);
        assert_eq!(assist_cone(base, 80.0), 0.0);
    }

    #[test]
    fn cursor_ray_matches_the_projection() {
        let v = vfov_from_h(90.0);
        let c = ray_dir(0.5, 0.5, v, 16.0 / 9.0).unwrap();
        assert!(near(c[0], 0.0, 1e-6) && near(c[1], 0.0, 1e-6) && near(c[2], -1.0, 1e-6));
        let e = ray_dir(1.0, 0.5, v, 16.0 / 9.0).unwrap();
        let len = (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt();
        assert!(near(len, 1.0, 1e-5));
        let ang = (-e[2]).acos().to_degrees();
        assert!(near(ang, 45.0, 0.01), "{ang}");
        assert_eq!(ray_dir(-0.1, 0.5, v, 1.0), None);
        assert_eq!(ray_dir(0.5, 1.1, v, 1.0), None);
        assert_eq!(ray_dir(f32::NAN, 0.5, v, 1.0), None);
    }
}
