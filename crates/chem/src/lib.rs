//! The Formula chemistry engine (Chemistry Bench plan, Phase 1).
//!
//! One crate that the browser game, the Bench free play page, future game
//! servers and the VR Bench all call, so a reaction behaves the same
//! everywhere. It holds three things:
//!
//! - the **codex**: the core tier of known reactions (`content/chem/core.json`),
//!   each balanced, sourced and tagged with the lowest age band it is taught at;
//! - **judging**: what happens when a player puts something in the chamber
//!   (a known reaction, one that needs a condition, one above their band, an
//!   unbalanced try, or a combination that is not in our data);
//! - **mystery predictions**: a simple rules model (atoms balance, every
//!   product can hold together) for combinations that are not in our data.
//!   A prediction is never called real; the page labels it "predicted by the
//!   model" and lists the checks that passed.
//!
//! Pure Rust, no Bevy. `web` adds the wasm-bindgen surface (`web.rs`).

#![deny(unsafe_code)]

pub mod discover;
/// Re-exported so the client builds its state JSON with the same crate.
pub use serde_json;
pub mod literature;
pub mod predict;
#[cfg(feature = "web")]
#[allow(unsafe_code)]
pub mod web;

use realgame_sim::formula::{parse_equation, side_atoms, Equation, Term};
use realgame_sim::tier::Tier;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CORE_JSON: &str = include_str!("../../../content/chem/core.json");
/// Element display data and everyday names for every codex substance.
pub const NAMES_JSON: &str = include_str!("../../../content/chem/names.json");

/// A condition the chamber can supply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Need {
    Heat,
    Spark,
    Light,
    Electricity,
    Catalyst,
    Pressure,
    Water,
}

impl Need {
    pub const ALL: [Need; 7] = [
        Need::Heat,
        Need::Spark,
        Need::Light,
        Need::Electricity,
        Need::Catalyst,
        Need::Pressure,
        Need::Water,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Need::Heat => "heat",
            Need::Spark => "spark",
            Need::Light => "light",
            Need::Electricity => "electricity",
            Need::Catalyst => "catalyst",
            Need::Pressure => "pressure",
            Need::Water => "water",
        }
    }

    pub fn from_id(s: &str) -> Option<Need> {
        Need::ALL.into_iter().find(|n| n.id() == s)
    }

    /// Whether `have` supplies this need. A spark is a small, quick heat, so
    /// strong heat also lights what a spark lights.
    pub fn met_by(self, have: &[Need]) -> bool {
        have.contains(&self) || (self == Need::Spark && have.contains(&Need::Heat))
    }
}

/// Whether the reaction as written gives out or takes in heat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Heat {
    Releases,
    Absorbs,
    Little,
}

/// How dangerous a reaction is to try for real, so the Bench can show young
/// players a safety badge. `Fire`: flames, sparks or strong heat; `Lab`:
/// reactive metals, poisonous or explosive substances, or industrial
/// conditions, for trained chemists only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Hazard {
    Fire,
    Lab,
}

/// One codex entry, as stored in `content/chem/core.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub family: String,
    pub band: Tier,
    pub equation: String,
    #[serde(default)]
    pub needs: Vec<Need>,
    /// `None` when the heat change is small or not well established, so the
    /// game says nothing about it rather than guess.
    pub heat: Option<Heat>,
    /// One sentence for Apprentice and up.
    pub say: String,
    /// Eight words or fewer, for Sprout and Seedling entries.
    #[serde(default)]
    pub short: Option<String>,
    pub source: String,
    /// `draft` until the internal science review, then `reviewed`.
    pub check: String,
    /// A safety badge for trying it for real; `None` for kitchen-safe ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hazard: Option<Hazard>,
}

#[derive(Deserialize)]
struct CoreFile {
    families: Vec<String>,
    reactions: Vec<Entry>,
}

/// The order in which judging compares species: by formula, coefficients
/// reduced by their shared divisor (so `4H2 + 2O2 -> 4H2O` matches
/// `2H2 + O2 -> 2H2O`).
pub type SideKey = BTreeMap<String, u32>;

pub(crate) fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Each side as formula -> coefficient, both sides divided by one shared
/// divisor. A species written twice on a side is summed.
pub fn reduced(eq: &Equation) -> (SideKey, SideKey) {
    let side = |terms: &[Term]| {
        let mut m = SideKey::new();
        for t in terms {
            *m.entry(t.formula.clone()).or_insert(0) += t.coef;
        }
        m
    };
    let (mut l, mut r) = (side(&eq.reactants), side(&eq.products));
    let g = l
        .values()
        .chain(r.values())
        .fold(0, |g, &c| gcd(g, c))
        .max(1);
    for v in l.values_mut().chain(r.values_mut()) {
        *v /= g;
    }
    (l, r)
}

/// `side` with its coefficients divided by their shared factor.
fn reduce_side(side: &SideKey) -> SideKey {
    let g = side.values().fold(0, |g, &c| gcd(g, c)).max(1);
    side.iter().map(|(f, &n)| (f.clone(), n / g)).collect()
}

fn species(side: &SideKey) -> BTreeSet<String> {
    side.keys().cloned().collect()
}

/// What happens when a player runs the chamber.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    /// A known reaction, at or below the player's band, with every need met.
    Known { id: String },
    /// A known reaction that needs something the chamber did not supply.
    NeedsCondition { id: String, missing: Vec<Need> },
    /// A real reaction taught at a higher band ("you'll learn this later").
    Locked { id: String, band: Tier },
    /// The atoms don't balance. `close` names a known reaction with the same
    /// species, when there is one, so the hint can say "nearly".
    Unbalanced {
        report: Vec<String>,
        close: Option<String>,
    },
    /// Balanced, not in our data, and the model thinks it could hold
    /// together. Never a claim about the real world.
    Mystery { checks: Vec<String> },
    /// Balanced, but the model says at least one product can't hold
    /// together, or nothing changes.
    NoReaction { reason: String },
}

pub struct Codex {
    pub families: Vec<String>,
    pub entries: Vec<Entry>,
    keys: Vec<(SideKey, SideKey)>,
    /// Every formula that appears in the codex, for the predictor's "known
    /// species" check.
    pub known_species: BTreeSet<String>,
}

impl Codex {
    pub fn load() -> Codex {
        Codex::from_json(CORE_JSON).expect("content/chem/core.json is valid")
    }

    pub fn from_json(json: &str) -> Result<Codex, String> {
        let file: CoreFile = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let mut keys = Vec::with_capacity(file.reactions.len());
        let mut known_species = BTreeSet::new();
        for e in &file.reactions {
            let eq = parse_equation(&e.equation).map_err(|err| format!("{}: {err}", e.id))?;
            if !eq.is_balanced() {
                return Err(format!("{}: not balanced", e.id));
            }
            for t in eq.reactants.iter().chain(eq.products.iter()) {
                known_species.insert(t.formula.clone());
            }
            keys.push(reduced(&eq));
        }
        Ok(Codex {
            families: file.families,
            entries: file.reactions,
            keys,
            known_species,
        })
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Judges a whole equation the player wrote (both sides).
    pub fn judge(&self, eq: &Equation, have: &[Need], band: Tier) -> Outcome {
        let (l, r) = reduced(eq);
        let (ls, rs) = (species(&l), species(&r));
        let same_species = self
            .keys
            .iter()
            .position(|(kl, kr)| species(kl) == ls && species(kr) == rs);
        if !eq.is_balanced() {
            return Outcome::Unbalanced {
                report: eq.imbalance_report(),
                close: same_species.map(|i| self.entries[i].id.clone()),
            };
        }
        if let Some(i) = self.keys.iter().position(|k| k.0 == l && k.1 == r) {
            return self.known(i, have, band);
        }
        if l == r || (ls == rs) {
            return Outcome::NoReaction {
                reason: "Both sides hold the same things, so nothing changes.".into(),
            };
        }
        predict::predict(eq, &self.known_species)
    }

    /// Runs the chamber with only reactants in it: finds a known reaction
    /// whose reactants match (same species, same ratio). With no match it
    /// returns `None`, and the page says nothing happened.
    pub fn run(&self, reactants: &[Term], have: &[Need], band: Tier) -> Option<(usize, Outcome)> {
        let probe = Equation {
            reactants: reactants.to_vec(),
            products: vec![],
            has_arrow: false,
            per: 1,
        };
        let (l, _) = reduced(&probe);
        // Every reaction with these reactants; prefer one whose needs are met
        // and that the player's band can see.
        // Compare reactant ratios: a stored key is reduced over the whole
        // equation (2H2O -> 2H2 + O2 keeps its 2), so reduce its reactant side
        // on its own, the same way the probe was.
        let mut matches: Vec<usize> = (0..self.entries.len())
            .filter(|&i| reduce_side(&self.keys[i].0) == l)
            .collect();
        matches.sort_by_key(|&i| {
            let e = &self.entries[i];
            let met = e.needs.iter().all(|n| n.met_by(have));
            (!met, e.band > band)
        });
        let i = *matches.first()?;
        Some((i, self.known(i, have, band)))
    }

    fn known(&self, i: usize, have: &[Need], band: Tier) -> Outcome {
        let e = &self.entries[i];
        if e.band > band {
            return Outcome::Locked {
                id: e.id.clone(),
                band: e.band,
            };
        }
        let missing: Vec<Need> = e
            .needs
            .iter()
            .copied()
            .filter(|n| !n.met_by(have))
            .collect();
        if missing.is_empty() {
            Outcome::Known { id: e.id.clone() }
        } else {
            Outcome::NeedsCondition {
                id: e.id.clone(),
                missing,
            }
        }
    }
}

/// The in-world Formula Bench (`realgame_sim::bench::judge_with`) asks this
/// codex whether a reaction spell's build is the real reaction, so the world
/// and the free-play Bench give the same answer. Every condition is supplied:
/// casting the spell is the spark or the heat, and the codex holds one entry
/// per equation, so conditions never change which reaction an equation is.
impl realgame_sim::bench::ReactionCodex for Codex {
    fn call(&self, eq: &Equation, band: Tier) -> realgame_sim::bench::CodexCall {
        use realgame_sim::bench::CodexCall;
        match self.judge(eq, &Need::ALL, band) {
            Outcome::Known { id } | Outcome::NeedsCondition { id, .. } => CodexCall::Known(id),
            Outcome::Locked { id, .. } => CodexCall::Locked(id),
            Outcome::Unbalanced { .. } => CodexCall::Unbalanced,
            Outcome::Mystery { .. } | Outcome::NoReaction { .. } => CodexCall::Other,
        }
    }
}

/// Atom totals of one side, for the page's balance scale.
pub fn atoms_of(terms: &[Term]) -> BTreeMap<String, u32> {
    side_atoms(terms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eq(s: &str) -> Equation {
        parse_equation(s).unwrap()
    }

    #[test]
    fn the_core_tier_loads_balanced_with_unique_ids_and_species() {
        let c = Codex::load();
        assert!(c.entries.len() >= 150, "{} entries", c.entries.len());
        let ids: BTreeSet<&str> = c.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids.len(), c.entries.len(), "duplicate id");
        let keys: BTreeSet<_> = c.keys.iter().collect();
        assert_eq!(keys.len(), c.entries.len(), "two entries share an equation");
        for e in &c.entries {
            assert!(c.families.contains(&e.family), "{}: family", e.id);
            assert!(!e.source.trim().is_empty(), "{}: source", e.id);
            assert!(e.say.len() <= 140, "{}: say is too long", e.id);
            if e.band <= Tier::Seedling {
                let s = e
                    .short
                    .as_deref()
                    .unwrap_or_else(|| panic!("{}: short", e.id));
                assert!(s.split_whitespace().count() <= 8, "{}: short", e.id);
            }
            assert!(e.check == "draft" || e.check == "reviewed", "{}", e.id);
        }
    }

    #[test]
    fn every_codex_substance_and_element_has_a_name() {
        let c = Codex::load();
        let names: serde_json::Value = serde_json::from_str(NAMES_JSON).unwrap();
        for f in &c.known_species {
            assert!(names["substances"][f].is_string(), "{f}: no everyday name");
            let atoms = realgame_sim::formula::parse_formula(f).unwrap();
            for el in atoms.keys() {
                assert!(
                    names["elements"][el]["name"].is_string(),
                    "{el}: no element data"
                );
            }
        }
    }

    /// Tapping bottles finds every reaction, including ones whose reactant
    /// coefficients share a factor (2H2O, 6CO2 + 6H2O).
    #[test]
    fn running_reactants_alone_finds_every_reaction() {
        let c = Codex::load();
        for (i, e) in c.entries.iter().enumerate() {
            let eq = parse_equation(&e.equation).unwrap();
            let (got, _) = c
                .run(&eq.reactants, &e.needs, Tier::Adept)
                .unwrap_or_else(|| panic!("{}", e.id));
            assert_eq!(
                c.keys[got].0, c.keys[i].0,
                "{} found {}",
                e.id, c.entries[got].id
            );
        }
    }

    #[test]
    fn dangerous_reactions_carry_a_safety_badge() {
        let c = Codex::load();
        for e in &c.entries {
            let fire = e.family == "combustion" || e.needs.contains(&Need::Spark);
            let lab = ["water_metal", "halogen", "electrolysis", "industrial"]
                .contains(&e.family.as_str())
                || ["nitrogen_triiodide", "hf_synthesis", "salt_from_elements"]
                    .contains(&e.id.as_str());
            if lab {
                assert_eq!(e.hazard, Some(Hazard::Lab), "{}", e.id);
            } else if fire {
                assert!(e.hazard.is_some(), "{} burns but has no badge", e.id);
            }
        }
    }

    #[test]
    fn every_band_has_a_codex_to_finish() {
        let c = Codex::load();
        for (band, min) in [
            (Tier::Sprout, 15),
            (Tier::Seedling, 30),
            (Tier::Apprentice, 90),
        ] {
            let n = c.entries.iter().filter(|e| e.band <= band).count();
            assert!(n >= min, "{band:?} reaches only {n}");
        }
    }

    #[test]
    fn a_known_reaction_matches_at_any_multiple() {
        let c = Codex::load();
        let all = [Need::Spark];
        assert_eq!(
            c.judge(&eq("4H2 + 2O2 -> 4H2O"), &all, Tier::Sprout),
            Outcome::Known {
                id: "hydrogen_burns".into()
            }
        );
        // The mock-up bug: per-side reduction made this fizzle.
        assert_eq!(
            c.judge(&eq("2H2 + O2 -> 2H2O"), &all, Tier::Sprout),
            Outcome::Known {
                id: "hydrogen_burns".into()
            }
        );
    }

    #[test]
    fn a_missing_condition_is_named() {
        let c = Codex::load();
        assert_eq!(
            c.judge(&eq("CH4 + 2O2 -> CO2 + 2H2O"), &[], Tier::Adept),
            Outcome::NeedsCondition {
                id: "methane_burns".into(),
                missing: vec![Need::Spark]
            }
        );
        // Heat lights what a spark lights.
        assert!(matches!(
            c.judge(&eq("CH4 + 2O2 -> CO2 + 2H2O"), &[Need::Heat], Tier::Adept),
            Outcome::Known { .. }
        ));
    }

    #[test]
    fn a_higher_band_reaction_is_locked_not_wrong() {
        let c = Codex::load();
        assert_eq!(
            c.judge(&eq("N2 + 3H2 -> 2NH3"), &[], Tier::Sprout),
            Outcome::Locked {
                id: "haber".into(),
                band: Tier::Apprentice
            }
        );
    }

    #[test]
    fn unbalanced_tries_say_so_and_point_at_the_near_miss() {
        let c = Codex::load();
        match c.judge(&eq("H2 + O2 -> H2O"), &[], Tier::Adept) {
            Outcome::Unbalanced { report, close } => {
                assert_eq!(close.as_deref(), Some("hydrogen_burns"));
                assert!(!report.is_empty());
            }
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn running_reactants_alone_finds_the_reaction() {
        let c = Codex::load();
        let r = eq("CH4 + 2O2").reactants;
        let (i, o) = c.run(&r, &[Need::Spark], Tier::Sprout).unwrap();
        assert_eq!(c.entries[i].id, "methane_burns");
        assert!(matches!(o, Outcome::Known { .. }));
        assert!(c
            .run(&eq("He + Ne").reactants, &[Need::Heat], Tier::Adept)
            .is_none());
    }

    #[test]
    fn running_picks_the_variant_the_conditions_allow() {
        let c = Codex::load();
        // C + H2O only reacts as water gas (needs heat).
        let r = eq("C + H2O").reactants;
        let (i, _) = c.run(&r, &[Need::Heat], Tier::Adept).unwrap();
        assert_eq!(c.entries[i].id, "water_gas");
    }

    #[test]
    fn nothing_changes_is_no_reaction() {
        let c = Codex::load();
        assert!(matches!(
            c.judge(&eq("H2O -> H2O"), &[], Tier::Adept),
            Outcome::NoReaction { .. }
        ));
    }
}
