# I-011 critique (spec critic)

**Verdict: revise** (right thing, small, checkable; three small fixes applied).

## Fit
- Right thing: doc-only, no bar pass rule changes, directly serves B4/B5/B8 and the first-person, ages 4-18, teacher-buyer target. Smallest possible (no client change). Checks are mostly runnable headless (CPU frame ms, bridge state, DOM measurements) or by panel; GS-11 and GS-23 keep the first-person rule intact (click-to-target is still first-person).
- Tests can fail: yes, each GS has a concrete check, and the "known gaps" table shows several currently fail (FOV, sensitivity, HUD text).

## Source spot-check (5 fetched)
1. XAG 117 (learn.microsoft.com): confirms avoid/disable camera shake, bobbing, motion blur, and "disable automatic camera movement". OK.
2. NN/g kids-cognition: "Kids under 6 failed to notice that feedback"; exaggerated expressions suggested; 7-year-old "What does this '2' mean?" OK. The 5-year-old coins claim was overstated (he did not grasp the objective, not the coin/number link). Fixed.
3. CHI 2022 mouse sensitivity (ar5iv 2203.12050): 20-80 cm/360 optimal range, as claimed. OK.
4. KQED Zombie Division: "seven times longer" on the intrinsic version, from the free-choice group, as claimed. OK.
5. Google Teacher Approved: "appropriateness standards for your app's youngest target age group" confirmed. The google.play host looks odd but is what play.google.com redirects to. OK.
Also checked the math: 70 deg vertical = 102 deg horizontal at 16:9; 60 deg = 91 deg; 0.0022 rad/px = about 9 cm/360 at 800 CPI. All correct.

## Changes applied to gameplay-standards.md
- GS-12: said "next rendered frame" but the check allowed 2 frames; now states the 2-frame limit with next frame as the target.
- GS-32: corrected the 5-year-old coin example to match the source.
- GS-5: P0 50 ms hitch rule excludes the first 5 s of load/shader compile (B3 owns load), so it cannot fail for a reason unrelated to pacing.

## Remaining notes (not blocking)
- Many GS items are P0 yet the current client fails several (GS-1, 3, 8, 18); the planner should sequence these as backlog items rather than treat the doc as blocking existing work.
- Wiring GS into critic/breaker/panel prompts remains the stated follow-up.
