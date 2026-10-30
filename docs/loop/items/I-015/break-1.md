# I-015 break-1 (wave-6 breaker)

Reviewed loop/I-015 @ 5b1a059 (5 commits over origin/main a3e50b1). Note: origin has no
`loop/I-015` ref (fetch failed); review used the local branch. Nothing was pushed.

Verified: all 9 changed files are inside the spec lanes (client src + Cargo.toml profile, golden.mjs,
perf.mjs); no web/**, Cargo.lock, build-web.sh touched. No existing golden/perf assertion removed
(golden diff is additions only; perf removals are the replaced workload/timeout code, with the
self-test growing from the base set). Build OK. clock.rs 3/3 and motion.rs 8/8 unit tests pass
(3b tunnelling proof included). perf --self-test 45/45. Golden `QUERY=?low` (frames 0.6-1.2 s):
all 7 new "clock:" checks PASS, walking 8.75 m in 1.75 s expected game time (5 m/s), no pane tunnelling,
cadence 6 casts, pause freezes gameTime. One FAIL (see S2).

## Blocking
- B1. CPU bar not met and not reproduced: perf `ONLY=low@4x RUNS=3` here gave cpu p50 34.4 ms
  (CI 28.2-40.8, load gate ok, throttle verified), against the commit message's 29.2 ms and the
  spec bar <= 16 ms. In-wasm CPUFRAME log p50 15.1 ms. Spec says B3 (material/mesh diet) runs when the
  median stays above 15 ms after B1-B2; it was not attempted. Either run B3 or record the stop-rule
  decision.
- B2. `docs/loop/items/I-015/lanes/client/round-1.md` does not exist in the branch (nor in the builder
  worktree), yet the spec and Cargo.toml:55-57 comment cite it for the perf numbers and the 3.56 -> 4.60 MB
  brotli claim. Numbers cannot be audited. (brotli is not installed here; build gzip = 7.84 MB, brotli
  claim unverified.)

## Should-fix
- S1. crates/bevy-client/src/main.rs:967-977 (cooldown carry): changes timing at 60 fps too (held fire
  can come up to one frame sooner; "exactly 18 frames" vs the old 18-or-19). Defensible and noted, but it
  is a gameplay timing change outside the stated clock fix; confirm with spec/standards owner.
- S2. golden `feel: the marker clears within 1 s` FAILED (1158 ms) at 0.7-1.2 s frames (tests/e2e/golden.mjs:673).
  It is a wall-clock check on a frame-driven HUD marker (not touched by this diff) so likely
  environmental, but I did not run it on the base; golden exits 1 in the software-GPU env. Verify on base.
- S3. tests/e2e/perf.mjs:81 `stepTimedOut` never fires while frames-since-start < 8, and the in-page
  workload has no absolute cap: a frozen/crashed page hangs the workload step (the TTI deadline only
  covers boot). Add a hard ceiling (e.g. 10 x ms).

## Nits
- N1. main.rs:~250 Time<Virtual> max_delta 0.25 makes the fixed-step chem run up to 7-8 ticks in one slow
  frame (CPU spike in the frame that is already slow); acceptable, but p99 in perf will show it.
- N2. player.rs:2 doc line is long; `Player::step` doc describes `boxes` helper that builds a Vec per
  frame (fine, one alloc per frame instead of per call).
