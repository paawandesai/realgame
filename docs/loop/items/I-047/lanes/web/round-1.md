# I-047 web lane, round 1: STOPPED at the probe (spec step 2 stop rule)

Base: dc1d336 (this branch's head before the lane, a620fd2, has only spec docs).
Build: `build-web.sh` at base, wasm gzip 5 088 075 B. Every browser run held
the chromium lock, `QUERY=?low`, 1280x720.

**Result: the lane stopped before the fix, as spec step 2 requires.** The probe
shows that the spec's signature does not describe the bug on two counts:

1. The bogus move is **not** about `(-W/2, -H/2)`. It is **minus the Playwright
   mouse position**: `(0,0)` when the mouse never moved, `(-640,-360)` at the
   centre, `(-100,-100)` at (100,100). In the golden it is `(-489,-645)`,
   because the lock is taken by clicking `#resume` (client 489.6, 645.9 after
   scroll). `|dx| = 489 < 0.4·W = 512`, so the both-axes rule misses the real
   golden case on x too.
2. The gap between the button event and the bogus move is **458-847 ms**
   (`timeStamp`), not under 50 ms. That is far past the spec's 150 ms ceiling.
   The bogus move is always **the first pointermove after the edge**, and that
   move is frame-aligned (the game runs at about 1.4-2 fps on this box). So any
   rule keyed to time can never fire here, and on a fast machine it would fire
   only by luck.

The spec's rule would therefore leave T1 failing, and so would any EDGE_MS up
to 150. Per the stop rule, I did not widen the rule. hud.js is unchanged. A
draft of the specced rule was written in scratch and never committed or built.

What is committed: the golden section `lockedClickNoTurn` (T1-T4 exactly as
specced, plus the critic's T1 event log), the probe script and its log, the
base section log, and this report.

## Probe (spec step 2), base build

`round1-probe.cjs` / `round1-probe-base.log`. A fresh page for each mouse
position. The lock is taken with `requestPointerLock()` after a key press, so
the mouse is not moved by the lock. Then `page.mouse.move(pos)` (except the
first case), then `page.mouse.down()`, 150 ms, `page.mouse.up()`. The recorder
is installed with `addInitScript`, at window capture before hud.js. (hud.js
calls `stopImmediatePropagation` on pointermove at window capture, so a
listener added later never sees pointermove.)

| Playwright mouse before press | bogus move after press (movement, client) | gap press -> bogus | bogus after release | gap release -> bogus |
|---|---|---|---|---|
| never moved (0,0) | none (all moves (0,0)) | - | none | - |
| centre (640,360) | (-640,-360), client (0,0) | 847.3 ms | (-640,-360), client (0,0) | 500.5 ms |
| (100,100) | (-100,-100), client (0,0) | 582.7 ms | (-100,-100), client (0,0) | 635.9 ms |
| golden T1: #resume click (489.6,645.9) | (-489,-645), client (489.64,645.89) | 457.8 ms | (-489,-645), client (489.64,645.89) | 512.8 ms |

Each bogus move is trusted. As a coalesced event it is the first part, followed
by (0,0) parts, and the `mousemove` twin carries the same numbers. It always
comes in the first pointermove dispatched after the edge. There is one bogus
move per button event: the press and the release each get their own.

## Failing-first record (base dc1d336, `lockedClickNoTurn` alone)

Run with a temporary local copy of golden that calls only this section after
boot (not committed). Log: `round1-base-section.log`.

| check | spec expectation on base | base result | numbers |
|---|---|---|---|
| T1 `lock: a real press and release does not turn the view` | FAIL | **FAIL** (as required) | yaw 0.00 -> 21.77, pitch 10.00 -> 31.77 (Δ 21.77° both), bogus (-489,-645) after press and after release |
| T2 `lock: a move of half the window right after a button event is dropped` | FAIL | **FAIL** (as required) | Δyaw 10.89°, Δpitch 10.89° |
| T3 `lock: a large move away from any click still turns the view` | PASS | **PASS** | Δyaw -10.89°, Δpitch -8.17° |
| T4 `lock: a flick right after a click still turns the view` | PASS | **PASS** | Δyaw -10.89°, Δpitch 0.00° |

The tests meet their failing-first conditions. The fix as specced would turn T2
green, but T2 models the wrong signature: (-W/2, -H/2) at 0 ms. T1 would stay
red, because the real event is (-489,-645) at about 500 ms.

## After (fix)

Not run. The lane stopped at the probe, so there is no fix, no branch build
with a fix, and no full golden. `npm test`: 73/73 pass (no hud.js coverage
there). The real-click checks `cast: a real click casts exactly once` and
`streams: a slow click (0.35 s) casts once` are untouched.

## For the spec owner: what a rule that fits the evidence could look like

These are facts and options only. The lane did not choose one.

- **Signature visible to the page (golden and normal play, where the lock comes
  from a click):** the first locked pointermove after a pointerdown or pointerup
  has `movement == -(round(clientX), round(clientY))` of the frozen lock point,
  with both components nonzero. In T1 that is exactly (-489,-645) against client
  (489.64,645.89). A real move that exactly equals minus the lock point, on the
  first event after a click, is close to impossible, and the rule needs no
  timing, so frame rate does not matter. It would miss the probe's
  API-lock-without-click case (client (0,0), movement -(mouse)). The game never
  takes the lock that way: it always locks from a click on the canvas or
  `#resume`.
- **An event-order rule** ("drop the first move after each button edge if it is
  large on both axes") would also fit. It needs a threshold, though, and
  (-100,-100) at (100,100) is only 5.4° per event.
- **Whether real browsers do this is still unknown.** The pattern (movement =
  minus the CDP mouse position) looks like a quirk of
  `Input.dispatchMouseEvent` under lock, not OS raw input. If so, the product
  impact is zero, and the flakes could instead be handled on the test side, for
  example by keeping the Playwright mouse at the lock point. Editing those
  checks is out of scope for this item.
- If the rule changes, T2 should model the real event: a synthetic pointerup,
  then in a later frame a pointermove with `clientX/Y` set and
  `movementX/Y = -(client)`. The (-W/2, -H/2) at 0 ms version does not model
  the real event.
