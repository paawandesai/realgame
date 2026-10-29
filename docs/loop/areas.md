# Areas

| area | paths | notes |
|------|-------|-------|
| sim | `crates/sim/**` | deterministic Rust rules engine (chemistry state, reactions); no Bevy dependency |
| client-core | `crates/bevy-client/src/main.rs`, `crates/bevy-client/src/player.rs`, `crates/bevy-client/src/bridge.rs` | first-person controller, camera, input, app wiring |
| client-world | `crates/bevy-client/src/world.rs`, `crates/bevy-client/src/chem.rs` | courtyard, targets, quests, in-world chemistry state |
| client-fx | `crates/bevy-client/src/fx.rs` | particles, VFX, audio |
| client-web | `crates/bevy-client/web/**`, `crates/bevy-client/build-web.sh` | HTML shell, HUD overlay, boot, CSP |
| content | `content/**` | spells, quests, science sources (RON/markdown) |
| analytics | `functions/**`, `dashboard/**`, `migrations/**`, `test/**` | Cloudflare Pages Functions + D1 |
| tests-e2e | `tests/e2e/**` | smoke/golden path, perf scripts (tester-owned) |
| infra | `.github/**`, `scripts/**`, `wrangler.toml` | CI, deploy |
| legacy-2d | `web/**` | retired 2D game; touch only to remove or redirect |

## Shared (one owner per item: the contract step, or one named lane)
- `Cargo.toml`, `Cargo.lock`, `crates/bevy-client/Cargo.toml`, `crates/bevy-client/Cargo.lock`
- `package.json`, `package-lock.json`, `tsconfig.json`, `.gitignore`, `deny.toml`
- `crates/bevy-client/src/lib.rs` (module list) if one exists
