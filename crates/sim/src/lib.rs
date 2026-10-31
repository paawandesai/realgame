//! Matter, chemistry and game rules for the chemistry spell sandbox.
//!
//! Pure Rust so the same code can run in the browser (as WebAssembly) and,
//! later, on authoritative game servers.

#![deny(unsafe_code)]

pub mod formula;
#[cfg(feature = "game")]
pub mod game;
#[cfg(feature = "game")]
pub mod grid;
#[cfg(feature = "game")]
pub mod palette;
pub mod reactions;
pub mod species;
pub mod spell;
pub mod spellbook;
pub mod tier;
// wasm-bindgen's generated glue uses unsafe internally; our own code here does not.
#[cfg(feature = "web")]
#[allow(unsafe_code)]
pub mod web;
