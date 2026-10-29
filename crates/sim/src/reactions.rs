//! The curated reaction table, loaded from `content/reactions.json`.
//!
//! The table is data, not code, so reviewers can check every value against its
//! source and the game can show where each number came from.

use crate::formula::{parse_equation, Equation};
use serde::Deserialize;

const REACTIONS_JSON: &str = include_str!("../../../content/reactions.json");

#[derive(Debug, Clone, Deserialize)]
pub struct Reaction {
    pub id: String,
    pub name: String,
    pub equation: String,
    /// Enthalpy change for the equation as written, kJ. Negative releases heat.
    pub delta_h_kj: f32,
    pub needs: String,
    pub tier_min: String,
    pub concept: String,
    pub real_world: String,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PhaseChange {
    pub id: String,
    pub temperature_c: f32,
    pub latent_heat_kj_per_mol: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct Table {
    reactions: Vec<Reaction>,
    phase_changes: Vec<PhaseChange>,
}

pub struct ReactionBook {
    pub reactions: Vec<(Reaction, Equation)>,
    pub phase_changes: Vec<PhaseChange>,
}

impl ReactionBook {
    pub fn load() -> ReactionBook {
        let table: Table =
            serde_json::from_str(REACTIONS_JSON).expect("content/reactions.json is valid");
        let reactions = table
            .reactions
            .into_iter()
            .map(|r| {
                let eq = parse_equation(&r.equation).expect("reaction equations in content parse");
                assert!(
                    eq.is_balanced(),
                    "content reaction {} must be balanced",
                    r.id
                );
                (r, eq)
            })
            .collect();
        ReactionBook {
            reactions,
            phase_changes: table.phase_changes,
        }
    }

    /// Finds the reaction with the same species on each side, ignoring coefficients.
    pub fn find_by_species(&self, eq: &Equation) -> Option<&(Reaction, Equation)> {
        let key = eq.species_key();
        self.reactions
            .iter()
            .find(|(_, known)| known.species_key() == key)
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
}
