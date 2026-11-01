//! Block chemistry glue (I-052, Lane B): runs `realgame_matter` on the voxel
//! world at 10 steps/s, turns a landed spell into one `matter::Emit`
//! (`on_impact`), pushes the player back from deep water (`wade_push`), and
//! draws captions, gas puffs, beads, the rig and the balloon crate.
//! Registered only in `Mode::Voxel`.
//!
//! Contract step: an empty plugin that owns its state keys (`Keys`), which
//! `publish` reads. Lane B fills it (spec section 12).

use bevy::prelude::*;

/// This plugin's values for the voxel-mode state keys (spec section 8);
/// `publish` formats them. Values, not JSON fragments.
#[derive(Resource, Debug, Clone, Default)]
pub struct Keys {
    /// `activeChunks`: chunks the matter step is working on.
    pub active_chunks: u32,
    /// `matterMs`: CPU time of the last matter step.
    pub matter_ms: f32,
    /// `gasCO2Max`: the largest CO2 fraction in the player's chunk.
    pub gas_co2_max: f32,
}

/// Registers block chemistry (Voxel mode only). Matter events go out as
/// `crate::MatterEvent` messages.
pub fn plugin(app: &mut App) {
    app.init_resource::<Keys>();
}
