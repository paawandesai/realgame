# I-044 web lane, round 2

Branch `loop/I-044`. Answers `break-1.md`. Golden runs `QUERY=?low`, each under the chromium lock, on the round-2 build (rust lane: the viewmodel orbits about the eye, see `../rust/round-2.md`).

## What changed in `tests/e2e/golden.mjs` (test 7 only, inside `reviewFollowups`)
- The gem's camera-space point follows the new Contract: `g = vm + Rx(tilt)·Ry(0.1)·(0, 0.0275, -0.0605)`, with `tilt = 17.6 - clamp(fov/2 - 5, 4, 17.6)` deg recomputed in the test from the live `state.fov` (not read back from the game).
- **Stricter, not looser:** the geometry half also requires `vm` within 0.002 per axis of `Rx(tilt)·(0.21, -0.18, -0.42)`, so `g` is known to be where the gem is. The log line prints the expected orbit pose.
- The pixel half is unchanged: the same 15x15 `blueIn` box at the projected gem centre `G`, the same `>= 10` blue / mirror `< 10` rule. With the orbit the whole gem is visible, so the box stays centred on the gem and still means "gem unoccluded" (the break's should-fix: not loosened, not moved). No other check touched; `hud.js` unchanged since round 1. Golden still has 127 checks.

## Blocking finding: test 7 at 21:9
| case | round 1 (y-only lift) | round 2 (orbit) |
|---|---|---|
| 1680x720, View 70 | FAIL: gem 11.71 deg, G=(1350,609), blue **0** | PASS: gem 11.72 deg (limit 13.70), G=(1336,609), blue **128**, mirror 0 |
| 1680x720, View 75 | weak: blue 45 | PASS: gem 13.22 deg (limit 15.20), G=(1295,617), blue **136**, mirror 0 |
| 960x720, View 70 | PASS | PASS: gem 17.62 deg, blue 105, mirror 0 |
`vm` read live: `[0.2100,-0.1359,-0.4363]` and `[0.2100,-0.1473,-0.4326]`, each equal to the expected orbit pose. Test 8 now reads `[0.2100,-0.1800,-0.4200]` exactly (round 1: -0.1799).

## Full golden, 3 runs on the round-2 build
| run | result | failures |
|---|---|---|
| 1 (`golden-round2-run1.log`) | **127/127** | none (both I-019 wall-time checks passed) |
| 2 (`golden-round2-run2.log`) | 125/127 | `cast: a real click casts exactly once` (casts 1 -> 2 -> 2, the known I-019 wall-time flake); `streams: a slow click (0.35 s) casts once` (span 0.280, max_dt 0.213, casts +2) |
| 3 (`golden-round2-run3.log`) | **127/127** | none |

Run 2's two failures are frame-time checks on cast cadence (a 0.35 s game-time hold against the 0.3 s cooldown with a 0.213 s frame; fps fell to about 1.2-1.4 in that run). This diff does not touch casting, input, streams or the cooldown, and both checks pass in runs 1 and 3. Recorded, not weakened. The second one is I-025's check; it is not on the known-flake list yet and should be added, or watched on main. Tests 6-9 passed in all three runs. So spec test 10's "3/3 green" holds for every I-044 check and for all checks in 2 of 3 runs.

## Age picker (break's unverified item)
The breaker saw no picker because `build-web.sh` does not copy `age.js` / `analytics.js` into `pkg-out` (only `scripts/build-site.sh` adds them to `web/dist`), so a forced-on `pkg-out` page cannot load the picker; DNT/GPC were not the cause (`navigator.doNotTrack` null, GPC null in headless Chromium). I built a site-like copy the way `build-site.sh` step 1 does (pkg-out + `age.js` + `analytics.js`, `rg-analytics` on) and ran `picker-check.mjs` (copied here) under the lock: the picker shows with 3 buttons, focus on `sprout`, **no pointer lock while it is up**, it closes on a pick, and a click on the canvas then locks the pointer. `PASS` (`picker-round2.log`). The diff still does not touch `age.js`, `analytics.js` or the picker markup.
