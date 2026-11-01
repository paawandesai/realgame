//! Collision boxes for `Player::step` (section 3.1 and 5.8): the solid
//! blocks, and separately the ice blocks, that overlap a box.
//!
//! Boxes are `[min_x, min_y, min_z, max_x, max_y, max_z]` in metres.
//! Stub (contract step): returns no boxes. Lane A fills it (V8).

use crate::World;

/// Solid, non-ice blocks overlapping `aabb` (at most about 60 around the
/// player's swept box).
pub fn solid_boxes(world: &World, aabb: [f32; 6]) -> Vec<[f32; 6]> {
    let _ = (world, aabb);
    Vec::new()
}

/// Ice blocks overlapping `aabb` (passed to `Player::step` as `ice_sheets`).
pub fn ice_boxes(world: &World, aabb: [f32; 6]) -> Vec<[f32; 6]> {
    let _ = (world, aabb);
    Vec::new()
}
