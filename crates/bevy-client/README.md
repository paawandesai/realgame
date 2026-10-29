# bevy-client: 3D rendering spike

Spike only (branch `claude/bevy-spike`). It renders the **current, unchanged** 2D simulation
(`crates/sim`) in 3D with Bevy 0.20.1: cells become a thin voxel slab (run-merged quads, about
1,000 per frame instead of about 12,000 cubes), with a perspective camera following the player,
a sun light, up to 4 point lights on fires, and emissive fire with HDR bloom. The sim steps at
its fixed 60 Hz in `FixedUpdate`.

It is its own Cargo workspace (see `[workspace]` in `Cargo.toml`), so Bevy stays out of the
main workspace, its lockfile, `cargo clippy --all-targets` and CI.

## Build and run on macOS (web, WebGL2)

```sh
# One-time setup
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked   # must match wasm-bindgen in Cargo.lock
# optional: brew install binaryen   (wasm-opt; only used with WASM_OPT=1)

# Build (first build takes about 5 to 10 minutes)
cd crates/bevy-client
./build-web.sh                # WebGL2 build into web/pkg-out/
# WEBGPU=1 ./build-web.sh     # WebGPU-only build (separate .wasm; no WebGL2 fallback)
# LEAN=1 ./build-web.sh       # no bloom: smaller download

# Serve (any static server that sends application/wasm for .wasm works)
python3 -m http.server 8080 --directory web/pkg-out
```

Then open, in Chrome:

| URL | What it does |
| --- | --- |
| `http://localhost:8080/` | Play: A/D or arrows walk, Space jumps, 1 water, 2 boom, 3 split, 4 fire (click the canvas first so it has focus) |
| `http://localhost:8080/?auto` | Walks right and casts a spell every 50 ticks by itself: the busy benchmark scene |
| `http://localhost:8080/?auto&lite` | Same, "Low" tier: no HDR, bloom or fire point lights |
| `http://localhost:8080/?frozen` | Sim paused, meshes built once: Bevy's own per-frame cost (combine with `&lite`) |

## Reading the numbers

Open DevTools > Console. Every 2 seconds the app logs one line like:

```
STATS fps=59.8 avg_ms=16.72 worst_ms=21.3 sim_ms_per_frame=2.41 ticks_per_frame=1.00 sim_max_ms=3.10 build_max_ms=0.60 upload_max_ms=0.10 quads=1056 solid=1030 glow=6 gas=20 player_x=400 wasm_mem_mb=42.9
```

- `fps`, `avg_ms`, `worst_ms`: frames per second, mean and worst frame time over the 2 s window.
- `sim_ms_per_frame`, `ticks_per_frame`, `sim_max_ms`: time in the simulation per rendered frame, sim ticks run per frame (1.0 at 60 fps; above 1 means the sim is catching up), slowest single tick.
- `build_max_ms`, `upload_max_ms`: slowest voxel-slab mesh rebuild, and handing the meshes to Bevy.
- `quads` (`solid`, `glow`, `gas`): mesh size this frame. `wasm_mem_mb`: WebAssembly memory.
- `FIRST_FRAME t=...ms`: time from navigation start to the third rendered frame (logged once).

For main-thread CPU per frame, use DevTools > Performance, or CPU throttling (4x, 6x) there to
approximate a low-end Chromebook.

## Native run (macOS)

```sh
cd crates/bevy-client
cargo run --release              # add -- --auto, --lite or --frozen like the URL flags
```

## 3D chemistry chunk benchmark

`bench3d/` is a dependency-free 3D falling-sand + heat step over 16x16x16 chunks (the slice
design's "reaction bubble"):

```sh
cd crates/bevy-client/bench3d
cargo run --release                                                     # native
cargo build --release --lib --target wasm32-unknown-unknown
node run-wasm.mjs target/wasm32-unknown-unknown/release/voxel_bench3d.wasm   # WASM
```
