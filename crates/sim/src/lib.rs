//! Matter, chemistry and game rules for the chemistry spell sandbox.
//!
//! Pure Rust so the same code can run in the browser (as WebAssembly) and,
//! later, on authoritative game servers.

#![deny(unsafe_code)]

pub mod chem;
pub mod events;
pub mod formula;
pub mod game;
pub mod grid;
pub mod palette;
pub mod reactions;
pub mod species;
pub mod spell;
// wasm-bindgen's generated glue uses unsafe internally; our own code here does not.
#[allow(unsafe_code)]
pub mod web;
