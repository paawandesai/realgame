# I-046 web lane, round 2

## Blocking: the round-1 break was incomplete (nothing run in a browser)

Everything the breaker could not run was run on the round-2 build (wasm gzip
5 086 327 B), each under the chromium lock.

### Full golden: 130/133 (`round2-golden-run2.log`)

| failing check | status |
|---|---|
| close streams: the targeted fire at the crosshair stays visible (Aim help on), G1 P0 | open from round 1, same numbers: the fire is already out on the frozen frame (after 0, `burning=false`). Round 1 reported this as a spec defect. Not changed here: fixing it means changing the check's rule, which needs a spec decision |
| close streams: the stream leaves the lifted hand at 21:9, View width 70, G6 (a) | open from round 1, same numbers: farthest stream px 25.6 px from Hs', limit 25.2 px. (b) and (c) pass. The expected hand point, now taken from feel.rs's constants, is the same as before (tilt 5.90°, Hs' (1308,589)) |
| cast: that real click puts lit[1] out (G5 second half) | known flake: the click carried view movement (yaw 302.2 -> 324.0, pitch -2.9 -> 18.9, aim at the cast frame -1), the input issue logged in round 1. Its first half, "a real click casts exactly once", passed |

Every existing check passed, including the three wall-time checks, "review: the
cursor ray matches the view off-centre at 4:3 and 21:9" and both "review:
Hints ..." checks. The first golden attempt (`round2-golden-run1-killed.log`,
115 pass, 0 real fails) was killed by the harness's 30 min background limit
mid-`closeStreams` while it waited on the lock behind another item, and was
rerun.

### Play-through at 16:9, 4:3, 21:9: 9/9 (`round2-play.mjs`, `round2-play.log`)

On a site-like copy (pkg-out + age.js + analytics.js, rg-analytics on, as
build-site.sh makes it), a fresh context per aspect (1280x720, 960x720,
1680x720):

- Age picker: visible before any pointer lock, 3 bands, closes on a pick. PASS x3.
- Hints: start line shows; locked, the hint is "Aim at a fire and click.",
  Hints off clears it, on brings it back; unlocked with Hints off the start line
  stays, as designed. PASS x3. (The first version of this script expected the
  start line to vanish with Hints off. That was my script's error; the golden
  guard "Hints off keeps the start line" is the designed behaviour.)
- Edge aim: point-and-click mode, view turned 35°, so the burning fire sits at
  94-95 % of the width. A real click there casts exactly once (casts 0 -> 1, a
  stream live) and puts that fire out (wet 0 -> 0.98, 800 -> 97 °C). No page
  errors. PASS x3.

## Should-fix: G6 recomputed the tilt with copied constants

G6 now reads `VM_GEM_DEG`, `VM_EDGE_MARGIN_DEG`, `VM_GEM_MIN_DEG` and
`VM_HAND` from `crates/bevy-client/src/feel.rs` and applies `feel::vm_hand`'s
formula (VM_HAND turned about the eye by `VM_GEM_DEG - clamp(vfov/2 - margin,
min, VM_GEM_DEG)`), with a cross-reference comment. If the constants are
retuned, the check follows them. If one is renamed or reshaped, G6 fails with
"could not read ... update G6 to match feel::vm_hand" and does not give a
wrong pixel verdict. The state line cannot report the hand: the state format
string is outside this item's lanes. The check's limits and rules are
unchanged.
