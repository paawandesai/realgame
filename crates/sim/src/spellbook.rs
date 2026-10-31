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

/// The spells, sorted by slot.
pub struct SpellBook {
    pub spells: Vec<SpellDef>,
}

impl SpellBook {
    pub fn parse(json: &str, book: &ReactionBook) -> Result<SpellBook, SpellError> {
        let _ = book;
        let file: SpellFile =
            serde_json::from_str(json).map_err(|e| SpellError::Json(e.to_string()))?;
        let _ = file.version;
        let mut spells = file.spells;
        spells.sort_by_key(|s| s.slot);
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
