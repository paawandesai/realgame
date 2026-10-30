# I-003 break-1 (main lane), breaker findings

Tested at lane HEAD aff49c6 on a fresh WebGL2 build (wasm 17.66 MB raw, Brotli total 3.547 MB). Software GPU (SwiftShader).

## What held up
- Full run: exit 1 (bars fail honestly), valid JSON, all four configs valid, 0 console errors. default@1x TTI 7127 ms, default@4x 14242 ms (B3.tti fails, bar 8000). low@1x cpu p50 10.9, low@4x 36.6 (ratio 3.4, throttle really applies; B2.cpu fails, bar 16). Brotli 3.547 MB passes.
- Exit 2 for missing dir and for unreachable BASE_URL. Missing wasm: exit 1 in 32 s, tti null, 2 console errors, missing_from_dir lists it. Truncated wasm and missing hud.js: pass=false, valid=false.
- PERF_NO_CAST=1 gives casts all 0, run invalid. Subset runs with raised thresholds give partial:true, pass false.
- `--mac` with MAC_CHANNEL=chromium MAC_HEADLESS=1 runs end to end: mode mac, fps_p50 0.27, fps_p5 0.18, webgl2 true, SwiftShader renderer, verdict fails as expected.
- Not run (cost): acceptance 7 repeatability twice, WEBGPU build, golden.mjs regression.

## Blocking
None.

## Should-fix
1. Mac mode renders at devicePixelRatio 1. `browser.newContext({viewport})` without `deviceScaleFactor` gives DPR 1, but the real MacBook screen is DPR 2 (4x the pixels). The Mac B2 number will be optimistic versus what Paawan sees. Set `deviceScaleFactor: 2` (or use `viewport: null` and size the window) in mac mode, and report `devicePixelRatio` in the JSON.
2. Mac recipe step 5 pipes through `tee`, so the shell reports tee's exit code, not perf.mjs's. The text "exit code is 0 for a pass" is false as written. Use `set -o pipefail` (or `bash -o pipefail -c`), or use `OUT=~/perf-mac-<sha>.json node ...`, which the script already supports, and drop the tee.
3. Mac mode does not check the window is visible or focused. A covered or background window throttles rAF, which the recipe calls "worthless", yet the script cannot tell. Record `document.visibilityState` and `document.hasFocus()` during sampling and mark the run invalid if hidden. Likewise enforce the recipe's "SwiftShader means the run does not count" rule in code (renderer matching swiftshader/llvmpipe makes the mac run invalid).
4. B3 size is the sum of every file in WEB_DIR, not of what the page downloads. A stray 40 MB unserved file in the dir moved the total from 3.5 to 43.5 MB and failed B3.size with `missing_from_dir` empty. It errs on the safe side, but it can mask the true number. Report `requested_brotli_bytes` (only files actually requested) next to the dir total, and flag dir files never requested.
5. With BASE_URL, the size comes from WEB_DIR, which can be a different build than the one being served. Only file names are cross-checked. Compare the response body length of each requested file (Playwright `response.body()` or Content-Length) against the dir file and fail on a mismatch.
6. TTI is quantized to the software-GPU frame interval. `P.tti` is stamped at the start of the next rAF tick after state exists, so on a 2 s/frame run it overstates by up to one interval (default@1x: FIRST_FRAME 4527 ms vs TTI 7127 ms). That inflates B3.tti in exactly the cloud setting. Poll `__gameState` from a high-rate `setInterval` or a MutationObserver-free timer and stamp the first time it is valid, or report the quantization error alongside.
7. The cloud B2 statistic is the p50 of only 31 frames at 0.5 to 1 fps (MIN_FRAMES extended the window to 35 to 102 s), with about 2 casts per spell. At that sample size, a single cast frame (cpu p95 117 ms at default@4x) swings the median across the 16 ms bar. Raise MIN_FRAMES (for example 100) or report a bootstrap range. Also, default@1x vs low@1x p50 (11.8 vs 10.9) barely differ, so the CPU number is probably dominated by synchronous SwiftShader waits (the spec's own risk). Consider also reporting Bevy-update-only ms.
8. Throttle is "applied" only because the CDP send did not throw. Nothing verifies it took effect per run (a busy-loop calibration in the page would). Acceptance 3 catches a dead throttle only when a full run is compared by a human.
9. No visual check: a black or blank canvas, or a game that updates state but draws nothing, passes TTI, workload and errors. At least read back a pixel or two via `page.screenshot` and check it is not uniform.

## Nits
- Timeout overshoot: with BASE_URL pointing at a server that never answers the wasm and TTI_TIMEOUT_S=15, the run took 64 s before exiting 1 (browser/context close waiting on the pending request). Fine for acceptance 5b, but "never hangs" is looser than the timeout suggests.
- Unhandled paths exit 1 with a stack trace and no JSON: `page.goto` timing out (goto uses TTI_TIMEOUT_S as its own timeout, so a slow index.html ends the whole run instead of recording tti null), and any `page.evaluate` throw in `pageWorkload`. Wrap `runOne` in try/catch and record an invalid run.
- The static server calls `decodeURIComponent` outside its try block, so a request with a malformed `%` escape throws inside an async handler and kills the process. Localhost only, but trivial to move inside the try.
- BASE_URL normalization `replace(/\/?$/, '/')` breaks a URL that already has a query or hash.
- `ONLY=" "` or `ONLY=,` yields an empty list: no runs, partial true, pass false, exit 1 (acceptable), but it should be a usage error (exit 2).
- Recipe: `jq` is used in step 6 but is not in the one-time tools check. A pre-run `rustup ... | grep` prints nothing on failure and returns 1, which is fine but easy to misread.
- No persona/backlog leakage found: the script and recipe contain no references to personas, the backlog or code outside the repo paths.
