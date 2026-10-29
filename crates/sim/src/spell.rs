//! Turning what a player typed (or tapped) into a reaction, and running that
//! reaction on whatever reagents are at hand.
//!
//! Nothing here can fail with a bare error: every input becomes a [`Plan`],
//! and every plan resolves to an [`Outcome`]. Unbalanced or short casts run as
//! far as the limiting reagent allows, and atoms are conserved exactly:
//! atoms drawn (from the suit and the air) = atoms in products + leftovers.

use crate::formula::{parse_equation, side_atoms, Atoms, Equation, ParseError, State, Term};
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
    // "fire" is the methane fireball, never hydrogen combustion: the boom
    // spell has its own words, and a fireball is a flame you can keep lit.
    ("fire", "CH4 + 2O2 -> CO2 + 2H2O"),
    ("fireball", "CH4 + 2O2 -> CO2 + 2H2O"),
    ("methane", "CH4"),
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
    Release { formula: String, coef: u32 },
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
        reason: Rejection,
    },
}

/// Why a cast could not become a spell, so the player hears something more
/// useful than "no".
#[derive(Debug, Clone, PartialEq)]
pub enum Rejection {
    /// Nothing typed.
    Empty,
    /// Not shaped like a formula at all (`asdf`, `H2 ++ O2`).
    NotAFormula,
    /// A capital letter group that is not an element (`Hh`).
    UnknownElement(String),
    /// A real-looking formula or a balanced reaction this trial does not have
    /// (`NaCl`, `2Na + Cl2 -> 2NaCl`).
    NotInWorld { balanced: bool },
    /// Atoms do not add up and no known reaction matches. `counts` are
    /// per-element lines ("O: 1 on the left, 2 on the right"). `pair` names a
    /// diatomic gas written as a lone atom (`O` for `O2`).
    Unbalanced {
        counts: Vec<String>,
        pair: Option<String>,
    },
    /// A known reaction, but with a state this trial has no data for
    /// (`H2(l)`), named as the term the player wrote.
    WrongState(String),
    /// A digit zero where the letter O was meant (the suggestion holds the fix).
    ZeroForO,
}

impl Rejection {
    /// A short machine name for the UI ("parse", "element", ...).
    pub fn code(&self) -> &'static str {
        match self {
            Rejection::Empty => "empty",
            Rejection::NotAFormula => "parse",
            Rejection::UnknownElement(_) => "element",
            Rejection::NotInWorld { .. } => "not_in_world",
            Rejection::Unbalanced { .. } => "unbalanced",
            Rejection::WrongState(_) => "state",
            Rejection::ZeroForO => "zero",
        }
    }
}

/// Elements that exist as two-atom molecules as gases or liquids at room
/// temperature.
const DIATOMIC: &[&str] = &["H", "N", "O", "F", "Cl", "Br", "I"];

/// Heat to turn one mole of liquid water into steam at 25 C, kJ/mol, read
/// from `content/reactions.json` (`water_vaporisation_25c`, NIST-sourced).
fn state_enthalpy_offset(book: &ReactionBook, formula: &str, state: State) -> Option<f32> {
    match (formula, state) {
        ("H2O", State::Liquid) => Some(0.0),
        ("H2O", State::Gas) => Some(book.latent_heat("water_vaporisation_25c")),
        _ => None,
    }
}

/// The enthalpy of a book reaction as written in the book, but with the
/// states the player typed. Only water's liquid/gas switch has sourced data;
/// any other change of state is `Err` naming the term.
pub fn delta_h_with_states(
    book: &ReactionBook,
    reaction: &Reaction,
    known: &Equation,
    typed: &Equation,
) -> Result<f32, String> {
    let mut dh = reaction.delta_h_kj;
    let sides = [
        (&known.reactants, &typed.reactants, -1.0f32),
        (&known.products, &typed.products, 1.0f32),
    ];
    for (book_side, typed_side, sign) in sides {
        for t in typed_side.iter() {
            let (Some(state), Some(k)) =
                (t.state, book_side.iter().find(|k| k.formula == t.formula))
            else {
                continue;
            };
            let Some(book_state) = k.state else { continue };
            if state == book_state {
                continue;
            }
            let shift = state_enthalpy_offset(book, &t.formula, state)
                .zip(state_enthalpy_offset(book, &t.formula, book_state))
                .map(|(a, b)| a - b);
            match shift {
                Some(per_mol) => dh += sign * per_mol * k.coef as f32,
                None => return Err(format!("{}({})", t.formula, state.symbol())),
            }
        }
    }
    Ok(dh)
}

/// Enthalpy, kJ, of a typed equation that matches a book reaction, scaled to
/// the amounts written (fractions included) and corrected for the states
/// written. `None` if it is not a book reaction in the book's proportions.
pub fn delta_h_for(book: &ReactionBook, typed: &Equation) -> Option<f32> {
    let (reaction, known) = book.find_by_species(typed)?;
    let dh = delta_h_with_states(book, reaction, known, typed).ok()?;
    let k0 = known.reactants.first()?;
    let t0 = typed.reactants.iter().find(|t| t.formula == k0.formula)?;
    // Every typed coefficient must be the same multiple of the book's.
    let ratio = t0.coef as f32 / k0.coef as f32;
    let proportional = known
        .reactants
        .iter()
        .chain(known.products.iter())
        .all(|k| {
            typed
                .reactants
                .iter()
                .chain(typed.products.iter())
                .find(|t| t.formula == k.formula)
                .is_some_and(|t| (t.coef as f32 - ratio * k.coef as f32).abs() < 1e-4)
        });
    proportional.then(|| dh * ratio / typed.per.max(1) as f32)
}

/// The formula text actually cast, after alias lookup.
pub fn resolve_text(text: &str) -> String {
    match alias(text) {
        Some(a) => a.to_string(),
        None => text.trim().to_string(),
    }
}

/// Parses typed text, forgiving all-lower-case typing ("h2o", "ch4"). A
/// failure keeps the error from the text as typed, unless that was only an
/// unknown element: "Hh" is reported as not an element, "asdf" as not a
/// formula (never as "A is not an element").
fn parse_lenient(text: &str) -> Result<Equation, ParseError> {
    let err = match parse_equation(text) {
        Ok(eq) => return Ok(eq),
        Err(e @ ParseError::BadCharacter(_)) => e,
        Err(e) => return Err(e),
    };
    parse_equation(&upper_keeping_states(text)).map_err(|_| err)
}

/// Upper-cases letters, but leaves state symbols like `(g)` and `(aq)` alone.
fn upper_keeping_states(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '(' {
            let state = ["(s)", "(l)", "(g)", "(aq)"].into_iter().find(|s| {
                rest.get(..s.len())
                    .is_some_and(|p| p.eq_ignore_ascii_case(s))
            });
            if let Some(s) = state {
                out.push_str(s);
                rest = &rest[s.len()..];
                continue;
            }
        }
        out.push(c.to_ascii_uppercase());
        rest = &rest[c.len_utf8()..];
    }
    out
}

fn unknown(reason: Rejection) -> Plan {
    Plan::Unknown {
        suggestion: None,
        reason,
    }
}

/// Turns typed text into a plan. `auto_balance` fixes coefficients for players
/// too young to balance equations themselves.
pub fn plan(book: &ReactionBook, text: &str, auto_balance: bool) -> Plan {
    let resolved = resolve_text(text);
    let eq = match parse_lenient(&resolved) {
        Ok(eq) => eq,
        Err(e) => {
            if let Some(fix) = zero_hint(book, &resolved) {
                return Plan::Unknown {
                    suggestion: Some(fix),
                    reason: Rejection::ZeroForO,
                };
            }
            return unknown(match e {
                ParseError::Empty => Rejection::Empty,
                ParseError::UnknownElement(el) => Rejection::UnknownElement(el),
                _ => Rejection::NotAFormula,
            });
        }
    };
    if eq.reactants.is_empty() {
        return unknown(Rejection::Empty);
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
        if let Some(fix) = zero_hint(book, &resolved) {
            return Plan::Unknown {
                suggestion: Some(fix),
                reason: Rejection::ZeroForO,
            };
        }
        return unknown(Rejection::NotInWorld { balanced: false });
    }
    let found = if eq.products.is_empty() {
        find_by_reactants(book, &eq)
    } else {
        book.find_by_species(&eq)
    };
    let Some((reaction, known)) = found else {
        if let Some(fix) = zero_hint(book, &resolved) {
            return Plan::Unknown {
                suggestion: Some(fix),
                reason: Rejection::ZeroForO,
            };
        }
        return unknown(not_a_book_reaction(book, &eq));
    };
    if let Err(term) = delta_h_with_states(book, reaction, known, &eq) {
        return unknown(Rejection::WrongState(term));
    }
    let typed_balanced = eq.is_balanced();
    let typed = if eq.products.is_empty() {
        // "2H2 + O2" with no arrow: keep the typed amounts, use the book's products.
        let mut e = known.clone();
        for t in e.reactants.iter_mut() {
            if let Some(r) = eq.reactants.iter().find(|r| r.formula == t.formula) {
                t.coef = r.coef;
                t.state = r.state;
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

/// Why an equation with no matching book reaction was rejected: a lone atom
/// of a two-atom gas (`H2O -> H2 + O`), atoms that do not add up, or a
/// balanced reaction this trial simply does not have.
fn not_a_book_reaction(book: &ReactionBook, eq: &Equation) -> Rejection {
    let mut paired = eq.clone();
    let mut pair = None;
    for t in paired
        .reactants
        .iter_mut()
        .chain(paired.products.iter_mut())
    {
        if DIATOMIC.contains(&t.formula.as_str()) {
            pair = Some(format!("{}2", t.formula));
            t.formula = format!("{}2", t.formula);
            for n in t.atoms.values_mut() {
                *n = 2;
            }
        }
    }
    if pair.is_some() && book.find_by_species(&paired).is_some() {
        return Rejection::Unbalanced {
            counts: paired.imbalance_report(),
            pair,
        };
    }
    if !eq.products.is_empty() && !eq.is_balanced() {
        return Rejection::Unbalanced {
            counts: eq.imbalance_report(),
            pair: None,
        };
    }
    Rejection::NotInWorld {
        balanced: eq.is_balanced(),
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
    if units == 0 {
        // A fizzle draws nothing: a missing reactant leaves the suit exactly
        // as it was, and no atoms are counted as wasted.
        r.drawn.clear();
        r.returned.clear();
        r.loose.clear();
        r.leftovers.clear();
        r.from_air.clear();
        r.dh_kj = 0.0;
    }
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

/// What did not react, as whole reagent molecules: "1 H2", or "½ O2" when
/// an odd oxygen atom is left (half a mole of O2, never a lone "O").
pub fn unreacted(returned: &Stock, loose: &Atoms, reactants: &[Term]) -> Vec<String> {
    let mut out = vec![];
    let mut loose = loose.clone();
    for t in reactants {
        let whole = returned.get(&t.formula).copied().unwrap_or(0);
        // Loose atoms of a one-element reagent (H2, O2) are a fraction of it.
        let mut part = (0, 1);
        if t.atoms.len() == 1 {
            let (el, per) = t.atoms.iter().next().expect("one element");
            if let Some(n) = loose.remove(el) {
                part = (n, *per);
            }
        }
        let (n, per) = part;
        let whole = whole + n / per;
        let rem = n % per;
        let text = match (whole, rem, per) {
            (0, 0, _) => continue,
            (w, 0, _) => format!("{w}"),
            (0, 1, 2) => "½".to_string(),
            (w, 1, 2) => format!("{w}½"),
            (0, r, p) => format!("{r}/{p}"),
            (w, r, p) => format!("{w} {r}/{p}"),
        };
        out.push(format!("{text} {}", t.formula));
    }
    // Anything else (never in this game's reactions) is named as atoms.
    for (el, n) in loose {
        out.push(format!("{n} {el} atom{}", if n == 1 { "" } else { "s" }));
    }
    out
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
        // Redesigned in playtest round 1: "fire" means the methane fireball,
        // not hydrogen combustion (that is "boom").
        assert_eq!(alias("fire"), Some("CH4 + 2O2 -> CO2 + 2H2O"));
        assert_eq!(alias("Fireball"), Some("CH4 + 2O2 -> CO2 + 2H2O"));
        assert_ne!(alias("fire"), alias("boom"));
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
    fn a_fizzle_for_a_missing_reactant_draws_nothing() {
        let book = ReactionBook::load();
        let Plan::React {
            reaction_id,
            typed,
            balanced,
            air_o2,
        } = plan(&book, "CH4 + 2O2 -> CO2 + 2H2O", false)
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
            &stock(&[("O2", 4), ("H2", 4)]),
        );
        assert_eq!(r.outcome, Some(Outcome::Fizzle));
        assert!(r.drawn.is_empty(), "{:?}", r.drawn);
        assert!(r.leftovers.is_empty(), "no atoms wasted: {:?}", r.leftovers);
        assert!(r.returned.is_empty() && r.loose.is_empty());
        assert_eq!(r.missing, vec!["CH4".to_string()]);
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
        let Plan::Unknown { suggestion, reason } = plan(&book, "2H20", false) else {
            panic!()
        };
        assert_eq!(suggestion.as_deref(), Some("2H2O"));
        assert_eq!(reason, Rejection::ZeroForO);
    }

    fn reason(book: &ReactionBook, text: &str) -> Rejection {
        match plan(book, text, false) {
            Plan::Unknown { reason, .. } => reason,
            other => panic!("{text} should be rejected, got {other:?}"),
        }
    }

    #[test]
    fn rejections_say_why() {
        let book = ReactionBook::load();
        assert_eq!(reason(&book, "asdf"), Rejection::NotAFormula);
        assert_eq!(reason(&book, "xyz"), Rejection::NotAFormula);
        assert_eq!(reason(&book, "H2 ++ O2"), Rejection::NotAFormula);
        assert_eq!(reason(&book, "   "), Rejection::Empty);
        assert_eq!(reason(&book, "Hh"), Rejection::UnknownElement("Hh".into()));
        assert_eq!(
            reason(&book, "NaCl"),
            Rejection::NotInWorld { balanced: false }
        );
        assert_eq!(
            reason(&book, "2Na + Cl2 -> 2NaCl"),
            Rejection::NotInWorld { balanced: true }
        );
        assert_eq!(
            reason(&book, "Na + Cl2 -> NaCl"),
            Rejection::Unbalanced {
                counts: vec!["Cl: 2 on the left, 1 on the right".into()],
                pair: None,
            }
        );
        // Oxygen gas comes in pairs: read as O2, the atoms don't add up.
        assert_eq!(
            reason(&book, "H2O -> H2 + O"),
            Rejection::Unbalanced {
                counts: vec!["O: 1 on the left, 2 on the right".into()],
                pair: Some("O2".into()),
            }
        );
        assert_eq!(
            reason(&book, "2H2(l) + O2 -> 2H2O"),
            Rejection::WrongState("H2(l)".into())
        );
    }

    #[test]
    fn state_symbols_cast_the_same_reaction() {
        let book = ReactionBook::load();
        let with = plan(&book, "2H2(g) + O2(g) -> 2H2O(g)", false);
        let without = plan(&book, "2H2 + O2 -> 2H2O", false);
        let id = |p: &Plan| match p {
            Plan::React {
                reaction_id,
                typed,
                balanced,
                ..
            } => (reaction_id.clone(), typed.to_string(), *balanced),
            other => panic!("{other:?}"),
        };
        assert_eq!(id(&with), id(&without));
        assert_eq!(id(&with).0, "water_synthesis");
        // Lower-case typing with states still works.
        assert_eq!(
            id(&plan(&book, "2h2(g) + o2(g) -> 2h2o(g)", false)),
            id(&without)
        );
    }

    #[test]
    fn enthalpy_follows_amounts_and_states() {
        let book = ReactionBook::load();
        let dh = |t: &str| delta_h_for(&book, &parse_equation(t).unwrap()).unwrap();
        assert!((dh("2H2 + O2 -> 2H2O") + 483.6).abs() < 0.01);
        assert!((dh("H2 + 1/2O2 -> H2O") + 241.8).abs() < 0.01);
        assert!((dh("H2 + 0.5O2 -> H2O") + 241.8).abs() < 0.01);
        assert!((dh("4H2 + 2O2 -> 4H2O") + 967.2).abs() < 0.01);
        // Liquid water holds 44.0 kJ/mol less than steam (sourced in the book).
        assert!((dh("2H2(g) + O2(g) -> 2H2O(l)") + 571.6).abs() < 0.01);
        assert!((dh("2H2O(l) -> 2H2(g) + O2(g)") - 571.7).abs() < 0.01);
        assert!((dh("2H2O(g) -> 2H2 + O2") - 483.7).abs() < 0.01);
        assert!(delta_h_for(&book, &parse_equation("H2 + O2 -> H2O").unwrap()).is_none());
    }
}
