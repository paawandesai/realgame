//! Cross-track event contract for v3 (DESIGN_V3 §12.2). Frozen after Hour 0:
//! change it only with every track's agreement, because the sim (Tracks A and
//! B) emits these and the web shell (Tracks C and D) reacts to them.
//!
//! Everything here is presentation-only: an event never feeds back into the
//! simulation, so emitting or dropping one cannot change a grid hash.

/// `Fx::kind` values added in v3, with what each `Fx` field carries. Fields
/// not listed keep their defaults (`tx`/`ty` = `x`/`y`, `dh_kj` 0, `id` 0).
pub mod fx_kind {
    /// A reaction fired for the first time this game (story, Trial or Lab):
    /// reveal its Grimoire card. `reaction_id` = the card id (a reaction id
    /// from `reactions::ids`, a phase change id, or a `flame_test_*` light
    /// card). v2 already emits "discover" on a cast's first impact with the
    /// same payload.
    pub const DISCOVER: &str = "discover";
    /// H2O2 decomposed into foam. `power` = foam cells made this tick.
    pub const FOAM: &str = "foam";
    /// Burning Mg met water or steam (`mg_steam`). `power` = 0..=1 strength.
    /// The white flash is capped at 3 Hz by the web shell (§10).
    pub const MG_FLARE: &str = "mg_flare";
    /// An Indicator pool's pH changed visibly. `power` = new pH (0..=14,
    /// e.g. 6.5), `dh_kj` = the neutralisation heat if any.
    pub const PH_SHIFT: &str = "ph_shift";
    /// A salt hit a flame and recoloured it. `reaction_id` =
    /// `flame_test_<el>` (lower case, e.g. `flame_test_na`), `power` =
    /// `Emitter as u8`.
    pub const SALT: &str = "salt";
    /// Sand put a flame out (oxidiser blocked). `power` = cells smothered.
    pub const SMOTHER: &str = "smother";
    /// The catalyst flies back to the suit (it is not used up). `x`/`y` =
    /// where it worked, `tx`/`ty` = the player's hand.
    pub const CATALYST_RETURN: &str = "catalyst_return";

    pub const ALL: [&str; 7] = [
        DISCOVER,
        FOAM,
        MG_FLARE,
        PH_SHIFT,
        SALT,
        SMOTHER,
        CATALYST_RETURN,
    ];
}

/// A presentation event raised inside the grid step (by `chem::grid_rules`).
/// `Game::tick` drains them into `Fx` after each step, so grid code never
/// needs the `Game`.
#[derive(Debug, Clone, PartialEq)]
pub struct ChemEvent {
    /// One of `fx_kind`.
    pub kind: &'static str,
    pub x: i32,
    pub y: i32,
    pub reaction_id: Option<&'static str>,
    /// Becomes `Fx::power` (see each kind for its meaning).
    pub value: f32,
}

/// At most this many events wait in the grid between drains, so a runaway
/// reaction cannot grow memory.
pub const MAX_PENDING_EVENTS: usize = 128;
