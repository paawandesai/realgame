# I-046 web lane, round 1

Golden changes (`tests/e2e/golden.mjs`), inside the spec's hunks only:
`async function closeStreams` between the end of `castingYouCanSee` and the
comment that opens `reviewFollowups`; its call line between the
`castingYouCanSee` and `reviewFollowups` calls; the click half of
`castingYouCanSee` test 9 split in two (G5). No key-order line, nothing inside
`reviewFollowups`, no end-of-file insertion. `closeStreams` restores the viewport,
View width, Aim help, sway and lock, and ends on `fresh()` with `streams === 0`.

Casts in `closeStreams` use the game's `cast` command inside the rAF watcher (the
click's queued cast) rather than a synthetic pointer click, because the viewport
changes can drop the pointer lock; the freeze rule (first frame with `impacts`
grown and `streams >= 1`) is I-025 test 7's.

Runs: dev harness = golden with the earlier sections cut (boot, I-019
`castingYouCanSee`, `closeStreams`), `QUERY=?low`, under the chromium lock. Base =
origin/main 76b5f13 (stream code identical to e70df08) built into its own worktree
and served with `BASE_URL`. Full logs: `base-e70-run.log`, `branch-dev-run.log`.

## Failing first (base) vs branch

### G1 P1 / P2 (Aim help off; warm after / before in a 0.16·VH square; pass >= 0.80)

| aspect | spot | spell | base ratio (try 1) | branch ratio (try 1) |
|---|---|---|---|---|
| 1680x720 | P1 | Water | 0.56 | 0.91 |
| 1680x720 | P1 | Frost | 0.57 | 0.93 |
| 1680x720 | P1 | CO2 | 0.49 | 0.88 |
| 1680x720 | P2 | Water | 0.11 | 1.02 |
| 1680x720 | P2 | Frost | 0.12 | 0.93 |
| 1680x720 | P2 | CO2 | 0.07 | 0.91 |
| 1024x768 | P1 | Water | 0.55 | 0.96 |
| 1024x768 | P1 | Frost | 0.58 | 0.93 |
| 1024x768 | P1 | CO2 | 0.56 | 0.98 |
| 1024x768 | P2 | Water | 0.08 | 1.04 |
| 1024x768 | P2 | Frost | 0.02 | 0.99 |
| 1024x768 | P2 | CO2 | 0.08 | 1.07 |

Failing-first requirement met: P2 fails at both aspects for Water on base (and
Frost and CO2). Base matches the model (P1 about 0.55, P2 0.00-0.03 to 0.12).
Check on branch: PASS. Check on base: FAIL.

### G2 (sky pose; frame <= 6 %, band <= 18 %, widest cross-section <= 0.24 f)

| aspect | spell | base frame / band / widest | branch frame / band / widest |
|---|---|---|---|
| 1680x720 | Fire | 8.39 % / 11.2 % / 0.408 f | 2.54 % / 3.9 % / 0.133 f |
| 1680x720 | Water | 11.68 % / 39.2 % / 0.504 f | 3.11 % / 8.3 % / 0.160 f |
| 1680x720 | Frost | 11.63 % / 38.9 % / 0.504 f | 3.07 % / 8.2 % / 0.160 f |
| 1680x720 | CO2 | 11.64 % / 38.9 % / 0.504 f | 3.08 % / 8.2 % / 0.160 f |
| 1024x768 | Fire | 5.08 % / 10.9 % / 0.420 f | 1.44 % / 1.2 % / 0.133 f |
| 1024x768 | Water | 7.70 % / 33.4 % / 0.549 f | 1.79 % / 2.5 % / 0.162 f |
| 1024x768 | Frost | 7.63 % / 33.1 % / 0.549 f | 1.75 % / 2.4 % / 0.162 f |
| 1024x768 | CO2 | 7.65 % / 33.2 % / 0.549 f | 1.76 % / 2.4 % / 0.162 f |

Failing-first met (Water fails at both aspects on base). Model: base 10-14 %,
36-44 %, 0.57 f; fix 1.7-3 %, 2.5-10 %, about 0.17 f. Branch PASS, base FAIL.

### G3 (attached along C -> Hs, r = 20 px to 0.48 f)

Base and branch: attached for all 4 spells at both aspects (a guard; a gap design
would fail it). PASS on both.

### G6 (1680x720, View width 70; tilt 5.90 deg, Hs' = (1308, 589), 229 px below C)

| | base Water | base Frost | branch Water | branch Frost |
|---|---|---|---|---|
| (a) farthest in-band stream px to Hs' (limit 25.2 px) | 340.0 px (at the bottom edge) | 340.0 px | **25.6 px** | **25.6 px** |
| (b) stream px > 0.04·VH past Hs' | 638 px, up to 339 px past | 614 px | 0 | 0 |
| (c) attached | yes | yes | yes | yes |

Failing-first met: (b) fails on base for Water. On the branch (b) and (c) pass and
the stream visibly ends at the gauntlet's gem (evidence
`branch-i046-g6-1680x720-vw70-water-after.png`), but **(a) fails by 0.4 px**
(25.6 vs 25.2), identically on all 3 attempts for both spells. Cause, measured
from the frozen frame: the turned hand point `Hs'` lies behind the spell gem, so
the last visible stream pixels are where the gem's silhouette crosses the edge of
the 0.03·VH band (along +14 px, across -21.6 px from `Hs'`). The stream itself is
about 76 px wide at the hand at this view (0.0324 m at 0.5 m with f = 1199 px) and
continues behind the gem. Tuning inside U1's limits cannot move that point (it is
set by the gem silhouette, not the stream width; the stream would have to be
narrower than lead_min 0.10 allows to stop reaching the band edge there). Per the
spec I did not loosen the threshold or redefine the band. **Stopped and reported.**

### G1 P0 (Aim help on, fire at the crosshair, 0.10·VH square, pass >= 0.80)

| | base | branch |
|---|---|---|
| 1680x720 Water/Frost (all tries) | after 0, ratio 0.00, `burning=false` | after 0, ratio 0.00, `burning=false` |

P0 fails on base and branch alike, for a reason that is not the stream: the frozen
frame is defined as the first frame where `impacts` has grown, and with Aim help on
the cast lands on the targeted fire and puts it out on that same frame
(`burning=false`, 0 warm pixels left). No stream shape can make it pass. The
stream-mask overlap diagnostic is not meaningful here (the vanished fire itself
counts as changed pixels). Per the spec's P0 rule, the lane did not tune anything
and **stops and reports**: P0 as specified cannot pass for an extinguishing spell;
the planner needs to redefine it (for example, freeze one frame before impact, or
use a target the spell cannot put out).

### G5 (split real-click check)

`cast: a real click casts exactly once` (count only, 20 + 20 frames) and `cast:
that real click puts lit[1] out` (an in-page recorder from before the click to the
first frame with `gameTime >= a.gameTime + 2.0`; 600-frame and 15 s caps end the
wait as a failure). Neither half is weakened.

| run | casts exactly once | puts lit[1] out |
|---|---|---|
| base dev run | PASS (1 -> 2 -> 2) | FAIL (never out in 2.0 game s, 7 frames) |
| branch dev run | PASS | FAIL (same) |
| mutation (view turned 40 deg off lit[1] before the click, local only) | PASS | FAIL |

The mutation check behaves as required. The extinguish half fails on base too:
this is the cause of the known main flake on the old combined check (it read
`lit[1] burning=true` in 4 of 8 recent main logs). The new diagnostic in the count
check shows why: the cast's aim at the cast frame is not `lit[1]` (in the mutation
run `aimObj` at the cast frame is -1 and the view moved further than the 40 deg the
test turned it, with pitch -2.87 -> 8.02, so the real `page.mouse.click` under
pointer lock also turns the view). The real click misses, so the fire stays lit.
That is a game- or harness-side issue outside this item's paths (the click
handling in `hud.js`/`main.rs` input code, or test 9's click setup above the split
block). The split itself is sound, so G5 is in, and it now fails honestly where the
old check flaked. **Reported, not fixed here.**

## Full golden on the branch

See the "Full run" section appended below.
