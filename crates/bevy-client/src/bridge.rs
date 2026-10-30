//! The link between the game and the page's JavaScript (`web/hud.js`).
//!
//! JS writes `window.__gameIn = { dx, dy, locked, paused, fire, px, py, cmds: [] }`:
//! raw
//! pointer-lock mouse deltas (from `movementX/Y`), whether the pointer is
//! locked, whether the pause overlay is up, the free pointer's position over
//! the canvas (-1 when away), and text commands (taps,
//! hotbar clicks, and the test hooks). The game drains it every frame.
//!
//! The game writes `window.__gameState`, a JSON string, every frame. The HUD
//! draws from it and a test can read it through `window.__game.state()`.
//!
//! On native builds these are no-ops (the game reads Bevy's own mouse input).

use bevy::prelude::*;

#[derive(Resource, Default, Debug)]
pub struct WebIn {
    /// Mouse movement this frame, CSS pixels.
    pub dx: f32,
    pub dy: f32,
    pub locked: bool,
    pub paused: bool,
    /// Left button held while the pointer is locked.
    pub fire: bool,
    /// Pointer position over the canvas (CSS pixels), when not locked.
    pub pointer: Option<(f32, f32)>,
    pub cmds: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::WebIn;
    use js_sys::{Array, Reflect};
    use wasm_bindgen::JsValue;

    fn get(o: &JsValue, k: &str) -> JsValue {
        Reflect::get(o, &JsValue::from_str(k)).unwrap_or(JsValue::UNDEFINED)
    }

    pub fn poll(input: &mut WebIn) {
        let Some(win) = web_sys::window() else {
            return;
        };
        let w: JsValue = win.into();
        let gi = get(&w, "__gameIn");
        if !gi.is_object() {
            return;
        }
        // A NaN or infinite delta would leave the camera NaN for good.
        let delta = |k: &str| {
            get(&gi, k)
                .as_f64()
                .filter(|v| v.is_finite())
                .unwrap_or(0.0)
                .clamp(-4000.0, 4000.0) as f32
        };
        input.dx = delta("dx");
        input.dy = delta("dy");
        let _ = Reflect::set(&gi, &"dx".into(), &0.0.into());
        let _ = Reflect::set(&gi, &"dy".into(), &0.0.into());
        input.locked = get(&gi, "locked").as_bool().unwrap_or(false);
        input.paused = get(&gi, "paused").as_bool().unwrap_or(false);
        input.fire = get(&gi, "fire").as_bool().unwrap_or(false);
        input.pointer = match (get(&gi, "px").as_f64(), get(&gi, "py").as_f64()) {
            (Some(x), Some(y)) if x >= 0.0 && y >= 0.0 => Some((x as f32, y as f32)),
            _ => None,
        };
        // Not cleared: `cast`/`tap` held back by the cooldown stay queued
        // (capped, so a flood of taps is dropped rather than stored).
        input.cmds.truncate(16);
        let cmds = get(&gi, "cmds");
        if Array::is_array(&cmds) {
            let arr = Array::from(&cmds);
            if arr.length() > 0 {
                for v in arr.iter() {
                    if let Some(s) = v.as_string() {
                        input.cmds.push(s);
                    }
                }
                let _ = Reflect::set(&gi, &"cmds".into(), &Array::new());
            }
        }
    }

    pub fn publish(json: &str) {
        if let Some(win) = web_sys::window() {
            let _ = Reflect::set(&win, &"__gameState".into(), &JsValue::from_str(json));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::WebIn;
    pub fn poll(input: &mut WebIn) {
        input.dx = 0.0;
        input.dy = 0.0;
        input.cmds.clear();
    }
    pub fn publish(_json: &str) {}
}

pub use imp::{poll, publish};
