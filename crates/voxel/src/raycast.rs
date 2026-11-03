//! Picking: Amanatides-Woo DDA through the block grid (section 5.8).
//!

use crate::materials::{flags, Phase};
use crate::{World, WORLD};

/// What a ray stops at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RayMode {
    /// Solid blocks only; liquids are passed through (the Pick, placing).
    Solids,
    /// Solid blocks and liquids (Frost and Fire target water).
    Fluids,
}

/// The first block a ray hits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// The block hit.
    pub cell: [i32; 3],
    /// The face it was entered through (unit axis vector); a block placed
    /// on the hit goes at `cell + normal`.
    pub normal: [i8; 3],
    /// Distance along `dir` (normalised) to the entry point.
    pub t: f32,
}

/// Casts from `origin` along `dir` up to `max_t` metres. `dir` need not be
/// unit length (it is normalised; a zero direction hits nothing). A ray
/// that starts outside the world is advanced to where it enters; a ray
/// that starts inside a block it would stop at hits that block at `t = 0`,
/// with the normal facing back along the ray's main axis.
pub fn raycast(
    world: &World,
    origin: [f32; 3],
    dir: [f32; 3],
    max_t: f32,
    mode: RayMode,
) -> Option<Hit> {
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if len < 1e-9 || !len.is_finite() || origin.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let d = dir.map(|v| v / len);
    // Clip the ray to the world box.
    let (mut t0, mut t1) = (0.0f32, max_t);
    for a in 0..3 {
        let hi = WORLD[a] as f32;
        if d[a].abs() < 1e-12 {
            if origin[a] < 0.0 || origin[a] >= hi {
                return None;
            }
            continue;
        }
        let (mut ta, mut tb) = ((0.0 - origin[a]) / d[a], (hi - origin[a]) / d[a]);
        if ta > tb {
            std::mem::swap(&mut ta, &mut tb);
        }
        t0 = t0.max(ta);
        t1 = t1.min(tb);
    }
    if t0 > t1 {
        return None;
    }
    let stops = |c: [i32; 3]| {
        let f = flags(world.get(c).mat);
        match mode {
            RayMode::Solids => f.phase == Phase::Solid,
            RayMode::Fluids => f.phase == Phase::Solid || f.phase == Phase::Liquid,
        }
    };
    let start = [0, 1, 2].map(|a| origin[a] + d[a] * t0);
    // Nudge a point on the world's face into the world.
    let mut cell = [0, 1, 2].map(|a| {
        let c = start[a].floor() as i32;
        c.clamp(0, WORLD[a] - 1)
    });
    let step = d.map(|v| if v > 0.0 { 1 } else { -1 });
    let mut t_max = [0f32; 3];
    let mut t_delta = [f32::INFINITY; 3];
    for a in 0..3 {
        if d[a].abs() < 1e-12 {
            t_max[a] = f32::INFINITY;
            continue;
        }
        let edge = if d[a] > 0.0 {
            cell[a] as f32 + 1.0
        } else {
            cell[a] as f32
        };
        t_max[a] = t0 + (edge - start[a]) / d[a];
        t_delta[a] = 1.0 / d[a].abs();
    }
    // The face the ray entered the first cell through.
    let main = (0..3)
        .max_by(|&a, &b| d[a].abs().total_cmp(&d[b].abs()))
        .unwrap_or(0);
    let mut normal = [0i8; 3];
    normal[main] = -step[main] as i8;
    let mut t = t0;
    loop {
        if !World::in_bounds(cell) || t > max_t {
            return None;
        }
        if stops(cell) {
            return Some(Hit { cell, normal, t });
        }
        let a = if t_max[0] < t_max[1] {
            if t_max[0] < t_max[2] {
                0
            } else {
                2
            }
        } else if t_max[1] < t_max[2] {
            1
        } else {
            2
        };
        t = t_max[a];
        t_max[a] += t_delta[a];
        cell[a] += step[a];
        normal = [0; 3];
        normal[a] = -step[a] as i8;
    }
}
