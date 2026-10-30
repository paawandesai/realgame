# Critique of I-015 spec (spec critic, fresh context)

**Verdict: revise.** Right problem, not the smallest version, and a few acceptance tests cannot fail or will flake. Lanes are mostly buildable from the contract. Two cautions: the spec is not buildable until I-007 merges, and I could not check any claim against code, because this checkout has no `crates/bevy-client`. I checked the facts only for internal consistency. `gameplay-standards.md` is absent from `docs/loop/` here (it exists only in `.claude/worktrees/*`). I read GS-5 from there.

## Is it the right thing for the players?
- Part A (the game clock matches the wall clock) is clearly right. Below 20 fps the player and spells slow down while the fire runs at full speed. That hurts the Chromebook kid and the QA persona, and it is why perf runs are invalid. Keep it.
- Part B (the 16 ms CPU bar) helps players only if real players run `?low`. The spec never says whether `?low` is auto-selected on weak hardware or is only a URL flag. If nobody gets it by default, B2's cloud half is a bar-pass, not a player gain. State in one line how a school Chromebook reaches the low path, or note that the bar is a proxy.
- The FXAA-off-under-`?low` change is a visible quality change for children. It is acceptable, but test 19 is the only guard and it is a human look.

## Is it the smallest version?
No. It bundles four things: the clock fix, a CPU diet of five steps, a load guard, and a perf-harness rewrite (D1–D7).
1. **Split step B4.** Mesh merging and material dedup is the largest, riskiest change. It touches `world.rs` and `fx.rs`, needs a colour table and a visual diff, and makes up most of the 30% GL-call target. Make B4 conditional: attempt it only if B1–B3 leave the median above 16 ms. Otherwise file it as its own item. The spec already has a stop rule. Put B4 behind it explicitly, and drop test 18's "reported, not gated" ambiguity.
2. **Cut B5** (allocation-free `publish`). It is about 30 ms of 40 s by the spec's own profile, so it is noise. Move it to an idea row.
3. **Cut D4/D5** (heap and WASM fields in perf, and the GL-call hook) unless B4 stays in. D5 is diagnostic only and never used for a verdict. The memory guard (tests 9 and 10) guards a leak the spec says does not exist. Keep test 9 in golden, which is cheap, and drop the perf copy (test 10) and D4.
4. **Cut B1** as a deliverable. It is evidence for the builder, not part of the product. Say so, or make it optional.

Minimum viable set: A1–A5, B2 (opt-level), B3 (FXAA), C guard, D1 (frame-aware timeout), D2 (load gate), D3 (p99), and tests 1–8, 11–14, 16.

## Can every acceptance test fail?
The e2e header says "each fails on 3dbd332". That is false for several tests.
- **Test 6 (no tunnelling)** passes on base, because the 0.05 clamp prevents tunnelling. It does not use `gameTime`. Relabel it a regression tripwire. The builder's local `MAX_STEP = 0.25` check is the real proof, and it is required, so keep that. Also state the expected failing frame sizes: with 0.5 s frames, 8.5 m/s covers 3.65 m to the pane, so tunnelling needs a step of about 2 m, which is the case. Say that.
- **Tests 9, 10, 14, 15 and 17** pass on base by design. Label them guards, not red-then-green tests.
- **Test 3** is close to a tautology. It tests a helper function, and "the distance equals the 15-call result" restates the substep loop. The risk is in `Player::step`, not in `walk_speed_after`. Replace it with a unit test that calls `Player::step` (or the extracted pure integrator, if it has no Bevy types) for dt 0.25 and for 15 steps of 1/60. It should assert the same end position within 1e-3 against a flat floor, and assert that a 0.1 m wall at 2 m is not crossed. If `player.rs` cannot compile under plain `rustc --test`, say so now. Otherwise the test lives in `cargo test -p`, not in the `rustc` command shown.
- **Test 5 (walk distance)** will flake. At 0.3–1.5 s frames the 3 s hold contains 2–10 frames. Key-down and key-up land mid-frame, and the 60 m/s² ramp is cut by quantisation. The 0.85 lower bound is not defensible with 2 frames. Define the window as the frames between the first sample after key-down and the last sample before key-up. Compute `expectedGame` over exactly those samples. Add a minimum of 4 frames in the window, or extend the hold until 4 frames have passed, or the test is invalid, not failed.
- **Test 4** is sound. Specify that the sampler runs only after the page is idle (no `tp` or `reset` in the window) and that the first sample is dropped.
- **Test 7** relies on a lit target (`lit[0]`) that must exist and burn for 6 s, and the cast cadence also depends on mana and ammo state. Say what resets before it, or assert on `casts` only and drop the `impacts` clause. Impacts also depend on I-019 flight-time behaviour.
- **Test 8** (grep for `min(0.05)`) is brittle. It misses `.min(0.05_f32)` and `f32::min(..., 0.05)`. Use `/min\(\s*0\.05/` plus a check that `clock::frame_dt` is called in each of the five named systems.
- **Test 11** is the one that matters, and it depends on an environment gate. The load gate waits 60 s and then marks the run invalid. On a shared loop machine this can starve the verdict. Say who guarantees a quiet box (the tester owns the gate, the orchestrator pauses builds), and what happens if the gate never passes: the item is `blocked`, not `failed`.
- **Test 12** requires "all 12 runs valid" on a shared box. Allow up to 2 re-runs of invalid runs, each logged, or accept 3 of 4 valid configurations per round. Otherwise the item is unmergeable for reasons unrelated to the code.
- **Test 19** ("looks the same") is a human or model eyeball with no threshold. Add a per-pixel mean abs difference limit for non-FXAA-edge regions, or accept it as breaker-only.

## Risks the spec does not cover
- **Projectile and target hit tests at dt 0.25.** Only the player is substepped. If projectiles move by `speed * dt` and test hits at the end point, a spell can fly through a target at large dt (25% of the way to a far target in one frame). Spell travel is a gameplay path, so check it. Either substep `projectiles` too, or confirm hits are by arrival time. Add one test: cast at a target 15 m away at 0.5 s frames and assert an impact.
- **Pause and tab refocus (GS-7).** Raising `max_delta` to 0.25 means a visible stall of 0.25 s of game time on resume instead of 0.05 s. State that pause freezes `Time<Virtual>` and `gameTime`. Test 4 should assert that `gameTime` does not advance while paused.
- **Jump arc.** With a smaller step the jump arc will differ slightly from base at 20 fps. "Unchanged" is true only at 60 fps and above. Say so, and keep the existing jump test as the check.

## Lane and contract problems
- **Client lane edits `tests/e2e/golden.mjs`** while `areas.md` marks `tests-e2e/**` as tester-owned. The builder then writes the tests that judge its own code. Move the golden section (tests 4–9, 13) to a separate `tests` lane, or to the perf lane, and have the tester own it. The lane table also lets the client lane overwrite `Cargo.toml`. That is fine, because the profile section is named, but do not allow `Cargo.lock` changes.
- **The client lane is one huge lane** across client-core, client-world and client-fx (`src/**`). The spec cannot be parallelised inside it. If B4 stays in, it should be a separate item.
- **The perf lane is buildable from the contract**, since it uses only `state.frame` and `STATS`. State the exact `STATS` regex and the `state.frame` location (it is in the JSON), so the lane does not need to read the client. D5's hook point ("the page hook") should name the file and function in `perf.mjs`.
- **Contract gaps.** `gameTime` and `dt` are consumed only by the golden tests, so they belong to the tests lane. Add the JSON key order (new keys appended at the end) to the contract, since test 13 depends on it.
- **Ordering.** The spec is blocked on I-007 (a `based_on` update and re-check of facts). Mark it `not dispatchable` in the backlog until the merge, and re-read the `min(0.05)` sites then. I-019's test 15 uses `min(0.05, interval)` and must be refreshed, but the spec rightly notes this.

## Specific changes requested
1. Restructure to the minimum set above. Move B4, B5, D4, D5 and test 10 and 18 to an "if the stop rule is not met" or separate item.
2. Fix the "each fails on base" claim: label tests 6, 9, 14 and 17 as guards.
3. Replace test 3 with a `Player::step` test, or state clearly why not.
4. Redefine test 5's window and add a minimum frame count.
5. Add a projectile-at-large-dt test and a paused-`gameTime` assertion.
6. Move golden.mjs ownership to the tester/tests lane.
7. Specify quiet-box ownership and the blocked state for tests 11 and 12, and allow logged re-runs.
8. Tighten the test 8 regex.
9. Say how real players reach `?low`, or state that B2's cloud half is a proxy.

## Planner response

Revised spec.md (revision 2). One line per requested change, then per finding.

1. Restructure to the minimum set: **accepted, with one variation.** B5, D4 and test 10 are cut to "Out of scope" (B5 as an idea row). Old B1 is now optional evidence inside the conditional step. Old B4 is not split into its own item. It stays as a conditional step B3, attempted only if B1–B2 leave the median above 15 ms. The reason: B1–B2 may not reach 16 ms on their own, and a separate item would cost a full extra cycle. D5 and test 18 run only with B3, and test 18 now records a miss rather than gating, so the earlier "reported, not gated" ambiguity is gone. I also added a fallback: if B2 is still above 16 ms after B3, the item can merge with a cut of at least 25%, so the clock fix is not held back. This is a planner decision. The bar itself is unchanged and B2 is recorded as failing.
2. "Each fails on base" claim: **accepted.** Tests are now labelled red or guard: 6, 9, 12, 14, 15 and 17 are guards. Test 7b is unclassified, and test 10 is removed.
3. Test 3: **accepted.** `player.rs` imports Bevy, so it cannot be tested with plain `rustc --test`, and the client crate is not part of CI's `cargo test`. The integrator is moved into a pure `motion.rs`, and `Player::step` wraps it. Test 3 compares one 0.25 s step against 15 steps of 1/60 s. Test 3b checks that a 0.1 m wall is not crossed with substeps and is crossed with n = 1, which proves the test can catch tunnelling.
4. Test 5 window: **accepted.** The window is now the samples between key-down and key-up. It needs at least 4 samples, with the hold extended up to 12 s; if it still has fewer, the run is invalid, not failed. I added 0.3 m of slack for the acceleration ramp.
5. Projectile test and paused `gameTime`: **both accepted, but not projectile substeps.** Projectiles move by `from.lerp(to, t/dur)` toward a target fixed at cast time, so they cannot tunnel. Test 7b now asserts the 15 m impact. `gameTime` becomes a game-owned accumulator that freezes while paused, because the pause overlay does not pause `Time<Virtual>`. Test 4 now asserts no advance while paused.
6. golden.mjs ownership: **accepted.** There is a new tester-owned `tests` lane, written from the Contract alone. The client lane may not touch `tests/**` or `Cargo.lock`.
7. Quiet box and blocked state: **accepted.** The tester owns the load gate and the orchestrator pauses builds. If the gate fails twice, the item is `blocked`, not failed. Up to 2 logged re-runs per configuration are allowed for environment reasons only, never when `workload_ok` is false (D8).
8. Test 8 regex: **accepted.** It now uses `/min\(\s*0\.05/` and an `f32::min(.., 0.05)` pattern, and checks that `clock::frame_dt(` appears in each of the five systems.
9. How players reach `?low`: **accepted.** It is a URL flag only, so I stated that B2's cloud half is a proxy and filed an idea row for auto-selecting `?low`. Part A is the change that helps slow machines on any tier.

Other findings:
- Test 4: idle window and first sample dropped. **Accepted.**
- Test 7: it now does `reset` first and asserts on `casts` only, with `impacts` moved to 7b. **Accepted.**
- Test 19: **accepted as breaker-only.** I did not add a pixel threshold, because removing FXAA changes every edge, so a mean-difference number cannot tell a regression from the intended change.
- Jump arc: added the note that it is unchanged at 60 fps and above (bit-identical at dt ≤ 1/60), and the golden `jump` check is the guard. **Accepted.**
- Tab-refocus stall: added a note that a refocus advances at most 0.25 s of game time per frame, kept safe by the substeps. **Accepted.**
- Perf lane contract: it now names `pageHook` (via `addInitScript`) as the D5 hook and the existing `JSON.parse(window.__gameState).frame` and STATS parsing. **Accepted.**
- JSON key order: the Contract now says new keys are appended at the end, `gameTime` then `dt`. **Accepted.**
- Ordering on I-007: the backlog row is marked not dispatchable, and the `min(0.05)` sites are re-read after the merge. **Accepted.**
- "One huge client lane": **partly accepted.** The golden tests moved out. The client lane stays one lane because B3 is sequenced after B1–B2 under the stop rule and is conditional.
