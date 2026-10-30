# I-013 lane main, round 1 (builder)

**Branch:** `loop/I-013` from `origin/main`. **Owned files:** `tests/e2e/perf.mjs`, `docs/loop/mac-perf.md`,
`docs/loop/items/I-013/**` (spec, this log, `lanes/main/accept.mjs`).
**Build under test:** `CARGO_TARGET_DIR=/home/claude/bevy-target crates/bevy-client/build-web.sh` (WebGL2,
default features) from this branch, copied to a scratch dir. wasm 17,656,950 B raw.
**Box:** 4 vCPU, Chromium 153.0.8010.12 headless, Playwright 1.63.0, renderer SwiftShader (software GPU,
fps meaningless). Shared with other agents throughout: load average 8 to 15 on 4 cores.

## Tests first

- `node tests/e2e/perf.mjs --self-test` with the tests written and no helpers: **1/35 passed** (34 failed,
  each `... is not defined`), exit 1. On origin/main the flag does not exist (exit 2).
- `accept.mjs --only a5,a6,a7` on the unchanged measurement code: **2/5 passed**. a5 (`throttle_verified`
  missing), a6 (`canvas` missing) and a7 (`ONLY=,` exited 1, not 2) failed.
- Three more unit tests were added test-first during the round (each seen failing before its code):
  `body_unverified`, `pageUrl` (`?debug`), `calibrationRatio`.

## Found while building: the workload was dead on origin/main

Since I-012 the game ignores `tp/move/look/turn/face` without `?debug`. origin/main's perf.mjs loads `/` or
`?low`, so every run had `yaw_deg: 0`, `workload_ok: false` and was invalid (seen in the pre-change a5/a6
runs and in the first a1 run of this round: 4/4 configs "workload incomplete"). The page URL now always
carries `debug`, which gates only the test commands (`main.rs` 611-619), the same as `golden.mjs`.

## Results (final code)

- `node tests/e2e/perf.mjs --self-test`: **38 passed, 0 failed**, exit 0.
- `npm ci --silent && npm test`: exit 0, **34/34** pass.
- `accept.mjs` full suite (run 3, acc3): **46/48**. The two failures were one cause: `default@4x` canvas
  check timed out in `locator.boundingBox` (30 s default) on a busy 4x-throttled page under load 14.
  Fixed (the box now comes from the same `page.evaluate` that hides the overlay; `page.screenshot` with a
  clip, 120 s, one retry). Re-run `--only a1,a6,a7` (acc4): **37/37**. a2 to a5 last ran in acc3 and passed
  (8/8); the change since touches only how the canvas box is read.
- I-003 regression (`docs/loop/items/I-003/lanes/main/accept.mjs --only t5,t6`): **4/4** (missing dir exit 2;
  missing wasm exit 1 in 33.8 s with tti null; no server exit 2; `PERF_NO_CAST` still fails the workload).

### Full cloud run (a1, acc4): exit 1, `valid: true`, 443.9 s

| bar | value | threshold | pass |
|---|---|---|---|
| B3.size (requested files only) | **3.547 MB** (3,547,434 B; dir total 3,547,631 B, unrequested: `favicon.svg`) | 15 | pass |
| B3.tti (`default@4x`) | **8,094.7 ms** (bound 272.5 ms; old rAF-tick stamp 10,188.7) | 8,000 | fail |
| B2.cpu (`low@4x` p50) | **29.5 ms**, 95% range [27.5, 36.8], does not straddle the bar | 16 | fail |

| config | tti | old rAF stamp | bound | FIRST_FRAME | frames | cpu p50 [95% range] | throttle ratio | canvas sd / colors | vis samples / hidden | workload |
|---|---|---|---|---|---|---|---|---|---|---|
| default@1x | 3,972.9 | 5,552.7 | 101.4 | 3,874 | 31 | 11.1 [10.1, 13.2] | 0.93 | 32.3 / 1405 | 18 / 0 | ok, yaw 1472 |
| default@4x | 8,094.7 | 10,188.7 | 272.5 | 7,833 | 31 | 45.1 [40.6, 51.9] | 4.51 | 31.1 / 1725 | 17 / 0 | ok, yaw 1422 |
| low@1x | 2,414.3 | 2,807.9 | 32.6 | 2,384 | 45 | 6.5 [5.9, 7.6] | 1.02 | 28.5 / 2173 | 24 / 0 | ok, yaw 2022 |
| low@4x | 6,009.0 | 6,570.7 | 76.8 | 5,939 | 31 | 29.5 [27.5, 36.8] | 4.09 | 41.5 / 1692 | 16 / 0 | ok, yaw 1330 |

TTI is now within 33 to 273 ms of FIRST_FRAME, where the old stamp ran 0.4 to 2.4 s late. That matters:
B3.tti at default@4x is 8.09 s against the 8 s bar, not 10.2 s. Every served body matched its dir file in
length (`bevy-client_bg.wasm` measured by re-fetching, since Playwright cannot hand over a streamed wasm body).

### Per test (acc3 for a2 to a5, acc4 for a1, a6, a7)

- **a1** all 33 checks pass: DPR 1, visibility sampled and never hidden, throttle verified (ratios above),
  canvas not blank, `tti_ms <= tti_raf_tick_ms`, CI brackets p50, every run valid, `body_mismatch: []`,
  `B3.size.value == requested_brotli_mb`, B2.cpu carries `ci95` and `ci_straddles_bar`.
- **a2** `--mac` with `MAC_CHANNEL=chromium MAC_HEADLESS=1`: `device_pixel_ratio: 2`, renderer SwiftShader,
  `invalid_reasons: ["software renderer: ANGLE (... SwiftShader driver)"]`, exit 1.
- **a3** 40 MB `stray.bin` added: dir total 43.548 MB, `unrequested: ["favicon.svg","stray.bin"]`, B3.size
  3.547 (the clean run's requested total).
- **a4** `BASE_URL` serving a `hud.js` with 39 extra bytes: `body_mismatch: [{"path":"hud.js","served":[13583],"dir":13544}]`,
  `valid: false`, exit 1.
- **a5** `PERF_SKIP_THROTTLE=1 ONLY=low@4x`: pair ratios give median 0.98, `ok: false`,
  `invalid_reasons: ["throttle not verified by calibration"]`, exit 1. (Before the interleaved median, a
  single min-of-3 pair under load 14 read 2.76 and passed; that is why the calibration takes the median of
  five interleaved pairs.)
- **a6** `PERF_BLANK_CANVAS=1`: `luma_stddev 0, distinct_colors 1, blank: true`, `invalid_reasons: ["blank canvas"]`, exit 1.
- **a7** `ONLY=,`: exit 2, `perf: ONLY lists no configuration: ","`.

## Deviations and notes

- Requests and served lengths are collected in every run (union), not only the first.
- Unfocused samples invalidate only mac runs; headless cloud runs report them (0 here).
- Visibility is sampled from the workload's 50 ms timer, which a busy software-GPU main thread fires only
  16 to 24 times per window. A hidden page would still be caught (the `visibilitychange` listener counts too).
- `MIN_FRAMES` default stays 30; the bootstrap range shows the p50's uncertainty instead (B2.cpu low@4x:
  [27.5, 36.8], far from 16, so more frames would not change the verdict today).
- Not done (nits, out of scope): BASE_URL query/hash normalization, Mac recipe `rustup` check wording.
