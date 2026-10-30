# I-003 lane main, round 1 (builder)

**Branch:** `loop/I-003`, from `origin/claude/fps-prototype` at `5a8f5fe` (the spec's `based_on` was `877131f`; the
branch moved on before this round, and the owned files do not overlap anything it changed).
**Owned files:** `tests/e2e/perf.mjs`, `docs/loop/mac-perf.md`. Round files: this log and `accept.mjs` (the
acceptance runner, which runs `perf.mjs` as a child process and checks every numbered test).
**Build under test:** `CARGO_TARGET_DIR=/home/claude/bevy-target ./crates/bevy-client/build-web.sh` (WebGL2,
default features) from this branch, copied into a scratch dir and tested from there. Nothing built is committed.

**Box:** 4 vCPU Intel Xeon @ 2.10 GHz, Chromium 153.0.8010.12 headless shell, Playwright 1.63.0, Node v22.22.0,
renderer `ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader driver)` (software GPU).
The box was shared with other agents during every run (load average 8 to 12 on 4 cores, with other
Chromium instances and `rustc` builds running). Absolute numbers are therefore pessimistic and noisy; the
comparisons inside one run (1x against 4x) are what the tests rely on.

## Tests first

`accept.mjs --only t2,t5,t6,t9` before `perf.mjs` existed: **0/10 passed** (every child exited with a
`MODULE_NOT_FOUND` error, exit 1, no JSON). Then `perf.mjs` was written and the full suite run.

## Results

Full suite: `node docs/loop/items/I-003/lanes/main/accept.mjs <pkg-out>`: **36/37**. The one failure was the t4
gzip check (below), fixed and re-run: `--only t4` **5/5**. So every acceptance check passes on the final code.

### First real measurement (full default run, `node tests/e2e/perf.mjs`)

Exit 1 (bars fail, every run valid). `pass == false`, `valid == true`, `partial == false`.

| bar | value | threshold | pass |
|---|---|---|---|
| B3.size (Brotli total) | **3.547 MB** (3,547,312 B) | 15 MB | **pass** |
| B3.tti (`default@4x`) | **16,721 ms** | 8,000 ms | **fail** |
| B2.cpu (`low@4x` cpu_ms.p50) | **33.8 ms** | 16 ms | **fail** |
| B2.mac | not measured here; see docs/loop/mac-perf.md | | |

| config | tti_ms | FIRST_FRAME t | frames | window s | cpu p50 / p95 / max ms | interval p50 / p95 ms | boot.js CPUFRAME p50 | errors | workload |
|---|---|---|---|---|---|---|---|---|---|
| default@1x | 8,991 | 5,760 | 31 | 102.6 | 15.3 / 38.7 / 68.6 | 2,717 / 4,866 | 10.7 | 0 | ok (casts 3/2/2/2, yaw 1472°) |
| default@4x | 16,721 | 11,305 | 31 | 115.4 | 45.9 / 87.0 / 312.8 | 3,433 / 4,983 | 36.2 | 0 | ok (casts 3/2/2/2, yaw 1422°) |
| low@1x | 4,447 | 3,556 | 31 | 45.6 | 12.7 / 66.1 / 132.1 | 1,300 / 2,033 | 9.5 | 0 | ok (casts 2/3/2/2, yaw 1422°) |
| low@4x | 9,538 | 8,574 | 31 | 31.3 | 33.8 / 68.2 / 75.0 | 933 / 1,283 | 28.0 | 0 | ok (casts 2/3/2/2, yaw 1422°) |

Both failing bars are the
measurement working. They become backlog items: "reduce `?low` CPU frame at 4x (33.8 ms against 16)" and
"reduce time to first interactive frame at 4x (16.7 s default, 9.5 s low, against 8 s)". Note that the
software GPU draws only one frame every 1.3 to 3.4 s here, so these TTI numbers include several very slow
early frames that a real GPU would not have; the Mac run is the honest check for the GPU side.

### Per test

1. **Runs, valid JSON.** Exit 1 (not 2). `schema: perf/1`, all four configurations, each with numeric
   `tti_ms`, `cpu_ms.p50/p95`, `frames == 31`, `workload_ok`, `throttle_applied`, 0 console errors. Verdicts
   carry value, threshold and pass. `pass (false) == size (true) AND tti (false) AND cpu (false) AND valid (true)`.
   Exit code matches `pass`. **PASS**
2. **Logic can fail.** `ONLY=low@1x --bars b3_brotli_mb=0.1`: exit 1, `B3.size.pass false`. `ONLY=low@4x
   --bars b2_cpu_p50_ms=0.01`: exit 1, `B2.cpu.pass false`. `ONLY=default@4x --bars b3_tti_ms=1`: exit 1.
   `ONLY=low@1x` with every bar raised: exit 1, `partial: true`, `B3.tti {value: null, pass: null, skipped: true}`.
   Full run with `b3_brotli_mb=1000,b3_tti_ms=1e9,b2_cpu_p50_ms=1e9`: **exit 0** (342.8 s). **PASS**
3. **Throttle applies.** `low@4x.cpu_ms.p50 / low@1x.cpu_ms.p50 = 33.8 / 12.7 = 2.66` (>= 1.5).
   `default@4x.tti_ms 16,721 > default@1x 8,991`. The software GPU's waits do not swamp the main-thread CPU
   number: throttling moves it by 2.7x (the GPU work runs in the GPU process; it shows up in the frame
   interval, not in rAF callback CPU). **PASS**
4. **Size.** Per file (perf vs `gzip -9c` vs `gzip -9nc`):
   ```
   bevy-client.js: perf=14804 gzip-9c=14819 gzip-9nc=14804 name=15B ok
   bevy-client_bg.wasm: perf=5032641 gzip-9c=5032661 gzip-9nc=5032641 name=20B ok
   boot.js: perf=688 gzip-9c=696 gzip-9nc=688 name=8B ok
   favicon.svg: perf=210 gzip-9c=222 gzip-9nc=210 name=12B ok
   hud.js: perf=3835 gzip-9c=3842 gzip-9nc=3835 name=7B ok
   index.html: perf=1104 gzip-9c=1115 gzip-9nc=1104 name=11B ok
   style.css: perf=1833 gzip-9c=1843 gzip-9nc=1833 name=10B ok
   ```
   Brotli round trip of the `.wasm` with `zlib.brotliDecompressSync`: identical bytes, 3,528,463 B, equal to
   the reported figure. Brotli `.wasm` 3.528 MB, within 1% of the README's "about 3.5 MB". Requested files:
   `bevy-client.js, bevy-client_bg.wasm, boot.js, hud.js, index.html, style.css`; `missing_from_dir: []`. **PASS**
   (after one fix, see Deviations 2).
5. **Broken input.** (a) `/nonexistent/i003`: exit 2, one stderr line `perf: web dir not found: ...`.
   (b) `.wasm` deleted, `ONLY=low@1x TTI_TIMEOUT_S=30`: exit 1 in 31.5 s, `tti_ms: null`, 2 console errors,
   `pass: false` (and the wasm shows in `missing_from_dir`). (c) `BASE_URL=http://127.0.0.1:9/`: exit 2,
   `perf: BASE_URL not reachable`. **PASS**
6. **Workload coverage.** Every configuration cast each spell 1 to 4 at least twice inside the window,
   saw 3 burning objects and turned 1,400+ degrees (table above). `PERF_NO_CAST=1 ONLY=low@1x`: casts
   all 0, `workload_ok: false`, exit 1. **PASS**
7. **Repeatable.** Two `ONLY=low@1x RUNS=1` runs: Brotli totals 3,547,312 and 3,547,312 (identical);
   `cpu_ms.p50` 10.3 and 12.7 ms (23% apart, within 30% on a loaded box). **PASS**
8. **Own hook vs boot.js.** Ratio `cpu_ms.p50 / cpuframe_log_p50_ms`: default@1x 15.3/10.7 = **1.43**,
   default@4x 45.9/36.2 = **1.27**, low@1x 12.7/9.5 = **1.34**, low@4x 33.8/28.0 = **1.21**. Gaps over 30% (1x
   runs): the script sums every rAF callback of a frame (Bevy's plus the HUD's), while boot.js times each
   callback alone, drops those under 0.3 ms, and averages per callback rather than per frame, so its median
   sits below the per-frame sum.
9. **Mac mode without a Mac.** `--mac` with `MAC_CHANNEL=chromium MAC_HEADLESS=1 MAC_SAMPLE_S=10`:
   `mode: "mac"`, `fps_p50 0.27`, `fps_p5 0.18`, renderer `ANGLE (Google, Vulkan 1.3.0 (SwiftShader ...))`,
   `webgl2: true`, `verdict.B2.mac {value: 0.18, threshold: 55, pass: false}` (expected on a software GPU).
   WebGPU sub-check, done: `WEBGPU=1 build-web.sh`, then `--mac` on it: exit 1 in 42 s,
   `error: "not a WebGL2 build"`, contexts `webgpu, webgpu-adapter`, plus the game's own panic
   "Unable to find a GPU!". **PASS**
10. **Recipe.** `docs/loop/mac-perf.md` has the 8 sections. Every command is fenced, the only placeholder
    is `<sha>` (the recording line format in section 7 is a format, not a command). Flags and env names
    used (`--mac`, the positional `WEB_DIR`) exist in `perf.mjs`. Paths match config.md Runtime
    (`~/Documents/realgame`, `~/bevy-target`). Playwright 1.63.0. No Chrome flags, no `sudo`. The Mac run
    itself: **not run** (no Remote Control session from this builder lane), so B2.mac stays unmeasured.
11. **No collateral.** `git diff --name-only origin/claude/fps-prototype...HEAD`: see the commit (only
    `tests/e2e/perf.mjs`, `docs/loop/mac-perf.md`, `docs/loop/items/I-003/**`). `npm test`: 34/34 pass. Golden
    path `QUERY=?low node tests/e2e/golden.mjs`: see "Golden" below. No `--no-verify`.

## Deviations from the spec (each small, each for a measured reason)

1. **The sampling window stretches to 30 frames.** On this box the software GPU draws a frame every 1.3 to
   3.4 s, so a fixed 20 s window holds 6 to 15 frames, under test 1's `frames >= 30`. The window is
   `SAMPLE_S` (20) long and then extends until it holds `MIN_FRAMES` (30) frames, capped at `MAX_SAMPLE_S`
   (150). The actual length is reported as `sample_s` per run. On a real GPU it never extends. A full run
   took 426 s (about 7 minutes), not 3 to 6.
2. **gzip uses the system `gzip -9n` when present** (fallback `node:zlib` level 9; `size.gzip_tool` says which).
   node:zlib's deflate came out 1.24% larger than GNU gzip on the `.wasm` (5,095,062 vs 5,032,641 B) and 4 B
   larger on `favicon.svg`, which failed test 4's 1% check. gzip is informational; Brotli (the bar) stays on
   node:zlib q11/lgwin 24. Test 4 compares to `gzip -9c` allowing exactly the stored file name in the gzip
   header (name + NUL, which `-9c` writes and a download never has), and to `gzip -9nc` at 1%: now 0.000%.
3. **The WebGL2 check watches which context the page asks for** (an init-script hook on
   `HTMLCanvasElement.prototype.getContext` and `navigator.gpu.requestAdapter`) instead of calling
   `getContext('webgl2')` on the game canvas, which would create a context on a canvas that has none yet. It
   also lets a WebGPU build stop early instead of waiting out the TTI timeout.
4. **The workload queues a whole cast in one frame** (`tp 0 5.5; face I; select N; cast`) and then waits for the
   cast count to rise; motion steps (`move`, `turn`, `look`) wait one frame. At under one frame a second a
   step-per-frame workload could not cast all four spells inside the window.

## Golden

`QUERY=?low node tests/e2e/golden.mjs` against the same build, run twice (once while the box was loaded,
once with it idle): **did not complete either time.** Both runs passed `boots`, `three fires at start`,
`mouse look turns`, `WASD moves` (and `jump`, `keys 1-4` in the first run), failed `a click over a target
locks the pointer and does not cast` (`locked=false, casts 0 -> 0`; headless Chromium refused pointer lock),
then the page crashed (`page.evaluate: Target crashed`) at about 0.1 fps with `wasm_mem_mb=35.0` (not wasm
memory growth). This lane changes no file that the golden path or the client reads (`golden.mjs`,
`crates/bevy-client/**` are untouched; `git diff` lists only the owned files), so this is the state of the
base branch on this box, not collateral from I-003. It belongs to I-001 (the golden path's owner) to
triage: the renderer crash under headless Chromium on a software GPU with a long, slow run.
Not counted as an I-003 failure; flagged for the orchestrator.
