# I-020: The first 30 seconds: a walk-and-cast coach, a safe start, and a click that hits the fire

**Status: blocked.** Do not send this item to lanes until I-007 and I-019 have both merged and the planner has moved `based_on` (see **After**). Revision 2 follows the critique (see `critique.md`, "Planner response"). The reading-level captions were split out to **I-021** (see the hand-off at the end).

**Moves:** B4 (every age can play), the move part. Today 0 of 5 personas pass it. The 6yo, 9yo and teacher have `time_to_move_s: null`, the 13yo took 11.8 s and qa took 19.5 s. All 5 already land a cast within 30 s (the 6yo at 21.9 s), and that must hold.
**Expected at the first paired panel after merge** (same seed and query, before and after I-020; no persona gets `?band`, so every persona plays the default real-player path):
- At least 4 of 5 personas move within 10 s, and the 6yo, 9yo and teacher all do. The target is 5 of 5; qa at 4x throttle is the risk.
- All 5 still land a cast within 30 s, and the 6yo does it within 20 s.
- No persona reports "I clicked the fire and nothing happened" (the 6yo's control blocker).
- Every rise in `burning` in the panel transcripts is followed by a spread line in `history` (6yo turn 28 and 9yo turn 29 saw the fires go from 1 to 2 with no caption).

B4 will still fail after this item: clarity ≥ 8 and zero text blockers need I-021, and zero control blockers also need I-007 and I-019. **No bar is reported as moved at merge.** The movement is judged at the next panel run. B1 (test 12), B2/B3 (test 13) and B6 (test 11) must not regress.

**based_on:** origin/main `3dbd332e2c3941011c3d0b957be4b2d9b1d44cd3` (2026-10-30). The code was read there, including `tests/e2e/golden.mjs`, `tests/e2e/perf.mjs` and `docs/loop/gameplay-standards.md`, the source of the GS rows below (added by I-011, 9c4b7e3; it is not on the loop branch). The panel evidence was read from `.loop-artifacts/panel/2026-10-30/*` at artifact 3dbd332 (nightly-1). The I-019 contract was read from its draft spec. The I-007 facts come from `loop/I-007` 870ce29.

**After:** I-007 and I-019 must both be merged. I-019 itself depends on I-007. Builders branch from origin/main after the later of the two merges. Before dispatch the planner:
1. moves `based_on` to that commit and re-checks the facts below;
2. writes `docs/loop/items/I-020/caption-strings.txt` from that commit (see §2 and test 3).

**Stale check.** The planner refreshes this spec before building if any of the following differ from what is listed under "Relied on":
- I-019's hint priority list, `Game.spell = 1` at boot, the `firesLeft` counter, `#firefinder`, or the rule that new DOM is built from `hud.js`;
- I-019's fire counter shows any countdown, timer or "spreading" cue that does not come from `burning` changing. While the world is held, such a cue would warn about a threat that is paused;
- I-007's `--ui` text scale, `IDLE_HINT` / `nudge`, or `CastGate` / the `cast_queue` path;
- the `__gameState` fields `pos`, `casts`, `impacts`, `caption`, `captionId`, `hover`, `aimObj`, `spell`, `locked`, `paused` or `outcome`;
- `perf.mjs` `pageWorkload` no longer begins with a `tp` more than 0.4 m from spawn followed by a cast at a named object (test 13 relies on this to end the hold during warm-up);
- `golden.mjs` starts waiting in real time (not `tick N`) for a spread, a lose or any chem change before its first landed cast (test 12 relies on this).

## Facts from 3dbd332 this spec relies on

- **Nothing asks the player to walk.** The player spawns at `(0, 0, 18)` facing the courtyard (`Player::default`). Every burning bush can be hit from spawn, because `pick` reaches 120 m and assist reaches 60 m. The only start hint is `Click to look around. Esc shows the controls and point-and-click mode.` W A S D appears only in the pause table. All three personas with `time_to_move: null` won without pressing a movement key.
- **The panel's move rule** (`tests/e2e/panel/measure.mjs`): a movement-key act that moves the player at least 0.05 m, or any act that leaves the player more than 0.5 m from spawn. In the nightly panel, the 13yo's `key w 2000` moved the player 0.5 m.
- **Keyboard focus.** Bevy/winit reads keys on the canvas. golden.mjs focuses the canvas with `page.evaluate` before it presses `w`. Nothing focuses the canvas before the first click, so a W pressed before any click may not reach the game.
- **A refused lock makes the first click do nothing.** In `hud.js`, a left `pointerdown` while unlocked calls `lock()`. When that first lock is refused, `lockDenied` becomes true but the click does not cast. Later short clicks cast with `cmd('cast')` at the **crosshair**, not at the cursor. The 6yo clicked the fire at (308, 340): nothing happened, and later clicks went to the crosshair.
- **Headless Chromium grants the lock.** golden's check `a click over a target locks the pointer and does not cast` passes on the e2e runner.
- **Spread is often not captioned.** In `react()` in `main.rs`, an uncaptioned `Ignited` (spread from a neighbour) is said only when `game.caption_age > 1.5`. Every cast resets `caption_age` with its cast caption, so while the player is casting, spread is silent. `Reseeded` is always said.
- **The world clock** is `chem_tick` in `FixedUpdate`. It returns early only while paused or after the outcome is decided. The debug `tick N` command in `control()` calls `chem.world.step` directly, not through `chem_tick`. `publish` reads the outcome from `w.outcome()`, so it follows the world, wherever the world was stepped from.
- **golden.mjs** (read for test 12): `three fires at start`, then a click that locks, mouse look, `WASD moves` (more than 1.5 m), jump, spell keys, and `left click casts while locked`. Then `fire spreads to neighbours` with `tick 2400`, and the lose with `tick 9000`, then `reset`. The win (around line 324) comes after `face`, `select 2` and casts that land (`impacts` rises). No step waits in real time for chem changes before a landed cast.
- **perf.mjs** (read for test 13): `pageWorkload` repeats `tp 0 5.5`, `face <dry hedge>`, `select 1`, `cast`, then motion steps, then casts at burning objects. The first step leaves the player 12.5 m from spawn and lands a cast on an object, so the hold ends in warm-up, before sampling. (With `PERF_NO_CAST=1` it would not end; test 13 does not use that mode.)
- **The science lint** (`test/science-sources.test.ts`) scans the `.rs` files under `crates/bevy-client/src`, plus the `.html` and `.js` files under `crates/bevy-client/web`. It flags any sentence-shaped string, formula or temperature that has no row in `content/science-sources.md`.

## Relied on from I-007 and I-019 (their contracts; not rebuilt here)

- **I-019.** Water is selected at boot. The fire counter (`firesLeft`, `#obj-line`, `.fire-pip`), the nearest-fire arrow (`#firefinder`), the spell streams, the CO₂ cloud, `#crosshair.fire`, the locked hint `Aim at a fire and click.`, `P` and `#pausebtn`, the latched click (`fire` command) and the start caption text all belong to I-019. New DOM is built from `hud.js` with `createElement`. The hint priority, highest first, is: banner, `note`, I-007 idle hint, locked hint, unlocked start hint.
- **I-007.** The settings block (including `hints`), the `--ui` text scale, `IDLE_HINT` with the Water-slot `nudge`, the hit markers and `CastGate`.

**This item does not change** any of the following: casting visuals, the fire counter, the arrow, the crosshair, the cast and click path while locked, any caption text, the I-007 hints, or settings.

## Problem

In the nightly panel, 3 of 5 personas (6yo, 9yo, teacher) never pressed a movement key, and the other 2 took 11.8 s and 19.5 s. Nothing on screen asks the player to walk, and the quest can be won from where the player starts. The 6yo's first click on a fire did nothing, because the lock was refused and the click was dropped. Later clicks went to the crosshair, not to the fire under the cursor. When fire spread (1 burning became 2), the caption that explains it was suppressed, because the player had just cast. Fires also spread while a new player is still learning the controls, so the first area is not a safe space (GS-23).

## Smallest version

### 1. The coach beat (Rust: `crates/bevy-client/src/onboard.rs`, new, Bevy-free)

- `pub enum Step { Walk, Cast, Done }`, and `pub struct Onboard { spawn: [f32; 2], moved: bool, landed: bool }` with `pub const MOVE_M: f32 = 0.4`.
- `Onboard::new(spawn_xz)` starts with `moved = false` and `landed = false`.
- `fn observe_pos(&mut self, xz: [f32; 2])` sets `moved` once the horizontal distance from `spawn` is at least `MOVE_M`. It never unsets it.
- `fn observe_landed(&mut self)` sets `landed`.
- `fn step(&self) -> Step` returns `Walk` if `!moved`, else `Cast` if `!landed`, else `Done`. The order is fixed: a player who casts first is still asked to walk.
- `fn holds_world(&self) -> bool` is `step() != Done`.
- `fn restart(&mut self)` clears both flags.

**Wiring in `main.rs`:**
- An `Onboard` resource is created at startup with `Player::default().pos`.
- `control()` calls `observe_pos(player.pos.xz())` every frame, after the movement step and after any `tp`/`move` command.
- `projectiles` calls `observe_landed()` when a projectile arrives with `obj: Some(_)`, whatever the spell or result.
- **World hold.** `chem_tick` returns early while `onboard.holds_world()`. Fires, heat spread, greenhouse heating and drying all wait until the player has walked and landed one cast. A new player therefore cannot lose while learning (GS-23, GS-25). The debug `tick N` command still steps the world while held, so golden's spread and lose checks keep working.
- **The hold is silent on purpose.** Nothing tells the player that the world is waiting. A player who never walks keeps a frozen world, which is safe, and the coach is already asking for the one action that ends the hold. No countdown or "spreading" cue may show while held (stale check, test 9).
- **No reset of the beat.** `reset()` (R, `cmd reset`, and I-005's post-pick reset) does **not** touch `Onboard`. The beat runs once per page load. A reset before the first walk therefore leaves the world held.
- **Replay.** The new command `onboard restart`, allowed without `?debug`, calls `Onboard::restart()` and then `reset()`. This is the "tutorial on demand" that GS-23 asks for.

### 2. Spread is always said (Rust: `crates/bevy-client/src/lines.rs`, new, Bevy-free)

- `pub fn spread_line_due(is_spread_ignite: bool, already_captioned: bool, caption_age: f32, is_melted_or_burnt_out: bool) -> bool`. It returns true for a spread ignite that has not been captioned, **whatever `caption_age` is**. For `Melted` and `BurntOut` it keeps today's rule (uncaptioned and `caption_age > 1.5`). Every other case returns false, so `react()` keeps its current handling for them.
- `pub fn event_lines(...) -> Vec<String>` holds the whole decision that `react()` makes today about which event lines to say. It takes the events, `caption_age`, and whatever `react()` reads today to build each line (for example the object names), with no Bevy types. It returns the lines in the order `react()` says them now, and uses `spread_line_due` for the condition. `react()` only calls `event_lines` and passes each result to `game.say`. It keeps no condition of its own. The builder picks the exact parameter list from the current `react()`. `lines.rs` may `use crate::chem`, because `chem.rs` has no Bevy types.
- **Every caption string stays byte-identical.** No `&'static str` or `format!` template in `chem.rs` or `main.rs` changes text. Strings move into `lines.rs` only if they are copied there unchanged.
- **The snapshot.** At dispatch the planner writes `docs/loop/items/I-020/caption-strings.txt` from `based_on`. Each line holds one caption string literal or `format!` template (with `{}` / `{name}` as written in the source) from `chem::Spell::cast_caption`, `chem::EventKind::caption`, `react()` and `reset()`. The spread template's line starts with `spread<TAB>`. The core lane does not edit this file.

### 3. The coach on screen (web: `crates/bevy-client/web/coach.js`, new; wired in `hud.js`)

- **`coachFor(state, now, doneAt)`** returns `{ step, text, icon, slot } | null`.
  - `step` is `walk`, `pick`, `cast` or `done`.
  - `pick` is `onboard === 'cast'` with `spell === 0` (Fire is selected, and fire cannot put out fire).
  - `done` lasts 2500 ms after `onboard` first reads `done` (`doneAt`), then the function returns `null`.
  - It returns `null` while `paused`, while `outcome !== 'playing'` (except during `done`), and before the first state.
  - `slot` is 2 for `pick` and `null` otherwise.
- **Pinned coach texts.** There is one plain text per step. It is written for the middle reader, and the icon carries it for pre-readers. I-021 adds sprout and apprentice variants.

| step | text | icon |
|------|------|------|
| walk | `Hold W to walk forward.` | `key-w` |
| pick | `Fire will not put out fire. Pick Water: press 2.` | `slot` |
| cast | `Aim at a fire and click to put it out.` | `mouse-left` |
| done | `Nice! Now put out every fire.` | `star` |

- **Reading rule for coach text** (test 7 enforces it; GS-21): at most 14 words, no word longer than 11 letters, no chemical formula or subscript, at most one temperature. Words are split on whitespace, with punctuation stripped.
- **DOM** (built by `hud.js` with `createElement`; `index.html` is not edited):
  - **`#coach`**, bottom centre, above the hotbar. Its class is `step-<step>` and it is hidden when `coachFor` is `null`. It holds a large icon (at least 64 px) and nothing else:
    - `key-w` is a keycap with "W" and an up arrow;
    - `mouse-left` is a mouse with the left button filled;
    - `slot` is an arrow pointing at the Water slot;
    - `star` is a star.
  - **The coach text goes into `#hint`**, so the panel's HUD reader sees it with no harness change. While the coach is non-null, it outranks every hint except the banner. The new priority is: banner, coach, `note`, I-007 idle, locked, start.
  - The text is at least 24 px times `--ui`, and has at least 4.5:1 contrast on the panel background (GS-18).
  - In the `pick` step, the Water slot (`.slot[data-spell="2"]`) gets class `coach`: a thick outline plus an arrow shape, not colour alone (GS-19).
  - `#coach` and `#hint` must not cover the crosshair, and must not overlap `#firefinder`, `#objective` or `#hotbar` at 1366×768 and at 1920×1080.
  - **Motion.** Only `done` animates: one scale pop from 1.0 to 1.15 to 1.0 over 300 ms. It does not flash (GS-6). Under `prefers-reduced-motion: reduce` there is no animation.
  - The coach ignores I-007's `hints` setting. It is the tutorial, and it ends on its own after one walk and one cast.
- **Replay button.** The pause card gets a button **`#coach-replay`**, "Play the tutorial again". It is at least 40 px tall (GS-22) and is reachable by Tab plus Enter (GS-13). Clicking it calls `showPause(false)` and `cmd('onboard restart')`, then focuses the canvas.
- **Focus.** On the first state, `hud.js` calls `canvas.focus()`, unless `#age` is visible or the game is paused. W then works before any click. The I-005 picker already refocuses the canvas after a pick.

### 4. A click on a fire casts even when the lock is refused (web: `hud.js`)

- At an unlocked left `pointerdown` (not point mode), record `{x, y}` and whether `state.hover >= 0`.
- Today a short click (moved 6 px or less) whose lock request is refused does nothing on the first refusal, and casts at the crosshair after that. Now it casts **at the cursor**, `cmd('tap x y')`, when a target was hovered at `pointerdown`, and otherwise `cmd('cast')` at the crosshair. This includes the very first click on the page.
- When the lock is **granted**, nothing changes: the click locks and does not cast. The golden check `a click over a target locks the pointer and does not cast` stays.
- `tap` already exists and is allowed without `?debug`.

## Acceptance tests

- **Rust:** `cargo test --manifest-path crates/bevy-client/Cargo.toml --locked`.
- **Web unit:** `node --test crates/bevy-client/web/`.
- **E2E:** `node tests/e2e/onboard.mjs <pkg-out dir>`. It serves the built `crates/bevy-client/web/pkg-out` and drives headless Chromium at 1280×720 with `?low`. It reads `window.__game.state()` and the DOM. "Within N frames" means N increments of `state.frame`. Objects are found by `objs[].n` and `burning`, never by a fixed index. The e2e file is first run, and its red state first counts, once all three lanes have merged.

1. **`Onboard`** (core).
   - `new([0,18])` gives `Walk` and `holds_world()` true.
   - `observe_pos([0, 17.7])` (0.3 m) gives `Walk`.
   - `observe_pos([0, 17.59])` (0.41 m) gives `Cast`.
   - `observe_pos([0, 18])` (back at spawn) still gives `Cast`.
   - `observe_landed()` gives `Done`, and `holds_world()` is false.
   - A fresh `Onboard` with `observe_landed()` first gives `Walk`, and a later `observe_pos` 1 m away gives `Done`.
   - `restart()` gives `Walk` with `holds_world()` true.
2. **Spread rule** (core).
   - `spread_line_due(true, false, 0.0, false)` is true.
   - `spread_line_due(true, true, 9.0, false)` is false.
   - `spread_line_due(false, false, 0.5, true)` is false and `spread_line_due(false, false, 2.0, true)` is true (Melted and BurntOut keep the 1.5 s rule).
   - **`event_lines` with a real spread.** The test builds a `chem::World` in its start state and steps it until `step` returns an `Ignited` event that is a spread (not a cast). With `caption_age = 0.0`, and that event not already captioned, `event_lines` returns a line that matches the snapshot's `spread` template (each `{…}` read as `.+`).
   - **Mutation:** putting `caption_age > 1.5` back into the spread branch of `spread_line_due` must fail both this test and the first assertion above.
3. **Caption strings unchanged** (core). A Rust test `include_str!`s `docs/loop/items/I-020/caption-strings.txt`, `src/chem.rs`, `src/main.rs` and `src/lines.rs`. It asserts that every snapshot line (with the `spread<TAB>` prefix removed) occurs verbatim in at least one of those sources. **Mutation:** changing one character of any cast caption must fail it.
4. **`coach.js`** (web).
   - `{onboard:'walk'}` gives step `walk`, icon `key-w`, and text `Hold W to walk forward.`
   - `{onboard:'cast', spell:0}` gives `pick` with `slot: 2`.
   - `{onboard:'cast', spell:1}` gives `cast`.
   - `{onboard:'done'}` at `now = doneAt + 2499` gives `done`, and at `doneAt + 2501` gives `null`.
   - `{onboard:'walk', paused:true}` gives `null`.
   - `{onboard:'walk', outcome:'lost'}` gives `null`.
5. **Every coach text and icon matches the pinned table** (web), for all 4 rows.
6. **Replay and focus wiring** (web, fake DOM). `hud.js`'s coach setup, run against a fake document with a fake canvas, creates `#coach` and `#coach-replay` with `createElement`. It calls `canvas.focus()` on the first state when `#age` is absent. Clicking `#coach-replay` sends `onboard restart`.
7. **Reading rule** (web). Every `COACH` text passes the §3 rule. **Mutations:** each of these must fail the test: making the walk text `Hold W to walk forward immediately.` (a 12-letter word), and making the pick text `Fire will not put out fire. Pick H₂O: press 2.` (a formula).
8. **E2E world hold** (contract, `?debug`).
   - On a fresh page with no input for 5 s: `onboard` is `"walk"`, `held` is true, `tick` is unchanged, `burning` is 3, and every `objs[].temp` is unchanged. No HUD text node matches `/spread|[0-9]+ ?s\b|hurry/i`, except inside `#coach`, `#hint` and the pause card.
   - **Reset before any walk:** `cmd('reset')` leaves `onboard` `"walk"` and `held` true, and `tick` stays unchanged for 2 s.
   - `cmd('move 0 1 0.6')` gives `onboard` `"cast"` once `pos` has moved at least 0.4 m, and `tick` is still unchanged.
   - `face <a burning bush>`, `select 2`, `cast` gives `onboard` `"done"` and `held` false within 10 frames of `impacts` rising. `tick` then rises within 60 frames.
   - On another fresh page, `cmd('tick 2400')` while held raises `tick` by 2400 (debug still bypasses).
   - After `done`, `cmd('reset')` leaves `onboard` `"done"`, and `tick` rises after the reset.
   - `cmd('onboard restart')` gives `onboard` `"walk"`, `held` true, `burning` 3, `casts` 0, and then a `tick` that stays unchanged for 3 s.
9. **E2E coach and focus, as a new player would play** (contract, **no `?debug`**). The steps run in this order, and nothing clicks before step 2 has passed:
   1. At the first state, `document.activeElement` is `#game`, `#coach` has class `step-walk` and is visible, and `#hint` text is `Hold W to walk forward.`
   2. `page.keyboard.down('w')`, wait 1500 ms, `up`: `pos` has moved at least 0.4 m from spawn. Within 2 frames of `onboard` becoming `"cast"`, `#coach` has `step-cast`, or `step-pick` if `spell` is 0.
   3. Press `1` (Fire): within 2 frames `#coach` has `step-pick`, `.slot[data-spell="2"]` has class `coach`, and `#hint` is the pick text. Press `2`: back to `step-cast`, and the class `coach` is gone.
   4. Sweep the mouse across the canvas at y = 0.5 × height in 16 px steps, stopping where `state.hover` is a burning object F, and click there. **The lock must be granted:** `locked` is true within 30 frames, and `casts` is unchanged (the golden rule). Otherwise the test fails with "lock refused in headless; the refused path is test 10".
   5. Steer with synthetic `pointermove` events carrying `movementX`, as golden does, until `aimObj === F`. Then do one `page.mouse.down()` and `up()`.
   - Pass when `onboard` is `"done"` within 30 s of the first state, `#coach` shows `step-done` and is hidden within 3500 ms after that, and `#hint` then shows the next hint by I-019's priority.
10. **E2E click-to-target when the lock is refused** (contract, `?debug` only for `face` and `turn`).
    - An init script makes `HTMLCanvasElement.prototype.requestPointerLock` return a rejected promise.
    - Fresh page, then `face <burning bush B>`, then `turn 150 0`, so B is off-centre and `aimObj !== B`.
    - Sweep the mouse as in test 9 until `hover === B`, then do one `page.mouse.click`. This is the first click on the page.
    - Within 60 frames, `casts` rose by 1 and `objs[B].burning` is false (Water is selected at boot, from I-019).
    - **Mutation:** reverting §4, so the first refused click casts nothing, must fail this test.
11. **E2E spread is always explained, and science register** (contract and web).
    - **Spread** (contract, `?debug`). Walk and land one cast as in test 8, so the hold ends. Then `face <non-burning bush>`, `select 2`, `cast`, and immediately after the impact (caption age < 1.5 s) send `cmd('tick 2400')` in the same `cmd` batch. Once `burning` has risen above its value before the tick, `#history` (pause open) contains a line that matches the snapshot's `spread` template. **Mutation:** restoring the `caption_age > 1.5` condition for spread must fail this check.
    - **Register** (web lane). `npm test` passes, so the four coach texts and the replay label each have a `C-` row with a verdict and sources (B6). `git diff <based_on>... -- content/science-sources.md` adds rows and removes none.
12. **B1 not regressed.** After `build-web.sh`, `QUERY=?low node tests/e2e/golden.mjs` passes 3 out of 3 runs, with `golden.mjs` unchanged and 0 console errors. The checks read for this claim are listed under "Facts" (spread and lose use `tick N`, and the win follows landed casts).
13. **B2/B3 guard.** `node tests/e2e/perf.mjs` at 4x throttle with `?low`, with `PERF_NO_CAST` unset.
    - **Noise floor first.** Run the based_on artifact as two batches of 3 runs. For each metric, the noise floor is the absolute difference between the two batch medians.
    - Then make 3 interleaved runs each of this artifact and the based_on artifact. Each limit is max(budget, 2 × noise floor):
      - median CPU frame p50: budget +1.0 ms;
      - Brotli total: budget +25 KB (this is deterministic, so it gets no noise allowance);
      - TTI: budget +0.5 s.
    - Each run's `workload_ok` is true. The run covers the world after `Done`, because the workload's first step teleports 12.5 m and lands a cast during warm-up (see Facts).
    - The contract lane records the noise floors, the limits and the medians in `round-1.md`.
14. **Scope.** `git diff --name-only <based_on>...` lists only the paths in the Lanes table below, plus `docs/loop/items/I-020/**`. Nothing under `crates/sim/`, `functions/`, `tests/e2e/golden.mjs`, `tests/e2e/perf.mjs`, `tests/e2e/panel/` or `crates/bevy-client/web/age.js` changes, and `crates/bevy-client/web/index.html` does not change.

## Gameplay standards touched

Source: `docs/loop/gameplay-standards.md` at origin/main 3dbd332.

| GS | P | How |
|----|---|-----|
| GS-6 | P0 | The coach has no flashing. Only `done` makes one 300 ms scale pop, and reduced motion turns it off. |
| GS-11 | P0 | The beat needs one key (W) plus a click. The click now works on the fire under the cursor even when the lock is refused (§4, test 10). |
| GS-13 | P1 | `#coach-replay` works with Tab and Enter. |
| GS-17 | P0 | `done` shows a big star and a short success line. I-007 and I-019 supply the hit marker and the cheer. |
| GS-18 | P0 | Coach text is at least 24 px times `--ui` with 4.5:1 contrast. The breaker measures it. Existing HUD sizes belong to I-016. |
| GS-19 | P0 | The coach icons differ by shape. The Water-slot cue is an outline plus an arrow, not colour alone. |
| GS-21 | P0 | Every coach step has an icon that shows the action, so a pre-reader sees a W keycap or a mouse. The coach text follows the plain-reader rule (test 7). Nothing auto-dismisses before the player acts: walk, pick and cast stay up until they are done. Reading levels for captions are I-021. |
| GS-22 | P1 | `#coach-replay` is at least 40 px tall. |
| GS-23 | P0 | Move and cast are learned by doing. The world is held, so the first area cannot be lost. The tutorial can be replayed from pause. |
| GS-24 | P0 | The objective line stays as it is (I-019 owns it). The coach adds a step-specific action line. The idle hint is I-007's. |
| GS-25 | P1 | No time pressure while learning: fire waits for the player's first walk and first landed cast, and no countdown shows while held. |
| GS-27 | P0 | The coach copy is friendly and neutral for every age. |
| GS-28 | P0 | No lure or auto-advance. The coach ends into normal play, and replay is opt-in. |
| GS-30 | P0 | The `pick` step keeps the science choice visible: fire cannot put out fire, so pick Water. Water at boot is I-019's choice. |

GS-1 to GS-5, GS-7 to GS-10, GS-12, GS-14 to GS-16, GS-20, GS-26, GS-29, GS-31 and GS-32 are not touched. The item makes no change to the camera, the locked cast path, feedback FX, audio, settings or caption text.

## Out of scope

- **Reading-level captions and labels: all of I-021** (see the hand-off below).
- Casting visuals, the fire counter, the nearest-fire arrow, `#crosshair.fire`, the locked hint, the start caption and pausing (all I-019).
- Sensitivity, FOV, the hit markers, the idle hint and settings (I-007).
- Walk speed and frame time under throttle. The 13yo and qa called walking slow; that is I-015's CPU budget.
- A world-space marker on the ground for the walk step. Voice-over or spoken coach lines (no audio pipeline yet).
- A visible "world is waiting" cue (the hold is silent on purpose; see §1).
- A second tutorial for the other spells (introduce, develop and twist each one: a later content item).

## Contract

**New `__gameState` fields** (existing fields unchanged):

| field | type | meaning |
|-------|------|---------|
| `onboard` | `"walk"`, `"cast"` or `"done"` | `Onboard::step()` |
| `held` | bool | `Onboard::holds_world()`: `chem_tick` is skipping |

**Commands** (`__game.cmd`, allowed without `?debug`): `onboard restart` calls `Onboard::restart()`, then `reset()`. `tick N` (debug) steps the world even while it is held. `reset` does not touch `Onboard`.

**Rust modules.** `crates/bevy-client/src/onboard.rs` and `crates/bevy-client/src/lines.rs`, with the signatures in §1 and §2. Both are Bevy-free (`lines.rs` may use `crate::chem`). `main.rs` adds `mod onboard; mod lines;` next to the other `mod` lines (there is no `lib.rs`).

**Web module** (ES module, no DOM access at import; `build-web.sh`'s `cp` line adds `coach.js`): `coach.js` exports `COACH` (the pinned table) and `coachFor`.

**DOM:**
- `#coach.step-{walk|pick|cast|done}`, with the coach text in `#hint`;
- `.slot.coach`;
- `#coach-replay` in the pause card.

All of it is created by `hud.js`. `index.html` is unchanged.

**Snapshot:** `docs/loop/items/I-020/caption-strings.txt`, written by the planner at dispatch (§2).

## Lanes

The three lanes build against the contract independently. Web tests use fake state objects and a fake DOM, and core tests are Rust unit tests. The contract lane's e2e is red until the core and web lanes have merged, and is first run then. Merge order is core, then web, then contract.

| lane | owns | builds |
|------|------|--------|
| core | `crates/bevy-client/src/main.rs`, `crates/bevy-client/src/onboard.rs`, `crates/bevy-client/src/lines.rs` | §1 and §2, tests 1 to 3, the new state fields, `onboard restart`, the world hold, the spread rule |
| web | `crates/bevy-client/web/hud.js`, `style.css`, `coach.js`, `*.test.mjs`, `crates/bevy-client/build-web.sh` (the `cp` line only), `content/science-sources.md` | §3 and §4, tests 4 to 7 and the register half of 11. The coach DOM and icons. Click-to-target on a refused lock. Canvas focus at the first state. |
| contract | `tests/e2e/onboard.mjs` | Tests 8 to 10 and the spread half of 11. Runs 12 and 13 and records them in `round-1.md`. |

**Shared files.** `main.rs` and `hud.js` are also edited by I-007 and I-019, which merge first. These lanes rebase onto them and keep their own changes to the blocks named above. If a builder finds that I-019's merged `hud.js` hint code cannot take the coach at the top of the priority list without editing I-019's own lines, they stop and report it in `round-1.md`, and the planner refreshes this spec.

## Hand-off to I-021 (reading-level captions; split out of revision 1)

I-021 is specced separately and goes after I-005 and I-020. It takes these parts of revision 1: `band.js` (`?band=`, default `seedling`, `window.__band`, no storage), `captions.rs` (`CAPTION_KEYS`, `spell_key`, `event_key`, and `captionKey`/`captionArg` in `publish`), `captions.js` (`CAPTIONS`, `captionFor`, sprout icons, `SLOTS`), the sprout and apprentice coach variants, the `age.js` one-line hook, and the panel `band` plumbing (`personas.mjs`, `pageUrl`, `daemon.mjs`, `docs/loop/panel.md`). It also takes revision 1's pinned sprout and seedling tables and its reading rules. The critique requires the following of I-021:
- a "string → key" table for every `format!` and `say` in `react()` and `reset()`, with `greenhouse_caught` and `station` described;
- `warningFor`/`lostFor` pinned with their current strings, or dropped;
- the I-005 hook tested by a node test on a fake DOM, with no by-hand alternative, and I-005's lane named as reviewer for the `age.js` line;
- the `SLOTS.apprentice` test reads `index.html` at test time;
- `docs/loop/panel.md` added to the tests-e2e area in `areas.md`, or the paragraph dropped;
- panel B4 stated as "if the band is known", plus one 6yo run with no `?band` (default `seedling`) as a guard;
- reading-rule mutations for both sprout and seedling;
- the claims moved from I-020: median clarity from 6 to at least 7, and no text blocker about formulas or "100 degrees".
