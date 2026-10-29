//! Turning what a player typed (or tapped) into a reaction, and running that
//! reaction on whatever reagents are at hand.
//!
//! Nothing here can fail with a bare error: every input becomes a [`Plan`],
//! and every plan resolves to an [`Outcome`]. Unbalanced or short casts run as
//! far as the limiting reagent allows, and atoms are conserved exactly:
//! atoms drawn (from the suit and the air) = atoms in products + leftovers.

use crate::formula::{parse_equation, side_atoms, Atoms, Equation};
use crate::reactions::{Reaction, ReactionBook};
use serde::Serialize;
use std::collections::BTreeMap;

/// Molecule counts by formula (`"H2" -> 4`). One count is one mole of game
/// matter; the suit gauges are measured in these.
pub type Stock = BTreeMap<String, u32>;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// Balanced, with enough reagents: the full effect.
    Full,
    /// Unbalanced or short: ran to the limiting reagent, leftovers returned.
    Partial,
    /// Understood, but nothing could react.
    Fizzle,
    /// Not a spell the game knows (yet).
    Unknown,
}

/// Words players reach for, mapped to the real formula the game casts. The
/// label always shows the formula, so words never replace the science.
const ALIASES: &[(&str, &str)] = &[
    ("water", "H2O"),
    ("h20", "H2O"),
    ("fire", "2H2 + O2 -> 2H2O"),
    ("boom", "2H2 + O2 -> 2H2O"),
    ("pop", "2H2 + O2 -> 2H2O"),
    ("split", "2H2O -> 2H2 + O2"),
    ("bubbles", "2H2O -> 2H2 + O2"),
    ("hydrogen", "H2"),
    ("oxygen", "O2"),
    ("cloud", "CO2"),
    ("smother", "CO2"),
    ("co2", "CO2"),
];

/// Substances the suit can carry and pour out.
pub const CARRIED: &[&str] = &["H2O", "H2", "O2", "CH4", "CO2"];

pub fn alias(text: &str) -> Option<&'static str> {
    let t = text.trim().to_ascii_lowercase();
    ALIASES.iter().find(|(k, _)| *k == t).map(|(_, v)| *v)
}

/// What a cast will try to do, decided from the text alone.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Pour `coef` moles of one carried substance.
    Release {
        formula: String,
        coef: u32,
    },
    /// Run a reaction from the book. `typed` holds the coefficients the player
    /// asked for (already auto-balanced for the youngest tier).
    React {
        reaction_id: String,
        typed: Equation,
        /// Lone hydrogen burns with oxygen from the air (about 21% O2).
        air_o2: bool,
        balanced: bool,
    },
    Unknown {
        suggestion: Option<String>,
    },
}

/// The formula text actually cast, after alias lookup.
pub fn resolve_text(text: &str) -> String {
    match alias(text) {
        Some(a) => a.to_string(),
        None => text.trim().to_string(),
    }
}

fn parse_lenient(text: &str) -> Option<Equation> {
    if let Ok(eq) = parse_equation(text) {
        return Some(eq);
    }
    // Lower-case typing ("h2o", "ch4") is still clearly a formula.
    let upper = text.to_ascii_uppercase();
    parse_equation(&upper).ok()
}

/// Turns typed text into a plan. `auto_balance` fixes coefficients for players
/// too young to balance equations themselves.
pub fn plan(book: &ReactionBook, text: &str, auto_balance: bool) -> Plan {
    let resolved = resolve_text(text);
    let Some(eq) = parse_lenient(&resolved) else {
        return Plan::Unknown {
            suggestion: zero_hint(book, &resolved),
        };
    };
    if eq.reactants.is_empty() {
        return Plan::Unknown { suggestion: None };
    }
    // Hydrogen alone (with or without "-> H2O") burns in air.
    let lone_h2 = eq.reactants.len() == 1
        && eq.reactants[0].formula == "H2"
        && eq.products.iter().all(|p| p.formula == "H2O");
    if lone_h2 {
        let typed = eq.clone();
        return Plan::React {
            reaction_id: "water_synthesis".into(),
            typed,
            air_o2: true,
            balanced: false,
        };
    }
    if !eq.has_arrow && eq.reactants.len() == 1 {
        let t = &eq.reactants[0];
        if CARRIED.contains(&t.formula.as_str()) {
            return Plan::Release {
                formula: t.formula.clone(),
                coef: t.coef,
            };
        }
        return Plan::Unknown {
            suggestion: zero_hint(book, &resolved),
        };
    }
    let found = if eq.products.is_empty() {
        find_by_reactants(book, &eq)
    } else {
        book.find_by_species(&eq)
    };
    let Some((reaction, known)) = found else {
        return Plan::Unknown {
            suggestion: zero_hint(book, &resolved),
        };
    };
    let typed_balanced = eq.is_balanced();
    let typed = if eq.products.is_empty() {
        // "2H2 + O2" with no arrow: keep the typed amounts, use the book's products.
        let mut e = known.clone();
        for t in e.reactants.iter_mut() {
            if let Some(r) = eq.reactants.iter().find(|r| r.formula == t.formula) {
                t.coef = r.coef;
            }
        }
        e
    } else if !typed_balanced && auto_balance {
        known.clone()
    } else {
        eq.clone()
    };
    let balanced = typed.is_balanced();
    Plan::React {
        reaction_id: reaction.id.clone(),
        typed,
        air_o2: false,
        balanced,
    }
}

fn find_by_reactants<'a>(
    book: &'a ReactionBook,
    eq: &Equation,
) -> Option<&'a (Reaction, Equation)> {
    let key = eq.species_key().0;
    book.reactions
        .iter()
        .find(|(_, known)| known.species_key().0 == key)
}

/// The most common slip for young players: a zero instead of the letter O.
fn zero_hint(book: &ReactionBook, text: &str) -> Option<String> {
    if !text.contains('0') {
        return None;
    }
    let alt = text.replace('0', "O");
    match parse_equation(&alt) {
        Ok(eq) if !eq.has_arrow => (eq.reactants.len() == 1
            && CARRIED.contains(&eq.reactants[0].formula.as_str()))
        .then_some(alt),
        Ok(eq) => book.find_by_species(&eq).is_some().then_some(alt),
        Err(_) => None,
    }
}

/// The result of running a plan on a stock of reagents.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Resolution {
    pub outcome: Option<Outcome>,
    /// Molecules taken from the suit.
    pub drawn: Stock,
    /// Atoms taken from the surrounding air (oxygen for lone hydrogen).
    pub from_air: Atoms,
    /// Molecules made (for a reaction) or poured out (for a release).
    pub products: Stock,
    /// Whole reagent molecules handed back to the suit.
    pub returned: Stock,
    /// Every leftover atom (the returned molecules plus loose atoms).
    pub leftovers: Atoms,
    /// Atoms that could not form a whole reagent molecule; the suit keeps them.
    pub loose: Atoms,
    /// Reagents the suit was short of.
    pub missing: Vec<String>,
    /// How many times the smallest product set formed.
    pub units: u32,
    /// Share of the full effect, 0..=1.
    pub power: f32,
    /// Enthalpy change of what actually reacted, kJ (sourced table value, scaled).
    pub dh_kj: f32,
}

fn add_atoms(into: &mut Atoms, from: &Atoms, times: u32) {
    for (el, n) in from {
        *into.entry(el.clone()).or_insert(0) += n * times;
    }
}

fn molecule_atoms(formula: &str) -> Atoms {
    crate::formula::parse_formula(formula).unwrap_or_default()
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Pours `coef` moles of a carried substance.
pub fn resolve_release(formula: &str, coef: u32, stock: &Stock) -> Resolution {
    let have = stock.get(formula).copied().unwrap_or(0);
    let n = coef.min(have);
    let mut r = Resolution {
        units: n,
        ..Default::default()
    };
    if n > 0 {
        r.drawn.insert(formula.to_string(), n);
        r.products.insert(formula.to_string(), n);
    }
    if n < coef {
        r.missing.push(formula.to_string());
    }
    r.power = n as f32 / coef.max(1) as f32;
    r.outcome = Some(if n == 0 {
        Outcome::Fizzle
    } else if n == coef {
        Outcome::Full
    } else {
        Outcome::Partial
    });
    r
}

/// Runs a reaction from the book on the reagents the player asked for, as far
/// as the limiting reagent allows. Products always follow the book's (balanced)
/// ratio; the typed coefficients only decide how much is drawn.
pub fn resolve_reaction(
    reaction: &Reaction,
    known: &Equation,
    typed: &Equation,
    balanced: bool,
    air_o2: bool,
    stock: &Stock,
) -> Resolution {
    let mut r = Resolution::default();
    // The smallest whole set of products: 2H2O -> H2O, CO2 + 2H2O stays.
    let g = known.products.iter().fold(0, |g, t| gcd(g, t.coef)).max(1);
    let unit: Vec<(String, u32)> = known
        .products
        .iter()
        .map(|t| (t.formula.clone(), t.coef / g))
        .collect();
    let mut unit_atoms = Atoms::new();
    for (f, c) in &unit {
        add_atoms(&mut unit_atoms, &molecule_atoms(f), *c);
    }
    let units_per_reaction = g as f32;

    // Draw what was asked for, up to what the suit holds.
    let mut available = Atoms::new();
    let mut requested_atoms = Atoms::new();
    for t in &typed.reactants {
        let have = stock.get(&t.formula).copied().unwrap_or(0);
        let n = t.coef.min(have);
        if n < t.coef {
            r.missing.push(t.formula.clone());
        }
        if n > 0 {
            *r.drawn.entry(t.formula.clone()).or_insert(0) += n;
            add_atoms(&mut available, &t.atoms, n);
        }
        add_atoms(&mut requested_atoms, &t.atoms, t.coef);
    }
    if air_o2 {
        // Air brings exactly the oxygen atoms the hydrogen can use.
        let h = available.get("H").copied().unwrap_or(0);
        let o_need = h / 2;
        if o_need > 0 {
            r.from_air.insert("O".into(), o_need);
            *available.entry("O".into()).or_insert(0) += o_need;
        }
        let h_req = requested_atoms.get("H").copied().unwrap_or(0);
        *requested_atoms.entry("O".into()).or_insert(0) += h_req / 2;
    }
    let units_for = |atoms: &Atoms| {
        unit_atoms
            .iter()
            .map(|(el, need)| atoms.get(el).copied().unwrap_or(0) / need)
            .min()
            .unwrap_or(0)
    };
    let units = units_for(&available);
    let ideal = units_for(&requested_atoms).max(1);
    r.units = units;

    for (f, c) in &unit {
        if units > 0 {
            r.products.insert(f.clone(), c * units);
        }
    }
    // Leftovers: everything drawn that did not end up in a product.
    let mut left = available.clone();
    for (el, need) in &unit_atoms {
        if let Some(v) = left.get_mut(el) {
            *v -= need * units;
        }
    }
    left.retain(|_, v| *v > 0);
    r.leftovers = left.clone();
    // Hand back whole reagent molecules where the atoms allow, the rest stays loose.
    for t in &typed.reactants {
        let drawn = r.drawn.get(&t.formula).copied().unwrap_or(0);
        let fits = t
            .atoms
            .iter()
            .map(|(el, n)| left.get(el).copied().unwrap_or(0) / n)
            .min()
            .unwrap_or(0)
            .min(drawn);
        if fits > 0 {
            r.returned.insert(t.formula.clone(), fits);
            for (el, n) in &t.atoms {
                if let Some(v) = left.get_mut(el) {
                    *v -= n * fits;
                }
            }
        }
    }
    left.retain(|_, v| *v > 0);
    r.loose = left;

    r.dh_kj = reaction.delta_h_kj * units as f32 / units_per_reaction;
    let mut power = units as f32 / ideal as f32;
    if air_o2 {
        // Air is only about 21% oxygen: a smaller, slower flame.
        power *= 0.5;
    }
    r.power = power.clamp(0.0, 1.0);
    let clean = r.leftovers.is_empty() && r.missing.is_empty();
    r.outcome = Some(if units == 0 {
        Outcome::Fizzle
    } else if balanced && clean && !air_o2 {
        Outcome::Full
    } else {
        Outcome::Partial
    });
    r
}

/// Total atoms in a stock of molecules.
pub fn stock_atoms(stock: &Stock) -> Atoms {
    let mut a = Atoms::new();
    for (f, n) in stock {
        add_atoms(&mut a, &molecule_atoms(f), *n);
    }
    a
}

/// Kid-readable leftover text, e.g. "+1 O".
pub fn leftover_label(atoms: &Atoms) -> String {
    atoms
        .iter()
        .map(|(el, n)| format!("+{n} {el}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Atoms of the typed reactants as a side total (used by tests and hints).
pub fn reactant_atoms(eq: &Equation) -> Atoms {
    side_atoms(&eq.reactants)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stock(items: &[(&str, u32)]) -> Stock {
        items.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn aliases_map_to_real_formulas() {
        assert_eq!(alias("Water"), Some("H2O"));
        assert_eq!(alias(" BOOM "), Some("2H2 + O2 -> 2H2O"));
        assert_eq!(alias("h20"), Some("H2O"));
        assert_eq!(alias("split"), Some("2H2O -> 2H2 + O2"));
        assert_eq!(alias("cloud"), Some("CO2"));
        assert_eq!(alias("xyz"), None);
        let book = ReactionBook::load();
        // Every alias resolves to something the game can actually cast.
        for (word, _) in ALIASES {
            assert!(
                !matches!(plan(&book, word, false), Plan::Unknown { .. }),
                "{word}"
            );
        }
    }

    #[test]
    fn unbalanced_hydrogen_runs_to_the_limiting_reagent() {
        let book = ReactionBook::load();
        let Plan::React {
            reaction_id,
            typed,
            balanced,
            air_o2,
        } = plan(&book, "H2 + O2 -> H2O", false)
        else {
            panic!("should be a reaction");
        };
        let (reaction, known) = book.by_id(&reaction_id).unwrap();
        let r = resolve_reaction(
            reaction,
            known,
            &typed,
            balanced,
            air_o2,
            &stock(&[("H2", 4), ("O2", 2)]),
        );
        assert_eq!(r.outcome, Some(Outcome::Partial));
        assert_eq!(r.products, stock(&[("H2O", 1)]));
        assert_eq!(r.leftovers, stock(&[("O", 1)]));
        assert_eq!(r.loose, stock(&[("O", 1)]));
        assert!((r.dh_kj - (-483.6 / 2.0)).abs() < 0.01);
    }

    #[test]
    fn lone_hydrogen_burns_with_air() {
        let book = ReactionBook::load();
        let p = plan(&book, "2H2", false);
        let Plan::React {
            reaction_id,
            typed,
            balanced,
            air_o2,
        } = p
        else {
            panic!("lone H2 should burn");
        };
        assert!(air_o2);
        let (reaction, known) = book.by_id(&reaction_id).unwrap();
        let r = resolve_reaction(
            reaction,
            known,
            &typed,
            balanced,
            air_o2,
            &stock(&[("H2", 4)]),
        );
        assert_eq!(r.outcome, Some(Outcome::Partial));
        assert_eq!(r.products, stock(&[("H2O", 2)]));
        assert_eq!(r.from_air, stock(&[("O", 2)]));
        assert!(r.power <= 0.5);
    }

    #[test]
    fn sprout_auto_balance_gives_full() {
        let book = ReactionBook::load();
        let Plan::React {
            reaction_id,
            typed,
            balanced,
            air_o2,
        } = plan(&book, "H2 + O2 -> H2O", true)
        else {
            panic!()
        };
        assert!(balanced);
        let (reaction, known) = book.by_id(&reaction_id).unwrap();
        let r = resolve_reaction(
            reaction,
            known,
            &typed,
            balanced,
            air_o2,
            &stock(&[("H2", 4), ("O2", 2)]),
        );
        assert_eq!(r.outcome, Some(Outcome::Full));
        assert!((r.dh_kj + 483.6).abs() < 0.01);
    }

    #[test]
    fn releases_are_full_partial_or_fizzle() {
        let s = stock(&[("H2O", 1)]);
        assert_eq!(resolve_release("H2O", 1, &s).outcome, Some(Outcome::Full));
        assert_eq!(
            resolve_release("H2O", 3, &s).outcome,
            Some(Outcome::Partial)
        );
        let r = resolve_release("CO2", 1, &s);
        assert_eq!(r.outcome, Some(Outcome::Fizzle));
        assert_eq!(r.missing, vec!["CO2".to_string()]);
    }

    #[test]
    fn unknown_text_is_unknown_with_zero_hint() {
        let book = ReactionBook::load();
        assert!(matches!(plan(&book, "xyz", false), Plan::Unknown { .. }));
        assert!(matches!(plan(&book, "", false), Plan::Unknown { .. }));
        assert!(matches!(plan(&book, "NaCl", false), Plan::Unknown { .. }));
        let Plan::Unknown { suggestion } = plan(&book, "2H20", false) else {
            panic!()
        };
        assert_eq!(suggestion.as_deref(), Some("2H2O"));
    }
}
