//! Keys for the "lab records" index (`content/chem/literature.json`), built
//! from the Open Reaction Database by `tools/chemdb/build_literature.py`.
//!
//! A key is a net equation in Hill formulas: species on both sides cancel,
//! coefficients share no common factor, and each side is sorted. The Python
//! builder makes the same key, so the page can look a player's equation up.

use realgame_sim::formula::{Atoms, Equation, Term};
use std::collections::BTreeMap;

/// Hill order: C, then H, then the rest A to Z. With no carbon, everything
/// is A to Z (`ClNa`, `H2O`). This matches RDKit's formulas in the index.
pub fn hill(atoms: &Atoms) -> String {
    let part = |el: &str, n: u32| {
        if n > 1 {
            format!("{el}{n}")
        } else {
            el.to_string()
        }
    };
    let mut out = String::new();
    let carbon = atoms.get("C").copied().unwrap_or(0);
    if carbon > 0 {
        out += &part("C", carbon);
        if let Some(&h) = atoms.get("H") {
            out += &part("H", h);
        }
    }
    for (el, &n) in atoms {
        if n == 0 || (carbon > 0 && (el == "C" || el == "H")) {
            continue;
        }
        out += &part(el, n);
    }
    out
}

fn side(terms: &[Term]) -> BTreeMap<String, u32> {
    let mut m = BTreeMap::new();
    for t in terms {
        *m.entry(hill(&t.atoms)).or_insert(0) += t.coef;
    }
    m
}

fn text(side: &BTreeMap<String, u32>) -> String {
    side.iter()
        .map(|(f, &n)| if n > 1 { format!("{n}{f}") } else { f.clone() })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// The index key for `eq`, or None if nothing is left after cancelling.
pub fn key(eq: &Equation) -> Option<String> {
    let (mut l, mut r) = (side(&eq.reactants), side(&eq.products));
    let shared: Vec<String> = l.keys().filter(|f| r.contains_key(*f)).cloned().collect();
    for f in shared {
        let k = l[&f].min(r[&f]);
        for s in [&mut l, &mut r] {
            let n = s.get_mut(&f).expect("shared species");
            *n -= k;
            if *n == 0 {
                s.remove(&f);
            }
        }
    }
    if l.is_empty() || r.is_empty() {
        return None;
    }
    let g = l
        .values()
        .chain(r.values())
        .fold(0, |g, &c| crate::gcd(g, c))
        .max(1);
    for v in l.values_mut().chain(r.values_mut()) {
        *v /= g;
    }
    Some(format!("{} -> {}", text(&l), text(&r)))
}

/// The reactant half of a key (`2H2 + O2`), for looking up what a set of
/// reactants turned into.
pub fn reactant_key(terms: &[Term]) -> Option<String> {
    let mut l = side(terms);
    if l.is_empty() {
        return None;
    }
    let g = l.values().fold(0, |g, &c| crate::gcd(g, c)).max(1);
    for v in l.values_mut() {
        *v /= g;
    }
    Some(text(&l))
}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_sim::formula::{parse_equation, parse_formula};

    fn k(s: &str) -> Option<String> {
        key(&parse_equation(s).unwrap())
    }

    #[test]
    fn hill_order() {
        let h = |f: &str| hill(&parse_formula(f).unwrap());
        assert_eq!(h("NaCl"), "ClNa");
        assert_eq!(h("H2O"), "H2O");
        assert_eq!(h("CH3COOH"), "C2H4O2");
        assert_eq!(h("NH3"), "H3N");
        assert_eq!(h("CO2"), "CO2");
        assert_eq!(h("Ca(OH)2"), "CaH2O2");
    }

    #[test]
    fn keys_cancel_reduce_and_sort() {
        assert_eq!(
            k("CO(NH2)2 + H2O -> CO2 + 2NH3").as_deref(),
            Some("CH4N2O + H2O -> CO2 + 2H3N")
        );
        assert_eq!(k("4H2 + 2O2 -> 4H2O").as_deref(), Some("2H2 + O2 -> 2H2O"));
        assert_eq!(
            k("Na2CO3 + Na2SO3 + SO2 -> CO2 + 2Na2SO3").as_deref(),
            Some("CNa2O3 + O2S -> CO2 + Na2O3S")
        );
        assert_eq!(k("H2O -> H2O"), None);
    }

    #[test]
    fn reactant_keys() {
        let eq = parse_equation("4H2 + 2O2").unwrap();
        assert_eq!(reactant_key(&eq.reactants).as_deref(), Some("2H2 + O2"));
    }

    /// Every key in the shipped index is already in the form `key` makes, so
    /// a player who types one of them finds it.
    #[test]
    fn shipped_index_round_trips() {
        let doc: serde_json::Value =
            serde_json::from_str(include_str!("../../../content/chem/literature.json")).unwrap();
        let reactions = doc["reactions"].as_object().unwrap();
        assert!(reactions.len() > 5000, "index has {}", reactions.len());
        assert_eq!(doc["licence"], "CC BY-SA 4.0");
        for (k, v) in reactions {
            let eq = parse_equation(k).unwrap_or_else(|e| panic!("{k}: {e}"));
            assert_eq!(key(&eq).as_deref(), Some(k.as_str()));
            assert!(v[0].as_u64().unwrap() >= 1);
        }
    }
}
