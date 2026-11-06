//! The hit lens (graphics plan G4; Paawan, 2026-11-05: the Bench is the
//! centrepiece): when a spell lands, a round close-up beside the crosshair
//! shows its chemistry at the atom scale for a moment, drawn by
//! `web/hit-lens.js` from the Bench's show records (`show_frame_json`).
//!
//! A discovered spell shows its reaction; the shipped spells show what
//! they run: Fire the methane burning, Water, Frost and CO2 their molecule
//! (they work by a phase change or by displacing air, not a reaction).
//! At most one lens every `GAP_S` seconds, so rapid casting stays calm.

use bevy::prelude::*;

use crate::chem::Spell;

/// Seconds between two lenses.
pub const GAP_S: f32 = 4.0;

pub fn plugin(app: &mut App) {
    app.init_resource::<Lens>().add_systems(Update, on_land);
}

#[derive(Resource, Default)]
struct Lens {
    last: Option<f32>,
}

/// What the lens shows for a landed cast: `rx:<reaction id>` (atoms
/// regrouping) or `mol:<formula>` (one molecule).
pub fn key(spell: Spell, disc: Option<u16>) -> Option<String> {
    if let Some(i) = disc {
        let (e, _) = crate::discover::entry(usize::from(i))?;
        return Some(format!("rx:{}", e.id));
    }
    Some(match spell {
        Spell::Fire => "rx:methane_burns".into(),
        Spell::Water => "mol:H2O".into(),
        Spell::Frost => "mol:N2".into(),
        Spell::Foam => "mol:CO2".into(),
    })
}

fn on_land(time: Res<Time>, mut lens: ResMut<Lens>, mut landed: MessageReader<crate::SpellLanded>) {
    let now = time.elapsed_secs();
    for l in landed.read() {
        if lens.last.is_some_and(|t| now - t < GAP_S) {
            continue;
        }
        if let Some(k) = key(l.spell, l.disc) {
            lens.last = Some(now);
            show(&k);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn show(key: &str) {
    use wasm_bindgen::{JsCast, JsValue};
    use web_sys::js_sys::{Array, Reflect};
    let Some(win) = web_sys::window() else { return };
    let q = Reflect::get(&win, &"__hitLens".into()).unwrap_or(JsValue::UNDEFINED);
    // Not `Array::from`: that is JavaScript's `Array.from`, a copy, so every
    // key after the first went into a throwaway array (Paawan, 2026-11-05:
    // "i only see the 'atom' when water is used, and only the first time").
    let q = if Array::is_array(&q) {
        q.unchecked_into::<Array>()
    } else {
        let a = Array::new();
        let _ = Reflect::set(&win, &"__hitLens".into(), &a);
        a
    };
    q.push(&JsValue::from_str(key));
}

#[cfg(not(target_arch = "wasm32"))]
fn show(_key: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_spells_show_their_chemistry() {
        assert_eq!(key(Spell::Fire, None).as_deref(), Some("rx:methane_burns"));
        assert_eq!(key(Spell::Water, None).as_deref(), Some("mol:H2O"));
        assert_eq!(key(Spell::Frost, None).as_deref(), Some("mol:N2"));
    }

    #[test]
    fn a_discovery_shows_its_reaction() {
        let (e, _) = crate::discover::entry(0).unwrap();
        assert_eq!(key(Spell::Fire, Some(0)), Some(format!("rx:{}", e.id)));
    }
}
