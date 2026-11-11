//! The Formula Bench rules: what happens when a player puts reagents in the
//! bowl and adds energy.
//!
//! Reactions come only from the sourced tables the 2D and 3D games use
//! (`content/reactions.json` and `content/known_reactions.json`), so the bench
//! never invents chemistry. Amounts are whole molecules; the bowl runs each
//! reaction to its limiting reagent and returns every leftover, so atoms are
//! conserved exactly.

#![deny(unsafe_code)]

use realgame_sim::formula::{parse_formula, Atoms, Equation};
use realgame_sim::reactions::ReactionBook;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Molecule counts by formula (`"H2" -> 2`).
pub type Bowl = BTreeMap<String, u32>;

/// How the player adds energy to the bowl.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Energy {
    /// A flick of flame: heat starts most reactions.
    Spark,
    /// Electricity, which can pull compounds apart.
    Zap,
}

impl Energy {
    pub fn parse(s: &str) -> Energy {
        if s.eq_ignore_ascii_case("zap") {
            Energy::Zap
        } else {
            Energy::Spark
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Every reagent was used up: the amounts matched the equation.
    Exact,
    /// The reaction ran to the limiting reagent and something is left over.
    Partial,
    /// The right reagents, but not enough of one to make even one batch.
    Short,
    /// The right reagents, but the wrong kind of energy (`H2O` needs a zap).
    WrongEnergy,
    /// Nothing in the books reacts like this.
    Nothing,
    /// The bowl is empty.
    Empty,
}

/// A sourced reaction, as the bench and the spellbook show it.
#[derive(Clone, Debug, Serialize)]
pub struct Spell {
    /// Stable id: the book id, or the equation for reactions from the wider list.
    pub id: String,
    pub name: String,
    pub equation: String,
    /// Enthalpy for the equation as written, kJ, when the book has it.
    pub delta_h_kj: Option<f32>,
    pub energy: &'static str,
    pub concept: Option<String>,
    pub real_world: Option<String>,
    pub safety: Option<String>,
    pub source: String,
    /// Reactant formulas with their coefficients.
    pub reactants: Vec<(String, u32)>,
    pub products: Vec<(String, u32)>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MixResult {
    pub status: Status,
    pub spell: Option<Spell>,
    /// How many times the equation ran.
    pub batches: u32,
    pub made: Bowl,
    pub left: Bowl,
    /// One plain sentence for the bench to show.
    pub message: String,
}

struct Entry {
    spell: Spell,
    eq: Equation,
}

fn book() -> &'static Vec<Entry> {
    static BOOK: OnceLock<Vec<Entry>> = OnceLock::new();
    BOOK.get_or_init(|| {
        let b = ReactionBook::load();
        let mut out = Vec::new();
        for (r, eq) in &b.reactions {
            out.push(Entry {
                spell: Spell {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    equation: r.equation.clone(),
                    delta_h_kj: Some(r.delta_h_kj),
                    energy: if r.needs == "electricity" {
                        "zap"
                    } else {
                        "spark"
                    },
                    concept: Some(r.concept.clone()),
                    real_world: Some(r.real_world.clone()),
                    safety: r.safety.clone(),
                    source: r.source.clone(),
                    reactants: terms(&eq.reactants),
                    products: terms(&eq.products),
                },
                eq: eq.clone(),
            });
        }
        for (r, eq) in &b.elsewhere {
            // Noble-gas fluorides are real but need conditions no bench has.
            if eq
                .reactants
                .iter()
                .any(|t| matches!(t.formula.as_str(), "Xe" | "Kr"))
            {
                continue;
            }
            out.push(Entry {
                spell: Spell {
                    id: r.equation.clone(),
                    name: r.name.clone(),
                    equation: r.equation.clone(),
                    delta_h_kj: None,
                    energy: "spark",
                    concept: None,
                    real_world: None,
                    safety: None,
                    source: r.source.clone(),
                    reactants: terms(&eq.reactants),
                    products: terms(&eq.products),
                },
                eq: eq.clone(),
            });
        }
        out
    })
}

fn terms(ts: &[realgame_sim::formula::Term]) -> Vec<(String, u32)> {
    ts.iter().map(|t| (t.formula.clone(), t.coef)).collect()
}

/// Every reaction the bench knows, for the spellbook.
pub fn catalog() -> Vec<Spell> {
    book().iter().map(|e| e.spell.clone()).collect()
}

/// Atoms in a bowl, for the "count the atoms" readout.
pub fn bowl_atoms(bowl: &Bowl) -> Atoms {
    let mut total = Atoms::new();
    for (f, n) in bowl {
        if let Ok(atoms) = parse_formula(f) {
            for (el, k) in atoms {
                *total.entry(el).or_insert(0) += k * n;
            }
        }
    }
    total
}

fn spell_energy(e: &Entry) -> Energy {
    Energy::parse(e.spell.energy)
}

/// Runs the bowl. Picks the reaction whose reactants are exactly the
/// substances in the bowl; failing that, the largest reaction whose reactants
/// are all in the bowl (the rest stays as leftovers).
pub fn mix(bowl: &Bowl, energy: Energy) -> MixResult {
    let bowl: Bowl = bowl
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    if bowl.is_empty() {
        return MixResult {
            status: Status::Empty,
            spell: None,
            batches: 0,
            made: Bowl::new(),
            left: Bowl::new(),
            message: "The bowl is empty. Pinch a reagent from the shelf and drop it in.".into(),
        };
    }
    let fits = |e: &Entry| e.eq.reactants.iter().all(|t| bowl.contains_key(&t.formula));
    let exact = |e: &Entry| fits(e) && e.eq.reactants.len() == bowl.len();
    let mut candidates: Vec<&Entry> = book().iter().filter(|e| exact(e)).collect();
    if candidates.is_empty() {
        candidates = book().iter().filter(|e| fits(e)).collect();
        candidates.sort_by_key(|e| std::cmp::Reverse(e.eq.reactants.len()));
    }
    // Same substances, different energy (water + spark does nothing; water +
    // zap splits it): prefer the reaction this energy can start.
    let chosen = candidates
        .iter()
        .find(|e| spell_energy(e) == energy)
        .copied();
    let Some(entry) = chosen else {
        if let Some(e) = candidates.first() {
            let need = if spell_energy(e) == Energy::Zap {
                "electricity (zap)"
            } else {
                "heat (spark)"
            };
            return MixResult {
                status: Status::WrongEnergy,
                spell: Some(e.spell.clone()),
                batches: 0,
                made: Bowl::new(),
                left: bowl,
                message: format!("Nothing happens. {} needs {need}.", e.spell.equation),
            };
        }
        return nothing(bowl);
    };
    let batches = entry
        .eq
        .reactants
        .iter()
        .map(|t| bowl[&t.formula] / t.coef)
        .min()
        .unwrap_or(0);
    if batches == 0 {
        let short: Vec<String> = entry
            .eq
            .reactants
            .iter()
            .filter(|t| bowl[&t.formula] < t.coef)
            .map(|t| format!("{} {}", t.coef, t.formula))
            .collect();
        return MixResult {
            status: Status::Short,
            spell: Some(entry.spell.clone()),
            batches: 0,
            made: Bowl::new(),
            left: bowl,
            message: format!(
                "Almost! {} needs at least {}.",
                entry.spell.equation,
                short.join(" and ")
            ),
        };
    }
    let mut left = bowl.clone();
    for t in &entry.eq.reactants {
        *left.get_mut(&t.formula).unwrap() -= t.coef * batches;
    }
    left.retain(|_, n| *n > 0);
    let mut made = Bowl::new();
    for t in &entry.eq.products {
        *made.entry(t.formula.clone()).or_insert(0) += t.coef * batches;
    }
    debug_assert_eq!(
        {
            let mut after = made.clone();
            for (k, v) in &left {
                *after.entry(k.clone()).or_insert(0) += v;
            }
            bowl_atoms(&after)
        },
        bowl_atoms(&bowl)
    );
    let status = if left.is_empty() {
        Status::Exact
    } else {
        Status::Partial
    };
    let message = if left.is_empty() {
        format!("Perfect ratio! {}", entry.spell.equation)
    } else {
        let extra: Vec<String> = left.iter().map(|(f, n)| format!("{n} {f}")).collect();
        format!(
            "It worked, with {} left over. The equation is {}.",
            extra.join(" and "),
            entry.spell.equation
        )
    };
    MixResult {
        status,
        spell: Some(entry.spell.clone()),
        batches,
        made,
        left,
        message,
    }
}

fn nothing(bowl: Bowl) -> MixResult {
    // A reaction that uses everything in the bowl plus one more reagent is
    // the most useful hint ("add O2").
    let hint = book()
        .iter()
        .filter(|e| {
            bowl.keys()
                .all(|f| e.eq.reactants.iter().any(|t| &t.formula == f))
                && e.eq.reactants.len() == bowl.len() + 1
        })
        .find_map(|e| {
            e.eq.reactants
                .iter()
                .find(|t| !bowl.contains_key(&t.formula))
                .map(|t| t.formula.clone())
        });
    let message = match hint {
        Some(f) => format!("Nothing reacts yet. Try adding {f}."),
        None => "These don't react with each other. Empty the bowl and try another mix.".into(),
    };
    MixResult {
        status: Status::Nothing,
        spell: None,
        batches: 0,
        made: Bowl::new(),
        left: bowl,
        message,
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    #![allow(unsafe_code)]
    use super::*;
    use wasm_bindgen::prelude::*;

    /// `bowl_json` is `{"H2":2,"O2":1}`; `energy` is `spark` or `zap`.
    /// Returns a `MixResult` as JSON.
    #[wasm_bindgen]
    pub fn bench_mix(bowl_json: &str, energy: &str) -> String {
        let bowl: Bowl = serde_json::from_str(bowl_json).unwrap_or_default();
        serde_json::to_string(&mix(&bowl, Energy::parse(energy))).unwrap()
    }

    /// Every reaction the bench knows, as JSON.
    #[wasm_bindgen]
    pub fn bench_catalog() -> String {
        serde_json::to_string(&catalog()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(items: &[(&str, u32)]) -> Bowl {
        items.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn exact_water() {
        let r = mix(&b(&[("H2", 2), ("O2", 1)]), Energy::Spark);
        assert_eq!(r.status, Status::Exact);
        assert_eq!(r.made, b(&[("H2O", 2)]));
        assert_eq!(r.spell.unwrap().id, "water_synthesis");
    }

    #[test]
    fn leftovers_conserve_atoms() {
        let bowl = b(&[("H2", 5), ("O2", 1)]);
        let r = mix(&bowl, Energy::Spark);
        assert_eq!(r.status, Status::Partial);
        assert_eq!(r.left, b(&[("H2", 3)]));
        let mut after = r.made.clone();
        after.extend(r.left.clone());
        assert_eq!(bowl_atoms(&after), bowl_atoms(&bowl));
    }

    #[test]
    fn water_needs_a_zap_to_split() {
        let spark = mix(&b(&[("H2O", 2)]), Energy::Spark);
        assert_eq!(spark.status, Status::WrongEnergy);
        let zap = mix(&b(&[("H2O", 2)]), Energy::Zap);
        assert_eq!(zap.status, Status::Exact);
        assert_eq!(zap.made, b(&[("H2", 2), ("O2", 1)]));
    }

    #[test]
    fn salt_from_the_wider_list() {
        let r = mix(&b(&[("Na", 2), ("Cl2", 1)]), Energy::Spark);
        assert_eq!(r.status, Status::Exact);
        assert_eq!(r.made, b(&[("NaCl", 2)]));
    }

    #[test]
    fn short_and_hint() {
        let r = mix(&b(&[("Fe", 3), ("O2", 3)]), Energy::Spark);
        assert_eq!(r.status, Status::Short);
        let r = mix(&b(&[("CH4", 1)]), Energy::Spark);
        assert_eq!(r.status, Status::Nothing);
        assert!(r.message.contains("O2"), "{}", r.message);
    }

    #[test]
    fn empty_bowl() {
        assert_eq!(mix(&Bowl::new(), Energy::Spark).status, Status::Empty);
    }

    #[test]
    fn noble_gases_stay_off_the_bench() {
        assert!(catalog().iter().all(|s| !s.equation.contains("Xe")));
        let r = mix(&b(&[("Xe", 1), ("F2", 1)]), Energy::Spark);
        assert_eq!(r.status, Status::Nothing);
    }
}
