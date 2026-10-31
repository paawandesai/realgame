//! The Formula Bench in the courtyard (I-028): the lab pad landmark, the
//! bench's command queue (`window.__gameIn.bench`) and its view
//! (`window.__benchState`). Judging is `realgame_sim::bench::judge`; the UI is
//! `web/bench.js`. See `docs/loop/items/I-028/spec.md`, sections 1, 5 and 6.
//!
//! Contract stub: `spawn_station` only inserts `BenchState`, and `run` only
//! publishes the closed view once. The client lane writes the station, the
//! command loop, the `learn` call and the change-only publish.

// Contract stub: the fields and constants are read once the client lane lands.
#![allow(dead_code)]

use bevy::prelude::*;
use realgame_sim::bench::{BenchView, ResultView};
use realgame_sim::tier::Tier;

use crate::bridge;

/// The pad's centre on the ground (x, z), about 41° left of the start view.
pub const STATION: (f32, f32) = (-3.0, 14.5);
/// The pad's radius (m): `atBench` while the player's feet are this close.
pub const RADIUS: f32 = 1.2;

/// The bench's state. `pub` so I-029's worked casts and I-030's trial read
/// it (`band` in particular, kept when the bench closes).
#[derive(Resource, Default)]
pub struct BenchState {
    pub open: bool,
    /// Set only by the `band` command.
    pub band: Option<Tier>,
    /// The picked slot; 0 = none.
    pub pick: u8,
    pub result: Option<ResultView>,
    /// Bumps per judged try.
    pub n: u32,
    /// The last JSON published, so `__benchState` is written only on change.
    pub last_json: String,
}

/// `Startup`: inserts `BenchState` (so `main.rs` needs no `init_resource`)
/// and spawns the lab pad and its flask icon.
pub fn spawn_station(mut commands: Commands) {
    commands.insert_resource(BenchState::default());
}

/// `Update`, after `control` and before `publish`: drains the bench queue,
/// judges tries, learns on a first correct build and publishes the view.
pub fn run(mut bench: ResMut<BenchState>) {
    let json = BenchView::closed(false, None).to_json();
    if json != bench.last_json {
        bridge::publish_bench(&json);
        bench.last_json = json;
    }
}
