# I-024 build lane, round 1: Part P (hot-crate opt-level)

**Outcome: no candidate kept.** P1 missed the CPU bar (0.960 > 0.92). P2 passed the CPU ratio (0.909), but won only 3 of 5 pairs and grew brotli by 0.83 MB (limit 0.50). `[profile.release]` stays at `opt-level = "z"`, with a comment that records these numbers. `[profile.release-speed]` is untouched. Item outcome for Part P: **measured, no keep**.

## Setup

**Base.** origin/main c4832d2, built once into `<scratchpad>/I-024/base-pkg-out`. It is code-identical to a0e2ecb: c4832d2, and the later f983c82, add only `docs/` files.

**Candidates.** Each candidate is c4832d2 plus only its `Cargo.toml` profile sections, in a separate worktree (`w8-I-024-p1`, `w8-I-024-p2`). Each was a clean `build-web.sh` with `CARGO_TERM_VERBOSE=true` and its own `CARGO_TARGET_DIR`. Every verdict used the same base dir.

**Box.** Every verdict ran under the Chromium lock. The box was shared with other wave-8 items: their builds ran during some rounds, and perf.mjs waited for load1 < 2 before each run. One run in P1 round 3 (low@4x base) was invalid and re-run once in its slot.

**Verdicts.** Each verdict is `node tests/e2e/perf-pairs.mjs ... --noise evidence/noise-pairs.json --keep-runs <dir>`, 5 rounds × 2 configs × 2 sides. The numbers below are copied from the committed verdict files (`evidence/*-pairs.json` and `.summary`), not recomputed by hand.

## Noise run (base vs base): `evidence/noise-pairs.json`

| config | median CPU p50 base / "cand" (ms) | paired TTI diff median (ms) | TTI spread, max − min (ms) |
|---|---|---|---|
| low@4x | 29.1 / 30.7 (ratio 1.055, wins 2/5) | −180.6 | **892.7** |
| default@4x | 39.0 / 39.4 | +94.8 | 2 472.2 |

The low@4x spread is above 300 ms, so rule 3's limit is 893 rounded up to the next 100 ms, **900 ms**. Every candidate verdict states `low_tti_limit_ms: 900`. The same build differed from itself by 5.5% in CPU median, which is why the bar is 0.92 and why the wins rule exists.

## Candidates

| | base | P1: ours 3, deps z | P2: P1 + 10 hot crates at 3 |
|---|---|---|---|
| test 5: our crate (`--crate-name bevy_client`) | z | `-C opt-level=3` | `-C opt-level=3` |
| test 5: `bevy_reflect` | z | `-C opt-level=z` | `-C opt-level=z` |
| test 5: `bevy_ecs` | z | z | `-C opt-level=3` (and the other nine: 3) |
| `unused manifest key` warnings | — | 0 | 0 |
| clean build (s) | 181 (base, warm deps) | 700 | 584 |
| wasm raw (bytes) | 17 679 322 | 18 127 784 (+2.5%) | 27 191 626 (+53.8%) |
| wasm gzip -9 (bytes) | 5 043 310 | 5 164 584 | 7 047 484 |
| requested brotli (MB, perf.mjs) | 3.563 | 3.621 (+0.058) | 4.392 (**+0.829**) |

Notes on test 5:
- The spec names the crate `realgame_bevy_client`. In this build the package `realgame-bevy-client` compiles as `--crate-name bevy_client` (a bin target). That is the line checked above.
- The P2 Cargo.lock check found each of the ten names exactly once, so none was dropped.
- The P2 build's first attempt failed with `No space left on device` (the shared disk was full). It was re-run clean after deleting this lane's own P1 target dir.
- Evidence: `evidence/P1-test5.txt`, `evidence/P2-test5.txt`.

The size results differ from I-015's whole-program run, where opt 3 gave 34.74 MB raw. Here P1 grew the wasm by only 0.45 MB, so fat LTO plus `minsize` on the z crates leaves little of our crate's opt 3 in the merged module. That matches the small CPU gain. P2 reaches 27.2 MB because most of the size is in bevy and wgpu code.

## Paired verdicts (test 6)

| rule | limit | P1 | P2 |
|---|---|---|---|
| 1. low@4x CPU p50 median, cand / base | ≤ 0.92 | 30.9 / 32.2 = **0.960** ✗ | 29.0 / 31.9 = 0.909 ✓ |
| 1. pairs where cand < base | ≥ 4/5 | 4/5 ✓ | **3/5** ✗ |
| 2. default@4x paired TTI diff median | ≤ +1 000 ms | −258.1 ✓ | +405.3 ✓ |
| 3. low@4x paired TTI diff median | ≤ +900 ms (noise) | +212.5 ✓ | −50.6 ✓ |
| 4. brotli, cand − base | ≤ +0.50 MB | +0.058 ✓ | **+0.829** ✗ |
| 5. low@4x p99 median, cand / base | ≤ 1.10 | 59.2 / 62.2 = 0.95 ✓ | 61.8 / 58.1 = 1.06 ✓ |
| `perf-pairs` verdict | | `keep: false`, `["cpu 0.960 > 0.92"]` | `keep: false`, `["wins 3/5", "brotli +0.83 > 0.50"]` |

Also recorded, not gated: the default@4x CPU p50 median was 40.6 vs 39.8 for P1 and 34.3 vs 42.9 for P2 (5/5 wins at default@4x for P2).

**Stop order.** P1 failed rule 1 by less than 5 points (0.960 ≤ 0.97) and passed rules 2 to 5, so P2 was allowed and was built. With neither kept, there is no final verdict to run (test 11 needs at least one kept part, and Part R was reverted at test 8, see `lanes/render/round-1.md`). Quiet-box time used: about 3 h over the noise run, P1 and P2, within the 8 h budget.

**What this says for a follow-up.** Our own crate's opt-level is worth about 4% at low@4x, inside the noise. The hot-crate set is worth about 9%, but it costs 0.83 MB of brotli (this lane did not split that by crate). A follow-up could try a smaller set, `bevy_ecs` plus `glam` only, to stay under +0.5 MB. Whether the 9% is real needs more than 5 pairs: P2 lost 2 of 5 pairs at low@4x, while it won 5/5 at default@4x.

## TTI phases (test 11 table, base and candidates)

Medians of the 5 runs per side, in ms, from the verdict files. `compile_init_ms` is biased low by streaming-compile overlap; read it together with `fetch_ms`.

| verdict / side | config | fetch | compile_init | first_frame | to_interactive | TTI |
|---|---|---|---|---|---|---|
| noise / base | low@4x | 465.5 | 1 671.9 | 3 530 | 67.6 | 5 781.6 |
| noise / base | default@4x | 470.1 | 1 649.0 | 4 488 | 230.4 | 6 857.3 |
| P1 / base | low@4x | 456.5 | 1 827.6 | 3 492 | 73.7 | 5 708.7 |
| P1 / cand | low@4x | 480.0 | 1 707.1 | 3 663 | 73.1 | 5 761.2 |
| P1 / base | default@4x | 454.5 | 1 745.5 | 4 589 | 259.1 | 7 099.6 |
| P1 / cand | default@4x | 459.0 | 1 653.0 | 4 484 | 262.5 | 7 047.5 |
| P2 / base | low@4x | 450.3 | 1 756.0 | 3 630 | 87.5 | 5 869.7 |
| P2 / cand | low@4x | 563.6 | 1 703.9 | 3 622 | 74.7 | 5 946.7 |
| P2 / base | default@4x | 474.5 | 1 792.5 | 4 569 | 268.4 | 7 003.4 |
| P2 / cand | default@4x | 527.0 | 1 884.0 | 4 856 | 253.1 | 7 532.1 |

The phase sums are not exactly TTI, because each column is its own median.

**For the next load-time item.** At default@4x, base TTI is about 6.9-7.1 s, and it splits as follows:
- `first_frame_ms` ≈ 4.5-4.6 s (about 65%): Startup systems, pipeline and shader compilation, and the first 3 frames.
- fetch plus compile_init ≈ 2.1-2.2 s.
- to_interactive ≈ 0.25 s.

So the next TTI lever is the first-frame phase: pipeline and shader warm-up. Wasm compile comes after it. In this loopback run, base's default@4x TTI median was under B3's 8 s on every verdict (medians 6.9-7.1 s, single base runs 6.0-8.5 s), with other items' load on the box and no network.
