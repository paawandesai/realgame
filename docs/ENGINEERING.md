# Engineering practices

How this repo stays safe, capable and efficient, and which published standard each practice comes from. The goal is a game children can use at school, so the bar is set by child-privacy law and by secure-development frameworks, not just by what's convenient.

## Standards we follow

| Standard | What it covers | How we use it |
| --- | --- | --- |
| [NIST SP 800-218, Secure Software Development Framework (SSDF) v1.1](https://nvlpubs.nist.gov/nistpubs/specialpublications/nist.sp.800-218.pdf) | Four practice groups: prepare the organisation, protect the software, produce well-secured software, respond to vulnerabilities | The checklist below is organised by these groups |
| [OWASP Application Security Verification Standard (ASVS) 5.0](https://softwaremill.com/whats-new-in-asvs-5-0/) | Testable security requirements for web apps, in three levels | Target Level 1 now; Level 2 before student accounts launch |
| [Amended COPPA Rule](https://bbbprograms.org/media/insights/blog/coppa-amended) (compliance date April 22, 2026) | Children's data in the US: written security program, retention policy, named third parties | Collect only anonymous play counts with no identifier that lasts beyond a day (`docs/ANALYTICS.md`), with a written 90-day retention policy; meet every remaining requirement before accounts launch |
| [National Data Privacy Agreement (NDPA)](https://fpf.org/blog/the-first-national-model-student-data-privacy-agreement-launches/) | Model contract US districts sign with edtech vendors | Design so we can sign it without exceptions |

## Practices, by SSDF group

### Prepare (PO)

- Security and privacy requirements live in this file and `SECURITY.md`, next to the code.
- Every reaction value has a cited source and a `reviewed_by` field; nothing reaches classrooms without an expert sign-off.

### Protect the software (PS)

- All changes go through pull requests with CI; `main` should have branch protection with required checks and one review (a repository setting).
- Dependencies are pinned by lockfiles (`Cargo.lock`, `web/package-lock.json`, and the root `package-lock.json` for repo tooling, including the exact `wrangler` version used to deploy) and installed with `--locked` / `npm ci`.
- Dependabot opens weekly update PRs for Cargo, npm and GitHub Actions.
- CI runs with read-only permissions.

### Produce well-secured software (PW)

- **Memory safety:** the simulation is Rust with `#![deny(unsafe_code)]`; only wasm-bindgen's generated glue is exempt.
- **Lints as errors:** `cargo clippy -D warnings`, `rustfmt`, and TypeScript `strict` with `noUncheckedIndexedAccess`.
- **Untrusted input:** formulas typed by players are length-capped, counts are capped and use saturating arithmetic, and a fuzz test throws 20,000 random strings at the parser.
- **Determinism:** the simulation uses a seeded RNG and fixed steps, with a test that two runs from the same seed match. That lets an authoritative server replay and check clients later.
- **Science regression tests:** enthalpies are re-derived from NIST formation values with Hess's law; stoichiometry tests check that 2 H2 + 1 O2 cells make exactly 2 H2O.
- **Gameplay regression tests:** the trial must stay completable, the fire must spread if ignored, and aiming water at it must put it out.
- **Minimal attack surface:** no framework, no third-party runtime code, no `innerHTML` (all text goes through `textContent`), and one kind of network call after load: the anonymous analytics events, same-origin, fire-and-forget, and skipped under Do Not Track or Global Privacy Control.
- **Browser hardening:** a strict Content Security Policy (`script-src 'self' 'wasm-unsafe-eval'`, no inline scripts, `frame-ancestors 'none'`) plus `nosniff`, `no-referrer`, HSTS and a locked-down Permissions-Policy, in `web/public/_headers`.
- **Supply-chain checks:** `cargo-deny` (RustSec advisories, licence allow-list, no unknown registries or git sources) and `npm audit` in CI.
- **Analytics data flow (`docs/ANALYTICS.md`):** the game posts at most a few events per session to same-origin `/api/event`, so the CSP keeps `connect-src 'self'`. A Cloudflare Pages Function accepts only four allow-listed event shapes (fixed event types, the three age bands, an integer room 0-15, fixed duration buckets, a 32-hex daily id), rejects cross-site posts and bodies over 512 bytes, caps each daily id at 120 events a day, and stores the UTC day, the daily id, the type, the band and the room or duration in D1, never an IP address, user agent, country or free text. The table repeats those rules as `CHECK` constraints. Rows older than 90 days are deleted on every write and every stats read. The dashboard reads only aggregates from `/api/stats`, which requires `DASHBOARD_TOKEN` (constant-time comparison, fails closed when unset).
- **Analytics tests (`npm test`, root):** `node:test` with no extra dependencies runs the real handlers and the real migration against `node:sqlite`: every malformed payload is rejected and not stored, the daily cap and the 90-day purge hold, the token check fails closed, the aggregates match hand-computed numbers, and the client's daily id rotates at UTC midnight and survives blocked storage. CI's `analytics` job also applies the migration to a local D1 with wrangler.

### Respond to vulnerabilities (RV)

- Private reporting path in `SECURITY.md`.
- Dependabot and `cargo-deny` flag newly disclosed advisories in dependencies.

## Efficiency budgets

School networks and low-end Chromebooks set these limits. CI enforces the first one.

| Budget | Limit | Today |
| --- | --- | --- |
| Total download | under 2 MB | about 300 KB raw, 115 KB gzipped |
| Simulation step | under 4 ms on a 4 GB Chromebook | 57,600 cells, three passes per step; to be measured on real devices |
| Memory | under 64 MB | grid state is about 1 MB |

## Child safety and privacy rules for future features

- No student emails, real names, photos, voice or location. Students join with a class code and a nickname.
- No chat with strangers. Any future chat is preset phrases or class-only, with moderation.
- No ads, no third-party analytics or trackers. Product analytics stay in our own database, are anonymous, and follow `docs/ANALYTICS.md`.
- Every new data field needs a written purpose and a deletion date before it ships.
