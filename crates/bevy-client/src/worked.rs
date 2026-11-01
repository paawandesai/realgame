//! Worked casts, then instant casts (I-029).
//!
//! Before a spell is Mastered, a click opens a small builder (`web/worked.js`)
//! and the world holds until the player rebuilds the formula; a correct build
//! casts at the click's target. A Mastered spell, a spell the player's band
//! cannot learn (the Sprout rule) and every spell while no band is known cast
//! at once. See `docs/loop/items/I-029/spec.md`.
//!
//! Pure Rust with no Bevy types and no crate imports, so it is tested with
//! `rustc --edition 2021 --test src/worked.rs` as well as `cargo test`.
//! The glue (judging, XP, the cast itself) lives in `main.rs`.

#![allow(dead_code)]

/// Whether worked casts are on at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastMode {
    Worked,
    Instant,
}

/// The cast mode: native builds are always instant; on the web, `?debug`
/// pages are instant unless `cast=worked`, and other pages are worked
/// (`cast=instant` is ignored without `?debug`).
pub fn mode_from(debug: bool, cast: Option<&str>, wasm: bool) -> CastMode {
    if !wasm {
        return CastMode::Instant;
    }
    if debug && cast != Some("worked") {
        return CastMode::Instant;
    }
    CastMode::Worked
}

/// What a click on a spell does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Instant,
    Worked,
}

impl Route {
    pub fn as_str(self) -> &'static str {
        match self {
            Route::Instant => "instant",
            Route::Worked => "worked",
        }
    }
}

/// Wrong submits in one worked cast before the answer is shown.
pub const HELP_AFTER: u8 = 2;

/// The route of a spell. `stage` is 0..=3 in I-027 `Stage` order
/// (Unknown, Learning, Practised, Mastered); `learnable` is `None` while no
/// band is known.
pub fn route(stage: u8, learnable: Option<bool>, mode: CastMode, ready: bool) -> Route {
    let _ = (stage, learnable, mode, ready);
    Route::Instant
}

/// The open worked cast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Work {
    pub id: u32,
    pub spell: usize,
    pub route: Route,
    pub wrong: u8,
}

impl Work {
    /// True once the player has had `HELP_AFTER` wrong submits.
    pub fn help(&self) -> bool {
        self.wrong >= HELP_AFTER
    }
}

/// What a submit did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Submit {
    /// The id is not the open worked cast's: nothing changed.
    Stale,
    /// A wrong build: no cast; `help` once the answer should be shown.
    Wrong { help: bool },
    /// A correct build: cast `spell`; `earns` is false after help.
    Cast { spell: usize, earns: bool },
}

/// At most one open worked cast, and the view the page draws it from.
#[derive(Default)]
pub struct Desk {
    work: Option<Work>,
    view: String,
}

impl Desk {
    /// Opens a worked cast, unless one is open, the route is not `Worked`
    /// or `spell` is not 0..4. True when it opened.
    pub fn request(&mut self, id: u32, spell: usize, route: Route) -> bool {
        let _ = (id, spell, route);
        false
    }

    /// Judged build for worked cast `id`.
    pub fn submit(&mut self, id: u32, correct: bool) -> Submit {
        let _ = (id, correct);
        Submit::Stale
    }

    /// The `BenchView` JSON for the open worked cast (trusted: from
    /// `BenchView::to_json`, emitted verbatim). Ignored while closed.
    pub fn set_view(&mut self, json: &str) {
        let _ = json;
    }

    /// Closes worked cast `id` with no cast. True when it closed.
    pub fn cancel(&mut self, id: u32) -> bool {
        let _ = id;
        false
    }

    pub fn is_open(&self) -> bool {
        self.work.is_some()
    }

    pub fn open(&self) -> Option<&Work> {
        self.work.as_ref()
    }

    /// `null`, or `{"id":..,"spell":..,"route":..,"wrong":..,"help":..,"view":..}`.
    pub fn state_json(&self) -> String {
        let _ = &self.view;
        "null".to_string()
    }
}
