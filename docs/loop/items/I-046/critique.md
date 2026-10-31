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
