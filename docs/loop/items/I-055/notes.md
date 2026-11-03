# I-055: voxel perf and feel harness (wave-15-mac, Paawan's Mac)

Base: `origin/loop/I-052` at 3315e28. Branch: `loop/I-055`. Owned: `tests/e2e/voxel-perf.mjs`, `docs/loop/items/I-055/**`.

## What it does

`tests/e2e/voxel-perf.mjs` loads the default page (the block world) with `?debug` and walks a fixed route. At each stop it teleports, settles, sweeps the view once round (12 looks) and walks forward 1.5 s. The stops:
1. spawn (128, 36, 205)
2. the Village workshop
3. the bench plaza
4. the Kiln terrace
5. the Frozen Fields edge at the ford (112, 44, 146)
6. the Marsh edge (150, 36, 152)

Per stop and overall, it records:
- **CPU per frame:** the summed CPU of every rAF callback that shares a frame timestamp. Bevy's update and render is one of them. It reports p50, p95, p99 and max.
- **Hitches:** frames over 33 ms.
- **Frame interval (headed only):** p50, p95, p99 and hitches.
- **Chunk state:** `chunksMeshed`, `meshQ`, `matterMs` and the biome, from the voxel state keys.
- **Memory:** the JS heap (`performance.memory`) and WASM memory (the game's `STATS ... wasm_mem_mb`).
- **How far the player moved.**
- **Console errors.**
- **Startup times:**
  - first state
  - the game's `FIRST_FRAME`
  - **first playable frame**: the first voxel state with the spawn ring meshed (`chunksMeshed >= 18`, the spec's TTI rule)

**Budgets** are read from the text of section 9 of `docs/loop/items/I-052/spec.md` at run time:
- median CPU ≤ 16 ms
- TTI ≤ 8 s
- meshing ≤ 2 / 4 ms
- WASM ≤ 96 MB
- draw calls ≤ 250

If the text changes and a number can't be parsed, the report names the defaulted ones. The pass rule is: median CPU, first playable frame and WASM memory all within budget, and no console errors. Meshing ms and draw calls are not published by the game yet. When Lane A publishes `meshMs`, they can be added as checks.

**Output:** `runs/<voxel|classic>-<headless|mac>-<time>.json` and `.md` (or `--out DIR`). The Markdown is printed too. Exit codes: 0 pass, 1 a budget failed, 2 setup error.

## Profiles

- **Headless (cloud):** `node tests/e2e/voxel-perf.mjs`. Uses Playwright's Chromium, `?low`, and a 4x CPU throttle through CDP, the B2 setting. It reports CPU time only; frame interval means nothing without a real GPU.
- **Headed (this Mac):** `node tests/e2e/voxel-perf.mjs --headed`. Uses real Chrome (`CHANNEL=chrome`), DPR 2, no throttle and the default tier. It reports CPU time and the real frame interval.
- **Other flags:**
  - `--classic`: smoke test on the courtyard. It uses 2-number `tp` inside the walls.
  - `--throttle N`, `--low` / `--no-low`, `--query k=v`, `--dwell S`
  - `--self-test`: helpers and budget parsing, no browser
- **Env:** `BASE_URL`, `WEB_DIR`, `TTI_TIMEOUT_S`.

## Smoke runs (`runs/`)

All three ran against the I-051-voxel build, which is the contract plus voxel_cast, served on :8093.

- `classic-headless-*`: PASS.
  - CPU p50 4.7 ms, p95 6.5, p99 7.6 at 4x throttle.
  - 0 hitches.
  - First playable 1.4 s.
  - WASM 33.7 MB.
- `classic-mac-*` (real Chrome, M5 Pro): PASS.
  - CPU p50 2.3 ms, p99 3.4.
  - Frame interval p50 13.3 ms, p99 14.4.
  - 0 hitches.
  - First playable 0.5 s.
  - WASM 36.1 MB.
- `voxel-headless-*`: FAIL, as it should be. The contract's voxel stub publishes `chunksMeshed: 0`, so the spawn ring never meshes and first playable is n/a. The harness runs to the end and says so.

## When Lane A lands

1. Build: `bash crates/bevy-client/build-web.sh`.
2. Run `node tests/e2e/voxel-perf.mjs` (cloud) and `node tests/e2e/voxel-perf.mjs --headed` (Mac).
3. If spawn heights change, edit `ROUTE` at the top of the script. Voxel `tp` takes x y z, and y is the ground height plus 2.
