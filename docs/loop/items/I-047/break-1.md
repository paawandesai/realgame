# I-047 break-1 (Breaker, wave 12)

Tested: `loop/I-047` at efc04a3, in a fresh detached worktree, with a fresh
`build-web.sh` build (wasm gzip 5 088 368 B). Every browser run held the
chromium lock, `?low`, 1280x720.

**Verdict: nothing to break, and the branch must not merge as it is.**
`hud.js` is unchanged from origin/main: the lane stopped at the spec's probe
(see `lanes/web/round-1.md`). The branch adds only the `lockedClickNoTurn`
golden section, and two of its four checks are red by design. Merging it would
make golden red.

## Full golden on the branch: 140/143, exit 1

Log: `break1-golden.log` (PASS/FAIL lines only).

| check | result | detail |
|---|---|---|
| `lock: a real press and release does not turn the view` (T1, new) | FAIL | yaw 0.00 -> 21.77, pitch 10.00 -> 31.77. The recorder saw `(-489,-645)` 4 times (pointermove and mousemove twin, after the press and after the release) |
| `lock: a move of half the window right after a button event is dropped` (T2, new) | FAIL | Δyaw 10.89, Δpitch 10.89 |
| `cast: a real click casts exactly once` (existing, known flake) | FAIL | `casts 1 -> 2 -> 2, lit[1] burning=true`: it cast once, but the view jumped off the fire, so the fire is still burning. The check moves the mouse to (640,360) first, so the press brings `(-640,-360)`. This is the bug, still there |
| `lock: a large move away from any click still turns the view` (T3, new) | PASS | Δyaw -10.89 |
| `lock: a flick right after a click still turns the view` (T4, new) | PASS | Δyaw -10.89 |
| `streams: a slow click (0.35 s) casts once` | PASS on its retry | first try `casts +2`, second `+1`: still flaky |
| everything else, including the age picker, hint gating, point-and-click, drag-to-look, `mouse look turns`, `aim turns while the fire button is held`, `cast: holding the button keeps casting`, `no console errors` | PASS | |

The new section leaves later sections alone: `spellProgress` and
`no console errors` pass after it.

## Requested breaks (no fix to attack, so these describe base behaviour)

The probe is `break1-probe.cjs` with log `break1-probe.log`. It locks the way
players do (pause, then click `#resume` at (489.6,645.9)), then:

| case | Δyaw / Δpitch | locked moves seen |
|---|---|---|
| A. press + release at the lock point | **21.77 / 21.77** | `(-489,-645)` 519-736 ms after each edge |
| B. one CDP move of +450 px (genuine flick, no press near) | **0 / 0** | `(+939,+645)` then later `(-939,-645)` |
| C. press + release after B | **21.77 / 21.77** | `(-939,-645)` after each edge, client still (489.64,645.89) |
| D. one CDP move to (339.6,345.9), two large axes | 0 / 0 | `(+339,+345)` then `(-339,-345)` |
| E. press, 5-step move of +300 px, release | 21.77 / 21.77 (casts +13) | each step is `(+pos)` then `(-pos)`; only the two edges count |
| F. synthetic pointerup + `(-640,-360)` (spec T2) | 10.89 / 10.89 | |
| H. `mouse.move(0,0)`, then H2 press + release | **0 / 0** | all `(0,0)` |

Touch, pad and keyboard look, point-and-click and the age picker share no code
with this branch (hud.js is unchanged), and golden passes them all.

## New findings for the spec owner

1. **Headless CDP moves under lock are not real movement.** Every
   `page.mouse.move` under lock produces two trusted pointermoves:
   `movement = +(new absolute mouse position)`, then one frame or more later
   `movement = -(that position)`. The ±200 clamp is symmetric, so they cancel
   and the turn is exactly 0 (B, D). A button event produces only the negative
   half, so it turns the view (A, C). So in headless Chromium, "a genuine fast
   flick still turns the view" **cannot be tested with real CDP input at all**,
   on base or with any fix. Only synthetic events (T3, T4) can test it. This
   matches golden's existing fallbacks in `mouse look turns` and `aim turns
   while the fire button is held`, which switch to a synthetic pointermove when
   real moves do not turn. It strengthens the lane's point that this looks like
   a CDP `Input.dispatchMouseEvent` artifact, not something real browsers do.
2. **The lane's candidate rule `movement == -(client)` does not hold.** In
   case C, the bogus move is `(-939,-645)` while client stays
   `(489.64,645.89)`. It is minus the CDP mouse position, not minus the lock
   point. And in my first probe run (same script, same build), client was
   `(0,0)` on every locked event, while in the second it was the lock point,
   so client is not even stable between runs. Golden's real-click check moves
   the mouse to (640,360) under lock before clicking, so a client-based rule
   would miss exactly the flaky check it is meant to fix.
3. **A test-side workaround works:** park the Playwright mouse at (0,0) while
   locked (`page.mouse.move(0,0)` makes `(0,0)` moves only). Then a press and
   release makes no bogus move and does not turn the view (H, H2). If the spec
   owner concludes this is headless-only, the click checks (and T1) could park
   the mouse at (0,0) after `relock()` instead of changing hud.js. Each such
   check must then also drop or invert its own `page.mouse.move(VW/2, VH/2)`.
4. A product-side rule that fits all the evidence would have to be "drop the
   first locked move after a button edge when it is large on both axes" (event
   order, no timing, no client). Test it against (-100,-100), which is under
   any sane threshold and stays a 5.4° turn per edge.

## Smaller notes on the new golden section

- T1's `Event.prototype.stopImmediatePropagation` patch is undone only on the
  success path. If `page.mouse.down/up` or `frames(20)` throws, the patch and
  the three recorder listeners stay installed for the rest of golden. Move the
  restore into a `finally`.
- The header comment and T2 still describe the bogus move as "(-VW/2, -VH/2)
  right after the edge". The evidence says minus the mouse position, about
  500 ms later (first move after the edge). T2 as written would go green under
  the specced rule and still miss the real event (the lane says this too).
