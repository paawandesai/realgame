# I-028 perf guard (test 31, B2/B3)

Base: origin/main dc1d336 (the branch base), built with `build-web.sh` in a detached worktree
under `<scratchpad>/I-028/base`. Candidate: the integrated `loop/I-028` at b8cf7fe (merge of
the sim and client/web lanes), `pkg-out` copied to `<scratchpad>/I-028/cand-pkg-out`. The
later integration fix (e3dc0af) only changes bench.js and style.css for an open bench; with the
bench closed it adds no work.

Command, under the Chromium lock:
`node tests/e2e/perf-pairs.mjs --base <base pkg-out> --cand <cand pkg-out> --rounds 3 --configs low@4x --out docs/loop/items/I-028/evidence/perf-pairs.json`

Exit code 1 ("not kept": cpu 0.972 > 0.92, wins 2/3). That is perf-pairs' own keep rule for
perf items, which the spec says to ignore. All six runs valid, no re-runs.

| run | base cpu p50 (ms) | cand cpu p50 (ms) |
|---|---|---|
| round 1 | 29.3 | 27.2 |
| round 2 | 27.0 | 27.8 |
| round 3 | 28.2 | 27.4 |
| **median** | **28.2** | **27.4** |

**Bar: cand median <= base median + 1.0 ms -> 27.4 <= 29.2: PASS** (-0.8 ms).

Brotli (quality 11, node zlib; no `brotli` binary on this box), whole `pkg-out`:

| | base | cand | growth |
|---|---|---|---|
| all files | 3,601,702 B | 3,624,504 B | +22,802 B (22.3 KB) |
| `bevy-client_bg.wasm` | 3,571,710 B | 3,581,102 B | +9,392 B |

**Bar: growth <= 40 KB: PASS.** (perf-pairs' own figure: 3.596 MB -> 3.620 MB.)
