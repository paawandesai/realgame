# I-046 spec critique (fresh context)

**Verdict: REVISE.** Right problem, right fix shape. One internal contradiction, one weak test, one missing main-case check, and one merge-order claim that is too optimistic. None of them needs a redesign.

Read: target.md, bars.md, areas.md, spec.md, and both cited screenshots. The screenshots confirm the problem. At 21:9 and 4:3 an opaque navy-outlined slab fills the lower right from the crosshair to the screen edge and sits under the "Aim at a fire" hint. A 7-year-old would lose anything in that quadrant while casting.

## 1. Right, smallest fix?

Yes, with one caveat.
- It stays first-person, so the hard rule in target.md is not touched.
- It reduces the slab without a gap, which I-025 rejected. A taper from a finger-width at the hand to full width at 0.8 m is the smallest shape that satisfies I-025 test 7 and GS-12 together.
- The spec's own model shows thinning alone is not enough (the near end stays 0.41 f wide) and the taper alone is not enough (21:9 still hides 40% of a fire). Using both is justified.
- The rejected options are rejected for stated reasons. A per-cast mesh churns assets (B2). A single frustum over the whole stream fails test 7.
- The cost is a two-part stream (root, lead, rod and outlines) plus one shared mesh. That is acceptable, and the entity budget is stated and checked.

The caveat is that the most common kid case is not tested. With Aim help on, the cast bends onto the fire, so the fire sits at the stream's far end, near the crosshair. G1 deliberately turns Aim help off and tests fire right of the crosshair. That is the right worst case, but nothing proves the targeted fire at the crosshair stays visible with Aim help on, which is the default. The taper helps there too, but the mist and impact flash can hide it, and G4's mist < 15% check does not say the fire is visible.

## 2. Can each acceptance test fail?

- **U1 contradicts the decision (must fix).** Fire has no outline, and the decision sets Fire's `thick` to 0.18. U1 requires `thick·max(outline_scale,1)` in [0.1875, 0.222] for all 4 spells. That gives Fire 0.18·1 = 0.18, which is below the lower bound. Either Fire's `thick` must be at least 0.1875 (for example 0.19), or the lower bound must not apply to Fire. The check on test 7 headroom does not apply to Fire at all, because Fire is skipped in test 7. Pick one and state it. Also note the rationale for the upper bound is looser than the bound: 10° below horizontal allows T of about 0.255, not 0.222.
- **U2 cannot fail on the stub.** On the stub (`lead 0`, `lead_min 1`), `stream_width(0) == thick·lead_min > 0` holds, non-decreasing holds, and `== thick` for s >= lead holds. U3 is likewise satisfiable by the stub. Only U1 and U4 are red. Add `stream_width(0) <= 0.05 m` (finger-wide), or `width(0) <= 0.25·thick`. Also add that `stream_width` at `lead/2` is strictly between the ends.
- **Hard-coded "4 spells".** I-026 adds spells for B8 (6 or more). U1 and U3 should iterate over every spell variant or style, so a new spell landing later fails the test rather than slipping past it.
- **G1** can fail (base 0.55 at P1, about 0 at P2). The validity rule (at least 150 warm pixels before) and the 3-attempts, one-pass rule are acceptable, because a real failure fails all 3 attempts. Add **P0**: Aim help **on**, the fire at the crosshair, after/before >= 0.80 in the frozen frame. This is the default player case.
- **G2** can fail (base 10-14% against 6%, and so on). Note the cross-section range r = 0.25f to 0.45f never samples the widest point of the taper (s = 0.8 m is about r = 0.19 f). The check passes with margin, but its stated "fix 0.17 f" is not what it measures. Say so, or extend the range to 0.15 f.
- **G3** passes on base by design. It is a guard against gap designs, not a failing-first test. That is acceptable, since the spec says so. State that the failing-first requirement covers G1 P2 and G2 only.
- **G5** can fail in principle but is ambiguous. "Within 20 frames **or** before gameTime >= a.gameTime + 2.0" reads as either window passes. Define it as: pass if `burning === false` by `gameTime >= a.gameTime + 2.0`; the frame count and the 15 s wall cap are only loop guards. There is also no failing-first for G5. Require a mutation check (for example, aim off the fire and see the check fail).
- G5 is not about slab visibility. It is break-1 Should-fix 2 bundled in because it touches the same golden function. That is tolerable as a chore, but label it a chore so a G5 problem does not block the stream change.

## 3. Lane boundary against I-026 and I-044

Mostly clean, with three things to tighten.
- **I-026.** There is no hunk overlap with the one `mod spells;` line, `chem.rs`, content or sim. The risk is semantic. I-026 adds or changes spells, and each needs a stream style. State whether `stream_style` and `StreamStyle` keys are touched by I-026 (the spec asserts no, but cites no grep). The rust lane's first step should be to grep callers and confirm. The "all spells" tests above cover the semantic case.
- **I-044.** The hunks are disjoint (golden sits inside `reviewFollowups`, and I-046's call line is outside it). But "either order" is too optimistic, because I-044 moves the gauntlet while the stream origin (`main.rs:1204`) stays fixed. If I-044 lands first, I-046's own G3 can fail through no fault of its own, and the lane is told to stop. Set the order explicitly: **I-046 merges before I-044**, and I-044 owns re-running `closeStreams` and any origin-follows-gem fix. Otherwise a B4-relevant item blocks on another item's side effect.
- **areas.md gap.** `cues.rs` (and `feel.rs`, appended by I-044) appear in no area in areas.md. The spec assigns `cues.rs` to "client-core, client-fx" anyway. Either add `cues.rs` to client-fx in areas.md, or say in the spec that this item is the explicit owner of those hunks. Without that, two lanes could both think they may edit it. Also, areas.md says tests-e2e is tester-owned, but the web lane builds the golden tests. Confirm the web lane is the tester here (it is natural, but say so).

## Required changes

1. Fix U1 versus Fire. Either raise Fire's `thick` to at least 0.19, or exempt Fire from the lower bound and say why.
2. Strengthen U2 with `stream_width(0) <= 0.25·thick` (or at most 0.05 m) and a strict midpoint check. Make U1 and U3 iterate over every spell, not "4".
3. Add G1 P0 (Aim help on, fire at the crosshair, after/before >= 0.80, at both aspects).
4. Clarify G5 pass logic (game-time 2.0 s is the pass line) and add a mutation failing-first. Mark G5 as a chore that does not gate the stream change.
5. State merge order: I-046 before I-044. Replace "either order" for I-044, and say I-044 owns the G3 rerun.
6. Resolve cues.rs ownership in areas.md, or declare it in the spec.
7. Minor: G2's cross-section range should include r = 0.15f to 0.20f, or the claim "fix 0.17 f" should be dropped. Correct the 10° rationale for U1's upper bound.

Approve after 1 to 6. Item 7 is optional.

## Round 3

**Verdict: REVISE, applied by the critic (two minimal edits, marked "(critic)" in the spec). With them in place the amendments are approved; no further planner round is needed.**

Read: target.md, bars.md, spec.md "## Round 3 amendments" (against the G1 P0, G5 and G6 text it replaces).

### A1. G1 P0, freeze on the last frame before impact
- **Can it fail on e70df08?** Yes. Before impact there is no flash or mist and the fire is still burning, so only the stream (and the ball) sits between the camera and the fire. The old rule froze on the impact frame, so it measured whether the fire was out (0.00 on both builds). The new freeze measures the right thing. Whether base actually goes below 0.80 is uncertain. At 18 m the full-width rod is only about 13 px across (f = 840, 0.286 m) in a 72 px square, so base may land at 0.8-0.9, not the 0.65-0.75 the amendment predicts. That is a prediction risk, not a soundness problem.
- **Loosened?** No. The square, the 0.80 threshold, the before rule, the 150-warm validity and the aspects are unchanged, and the validity rule (fire still burning, `aimObj` is the centred fire, next frame is the impact) stops a frozen frame where the fire is already out from counting.
- **Gap found (fixed, critic edit 1).** "Next observed frame has impacts + 1" does not prove the two frames are adjacent. If the rAF watcher misses a game step, the after shot is an earlier frame where the head is farther from the fire and hides less of it. That makes the check easier on both builds. Added a validity condition: the after frame and the impact frame differ by at most one game step in `gameTime`.
- **Process fix (critic edit 2).** If P0 passes on base, the old text stopped the lane only so the planner could turn P0 into a guard. Now the lane records the numbers, P0 becomes a guard that must still pass on the branch, and the lane continues. This does not loosen the pass line, the square or the freeze rule, and it saves a round on the likely outcome above. Failing-first for this item stays carried by G1 P2, G2 and G6 (a)/(b).

### A2. G6 (a), measured along the stream's own line
- **Can it fail on e70df08 for the wrong-origin defect?** Yes. The base stream runs from the unturned hand to the bottom edge, so in-band pixels go well past `Hs'` (t_max − L = +339 px against ±25.2). A stream drawn from a partly turned hand also fails: about 1.2° of tilt error is enough to leave the ±25 px window at f = 1199. A stream that stops short of the hand fails the lower bound, so (a) keeps the one job (b) and (c) cannot do.
- **Loosened?** Only across the line, and only within the band's existing ±0.03·VH. The old Euclidean rule let the band's half-width leak into where the stream ends, so a correct 78 px wide end-cap failed it. The along-axis tolerance stays at 0.035·VH. Being off sideways is still bounded by the band, and the attachment check (c) still applies. I accept it as a correct re-derivation, not a weakening. (b) and (c) are untouched. It relies on the band extending past `Hs'` toward the screen edge, which (b) already requires and which the measured base +339 px confirms.

### A3. G5, a press with no move and the aim checked at the cast frame
- **Deterministic?** Yes, as far as the test can control. `down`/`up` removes the `mouseMoved` that caused the saturated ±200 px deltas. The view must hold still for 5 frames before and after the aim, plus `cooldown === 0`. The aim and view must be within 0.5° at the cast frame, and there are 3 attempts. If all 3 are invalid, both checks FAIL; nothing is ever skipped or passed. If the view still moves, step 8 stops the lane and does not widen 0.5°. That is sound.
- **Loosened?** No. The count half is unchanged (exactly once over 20 + 20 frames), is judged on every attempt and is never retried. The extinguish half keeps the same target `lit[1]` and the same game-time line of 2.0 s, and the caps still end the wait as a failure. The new mutation (Fire at `lit[1]`) proves the extinguish half can fail with the aim valid. The old aim-away mutation must still FAIL, never pass.
- **Coverage note.** The real-click view jump is a genuine (small) `hud.js` bug that test 9 no longer exercises. It is outside I-046's lanes, and the backlog note includes a golden check for it, so it is tracked rather than hidden. The planner should file that chore this wave.

### Changes made (critic)
1. A1 validity: added the condition that the after frame and the impact frame are at most one game step apart in `gameTime`.
2. A1 failing-first: if P0 passes on base, it becomes a guard that must still pass on the branch, and the lane continues instead of stopping. The threshold, square and freeze rule are unchanged.
