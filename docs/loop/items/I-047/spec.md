# I-047: A click under pointer lock must not turn the view

**based_on:** dc1d3364979e5451790c52b8a1438571df4e45f3 (origin/main, after PR #44 / I-027). **Track:** product (input bug). **Bars:** B4 (no persona reports a control blocker: a click that knocks the aim off the fire is one), B1 (golden stays green; removes the likely cause of two known flakes). **Source:** I-046 web lane round 3 (`.claude/worktrees/w11-I-046/docs/loop/items/I-046/lanes/web/round-3.md` section A3, and `round3-g5-diag.log` lines 209-216, 271-280). `hud.js` means `crates/bevy-client/web/hud.js`, `golden` means `tests/e2e/golden.mjs`. Line numbers are at dc1d336.

## Problem

- Under pointer lock, headless Chromium follows **each** `pointerdown` and `pointerup` with a `pointermove` (and its `mousemove` twin) whose movement is `(-VW/2, -VH/2)`, e.g. `(-640, -360)` at 1280x720, with `client (0,0)` and `buttons 0` (diag log 212-216: press at game frame 58, release at 59, one bogus move after each). Real browsers may do the same (Chrome has a history of a first-event jump after lock; `hud.js:277-278` already mentions one).
- `hud.js:279` `delta` **clamps** each axis to ±200 px instead of dropping the event. So every button event turns the view by 200 px · `LOOK_PER_PX` 0.00095 = 0.19 rad = **10.9° in yaw and pitch**; a click (press + release) turns it 21.8° (recorder rows: yaw 302.2 -> 313.08 -> 323.97, pitch -2.87 -> 8.02 -> 18.9). The aim leaves the fire (`aimObj` 3 -> -1).
- The bogus move also sets `moveWorks = true` (`hud.js:317`), and the `mousemove` fallback (`hud.js:341-345`) would count the twin if `moveWorks` were still false.
- Effect for players: if real browsers do this, a kid aims at a fire, clicks, and the view jumps up-left so the cast misses. Effect for tests: the likely cause of the known flakes `cast: a real click casts exactly once` (`golden:1800-1817`, uses `page.mouse.click`) and `streams: a slow click (0.35 s) casts once` (`golden:1509-1553`), and it made I-046's G5 invalid on every attempt.

## Bar moved

B4 (a click casts where the crosshair was, no view jump) and B1 (the two flaky checks above should become stable; a new check guards it).

## Smallest fix (hud.js only)

1. **Record the time of each button and lock event.** A capture-phase listener on `window` for `pointerdown`, `pointerup` (any button) and `pointerlockchange` stores `lastEdgeT = e.timeStamp` (`performance.now()` for `pointerlockchange` if its `timeStamp` is unusable). It must run before the move it guards, so register it in capture on `window` next to `onMove`.
2. **Classify a locked move as implausible** when **all** hold:
   - it arrives within `EDGE_MS = 50` ms after `lastEdgeT` (by `e.timeStamp`), and
   - `|dx| >= 0.4 · W` **and** `|dy| >= 0.4 · H`, with `W, H` = `window.innerWidth, innerHeight` (the canvas fills the window).
   Both axes large at once, at about half the window, right after a button edge, is the bug's signature. A real flick is mostly along one axis, and a real move far from a button edge is never touched. The builder first probes (in the lane's scratch, not committed) the `timeStamp` gap between the edge and the bogus move in headless Chromium and records it; if it exceeds 50 ms, `EDGE_MS` may rise to at most 150 ms, recorded in the lane report. (critic) The same probe also records the bogus move's `movementX/Y` and `clientX/Y` with the Puppeteer mouse at three positions before the press: never moved (0,0), the canvas centre, and (100,100). If the bogus delta is not about `(-W/2, -H/2)` in all three (for example it is `(mouseX - W/2, mouseY - H/2)`, which is near zero from the centre), the both-axes-large signature does not describe the bug: the lane stops and reports to the spec owner instead of widening the rule on its own.
3. **Drop, do not clamp.** An implausible event contributes `(0, 0)`: it adds nothing to `input.dx/dy`, does not set `moveWorks`, and does not add to `drag.moved`. Apply the test to the raw `movementX/Y` of each coalesced part (`hud.js:285-294`) and to the event itself, before `delta`. (critic) Test the same unit that is summed into the look delta: when coalesced parts are used, test each part and never drop a whole event because its summed parent is large; when there are none (untrusted events have none), test the event. Edge timing uses the parent event's `timeStamp`. The `mousemove` fallback (`hud.js:341-345`) uses the same test. Unlocked hover/drag position code is unchanged.
4. **Everything else stays.** The ±200 px clamp stays for plausible moves (removing it is out of scope). No change to sensitivity, Rust code, `cmd('fire')`, the drag rules, or the `pointerrawupdate` guard. Add one comment saying why the event is dropped (headless Chromium sends `(-VW/2, -VH/2)` after each locked button edge; see I-047).

## Acceptance (golden; new section `lockedClickNoTurn`)

A new `async function lockedClickNoTurn(...)` placed after the `reviewFollowups` function and called on a new line right after the `reviewFollowups` call (`golden:347`). It takes the same arguments as `castingYouCanSee`, uses its own copy of the `relock` helper pattern (`golden:1720`), `set sway off` for the section, and restores sway and the pose afterwards (`fresh()`). Each check logs yaw/pitch before and after, frames, and the raw numbers. A yaw difference is taken modulo 360 (shortest arc).

- **T1 (failing first). A real press + release with no mouse move does not turn the view.** Locked (`relock()`; if the lock cannot be taken the check fails, it does not skip). `look 0 10`, wait for 5 consecutive game frames with yaw and pitch unchanged (10 s cap). `page.mouse.down()` then `page.mouse.up()` at the current mouse position (no `page.mouse.move` between). (critic) The check logs the Puppeteer mouse position used and the raw `movementX/Y` of every locked move seen in the 20 frames (a temporary `window` capture listener added by the check), so a pass or fail shows whether the bogus move happened. Wait 20 game frames. **Pass: |Δyaw| < 0.5° and |Δpitch| < 0.5°**, 1 attempt, no retries. On dc1d336 it must fail (I-046 measured 21.8° on both axes). If it passes on base, the lane stops and reports; it does not change the setup to make base fail.
- **T2 (failing first, deterministic). A synthetic edge plus the bogus move does not turn the view.** Locked, view still as in T1. In one `page.evaluate`: dispatch on `document` a `PointerEvent('pointerup', {button: 0, pointerType: 'mouse', bubbles: true})`, then on `#game` a `PointerEvent('pointermove', {bubbles: true, pointerType: 'mouse', movementX: -innerWidth/2, movementY: -innerHeight/2})`. Wait 10 frames. **Pass: |Δyaw| < 0.5° and |Δpitch| < 0.5°.** Fails on base (10.9°). This guards the fix even if a future Chromium stops sending the bogus move.
- **T3. A genuine large move still turns the view.** Locked, still view, no button edge for at least 500 ms (wall) before. Dispatch on `#game` a synthetic `pointermove` with `movementX: 300, movementY: 150` (both axes large-ish, diagonal, but not right after an edge). Wait 10 frames. **Pass: |Δyaw| >= 5°** (expect 10.9° from the clamp). Must pass on base and branch.
- **T4. A flick right after a press still turns the view.** Locked, still view, `look 0 10` again. In one `page.evaluate`: dispatch a synthetic `pointerup` (button 0, as in T2) and then immediately a `pointermove` with `movementX: 300, movementY: 0`. Wait 10 frames. **Pass: |Δyaw| >= 5°** (one-axis move right after an edge is kept). Guards against a timing-only filter that would eat real flicks while shooting.
- **T5. Regressions unchanged, unedited:** `mouse look turns`, `aim turns while the fire button is held`, `drag-to-look turns the view without pointer lock`, `left click casts while locked`, `streams: a slow click (0.35 s) casts once`, `cast: a real click casts exactly once`, `cast: holding the button keeps casting`, and every other existing check pass in the full golden, **3 runs out of 3** with `BASE_URL= QUERY=?low` (a known load flake outside this item, `clock: cast cadence keeps the 0.3 s cooldown in wall time`, is reported but does not gate if it also fails on base). The two click checks that were flaky are expected to pass 3/3; if either still fails, the lane reports its detail line (do not edit those checks).
- **Failing-first record.** The lane runs `lockedClickNoTurn` alone (a temporary local call order or early return, not committed) on a dc1d336 build and records T1 and T2 failing with their Δ numbers, and T3/T4 passing, before the fix.

## Out of scope

Removing or changing the ±200 px clamp for plausible moves; sensitivity or `LOOK_PER_PX`; any Rust change; the I-046 test 9 rewrite (G5, owned by I-046's branch; this fix should make it valid); touch/pen input; Safari/Firefox-specific lock quirks beyond the rule above; I-028's `bench.js`, `bench.rs`, `style.css` and golden's `formulaBench` (not touched).

## Lanes

Single lane, no contract step.

| lane | area | owns (only these hunks) | builds |
|---|---|---|---|
| web | client-web + tests-e2e | `hud.js`: the pointer-movement block (`delta`/`movement`/`onMove`/`mousemove` fallback, about lines 277-345) plus the new edge-time listener next to it. `golden`: new `lockedClickNoTurn` function after `reviewFollowups` and one call line after `golden:347` | probe the edge gap; base run of the new section (T1/T2 red); fix; wasm build (`build-web.sh`); full golden 3/3 at `?low` holding the chromium lock |

## Overlap and merge order

- **I-028 (building):** owns `bench.js`, `bench.rs`, `style.css`, golden `formulaBench`. No shared hunk; if both add a call line near `golden:347-349`, the later merge rebases the one-line conflict.
- **I-046 (open):** edits golden `closeStreams` and the test 9 block of `castingYouCanSee`; I-047 edits neither. Either order works; after both merge, I-046's G5 should stop being invalid.
