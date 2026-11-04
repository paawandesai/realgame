//! The mystery-card model: could this balanced combination, which is not in
//! our data, hold together?
//!
//! It is deliberately simple and says so. A product passes when it is a
//! species the codex already knows, an element in a form it really takes
//! (`O2`, `P4`, a metal atom), or a compound that one of two textbook rules
//! allows:
//!
//! - **with a metal**: oxidation states from a short table of common ones
//!   can cancel to zero (`NaCl`: +1 and -1);
//! - **all non-metals**: each atom's usual number of bonds can join the
//!   molecule into one connected piece (`CH4`: 4 + 4 x 1 bonds, enough to
//!   link 5 atoms).
//!
//! It never estimates energy yet, so a mystery card says "might hold
//! together", never "will react".

use crate::Outcome;
use realgame_sim::formula::{Atoms, Equation};
use std::collections::BTreeSet;

const METALS: &[&str] = &[
    "Li", "Be", "Na", "Mg", "Al", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu",
    "Zn", "Ga", "Rb", "Sr", "Ag", "Sn", "Cs", "Ba", "Pt", "Au", "Hg", "Pb", "U",
];

/// Common oxidation states, for compounds with a metal in them.
fn states(el: &str) -> &'static [i32] {
    match el {
        "H" => &[1, -1],
        "Li" | "Na" | "K" | "Rb" | "Cs" | "Ag" => &[1],
        "Be" | "Mg" | "Ca" | "Sr" | "Ba" | "Zn" | "Ni" => &[2],
        "Al" | "Ga" | "Sc" | "B" => &[3],
        "Ti" => &[2, 3, 4],
        "V" => &[2, 3, 4, 5],
        "Cr" => &[2, 3, 6],
        "Mn" => &[2, 3, 4, 6, 7],
        "Fe" | "Co" => &[2, 3],
        "Cu" | "Hg" => &[1, 2],
        "Au" => &[1, 3],
        "Sn" | "Pb" | "Pt" | "Ge" => &[2, 4],
        "U" => &[3, 4, 5, 6],
        "C" => &[-4, -3, -2, -1, 0, 1, 2, 3, 4],
        "Si" => &[4, -4],
        "N" => &[-3, -2, -1, 1, 2, 3, 4, 5],
        "P" | "As" => &[-3, 3, 5],
        "O" => &[-2, -1],
        "S" | "Se" => &[-2, 2, 4, 6],
        "F" => &[-1],
        "Cl" | "I" => &[-1, 1, 3, 5, 7],
        "Br" => &[-1, 1, 3, 5],
        "Xe" => &[2, 4, 6],
        "Kr" => &[2],
        _ => &[],
    }
}

/// Usual numbers of bonds, for molecules of non-metals only.
fn valences(el: &str) -> &'static [u32] {
    match el {
        "H" | "F" => &[1],
        "Cl" | "Br" | "I" => &[1, 3, 5, 7],
        "O" | "Se" => &[2],
        "S" => &[2, 4, 6],
        "N" | "As" => &[3, 5],
        "P" => &[3, 5],
        "C" => &[4, 2],
        "Si" | "Ge" => &[4],
        "B" => &[3],
        "Xe" => &[2, 4, 6],
        "Kr" => &[2],
        _ => &[],
    }
}

/// Elements as they really occur on their own.
fn elemental_ok(el: &str, n: u32) -> bool {
    match el {
        "H" | "N" | "F" | "Cl" | "Br" | "I" => n == 2,
        "O" => n == 2 || n == 3,
        "P" => n == 4 || n == 1,
        "S" => n == 1 || n == 8,
        "C" | "Si" | "B" | "He" | "Ne" | "Ar" | "Kr" | "Xe" => n == 1,
        _ => METALS.contains(&el) && n == 1,
    }
}

/// Some choice of one oxidation state per element sums to zero.
fn states_cancel(atoms: &[(&str, u32)]) -> bool {
    fn go(atoms: &[(&str, u32)], sum: i64) -> bool {
        match atoms.split_first() {
            None => sum == 0,
            Some(((el, n), rest)) => states(el)
                .iter()
                .any(|&s| go(rest, sum + s as i64 * *n as i64)),
        }
    }
    go(atoms, 0)
}

/// Some choice of one valence per element links every atom into one piece:
/// the bond ends pair up (even total), there are enough for a tree
/// (total >= 2 (atoms - 1)), and no atom needs more bonds than the rest can
/// give (largest <= total - largest).
fn bonds_connect(atoms: &[(&str, u32)]) -> bool {
    fn go(atoms: &[(&str, u32)], chosen: &mut Vec<(u32, u32)>) -> bool {
        match atoms.split_first() {
            None => {
                let n: u32 = chosen.iter().map(|c| c.1).sum();
                let total: u32 = chosen.iter().map(|(v, k)| v * k).sum();
                let max = chosen.iter().map(|c| c.0).max().unwrap_or(0);
                n >= 2 && total.is_multiple_of(2) && total >= 2 * (n - 1) && max <= total - max
            }
            Some(((el, n), rest)) => valences(el).iter().any(|&v| {
                chosen.push((v, *n));
                let ok = go(rest, chosen);
                chosen.pop();
                ok
            }),
        }
    }
    go(atoms, &mut Vec::new())
}

/// Why `formula` can't hold together, or `None` when the model allows it.
pub fn problem(formula: &str, atoms: &Atoms, known: &BTreeSet<String>) -> Option<String> {
    if known.contains(formula) {
        return None;
    }
    let list: Vec<(&str, u32)> = atoms.iter().map(|(e, n)| (e.as_str(), *n)).collect();
    if let [(el, n)] = list.as_slice() {
        return if elemental_ok(el, *n) {
            None
        } else {
            Some(format!("{el} doesn't stay together as {formula}."))
        };
    }
    for (el, _) in &list {
        if states(el).is_empty() && !METALS.contains(el) {
            return Some(format!(
                "{el} hardly ever forms compounds, so the model can't place {formula}."
            ));
        }
    }
    let has_metal = list.iter().any(|(e, _)| METALS.contains(e));
    let ok = if has_metal {
        list.iter().any(|(e, _)| !METALS.contains(e)) && states_cancel(&list)
    } else {
        bonds_connect(&list)
    };
    if ok {
        None
    } else if has_metal {
        Some(format!("The charges in {formula} can't cancel out."))
    } else {
        Some(format!(
            "The atoms in {formula} can't all bond into one molecule."
        ))
    }
}

/// A balanced equation that is not in the codex: a mystery card, or no
/// reaction with the model's reason.
pub fn predict(eq: &Equation, known: &BTreeSet<String>) -> Outcome {
    for t in &eq.products {
        if let Some(reason) = problem(&t.formula, &t.atoms, known) {
            return Outcome::NoReaction { reason };
        }
    }
    Outcome::Mystery {
        checks: vec![
            "Every atom on the left is on the right.".into(),
            "Each product could hold together (simple bonding rules).".into(),
            "Not in our reaction data yet, so no one has confirmed it here.".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Codex, Need, Outcome};
    use realgame_sim::formula::{parse_equation, parse_formula};
    use realgame_sim::tier::Tier;

    fn ok(f: &str) -> bool {
        problem(f, &parse_formula(f).unwrap(), &BTreeSet::new()).is_none()
    }

    #[test]
    fn real_compounds_pass() {
        for f in [
            "NaCl",
            "MgO",
            "Fe2O3",
            "CaCO3",
            "Al2(SO4)3",
            "KMnO4",
            "CH4",
            "C3H8",
            "C2H5OH",
            "H2O2",
            "NH3",
            "CO2",
            "CO",
            "SO3",
            "HCN",
            "C6H6",
            "PCl5",
            "SF6",
            "H2SO4",
            "XeF4",
            "O3",
        ] {
            assert!(ok(f), "{f} should pass");
        }
    }

    #[test]
    fn impossible_ones_fail() {
        for f in ["NaCl2", "H3", "MgO2Na9", "CH5", "HeO", "Na2", "O5", "NeCl"] {
            assert!(!ok(f), "{f} should fail");
        }
    }

    #[test]
    fn every_core_species_also_passes_without_the_known_list() {
        // The model should not need the codex to accept real chemistry; the
        // listed exceptions are odd-electron molecules (radicals) and mixed
        // charge states the simple rules can't express.
        let c = Codex::load();
        let exceptions = ["NO", "NO2", "NI3"];
        for f in &c.known_species {
            if exceptions.contains(&f.as_str()) {
                continue;
            }
            assert!(ok(f), "core species {f} fails the model");
        }
    }

    #[test]
    fn a_new_balanced_combination_is_a_mystery() {
        let c = Codex::load();
        // Real (sodium meets fluorine) but not in the core tier.
        let o = c.judge(
            &parse_equation("2Na + F2 -> 2NaF").unwrap(),
            &[Need::Heat],
            Tier::Sprout,
        );
        assert!(matches!(o, Outcome::Mystery { .. }), "{o:?}");
        let o = c.judge(
            &parse_equation("Na + Cl2 -> NaCl2").unwrap(),
            &[],
            Tier::Adept,
        );
        assert!(matches!(o, Outcome::NoReaction { .. }), "{o:?}");
    }
}
