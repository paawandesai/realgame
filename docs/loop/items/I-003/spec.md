# I-003: Perf script (size, load, CPU frame ms) and the Mac real-GPU recipe

**Revision:** 2 (critic edits applied, see critique.md)
**Moves:** B2 (cloud CPU half, and makes the Mac half runnable), B3. Expected change: both bars go from *unmeasured* to *measured*. After this item, one command prints a JSON verdict for B3 and the cloud half of B2 on any built web dir, and `docs/loop/mac-perf.md` turns the Mac half of B2 into a recipe someone can copy and paste. Today `docs/loop/config.md` says `perf: todo`. The planner expects B3 size to pass easily: the README reports about 3.5 MB Brotli against the 15 MB bar. B3 load time and the B2 CPU half are honestly unknown at 4x throttle on a software GPU. Either result is fine. A fail is the measurement working, and it becomes a backlog item.
**based_on:** `877131fe4aa8293b033d44502cb20c83ab5dec16` (origin/claude/fps-prototype, PR #13, not merged). Loop docs were read from origin/main at `a7b48bd`.
**Area:** tests-e2e (`tests/e2e/perf.mjs`), plus one loop doc (`docs/loop/mac-perf.md`). Track: measurement. After: I-001 (the client). The lane branches from `origin/claude/fps-prototype`, because `tests/e2e/` and the client exist only there.

## Problem

Two of the eight bars cannot be scored, because nothing measures them:

- **B3, Fast to load:** Brotli download <= 15 MB total, and time to first interactive frame <= 8 s at 4x CPU throttle in cloud headless.
- **B2, Smooth on real hardware:** median CPU frame <= 16 ms with `?low` at 4x throttle in cloud headless, and p5 fps >= 55 over 60 s of the golden path on Paawan's Mac (M5 Pro, Chrome, WebGL2).

What exists today is only a starting point:
- `build-web.sh` prints the gzip size of the `.wasm` file, and its Brotli size only if a `brotli` CLI is installed. The cloud box has none. It ignores the JS, HTML and CSS files.
- `boot.js` logs `CPUFRAME` every 2 s. It times each `requestAnimationFrame` callback on its own (the HUD's callback and Bevy's are separate), drops any callback under 0.3 ms, and clears its buffer each window. So it is not a per-frame number, and under throttling the HUD's callbacks cross 0.3 ms and pull the median down.
- The Rust side logs `FIRST_FRAME` at the third Bevy frame. That does not prove the game accepts input (`window.__game.state()` returns a state).
- No script applies CPU throttling, and none compares results with the bars.
- Software GPU in the cloud makes fps meaningless there. The real-GPU number can only come from the Mac, through Remote Control, and no written recipe exists for that run.

## Smallest version

One Node ESM script, `tests/e2e/perf.mjs`, with the same style and dependencies as `golden.mjs`: `playwright` through `createRequire`, plus its own static server. It adds no npm dependencies. It uses `node:zlib` for Brotli and gzip.

```
node tests/e2e/perf.mjs [WEB_DIR]        # default crates/bevy-client/web/pkg-out
```

Env and flags:
- `BASE_URL`: test a running server instead of serving `WEB_DIR`. The size step still reads `WEB_DIR`.
- `SAMPLE_S`: sampling window per run, default 20.
- `WARMUP_S`: warm-up per run, default 3. Samples taken during warm-up are dropped.
- `RUNS`: repeats per configuration, default 1. With more than one run, the reported value is the median.
- `OUT`: also write the JSON to this path.
- `ONLY`: a comma-separated subset of configurations, for example `low@4x`.
- `--bars k=v,...`: override any bar threshold, for example `b3_brotli_mb=0.1`. Used by the acceptance tests.
- `--mac` mode: see below.

### Cloud mode (default)

1. **Size:** walk `WEB_DIR` and compress every file in memory with Brotli (quality 11, `BROTLI_MODE_GENERIC`, window size 24) and with gzip level 9. Report a per-file table and the totals in bytes and MB, where 1 MB = 10^6 bytes. Then add the files the page actually requested during the first cloud run, from Playwright `request` events. Any requested same-origin file missing from `WEB_DIR` is listed under `missing_from_dir`, and it fails the run, because the size total would be incomplete.
2. **Four configurations:** `default@1x`, `default@4x`, `low@1x`, `low@4x`. Each runs in a fresh browser context and page, so nothing is cached. The viewport is 1280x720, the same as `golden.mjs`.
   - Throttle: before `page.goto`, open a CDP session and send `Emulation.setCPUThrottlingRate {rate}`. Rate 1 still sends the call.
   - Timing hook: before navigation, `page.addInitScript` installs the script's own wrapper on `requestAnimationFrame`, which runs before `boot.js` adds its own. For each rAF tick, keyed by the callback timestamp, it adds up the time spent in every callback of that tick. That sum is **CPU frame ms** (main-thread CPU per displayed frame: Bevy update and render, plus the HUD). Frame intervals come from tick timestamps. The script does not rely on `boot.js`'s `CPUFRAME`. It parses those lines anyway and reports their last p50 as `cpuframe_log_p50_ms`, for cross-checking only.
   - **Time to first interactive frame (TTI):** milliseconds from navigation start (`performance.now()` at the start of a tick) to the first rAF tick where **both** of these hold: the Bevy `FIRST_FRAME` line has been logged, and `window.__game.state()` returns an object with a non-empty object list and `outcome === 'playing'`. The script also reports the raw `FIRST_FRAME t` value. The cap is `TTI_TIMEOUT_S` (default 120). On a timeout, TTI is `null` and the run fails. The script never hangs.
   - **Workload ("golden-path workload"):** once interactive, a fixed command loop drives the game through `window.__game.cmd`. It moves (`move`), looks (`look` / `turn`), aims with `face I` at each target type, does `select 1..4`, casts each spell, and sends `reset` if the outcome stops being `playing`. The loop is the same every run (a fixed script, no randomness), and it keeps going through warm-up and sampling. Samples are taken only from rAF ticks inside the sampling window. Before the window ends, the run checks and reports that: every spell was cast at least once, at least one object was burning, and the camera moved. A run that fails these checks is marked `workload_ok: false`, and it fails.
   - **Reported per run:** `tti_ms`, `first_frame_log_ms`, `frames` (ticks sampled), `cpu_ms {p50, p95, max}`, `frame_interval_ms {p50, p95}` (informational, because the GPU is software), `cpuframe_log_p50_ms`, `console_errors` (the count and the first 5), `panics`, `workload_ok`, `throttle_applied`. Any console error or panic makes the run invalid, which counts as a fail.
3. **Verdict:** When `ONLY` omits a configuration a bar needs, that bar reports `{pass: null, skipped: true}`. A skipped bar is not a failure, but `pass` is then `false` and the JSON carries `partial: true`, so a subset run can never print a full pass. Only a full run (no `ONLY`) can exit 0. Threshold values parse as JS numbers (`1e9` is valid).
   - `B3.size`: Brotli total <= `b3_brotli_mb` (15).
   - `B3.tti`: `default@4x` TTI <= `b3_tti_ms` (8000). The bar does not name a tier, so the stricter default build is used. `low@4x` TTI is reported too.
   - `B2.cpu`: `low@4x` `cpu_ms.p50` <= `b2_cpu_p50_ms` (16).
   - Also reported, but outside the pass rule: `B2.mac` set to `"not measured here; see docs/loop/mac-perf.md"`.
   - `pass` is true only when B3.size, B3.tti and B2.cpu pass **and** every run is valid. Exit codes: 0 means pass, 1 means a bar failed or a run was invalid, 2 means a usage or environment error (missing dir, Playwright missing, the server cannot start).
4. **Output:** a single JSON object on stdout, so `node perf.mjs | jq` works. Progress goes to stderr. The JSON's top-level keys are `schema` (`"perf/1"`), `based_on` (from `git rev-parse HEAD`, or `null`), `web_dir`, `env` (Node, Chromium and Playwright versions, `os.cpus()[0].model`, CPU count, and `gpu: "software"` when the WebGL renderer string contains SwiftShader or llvmpipe), `size`, `runs` (an object keyed by configuration), `bars` (the thresholds used), `verdict` (per-bar `{value, threshold, pass}`) and `pass`.

### Mac mode (`--mac`)

This mode is for the real-GPU half of B2, run on Paawan's Mac through Remote Control. It launches Playwright with `channel: 'chrome'` and `headless: false`, which uses the installed Google Chrome with the real GPU. It applies no throttle, uses the default tier and no `?low`, and sets the viewport to `MAC_VW`x`MAC_VH` (default 1512x945).

Steps: wait for TTI, run 5 s of warm-up, then run the same golden-path workload for `MAC_SAMPLE_S` (default 60) seconds. fps comes from the rAF tick intervals of the script's own hook: `fps_p50 = 1000 / interval_p50`, and `fps_p5 = 1000 / interval_p95`. The run also reports the last `STATS fps_p5` line as a cross-check.

It reads `WEBGL_debug_renderer_info` (`UNMASKED_RENDERER_WEBGL`) from a scratch canvas, and checks that the game canvas has a `webgl2` context. A WebGPU build fails with the message `not a WebGL2 build`.

Verdict: `B2.mac`: `fps_p5 >= b2_mac_p5_fps` (55). The same checks apply as in cloud mode: workload_ok, 0 console errors, 0 panics. The JSON uses the same schema, with `mode: "mac"`.

### `docs/loop/mac-perf.md`

An exact recipe the orchestrator can paste into a Remote Control session, with each command in a fenced block. It uses the paths in `docs/loop/config.md`: clone `~/Documents/realgame`, target dir `~/bevy-target`, Chrome 155. Sections:

1. **Preconditions:** charger connected, Low Power Mode off, no other heavy apps, the Chrome window in front and not minimized (background tabs throttle rAF), display at its native refresh rate. A note that ProMotion caps fps at 120, so p5 can never exceed the refresh rate.
2. **Sync:** `git -C ~/Documents/realgame fetch origin`, then `git -C ~/Documents/realgame checkout --detach <sha>`. `<sha>` is the commit under test, and the recipe says where it comes from: the item's merge or the PR head.
3. **One-time tools:** check that `rustup target list --installed` includes `wasm32-unknown-unknown`, that `wasm-bindgen --version` prints `0.2.129` (with the install command if it does not), that `node -v` is 18 or later, and run `npm i --no-save playwright@1.63.0` in a scratch dir or the clone (no package.json change). It does not download a browser, because the run uses `channel: 'chrome'`.
4. **Build:** `CARGO_TARGET_DIR=~/bevy-target bash crates/bevy-client/build-web.sh`, as a WebGL2 build, with no `WEBGPU=1`.
5. **Run:** `node tests/e2e/perf.mjs --mac crates/bevy-client/web/pkg-out | tee ~/perf-mac-<sha>.json`, with the expected duration (about 90 s).
6. **Read the result:** which JSON fields to copy back (`verdict.B2.mac`, `runs.mac.fps_p50`, `fps_p5`, `renderer`, `based_on`), and the pass rule (p5 >= 55, 0 errors, WebGL2).
7. **Recording:** the orchestrator, not the Mac session, appends the result to `docs/loop/scoreboard.jsonl` or `checks.md` "Last run". The recipe states the line format but does not edit those files.
8. **Troubleshooting:** fps stuck at 60 or 30 (power saving or a background window), `channel 'chrome' not found`, a wasm-bindgen version mismatch, and a black canvas (WebGPU build).

The recipe must not tell anyone to change Chrome flags, install drivers, or edit system settings beyond the preconditions in step 1.

## Acceptance tests

Run these on a fresh build of `based_on` (or the lane's HEAD) made with `CARGO_TARGET_DIR=/home/claude/bevy-target bash crates/bevy-client/build-web.sh`, with `/home/claude/bevy-target/tools` on `PATH`. The builder pastes every command's output, trimmed to the relevant lines, into `docs/loop/items/I-003/lanes/main/round-1.md`. That file is written by the orchestrator or the builder as the round log, and it is not an owned deliverable.

1. **Runs and produces valid JSON.** `node tests/e2e/perf.mjs > /tmp/p.json; echo $?` exits with 0 or 1, not 2. `jq` parses the output. The output has `schema == "perf/1"` and all four configurations. Every configuration has a numeric `tti_ms`, a numeric `cpu_ms.p50` and `cpu_ms.p95`, `frames >= 30`, `workload_ok == true`, `throttle_applied == true`, and `console_errors.count == 0`. The output has `verdict.B3.size`, `verdict.B3.tti` and `verdict.B2.cpu`, each with value, threshold and pass, and `pass` equals the AND of the three verdicts and run validity. (lane: main)
2. **The pass/fail logic can fail.** Run `ONLY=low@1x node tests/e2e/perf.mjs --bars b3_brotli_mb=0.1` (`ONLY` is an env var, never an argument). It exits 1, with `verdict.B3.size.pass == false` and `pass == false`. Run `--bars b2_cpu_p50_ms=0.01` with `ONLY=low@4x`. It exits 1, with `verdict.B2.cpu.pass == false`. Run `--bars b3_tti_ms=1` with `ONLY=default@4x`. It exits 1. Then run a FULL run (no `ONLY`) with every threshold raised very high (`b3_brotli_mb=1000,b3_tti_ms=1e9,b2_cpu_p50_ms=1e9`). It exits 0. Also confirm a subset run with raised thresholds exits 1 with `partial: true`. (lane: main)
3. **Throttling really applies.** In a full run: `low@4x.cpu_ms.p50 >= 1.5 × low@1x.cpu_ms.p50`, and `default@4x.tti_ms > default@1x.tti_ms`. If the first check fails, the builder investigates and reports in round-1.md whether the software GPU's synchronous waits dominate the main thread. Then either the measurement is fixed, or the item parks with that evidence. It is not waved through. (lane: main)
4. **Size is correct.** For every file, gzip bytes are within 1% of `gzip -9c FILE | wc -c`. Brotli bytes decompress back to identical content: the builder checks a round trip with `zlib.brotliDecompressSync` on the `.wasm` file and records it. The Brotli `.wasm` size is within 10% of the README's "about 3.5 MB" figure, or the difference is explained (for example, a LEAN build or a new feature). `missing_from_dir` is empty for the real build. (lane: main)
5. **Broken input fails loudly and never hangs.** (a) A non-existent `WEB_DIR` exits 2 with a one-line message. (b) Copy `pkg-out` to a temp dir, delete `bevy-client_bg.wasm`, and run with `ONLY=low@1x TTI_TIMEOUT_S=30`. It exits 1 within 90 s, with `tti_ms == null`, at least 1 console error recorded, and `pass == false`. (c) Run with `BASE_URL` pointing at a port with no server. It exits 2. (lane: main)
6. **The workload covers the golden-path actions.** The JSON reports per-spell cast counts for the sampling window, and each of spells 1 to 4 has at least 1. The run also records at least one burning object and at least 90° of total yaw change. Then temporarily set the workload to skip casting, using an env flag `PERF_NO_CAST=1` that exists only for this test. `workload_ok` becomes false and the run exits 1. (lane: main)
7. **Repeatable.** Run `ONLY=low@1x RUNS=1` twice. The Brotli totals are byte-identical, and the two `cpu_ms.p50` values are within 30% of each other. The builder records both. (lane: main)
8. **Own hook versus the boot.js log.** The JSON reports `cpuframe_log_p50_ms`. round-1.md gives the ratio of `cpu_ms.p50` to `cpuframe_log_p50_ms` for each configuration, with one sentence explaining any gap larger than 30%: the HUD callbacks are counted, and boot.js drops callbacks under 0.3 ms. (lane: main)
9. **Mac mode works without a Mac.** In the cloud, `node tests/e2e/perf.mjs --mac` with `MAC_CHANNEL=chromium MAC_HEADLESS=1 MAC_SAMPLE_S=10` (env overrides that exist so the code path can be tested) runs end to end. It prints `mode: "mac"`, numeric `fps_p50` and `fps_p5`, a `renderer` string and `webgl2: true`, plus `verdict.B2.mac` (expected to fail on a software GPU, and that is fine). A WebGPU build (`WEBGPU=1`), if one can be built in time, makes it fail with `not a WebGL2 build`. If that build is too slow, the builder may skip this sub-check and note it. (lane: main)
10. **The recipe is exact.** `docs/loop/mac-perf.md` has the 8 sections above. Every shell command in it is in a fenced block with no placeholders except `<sha>`. The flags and env names in the recipe exist in `perf.mjs`: a reviewer greps each one. The paths match `docs/loop/config.md` `## Runtime`. The playwright version matches the cloud install (1.63.0). It contains no Chrome flag changes and no `sudo`. If the orchestrator's Remote Control session can reach the Mac during this round, the tester runs the recipe once and pastes the JSON into round-1.md, which gives the first B2 Mac number. If it cannot, the result is "not run" and does not block. (lane: main)
11. **No collateral.** `git diff --name-only origin/claude/fps-prototype...HEAD` lists only `tests/e2e/perf.mjs`, `docs/loop/mac-perf.md` and files under `docs/loop/items/I-003/`. `node tests/e2e/golden.mjs` with `QUERY=?low` still passes, and `npm test` and the configured `test` command are unaffected. No commit uses `--no-verify`. (lane: main)

## Out of scope

- Fixing any perf result. A failing B2 or B3 number becomes a backlog item, for example "reduce `?low` CPU frame at 4x".
- Changing the client (`crates/bevy-client/**`), including `boot.js`'s `CPUFRAME` or the Rust `FIRST_FRAME`. The script measures from outside.
- Editing `docs/loop/config.md` (`perf:` command), `checks.md` "Last run", `bars.md` or `scoreboard.jsonl`. After merge, the orchestrator sets `perf: node tests/e2e/perf.mjs` in config.md.
- CI wiring. A ~15 min perf job in GitHub Actions is a follow-up.
- GPU frame time, WebGPU builds and any perf work on native.
- Chromebook or real-device runs other than the Mac.
- Adding `playwright` or any other dependency to `package.json`.

## Contract

none

## Risks and notes for the orchestrator

- **Software GPU distorts CPU frame ms.** SwiftShader or llvmpipe runs in the GPU process, but synchronous WebGL calls can block the renderer's main thread, and that time lands in "CPU frame". Test 3 exists to catch the case where throttling does not move the number. If it fails, B2's cloud half may need redefining (for example, Bevy update ms only). That is a bars change for Paawan or the arbiter, not a change for this lane.
- **Two clocks.** TTI uses the page's `performance.now()`, whose origin is navigation start. `FIRST_FRAME` uses the same clock through `now_ms()`. Both values are reported.
- **Lane base.** The lane works on `claude/fps-prototype` (PR #13). If PR #13 merges first, rebase onto main. The owned files do not collide with anything.
- **Runtime.** A full cloud run takes 4 configurations × (TTI + 23 s), plus browser start. That is about 3 to 6 minutes on the software GPU, which fits the ~15 min level-2 budget.

## Follow-ups (backlog candidates, not built here)

- Set `perf:` in `docs/loop/config.md` and add the Mac recipe to the checks.md level-2 row (orchestrator, after merge).
- A GitHub Actions perf job that uploads the JSON as an artifact and posts the verdict on the PR.
- A Chromebook-class proxy profile, for example 6x throttle with a 1366x768 viewport, if panel QA reports stutter that 4x does not show.

## Lanes

Merge order is the row order.

| lane | owns | builds |
|------|------|--------|
| main | `tests/e2e/perf.mjs`, `docs/loop/mac-perf.md` | cloud perf script (size, TTI, CPU frame ms at 1x and 4x for default and `?low`, JSON and a B2/B3 verdict), `--mac` real-GPU mode, and the Mac Remote Control recipe |
