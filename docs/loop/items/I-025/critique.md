# I-025 spec critique (spec critic, fresh context)

**Verdict: REVISE (accept after the Must-fix items below). Not `wrong-thing`.** The item is the right thing. It fixes four real, small confusions that the I-019 breaker found, for the 4-8 band that B4 is about. It stays first-person and adds no text. The overlap with I-022 and I-024 is clean (checked below). The tests are mostly able to fail on a faint stream. But the stream mesh is specified in a way that cannot be built as written, one limit table contradicts its own unit test, and the mist can put large white blobs in front of the camera. Fix those before the lanes start.

Checked against: `target.md`, `bars.md`, `areas.md`, `gameplay-standards.md` (origin/main c4832d2), code at a0e2ecb (`main.rs`, `cues.rs`), I-022 and I-024 specs.

## Right thing for kids 4-18

- Yes. Four things, all follow-ups to a merged item: a stream a 7-year-old can see, an arrow that does not point away from the fire being aimed at, one click = one cast, and the hint order written down. This is the B4 "lands a cast within 30 s" path, so it is on the critical path to the panel. It does not wait on, or conflict with, the learn/master/cast roadmap in `target.md`.
- Smallest version: it is already near the smallest. Splitting into two items (streams vs. finder/click/hint) would add a golden-run cycle for no gain, because they share the golden edits and the key list. Keep one item.
- GS walk is mostly sound (6, 9, 11, 12, 15, 17, 24 fine). Two corrections below (GS-19 and GS-6).

## Must-fix

1. **The taper cannot be built as written.** Item 1 says `kit.cyl` "scaled `(thick*w(a), len, thick*w(a))` where `w` narrows linearly from 1 at the hand to `head_frac` at the head". One `Transform` scale gives one constant radius. A cylinder cannot narrow along its length. The lane would have to guess: use the cone (then the far end is wrong), or make `w` depend on flight progress (the whole stream thins over time, which is not what the text says).
   - **Change:** drop `head_frac` and the taper. The stream is a constant-width cylinder of `thick`. The spec already notes that perspective makes it look tapered (`4T*r` px). Remove `head_frac` from `StreamStyle`, the table and unit test 1. Say "constant width".
   - This also fixes a test margin: the far end at `head_frac` 0.8 and `thick` 0.18 is 0.576 r px wide, below test 7's 0.6 r threshold, so the core alone could fail at the legal minimum and only the outline would save it.

2. **The Water limits contradict unit test 1.** The table allows Water core relative luminance <= 0.30. Test 1 needs core contrast against the sky >= 2.0. The sky `[0.55, 0.78, 0.98]` has L about 0.53, so contrast = (0.53+0.05)/(L+0.05) >= 2 needs L <= 0.24. A colour at L 0.25-0.30 satisfies the table and fails the test.
   - **Change:** set the table limit to L <= 0.24, so one number rules.
   - Also note that unit tests measure the sRGB constant, but pixels are measured after lighting and Reinhard. The e2e tests 7 and 8 are the real gate. Say that in one line so the lane does not stop at the unit test.

3. **Mist near the camera can cover the screen.** Mist spawns "at random points along its current length", size 0.25-0.4 m, and `P::Frost` is pale, unlit and grows. The stream starts at the hand, 0.5 m in front of the eye. A 0.4 m blob at 0.5-1 m depth is 250-500 px wide at 720p. At 6-12 blobs that is a white flash in the child's face and a view blocker, which GS-6 and GS-17 care about. Test 10 only runs the flashing guard for Frost, and only at the lit[0] pose.
   - **Change:** emit mist only at points at least 2 m from the eye (and, for example, in the far 70% of the current length). State the rule in the spec so the lane does not choose.
   - Extend test 10's guard to CO2 as well as Frost, and take the 20 frames at both a near target (under 6 m, where the stream is shortest and the mist is closest) and the far pose.
   - Add a coverage check to test 10: the fraction of the frame that is near-white mist stays under 15%.

4. **The stream is large at the hand, and the spec only half admits it.** A 0.2 m stream at 0.5 m depth is about 256 px wide, and with the outline at 1.35x about 345 px. The "4-5% of the frame" in the GS walk ignores the outline: it is about 6%. That is within GS-6's 20%, but it is the biggest object on screen during every cast and sits over the lower right.
   - **Change:** fix the number in the GS walk (about 6% with the outline). Decide whether the stream should start 0.3-0.5 m ahead of the hand to cut the near blob. If so, test 7 radii need no change (they start at r = 20 and go to 110 px, far from the hand). If not, record that this was considered.

5. **GS-19 is over-claimed.** The GS walk says the three streams "differ in shape as well as colour (a solid rod vs a rod in cold fog)". Frost and CO2 have the same shape, the same mist (6-12 `P::Frost`) and near-identical white cores. They differ by colour only, and only slightly (white vs pale cyan).
   - **Change:** either give Frost and CO2 different shapes (for example Frost with the higher mist count and a thinner core, CO2 with a wider core and a slower, sinking mist), or change the GS walk to say that spells are told apart by the hotbar icon, formula and sound, with the stream colour as a secondary cue. Do not claim a shape difference that is not built.

## Should-fix

6. **Test 9 (slow click) is timing-fragile.** It polls every 20 ms and needs `mouse.up` between `gameTime` g0+0.35 and g0+0.47, a 0.12 s window. `gameTime` advances in steps of up to 0.25 s in headless software rendering, so the window can be missed repeatedly, and 3 retries will not fix a systematically slow frame rate.
   - **Change:** state the real pass condition, which is that no sampled frame saw `held >= 0.5`. Accept any press whose logged `gameTime` span is below `0.5 - max_dt`. Use the unit test 2 driver for the exact boundaries and let the e2e only prove the live wiring.

7. **Occlusion is a bigger risk than the spec says.** "Out of scope: a fire behind the greenhouse whose bearing is on screen counts as in view." The greenhouse is the main building. A child facing it with the last fire behind it gets no arrow, and the idle hint only appears after 45 s. That recreates the "no guidance" case this item removes.
   - `world::raycast` already exists (used by `pick`), so a line-of-sight check from the eye to each fire is a few lines and at most a handful of fires per frame.
   - **Change:** prefer adding it (`fire_in_view` takes a `visible` flag per fire). If it stays out of scope, state in the spec a named follow-up item and add a test that records the behaviour, so the panel brief can warn about it.

8. **The native `just_pressed` click (item 4, last bullet) is untested, wasm-only product.** The shipped game is the browser build. Native is dev only.
   - **Change:** cut it, or keep it as one line with "untested, native dev only" and no review time. It currently reads like a requirement.

9. **Science rows record tuned constants.** "The `value` is the sRGB triple as committed" has no lint that checks it matches `cues.rs`, so the row goes stale the first time the lane or a later item tunes a colour.
   - **Change:** the row's value is the claim and the limit, for example "CO2 discharge shown as white (all channels >= 0.85)", the same words as the unit-test bounds. Sources stay as stated (fetched, URL and date, stop if unsourced).
   - The verdicts are truthful as written: Water `simplified` (a real jet is clear or white; water is blue only in thick layers), CO2 and Frost `correct` only if the fetched source supports a white cloud made of dry-ice particles and condensed vapour (CO2) or condensed water vapour (liquid nitrogen fog).

10. **Hint table (test 11) is under-specified.** Rows list only some of `pickHint`'s inputs. Say that unlisted fields default to `playing:true, idle:false, note:'', locked:false, extinguished:0, casts:0`. As written, "`locked:true, extinguished:1` gives `''`" would depend on a `note` the lane has to guess.

11. **Frost timing changes slightly.** The old beam lived 0.25 s. The new Frost stream lives `dur + 0.2` = 0.23 s (min flight 0.03 s). That is fine, but unit tests do not pin it. Add one line saying Frost's stream life is by the same rule as the others, and that the golden stream checks use `streams >= 1` at the first impact frame (true on this build, because `projectiles` handles the stream before the impact on the same frame).

## Can the tests fail on a faint stream?

- **Yes for tests 7 and 8**, with the cross-section runs of at least 0.6 r px against a before-shot, measured at ten radii and with the contrast-or-distance rule. A thin pale cone at the far end would fail both. The spec also commits to running them on a0e2ecb first and stopping if Water passes. Good.
- **No for the old I-019 check**, which the spec correctly keeps but notes is not enough.
- Weak spot: test 7 passes if the dark outline alone supplies the run. That is acceptable, because the outline is part of the intended look. But the unit tests do not check the outline against the ground (test 8 does). Add "outline vs a dark-green ground sample (for example `[0.2, 0.4, 0.2]`) >= 2.0" to unit test 1, or accept that test 8 is the only guard and say so.
- Test 8's "skip noisy cross-sections, need >= 6 not skipped, >= 80% pass" is the most complex rule in the spec. Keep it, but have the lane log how many were skipped, so a pose that skips almost everything fails rather than passes quietly.

## Lanes buildable from the contract alone?

- Mostly yes. The contract stub, key order, the two golden edits, the lane ownership and the seam checks are explicit. The rust lane can write `cues.rs` unit tests first. The web lane can write `pickHint` and test 11 without the rust lane.
- Blockers to buildability: items 1 and 3 above (taper, mist placement). After those two changes, a lane can build without asking.
- `Stream` gains `mist_acc`; the `projectiles` stream loop already has `dt`. The outline child needs no new query: if `despawn` does not take children in this Bevy version (the spec already tells the lane to check), use `despawn_recursive` or whichever is current. Fine.
- Stub risk: with `held_secs` returning 0 and `hold_repeats` false in the stub, the stub's own build breaks the existing "holding the button keeps casting" check only if the stub wires it. The spec forbids that. Good.

## Science truthfulness

- Frost as a liquid-nitrogen jet with white fog of condensed water vapour: correct. CO2 as white dry-ice snow plus condensed vapour: correct. Water drawn clearly blue: `simplified` and said so. No player-facing text changes. The "CO2 is denser than air, so mist sinks" remark is only a design note, and the spec says it makes no rate claim; keep it that way (do not let a lane add a caption).
- The Braun and Smirnov citation for water colour is a reasonable starting point; the lane must fetch it, as the spec requires.

## Overlap with I-022 and I-024

- **I-022:** no shared lines. I-022's `main.rs` hunks are `Settings` (91-113, 402-412) and `aim_and_cast` 952, one line after 980, 983 and 987. I-025's block goes after 1017 and only reads `fov_y` and `aspect`. After I-022, `fov_y` is `vfov_for(hfov, aspect)`, so `fireInView` uses the true horizontal width without a change. I-022 says it does not edit `frame()`'s hint block (659-671), and I-025 edits only the `h` computation and `findOn` (665-673), while I-022 edits `setHint` (441-444); no overlap. Golden: I-022's `reviewFollowups` is after `castingYouCanSee` and called after line 343; I-025's function is before it and called after 341. Neither adds a state key (I-022 leaves the key list alone).
  - One thing to confirm at merge: the `hintAllowed` gate sits downstream of `pickHint`, so a hint chosen by `pickHint` is still hidden when Hints is off. The spec says so. Add one test row (test 11, live): with Hints off, `pickHint` still returns the text, and `#hint` stays empty. That pins the seam.
- **I-024:** only the `world::build` call at `main.rs:464` and `world.rs`; I-025 does not touch either. Kit meshes are only scaled via `Transform` (Part R rule kept). I-024 test 12 relies on I-019's stream pixel checks; I-025 keeps them and adds stricter ones, so a later I-024 run covers more. Fine.
- I-020 and I-021 were not checked hunk by hunk (the spec says so). Acceptable, since they are blocked or on older bases.

## Required changes, in order

1. Remove `head_frac` and the taper; constant-width `kit.cyl`.
2. Water core luminance limit <= 0.24 in the table.
3. Mist spawn rule (>= 2 m from the eye) and a near-target, CO2 and coverage addition to test 10.
4. Correct the screen-area figure; decide on starting the stream ahead of the hand.
5. Fix or soften the GS-19 claim.
6. Reword test 9's pass condition around `held` and `max_dt`.
7. Occlusion: add the raycast check or name a follow-up item with a recorded-behaviour test.
8. Cut or demote the native `just_pressed` bullet.
9. Science rows record the claim and limit, not the tuned triple.
10. Default inputs for the hint table; add the Hints-off seam row.
11. Pin Frost stream life in one line.

## Planner response

Spec revised to Revision 2.

1. Accept. `head_frac` and the taper are gone; the stream is a constant-width `kit.cyl` of `thick` (table, struct, stub, unit test 1, lanes).
2. Accept, and went further: Water core L is 0.175-0.24, and every outline is L <= 0.025. The old outline limit (<= 0.05) needed core L >= 0.25 for the 3.0 outline-vs-core test, so it contradicted the 0.24 cap too. Added the line that e2e 7 and 8 are the real gate.
3. Accept. Mist spawns only on the far 70 % of the current length and at least 2 m from the eye. Test 10 now covers Frost and CO2 at a far and a near (under 6 m) pose, with a near-white coverage limit under 15 %.
4. Accept the number (about 6 % with the outline, in the GS-6 row). Reject the lead-in gap and record why: I-019's first-frame blue check and GS-12 depend on the near end of a short stream.
5. Accept the soft option. GS-19 now says the hotbar icon, formula and sound tell spells apart, colour is secondary, and Frost and CO2 look alike. No shape difference is claimed.
6. Accept. Test 9 is valid when the press's span is below `0.5 - max_dt`, so no frame saw `held >= 0.5`. Unit test 2 owns the exact boundaries.
7. Accept the alternative, not the raycast. `fire_off` is bearing-only at a0e2ecb, so a fire behind the greenhouse already hides the arrow there. Fixing that needs the frozen finder lines and an on-screen arrow. Named follow-up "occlusion-aware finder"; new test 6(d) records the behaviour from `tp 0 -30` for the panel brief.
8. Accept. Native `just_pressed` is one optional line: untested, native dev only, not a requirement.
9. Accept. The `value` cell holds the claim and its limit in unit-test words, not the triple. The verdict conditions now say exactly what each source has to support.
10. Accept. Test 11 states the default fields. Hints-off seam: `pickHint` takes no Hints input. Before I-022, the Hints setting does not gate `AIM_HINT`, so the live "#hint stays empty" check goes to the seam breaker of whichever of I-022 and I-025 merges second.
11. Accept. Frost's life follows the shared rule (about 0.23 s versus the beam's 0.25 s, noted under unit test 1). The seam breaker checks `streams >= 1` on the first Frost impact frame.
- Outline vs ground in unit test 1: accept. Outline vs `[0.2,0.4,0.2]` >= 2.0 is added, which is possible because of the 0.025 outline limit. Test 8 is still the real ground guard.
- Test 8 skipped-count logging: accept.
