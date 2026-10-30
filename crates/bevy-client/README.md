# bevy-client: first-person 3D prototype

The first playable first-person build of Alchemist's Trial, where real chemistry is magic.
You walk around one small courtyard (about 48 × 48 m) and cast four spells from a glowing
gauntlet. Each spell does what the real chemistry does.

| Key | Spell | The real chemistry |
| --- | --- | --- |
| 1 | Fire | A gas flame: methane burns, CH₄ + 2O₂ → CO₂ + 2H₂O, and heat. One cast heats a target by 300 °C, so something chilled by liquid nitrogen must warm up first |
| 2 | Water | Water soaks up heat and cools things below their ignition temperature |
| 3 | Frost | Liquid nitrogen (N₂) boils at −196 °C (77 K) and freezes water |
| 4 | CO₂ | CO₂ from an extinguisher is heavier than air and smothers fire by pushing the oxygen away. The blanket stays until the object has cooled below its ignition temperature |

**Objective, "Save the Greenhouse":** three bushes are on fire, and heat spreads slowly from bush
to bush toward the greenhouse (the first neighbour catches after about a minute; left alone,
the greenhouse goes after about 2.5 minutes, and a red heat meter warns for the last 30 s).
Put out three fires before any reach it: the ones burning at the start, or ones the heat
spread to from one of those (a fire you light yourself, or one spread from it, does not
count). If nothing is burning before three are out (a fire burnt out by itself), the bush
farthest from the greenhouse catches fire, so the level always ends. Fire will not light the greenhouse
itself. A banner shows when you win; the result then stays until you press R (a spell still
in flight cannot change it, and R makes such spells vanish). You also find a dry hedge you can set alight, an ice wall that fire melts into a puddle
(frost refreezes the puddle into an ice sheet you slide on), a hot steam vent, and, behind
the ice wall, the lab bench, a placeholder for the water-splitting station.

Built with Bevy 0.20.1 from primitives only (no external assets): flat-shaded low-poly
meshes, bright toon colours, two tiny procedural textures (stone blocks, grass), emissive
flames with HDR bloom, 4x MSAA plus FXAA. Download: about 3.5 MB of WebAssembly with Brotli
(5.0 MB gzip). Its own Cargo workspace (see
`[workspace]` in `Cargo.toml`), so Bevy stays out of the main workspace and its lockfile.

## Code map

| File | What it is |
| --- | --- |
| `src/chem.rs` | The chemistry rules engine: a deterministic per-object state machine (temperature, burning, fuel, wet, ice, CO2 blanket) driven by spells and a fixed 30 Hz step. No Bevy dependency, so it can become the shared rules engine. Unit tests: `rustc --edition 2021 --test src/chem.rs -o /tmp/chem && /tmp/chem` |
| `src/world.rs` | The courtyard: shared meshes and materials, colliders, targets, ray casts |
| `src/player.rs` | Kinematic first-person character: AABB move-and-slide, no physics crate |
| `src/fx.rs` | A fixed pool of particles (fire, smoke, steam, frost mist, foam, water) |
| `src/bridge.rs` | Link to the page's JavaScript HUD (`web/hud.js`) |
| `src/main.rs` | App setup, input, aiming and aim assist, spells, projectiles, visuals, STATS |
| `web/` | Page, HUD (HTML/CSS), strict CSP (`script-src 'self' 'wasm-unsafe-eval'`, no inline script or style) |

## Build and run on macOS (web, WebGL2)

```sh
# One-time setup
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked   # must match wasm-bindgen in Cargo.lock

# Build (the first build takes about 5 to 10 minutes)
cd crates/bevy-client
./build-web.sh                # WebGL2 build into web/pkg-out/
# LEAN=1 ./build-web.sh       # no bloom or LUT tonemapping: smaller download
# WEBGPU=1 ./build-web.sh     # WebGPU-only build (no WebGL2 fallback)

# Serve (any static server that sends application/wasm for .wasm works)
python3 -m http.server 8080 --directory web/pkg-out
```

Then open `http://localhost:8080/` in Chrome and click the courtyard.

Native (for development, not yet tested for this prototype): `cargo run --release` (add
`-- --low`, `--sens=1.5` and so on). The HUD is web-only.

## Controls

| Action | Mouse and keyboard |
| --- | --- |
| Look around | Click the courtyard to lock the mouse, then move it. Or hold a mouse button and drag (works without the lock) |
| Move | W A S D or the arrow keys |
| Jump / sprint | Space / hold Shift |
| Cast | Left click (hold to keep casting) |
| Change spell | 1 to 4, the mouse wheel, or click the hotbar |
| Read the last message again | C (Esc also lists the recent ones) |
| Free the mouse and pause | Esc (shows the controls) |
| Restart | R |

**One scheme for every age.** Aim assist gently pulls a cast onto a target within about 4°
of the crosshair (the crosshair turns yellow when a cast will land on something; a hint
says so until three spells have landed). In **Point and click mode** (from the Esc card), a
young child can simply click a target to cast at it, and drag to look; the cursor becomes a
hand over anything that can be cast at. Taps on a touch screen always work that way.
Captions stay 8 s and the hotbar formulas stay visible on narrow screens. There is no on-screen touch
joystick yet: on a touch screen you can tap targets to cast, but you cannot walk.

## URL flags

| Flag | Effect |
| --- | --- |
| `?sens=1.5` | Mouse sensitivity (default 1; 0.0022 radians per pixel at 1). Raw deltas, applied every frame, no smoothing |
| `?assist=0` | Turn aim assist off (it is on by default) |
| `?low` | Low tier: no bloom, no shadows, no fire lights, no MSAA, fewer particles, rendered at 0.75 scale |
| `?scale=0.5` | Render scale from 0.25 to 1 (the drawing buffer really shrinks; CSS scales it back up) |
| `?aa=msaa` | Anti-aliasing: `msaa` (4x), `fxaa`, `both` or `off`. Default: `both`, or `fxaa` with `?low` |
| `?debug` | Enables the test commands that move the player or the clock (see Test hook) |

## Reading the numbers

DevTools > Console. Every 2 seconds the game logs a line like:

```
STATS fps=119.8 fps_p50=120.0 fps_p5=109.1 avg_ms=8.35 p50_ms=8.33 p95_ms=9.17 worst_ms=12.4 update_ms_p50=0.90 update_ms_p95=1.30 entities=770 burning=3 extinguished=0 wasm_mem_mb=34.9 heap_live_mb=24.3 heap_peak_mb=26.6 biggest_alloc_mb=4.0 winit_events=1
CPUFRAME n=240 p50_ms=2.40 p95_ms=3.10 max_ms=4.20
```

- `fps_p50`, `fps_p5`: median and 5th-percentile frame rate over the window (from frame times).
- `update_ms_*`: time in Bevy's main schedule (game logic) per frame.
- `CPUFRAME`: main-thread CPU per frame, measured around each `requestAnimationFrame`
  callback (Bevy's update and render both run inside it). Use this for CPU cost.
- `entities`: all ECS entities (about 320 are the pooled particles).
- `wasm_mem_mb`, `heap_*`: WebAssembly memory, live heap bytes, peak, biggest single allocation.
- `winit_events`: window events Bevy buffered this frame (should stay small; see `web/hud.js`
  for why pointer moves are read there and kept away from winit).
- `FIRST_FRAME t=...ms`: navigation start to the third frame (logged once).

## Test hook

`window.__game` (from `web/hud.js`): `state()` returns the game state (spell, aim, every
object's chemistry, objective, outcome, caption); `cmd(text)` queues a command: `select N`,
`cast`, `tap FX FY` (fractions of the canvas, 0 to 1), `reset`, and, only with `?debug`, `look YAW PITCH`, `turn DX DY`,
`face I`, `tp X Z`, `move RIGHT FWD SECS` and `tick N` (step the chemistry N times, at most
20 000). Numbers that are not finite read as 0. `cast` and `tap` wait for the same 0.3 s
cooldown as the mouse, one per cooldown, and wait while paused. The golden-path test,
`tests/e2e/golden.mjs`, drives a headless Chromium through all of it (it adds `debug`
itself). It is run by the product loop's smoke step, not by push CI.

## 3D chemistry chunk benchmark

`bench3d/` is a dependency-free 3D falling-sand + heat step over 16x16x16 chunks (the slice
design's "reaction bubble"):

```sh
cd crates/bevy-client/bench3d
cargo run --release                                                     # native
cargo build --release --lib --target wasm32-unknown-unknown
node run-wasm.mjs target/wasm32-unknown-unknown/release/voxel_bench3d.wasm   # WASM
```
