//! Picking: Amanatides-Woo DDA through the block grid (section 5.8).
//!
//! Stub (contract step): always misses. Lane A fills it (V7).

use crate::World;

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

/// Casts from `origin` along `dir` up to `max_t` metres.
pub fn raycast(
    world: &World,
    origin: [f32; 3],
    dir: [f32; 3],
    max_t: f32,
    mode: RayMode,
) -> Option<Hit> {
    let _ = (world, origin, dir, max_t, mode);
    None
}
