//! World generation (section 4): a column heightmap from seeded value noise
//! per biome, then the stamps and their protected boxes.
//!
//! Stub (contract step): returns an all-air world carrying the seed. Lane A
//! fills flat biomes first (V1, V2), then noise and every stamp (V3, V4).

use crate::World;

/// Generates the world for `seed`. Biome placement is fixed; noise only
/// shapes the ground inside each biome, so every seed has the same puzzles.
pub fn generate(seed: u64) -> World {
    let mut w = World::new();
    w.seed = seed;
    w
}
