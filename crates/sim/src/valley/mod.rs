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

/// Material ids from `content/materials.json` (the same rows as
/// `realgame_voxel::mat`, which this crate cannot depend on; the client's
/// `story` tests that they match), plus the Furnace, appended as row 27.
pub mod mat {
    pub const DIRT: u8 = 1;
    pub const STONE: u8 = 4;
    pub const WOOD: u8 = 7;
    pub const COAL_ORE: u8 = 15;
    pub const FURNACE: u8 = 27;
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

    /// A quest already at `step` (the `quest <n>` debug command and
    /// `?biome=` seeding). Stub.
    pub fn at(step: Step) -> Quest {
        let _ = step;
        Quest { step: 0 }
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

/// Paawan's partial mastery (2026-11-01): in one run, a spell becomes
/// Practised after [`PRACTISE_CASTS`] correct worked casts, which shortens
/// its worked cast to a one-step confirm. Full Mastered stays the 2-day rule
/// (I-030). Biome gates need the previous biome's spell Practised.
pub mod practice {
    /// Correct worked casts in a run that make a spell Practised.
    pub const PRACTISE_CASTS: u8 = 3;
    /// Spells in `Spell::ALL` order: Fire, Water, Frost, CO2.
    pub const SPELLS: usize = 4;

    /// Correct worked casts per spell in this run (not saved).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct RunPractice {
        pub casts: [u8; SPELLS],
    }

    impl RunPractice {
        /// Counts one correct worked cast of `spell`; returns true the moment
        /// it becomes Practised in this run. Stub.
        pub fn record(&mut self, spell: usize) -> bool {
            let _ = spell;
            false
        }

        /// Practised in this run. Stub.
        pub fn practised(&self, spell: usize) -> bool {
            let _ = spell;
            false
        }

        /// The stage the worked-cast route should use: the saved stage, raised
        /// to Practised (2) once practised in this run; never lowered, never
        /// raised to Mastered (3). Stub.
        pub fn stage(&self, spell: usize, saved: u8) -> u8 {
            let _ = spell;
            saved
        }
    }

    /// The spell a biome's gate needs Practised (the previous biome's spell),
    /// or `None` for an ungated biome. Stub.
    pub fn gate_spell(biome: &str) -> Option<usize> {
        let _ = biome;
        None
    }

    /// Whether the way into `biome` is open for this run. Stub.
    pub fn biome_open(biome: &str, run: &RunPractice) -> bool {
        let _ = (biome, run);
        false
    }
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
    fn pack_stacks_and_overflows() {
        let dirt = Item::Block(mat::DIRT);
        let mut p = Pack::default();
        // 64 stack limit: 100 dirt fills one slot and starts a second.
        assert_eq!(p.add(dirt, 100), 0);
        assert_eq!(p.count(dirt), 100);
        assert_eq!(p.slots[0], Some(Stack { item: dirt, n: 64 }));
        assert_eq!(p.slots[1], Some(Stack { item: dirt, n: 36 }));
        // Tools and keys are 1 per slot.
        assert_eq!(p.add(Item::Pick, 1), 0);
        assert_eq!(p.add(Item::Pick, 1), 0);
        assert_eq!(p.count(Item::Pick), 2);
        assert_eq!(p.slots.iter().flatten().filter(|s| s.item == Item::Pick).count(), 2);
        // Fill every slot, then the leftover comes back.
        let stone = Item::Block(mat::STONE);
        let free = p.slots.iter().filter(|s| s.is_none()).count() as u16;
        assert_eq!(p.add(stone, free * STACK_MAX + 10), 10, "leftover on full");
        assert!(p.slots.iter().all(Option::is_some));
        assert_eq!(p.add(Item::Torch, 3), 3, "a full pack takes nothing new");
        // `take` is all or nothing.
        let before = p.clone();
        assert!(!p.take(dirt, 101), "more than there is");
        assert_eq!(p, before, "a failed take changes nothing");
        assert!(p.take(dirt, 70));
        assert_eq!(p.count(dirt), 30);
        assert!(p.take(dirt, 30));
        assert_eq!(p.count(dirt), 0);
        assert!(p.slots.iter().all(|s| s.is_none_or(|s| s.item != dirt)), "emptied slots are freed");
        // JSON: one entry per filled slot.
        let mut q = Pack::default();
        q.add(Item::Block(mat::COAL_ORE), 2);
        q.add(Item::KilnSpark, 1);
        assert_eq!(q.to_json(), r#"[{"item":"coal_ore","n":2},{"item":"kiln_spark","n":1}]"#);
    }

    /// C2: `craft("torch")` fails with `NotEnough{coal}` and leaves the pack
    /// unchanged; with both it gives 4 torches.
    #[test]
    fn torch_needs_wood_and_coal() {
        let (wood, coal) = (Item::Block(mat::WOOD), Item::Block(mat::COAL_ORE));
        let mut p = Pack::default();
        p.add(wood, 1);
        let before = p.clone();
        assert_eq!(
            craft(&mut p, "torch"),
            Err(CraftError::NotEnough { item: coal, have: 0, need: 1 })
        );
        assert_eq!(p, before, "a refused craft leaves the pack unchanged");
        p.add(coal, 1);
        assert_eq!(craft(&mut p, "torch"), Ok(()));
        assert_eq!(p.count(Item::Torch), 4);
        assert_eq!((p.count(wood), p.count(coal)), (0, 0));
        assert_eq!(craft(&mut p, "nonsense"), Err(CraftError::UnknownRecipe));
        // The furnace: 8 stone.
        let r = RECIPES.iter().find(|r| r.id == "furnace").expect("furnace recipe");
        assert_eq!(r.needs, &[(Item::Block(mat::STONE), 8)]);
        assert_eq!(r.gives, (Item::Block(mat::FURNACE), 1));
        for r in RECIPES {
            assert!(!r.source.is_empty(), "{} has a source", r.id);
        }
    }

    /// C3: events out of order do not skip steps; repeating an event does not
    /// complete a step twice.
    #[test]
    fn quest_advances_only_in_order() {
        use QuestEvent as E;
        let coal = Item::Block(mat::COAL_ORE);
        let chain = [
            E::IntroDone,
            E::WorkshopOut { by_water: true },
            E::CrossedFord,
            E::GateMelted,
            E::Holding { item: coal, total: 1 },
            E::CrossedHollow,
            E::Holding { item: Item::KilnSpark, total: 1 },
            E::Invented { relights: true },
        ];
        let mut q = Quest::new();
        // Out of order: a later event does nothing at the intro.
        for e in &chain[1..] {
            assert_eq!(q.on(*e), None, "{e:?} before the intro");
        }
        assert_eq!(q.step(), 0);
        for (k, e) in chain.iter().enumerate() {
            // Every later event is ignored before this one.
            for later in &chain[k + 1..] {
                assert_eq!(q.on(*later), None, "{later:?} at step {k}");
            }
            assert_eq!(q.on(*e), Some(k as Step), "{e:?} completes step {k}");
            assert_eq!(q.step(), k as Step + 1);
            // Repeating it does not complete a step twice.
            assert_eq!(q.on(*e), None, "{e:?} again");
            assert_eq!(q.step(), k as Step + 1);
        }
        assert_eq!(q.step(), 8, "the ending");
        // The workshop counts only when water put a fire out; a fizzled
        // invention does not finish.
        let mut q = Quest::new();
        q.on(E::IntroDone);
        assert_eq!(q.on(E::WorkshopOut { by_water: false }), None);
        assert_eq!(q.on(E::Holding { item: coal, total: 0 }), None);
        let mut q = Quest::new();
        for e in &chain[..7] {
            q.on(*e);
        }
        assert_eq!(q.step(), 7);
        assert_eq!(q.on(E::Invented { relights: false }), None);
        assert_eq!(q.step(), 7);
        // Holding the wrong item at the coal step does nothing.
        let mut q = Quest::new();
        for e in &chain[..4] {
            q.on(*e);
        }
        assert_eq!(q.on(E::Holding { item: Item::Block(mat::DIRT), total: 5 }), None);
        assert_eq!(q.step(), 4);
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
    fn usable_follows_band() {
        let book = crate::spellbook::SpellBook::load(&crate::reactions::ReactionBook::load());
        let def = |id: &str| book.by_id(id).expect(id).clone();
        let (fire, frost) = (def("fire"), def("frost"));
        assert!(usable(&fire, Tier::Sprout, 0), "Fire at Sprout: usable while unknown");
        assert!(!usable(&frost, Tier::Seedling, 0), "Frost at Seedling: not while unknown");
        for stage in 1..=3 {
            assert!(usable(&frost, Tier::Seedling, stage), "Frost at Seedling, stage {stage}");
        }
    }

    /// C6: every step has a line for all four bands and for no band, at most
    /// 90 characters.
    #[test]
    fn quest_lines_exist_per_band() {
        let bands = [None, Some(Tier::Sprout), Some(Tier::Seedling), Some(Tier::Apprentice), Some(Tier::Adept)];
        for step in 1..=STEPS {
            let q = Quest::at(step);
            for b in bands {
                let line = q.line(b);
                assert!(!line.is_empty(), "step {step} {b:?}");
                assert!(line.chars().count() <= 90, "step {step} {b:?}: {} chars", line.chars().count());
                assert!(!line.chars().any(|c| c == '"' || c == '\\' || c.is_control()), "{line:?}");
            }
            assert_eq!(q.line(None), q.line(Some(Tier::Seedling)), "no band reads as Seedling");
        }
    }

    // -----------------------------------------------------------------------
    // Paawan's partial mastery (coordinator, 2026-11-01).
    // -----------------------------------------------------------------------

    #[test]
    fn three_correct_worked_casts_make_a_spell_practised_in_the_run() {
        use practice::*;
        let mut run = RunPractice::default();
        assert!(!run.practised(1));
        assert!(!run.record(1));
        assert!(!run.record(1));
        assert!(!run.practised(1), "two casts are not enough");
        assert!(run.record(1), "the third makes Water Practised");
        assert!(run.practised(1));
        assert!(!run.record(1), "it becomes Practised once");
        assert!(!run.practised(2), "other spells are separate");
        // The route stage: raised to Practised, never lowered, never Mastered.
        assert_eq!(run.stage(1, 1), 2, "Learning -> Practised in this run");
        assert_eq!(run.stage(1, 3), 3, "Mastered stays Mastered");
        assert_eq!(run.stage(2, 1), 1, "an unpractised spell keeps its stage");
        assert_eq!(run.stage(1, 0), 2);
        assert_eq!(PRACTISE_CASTS, 3);
        // Out-of-range spells are ignored.
        assert!(!run.record(9));
        assert!(!run.practised(9));
    }

    #[test]
    fn biome_gates_need_the_previous_biomes_spell_practised() {
        use practice::*;
        // Village (Water) -> Frozen Fields (Frost) -> Marsh (CO2) -> Cave.
        assert_eq!(gate_spell("village"), None);
        assert_eq!(gate_spell("kiln"), None);
        assert_eq!(gate_spell("frozen"), Some(1), "Water");
        assert_eq!(gate_spell("marsh"), Some(2), "Frost");
        assert_eq!(gate_spell("cave"), Some(3), "CO2");
        let mut run = RunPractice::default();
        assert!(biome_open("village", &run));
        assert!(!biome_open("frozen", &run));
        for _ in 0..3 {
            run.record(1);
        }
        assert!(biome_open("frozen", &run));
        assert!(!biome_open("marsh", &run), "Frost not practised yet");
        for _ in 0..3 {
            run.record(2);
        }
        assert!(biome_open("marsh", &run));
        assert!(!biome_open("cave", &run));
        assert!(biome_open("nowhere", &run), "an unknown biome is not gated");
    }
}
