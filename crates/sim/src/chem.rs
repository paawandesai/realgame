//! v3 chemistry hooks (DESIGN_V3 §12.3). Track A owns this file after Hour 0
//! and never edits `game.rs`: everything v3 chemistry needs from the game
//! comes in through `CastCtx` / `ImpactCtx`, and everything it sends back
//! goes out as a `CastOutcome`, an `ExtraShot`, `Fx` or a grid `ChemEvent`.
//!
//! The three entry points are stubs at Hour 0, so the v2 story behaves (and
//! hashes) exactly as before:
//! - `cast_extra`: called first in `Game::cast`. `Some` claims the cast.
//! - `impact_extra`: called when a bolt launched by `cast_extra` lands.
//! - `grid_rules`: called from the grid's chemistry pass for every v3
//!   species cell (and any other cell Track A routes here).

use crate::game::{CastOutcome, Fx, Tier};
use crate::grid::Grid;
use crate::reactions::ReactionBook;
use crate::species::Emitter;
use crate::spell::{Outcome, Stock};

/// The v3 spells: (word a player may type, formula the tray casts, the
/// `CastOutcome::spell` name). The web tray (Track C) sends the formula
/// column for tiles; `cast_extra` must accept every word and formula here,
/// case-insensitively for words (DESIGN_V3 §4.2 "Spells").
pub const V3_SPELLS: [(&str, &str, &str); 9] = [
    ("catalyst", "2H2O2 -> 2H2O + O2", "catalyst"),
    ("vinegar", "CH3COOH", "acid"),
    ("washing soda", "Na2CO3", "base"),
    ("sand", "SiO2", "sand"),
    ("salt", "NaCl", "salt"),
    ("lithium salt", "LiCl", "salt"),
    ("potash salt", "KCl", "salt"),
    ("copper salt", "CuCl2", "salt"),
    ("strontium salt", "SrCl2", "salt"),
];

/// What `cast_extra` sees of the game. Borrowed from `Game` fields for the
/// length of one cast.
pub struct CastCtx<'a> {
    /// This cast's id: put it on the `CastOutcome` and on every `Fx`.
    pub id: u32,
    pub tier: Tier,
    pub book: &'a ReactionBook,
    pub grid: &'a mut Grid,
    /// Suit gauges in moles (spend reagents here).
    pub jars: &'a mut Stock,
    /// Where the spell leaves the hand, and where it is aimed.
    pub hand: (f32, f32),
    pub aim: (i32, i32),
    pub ticks: u64,
    /// Reaction ids already discovered this game.
    pub discovered: &'a [String],
    /// Events for the cast itself (the game emits them in order).
    pub fx: Vec<Fx>,
    /// Set this to fly a bolt from the hand to the aim through the normal
    /// bolt pipeline ("charge"/"bolt"/"impact" Fx); `impact_extra` runs
    /// when it lands. Leave it None for an instant effect.
    pub launch: Option<ExtraShot>,
}

/// A v3 spell in flight.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtraShot {
    /// "catalyst", "acid", "base", "sand" or "salt".
    pub spell: &'static str,
    pub reaction_id: Option<String>,
    pub outcome: Outcome,
    pub dh_kj: f32,
    /// Share of the full effect, 0..=1.
    pub power: f32,
    /// Spell-specific amount (moles, or cells for sand).
    pub amount: u32,
    /// The salt's emitter (`Emitter::None` for other spells).
    pub emitter: Emitter,
    /// The cast discovered its reaction (the game adds a "discover" Fx).
    pub first: bool,
}

/// What `impact_extra` sees when a v3 bolt lands. Raise presentation events
/// with `grid.event(..)`; the game turns them into `Fx` after the step.
pub struct ImpactCtx<'a> {
    pub id: u32,
    pub tier: Tier,
    pub book: &'a ReactionBook,
    pub grid: &'a mut Grid,
    pub hand: (f32, f32),
    pub ticks: u64,
}

/// Claims a cast that the v2 pipeline does not know (DESIGN_V3 §4.2 spells).
/// Return None to let `spell::plan` handle it as before. Stub: always None.
pub fn cast_extra(ctx: &mut CastCtx, text: &str) -> Option<CastOutcome> {
    let _ = (ctx, text);
    None
}

/// A bolt launched by `cast_extra` landed at `at`. Stub: does nothing.
pub fn impact_extra(ctx: &mut ImpactCtx, shot: &ExtraShot, at: (i32, i32)) {
    let _ = (ctx, shot, at);
}

/// Per-cell v3 rules in the grid's chemistry pass (Mg burning, foam decay,
/// pH mixing, limewater, sand smothering...). Called once per step for each
/// v3 species cell not already reacted this frame. Stub: does nothing.
pub fn grid_rules(grid: &mut Grid, x: i32, y: i32) {
    let _ = (grid, x, y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_table_formulas_are_distinct() {
        let mut f: Vec<&str> = V3_SPELLS.iter().map(|s| s.1).collect();
        f.sort();
        f.dedup();
        assert_eq!(f.len(), V3_SPELLS.len());
    }
}
