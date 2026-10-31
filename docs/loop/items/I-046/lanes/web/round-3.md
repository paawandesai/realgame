# I-046 web lane, round 3

Scope: spec "Round 3 amendments" A1 (G1 P0), A2 (G6 (a)) and A3 (G5 setup),
with the critic's round-3 edits. Golden edits stay inside this lane's
existing hunks (`closeStreams` and the test 9 block of `castingYouCanSee`).
No other check's pass condition changed. No game code changed.

Builds: branch at this head (wasm gzip 5 087 393 B); base e70df08 built in a
temporary worktree (wasm gzip 5 083 242 B; the worktree was removed after).
Every browser run held the chromium lock, `QUERY=?low`.

Logs in this folder: `round3-base-e70-dev.log` (base, `closeStreams` only),
`round3-golden-run1.log` and `round3-golden-run2.log` (full golden, branch),
`round3-g5-diag.log` (test 9 with a per-frame recorder dump and every pointer
event, trimmed to the events with movement).

## Summary

| | result |
|---|---|
| Full golden, branch, run 1 | **130/133**. Red: G1 P0, both G5 checks |
| Full golden, branch, run 2 | **129/133**. Red: G1 P0, both G5 checks, and `clock: cast cadence keeps the 0.3 s cooldown in wall time` (casts=10 need 11; pre-existing wall-time check outside this lane, passed in run 1 with casts=6 need 4; the same failure is in the breakers' `brk8-golden.log` and `brk9-golden2.log` on other branches, so a load flake on this shared box) |
| G6 (A2) | failing-first met (base t_max - L = +339.3 px); **branch PASS** (+14.2 px), both runs |
| G1 P0 (A1) | failing-first met (base fails every spell at both aspects, all attempts valid); **branch FAILS** (Water 0.67-0.73, Frost 0.36-0.58, CO2 0.39-0.58). Stop rule: lane stops and reports |
| G5 (A3) | **press with no move still turns the view** (Δyaw = Δpitch = 10.88° per button event). Stop rule (step 8): lane stops and reports the recorder rows |

## A2. G6 (a) along the stream's line

New (a): `t_max` = largest `(p - C)·u` over in-band stream pixels, pass when
`|t_max - L| <= 0.035·VH` (±25.2 px at 1680x720). (b) and (c) unchanged.

Note on the spec text: it says `L` is about 229 px. 229 px is how far `Hs'`
sits **below** C; `L = |Hs' - C|` is 520.9 px (Hs' = (1308,589), C =
(840,360)). The check uses the spec's definition (`L = |Hs' - C|`), and the
spec's expected values (branch about +14 px, base +339 px) match that
definition exactly.

| 1680x720, View width 70, tilt 5.90° | base e70df08 Water | base Frost | branch Water | branch Frost |
|---|---|---|---|---|
| old (a): Euclidean distance to Hs' (limit 25.2) | 340.0 FAIL | 340.0 FAIL | 25.6 FAIL | 25.6 FAIL |
| **new (a): t_max - L (limit ±25.2)** | **+339.3 FAIL** | **+339.3 FAIL** | **+14.2 PASS** | **+14.2 PASS** |
| across offset of that pixel | -21.2 | -21.2 | -21.3 | -21.3 |
| (b) px past Hs' + 0.04·VH | 638 FAIL | 614 FAIL | 0 PASS | 0 PASS |
| (c) attached | yes | yes | yes | yes |
| check | FAIL | | **PASS** (runs 1 and 2) | |

Failing-first: (a) and (b) both fail on e70df08 for Water (and Frost), with the
new rule. The branch passes on every attempt.

## A1. G1 P0 on the last frame before impact

Implemented as written: an rAF watcher sends the `cast`, pauses the game on
every frame from the cast on with `streams >= 1` and `impacts` unchanged, takes
a screenshot (HUD and pause card hidden), records the state and unpauses; the
after shot is the last one before the first frame with `impacts` grown.
Validity: after frame burning, `aimObj` = the centred fire, `streams >= 1`;
next frame `impacts + 1` and at most one game step later (`gameTime` gap <=
that frame's logged `dt`); at least 2 pre-impact frames; 120 frames / 30 s
caps; plus the unchanged `before >= 150`. `assisted` is logged.

**One addition the amendment did not foresee: a pinned game step during the
capture.** A spell lands within 0.25 s of game time (`cues::MAX_FLIGHT`) and
the game clamps a frame to 0.25 s (`clock::MAX_FRAME_DT`). This box runs the
game at about 1 fps at 1680x720, so every frame is a 0.25 s step and the
frame after the cast is already the impact frame. Probed on the branch before
any change: Water, Frost and CO2 all went cast -> impact with no pre-impact
frame (`dt 0.25`, impact on the first frame seen). Every attempt would have
been INVALID (fewer than 2 pre-impact frames) on both builds. So, for the
capture only, the test replaces the page's `performance.now` (which Bevy's
clock reads through wasm-bindgen) with one that advances a fixed step per
animation frame, waits until the state's `dt` equals that step (Bevy follows
after about 2 frames), and restores the real clock right after the impact
frame (a forward jump, since the real clock is ahead). The step is **1/20 s**:
1/30 s was tried first, and at about 5 s per paused screenshot Water's 0.25 s
flight hit the 30 s wall cap after 6 frames on all three attempts (base
log, ratio 0.62-0.65 at that point). With 1/20 s Water and CO2 give 4
pre-impact frames (the last at game +0.15 s, head 0.05 s short of the fire),
Frost 2 (0.154 s flight). The square, the 0.80 line, the before rule, the
pose and the aspects are unchanged. This step is a test setup choice the
critic has not reviewed; it is flagged here.

Ratios (warm after / before in the 0.10·VH square at C, pass >= 0.80); every
attempt below was VALID (burning, aim on the centred fire, impact next frame
one 0.05 s step later):

| aspect | spell | base e70df08, tries 1/2/3 | branch run 1, tries 1/2/3 | branch run 2 |
|---|---|---|---|---|
| 1680x720 | Water | 0.62 / 0.62 / 0.65 FAIL | 0.70 / 0.67 / 0.73 FAIL | 0.71 / 0.69 / 0.64 FAIL |
| 1680x720 | Frost | 0.41 / 0.26 / 0.59 FAIL | 0.49 / 0.53 / 0.58 FAIL | 0.54 / 0.56 / 0.34 FAIL |
| 1680x720 | CO2 | 0.44 / 0.53 / 0.49 FAIL | 0.52 / 0.47 / 0.52 FAIL | 0.41 / 0.50 / 0.58 FAIL |
| 1024x768 | Water | 0.67 / 0.60 / 0.65 FAIL | 0.71 / 0.72 / 0.73 FAIL | 0.67 / 0.68 / 0.72 FAIL |
| 1024x768 | Frost | 0.63 / 0.50 / 0.37 FAIL | 0.52 / 0.57 / 0.36 FAIL | 0.58 / 0.53 / 0.11 FAIL |
| 1024x768 | CO2 | 0.57 / 0.52 / 0.47 FAIL | 0.39 / 0.58 / 0.43 FAIL | 0.51 / 0.47 / 0.36 FAIL |

(Old rule, rounds 1-2: ratio 0.00 on base and branch alike, `burning=false`.)

Failing-first: met. P0 fails for Water at both aspects on e70df08 (0.60-0.67,
the spec predicted 0.65-0.75), with valid attempts.

**On the branch P0 fails** (Water 0.64-0.73 over both runs, against 0.80). The branch is a
few points better than base for Water (about +0.07), not enough. Evidence:
`evidence/base-i046-g1-p0-1680x720-water-after.png` and
`evidence/branch-i046-g1-p0-1680x720-water-after.png`. In both, the stream
reaches the fire from the lower right as a wedge: on screen it is widest
where it is nearest the camera, and the part that crosses the 72 px square
is the stream at about 4-5 m depth (about 30-40 px wide at f = 840), not the
13 px far end the spec's estimate used. The taper (first 0.8 m) does not
reach that part. Frost and CO2 are lower than Water on both builds; their cold
mist sits on the far 70 % of the stream and adds cover (the mask-in-square
diagnostic is higher for them).

Per A1 this is a product finding. The amendment lets the rust lane tune only
inside U1 (`thick·outline_scale >= 0.1875`): from 0.216 m to 0.1875 m is 13 %
narrower, and the branch-over-base gain for a 24 % narrower rod was about
+0.07, so U1's floor is not expected to reach 0.80 for Water (and not for
Frost or CO2). The web lane did not loosen 0.80, the square or the freeze
rule. **Stopped and reported** for the planner.

## A3. G5: a press with no move

Implemented as written: `page.mouse.down()` + `page.mouse.up()` at the current
position, the existing `page.mouse.move(VW/2, VH/2)` kept; 5 still frames
before the aim and 5 still frames with `aimObj === lit[1]` and
`cooldown === 0` after it (5 s cap); the recorder logs `yaw`, `pitch`,
`aimObj` and `casts` every frame; the precondition at the cast frame; up to 3
attempts, a count failure fails the count check, all invalid fails both
checks with "view moved during the click". Pass conditions unchanged.

**Result: the press with no move still turns the view.** Every attempt in
both full runs and the diagnostic run was invalid:

| run | attempt | Δyaw at the cast frame | Δpitch | aimObj press -> cast frame | count |
|---|---|---|---|---|---|
| full run 1 | 1, 2, 3 | 10.88° | 10.89° | 3 -> -1 | ok (1 -> 2 -> 2, 0 -> 1 -> 1) |
| full run 2 | 1, 2, 3 | 10.88° | 10.89° | 3 -> -1 | ok |
| diagnostic | 1 | 10.88° (21.77° a frame later) | 10.89° (21.77°) | 3 -> -1 | ok |
| diagnostic | 3 | 21.77° | 21.77° | 3 -> -1 | ok |

Per-frame recorder rows, diagnostic attempt 1 (`[frame, gameTime, lit[1]
burning, casts, yaw, pitch, aimObj]`, a = frame 55, yaw 302.2, pitch -2.87):

```
[56,7.5,true,1,302.2,-2.87,3] [57,7.75,true,1,302.2,-2.87,3] [58,8,true,1,302.2,-2.87,3]
[59,8.25,true,2,313.08,8.02,-1] [60,8.5,true,2,323.97,18.9,-1] [61,8.75,true,2,323.97,18.9,-1]
```

Cause (from the event dump, listener on `window` in the capture phase,
registered before `hud.js`): under the pointer lock, headless Chromium follows
**each** of `pointerdown` and `pointerup` with a `pointermove` whose movement is
`(-640, -360)`, that is `(-VW/2, -VH/2)`: the locked pointer reports client
(0,0), and the press at (640,360) is turned into a move from there.
`hud.js`'s `delta` clamps each to ±200 px instead of dropping it, so each
button event turns the view by 200 px · 0.00095 · (180/π) = 10.9° on both axes.
That is two saturated events per click (21.8°), exactly the round-2 numbers.
So the amendment's premise (the move inside `page.mouse.click` is the trigger)
is not the whole cause: the button events themselves carry the bogus move.

Per step 8 the lane stops and reports. It did not widen 0.5°, add attempts
or change the setup further. The A3 mutations were not run: with no valid
attempt possible, the Fire-at-`lit[1]` mutation cannot reach the extinguish
judgment, and the aim-away mutation can only end as "view moved" (which the
real runs already show is a FAIL, never a pass or skip).

Two possible fixes, neither in this lane's scope, for the planner:
1. The backlog note's `hud.js` fix (drop, not clamp, a locked event whose
   movement is implausible). It would make test 9 pass as written, and the
   game would stop turning on these events for real players too.
2. A test-side setup change: move the mouse to (0,0) (where the locked pointer
   reports itself) before the press, and let the view settle, so the
   synthesized moves are (0,0). That is a change to A3 step 1 and needs a
   spec decision.

## Still red after this round

- G1 P0 on the branch (product finding, A1 stop rule).
- `cast: a real click casts exactly once` and `cast: that real click puts lit[1] out`
  (G5 invalid on every attempt, A3 step 8 stop rule; the count itself was right
  in every attempt).

- Run 2 only: `clock: cast cadence keeps the 0.3 s cooldown in wall time`
  (pre-existing, outside this lane, load flake; passed in run 1).

Everything else passed in both full runs, including G1 P1/P2, G2, G3, G6 and
every other pre-existing check.
