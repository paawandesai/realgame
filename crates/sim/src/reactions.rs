//! The curated reaction table, loaded from `content/reactions.json`.
//!
//! The table is data, not code, so reviewers can check every value against its
//! source and the game can show where each number came from.

use crate::formula::{parse_equation, Equation};
use serde::Deserialize;

/// Reaction and Grimoire card ids added in v3 (DESIGN_V3 §4.2). Track A adds
/// each to `content/reactions.json` (with a source and `reviewed_by: null`)
/// when it implements it; the web shell keys Grimoire cards on these strings.
pub mod ids {
    pub const MG_COMBUSTION: &str = "mg_combustion";
    pub const MG_CO2: &str = "mg_co2";
    pub const MG_STEAM: &str = "mg_steam";
    pub const H2O2_DECOMP: &str = "h2o2_decomp";
    pub const NEUTRALISATION: &str = "neutralisation";
    pub const CO2_DISSOLVES: &str = "co2_dissolves";
    pub const LIMEWATER_TEST: &str = "limewater_test";
    /// Light cards: atomic emission, not reactions (one per `Emitter` salt).
    pub const FLAME_TEST_NA: &str = "flame_test_na";
    pub const FLAME_TEST_LI: &str = "flame_test_li";
    pub const FLAME_TEST_K: &str = "flame_test_k";
    pub const FLAME_TEST_CU: &str = "flame_test_cu";
    pub const FLAME_TEST_SR: &str = "flame_test_sr";

    /// The seven v3 reactions, in §4.2 order.
    pub const V3_REACTIONS: [&str; 7] = [
        MG_COMBUSTION,
        MG_CO2,
        MG_STEAM,
        H2O2_DECOMP,
        NEUTRALISATION,
        CO2_DISSOLVES,
        LIMEWATER_TEST,
    ];
    /// The five light cards.
    pub const LIGHT_CARDS: [&str; 5] = [
        FLAME_TEST_NA,
        FLAME_TEST_LI,
        FLAME_TEST_K,
        FLAME_TEST_CU,
        FLAME_TEST_SR,
    ];
}

pub const REACTIONS_JSON: &str = include_str!("../../../content/reactions.json");
/// Real reactions this trial cannot cast, so the game only calls a typed
/// equation "real chemistry" when it really is.
pub const KNOWN_REACTIONS_JSON: &str = include_str!("../../../content/known_reactions.json");

/// A real, sourced reaction that is not in this trial.
#[derive(Debug, Clone, Deserialize)]
pub struct KnownReaction {
    pub equation: String,
    pub name: String,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KnownTable {
    reactions: Vec<KnownReaction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Reaction {
    pub id: String,
    pub name: String,
    pub equation: String,
    /// The same equation with state symbols at 25 °C (`2H2O(g)`), which the
    /// enthalpy refers to.
    pub equation_states: String,
    /// Enthalpy change for the equation as written, kJ. Negative releases heat.
    pub delta_h_kj: f32,
    pub needs: String,
    pub tier_min: String,
    pub concept: String,
    /// The same idea in six words or fewer, picture-first, for Sprout.
    pub short_sprout: String,
    /// The same idea in eight words or fewer, for Seedling.
    pub short_seedling: String,
    pub real_world: String,
    /// A one-line real-world safety note, for reactions that are dangerous
    /// outside a game.
    #[serde(default)]
    pub safety: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PhaseChange {
    pub id: String,
    pub temperature_c: f32,
    pub latent_heat_kj_per_mol: f32,
    #[serde(default)]
    pub note: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Table {
    reactions: Vec<Reaction>,
    phase_changes: Vec<PhaseChange>,
}

pub struct ReactionBook {
    pub reactions: Vec<(Reaction, Equation)>,
    pub phase_changes: Vec<PhaseChange>,
    /// Real reactions outside this trial (`content/known_reactions.json`).
    pub elsewhere: Vec<(KnownReaction, Equation)>,
}

impl ReactionBook {
    pub fn load() -> ReactionBook {
        let table: Table =
            serde_json::from_str(REACTIONS_JSON).expect("content/reactions.json is valid");
        let reactions = table
            .reactions
            .into_iter()
            .map(|r| {
                let plain =
                    parse_equation(&r.equation).expect("reaction equations in content parse");
                let eq = parse_equation(&r.equation_states)
                    .expect("reaction equations with states in content parse");
                assert_eq!(
                    eq.to_string(),
                    plain.to_string(),
                    "content reaction {}: equation_states must match equation",
                    r.id
                );
                assert!(
                    eq.reactants
                        .iter()
                        .chain(eq.products.iter())
                        .all(|t| t.state.is_some()),
                    "content reaction {}: every term needs a state symbol",
                    r.id
                );
                assert!(
                    eq.is_balanced(),
                    "content reaction {} must be balanced",
                    r.id
                );
                (r, eq)
            })
            .collect();
        let known: KnownTable = serde_json::from_str(KNOWN_REACTIONS_JSON)
            .expect("content/known_reactions.json is valid");
        let elsewhere = known
            .reactions
            .into_iter()
            .map(|r| {
                let eq = parse_equation(&r.equation).expect("known reaction equations parse");
                assert!(
                    eq.is_balanced(),
                    "known reaction {} must be balanced",
                    r.equation
                );
                assert!(
                    !r.source.trim().is_empty(),
                    "known reaction {} needs a source",
                    r.equation
                );
                (r, eq)
            })
            .collect();
        ReactionBook {
            reactions,
            phase_changes: table.phase_changes,
            elsewhere,
        }
    }

    /// Finds the reaction with the same species on each side, ignoring coefficients.
    pub fn find_by_species(&self, eq: &Equation) -> Option<&(Reaction, Equation)> {
        let key = eq.species_key();
        self.reactions
            .iter()
            .find(|(_, known)| known.species_key() == key)
    }

    /// A real reaction outside this trial with the same species on each side.
    pub fn find_elsewhere(&self, eq: &Equation) -> Option<&KnownReaction> {
        let key = eq.species_key();
        self.elsewhere
            .iter()
            .find(|(_, known)| known.species_key() == key)
            .map(|(r, _)| r)
    }

    pub fn by_id(&self, id: &str) -> Option<&(Reaction, Equation)> {
        self.reactions.iter().find(|(r, _)| r.id == id)
    }

    pub fn latent_heat(&self, id: &str) -> f32 {
        self.phase_changes
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.latent_heat_kj_per_mol)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_loads_and_every_reaction_balances() {
        let book = ReactionBook::load();
        assert!(book.reactions.len() >= 4);
    }

    #[test]
    fn known_reactions_elsewhere_are_sourced_and_not_castable() {
        let book = ReactionBook::load();
        assert!(book.elsewhere.len() >= 10);
        for (r, eq) in &book.elsewhere {
            assert!(r.source.len() > 10, "{}", r.equation);
            assert!(
                book.find_by_species(eq).is_none(),
                "{} is castable, so it does not belong in known_reactions.json",
                r.equation
            );
        }
        let salt = parse_equation("2Na + Cl2 -> 2NaCl").unwrap();
        assert!(book.find_elsewhere(&salt).is_some());
        let made_up = parse_equation("He + O2 -> HeO2").unwrap();
        assert!(book.find_elsewhere(&made_up).is_none());
    }

    #[test]
    fn finds_reaction_regardless_of_coefficients() {
        let book = ReactionBook::load();
        let typed = parse_equation("H2 + O2 -> H2O").unwrap();
        assert_eq!(
            book.find_by_species(&typed).unwrap().0.id,
            "water_synthesis"
        );
    }

    #[test]
    fn enthalpies_match_formation_values() {
        // Hess's law from NIST formation enthalpies, kJ/mol.
        let (h2o_g, h2o_l, co2, ch4) = (-241.826_f32, -285.830_f32, -393.51_f32, -74.87_f32);
        let book = ReactionBook::load();
        let dh = |id: &str| book.by_id(id).unwrap().0.delta_h_kj;
        assert!((dh("water_synthesis") - 2.0 * h2o_g).abs() < 0.1);
        assert!((dh("electrolysis_water") + 2.0 * h2o_l).abs() < 0.1);
        assert!((dh("methane_combustion") - (co2 + 2.0 * h2o_g - ch4)).abs() < 0.1);
        assert!((dh("carbon_combustion") - co2).abs() < 0.1);
    }

    #[test]
    fn steam_and_liquid_water_differ_by_the_heat_of_vaporisation() {
        // Hess's law: splitting liquid water (+571.7 kJ) and burning hydrogen to
        // steam (-483.6 kJ) differ by vaporising 2 mol of water at 25 °C.
        let book = ReactionBook::load();
        let dh = |id: &str| book.by_id(id).unwrap().0.delta_h_kj;
        let vap = book.latent_heat("water_vaporisation_25c");
        assert!((vap - 44.0).abs() < 0.01);
        assert!((dh("electrolysis_water") + dh("water_synthesis") - 2.0 * vap).abs() < 0.2);
        let p = book
            .phase_changes
            .iter()
            .find(|p| p.id == "water_vaporisation_25c")
            .unwrap();
        assert!(p.source.contains("NIST") && p.source.contains("44.0"));
        assert!(p.note.as_deref().unwrap_or("").contains("2 × 44.0"));
    }
}
