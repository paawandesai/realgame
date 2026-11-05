//! Discoveries become spells (I-053, Bench plan Option B steps 3-6).
//!
//! A reaction found on the Chemistry Bench can be saved as a spell. This
//! module holds the pure rules, so the world and the Bench page agree:
//!
//! - **effects** (`content/chem/effects.json`): every codex reaction maps to
//!   one of a small set of spell effects, derived from its family, heat,
//!   hazard and products by ordered rules. `kind` is the look (a data value
//!   the renderer styles), `runs` the world chemistry rule it reuses, with a
//!   `strength` that scales that rule;
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

use crate::{Codex, Entry, Hazard, Heat};

pub const EFFECTS_JSON: &str = include_str!("../../../content/chem/effects.json");
pub const BOTTLES_JSON: &str = include_str!("../../../content/chem/bottles.json");
pub const QUESTS_JSON: &str = include_str!("../../../content/chem/quests.json");

/// The quest event the block world reports when ice or snow melts.
pub const EVENT_MELTED: &str = "melted";

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
}

impl EffectKind {
    pub const ALL: [EffectKind; 6] = [
        EffectKind::Fire,
        EffectKind::Water,
        EffectKind::Frost,
        EffectKind::Foam,
        EffectKind::Cloud,
        EffectKind::Glow,
    ];

    pub fn id(self) -> &'static str {
        match self {
            EffectKind::Fire => "fire",
            EffectKind::Water => "water",
            EffectKind::Frost => "frost",
            EffectKind::Foam => "foam",
            EffectKind::Cloud => "cloud",
            EffectKind::Glow => "glow",
        }
    }
}

/// The world chemistry rule a spell reuses: one of the four shipped spells'
/// rules, or `None` (a puff that changes nothing, for a gas that neither
/// burns nor smothers in the world's rules).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runs {
    Fire,
    Water,
    Frost,
    Foam,
    None,
}

impl Runs {
    pub fn id(self) -> &'static str {
        match self {
            Runs::Fire => "fire",
            Runs::Water => "water",
            Runs::Frost => "frost",
            Runs::Foam => "foam",
            Runs::None => "none",
        }
    }

    /// The hotbar slot (1-4) whose shipped spell has this rule, where the
    /// discovered spell goes; a `None` puff sits on the CO2 (foam) slot.
    pub fn slot(self) -> u8 {
        match self {
            Runs::Fire => 1,
            Runs::Water => 2,
            Runs::Frost => 3,
            Runs::Foam | Runs::None => 4,
        }
    }

    /// The scale of the rule's emit for a spell of `strength` (0 for a puff).
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
            return Err("effects.json: kinds must list the six effect kinds in order".into());
        }
        let names: serde_json::Value =
            serde_json::from_str(crate::NAMES_JSON).map_err(|e| e.to_string())?;
        let gases = names["gases"]
            .as_array()
            .ok_or("names.json: no gases")?
            .iter()
            .filter_map(|g| g.as_str().map(str::to_string))
            .collect();
        Ok(Effects {
            file,
            gases,
            shows: crate::show::Shows::load(),
        })
    }

    /// The show records the effects are built from.
    pub fn shows(&self) -> &crate::show::Shows {
        &self.shows
    }

    fn matches(&self, w: &When, e: &Entry, eq: &Equation) -> bool {
        let list =
            |l: &Option<Vec<String>>, v: &str| l.as_ref().is_none_or(|l| l.iter().any(|x| x == v));
        list(&w.family, &e.family)
            && list(&w.heat, heat_id(e.heat))
            && list(&w.hazard, hazard_id(e.hazard))
            && w.gas_product.is_none_or(|g| g == self.gas_product(eq))
            && w.needs_any.is_none_or(|n| n == !e.needs.is_empty())
    }

    fn gas_product(&self, eq: &Equation) -> bool {
        eq.products.iter().any(|t| self.gases.contains(&t.formula))
    }

    /// The effect of codex entry `e`.
    pub fn of(&self, e: &Entry) -> Effect {
        let eq = eq_of(e);
        let over = self.file.overrides.get(&e.id);
        let kind = over.and_then(|o| o.kind).unwrap_or_else(|| {
            self.file
                .kind_rules
                .iter()
                .find(|r| self.matches(&r.when, e, &eq))
                .map_or(EffectKind::Water, |r| r.kind)
        });
        let show = self.shows.get(&e.id);
        // Strength comes from |ΔH| on the documented scale
        // (`show::strength_of`, I-054 spec section 6); with no ΔH, from the
        // heat-class rules.
        let strength =
            over.and_then(|o| o.strength)
                .unwrap_or_else(|| match show.and_then(|r| r.dh.kj) {
                    Some(kj) => {
                        let driven_cold = e.heat == Some(Heat::Absorbs) && !e.needs.is_empty();
                        crate::show::strength_of(kj, driven_cold)
                    }
                    None => self
                        .file
                        .strength_rules
                        .iter()
                        .find(|r| self.matches(&r.when, e, &eq))
                        .map_or(0.4, |r| r.strength),
                });
        let mut runs = self.file.runs.get(&kind).copied().unwrap_or(Runs::None);
        // A fizz smothers a fire only when every gas it gives is one that
        // neither burns nor feeds a flame (CO2, N2); H2 or O2 would not.
        if kind == EffectKind::Foam {
            let all_smother = eq
                .products
                .iter()
                .filter(|t| self.gases.contains(&t.formula))
                .all(|t| self.file.smothers.contains(&t.formula));
            if !all_smother {
                runs = Runs::None;
            }
        }
        // Heat given off in a solution warms a liquid: it splashes like
        // water rather than burning.
        if kind == EffectKind::Glow && self.file.aqueous_families.contains(&e.family) {
            runs = Runs::Water;
        }
        let heat = match e.heat {
            Some(Heat::Releases) => 1,
            Some(Heat::Absorbs) => -1,
            Some(Heat::Little) | None => 0,
        };
        // The tint is the show record's: a fire's flame colour, otherwise the
        // first coloured product (precipitate, gas or solution).
        let tint = show.and_then(|r| r.tint(runs == Runs::Fire));
        Effect {
            reaction_id: e.id.clone(),
            kind,
            heat,
            strength,
            tint,
            runs,
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bottles {
    pub version: u32,
    pub about: String,
    /// Substances the air supplies free.
    pub air: Vec<String>,
    pub items: Vec<BottleItem>,
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
        assert!(Effects::from_json(&EFFECTS_JSON.replace("\"glow\"]", "\"cloud\"]")).is_err());
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
        // Hydrogen does not smother a fire.
        let zinc = fx("zinc_acid");
        assert_eq!((zinc.kind, zinc.runs), (EffectKind::Foam, Runs::None));
        assert_eq!(fx("silver_chloride").kind, EffectKind::Cloud);
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
        assert_eq!(Runs::None.amount(0.5), 0.0);
        assert_eq!(Runs::Fire.amount(1.5), 1.0);
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
