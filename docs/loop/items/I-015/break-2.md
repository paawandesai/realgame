# I-015 break-2 (wave-6 breaker, round 2)

Reviewed origin/loop/I-015 @ cd27edc (round-2 diff: fa04f84..cd27edc = Cargo.toml comment, perf.mjs
stepTimedOut cap, notes and evidence; no client source change). Fetched with an explicit refspec.

## Verified
- Build: `build-web.sh` release OK, wasm gzip 7 838 627 B (matches break-1's 7.84 MB).
- Unit tests: clock.rs 3/3, motion.rs 8/8. `perf.mjs --self-test` 46/46 (new frozen-page case passes).
- Golden `QUERY=?low` under the Chromium lock: 75/75 PASS, exit 0. The "marker clears within 1 s"
  check (break-1 S2) passed this time, which supports the environmental explanation.
- Round-1 B2 (missing round-1.md): FIXED. round-1.md and evidence/ exist; the medians in the notes
  recompute from the evidence files (round 1 base 47.7/43.2/32.8 -> 43.2, opt 3 35.1/29.2/26.9 -> 29.2;
  round 2 base 30.4/44.8/39.9 -> 39.9, B1+B2 36.4/31.3/29.5 -> 31.3, B3 30.4/33.2/31.9 -> 31.9).
  Cargo.toml comment now cites real files and measured numbers.
- Round-1 B3 (process, branch unpushed): FIXED, ref fetchable.
- S3 (frozen page hangs workload step): FIXED. `stepTimedOut` caps at 10x timeout; the function is
  injected via `${stepTimedOut}` so the new default parameters travel with it. No regressions in the
  existing 8-frame rule.
- Round-1 B1 (B3 step not attempted / stop-rule undocumented): FIXED procedurally. B3 was attempted, measured
  (draws 39.8 -> 37.9, GL calls +2%, CPU 31.3 -> 31.9 ms) and reverted under the spec's 0.5 ms rule; patch kept
  out of the branch as b3-static-merge.patch. Verified the branch source has no B3 code.

## Blocking
None for the code. The following is a decision for the orchestrator, not a defect:
- Test 11 (>= 25% relative cut) is not settled. Round 1 says -32%, round 2 says -21.6% (39.9 -> 31.3),
  and the base's own spread (30.4-47.7) exceeds the effect. The 16 ms bar is not met (about 30 ms). Per
  spec fallback the item merges only if test 11 holds on a quiet box; otherwise it parks. The tester
  must run test 11 on a quiet machine. Do not merge on the current evidence alone.

## Should-fix
- S1 stays open: the cooldown carry changes held-fire timing at 60 fps (~3% faster, 3.33 casts/s vs ~3.25
  with rAF jitter). The builder flagged it and offered to limit the carry to frames longer than
  `clock::MAX_STEP`. The spec owner has not answered; decide before merge.
- test 12 absolute bar (TTI <= 8 s at default@4x) fails on this loaded box for base and candidate alike
  (10.5-12.7 s); only the quiet-box tester run can confirm. Keep rule (<= base + 500 ms) holds (+285 ms).
- Evidence is single-run per cell with +/-25% spread; the "B3 does not help" conclusion (-0.6 ms) is inside
  that noise, so it rests mainly on the draw/GL-call counts, which are deterministic and do show no gain.

## Nits
- round-2.md says "the branch now carries spec.md and critique.md"; fine, but the spec still reads
  "26.8 -> <= 16 ms" while the notes now document ~30 ms; the planner should file the next-cut item.
