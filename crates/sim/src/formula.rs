//! Chemical equation parsing and balance checking.
//!
//! Accepts what a student would type: `2H2 + O2 -> 2H2O`, `2H₂ + O₂ → 2H₂O`,
//! `Ca(OH)2`, or a single term with no arrow (a "release" from a reagent jar).
//! State symbols (`H2O(g)`, `NaCl(aq)`) and simple fractional coefficients
//! (`1/2O2`, `0.5O2`, `½O2`) are accepted too.

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

/// Largest denominator accepted in a fractional coefficient (`1/2`, `0.25`).
const MAX_DENOMINATOR: u32 = 12;

/// A state symbol written after a formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Solid,
    Liquid,
    Gas,
    Aqueous,
}

impl State {
    pub fn symbol(self) -> &'static str {
        match self {
            State::Solid => "s",
            State::Liquid => "l",
            State::Gas => "g",
            State::Aqueous => "aq",
        }
    }

    /// Reads `s`, `l`, `g` or `aq` (any case for `l`, `g`, `aq`; `s` only in
    /// lower case, so `(S)` stays a sulfur group).
    fn from_symbol(text: &str) -> Option<State> {
        match text {
            "s" => Some(State::Solid),
            "l" | "L" => Some(State::Liquid),
            "g" | "G" => Some(State::Gas),
            _ if text.eq_ignore_ascii_case("aq") => Some(State::Aqueous),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub coef: u32,
    /// The formula without its coefficient or state, in plain ASCII (`H2O`).
    pub formula: String,
    pub atoms: Atoms,
    /// The state symbol the player wrote, if any.
    pub state: Option<State>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Equation {
    pub reactants: Vec<Term>,
    /// Empty when the player typed a single side with no arrow.
    pub products: Vec<Term>,
    pub has_arrow: bool,
    /// Fractional coefficients are stored scaled up to whole numbers; the
    /// equation as written is the stored one divided by `per`
    /// (`H2 + 1/2O2 -> H2O` is stored as `2H2 + O2 -> 2H2O` with `per` 2).
    pub per: u32,
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
    BadCoefficient,
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
            ParseError::BadCoefficient => {
                write!(f, "Use whole numbers or simple fractions like 1/2.")
            }
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
            '½' => out.push_str("1/2"),
            '¼' => out.push_str("1/4"),
            '¾' => out.push_str("3/4"),
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
    let (reactants, products, has_arrow) = match sides.len() {
        1 => (parse_side(sides[0])?, vec![], false),
        2 => {
            let products = if sides[1].trim().is_empty() {
                vec![]
            } else {
                parse_side(sides[1])?
            };
            (parse_side(sides[0])?, products, true)
        }
        _ => return Err(ParseError::TooManyArrows),
    };
    // Scale fractional coefficients up to whole numbers by the least common
    // multiple of their denominators.
    let per = reactants
        .iter()
        .chain(products.iter())
        .fold(1u32, |l, (_, den)| lcm(l, *den));
    if per > MAX_DENOMINATOR {
        return Err(ParseError::BadCoefficient);
    }
    let whole = |side: Vec<(Term, u32)>| -> Vec<Term> {
        side.into_iter()
            .map(|(mut t, den)| {
                t.coef = t.coef.saturating_mul(per / den).clamp(1, MAX_COUNT);
                t
            })
            .collect()
    };
    Ok(Equation {
        reactants: whole(reactants),
        products: whole(products),
        has_arrow,
        per,
    })
}

fn gcd_u32(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd_u32(b, a % b)
    }
}

fn lcm(a: u32, b: u32) -> u32 {
    (a / gcd_u32(a, b).max(1)).saturating_mul(b)
}

fn parse_side(side: &str) -> Result<Vec<(Term, u32)>, ParseError> {
    side.split('+')
        .map(|t| parse_scaled_term(t.trim()))
        .collect()
}

/// Reads a leading coefficient: `2`, `1/2`, `0.5` or nothing. Returns the
/// coefficient as a reduced fraction and the number of bytes it used.
fn read_coefficient(s: &str) -> Result<(u32, u32, usize), ParseError> {
    let b = s.as_bytes();
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let num = |from: usize, len: usize| -> u64 {
        s[from..from + len]
            .parse::<u64>()
            .unwrap_or(u64::MAX)
            .min(u64::from(MAX_COUNT) * 1000)
    };
    let n1 = digits(0);
    if n1 == 0 {
        return Ok((1, 1, 0));
    }
    let a = num(0, n1);
    let (top, bottom, used) = match b.get(n1) {
        Some(b'/') => {
            let n2 = digits(n1 + 1);
            if n2 == 0 {
                return Err(ParseError::BadCoefficient);
            }
            (a, num(n1 + 1, n2), n1 + 1 + n2)
        }
        Some(b'.') => {
            let n2 = digits(n1 + 1);
            if n2 == 0 || n2 > 3 {
                return Err(ParseError::BadCoefficient);
            }
            let scale = 10u64.pow(n2 as u32);
            (a * scale + num(n1 + 1, n2), scale, n1 + 1 + n2)
        }
        _ => (a, 1, n1),
    };
    if bottom == 0 || top == 0 {
        return if bottom == 0 {
            Err(ParseError::BadCoefficient)
        } else {
            // "0H2" has always meant one molecule (clamped); keep that.
            Ok((1, 1, used))
        };
    }
    let g = {
        let (mut x, mut y) = (top, bottom);
        while y != 0 {
            (x, y) = (y, x % y);
        }
        x
    };
    let (top, bottom) = (top / g, bottom / g);
    if bottom > u64::from(MAX_DENOMINATOR) {
        return Err(ParseError::BadCoefficient);
    }
    Ok((top.min(u64::from(MAX_COUNT)) as u32, bottom as u32, used))
}

/// Splits a trailing state symbol, `(g)`, off a term.
fn split_state(s: &str) -> (&str, Option<State>) {
    if let (Some(open), true) = (s.rfind('('), s.ends_with(')')) {
        if let Some(state) = State::from_symbol(&s[open + 1..s.len() - 1]) {
            return (&s[..open], Some(state));
        }
    }
    (s, None)
}

fn parse_scaled_term(raw: &str) -> Result<(Term, u32), ParseError> {
    let s: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() {
        return Err(ParseError::EmptyTerm);
    }
    let (top, bottom, used) = read_coefficient(&s)?;
    let (formula, state) = split_state(&s[used..]);
    if formula.is_empty() {
        return Err(ParseError::EmptyTerm);
    }
    let atoms = parse_formula(formula)?;
    Ok((
        Term {
            coef: top.clamp(1, MAX_COUNT),
            formula: formula.to_string(),
            atoms,
            state,
        },
        bottom,
    ))
}

/// Parses one term with a whole-number coefficient (`2H2O`, `H2O(l)`).
pub fn parse_term(raw: &str) -> Result<Term, ParseError> {
    match parse_scaled_term(raw)? {
        (t, 1) => Ok(t),
        _ => Err(ParseError::BadCoefficient),
    }
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

    /// The same equation with its coefficients divided by their greatest
    /// common divisor (4H2 + 2O2 -> 4H2O becomes 2H2 + O2 -> 2H2O), and the
    /// factor it was divided by.
    pub fn simplest(&self) -> (Equation, u32) {
        fn gcd(a: u32, b: u32) -> u32 {
            if b == 0 {
                a
            } else {
                gcd(b, a % b)
            }
        }
        let k = self
            .reactants
            .iter()
            .chain(self.products.iter())
            .fold(0, |g, t| gcd(g, t.coef))
            .max(1);
        let mut eq = self.clone();
        for t in eq.reactants.iter_mut().chain(eq.products.iter_mut()) {
            t.coef /= k;
        }
        (eq, k)
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

impl Equation {
    fn render(&self, states: bool, as_written: bool) -> String {
        let per = if as_written { self.per.max(1) } else { 1 };
        let side = |terms: &[Term]| {
            terms
                .iter()
                .map(|t| {
                    let g = gcd_u32(t.coef, per).max(1);
                    let (top, bottom) = (t.coef / g, per / g);
                    let coef = match (top, bottom) {
                        (1, 1) => String::new(),
                        (n, 1) => n.to_string(),
                        (n, d) => format!("{n}/{d}"),
                    };
                    let state = match (states, t.state) {
                        (true, Some(s)) => format!("({})", s.symbol()),
                        _ => String::new(),
                    };
                    format!("{coef}{}{state}", t.formula)
                })
                .collect::<Vec<_>>()
                .join(" + ")
        };
        if self.has_arrow {
            format!("{} -> {}", side(&self.reactants), side(&self.products))
        } else {
            side(&self.reactants)
        }
    }

    /// The equation with its state symbols, e.g. `2H2(g) + O2(g) -> 2H2O(g)`.
    pub fn with_states(&self) -> String {
        self.render(true, false)
    }

    /// The equation with state symbols, taking any the player left out from
    /// `book` (the same reaction as the sourced table writes it).
    pub fn with_states_from(&self, book: Option<&Equation>) -> String {
        let mut eq = self.clone();
        if let Some(book) = book {
            let all: Vec<&Term> = book.reactants.iter().chain(book.products.iter()).collect();
            for t in eq.reactants.iter_mut().chain(eq.products.iter_mut()) {
                if t.state.is_none() {
                    t.state = all
                        .iter()
                        .find(|k| k.formula == t.formula)
                        .and_then(|k| k.state);
                }
            }
        }
        eq.with_states()
    }

    /// The equation as the player wrote it, fractions included
    /// (`H2 + 1/2O2 -> H2O`).
    pub fn as_written(&self) -> String {
        self.render(false, true)
    }
}

/// Whole-number coefficients, no state symbols: the form the game casts.
impl fmt::Display for Equation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render(false, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplest_form_divides_out_the_common_factor() {
        let eq = parse_equation("4H2 + 2O2 -> 4H2O").unwrap();
        let (s, k) = eq.simplest();
        assert_eq!(s.to_string(), "2H2 + O2 -> 2H2O");
        assert_eq!(k, 2);
        let (s, k) = parse_equation("2H2 + O2 -> 2H2O").unwrap().simplest();
        assert_eq!((s.to_string().as_str(), k), ("2H2 + O2 -> 2H2O", 1));
    }

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
        let alphabet: Vec<char> = "HOCNaClFe()0123456789+-> =→₂₉ xyz\u{0}/.½glsaq"
            .chars()
            .collect();
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
    fn state_symbols_are_read_and_kept_apart_from_the_formula() {
        let eq = parse_equation("2H2(g) + O2(g) -> 2H2O(l)").unwrap();
        let plain = parse_equation("2H2 + O2 -> 2H2O").unwrap();
        assert_eq!(eq.species_key(), plain.species_key());
        assert_eq!(eq.to_string(), plain.to_string());
        assert!(eq.is_balanced());
        assert_eq!(eq.products[0].state, Some(State::Liquid));
        assert_eq!(eq.with_states(), "2H2(g) + O2(g) -> 2H2O(l)");
        let aq = parse_equation("NaCl(aq)").unwrap();
        assert_eq!(aq.reactants[0].state, Some(State::Aqueous));
        assert_eq!(aq.reactants[0].formula, "NaCl");
        // A group with a capital S is sulfur, not "solid".
        assert_eq!(parse_formula("Fe(S)2").unwrap().get("S"), Some(&2));
    }

    #[test]
    fn fractional_coefficients_scale_to_whole_numbers() {
        for text in ["H2 + 1/2O2 -> H2O", "H2 + 0.5O2 -> H2O", "H2 + ½O2 -> H2O"] {
            let eq = parse_equation(text).unwrap();
            assert_eq!(eq.per, 2, "{text}");
            assert_eq!(eq.to_string(), "2H2 + O2 -> 2H2O", "{text}");
            assert_eq!(eq.as_written(), "H2 + 1/2O2 -> H2O", "{text}");
            assert!(eq.is_balanced());
        }
        assert_eq!(parse_equation("2H2 + O2 -> 2H2O").unwrap().per, 1);
        assert_eq!(parse_equation("1/0H2"), Err(ParseError::BadCoefficient));
        assert_eq!(parse_equation("1/97H2"), Err(ParseError::BadCoefficient));
    }

    #[test]
    fn single_term_is_a_release() {
        let eq = parse_equation("H2O").unwrap();
        assert!(!eq.has_arrow);
        assert!(!eq.is_balanced());
    }
}
