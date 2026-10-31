//! The 3D game's spells as data (`content/spells.json`), checked against the
//! sourced reaction table.
//!
//! Every spell is a real formula plus player text, and each one names the
//! sourced entry it rests on: a reaction it runs, a phase change that absorbs
//! heat, or a row of the science register it cites.

use crate::formula::{self, parse_equation};
use crate::reactions::ReactionBook;
use crate::tier::Tier;
use serde::{Deserialize, Deserializer};

pub const SPELLS_JSON: &str = include_str!("../../../content/spells.json");

/// A formula as content writes it, parsed once.
#[derive(Debug, Clone, PartialEq)]
pub struct Formula {
    pub text: String,
    pub equation: formula::Equation,
}

impl Formula {
    pub fn parse(s: &str) -> Result<Formula, formula::ParseError> {
        Ok(Formula {
            text: s.to_string(),
            equation: parse_equation(s)?,
        })
    }

    /// True when the formula has an arrow (a reaction, not a substance).
    pub fn is_reaction(&self) -> bool {
        self.equation.has_arrow
    }

    /// The single term, when there is no arrow and exactly one term.
    pub fn single(&self) -> Option<&formula::Term> {
        match (self.equation.has_arrow, self.equation.reactants.as_slice()) {
            (false, [t]) => Some(t),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for Formula {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Formula, D::Error> {
        let s = String::deserialize(d)?;
        Formula::parse(&s)
            .map_err(|e| serde::de::Error::custom(format!("formula \"{s}\" does not parse: {e}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    Reaction,
    PhaseChange,
    Register,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The spell's formula is exactly the referenced reaction.
    Runs,
    /// The spell is a substance whose phase change absorbs heat.
    AbsorbsHeat,
    /// A citation of a science-register row; not an engine check.
    Cites,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReactionRef {
    pub kind: RefKind,
    pub id: String,
    pub role: Role,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellDef {
    /// Permanent key (saved and sent by later items). Never renamed.
    pub id: String,
    pub slot: u8,
    /// The client's chemistry rule (`chem::Spell::from_effect_id`). Never saved.
    pub effect: String,
    pub name: String,
    /// The formula text shown in game state.
    pub label: String,
    pub hotbar_label: String,
    pub cast_caption: String,
    pub formula: Formula,
    pub tier_min: Tier,
    pub reaction: ReactionRef,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpellError {
    Json(String),
    BadId(String),
    DuplicateId(String),
    DuplicateSlot(u8),
    SlotGap,
    BadText { spell: String, field: &'static str },
    UnknownRef { spell: String, id: String },
    Unsourced { spell: String, id: String },
    TierBelowReaction { spell: String },
    RoleMismatch { spell: String, why: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpellFile {
    version: u32,
    #[serde(default)]
    #[allow(dead_code)]
    notes: Option<String>,
    spells: Vec<SpellDef>,
}

/// `^[a-z][a-z0-9_]{0,31}$`
fn is_content_id(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 32
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
}

/// `^C-[0-9]{3}$`, a claim row of `content/science-sources.md`.
fn is_register_id(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() == 5 && b.starts_with(b"C-") && b[2..].iter().all(u8::is_ascii_digit)
}

/// 1 to `max` characters, and nothing that could break the client's state
/// JSON, which interpolates some of these fields unescaped: no `"`, `\` or
/// control characters.
fn is_player_text(text: &str, max: usize) -> bool {
    let n = text.chars().count();
    (1..=max).contains(&n)
        && !text
            .chars()
            .any(|c| c == '"' || c == '\\' || c.is_control())
}

fn mismatch(s: &SpellDef, why: &str) -> SpellError {
    SpellError::RoleMismatch {
        spell: s.id.clone(),
        why: why.to_string(),
    }
}

fn unsourced(s: &SpellDef, source: &str) -> Result<(), SpellError> {
    if source.trim().chars().count() <= 10 {
        return Err(SpellError::Unsourced {
            spell: s.id.clone(),
            id: s.reaction.id.clone(),
        });
    }
    Ok(())
}

fn unknown(s: &SpellDef) -> SpellError {
    SpellError::UnknownRef {
        spell: s.id.clone(),
        id: s.reaction.id.clone(),
    }
}

/// Resolves a spell's reference and checks its role and tier rules.
fn check_reference(s: &SpellDef, book: &ReactionBook) -> Result<(), SpellError> {
    let r = &s.reaction;
    match r.kind {
        RefKind::Reaction => {
            let (reaction, eq) = book
                .reactions
                .iter()
                .find(|(x, _)| x.id == r.id)
                .ok_or_else(|| unknown(s))?;
            unsourced(s, &reaction.source)?;
            if r.role != Role::Runs {
                return Err(mismatch(s, "a reaction reference needs role runs"));
            }
            if !s.formula.is_reaction() {
                return Err(mismatch(s, "runs needs a formula with an arrow"));
            }
            if !s.formula.equation.is_balanced() {
                return Err(mismatch(s, "runs needs a balanced formula"));
            }
            if s.formula.equation.to_string() != eq.to_string() {
                return Err(mismatch(
                    s,
                    "runs needs exactly the referenced reaction's equation",
                ));
            }
            let floor = Tier::from_id(&reaction.tier_min).ok_or_else(|| {
                SpellError::Json(format!(
                    "reaction {} has unknown tier_min {:?}",
                    reaction.id, reaction.tier_min
                ))
            })?;
            if s.tier_min < floor {
                return Err(SpellError::TierBelowReaction {
                    spell: s.id.clone(),
                });
            }
        }
        RefKind::PhaseChange => {
            let phase = book
                .phase_changes
                .iter()
                .find(|p| p.id == r.id)
                .ok_or_else(|| unknown(s))?;
            unsourced(s, &phase.source)?;
            if r.role != Role::AbsorbsHeat {
                return Err(mismatch(
                    s,
                    "a phase_change reference needs role absorbs_heat",
                ));
            }
            let term = s
                .formula
                .single()
                .ok_or_else(|| mismatch(s, "absorbs_heat needs a single substance"))?;
            if term.formula != phase.substance {
                return Err(mismatch(
                    s,
                    "absorbs_heat needs the phase change's substance",
                ));
            }
            if term.state.is_none() {
                return Err(mismatch(s, "absorbs_heat needs a state symbol"));
            }
            if phase.latent_heat_kj_per_mol <= 0.0 {
                return Err(mismatch(s, "absorbs_heat needs a positive latent heat"));
            }
        }
        RefKind::Register => {
            if !is_register_id(&r.id) {
                return Err(unknown(s));
            }
            if r.role != Role::Cites {
                return Err(mismatch(s, "a register reference needs role cites"));
            }
        }
    }
    Ok(())
}

/// The spells, sorted by slot.
pub struct SpellBook {
    pub spells: Vec<SpellDef>,
}

impl SpellBook {
    /// Parses and validates `json` against the sourced reaction table.
    ///
    /// Checks run in this order and the first error wins: JSON and schema,
    /// id pattern (ids and effects), unique ids, unique slots, slots exactly
    /// `1..=n`, player text, then each spell's reference and tier.
    pub fn parse(json: &str, book: &ReactionBook) -> Result<SpellBook, SpellError> {
        let file: SpellFile =
            serde_json::from_str(json).map_err(|e| SpellError::Json(e.to_string()))?;
        if file.version != 1 {
            return Err(SpellError::Json(format!(
                "spells.json version {} is not supported (expected 1)",
                file.version
            )));
        }
        let mut spells = file.spells;
        for s in &spells {
            for id in [&s.id, &s.effect] {
                if !is_content_id(id) {
                    return Err(SpellError::BadId(id.clone()));
                }
            }
        }
        for (i, s) in spells.iter().enumerate() {
            if spells[..i].iter().any(|t| t.id == s.id) {
                return Err(SpellError::DuplicateId(s.id.clone()));
            }
        }
        for (i, s) in spells.iter().enumerate() {
            if spells[..i].iter().any(|t| t.slot == s.slot) {
                return Err(SpellError::DuplicateSlot(s.slot));
            }
        }
        spells.sort_by_key(|s| s.slot);
        let contiguous = spells
            .iter()
            .enumerate()
            .all(|(i, s)| usize::from(s.slot) == i + 1);
        if spells.is_empty() || spells.len() > 9 || !contiguous {
            return Err(SpellError::SlotGap);
        }
        for s in &spells {
            for (field, text, max) in [
                ("name", &s.name, 40),
                ("label", &s.label, 80),
                ("hotbar_label", &s.hotbar_label, 80),
                ("cast_caption", &s.cast_caption, 200),
            ] {
                if !is_player_text(text, max) {
                    return Err(SpellError::BadText {
                        spell: s.id.clone(),
                        field,
                    });
                }
            }
        }
        for s in &spells {
            check_reference(s, book)?;
        }
        Ok(SpellBook { spells })
    }

    pub fn load(book: &ReactionBook) -> SpellBook {
        match SpellBook::parse(SPELLS_JSON, book) {
            Ok(b) => b,
            Err(e) => panic!("content/spells.json is valid: {e:?}"),
        }
    }

    pub fn by_id(&self, id: &str) -> Option<&SpellDef> {
        self.spells.iter().find(|s| s.id == id)
    }

    pub fn by_slot(&self, slot: u8) -> Option<&SpellDef> {
        self.spells.iter().find(|s| s.slot == slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::State;
    use serde_json::{json, Value};

    fn shipped() -> SpellBook {
        SpellBook::load(&ReactionBook::load())
    }

    /// Parses the shipped file after `edit` changes it.
    fn mutated(edit: impl FnOnce(&mut Value)) -> Result<SpellBook, SpellError> {
        mutated_with(&ReactionBook::load(), edit)
    }

    fn mutated_with(
        book: &ReactionBook,
        edit: impl FnOnce(&mut Value),
    ) -> Result<SpellBook, SpellError> {
        let mut v: Value = serde_json::from_str(SPELLS_JSON).unwrap();
        edit(&mut v);
        SpellBook::parse(&v.to_string(), book)
    }

    fn spell(v: &mut Value, i: usize) -> &mut Value {
        &mut v["spells"][i]
    }

    fn err(r: Result<SpellBook, SpellError>) -> SpellError {
        match r {
            Ok(_) => panic!("expected an error, got Ok"),
            Err(e) => e,
        }
    }

    // Test 1.
    #[test]
    fn shipped_book_loads_in_slot_order() {
        let b = shipped();
        let ids: Vec<&str> = b.spells.iter().map(|s| s.id.as_str()).collect();
        let slots: Vec<u8> = b.spells.iter().map(|s| s.slot).collect();
        let effects: Vec<&str> = b.spells.iter().map(|s| s.effect.as_str()).collect();
        assert_eq!(ids, ["fire", "water", "frost", "co2"]);
        assert_eq!(slots, [1, 2, 3, 4]);
        assert_eq!(effects, ["fire", "water", "frost", "foam"]);
        assert_eq!(b.by_id("co2").unwrap().id, b.by_slot(4).unwrap().id);
        assert_eq!(b.by_id("co2").unwrap().slot, 4);
        assert!(b.by_id("foam").is_none());
    }

    // Test 2: the strings the client showed at the branch base, byte for byte.
    #[test]
    fn player_text_matches_the_base_exactly() {
        let want: [[&str; 4]; 4] = [
            [
                "Fire",
                "CH₄ + 2O₂ → CO₂ + 2H₂O",
                "CH₄ + 2O₂ → CO₂ + 2H₂O",
                "Fire spell: a gas flame. Methane burns: CH₄ + 2O₂ → CO₂ + 2H₂O, and lots of heat.",
            ],
            [
                "Water",
                "H₂O absorbs heat",
                "cools below ignition",
                "Water soaks up heat and cools things below their ignition temperature.",
            ],
            [
                "Frost",
                "N₂(l) boils at −196 °C",
                "N₂(l) boils at −196 °C",
                "Liquid nitrogen boils at −196 °C (77 K), pulling heat out of anything it touches.",
            ],
            [
                "CO₂",
                "CO₂ blanket, no O₂",
                "CO₂ displaces O₂",
                "CO₂ from an extinguisher is heavier than air: it smothers fire by pushing the oxygen away.",
            ],
        ];
        let b = shipped();
        assert_eq!(b.spells.len(), 4);
        for (s, w) in b.spells.iter().zip(want) {
            assert_eq!(s.name, w[0], "name of {}", s.id);
            assert_eq!(s.label, w[1], "label of {}", s.id);
            assert_eq!(s.hotbar_label, w[2], "hotbar_label of {}", s.id);
            assert_eq!(s.cast_caption, w[3], "cast_caption of {}", s.id);
        }
    }

    // Test 3.
    #[test]
    fn every_spell_traces_to_a_sourced_entry() {
        let book = ReactionBook::load();
        let b = SpellBook::load(&book);
        let pairs: Vec<(&str, RefKind, &str, Role)> = b
            .spells
            .iter()
            .map(|s| {
                (
                    s.id.as_str(),
                    s.reaction.kind,
                    s.reaction.id.as_str(),
                    s.reaction.role,
                )
            })
            .collect();
        assert_eq!(
            pairs,
            [
                ("fire", RefKind::Reaction, "methane_combustion", Role::Runs),
                (
                    "water",
                    RefKind::PhaseChange,
                    "water_boiling",
                    Role::AbsorbsHeat
                ),
                (
                    "frost",
                    RefKind::PhaseChange,
                    "nitrogen_boiling",
                    Role::AbsorbsHeat
                ),
                ("co2", RefKind::Register, "C-035", Role::Cites),
            ]
        );
        for s in &b.spells {
            let source = match s.reaction.kind {
                RefKind::Reaction => {
                    &book
                        .reactions
                        .iter()
                        .find(|(r, _)| r.id == s.reaction.id)
                        .unwrap()
                        .0
                        .source
                }
                RefKind::PhaseChange => {
                    &book
                        .phase_changes
                        .iter()
                        .find(|p| p.id == s.reaction.id)
                        .unwrap()
                        .source
                }
                RefKind::Register => continue,
            };
            assert!(source.trim().len() > 10, "{} is sourced", s.id);
        }
        // The role rules pass: the shipped book parses with validation on.
        assert!(SpellBook::parse(SPELLS_JSON, &book).is_ok());
    }

    // Test 4a.
    #[test]
    fn unknown_reaction_id_is_rejected() {
        let e = err(mutated(|v| {
            spell(v, 0)["reaction"]["id"] = json!("methane_burns")
        }));
        assert!(matches!(e, SpellError::UnknownRef { .. }), "{e:?}");
    }

    // Test 4b.
    #[test]
    fn unbalanced_runs_formula_is_rejected() {
        let e = err(mutated(|v| {
            spell(v, 0)["formula"] = json!("CH4 + O2 -> CO2 + H2O")
        }));
        assert!(matches!(e, SpellError::RoleMismatch { .. }), "{e:?}");
    }

    // Test 4c.
    #[test]
    fn runs_needs_the_exact_reaction() {
        let e = err(mutated(|v| {
            spell(v, 0)["formula"] = json!("2H2 + O2 -> 2H2O")
        }));
        assert!(matches!(e, SpellError::RoleMismatch { .. }), "{e:?}");
    }

    // Test 4d.
    #[test]
    fn absorbs_heat_needs_the_same_substance() {
        let e = err(mutated(|v| spell(v, 2)["formula"] = json!("O2(l)")));
        assert!(matches!(e, SpellError::RoleMismatch { .. }), "{e:?}");
    }

    // Test 4e.
    #[test]
    fn register_id_must_match_the_pattern() {
        let e = err(mutated(|v| {
            spell(v, 3)["reaction"] = json!({"kind": "register", "id": "C35", "role": "cites"})
        }));
        assert!(matches!(e, SpellError::UnknownRef { .. }), "{e:?}");
    }

    // Test 4f.
    #[test]
    fn cites_and_register_only_go_together() {
        let e = err(mutated(
            |v| {
                spell(v, 3)["reaction"] =
                    json!({"kind": "reaction", "id": "carbon_combustion", "role": "cites"})
            },
        ));
        assert!(matches!(e, SpellError::RoleMismatch { .. }), "{e:?}");
        let e = err(mutated(|v| {
            spell(v, 3)["reaction"] = json!({"kind": "register", "id": "C-035", "role": "runs"})
        }));
        assert!(matches!(e, SpellError::RoleMismatch { .. }), "{e:?}");
    }

    // Test 4g.
    #[test]
    fn a_phase_change_is_not_a_reaction() {
        let e = err(mutated(
            |v| {
                spell(v, 1)["reaction"] =
                    json!({"kind": "reaction", "id": "water_boiling", "role": "absorbs_heat"})
            },
        ));
        assert!(matches!(e, SpellError::UnknownRef { .. }), "{e:?}");
    }

    // Test 4h.
    #[test]
    fn ids_and_slots_are_unique_and_contiguous() {
        let e = err(mutated(|v| spell(v, 1)["id"] = json!("fire")));
        assert_eq!(e, SpellError::DuplicateId("fire".into()));
        let e = err(mutated(|v| spell(v, 2)["slot"] = json!(2)));
        assert_eq!(e, SpellError::DuplicateSlot(2));
        let e = err(mutated(|v| spell(v, 3)["slot"] = json!(5)));
        assert_eq!(e, SpellError::SlotGap);
    }

    // Test 4i.
    #[test]
    fn spell_tier_cannot_be_below_its_reaction() {
        let e = err(mutated(|v| spell(v, 0)["tier_min"] = json!("sprout")));
        assert!(matches!(e, SpellError::TierBelowReaction { .. }), "{e:?}");
    }

    // Test 4j.
    #[test]
    fn text_that_could_break_state_json_is_rejected() {
        let e = err(mutated(|v| spell(v, 1)["name"] = json!("Wa\"ter")));
        assert!(
            matches!(e, SpellError::BadText { field: "name", .. }),
            "{e:?}"
        );
        let e = err(mutated(|v| {
            spell(v, 1)["cast_caption"] = json!("Water soaks\nup heat.")
        }));
        assert!(
            matches!(
                e,
                SpellError::BadText {
                    field: "cast_caption",
                    ..
                }
            ),
            "{e:?}"
        );
        let e = err(mutated(|v| spell(v, 1)["label"] = json!("H2O \\ heat")));
        assert!(
            matches!(e, SpellError::BadText { field: "label", .. }),
            "{e:?}"
        );
        let e = err(mutated(|v| spell(v, 1)["hotbar_label"] = json!("")));
        assert!(
            matches!(
                e,
                SpellError::BadText {
                    field: "hotbar_label",
                    ..
                }
            ),
            "{e:?}"
        );
    }

    // Test 4k.
    #[test]
    fn schema_errors_are_json_errors() {
        let e = err(mutated(|v| spell(v, 0)["colour"] = json!("red")));
        assert!(matches!(e, SpellError::Json(_)), "{e:?}");
        let e = err(mutated(|v| spell(v, 1)["tier_min"] = json!("toddler")));
        assert!(matches!(e, SpellError::Json(_)), "{e:?}");
        let e = err(mutated(|v| v["version"] = json!(2)));
        assert!(matches!(e, SpellError::Json(_)), "{e:?}");
        let e = err(mutated(|v| spell(v, 1)["formula"] = json!("Xx2")));
        match e {
            SpellError::Json(m) => assert!(m.contains("Xx2"), "{m}"),
            other => panic!("expected Json, got {other:?}"),
        }
    }

    // Test 4l.
    #[test]
    fn an_unsourced_reaction_is_rejected() {
        let mut book = ReactionBook::load();
        for (r, _) in book.reactions.iter_mut() {
            if r.id == "methane_combustion" {
                r.source = String::new();
            }
        }
        let e = err(mutated_with(&book, |_| {}));
        assert!(matches!(e, SpellError::Unsourced { .. }), "{e:?}");
    }

    // Test 4m.
    #[test]
    fn ids_must_be_lowercase_words() {
        let e = err(mutated(|v| spell(v, 0)["id"] = json!("Fire!")));
        assert_eq!(e, SpellError::BadId("Fire!".into()));
    }

    // Test 7.
    #[test]
    fn formulas_parse_from_unicode_and_ascii() {
        let a = Formula::parse("CH₄ + 2O₂ → CO₂ + 2H₂O").unwrap();
        let b = Formula::parse("CH4 + 2O2 -> CO2 + 2H2O").unwrap();
        assert_eq!(a.equation.to_string(), b.equation.to_string());
        assert!(a.is_reaction());
        let n = Formula::parse("N2(l)").unwrap();
        let t = n.single().expect("one term");
        assert_eq!(t.formula, "N2");
        assert_eq!(t.state, Some(State::Liquid));
        assert!(!n.is_reaction());
        assert!(a.single().is_none());

        let fire = shipped();
        let fire = fire.by_id("fire").unwrap();
        let label = Formula::parse(&fire.label).unwrap();
        assert_eq!(
            label.equation.to_string(),
            fire.formula.equation.to_string()
        );
    }
}
