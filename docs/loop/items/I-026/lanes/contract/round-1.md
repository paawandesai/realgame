# I-026 contract lane, round 1

Builder: wave 10. Branch `loop/I-026` from origin/main 4c7c9a1 (b8fa548 plus the wave-10 loop-state commit; no code change between them). Commit `25efe87`.

## What landed

- `content/spells.json` (version 1), the four spells with the exact strings from the spec's table, `formula`, `tier_min` and `reaction` refs as specced (`co2` has `effect: foam`, `reaction: {register, C-035, cites}`).
- `crates/sim/Cargo.toml`: `[features] default = ["web"]`, `game = []`, `web = ["game", "dep:wasm-bindgen"]`; `wasm-bindgen` is `optional = true`.
- `crates/sim/src/lib.rs`: `game`, `grid`, `palette` behind `feature = "game"`, `web` behind `feature = "web"`; `spellbook` and `tier` always compiled.
- `crates/sim/src/tier.rs` (4-band `Tier`, complete) and `crates/sim/src/spellbook.rs` (all contract types, the full `impl Deserialize for Formula`; `parse` only deserialized in the stub).
- `crates/bevy-client/Cargo.toml` `[dependencies]`: `realgame-sim = { path = "../sim", default-features = false }`. No profile change.
- `crates/bevy-client/Cargo.lock`: adds `realgame-sim`, `serde_json 1.0.154`, `itoa 1.0.18`, `zmij 1.0.23` (resolved with `cargo metadata`, minimal; the same versions as the root lock). Root `Cargo.lock` unchanged (`--locked` passes).

## Checks on the stub

| check | result |
|---|---|
| `cargo test --locked -p realgame-sim` (root, default features) | 185 passed |
| `cargo test -p realgame-sim --no-default-features --locked` | 29 passed (the 2D game modules are compiled out) |
| `cargo check --locked` in `crates/bevy-client` | ok (one pre-existing native-only `dead_code` warning in `bridge.rs`, unchanged from main) |
| `cargo fmt --all --check`, `cargo fmt --check` (client) | clean |

## Follow-up (not in any lane's paths)

- CI's `bevy` paths filter (`ci.yml`, `changes` job) lists only `crates/bevy-client/**`. The client now also depends on `crates/sim/**` and `content/spells.json`, so a sim-only change no longer re-runs the client's fmt/chem/wasm-clippy job. `content/**` already triggers `npm test` (test 19) and the Rust jobs, so the content seam is covered; the gap is client wasm clippy after a sim API change. Suggest adding `crates/sim/**` and `content/**` to the `bevy` filter.
