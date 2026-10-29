# Engineering practices

How this repo stays safe, capable and efficient, and which published standard each practice comes from. The goal is a game children can use at school, so the bar is set by child-privacy law and by secure-development frameworks, not just by what's convenient.

## Standards we follow

| Standard | What it covers | How we use it |
| --- | --- | --- |
| [NIST SP 800-218, Secure Software Development Framework (SSDF) v1.1](https://nvlpubs.nist.gov/nistpubs/specialpublications/nist.sp.800-218.pdf) | Four practice groups: prepare the organisation, protect the software, produce well-secured software, respond to vulnerabilities | The checklist below is organised by these groups |
| [OWASP Application Security Verification Standard (ASVS) 5.0](https://softwaremill.com/whats-new-in-asvs-5-0/) | Testable security requirements for web apps, in three levels | Target Level 1 now; Level 2 before student accounts launch |
| [Amended COPPA Rule](https://bbbprograms.org/media/insights/blog/coppa-amended) (compliance date April 22, 2026) | Children's data in the US: written security program, retention policy, named third parties | Collect nothing until accounts exist; then meet every requirement before launch |
| [National Data Privacy Agreement (NDPA)](https://fpf.org/blog/the-first-national-model-student-data-privacy-agreement-launches/) | Model contract US districts sign with edtech vendors | Design so we can sign it without exceptions |

## Practices, by SSDF group

### Prepare (PO)

- Security and privacy requirements live in this file and `SECURITY.md`, next to the code.
- Every reaction value has a cited source and a `reviewed_by` field; nothing reaches classrooms without an expert sign-off.

### Protect the software (PS)

- All changes go through pull requests with CI; `main` should have branch protection with required checks and one review (a repository setting).
- Dependencies are pinned by lockfiles (`Cargo.lock`, `web/package-lock.json`) and installed with `--locked` / `npm ci`.
- Dependabot opens weekly update PRs for Cargo, npm and GitHub Actions.
- CI runs with read-only permissions.

### Produce well-secured software (PW)

- **Memory safety:** the simulation is Rust with `#![deny(unsafe_code)]`; only wasm-bindgen's generated glue is exempt.
- **Lints as errors:** `cargo clippy -D warnings`, `rustfmt`, and TypeScript `strict` with `noUncheckedIndexedAccess`.
- **Untrusted input:** formulas typed by players are length-capped, counts are capped and use saturating arithmetic, and a fuzz test throws 20,000 random strings at the parser.
- **Determinism:** the simulation uses a seeded RNG and fixed steps, with a test that two runs from the same seed match. That lets an authoritative server replay and check clients later.
- **Science regression tests:** enthalpies are re-derived from NIST formation values with Hess's law; stoichiometry tests check that 2 H2 + 1 O2 cells make exactly 2 H2O.
- **Gameplay regression tests (`cargo test`):** a scripted run completes all four rooms on five seeds; each room's goal needs a player cause and accepts more than one solution; the hedge fire spreads within its fuel budget when ignored and goes out in three aimed water casts or fewer; pointing at a goal marker (as a naive player does) locks onto that goal's real target; every cast resolves to an outcome with an effect, and unbalanced casts run on the limiting reagent with atoms conserved.
- **Performance tests:** `bench_600_ticks` (the active region as played) and `bench_full_level_600_ticks` (all four rooms every tick) report ms/step in release builds; heat-pass sleeping is checked bit for bit against the plain pass on random scenes, edge cases and a scripted game.
- **Browser playtests (Playwright, run before merging gameplay changes; the scripts are throwaway and not shipped):** a naive player cold-starts each tier from the start screen and plays all four rooms with that tier's own controls (Sprout cards and the go arrow, Seedling tiles and the Cast button, Apprentice typing with the keyboard and pointing at goal markers), mixing words (`water`, `fire`, `boom`, `bubbles`), suggested spells, junk strings, wrong spells and mashing until the suit is empty. It records time to the first visible particles and the first sound, any miss shown without its icon, any visible `undefined`/`NaN`/error text, time per room and stuck states. Last run (2026-10-29), all three tiers: first particles and sound 60-200 ms after the first cast, no text-only misses, every room done in under 70 s (most under 20 s), trial complete. Fixes it drove: mouse-clicked HUD buttons now hand focus back to the game (Space was re-pressing Cast instead of jumping); typed spells that begin with a game key or a digit (`water`, `boom`, `2H2 + O2 -> 2H2O`) reach the spell box whole; the ice marker sits on the ice and markers lock onto their target. Also checked in the browser: `?gl=1` gets a WebGL context and draws the scene, a null `getContext('webgl')` or a lost context falls back to Canvas2D with no console errors, and frame times at 6x CPU throttle (see the budget table).
- **Minimal attack surface:** no framework, no third-party runtime code, no `innerHTML` (all text goes through `textContent`), and no network calls after load.
- **Browser hardening:** a strict Content Security Policy (`script-src 'self' 'wasm-unsafe-eval'`, no inline scripts, `frame-ancestors 'none'`) plus `nosniff`, `no-referrer`, HSTS and a locked-down Permissions-Policy, in `web/public/_headers`.
- **Supply-chain checks:** `cargo-deny` (RustSec advisories, licence allow-list, no unknown registries or git sources) and `npm audit` in CI.

### Respond to vulnerabilities (RV)

- Private reporting path in `SECURITY.md`.
- Dependabot and `cargo-deny` flag newly disclosed advisories in dependencies.

## Efficiency budgets

School networks and low-end Chromebooks set these limits. CI enforces the first one.

| Budget | Limit | Today (measured 2026-10-29) |
| --- | --- | --- |
| Total download | under 2 MB | `web/dist` 539,504 bytes raw (`du -b`), about 175 KB gzipped; the wasm module is 432 KB raw / 149 KB gzipped. `wasm-opt` stays off because `binaryen` is not installed on build machines and is never downloaded at build time |
| Simulation step (native, release) | under 2 ms | `cargo test --release bench_ -- --nocapture`: active region as played (`bench_600_ticks`) about 0.5 ms/step; worst case with all four rooms stepped every tick while the hedge burns and spells land (`bench_full_level_600_ticks`) 1.5-2.0 ms/step, depending on machine load |
| Frame work at 6x CPU throttle | under 16 ms average | Playwright + CDP `Emulation.setCPUThrottlingRate(6)`, 10 s with the hedge fire burning, 1280 x 800 headless Chromium: 13.4-14.1 ms of script work per frame on all three tiers (sim about 8-9 ms, world render about 2.4 ms, overlay under 0.5 ms, HUD about 1.8 ms). Headless Chromium rasterises canvases in software, so rAF intervals there are 23-26 ms; on GPU-composited devices that cost moves off the main thread |
| WebGL renderer (`?gl=1`, opt-in) | not slower than Canvas2D | Not met in headless Chromium, where WebGL runs on SwiftShader (CPU) and every frame is read back for software compositing: at 6x throttle about 20 ms work per frame against 14-16 ms for Canvas2D, and 1.0 against 0.4 ms render at 1x. So Canvas2D stays the default; WebGL is a flag until it can be measured on real GPUs (school Chromebooks). It falls back to Canvas2D, with no console errors, when `getContext('webgl')` returns null, a shader fails, or the context is lost mid-game |
| Memory | under 64 MB | grid state about 4 MB (1280 x 180 cells, plus the heat-sleep snapshot) |

How the step budget is met: fixed 60 Hz steps over an active region of 640 columns around the camera; static species property tables; heat-pass sleeping (64-cell row segments whose inputs are bit-for-bit unchanged since a step in which they changed nothing are skipped; checked against a snapshot so no write can be missed, and tested to give bit-identical results); and the overlay draws glow on its own quarter-resolution canvas, caps its backing store at 4 px per cell, and is not cleared or redrawn while idle.

## Child safety and privacy rules for future features

- No student emails, real names, photos, voice or location. Students join with a class code and a nickname.
- No chat with strangers. Any future chat is preset phrases or class-only, with moderation.
- No ads, no third-party analytics or trackers. Product analytics stay in our own database.
- Every new data field needs a written purpose and a deletion date before it ships.
