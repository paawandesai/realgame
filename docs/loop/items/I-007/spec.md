# I-007: Feel pass: sensitivity, FOV, hit feedback, cooldown ring, no head bob

**Moves:** B4 (clarity, no control blocker), B5 (fun). Expected movement: none measurable until the I-006 panel exists, so **no bar value changes at merge, and B4/B5 are not reported as moved at merge.** Prediction for the first paired panel run (same seed, artifact before vs after I-007): no persona reports "too fast/slow to look" or "couldn't tell if it worked" as a blocker, and median clarity rises by at least 1 point. B2 and B3 must not regress (tests 12 and 13).
**based_on:** main f6504c4 (now 661b9ba), 2026-10-29. Code read from `claude/fps-prototype` at 6d5463e (I-001, not yet merged). Facts this spec relies on from that commit:
- `player::LOOK_PER_PX = 0.0022` rad per CSS px at sens 1. `Player::look` wraps yaw into [0, 2π), and `__gameState.yaw` is that value in degrees.
- `player::EYE = 1.6`, and `Player::eye()` is `pos + Y * EYE`. The `FpsCam` translation is set to `player.eye()` and its rotation to `player.rotation()` at the end of `control()` in `main.rs`.
- The camera FOV is a hard-coded `70f32.to_radians()` in `setup()` in `main.rs`.
- `aim_and_cast`, `projectiles` and `viewmodel` are all systems in `crates/bevy-client/src/main.rs`. The `Projectile` component (with `obj: Option<usize>` and `to: Vec3`) is defined there too. No change to `world.rs`, `chem.rs` or `fx.rs` is needed.
- `chem::World::apply` pushes `chem::Event { obj, kind: chem::EventKind }`. `Water` on a non-burning flammable object → `Soaked` and sets `wet = 1.0`. `Fire` on a wet, non-burning, flammable object → `Fizzle(Fizzle::Wet)`. `Water` on a burning object → `Extinguished(Spell::Water)`. `Fire` on an already-burning object pushes no event.
- Today every queued `tap`/`cast` in a frame fires (no gate), and the mouse fires only when `game.cooldown <= 0`.

**Stale check:** if I-001 merges with different values or names for any fact above, or for `Settings`, `Game.cooldown`, `Game.recoil`, the `__gameState` fields or the `__game.cmd` grammar, the planner refreshes this spec before building.
**After:** I-001 (must be merged). The e2e test uses Playwright. If I-002 has merged, use its pinned version; otherwise the contract lane adds it. Test 12 uses I-003's CPU frame metric if I-003 has merged; otherwise it uses the fallback written in test 12.

**Problem:** A 6-to-18-year-old in a school browser lab cannot change mouse sensitivity (it exists only as `?sens=`, which no student will type) or field of view (fixed at a 70° vertical FOV). After a cast they get no signal at the crosshair that it landed, or whether it did anything. The fixed 0.3 s cooldown is invisible, so a held click feels like the game is dropping inputs. Taps when the pointer is not locked (the 6-year-old's path) skip the cooldown altogether, so tap mode and mouse mode feel different. Nothing today prevents someone from adding camera bob later, and camera bob makes young players motion-sick.

**Smallest version:**
- **Settings.** Add a pause-card "Settings" block with a sensitivity slider (0.10–5.00, step 0.05, default 1.00) and an FOV slider (60–85° vertical, step 1, default 70). Both apply live. They persist in `localStorage` under `alchemist.settings.v1`. Precedence is URL (`?sens=`, `?fov=`) > stored value > default. Storage is wrapped in try/catch, so a page whose storage is blocked still boots with the defaults. The defaults are the current values, not tuned ones. Tuning them is a follow-up for the first I-006 panel run, because the defaults are what most players will get.
- **Cast gate.** One `CastGate` is shared by mouse hold and `tap`. Both use `Source::User`. The `cast` test command uses `Source::Test` and bypasses the gate, so I-002's smoke script is unaffected. A User request made during the cooldown is buffered, holding at most one, and fires when the cooldown reaches 0. Any further request while one is buffered is dropped. The buffered cast is cleared (not fired) on pause, `reset`, or when the quest outcome is no longer `Playing`.
- **Cooldown ring.** A ring around the crosshair empties as the cooldown runs out and is hidden once it reaches 0.
- **Crosshair.** The crosshair pulses (scale 1.0 → 1.35 → 1.0 over 120 ms) on each cast. It keeps its existing `on` state when the aim is on a target.
- **Hit marker.** When a projectile lands on a target, an X marker appears. There are three kinds, told apart by shape as well as colour: `hit` (18 px white, 250 ms), `fizzle` (14 px grey at 50% opacity, 250 ms) and `out` (28 px gold `#ffe14a` with a ring, 400 ms). The marker sits at the crosshair (canvas centre) when the pointer is locked, and at the impact's screen point when it is not. A miss (no target object) shows no marker. No further kinds are added.
- **Reduced motion.** Under `@media (prefers-reduced-motion: reduce)`, the pulse animation is off and the ring changes without transition. The marker still appears, since it is the information.
- **Recoil (unchanged).** The existing single recoil value, kick formula and 6.0/s decay are kept as they are. Recoil moves only the gauntlet viewmodel, never the camera. Test 8 guards that.
- **No head bob (guard, not a build).** Nothing bobs the camera today, so test 8 already passes on the I-001 build and cannot fail on it. It is a regression guard and is not counted as delivered feel. The rule it guards: camera translation is exactly `player.eye()`, camera rotation is exactly `player.rotation()`, and the camera FOV is exactly the setting, every frame, with no bob, shake or sprint FOV. The existing viewmodel sway (≤ 0.01 m) stays, because it moves the gauntlet, not the view.

**Acceptance tests.** Rust tests run with `cargo test --manifest-path crates/bevy-client/Cargo.toml --locked`. Web unit tests run with `node --test crates/bevy-client/web/`. The e2e test is `node tests/e2e/feel.mjs <artifact dir> [--base <base artifact dir>]`: it serves the built `web/pkg-out` and drives headless Chromium with the pointer not locked, using `window.__game.cmd` and `.state()`. Objects are looked up by `objs[].n`, never by index. "Within N frames" means N increments of `state.frame`. The e2e file is not run (and its red state is not a failure) until the core and web lanes have both merged.
1. `feel::hit_kind(&[chem::Event])`, judged on `event.kind`:
   - Any `Extinguished(_)` → `Out`, even when mixed with other events.
   - Otherwise, any event that is not `Fizzle(_)` → `Hit`. `Station`, `Soaked`, `Chilled`, `Steam`, `Smothered`, `Ignited` and `Froze` each → `Hit`.
   - All events `Fizzle(_)` → `Fizzle`. An empty slice → `Fizzle` (this is Fire on an already-burning object).
   (lane: core)
2. `CastGate`, starting from rest:
   - `request(User)` returns true and sets the cooldown to 0.30 s.
   - A second `request(User)` returns false and sets `queued = true`.
   - A third `request(User)` returns false, with `queued` still true.
   - `tick(0.29)` returns false. `tick(0.02)` returns true exactly once, then `queued` is false and the cooldown is 0.30 s.
   - `request(Test)` returns true at any cooldown, and does not change `queued`.
   - `clear()` sets `queued = false`, and a following `tick(1.0)` returns false.
   (lane: core)
3. `feel::clamp_sens`: NaN, ±inf, 0 and negative values → `None`. 20 → 10.0. 0.01 → 0.05. `feel::clamp_fov`: NaN and ±inf → `None`. 30 → 60.0. 120 → 85.0. 72.5 → 72.5. (lane: core)
4. `settings.js` `loadSettings(search, storage)`:
   - Empty search and empty storage → `{sens:1, fov:70}`.
   - Storage `{"sens":2,"fov":80}` → those values.
   - Search `?sens=3` → sens 3, even with storage sens 2.
   - A storage whose `getItem` throws → the defaults.
   - Invalid JSON → the defaults.
   - Stored sens 99 → 10. Stored fov 10 → 60.
   - `saveSettings` with a storage whose `setItem` throws does not throw.
   (lane: web)
5. `settings.js` `ringFraction(cd, max)`: (0.15, 0.3) → 0.5. (0, 0.3) → 0. (-1, 0.3) → 0. (0.3, 0) → 0. (0.6, 0.3) → 1.
   `markerPos(state, rect, clientW, clientH)`, where `rect` is the canvas `getBoundingClientRect()` and `clientW`/`clientH` are the canvas `clientWidth`/`clientHeight`:
   - `locked` → the centre of `rect`.
   - Not locked with hitX 100, hitY 50, rect.width 2 × clientW, rect.height 2 × clientH → x = rect.left + 200, y = rect.top + 100.
   - hitX −1 → `null`. hitY −1 → `null`.
   (lane: web)
6. e2e sensitivity. The expected value comes from `LOOK_PER_PX = 0.0022`: 100 px × 0.0022 × 2 = 0.44 rad = 25.21°, and yaw wraps, so 360 − 25.21 = 334.79.
   - `set sens 2`, then `look 0 0`, then `turn 100 0` → `yaw` is 334.8 ± 0.1.
   - Ratio check, independent of the constant: from `look 90 0`, the yaw change from `turn 100 0` at sens 2 is twice the change at sens 1, within 1%.
   - `set sens abc` → `sens` stays 2. Then `reset` → `sens` stays 2.
   (lane: contract)
7. e2e FOV:
   - `set fov 80` → `fov` = 80.0 within 2 frames.
   - `set fov 200` → 85.0.
   - Loading `?fov=65` (with empty storage) → 65.0.
   - `fov` stays constant across 60 frames of `move 0 1 1` and 3 casts.
   (lane: contract)
8. e2e head-bob and recoil guard. This test passes on the I-001 build by design (see "No head bob").
   - During `move 0 1 1.5`, sample every frame (at least 30 samples). Each sample satisfies `cam[1] − pos[1]` = 1.60 ± 0.005 (EYE), and `cam[0]`, `cam[2]` match `pos[0]`, `pos[2]` within 0.005.
   - `yaw` and `pitch` are identical before a `cast` and 100 ms after it.
   (lane: contract)
9. e2e cooldown and buffering. This test reads game state, not wall-clock time, so it does not flake under throttling.
   - Run `face <bush>`, then send two `tap` commands at the canvas centre in the same `cmd` batch. In the first frame that shows them, `casts` has risen by exactly 1 and `queued` is true.
   - In every following frame until `casts` rises again, `queued` is true and `cooldown` > 0 and is non-increasing.
   - `casts` rises by the second 1 in a frame where `cooldown` was ≤ 0.05 in the previous sample. In that frame, `queued` is false and `cooldown` > 0.2 (it was re-armed).
   - `casts` never rises by more than 2 from the two taps.
   - Directly after a cast, `#crosshair` has class `cooling` and `--cd` is in (0, 1]. It loses `cooling` within 2 frames of `cooldown` reaching 0.
   - `#crosshair` gets class `pulse` within 2 frames of `casts` changing.
   (lane: contract)
10. e2e hit markers (unlocked):
    - Water on a non-burning bush (`face`, `select 2` (`Spell::ALL` is Fire, Water, Frost, Foam, 1-based), then `tap` at the canvas centre) → `hitId` increments, `hitKind` is "hit" (the event is `Soaked`), and `#hitmarker` has the classes `show kind-hit`.
    - Fire (`select 1`) on the same bush right after (it is now wet) → `hitKind` is "fizzle" (the event is `Fizzle(Wet)`), with classes `show kind-fizzle`.
    - Water on a burning bush (`objs[].burning` true) → `hitKind` is "out" (the event is `Extinguished(Water)`), with classes `show kind-out`. `show` is cleared within 0.6 s.
    - Position: in the frame that `show` is set on any of the above, the centre of `#hitmarker`'s bounding box is within 4 px of `markerPos(state, canvasRect, clientW, clientH)` computed in the test from the same state.
    - A cast at open sky (`look 0 60`) → `hitId` does not change.
    (lane: contract)
11. e2e settings UI:
    - Open pause (`__game.pause(true)`). Set `#sens` to 2.5 and dispatch `input` → `state.sens` is 2.5 within 2 frames, `#sens-val` text is "2.50", and `localStorage['alchemist.settings.v1']` parses to sens 2.5.
    - Reload → `sens` is 2.5.
    - Run with an init script that makes `Storage.prototype.getItem` throw → `sens` is 1, `fov` is 70, and no console errors.
    - Run with `prefers-reduced-motion: reduce` emulated → after a cast, the computed `animation-name` of `#crosshair` is `none`.
    (lane: contract)
12. Perf (B2 guard). Same machine, `?low`, headless Chromium with 4x CPU throttle. The scenario exercises the new code: for 30 s, every 0.4 s, `face` the next target in `objs[]` order and `tap` at the canvas centre, cycling spells with `select`, and send `reset` whenever `outcome` is not "playing" so casts never stop. This keeps the ring, pulse and marker updating. Run 5 interleaved pairs of the base artifact and this artifact. Pass: the median of this artifact's per-run CPU frame p50 is ≤ base median + 1.0 ms, and ≤ 16 ms absolute (the B2 line).
    - Metric: I-003's CPU frame p50 if I-003 has merged. Otherwise, the p50 of in-page `requestAnimationFrame` deltas, which is the same for both artifacts.
    - The base artifact is main at this item's merge-base with `crates/bevy-client/build-web.sh`. The tester builds it into `<scratchpad>/I-007/base-pkg-out` and passes it as `--base`. The contract lane only writes the script.
    (lane: contract)
13. Size (B3 guard): Brotli of `pkg-out` (wasm + js + css + html) grows ≤ 30 KB against the same base artifact. (lane: contract)

**Out of scope:**
- Per-spell recoil (cut: cosmetic, and no persona complaint ties to it).
- A head-bob option, camera shake and sprint FOV.
- Invert-Y and gamepad/touch look sensitivity.
- Per-spell cooldowns.
- Hit sounds (the fx audio is not built yet).
- A separate viewmodel camera, so the gauntlet will look smaller or larger as FOV changes.
- Damage numbers.
- Tuning the default sens and FOV (follow-up after the first I-006 panel).
- Wiring these tests into CI or the `test:` command (infra item).
- Editing I-002's smoke script.

## Contract
**Commands** (`__game.cmd`, parsed in `control()` in `main.rs`; unknown values are ignored):
- `set sens <f32>`: the value goes through `clamp_sens`, and `None` leaves the current value.
- `set fov <deg f32>`: the value goes through `clamp_fov` and is applied to the `FpsCam` `PerspectiveProjection.fov` in radians.
- `tap X Y` requests a cast with `Source::User`. `cast` requests one with `Source::Test`. Mouse hold (pointer locked, not paused) requests with `Source::User` each frame it is held.
- `reset` and pause call `CastGate::clear()`. They do not change sens or FOV.

**Boot order and precedence.** Rust reads `?sens=` and `?fov=` at startup (`read_settings`, as it already does for `?sens=`). That value is only provisional. At boot, the HUD computes the effective values with `loadSettings` (URL > storage > default, the same rule) and sends `set sens` and `set fov` once. Because the HUD applies the URL first too, the two cannot disagree: with a URL value both give the URL value, and without one the HUD's stored or default value wins. On slider `input` the HUD sends the new value and saves it.

**New `__gameState` fields** (existing fields unchanged):

| field | type | meaning |
|-------|------|---------|
| `fov` | number, 1 decimal | vertical degrees, read from the camera's Projection, not from Settings |
| `cam` | [x,y,z], 3 decimals | `FpsCam` Transform translation |
| `cooldown` | number ≥ 0, 3 decimals | seconds left |
| `cooldownMax` | number | 0.3 |
| `queued` | bool | a buffered cast is waiting |
| `hitId` | u32 | +1 per projectile impact with `obj` set |
| `hitKind` | "none", "hit", "fizzle" or "out" | kind of the last impact |
| `hitX`, `hitY` | number, 1 decimal | the impact point (`Projectile.to`) from `Camera::world_to_viewport` at impact, in logical px. This is the same space as `tap X Y`, the canvas client size. Both −1 when off-screen or behind the camera. |

`sens` already exists and must reflect `set`.

**Rust** (`crates/bevy-client/src/feel.rs`, no Bevy types except `Vec3` if needed). `main.rs` adds `mod feel;` next to the other `mod` lines (there is no `lib.rs`).
- `enum HitKind { Hit, Fizzle, Out }` and `fn hit_kind(&[crate::chem::Event]) -> HitKind`.
- `fn clamp_sens(f32) -> Option<f32>` (finite and > 0, then clamped to 0.05..=10.0) and `fn clamp_fov(f32) -> Option<f32>` (finite, then clamped to 60.0..=85.0).
- `enum Source { User, Test }` and `struct CastGate { pub cooldown: f32, pub queued: bool }`, with `pub const COOLDOWN: f32 = 0.3`.
- `CastGate` has `fn request(&mut self, Source) -> bool`, `fn tick(&mut self, dt: f32) -> bool` (true means fire the buffered cast now; it re-arms the cooldown) and `fn clear(&mut self)`.
- In `main.rs`, `Game.cooldown` is replaced by (or mirrors) the gate. Where `projectiles` calls `chem.world.apply`, it computes `hit_kind` on that impact's events, increments `hitId`, and records `hitX`/`hitY`. The `projectiles` system then needs the `FpsCam` `Camera` and `GlobalTransform` query. This is all inside `main.rs`, which the core lane owns.

**Web** (`crates/bevy-client/web/settings.js`, ES module, no DOM access):
- `loadSettings(search: string, storage) -> {sens, fov}` and `saveSettings(storage, {sens, fov})`.
- `ringFraction(cd, max)` and `markerPos(state, rect, clientW, clientH) -> {x, y} | null`, as defined in test 5.
- The HUD derives every class and variable from `__gameState` only: `--cd = ringFraction(cooldown, cooldownMax)`, `cooling` while `cooldown > 0`, `pulse` when `casts` changes, and `#hitmarker` `show kind-<hitKind>` when `hitId` changes, positioned by `markerPos`. Rust writes no DOM.
- DOM ids and classes: `#sens`, `#sens-val`, `#fov`, `#fov-val`, `#hitmarker.show.kind-{hit|fizzle|out}`, `#crosshair.cooling` with CSS variable `--cd` (0..1), and `#crosshair.pulse`. `style.css` has a `prefers-reduced-motion: reduce` rule that sets `#crosshair` `animation: none` and removes the ring transition.

## Lanes
Merge order is the row order. `core` and `web` build against the contract above independently: `web` tests use fake state objects, and `core` tests are Rust unit tests. The contract lane's e2e is expected to be red until both have merged, and is first run then.

| lane | owns | builds |
|------|------|--------|
| contract | `tests/e2e/feel.mjs`, `package.json`, `package-lock.json` | e2e tests 6–13 as written; adds a pinned `playwright` devDependency only if I-002 hasn't |
| core | `crates/bevy-client/src/main.rs`, `crates/bevy-client/src/player.rs`, `crates/bevy-client/src/bridge.rs`, `crates/bevy-client/src/feel.rs` | feel.rs plus tests 1–3; `mod feel;`; the `set` commands and `?fov=`; CastGate wired into mouse, `tap` and `cast`, with clear on pause/reset/end; hit kind and screen point on impact in `projectiles`; new state fields |
| web | `crates/bevy-client/web/**`, `crates/bevy-client/build-web.sh` | settings.js plus `settings.test.mjs` (tests 4–5); pause-card sliders; boot-time `set`; cooldown ring, pulse, hit marker and reduced-motion rule in hud.js/style.css; build-web.sh copies settings.js (it copies files by name today) |

## Critique responses
Applied from `critique.md`: per-spell recoil cut (old test 2 and the `recoil` state field dropped); head bob relabelled as a guard that cannot fail on the current build; `prefers-reduced-motion` added; test 6 (old 7) now states `LOOK_PER_PX` and adds a ratio check; test 8 (old 9) states EYE = 1.6; test 9 (old 10) now uses game state, not wall-clock time; test 10 (old 11) adds a `hit` case, names the exact chem events, and checks marker position end to end; test 12 (old 13) exercises casts, uses 5 pairs and a +1.0 ms tolerance plus the 16 ms absolute line, and names who builds the base; the `projectiles` system is confirmed to be in `main.rs` (core-owned, no exception needed); `tap` → `Source::User` stated; `rect` defined; HUD derives `--cd`, `cooling` and `pulse` from state only; boot precedence stated; contract-lane e2e not run until complete; `mod feel;` instruction added.

Rejected or changed:
- "Measure test 9 in frame count × dt": the state has no `dt`, so the test uses the `cooldown` and `queued` fields instead. This meets the critic's aim (no wall-clock bound) without adding a field.
- "Widen the tolerance *or* use more runs" for perf: I did both, but kept the bar tight enough (+1.0 ms) to catch a real HUD regression. A wider tolerance would make it unable to fail.
- "Defaults were tuned by the panel": they were not, and the spec now says so honestly. Tuning is listed as a follow-up and not claimed.
- Added beyond the critique: `CastGate::clear()` and clearing the buffered cast on pause, reset or quest end. Otherwise a queued cast could fire into a paused or finished game.
