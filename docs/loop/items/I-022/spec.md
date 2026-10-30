# I-022: Honest settings and a safe age picker (wave-5 review follow-ups)

**based_on:** origin/main a0e2ecbb0d0b6de444f274674fa1cdb8dfba0505 (2026-10-30, after I-019 (#29) and wave-7 state (#30) merged). Written against a3e50b1; refreshed in wave 8 (see "Refresh (wave 8)" at the end).

**Revision 2** (critique accepted in full; see critique.md "Planner response"). One small item that closes the should-fix findings of two breaker reports on origin/main: `docs/loop/items/I-007/break-1-main.md` (SF-1, SF-2, SF-3) and `docs/loop/items/I-005/break-1-main.md` (should-fix 1, 2, 3). Nits from those reports are out of scope unless listed below.

**After I-019.** I-019 ("Casting you can see") merged at a0e2ecb. It added `cues.rs`, `clock.rs`, `motion.rs`, the fire latch and streams, the fire finder (`fireOff`, computed in `aim_and_cast` from `fov_y` and `aspect`), the hint priority in `frame()` (banner > idle > note > locked > start), `__game.hudState()`, 8 new state keys after `dt`, and the golden section `castingYouCanSee`. This item builds on that code and still does not edit I-019's `frame()` hint, objective, crosshair or finder code, the command filter, the cast branch, `cues.rs` or the `__gameState` format string. See "Clear of I-019" under Contract.

## Facts from a0e2ecb this spec relies on

- **The Hints box only gates the idle hint.**
  - `hud.js:607` (`updateIdle`, `hud.js:606-612`) reads `settings.hints`, and the checkbox handler at `hud.js:507` re-runs `updateIdle` when Hints is unticked. Nothing else reads it.
  - The ordinary hints are worked out in `frame()` at `hud.js:659-671` (I-019's priority: nothing while `outcome !== 'playing'`, then the idle hint, then `note`, then the locked hint, then the start hint) and written by `setHint` (`hud.js:441-444`) whatever the box says:
    - the locked hint `AIM_HINT` = `Aim at a fire and click.` (`hud.js:32`), while `state.locked && state.extinguished === 0`;
    - the unlocked start hint `START_HINT` = `Click to look around. Esc shows the controls and point-and-click mode.` (`hud.js:33`), while unlocked and `casts === 0`;
    - `note`: lock-refused messages (`hud.js:203`, `208`), and the point-and-click note set at `hud.js:430`. `frame()` clears `note` while `state.locked` (`hud.js:663`).
  - `setHint` is also called by `updateIdle` (`setHint('')`, `hud.js:611`). No other caller writes `#hint`.
  - The I-007 golden check output shows the point-and-click note on screen right after Hints was switched off.
- **The tap cone ignores Aim help.** `main.rs:987`: `let tap_assist = settings.assist.max(3f32.to_radians()) * 1.5;`.
  - With Aim help off (`settings.assist == 0`) this is still 4.5°.
  - It is used both for `game.hover` (`main.rs:1027`) and for `tap x y` casts (`main.rs:1065`).
  - With Aim help on it is 6° (`ASSIST` = 4°, `main.rs:116`).
- **"View width" is exact only at 16:9.**
  - `feel::vfov_from_h` (`feel.rs:27`) hard-codes `9/16`.
  - `Settings::fov_y` (`main.rs:110-112`) returns it. The projection is set from it at `main.rs:521` (`setup`) and every frame in `control` at `main.rs:865-874`.
  - `aim_and_cast` (`main.rs:948`) builds the cursor ray from `let fov_y = settings.fov_y();` (`main.rs:983`) and the live viewport aspect (`main.rs:975-980`). I-019's fire finder (`main.rs:~989-1022`) uses the same `fov_y` and `aspect` to work out the horizontal angle and `fireOff`. The ray and the projection therefore agree at any aspect, but the horizontal angle the player sees is 90° only at 16:9. At 4:3 it is about 74°. At 21:9 it is about 113°, which breaks GS-1's "not above about 100°".
  - The `fov` state field is `settings.fov_y()` (`main.rs:1610`, in `publish`), the vertical angle in degrees.
- **The analytics id is re-read on every event.**
  - `analytics.js` `send(event)` calls `dailyId(storage(), new Date())` for each event (`analytics.js:234`). `start()` passes that `send` to `createSession`.
  - A session that crosses UTC midnight sends its later `quest_complete` and `session_end` under a new id.
- **The server stamps the day from its own clock and counts any event as a player.**
  - `handleStats` counts `players` as `COUNT(DISTINCT daily_id)` over **all** event types, per day and per day plus band (`functions/_lib/analytics.ts`, the `days` and `dayBands` queries).
  - So an event that arrives after midnight adds a player to the new day even when no `session_start` was sent that day.
  - **Pinning the id on the client alone does not fix this.** The pinned id still arrives on the new server day and is still counted there. The double count goes away only when `players` is counted from `session_start` rows. This item does both. The pin keeps one session on one id. The query change stops the phantom player. The query change does not change what data is collected: same columns, same events.
- **The age picker shows up late.**
  - `#age` is `hidden` in `index.html`. It is shown only after `boot.js:35`'s `import('./age.js')` resolves and `startAgePicker()` runs.
  - Until then the "Click to look around" canvas is live. A click can take the pointer lock (`hud.js` `lock()`), and the picker then opens while the cursor is locked and cannot click it.
  - `startAgePicker` neither calls `exitPointerLock` nor sets `inert`.
- **The game behind the picker is not inert.**
  - `#age` has `aria-modal="true"`, and that does not make `#stage`, `#hud`, `#loading` or `#pause` inert.
  - The picker swallows key and pointer events in JS, but assistive tech can still reach the HUD and pause buttons behind it.
- **hud reacts to a lock release.** `syncLock` (`hud.js:241-260`) calls `showPause(true)` when the lock is released after it was held once. So when the picker releases the lock, the pause card opens behind the picker, and after the pick the child sees "Click to play". This is expected and acceptable: one click locks again.

## Problem

Three settings in the pause menu do less than their labels say, and the age picker can trap a child or leave the game reachable behind it:

- **Hints.** A student or teacher who unticks "Hints" still gets the locked coaching hint on every start. The label misleads (GS-24 "players can turn hints down", GS-26 hint frequency).
- **Aim help.** With Aim help off, point-and-click and hover still bend toward targets within 4.5°. The box is not an off switch for the path younger players use (GS-10 "adjustable at runtime").
- **View width.** "View width 90°" is 74° on a 4:3 classroom projector and 113° at 21:9 (GS-1, GS-2). The setting is not honest at any aspect other than 16:9.
- **Players.** Any session that crosses UTC midnight adds a phantom player to the new day. This makes B7's dashboard numbers wrong for the teacher reading them. After this item, `funnel` still groups a `quest_complete` sent after midnight under the new day, so a day can show a completed quest with 0 players. That is a known leftover (backlog follow-up), not a new bug. The PR says in one line that `players` now means "devices with a `session_start` that day" and that the data collected is unchanged.
- **Picker and lock.** On a slow link a child can lock the pointer before the picker appears, and then cannot click it.
- **Picker and assistive tech.** The modal is not modal for assistive tech.

## Bars moved

- **B7 (deploy-ready with analytics):** the main bar this item moves. Players are counted correctly across midnight (test 6), one session uses one id (tests 4 and 12), and the picker works on a slow link and is properly modal (tests 13 and 14). B7 stays passing, and the new checks go into `analytics-e2e.mjs`.
- **B4 (every age can play):** small and indirect. The settings now do what they say, for teachers who turn assists down and for 4:3 projectors. No panel prediction is made, and nothing is reported as moved at merge.
- **B1:** must stay green. The new golden checks are added.
- **B2, B3:** must not regress. The changes are a few lines with no new assets (test 18).
- **B6:** unchanged. No in-game string is added or changed. `docs/ANALYTICS.md` gets one sentence, which is not in-game copy.

## Smallest version

1. **Hints off hides coaching hints.**
   - Add a helper next to `setHint` in `hud.js`, and make `setHint` pass its text through it before writing `#hint`:
     ```js
     // Hints off (settings): coaching hints are hidden. Status notes (`note`)
     // and the start line (unlocked, nothing cast yet) stay: they say how to
     // play at all, not how to play better. The idle hint is gated in updateIdle.
     const hintAllowed = (text) => settings.hints || !text || text === note
       || (!input.locked && !!state && state.casts === 0);
     ```
     `setHint(text)` writes `hintAllowed(text) ? text : ''` to both the text and the `hidden` class.
   - `hintAllowed` is declared next to `setHint` (`hud.js:441`), after `settings` (`hud.js:212`) and `note` (`hud.js:138`), so both are in scope.
   - The rule depends on state, not on hint strings. I-019's hints all reach `#hint` through `setHint`, so the gate sits in front of every one of them:
     - `AIM_HINT` (`Aim at a fire and click.`) is hidden with Hints off;
     - `IDLE_HINT` is already off with Hints off (`updateIdle` gates it), and is passed anyway when Hints is on;
     - `note` (point-and-click, lock refused) and `START_HINT` (unlocked, `casts === 0`) stay;
     - the banner case writes `''`, which is always allowed, so "nothing under a banner" is unchanged.
   - Any hint a later item adds through `setHint` is gated with no change here.
   - The point-and-click note (`Point and click: click a glowing target to cast at it. Drag to look.`) is a `note`, so it stays. It confirms the mode the player just chose and is the only place that says how to look in that mode (GS-24: controls always available). This is a deliberate decision. The I-007 breaker's observation is answered by gating the coaching hints, not this note.
   - The "Hints" label and the `#hints` id stay. The `updateIdle` gate stays as it is.
   - With Hints off, a locked first-time player (`casts === 0`) sees no coaching line. That is intended. The objective line and the pause card's controls table stay (GS-24), so this is not a blocker.
   - No other code in `frame()` changes. The `h` computation at `hud.js:659-671` is I-019's and is not edited.
2. **Aim help off means no tap cone.**
   - New pure function in `feel.rs`:
     ```rust
     /// The cone for point-and-click taps and hover: 1.5x the aim-assist cone,
     /// at least 4.5 deg, when Aim help is on; 0 when it is off.
     pub fn tap_cone(assist: f32) -> f32
     ```
     It returns `assist.max(3°) * 1.5` for a finite `assist > 0`, and `0.0` otherwise (including NaN and negative values).
   - `main.rs:987` becomes `let tap_assist = feel::tap_cone(settings.assist);`.
   - `pick` with `assist <= 0` already returns the plain ray hit (`main.rs:902`), so a tap exactly on a target still hits with Aim help off.
3. **View width is the real horizontal angle at any aspect.**
   - New pure function in `feel.rs`:
     ```rust
     /// Vertical field of view (degrees) that gives `h_deg` horizontally at
     /// this aspect (width / height). A non-finite or non-positive aspect
     /// counts as 16:9; the aspect is clamped to [1.0, 4.0], so in a portrait
     /// window the view is as tall as it is wide and never stretches upward.
     pub fn vfov_for(h_deg: f32, aspect: f32) -> f32
     ```
     It computes `2·atan(tan(h/2) / a)`.
   - `vfov_from_h(h)` stays and becomes `vfov_for(h, 16.0 / 9.0)`, so existing callers and tests are unchanged.
   - `Settings` (`main.rs:91-106`) gets `aspect: f32`, default `16.0 / 9.0` (set in the struct literal in `read_settings`, `main.rs:402-412`). Its `hfov` doc comment ("Horizontal field of view at 16:9") drops "at 16:9". `Settings::fov_y()` becomes `feel::vfov_for(self.hfov, self.aspect)`.
   - **`aim_and_cast` writes the aspect.** It already computes `aspect` from the live camera viewport (`logical_viewport_size`, falling back to the window) at `main.rs:975-980`. Its parameter `settings: Res<Settings>` (`main.rs:952`) becomes `mut settings: ResMut<Settings>`, and right after `aspect` is computed it sets `settings.aspect = aspect;`. No new system parameters are added anywhere. (`control`, which holds the per-frame camera block, has no `Camera` or `Window` in scope, so it does not compute the aspect itself.)
   - The per-frame camera block in `control` (`main.rs:865-874`) is **not edited**: it keeps calling `settings.fov_y()`, which now reads `settings.aspect`. It may see the aspect one frame late after a resize. Tests allow 10 frames.
   - `Settings` is written every frame by `aim_and_cast`. Nothing in the crate uses `is_changed` on it (checked again at a0e2ecb: no hits), so the extra writes change no behaviour. The other readers (`sync_visuals` `main.rs:1362`, `viewmodel` `main.rs:1511`, `publish` `main.rs:1552`) take `Res<Settings>` and are not edited.
   - `aim_and_cast` replaces `let fov_y = settings.fov_y();` (`main.rs:983`) with `let fov_y = feel::vfov_for(settings.hfov, aspect);`, using the aspect it computes on that frame. The cursor ray is exact on the frame of a resize, and the projection catches up one frame later. I-019's fire finder reads this same `fov_y`, so its horizontal angle becomes exactly `settings.hfov` at any aspect. The finder code itself is not edited.
   - The setup projection (`main.rs:521`) is left as it is. The first frame corrects it.
   - The `fov` state field keeps its meaning: the vertical angle actually applied, in degrees. The `__gameState` format string and its argument list are **not** edited.
   - The label stays `View width` with `°`. It is now true for any window from 1:1 to 4:1.
4. **One id per session.**
   - `analytics.js` `send(event, id = dailyId(storage(), new Date()))`. The new optional second argument is used as is (`eventBody` still validates it).
   - `start(band)` reads `const id = dailyId(storage(), new Date())` once and passes `send: (e) => send(e, id)` to `createSession`.
   - The id still rotates on the next page load. `dailyId`, `createSession` and `eventBody` are unchanged.
5. **Players are counted from session_start.**
   - In `handleStats` the `days` query's `players` becomes `COUNT(DISTINCT CASE WHEN type = 'session_start' THEN daily_id END)`. The `dayBands` query gets the same change (or a `type = 'session_start'` filter, which must still return a 0 row where the shape needs one, so `shapeStats` input is unchanged).
   - `sessions`, `funnel` and `durations` are unchanged.
   - `docs/ANALYTICS.md`, in the dashboard section (around line 111-120), gets one sentence: "A player is counted on the UTC day their session started; a session that runs past midnight is not counted again on the next day."
6. **The picker frees the mouse and makes the game inert.**
   - In `startAgePicker`, right before `root.hidden = false`:
     - if `document.pointerLockElement` is set, call `document.exitPointerLock()` (inside the existing try);
     - set `inert = true` on `#stage`, `#hud`, `#loading` and `#pause` (whichever exist), and remember their previous `inert` values.
   - While the picker is open, a `pointerlockchange` listener on `document` calls `exitPointerLock()` again if a lock lands late (a lock request that was already in flight). The listener is removed on pick.
   - The listener and the `inert` changes are added only on the path where the picker is shown. When `startAgePicker` returns false (opt-out, or a band is already stored), nothing is added. Do not add the listener earlier.
   - On pick, **before** `document.getElementById('game')?.focus()`, restore the remembered `inert` values. A focus call on an inert element is ignored.
   - `index.html`, `boot.js` and `scripts/build-site.sh` are not changed. The build-time unhide option is not taken.

## Acceptance tests

Each test fails on a0e2ecb unless it is marked as a guard. Guards pass on a0e2ecb and must keep passing.

**Rust unit tests** go in the `tests` module of `feel.rs`. They run with `rustc --edition 2021 --test crates/bevy-client/src/feel.rs -o <scratchpad>/feel && <scratchpad>/feel`. They fail today because the functions do not exist.

1. **`tap_cone`.**
   - `tap_cone(4°)` = 6° ± 1e-6 rad. `tap_cone(1°)` = 4.5°.
   - `tap_cone(0.0)`, `tap_cone(-0.1)` and `tap_cone(NaN)` are exactly `0.0`.
   - `tap_cone(4°)` ≥ 4°, so with Aim help on the tap cone is never narrower than the 4° locked cone (`ASSIST` in `main.rs`).
2. **`vfov_for`.**
   - `(90, 16/9)` = 58.72 ± 0.05, the same as `vfov_from_h(90)`.
   - `(90, 4/3)` = 73.74 ± 0.05.
   - `(90, 21/9)` = 46.40 ± 0.05.
   - `(70, 16/9)` = 43.0 ± 0.1.
   - `(110, 1.0)` = 110.0 ± 0.05.
   - `(90, 0.5)` equals `(90, 1.0)` (clamp). `(90, 9.0)` equals `(90, 4.0)`.
   - `(90, NaN)`, `(90, 0.0)` and `(90, -1.0)` each equal `(90, 16/9)`.
3. **Ray and projection agree at any aspect.** For each aspect a in {4/3, 16/9, 21/9}, `ray_dir(1.0, 0.5, vfov_for(90, a), a)` lies 45° ± 0.01 from straight ahead. That means the right edge of the screen is exactly half the View width. The existing `feel.rs` tests (4 of them) and the `chem.rs` tests still pass.

**Node unit tests** run with `npm test`.

4. **`send` uses the id it is given** (`test/bevy-analytics.test.ts`).
   - Stub `navigator` with `Object.defineProperty(globalThis, 'navigator', { value: { sendBeacon: (u, b) => (bodies.push(b), true) }, configurable: true })`, and restore it in `finally`.
   - With no `localStorage`, call `send({ type: 'session_end', band: 'seedling', dur: 'lt1m' }, ID)` with a fixed 32-hex `ID`.
   - Exactly one body is sent, and its `id === ID`.
   - On a0e2ecb the id is a fresh random one.
5. **Guard:** every existing test in `test/bevy-analytics.test.ts` passes unedited: `dailyId` rotation, `createSession`, `eventBody`.
6. **A session across midnight is one player** (`test/handlers.test.ts`, new test, real handlers on `memoryD1`).
   - `session_start` for id A at `2026-10-29T23:59:00Z`, then `quest_complete` and `session_end` for id A at `2026-10-30T00:01:00Z`.
   - `handleStats` at `2026-10-30T00:02:00Z`: `players.today === 0`, `daily` for 2026-10-29 has `players: 1`, and `players.playerDays === 1`.
   - On a0e2ecb, `players.today` is 1 and `playerDays` is 2.
   - **Zero row:** in the same test, the band rows for 2026-10-30 (`todayByBand` and the per-day band rows) report `players: 0` for `seedling` (a 0 row, whether it comes from the query or from `shapeStats` filling missing bands) rather than a missing row or `players: 1`.
   - **Guard:** the existing test "stats aggregate players, bands, sessions, funnel and durations" passes unedited.

**Golden e2e checks** are new `check(...)` calls in `tests/e2e/golden.mjs`. They go in a new function `reviewFollowups({ page, st, cmd, until, frames, sleep, idx, lit, locked, base, browser, errors })` (the same argument object the `feelAndStandards` and `castingYouCanSee` calls pass), defined after `castingYouCanSee` (which ends at `golden.mjs:1758`). It is called on its own line right after the `castingYouCanSee(...)` call at `golden.mjs:343` and before the `no console errors` check at `golden.mjs:345`, so that check covers it.
- **Order is fixed: after `castingYouCanSee`.** I-019's check "cast: the locked hint says to aim at a fire" needs Hints on, and its finder checks use the 16:9 `hfovOf` at the default viewport. Running after it means a failed restore in this function cannot break I-019's checks.
- On entry the pointer is free and point mode may still be on (`castingYouCanSee` ends with `fresh()` after its point-mode and banner checks). Settings are the defaults (I-007's test 15 cleared storage and rebooted).
- The function defines its own small helpers (`setCtl` and `checked` from `feelAndStandards`; `within`, `unpause`, `fresh`, `tp`, `aimAt` and `relock` from `castingYouCanSee`), copied. It does not hoist or edit them. It may also copy `hudRead` to read `#hint` together with `__game.hudState()`.
- No existing check is edited.
- It runs as `QUERY=?low node tests/e2e/golden.mjs`.
- "Within N frames" means N increments of `state.frame`.
- If the pointer cannot be locked, a test that needs the lock **fails**. It is not skipped.

7. **Hints off hides the locked hint.**
   - `fresh()`, then take the lock with the copied `relock()` (the pause card's `#resume`, retried, because a canvas click in point mode is a tap, not a lock; `lock()` turns point mode off). With `#hints` ticked, `locked` and `extinguished === 0`, `#hint` is visible and non-empty within 2 frames (guard).
   - Untick `#hints` (`setCtl('#hints', false)`). Within 2 frames, `#hint` is hidden or empty.
   - Tick it again. Within 2 frames, `#hint` is non-empty again.
   - The test checks visible/empty only, never the hint wording. With I-019 merged the hint shown here is `AIM_HINT` (`Aim at a fire and click.`), and it must vanish with Hints off.
   - `note` is empty here: a successful lock clears it (`hud.js:197` and `252`), and `frame()` clears it again while locked (`hud.js:663`). The idle hint is not due, because `fresh()` restarts the idle clock. If the lock is not taken, the test fails on its precondition.
   - On a0e2ecb the locked hint stays visible with Hints off.
8. **Hints off keeps the start line and notes (guard).**
   - With `#hints` unticked, release the lock (`document.exitPointerLock()`), close the pause card with `__game.pause(false)`, and `fresh()` so `casts === 0`. `#hint` is non-empty (the start line).
   - Click `#pointmode`. `#hint` is non-empty (the point-and-click note).
   - Re-tick `#hints` at the end, and save, so later runs start from the defaults.
9. **Aim help off: no tap or hover cone.**
   - The pose is fixed: `tp 4 2` and target `bushes[5]`. The lane does not choose another one.
   - `fresh()`, `tp(4, 2)`, and find the near miss on `bushes[5]` (`bushes = idx('bush')`) the way I-007 test 10 does (`golden.mjs:578-604`): `aimAt(b)`, then `turn 9 0` steps until `aimObj === b && assisted === true`. If no near miss is found, this test fails.
   - Release the lock and enter point mode (`#pointmode`). Move the mouse to the canvas centre (`VW/2+1, VH/2+1`, then `VW/2, VH/2`).
   - With `#assist` ticked, `hover === b` within 15 s (guard: the tap cone is wider than the locked cone).
   - Untick `#assist`. Within 10 frames, `hover === -1`.
   - Re-tick it. Within 10 frames, `hover === b`.
   - Then `cmd('face ' + lit[0])` with `#assist` unticked. `hover === lit[0]` at the centre: an exact hit still works with Aim help off.
   - On a0e2ecb, `hover` stays `b` with Aim help off.
10. **View width holds at 4:3 and 21:9, live.** Let `vfovAt(h, a) = 2·atan(tan(h/2)/a)` in degrees.
    - `page.setViewportSize({ width: 960, height: 720 })`. Within 10 frames, `|fov − vfovAt(90, 4/3)| ≤ 0.3` (73.74).
    - `page.setViewportSize({ width: 1680, height: 720 })`. Within 10 frames, `|fov − vfovAt(90, 21/9)| ≤ 0.3` (46.40).
    - At 960×720, set `#fov` to 100. Within 10 frames, `|fov − vfovAt(100, 4/3)| ≤ 0.3`, and `#fov-val` reads `100°`. Set it back to 90.
    - At 960×720, `face lit[0]` and put the mouse at the centre in point mode: `hover === lit[0]`. The ray still matches the projection.
    - Restore `VW×VH`. Within 10 frames, `|fov − 58.72| ≤ 0.2`. The test restores the viewport even when an earlier step fails (`try`/`finally`).
    - On a0e2ecb, `fov` stays 58.72 at every size.
11. **Golden stays green.** All existing checks (98 at a0e2ecb), including I-007's "feel and standards" section, I-019's "casting you can see" section and the state key-order check (`BASE_KEYS`, `gameTime`, `dt`, then `I019_KEYS`, `golden.mjs:1045-1047`), pass 3/3 with `QUERY=?low` and no threshold edits. Run golden in isolation, because concurrent Chromiums crash (I-017).

**Analytics e2e checks** are new `check(...)` calls appended to `tests/e2e/analytics-e2e.mjs`. They go **after** the existing D1 checks (after `D1: events has exactly id, day, ...`), so the existing `players.today === 2` and "no other daily_id" checks are unaffected. Each run uses `openRun(browser, origin)` and closes its context. They run as `node tests/e2e/analytics-e2e.mjs` (or `SKIP_BUILD=1`).

12. **One id across midnight.**
    - **Version check first.** `playwright` is not in `package.json`. The e2e scripts `require('playwright')`, which resolves to the global install (1.63.0 in this environment, checked 2026-10-30). The lane reads `require('playwright/package.json').version` before writing the test. If it is older than 1.45 (no `page.clock`), the lane reports test 12 as blocked and does not rewrite it another way.
    - `setFixedTime` fakes only `Date` and `Date.now`, and timers keep running. `session_end`'s `dur` comes from `performance.now()` (`analytics.js:268`), which it does not touch. The lane confirms this by checking that the `session_end` body carries a valid `dur`.
    - New run. Before `goto`, `page.clock.setFixedTime(new Date('2026-10-29T23:59:30Z'))`.
    - Pick `seedling` and wait for `session_start` on the wire.
    - `page.clock.setFixedTime(new Date('2026-10-30T00:00:30Z'))`, then `page.evaluate(() => window.dispatchEvent(new Event('pagehide')))`. Wait up to 10 s for a `session_end` body.
    - The `session_start` and `session_end` bodies carry the same `id`.
    - On a0e2ecb they differ.
13. **The picker frees a lock taken before it appeared.**
    - New run with a **held route**: the `page.route('**/age.js', ...)` handler awaits a promise that the test resolves itself, then calls `route.continue()`. There is no fixed delay. A 60 s cap resolves the promise anyway, and in that case the check fails.
    - After `firstState(page)`, wait until `#loading` is hidden (it has class `hidden`, set by `hud.js:622`) and `document.elementFromPoint(centre)` is the `#game` canvas. Otherwise the click lands on the full-screen overlay. Then, with the picker still held back, click the canvas centre and wait up to 5 s for `document.pointerLockElement?.id === 'game'`. If the lock is not taken, the check **fails** (precondition).
    - Only after the lock is confirmed does the test resolve the held promise, so `age.js` loads.
    - Wait for `#age` to be visible. Within 1 s of that, `document.pointerLockElement === null`.
    - `page.click('button[data-band="sprout"]')` closes the picker (`#age.hidden === true`), and a `session_start` with band `sprout` is on the wire within 10 s.
    - On a0e2ecb the lock is still held when the picker opens.
14. **The game is inert behind the picker.**
    - In the run from test 13, while `#age` is visible: `#stage`, `#hud`, `#loading` and `#pause` all have `inert === true`, and `#age` itself has `inert === false` with no inert ancestor (`#age.closest('[inert]') === null`), so the pick buttons stay usable.
    - After the pick, all four have `inert === false`, and `document.activeElement.id === 'game'`.
    - On a0e2ecb, `inert` is false while the picker is open.
15. **Guard:** every existing `analytics-e2e.mjs` check passes (48 at a3e50b1; the file is unchanged at a0e2ecb), including the keyboard run's focus checks and the opt-out runs (no picker, no request).

**Other guards**

16. `npm run typecheck` is clean. `npm test` is green, including `science-sources.test.ts` with no row changes.
17. **Lane hygiene.** `git diff --name-only origin/main...` lists only the paths in Lanes. In particular, it lists none of: `crates/bevy-client/src/{cues,clock,motion,world,fx,chem,bridge,player}.rs`, `index.html`, `boot.js`, `style.css`, `build-web.sh`, `scripts/**` or `tests/e2e/panel/**`. `golden.mjs` and `analytics-e2e.mjs` have zero deleted lines.
18. **B2/B3 guard (tester, no script change).** `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs`, against a0e2ecb built into `<scratchpad>/I-022/base-pkg-out`. The median CPU frame p50 must be ≤ base + 0.5 ms. The Brotli size of `pkg-out` grows by at most 2 KB.

## GS walk (screens, controls and effects this item touches)

| GS | P | status |
|----|---|--------|
| 1 | P0 | **met at any aspect**: 90° horizontal is now 90° at 4:3 and 21:9 too (test 10). At 21:9 the old 113° breaks the "about 100° or less" ceiling, and that case is fixed |
| 2 | P1 | met: the slider value is the true horizontal angle from 1:1 to 4:1. It is saved as before |
| 10 | P0 | **met** for the click path too: Aim help off turns off the locked cone, the tap cone and the hover cone (test 9). On by default as before |
| 11 | P0 | unchanged: point-and-click with Aim help on keeps its 6° cone |
| 18 | P0 | unchanged: no new text |
| 21 | P0 | unchanged: no new copy. The start line and notes stay with Hints off |
| 24 | P0 | **met**: hints can be turned down. The objective and the controls (pause card, start line, point-mode note) stay available |
| 26 | P1 | improved: the Hints control does what its label says, and it changes nothing about quest state |
| 22 | P1 | unchanged: the picker buttons are untouched (168 px) |
| 6 | P0 | unchanged: no animation added |

## Out of scope

- **I-019's hud.js work (merged):** hint priority, banner suppression, the locked-hint string, `#obj-line` and pips, `#crosshair.fire`, `#firefinder`, Esc/P and `__game.hudState()`. This item does not edit the `frame()` hint, objective, crosshair, finder or banner code.
- **I-007 nits:**
  - N-1: `?sens=` clamp docs.
  - N-2: re-ticking Hints does not restart the idle clock. A one-line follow-up the planner files in the backlog.
  - N-3: provisional settings on the first frames.
  - N-4: hotbar width at 200%.
- **I-005 nits:**
  - picker heading wording ("Pick your age" or "How old are you?");
  - the silent failure when `age.js` is missing;
  - band switching on reload;
  - noting self-reported bands in ANALYTICS.md;
  - `rg.dailyId` staying in storage after its day ends;
  - the spec text about `build-web.sh`.
- **Unhiding the picker at build time**, or any change to `index.html`, `boot.js` or `scripts/build-site.sh`.
- **The cross-midnight effect on `funnel` and `durations`.** A `quest_complete` or `session_end` after midnight is still grouped under the new day. Only `players` is fixed here.
- **Renaming "Hints"**, or adding hint-frequency presets (GS-26 long term).
- **Portrait and phone layouts** beyond the aspect clamp.
- `crates/bevy-client/README.md` (the settings rows are unchanged in meaning).

## Contract

No stub is needed. The lanes share no new symbol. The only cross-lane seam is that golden tests 9 and 10 read `hover` and `fov`, whose names, types and units are unchanged.

- **`fov` (state):** the vertical field of view actually on the projection, in degrees. From this item on, it equals `vfov_for(hfov, live aspect)`. Golden's existing `vfov(h)` and `hfovOf(v)` helpers (`golden.mjs:360-361`, both 16:9) stay right at the default 1280×720 viewport, where I-007's and I-019's checks run. `fireOff` and `fireBearing` keep their meaning; `fireOff` is worked out from the `fov_y` that `aim_and_cast` now takes from `vfov_for(hfov, aspect)`, so its horizontal angle is exactly the View width at any aspect.
- **`feel.rs` additions:** `pub fn tap_cone(assist: f32) -> f32` and `pub fn vfov_for(h_deg: f32, aspect: f32) -> f32`. `vfov_from_h` is kept.
- **`Settings.aspect: f32`:** written once per frame by `aim_and_cast`. It is read only by `Settings::fov_y`.
- **`analytics.js`:** `send(event, id?)`, a backward-compatible signature. `start` pins the id.
- **`/api/stats` shape:** unchanged. Only the meaning of `players` (and `todayByBand` and `daily[].players`) changes: devices with a `session_start` that day.

**Clear of I-019.** I-019 merged first (a0e2ecb); this item is written on top of it. Where the two meet:

| file | I-022 touches | I-019 touches | resolution |
|------|---------------|---------------|------------|
| `main.rs` | `Settings` struct, impl and literal (91-106, 108-113, 402-412); in `aim_and_cast` only: the `settings` parameter line 952 (`Res` to `ResMut`), one new `settings.aspect = aspect;` line after 980, and lines 983 and 987 | the command filter (now 816-819), the fire finder (~989-1022, reads `fov_y` and `aspect`), the `fire` click count and cast queue (~1039-1067), `cues::flight_secs` (1124), new state fields in the format string (1581-1630) | I-022 edits none of I-019's lines. The finder keeps reading `fov_y`; after this item that value is `vfov_for(hfov, aspect)` |
| `hud.js` | `setHint` plus the new `hintAllowed` (441-444) | `frame()` hint priority (659-671), objective, crosshair, finder; key handling; `hudState` | I-019 did not rewrite `setHint`; banner suppression lives in `frame()` (`!playing` gives `''`). The gate in `setHint` covers every I-019 hint. A visible banner still hides everything |
| `golden.mjs` | a new `reviewFollowups` function after `castingYouCanSee`, plus one call line after the `castingYouCanSee(...)` call (343) | `castingYouCanSee` and its call line | append only; `reviewFollowups` runs last, before `no console errors` |

No I-022 test reads a hint string, the objective text, `firesLeft` or the default spell, so I-019's code cannot break them. No I-019 test reads `Settings.aspect` or the tap cone, and none runs after `reviewFollowups`.

## Lanes

| lane | owns | builds |
|------|------|--------|
| rust | `crates/bevy-client/src/feel.rs`; `crates/bevy-client/src/main.rs` (only the `Settings` struct, its impl and literal, and in `aim_and_cast` the `settings` parameter (952), one `settings.aspect = aspect;` line, and lines 983 and 987) | Unit tests 1–3 first. Then `tap_cone` and `vfov_for` (`vfov_from_h` delegates); `Settings.aspect` plus `fov_y()`; `aim_and_cast` takes `ResMut<Settings>`, writes the aspect, and uses `vfov_for(settings.hfov, aspect)` and `tap_cone`. Must not edit `control` or its camera block. Build native (`cargo build`) and wasm to confirm. Must not touch the command filter (816-819), the fire finder, the cast queue and cast branch, the `__gameState` format string or its arguments, `cues.rs`, `clock.rs`, `motion.rs`, `world.rs`, `fx.rs`, `chem.rs`, `bridge.rs`, `player.rs`, `web/**` or tests |
| web | `crates/bevy-client/web/hud.js` (only `setHint` and the new `hintAllowed` beside it), `crates/bevy-client/web/analytics.js`, `crates/bevy-client/web/age.js`, `functions/_lib/analytics.ts` (the `days` and `dayBands` queries only), `test/bevy-analytics.test.ts`, `test/handlers.test.ts`, `tests/e2e/golden.mjs` (append only: `reviewFollowups` after `castingYouCanSee`, one call line after line 343), `tests/e2e/analytics-e2e.mjs` (append only), `docs/ANALYTICS.md` (one sentence) | Tests 4, 6, 7–10 and 12–14 first. Then the `hintAllowed` gate; `send(event, id?)` and the pinned id in `start`; `players` from `session_start` in both queries; `exitPointerLock` plus the late-lock listener plus `inert` set and restore in `startAgePicker` (restore before focus). Must not touch `index.html`, `boot.js`, `style.css`, `build-web.sh`, `scripts/**`, `migrations/**`, the panel harness, or any `hud.js` code inside `frame()` |

**Seam:** golden tests 9 and 10 need the rust lane. The web lane writes them against the contract, and they go green at the shared build after both lanes merge. Tests 7, 8 and 12–14 need only the web lane. The seam breaker checks three things:
- At 4:3 and 21:9, the projection's `fov` and the cursor ray agree (test 10's hover check).
- `hintAllowed` is the only gate between hint text and `#hint`.
- `inert` is restored before the canvas is focused.

## Refresh (wave 8)

Stale check against origin/main a0e2ecb (I-019 #29 and wave-7 state #30 merged since a3e50b1). `git diff a3e50b1..a0e2ecb` touches `main.rs`, `hud.js`, `style.css`, `golden.mjs`, `perf.mjs`, new `cues.rs`, `clock.rs`, `motion.rs`, and `chem.rs`, `fx.rs`, `player.rs`, `world.rs`, `Cargo.toml`. It does not touch `feel.rs`, `analytics.js`, `age.js`, `boot.js`, `index.html`, `functions/**`, `test/**` or `analytics-e2e.mjs`. Scope is unchanged. Changes:

1. **based_on** set to a0e2ecb. The "Next to I-019" paragraph became "After I-019": I-019 is merged and this item builds on it. "Fails on a3e50b1" lines now say a0e2ecb (the analytics and picker paths are byte-identical, so those claims carry over).
2. **hud.js line numbers:** `setHint` 403-406 to 441-444; `updateIdle` 532 to 606-612 (the `settings.hints` read is at 607, and the checkbox handler at 507 also calls `updateIdle`); `frame()` hint block 579-591 to 659-671; point-and-click note 394 to 430; lock clears `note` at 187/241 to 197/252, plus the new `frame()` clear at 663; `syncLock` 231-247 to 241-260; `#loading` hidden 547 to 622.
3. **Hint facts:** the locked hint is now `AIM_HINT` = `Aim at a fire and click.` (`hud.js:32`), shown while `extinguished === 0` (was `impacts < 3`). The start hint is `START_HINT` (`hud.js:33`). The priority is banner > idle > note > locked > start, all written through `setHint`. I-019 did **not** rewrite `setHint`, so the `hintAllowed` gate in `setHint` sits in front of every I-019 hint; the spec now lists each (AIM hidden, IDLE already gated, note and START kept, the banner's `''` always allowed). The "if I-019 rewrites setHint" conflict note is replaced by that fact.
4. **main.rs line numbers:** `Settings` 85-107 to 91-106, `fov_y` 104 to 110-112, struct literal ~305 to 402-412 (in `read_settings`); `ASSIST` at 116; setup projection 425 to 521; `control` camera block 757-767 to 865-874; `aim_and_cast` at 948, its `settings` parameter ~776 to 952, `aspect` ~863-868 to 975-980, `let fov_y` 875 to 983, `tap_assist` 879 to 987; hover use 889 to 1027; tap cast 922 to 1065; `pick`'s `assist <= 0` 794 to 902; `fov` state argument 1407 to 1610; command filter "line 662" to 816-819; format string ~1378-1410 to 1581-1630.
5. **Fire finder:** I-019's finder in `aim_and_cast` reads the same local `fov_y` and `aspect`. Replacing line 983 with `vfov_for(settings.hfov, aspect)` makes the finder's horizontal angle exactly the View width at any aspect. The finder code is not edited; the rust lane's "must not touch" list now names it, the cast queue, `clock.rs` and `motion.rs`.
6. **`is_changed`:** re-checked at a0e2ecb, still no hits. The other `Res<Settings>` readers (`sync_visuals`, `viewmodel`, `publish`) are listed and not edited. The `hfov` doc comment drops "at 16:9".
7. **Golden placement:** `reviewFollowups` is now defined after `castingYouCanSee` and called after the `castingYouCanSee(...)` call at `golden.mjs:343`, before `no console errors` at 345 (was "after `feelAndStandards` at 339, either order"). Reason: I-019's check "cast: the locked hint says to aim at a fire" needs Hints on, and its finder checks use the 16:9 `hfovOf`, so this item's Hints and viewport changes must run last. The argument object matches the existing calls (`frames`, `idx`, `locked` added). Helpers are copied from `feelAndStandards` (`setCtl`, `checked`) and `castingYouCanSee` (`within`, `unpause`, `fresh`, `tp`, `aimAt`, `relock`, optionally `hudRead`).
8. **Test 7:** takes the lock with the copied `relock()` (via `#resume`), because on entry point mode may still be on and a canvas click there is a tap. The precondition is `extinguished === 0` (was `impacts === 0`). Still checks visible/empty only.
9. **Test 9:** cites I-007 test 10 at `golden.mjs:578-604` and `bushes = idx('bush')`. **Test 10:** restores the viewport in `finally`. **Test 11:** names the 98 existing checks, I-019's section and the key-order check (`BASE_KEYS`, `gameTime`, `dt`, `I019_KEYS`, `golden.mjs:1045-1047`); this item adds no state key.
10. **Contract:** golden's `vfov`/`hfovOf` are at `golden.mjs:360-361` (both 16:9; I-019 did not add a `VW/VH` version). The "Clear of I-019" table is rewritten with a0e2ecb line ranges.
11. **Test 17 (lane hygiene):** `clock.rs` and `motion.rs` added to the paths the diff must not list. **Test 18:** the perf base is a0e2ecb; `ONLY=low@4x RUNS=3` is still valid in the new `perf.mjs`.

12. **Re-check (wave 8, second pass).** origin/main moved to c4832d2 (#31, roadmap v2), which touches only `docs/loop/*.md`; `git diff a0e2ecb..c4832d2 -- crates tests functions test` is empty, so `based_on` stays a0e2ecb and every code citation above holds. Every hud.js `setHint` call (441-444 body, 611 in `updateIdle`, 671 in `frame()`), the `frame()` hint block (659-671), `AIM_HINT`/`START_HINT` (32-33), the `main.rs` `aim_and_cast` lines (948, 952, 975-980, 983, 987, 1027, 1065) and the golden key lists (`I019_KEYS`, `golden.mjs:1045-1047`) were re-read and match. `setHint` is still the only writer of `#hint`, so `hintAllowed` sits in front of every I-019 hint. Two citations corrected: `settings` in hud.js is declared at `hud.js:212` (was 213), and the `control` camera block is `main.rs:865-874` (was 866-874; 865 is its `if let Ok(...) = cam.single_mut()` line).
