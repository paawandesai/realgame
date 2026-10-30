# I-013 break-1 (main lane), breaker findings

Tested at d8bbefc on a fresh WebGL2 build of the branch (wasm 17.66 MB raw, Brotli total 3.548 MB), SwiftShader.

## What held up (all 9 original findings are fixed as specified)
- `--self-test`: 38 passed, 0 failed. `accept.mjs`: 48/48 (a1 to a7), including the full cloud run (all four configs valid, throttle ratios 0.95/4.24/1.01/4.04, no blank canvas, TTI 4.9 s vs old stamp 7.9 s, bound 44 to 206 ms, CI brackets p50).
- Mac mode headed (Xvfb) and headless: DPR 2, canvas screenshot 3024x1890, SwiftShader run invalid with a named reason.
- Usage errors exit 2: `ONLY=,`, `ONLY=" "`, unknown config, `MIN_FRAMES` 0 / 2.5 / abc, `MAC_DPR=0.5`, negative `SAMPLE_S`.
- Stray 40 MB file: B3.size unchanged, stray listed under `unrequested`. Extra-bytes `hud.js` via BASE_URL: caught.

## Blocking
None.

## Should-fix
1. Same-length body swap is not caught (finding 5 only half closed). `body_mismatch` compares lengths. Reproduced: a BASE_URL server serving a copy of `hud.js` with one byte changed (same 13544 bytes), WEB_DIR the original: `body_mismatch == []`, requested size unchanged. The body (or the refetched buffer) is already in hand at that point, so compare a hash (sha256 of the served body vs the dir file) instead of, or as well as, the length.
2. The mac `unfocused` check is very likely vacuous. Playwright emulates page focus, so `document.hasFocus()` returned true in every sample, both headless and headed under Xvfb (`unfocused: 0` in every mac run I did and in all acceptance runs). So "window lost focus" can never trip, yet `mac-perf.md` and the spec present it as a guard. `hidden` still works only if Chrome reports occlusion/minimize. Either say plainly in the recipe that focus loss is not detectable (the covered-window caveat is there, but the intro claims the lost-focus check), or drop the claim. Cannot be verified on a real Mac from here; worth one look at `visibility` in the first real Mac JSON.
3. BASE_URL with a path prefix (`http://host/game/`) gives a misleading failure. `sameOriginPath` keeps the prefix (`game/hud.js`), nothing matches the dir, so `missing_from_dir` lists every file, B3.size `value: null` and the run is invalid. Reproduced. It fails closed, but spec item 5 says BASE_URL "works the same"; strip the base path (or reject a path in BASE_URL with exit 2).

## Nits
- Blank-canvas check is a heuristic on a composite screenshot: with `canvas{opacity:0}` and a CSS gradient behind it (94 colors, stddev 13.6) the run was `valid: true`. A game that draws only a sky gradient would pass. Acceptable for the stated purpose.
- Visibility sampling is sparse on a software GPU (17 to 19 samples per 20 s run, because the 50 ms interval is starved by 1 to 2 s frames). Fine on a real GPU, but "no hidden samples" is weak evidence in cloud runs.
- `browser.newContext()` / `newPage()` in `runOne` sit outside the try, so a browser crash there still ends in a stack trace rather than an invalid run.
- BASE_URL normalization `replace(/\/?$/, '/')` still mangles a URL with a query or hash (`...?x=1#h` becomes `...?x=1#h/`). Carried over from I-003 nits, not in the spec list.
- `RUNS=0` is silently run as 1; `MAC_DPR=1` is accepted and then makes the DPR check pass at 1 (reported in `env.device_scale_factor`, but a mac run with a lowered DPR can still say `valid: true`).
- `env.gpu` regex only matches swiftshader|llvmpipe, not the `software|softpipe` set used for the mac validity rule. Cosmetic.
- Recipe: `cd ... && OUT=... node ... > /dev/null; echo "perf exit=$?"` prints the status of the whole `&&` list, so a failed `cd` prints `perf exit=1` (looks like a bar failure). Also `brew install jq` assumes Homebrew.
