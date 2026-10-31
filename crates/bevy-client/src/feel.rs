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

/// Vertical field of view (degrees) for a horizontal one at 16:9. The game
/// uses `vfov_for` with the live aspect; this 16:9 form is kept for the tests.
#[allow(dead_code)]
pub fn vfov_from_h(h_deg: f32) -> f32 {
    vfov_for(h_deg, 16.0 / 9.0)
}

/// Vertical field of view (degrees) that gives `h_deg` horizontally at
/// this aspect (width / height). A non-finite or non-positive aspect
/// counts as 16:9; the aspect is clamped to [1.0, 4.0], so in a portrait
/// window the view is as tall as it is wide and never stretches upward.
pub fn vfov_for(h_deg: f32, aspect: f32) -> f32 {
    let a = if aspect.is_finite() && aspect > 0.0 {
        aspect.clamp(1.0, 4.0)
    } else {
        16.0 / 9.0
    };
    (2.0 * ((h_deg.to_radians() * 0.5).tan() / a).atan()).to_degrees()
}

/// The cone for point-and-click taps and hover: 1.5x the aim-assist cone,
/// at least 4.5 deg, when Aim help is on; 0 when it is off.
pub fn tap_cone(assist: f32) -> f32 {
    if assist.is_finite() && assist > 0.0 {
        assist.max(3f32.to_radians()) * 1.5
    } else {
        0.0
    }
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

/// The spell gem's angle below the view centre at the default look, degrees
/// (16:9 window, View width 90). See `VM_REST` and `vm_rest` for the geometry.
pub const VM_GEM_DEG: f32 = 17.6;
/// The gem keeps at least this far inside the bottom edge, degrees. It covers
/// the gem's radius (about 1.6 deg), the walk bob (under 1 deg) and the glow.
pub const VM_EDGE_MARGIN_DEG: f32 = 5.0;
/// Guard only: never lift the gem closer to the crosshair than this, degrees.
/// Supported settings (aspect clamped to 4, View width >= 70) never reach it.
pub const VM_GEM_MIN_DEG: f32 = 4.0;
/// The viewmodel root's camera-space rest position at the default view,
/// metres (before bob and recoil). The spell gem is its child at local
/// (0, 0.05, -0.11) under the root's scale 0.55, so it sits at about
/// (0.21, -0.1525, -0.4805): atan(0.1525 / 0.4805) = 17.61 deg below centre,
/// which `VM_GEM_DEG` rounds to 17.6.
pub const VM_REST: [f32; 3] = [0.21, -0.18, -0.42];

/// How far below the view centre the spell gem sits, in degrees, for a
/// vertical field of view of `vfov_deg`: today's 17.6 deg, lifted when the
/// view is so short that the gem would come within 5 deg of the bottom edge,
/// and never above 4 deg. A non-finite or non-positive input gives 17.6.
pub fn vm_gem_deg(vfov_deg: f32) -> f32 {
    if !vfov_deg.is_finite() || vfov_deg <= 0.0 {
        return VM_GEM_DEG;
    }
    VM_GEM_DEG
        .min(vfov_deg / 2.0 - VM_EDGE_MARGIN_DEG)
        .max(VM_GEM_MIN_DEG)
}

/// How far the whole viewmodel is pitched up about the eye, in degrees, so
/// the gem sits `vm_gem_deg(vfov_deg)` below centre: 0 at the default view.
pub fn vm_tilt_deg(vfov_deg: f32) -> f32 {
    VM_GEM_DEG - vm_gem_deg(vfov_deg)
}

/// The viewmodel's rest pose for a vertical fov: the root's camera-space
/// translation and the extra pitch (radians, about camera x) to put before
/// its own rotation. The pose is `VM_REST` and the root's rotation orbited
/// rigidly about the eye by `vm_tilt_deg`. A rigid turn about the eye only
/// moves the gauntlet's image on screen; it does not change which part hides
/// which, so the gem stays clear of the wrist ring as at the default view
/// (a y-only lift would push the gem behind the ring, I-044 break 1). At the
/// default view the translation is exactly `VM_REST` and the pitch 0.
pub fn vm_rest(vfov_deg: f32) -> ([f32; 3], f32) {
    let a = vm_tilt_deg(vfov_deg).to_radians();
    let (s, c) = a.sin_cos();
    let [x, y, z] = VM_REST;
    ([x, y * c - z * s, y * s + z * c], a)
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

    #[test]
    fn tap_cone_follows_aim_help() {
        let d = |x: f32| x.to_radians();
        assert!(near(tap_cone(d(4.0)), d(6.0), 1e-6), "{}", tap_cone(d(4.0)));
        assert!(near(tap_cone(d(1.0)), d(4.5), 1e-6), "{}", tap_cone(d(1.0)));
        assert_eq!(tap_cone(0.0), 0.0);
        assert_eq!(tap_cone(-0.1), 0.0);
        assert_eq!(tap_cone(f32::NAN), 0.0);
        // Never narrower than the 4 deg locked cone when Aim help is on.
        assert!(tap_cone(d(4.0)) >= d(4.0));
    }

    #[test]
    fn view_width_holds_at_any_aspect() {
        let w = 16.0 / 9.0;
        let v = |h: f32, a: f32| vfov_for(h, a);
        assert!(near(v(90.0, w), 58.72, 0.05), "{}", v(90.0, w));
        assert_eq!(v(90.0, w), vfov_from_h(90.0));
        assert!(
            near(v(90.0, 4.0 / 3.0), 73.74, 0.05),
            "{}",
            v(90.0, 4.0 / 3.0)
        );
        assert!(
            near(v(90.0, 21.0 / 9.0), 46.40, 0.05),
            "{}",
            v(90.0, 21.0 / 9.0)
        );
        assert!(near(v(70.0, w), 43.0, 0.1), "{}", v(70.0, w));
        assert!(near(v(110.0, 1.0), 110.0, 0.05), "{}", v(110.0, 1.0));
        assert_eq!(v(90.0, 0.5), v(90.0, 1.0));
        assert_eq!(v(90.0, 9.0), v(90.0, 4.0));
        assert_eq!(v(90.0, f32::NAN), v(90.0, w));
        assert_eq!(v(90.0, 0.0), v(90.0, w));
        assert_eq!(v(90.0, -1.0), v(90.0, w));
    }

    #[test]
    fn ray_and_projection_agree_at_any_aspect() {
        for a in [4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0] {
            let e = ray_dir(1.0, 0.5, vfov_for(90.0, a), a).unwrap();
            let ang = (-e[2]).acos().to_degrees();
            assert!(near(ang, 45.0, 0.01), "aspect {a}: {ang}");
        }
    }

    // I-044: the viewmodel's spell gem stays on screen at any supported view.

    #[test]
    fn vm_gem_matches_today_at_the_default_view() {
        assert!(near(vm_gem_deg(58.72), 17.6, 1e-4), "{}", vm_gem_deg(58.72));
        // Stricter than the spec's -0.18 +- 1e-3: the default pose is exact.
        let (p, a) = vm_rest(58.72);
        assert_eq!(a, 0.0);
        assert_eq!(p, VM_REST);
        assert_eq!(vm_tilt_deg(58.72), 0.0);
    }

    #[test]
    fn vm_gem_keeps_the_edge_margin_at_every_view() {
        for h in 70..=110 {
            for a in [4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0] {
                let v = vfov_for(h as f32, a);
                let g = vm_gem_deg(v);
                if v / 2.0 - 5.0 >= 4.0 {
                    assert!(g <= v / 2.0 - 5.0 + 1e-4, "h {h} a {a}: {g} vs {v}");
                }
                assert!(g >= 4.0, "h {h} a {a}: {g}");
            }
        }
        let g = vm_gem_deg(vfov_for(70.0, 21.0 / 9.0));
        assert!(g <= 11.75, "{g}");
    }

    #[test]
    fn vm_gem_rises_with_fov_and_holds_its_floor() {
        let mut prev = f32::NEG_INFINITY;
        let mut v = 10.0_f32;
        while v <= 120.0 {
            let g = vm_gem_deg(v);
            assert!(g >= prev, "vfov {v}: {g} < {prev}");
            prev = g;
            v += 0.5;
        }
        assert!(near(vm_gem_deg(10.0), 4.0, 1e-6), "{}", vm_gem_deg(10.0));
        assert!(near(vm_gem_deg(16.0), 4.0, 1e-6), "{}", vm_gem_deg(16.0));
        assert_eq!(vm_gem_deg(f32::NAN), 17.6);
        assert_eq!(vm_gem_deg(-1.0), 17.6);
        assert_eq!(vm_gem_deg(f32::INFINITY), 17.6);
    }

    /// The gem's camera-space point for a rest pose: the root plus
    /// Rx(pitch) of its local offset (0, 0.05, -0.11) * 0.55.
    fn gem_at(v: f32) -> [f32; 3] {
        let (p, a) = vm_rest(v);
        let (s, c) = a.sin_cos();
        let (gy, gz) = (0.05 * 0.55, -0.11 * 0.55);
        [p[0], p[1] + gy * c - gz * s, p[2] + gy * s + gz * c]
    }

    #[test]
    fn vm_rest_puts_the_gem_at_its_angle() {
        for v in [30.0, 33.4, 46.4, 58.72, 90.0] {
            let g = gem_at(v);
            let ang = (-g[1] / -g[2]).atan().to_degrees();
            assert!(near(ang, vm_gem_deg(v), 0.01), "vfov {v}: {ang}");
        }
    }

    #[test]
    fn vm_rest_is_a_rigid_turn_about_the_eye() {
        // Distance from the eye and the angle between root and gem as seen
        // from the eye are kept, so the gauntlet's image is only moved.
        let len = |p: [f32; 3]| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        let r0 = len(VM_REST);
        let g0 = gem_at(58.72);
        for v in [30.0, 33.4, 36.4, 43.0, 58.72] {
            let (p, a) = vm_rest(v);
            assert!(near(len(p), r0, 1e-5), "vfov {v}: {}", len(p));
            let g = gem_at(v);
            assert!(near(len(g), len(g0), 1e-5), "vfov {v}");
            let d = |u: [f32; 3], w: [f32; 3]| {
                ((u[0] - w[0]).powi(2) + (u[1] - w[1]).powi(2) + (u[2] - w[2]).powi(2)).sqrt()
            };
            assert!(near(d(p, g), d(VM_REST, g0), 1e-5), "vfov {v}");
            assert!(near(a.to_degrees(), vm_tilt_deg(v), 1e-4), "vfov {v}");
            assert!(a >= 0.0, "vfov {v}: tilt {a}");
        }
        // 21:9 at View width 70: lifted by about 5.9 deg.
        assert!(near(vm_tilt_deg(vfov_for(70.0, 21.0 / 9.0)), 5.9, 0.05));
    }
}
