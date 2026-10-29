//! Chemical equation parsing and balance checking.
//!
//! Accepts what a student would type: `2H2 + O2 -> 2H2O`, `2H₂ + O₂ → 2H₂O`,
//! `Ca(OH)2`, or a single term with no arrow (a "release" from a reagent jar).

use std::collections::BTreeMap;
use std::fmt;

/// Element symbols the parser accepts. Anything else is reported as unknown,
/// so a typo like `Hh2` never silently becomes a new element.
const ELEMENTS: &[&str] = &[
    "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S", "Cl",
    "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga", "Ge", "As",
    "Se", "Br", "Kr", "Rb", "Sr", "Ag", "Sn", "I", "Xe", "Cs", "Ba", "Pt", "Au", "Hg", "Pb", "U",
];

pub type Atoms = BTreeMap<String, u32>;

/// Longest formula the game accepts, in characters. Keeps parsing cheap and
/// bounds bracket nesting depth.
pub const MAX_INPUT_LEN: usize = 80;
/// Largest coefficient or subscript accepted.
const MAX_COUNT: u32 = 999;

#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub coef: u32,
    /// The formula without its coefficient, in plain ASCII (`H2O`).
    pub formula: String,
    pub atoms: Atoms,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Equation {
    pub reactants: Vec<Term>,
    /// Empty when the player typed a single side with no arrow.
    pub products: Vec<Term>,
    pub has_arrow: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    Empty,
    UnknownElement(String),
    BadCharacter(char),
    UnbalancedParentheses,
    EmptyTerm,
    TooManyArrows,
    TooLong,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "Write a formula first."),
            ParseError::UnknownElement(e) => write!(f, "\"{e}\" is not an element symbol."),
            ParseError::BadCharacter(c) => write!(f, "\"{c}\" can't be used in a formula."),
            ParseError::UnbalancedParentheses => write!(f, "A bracket is missing its partner."),
            ParseError::EmptyTerm => write!(f, "There's an empty spot next to a + sign."),
            ParseError::TooManyArrows => write!(f, "Use only one arrow."),
            ParseError::TooLong => write!(f, "That formula is too long."),
        }
    }
}

/// Turns unicode subscripts and arrow variants into plain ASCII.
pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '₀'..='₉' => out.push(char::from(b'0' + (c as u32 - '₀' as u32) as u8)),
            '→' | '⟶' | '⇒' => out.push_str("->"),
            _ => out.push(c),
        }
    }
    out.replace("=>", "->").replace('=', "->")
}

pub fn parse_equation(input: &str) -> Result<Equation, ParseError> {
    if input.chars().count() > MAX_INPUT_LEN {
        return Err(ParseError::TooLong);
    }
    let text = normalize(input);
    if text.trim().is_empty() {
        return Err(ParseError::Empty);
    }
    let sides: Vec<&str> = text.split("->").collect();
    match sides.len() {
        1 => Ok(Equation {
            reactants: parse_side(sides[0])?,
            products: vec![],
            has_arrow: false,
        }),
        2 => {
            let products = if sides[1].trim().is_empty() {
                vec![]
            } else {
                parse_side(sides[1])?
            };
            Ok(Equation {
                reactants: parse_side(sides[0])?,
                products,
                has_arrow: true,
            })
        }
        _ => Err(ParseError::TooManyArrows),
    }
}

fn parse_side(side: &str) -> Result<Vec<Term>, ParseError> {
    side.split('+').map(|t| parse_term(t.trim())).collect()
}

pub fn parse_term(raw: &str) -> Result<Term, ParseError> {
    let s: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() {
        return Err(ParseError::EmptyTerm);
    }
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    let formula = s[digits.len()..].to_string();
    if formula.is_empty() {
        return Err(ParseError::EmptyTerm);
    }
    let coef = if digits.is_empty() {
        1
    } else {
        digits
            .parse::<u32>()
            .unwrap_or(MAX_COUNT)
            .clamp(1, MAX_COUNT)
    };
    let atoms = parse_formula(&formula)?;
    Ok(Term {
        coef,
        formula,
        atoms,
    })
}

/// Parses a molecular formula such as `Ca(OH)2` into atom counts.
pub fn parse_formula(formula: &str) -> Result<Atoms, ParseError> {
    let chars: Vec<char> = formula.chars().collect();
    let mut pos = 0;
    let atoms = parse_group(&chars, &mut pos)?;
    if pos != chars.len() {
        return Err(ParseError::UnbalancedParentheses);
    }
    Ok(atoms)
}

fn parse_group(chars: &[char], pos: &mut usize) -> Result<Atoms, ParseError> {
    let mut atoms = Atoms::new();
    while *pos < chars.len() {
        let c = chars[*pos];
        if c == '(' {
            *pos += 1;
            let inner = parse_group(chars, pos)?;
            if *pos >= chars.len() || chars[*pos] != ')' {
                return Err(ParseError::UnbalancedParentheses);
            }
            *pos += 1;
            let n = read_number(chars, pos);
            for (el, k) in inner {
                let e = atoms.entry(el).or_insert(0);
                *e = e.saturating_add(k.saturating_mul(n));
            }
        } else if c == ')' {
            return Ok(atoms);
        } else if c.is_ascii_uppercase() {
            let mut sym = c.to_string();
            *pos += 1;
            while *pos < chars.len() && chars[*pos].is_ascii_lowercase() {
                sym.push(chars[*pos]);
                *pos += 1;
            }
            if !ELEMENTS.contains(&sym.as_str()) {
                return Err(ParseError::UnknownElement(sym));
            }
            let n = read_number(chars, pos);
            let e = atoms.entry(sym).or_insert(0);
            *e = e.saturating_add(n);
        } else {
            return Err(ParseError::BadCharacter(c));
        }
    }
    Ok(atoms)
}

fn read_number(chars: &[char], pos: &mut usize) -> u32 {
    let start = *pos;
    while *pos < chars.len() && chars[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if start == *pos {
        1
    } else {
        // Capped so silly inputs like H99999999999 can't overflow the counts.
        chars[start..*pos]
            .iter()
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(MAX_COUNT)
            .min(MAX_COUNT)
    }
}

pub fn side_atoms(terms: &[Term]) -> Atoms {
    let mut total = Atoms::new();
    for t in terms {
        for (el, n) in &t.atoms {
            let e = total.entry(el.clone()).or_insert(0);
            *e = e.saturating_add(n.saturating_mul(t.coef));
        }
    }
    total
}

impl Equation {
    pub fn is_balanced(&self) -> bool {
        !self.products.is_empty() && side_atoms(&self.reactants) == side_atoms(&self.products)
    }

    /// The same species with different coefficients, if small whole numbers
    /// (1 to 8) can balance it. Used for the youngest tier and for hints.
    pub fn balanced_version(&self) -> Option<Equation> {
        let n = self.reactants.len() + self.products.len();
        if self.products.is_empty() || n > 5 {
            return None;
        }
        let mut coefs = vec![1u32; n];
        loop {
            let mut candidate = self.clone();
            for (i, t) in candidate
                .reactants
                .iter_mut()
                .chain(candidate.products.iter_mut())
                .enumerate()
            {
                t.coef = coefs[i];
            }
            if candidate.is_balanced() {
                return Some(candidate);
            }
            // Count through every combination of coefficients 1..=8.
            let mut i = 0;
            loop {
                if i == n {
                    return None;
                }
                coefs[i] += 1;
                if coefs[i] <= 8 {
                    break;
                }
                coefs[i] = 1;
                i += 1;
            }
        }
    }

    /// A short, kid-readable comparison of atoms on each side, e.g.
    /// "H: 2 on the left, 4 on the right".
    pub fn imbalance_report(&self) -> Vec<String> {
        let left = side_atoms(&self.reactants);
        let right = side_atoms(&self.products);
        let mut els: Vec<&String> = left.keys().chain(right.keys()).collect();
        els.sort();
        els.dedup();
        els.into_iter()
            .filter_map(|el| {
                let l = left.get(el).copied().unwrap_or(0);
                let r = right.get(el).copied().unwrap_or(0);
                (l != r).then(|| format!("{el}: {l} on the left, {r} on the right"))
            })
            .collect()
    }

    /// Species formulas on each side, sorted, ignoring coefficients.
    pub fn species_key(&self) -> (Vec<String>, Vec<String>) {
        let mut l: Vec<String> = self.reactants.iter().map(|t| t.formula.clone()).collect();
        let mut r: Vec<String> = self.products.iter().map(|t| t.formula.clone()).collect();
        l.sort();
        r.sort();
        (l, r)
    }
}

impl fmt::Display for Equation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side = |terms: &[Term]| {
            terms
                .iter()
                .map(|t| {
                    if t.coef == 1 {
                        t.formula.clone()
                    } else {
                        format!("{}{}", t.coef, t.formula)
                    }
                })
                .collect::<Vec<_>>()
                .join(" + ")
        };
        if self.has_arrow {
            write!(f, "{} -> {}", side(&self.reactants), side(&self.products))
        } else {
            write!(f, "{}", side(&self.reactants))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_water_synthesis() {
        let eq = parse_equation("2H2 + O2 -> 2H2O").unwrap();
        assert_eq!(eq.reactants.len(), 2);
        assert_eq!(eq.reactants[0].coef, 2);
        assert_eq!(eq.products[0].atoms.get("O"), Some(&1));
        assert!(eq.is_balanced());
    }

    #[test]
    fn accepts_unicode_subscripts_and_arrows() {
        let eq = parse_equation("2H₂ + O₂ → 2H₂O").unwrap();
        assert!(eq.is_balanced());
    }

    #[test]
    fn detects_unbalanced() {
        let eq = parse_equation("H2 + O2 -> H2O").unwrap();
        assert!(!eq.is_balanced());
        assert_eq!(
            eq.imbalance_report(),
            vec!["O: 2 on the left, 1 on the right"]
        );
    }

    #[test]
    fn auto_balances_methane_combustion() {
        let eq = parse_equation("CH4 + O2 -> CO2 + H2O").unwrap();
        let fixed = eq.balanced_version().unwrap();
        assert_eq!(fixed.to_string(), "CH4 + 2O2 -> CO2 + 2H2O");
    }

    #[test]
    fn parses_parentheses() {
        let atoms = parse_formula("Ca(OH)2").unwrap();
        assert_eq!(atoms.get("O"), Some(&2));
        assert_eq!(atoms.get("H"), Some(&2));
        assert_eq!(atoms.get("Ca"), Some(&1));
    }

    #[test]
    fn rejects_unknown_elements() {
        assert_eq!(
            parse_formula("Hh2"),
            Err(ParseError::UnknownElement("Hh".into()))
        );
    }

    #[test]
    fn hostile_input_never_panics() {
        // A small fuzz loop over random strings drawn from formula-like characters.
        let alphabet: Vec<char> = "HOCNaClFe()0123456789+-> =→₂₉ xyz\u{0}".chars().collect();
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..20_000 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let len = (seed % 90) as usize;
            let mut s = String::new();
            let mut r = seed;
            for _ in 0..len {
                r = r
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                s.push(alphabet[(r >> 33) as usize % alphabet.len()]);
            }
            if let Ok(eq) = parse_equation(&s) {
                let _ = eq.is_balanced();
                let _ = eq.imbalance_report();
                let _ = eq.balanced_version();
            }
        }
    }

    #[test]
    fn huge_numbers_are_capped_not_overflowed() {
        let eq = parse_equation("99999999999(H99999999999)99999999 -> H2").unwrap();
        assert!(!eq.is_balanced());
        assert_eq!(parse_equation(&"H".repeat(200)), Err(ParseError::TooLong));
    }

    #[test]
    fn single_term_is_a_release() {
        let eq = parse_equation("H2O").unwrap();
        assert!(!eq.has_arrow);
        assert!(!eq.is_balanced());
    }
}
