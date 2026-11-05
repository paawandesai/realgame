//! Discoveries become spells (I-053, Bench plan Option B steps 3-6).
//!
//! A reaction found on the Chemistry Bench can be saved as a spell. This
//! module holds the pure rules, so the world and the Bench page agree:
//!
//! - **effects** (`content/chem/effects.json`): every codex reaction maps to
//!   one of a small set of spell effects, derived from its family, heat,
//!   hazard, products and show record by ordered rules. `kind` is the look (a
//!   data value the renderer styles), `runs` the world use (I-056: burns,
//!   splashes, freezes, smothers, builds a wall, reveals, lifts or pushes),
//!   with a `strength` that scales it. A reaction with no use that fits is
//!   **Bench only**: it cannot be saved as a spell, and `bench_only` says why
//!   in plain words;
//! - **bottles** (`content/chem/bottles.json`): which pack item supplies
//!   which substance in story mode, and what a cast needs and uses up;
//! - **chemistry quests** (`content/chem/quests.json`);
//! - **mastery**: a discovered spell is built [`MASTERY_BUILDS`] times
//!   (worked casts) before it casts instantly;
//! - [`spell_def`]: the spell-book entry the world's Formula Bench judge
//!   (`realgame_sim::bench::judge_with`) checks a build against.
//!
//! See `docs/loop/items/I-053/spec.md`.

use std::collections::{BTreeMap, BTreeSet};

use realgame_sim::formula::{parse_equation, Equation};
use realgame_sim::spellbook::{Formula, ReactionRef, RefKind, Role, SpellDef};
use realgame_sim::tier::Tier;
use serde::{Deserialize, Serialize};

use crate::show::{ShowRecord, Visible};
use crate::{Codex, Entry, Hazard, Heat};

pub const EFFECTS_JSON: &str = include_str!("../../../content/chem/effects.json");
pub const BOTTLES_JSON: &str = include_str!("../../../content/chem/bottles.json");
pub const QUESTS_JSON: &str = include_str!("../../../content/chem/quests.json");

/// The quest event the block world reports when ice or snow melts.
pub const EVENT_MELTED: &str = "melted";
/// A burning gas vent or block went out for lack of oxygen (I-057).
pub const EVENT_SMOTHERED: &str = "smothered";
/// Something cold landed in the hot vent of the charcoal clamp (I-057).
pub const EVENT_COOLED: &str = "cooled";
/// A wall of solid took the place of water in the river (I-057).
pub const EVENT_WALLED_WATER: &str = "walled_water";
/// A reveal showed something hidden: a cache or the hidden mark (I-057).
pub const EVENT_REVEALED: &str = "revealed";
/// Every quest event the world reports.
pub const EVENTS: [&str; 5] = [
    EVENT_MELTED,
    EVENT_SMOTHERED,
    EVENT_COOLED,
    EVENT_WALLED_WATER,
    EVENT_REVEALED,
];

/// Correct worked builds before a discovered spell casts instantly (the
/// mastery rule: build its equation twice).
pub const MASTERY_BUILDS: u8 = 2;

/// The look of a spell: what the renderer styles. Every value is a data
/// value; the renderer reuses the existing spawners until it styles a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind {
    Fire,
    Water,
    Frost,
    Foam,
    Cloud,
    Glow,
    /// A solid comes out of a solution: a short-lived wall (I-056).
    Wall,
    /// A colour appears or a metal coats: marks and reveals (I-056).
    Reveal,
}

impl EffectKind {
    pub const ALL: [EffectKind; 8] = [
        EffectKind::Fire,
        EffectKind::Water,
        EffectKind::Frost,
        EffectKind::Foam,
        EffectKind::Cloud,
        EffectKind::Glow,
        EffectKind::Wall,
        EffectKind::Reveal,
    ];

    pub fn id(self) -> &'static str {
        match self {
            EffectKind::Fire => "fire",
            EffectKind::Water => "water",
            EffectKind::Frost => "frost",
            EffectKind::Foam => "foam",
            EffectKind::Cloud => "cloud",
            EffectKind::Glow => "glow",
            EffectKind::Wall => "wall",
            EffectKind::Reveal => "reveal",
        }
    }
}

/// What a spell does in the world: one of the four shipped spells' matter
/// rules (fire, water, frost, foam), one of the world uses I-056 adds (a
/// wall, a reveal, a lift or a push), or `None`: Bench only, never cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runs {
    Fire,
    Water,
    Frost,
    Foam,
    /// A short-lived wall of blocks where it lands (a solid that forms).
    Wall,
    /// Hidden caches near where it lands show for a while (a colour that
    /// appears marks them).
    Reveal,
    /// Hydrogen rises: it lifts a player standing where it lands.
    Lift,
    /// A burst of gas pushes a player near where it lands.
    Push,
    /// Bench only: it cannot be saved as a spell.
    None,
}

impl Runs {
    pub const ALL: [Runs; 9] = [
        Runs::Fire,
        Runs::Water,
        Runs::Frost,
        Runs::Foam,
        Runs::Wall,
        Runs::Reveal,
        Runs::Lift,
        Runs::Push,
        Runs::None,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Runs::Fire => "fire",
            Runs::Water => "water",
            Runs::Frost => "frost",
            Runs::Foam => "foam",
            Runs::Wall => "wall",
            Runs::Reveal => "reveal",
            Runs::Lift => "lift",
            Runs::Push => "push",
            Runs::None => "none",
        }
    }

    /// What the spell does in the world, in plain words (the Bench's Save
    /// button and the pack say it).
    pub fn world(self) -> &'static str {
        match self {
            Runs::Fire => "burns, melts ice and lights the dark",
            Runs::Water => "splashes warm or plain water",
            Runs::Frost => "freezes water and cools hot things",
            Runs::Foam => "smothers a fire with carbon dioxide",
            Runs::Wall => "builds a short-lived wall or plugs a gap",
            Runs::Reveal => "shows hidden caches nearby for a while",
            Runs::Lift => "lifts you like a hydrogen balloon",
            Runs::Push => "pushes you with a burst of gas",
            Runs::None => "",
        }
    }

    /// Whether it lands as a matter emit (the four shipped spells' rules);
    /// the other uses are carried out by the world (`world_use`).
    pub fn matter(self) -> bool {
        matches!(self, Runs::Fire | Runs::Water | Runs::Frost | Runs::Foam)
    }

    /// The hotbar slot (1-4) whose shipped spell's flight it uses, where the
    /// discovered spell goes by default: a wall flies like Frost (a short
    /// spray that stops at the first block), a reveal like Fire, a lift like
    /// Water (whose power is the H2 lift) and a push like CO2 (the dash).
    pub fn slot(self) -> u8 {
        match self {
            Runs::Fire | Runs::Reveal => 1,
            Runs::Water | Runs::Lift => 2,
            Runs::Frost | Runs::Wall => 3,
            Runs::Foam | Runs::Push | Runs::None => 4,
        }
    }

    /// The scale of the rule's emit for a spell of `strength` (0 for Bench
    /// only, which never casts).
    pub fn amount(self, strength: f32) -> f32 {
        if self == Runs::None {
            0.0
        } else {
            strength.clamp(0.0, 1.0)
        }
    }
}

/// A codex reaction's spell effect: what the graphics thread styles and
/// what the world runs. Hit position and normal come from the cast.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Effect {
    /// The codex id (`content/chem/core.json`).
    pub reaction_id: String,
    pub kind: EffectKind,
    /// -1 absorbs heat, 0 little or unknown, +1 releases heat.
    pub heat: i8,
    /// 0 to 1: how strongly the effect acts (1 = the shipped spell).
    pub strength: f32,
    /// sRGB hex of the visible change (`#rrggbb`), when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,
    pub runs: Runs,
    /// What it does in the world, in plain words (empty for Bench only).
    #[serde(rename = "use")]
    pub world: String,
    /// Why it stays on the Bench, in plain words: set exactly when `runs`
    /// is `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bench_only: Option<String>,
}

impl Effect {
    /// Whether it can be saved as a spell (it has a world use).
    pub fn saveable(&self) -> bool {
        self.runs != Runs::None
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct When {
    #[serde(default)]
    family: Option<Vec<String>>,
    #[serde(default)]
    heat: Option<Vec<String>>,
    #[serde(default)]
    hazard: Option<Vec<String>>,
    #[serde(default)]
    gas_product: Option<bool>,
    #[serde(default)]
    needs_any: Option<bool>,
    /// The show record shows at least one of these.
    #[serde(default)]
    visible_any: Option<Vec<Visible>>,
    /// At least one product gas is one of these.
    #[serde(default)]
    gas_any: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KindRule {
    when: When,
    kind: EffectKind,
    #[allow(dead_code)]
    why: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrengthRule {
    when: When,
    strength: f32,
    #[allow(dead_code)]
    why: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Override {
    #[serde(default)]
    kind: Option<EffectKind>,
    #[serde(default)]
    strength: Option<f32>,
    #[allow(dead_code)]
    why: String,
}

/// A fizz whose gas does not smother a fire: what it does instead, by the
/// first rule that matches (I-056).
/// Products that keep a reaction on the Bench whatever its kind (I-056
/// review M1): poisonous gases and vapours.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Poisons {
    species: Vec<String>,
    /// The name of each as a gas or vapour (HCl is hydrogen chloride, not
    /// the acid a bottle holds); others use names.json.
    names: BTreeMap<String, String>,
    /// The reason; `{gas}` is the product's name.
    bench_only: String,
    #[allow(dead_code)]
    why: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GasUse {
    when: When,
    #[serde(default)]
    runs: Option<Runs>,
    /// Bench only, with this reason; `{gas}` is the matched gas's name.
    #[serde(default)]
    bench_only: Option<String>,
    #[allow(dead_code)]
    why: String,
}

/// The data values of the world uses I-056 adds; each scales with the
/// spell's strength between its two values.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WallSpec {
    /// The block it is built of (`realgame_voxel::mat::NAMES`).
    pub material: String,
    pub width: [u8; 2],
    pub height: [u8; 2],
    /// Seconds before it crumbles back to air.
    pub seconds: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevealSpec {
    /// Caches within this many metres of the landing show.
    pub radius_m: [f32; 2],
    pub seconds: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LiftSpec {
    /// A player within this many metres of the landing is lifted.
    pub radius_m: f32,
    pub blocks: [f32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PushSpec {
    pub radius_m: f32,
    /// m/s, away from the landing.
    pub speed: [f32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSpec {
    pub wall: WallSpec,
    pub reveal: RevealSpec,
    pub lift: LiftSpec,
    pub push: PushSpec,
}

/// `lo + (hi - lo) * strength`.
pub fn lerp(range: [f32; 2], strength: f32) -> f32 {
    range[0] + (range[1] - range[0]) * strength.clamp(0.0, 1.0)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectsFile {
    #[allow(dead_code)]
    version: u32,
    #[allow(dead_code)]
    about: String,
    kinds: Vec<EffectKind>,
    runs: BTreeMap<EffectKind, Runs>,
    smothers: Vec<String>,
    poisons: Poisons,
    gas_uses: Vec<GasUse>,
    world: WorldSpec,
    aqueous_families: Vec<String>,
    kind_rules: Vec<KindRule>,
    strength_rules: Vec<StrengthRule>,
    overrides: BTreeMap<String, Override>,
    #[allow(dead_code)]
    strength_scale: String,
    #[allow(dead_code)]
    tint_rule: String,
}

/// The effect rules, loaded once, with the reaction show records
/// (`crate::show`, I-054) that give each spell its tint and strength.
pub struct Effects {
    file: EffectsFile,
    gases: BTreeSet<String>,
    names: BTreeMap<String, String>,
    shows: crate::show::Shows,
}

fn heat_id(h: Option<Heat>) -> &'static str {
    match h {
        Some(Heat::Releases) => "releases",
        Some(Heat::Absorbs) => "absorbs",
        Some(Heat::Little) => "little",
        None => "none",
    }
}

fn hazard_id(h: Option<Hazard>) -> &'static str {
    match h {
        Some(Hazard::Fire) => "fire",
        Some(Hazard::Lab) => "lab",
        None => "none",
    }
}

fn eq_of(e: &Entry) -> Equation {
    parse_equation(&e.equation).expect("codex equations parse")
}

impl Effects {
    pub fn load() -> Effects {
        Effects::from_json(EFFECTS_JSON).expect("content/chem/effects.json is valid")
    }

    pub fn from_json(json: &str) -> Result<Effects, String> {
        let file: EffectsFile = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if file.kinds != EffectKind::ALL {
            return Err("effects.json: kinds must list the eight effect kinds in order".into());
        }
        let names: serde_json::Value =
            serde_json::from_str(crate::NAMES_JSON).map_err(|e| e.to_string())?;
        let gases = names["gases"]
            .as_array()
            .ok_or("names.json: no gases")?
            .iter()
            .filter_map(|g| g.as_str().map(str::to_string))
            .collect();
        let names = names["substances"]
            .as_object()
            .ok_or("names.json: no substances")?
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
            .collect();
        Ok(Effects {
            file,
            gases,
            names,
            shows: crate::show::Shows::load(),
        })
    }

    /// The world uses' data values (wall, reveal, lift, push).
    pub fn world(&self) -> &WorldSpec {
        &self.file.world
    }

    /// The show records the effects are built from.
    pub fn shows(&self) -> &crate::show::Shows {
        &self.shows
    }

    fn matches(&self, w: &When, e: &Entry, eq: &Equation, rec: Option<&ShowRecord>) -> bool {
        let list =
            |l: &Option<Vec<String>>, v: &str| l.as_ref().is_none_or(|l| l.iter().any(|x| x == v));
        list(&w.family, &e.family)
            && list(&w.heat, heat_id(e.heat))
            && list(&w.hazard, hazard_id(e.hazard))
            && w.gas_product.is_none_or(|g| g == self.gas_product(eq))
            && w.needs_any.is_none_or(|n| n == !e.needs.is_empty())
            && w.visible_any
                .as_ref()
                .is_none_or(|v| rec.is_some_and(|r| v.iter().any(|x| r.has(*x))))
            && w.gas_any
                .as_ref()
                .is_none_or(|g| self.product_gases(eq).any(|f| g.contains(&f.to_string())))
    }

    fn product_gases<'a>(&'a self, eq: &'a Equation) -> impl Iterator<Item = &'a str> + 'a {
        eq.products
            .iter()
            .map(|t| t.formula.as_str())
            .filter(|f| self.gases.contains(*f))
    }

    fn gas_product(&self, eq: &Equation) -> bool {
        eq.products.iter().any(|t| self.gases.contains(&t.formula))
    }

    /// The effect of codex entry `e`.
    pub fn of(&self, e: &Entry) -> Effect {
        let eq = eq_of(e);
        let over = self.file.overrides.get(&e.id);
        let show = self.shows.get(&e.id);
        let kind = over.and_then(|o| o.kind).unwrap_or_else(|| {
            self.file
                .kind_rules
                .iter()
                .find(|r| self.matches(&r.when, e, &eq, show))
                .map_or(EffectKind::Water, |r| r.kind)
        });
        // Strength comes from |ΔH| on the documented scale
        // (`show::strength_of`, I-054 spec section 6); with no ΔH, from the
        // heat-class rules.
        let strength =
            over.and_then(|o| o.strength)
                .unwrap_or_else(|| match show.and_then(|r| r.dh.kj_per) {
                    Some(kj) => {
                        let driven_cold = e.heat == Some(Heat::Absorbs) && !e.needs.is_empty();
                        crate::show::strength_of(kj, driven_cold)
                    }
                    None => self
                        .file
                        .strength_rules
                        .iter()
                        .find(|r| self.matches(&r.when, e, &eq, show))
                        .map_or(0.4, |r| r.strength),
                });
        let mut runs = self.file.runs.get(&kind).copied().unwrap_or(Runs::None);
        let mut bench_only = None;
        // A fizz smothers a fire only when every gas it gives is one that
        // neither burns nor feeds a flame (CO2, N2); H2 or O2 would not.
        // Any other fizz takes the first gas use that fits (I-056): a flame
        // burns, a poisonous gas stays on the Bench, hydrogen lifts, any
        // other gas pushes.
        if kind == EffectKind::Foam {
            let all_smother = self
                .product_gases(&eq)
                .all(|f| self.file.smothers.iter().any(|s| s == f));
            if !all_smother {
                let rule = self
                    .file
                    .gas_uses
                    .iter()
                    .find(|r| self.matches(&r.when, e, &eq, show));
                runs = rule.and_then(|r| r.runs).unwrap_or(Runs::None);
                if let Some(why) = rule.and_then(|r| r.bench_only.as_ref()) {
                    let gas = rule
                        .and_then(|r| r.when.gas_any.as_ref())
                        .and_then(|g| self.product_gases(&eq).find(|f| g.iter().any(|x| x == f)))
                        .map_or_else(String::new, |f| {
                            self.names.get(f).cloned().unwrap_or_else(|| f.to_string())
                        });
                    bench_only = Some(why.replace("{gas}", &gas));
                }
            }
        }
        // Heat given off in a solution warms a liquid: it splashes like
        // water rather than burning.
        if kind == EffectKind::Glow && self.file.aqueous_families.contains(&e.family) {
            runs = Runs::Water;
        }
        // A reaction that makes a poisonous gas or vapour stays on the Bench,
        // whatever it looks like (I-056 review M1).
        if let Some(f) = eq
            .products
            .iter()
            .map(|t| t.formula.as_str())
            .find(|f| self.file.poisons.species.iter().any(|x| x == f))
        {
            let name = self
                .file
                .poisons
                .names
                .get(f)
                .or_else(|| self.names.get(f))
                .cloned()
                .unwrap_or_else(|| f.to_string());
            runs = Runs::None;
            bench_only = Some(self.file.poisons.bench_only.replace("{gas}", &name));
        }
        let heat = match e.heat {
            Some(Heat::Releases) => 1,
            Some(Heat::Absorbs) => -1,
            Some(Heat::Little) | None => 0,
        };
        // The tint is the show record's: a fire's flame colour, otherwise the
        // first coloured product (precipitate, gas or solution).
        let tint = show.and_then(|r| r.tint(runs == Runs::Fire));
        if runs == Runs::None && bench_only.is_none() {
            bench_only = Some("Nothing it makes does anything out in the world yet.".into());
        }
        Effect {
            reaction_id: e.id.clone(),
            kind,
            heat,
            strength,
            tint,
            runs,
            world: runs.world().to_string(),
            bench_only,
        }
    }

    /// Every codex reaction's effect, in codex order.
    pub fn all(&self, codex: &Codex) -> Vec<Effect> {
        codex.entries.iter().map(|e| self.of(e)).collect()
    }

    /// The sources of every colour a tint can come from (shown on the
    /// Bench page's credits).
    pub fn tint_sources(&self) -> Vec<String> {
        ["COLOUR-GE", "COLOUR-FLAME", "COLOUR-HC"]
            .iter()
            .filter_map(|k| self.shows.sources.get(*k).cloned())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Bottles (story mode)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BottleItem {
    /// The pack item's key (`realgame_sim::valley::Item::key`).
    pub item: String,
    /// The codex formula it supplies.
    pub species: String,
    /// One is used up per cast.
    pub consumed: bool,
    pub why: String,
    /// Where the player gets this item, in plain words for the youngest band
    /// (the missing-bottle caption and the pack show it).
    pub find: String,
    /// How the world gives it (I-057): never by digging.
    pub from: Source,
}

/// How the world gives a bottle item (I-057). Mining and digging were
/// dropped (Paawan); these are the ways left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// A bottle stash (`Bottles::stashes`): walk up to it.
    Stash,
    /// The charcoal clamp (step 4).
    Clamp,
    /// The electrolysis rig in the cave.
    Rig,
    /// The Pick scoops it up from a block on the surface.
    Pick,
    /// Not given anywhere in this valley (an alternative only).
    Nowhere,
}

/// A place in the valley where bottle items can be picked up (I-057): walk
/// up to it and they go into the pack; it fills again after
/// `Bottles::refill_s`. A hidden stash shows only once a Reveal spell lands
/// near it.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Stash {
    pub id: String,
    /// Where it is, in plain words.
    pub place: String,
    /// The feet cell it stands on (x, y, z): a marker block goes there.
    pub at: [i32; 3],
    /// The existing block that marks it (a material name).
    pub block: String,
    #[serde(default)]
    pub hidden: bool,
    /// Pack item keys and how many of each one visit gives.
    pub items: Vec<(String, u16)>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bottles {
    pub version: u32,
    pub about: String,
    /// Substances the air supplies free.
    pub air: Vec<String>,
    pub items: Vec<BottleItem>,
    /// Where bottle items are found (I-057).
    pub stashes: Vec<Stash>,
    /// Seconds before a stash fills again after a visit.
    pub refill_s: f32,
}

/// What a cast of a discovered spell takes from the pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Pack items to take one of (consumed bottles only).
    pub take: Vec<String>,
    /// Reactants that no held item supplies (empty: the cast can go).
    pub missing: Vec<String>,
}

impl Bottles {
    pub fn load() -> Bottles {
        serde_json::from_str(BOTTLES_JSON).expect("content/chem/bottles.json is valid")
    }

    /// The bottle a pack item is, if any.
    pub fn of_item(&self, item: &str) -> Option<&BottleItem> {
        self.items.iter().find(|b| b.item == item)
    }

    /// What casting reaction `e` needs, given `held(item_key)` (how many of
    /// that item the pack holds): for each reactant the air does not supply,
    /// the first held item that supplies it.
    pub fn plan(&self, e: &Entry, held: &dyn Fn(&str) -> u32) -> Plan {
        let eq = eq_of(e);
        let mut species: Vec<&str> = eq.reactants.iter().map(|t| t.formula.as_str()).collect();
        species.dedup();
        let mut plan = Plan {
            take: Vec::new(),
            missing: Vec::new(),
        };
        for s in species {
            if self.air.iter().any(|a| a == s) {
                continue;
            }
            match self
                .items
                .iter()
                .find(|b| b.species == s && held(&b.item) > 0)
            {
                Some(b) if b.consumed => plan.take.push(b.item.clone()),
                Some(_) => {}
                None => plan.missing.push(s.to_string()),
            }
        }
        plan
    }

    /// Whether the world can ever supply every reactant of `e` (air plus
    /// some bottle item).
    pub fn suppliable(&self, e: &Entry) -> bool {
        self.plan(e, &|_| 1).missing.is_empty()
    }

    /// Whether a player can find bottle item `item` in story mode: the
    /// clamp, the rig or the Pick give it, or a stash that is not hidden
    /// holds it (I-057).
    pub fn findable(&self, item: &str) -> bool {
        match self.of_item(item).map(|b| b.from) {
            Some(Source::Clamp | Source::Rig | Source::Pick) => true,
            Some(Source::Stash) => self
                .stashes
                .iter()
                .any(|s| !s.hidden && s.items.iter().any(|(k, n)| k == item && *n > 0)),
            Some(Source::Nowhere) | None => false,
        }
    }

    /// Whether every reactant of `e` comes from the air or from a bottle
    /// item the player can find (I-057).
    pub fn findable_plan(&self, e: &Entry) -> bool {
        self.plan(e, &|k| u32::from(self.findable(k)))
            .missing
            .is_empty()
    }
}

// ---------------------------------------------------------------------------
// Chemistry quests
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuestNeeds {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heat: Option<i8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runs: Option<Runs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<EffectKind>,
}

impl QuestNeeds {
    /// Whether a discovered spell with effect `fx` meets these needs.
    pub fn met_by(&self, fx: &Effect) -> bool {
        self.heat.is_none_or(|h| h == fx.heat)
            && self.runs.is_none_or(|r| r == fx.runs)
            && self.kind.is_none_or(|k| k == fx.kind)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChemQuest {
    pub id: String,
    /// Checked in the block world now (the others are specified).
    pub live: bool,
    pub title: String,
    pub needs: QuestNeeds,
    /// The world event that finishes it near the landing.
    pub event: String,
    pub radius_m: f32,
    /// How long (s) after a matching spell lands the event still counts.
    #[serde(default = "default_window")]
    pub window_s: f32,
    /// Where it happens, in plain words (I-057).
    #[serde(default)]
    pub place: String,
    /// The event must happen within `near_m` of `at` (x, y, z), when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub near_m: Option<f32>,
    /// The objective line (Seedling and up) and Sprout's.
    #[serde(default)]
    pub line: String,
    #[serde(default)]
    pub line_sprout: String,
    /// Names the kind of reaction it needs, never the answer.
    #[serde(default)]
    pub hint: String,
    #[serde(default)]
    pub hint_sprout: String,
    /// Said when it is done.
    #[serde(default)]
    pub done: String,
    /// Example reactions that solve it (checked by the tests; the spec's
    /// quest table and the Guide's links use them).
    #[serde(default)]
    pub solved_by: Vec<String>,
}

fn default_window() -> f32 {
    2.5
}

impl ChemQuest {
    /// Whether an event at `p` is in the quest's place.
    pub fn in_place(&self, p: [f32; 3]) -> bool {
        match (self.at, self.near_m) {
            (Some(a), Some(r)) => {
                let d = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
                (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() <= r
            }
            _ => true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Quests {
    pub version: u32,
    pub about: String,
    pub quests: Vec<ChemQuest>,
}

impl Quests {
    pub fn load() -> Quests {
        serde_json::from_str(QUESTS_JSON).expect("content/chem/quests.json is valid")
    }
}

// ---------------------------------------------------------------------------
// Mastery and the spell-book entry
// ---------------------------------------------------------------------------

/// The lowest band that builds a discovered spell: the reaction's own band,
/// and never Sprout, whose builder makes single substances only (a Sprout
/// casts its discoveries without building, as Fire at Sprout does).
pub fn build_band(e: &Entry) -> Tier {
    e.band.max(Tier::Seedling)
}

/// Whether a discovered spell casts instantly: built `builds` times, or a
/// band that does not build it (below [`build_band`]).
pub fn instant(e: &Entry, band: Tier, builds: u8) -> bool {
    builds >= MASTERY_BUILDS || band < build_band(e)
}

/// Whether `band` may use the reaction at all (the codex band).
pub fn usable(e: &Entry, band: Tier) -> bool {
    e.band <= band
}

/// The spell-book entry for a discovered spell, so the world's Formula
/// Bench judge checks a build against the codex reaction. Its slot is the
/// slot of the shipped spell whose rule it runs.
pub fn spell_def(e: &Entry, fx: &Effect) -> SpellDef {
    let pretty = e.equation.replace(" -> ", " → ");
    SpellDef {
        id: e.id.clone(),
        slot: fx.runs.slot(),
        effect: fx.runs.id().to_string(),
        name: e.name.clone(),
        label: pretty.clone(),
        hotbar_label: e.name.clone(),
        cast_caption: e.short.clone().unwrap_or_else(|| e.say.clone()),
        formula: Formula::parse(&e.equation).expect("codex equations parse"),
        tier_min: build_band(e),
        reaction: ReactionRef {
            kind: RefKind::Reaction,
            id: e.id.clone(),
            role: Role::Runs,
        },
        prompt_sprout: None,
        prompt_seedling: Some(format!(
            "{}: change the numbers until the scale is level.",
            e.name
        )),
        prompt_apprentice: Some(format!("{}: write the balanced equation.", e.name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_sim::bench::{judge_with, Code};

    fn fx(id: &str) -> Effect {
        let c = Codex::load();
        Effects::load().of(c.get(id).unwrap())
    }

    /// The graphics contract: every codex reaction has an effect with a
    /// kind, heat, strength in 0..1 and its reaction id.
    #[test]
    fn every_codex_reaction_maps_to_an_effect() {
        let c = Codex::load();
        let fx = Effects::load();
        let all = fx.all(&c);
        assert_eq!(all.len(), c.entries.len());
        for (e, f) in c.entries.iter().zip(&all) {
            assert_eq!(f.reaction_id, e.id);
            assert!(EffectKind::ALL.contains(&f.kind), "{}", e.id);
            assert!([-1, 0, 1].contains(&f.heat), "{}", e.id);
            assert!(
                (0.0..=1.0).contains(&f.strength) && f.strength > 0.0,
                "{}: strength {}",
                e.id,
                f.strength
            );
            if let Some(t) = &f.tint {
                assert!(
                    t.len() == 7
                        && t.starts_with('#')
                        && t[1..].chars().all(|c| c.is_ascii_hexdigit()),
                    "{}: tint {t}",
                    e.id
                );
            }
            // Heat agrees with the codex.
            match e.heat {
                Some(Heat::Releases) => assert_eq!(f.heat, 1, "{}", e.id),
                Some(Heat::Absorbs) => assert_eq!(f.heat, -1, "{}", e.id),
                _ => assert_eq!(f.heat, 0, "{}", e.id),
            }
        }
        // The serialized value carries the four contract fields.
        let v = serde_json::to_value(&all[0]).unwrap();
        for k in ["reaction_id", "kind", "heat", "strength"] {
            assert!(v.get(k).is_some(), "{k}");
        }
    }

    #[test]
    fn the_effect_data_is_consistent() {
        let c = Codex::load();
        let f = Effects::load();
        assert!(Effects::from_json(&EFFECTS_JSON.replace("\"reveal\"]", "\"cloud\"]")).is_err());
        for k in EffectKind::ALL {
            assert!(f.file.runs.contains_key(&k), "{k:?} has no rule");
        }
        for id in f.file.overrides.keys() {
            assert!(c.get(id).is_some(), "override {id} is not a codex reaction");
        }
        for fam in &f.file.aqueous_families {
            assert!(c.families.contains(fam), "{fam}");
        }
        for s in &f.file.smothers {
            assert!(c.known_species.contains(s), "{s} is not a codex substance");
        }
        assert!(!f.tint_sources().is_empty());
        // Every tint is a sourced colour of the reaction's show record.
        for e in &c.entries {
            let fx = f.of(e);
            let rec = f.shows.get(&e.id).unwrap();
            if let Some(t) = &fx.tint {
                let flame = rec.flame.as_ref().and_then(|x| x.light.hex.as_ref());
                let coloured = rec
                    .species
                    .iter()
                    .filter_map(|s| s.colour.as_ref())
                    .any(|c| c.hex.as_ref() == Some(t) && c.hex_estimated);
                assert!(
                    flame == Some(t) || coloured,
                    "{}: tint {t} has no source",
                    e.id
                );
            }
        }
        // The last rule of each list matches everything, so nothing falls through.
        assert!(f.file.kind_rules.last().unwrap().when.family.is_none());
        assert!(f.file.strength_rules.last().unwrap().when.heat.is_none());
    }

    #[test]
    fn effects_follow_the_chemistry() {
        let m = fx("methane_burns");
        assert_eq!(
            (m.kind, m.runs, m.heat, m.strength),
            (EffectKind::Fire, Runs::Fire, 1, 1.0)
        );
        let c = fx("carbon_burns");
        assert_eq!((c.kind, c.runs), (EffectKind::Fire, Runs::Fire));
        // CO2 from chalk and acid smothers: it runs the CO2 rule.
        let chalk = fx("chalk_acid");
        assert_eq!((chalk.kind, chalk.runs), (EffectKind::Foam, Runs::Foam));
        // Hydrogen does not smother a fire: it lifts (I-056).
        let zinc = fx("zinc_acid");
        assert_eq!((zinc.kind, zinc.runs), (EffectKind::Foam, Runs::Lift));
        assert_eq!(fx("silver_chloride").kind, EffectKind::Wall);
        assert_eq!(fx("silver_chloride").runs, Runs::Wall);
        assert_eq!(fx("silver_chloride").tint.as_deref(), Some("#f4f4f4"));
        assert_eq!(fx("golden_rain").tint.as_deref(), Some("#ffd21f"));
        let mg = fx("magnesium_burns");
        assert_eq!(mg.tint.as_deref(), Some("#f5f7ff"));
        assert_eq!(fx("baking_soda_vinegar").kind, EffectKind::Frost);
        assert_eq!(fx("baking_soda_vinegar").heat, -1);
        // A warm neutralisation splashes, it does not burn.
        let n = fx("neutralise_hcl_naoh");
        assert_eq!((n.kind, n.runs), (EffectKind::Glow, Runs::Water));
        assert_eq!(n.tint, None);
        assert_eq!(fx("thermite").kind, EffectKind::Fire);
        assert_eq!(fx("thermite").strength, 1.0);
        // Driven reactions take in less heat than ones that feel cold.
        assert!(fx("limestone_roast").strength < fx("baking_soda_vinegar").strength);
        // A metal coating a metal in a solution marks it: a reveal.
        assert_eq!(fx("iron_copper_sulfate").runs, Runs::Reveal);
        // Oxygen from peroxide pushes; a poisonous gas stays on the Bench.
        assert_eq!(fx("peroxide_splits").runs, Runs::Push);
        let no2 = fx("no_oxidises");
        assert_eq!(no2.runs, Runs::None);
        assert_eq!(
            no2.bench_only.as_deref(),
            Some("It makes nitrogen dioxide, which is poisonous to breathe, so it stays on the Bench.")
        );
        // Potassium on water lights its hydrogen: it burns.
        assert_eq!(fx("potassium_water").runs, Runs::Fire);
        assert_eq!(Runs::None.amount(0.5), 0.0);
        assert_eq!(Runs::Fire.amount(1.5), 1.0);
    }

    /// I-056: every reaction either has a world use (and can be saved as a
    /// spell) or is Bench only with a reason; no spell is a dud puff.
    /// M1: a reaction that makes a poisonous gas or vapour stays on the
    /// Bench whatever its kind (ice, fire or reveal), mercury and ozone too.
    #[test]
    fn every_poisonous_product_keeps_a_reaction_on_the_bench() {
        let c = Codex::load();
        let f = Effects::load();
        for e in &c.entries {
            let eq = eq_of(e);
            let x = f.of(e);
            let poison = eq
                .products
                .iter()
                .any(|t| f.file.poisons.species.contains(&t.formula));
            assert_eq!(!x.saveable(), poison, "{}: {:?}", e.id, x.runs);
        }
        for id in [
            "ammonium_chloride_heat",
            "salt_electrolysis",
            "water_gas",
            "sulfur_burns",
            "carbon_little_air",
            "chlorine_bromide",
            "mercury_oxide_heat",
            "ozone_forms",
            "copper_nitrate_heat",
        ] {
            let x = f.of(c.get(id).unwrap());
            assert_eq!(x.runs, Runs::None, "{id}");
            assert!(x.bench_only.is_some(), "{id}");
        }
        assert!(f
            .of(c.get("mercury_oxide_heat").unwrap())
            .bench_only
            .unwrap()
            .contains("mercury vapour"));
    }

    /// M2: a Bench-only reason names the gas, not the acid a bottle holds.
    #[test]
    fn bench_only_reasons_name_the_gas_not_the_acid() {
        let c = Codex::load();
        let f = Effects::load();
        for e in &c.entries {
            if let Some(why) = f.of(e).bench_only {
                assert!(!why.contains("acid"), "{}: {why}", e.id);
            }
        }
        let hcl = f.of(c.get("hcl_synthesis").unwrap()).bench_only.unwrap();
        assert!(hcl.contains("hydrogen chloride"), "{hcl}");
        let hbr = f.of(c.get("hbr_synthesis").unwrap()).bench_only.unwrap();
        assert!(hbr.contains("hydrogen bromide"), "{hbr}");
    }

    /// M5: an ice spell only from a reaction that takes in heat by itself;
    /// one driven by heat, light or electricity never chills.
    #[test]
    fn frost_only_from_reactions_that_feel_cold_by_themselves() {
        let c = Codex::load();
        let f = Effects::load();
        for e in &c.entries {
            if f.of(e).kind == EffectKind::Frost {
                assert!(e.needs.is_empty(), "{}: driven by {:?}", e.id, e.needs);
            }
        }
        for id in ["limestone_roast", "glass_making", "cement_clinker"] {
            assert_ne!(f.of(c.get(id).unwrap()).kind, EffectKind::Frost, "{id}");
        }
        assert_eq!(
            f.of(c.get("baking_soda_vinegar").unwrap()).kind,
            EffectKind::Frost
        );
    }

    #[test]
    fn every_saveable_spell_has_a_world_use_and_bench_only_ones_say_why() {
        let c = Codex::load();
        let f = Effects::load();
        let mut uses: BTreeMap<&str, usize> = BTreeMap::new();
        for e in &c.entries {
            let x = f.of(e);
            *uses.entry(x.runs.id()).or_default() += 1;
            if x.saveable() {
                assert!(!x.world.is_empty(), "{}: no world use", e.id);
                assert!(x.bench_only.is_none(), "{}", e.id);
                assert!(x.runs.amount(x.strength) > 0.0, "{}: a dud", e.id);
            } else {
                let why = x
                    .bench_only
                    .as_deref()
                    .unwrap_or_else(|| panic!("{}: Bench only with no reason", e.id));
                assert!(
                    why.ends_with('.') && why.len() <= 120 && !why.contains('{'),
                    "{}: {why}",
                    e.id
                );
                assert!(x.world.is_empty(), "{}", e.id);
            }
            // The world use follows the show record (the plan's table).
            let rec = f.shows().get(&e.id).unwrap();
            match x.runs {
                Runs::Wall => assert!(rec.has(Visible::Precipitate), "{}", e.id),
                Runs::Reveal => assert!(
                    rec.has(Visible::Coating) || rec.has(Visible::ColourChange),
                    "{}",
                    e.id
                ),
                Runs::Lift => assert!(
                    rec.gases.iter().any(|g| g.formula == "H2"
                        && g.drift == crate::show::Drift::Rises
                        && g.side == crate::show::Side::Product),
                    "{}",
                    e.id
                ),
                Runs::Push => assert!(!rec.gases.is_empty(), "{}", e.id),
                Runs::Frost => assert_eq!(x.heat, -1, "{}", e.id),
                _ => {}
            }
        }
        // Every use is used, and the rule tier's precipitates build walls.
        for r in Runs::ALL {
            assert!(uses.get(r.id()).copied().unwrap_or(0) > 0, "{:?} unused", r);
        }
        for e in c
            .entries
            .iter()
            .filter(|e| e.rule.as_deref() == Some("precipitate"))
        {
            assert_eq!(f.of(e).runs, Runs::Wall, "{}", e.id);
        }
        // The new uses' data values are sane.
        let w = f.world();
        assert!(w.wall.width[0] >= 1 && w.wall.width[0] <= w.wall.width[1]);
        assert!(w.wall.height[0] >= 1 && w.wall.height[0] <= w.wall.height[1]);
        assert!(w.wall.seconds > 0.0 && w.reveal.seconds > 0.0);
        assert!(w.reveal.radius_m[0] > 0.0 && w.reveal.radius_m[0] <= w.reveal.radius_m[1]);
        assert!(w.lift.radius_m > 0.0 && w.push.radius_m > 0.0);
        assert_eq!(lerp([2.0, 4.0], 0.5), 3.0);
        assert_eq!(lerp([2.0, 4.0], 7.0), 4.0);
    }

    #[test]
    fn bottles_name_codex_substances_and_supply_carbon_burning() {
        let c = Codex::load();
        let b = Bottles::load();
        for i in &b.items {
            assert!(c.known_species.contains(&i.species), "{}", i.species);
            // Every bottle says where to get it, in short plain sentences.
            assert!(
                !i.find.is_empty() && i.find.len() <= 140 && i.find.ends_with('.'),
                "{}: {:?}",
                i.item,
                i.find
            );
            assert!(
                realgame_sim::valley::Item::from_key(&i.item).is_some(),
                "{} is not a pack item",
                i.item
            );
        }
        let carbon = c.get("carbon_burns").unwrap();
        let none = b.plan(carbon, &|_| 0);
        assert_eq!(none.missing, vec!["C".to_string()]);
        // Charcoal (the story's carbon, from the clamp) is offered first.
        assert_eq!(
            b.items
                .iter()
                .find(|i| i.species == "C")
                .map(|i| i.item.as_str()),
            Some("charcoal")
        );
        let coal = b.plan(carbon, &|k| u32::from(k == "coal_ore"));
        assert!(coal.missing.is_empty());
        assert_eq!(coal.take, vec!["coal_ore".to_string()]);
        // Hydrogen from the balloon is not used up.
        let h = b.plan(c.get("hydrogen_burns").unwrap(), &|k| {
            u32::from(k == "h2_balloon")
        });
        assert!(h.missing.is_empty() && h.take.is_empty());
        // Never by digging: every stash item is a bottle item from a stash,
        // and stashes are on real cells with something in them.
        for st in &b.stashes {
            assert!(!st.items.is_empty() && !st.place.is_empty(), "{}", st.id);
            for (k, n) in &st.items {
                let item = b
                    .of_item(k)
                    .unwrap_or_else(|| panic!("{}: {k} is no bottle", st.id));
                assert_eq!(item.from, Source::Stash, "{k} is in a stash");
                assert!(*n > 0);
            }
            assert!(st.at[1] > 0, "{}", st.id);
        }
        assert!(b.refill_s > 0.0);
        // The world supplies at least one Sprout fire-type and the quest's needs.
        let quests = Quests::load();
        let live: Vec<_> = quests.quests.iter().filter(|q| q.live).collect();
        assert!(!live.is_empty());
        let fx = Effects::load();
        for q in live {
            assert!(
                c.entries.iter().any(|e| e.band == Tier::Sprout
                    && b.suppliable(e)
                    && q.needs.met_by(&fx.of(e))),
                "{}: no Sprout reaction the world supplies meets it",
                q.id
            );
        }
    }

    /// I-057: one live quest per world use, each solvable with bottles the
    /// player can find (no debug `give`, no digging), its examples real.
    #[test]
    fn every_quest_is_solvable_from_findable_bottles() {
        let c = Codex::load();
        let b = Bottles::load();
        let fx = Effects::load();
        let quests = Quests::load();
        // Every bottle item says how the world gives it; a stash item is in
        // a stash anyone can see.
        for i in &b.items {
            if i.from == Source::Stash {
                assert!(b.findable(&i.item), "{} is in no visible stash", i.item);
            }
        }
        assert!(b.findable("charcoal") && b.findable("vinegar") && !b.findable("coal_ore"));
        let live: Vec<_> = quests.quests.iter().filter(|q| q.live).collect();
        // One per world use: melt (fire), smother (foam), cool (frost),
        // wall and reveal.
        let mut uses: Vec<Runs> = live.iter().filter_map(|q| q.needs.runs).collect();
        uses.sort();
        assert_eq!(
            uses,
            vec![
                Runs::Fire,
                Runs::Frost,
                Runs::Foam,
                Runs::Wall,
                Runs::Reveal
            ]
        );
        for q in live {
            assert!(EVENTS.contains(&q.event.as_str()), "{}: {}", q.id, q.event);
            for t in [
                &q.line,
                &q.line_sprout,
                &q.hint,
                &q.hint_sprout,
                &q.done,
                &q.place,
            ] {
                assert!(!t.is_empty(), "{}: a text is missing", q.id);
            }
            assert!(q.hint.starts_with("Hint: ") && q.hint_sprout.starts_with("Hint: "));
            assert!(q.window_s > 0.0 && q.radius_m > 0.0);
            assert_eq!(q.at.is_some(), q.near_m.is_some(), "{}", q.id);
            // Its examples are real reactions that meet its needs.
            assert!(!q.solved_by.is_empty(), "{}", q.id);
            for id in &q.solved_by {
                let e = c
                    .get(id)
                    .unwrap_or_else(|| panic!("{}: no reaction {id}", q.id));
                let f = fx.of(e);
                assert!(
                    f.saveable() && q.needs.met_by(&f),
                    "{}: {id} does not solve it",
                    q.id
                );
                // The hint names the kind of reaction, never the answer.
                assert!(!q.hint.contains(&e.name) && !q.hint_sprout.contains(&e.name));
            }
            // At least one example's bottles can all be found, and one at
            // Sprout (every band can do every quest).
            let findable = |e: &Entry| q.needs.met_by(&fx.of(e)) && b.findable_plan(e);
            assert!(
                q.solved_by.iter().any(|id| findable(c.get(id).unwrap())),
                "{}: no example's bottles can be found",
                q.id
            );
            assert!(
                c.entries
                    .iter()
                    .any(|e| e.band == Tier::Sprout && findable(e)),
                "{}: no Sprout reaction's bottles can be found",
                q.id
            );
        }
        // A shipped-like spell that does not match never meets a quest's
        // needs: a fizz that lifts is no smother, a hot fizz no frost.
        let need = |id: &str| {
            quests
                .quests
                .iter()
                .find(|q| q.id == id)
                .unwrap()
                .needs
                .clone()
        };
        assert!(!need("smother_with_co2").met_by(&fx.of(c.get("zinc_acid").unwrap())));
        assert!(!need("cool_the_vent").met_by(&fx.of(c.get("limestone_roast").unwrap())));
        assert!(!need("wall_in_the_river").met_by(&fx.of(c.get("iron_copper_sulfate").unwrap())));
        assert!(!need("reveal_the_hidden_mark").met_by(&fx.of(c.get("silver_chloride").unwrap())));
    }

    /// I-057: a Guide lesson that points at a quest teaches a reaction that
    /// solves it.
    #[test]
    fn guide_lessons_point_at_quests_their_reactions_solve() {
        let guide: serde_json::Value =
            serde_json::from_str(include_str!("../../../content/chem/guide.json")).unwrap();
        let c = Codex::load();
        let fx = Effects::load();
        let quests = Quests::load();
        let mut n = 0;
        for (band, b) in guide["bands"].as_object().unwrap() {
            for l in b["lessons"].as_array().unwrap() {
                let Some(id) = l["world"]["quest"].as_str() else {
                    continue;
                };
                let q = quests.quests.iter().find(|q| q.id == id && q.live);
                let q = q.unwrap_or_else(|| panic!("{band} {}: no live quest {id}", l["id"]));
                let solves = l["reactions"].as_array().unwrap().iter().any(|r| {
                    c.get(r.as_str().unwrap())
                        .is_some_and(|e| q.needs.met_by(&fx.of(e)))
                });
                assert!(
                    solves,
                    "{band} {}: none of its reactions solves {id}",
                    l["id"]
                );
                n += 1;
            }
        }
        assert!(n >= 5);
    }

    #[test]
    fn a_discovered_spell_is_judged_by_the_codex() {
        let c = Codex::load();
        let fx = Effects::load();
        for e in &c.entries {
            let def = spell_def(e, &fx.of(e));
            let band = Tier::Adept;
            let v = judge_with(&c, std::slice::from_ref(&def), def.slot, band, &e.equation);
            assert!(v.ok, "{}: {:?}", e.id, v.code);
        }
        let e = c.get("carbon_burns").unwrap();
        let def = spell_def(e, &fx.of(e));
        let v = judge_with(
            &c,
            std::slice::from_ref(&def),
            def.slot,
            Tier::Seedling,
            "C + O2 -> CO",
        );
        assert!(!v.ok);
        let v = judge_with(
            &c,
            std::slice::from_ref(&def),
            def.slot,
            Tier::Seedling,
            "2C + 2O2 -> 2CO2",
        );
        assert!(v.ok, "a multiple is still the reaction");
        let haber = c.get("haber").unwrap();
        let def = spell_def(haber, &fx.of(haber));
        let v = judge_with(
            &c,
            std::slice::from_ref(&def),
            def.slot,
            Tier::Seedling,
            &haber.equation,
        );
        assert_eq!(v.code, Code::NotForBand);
    }

    #[test]
    fn mastery_is_two_builds_and_sprout_never_builds() {
        let c = Codex::load();
        let e = c.get("carbon_burns").unwrap();
        assert!(!instant(e, Tier::Seedling, 0));
        assert!(!instant(e, Tier::Seedling, 1));
        assert!(instant(e, Tier::Seedling, 2));
        assert!(instant(e, Tier::Sprout, 0));
        assert!(usable(e, Tier::Sprout));
        assert!(!usable(c.get("haber").unwrap(), Tier::Seedling));
    }
}
