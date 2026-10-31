# I-044 spec critique

**Verdict: revise**

Checked against origin/loop/I-022 at 7b42fc5: `main.rs` 585, 617-623, 975-991, 1520-1541 and the commands at 757-812; `feel.rs` (`vfov_for`, `ray_dir`); `player.rs` (`look`, `face`); `hud.js` 407-450 and 655-676; `golden.mjs` 1923-1964.

## Right thing? Yes
The three problems are real and the code shows them.
- (b) The gem's angle checks out. Local `(0, 0.05, -0.11)` × 0.55 on the root at `(0.21, -0.18, -0.42)` puts it 17.6 deg below centre. `vfov_for(70, 21/9)` = 33.4 deg, so the half-angle is 16.7 and the gem is off screen. The gem's colour is the only in-world sign of which spell is ready, so this matters for B4.
- (c) The flash happens as described. `frame()` chooses the hint from `state.locked`, and `hintAllowed` checks `input.locked`. Moving the gate to `state.locked` is the smallest fix.
- (a) The centre-only hover check cannot fail. `ray_dir(0.5, 0.5, ..)` is `(0, 0, -1)` for any fov.

The scope is small: one pure function, two literals, one gate, and golden checks. It stays first-person and does not drift from the hard rule.

## Required changes

1. **Test 6's wrong-ray control cannot fail as written.** With View width 90, a "16:9 window" ray uses vfov 58.72 and aspect 16/9. Its horizontal half-tan is tan(29.36°)·16/9 = 1.00, which is the same 90° horizontal fov as the live view at 4:3 and at 21:9. So P'.x equals P.x. The pose (`face`, then a yaw-only `turn`) also leaves the target vertically centred, where a vfov error moves nothing. P' is therefore about equal to P, and `|P' - P| >= 25 px` fails on every pose in the fallback list. The check fails for a broken test, not for broken code. Fix it as follows:
   - Put the target off-centre on **both** axes. After `face`, `turn` with dx and dy so the target sits near (0.75W, 0.30H). The vertical offset is tan = 0.4·tan(vfov/2). Require P within 0.05W and 0.05H of that spot.
   - Use controls that model the bugs I-022 fixed, and require each control to land at least 25 px from P with `hover !== lit[0]`:
     - **(i) Stale vfov, live aspect:** `ray_dir(.., 58.72, W/H)`. At 4:3 the x shift is about 80 px; at 21:9 it is about 100 px.
     - **(ii) Live vfov, 16:9 aspect:** `ray_dir(.., state.fov, 16/9)`.
   - Keep "log the pose used" and the fallback poses.
2. **Test 6 cannot "fail on 7b42fc5".** I-022 already builds the ray from the live `vfov_for(hfov, aspect)` (`main.rs:988-991`), so the correct-ray hit passes there. The spec should say: test 6 **passes** on 7b42fc5, and the wrong-ray controls in change 1 show it can fail. The lane records the control distances and hover reads as that evidence. Only tests 7 and 9 (or 9's source check) must fail on 7b42fc5.
3. **The unit tests never exercise the 4 deg floor.** Test 2's "`>= 4` always" cannot bite, because the smallest v in its grid is 33.4, which gives 11.7. Test 3 checks only that the function never decreases. Add `vm_gem_deg(10.0) == 4.0` and `vm_gem_deg(16.0) == 4.0` to test 3.
4. **Fix the Out-of-scope wording about the floor.** It says aspects wider than 21:9 clamp the gem at 4 deg. `vfov_for` clamps aspect to 4, and the smallest View width is 70, so the smallest v/2 - 5 is about 4.9. The floor is never reached with supported settings. Reword it as "the floor is a guard only; at aspect 4 the gem sits about 4.9 deg below centre". Keep the floor or drop it, the builder can choose.

## Nits (no change required)
- The root's rest rotation `Ry(0.1)` puts the gem at z ≈ -0.4802, not -0.4805. That is a 0.003 deg difference and irrelevant. Golden test 7 already applies `Ry(0.1)`, and its 3 deg margin against the 5 deg design margin absorbs it.
- `yaw` and `pitch` are written to state in degrees with 2 decimals, so there is no precision problem in projecting P.
- Test 7's pixel check is sound. Water's glow is `srgb(0.2, 0.55, 1.0)`, so `B >= R + 40 && B >= 120` holds, and the mirrored-box control stops a blue scene from passing it.
- Test 9's fallback source check (`hintAllowed` must not contain `input.locked`) is brittle. It is acceptable because it applies only when the flash cannot be caught, and the observer check stays either way.

## Planner response

1. **Test 6's control cannot fail: accept.** At View width 90 the horizontal half-tan is 1 at every aspect up to 4, and a yaw-only turn keeps the target on the centre row, so the old control moved nothing. Revision 2 places the target at about (0.75W, 0.30H) with `look` (not `turn`, so sensitivity does not matter) and uses controls (i) `ray_dir(.., 58.72, W/H)` and (ii) `ray_dir(.., state.fov, 16/9)`, each required at least 25 px from P with `hover !== lit[0]`. Expected shifts were checked: (i) about 80/48 px (x/y) at 4:3 and 100 px at 21:9; (ii) about 60 px at 4:3 and 130 px at 21:9.
2. **Test 6 cannot fail on 7b42fc5: accept.** `main.rs:988-991` already uses the live `vfov_for`. The spec now says test 6 passes there and its control distances and hover reads are the evidence; only 7 and 9 (or 9's source check) must fail.
3. **Floor never exercised: accept.** Test 3 now asserts `vm_gem_deg(10.0) == 4.0` and `vm_gem_deg(16.0) == 4.0`.
4. **Out-of-scope floor wording: accept.** `vfov_for(70, 4)` = 19.86 deg, so the smallest v/2 − 5 is 4.93 and the floor is never reached. The wording is fixed, and the floor is now described as a guard only, which the builder may keep or drop.

No point disputed, so tie_break was not needed.

### Re-check of Revision 2
- Point 1: test 6 needs P within 0.05W and 0.05H of (0.75W, 0.30H), and both controls are listed with the 25 px and `hover !== lit[0]` requirements. Pose fallbacks and logging are kept. Done.
- Point 2: the golden preamble says tests 7 and 9 must fail and test 6 passes on 7b42fc5. The web lane row records control evidence for test 6 instead of a failure. Done.
- Point 3: unit test 3 has both floor asserts. Done.
- Point 4: Out of scope no longer claims the gem clamps at 4 deg, and a new bullet explains the guard-only floor (about 4.9 deg at aspect 4). Done.
