//! The Formula Bench's judge (I-028): is a built formula the spell's formula,
//! at the player's age band? See `docs/loop/items/I-028/spec.md`, section 4.
//!
//! `judge` is the only checker in the core loop: the bench, I-029's worked
//! casts and I-030's trial all send it composed text. JS and client Rust
//! never decide right or wrong. `BenchView` is the one JSON shape
//! `window.__benchState` carries; building it here keeps escaping of
//! player-typed text in one tested place.
//!
//! Contract stub: `judge`, `recipe`, `learnable` and `prompt` have
//! placeholder bodies (the sim lane writes them); the types, `BenchView::closed`
//! and `BenchView::to_json` are final.

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
/// def's `slot` field, never an index) at `band`. Spec section 4.
pub fn judge(spells: &[SpellDef], slot: u8, band: Tier, text: &str) -> Verdict {
    // Contract stub: the sim lane writes the body.
    let _ = (spells, slot, band, text);
    Verdict {
        ok: false,
        code: Code::Parse,
        ..Verdict::default()
    }
}

/// One entry per term of the spell's target formula, in order.
pub fn recipe(def: &SpellDef) -> Vec<RecipeTerm> {
    // Contract stub: the sim lane writes the body.
    let _ = def;
    vec![]
}

/// True when `band >= def.tier_min`.
pub fn learnable(def: &SpellDef, band: Tier) -> bool {
    // Contract stub: the sim lane writes the body.
    let _ = (def, band);
    false
}

/// The spell's prompt for `band` (Adept uses the Apprentice prompt).
pub fn prompt(def: &SpellDef, band: Tier) -> Option<&str> {
    // Contract stub: the sim lane writes the body.
    let _ = (def, band);
    None
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
