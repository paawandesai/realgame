# I-027 break-1 (wave 11, fresh breaker)

Branch loop/I-027 at 3d18e2e. Verdict: **no blocking findings.**

## Verified
- Lanes (spec test 25): changed non-doc files are only `build-web.sh` (line 21, one word), `main.rs`, `progress.rs`, `boot.js`, `progress.js`, `test/bevy-progress.test.ts`, `tests/e2e/golden.mjs`. No forbidden path. golden deletions are only the key-order lines.
- `rustc --test progress.rs`: 12 passed. `cargo test --locked` (CARGO_TARGET_DIR=/home/claude/bevy-target): 64 passed. Node `test/bevy-progress.test.ts`: 10 passed.
- `./build-web.sh` (needs /home/claude/bevy-target/tools/bin on PATH for wasm-bindgen): ok; `pkg-out/progress.js` byte-identical to `web/progress.js` (25a).
- Golden under the lock, `QUERY=?low`: 139 PASS, 0 FAIL, exit 0, including all 12 new `progress:` checks (16-23, 25a, 25b).
- Play as a 7-year-old (sprout band) and a 13-year-old (apprentice band) at 1280x720, 960x720, 1680x720 on a site-like copy (rg-analytics on): the age picker shows before pointer lock (no lock held), has 3 bands, closes on a pick; a `prog learn` after the pick is saved to localStorage and matches `progSave`; in point-and-click mode a fire turned to 94-95 % of the width is hovered at that spot and one click casts once and wets that fire (18 of 18 of these checks pass). Gameplay unchanged (frost cast works with water mastered and with all unknown).
- 6 play checks "Hints off/on" failed in my script only because `#hints` (the toggle) does not exist on this branch or on origin/main; not an I-027 defect. The static start hint shows at every size. This item adds no hint, text or DOM.

## Blocking
None.

## Should-fix
1. `crates/bevy-client/web/progress.js:75` sends `prog day` once at boot and never again. A tab left open across local midnight (a classroom laptop that sleeps overnight) keeps yesterday's day number, so builds made after midnight count as the same day and the 2-day rule cannot be met in that page; the reverse also holds for a tab opened before midnight. Re-send `prog day <localDay>` from `poll` (cheap, only when the value changed) and on `visibilitychange` to visible. `progress.rs` `set_day` already does not bump `rev`. I-030 depends on this being right.
2. `progress.js:55` (and 47-62): saves are last-writer-wins with no check of what is stored. Two tabs of the game: the tab loaded earlier writes its older state over the other tab's newer progress, which can silently take a Mastered stage away (breaks "mastery never fades"). Smallest fix: before `setItem`, `readSave`; if the stored save is not what this tab last wrote or loaded, send `prog load` of a merge, or at least do not overwrite. Can be deferred to I-030 if noted there.

## Nits
- `main.rs:1767-1769`: `publish` builds `state_json()` and `encode()` strings every publish, although they only change when `rev` changes. Cached by `rev` it would save an allocation per frame. Measured `update_ms_p50` 1.1 ms in golden, so not a budget problem.
- `progress.rs` `apply_cmd` `load` and `day` are allowed without `?debug` (as specced), so anyone can set Mastered from the console. Fine for a local-only save, but the spec's "Not a cheat outside debug" wording covers only learn, worked, instant, build, predict and clear.
- Not run: the temporary deliberately mismatched `SPELL_IDS` native-panic check (seam breaker item); the native cargo target is shared and disk is tight. The Node test 15b ties `spells.json` to `SPELL_IDS` and passes.
