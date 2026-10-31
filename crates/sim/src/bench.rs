//! The Formula Bench's judge (I-028): is a built formula the spell's formula,
//! at the player's age band? See `docs/loop/items/I-028/spec.md`, section 4.
//!
//! `judge` is the only checker in the core loop: the bench, I-029's worked
//! casts and I-030's trial all send it composed text. JS and client Rust
//! never decide right or wrong. `BenchView` is the one JSON shape
//! `window.__benchState` carries; building it here keeps escaping of
//! player-typed text in one tested place.
//!

use crate::formula::{parse_equation, side_atoms, Equation, ParseError, Term};
use crate::spellbook::SpellDef;
use crate::tier::Tier;
use serde::Serialize;

/// Why a build is right or not. Serializes as snake_case (`atoms_off`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    Ok,
    #[default]
    Parse,
    Empty,
    Element,
    NotSingle,
    AtomsOff,
    OtherSpell,
    StateMissing,
    WrongState,
    NoProducts,
    WrongSpecies,
    Unbalanced,
    NotForBand,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize)]
pub struct Verdict {
    pub ok: bool,
    pub code: Code,
    /// AtomsOff: (el, have, want). Unbalanced: (el, left, right). Sorted by element.
    pub diffs: Vec<(String, u32, u32)>,
    /// OtherSpell: the slot whose formula this builds; else 0.
    pub other: u8,
    /// Element: the symbol; Parse: the bad character (or ""); StateMissing: the
    /// formula; WrongState: the target state symbol. Player-typed: escaped by serde.
    pub bad: String,
    /// Ok on a multiple or a non-canonical formula: the canonical text; else "".
    pub simplest: String,
    /// Ok on a single substance typed with a coefficient > 1 (Apprentice+): that coefficient; else 0.
    pub coef: u32,
}

/// One term of a spell's target formula, for the Sprout and Seedling recipes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RecipeTerm {
    /// 0 before the arrow, 1 after.
    pub side: u8,
    pub coef: u32,
    /// ASCII, without state.
    pub formula: String,
    /// (element, count) in the order they appear in the formula.
    pub atoms: Vec<(String, u32)>,
    /// The state symbol (`l`, `g`, ...), if any.
    pub state: Option<String>,
}

/// Judges `text` as a build of the spell in `slot` (1-4, looked up by the
/// def's `slot` field, never an index) at `band`. Spec section 4; the first
/// matching rule wins.
pub fn judge(spells: &[SpellDef], slot: u8, band: Tier, text: &str) -> Verdict {
    // 1. Which spell, and may this band learn it?
    let Some(def) = spells.iter().find(|d| d.slot == slot) else {
        return fizzle(Code::NotForBand);
    };
    if !learnable(def, band) {
        return fizzle(Code::NotForBand);
    }
    // 2. Does it parse?
    let typed = match parse_equation(text) {
        Ok(eq) => eq,
        Err(ParseError::Empty) => return fizzle(Code::Empty),
        Err(ParseError::UnknownElement(el)) => {
            return Verdict {
                bad: el,
                ..fizzle(Code::Element)
            }
        }
        Err(ParseError::BadCharacter(c)) => {
            return Verdict {
                bad: c.to_string(),
                ..fizzle(Code::Parse)
            }
        }
        Err(_) => return fizzle(Code::Parse),
    };
    match def.formula.single() {
        Some(target) => judge_single(spells, def, target, band, &typed),
        None => judge_reaction(spells, def, &typed),
    }
}

fn fizzle(code: Code) -> Verdict {
    Verdict {
        ok: false,
        code,
        ..Verdict::default()
    }
}

fn ok() -> Verdict {
    Verdict {
        ok: true,
        code: Code::Ok,
        ..Verdict::default()
    }
}

/// Rule 3: the target is one substance (Water, Frost, CO2).
fn judge_single(
    spells: &[SpellDef],
    def: &SpellDef,
    target: &Term,
    band: Tier,
    typed: &Equation,
) -> Verdict {
    let u = match (typed.has_arrow, typed.reactants.as_slice()) {
        (false, [u]) => u,
        _ => return fizzle(Code::NotSingle),
    };
    // Atoms in one molecule; the coefficient (an amount) is ignored.
    if u.atoms != target.atoms {
        let other = spells
            .iter()
            .filter(|d| d.slot != def.slot)
            .find(|d| d.formula.single().is_some_and(|t| t.atoms == u.atoms));
        if let Some(other) = other {
            return Verdict {
                other: other.slot,
                ..fizzle(Code::OtherSpell)
            };
        }
        let mut els: Vec<&String> = u.atoms.keys().chain(target.atoms.keys()).collect();
        els.sort();
        els.dedup();
        let diffs = els
            .into_iter()
            .filter_map(|el| {
                let have = u.atoms.get(el).copied().unwrap_or(0);
                let want = target.atoms.get(el).copied().unwrap_or(0);
                (have != want).then(|| (el.clone(), have, want))
            })
            .collect();
        return Verdict {
            diffs,
            ..fizzle(Code::AtomsOff)
        };
    }
    // Sprout and Seedling never see state codes: the recipe gives the state.
    if band < Tier::Apprentice {
        return ok();
    }
    if let Some(want) = target.state {
        match u.state {
            None => {
                return Verdict {
                    bad: target.formula.clone(),
                    ..fizzle(Code::StateMissing)
                }
            }
            Some(s) if s != want => {
                return Verdict {
                    bad: want.symbol().to_string(),
                    ..fizzle(Code::WrongState)
                }
            }
            Some(_) => {}
        }
    }
    // The amount the player wrote: a whole number, or 0 for a fraction.
    let per = typed.per.max(1);
    let amount = if u.coef % per == 0 { u.coef / per } else { 0 };
    let canonical = u.formula == target.formula && per == 1 && u.coef == 1;
    Verdict {
        simplest: if canonical {
            String::new()
        } else {
            def.formula.equation.with_states()
        },
        coef: if amount > 1 { amount } else { 0 },
        ..ok()
    }
}

/// Rule 4: the target is a reaction (Fire). Typed states are not judged.
fn judge_reaction(spells: &[SpellDef], def: &SpellDef, typed: &Equation) -> Verdict {
    let target = &def.formula.equation;
    if !typed.has_arrow || typed.products.is_empty() {
        return fizzle(Code::NoProducts);
    }
    let key = typed.species_key();
    if key != target.species_key() {
        let other = spells
            .iter()
            .filter(|d| d.slot != def.slot && d.formula.is_reaction())
            .find(|d| d.formula.equation.species_key() == key);
        return match other {
            Some(other) => Verdict {
                other: other.slot,
                ..fizzle(Code::OtherSpell)
            },
            None => fizzle(Code::WrongSpecies),
        };
    }
    if !typed.is_balanced() {
        let left = side_atoms(&typed.reactants);
        let right = side_atoms(&typed.products);
        let mut els: Vec<&String> = left.keys().chain(right.keys()).collect();
        els.sort();
        els.dedup();
        let diffs = els
            .into_iter()
            .filter_map(|el| {
                let l = left.get(el).copied().unwrap_or(0);
                let r = right.get(el).copied().unwrap_or(0);
                (l != r).then(|| (el.clone(), l, r))
            })
            .collect();
        return Verdict {
            diffs,
            ..fizzle(Code::Unbalanced)
        };
    }
    let (simple, k) = typed.simplest();
    if coefs(&simple) != coefs(&target.simplest().0) {
        return fizzle(Code::WrongSpecies);
    }
    Verdict {
        simplest: if k > 1 || typed.per > 1 {
            target.with_states()
        } else {
            String::new()
        },
        ..ok()
    }
}

/// One side of an equation as sorted (formula, coefficient) pairs.
type Side<'a> = Vec<(&'a str, u32)>;

/// (formula, coefficient) per side, sorted: term order does not matter.
fn coefs(eq: &Equation) -> (Side<'_>, Side<'_>) {
    (sorted_coefs(&eq.reactants), sorted_coefs(&eq.products))
}

fn sorted_coefs(terms: &[Term]) -> Vec<(&str, u32)> {
    let mut v: Vec<(&str, u32)> = terms.iter().map(|t| (t.formula.as_str(), t.coef)).collect();
    v.sort_unstable();
    v
}

/// One entry per term of the spell's target formula, in order.
pub fn recipe(def: &SpellDef) -> Vec<RecipeTerm> {
    let eq = &def.formula.equation;
    let side = |terms: &[Term], side: u8| {
        terms
            .iter()
            .map(move |t| RecipeTerm {
                side,
                coef: t.coef,
                formula: t.formula.clone(),
                atoms: formula_order(t),
                state: t.state.map(|s| s.symbol().to_string()),
            })
            .collect::<Vec<_>>()
    };
    let mut out = side(&eq.reactants, 0);
    out.extend(side(&eq.products, 1));
    out
}

/// A term's atoms in the order their symbols first appear in its formula
/// (`Term.atoms` is a `BTreeMap`, which iterates alphabetically).
fn formula_order(t: &Term) -> Vec<(String, u32)> {
    let chars: Vec<char> = t.formula.chars().collect();
    let mut out: Vec<(String, u32)> = vec![];
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_uppercase() {
            let mut sym = chars[i].to_string();
            i += 1;
            while i < chars.len() && chars[i].is_ascii_lowercase() {
                sym.push(chars[i]);
                i += 1;
            }
            if !out.iter().any(|(e, _)| *e == sym) {
                if let Some(&n) = t.atoms.get(&sym) {
                    out.push((sym, n));
                }
            }
        } else {
            i += 1;
        }
    }
    // Anything the walk missed (never, for a parsed formula) goes last.
    for (el, &n) in &t.atoms {
        if !out.iter().any(|(e, _)| e == el) {
            out.push((el.clone(), n));
        }
    }
    out
}

/// True when `band >= def.tier_min`.
pub fn learnable(def: &SpellDef, band: Tier) -> bool {
    band >= def.tier_min
}

/// The spell's prompt for `band` (Adept uses the Apprentice prompt).
pub fn prompt(def: &SpellDef, band: Tier) -> Option<&str> {
    match band {
        Tier::Sprout => def.prompt_sprout.as_deref(),
        Tier::Seedling => def.prompt_seedling.as_deref(),
        Tier::Apprentice | Tier::Adept => def.prompt_apprentice.as_deref(),
    }
}

/// A spell in the bench's list, learnable at the band.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpellCard {
    pub slot: u8,
    pub id: String,
    pub name: String,
    /// ASCII, from the def, with state (`H2O(l)`, `CH4 + 2O2 -> CO2 + 2H2O`).
    pub formula: String,
    /// `unknown|learning|practised|mastered`.
    pub stage: String,
    pub prompt: String,
}

/// The last judged try.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResultView {
    /// Bumps on every judged `try`.
    pub n: u32,
    pub slot: u8,
    #[serde(flatten)]
    pub verdict: Verdict,
    /// True only when this try moved the spell from Unknown to Learning.
    pub learned: bool,
    /// The spell's stage after the try.
    pub stage: String,
}

/// What `window.__benchState` carries. Key order in the JSON is field order.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BenchView {
    /// Always 1.
    pub v: u8,
    #[serde(rename = "atBench")]
    pub at_bench: bool,
    pub open: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub band: Option<Tier>,
    /// Learnable at the band, slot order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spells: Option<Vec<SpellCard>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pick: Option<u8>,
    /// Sprout and Seedling only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe: Option<Vec<RecipeTerm>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ResultView>,
}

impl BenchView {
    /// The closed view: `{"v":1,"atBench":..,"open":false}`, plus `band` once set.
    pub fn closed(at_bench: bool, band: Option<Tier>) -> BenchView {
        BenchView {
            v: 1,
            at_bench,
            open: false,
            band,
            spells: None,
            pick: None,
            recipe: None,
            result: None,
        }
    }

    pub fn to_json(&self) -> String {
        // Every field is a string, number, bool, list or map with string
        // keys, so serializing cannot fail.
        serde_json::to_string(self).expect("BenchView always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::{parse_equation, side_atoms, Term};
    use crate::reactions::ReactionBook;
    use crate::spellbook::SpellBook;

    fn spells() -> Vec<SpellDef> {
        SpellBook::load(&ReactionBook::load()).spells
    }

    fn def(spells: &[SpellDef], slot: u8) -> &SpellDef {
        spells.iter().find(|d| d.slot == slot).unwrap()
    }

    const FIRE: u8 = 1;
    const WATER: u8 = 2;
    const FROST: u8 = 3;
    const CO2: u8 = 4;

    fn v(code: Code) -> Verdict {
        Verdict {
            ok: code == Code::Ok,
            code,
            ..Verdict::default()
        }
    }

    fn diffs(d: &[(&str, u32, u32)]) -> Vec<(String, u32, u32)> {
        d.iter().map(|(e, a, b)| (e.to_string(), *a, *b)).collect()
    }

    /// Test 3's rule: an `Ok` agrees with the rules engine. Also: never `Ok`
    /// below the spell's band, and at Apprentice+ the typed state matches.
    fn agrees(spells: &[SpellDef], slot: u8, band: Tier, text: &str, got: &Verdict) {
        if !got.ok {
            assert_ne!(got.code, Code::Ok, "{text:?}: ok false but code Ok");
            return;
        }
        assert_eq!(got.code, Code::Ok, "{text:?}");
        let d = def(spells, slot);
        assert!(band >= d.tier_min, "{text:?} Ok below the band");
        let typed = parse_equation(text).expect("an Ok parses");
        let target = &d.formula.equation;
        if d.formula.is_reaction() {
            assert!(typed.is_balanced(), "{text:?} Ok but unbalanced");
            assert_eq!(typed.species_key(), target.species_key(), "{text:?}");
        } else {
            assert!(!typed.has_arrow, "{text:?}");
            assert_eq!(typed.reactants.len(), 1, "{text:?}");
            let unit = |t: &Term| Term {
                coef: 1,
                ..t.clone()
            };
            let u = unit(&typed.reactants[0]);
            let t = unit(d.formula.single().unwrap());
            assert_eq!(
                side_atoms(std::slice::from_ref(&u)),
                side_atoms(std::slice::from_ref(&t)),
                "{text:?}"
            );
            if band >= Tier::Apprentice && t.state.is_some() {
                assert_eq!(u.state, t.state, "{text:?} Ok with the wrong state");
            }
        }
    }

    fn check(spells: &[SpellDef], slot: u8, band: Tier, text: &str) -> Verdict {
        let got = judge(spells, slot, band, text);
        agrees(spells, slot, band, text, &got);
        got
    }

    /// (slot, band, text, simplest, coef) for every Ok of test 1.
    fn test1_oks() -> Vec<(u8, Tier, &'static str, &'static str, u32)> {
        let mut out = vec![];
        for (slot, text) in [(WATER, "H2O"), (FROST, "N2"), (CO2, "CO2")] {
            out.push((slot, Tier::Sprout, text, "", 0));
            out.push((slot, Tier::Seedling, text, "", 0));
        }
        out.push((FIRE, Tier::Seedling, "CH4 + 2O2 -> CO2 + 2H2O", "", 0));
        for band in [Tier::Apprentice, Tier::Adept] {
            for (slot, text) in [
                (WATER, "H2O(l)"),
                (WATER, "H₂O(l)"),
                (FROST, "N2(l)"),
                (FROST, "N₂(l)"),
                (CO2, "CO2(g)"),
                (FIRE, "CH4 + 2O2 -> CO2 + 2H2O"),
                (FIRE, "CH₄ + 2O₂ → CO₂ + 2H₂O"),
                (FIRE, "CH4(g) + 2O2(g) -> CO2(g) + 2H2O(l)"),
            ] {
                out.push((slot, band, text, "", 0));
            }
            out.push((
                FIRE,
                band,
                "2CH4 + 4O2 -> 2CO2 + 4H2O",
                "CH4 + 2O2 -> CO2 + 2H2O",
                0,
            ));
            out.push((WATER, band, "OH2(l)", "H2O(l)", 0));
            out.push((WATER, band, "2H2O(l)", "H2O(l)", 2));
        }
        out
    }

    // Test 1.
    #[test]
    fn correct_builds_per_band_are_ok() {
        let s = spells();
        for (slot, band, text, simplest, coef) in test1_oks() {
            let got = check(&s, slot, band, text);
            let want = Verdict {
                simplest: simplest.to_string(),
                coef,
                ..v(Code::Ok)
            };
            assert_eq!(got, want, "slot {slot} {band:?} {text:?}");
        }
        // Adept gives the same verdicts as Apprentice.
        for (slot, band, text, _, _) in test1_oks() {
            if band == Tier::Apprentice {
                assert_eq!(
                    judge(&s, slot, Tier::Adept, text),
                    judge(&s, slot, Tier::Apprentice, text),
                    "{text:?}"
                );
            }
        }
    }

    // Test 2.
    #[test]
    fn wrong_builds_fizzle_with_the_specific_code() {
        let s = spells();
        let sp = Tier::Sprout;
        let se = Tier::Seedling;
        let ap = Tier::Apprentice;
        let cases: Vec<(u8, Tier, &str, Verdict)> = vec![
            (
                WATER,
                sp,
                "HO",
                Verdict {
                    diffs: diffs(&[("H", 1, 2)]),
                    ..v(Code::AtomsOff)
                },
            ),
            (
                WATER,
                sp,
                "H2O2",
                Verdict {
                    diffs: diffs(&[("O", 2, 1)]),
                    ..v(Code::AtomsOff)
                },
            ),
            (
                WATER,
                sp,
                "H2OC",
                Verdict {
                    diffs: diffs(&[("C", 1, 0)]),
                    ..v(Code::AtomsOff)
                },
            ),
            (WATER, sp, "", v(Code::Empty)),
            (
                FROST,
                sp,
                "H2O",
                Verdict {
                    other: WATER,
                    ..v(Code::OtherSpell)
                },
            ),
            (
                CO2,
                se,
                "N2",
                Verdict {
                    other: FROST,
                    ..v(Code::OtherSpell)
                },
            ),
            (
                FIRE,
                se,
                "CH4 + O2 -> CO2 + H2O",
                Verdict {
                    diffs: diffs(&[("H", 4, 2), ("O", 2, 3)]),
                    ..v(Code::Unbalanced)
                },
            ),
            (
                FROST,
                ap,
                "N2",
                Verdict {
                    bad: "N2".into(),
                    ..v(Code::StateMissing)
                },
            ),
            (
                FROST,
                ap,
                "N2(g)",
                Verdict {
                    bad: "l".into(),
                    ..v(Code::WrongState)
                },
            ),
            (
                WATER,
                ap,
                "H2O(g)",
                Verdict {
                    bad: "l".into(),
                    ..v(Code::WrongState)
                },
            ),
            (
                WATER,
                ap,
                "Xx2",
                Verdict {
                    bad: "Xx".into(),
                    ..v(Code::Element)
                },
            ),
            (WATER, ap, "H2 ++ O2", v(Code::Parse)),
            (FIRE, ap, "CH4 + 2O2", v(Code::NoProducts)),
            (FIRE, ap, "2H2 + O2 -> 2H2O", v(Code::WrongSpecies)),
            (WATER, ap, "H2O(l) -> H2O(g)", v(Code::NotSingle)),
            // States are given by the recipe below Apprentice.
            (FROST, sp, "N2(g)", v(Code::Ok)),
            (FROST, se, "N2(g)", v(Code::Ok)),
            (FIRE, sp, "CH4 + 2O2 -> CO2 + 2H2O", v(Code::NotForBand)),
            (FIRE, sp, "", v(Code::NotForBand)),
            (WATER, ap, "H2O(l)", v(Code::Ok)),
        ];
        for (slot, band, text, want) in cases {
            assert_eq!(
                check(&s, slot, band, text),
                want,
                "{slot} {band:?} {text:?}"
            );
        }
        // No def with that slot: NotForBand, never a panic.
        for slot in [0u8, 5, 9, 255] {
            for band in Tier::ALL {
                assert_eq!(judge(&s, slot, band, "H2O(l)"), v(Code::NotForBand));
            }
        }
        // A bad character is reported.
        assert_eq!(
            judge(&s, WATER, ap, "H2O$"),
            Verdict {
                bad: "$".into(),
                ..v(Code::Parse)
            }
        );
    }

    // Test 3b: written by hand from [S7]/[S4], not generated with the parser.
    #[test]
    fn hand_written_chemistry_table() {
        let s = spells();
        let oks: [(u8, &str); 12] = [
            (WATER, "H2O(l)"),                 // water is two hydrogens and one oxygen, liquid
            (WATER, "H₂O(l)"),                 // the same, written with subscripts
            (FROST, "N2(l)"),                  // nitrogen is diatomic; liquid at 77 K
            (FROST, "N₂(l)"),                  // the same, with a subscript
            (CO2, "CO2(g)"),                   // one carbon, two oxygens, a gas at room temperature
            (CO2, "CO₂(g)"),                   // the same, with a subscript
            (FIRE, "CH4 + 2O2 -> CO2 + 2H2O"), // methane combustion, balanced [S7]
            (FIRE, "CH₄ + 2O₂ → CO₂ + 2H₂O"),  // the same, Unicode
            (FIRE, "CH4(g) + 2O2(g) -> CO2(g) + 2H2O(l)"), // with states; the target has none
            (FIRE, "2O2 + CH4 -> 2H2O + CO2"), // term order does not change a reaction
            (FIRE, "2CH4 + 4O2 -> 2CO2 + 4H2O"), // a multiple is still balanced and proportional
            (WATER, "2H2O(l)"), // two water molecules are still water (amount is not judged)
        ];
        let not_oks: [(u8, &str); 12] = [
            (WATER, "H2O2(l)"),               // hydrogen peroxide, not water
            (WATER, "HO(l)"),                 // a hydroxyl radical, not water
            (CO2, "NO2(g)"),                  // nitrogen dioxide
            (CO2, "CO(g)"),                   // carbon monoxide
            (FROST, "N(l)"),                  // one nitrogen atom, not the N2 molecule
            (FIRE, "CH4 + O2 -> CO2 + 2H2O"), // O unbalanced: 2 before, 4 after
            (FIRE, "2H2 + O2 -> 2H2O"),       // hydrogen burning, not methane
            (FIRE, "CH4 + 2O2 -> CO + 2H2O"), // wrong product (CO); unbalanced too
            (FIRE, "CH4 -> C + 2H2"),         // decomposition, no oxygen
            (FROST, "N2O(l)"),                // nitrous oxide, not nitrogen
            (WATER, "H2O(s)"),                // ice: this spell is liquid water
            (CO2, "CO2(s)"),                  // dry ice: this spell is the gas
        ];
        for (slot, text) in oks {
            assert!(check(&s, slot, Tier::Apprentice, text).ok, "{text:?} is Ok");
        }
        for (slot, text) in not_oks {
            assert!(
                !check(&s, slot, Tier::Apprentice, text).ok,
                "{text:?} is not Ok"
            );
        }
    }

    /// A deterministic LCG (Numerical Recipes constants).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) as u32
        }
    }

    // Test 4.
    #[test]
    fn never_panics_never_a_false_ok() {
        let s = spells();
        let alphabet: Vec<char> = "HONCXx()lgs2 4+->→₂\"\\".chars().collect();
        let mut oks = 0u32;
        for seed in 0..20u64 {
            let mut rng = Lcg(seed);
            for _ in 0..20_000 {
                let len = rng.next() % 91;
                let text: String = (0..len)
                    .map(|_| alphabet[(rng.next() as usize) % alphabet.len()])
                    .collect();
                for slot in 0..=5u8 {
                    for band in Tier::ALL {
                        let got = check(&s, slot, band, &text);
                        oks += u32::from(got.ok);
                        let view = BenchView {
                            result: Some(ResultView {
                                n: 1,
                                slot,
                                verdict: got,
                                learned: false,
                                stage: "unknown".into(),
                            }),
                            ..BenchView::closed(false, Some(band))
                        };
                        let json = view.to_json();
                        let back: serde_json::Value =
                            serde_json::from_str(&json).expect("to_json parses back");
                        assert_eq!(back["result"]["slot"], u64::from(slot));
                    }
                }
            }
        }
        // Test 3 over the Oks of test 1 too (a positive control: the rule
        // is checked on real Oks, not only on fizzles).
        for (slot, band, text, _, _) in test1_oks() {
            assert!(check(&s, slot, band, text).ok, "{text:?}");
            oks += 1;
        }
        assert!(oks > 0);
    }

    // Test 5.
    #[test]
    fn recipe_lists_terms_in_order() {
        let s = spells();
        let a = |p: &[(&str, u32)]| -> Vec<(String, u32)> {
            p.iter().map(|(e, n)| (e.to_string(), *n)).collect()
        };
        assert_eq!(
            recipe(def(&s, WATER)),
            vec![RecipeTerm {
                side: 0,
                coef: 1,
                formula: "H2O".into(),
                atoms: a(&[("H", 2), ("O", 1)]),
                state: Some("l".into()),
            }]
        );
        assert_eq!(
            recipe(def(&s, FROST)),
            vec![RecipeTerm {
                side: 0,
                coef: 1,
                formula: "N2".into(),
                atoms: a(&[("N", 2)]),
                state: Some("l".into()),
            }]
        );
        assert_eq!(
            recipe(def(&s, CO2)),
            vec![RecipeTerm {
                side: 0,
                coef: 1,
                formula: "CO2".into(),
                atoms: a(&[("C", 1), ("O", 2)]),
                state: Some("g".into()),
            }]
        );
        let fire = recipe(def(&s, FIRE));
        assert_eq!(
            fire.iter().map(|t| t.side).collect::<Vec<_>>(),
            [0, 0, 1, 1]
        );
        assert_eq!(
            fire.iter().map(|t| t.coef).collect::<Vec<_>>(),
            [1, 2, 1, 2]
        );
        assert_eq!(
            fire.iter().map(|t| t.formula.as_str()).collect::<Vec<_>>(),
            ["CH4", "O2", "CO2", "H2O"]
        );
        assert_eq!(fire[0].atoms, a(&[("C", 1), ("H", 4)]));
        assert_eq!(fire[3].atoms, a(&[("H", 2), ("O", 1)]));
        assert!(fire.iter().all(|t| t.state.is_none()));
    }

    // Test 5, extra: formula order, not alphabetical (Atoms is a BTreeMap).
    #[test]
    fn recipe_atoms_follow_the_formula_not_the_alphabet() {
        let mut s = spells();
        let water = s.iter_mut().find(|d| d.slot == WATER).unwrap();
        water.formula = crate::spellbook::Formula::parse("OH2(l)").unwrap();
        let r = recipe(water);
        assert_eq!(r[0].atoms, vec![("O".to_string(), 1), ("H".to_string(), 2)]);
        water.formula = crate::spellbook::Formula::parse("Ca(OH)2").unwrap();
        let r = recipe(water);
        assert_eq!(
            r[0].atoms,
            vec![
                ("Ca".to_string(), 1),
                ("O".to_string(), 2),
                ("H".to_string(), 2)
            ]
        );
    }

    #[test]
    fn learnable_and_prompt_follow_the_band() {
        let s = spells();
        let fire = def(&s, FIRE);
        let water = def(&s, WATER);
        assert!(!learnable(fire, Tier::Sprout));
        assert!(learnable(fire, Tier::Seedling));
        assert!(learnable(fire, Tier::Adept));
        assert!(Tier::ALL.into_iter().all(|b| learnable(water, b)));
        assert_eq!(prompt(fire, Tier::Sprout), None);
        assert_eq!(
            prompt(water, Tier::Sprout),
            Some("Make water! Copy the balls in the picture.")
        );
        assert_eq!(
            prompt(water, Tier::Seedling),
            Some("Water: build one water molecule from atom tiles.")
        );
        let ap = "Water: write the formula of liquid water, with its state symbol.";
        assert_eq!(prompt(water, Tier::Apprentice), Some(ap));
        assert_eq!(prompt(water, Tier::Adept), Some(ap));
    }

    // Test 6.
    #[test]
    fn view_json_has_the_contract_shape() {
        assert_eq!(
            BenchView::closed(true, None).to_json(),
            r#"{"v":1,"atBench":true,"open":false}"#
        );
        assert_eq!(
            BenchView::closed(false, Some(Tier::Seedling)).to_json(),
            r#"{"v":1,"atBench":false,"open":false,"band":"seedling"}"#
        );
        let s = spells();
        let band = Tier::Seedling;
        // Cards built the way the client builds them: the def's formula text.
        let cards: Vec<SpellCard> = s
            .iter()
            .filter(|d| learnable(d, band))
            .map(|d| SpellCard {
                slot: d.slot,
                id: d.id.clone(),
                name: d.name.clone(),
                formula: d.formula.text.clone(),
                stage: "unknown".into(),
                prompt: prompt(d, band).unwrap_or_default().to_string(),
            })
            .collect();
        let formulas: Vec<&str> = cards.iter().map(|c| c.formula.as_str()).collect();
        assert_eq!(
            formulas,
            ["CH4 + 2O2 -> CO2 + 2H2O", "H2O(l)", "N2(l)", "CO2(g)"]
        );
        for d in &s {
            // ASCII and canonical: what `judge` would call the target's text.
            assert!(d.formula.text.is_ascii());
            assert_eq!(d.formula.text, d.formula.equation.with_states());
        }
        let view = BenchView {
            v: 1,
            at_bench: false,
            open: true,
            band: Some(band),
            spells: Some(cards),
            pick: Some(WATER),
            recipe: Some(recipe(def(&s, WATER))),
            result: Some(ResultView {
                n: 3,
                slot: WATER,
                verdict: Verdict {
                    bad: "\"\\".into(),
                    ..v(Code::Element)
                },
                learned: false,
                stage: "unknown".into(),
            }),
        };
        let json = view.to_json();
        let back: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(back["result"]["bad"], "\"\\");
        assert_eq!(back["result"]["code"], "element");
        assert_eq!(back["result"]["n"], 3);
        assert_eq!(back["spells"][1]["formula"], "H2O(l)");
        assert_eq!(back["recipe"][0]["atoms"][0][0], "H");
        assert_eq!(back["recipe"][0]["atoms"][0][1], 2);
        // Top-level keys in the Contract's order.
        let keys = [
            "\"v\":",
            "\"atBench\":",
            "\"open\":",
            "\"band\":",
            "\"spells\":",
            "\"pick\":",
            "\"recipe\":",
            "\"result\":",
        ];
        let pos: Vec<usize> = keys.iter().map(|k| json.find(k).unwrap()).collect();
        assert!(pos.windows(2).all(|w| w[0] < w[1]), "{json}");
        assert!(json.starts_with("{\"v\":1,\"atBench\":false,\"open\":true,"));
        // The result's keys: n, slot, the flattened verdict, learned, stage.
        let r = &json[pos[7]..];
        let rk = [
            "\"n\":",
            "\"slot\":",
            "\"ok\":",
            "\"code\":",
            "\"diffs\":",
            "\"other\":",
            "\"bad\":",
            "\"simplest\":",
            "\"coef\":",
            "\"learned\":",
            "\"stage\":",
        ];
        let rp: Vec<usize> = rk.iter().map(|k| r.find(k).unwrap()).collect();
        assert!(rp.windows(2).all(|w| w[0] < w[1]), "{r}");
    }
}
