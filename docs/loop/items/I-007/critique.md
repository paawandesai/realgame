# I-007 spec critique (fresh eyes)

**Verdict: revise (minor).** The design is right and the smallest honest set. Five small fixes, now applied to `spec.md`. After them, ready to dispatch.

## Why the shape is right
- It is the first-person game: FOV, look speed, hand sway, crosshair ring and hit marker all serve the shooter-style control scheme in `target.md`. No camera drift, no 2D.
- It is the smallest set that clears the P0s it touches. GS-1, 3, 4, 8, 10, 12, 15, 17, 18, 19, 21 and 24 each need something here. The seven settings controls each map to a P0 or a needed off switch (sens GS-8, FOV GS-1/2, sway GS-3, assist GS-10, hints GS-24, sound for the new audio, text size GS-18). Nothing is gold-plated. Folding I-016 in and dropping CastGate is sound.
- Tests can fail on a94fc48 (the spec says why for each), the unit tests cover the new pure functions, and the maths checks out: 2π/0.00095 / 315 = 21.0 cm/360, 100 px = 5.443°, 90°h = 58.72°v, and the old golden look thresholds still pass (10.9° and 6.5°).
- Honest about what it does not meet: GS-23 (not met, backlog), GS-5, GS-9, GS-20 and GS-25/26 deferred with reasons.

## GS P0 walk
1, 3, 4, 8, 10, 12, 15, 17, 18, 19: met and tested. 6: tested for new effects only (the spec says the existing-VFX audit goes to the backlog; that must really be filed). 5: not touched, guarded by the median (test 21). 11, 27, 28, 30–32: not touched, no new risk. 21: one grade-3 hint plus a visual glow for pre-readers, no auto-dismiss. 23: not met, out of scope, backlog row required. 24: met, with the fixes below.

## Changes made to spec.md
1. **Wrong number in item 7.** It said the marker is "under 0.5%" of the screen. A 96 px `out` star on 1280×720 is about 1%. Corrected (the GS-6 conclusion is unchanged).
2. **Loudness unspecified (item 8).** Default-on sound in a classroom with no level is a startle risk for 4-year-olds and for teachers. Added a peak gain of at most 0.15 and a `peak` field on `lastSound()`.
3. **Idle hint and pause (item 10, test 18).** Paused time could count toward the 45 s, so a player returning from Esc could be nudged at once, which also cuts across GS-7. The idle clock now restarts on pause and resume, with a test step.
4. **Idle hint is quest-specific.** The text names Water and a fire. It is right for this slice but wrong for later quests (B8 asks for three or more). Said so, and told the planner to file a backlog row so each new quest brings its own hint.
5. **GS-22 claimed but untested.** The walk said new controls are at least 24 px with no check, and native checkboxes are about 13 px. Added a size assertion to test 16 (24 px minimum, 40 px for `#resume` and `#pointmode`).

## Notes for the planner (no spec change)
- `areas.md` lists `tests/e2e/**` as tester-owned, but the single lane owns `golden.mjs`. Keep it as one lane, but the tester should review the new checks, and the item notes should record the exception.
- Halving look speed (0.0022 to 0.00095) changes how the panel personas' `drag` moves turn the view. Record the change in the round notes so a clarity dip in the next panel is not misread. The slider covers trackpads.
- Test 18 adds about 50 s of wall clock and test 8 about 10 s to each of the 3 golden runs. Accepted.
- Test 8 is a guard that passes today (the spec says so). Test 19 only checks small HUD effects, so it is cheap insurance, not a hard test.
