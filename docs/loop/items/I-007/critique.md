# Critique of I-007 spec (spec critic, fresh eyes)

**Verdict: revise**

The item is the right kind of thing. The first-person rule is respected: the work is sensitivity, FOV, crosshair and hit feedback, and no head bob. Each piece serves a real buyer problem: kids, unreachable `?sens=`, unclear cast feedback, motion sickness. It is not wrong-thing. It needs a few fixes before lanes can build from the contract alone, and it is slightly bigger than the smallest version.

## Is it the smallest version?
Mostly, but trim these:
1. **Per-spell recoil (tests 2, state field `recoil`)** is cosmetic and is not tied to any persona complaint. It also drags in `Spell` and a tuning table. Cut it, or keep a single recoil value. If kept, say why it moves B4 or B5.
2. **Three marker kinds.** `out` (win) and `hit` carry the value. `fizzle` ("did nothing") is the useful negative signal for B4 clarity, so keep it. Do not add more kinds.
3. **"No head bob" is a guard, not a build.** Nothing bobs today, so test 9 passes before any code is written. Say so, and do not count it as delivered feel. Keep the test as a regression guard, but state that it cannot fail on the current build.
4. **Missing and cheap:** the crosshair pulse and ring animation should honour `prefers-reduced-motion`. This is a one-line CSS rule and fits the motion-sickness goal. Add it.

## Can the acceptance tests fail?
Tests 1–6 and 12 can fail and are well specified. Problems:
- **Test 7** (`yaw` 334.8) depends on the I-001 look constant (about 0.126 deg per px at sens 1). It is not in the contract. A lane cannot verify the number from this spec. State the constant or write the assertion as a ratio: `turn 100 0` at sens 2 is twice the yaw change of sens 1, within 1%.
- **Test 9** needs the eye height (1.60 m). The contract never states `player.eye()` height. State it, or assert `cam[1] − pos[1]` is constant across samples.
- **Test 10** is timing-based at "≥ 0.28 s" against a 0.30 s cooldown. At a throttled frame rate this flakes in both directions. Either measure in game time (frame count × dt, or `cooldown` from state) or drop the wall-clock bound and assert on `queued` and `cooldown` state.
- **Test 11** never exercises `hit`, only `out` and `fizzle`. Add one `hit` case (for example Water on a non-burning bush, or a Station touch). The "Water then Fire = fizzle" case is a chemistry claim from I-001 that is not cited. Name the exact event the engine returns, or the lane will guess.
- **Marker position is untested end to end.** Add a check that unlocked `#hitmarker` is positioned near the `hitX/hitY` projection.
- **Test 13** (perf, 30 s idle at spawn) does not exercise the new code: no cast, no ring, no marker, no HUD updates. A +0.5 ms median across 3 runs under 4x throttle is also inside the noise. It therefore cannot reliably fail for the right reason. Change it to run the golden-path casts, widen the tolerance or use more runs, and say who builds the base artifact (main) and where it lives.
- Test 14 is fine.

## Are lanes buildable from the contract alone?
Not quite. Fix these:
1. **File ownership gap.** The spec says to modify the `projectiles` system, the impact point and the hit kind. The spec does not say where that system lives. If it is in `world.rs`, `chem.rs` or `fx.rs`, then the `core` lane (main/player/bridge/feel) cannot edit it, and areas.md forbids it. Name the file, or add it to one lane as an explicit exception, or require that all new logic go in `feel.rs`/`bridge.rs`.
2. **`tap` vs `cast` source.** The contract says `cast` bypasses the gate (`Source::Test`). It never says that `tap` goes through `Source::User`. Test 10 requires it. Add the line.
3. **`markerPos(state, rect, ...)`: `rect` is undefined.** State that it is the canvas `getBoundingClientRect()`. Also say what happens for `hitY` of −1, and that `locked` places the marker at the canvas centre. The crosshair is assumed to be centred there.
4. **Who writes `--cd` and the classes `cooling` and `pulse`?** Assigned to the web lane in the table, but derived from state fields (`cooldown`, `cooldownMax`, `casts`). Put in the contract that web computes them from `__gameState` only.
5. **`reset` and `set` ordering at boot.** The HUD sends `set` once, but Rust also reads `?fov=`. Say that Rust reads the URL first and the HUD's effective value then overrides it, with the same precedence rule, so the two cannot disagree.
6. **Merge order.** The `contract` lane merges first and its tests are red until `core` and `web` land. State that the contract lane's e2e is not run until the item is complete, so it does not read as a failure.
7. **`feel.rs` imports `Spell` and `chem::Event`.** Confirm that the type names exist on `claude/fps-prototype`. This is covered by the stale-check, but add a `mod feel;` instruction for `main.rs`, since there is no `lib.rs`.

## Buyer fit
- Settings live only on the pause card. The 6-year-old and the teacher in a lab will rarely find them. Defaults carry the experience, so one line is needed: the defaults (sens 1.00, FOV 70) were tuned by the panel, not guessed. Note this as a follow-up for the first panel.
- The prediction ("median clarity +1 point") cannot be checked until I-006 exists. That is acceptable for a feel pass, but do not report B4 or B5 as moved at merge.

## Required changes (summary)
1. Cut or justify per-spell recoil. Label head bob as a guard only.
2. Add `prefers-reduced-motion`.
3. Fix tests 7, 9, 10, 11 and 13 as above (constants, timing, a `hit` case, a perf test that exercises the new code).
4. Name the file of the `projectiles` system and resolve ownership.
5. Add `tap` → `Source::User`, define `rect`, `--cd` ownership, boot precedence, and the `mod feel;` instruction.
