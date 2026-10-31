# I-044 web lane, round 1

Branch `loop/I-044`. Golden runs with `QUERY=?low`, under the chromium lock.

## What changed
- `tests/e2e/golden.mjs` (inside `reviewFollowups` only): test 10's centre-only hover read and its check are replaced by test 6 (`review: the cursor ray matches the view off-centre at 4:3 and 21:9`, Aim help off, restored in `finally`); a new block after test 10 adds test 7 (`review: the gauntlet's spell gem stays on screen at 21:9`), test 8 (`review: the gauntlet does not move at the default view`) and test 9 (`review: Esc with Hints off never flashes the aim hint`). Helpers `toCam`, `toPx`, `lookTo`, `hoverAt`, `rayOffCentre` sit just before test 10.
- `crates/bevy-client/web/hud.js`: `hintAllowed` reads `(!!state && !state.locked && state.casts === 0)` instead of `!input.locked`.
- Test 9's flash was caught by the observer on the base, so the fallback source check was not added: golden has 127 checks.

## Tests first: the new checks on the base build (main 4c7c9a1, code = b8fa548)
Full log: `base-review-run.log` (only `reviewFollowups` plus boot ran; 41/43).

| check | base | needed |
|---|---|---|
| 6 off-centre cursor ray | PASS | pass (controls are the evidence) |
| 7 gem on screen at 21:9 | **FAIL** (1680x720 View 70: gem 17.62 deg below, limit 13.70, G=(1350,741) off screen, blue 0) | fail |
| 8 default view unchanged | PASS (vm [0.2100,-0.1800,-0.4200]) | pass |
| 9 Esc flash, Hints off | **FAIL**, caught on try 1: shown ["Aim at a fire and click.", "Click to look around. ..."] | fail |

### Test 6 evidence on the base (pose `tp 0 5.5`, `face lit[0]`, first pose passed at both sizes)
| viewport | fov | P | hover at P | control | P' | abs(P'-P) | hover at P' |
|---|---|---|---|---|---|---|---|
| 960x720 | 73.74 | (728,234) | 0 (= lit[0]) | (i) stale vfov 58.72 | (810,192) | 92.6 px | -1 |
| 960x720 | 73.74 | | | (ii) 16:9 aspect | (666,234) | 61.9 px | -1 |
| 1680x720 | 46.40 | (1264,236) | 0 | (i) stale vfov 58.72 | (1163,266) | 105.2 px | -1 |
| 1680x720 | 46.40 | | | (ii) 16:9 aspect | (1396,236) | 132.5 px | -1 |

Same numbers on the branch build.

## The branch build: full golden (`golden-full-run.log`, 125/127)
| check | result |
|---|---|
| 6 off-centre cursor ray | PASS (numbers as above) |
| 7 gem on screen at 21:9 | **FAIL**: see below |
| 8 default view unchanged | PASS: vm [0.2100,-0.1799,-0.4200] |
| 9 Esc flash, Hints off | PASS: shown ["Click to look around. ..."] only |
| `cast: a real click casts exactly once` | FAIL: the known I-019 wall-time flake under box load (casts 1 -> 2 -> 2); recorded, not touched |
| every other existing check | PASS |

A first full run was killed by the harness's 30 min background limit while another item's golden held the box (fps about 1.5); it is not counted.

### Test 7: before / after
| viewport, View width | vfov | gem below centre, before | after | limit (fov/2 - 3) | geometry | G after | blue in box at G (mirror) before | after |
|---|---|---|---|---|---|---|---|---|
| 1680x720, 70 | 33.41 | 17.62 | 11.71 | 13.70 | after PASS | (1350,609) | 0 (0), G off screen | **0** (0) |
| 1680x720, 75 | 36.41 | 17.62 | 13.21 | 15.20 | after PASS | (1305,617) | 123 (0) | **45** (0) |
| 960x720, 70 | 55.41 | 17.62 | 17.61 | 24.70 | PASS both | (771,578) | 105 (0) | 105 (0) |

The geometry half of test 7 passes everywhere on the branch. The pixel half fails at 1680x720 with View width 70, because the gem is lifted but **its own gauntlet hides its centre**.

### Why: the gold wrist ring hides the gem's lower half when it is lifted (spec defect)
The gold ring child (local z -0.03, scale (0.13, 0.13, 0.06)) has its top at local y 0.065, above the gem's centre (local y 0.05), and it sits between the eye and the gem. In camera space the ring's far top edge is at (root y + 0.03575, z -0.453) and the gem centre at (root y + 0.0275, z -0.4805). The ray to the gem's centre clears that edge only when the gem is at least **16.7 deg** below centre (tan > 0.300). At the default 17.6 deg it clears by 0.06 deg. A y-only lift to 11.7 deg puts the ring's edge 0.31 deg (about 7 px) above G, and to 13.2 deg 0.22 deg (about 5 px) above G, so the 15x15 box at G sees 0 and 45 blue pixels (3 rows), as measured. Screenshots (HUD hidden): `shots/gem-1680-70.png`, `shots/gem-1680-75.png`, `shots/gem-960-70.png`. In each, only the top of the gem shows above the gold block. The cue is on screen as a blue dome, but its centre is not.

So with the Contract (only the root's y moves; children, scale, rest rotation fixed) the gem's centre cannot be both on screen with the 5 deg margin at 21:9 / View width 70 (needs <= 11.7 deg) and unoccluded (needs >= 16.7 deg). Test 7 was not weakened. Options for the planner:
1. Orbit the whole viewmodel about the eye by (17.6 - vm_gem_deg) when lifting (translation and an equal Rx tilt). The gem then looks exactly as it does at the default view and stays unoccluded. This changes the Contract (`g = vm + Ry(0.1)·offset` would need the tilt) and test 7's formula.
2. Raise the gem (or lower the gold ring) in the child list so the ring never covers it. Touches children, and the default view changes.
3. Centre test 7's pixel box on the visible dome (for example G minus the gem's projected radius / 2 in y). A spec change to the test, not a loosening by the builder.
