# I-047 web lane, round 2: signature rule built; T1-T7 green; the builder probe's real-click case fails once and golden's real-click check failed once

Base: b2be429 (spec round 2 on top of 0ef81d9). Build: `build-web.sh`, wasm
gzip 5 088 464 B (hud.js is copied, not compiled, so the base runs used the
same build with the base hud.js copied into `pkg-out`). Every browser run held
the chromium lock, `QUERY=?low`, 1280x720.

**Summary.** The rule is in `hud.js` as specced (edge listener on window capture
for pointerdown/pointerup plus document `pointerlockchange`; `lastPtr`,
`edgePos`, `cdpPos`, `armPtr`/`armMouse`; four candidates, 2 px on both axes;
drop the unit, not the event; ±200 clamp kept). T1, T2 and T7 now pass, T3, T4
and T6 still pass. **But the spec's builder check fails one case**, and the
full golden failed `cast: a real click casts exactly once` (144/145). The cause
is most likely the same (the golden logs no events there, but a cast that
misses the fire it was aimed at fits the 10.89° turn), and it is one the
spec's argument assumed away: when Playwright's
click sends its move and the press and release **in the same frame**, Chromium
sends **one** `-P` after all three, not one per event. That `-P` is the genuine
cancel of the click's own `+P`; the rule drops it (first non-zero unit after the
edge, matches `cdpPos` and the centre), so the kept `+P` (clamped to 200)
turns the view 10.89° on both axes. On base that same sequence nets 0. I did
not widen or change the rule (spec stop rule); details and options below.

## Failing-first record (`lockedClickNoTurn` alone, temporary local runner, not committed)

Logs: `round2-section-base.log`, `round2-section-fix.log`.

| check | base hud.js | fixed hud.js |
|---|---|---|
| T1 `lock: a real press and release does not turn the view` | **FAIL** Δyaw 21.77°, Δpitch 21.77° | **PASS** Δ 0.00 / 0.00 (both `(-489,-645)` dropped) |
| T2 `lock: a move of half the window right after a button event is dropped` | **FAIL** Δ 10.89 / 10.89 | **PASS** Δ 0.00 / 0.00 |
| T3 `lock: a large move away from any click still turns the view` | PASS Δ -10.89 / -8.17 | PASS Δ -10.89 / -8.16 |
| T4 `lock: a flick right after a click still turns the view` | PASS Δ -10.89 / 0.00 | PASS Δ -10.89 / 0.00 |
| T6 `lock: a real first move after a press still turns the view` (new) | PASS expected 6.53 / 5.44, got 6.53 / 5.44 | PASS expected 6.53 / 5.44, got 6.53 / 5.44 |
| T7 `lock: the bogus move after a moved CDP mouse is dropped` (new) | **FAIL** Δ 10.89 / 10.89 | **PASS** Δ 0.00 / 0.00 |

T6 sign note: the spec gives "about -6.5°" for yaw. The game applies
`yaw -= dx·LOOK_PER_PX·sens` (T3: +300 px gives Δyaw -10.89°), so the test
computes the expectation the way the game applies it on both axes: Δyaw
+6.53°, Δpitch +5.44°. Thresholds as specced (±0.5°, |Δyaw| ≥ 1°).

Golden edits in `lockedClickNoTurn` only: T6, T7, T1's patch restore and
recorder removal moved into `finally`, header and T2 comments corrected. T1-T4
assertions, setup and thresholds unchanged.

## Builder probe on the fixed build (`round2-probe.cjs`, `round2-probe-fix.log`)

The recorder runs before hud.js and mirrors the rule per raw unit.

| case | units after the edges (m -> verdict) | Δyaw / Δpitch | pass (< 0.5°) |
|---|---|---|---|
| round-1: API lock, mouse never moved | none non-zero | 0.00 / 0.00 | PASS |
| round-1: mouse to centre, 500 ms, press, release | press: `(-640,-360)` dropped (cdpPos); release: same | 0.00 / 0.00 | PASS |
| round-1: mouse to (100,100) | press and release: `(-100,-100)` dropped (cdpPos) | 0.00 / 0.00 | PASS |
| lock by clicking `#resume` | `(-489,-645)` after the click, dropped (edgePos) | - | - |
| break-1 A: press+release at the lock point | both `(-489,-645)` dropped (edgePos) | 0.00 / 0.00 | PASS |
| break-1 C: move +450, 2 frames, press+release | `+(939,645)`, `-(939,645)` kept; press and release `(-939,-645)` dropped (cdpPos) | 0.00 / 0.00 | PASS |
| golden real click: move to centre, 2 frames, `mouse.click` | `+P`, `-P` kept (the move); then **`+P`, pointerdown, pointerup all at ts 41872.3**, then **one** `-P` at 42375.1, dropped (cdpPos) | **-10.89 / -10.89** | **FAIL** |
| same click again (mouse already at centre) | `+P` 55657.9, `-P` 55658.0 kept; press `-P` dropped, release `-P` dropped | 0.00 / 0.00 | PASS |

Order recorded (the spec asked for it): `+P` always came before the press.
What the spec did not foresee is the count. `mouse.click` always sends its
own move first (a `+P`, even with the mouse already at P). When the move, press
and release land in one frame, the browser sends one `-P` for all three; when
they are spread, it sends one `-P` each. The rule cannot tell those apart: in
both cases the first non-zero unit after the edge is `-P`.

This is not literally one of the spec's stop conditions (the `-P` is the first
non-zero unit after its edge and matches a candidate), but it fails the
probe's pass condition, so I report it rather than tune the rule.

## Full golden (fixed build), 1 run

`round2-golden-full.log` (check lines only). **144/145.**

| check | result |
|---|---|
| `cast: a real click casts exactly once` | **FAIL** `casts 1 -> 2 -> 2 (20 frames on), lit[1] burning=true` (cast, but missed: the turn above) |
| `streams: a slow click (0.35 s) casts once` | PASS on the first attempt, spans `span 0.260 max_dt 0.164 casts +1` |
| `clock: cast cadence keeps the 0.3 s cooldown in wall time` | PASS |
| T1-T4, T6, T7 | all PASS |
| every other check | PASS |

## Targeted real-click runs (temporary runner: boot, then `streamsAndFinder` + `castingYouCanSee`; not committed)

`round2-realclick-{1,2,3}.log`, 43/43 each.

| run | `cast: a real click casts exactly once` | `streams: a slow click (0.35 s) casts once` (spans) |
|---|---|---|
| 1 | PASS `casts 1 -> 2 -> 2`, burning=false | PASS first attempt, `span 0.213 max_dt 0.132 casts +1` |
| 2 | PASS | PASS first attempt, `span 0.250 max_dt 0.185 casts +1` |
| 3 | PASS | PASS first attempt, `span 0.247 max_dt 0.154 casts +1` |

So the real-click check passed 3/3 targeted and failed 1/1 in the full golden;
it depends on whether the click's move, press and release land in one frame.
T5 (3 clean full runs) is **not met**.

## Other checks

- `npm test`: 73/73 pass.
- No native debug dir created. No change to `bench.js`, `bench.rs`,
  `style.css` or golden's `formulaBench`.

## For the spec owner: options (not chosen by the lane)

1. **Pair instead of drop blindly:** do not drop a `-c` that matches `cdpPos`
   when no `-cdpPos` has been kept since that `+cdpPos` was seen (i.e. the
   `-P` is still owed to the move). In the failing case the only `-P` is owed,
   so it is kept and the net is 0; in the spread case the move's `-P` is kept
   first, so the later edge `-P`s are dropped. That changes the `cdpPos`
   candidate's semantics (a new state bit), so it is a spec decision.
2. **Count-based:** keep a running "owed" count of `+P` units not yet
   cancelled and drop a `-P` only when the owed count is 0. Same idea, stated
   as balance.
3. **Test side:** keep the CDP mouse at the lock point before the real click
   (out of scope for this item; the real-click check is not this lane's).

Real mice are unaffected either way: the remaining risk is only to headless
CDP sequences.
