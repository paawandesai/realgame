# Formula (RealGame) handoff guide

As of 2026-11-26. The living copy of this guide is a Claude Doc: https://claude.ai/artifact/GVqisQxyYgjVSzkCjQPw5P

## Start here

Formula is a browser game where real chemistry is the magic: players build balanced equations at a Formula Bench, practise them into spells, and the game's Rust simulation decides what each spell does. It is the first "world" of RealGame, a planned learning MMO where each science is a discipline (Formula = chemistry, Runes = circuits). A playable build is live at [realgame.pages.dev](https://realgame.pages.dev), and all code is in [github.com/paawandesai/realgame](https://github.com/paawandesai/realgame).

This guide is for a chemical engineer taking the project over. You do not need to code to own the chemistry: the reactions, sources and age bands live in plain JSON and Markdown files that a reviewer can read and edit. Changing game behaviour does need Rust or TypeScript, so pair with a developer or with Claude (see "Working with Claude" in the access checklist).

**Your first week**

1. Play the live game for 15 minutes, then open the free-play Chemistry Bench at [realgame.pages.dev/bench-free.html](https://realgame.pages.dev/bench-free.html) and try reactions at each age band.
2. Read `content/chem/core.json` (the 182 reactions players can make) and `content/science-sources.md` (every science sentence the game shows, with its source and verdict).
3. Read the source audit on branch `loop/bench-play` (`docs/loop/items/I-058/spec.md`): it lists 13 reactions where a teacher or chemist should rule, and 50 whose sources were not fully checked.
4. Decide whether to bring the larger reaction set from that branch onto the live game (section "Open science work").
5. Get the accesses in the checklist below, so you can review, merge and deploy without Paawan.

## Vision and goals

The near-term goal is 10,000 school students playing, reached through Paawan's teacher contacts across grades and subjects; the long-term goal is a global game used from school through PhD level. The original target date for 10,000 was about 2026-11-19; current player counts are on the private analytics dashboard at realgame.pages.dev/dashboard/.

- **The Formula Bench is the centrepiece.** Everything is built around it: players learn a reaction at the Bench, and two correct builds turn it into a spell they can cast instantly. Wrong builds do not count; a correct build after a hint does.
- **Learn, master, then invent.** Players first learn known reactions for their age band, then master them through repetition, then try combinations of their own. Older students see a discovery list and can propose novel reactions, which the engine checks but never calls real.
- **Accuracy is a promise.** Every reaction and every science sentence a player sees must cite a real source, or it is hidden.
- **The open world is the demo.** A short 3D adventure (a cold Kiln to relight across a village, frozen ford, marsh and crystal cave) shows chemistry acting on a world. The Bench is what the MMO is built around.
- **Order of work Paawan set:** puzzles and rescues for the beta, then creatures, then a shared-world co-op MMO. The beta uses class codes and nicknames (no student personal data); sign-in for anyone comes later, then a push into schools.
- **Spin-off:** Formula Bench VR, a hand-tracked seated Bench for Meta Quest that reuses the same Rust engine, built for Meta's VR Start competition (deadline was 2026-11-18).

## How the chemistry works

The chemistry is plain data files checked by a Rust engine. A player's equation is parsed into atoms, compared with the reaction list, and given one of six outcomes; separate world rules then decide what a spell does to things it hits (heat, fire, ice, gases). Rust runs in the browser as WebAssembly, and the same crates are meant to run on future game servers and in the VR Bench.

**What happens when a player runs the Bench** (`crates/chem/src/lib.rs`, `Codex::judge`):

| Outcome | When | What the player sees |
| --- | --- | --- |
| Known | A listed reaction at or below their age band, with every condition supplied | The reaction runs, with its plain-language line |
| Needs condition | Listed, but missing heat, spark, light, electricity, catalyst, pressure or water | Which condition is missing |
| Locked | A real listed reaction taught at a higher band | "You'll learn this later" |
| Unbalanced | Atoms or charge don't match | Which atoms are off, and the nearest known reaction |
| Mystery | Balanced, not listed, and a simple model thinks the products could exist | "Predicted by the model", never "real" |
| No reaction | Balanced, but a product fails the model, or nothing changes | The reason |

The mystery model (`crates/chem/src/predict.rs`) is deliberately simple: a product passes if it is a known species, an element in its real form (O2, P4, a metal), a metal compound whose common oxidation states can sum to zero, or a non-metal molecule whose usual bond counts can connect it. It does not estimate energy or kinetics, so it says "might hold together", never "will react". This is a good place for a chemical engineer to add thermodynamics.

**Where the chemistry lives**

| File | What it holds | Edit without coding? |
| --- | --- | --- |
| `content/chem/core.json` | The 182 playable reactions on the live game: equation, family (18, such as combustion or precipitation), lowest age band, conditions needed, heat released or absorbed, hazard, kid-friendly lines, source, review status | Yes |
| `content/chem/guide.json` | Bench lessons, walkthroughs, hints and challenges per age band, each tied to reactions and an NGSS standard | Yes |
| `content/chem/names.json` | Element data and everyday names for every substance | Yes |
| `content/chem/literature.json` | "Lab records": 8,338 real published reactions from the Open Reaction Database, indexed so a player's equation can be matched to one (CC BY-SA 4.0) | Rebuilt by script |
| `content/materials.json` | 27 world materials with sourced density, melting and boiling points, heat capacity, conductivity, ignition temperature | Yes |
| `content/spells.json` | The four world spells (Fire, Water, Frost, CO2) and their Bench prompts per band | Yes |
| `content/science-sources.md` | The register of every science sentence the 3D game shows (106 claims, 38 numeric parameters) with verdict and source | Yes |
| `crates/chem` | The engine above: reaction list, judging, mystery model, lab-record lookup | Rust |
| `crates/sim` | Formula parser, age-band tiers, the original 2D game's simulation | Rust |
| `crates/matter` | Heat, fire, ice/water/steam, volume-conserving water, a coarse gas field (CO2 pools, H2 rises), balloon lift | Rust |
| `crates/bevy-client/src/chem.rs` | The 3D world's per-object rules: temperature, burning, fuel, wetness, ice, CO2 blanket, stepped 30 times a second | Rust |

**The chemdb reaction database.** `tools/chemdb` builds a reference set of 1,512,217 unique reactions from the Open Reaction Database (all 93 datasets, built 2026-11-04 with RDKit). Each record has Hill-order formulas, reaction SMILES, conditions (temperature, catalyst, solvent) and a balanced flag; only 28,864 (1.9 %) balance, because most literature records list only the main product. The data files (98 MB and 7.6 MB, gzipped) are too large for git and sit in the project's shared folder under `realgame/chemdb/`; `tools/chemdb/README.md` covers the schema and licence. The importer for Daniel Lowe's CC0 USPTO patent set is written but has never run, because the build machine could not reach figshare. ORD data is share-alike (CC BY-SA 4.0), so any redistributed copy must keep that licence and attribution.

Tests guard the science: `cargo test` checks every reaction balances and loads, and `npm test` fails if the game shows a science-looking sentence with no row in `science-sources.md`.

## Open science work

The biggest open job for a chemist is a human review: no reaction or claim in the game has yet been checked by a credentialed chemist or teacher. All checks so far were done by independent AI reviewers against open sources, so the honest wording today is "sourced and checked by an automated audit", not "expert-verified".

**Two versions of the chemistry exist.** The live game (branch `main`) has the smaller set; the larger, audited, re-banded set is on branch `loop/bench-play` and was rolled back from the live site on 2026-11-06 because its one-Bench interface was too heavy for browsers, not because of the chemistry.

| | Live game (`main`) | Branch `loop/bench-play` |
| --- | --- | --- |
| Playable reactions | 182 hand-written (core) | 486: 182 core + 304 generated from rules (solubility table, reactivity series, fuels) |
| Bench-only reactions | none | 30 toxic-gas reactions, never cast in the world |
| Evidence per reaction | a `source` line | `evidence` field (kind, source, URL, quoted claim, checked flag) plus `content/chem/evidence.csv` |
| Bands | by age, before the grade decision | by school grade (below) |
| Gate | none | a reaction with no evidence, or `unverified`, is hidden on load |

**Source audit (item I-058, on the branch).** All 486 reactions cite an open source (OpenStax, Wikipedia, exam-board practical guides). 436 were checked against the fetched page on 2026-11-05; 50 rely on a general rule or an analogue and are marked `checked: false`. Three visible errors were fixed. To do:

- [ ] Rule on the 13 known simplifications, for example alumina electrolysis shown giving O2 instead of CO2 at carbon anodes, "CuCO3" standing for basic copper carbonate, rust as anhydrous Fe2O3, precipitate colours (AgBr cream vs pale yellow, Fe(OH)2 green vs white).
- [ ] Confirm the 50 unchecked reactions (44 core, mostly neutralisations and combustions of close relatives; 6 rule-generated).
- [ ] Re-open a sample of the checked URLs yourself, one per family, since the AI fetcher returned a reading of each page rather than the page.
- [ ] Check what the audit did not cover: flame colours, speeds, the "feels" lines, colours cited to Greenwood and Earnshaw, and flash points that decide spark versus heat for fuels.
- [ ] Review the 3D world's register, `content/science-sources.md`: 106 claims and 38 parameters, several marked `simplified` (for example "CO2 blanket, no O2" is a limiting case; CO2 mainly works by absorbing heat). The review procedure is `docs/loop/science-review.md`.
- [ ] Record your review on each entry you check (the `check` field in core.json; `reviewed_by` in the older `content/reactions.json`).

**Age bands (item I-059, on the branch).** Paawan decided on 2026-11-05 that bands follow US school grades 6 to 12, because ages 4 to 7 were too young for the content. The band ids stayed the same, so saved progress and analytics keep working.

| Band | Grades | Standards | Reactions (core + rule) | What the Bench shows |
| --- | --- | --- | --- | --- |
| Sprout | 6–7 | NGSS MS-PS1-1, 2, 5 | 12 | Formulas and everyday names, word equations, molecules as models; equations shown balanced, never balanced by hand |
| Seedling | 8–9 | NGSS MS-PS1-1 to 6, HS-PS1-2, 7 | 33 (26 + 7) | Balance scale with help; synthesis, decomposition, combustion, displacement; warm or cold |
| Apprentice | 10–11 | NGSS HS-PS1-1, 2, 4, 6, 7 | 149 (77 + 72) | Full balancing, solubility rules, reactivity series, conditions, states on the result card |
| Adept | 12+ (AP, college, adults) | HS-PS1 plus AP Chemistry topics | 292 (67 + 225) | ΔH in kJ, flame temperature, gas densities, redox, lab records, later inventions |

Each band also plays everything below it. Open questions on bands: the 3D world still offers only three choices (Adept is read as Apprentice), and adding a fourth changes what analytics collects, so it needs a decision. Paawan also asked that bands be re-checked after the source audit; the 30 moves made are listed in `docs/loop/items/I-059/spec.md` (on the branch) with a reason each.

**Other open science items**

- The Bench design has 21 open questions with suggested defaults, awaiting answers: [bench questions](https://claude.ai/artifact/NPRQ2mCoFtba7GAswKMZa9).
- The Lowe USPTO patent reactions (CC0) were never imported; once figshare is reachable, `tools/chemdb` can rebuild without share-alike terms.
- Inventions for older students: the mystery model has no thermodynamics yet. Adding ΔG from formation data would let the game say whether a novel combination is favourable.

## How to run, build and deploy

You can play and review without installing anything: the live site is [realgame.pages.dev](https://realgame.pages.dev), and any branch can be deployed as a preview at its own address. Building yourself needs Rust, Node 22 and two WebAssembly tools; the first 3D build takes 5 to 10 minutes.

**What the site serves**

| Address | What it is |
| --- | --- |
| `/` | The 3D game (Rust + Bevy compiled to WebAssembly, about 3.5 MB download) |
| `/bench-free.html` | The free-play Chemistry Bench |
| `/classic/` | The original 2D game, "Alchemist's Trial" |
| `/dashboard/` | Anonymous play counts by age band, behind a password (`DASHBOARD_TOKEN`) |

**Deploy (no install needed).** On GitHub: Actions, then Deploy, then Run workflow. `production` only accepts commits on `main`, so only merged, approved work goes live; run it again with an older commit to roll back. `preview` deploys any branch to `https://<branch>.realgame.pages.dev`, with analytics off. The steps are in `docs/DEPLOY.md`.

**Build on your own computer**

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm ci
npm run build:site                    # everything into web/dist
python3 -m http.server 8080 --directory web/dist
```

Then open `http://localhost:8080/`. For the 3D game alone: `cd crates/bevy-client && ./build-web.sh`, which writes `web/pkg-out/`.

**Checks.** `cargo test` (Rust, including the chemistry tests) and `npm test` (analytics, science-register lint) from the repository root. Git hooks run quick checks on commit and the full set on push; GitHub runs the same plus dependency audits on every pull request (`.github/workflows/ci.yml`).

**Analytics and privacy.** The game records only anonymous events (session start with age band, quest complete, session end) in a Cloudflare D1 database, honours Do Not Track, stores no IP address or user agent, and deletes events after 90 days. Details in `docs/ANALYTICS.md`. Everything fits Cloudflare's free plan.

**Working rules Paawan set.** Run builds and tests in the cloud, not on someone's laptop while they use it. Merge to `main` only after a person has played the build. Keep fast checks local and CI lean.

## Key decisions so far

The direction moved from a 2D sandbox to a 3D open world to a game built around the Formula Bench, each time because of how Paawan felt playing it. Newest first:

| Date (2026) | Decision | Why |
| --- | --- | --- |
| Nov 6 | Live site rolled back to the lighter Nov 5 build; the realistic look and the single combined Bench stay on branches | The new build was too heavy for the browser |
| Nov 6 | Two correct builds make a spell instant; wrong builds don't count, a correct build after a hint does | Mastery through repetition without punishing help |
| Nov 5 | Age bands follow grades 6 to 12 (Sprout, Seedling, Apprentice, Adept) | Earlier age bands put too-hard content in front of young children |
| Nov 5 | Every playable reaction must cite a real source or be hidden | Accuracy is the game's promise to schools |
| Nov 5 | Live at realgame.pages.dev; deploys are a manual button, production only from `main` | Only reviewed work goes live |
| Nov 4 | The Formula Bench is the core of the MMO; the open world is a demo | Paawan liked the Bench mock-up most |
| Nov 4 | World should look and behave real (references: Noita and Powder Toy for matter, Teardown for fire and smoke, Half-Life: Alyx for Bench detail); graphics upgrade comes after verifying the current build | Realism is the hook; performance comes first |
| Nov 4 | Merge only after Paawan plays and verifies | Paawan judges progress by playing, so a person signs off before anything reaches main |
| Nov 3 | Keep Bevy; mining dropped; voiceovers off by default; making charcoal as a puzzle step | Bevy suits realistic physics; no engine switch before the deadline |
| Nov 2 | Block world dropped for smooth ground; Minecraft was only a reference for "open world" | It looked like a bad Minecraft copy and gave no reason to learn spells |
| Nov 1 | Open world with dungeons and quests, solo or in teams; craft, master, then invent reactions; Unreal rejected | Levels felt too simple; Unreal costs browser size and time |
| Nov 1 | Naming: each science is a discipline (Formula = chemistry, Runes = circuits); title "Formula, a RealGame world" | One umbrella, many sciences |
| Oct 30 | Not just a shooter: learn spells at the Bench by age band; order is puzzles and rescues, creatures, then co-op MMO; class codes for beta | Learning chemistry, not aiming, should win levels |
| Oct 30 | Build Formula Bench VR for Meta's competition in parallel | Paawan has a Quest and joined Meta Start |
| Oct 29 | Switch from 2D to true first-person 3D in Bevy | The 2D game converted to 3D felt like a flat diorama |
| Oct 29 | Stack: Rust simulation to WebAssembly, TypeScript UI, Luau later for player scripting; Cloudflare Pages hosting | One chemistry engine for browser, servers and VR |
| Oct 28 | Concept: an MMO where real skills are magic, school through PhD, science only, sci-fi cultivation tone | Paawan's Game Week idea |

The game also started as a Game Week course assignment (Option 1, Exploration / Adventure), submitted in early November; `docs/submission.md` is that write-up.

## Access checklist

Paawan does these once, before the handover; the new owner then needs no one else to review, merge or deploy.

**GitHub (code and deploys)**

- [ ] Add the new owner as a collaborator on [paawandesai/realgame](https://github.com/paawandesai/realgame/settings/access) with Maintain or Admin, or transfer the repository to them (Settings, then Transfer ownership).
- [ ] If transferring, the deploy secrets (`CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`, `DASHBOARD_TOKEN`) move with it, but should be replaced with tokens from the new owner's Cloudflare account.
- [ ] Confirm they can run Actions, then Deploy.

**Cloudflare (the live site and analytics)**

- [ ] Invite them to the Cloudflare account that hosts the `realgame` Pages project and the `realgame-analytics` D1 database (Manage account, then Members), or have them create their own account and redeploy there, which creates a new project and database on first run.
- [ ] Hand over the dashboard password (`DASHBOARD_TOKEN`) through a password manager, never in chat or email.

**Files outside the repository**

- [ ] The project's shared folder: the chemdb data (`realgame/chemdb/`, about 105 MB), screenshots, playtest notes, research on game assets. Download it and share it through Drive or similar.
- [ ] Note the share-alike licence on the chemdb files (CC BY-SA 4.0) when sharing them.

**Design documents (Claude artifacts)**, each private until Paawan shares it, or exports it to PDF or Markdown:

- [ ] Share or export each link in the next section.

**Working with Claude**

- [ ] The Claude project this was built in is private to Paawan and cannot add members. The new owner can start their own Claude project on the repository and point it at this guide, so their Claude starts with the same context.

## Where everything lives

**Repository map.** The top-level `README.md` still describes the older 2D game; this guide and `docs/submission.md` describe the current one.

| Path | What it is |
| --- | --- |
| `content/` | All chemistry and science data (see "How the chemistry works") |
| `crates/chem`, `crates/sim`, `crates/matter`, `crates/voxel` | The Rust engines: reactions, formulas and tiers, world physics and chemistry, the terrain |
| `crates/bevy-client` | The 3D game; `web/` inside it holds the page, HUD and the Bench pages (`bench.js`, `bench-free.html`) |
| `tools/chemdb` | Python scripts that build the reaction database from ORD |
| `web/` | The original 2D game |
| `functions/`, `migrations/`, `dashboard/` | Anonymous analytics: Cloudflare Functions, database schema, dashboard page |
| `docs/` | Deploy, analytics, engineering practice, art bible, submission write-up |
| `docs/loop/` | Quality bars, the science-review protocol, the backlog, and a spec for every work item (`items/I-NNN`) |

**Branches worth knowing.** `main` is the live game. `loop/bench-play` holds the 486-reaction audited set, the grade bands, the single combined Bench and the realistic look, all rolled back from the live site on 2026-11-06 for performance. Other `loop/I-NNN` and `claude/*` branches are older work items.

**Design documents (Claude artifacts)**

| Document | What it covers |
| --- | --- |
| [Formula concept doc](https://claude.ai/artifact/CDHLYY1LHMfiQjW7Pq39ZA) | The original MMO idea, tone and stack |
| [Design doc](https://claude.ai/artifact/Kg2m9K27rvzV8jt9umgaTt) | The open-world design and shared Rust crates for browser, servers and VR |
| [Re-plan](https://claude.ai/artifact/WNHo4L6s1XdkZfrJKHG2q8) | The Nov 1 block-world plan (later replaced by smooth ground) |
| [Smoothing plan](https://claude.ai/artifact/5gfJKDvJW3PaWxzL7bnbKf) | Smooth-ground world, puzzle steps, controls |
| [Submission plan](https://claude.ai/artifact/V312nSQ6i8T5w7b4SsPqoy) | The Game Week submission bars and items |
| [Bench mock-up](https://claude.ai/artifact/FbdMvtUqquCCTveBF6Z66B) | The Bench look Paawan liked |
| [Chemistry Bench plan](https://claude.ai/artifact/MsZ7ZESZ3LjFCQBYfXKSbx) | Reaction database, free play, inventions (approved) |
| [Bench: age bands, guide and integration](https://claude.ai/artifact/HgkAyYebMiTYeA3VZC1SmL) | Bands, lessons, how the Bench joins the world (approved) |
| [Bench: free play to spells](https://claude.ai/artifact/ARwyByPYQG5Lp6e32JGc1a) | Discoveries become spells (approved) |
| [Bench questions](https://claude.ai/artifact/NPRQ2mCoFtba7GAswKMZa9) | 21 open questions with defaults, unanswered |
| [Graphics plan](https://claude.ai/artifact/CVmLpTpyZDSEUQmF23bGwz) | Realism upgrade (approved, after the current build is verified) |
| [World look sketches](https://claude.ai/artifact/QG3fcrHJcEmmAobRLpNDxF) | Open-world look options, awaiting a choice |
| [Formula Bench VR plan](https://claude.ai/artifact/VHxxxjnpZsoaVTyZBanfBQ) | The Meta Quest spin-off |

**Project shared folder (`realgame/`)**: `chemdb/` (the 1.5 million-reaction database and its README), `3D_PLAN.md` and `3D_SLICE_DESIGN.md` (the Oct 29 engine decision and 3D design), `art-bible.md`, `submission.md`, `playtest/final-walkthrough.md` (a step-by-step play check), `research/` (a lecture on game assets), and screenshots under `previews/`.
