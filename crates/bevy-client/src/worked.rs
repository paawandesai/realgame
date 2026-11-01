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
    // Rows 1-4 of the table: instant mode or no overlay, Mastered, no band,
    // and a spell the band cannot learn (the Sprout rule) all cast at once.
    if mode == CastMode::Worked && ready && stage < 3 && learnable == Some(true) {
        Route::Worked
    } else {
        Route::Instant
    }
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
        if self.work.is_some() || route != Route::Worked || spell >= 4 {
            return false;
        }
        self.work = Some(Work {
            id,
            spell,
            route,
            wrong: 0,
        });
        self.view.clear();
        true
    }

    /// Judged build for worked cast `id`.
    pub fn submit(&mut self, id: u32, correct: bool) -> Submit {
        let Some(w) = self.work.as_mut().filter(|w| w.id == id) else {
            return Submit::Stale;
        };
        if !correct {
            w.wrong = w.wrong.saturating_add(1);
            return Submit::Wrong { help: w.help() };
        }
        let cast = Submit::Cast {
            spell: w.spell,
            earns: !w.help(),
        };
        self.work = None;
        self.view.clear();
        cast
    }

    /// The `BenchView` JSON for the open worked cast (trusted: from
    /// `BenchView::to_json`, emitted verbatim). Ignored while closed.
    pub fn set_view(&mut self, json: &str) {
        if self.work.is_some() {
            self.view = json.to_string();
        }
    }

    /// Closes worked cast `id` with no cast. True when it closed.
    pub fn cancel(&mut self, id: u32) -> bool {
        if self.work.as_ref().is_some_and(|w| w.id == id) {
            self.work = None;
            self.view.clear();
            true
        } else {
            false
        }
    }

    pub fn is_open(&self) -> bool {
        self.work.is_some()
    }

    pub fn open(&self) -> Option<&Work> {
        self.work.as_ref()
    }

    /// `null`, or `{"id":..,"spell":..,"route":..,"wrong":..,"help":..,"view":..}`.
    pub fn state_json(&self) -> String {
        let Some(w) = &self.work else {
            return "null".to_string();
        };
        let view = if self.view.is_empty() {
            "null"
        } else {
            &self.view
        };
        format!(
            "{{\"id\":{},\"spell\":{},\"route\":\"{}\",\"wrong\":{},\"help\":{},\"view\":{view}}}",
            w.id,
            w.spell,
            w.route.as_str(),
            w.wrong,
            w.help()
        )
    }
}

/// Game seconds after an accepted worked cast during which clicks neither
/// cast nor open the next builder (break 1, should-fix 3): a spam-clicking
/// child sees the spell land before anything new opens.
pub const GRACE_SECS: f32 = 0.4;

/// The pause after an accepted worked cast. A click that arrives during it
/// is ignored, and so is that press while it stays held, even past the end.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Grace {
    left: f32,
    wait_up: bool,
}

impl Grace {
    /// Starts the pause (a correct build was just accepted).
    pub fn start(&mut self) {
        self.left = GRACE_SECS;
    }

    /// Advances one frame of `dt` game seconds. `clicked`: a click arrived
    /// this frame; `down`: the button is held now. True while clicks and
    /// held fire are ignored.
    pub fn tick(&mut self, dt: f32, clicked: bool, down: bool) -> bool {
        // (A small margin: 0.4 - 4 x 0.1 is not exactly 0 in f32.)
        let during = self.left > 1e-4;
        self.left = (self.left - dt.max(0.0)).max(0.0);
        if during && clicked {
            self.wait_up = true;
        } else if !down {
            self.wait_up = false;
        }
        during || self.wait_up
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Should-fix 3: the pause after an accepted cast.
    #[test]
    fn grace_after_accept() {
        let mut g = Grace::default();
        assert!(!g.tick(0.016, true, true), "no pause before an accept");
        g.start();
        assert!(g.tick(0.1, true, false), "a click in the pause is ignored");
        assert!(g.tick(0.1, false, false));
        assert!(g.tick(0.1, false, false));
        assert!(
            g.tick(0.1, false, false),
            "the frame that ends the pause still ignores"
        );
        assert!(!g.tick(0.1, true, false), "a click after the pause counts");
        // A press that starts in the pause stays ignored while held.
        g.start();
        assert!(g.tick(0.25, true, true));
        assert!(
            g.tick(0.25, false, true),
            "held past the pause: still ignored"
        );
        assert!(g.tick(0.25, false, true));
        assert!(!g.tick(0.25, false, false), "released: clicks count again");
        assert!(!g.tick(0.25, true, true));
    }

    // 1. Route table: 4 stages x 3 learnable x 3 (mode, ready) = 36 cases.
    #[test]
    fn route_table() {
        let mut n = 0;
        for stage in 0..=3u8 {
            for learnable in [None, Some(false), Some(true)] {
                for (mode, ready) in [
                    (CastMode::Worked, true),
                    (CastMode::Worked, false),
                    (CastMode::Instant, true),
                ] {
                    let want = if mode == CastMode::Instant || !ready {
                        Route::Instant // row 1
                    } else if stage == 3 {
                        Route::Instant // row 2: Mastered
                    } else if learnable.is_none() {
                        Route::Instant // row 3: no band
                    } else if learnable == Some(false) {
                        Route::Instant // row 4: the Sprout rule
                    } else {
                        Route::Worked // row 5
                    };
                    assert_eq!(
                        route(stage, learnable, mode, ready),
                        want,
                        "stage {stage} learnable {learnable:?} mode {mode:?} ready {ready}"
                    );
                    n += 1;
                }
            }
        }
        assert_eq!(n, 36);
        assert_eq!(
            route(0, Some(false), CastMode::Worked, true),
            Route::Instant
        );
        assert_eq!(route(1, Some(true), CastMode::Worked, true), Route::Worked);
        assert_eq!(route(3, Some(true), CastMode::Worked, true), Route::Instant);
        assert_eq!(Route::Instant.as_str(), "instant");
        assert_eq!(Route::Worked.as_str(), "worked");
    }

    // 2. Mode.
    #[test]
    fn mode() {
        assert_eq!(mode_from(true, None, true), CastMode::Instant);
        assert_eq!(mode_from(true, Some("worked"), true), CastMode::Worked);
        assert_eq!(mode_from(false, None, true), CastMode::Worked);
        assert_eq!(mode_from(false, Some("instant"), true), CastMode::Worked);
        for debug in [false, true] {
            for cast in [None, Some("worked")] {
                assert_eq!(mode_from(debug, cast, false), CastMode::Instant);
            }
        }
    }

    // 3. Open and close.
    #[test]
    fn open_and_close() {
        let mut d = Desk::default();
        assert!(!d.is_open());
        assert!(d.request(1, 1, Route::Worked));
        let w = d.open().expect("open").clone();
        assert_eq!((w.id, w.spell, w.wrong), (1, 1, 0));
        assert!(!w.help());
        assert!(!d.request(2, 1, Route::Worked));
        assert_eq!(d.open().map(|w| w.id), Some(1));
        let mut e = Desk::default();
        assert!(!e.request(3, 1, Route::Instant));
        assert!(!e.request(3, 4, Route::Worked));
        assert!(!e.request(3, 9, Route::Worked));
        assert!(!e.is_open());
        assert!(d.cancel(1));
        assert!(!d.is_open());
        assert!(!d.cancel(7));
        assert!(d.request(5, 2, Route::Worked));
        assert_eq!(d.open().map(|w| w.id), Some(5));
    }

    // 4. Submit.
    #[test]
    fn submit() {
        let mut d = Desk::default();
        assert!(d.request(1, 1, Route::Worked));
        assert_eq!(d.submit(1, false), Submit::Wrong { help: false });
        assert_eq!(d.submit(1, false), Submit::Wrong { help: true });
        assert!(d.is_open());
        assert_eq!(
            d.submit(1, true),
            Submit::Cast {
                spell: 1,
                earns: false
            }
        );
        assert!(!d.is_open());
        assert!(d.request(2, 3, Route::Worked));
        assert_eq!(
            d.submit(2, true),
            Submit::Cast {
                spell: 3,
                earns: true
            }
        );
        assert!(d.request(3, 0, Route::Worked));
        assert_eq!(d.submit(3, false), Submit::Wrong { help: false });
        assert_eq!(
            d.submit(3, true),
            Submit::Cast {
                spell: 0,
                earns: true
            }
        );
        assert!(d.request(4, 2, Route::Worked));
        assert_eq!(d.submit(9, true), Submit::Stale);
        assert_eq!(d.open().map(|w| (w.id, w.wrong)), Some((4, 0)));
        let mut closed = Desk::default();
        assert_eq!(closed.submit(1, true), Submit::Stale);
    }

    // 5. state_json.
    #[test]
    fn state_json() {
        let mut d = Desk::default();
        assert_eq!(d.state_json(), "null");
        d.set_view("{\"v\":1}");
        assert_eq!(d.state_json(), "null");
        assert!(d.request(1, 1, Route::Worked));
        assert_eq!(
            d.state_json(),
            "{\"id\":1,\"spell\":1,\"route\":\"worked\",\"wrong\":0,\"help\":false,\"view\":null}"
        );
        d.set_view("{\"v\":1,\"open\":true}");
        assert_eq!(
            d.state_json(),
            "{\"id\":1,\"spell\":1,\"route\":\"worked\",\"wrong\":0,\"help\":false,\"view\":{\"v\":1,\"open\":true}}"
        );
        d.submit(1, false);
        d.submit(1, false);
        assert!(d.state_json().contains("\"wrong\":2,\"help\":true"));
        assert!(d.cancel(1));
        assert_eq!(d.state_json(), "null");
        assert!(d.request(2, 1, Route::Worked));
        assert!(d.state_json().ends_with("\"view\":null}"));
    }

    // 6. No panic: random operations keep is_open() in step with a model.
    #[test]
    fn no_panic() {
        for seed in 0..20u64 {
            let mut x = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let mut next = move || {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (x >> 33) as u32
            };
            let mut d = Desk::default();
            let mut model: Option<u32> = None;
            for _ in 0..50_000 {
                let op = next() % 4;
                let id = next() % 8;
                match op {
                    0 => {
                        let spell = (next() % 6) as usize;
                        let r = if next() % 2 == 0 {
                            Route::Worked
                        } else {
                            Route::Instant
                        };
                        let ok = d.request(id, spell, r);
                        let want = model.is_none() && spell < 4 && r == Route::Worked;
                        assert_eq!(ok, want);
                        if ok {
                            model = Some(id);
                        }
                    }
                    1 => {
                        let correct = next() % 2 == 0;
                        let s = d.submit(id, correct);
                        match (model, s) {
                            (Some(m), Submit::Cast { .. }) => {
                                assert!(m == id && correct);
                                model = None;
                            }
                            (Some(m), Submit::Wrong { .. }) => assert!(m == id && !correct),
                            (Some(m), Submit::Stale) => assert_ne!(m, id),
                            (None, s) => assert_eq!(s, Submit::Stale),
                        }
                    }
                    2 => {
                        let ok = d.cancel(id);
                        assert_eq!(ok, model == Some(id));
                        if ok {
                            model = None;
                        }
                    }
                    _ => d.set_view("{\"v\":1}"),
                }
                assert_eq!(d.is_open(), model.is_some());
                let _ = d.state_json();
            }
        }
    }
}
