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

Full `BASE_URL= QUERY=?low node tests/e2e/golden.mjs` at `0668400` (the notes commit changes no code), each run alone under the Chromium lock. 124 checks: 107 existing + 17 new.

| run | result | failures |
|---|---|---|
| 1 | 123/124 | `cast: a real click casts exactly once` (casts 1 -> 2 -> 2, lit[1] burning=true) |
| 2 | 123/124 | same |
| 3 | 122/124 | same, plus `clock: cast cadence keeps the 0.3 s cooldown in wall time` (casts 9, need 10, 18 frames) |

All 17 `streams:` checks passed in all 3 runs. I-019's blue-pixel check (after 54794 px, need 5676), its flashing guard, its `holding the button keeps casting` (+3 casts over about 1.55 game s), test 16 (`hid: another fire in view` after 3 steps) and I-022's two `review: Hints off ...` checks passed in all 3 runs, with no edits.

**Neither failure comes from this branch:**
- `cast: a real click casts exactly once` fails the same way on the base. Run as `ONLY=castingYouCanSee` on the stub build (05d43a5 plus the neutral stub, which does not touch casting) on the same machine the same night: `FAIL cast: a real click casts exactly once casts 1 -> 2 -> 2 (20 frames on), lit[1] burning=true`. I-022's round note records the same failure on its a0e2ecb base, and I-024's on main at load average 4.7. The click casts once (casts +1), but lit[1] is still burning 20 frames later. It needs its own item; this item does not edit the check.
- `clock: cast cadence ...` sends `cast` commands every 100 ms and never uses the held path, so the hold delay cannot change it. I-022 recorded the same miss (casts 9 or 10, need one more, at 18-19 frames) when frames were slow. Load average was about 4-6 during these runs, from other sessions on the box.

So the gate "every existing check 3/3" is **not met**, but only by the two failures above, which also happen on main. Test 17 (perf pairs against a 05d43a5 build) is the tester's job and was not run here. Brotli size is in the rust note (+1,310 B).

Slow click (test 9) per run: run 1 had one invalid attempt (span 0.382, max_dt 0.182, casts +2, so it was retried) and then a valid one (span 0.244, +1 cast). Runs 2 and 3: first attempt valid (span 0.293 and 0.226, +1 cast). First repeat while held: 0.537, 0.648 and 0.640 s after the click.
