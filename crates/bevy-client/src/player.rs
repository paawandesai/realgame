//! The first-person kinematic character: mouse look, walking, sprinting and
//! jumping, with move-and-slide against axis-aligned boxes. No physics crate.

use crate::chem;
use crate::world::{Aabb, Colliders, HALF};
use bevy::prelude::*;

pub const RADIUS: f32 = 0.35;
pub const HEIGHT: f32 = 1.75;
pub const EYE: f32 = 1.6;
const WALK: f32 = 5.0;
const SPRINT: f32 = 8.5;
const JUMP: f32 = 6.0;
const GRAVITY: f32 = 18.0;
/// Radians per CSS pixel of mouse movement at `?sens=1`.
pub const LOOK_PER_PX: f32 = 0.0022;

#[derive(Resource, Debug, Clone)]
pub struct Player {
    /// Feet position.
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub grounded: bool,
    /// Distance walked, for the viewmodel bob.
    pub stride: f32,
    pub on_ice: bool,
}

impl Default for Player {
    fn default() -> Self {
        Player {
            pos: Vec3::new(0.0, 0.0, 18.0),
            vel: Vec3::ZERO,
            yaw: 0.0,
            pitch: -0.05,
            grounded: true,
            stride: 0.0,
            on_ice: false,
        }
    }
}

impl Player {
    pub fn eye(&self) -> Vec3 {
        self.pos + Vec3::Y * EYE
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0)
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation() * Vec3::NEG_Z
    }

    /// Mouse look from raw deltas: applied directly, no smoothing.
    pub fn look(&mut self, dx: f32, dy: f32, sens: f32) {
        self.yaw -= dx * LOOK_PER_PX * sens;
        self.pitch = (self.pitch - dy * LOOK_PER_PX * sens).clamp(-1.55, 1.55);
        self.yaw = self.yaw.rem_euclid(std::f32::consts::TAU);
    }

    /// Turns to face a point.
    pub fn face(&mut self, p: Vec3) {
        let d = p - self.eye();
        self.yaw = (-d.x).atan2(-d.z).rem_euclid(std::f32::consts::TAU);
        self.pitch = (d.y / d.xz().length().max(1e-3)).atan().clamp(-1.55, 1.55);
    }

    fn body(&self) -> Aabb {
        Aabb {
            min: self.pos - Vec3::new(RADIUS, 0.0, RADIUS),
            max: self.pos + Vec3::new(RADIUS, HEIGHT, RADIUS),
        }
    }

    /// One movement step. `wish` is (right, forward) in -1..1.
    pub fn step(
        &mut self,
        wish: Vec2,
        sprint: bool,
        jump: bool,
        dt: f32,
        colliders: &Colliders,
        chem: &chem::World,
        ice_sheets: &[Aabb],
    ) {
        let fwd = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let wish = wish.clamp_length_max(1.0);
        let speed = if sprint { SPRINT } else { WALK };
        let target = (right * wish.x + fwd * wish.y) * speed;
        // Snappy on the ground, slippery on ice, a little air control.
        let feet = Aabb {
            min: self.pos - Vec3::new(0.1, 0.3, 0.1),
            max: self.pos + Vec3::new(0.1, 0.3, 0.1),
        };
        self.on_ice = self.grounded && ice_sheets.iter().any(|a| a.overlaps(&feet));
        let accel = match (self.grounded, self.on_ice) {
            (true, false) => 60.0,
            (true, true) => 4.0,
            (false, _) => 12.0,
        };
        let mut h = Vec3::new(self.vel.x, 0.0, self.vel.z);
        let diff = target - h;
        h += diff.clamp_length_max(accel * dt);
        self.vel.x = h.x;
        self.vel.z = h.z;
        if jump && self.grounded {
            self.vel.y = JUMP;
            self.grounded = false;
        }
        self.vel.y -= GRAVITY * dt;

        // Move one axis at a time, pushing out of any box we end up inside.
        let solids: Vec<Aabb> = colliders.solid(chem).copied().collect();
        let before = self.pos;
        self.grounded = false;
        for axis in [0usize, 2, 1] {
            let d = self.vel[axis] * dt;
            if d == 0.0 {
                continue;
            }
            self.pos[axis] += d;
            let mut b = self.body();
            for s in &solids {
                if !b.overlaps(s) {
                    continue;
                }
                let (lo, hi) = match axis {
                    0 => (s.min.x - RADIUS, s.max.x + RADIUS),
                    2 => (s.min.z - RADIUS, s.max.z + RADIUS),
                    _ => (s.min.y - HEIGHT, s.max.y),
                };
                if d > 0.0 {
                    self.pos[axis] = lo - 1e-4;
                } else {
                    self.pos[axis] = hi + 1e-4;
                    if axis == 1 {
                        self.grounded = true;
                    }
                }
                self.vel[axis] = 0.0;
                b = self.body();
            }
        }
        if self.pos.y <= 0.0 {
            self.pos.y = 0.0;
            self.vel.y = 0.0;
            self.grounded = true;
        }
        let lim = HALF - RADIUS;
        self.pos.x = self.pos.x.clamp(-lim, lim);
        self.pos.z = self.pos.z.clamp(-lim, lim);
        self.stride += (self.pos - before).xz().length();
    }
}
