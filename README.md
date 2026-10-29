# Alchemist's Trial

A chemistry spell game, and the first playable slice of a larger game where real skills are the magic system. You walk through four rooms and cast real chemical equations as spells: each one flies as a bolt, hits, and the simulation decides what happens. Water douses a burning hedge, a balanced `2H2 + O2 -> 2H2O` melts an ice wall, electricity splits water so hydrogen rises under a stone roof to be lit, and carbon dioxide (denser than air) smothers the last fire before you light the brazier.

It runs in the browser (school Chromebooks first), downloads about 280 KB gzipped, and collects no personal data: only anonymous play counts, described in `docs/ANALYTICS.md`.

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

## Deploy

`npx wrangler login`, then `npm run deploy` publishes the game to Cloudflare Pages with its analytics. Step-by-step instructions, including building from GitHub instead, are in `docs/DEPLOY.md`.

## How to play

Pick an age band on the first screen (you can change it any time from the gear menu). Your suit already carries water, hydrogen and oxygen, so the first spell works straight away: no gathering first.

1. **Splash Garden.** A hedge is burning in front of the gate. Cast water (`H2O`, or just `water`) at it until it is out.
2. **Frozen Gate.** An ice wall blocks the way. A balanced `2H2 + O2 -> 2H2O` (`boom`) melts it; the meltwater fills the pit.
3. **Crystal Lab.** Cast water at the crystal pool (or `split`): electricity splits it 2:1 into hydrogen and oxygen. Hydrogen is lighter than air and collects under the stone roof; light it with boom to blow the door open.
4. **Marsh Door.** Methane bubbles from the marsh. Burn it (`CH4 + 2O2 -> CO2 + 2H2O`, `fire`) to capture CO2, put out the hedge with water or a CO2 `cloud`, then light the brazier by the door.

Every room shows a goal marker and two or three short steps. If you get stuck, Mote's hints get more specific the longer you wait. After the last door you can replay, change age, or open free play (all rooms and spells, with seeded challenges).

| Action | Keyboard and mouse | Touch |
| --- | --- | --- |
| Move, jump | `A` `D` or arrows, `Space` | On-screen arrows, Jump |
| Aim | Mouse; `T` aims at the goal; `Shift` + arrows nudge | Drag or tap the world |
| Cast | `Enter`, or the Cast button | Cast button (Sprout: tap a picture card) |
| Type a spell (Apprentice) | Just start typing, then `Enter`; `Up` recalls the last one, `Tab` cycles | Spell box |
| Saved spells | `1` to `9` | Slot buttons |
| Gather from the nearest source | `G` | Gather button |
| Formula overlay (Seedling, Apprentice) | `Q` | Build-a-spell button |
| Heat lens (see temperature) | `L` | Lens button |
| Grimoire (spells you found) | `B` or `N` | Grimoire button |
| Close overlays | `Esc` | Close buttons |

The tiers:

- **Sprout (4 to 7):** picture cards, everything read aloud, and Mote balances equations for you. Tap a card and it flies at the goal.
- **Seedling (8 to 10):** symbol tiles and a balance scale; each room's spell is loaded into the bar when you arrive. An unbalanced spell sputters weakly and offers a Fix it that explains first.
- **Apprentice (11 and up):** type formulas or words. You balance it yourself, and casts show real enthalpy values (ΔH) with their source.

The gear menu has sound, voice, reduced motion, game speed (0.5x, 0.75x, 1x), restart room and change age.

## What's new in v2

v1 was a single 320-cell strip where most spells only produced a text error until you had gathered the right matter. v2 rebuilt the game around "every cast does something you can see":

- Spells fly as bolts and react where they land, with particles, glow, screen shake and synthesised sound scaled by the real ΔH.
- The suit starts charged and refills itself (condenser for water, electrolyser for hydrogen and oxygen), so casting never starts with an empty jar.
- Four rooms (1280 cells wide) with a following camera, a bigger and faster player with coyote time and jump buffering, and a fixed 60 Hz simulation.
- Goals are physical (fire out, ice gone, door open), shown as markers and ticked steps instead of a five-item text list, with a hint ladder for each room.
- Three tiers that really differ: pictures and voice, tiles with a one-tap fix, or typed equations with sourced energies.
- Honest feedback: only sourced reactions are called real chemistry, unbalanced spells say exactly which atoms do not match, and messages only credit causes the player produced.
- Grimoire, end screen, free play with seeded challenges, touch and small-screen layouts, and an optional WebGL renderer (`?gl=1`).

The full design and what was built is in [docs/DESIGN_V2.md](docs/DESIGN_V2.md).

## Layout

| Path | What it is |
| --- | --- |
| `crates/sim` | The simulation in Rust: formula parser, reaction table, matter grid, game rules, browser bindings |
| `content/reactions.json` | Every reaction and phase change, with sourced values. Data, not code, so it can be reviewed |
| `content/known_reactions.json` | Real, sourced reactions outside this trial, so the game can say "real chemistry, not here yet" |
| `web` | The browser shell in TypeScript: input, HUD, formula ring. No framework, no third-party runtime code |
| `docs/DESIGN_V2.md` | The v2 design and how it was built |
| `docs/ENGINEERING.md` | The engineering and safety practices this repo follows, and why |
| `functions/`, `migrations/` | Cloudflare Pages Functions for anonymous play counts (`/api/event`, `/api/stats`) and their D1 schema |
| `dashboard/` | The static analytics dashboard served at `/dashboard/` |
| `docs/DEPLOY.md`, `docs/ANALYTICS.md` | How to deploy to Cloudflare, and exactly what the analytics collect |

## Science and accuracy

All enthalpies come from standard enthalpies of formation in the NIST Chemistry WebBook, and latent heats from the CRC Handbook. A unit test re-derives each one with Hess's law, so a typo in the content file fails the build. Where the game simplifies (wood is treated as pure carbon, flame temperatures are capped at real adiabatic values), the code says so next to the rule. No content entry has been reviewed by a credentialed expert yet; `reviewed_by` is `null` until one is.

## Checks before you push

Running `npm install` once at the repository root sets up git hooks (husky):

- **pre-commit** (about 1 s): `rustfmt --check` on staged Rust files, and `tsc` when web TypeScript, or the Functions, tests or dashboard script, are staged.
- **pre-push** (about 10 s warm, under a minute from cold): rustfmt, clippy for native and wasm32, `cargo test`, the analytics typecheck and tests (`npm test`), the WebAssembly build, `tsc` and the Vite build.

If a hook fails, fix the problem rather than bypassing it; CI runs the same checks plus the dependency audits.
