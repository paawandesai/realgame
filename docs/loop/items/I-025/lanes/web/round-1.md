# I-025 web lane, round 1

Commits on `loop/I-025`: `339aacf` (streamsAndFinder, pickHint, findOn), `4bdea52` (science rows, I-019 amendment), `790cc24` and `0668400` (test 9 press timing).

## What changed

- `hud.js`: module-level `pickHint({ playing, idle, note, locked, extinguished, casts })` after the constants, returning `''`, `IDLE_HINT`, `note`, `AIM_HINT`, `''` or `START_HINT` in today's order (banner > idle > note > locked > start), with the order and this spec cited above it. `frame()` keeps `if (state.locked) note = ''` and `updateIdle(now)` and then calls `setHint(pickHint({ ..., idle: idleOn, note, ... }))`. `findOn` gains `&& !state.fireInView`. `window.__game.pickHint` added. `hintAllowed`/`setHint` (441-450 at 05d43a5) and `updateIdle` are untouched.
- `golden.mjs`: `streamsAndFinder(...)`, defined just before `castingYouCanSee` (above its header comment) and called after `feelAndStandards`; call order feel, I-025, I-019, I-022. Its helpers are local copies. It hides `#hud`/`#pause` with `style.visibility` for every screenshot of tests 7, 8 and 10, and restores them. It ends `fresh()`, unpaused, sway restored, and relocked if it was locked on entry (Hints are never changed).
- `content/science-sources.md`: P-017 (Water), P-018 (Frost), P-019 (CO2); P-016 is still reserved. Sources fetched 2026-10-30:
  - [S21] Braun and Smirnov, J. Chem. Educ. 70(8) 612 (1993), via https://www.webexhibits.org/causesofcolor/ref/Braun-Smirnov.html: intrinsic blue from absorption in the red, seen through several metres of water. Water stays `simplified`.
  - [S22] Michigan Tech Physics, "Liquid Nitrogen Safety", https://www.mtu.edu/physics/research/safety/pdfs/liquid-nitrogen-safety.pdf: "the cloud that forms when you pour liquid nitrogen is condensed water vapor from the air, not nitrogen gas". Frost is `correct`.
  - [S23] U.S. Navy NAVEDTRA 14146, https://navyadvancement.tpub.com/14146/css/Halon-124.htm: "A typical discharge of liquid CO2 has a white, cloudy appearance caused by finely divided dry ice particles ... Some water will condense from the atmosphere creating additional fog". CO2 is `correct`.
  - **Deviation:** the register lint (`test/science-sources.test.ts`) requires a numeric `value`, so the claim-and-limit text could not go in that cell. Each row's `value` is the unit test's lower limit (0.8, 0.75, 0.85, unit sRGB), the `constant` names the limit, and the note carries the claim in the unit-test words. The value is still a limit, not the tuned colour, so it cannot go stale. `npm test`: 53 pass, 0 fail (`33 client claims, 33 sourced, 18 params, 0 error verdicts`).
- `docs/loop/items/I-019/spec.md`: the amendment paragraph appended at the end.

## Failing first (05d43a5 + stub, `e37a8f5`, `?low`)

Built from `e37a8f5` and run as `ONLY=streamsAndFinder` through a scratch copy of golden that stops after the pointer lock check (the scratch copy is not committed).

| test | check | stub | this branch |
|---|---|---|---|
| 5 | fireInView is a boolean, the last state key | PASS (the stub adds the key, see note) | PASS |
| 6a | no arrow while aiming at a fire | FAIL: target 0, nearFire 6, fireOff true, fireInView false, finder shown | PASS (fireInView true, finder hidden) |
| 6b | finder exactly when no fire in view (sweep) | FAIL (aimed fire with fireInView false at yaw 0, 60, 300) | PASS (24/24 yaws) |
| 6c | sweep both suppresses and shows | FAIL (0 suppressed, 19 shown) | PASS (8 suppressed, 11 shown) |
| 6d | occlusion recorded | PASS | PASS; logged `tp 0 -30`, face lit[2]: aimObj -1, nearFire 6, fireOff false, fireInView true, finder hidden (bearing-only, as before: the known gap for the panel brief) |
| 7 | Water vs sky | FAIL 0/10, runs all 0 | PASS 10/10 |
| 7 | Frost vs sky | FAIL (no stream: caught -1) | PASS 10/10 |
| 7 | CO2 vs sky | FAIL 0/10 (runs 0.5 r: the 0.13 m cone) | PASS 10/10 |
| 8 | Water / Frost / CO2 vs ground (pose -4,14, lit[0]) | FAIL 2/10, 2/10, 2/10 (0 skipped) | PASS 10/10 each (0 skipped) |
| 9 | slow click casts once | PASS* | PASS |
| 9 | hold repeats, first after 0.5 s | FAIL (first repeat 0.348 s) | PASS (first repeat 0.689-0.708 s) |
| 10 | Frost/CO2 flashing | PASS | PASS |
| 10 | Frost/CO2 mist coverage | PASS | PASS |
| 11 | pickHint table | FAIL (`pickHint` missing) | PASS |
| 11 | point-and-click note within 2 frames | PASS | PASS |

Notes:
- Test 5 cannot fail on the stub: the Contract has the stub add `fireInView`, and the key-order check is what enforces it. It would fail on bare 05d43a5.
- *The stub's slow-click PASS was a test bug: g0 was read after the press had already landed, so the press ended after one frame (span 0.000). Fixed in `790cc24`/`0668400`: g0 is read just before `mouse.down()`. Test 9 still fails on the stub through its hold half, and the probe below shows the stub double-casts a 0.35 s press.
- Test 7 did not pass on 05d43a5 for Water, so no stop was needed.

### Cross-section runs (px, r = 20..110), this branch

| test | spell | runs | need 0.6 r |
|---|---|---|---|
| 7 sky | Water | 29 42 56 70 81 96 109 121 129 137 | 12 .. 66 |
| 7 sky | Frost | 28 41 54 70 80 95 109 121 128 135 | |
| 7 sky | CO2 | 28 41 54 70 80 96 109 121 128 135 | |
| 8 ground | Water | 39 51 63 78 90 103 112 121 129 137 | |
| 8 ground | Frost | 40 50 62 78 88 102 112 121 109 108 | |
| 8 ground | CO2 | 40 49 63 78 88 102 112 121 109 108 | |
| 7 sky, stub | CO2 | 10 16 21 27 31 36 41 47 51 57 | |

The run is capped by the segment, 2(0.4 r + 24) + 1 px (137 at r = 110).

### Test 9 frames (probe, Water, `look 0 60`)

Stub, 0.35 s press: frames `[gameTime, dt, casts, down]` 5.954/0.25/1/down, 6.204/0.25/1/down, 6.358/0.154/2/down, 6.43/0.072/2/down, 6.574/0.144/3/down ... a second cast 0.40 s after the click and a third at 0.62 s. This branch: casts stay at 1 until the press has been held 0.5 s of game time (the probe press that lasted 0.566 s gave its one repeat on that frame). Test 10 numbers: flashing worst frame-to-frame change Frost far 0.6 %, Frost near 7.0 %, CO2 far 0.4 %, CO2 near 6.1 %; near-white coverage at most 1.0 % (limits 20 % and 15 %). The near pose is `tp 0 0` facing lit[2], which aims at lit[2].

## Panel brief note (test 16)

The Water, Frost and CO2 streams now have dark outlines (a toon convention, an inverted hull). An outline is not a shadow bug. The stream is also wide at the hand (about 280 px core, 370 px with the outline at 1280x720), as the GS-6 row in the spec expects. Known gap for the panel: a fire behind the greenhouse whose bearing is on screen counts as "in view" (test 6d), so no arrow shows there (follow-up: occlusion-aware finder).

## Golden

GOLDEN_PLACEHOLDER
