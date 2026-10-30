# I-015 orchestrator decision (2026-10-30, wave 6)

**Parked under the spec's own fallback rule; Parts A, C, D and step B2 continue as I-023, step B1 becomes I-024.**

## Test 11 on the quietest box available

Interleaved `ONLY=low@4x RUNS=1`, 3 rounds, base a3e50b1 against loop/I-015 cd27edc (B1+B2), run under the Chromium lock with no other suite running (others' load1 0.9 to 1.7; gate passed on every run). Raw lines: `evidence/round3-quiet-low4x.summary`.

| | r1 | r2 | r3 | median CPU p50 | median low@4x TTI |
|---|---|---|---|---|---|
| base | 40.4 | 43.3 | 48.2 | **43.3 ms** | 8 716 ms |
| B1+B2 | 30.8 | 34.5 | 39.3 | **34.5 ms** | 10 300 ms |

- Relative cut: **-20.3%** (per round -23.8%, -20.3%, -18.5%). Round 2 gave -21.6%. Only round 1 (-32%, B1 alone) cleared 25%. Test 11 requires at least 25%, so it fails.
- The candidate is also about 1.6 s slower to first interactive frame at low@4x, beyond the spec's keep rule (base + 500 ms) for this configuration. The likely cause is B1's larger wasm (brotli 3.56 to 4.60 MB).

Spec, "Smallest version" B: "If the relative cut also fails, the item is parked." So I-015 is parked as specced.

## What carries on

- **I-023** (new, from I-015 unchanged): Part A (game clock keeps up with the wall clock, dt clamp 0.25 s with player substeps, `gameTime`/`dt` in state), Part C (golden "clock and budget" checks), Part D (perf tool hardening: frame-aware timeouts with a 10x cap, load gate, re-runs, p99, GL count) and step B2 (no FXAA under `?low`). These are correctness and measurement fixes; the spec says they "should not wait on the last milliseconds". Branch loop/I-023 is loop/I-015 with step B1 reverted. Break-1 and break-2 reviewed this code; the only change is the revert.
- **I-024** (proposed): the speed-versus-download trade of opt-level 3 (CPU -20% at low@4x, TTI +1.4 to 1.6 s, brotli +1.04 MB), plus the next CPU cut from `lanes/client/round-2.md` (texture and sampler rebinding dominate GL calls; B3 static merging bought nothing).
- **S1 (cooldown carry)**, decided by the arbiter rule: keep it. Held fire averages exactly the specced 0.3 s at any frame rate; base's 0.308 s at 60 fps was frame jitter, and steady timing across slow and fast machines is the point of Part A.
