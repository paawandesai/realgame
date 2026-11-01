//! The block world's view (I-052, Lane A): chunk entities and the
//! time-sliced mesh queue, the generated atlas, sky cover and lighting,
//! break and place with the Pick, the 1-9 hotbar and `?biome=` spawns.
//! Registered only in `Mode::Voxel`.
//!
//! Contract step: an empty plugin that owns its state keys (`Keys`), which
//! `publish` reads. Lane A fills it (spec section 12).

use bevy::prelude::*;

/// This plugin's values for the voxel-mode state keys (spec section 8);
/// `publish` formats them. Values, not JSON fragments.
#[derive(Resource, Debug, Clone, Default)]
pub struct Keys {
    /// `biome`: the id of the biome the player is in.
    pub biome: &'static str,
    /// `hot`: the selected hotbar slot, 1-9.
    pub hot: u8,
    /// `lookCell`: `[x, y, z, mat]` under the crosshair, or none.
    pub look_cell: Option<[i32; 4]>,
    /// `digT`: dig progress 0..1.
    pub dig_t: f32,
    /// `chunksMeshed`: chunks with a current mesh.
    pub chunks_meshed: u32,
    /// `meshQ`: chunks waiting in the mesh queue.
    pub mesh_q: u32,
}

/// Registers the view (Voxel mode only).
pub fn plugin(app: &mut App) {
    let biome = app
        .world()
        .get_resource::<crate::StartBiome>()
        .map_or(realgame_voxel::Biome::Village, |b| b.0);
    app.insert_resource(Keys {
        biome: biome.id(),
        hot: 2,
        ..default()
    });
}
