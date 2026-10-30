# I-024 measure lane, round 1: Part T (TTI phases and the pair driver)

Base: origin/main c4832d2. It is code-identical to a0e2ecb: `git diff a0e2ecb c4832d2` touches only `docs/loop/`, so `perf.mjs`, `boot.js` and the client are a0e2ecb's.

## Test 1: `ttiPhases` and STARTUP parsing (`node tests/e2e/perf.mjs --self-test`, repo root)

| tree | passed | failed |
|---|---|---|
| base (c4832d2, before editing) | **N = 46** | 0 |
| branch (3fb9550) | 50 (46 existing + 4 new) | 0 |

The new cases were red before the change because `ttiPhases` and `startupLogMs` did not exist. They cover:
- the 900/2400/3100/3300 example giving `{900, 1500, 700, 200}`;
- null ends giving a null for that phase only;
- a negative `compile_init_ms` kept as it is, with `warn` set;
- the anchored `STARTUP t=` regex: 2412 parses, while `STARTUPX`, `XSTARTUP` and `t=abc` give null.

## Test 2: `keepRule` and `pairOrder` (`node tests/e2e/perf-pairs.mjs --self-test`)

15 passed, 0 failed. The file did not exist on base. The cases are the spec's:
- keep;
- `wins 2/5`;
- `cpu 0.930 > 0.92`;
- `low@4x tti +501 > 500`;
- `default@4x tti +1001 > 1000` (with +900 allowed);
- noise limits 420 → 500, 640 → 700 (+650 allowed), 1 400 → 1 000;
- `p99 1.11 > 1.10`;
- brotli +0.51 not kept;
- an invalid run after its re-run exiting 2 with no verdict;
- the 5-round order `[b,c],[c,b],[b,c],[c,b],[b,c]`;
- a driver re-run, the noise spread, and `runFields`.

The 15th case, added in 2192da7, covers `--keep-runs`. That option keeps each `perf.mjs` result as `<dir>/<config>-r<round>-<side>-a<attempt>.json` and reuses it, so a verdict killed by a container restart (about every 1.5-2 h, while a verdict is 20 serial runs) resumes with the same schedule instead of starting over. A re-used attempt file is replayed in its original slot, so the re-run rule is unchanged.

## Test 3: phases recorded on the base build (`ONLY=low@1x RUNS=1`)

| field | value (ms) |
|---|---|
| `boot_ms` | 45 |
| `wasm_response_end_ms` | 171.6 |
| `startup_log_ms` | 557 |
| `first_frame_log_ms` | 2 711 |
| `tti_ms` | 2 730 |
| `tti_phases.fetch_ms` | 171.6 |
| `tti_phases.compile_init_ms` | 385.4 |
| `tti_phases.first_frame_ms` | 2 154 |
| `tti_phases.to_interactive_ms` | 19 |
| sum of phases | 2 730.0 (= `tti_ms`) |

All four timestamps are non-null. `first_frame_ms` and `to_interactive_ms` are non-negative, and there is no `warn`. The run itself was `valid: false` ("machine busy: load1=3.94", because other items were building), which does not affect the field check. On a0e2ecb these fields are absent. At low@1x, about 79% of time to interactive is `first_frame_ms`: Startup systems, pipeline and shader compilation, and the first 3 frames. The 4x phase table is in `lanes/build/round-1.md`.

## Contract as landed

- **perf run JSON**, per run and in `medians`: `boot_ms`, `wasm_response_end_ms`, `startup_log_ms` and `tti_phases {fetch_ms, compile_init_ms, first_frame_ms, to_interactive_ms, warn?}`. New keys are appended. No thresholds, validity rules or existing keys changed.
- **`tests/e2e/perf-pairs.mjs`**: the CLI, verdict JSON, exit codes and exports as in the spec. The one optional addition is `--keep-runs <dir>`.

## Checks on the final branch tree (801d1d6 + this note)

The branch's client source equals origin/main: Part R was reverted, no profile was kept, and the only `Cargo.toml` change is a comment. The web build's `pkg-out` is byte-identical to the base build, all six files.

| check | result |
|---|---|
| `npm test` | 50 pass, 0 fail |
| `perf.mjs --self-test` / `perf-pairs.mjs --self-test` | 50/50, 15/15 |
| `rustc --edition 2021 --test` clock / motion / chem / feel | 3 / 8 / 19 / 4 pass |
| `cargo fmt --check` | clean |
| `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` | clean |
| `build-web.sh` | rc 0, 188 s incremental, wasm identical to base |
| golden `QUERY=?low`, run 1 (lock held) | 97/98: `cast: a real click casts exactly once` failed (casts 1 → 2 → 2) at fps 1.7 while the box's load average was about 4.7 from other items |
| golden `QUERY=?low`, run 2 (lock held) | **98/98** |

The run 1 failure is on an artifact byte-identical to origin/main's, so it is load flake in an existing check, not an I-024 change. Golden at the default tier was not run separately: the artifact is the same bytes as main's.
