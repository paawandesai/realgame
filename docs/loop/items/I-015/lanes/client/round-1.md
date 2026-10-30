# I-015 client lane, round 1 (written in round 2 from round 1's saved data)

Round 1 cited this file from the spec's stale check and from `crates/bevy-client/Cargo.toml`
but never committed it (break-1 B2). The round-2 builder wrote it from round 1's saved perf
JSON and build logs in the session scratchpad. Nothing here was re-measured; the raw lines are
committed beside it in `../../evidence/round1-*.{summary,txt}`. Round 2's own numbers are in
`round-2.md`.

## Findings recorded during the build (cited by spec line 3)

1. **Cooldown quantisation.** At dt 0.25 the old reset (`cooldown = 0.3`, dropping the part of
   the frame past zero) gives one cast per 2 frames under held fire (2 casts/s, not 3.3), so
   golden test 7 cannot pass from the clamp change alone. `aim_and_cast` carries the overshoot of
   the frame in which the cooldown ran out (main.rs, "If the cooldown ran out during this
   frame"). See round-2.md, S1, for what this does at 60 fps.
2. **`motion::step`/`advance` take one more argument than the spec's signature**: `bound`
   (`world::HALF`), so the per-step edge clamp stays in the integrator without `motion.rs`
   importing the Bevy-side `world`.

## Step B1: opt-level (ONLY=low@4x, RUNS=1, 3 interleaved rounds)

Order per round: base (origin/main a3e50b1 at its own `z`), then this branch at `z`, `s`, `2`,
`3`. Fat LTO, one codegen unit, `panic = "abort"` and `strip` unchanged throughout. Every
counted run was valid (throttle verified, load gate passed). The re-runs were for
`machine busy` only.

| build | r1 | r2 | r3 | median CPU p50 (ms) | requested brotli (MB) | wasm raw (B) |
|---|---|---|---|---|---|---|
| base (main, z) | 47.7 | 43.2 | 32.8 | **43.2** | 3.557 | 17 666 772 |
| branch, z | 59.9 | 49.2 | 33.6 | 49.2 | 3.559 | 17 667 734 |
| branch, s | 47.8 | 38.3 | 33.7 | 38.3 | 3.937 | 23 161 541 |
| branch, 2 | 32.2 | 30.5 | 27.8 | 30.5 | 4.533 | 32 608 092 |
| branch, 3 | 35.1 | 29.2 | 26.9 | **29.2** | 4.599 | 34 743 889 |

So 3 was kept (median 43.2 to 29.2 ms, -32% against base). This is where the commit
message's "43.2 -> 29.2 ms" and Cargo.toml's "3.56 -> 4.60 MB" come from. The brotli
figures are perf.mjs's `size.requested_brotli_mb` (node:zlib, quality 11), not the `brotli` CLI.
The CLI is not installed on this box. The build script's `gzip -9` of the wasm alone is 7.84 MB
at opt 3, which is the figure break-1 saw. Clean build times were z 272 s, s 941 s, 2 1259 s and
3 1193 s.

The load was high throughout: the "others" load1 sat between 0.8 and 2.0 on 4 CPUs, close to the
gate's 2.0. The spread is large: base ran from 32.8 to 47.7 across rounds. Round 1 did not take
the default@4x TTI pairs that B1's keep rule asks for, and it did not take the B2 (FXAA-off)
pair. Round 2 took both.
