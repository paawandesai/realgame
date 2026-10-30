# I-022 rust lane, round 1

Branch `loop/I-022`, based on origin/main a0e2ecb (c4832d2 has no code change).
Commits: eb848f0 (tests 1-3, failing), b9074c8 (`tap_cone`, `vfov_for`, `Settings.aspect`), 026ac8d (keep `vfov_from_h` for the tests under wasm32 clippy).

## What changed

- `feel.rs`: `tap_cone(assist)` (1.5x the cone, at least 4.5 deg, 0 for off/NaN/negative) and `vfov_for(h, aspect)` (aspect clamped to [1, 4], non-finite or non-positive counts as 16:9). `vfov_from_h` delegates to `vfov_for(h, 16/9)`; no game code calls it any more, so it carries `#[allow(dead_code)]` (wasm32 clippy `-D warnings`), and its tests are unchanged.
- `main.rs`: `Settings.aspect` (default 16/9, `hfov` doc drops "at 16:9"); `fov_y()` uses it. `aim_and_cast` takes `ResMut<Settings>`, writes `settings.aspect = aspect` after computing the live aspect, builds `fov_y` from `vfov_for(settings.hfov, aspect)`, and uses `feel::tap_cone(settings.assist)`. `control`, the finder, the cast queue, the command filter and the format string are untouched.

## Tests 1-3 (feel.rs unit tests)

| | a0e2ecb | loop/I-022 |
|---|---|---|
| `rustc --test feel.rs` | does not compile: `error[E0425]: cannot find function tap_cone` (and `vfov_for`) | 7 passed (4 existing + 3 new) |
| `rustc --test chem.rs` | 19 passed | 19 passed |

## View width, measured (horizontal angle the player actually sees at View width 90)

| aspect | vertical fov before | horizontal before | vertical fov after | horizontal after | golden `fov` after (test 10) |
|---|---|---|---|---|---|
| 4:3 (960x720) | 58.72 | 73.74 | 73.74 | 90.00 | 73.74 |
| 16:9 (1280x720) | 58.72 | 90.00 | 58.72 | 90.00 | 58.72 |
| 21:9 (1680x720) | 58.72 | 105.39 | 46.40 | 90.00 | 46.40 |
| 4:3 at View width 100 | 67.67 | 83.58 | 83.58 | 100.00 | 83.58 |

Correction to the spec's "Facts": at 21:9 the old horizontal angle is 105.4 deg, not "about 113". It is still above GS-1's "about 100 deg" ceiling, so the case and the fix stand.

## Aim help off (golden test 9, pose `tp 4 2`, `bushes[5]`)

| | hover with Aim help on | hover with Aim help off | on again | exact hit, Aim help off |
|---|---|---|---|---|
| loop/I-022 | 5 (bush) | -1 | 5 | `lit[0]` (0) |

## Checks

- `cargo fmt --check` (crates/bevy-client): clean.
- `cargo clippy --release --locked --target wasm32-unknown-unknown -- -D warnings`: clean.
- `build-web.sh`: OK. Brotli (q11) of `pkg-out`: base 3,566,853 B, I-022 3,567,893 B, +1,040 B (wasm +903, hud.js +137); limit is +2 KB.
- A native `cargo build` was not run: on this 4-CPU machine it competes with the Chromium suites, and the wasm32 build and clippy cover the same code.

## B2/B3 guard (test 18): `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs`, under the Chromium lock

Base is `I-022/base-pkg-out` (a0e2ecb web files byte-identical; wasm from c4832d2, whose crate source equals a0e2ecb). Candidate is this branch's `pkg-out`.

| build | repeats cpu p50 (ms) | median cpu p50 (ms) | perf.mjs brotli total (B) | valid |
|---|---|---|---|---|
| base | 33.6, 30.6, 30.0 | 30.6 | 3,562,788 | 1 of 3 repeats valid (the others flagged "machine busy", load1 2.2-2.5, while another session's perf-pairs waited) |
| loop/I-022 | 30.6, 32.0, 29.3 | 30.6 | 3,563,772 | valid |
| delta | | **+0.0 ms** (limit +0.5) | **+984 B** (limit +2 KB) | |

A clean base re-run is queued behind another session's perf-pairs; if it lands it is added below.
