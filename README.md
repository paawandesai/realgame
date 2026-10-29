# Alchemist's Trial

A chemistry spell sandbox, and the first playable slice of a larger game where real skills are the magic system. You gather matter, write real chemical equations, and the simulation decides what happens: water douses fire, electricity splits water into hydrogen and oxygen, and a balanced `2H2 + O2 -> 2H2O` makes a sharp pop that lights a torch.

It runs in the browser (school Chromebooks first), downloads about 115 KB gzipped, and collects no personal data.

## Run it locally

You need Rust (stable), the `wasm32-unknown-unknown` target, `wasm-pack`, and Node 22.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
cd web
npm ci
npm run dev        # builds the Rust simulation to WebAssembly, then serves http://127.0.0.1:5173
```

Run the simulation's tests from the repository root:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

## How to play

| Action | Keyboard and mouse | Touch |
| --- | --- | --- |
| Move, jump | `A` `D`, `Space` | On-screen arrows, Jump |
| Aim | Mouse | Tap the world |
| Gather what's under the cursor | Hold right-click or `G` | Hold Gather |
| Open the formula ring (time slows) | `Q` | Ring |
| Cast | Left-click, or `Enter` in the ring | Cast button |
| Saved spells | `1` to `5` | Slot buttons |
| Insight lens (see temperature) | Hold `Tab` | Lens button |
| Lab notebook | `N` | Notebook button |

The first screen asks for an age band, which sets the tier:

- **Sprout (4 to 7):** picture tiles, everything read aloud, and Mote balances equations for you.
- **Seedling (8 to 10):** symbols; an unbalanced equation fizzles harmlessly and offers a one-tap fix.
- **Apprentice (11 and up):** you balance it yourself, and casts show real enthalpy values with their source.

## Layout

| Path | What it is |
| --- | --- |
| `crates/sim` | The simulation in Rust: formula parser, reaction table, matter grid, game rules, browser bindings |
| `content/reactions.json` | Every reaction and phase change, with sourced values. Data, not code, so it can be reviewed |
| `web` | The browser shell in TypeScript: input, HUD, formula ring. No framework, no third-party runtime code |
| `docs/ENGINEERING.md` | The engineering and safety practices this repo follows, and why |

## Science and accuracy

All enthalpies come from standard enthalpies of formation in the NIST Chemistry WebBook, and latent heats from the CRC Handbook. A unit test re-derives each one with Hess's law, so a typo in the content file fails the build. Where the game simplifies (wood is treated as pure carbon, flame temperatures are capped at real adiabatic values), the code says so next to the rule. No content entry has been reviewed by a credentialed expert yet; `reviewed_by` is `null` until one is.

## Checks before you push

Running `npm install` once at the repository root sets up git hooks (husky):

- **pre-commit** (about 1 s): `rustfmt --check` on staged Rust files, and `tsc` when web TypeScript is staged.
- **pre-push** (about 10 s warm, under a minute from cold): rustfmt, clippy for native and wasm32, `cargo test`, the WebAssembly build, `tsc` and the Vite build.

If a hook fails, fix the problem rather than bypassing it; CI runs the same checks plus the dependency audits.
