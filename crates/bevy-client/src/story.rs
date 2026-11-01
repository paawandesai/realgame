//! The game layer's glue (I-052, Lane C): intro, objective line and arrow,
//! the quest (`realgame_sim::valley::Quest`), pack, crafting, the Invention
//! Bench and the ending. Registered only in `Mode::Voxel`.
//!
//! Contract step: `on_event` already receives every `PlayerEvent` and
//! `MatterEvent` message, so Lane C never waits on `main.rs`. Lane C fills it
//! (spec section 12).

use bevy::prelude::*;

use crate::{MatterEvent, PlayerEvent};

/// This plugin's values for the voxel-mode state keys (spec section 8);
/// `publish` formats them. Values, not JSON fragments.
#[derive(Resource, Debug, Clone, Default)]
pub struct Keys {
    /// `quest`: the step, 0-8.
    pub quest: u8,
    /// `questLine`: the objective text.
    pub quest_line: String,
    /// `pack`: `(item, n)` per filled slot.
    pub pack: Vec<(String, u16)>,
    /// `objBearing`: degrees to the step's target, or none.
    pub obj_bearing: Option<f32>,
    /// `ending`: the ending card has shown.
    pub ending: bool,
}

/// Registers the story (Voxel mode only).
pub fn plugin(app: &mut App) {
    app.init_resource::<Keys>()
        .add_systems(Update, on_event.before(crate::publish));
}

/// Hears what the world and the player did. Stub: drains the messages.
pub fn on_event(mut player: MessageReader<PlayerEvent>, mut matter: MessageReader<MatterEvent>) {
    for _ in player.read() {}
    for _ in matter.read() {}
}
