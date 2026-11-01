//! I-052 block valley game layer (spec sections 6-7): the pack, crafting,
//! the quest state machine and the Invention Bench. Pure, no Bevy: the quest
//! never reads cells; it hears events from the client's `blockchem`,
//! `voxel_view` and benches (spec section 10, contract rules).
//!
//! Contract step: the types and signatures are fixed here; the bodies marked
//! "stub" are filled by Lane C (spec section 12), which owns this module and
//! may add variants and helpers.

use crate::spellbook::SpellDef;
use crate::tier::Tier;

/// Something the pack can hold. `Block` carries a material id (the row in
/// `content/materials.json`, `realgame_voxel::mat`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Block(u8),
    Pick,
    Torch,
    O2Flask,
    H2Balloon,
    KilnSpark,
}

/// Up to 64 of a block, 1 of a tool or key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stack {
    pub item: Item,
    pub n: u16,
}

/// Most of one block in a slot.
pub const STACK_MAX: u16 = 64;
/// Pack slots.
pub const SLOTS: usize = 20;

/// The player's pack: 20 slots, and keys 6-9 bound to slots.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pack {
    pub slots: [Option<Stack>; SLOTS],
    /// Keys 6-9 -> slot index.
    pub hot: [Option<u8>; 4],
}

impl Pack {
    /// Adds `n` of `item`; returns how many did not fit. Stub: adds nothing.
    pub fn add(&mut self, item: Item, n: u16) -> u16 {
        let _ = item;
        n
    }

    /// Takes `n` of `item`, all or nothing. Stub: never takes.
    pub fn take(&mut self, item: Item, n: u16) -> bool {
        let _ = (item, n);
        false
    }

    /// How many of `item` the pack holds. Stub: 0.
    pub fn count(&self, item: Item) -> u32 {
        let _ = item;
        0
    }

    /// The `pack` state key: a JSON array of `{item, n}`. Stub: empty.
    pub fn to_json(&self) -> String {
        "[]".to_string()
    }
}

/// A Formula Bench "Make things" recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipe {
    pub id: &'static str,
    pub needs: &'static [(Item, u16)],
    pub gives: (Item, u16),
    /// Key into `content/science-sources.md`.
    pub source: &'static str,
}

/// Must recipes: torch (1 wood + 1 coal -> 4 torches) and furnace (8 stone).
/// Stub: empty until Lane C fills it.
pub const RECIPES: &[Recipe] = &[];

/// Why `craft` refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CraftError {
    NotEnough { item: Item, have: u32, need: u16 },
    UnknownRecipe,
}

/// Crafts recipe `id` from the pack, all or nothing. Stub: unknown recipe.
pub fn craft(pack: &mut Pack, id: &str) -> Result<(), CraftError> {
    let _ = (pack, id);
    Err(CraftError::UnknownRecipe)
}

/// A quest step: 0 the intro, 1-7 the steps in section 7, 8 the ending.
pub type Step = u8;
/// Steps after the intro.
pub const STEPS: Step = 7;

/// What the client tells the quest (from `MatterEvent`s, `PlayerEvent`s and
/// the benches; built in the client's `story`). Lane C may add variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestEvent {
    IntroDone,
    /// No burning cell left in the workshop stamp; `by_water` if at least one
    /// was put out by water.
    WorkshopOut {
        by_water: bool,
    },
    /// The player stands at z < 150 inside the ford box.
    CrossedFord,
    /// The gate stamp's ice is all gone.
    GateMelted,
    /// The pack now holds `total` of `item`.
    Holding {
        item: Item,
        total: u32,
    },
    /// Every hollow vent was out at once and the player crossed.
    CrossedHollow,
    /// The Invention Bench accepted a reaction hot enough for the Kiln.
    Invented {
        relights: bool,
    },
}

/// The quest state machine (section 7). Pure: it never polls the world.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Quest {
    step: Step,
}

impl Quest {
    pub fn new() -> Quest {
        Quest { step: 0 }
    }

    /// Feeds one event; returns the step just completed, if any.
    /// Stub: never advances.
    pub fn on(&mut self, event: QuestEvent) -> Option<Step> {
        let _ = event;
        None
    }

    /// The current step (0-8).
    pub fn step(&self) -> Step {
        self.step
    }

    /// The objective line for the current step and band (`None`: no band
    /// chosen, Seedling text). Stub: empty.
    pub fn line(&self, band: Option<Tier>) -> &'static str {
        let _ = band;
        ""
    }
}

/// The Invention Bench (section 6): builds a reaction from 1-3 pack items and
/// asks `crate::reactions` whether it is real, exothermic and hot enough.
pub mod invent {
    use super::Item;
    use crate::bench::Code;
    use crate::tier::Tier;

    /// The bench's verdict.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Invention {
        /// A balanced, exothermic reaction from `content/reactions.json`
        /// uses only these reactants.
        pub ok: bool,
        /// Its flame in pure O2 reaches 1700 °C: it relights the Kiln.
        pub relights: bool,
        /// The reaction id (`carbon_combustion`, `water_synthesis`, ...).
        pub reaction: Option<String>,
        /// For typed text: the Formula Bench code (`atoms_off`, ...).
        pub code: Code,
        /// A kind reason for a fizzle or a refusal.
        pub reason: String,
        pub flame_c: f32,
    }

    /// Judges the items on the bench, or `text` when Apprentice or Adept
    /// typed an equation. Stub: refuses everything.
    pub fn judge(items: &[Item], band: Tier, text: &str) -> Invention {
        let _ = (items, band, text);
        Invention::default()
    }
}

/// HUD hint only (section 7): a spell is usable at stage >= Learning when the
/// band can learn it, and always when it cannot (Fire at Sprout), matching
/// the client's `worked::route`. `stage` uses the client's numbering (0
/// Unknown, 1 Learning, 2 Practised, 3 Mastered). Stub: false.
pub fn usable(def: &SpellDef, band: Tier, stage: u8) -> bool {
    let _ = (def, band, stage);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tier::Tier;

    // -----------------------------------------------------------------------
    // Contract step (section 10, item 5): green from the first merge.
    // -----------------------------------------------------------------------

    #[test]
    fn valley_contract_types_exist() {
        let p = Pack::default();
        assert_eq!(p.slots.len(), 20);
        assert!(p.slots.iter().all(Option::is_none));
        assert_eq!(p.hot, [None; 4]);
        let _ = Stack {
            item: Item::Block(7),
            n: 1,
        };
        for it in [
            Item::Pick,
            Item::Torch,
            Item::O2Flask,
            Item::H2Balloon,
            Item::KilnSpark,
        ] {
            let _ = Stack { item: it, n: 1 };
        }
        let _: &[Recipe] = RECIPES;
        let q = Quest::new();
        assert_eq!(q.step(), 0, "the quest starts at the intro");
        assert_eq!(STEPS, 7, "steps 1-7 after the intro (section 7)");
        let _ = |q: &mut Quest| q.on(QuestEvent::IntroDone);
        let _ = |q: &Quest| q.line(Some(Tier::Seedling)).len() + q.line(None).len();
        let _ = |b: Tier| invent::judge(&[Item::Block(15), Item::O2Flask], b, "");
    }

    // -----------------------------------------------------------------------
    // Acceptance (section 11, C1-C6). Bodies by Lane C (spec section 12);
    // each ignored body is `todo!()`, so un-ignoring without one fails.
    // -----------------------------------------------------------------------

    /// C1: 64 stack limit, leftover on full, `take` is all-or-nothing.
    #[test]
    #[ignore = "Lane C (Sun night): body with Pack"]
    fn pack_stacks_and_overflows() {
        todo!("C1")
    }

    /// C2: `craft("torch")` fails with `NotEnough{coal}` and leaves the pack
    /// unchanged; with both it gives 4 torches.
    #[test]
    #[ignore = "Lane C (Sun night): body with craft"]
    fn torch_needs_wood_and_coal() {
        todo!("C2")
    }

    /// C3: events out of order do not skip steps; repeating an event does not
    /// complete a step twice.
    #[test]
    #[ignore = "Lane C (Sun night): body with the quest table"]
    fn quest_advances_only_in_order() {
        todo!("C3")
    }

    /// C4: `{Coal, O2Flask}` -> `carbon_combustion`, relights; `{H2Balloon,
    /// O2Flask}` -> `water_synthesis`, relights; `{Coal}` -> fizzle "needs
    /// oxygen"; `{water}` -> fizzle "nothing to burn"; typed `C + O2 -> CO2` at
    /// Apprentice is accepted; typed `C + O2 -> CO` is refused with the
    /// existing `atoms_off` code and its reason (unbalanced as written; the
    /// reason must not call carbon monoxide impossible).
    #[test]
    #[ignore = "Lane C (Tue): body with the Invention Bench"]
    fn invention_judges_real_reactions() {
        todo!("C4")
    }

    /// C5: Fire at Sprout is usable while unknown; Frost at Seedling is usable
    /// only from Learning.
    #[test]
    #[ignore = "Lane C (Sun night): body with usable"]
    fn usable_follows_band() {
        todo!("C5")
    }

    /// C6: every step has a line for all four bands and for no band, at most
    /// 90 characters.
    #[test]
    #[ignore = "Lane C (Sun night): body with the quest lines"]
    fn quest_lines_exist_per_band() {
        todo!("C6")
    }
}
