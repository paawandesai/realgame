# Formula Bench VR

Our entry for the Meta VR Start Developer Competition 2026 (deadline
Nov 18, 2026, 12:00 PM PT): a seated, hands-only mixed-reality alchemy bench.
Pinch reagents off the shelf, drop them in the bowl, then press **Spark** (heat)
or **Zap** (electricity) to see what forms.

The chemistry is the same Rust rules engine as the main game: `crates/bench`
wraps `crates/sim` (sourced reactions only, atoms always conserved) and is
compiled to WebAssembly into `src/pkg/`. The 3D client is Meta's Immersive Web
SDK (WebXR, three.js), so judges open a URL in the Quest browser.

## Run it

Needs Node 22, Rust with the `wasm32-unknown-unknown` target, and `wasm-pack`.

```sh
cd vr
npm install
npm run dev        # builds the WASM, starts the dev server and an emulator window
```

On a Quest 3/3S on the same Wi-Fi: run `npm run dev:status`, open one of the
`runtimeUrls.network` addresses in the Quest browser, accept the local
certificate warning, and press **Start at my desk**. Sit at a desk; the bench
appears just in front of you, below eye level.

In a desktop browser the bench also works with the mouse: click a reagent to
send it to the bowl, click the buttons to react.

## Layout

- `src/bench.ts`: the bench (shelf, bowl, buttons, result board, effects)
- `src/chem.ts`: the WASM bridge, atom colours and formula labels
- `src/label.ts`: canvas text panels; `src/sound.ts`: synthesised cues
- `iwsdk.config.json`: XR mode (AR passthrough), hand tracking, pinch-to-grab
- `AGENTS.md`, `.claude/`: IWSDK's own reference docs for coding agents
