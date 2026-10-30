//! The player's movement integrator on plain data: acceleration, jump,
//! gravity and axis-by-axis push-out against axis-aligned boxes. No Bevy
//! types, so the unit tests run with plain `rustc`:
//! `rustc --edition 2021 --test src/motion.rs -o /tmp/motion && /tmp/motion`
//!
//! `advance` splits a frame's dt into steps of at most `clock::MAX_STEP`
//! (1/60 s), so a long frame (up to `clock::MAX_FRAME_DT`, 0.25 s) cannot
//! carry the body through a thin wall. At 60 fps and above it runs `step`
//! exactly once, so movement there is bit-for-bit what one step gives.

// Under `rustc --test src/motion.rs` this file is the crate root, so it
// brings in the clock itself; inside the game it uses the crate's.
#[cfg(not(test))]
use crate::clock;
#[cfg(test)]
#[allow(dead_code)]
#[path = "clock.rs"]
mod clock;

/// Body half-width (metres).
pub const RADIUS: f32 = 0.35;
/// Body height (metres).
pub const HEIGHT: f32 = 1.75;
/// Take-off speed of a jump (m/s).
pub const JUMP: f32 = 6.0;
/// Gravity (m/s²).
pub const GRAVITY: f32 = 18.0;
/// Horizontal acceleration (m/s²): snappy on the ground, slippery on ice, a
/// little air control.
pub const ACCEL_GROUND: f32 = 60.0;
pub const ACCEL_ICE: f32 = 4.0;
pub const ACCEL_AIR: f32 = 12.0;

/// An axis-aligned box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box3 {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Box3 {
    pub fn overlaps(&self, o: &Box3) -> bool {
        (0..3).all(|k| self.min[k] < o.max[k] && self.max[k] > o.min[k])
    }
}

/// The moving body: feet position, velocity and contact state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub grounded: bool,
    pub on_ice: bool,
}

impl Body {
    fn aabb(&self) -> Box3 {
        let p = self.pos;
        Box3 {
            min: [p[0] - RADIUS, p[1], p[2] - RADIUS],
            max: [p[0] + RADIUS, p[1] + HEIGHT, p[2] + RADIUS],
        }
    }
}

/// One explicit movement step of `dt` seconds toward the horizontal
/// velocity `target_h` (x, z), against `solids`. `ice` boxes under the feet
/// make the ground slippery. The body's centre is kept within `±bound` on x
/// and z (the courtyard's edge).
pub fn step(
    body: &mut Body,
    target_h: [f32; 2],
    jump: bool,
    dt: f32,
    solids: &[Box3],
    ice: &[Box3],
    bound: f32,
) {
    let p = body.pos;
    let feet = Box3 {
        min: [p[0] - 0.1, p[1] - 0.3, p[2] - 0.1],
        max: [p[0] + 0.1, p[1] + 0.3, p[2] + 0.1],
    };
    body.on_ice = body.grounded && ice.iter().any(|a| a.overlaps(&feet));
    let accel = match (body.grounded, body.on_ice) {
        (true, false) => ACCEL_GROUND,
        (true, true) => ACCEL_ICE,
        (false, _) => ACCEL_AIR,
    };
    // Move the horizontal velocity toward the target by at most accel * dt.
    let (mut hx, mut hz) = (body.vel[0], body.vel[2]);
    let (mut dx, mut dz) = (target_h[0] - hx, target_h[1] - hz);
    let max = accel * dt;
    let len_sq = dx * dx + dz * dz;
    if len_sq > max * max {
        let len = len_sq.sqrt();
        dx = max * (dx / len);
        dz = max * (dz / len);
    }
    hx += dx;
    hz += dz;
    body.vel[0] = hx;
    body.vel[2] = hz;
    if jump && body.grounded {
        body.vel[1] = JUMP;
        body.grounded = false;
    }
    body.vel[1] -= GRAVITY * dt;

    // Move one axis at a time, pushing out of any box we end up inside.
    body.grounded = false;
    for axis in [0usize, 2, 1] {
        let d = body.vel[axis] * dt;
        if d == 0.0 {
            continue;
        }
        body.pos[axis] += d;
        let mut b = body.aabb();
        for s in solids {
            if !b.overlaps(s) {
                continue;
            }
            let (lo, hi) = match axis {
                0 => (s.min[0] - RADIUS, s.max[0] + RADIUS),
                2 => (s.min[2] - RADIUS, s.max[2] + RADIUS),
                _ => (s.min[1] - HEIGHT, s.max[1]),
            };
            if d > 0.0 {
                body.pos[axis] = lo - 1e-4;
            } else {
                body.pos[axis] = hi + 1e-4;
                if axis == 1 {
                    body.grounded = true;
                }
            }
            body.vel[axis] = 0.0;
            b = body.aabb();
        }
    }
    if body.pos[1] <= 0.0 {
        body.pos[1] = 0.0;
        body.vel[1] = 0.0;
        body.grounded = true;
    }
    let lim = bound - RADIUS;
    body.pos[0] = body.pos[0].clamp(-lim, lim);
    body.pos[2] = body.pos[2].clamp(-lim, lim);
}

/// One frame of movement: `dt` split into `n` equal steps of at most
/// `clock::MAX_STEP`, with the jump request on the first step only.
pub fn advance(
    body: &mut Body,
    target_h: [f32; 2],
    jump: bool,
    dt: f32,
    solids: &[Box3],
    ice: &[Box3],
    bound: f32,
) {
    let (n, h) = clock::substeps(dt, clock::MAX_STEP);
    for k in 0..n {
        step(body, target_h, jump && k == 0, h, solids, ice, bound);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUND: f32 = 1000.0;

    fn floor() -> Box3 {
        Box3 {
            min: [-100.0, -1.0, -100.0],
            max: [100.0, 0.0, 100.0],
        }
    }

    fn at_rest() -> Body {
        Body {
            pos: [0.0, 0.0, 0.0],
            vel: [0.0, 0.0, 0.0],
            grounded: true,
            on_ice: false,
        }
    }

    fn near(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() <= tol)
    }

    #[test]
    fn one_long_frame_matches_many_short_ones() {
        let solids = [floor()];
        let mut one = at_rest();
        advance(&mut one, [5.0, 0.0], false, 0.25, &solids, &[], BOUND);
        let mut many = at_rest();
        for _ in 0..15 {
            advance(
                &mut many,
                [5.0, 0.0],
                false,
                1.0 / 60.0,
                &solids,
                &[],
                BOUND,
            );
        }
        assert!(
            near(one.pos, many.pos, 1e-3),
            "{:?} vs {:?}",
            one.pos,
            many.pos
        );
        assert!(
            near(one.vel, many.vel, 1e-3),
            "{:?} vs {:?}",
            one.vel,
            many.vel
        );
        // It really walked: about 5 m/s after the 60 m/s² ramp.
        assert!(one.pos[0] > 1.0 && one.pos[0] < 1.25, "x = {}", one.pos[0]);
    }

    #[test]
    fn substeps_stop_at_a_thin_wall_and_one_step_tunnels() {
        // A 0.1 m pane with its near face at x = 1.0.
        let wall = Box3 {
            min: [1.0, 0.0, -5.0],
            max: [1.1, 3.0, 5.0],
        };
        let solids = [floor(), wall];
        let sprinting = Body {
            vel: [8.5, 0.0, 0.0],
            ..at_rest()
        };
        let mut b = sprinting;
        advance(&mut b, [8.5, 0.0], false, 0.25, &solids, &[], BOUND);
        assert!(b.pos[0] <= 1.0 - RADIUS + 1e-4, "x = {}", b.pos[0]);
        // Without substeps one 0.25 s step moves 2.125 m and ends past the pane.
        let mut t = sprinting;
        step(&mut t, [8.5, 0.0], false, 0.25, &solids, &[], BOUND);
        assert!(t.pos[0] - RADIUS > 1.1, "x = {}", t.pos[0]);
    }

    #[test]
    fn at_or_below_max_step_advance_is_one_step() {
        let solids = [floor()];
        for dt in [1.0 / 60.0, 1.0 / 144.0, 0.004] {
            for jump in [false, true] {
                let mut a = Body {
                    vel: [1.0, 0.0, -2.0],
                    ..at_rest()
                };
                let mut s = a;
                for _ in 0..30 {
                    advance(&mut a, [3.0, 4.0], jump, dt, &solids, &[], BOUND);
                    step(&mut s, [3.0, 4.0], jump, dt, &solids, &[], BOUND);
                }
                assert_eq!(a, s, "dt {dt} jump {jump}");
            }
        }
    }

    #[test]
    fn ice_is_slippery_and_jump_needs_ground() {
        let ice = [Box3 {
            min: [-5.0, -0.1, -5.0],
            max: [5.0, 0.05, 5.0],
        }];
        let mut b = at_rest();
        step(&mut b, [5.0, 0.0], false, 0.1, &[floor()], &ice, BOUND);
        assert!(b.on_ice);
        assert!(
            (b.vel[0] - ACCEL_ICE * 0.1).abs() < 1e-5,
            "vx = {}",
            b.vel[0]
        );

        let mut j = at_rest();
        advance(&mut j, [0.0, 0.0], true, 0.1, &[floor()], &[], BOUND);
        assert!(j.pos[1] > 0.4 && !j.grounded, "y = {}", j.pos[1]);
        // In the air a second jump does nothing.
        let vy = j.vel[1];
        step(&mut j, [0.0, 0.0], true, 0.01, &[floor()], &[], BOUND);
        assert!((j.vel[1] - (vy - GRAVITY * 0.01)).abs() < 1e-5);
    }

    #[test]
    fn courtyard_edge_holds() {
        let mut b = Body {
            pos: [23.0, 0.0, 0.0],
            vel: [8.5, 0.0, 0.0],
            ..at_rest()
        };
        advance(&mut b, [8.5, 0.0], false, 0.25, &[floor()], &[], 24.0);
        assert!(
            (b.pos[0] - (24.0 - RADIUS)).abs() < 1e-5,
            "x = {}",
            b.pos[0]
        );
    }
}
