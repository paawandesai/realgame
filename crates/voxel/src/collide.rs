//! Collision boxes for `Player::step` (section 3.1 and 5.8): the solid
//! blocks, and separately the ice blocks, that overlap a box.
//!
//! Boxes are `[min_x, min_y, min_z, max_x, max_y, max_z]` in metres, one per
//! block, in x, then y, then z order. Ice is solid too: the client passes
//! `solid_boxes` and `ice_boxes` together as the solids, and `ice_boxes`
//! again as the slippery sheets.

use crate::materials::flags;
use crate::{mat, World};

/// Solid, non-ice blocks overlapping `aabb` (at most about 60 around the
/// player's swept box).
pub fn solid_boxes(world: &World, aabb: [f32; 6]) -> Vec<[f32; 6]> {
    boxes(world, aabb, |m| m != mat::ICE && flags(m).solid)
}

/// Ice blocks overlapping `aabb` (passed to `Player::step` as `ice_sheets`).
pub fn ice_boxes(world: &World, aabb: [f32; 6]) -> Vec<[f32; 6]> {
    boxes(world, aabb, |m| m == mat::ICE)
}

fn boxes(world: &World, aabb: [f32; 6], keep: impl Fn(u8) -> bool) -> Vec<[f32; 6]> {
    let mut out = Vec::new();
    if aabb.iter().any(|v| !v.is_finite()) {
        return out;
    }
    let lo = [0, 1, 2].map(|a| aabb[a].floor() as i32);
    // A box that ends exactly on a block face does not touch the next block.
    let hi = [0, 1, 2].map(|a| aabb[a + 3].ceil() as i32 - 1);
    for z in lo[2]..=hi[2] {
        for y in lo[1]..=hi[1] {
            for x in lo[0]..=hi[0] {
                if keep(world.get([x, y, z]).mat) {
                    let (fx, fy, fz) = (x as f32, y as f32, z as f32);
                    out.push([fx, fy, fz, fx + 1.0, fy + 1.0, fz + 1.0]);
                }
            }
        }
    }
    out
}
