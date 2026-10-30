//! The first-person kinematic character: mouse look, walking, sprinting and
//! jumping, with move-and-slide against axis-aligned boxes (`motion`). No physics crate.

use crate::motion::{self, Body, Box3};
use crate::world::{Aabb, HALF};
use bevy::prelude::*;

/// Body size and the jump/gravity arithmetic live in `motion`.
pub use crate::motion::RADIUS;
pub const EYE: f32 = 1.6;
const WALK: f32 = 5.0;
const SPRINT: f32 = 8.5;
/// Radians per CSS pixel of mouse movement at `?sens=1` (about 21 cm per
/// turn at 800 CPI; defined with the other comfort numbers in `feel`).
pub use crate::feel::LOOK_PER_PX;

/// What the player asks for this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct MoveInput {
    /// (right, forward), each in -1..1.
    pub wish: Vec2,
    pub sprint: bool,
    pub jump: bool,
}

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

    /// One frame of movement against the world's solid boxes: `dt` (the
    /// clamped frame time, up to `clock::MAX_FRAME_DT`) is split into
    /// steps of at most 1/60 s by `motion::advance`. `solids` and
    /// `ice_sheets` are built once per frame (`boxes`).
    pub fn step(&mut self, input: MoveInput, dt: f32, solids: &[Box3], ice_sheets: &[Box3]) {
        let MoveInput { wish, sprint, jump } = input;
        let fwd = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let wish = wish.clamp_length_max(1.0);
        let speed = if sprint { SPRINT } else { WALK };
        let target = (right * wish.x + fwd * wish.y) * speed;
        let mut body = Body {
            pos: self.pos.to_array(),
            vel: self.vel.to_array(),
            grounded: self.grounded,
            on_ice: self.on_ice,
        };
        let before = self.pos;
        motion::advance(
            &mut body,
            [target.x, target.z],
            jump,
            dt,
            solids,
            ice_sheets,
            HALF,
        );
        self.pos = Vec3::from_array(body.pos);
        self.vel = Vec3::from_array(body.vel);
        self.grounded = body.grounded;
        self.on_ice = body.on_ice;
        self.stride += (self.pos - before).xz().length();
    }
}

/// World boxes as the integrator's plain boxes.
pub fn boxes<'a>(it: impl IntoIterator<Item = &'a Aabb>) -> Vec<Box3> {
    it.into_iter()
        .map(|a| Box3 {
            min: a.min.to_array(),
            max: a.max.to_array(),
        })
        .collect()
}
