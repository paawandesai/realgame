# I-013: Harden the perf script (I-003 break-1 should-fix list)

**Moves:** B2 and B3 trust. No bar value is expected to move on its own; the numbers become harder to fake
or misread. **based_on:** origin/main. **Area:** `tests/e2e/perf.mjs`, `docs/loop/mac-perf.md`.
Source: `docs/loop/items/I-003/break-1-main.md`, should-fix 1 to 9.

## Changes

1. **Mac DPR.** Mac mode creates its context with `deviceScaleFactor` = `MAC_DPR` (default 2). Every run
   reports `device_pixel_ratio` read from the page. In mac mode a page DPR below `MAC_DPR` makes the run invalid.
2. **Real exit code in the recipe.** `mac-perf.md` step 5 writes the JSON with `OUT=` (no `| tee`) and
   prints `$?`. `jq` joins the one-time tools check.
3. **Visibility and renderer.** During sampling the workload loop records `document.visibilityState` and
   `document.hasFocus()` every tick: `visibility {samples, hidden, unfocused}`. Any hidden sample makes the
   run invalid (both modes). In mac mode an unfocused sample, or a renderer matching
   `swiftshader|llvmpipe|software`, also makes the run invalid. Every invalid run lists `invalid_reasons`.
4. **Size counts only what the page requested.** Requests are collected in every run (union).
   `size.requested_brotli_bytes/mb` is the Brotli sum over requested files found in the dir, and B3.size
   uses it. The dir total stays reported. `size.unrequested` lists dir files the page never asked for
   (flag only). No request at all gives B3.size `value: null` (fail).
5. **Served bodies match the dir.** For every same-origin response the body length is recorded.
   `size.body_mismatch` lists `{path, served, dir}` where they differ; a non-empty list makes the run invalid.
   Works the same with `BASE_URL`.
6. **TTI not frame-quantized.** The interactive check runs right after every rAF callback and from a 5 ms
   `setInterval`. `tti_ms` is the first time it holds; `tti_bound_ms` is the gap to the last failing check
   (the measurement's uncertainty); `tti_raf_tick_ms` keeps the old start-of-next-tick stamp for comparison.
7. **CPU p50 range.** `cpu_ms.p50_ci95 = [lo, hi]` from a seeded bootstrap (2000 resamples) of the
   sampled frames. `verdict.B2.cpu` carries `ci95` and `ci_straddles_bar`. `MIN_FRAMES` stays configurable
   (now validated as an integer >= 1).
8. **Throttle calibration per run.** After sampling (or a TTI timeout), a fixed busy loop is timed in the page
   in the run's own throttle state and again at rate 1, five interleaved pairs; the ratio is the median pair
   ratio (robust to load bursts from other processes). `throttle_calibration {rate, iterations, pairs_ms, ratio, ok}`;
   ok when `ratio >= 0.6 * rate` (and `<= 1.6 * rate`), or for rate 1 when `ratio <= 1.5`.
   `throttle_verified` must be true for a valid cloud run (`throttle_applied` is kept).
9. **Blank canvas.** A PNG screenshot of the canvas area with the DOM overlay hidden is decoded in Node.
   `canvas {w, h, luma_stddev, distinct_colors, blank}`; blank when stddev < 2 or fewer than 8 distinct
   (quantized) colors. A blank canvas makes the run invalid.

Found while building: since I-012 the game ignores `tp/move/look/turn/face` without `?debug`, so on
origin/main every perf run reports `yaw_deg: 0` and `workload_ok: false` (every run invalid). The page URL
now always carries `debug` (it gates only the test commands), as `golden.mjs` already does.

Also from the nits: the static server decodes the path inside its `try`; a throw inside one run is
recorded as an invalid run instead of a stack trace; `ONLY` that parses to nothing is a usage error (exit 2).

Test-only hooks (like `PERF_NO_CAST`), each makes a run invalid on purpose: `PERF_SKIP_THROTTLE=1` (never
sends the CDP throttle), `PERF_BLANK_CANVAS=1` (paints the canvas black with CSS).

## Acceptance tests (each can fail)

Unit: `node tests/e2e/perf.mjs --self-test` runs in-process tests with no browser and exits 0 only if all
pass. On origin/main it exits 2 (unknown flag). It covers: context options (mac DPR 2, cloud 1); mac
invalid reasons (SwiftShader, hidden, unfocused, low DPR) and a clean mac run with none; requested-size
accounting with a stray file (stray listed, not counted) and with no requests (null); body mismatch;
TTI tracker bound; bootstrap CI contains the median, is seeded, and narrows with more samples; throttle
check ok/not ok; PNG decode + blank detection on synthetic uniform and noisy images; a lint of
`mac-perf.md` (run line uses `OUT=`, no unguarded `| tee`, `jq` checked).

E2E (`docs/loop/items/I-013/lanes/main/accept.mjs WEB_DIR`, a real WebGL2 build):
- **a1** full cloud run: exit 0/1, every run has `device_pixel_ratio == 1`, `visibility.hidden == 0`,
  `throttle_verified == true` with ratio in range, `canvas.blank == false`, `tti_ms <= tti_raf_tick_ms`,
  numeric `tti_bound_ms`, `cpu_ms.p50_ci95` brackets p50, `size.body_mismatch == []`, and
  `verdict.B3.size.value == size.requested_brotli_mb`.
- **a2** mac mode with `MAC_CHANNEL=chromium MAC_HEADLESS=1`: `device_pixel_ratio == 2`, renderer SwiftShader,
  `valid == false`, `invalid_reasons` names the software renderer, exit 1.
- **a3** a copy of the dir plus a 40 MB random `stray.bin`: B3.size value within 1% of the clean run's
  requested total, `unrequested` contains `stray.bin`, dir total >= 40 MB.
- **a4** `BASE_URL` serving a copy whose `hud.js` has extra bytes, `WEB_DIR` the original: `body_mismatch`
  names `hud.js`, `valid == false`, exit 1.
- **a5** `PERF_SKIP_THROTTLE=1 ONLY=low@4x`: `throttle_verified == false`, run invalid, exit 1.
- **a6** `PERF_BLANK_CANVAS=1 ONLY=low@1x`: `canvas.blank == true`, run invalid, exit 1.
- **a7** `ONLY=,` exits 2.
