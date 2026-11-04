# gfx-lab: graphics G0 spike

Graphics phase G0 from [Formula: graphics upgrade plan](https://claude.ai/artifact/CVmLpTpyZDSEUQmF23bGwz). It is not part of the game: it has its own Cargo workspace (like `bevy-client`) and touches no game files.

It holds two things:

1. **A Bevy test scene** (Bevy 0.20.1, the game's version). It shows a lab bench, a glass chamber, and H2O, CH4 and CO2 at real bond lengths and angles. The molecules are drawn with the ray-traced atom impostors planned for G1 (`src/atoms.rs`, `src/atoms.wesl`). You switch each rendering feature on from the URL (`?f=shadows,transmission`), and the page reports its adapter and frame times as `window.__gfxlab`.
2. **A Bench preview harness** (`bench-preview/`). It drives the free-play Bench's three.js scene (`crates/bevy-client/web/bench-scene.js`) with fixed molecules, so a look change can be captured before and after without building the chemistry wasm.

## Run it

```sh
# Bevy scene: both APIs into web/pkg-out/{webgl2,webgpu}
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
RUSTUP_TOOLCHAIN=1.97.1 ./build-web.sh          # Bevy 0.20.1 needs rustc 1.97.1
node run-matrix.mjs /tmp/matrix                  # every feature, both APIs, headless
FEATURES=base,ssao APIS=webgl2 node run-matrix.mjs /tmp/m

# Bench preview: three captures (plate, reaction, low tier), from the repo root
node crates/gfx-lab/bench-preview/shoot.mjs /tmp/bench-shots label
```

The cloud has no GPU. WebGL2 runs on SwiftShader, so frame times are only useful as relative costs. Headless WebGPU on SwiftShader loses its device at startup, so the WebGPU column comes from Bevy's own capability checks until a tester runs it on real hardware. Results are in `docs/graphics/feature-matrix.md`.
