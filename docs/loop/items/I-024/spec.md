# I-024: Speed without the download (hot-crate opt-level, shared low material, TTI split)

**Revision 2** (after `critique.md`; see its "Planner response"). Changes: 5-round pairs with a base-vs-base noise run, TTI limits on paired differences, a p99 gate, P3 and Part W cut to follow-ups, a stop order and box budget, Part R's object/scenery rule and Builder design, relative GL targets, test fixes, and the I-019 merge order.

**based_on:** origin/main 265a93e3c53c0046bf2493e52bc85f6d984676fd (2026-10-30, after I-023 merged: game clock, perf hardening, no FXAA under `?low`; release profile back at `opt-level = "z"`).

**Origin.** Step B1 of I-015, split out when I-015 parked (`docs/loop/items/I-015/decision.md`). I-015 showed the whole-program trade: opt-level 3 cut CPU but cost download and load time. This item looks for the part of that speed-up that does not cost load time, and adds one render-side cut that I-015 identified but was not allowed to make.

## Facts from 265a93e this spec relies on

- **The release profile** (`crates/bevy-client/Cargo.toml`, its own workspace) is `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`. `[profile.release-speed]` (inherits release, opt 3) exists and is unused by `build-web.sh`. The root `Cargo.toml` profile does not apply to this crate.
- **I-015's whole-program measurements** (`I-015/lanes/client/round-1.md`, `round-2.md`, `decision.md`, `evidence/`):
  - On the quietest box, base 43.3 ms and opt-3 34.5 ms median CPU p50 at low@4x (−20.3%). Low@4x TTI went from 8 716 to 10 300 ms (+1.6 s). At default@4x, TTI was +285 ms (11 227 to 11 512, loaded box).
  - Wasm raw / requested brotli: z 17.67 MB / 3.557 MB, s 23.16 / 3.937, 2 32.61 / 4.533, 3 34.74 / 4.600. Clean build times: z 272 s, s 941 s, 2 1 259 s, 3 1 193 s.
  - Base runs spread from 30.4 to 48.2 ms across rounds on the same build. The noise is bigger than a 10% effect, so only paired, interleaved rounds count.
- **TTI is load-and-compile, not download, in the cloud perf run.** `perf.mjs` serves `WEB_DIR` from a loopback `node:http` server with no `Content-Encoding` and no network throttle. So the +1.6 s at low@4x came from compiling and instantiating a wasm twice the raw size under a 4x CPU throttle, not from the extra brotli bytes. Brotli (3.56 MB) is far inside B3's 15 MB, and target.md accepts a bigger download. Load time is the cost to protect.
- **What TTI is made of is not recorded.** `perf.mjs` records `tti_ms`, `tti_bound_ms`, `tti_raf_tick_ms` and `first_frame_log_ms` (the `FIRST_FRAME` line, logged at frame 3 by `main.rs`). It does not record when the wasm finished downloading, nor the `STARTUP t=<ms>` line that `setup` logs (main.rs:384) once the wasm is instantiated and the app is built. `boot.js` sets `window.__bootStart`; nothing reads it.
- **Monomorphised generics compile in the crate that uses them.** Bevy's system and query code is generic and is instantiated in `realgame-bevy-client` itself (release builds do not share generics). So our crate's opt-level governs much of the hot ECS code, and a `bevy_ecs` override alone will not reach it.
- **Per-package opt-levels and fat LTO.** Cargo applies `[profile.release.package.<name>]` to that crate's own codegen. With `lto = "fat"`, the merged module is then optimised at the root crate's level. Functions from `z` crates keep LLVM's `optsize`/`minsize` attributes, which limit inlining and unrolling in that pass. Whether the overrides survive LTO for this binary is an empirical question, and test 5 settles it.
- **The render side.** I-015 round 2 measured `PERF_GL_COUNT=1 ONLY=low@1x` (`I-015/evidence/round2-gl-calls-low1x.txt`): 1 894 GL calls and 39.8 draws per frame. Top calls: texParameteri 440, bindSampler 237, activeTexture 221, bindTexture 221. That is about 11 texture binds per draw. wgpu's GLES backend rebinds every texture and sampler in a bind group on each `set_bind_group`, plus two `texParameteri` calls per texture for the mip clamp. Bevy already skips a `set_bind_group` when the group has not changed.
  - So the cost tracks how often the material bind group changes. B3's mesh merge cut draws by 5% and saved nothing. B3 shared only the five state materials.
  - `world::build` makes 10 distinct untextured `toon` scenery materials (trim, cap, path, stone, stone_dark, wood, white, leaf, leaf2, metal; world.rs:477–487). Each is a separate `StandardMaterial` with the same roughness and reflectance and no texture. Every switch between them rebinds the fallback textures and samplers of the material group.
  - `world::build` takes no `low` flag today (main.rs:386).
  - The kit has four shape meshes: cube, ball, cyl and cone (world.rs:186, 301). Trees use the cone with `leaf`.
  - Chemistry objects keep a `StateMats` per object. For plain objects it is built by `plain(m)` (world.rs:804) with `normal: m.clone()`, where `m` can be one of the palette materials. The look code later writes `mats.<state>` back onto the entities in `Target.bodies`, `Target.wall` and `Target.puddle`.
- **`wasm-opt` is not installed on this box.** `build-web.sh` runs `wasm-opt -Oz` only with `WASM_OPT=1`. The spike found it saved under 1% compressed at z. Tuning it is out of scope (see Out of scope). The `brotli` CLI is not installed either. Brotli figures are `perf.mjs`'s `size.requested_brotli_mb` (node:zlib, quality 11).

## Problem

B2's cloud half (CPU p50 ≤ 16 ms at low@4x) is red, at about 40 ms on a quiet box. B3's TTI half (≤ 8 s at default@4x) is red too, at 10.5–11.7 s on a loaded box, base included. The one proven CPU lever, opt-level 3 everywhere, pays for its 20% with 1.6 s of low@4x load time. That moves B3 further from green, so it was reverted. We also cannot say which part of load time to attack, because TTI is a single number.

**Goal:** the largest CPU cut at low@4x that costs no load time, measured in a way the next perf item can repeat without debate. Load time is split into phases so that the next TTI item targets the right one.

## Bars moved

- **B2 (cloud half):** the intended move. Prediction on the quiet box, kept set against base (265a93e), paired: median CPU p50 at low@4x falls by at least 15% (about 40 to 34 ms). The hot-crate profile gives about 10–15%, of the 20% that whole-program opt 3 gave. The shared low material gives another 5–10% if the GL hypothesis holds. B2 stays red: 16 ms needs a further item. The Mac half (p5 fps) is not measured by this item. The orchestrator may take one Mac run on the merged artifact as a bonus reading.
- **B3:** must not regress. TTI guards are in the keep rule. Brotli may grow by at most 0.5 MB. The TTI split (Part T) does not move B3. It produces the numbers that the next load-time item is specced from.
- **B1:** must stay green (golden 3/3 at `?low` and default).
- **B4, B5, B6:** untouched. The only visible change is under `?low`, and it is a guarded no-op on the picture (test 9).

## Smallest version

Three parts, each its own commit and each measured alone against the same base build. Parts P and R are candidates under the keep rule below. Part T is measurement and always lands.

### Keep rule (applies to every candidate in P and R)

All measurements are paired and interleaved on the quiet box, with `tests/e2e/perf-pairs.mjs` (Part T).
- **CPU:** 5 rounds of `ONLY=low@4x RUNS=1`. The order alternates by round: base first in rounds 1, 3 and 5, candidate first in rounds 2 and 4.
- **TTI:** the same 5 rounds also run `ONLY=default@4x RUNS=1`, with the same alternation. low@4x TTI comes from the CPU runs.
- Every counted run must be valid, which means the throttle is verified and the load gate passes. An invalid run is re-run once (one extra `perf.mjs` child, same side and slot). If it is still invalid, `perf-pairs` exits 2 with no verdict.
- **Noise figure.** Once per item, before any candidate, the orchestrator runs `perf-pairs --base <base> --cand <base>` (base against itself, 5 rounds) and commits it as `evidence/noise-pairs.json`. Its per-config spread of paired TTI differences (max − min of cand − base) is the noise input to rule 3.

A candidate is **kept** only when all of these hold. "Paired difference" means candidate minus base within one round.
1. The median low@4x CPU p50 is ≤ 0.92 × the base median, and the candidate is below base in at least 4 of the 5 pairs.
2. The median paired default@4x TTI difference is ≤ +1 000 ms. (Base alone spreads about 1.2 s at default@4x, so +500 ms is under the noise.)
3. The median paired low@4x TTI difference is ≤ +500 ms. This is the condition that caught whole-program opt 3 (+1.6 s). If the noise run's low@4x spread is above 300 ms, the limit is that spread rounded up to the next 100 ms, capped at 1 000 ms, and the verdict states the limit used.
4. Requested brotli is ≤ base + 0.50 MB.
5. The median low@4x p99 is ≤ 1.10 × the base median p99 (GS-5, no new hitches).
6. Golden is green at `?low` (run alone).

Otherwise the candidate is reverted and recorded with its numbers. When two candidates of the same part both pass, the one with the lower CPU median is kept. If their medians are within 1.0 ms, the one with the smaller brotli wins.

### Order and budget on the quiet box

Each verdict is 5 rounds × 2 configs × 2 sides = 20 serial perf runs. Clean builds take 272 s (z) to about 1 260 s (opt 2/3).
- Order: noise run, then P1, then R, then P2 only if allowed, then final (test 11).
- **P2 is built and measured only if** P1 is kept, or P1 fails rule 1 by less than 5 points (CPU ratio ≤ 0.97) while passing rules 2 to 5.
- Budget: at most 5 verdicts (noise, P1, R, P2, final) and at most 8 hours of quiet-box time for the item. If the budget runs out, P2 is dropped first, and the round note says so. The I-015 stop rule (stop at the bar) does not apply: 16 ms is out of reach here.

`perf-pairs.mjs` computes this verdict itself (`keepRule`, test 2). The round notes paste its output and do not recompute it by hand.

### Part T: TTI split and the pair driver (always lands; measure lane)

1. **Phases in every perf run.** `perf.mjs` adds these fields to each run's JSON. Each is null when it was not observed.
   - `boot_ms`: `window.__bootStart`.
   - `wasm_response_end_ms`: the `responseEnd` of the `PerformanceResourceTiming` entry whose name ends in `_bg.wasm`.
   - `startup_log_ms`: the `t` of the `STARTUP t=<ms>` console line, caught the same way as `FIRST_FRAME`.
   - `startup_log_ms` uses an anchored match, `/\bSTARTUP t=(\d+(?:\.\d+)?)ms\b/`.
   - `tti_phases`: an object built by a new pure helper, `ttiPhases(run)`, with:
     - `fetch_ms = wasm_response_end_ms`
     - `compile_init_ms = startup_log_ms − wasm_response_end_ms` (compile, instantiate, plugin build)
     - `first_frame_ms = first_frame_log_ms − startup_log_ms` (Startup systems, pipelines and shaders, the first 3 frames)
     - `to_interactive_ms = tti_ms − first_frame_log_ms`
     - A phase is null if either end is null. A negative phase is kept as is and flagged in `tti_phases.warn`, never clamped.
     - **Caveat (stated in the contract).** With streaming instantiation, compile overlaps the download, so `compile_init_ms` counts only the compile work left after `responseEnd`. It is overlap-biased low, and `fetch_ms` includes some compile. A cached or streamed fetch can even make `compile_init_ms` negative, which is then flagged, not hidden. The next load-time item must read the two phases together.
   - The medians block gets the four phases too.
   - All four timestamps use `performance.now()` with the navigation as origin. The page hook already shares that clock with `now_ms()` in Rust.
2. **The pair driver** is a new file, `tests/e2e/perf-pairs.mjs`.
   - Usage: `node tests/e2e/perf-pairs.mjs --base <WEB_DIR> --cand <WEB_DIR> [--rounds 5] [--configs low@4x,default@4x] [--noise <noise-pairs.json>] [--out <file.json>]`. `--rounds` defaults to 5. `--noise` supplies the spread for rule 3; without it, the fixed +500 ms limit is used.
   - For each config and round, it runs `perf.mjs` as a child process with `WEB_DIR`, `ONLY=<config>`, `RUNS=1` and `OUT=<tmp>`, in the alternating order above.
   - It prints one line per run: config, round, side, CPU p50, p99, TTI, the four phases, valid. It ends with a verdict object: medians per side and config, the median paired TTI differences, `brotli_mb` per side, the per-pair wins, the low@4x TTI limit used, `keep: true|false` and `reasons[]`. With `--base` equal to `--cand` it also prints the TTI spread per config (the noise figure).
   - Its exit code is 0 when kept, 1 when not kept, and 2 on a usage or environment error, or an invalid run after its one re-run.
   - `--self-test` runs the `keepRule` and order unit tests without a browser, and exits 0 when all pass and 1 otherwise.
   - It never builds anything. Building the dirs is the lane's job.

### Part P: hot-crate opt-level (build lane)

Measure up to two profiles, in the order and under the budget above. Each is a clean `cargo build --release` with its own `CARGO_TARGET_DIR` under the scratchpad, so the base `target/` is not invalidated. `lto = "fat"`, `codegen-units = 1`, `panic`, `strip` stay unchanged in all of them.
- **P1, "ours 3, deps z":** `[profile.release] opt-level = 3`, plus `[profile.release.package."*"] opt-level = "z"`. `"*"` matches every non-member crate, so only `realgame-bevy-client` itself, and the generics it instantiates, get opt 3.
- **P2, "hot crates 3":** P1, plus `opt-level = 3` overrides for `bevy_ecs`, `bevy_render`, `bevy_pbr`, `bevy_core_pipeline`, `bevy_transform`, `bevy_mesh`, `glam`, `wgpu`, `wgpu-core` and `wgpu-hal`. Named overrides take precedence over `"*"`. Before building, the lane checks that each name appears as a `name = "..."` entry in `crates/bevy-client/Cargo.lock` (an override for an absent crate only warns), and drops and records any that do not.
- (P3, naga at 3, is a follow-up item, not part of this one.)

Rules for this part:
- Each candidate's round note records: wasm raw bytes, requested brotli, clean build seconds, the `perf-pairs` verdict, and the medians of the four TTI phases.
- `cargo build` of each candidate prints no `unused manifest key` warning.
- The kept profile, or none, goes into `Cargo.toml` with a comment that cites `I-024/lanes/build/round-1.md` and states the CPU, TTI and brotli numbers.
- `[profile.release-speed]` is left alone.

### Part R: one shared material for static scenery under `?low` (render lane, parallel to P)

- `world::build` gains a trailing `low: bool` parameter. `setup` passes `settings.low`. That is the only `main.rs` change.
- **Which pieces are scenery.** A piece is static scenery when its entity is spawned with one of the 10 untextured `toon` palette materials (world.rs:477–487) with one of the four kit shape meshes, **and** it is not referenced by any `Target` (not in `bodies`, not `wall`, not `puddle`, not in `flames`, not `foam`). The swap is a post-pass at the end of `build`, after every `Target` exists, so the rule does not depend on which `Builder` method spawned the piece:
  - `Builder::piece` (and so `block` and `tree`), when `low` is set, records `(entity, mesh handle, material handle)` for palette pieces in `low.candidates`.
  - At the end of `build`, the post-pass collects every entity referenced by any `Target` into a set, skips those, and re-inserts `Mesh3d` and `MeshMaterial3d` on the rest.
  - Why: `plain(m)` gives chem objects `StateMats.normal = m` (a palette handle). If such a body got a vertex-colour mesh, restoring `mats.normal` would draw palette colour × vertex colour, visibly darker. Target entities keep their original mesh and material.
- **Builder design.** `Builder` gains `low: Option<LowShare>`, where `LowShare` holds:
  - `shared: Handle<StandardMaterial>`: one `toon` material, white base, the same `perceptual_roughness` 0.9 and `reflectance` 0.2.
  - `palette: HashMap<AssetId<StandardMaterial>, LinearRgba>`: the 10 palette handles to their linear colour, filled right after they are made.
  - `cache: HashMap<(AssetId<Mesh>, [u32; 4]), Handle<Mesh>>`: the colour key is the four channels' `f32::to_bits`.
  - `candidates: Vec<(Entity, Handle<Mesh>, Handle<StandardMaterial>)>` and a `pieces` count.
  - The post-pass has `&mut Assets<Mesh>`. A copy is `meshes.get(kit_mesh).unwrap().clone()` with `insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[r, g, b, a]; n])`, where `n` is the vertex count. StandardMaterial multiplies vertex colour into the base colour, so the shading is the same.
  - With four kit shapes (cube, ball, cyl, cone; leaf cones included), there are at most 10 × 4 = 40 meshes.
- Vertex-colour meshes use one extra pipeline variant (`VERTEX_COLORS`). That is expected. The lane checks in the R verdict that the median `first_frame_ms` did not grow by more than 100 ms (rule 3 also catches TTI).
- Logged once: `LOW_SHARED_MAT colours=<n> pieces=<m> meshes=<k>`, where `pieces` counts swapped entities only.
- Left unchanged:
  - textured materials (grass and walls)
  - `glow` materials
  - glass and every other transparent material
  - every entity referenced by a `Target`, and every `StateMats` (the targets swap these at runtime)
  - the kit's own `water`, `ice`, `flame` and `foam`
  - colliders
  - everything when `low` is false, where the code path is identical to base
- Expected effect: texParameteri per frame at low@1x falls from about 440 by at least 30% (hard check, test 8). The expectation is about 1 300 GL calls and about 200 texParameteri, but that needs every bind-group change to come from the palette, so it is not the gate. If texParameteri does not fall by 30%, the hypothesis is wrong. The lane then records the counts, reverts, and does **not** go on to an atlas of textured materials.
- R is judged against base by the keep rule. If both P and R are kept, the final combined artifact is also run against base (test 11).

## Acceptance tests

"Red" means the test fails on 265a93e. "Guard" means it passes there and must keep passing. Quiet-box runs follow I-015's protocol: the orchestrator pauses builds and other perf suites, the Chromium lock is held, and if the load gate does not pass after one retry 10 minutes later, the item is `blocked` (environment), not `failed`.

**Pure unit tests (no browser, no build)**

1. **`ttiPhases` (red: it does not exist).** `node tests/e2e/perf.mjs --self-test` includes these new cases:
   - `{wasm_response_end_ms: 900, startup_log_ms: 2400, first_frame_log_ms: 3100, tti_ms: 3300}` gives `{fetch_ms: 900, compile_init_ms: 1500, first_frame_ms: 700, to_interactive_ms: 200}`.
   - Any null end gives null for that phase only.
   - `startup_log_ms < wasm_response_end_ms` keeps the negative value and sets `warn`.
   - A `STARTUP t=2412ms` console line parses to 2412, while `STARTUPX t=1ms`, `XSTARTUP t=1ms` and `STARTUP t=abc` give null.
   - All N existing cases still pass, where N is measured by running `--self-test` from the repo root on 265a93e **before** editing, and recorded in the round note. (The critic got 45 pass, 1 fail when run outside the repo.)
2. **`keepRule` and run order (red: the file does not exist).** `node tests/e2e/perf-pairs.mjs --self-test`. CPU lists are low@4x per round; TTI values are median paired differences.
   - Base CPU [40, 43, 48, 42, 45] and candidate [35, 38, 41, 37, 40] (median 38 ≤ 0.92 × 43 = 39.56, 5/5 wins), TTI +200 at both configs, p99 +5%, brotli +0.2: keep.
   - Base CPU [30, 43, 48, 44, 45] and candidate [31, 39, 49, 38, 45]: median 39 ≤ 0.92 × 44 = 40.48 passes, but only 2 wins in 5, so not kept, with reason `wins 2/5`.
   - Candidate CPU median 40 against base 43 (0.930), 5/5 wins: not kept, `cpu 0.930 > 0.92`.
   - low@4x TTI +501 ms, no noise input: not kept, `low@4x tti +501 > 500`. default@4x TTI +1 001: not kept, `default@4x tti +1001 > 1000`. default@4x +900: allowed.
   - Noise input with low@4x spread 420 ms: the limit is 500 (420 rounds up to 500); with spread 640 ms the limit is 700, and +650 is allowed; with spread 1 400 ms the limit is 1 000.
   - Median p99 +11%: not kept, `p99 1.11 > 1.10`.
   - Brotli +0.51 MB: not kept.
   - Any invalid run after its re-run: exit code 2, with no verdict.
   - With 5 rounds, the order is `[base, cand], [cand, base], [base, cand], [cand, base], [base, cand]`.

**Measurement plumbing**

3. **Phases are recorded (red).** `ONLY=low@1x RUNS=1 node tests/e2e/perf.mjs` on the base build. The run JSON has non-null `boot_ms`, `wasm_response_end_ms`, `startup_log_ms` and `first_frame_log_ms`, and has `tti_phases` with four numbers whose sum equals `tti_ms` to within 1 ms (a guard against nulls; the sum holds by construction). The real check: `first_frame_ms` and `to_interactive_ms` are non-negative. If `compile_init_ms` is negative, the run note records it with `warn` set and names the overlap caveat; it is not a failure. On 265a93e the fields are absent.
4. *(Cut: Part W and `WASM_OPT_LEVEL` moved to a follow-up. Number kept so later references stay stable.)*

**Part P**

5. **The overrides reach the compiler (red on base).** Run on P1 whenever P1 is built (kept or not), and on P2 if built. Do a clean `cargo build --release --locked --target wasm32-unknown-unknown -v`. In the log, the `rustc` line for `--crate-name realgame_bevy_client` has `-C opt-level=3`, and the line for `--crate-name bevy_reflect` has `-C opt-level=z`. For P2, the line for `--crate-name bevy_ecs` has `-C opt-level=3`. On base, every line has `opt-level=z`. The build prints no `unused manifest key` warning.
   - Informational only: the wasm raw size, against base (about 17.67 MB) and whole-program opt 3 (about 34.74 MB). It is recorded, not judged. The `-v` lines are the proof that the overrides reached rustc. Whether they survived fat LTO shows up in the CPU verdict.
6. **Paired verdicts for P1 (and P2 if built).** Each candidate's `perf-pairs` output and evidence lines are committed under `evidence/P<n>-pairs.json` and `.summary`. The kept one has `keep: true`. A part with no kept candidate is a valid outcome (see the fallback below).

7. *(Cut with Part W.)*

**Part R**

8. **Fewer texture rebinds (red).** `PERF_GL_COUNT=1 ONLY=low@1x RUNS=1` on base and on the R build, same box. Hard check: R's `top.texParameteri` is ≤ 0.70 × base's, and R's draws per frame are within 5% of base's. On base it is about 440 texParameteri and 39.8 draws. Expectation, recorded but not gated: about 1 300 GL calls and about 200 texParameteri. The `LOW_SHARED_MAT` line appears once, with `colours=10` and `meshes` ≤ 40. On a default-tier load (no `?low`) the line does not appear.
9. **The picture does not change (guard).**
   - Take screenshots of 5 fixed poses (the I-015 B3 set: spawn, facing the greenhouse, the lab bench, the hedge row, the courtyard corner; poses by `tp` and `look`, with the game paused) at `?low`, on base and on R.
   - The mean absolute per-channel difference must be ≤ 0.5 (of 255) per pose, ignoring the HUD layer (`#hud` hidden by the test). The expected value is about 0: vertex colour × white base is the same f32 product.
   - At the default tier the same comparison is ≤ 0.2 per pose.
   - Two base-vs-base captures give the noise figure, which is logged.
   - **State round trip (the StateMats hazard).** At `?low` on R: screenshot a pose framing a plain-material chem object (a bush and the greenhouse frame), drive it to Wet and back to Normal with the existing state keys or `look` commands, wait for the look to settle, and screenshot again. The mean difference to the first shot is ≤ 0.5 per channel. Also, the `LOW_SHARED_MAT` `pieces` count equals the number of palette pieces minus those referenced by a `Target`, which the lane prints from the same post-pass.
   - All poses are captured on 265a93e-based builds, before I-019 is merged in (see Overlap).
10. **Paired verdict for R** against base, in `evidence/R-pairs.json`.

**Item level**

11. **The combined kept set beats base.** If more than one part is kept, the merged artifact (P winner + R) gets its own paired verdict against base, in `evidence/final-pairs.json`, which must have `keep: true`. If exactly one part is kept, its test 6 or 10 verdict counts. If I-019 has merged by then, test 11 is re-run with base = origin/main at merge time (rebuilt once) and the candidate = that tree plus I-024's kept parts. The note records both bases (265a93e for lane evidence, the merge-time SHA for test 11) and never mixes their numbers with I-019's own test 21. The round note prints the four TTI phases of base and of final side by side. That table is what the next load-time item is specced from.
12. **Guards.**
    - `rustc --edition 2021 --test` passes for `clock.rs`, `motion.rs`, `chem.rs` and `feel.rs`.
    - `cargo fmt --check` is clean, and `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` is clean.
    - `npm test` is green.
    - Golden passes 3/3 at `QUERY=?low` and 3/3 at default, run alone (I-017).
    - `perf.mjs --self-test` and `perf-pairs.mjs --self-test` pass.

**Fallback.** If no candidate in P or R is kept, the item still merges Part T (tests 1–3 and 12, plus the noise run) and the evidence, and is closed as `measured, no keep`. The decision then cites the phase table from test 11 (base only) to pick the next CPU or TTI lever. It does not re-try whole-program opt 3.

## GS walk

| GS | P | status |
|----|---|--------|
| 5 | P0 | improved or unchanged: a lower CPU median at low@4x. p99 is recorded in every pair line and gated: keep rule 5 (median p99 ≤ 1.10 × base). Single-frame `max` is recorded, not gated, because one run's max is one frame |
| 6 | P0 | unchanged: no new effects |
| 12 | P0 | unchanged or improved: faster frames only |
| 19 | P0 | unchanged: colours under `?low` are identical by construction (vertex colour × white); test 9 |
| others | | not touched: no new screens, controls, text or effects |

## Out of scope

- Part W (`wasm-opt` level, binaryen install, `WASM_OPT_LEVEL`, flipping the `WASM_OPT` default): the spike showed under 1% at z, and a default flip would make deploy and CI depend on `wasm-opt`. Follow-up item.
- P3 (`naga` at opt 3, aimed at `first_frame_ms`): follow-up item, specced from the Part T phase table.
- Whole-program `opt-level` 2 or 3, `s`, thin LTO, `codegen-units > 1`, and `-C target-cpu` or SIMD flags. A follow-up may try `lto = "thin"` if test 5 shows that fat LTO erases the overrides.
- A texture or sampler atlas for the *textured* materials (grass, walls), merging `glow` materials, render-graph or shader changes, and patching wgpu's GLES bind path. Part R stops at the 10 untextured palette materials.
- Static mesh merging (I-015 B3: measured, no gain).
- Fixing load time itself: async or streaming instantiation changes in `boot.js`, pipeline pre-warming, splitting the wasm, serving pre-compressed files. Part T produces the data for that item.
- The Mac half of B2 beyond an optional bonus reading, and any change to the `perf.mjs` thresholds or validity rules.
- `index.html`, `boot.js`, `hud.js`, `golden.mjs` (no new golden checks; tests 8 and 9 are scripted in the render lane's evidence script under `docs/loop/items/I-024/lanes/render/`), `functions/`, `dashboard/`, the panel harness.

## Contract

Landed first by the measure lane (Part T's field names and CLI). The other lanes code against it and may use `perf.mjs` by hand until it lands.

- **perf run JSON**, added per run and in `medians`: `boot_ms`, `wasm_response_end_ms`, `startup_log_ms` (numbers or null), and `tti_phases: {fetch_ms, compile_init_ms, first_frame_ms, to_interactive_ms, warn?}`. Existing fields are unchanged and keep their order. New keys are appended. `compile_init_ms` is overlap-biased under streaming compile and may be negative (flagged in `warn`); read it with `fetch_ms`.
- **`perf-pairs.mjs`**:
  - CLI as in Part T.
  - Verdict JSON: `{base, cand, rounds, configs, runs:[{config, round, side, cpu_p50_ms, p99_ms, tti_ms, tti_phases, valid, rerun}], medians:{<side>:{<config>:{cpu_p50_ms, tti_ms, p99_ms, tti_phases}}}, paired_tti_diff_ms:{<config>: n}, tti_spread_ms:{<config>: n}, low_tti_limit_ms, brotli_mb:{base, cand}, wins:{<config>: n}, keep, reasons:[]}`.
  - Defaults: `--rounds 5`. One re-run per invalid run.
  - Exit codes: 0 kept, 1 not kept, 2 error (including still invalid after re-run). `--self-test`: 0 all pass, 1 otherwise.
  - Exported pure functions: `keepRule(verdictInput, noise?) -> {keep, reasons, low_tti_limit_ms}` and `pairOrder(rounds) -> [[side, side], ...]` (base first in odd rounds).
- **`world::build(commands, meshes, materials, images, low: bool)`**, and the log line `LOW_SHARED_MAT colours=<n> pieces=<m> meshes=<k>` (`pieces` counts swapped non-Target entities; `meshes` ≤ 40).

## Lanes

| lane | owns | builds |
|------|------|--------|
| measure | `tests/e2e/perf.mjs`, `tests/e2e/perf-pairs.mjs` (new) | Tests 1 and 2 first (red), then the phase fields, `ttiPhases`, STARTUP parsing, and the pair driver with `keepRule` and `pairOrder`. Test 3 on the base build. Lands the contract. Must not change thresholds, validity rules or the existing JSON keys |
| build | `crates/bevy-client/Cargo.toml` (profile sections only; the shared-file owner for this item), `docs/loop/items/I-024/lanes/build/**`, `docs/loop/items/I-024/evidence/P*` | P1 first: clean build in its own target dir, test 5, a `perf-pairs` verdict on the quiet box. Then P2 only if the stop order allows (check crate names in `Cargo.lock` first). Commits the kept profile, with a comment citing the numbers. Must not touch `src/**`, `build-web.sh` or `Cargo.lock` (no dependency changes) |
| render | `crates/bevy-client/src/world.rs`, the one `world::build` call in `crates/bevy-client/src/main.rs`, `docs/loop/items/I-024/lanes/render/**`, `docs/loop/items/I-024/evidence/R*` | Test 8 on base first (GL count). Then the `low` parameter, `LowShare` in `Builder`, the end-of-`build` post-pass that skips Target entities, the per-colour vertex-colour mesh cache, the shared white toon material, `LOW_SHARED_MAT`, tests 8, 9 (including the Wet round trip) and 10. If test 8 shows no 30% drop, it reverts and records. Must not touch `Cargo.toml`, `fx.rs`, `chem.rs`, `cues.rs`, `web/**` or tests. Adds no state keys |

**Overlap with I-019.** Both items touch `world.rs` and `main.rs`, in different hunks (I-019: `Look` move to `cues.rs`, `state_mats` hot/burning lines, spawn code, state keys, caption; I-024: `build` signature, `Builder`, palette, the one `world::build` call). I-024 adds no state keys, so I-019's golden key-count check is unaffected.
- **Merge order: I-019 lands first** (user-visible, larger contract). I-024's render lane rebases onto it before merge and re-runs `cargo fmt`, clippy, `npm test` and golden. The `Target`-reference rule in the post-pass must still hold after the rebase (I-019's new stream entities are not palette pieces or are Target-referenced; the lane checks).
- Lane evidence stays on 265a93e builds; test 11 is re-based as stated there.
- The quiet box is shared: I-019's test 21 joins the orchestrator's serial schedule, and its runs never overlap I-024's verdicts.

**Seam:** build and render touch disjoint files and are measured separately against the same base artifact (265a93e, built once into `<scratchpad>/I-024/base-pkg-out` and shared read-only). The orchestrator runs test 11 on the merged result. The quiet box is a serial resource: the orchestrator schedules each lane's `perf-pairs` runs one at a time, and no build may run during them. The seam breaker checks four things: that the build and render candidates were measured against the same base dir, that every verdict file comes from `perf-pairs.mjs` output rather than hand-typed numbers, that the `Cargo.toml` comment numbers match `evidence/`, and that `evidence/noise-pairs.json` exists and its spread is the one the verdicts used.
