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

impl Item {
    /// How many fit in one slot: 64 of a block, 1 of a tool or key.
    pub fn stack_max(self) -> u16 {
        match self {
            Item::Block(_) => STACK_MAX,
            _ => 1,
        }
    }

    /// The item's key in the `pack` state (a material name for a block).
    pub fn key(self) -> &'static str {
        match self {
            Item::Block(id) => mat::NAMES.get(id as usize).copied().unwrap_or("block"),
            Item::Pick => "pick",
            Item::Torch => "torch",
            Item::O2Flask => "o2_flask",
            Item::H2Balloon => "h2_balloon",
            Item::KilnSpark => "kiln_spark",
        }
    }

    /// What the pack shows for the item.
    pub fn label(self) -> String {
        match self {
            Item::Pick => "Pick".into(),
            Item::Torch => "Torch".into(),
            Item::O2Flask => "O₂ flask".into(),
            Item::H2Balloon => "H₂ balloon".into(),
            Item::KilnSpark => "Kiln Spark".into(),
            Item::Block(id) if id == mat::COAL_ORE => "Coal".into(),
            Item::Block(_) => {
                let k = self.key().replace('_', " ");
                let mut c = k.chars();
                c.next()
                    .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
            }
        }
    }

    /// The item for a key (`pick`, `coal_ore`, ...), for debug `give` and
    /// the page's commands.
    pub fn from_key(key: &str) -> Option<Item> {
        match key {
            "pick" => Some(Item::Pick),
            "torch" => Some(Item::Torch),
            "o2_flask" => Some(Item::O2Flask),
            "h2_balloon" => Some(Item::H2Balloon),
            "kiln_spark" => Some(Item::KilnSpark),
            "coal" => Some(Item::Block(mat::COAL_ORE)),
            _ => mat::NAMES
                .iter()
                .position(|n| *n == key)
                .filter(|&i| i > 0)
                .map(|i| Item::Block(i as u8)),
        }
    }
}

impl Pack {
    /// Adds `n` of `item`; returns how many did not fit. Fills slots that
    /// already hold it first, then empty slots, in slot order.
    pub fn add(&mut self, item: Item, n: u16) -> u16 {
        let max = item.stack_max();
        let mut left = n;
        for s in self.slots.iter_mut().flatten() {
            if left == 0 {
                break;
            }
            if s.item == item && s.n < max {
                let k = left.min(max - s.n);
                s.n += k;
                left -= k;
            }
        }
        for s in self.slots.iter_mut() {
            if left == 0 {
                break;
            }
            if s.is_none() {
                let k = left.min(max);
                *s = Some(Stack { item, n: k });
                left -= k;
            }
        }
        left
    }

    /// Takes `n` of `item`, all or nothing (from the last slots first);
    /// emptied slots are freed.
    pub fn take(&mut self, item: Item, n: u16) -> bool {
        if self.count(item) < u32::from(n) {
            return false;
        }
        let mut left = n;
        for slot in self.slots.iter_mut().rev() {
            if left == 0 {
                break;
            }
            if let Some(s) = slot {
                if s.item == item {
                    let k = left.min(s.n);
                    s.n -= k;
                    left -= k;
                    if s.n == 0 {
                        *slot = None;
                    }
                }
            }
        }
        true
    }

    /// How many of `item` the pack holds.
    pub fn count(&self, item: Item) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|s| s.item == item)
            .map(|s| u32::from(s.n))
            .sum()
    }

    /// The `pack` state key: a JSON array of `{item, n}`, one per filled slot.
    pub fn to_json(&self) -> String {
        let parts: Vec<String> = self
            .slots
            .iter()
            .flatten()
            .map(|s| format!("{{\"item\":\"{}\",\"n\":{}}}", s.item.key(), s.n))
            .collect();
        format!("[{}]", parts.join(","))
    }

    /// `(key, n)` per filled slot, for the client's state keys.
    pub fn entries(&self) -> Vec<(&'static str, u16)> {
        self.slots
            .iter()
            .flatten()
            .map(|s| (s.item.key(), s.n))
            .collect()
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
/// Both are game recipes, not chemistry claims ([S0], design choice).
pub const RECIPES: &[Recipe] = &[
    Recipe {
        id: "torch",
        needs: &[(Item::Block(mat::WOOD), 1), (Item::Block(mat::COAL_ORE), 1)],
        gives: (Item::Torch, 4),
        source: "S0",
    },
    Recipe {
        id: "furnace",
        needs: &[(Item::Block(mat::STONE), 8)],
        gives: (Item::Block(mat::FURNACE), 1),
        source: "S0",
    },
];

/// Why `craft` refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CraftError {
    NotEnough {
        item: Item,
        have: u32,
        need: u16,
    },
    UnknownRecipe,
    /// The result would not fit in the pack.
    PackFull,
}

/// Crafts recipe `id` from the pack, all or nothing: every input must be
/// there and the result must fit, or the pack is left unchanged.
pub fn craft(pack: &mut Pack, id: &str) -> Result<(), CraftError> {
    let r = RECIPES
        .iter()
        .find(|r| r.id == id)
        .ok_or(CraftError::UnknownRecipe)?;
    for &(item, need) in r.needs {
        let have = pack.count(item);
        if have < u32::from(need) {
            return Err(CraftError::NotEnough { item, have, need });
        }
    }
    let mut next = pack.clone();
    for &(item, need) in r.needs {
        next.take(item, need);
    }
    let (item, n) = r.gives;
    if next.add(item, n) > 0 {
        return Err(CraftError::PackFull);
    }
    *pack = next;
    Ok(())
}

/// Whether recipe `id` can be made from the pack now (the Make tab greys
/// out the others).
pub fn can_craft(pack: &Pack, id: &str) -> bool {
    craft(&mut pack.clone(), id).is_ok()
}

/// Material ids from `content/materials.json` (the same rows as
/// `realgame_voxel::mat`, which this crate cannot depend on; the client's
/// `story` tests that they match), plus the Furnace, appended as row 27.
pub mod mat {
    pub const DIRT: u8 = 1;
    pub const STONE: u8 = 4;
    pub const WOOD: u8 = 7;
    pub const PLANKS: u8 = 9;
    pub const WATER: u8 = 12;
    pub const COAL_ORE: u8 = 15;
    pub const FURNACE: u8 = 27;
    /// Row names, in id order (`realgame_voxel::mat::NAMES` plus `furnace`).
    pub const NAMES: [&str; 28] = [
        "air",
        "dirt",
        "grass",
        "sand",
        "stone",
        "cliffstone",
        "runestone",
        "wood",
        "leaves",
        "planks",
        "snow",
        "ice",
        "water",
        "steam",
        "mud",
        "coal_ore",
        "iron_ore",
        "crystal",
        "glass",
        "iron",
        "brick",
        "ash",
        "torch",
        "bench",
        "invention_bench",
        "kiln_hearth",
        "gas_vent",
        "furnace",
    ];
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

    /// Feeds one event; returns the step just completed, if any. Only the
    /// event that finishes the current step counts, so nothing is skipped and
    /// nothing completes twice.
    pub fn on(&mut self, event: QuestEvent) -> Option<Step> {
        use QuestEvent as E;
        let done = match (self.step, event) {
            (0, E::IntroDone) => true,
            (1, E::WorkshopOut { by_water: true }) => true,
            (2, E::CrossedFord) => true,
            (3, E::GateMelted) => true,
            (4, E::Holding { item, total }) => item == Item::Block(mat::COAL_ORE) && total >= 1,
            (5, E::CrossedHollow) => true,
            (6, E::Holding { item, total }) => item == Item::KilnSpark && total >= 1,
            (7, E::Invented { relights: true }) => true,
            _ => false,
        };
        if !done {
            return None;
        }
        let finished = self.step;
        self.step += 1;
        Some(finished)
    }

    /// A quest already at `step` (the `quest <n>` debug command and
    /// `?biome=` seeding); clamped to the ending.
    pub fn at(step: Step) -> Quest {
        Quest {
            step: step.min(STEPS + 1),
        }
    }

    /// The current step (0-8).
    pub fn step(&self) -> Step {
        self.step
    }

    /// The objective line for the current step and band (`None`: no band
    /// chosen, Seedling text).
    pub fn line(&self, band: Option<Tier>) -> &'static str {
        let k = match band {
            Some(Tier::Sprout) => 0,
            Some(Tier::Seedling) | None => 1,
            Some(Tier::Apprentice) => 2,
            Some(Tier::Adept) => 3,
        };
        LINES.get(self.step as usize).map_or("", |l| l[k])
    }
}

/// The objective line per step (0 intro .. 8 ending) and band (Sprout,
/// Seedling, Apprentice, Adept). At most 90 characters (C6).
const LINES: [[&str; 4]; 9] = [
    [
        "Welcome to the valley!",
        "Welcome to the valley. The Great Kiln has gone cold.",
        "Welcome to the valley. The Great Kiln has gone cold.",
        "Welcome to the valley. The Great Kiln has gone cold.",
    ],
    [
        "Fire at the workshop! Learn Water at the Bench, then splash it.",
        "The workshop is on fire! Learn Water at the Formula Bench, then put it out.",
        "The workshop is burning. Learn Water at the Formula Bench and cool it out.",
        "The workshop is burning. Build H₂O at the Formula Bench and cool the wood below ignition.",
    ],
    [
        "The river is too deep. Learn Frost and make an ice path.",
        "The river is too deep. Learn Frost and freeze a path across.",
        "The river is too deep to wade. Learn Frost (N₂, −196 °C) and freeze a path.",
        "Too deep to wade. Learn Frost: liquid N₂ at −196 °C freezes the top layer first.",
    ],
    [
        "An ice gate! Melt it with Fire.",
        "An ice gate blocks the cliff. Melt it with Fire.",
        "An ice gate blocks the cliff. Melt it with Fire: H₂O(s) → H₂O(l).",
        "An ice gate blocks the cliff. Melt it with Fire; ice takes a lot of heat to melt.",
    ],
    [
        "Dig some coal with the Pick (key 5).",
        "Dig some coal from the seam (hold the Pick, key 5).",
        "Dig coal from the seam (Pick, key 5). Coal is mostly carbon, C.",
        "Mine coal from the seam (Pick, key 5): carbon fuel for torches and the Kiln.",
    ],
    [
        "Gas fire! Learn CO₂, block the gap, then fill the hollow.",
        "Marsh gas burns on the boardwalk. Learn CO₂, close the gap in the rim, smother it.",
        "Methane burns in the hollow. Learn CO₂, close the rim gap, let the CO₂ pool.",
        "CH₄ burns in the hollow. Learn CO₂, seal the rim gap; dense CO₂ pools and smothers.",
    ],
    [
        "It's dark! Make a torch, split water, lift the crate.",
        "It's dark. Make a torch, split water at the rig, and lift the crate.",
        "Make a torch, split water at the rig (2H₂O → 2H₂ + O₂), and lift the crate.",
        "Light a torch, run the rig (2H₂O → 2H₂ + O₂), and choose the gas that lifts the crate.",
    ],
    [
        "Take the Spark home and make the Kiln hot again!",
        "Bring the Kiln Spark home and invent a reaction hot enough to relight the Great Kiln.",
        "Bring the Spark home. Invent a reaction hot enough (1700 °C) for the Kiln.",
        "Bring the Spark home. Invent a reaction whose flame in pure O₂ tops 1700 °C.",
    ],
    [
        "You did it! The Kiln is warm again.",
        "You relit the Great Kiln!",
        "You relit the Great Kiln!",
        "You relit the Great Kiln!",
    ],
];

/// The intro's three cards per band (Sprout, Seedling, Apprentice, Adept),
/// over the cold Kiln (spec section 8). No band reads as Seedling.
pub const INTRO: [[&str; 3]; 4] = [
    [
        "The Great Kiln kept our valley warm.",
        "Last night it went cold!",
        "You are the apprentice. Learn magic chemistry and light the Kiln again!",
    ],
    [
        "The Great Kiln kept our valley warm and made all our glass.",
        "Last night it went cold.",
        "You are the apprentice. Learn real chemistry at the Formula Bench, explore the valley, and relight the Kiln.",
    ],
    [
        "The Great Kiln kept our valley warm and made all our glass.",
        "Last night it went cold.",
        "You are the apprentice. Learn real chemistry at the Formula Bench, explore the valley, and relight the Kiln. A flame hot enough to melt sand needs fuel and a lot of oxygen.",
    ],
    [
        "The Great Kiln kept our valley warm and made all our glass.",
        "Last night it went cold.",
        "You are the apprentice. Learn real chemistry at the Formula Bench, explore the valley, and relight the Kiln. Glass needs about 1700 °C: fuel burning in pure oxygen gets there.",
    ],
];

/// A biome's name for the objective title and the biome toast.
pub fn biome_name(id: &str) -> &'static str {
    match id {
        "village" => "The Village",
        "frozen" => "The Frozen Fields",
        "marsh" => "The Marsh",
        "cave" => "The Crystal Cave",
        "kiln" => "The Kiln Terrace",
        _ => "Formula",
    }
}

/// The intro cards for `band` (`None`: Seedling).
pub fn intro(band: Option<Tier>) -> [&'static str; 3] {
    let k = match band {
        Some(Tier::Sprout) => 0,
        Some(Tier::Seedling) | None => 1,
        Some(Tier::Apprentice) => 2,
        Some(Tier::Adept) => 3,
    };
    INTRO[k]
}

/// The Invention Bench (section 6): builds a reaction from 1-3 pack items and
/// asks `crate::reactions` whether it is real, exothermic and hot enough.
pub mod invent {
    use super::Item;
    use crate::bench::Code;
    use crate::formula::{parse_equation, side_atoms, Equation};
    use crate::reactions::ReactionBook;
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

    /// A fuel's flame in pure O2, °C: the same value as `grid::FLAME_C_IN_O2`
    /// (that module is behind the `game` feature). Carbon in O2 and the
    /// hydrogen-oxygen flame are both about 2800 °C.
    pub const FLAME_IN_O2_C: f32 = 2800.0;

    /// The Kiln needs a flame at least this hot to relight (spec section 6).
    pub const KILN_MIN_C: f32 = 1700.0;

    /// The species an item brings to the bench (spec section 6): coal and
    /// wood are C(s) (wood simplified to carbon, as `carbon_combustion`'s
    /// `real_world` line says), the flask O2, the balloon H2, water H2O.
    pub fn species(item: Item) -> Option<&'static str> {
        use super::mat;
        match item {
            Item::Block(mat::COAL_ORE | mat::WOOD | mat::PLANKS) => Some("C"),
            Item::Block(mat::WATER) => Some("H2O"),
            Item::O2Flask => Some("O2"),
            Item::H2Balloon => Some("H2"),
            _ => None,
        }
    }

    /// How hot the reaction's flame gets in pure O2, °C, or 0 when it is not
    /// a burning in oxygen. Carbon and hydrogen both burn at about 2800 °C in
    /// pure oxygen (`grid::FLAME_C_IN_O2`), well over the Kiln's 1700 °C.
    fn flame_c(eq: &Equation, delta_h_kj: f32) -> f32 {
        let burns_in_o2 = eq.reactants.iter().any(|t| t.formula == "O2");
        if burns_in_o2 && delta_h_kj < 0.0 {
            FLAME_IN_O2_C
        } else {
            0.0
        }
    }

    fn verdict(r: &crate::reactions::Reaction, eq: &Equation) -> Invention {
        let flame_c = flame_c(eq, r.delta_h_kj);
        let relights = flame_c >= KILN_MIN_C;
        let pretty = eq.to_string().replace("O2", "O₂").replace("H2", "H₂");
        let reason = if relights {
            format!(
                "{pretty} gives off {:.1} kJ. In pure oxygen it burns at about {:.0} °C: hot enough for the Kiln!",
                -r.delta_h_kj, flame_c
            )
        } else {
            format!("{pretty} is real, but it is not hot enough for the Kiln.")
        };
        Invention {
            ok: true,
            relights,
            reaction: Some(r.id.clone()),
            code: Code::Ok,
            reason,
            flame_c,
        }
    }

    fn fizzle(code: Code, reason: impl Into<String>) -> Invention {
        Invention {
            code,
            reason: reason.into(),
            ..Invention::default()
        }
    }

    /// Judges the items on the bench, or `text` when Apprentice or Adept
    /// typed an equation (then `items` is what the pack holds, and the typed
    /// reactants must be among them). A reaction counts when it is in
    /// `content/reactions.json`, gives off heat, and uses only what is on the
    /// bench.
    pub fn judge(items: &[Item], band: Tier, text: &str) -> Invention {
        let book = ReactionBook::load();
        let mut have: Vec<&'static str> = items.iter().filter_map(|&i| species(i)).collect();
        have.sort_unstable();
        have.dedup();
        let typed = text.trim();
        if matches!(band, Tier::Apprentice | Tier::Adept) && !typed.is_empty() {
            return judge_typed(&book, &have, typed);
        }
        if items.is_empty() {
            return fizzle(
                Code::Empty,
                "Put one to three things from your pack on the bench.",
            );
        }
        if let Some(odd) = items.iter().find(|&&i| species(i).is_none()) {
            return fizzle(
                Code::WrongSpecies,
                format!(
                    "The {} does not take part in a reaction here. Try coal, O₂ or H₂.",
                    odd.label()
                ),
            );
        }
        // The hottest exothermic reaction that uses only what is here.
        let found = book
            .reactions
            .iter()
            .filter(|(r, eq)| {
                r.delta_h_kj < 0.0
                    && eq
                        .reactants
                        .iter()
                        .all(|t| have.contains(&t.formula.as_str()))
            })
            .max_by(|a, b| {
                flame_c(&a.1, a.0.delta_h_kj)
                    .total_cmp(&flame_c(&b.1, b.0.delta_h_kj))
                    .then(b.0.delta_h_kj.total_cmp(&a.0.delta_h_kj))
            });
        if let Some((r, eq)) = found {
            return verdict(r, eq);
        }
        let fuel = have.iter().any(|s| matches!(*s, "C" | "H2"));
        let reason = if have == ["H2O"] {
            "Water is hydrogen that has already burnt, so there is nothing to burn."
        } else if fuel && !have.contains(&"O2") {
            if have.contains(&"C") {
                "Coal needs oxygen to burn. Air has only 21 %; the Kiln needs more."
            } else {
                "Hydrogen needs oxygen to burn. Air has only 21 %; the Kiln needs more."
            }
        } else if have.contains(&"O2") && !fuel {
            "Oxygen helps things burn, but it needs a fuel too. Try coal or hydrogen."
        } else {
            "These do not react to give off heat. Try a fuel with oxygen."
        };
        fizzle(Code::WrongSpecies, reason)
    }

    fn judge_typed(book: &ReactionBook, have: &[&str], text: &str) -> Invention {
        let eq = match parse_equation(text) {
            Ok(eq) => eq,
            Err(e) => return fizzle(Code::Parse, e.to_string()),
        };
        if !eq.has_arrow || eq.products.is_empty() {
            return fizzle(
                Code::NoProducts,
                "Write what it makes after an arrow, like C + O2 -> CO2.",
            );
        }
        if !eq.is_balanced() {
            let left = side_atoms(&eq.reactants);
            let right = side_atoms(&eq.products);
            let mut els: Vec<&String> = left.keys().chain(right.keys()).collect();
            els.sort();
            els.dedup();
            let diffs: Vec<String> = els
                .into_iter()
                .filter_map(|el| {
                    let l = left.get(el).copied().unwrap_or(0);
                    let r = right.get(el).copied().unwrap_or(0);
                    (l != r).then(|| format!("{el}: {l} before, {r} after"))
                })
                .collect();
            return fizzle(
                Code::Unbalanced,
                format!(
                    "Not balanced as written ({}). Count the atoms on each side.",
                    diffs.join("; ")
                ),
            );
        }
        let Some((r, known)) = book.find_by_species(&eq) else {
            let reason = match book.find_elsewhere(&eq) {
                Some(k) => format!("{} is real chemistry, but it will not relight the Kiln.", k.name),
                None => "That is balanced, but it is not a reaction the bench knows. Try a fuel burning in O2.".to_string(),
            };
            return fizzle(Code::WrongSpecies, reason);
        };
        if eq.simplest().0.species_key() != known.species_key()
            || coefs(&eq.simplest().0) != coefs(&known.simplest().0)
        {
            return fizzle(
                Code::WrongSpecies,
                "Those numbers do not match the real reaction. Check the coefficients.",
            );
        }
        if r.delta_h_kj >= 0.0 {
            return fizzle(
                Code::WrongSpecies,
                format!("{} takes in heat, so it would cool the Kiln.", r.name),
            );
        }
        if let Some(t) = known
            .reactants
            .iter()
            .find(|t| !have.contains(&t.formula.as_str()))
        {
            let what = t.formula.replace("O2", "O₂").replace("H2", "H₂");
            return fizzle(
                Code::WrongSpecies,
                format!("Your pack has no {what} for that. Find some first."),
            );
        }
        verdict(r, known)
    }

    fn coefs(eq: &Equation) -> Vec<(String, u32)> {
        let mut v: Vec<(String, u32)> = eq
            .reactants
            .iter()
            .chain(eq.products.iter())
            .map(|t| (t.formula.clone(), t.coef))
            .collect();
        v.sort();
        v
    }
}

/// HUD hint only (section 7): a spell is usable at stage >= Learning when the
/// band can learn it, and always when it cannot (Fire at Sprout), matching
/// the client's `worked::route`. `stage` uses the client's numbering (0
/// Unknown, 1 Learning, 2 Practised, 3 Mastered).
pub fn usable(def: &SpellDef, band: Tier, stage: u8) -> bool {
    !crate::bench::learnable(def, band) || stage >= 1
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
        /// it becomes Practised in this run.
        pub fn record(&mut self, spell: usize) -> bool {
            let Some(c) = self.casts.get_mut(spell) else {
                return false;
            };
            if *c >= PRACTISE_CASTS {
                return false;
            }
            *c += 1;
            *c == PRACTISE_CASTS
        }

        /// Practised in this run.
        pub fn practised(&self, spell: usize) -> bool {
            self.casts.get(spell).is_some_and(|&c| c >= PRACTISE_CASTS)
        }

        /// The stage the worked-cast route should use: the saved stage, raised
        /// to Practised (2) once practised in this run; never lowered, never
        /// raised to Mastered (3).
        pub fn stage(&self, spell: usize, saved: u8) -> u8 {
            if self.practised(spell) {
                saved.max(2)
            } else {
                saved
            }
        }
    }

    /// The spell a biome's gate needs Practised (the previous biome's spell),
    /// or `None` for an ungated biome: the Frozen Fields need Water (the
    /// Village's), the Marsh needs Frost, the Crystal Cave needs CO2.
    pub fn gate_spell(biome: &str) -> Option<usize> {
        match biome {
            "frozen" => Some(1),
            "marsh" => Some(2),
            "cave" => Some(3),
            _ => None,
        }
    }

    /// Whether the way into `biome` is open for this run.
    pub fn biome_open(biome: &str, run: &RunPractice) -> bool {
        gate_spell(biome).is_none_or(|s| run.practised(s))
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
        assert_eq!(
            p.slots
                .iter()
                .flatten()
                .filter(|s| s.item == Item::Pick)
                .count(),
            2
        );
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
        assert!(
            p.slots.iter().all(|s| s.is_none_or(|s| s.item != dirt)),
            "emptied slots are freed"
        );
        // JSON: one entry per filled slot.
        let mut q = Pack::default();
        q.add(Item::Block(mat::COAL_ORE), 2);
        q.add(Item::KilnSpark, 1);
        assert_eq!(
            q.to_json(),
            r#"[{"item":"coal_ore","n":2},{"item":"kiln_spark","n":1}]"#
        );
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
            Err(CraftError::NotEnough {
                item: coal,
                have: 0,
                need: 1
            })
        );
        assert_eq!(p, before, "a refused craft leaves the pack unchanged");
        p.add(coal, 1);
        assert_eq!(craft(&mut p, "torch"), Ok(()));
        assert_eq!(p.count(Item::Torch), 4);
        assert_eq!((p.count(wood), p.count(coal)), (0, 0));
        assert_eq!(craft(&mut p, "nonsense"), Err(CraftError::UnknownRecipe));
        // The furnace: 8 stone.
        let r = RECIPES
            .iter()
            .find(|r| r.id == "furnace")
            .expect("furnace recipe");
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
            E::Holding {
                item: coal,
                total: 1,
            },
            E::CrossedHollow,
            E::Holding {
                item: Item::KilnSpark,
                total: 1,
            },
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
        assert_eq!(
            q.on(E::Holding {
                item: coal,
                total: 0
            }),
            None
        );
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
        assert_eq!(
            q.on(E::Holding {
                item: Item::Block(mat::DIRT),
                total: 5
            }),
            None
        );
        assert_eq!(q.step(), 4);
    }

    /// C4: `{Coal, O2Flask}` -> `carbon_combustion`, relights; `{H2Balloon,
    /// O2Flask}` -> `water_synthesis`, relights; `{Coal}` -> fizzle "needs
    /// oxygen"; `{water}` -> fizzle "nothing to burn"; typed `C + O2 -> CO2` at
    /// Apprentice is accepted; typed `C + O2 -> CO` is refused with the
    /// existing `atoms_off` code and its reason (unbalanced as written; the
    /// reason must not call carbon monoxide impossible).
    #[test]
    fn invention_judges_real_reactions() {
        use crate::bench::Code;
        let coal = Item::Block(mat::COAL_ORE);
        let water = Item::Block(mat::WATER);
        for band in Tier::ALL {
            let v = invent::judge(&[coal, Item::O2Flask], band, "");
            assert!(v.ok && v.relights, "{band:?}: coal + O2 {v:?}");
            assert_eq!(v.reaction.as_deref(), Some("carbon_combustion"));
            assert!(v.flame_c >= invent::KILN_MIN_C, "{v:?}");
            let v = invent::judge(&[Item::H2Balloon, Item::O2Flask], band, "");
            assert!(v.ok && v.relights, "{band:?}: H2 + O2 {v:?}");
            assert_eq!(v.reaction.as_deref(), Some("water_synthesis"));
            // Order on the bench does not matter.
            let v = invent::judge(&[Item::O2Flask, coal], band, "");
            assert_eq!(v.reaction.as_deref(), Some("carbon_combustion"));
            // Wood is carbon too (simplified, as the reaction's real_world says).
            let v = invent::judge(&[Item::Block(mat::WOOD), Item::O2Flask], band, "");
            assert!(v.ok && v.relights, "{band:?}: wood + O2 {v:?}");
        }
        // Fizzles, each with its kind reason.
        let v = invent::judge(&[coal], Tier::Seedling, "");
        assert!(!v.ok && !v.relights);
        assert!(v.reason.contains("oxygen"), "{v:?}");
        let v = invent::judge(&[water], Tier::Seedling, "");
        assert!(!v.ok && !v.relights);
        assert!(v.reason.contains("nothing to burn"), "{v:?}");
        let v = invent::judge(&[Item::O2Flask], Tier::Seedling, "");
        assert!(!v.ok && v.reason.contains("fuel"), "{v:?}");
        let v = invent::judge(&[], Tier::Seedling, "");
        assert!(!v.ok && !v.reason.is_empty(), "{v:?}");
        let v = invent::judge(&[Item::Pick, Item::O2Flask], Tier::Seedling, "");
        assert!(!v.ok, "a Pick is not a reactant: {v:?}");
        // Apprentice types the equation: accepted when the pack holds the
        // reactants.
        let have = [coal, Item::O2Flask];
        let v = invent::judge(&have, Tier::Apprentice, "C + O2 -> CO2");
        assert!(v.ok && v.relights, "{v:?}");
        assert_eq!(v.code, Code::Ok);
        assert_eq!(v.reaction.as_deref(), Some("carbon_combustion"));
        let v = invent::judge(
            &[Item::H2Balloon, Item::O2Flask],
            Tier::Adept,
            "2H2 + O2 -> 2H2O",
        );
        assert!(v.ok && v.relights, "{v:?}");
        // Unbalanced as written: refused with the bench's code and diffs,
        // and the reason never calls carbon monoxide impossible.
        let v = invent::judge(&have, Tier::Apprentice, "C + O2 -> CO");
        assert!(!v.ok && !v.relights, "{v:?}");
        assert_eq!(v.code, Code::Unbalanced, "{v:?}");
        assert!(v.reason.contains("balanced"), "{v:?}");
        let low = v.reason.to_lowercase();
        assert!(
            !low.contains("impossible")
                && !low.contains("not real")
                && !low.contains("can't exist"),
            "{v:?}"
        );
        // Typed, balanced, but the pack lacks a reactant.
        let v = invent::judge(&[coal], Tier::Apprentice, "C + O2 -> CO2");
        assert!(!v.ok && v.reason.contains("O₂"), "{v:?}");
        // Typed text is ignored below Apprentice: the cards decide.
        let v = invent::judge(&have, Tier::Seedling, "C + O2 -> CO");
        assert!(v.ok, "{v:?}");
        // Typed nonsense is a parse refusal.
        let v = invent::judge(&have, Tier::Apprentice, "C + Qx -> ?");
        assert!(!v.ok && v.code != Code::Ok, "{v:?}");
        // Electrolysis takes in heat: never a Kiln reaction.
        let v = invent::judge(&[water], Tier::Adept, "2H2O -> 2H2 + O2");
        assert!(!v.ok && !v.relights, "{v:?}");
    }

    /// C5: Fire at Sprout is usable while unknown; Frost at Seedling is usable
    /// only from Learning.
    #[test]
    fn usable_follows_band() {
        let book = crate::spellbook::SpellBook::load(&crate::reactions::ReactionBook::load());
        let def = |id: &str| book.by_id(id).expect(id).clone();
        let (fire, frost) = (def("fire"), def("frost"));
        assert!(
            usable(&fire, Tier::Sprout, 0),
            "Fire at Sprout: usable while unknown"
        );
        assert!(
            !usable(&frost, Tier::Seedling, 0),
            "Frost at Seedling: not while unknown"
        );
        for stage in 1..=3 {
            assert!(
                usable(&frost, Tier::Seedling, stage),
                "Frost at Seedling, stage {stage}"
            );
        }
    }

    /// C6: every step has a line for all four bands and for no band, at most
    /// 90 characters.
    #[test]
    fn intro_has_three_cards_per_band_about_the_kiln() {
        for b in [
            None,
            Some(Tier::Sprout),
            Some(Tier::Seedling),
            Some(Tier::Apprentice),
            Some(Tier::Adept),
        ] {
            let cards = intro(b);
            assert!(
                cards
                    .iter()
                    .all(|c| !c.is_empty() && c.chars().count() <= 200),
                "{b:?}"
            );
            assert!(cards[0].contains("Kiln"), "{b:?}");
            assert!(!cards.iter().any(|c| c
                .chars()
                .any(|ch| ch == '"' || ch == '\\' || ch.is_control())));
        }
        assert_eq!(intro(None), intro(Some(Tier::Seedling)));
    }

    #[test]
    fn quest_lines_exist_per_band() {
        let bands = [
            None,
            Some(Tier::Sprout),
            Some(Tier::Seedling),
            Some(Tier::Apprentice),
            Some(Tier::Adept),
        ];
        for step in 1..=STEPS {
            let q = Quest::at(step);
            for b in bands {
                let line = q.line(b);
                assert!(!line.is_empty(), "step {step} {b:?}");
                assert!(
                    line.chars().count() <= 90,
                    "step {step} {b:?}: {} chars",
                    line.chars().count()
                );
                assert!(
                    !line
                        .chars()
                        .any(|c| c == '"' || c == '\\' || c.is_control()),
                    "{line:?}"
                );
            }
            assert_eq!(
                q.line(None),
                q.line(Some(Tier::Seedling)),
                "no band reads as Seedling"
            );
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
