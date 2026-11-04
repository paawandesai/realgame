//! The browser surface for the Bench free play page (`bench-free.html`).
//! Everything goes in and out as JSON strings, so the page needs no types.

use crate::{Codex, Need, Outcome};
use realgame_sim::formula::parse_equation;
use realgame_sim::tier::Tier;
use serde_json::json;
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

fn codex() -> &'static Codex {
    static C: OnceLock<Codex> = OnceLock::new();
    C.get_or_init(Codex::load)
}

fn band(s: &str) -> Tier {
    Tier::from_id(s).unwrap_or(Tier::Apprentice)
}

fn needs(csv: &str) -> Vec<Need> {
    csv.split(',')
        .filter_map(|s| Need::from_id(s.trim()))
        .collect()
}

/// The whole codex: families and entries (the page hides what a band can't
/// see yet).
#[wasm_bindgen]
pub fn codex_json() -> String {
    let c = codex();
    json!({ "families": c.families, "reactions": c.entries }).to_string()
}

/// Element display data and everyday substance names
/// (`content/chem/names.json`).
#[wasm_bindgen]
pub fn names_json() -> String {
    crate::NAMES_JSON.to_string()
}

/// Judges `input`, which is either a whole equation (`2H2 + O2 -> 2H2O`) or
/// just what is in the chamber (`2H2 + O2`). `have` is a comma list of
/// needs; `band_id` is `sprout`, `seedling`, `apprentice` or `adept`.
#[wasm_bindgen]
pub fn judge_json(input: &str, have: &str, band_id: &str) -> String {
    let c = codex();
    let eq = match parse_equation(input) {
        Ok(eq) => eq,
        Err(e) => return json!({ "kind": "parse_error", "message": e.to_string() }).to_string(),
    };
    let (have, band) = (needs(have), band(band_id));
    let outcome = if eq.has_arrow {
        c.judge(&eq, &have, band)
    } else {
        match c.run(&eq.reactants, &have, band) {
            Some((_, o)) => o,
            None => Outcome::NoReaction {
                reason: "Nothing in our reaction data happens with just these.".into(),
            },
        }
    };
    serde_json::to_string(&outcome).unwrap_or_else(|_| "{}".into())
}

/// Keys for looking `input` up in the lab records index
/// (`chem-literature.json`): `{"key": "..."}` for a whole equation (null when
/// nothing is left once shared species cancel), plus `"left"`, the reactant
/// half. `{"kind":"parse_error"}` if it doesn't parse.
#[wasm_bindgen]
pub fn literature_key_json(input: &str) -> String {
    match parse_equation(input) {
        Ok(eq) => json!({
            "key": if eq.has_arrow { crate::literature::key(&eq) } else { None },
            "left": crate::literature::reactant_key(&eq.reactants),
        })
        .to_string(),
        Err(e) => json!({ "kind": "parse_error", "message": e.to_string() }).to_string(),
    }
}

/// Every codex reaction's spell effect (`discover::Effects`, I-053), keyed
/// by reaction id, so the page can say what a saved discovery casts as.
#[wasm_bindgen]
pub fn effects_json() -> String {
    let fx = crate::discover::Effects::load();
    let map: serde_json::Map<String, serde_json::Value> = fx
        .all(codex())
        .into_iter()
        .map(|f| (f.reaction_id.clone(), json!(f)))
        .collect();
    serde_json::Value::Object(map).to_string()
}
