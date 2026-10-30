# I-024 break-1 (wave 8 breaker, fresh)

Outcome of the item: measured, no keep (P1, P2 not kept; R reverted; Part T lands). Diff vs origin/main: `crates/bevy-client/Cargo.toml` (comment only, +6), `tests/e2e/perf.mjs`, `tests/e2e/perf-pairs.mjs`. All inside the measure and build lanes; no `src/**`, `Cargo.lock`, `build-web.sh` change. origin/main has not touched crates/tests since the merge base c4832d2.

## Blocking
None. No change is kept, so no TTI or brotli regression ships (release profile is still `opt-level = "z"`).

## Should-fix
- crates/bevy-client/Cargo.toml:58-63: the comment says P1 "30.9 vs 32.2 ms" (0.960). My own paired rerun of base vs P1 (5 rounds, low@4x, under the lock; evidence/break1-P1-pairs.json) gave 28.7 vs 28.8 ms (0.997), 3/5 wins, p99 ratio 1.31, TTI diff -145 ms. So P1's "about 4% win" is not reproducible; the comment and build round-1 line 61 ("worth about 4%") should say "inside the noise, not reproduced", so the follow-up does not budget for it.
- Noise: the lane's base-vs-base run gave cpu ratio 1.055 and 2/5 wins; the spec said the P2 9% needed more than 5 pairs. P2 (0.909, 3/5) is plausible but unproven; the follow-up (bevy_ecs + glam only) should use more rounds, not 5.
- docs/loop/items/I-024/lanes/render/round-1.md: the R build logged `target_skipped=0` with `candidates=91 pieces=91`. If no palette piece is referenced by a Target, the post-pass skip path was never exercised (test 9 Wet round-trip hazard). Before anyone revives de4648f, check that `plain(m)` objects really reuse palette handles, or add a test that exercises the skip.
- Same file: R also failed the draws gate (-5.4% vs the 5% limit) in addition to texParameteri 0.773 (> 0.70). The summary calls it marginal; fine, but the recorded outcome should list both failures (it does in the table).
- Golden `?low` on the base-identical artifact (base-pkg-out, since the candidate tree has no code change) gave 96/98 under load avg ~4 (other agents were running Chromium or builds despite the lock; fps 1.5-3). Failures: `clock: cast cadence keeps the 0.3 s cooldown in wall time casts=10 need 11` and `cast: a real click casts exactly once`. Both are wall-time and frame-rate dependent, and the code is unchanged from base, so not attributable to I-024; the lane's 98/98 rerun stands. Not re-run: the box was not quiet.

## Verified
- `perf.mjs --self-test` 50/50 (base N = 46 + 4 new); `perf-pairs.mjs --self-test` 15/15.
- `cargo metadata --offline` parses the changed manifest (comment-only change; no wasm rebuild needed for a comment).
- Evidence files P1/P2/noise-pairs were produced by perf-pairs (verdict objects, per-run lines). The rule-3 limit of 900 ms in P1/P2 comes from the noise spread (892.7 rounds up to 900) as the spec says; the noise summary itself shows 500 because it was run without `--noise`.
- Keep-rule application is honest: P1 fails CPU; P2 fails wins (3/5) and brotli (+0.83 MB vs 0.50); neither would raise TTI or brotli past the limits in the shipped tree.
- Phase fields are appended; existing JSON keys keep order; no thresholds or validity rules changed (`ttiPhases`, `startupLogMs`, anchored regex all match the spec).

## Nits
- tests/e2e/perf.mjs `ttiPhases`: `fetch_ms` is `wasm_response_end_ms` on the navigation clock, not a duration of the fetch alone (it includes HTML and JS time); the caveat comment covers streaming but not this.
- tests/e2e/perf-pairs.mjs: a rerun after "machine busy" reuses the same slot but nothing in the printed line separates a first invalid result from the counted one beyond `rerun`; fine.
