# I-015 builder, round 2 (all lanes; answers break-1)

Branch `loop/I-015`. It had never been pushed: origin had no such ref, which is why break-1 could
not fetch it. This round fast-forwards the breaker's commit fa04f84 (`break-1.md`) onto it and
pushes it. The base is origin/main a3e50b1.

Box: 4 CPUs and 16 GB, Chromium with the SwiftShader software GPU. The box was **not quiet**:
other sessions' builds and golden runs kept the load1 of other processes between 1.3 and 2.0, just
under the gate (2.0). Every run below is valid (throttle verified, load gate passed, workload
complete), but the spread between runs is ±25%. Every perf suite ran under the machine-wide
Chromium lock.

## B1 (blocking): CPU bar not met

**Step B3 was attempted, measured and reverted under the spec's own rule. B2's cloud half stays
failing.**

### Round 2 interleaved pairs, `ONLY=low@4x RUNS=1`, 3 rounds (base, B1+B2, B1+B2+B3)

| build | r1 | r2 | r3 | median CPU p50 (ms) | p99 r1/r2/r3 | frame interval p50 | low@4x TTI (ms) |
|---|---|---|---|---|---|---|---|
| base a3e50b1 | 30.4 | 44.8 | 39.9 | **39.9** | 65.1 / 86.3 / 69.3 | 567-700 | 7392 / 8534 / 9327 |
| B1+B2 (HEAD) | 36.4 | 31.3 | 29.5 | **31.3** | 64.2 / 47.7 / 54.0 | 450-533 | 8901 / 9360 / 9360 |
| B1+B2+B3 | 30.4 | 33.2 | 31.9 | **31.9** | 51.9 / 60.7 / 72.5 | 533-567 | 8711 / 9407 / 9202 |

Raw lines, with CIs, in-wasm CPUFRAME and load: `../../evidence/round2-low4x.summary`.
Requested brotli is 3.557 MB on base, 4.600 MB on HEAD and 4.605 MB with B3.

- **Bar (≤ 16 ms):** not met. The B1+B2 median is 31.3 ms here, against break-1's 34.4 and round
  1's 29.2 (B1 only). All three readings come from one build family on a loaded box.
- **Relative cut (test 11, ≥ 25% required):** 39.9 → 31.3 is **-21.6%** in round 2, against
  round 1's 43.2 → 29.2 (-32%). The two rounds disagree about which side of 25% the cut falls on.
  The base's own spread (30.4 to 47.7 over six runs) is larger than the effect, so the box cannot
  settle test 11. Test 11 needs the quiet box the spec reserves for the tester; that is the
  orchestrator's call.

### Step B3: what was tried and what it bought

The planner's spec assumed about 2 045 GL calls and many draws per frame. Measured with D5
(`PERF_GL_COUNT=1 ONLY=low@1x`; `../../evidence/round2-gl-calls-low1x.txt`), Bevy already batches
the scene:

| build | GL calls / frame | draws / frame | top calls |
|---|---|---|---|
| B1+B2 | 1 894 | 39.8 | texParameteri 440, bindSampler 237, activeTexture 221, bindTexture 221, disable 131, bindBuffer 122 |
| B1+B2+B3 (static merge) | 1 939 | 37.9 | texParameteri 434, bindSampler 234, activeTexture 218, bindTexture 218, bindBuffer 140, disable 129 |

The cost is per draw, not per entity: about 48 GL calls per draw, most of them texture and sampler
rebinding (texParameteri runs twice per bound texture, for wgpu's mip-level clamp) that the GLES
backend repeats for each bind group. The spec rules out render-graph and shader changes, so this
item cannot remove that rebinding.

What B3 did (patch kept as `b3-static-merge.patch`, not on the branch):
- It baked the 133 static, opaque scenery pieces into 17 meshes, one per material, at startup
  (`STATIC_MERGE pieces=133 meshes=17`). Normals went through `Mesh::transformed_by`, which applies
  the rotation and the inverse scale. Colliders were unchanged. Targets, flames, foam, the ice wall,
  puddles, glass and the lab bench props kept their own entities.
- It shared the five state materials that look the same on every burnable kind (hot, burning,
  charred, frosty, foamed) between bush, hedge and greenhouse.
- The picture was unchanged. Screenshots of 5 views at `?low` and at full tier, B2 against B3,
  had a mean pixel difference of 0.0-0.17 except one view. That view differed only in a HUD caption
  that was mid-fade, not in the scenery.
- STARTUP to FIRST_FRAME, one load each: B2 2.70 s at `?low` and 3.59 s at full tier, B3 2.70 s and 4.27 s. That is one sample per tier, so it is within the noise.
- Result: draws fell from 39.8 to 37.9 (-5%), GL calls rose 2% (more vertex-attribute setup per
  merged mesh), and the median CPU p50 went from 31.3 to 31.9 ms. That misses the 30% GL goal and
  the 0.5 ms keep threshold, so **B3 was reverted** ("a step that does not lower the median by at
  least 0.5 ms ... is reverted and noted").
- I did not take the named profile (optional evidence). The planner's own profiles from the
  nightly artifact are in the session scratchpad (`i015/steady-4.cpuprofile`). The next cut should
  start from the render side's per-draw bind cost or from schedule overhead, not from draw count.

Under the spec's fallback rule, B2's cloud half is recorded as **still failing** after B1-B3. The
item can merge only if test 11's ≥ 25% relative cut holds on the quiet box; otherwise it parks.
Parts A, C and D (clock, TTI guard, valid perf runs) do not depend on that number.

### default@4x TTI guard (B1 keep rule; round 1 never measured it)

Interleaved, `ONLY=default@4x RUNS=1`, 3 rounds (`../../evidence/round2-default4x.summary`):

| build | TTI r1 / r2 / r3 (ms) | median TTI | CPU p50 r1 / r2 / r3 | median CPU p50 |
|---|---|---|---|---|
| base a3e50b1 | 11 227 / 11 744 / 10 488 | **11 227** | 65.7 / 63.0 / 58.9 | 63.0 |
| B1+B2 (HEAD) | 12 660 / 11 512 / 10 898 | **11 512** | 50.8 / 46.8 / 34.7 | 46.8 (-26%) |

- **Keep rule (TTI ≤ base + 500 ms): holds.** The candidate is +285 ms, so opt-level 3 stays.
- Test 12's absolute bar (≤ 8 000 ms in each run) fails here **for the base too**: 10.5-11.7 s,
  against the planner's quiet-box 6.6 s. This is the same load effect the spec describes for
  nightly 1, so it is a reading for the tester's quiet-box run, not a verdict.

## B2 (blocking): round-1.md missing

`round-1.md` now exists beside this file. It was written this round from round 1's saved perf
JSON and build logs, and says so. The raw lines are in `../../evidence/round1-b1-low4x.summary`
and `round1-opt-sizes.txt`. They back the "43.2 -> 29.2 ms" (medians of 3 interleaved runs:
base 47.7/43.2/32.8, opt 3 35.1/29.2/26.9) and the "3.56 -> 4.60 MB" (perf.mjs's
`size.requested_brotli_mb`, node:zlib quality 11; the `brotli` CLI is not installed on this box).
break-1's 7.84 MB is the build script's `gzip -9` of the wasm alone, which is a different measure.
Round 2 re-measured requested brotli at 3.557 → 4.600 MB. The Cargo.toml comment now cites
round-1.md and round-2.md and states the numbers. spec.md and critique.md were untracked in the
builder worktree, and the branch now carries them.

## B3 (blocking): process

`loop/I-015` is pushed with fa04f84 (break-1.md) fast-forwarded in, so the breaker's commit is on
the branch.

## Should-fix

- **S1 (cooldown carry at 60 fps).** Kept, and flagged for the planner. In f32 the old reset with
  an exact 1/60 s dt already gives 18 frames (0.3 - 18/60 = -3.7e-9). The "18 or 19" comes only from
  rAF jitter: with frames of 16.6-16.8 ms the old code sometimes needs a 19th frame, which averages
  about 0.308 s per cast (3.25 casts/s). The carry makes the long-run interval exactly the nominal
  0.3 s (3.33 casts/s, about 3% faster held fire at 60 fps). The cooldown length is unchanged. The
  spec's stale check (spec.md line 3) already records this behaviour ("exactly 18 frames, never
  19"). **Question for the planner (spec owner):** is matching the nominal 0.3 s at 60 fps
  acceptable? If not, the carry can be limited to frames longer than `clock::MAX_STEP`, which makes
  60 fps identical to base.
- **S2 (golden "marker clears within 1 s").** I verified it on the base. Golden `QUERY=?low` on
  the base artifact (a3e50b1, its own golden.mjs): 67 PASS, 0 FAIL, marker 603 ms. On HEAD: **75
  PASS, 0 FAIL, exit 0**, marker 491 ms. The 75 are the base's 67 plus 8 "clock:" checks. `WASD
  moves` 2.33 m and `jump` y 0.9 pass. The check is a 700 ms `setTimeout` in hud.js that runs late
  when the software GPU stalls the main thread. break-1's 1158 ms was the environment, not this
  diff. No test was changed.
- **S3 (perf step with no absolute cap).** `stepTimedOut` now also gives up at 10x the step's
  wall-clock timeout (60 s for a spell step, 40 s for a motion step) whatever the frame count, so a
  frozen page cannot hold a step. The existing self-test cases are unchanged. The new case covers
  59 999 → false and 60 000 → true with 0 frames. `perf.mjs --self-test`: 46 passed, 0 failed (was 45).

## Checks run this round

- `rustc --test` clock.rs 3/3 and motion.rs 8/8. `cargo fmt --check` clean. `cargo clippy --target
  wasm32-unknown-unknown --locked -- -D warnings` clean (with B3; after the revert world.rs
  matches the reviewed code).
- Release build (`build-web.sh`): wasm 34 743 887 B; requested brotli 4.600 MB (guard 8 MB).
- Golden `QUERY=?low`: HEAD 75/75 PASS; base 67/67 PASS (S2).
- `perf.mjs --self-test` 46/46.
