# I-026: Spells as data (phase 1)

**based_on:** origin/main c4832d2e86c2408443124960b130fc6208a89aab (2026-10-30, merge of PR #31, roadmap v2; code state as at a0e2ecb plus I-019). All facts below were read with `git show c4832d2:<path>`.

**Revision 2** (after `critique.md`; see its "Planner response"). Changes: `chem.rs` methods renamed to `effect_id`/`from_effect_id` and a `Spell::spell_id()` added, so I-027 can never save `foam`; the `smothers` role is dropped and CO2 is traced to its sourced register row (`kind: register`, role `cites`) instead of `carbon_combustion`; B10 wording narrowed; the effect-id seam gets a Node check; the perf gate is report-only, test 16 is a single-caller check and test 18 is a builder log step; `impl Deserialize for Formula` moves into the stub and `ClientSpells` is defined; `Tier::ages()` is dropped; Fire-is-Seedling is stated as a decision; a purpose line, a GS walk and a brotli re-baseline rule are added.

**Purpose (read before scoring):** zero player-visible change, by design. This item exists to unblock I-027, I-028, I-029, I-030 and the B10 foundation. Do not score it for fun or feel.

**Backlog row:** I-026, bars B10 and B6. The row says "after I-022, I-025"; it should say **"none (merge-order only)"**, because neither is a code dependency. The planner fixes the row's dependency cell on origin/main's `backlog.md`. This spec is written so that it has **no shared hunks** with I-022, I-024 or I-025 and can build in parallel with them (see "Overlap and merge order").

## Facts from c4832d2 this spec relies on

`chem.rs` means `crates/bevy-client/src/chem.rs`, `main.rs` means `crates/bevy-client/src/main.rs`, `sim` means `crates/sim`.

- **The 3D client hard-codes its four spells.**
  - `chem::Spell { Fire, Water, Frost, Foam }` with `Spell::ALL` in that order (`chem.rs:59-67`). The index into `ALL` is the spell number everywhere (`game.spell`, `select N` picks `N-1`, keys 1-4, `cues::flight_secs`).
  - `impl Spell` (`chem.rs:66-104`) has three string methods: `name()` (69), `formula()` (78) and `cast_caption()` (88).
  - Outside `chem.rs` these are called in exactly three places: `spell.cast_caption()` at `main.rs:1098` and `name()`, `formula()` at `main.rs:1587-1588` (the `__gameState` `spellName` and `formula` fields). The format string interpolates them **raw, with no `esc()`** (`main.rs:1581`).
  - The hotbar is static HTML (`web/index.html:28-31`), with its own label text: `.name` and `.formula` spans per slot. Water's and CO2's hotbar labels differ from `formula()` (`cools below ignition`, `CO₂ displaces O₂`).
  - Rules (what each spell does to an object) live in `chem::apply_inner` and the `Extinguished(Spell)` captions. They are keyed by the enum, not by text.
  - `chem.rs` is Bevy-free and is tested with plain `rustc --edition 2021 --test src/chem.rs` (CI `ci.yml:294`). Its tests never call `name`, `formula` or `cast_caption`.
- **The exact strings today** (these are the parity values for test 2):

  | slot | `name()` | `formula()` (state `formula`) | hotbar `.formula` (index.html) | `cast_caption()` |
  |---|---|---|---|---|
  | 1 | `Fire` | `CH₄ + 2O₂ → CO₂ + 2H₂O` | `CH₄ + 2O₂ → CO₂ + 2H₂O` | `Fire spell: a gas flame. Methane burns: CH₄ + 2O₂ → CO₂ + 2H₂O, and lots of heat.` |
  | 2 | `Water` | `H₂O absorbs heat` | `cools below ignition` | `Water soaks up heat and cools things below their ignition temperature.` |
  | 3 | `Frost` | `N₂(l) boils at −196 °C` | `N₂(l) boils at −196 °C` | `Liquid nitrogen boils at −196 °C (77 K), pulling heat out of anything it touches.` |
  | 4 | `CO₂` | `CO₂ blanket, no O₂` | `CO₂ displaces O₂` | `CO₂ from an extinguisher is heavier than air: it smothers fire by pushing the oxygen away.` |

  The hotbar `.name` spans equal `name()` for all four.
- **crates/sim already has the science.**
  - `formula.rs` (no dependencies beyond std): `parse_equation` accepts ASCII and Unicode (`2H₂ + O₂ → 2H₂O`), state symbols, and a single term with no arrow. `Equation` has `is_balanced`, `species_key`, `to_string` (whole-number, no states).
  - `reactions.rs`: `ReactionBook::load()` embeds `content/reactions.json` **and** `content/known_reactions.json` with `include_str!`, parses every equation, and asserts balance and state symbols. `Reaction.tier_min` is a `String` (`"sprout"`, `"seedling"`). `PhaseChange` has `id`, `temperature_c`, `latent_heat_kj_per_mol`, `note`, `source`. The JSON rows also carry `substance` (`"H2O"`), which the Rust struct ignores today.
  - `content/reactions.json` has 4 reactions (`water_synthesis`, `electrolysis_water`, `methane_combustion` (tier_min `seedling`), `carbon_combustion` (`seedling`)) and 3 phase changes (`water_melting`, `water_boiling` 100 °C / 40.65 kJ/mol, `water_vaporisation_25c`). **There is no nitrogen entry.** Frost's −196 °C is sourced only in `content/science-sources.md` ([S1], NIST N2 phase data, Tboil 77.34 K).
  - `Tier { Sprout, Seedling, Apprentice }` lives inside `game.rs:150` (the 4,500-line 2D game module), derives `Serialize` only, and `Tier::parse` maps anything unknown to `Apprentice`. There is no `Adept`.
  - Other consumers of `reactions.json`: the 2D `web/src/book.ts` looks phase changes up by id only, and `game/free.rs` unlocks pages by id. Adding a row changes neither.
- **sim's manifest** (`crates/sim/Cargo.toml`): `crate-type = ["cdylib", "rlib"]`, deps `serde` (derive), `serde_json`, `wasm-bindgen`, and **no features**. `lib.rs` exposes `formula, game, grid, palette, reactions, species, spell, web`. `web.rs` is the 2D wasm-bindgen surface (`#[wasm_bindgen] pub struct WebGame`, `pub fn reaction_book`, `pub fn heat_lut`). Module dependencies: `formula` uses std only; `reactions` uses `formula`; `spell` uses `formula` and `reactions`; `grid`, `palette`, `game` and `web` use `species` and each other.
- **The client's build.** `crates/bevy-client` is its own workspace with its own `Cargo.lock`. `build-web.sh` runs `cargo build --release --locked`, so any new dependency needs that lock updated in the same PR. The client lock has `serde`, `serde_core`, `serde_derive` and `memchr`, but **not** `serde_json`. Release profile is `opt-level = "z"`, fat LTO, `panic = "abort"` (I-024 may change the profile). Brotli of the wasm was about 3.56 MB at opt-level z (I-015).
- **CI** (`ci.yml`): root `cargo clippy --all-targets --locked -D warnings`, `cargo test --locked` (sim, default features), wasm clippy for sim (`:145`); bevy-client job runs `cargo fmt --check`, the plain-`rustc` chem test and wasm clippy (`:291-300`); `cargo deny` on both manifests (`:369-375`).
- **The science lint** (`test/science-sources.test.ts`) scans only `crates/bevy-client/src/**.rs` and `web/**.{html,js}` (`scanClient`, `:476-498`). A `C-` row is stale if a file in its `where` cell no longer contains its text (`:676-688`). A claim is satisfied by text alone, whatever file it is in. So moving a string out of `chem.rs` makes rows C-001 to C-004, C-006, C-007, C-034, C-035 stale until their `where` changes, and a new data file is **not** scanned unless the lint learns to read it.
- **Golden** has 98 checks at a0e2ecb/c4832d2. It pins the state key order (`BASE_KEYS` includes `spellName`, `formula`) and `keys 1-4 select spells` (`golden.mjs:162`). It does not compare the spell strings' values.

## Problem

The roadmap's core loop (target.md "Core loop") needs every spell to be a real reaction plus settings. Players learn it at a Formula Bench by building its formula at their band's level, master it, and later craft new ones. The 3D client cannot do any of that. Its spells are an enum with string literals, nothing ties a spell to a reaction, and the engine that parses and balances formulas (crates/sim) is not linked into the client at all. Every phase-2 item (I-027 progress keyed by spell, I-028 Bench, I-029 worked casts, I-030 mastery trial) needs one stable answer to "what is a spell, what is its formula, and what sourced reaction is it?" B10 ("every craftable combination matches the rules engine and a cited source") cannot start until spells are data the engine can check.

**Goal:** the four spells come from `content/spells.json`, loaded and validated through crates/sim, and each spell is traced to a sourced entry by a rule a test checks: Fire, Water and Frost to a sourced entry in `content/reactions.json` that the engine checks, and CO2 to its sourced row in the science register. The game behaves **exactly** as at c4832d2: same strings, same rules, same golden.

## Bars moved

- **B10 (crafting is correct):** foundation only. After this item, the craftable set (4 spells) is data. **3 of 4 spells** (Fire, Water, Frost) are traced to a sourced reaction or phase change under an engine-checked role. CO2 is traced to the sourced register row for its own caption (C-035, [S19] [S14]), which a Node test checks; that is a citation, not an engine check, and is reported as such. B10 is scored on crafting, which arrives in I-031, so nothing is reported as moved at merge.
- **B6 (science is right):** must stay at 0 errors and 100% sourced. The lint learns to scan the new data file, so this item cannot open a hole in B6 coverage (test 17).
- **B1:** must stay green with **no edit** to `tests/e2e/golden.mjs` (test 14).
- **B2, B3:** B3 must not regress beyond the brotli budget (test 13, a hard gate). B2 is recorded, not gated (test 13): the parse runs once per boot, and a 1 ms gate is below the measured noise.

## Gameplay standards walk

No GS item is touched (GS-1 to GS-35): this item adds no screen, control, camera, effect, sound or text change.

## Content decision: Fire is Seedling and up

`Fire.tier_min = seedling` follows from `methane_combustion.tier_min = seedling` (the rule `spell.tier_min >= reaction.tier_min`). This is a **decision**, not an accident: Sprout (ages 4-7) will not be able to learn Fire once I-028 enforces `tier_min`. The client ignores `tier_min` in this item. I-028 and I-029 must check that no Sprout-reachable quest requires casting Fire (the first-quest fires are put out with Water, Frost and CO2, so none is expected).

## Smallest version

1. **`content/spells.json`** (schema in Contract). Four entries in slot order, with the exact strings from the table above, plus `formula`, `tier_min` and a `reaction` reference:

   | id | slot | effect | formula | tier_min | reaction (kind, id, role) |
   |---|---|---|---|---|---|
   | `fire` | 1 | `fire` | `CH4 + 2O2 -> CO2 + 2H2O` | `seedling` | reaction, `methane_combustion`, `runs` |
   | `water` | 2 | `water` | `H2O(l)` | `sprout` | phase_change, `water_boiling`, `absorbs_heat` |
   | `frost` | 3 | `frost` | `N2(l)` | `sprout` | phase_change, `nitrogen_boiling`, `absorbs_heat` |
   | `co2` | 4 | `foam` | `CO2(g)` | `sprout` | register, `C-035`, `cites` |

   - `id` is the permanent key that I-027 saves and sends in analytics. It is never renamed. In the client it is read **only** through `chem::Spell::spell_id()` (= `def().id`).
   - `effect` names the rule in the client's chemistry engine (`chem::Spell`), read through `chem::Spell::effect_id()`. The spell id `co2` and the effect `foam` differ on purpose: phase-3 crafted spells will share effects. **Never save or send an effect id.**
   - `tier_min` is the lowest band that may learn or craft it at the Bench (I-028). **The client ignores it in this item**, so every band keeps all four spells, as today. Fire is `seedling`: see "Content decision" above.
   - **Why CO2 cites a register row:** no entry in `reactions.json` sources smothering. `carbon_combustion`'s source is the enthalpy of formation of CO2, which says nothing about putting fire out, so referencing it would be false provenance. C-035 is the register row for CO2's own cast caption, verdict `simplified`, sourced [S5] [S8] [S14] [S19] ([S19] § 1.4.2: CO2 "extinguishes fire by reducing the concentration of oxygen"). A sourced inert-gas or fire-triangle entry in `reactions.json` that the engine can check is later work (filed as a follow-up for B10).
2. **One new sourced phase change in `content/reactions.json`:** `nitrogen_boiling`, with `substance` `"N2"`, `temperature_c` −195.8 (77.34 K − 273.15, rounded to 0.1), the enthalpy of vaporization at the normal boiling point copied from the NIST page [S1] (about 5.57 kJ/mol; the builder records the exact page value and the page's citation in `source`), and `reviewed_by: null`. `content/science-sources.md`'s [S1] entry is extended, if needed, so that it names the enthalpy of vaporization as well as Tboil. The science reviewer checks the recorded value against the NIST page itself, not against this spec. The rounded temperature must equal `chem::LN2_C` (−196), which test 19 pins. No reaction rows change.
3. **crates/sim becomes a library the client can link without the 2D game.**
   - Features: `default = ["web"]`, `game = []`, `web = ["game", "dep:wasm-bindgen"]`; `wasm-bindgen` becomes `optional = true`.
   - `lib.rs`: `formula`, `reactions`, `species`, `spell`, `spellbook` (new) and `tier` (new) are always compiled. `game`, `grid` and `palette` are behind `#[cfg(feature = "game")]`, and `web` is behind `#[cfg(feature = "web")]`. Default features build exactly what is built today, so the 2D game, `wasm-pack` and root CI are unchanged.
   - `reactions.rs`: `PhaseChange` gains `#[serde(default)] pub substance: String`. No other change to `ReactionBook`.
   - New `tier.rs` with the 4-band `Tier` (Contract). `game::Tier` (3 bands, 2D, retired) is **not** changed or replaced; it gets no conversions. The 2D game does not use the new type.
   - New `spellbook.rs`: `Formula`, `ReactionRef`, `RefKind`, `Role`, `SpellDef`, `SpellBook`, `SpellError`, and the validation rules in the Contract.
4. **The client links sim.**
   - `crates/bevy-client/Cargo.toml` `[dependencies]`: `realgame-sim = { path = "../sim", default-features = false }`. `crates/bevy-client/Cargo.lock` updated (adds `realgame-sim`, `serde_json` and its small deps). No profile change.
   - New `crates/bevy-client/src/spells.rs` (Bevy-free): a `OnceLock<ClientSpells>` whose initialiser is the **only** place in the client that calls `ReactionBook::load()` and `SpellBook::load(&book)`. It maps each `SpellDef.effect` to `chem::Spell::from_effect_id`, and checks that slot `i + 1` maps to `Spell::ALL[i]` for every `i`. Any mismatch panics with a message that names the spell. The shipped content cannot reach that panic: sim's tests run the same validation on the same embedded file, and test 19 checks the effect ids against `chem.rs` in `npm test` (CI), before any build ships.
   - `spells.rs` holds an inherent `impl chem::Spell` (legal in the same crate) with `def()`, `spell_id()`, `name()`, `formula()` and `cast_caption()`, each returning from the loaded book (`spell_id()` returns `def.id`; `formula()` returns `def.label`). **`main.rs:1098` and `1587-1588` compile unchanged.**
   - `chem.rs`: the three string methods (`chem.rs:69-103`) are deleted. `Spell::effect_id(self) -> &'static str` (`"fire"`, `"water"`, `"frost"`, `"foam"`) and `Spell::from_effect_id(&str) -> Option<Spell>` are added, with a unit test. The names say "effect" on purpose so nobody mistakes them for the saved spell id. No rule, constant or caption in `chem.rs` changes.
   - `main.rs`: exactly one line, `mod spells;`, between `mod player;` and `mod world;` (`main.rs:22-23`).
   - **JSON safety:** because `main.rs:1581` interpolates `name` and `label` raw into JSON, validation rejects any spell text containing `"`, `\` or a control character.
5. **Science register.** No claim text changes. The `where` cell of every row whose string moves out of `chem.rs` becomes `content/spells.json`, keeping `crates/bevy-client/web/index.html` where it is already listed:
   - C-001, C-002, C-003, C-004, C-006, C-007, C-034 and C-035;
   - C-031 and C-032 (hotbar labels, now also in `spells.json`) add `content/spells.json`;
   - the `CO2` not-claim row's `where` becomes `content/spells.json, crates/bevy-client/web/index.html`.
   - The lint learns to scan `content/spells.json`. It reads the string **values** of the keys `name`, `label`, `hotbar_label` and `cast_caption` in each `spells[]` entry, and nothing else (ids, effects, formulas, notes and reaction refs are not player text). It applies the same `normalize` and `isClaim` as for client files.
6. **The index.html hotbar stays static** (not edited). A Node test pins it to `spells.json` so the two cannot drift (test 19). Rendering the hotbar from data is I-028 or I-029's job.

## Acceptance tests

Each test names how it fails on c4832d2 (or on a no-op implementation). Sim tests run with `cargo test -p realgame-sim --locked` from the repo root and again with `--no-default-features`.

**Sim unit tests** (in `spellbook.rs` and `tier.rs` unless noted). They fail today because the modules do not exist.

1. **Shipped book loads.** `SpellBook::load(&ReactionBook::load())` returns 4 spells with ids `[fire, water, frost, co2]`, slots `[1, 2, 3, 4]` and effects `[fire, water, frost, foam]`, in that order. `by_id("co2")` and `by_slot(4)` return the same entry; `by_id("foam")` returns `None`.
2. **Parity with c4832d2.** For each slot, `name`, `label`, `hotbar_label` and `cast_caption` equal **byte for byte** the literals in the "exact strings" table above, which the test hard-codes. Changing one character in `spells.json` fails it.
3. **Every spell traces to a sourced entry.** For Fire, Water and Frost, `reaction` resolves in the `ReactionBook`, its `source` has more than 10 characters, and its role rule passes. The test asserts the four pairs exactly: `fire → reaction methane_combustion runs`, `water → phase_change water_boiling absorbs_heat`, `frost → phase_change nitrogen_boiling absorbs_heat`, `co2 → register C-035 cites`. (Sim checks only the register id's pattern; test 19 checks that the row exists and is sourced.)
4. **Validation rejects bad content** (`SpellBook::parse` on mutated JSON; each case asserts the **specific** `SpellError` variant, so a validator that returns `Ok` fails every case):
   - a. `methane_combustion` → `methane_burns`: `UnknownRef`.
   - b. Fire's formula `CH4 + O2 -> CO2 + H2O` (unbalanced): `RoleMismatch` (rule: `runs` needs the exact reaction).
   - c. Fire's formula `2H2 + O2 -> 2H2O`, still referencing `methane_combustion`: `RoleMismatch`.
   - d. Frost's formula `O2(l)`: `RoleMismatch` (rule: `absorbs_heat` needs `substance` to match).
   - e. CO2's ref `{"kind": "register", "id": "C35", "role": "cites"}`: `UnknownRef` (register ids match `^C-[0-9]{3}$`).
   - f. CO2's ref `{"kind": "reaction", "id": "carbon_combustion", "role": "cites"}`, and separately `{"kind": "register", "id": "C-035", "role": "runs"}`: `RoleMismatch` (`cites` goes only with `register`, and `register` only with `cites`).
   - g. Water's ref `{"kind": "reaction", "id": "water_boiling"}`: `UnknownRef` (it is a phase change, not a reaction).
   - h. Two spells with id `fire`: `DuplicateId`. Two with slot 2: `DuplicateSlot`. Slots `1, 2, 3, 5`: `SlotGap`.
   - i. Fire with `tier_min: "sprout"`: `TierBelowReaction`.
   - j. Water's `name` set to `Wa"ter`, and separately `cast_caption` with a `\n`: `BadText`.
   - k. An unknown field `"colour"` on a spell, an unknown tier `"toddler"`, `"version": 2` and a formula `Xx2`: each gives `Json` (unknown fields are denied; the formula error message names `Xx2`).
   - l. A `ReactionBook` clone whose `methane_combustion.source` is `""`: `Unsourced`.
   - m. An id `Fire!`: `BadId` (ids match `^[a-z][a-z0-9_]{0,31}$`).
5. **Nitrogen row** (in `reactions.rs` tests): `nitrogen_boiling` exists, `substance == "N2"`, `temperature_c` is in [−195.85, −195.75], `temperature_c.round() == -196.0`, `latent_heat_kj_per_mol` is in [5.5, 5.65], and `source` contains `NIST` and `Nitrogen`. It fails today because the row is missing.
6. **Tier.** `Sprout < Seedling < Apprentice < Adept`. `from_id` and `id` round-trip all four, and `from_id("Sprout")` and `from_id("")` give `None`. Every `tier_min` in `reactions.json` parses with `Tier::from_id` (today's 2D `Tier::parse` would silently accept a typo).
7. **Formula.** `Formula::parse("CH₄ + 2O₂ → CO₂ + 2H₂O")` and `Formula::parse("CH4 + 2O2 -> CO2 + 2H2O")` give equal `equation.to_string()`. `Formula::parse("N2(l)")` is a single term (`single()` is `Some`, its formula is `N2`, its state is liquid), and `is_reaction()` is false. For the Fire spell, `label` parses to the same equation as `formula` (the hotbar and state show the formula the engine checks).
8. **Feature gating.**
   - `cargo test -p realgame-sim --no-default-features --locked` compiles and passes tests 1-7. Today the feature flag does not exist, so the command errors.
   - `cargo tree -p realgame-sim --no-default-features -e normal` lists no `wasm-bindgen`.
   - `cargo clippy -p realgame-sim --no-default-features --target wasm32-unknown-unknown --locked -- -D warnings` is clean.
9. **2D unchanged.** Root `cargo test --locked` and `cargo clippy --all-targets --locked -- -D warnings` pass with default features, with no edits to the existing sim tests. `npm run build:site` still builds the classic game at `/classic/`.

**Client** (from `crates/bevy-client`).

10. **chem.rs.** `rustc --edition 2021 --test src/chem.rs -o <scratch>/chem && <scratch>/chem` passes, including the new `effect_ids_round_trip`: `Spell::ALL.map(Spell::effect_id) == ["fire", "water", "frost", "foam"]`, `from_effect_id` inverts each, and `from_effect_id("co2")` and `from_effect_id("Fire")` give `None`. `grep -c "Fire spell: a gas flame" src/chem.rs` is 0 (the strings moved). `grep -cE "fn (id|from_id)\b" src/chem.rs` is 0 (no method named plain `id`, so I-027 cannot pick the effect id by mistake).
11. **Builds.** `cargo fmt --check`, `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings`, a native `cargo check --locked`, and `./build-web.sh` all pass. `cargo deny --manifest-path crates/bevy-client/Cargo.toml --locked check advisories bans licenses sources` passes.
12. **No 2D exports leak into the 3D wasm.** After `build-web.sh` (which writes `web/pkg-out/`, `--out-name bevy-client`; `build-web.sh:19-22` at c4832d2), `test -s web/pkg-out/bevy-client.js` succeeds and `grep -cE "WebGame|reaction_book|heat_lut" web/pkg-out/bevy-client.js` is 0. If sim were linked with default features, the wasm-bindgen glue would list them. The `test -s` stops the grep passing on a missing file.
13. **Size (hard gate) and speed (recorded) (tester).**
    - Build the base (branch base) and the branch the same way into separate dirs under `<scratchpad>/I-026/`.
    - **Gate:** brotli (`-q 11`) of `bevy-client_bg.wasm` grows by **at most 60 KB**. Log both byte counts and the delta, and record them in `docs/loop/items/I-026/evidence/size.md`. If the delta is over 40 KB, also attach `twiggy top -n 30` of the branch wasm (or `wasm-objdump -h` section sizes when twiggy is not available), so the next item knows where it went. If I-024's profile change makes 60 KB unreachable, the **tester** re-sets the budget from the base of record (post-I-024 origin/main) and records why; the builder never changes it.
    - **Recorded, not gated:** if I-024's `tests/e2e/perf-pairs.mjs` is merged, run it base against branch at `low@4x` (its default rounds, with its noise run) and paste its verdict. Otherwise `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs`, 3 interleaved pairs. Record median CPU p50 and TTI per side in `evidence/size.md`. Base-vs-base spread is 30-48 ms (I-024), so a 1 ms gate could not discriminate; the structural guard is test 16.
14. **Golden unchanged.** `git diff <base>..HEAD -- tests/e2e/golden.mjs` is empty. `QUERY=?low node tests/e2e/golden.mjs` passes every check, 3 of 3 runs, with no console errors. That is 98/98 at c4832d2, or the full count at whatever origin/main the branch is rebased onto. Run golden in isolation (I-017).
15. **Runtime parity (tester; a throwaway script in the scratchpad, not committed).** Against the base build and the branch build, with `QUERY=?low`: for N in 1..4, `reset`, `select N`, read `state.spellName` and `state.formula`, then `look 0 60` (sky), `cmd('cast')`, and wait for `casts` +1 and read `state.caption`. All 12 strings are equal between base and branch, and so are `JSON.parse` of the first published state (keys and spell values). To show the check can fail, swap two `cast_caption`s in a local `spells.json`, rebuild, and confirm it reports the difference; then discard the swap.
16. **Parsed once, by one caller.** `grep -rnE "SpellBook::(parse|load)|ReactionBook::load" crates/bevy-client/src` finds matches only in `spells.rs`, and only inside the `OnceLock` initialiser (`get_or_init` closure). It fails on c4832d2 (no `spells.rs`, so the positive half finds nothing) and on any per-frame call added elsewhere. This is a line in `test/spells.test.ts` (test 19), so it runs in CI.

**Lint and Node** (`npm test`).

17. **The lint scans spells.json.** New cases in `test/science-sources.test.ts` use the existing fixture scan root (`SCIENCE_SCAN_ROOT`):
    - A fixture `content/spells.json` whose `cast_caption` is `Water boils at 50 °C.` (unregistered) makes the lint report it as unsourced, with file `content/spells.json`.
    - The same sentence under a top-level `notes` key, or in a spell's `id` or `formula`, is **not** reported.
    - The real repo passes: no unsourced claims, no stale rows.
    - It fails today: the scan never opens `content/spells.json`, so the first case is not reported.
18. *(Not an acceptance test; a builder log step.)* After the `where` edits in item 5, `npm test` is green. Before them (with the strings moved), the lint reports C-001, C-002, C-003, C-004, C-006, C-007, C-034 and C-035 stale. The lint-lane builder logs that red step in the round note.
19. **`test/spells.test.ts` (new; runs in CI's `npm test`, `ci.yml:339`, which is the client-side CI check for the seam).**
    - The `.slot` buttons in `crates/bevy-client/web/index.html`, in document order, have `data-spell` `1..4` equal to the `slot` values. Their `.name` and `.formula` span texts (whitespace collapsed) equal `name` and `hotbar_label`.
    - **Effect-id seam:** the `effect_id()` match arms parsed out of `crates/bevy-client/src/chem.rs`, in `Spell::ALL` order, equal the `effect` values of `spells[]` in slot order. Changing one `effect` in `spells.json`, or one arm in `chem.rs`, fails it.
    - `LN2_C` parsed from `chem.rs` equals `Math.round(nitrogen_boiling.temperature_c)` from `content/reactions.json`.
    - Every `spells[].reaction` with kind `reaction` or `phase_change` exists in `reactions.json` under the list its `kind` names.
    - Every `spells[].reaction` with kind `register` names a `C-` row in `content/science-sources.md` whose verdict is `correct` or `simplified`, whose sources cell has at least one `[S..]` id defined under `## Sources`, and whose claim (after the lint's `normalize`) equals the normalized value of one of that spell's player-text fields. For CO2 that is C-035 and its `cast_caption`.
    - Test 16's single-caller check.
    - It fails today (no `spells.json`).

## Out of scope

- **Any behaviour change.** No change to rules, captions in `EventKind::caption`, `Extinguished(Spell)` texts, speeds, colours, cooldowns, `cues.rs`, the hotbar DOM, the key bindings, the `% 4` cycling (`main.rs:732`) or `select`'s `clamp(1, 4)` (`main.rs:760`). Generalising the hotbar to N spells is I-029.
- **Enforcing `tier_min` in the client**, an age-band switch, or an `Adept` choice in the age picker (I-028). `age.js`, `analytics.js`, `index.html`, `hud.js`, `style.css` and `boot.js` are not edited.
- **Progress, XP, saves, analytics events** (I-027). This item only fixes the `id` they key on.
- **Per-band Bench representations** (Sprout cards, Seedling tiles, Adept conditions). I-028 adds them as new `#[serde(default)]` fields (see Contract, "Extending").
- **Crafting settings** (amount, form, temperature, trigger) and energy as mana (I-031).
- Moving `chem.rs` into sim, or making sim drive the 3D rules. The rules stay in the client's `chem.rs`. A server-authoritative engine is later work.
- Changing the 2D game, `game::Tier`, `Tier::parse`, `spell.rs` aliases or `known_reactions.json`. Splitting `ReactionBook::load` so the client skips `known_reactions.json` (about 4.7 KB of text) is only worth doing if test 13's size report says so.
- Science claims about latent heat or reaction energies in the 3D UI. None are shown in this item. When I-028 shows reaction text from `reactions.json`, it must extend the lint's JSON scan to it.
- `crates/bevy-client/README.md` (one line about `content/spells.json`, filed as a docs follow-up).

## Contract

Lands first, on the branch base, as one commit by the contract builder. It has no behaviour of its own; lanes code against it.

### Files the stub creates or changes

- `crates/sim/Cargo.toml`: the `[features]` table and `optional = true` on `wasm-bindgen` (item 3).
- `crates/sim/src/lib.rs`: the `cfg` gates and `pub mod spellbook; pub mod tier;`.
- `crates/sim/src/tier.rs` and `crates/sim/src/spellbook.rs`: the types below, complete, **including the full `impl Deserialize for Formula`** (without it the stub does not compile). `SpellBook::parse` in the stub **deserializes only** and returns `Ok` (no validation rules), and the stub's `load` calls `parse(..).expect(..)`.
- `crates/bevy-client/src/spells.rs`: the `ClientSpells` type and the `book()` signature with `todo!()` bodies are **not** in the stub; the client lane owns that file. The stub only makes `realgame-sim` linkable.
- `content/spells.json`: the full file (the data is part of the contract).
- `crates/bevy-client/Cargo.toml` (`[dependencies]` only) and `crates/bevy-client/Cargo.lock`.
- `Cargo.lock` (root), only if cargo changes it.

### Content schema: `content/spells.json`, version 1

```json
{
  "version": 1,
  "notes": "free text for maintainers; never shown, never scanned",
  "spells": [
    {
      "id": "fire",
      "slot": 1,
      "effect": "fire",
      "name": "Fire",
      "label": "CH₄ + 2O₂ → CO₂ + 2H₂O",
      "hotbar_label": "CH₄ + 2O₂ → CO₂ + 2H₂O",
      "cast_caption": "Fire spell: a gas flame. Methane burns: CH₄ + 2O₂ → CO₂ + 2H₂O, and lots of heat.",
      "formula": "CH4 + 2O2 -> CO2 + 2H2O",
      "tier_min": "seedling",
      "reaction": { "kind": "reaction", "id": "methane_combustion", "role": "runs" }
    }
  ]
}
```

| field | type | rule |
|---|---|---|
| `version` | integer | must be `1` |
| `notes` | string | optional; ignored |
| `spells[].id` | string | `^[a-z][a-z0-9_]{0,31}$`, unique, permanent (saved by I-027) |
| `spells[].slot` | integer 1-9 | unique; the set is exactly `1..=n` |
| `spells[].effect` | string | a key the client's `chem::Spell::from_effect_id` knows; sim checks only the id pattern |
| `spells[].name` | string | player text; 1-40 chars; no `"`, `\` or control chars |
| `spells[].label` | string | player text (state `formula`); 1-80 chars; same character rule |
| `spells[].hotbar_label` | string | player text; 1-80 chars; same character rule |
| `spells[].cast_caption` | string | player text; 1-200 chars; same character rule |
| `spells[].formula` | string | parses with `formula::parse_equation`; ASCII as stored |
| `spells[].tier_min` | `"sprout" \| "seedling" \| "apprentice" \| "adept"` | ≥ the referenced reaction's `tier_min` (reactions only) |
| `spells[].reaction` | object | `{kind, id, role}`, all required; `kind` is `reaction`, `phase_change` or `register`; see below |

Unknown fields at any level are an error.

### Rust types (`realgame_sim`)

```rust
// tier.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier { Sprout, Seedling, Apprentice, Adept }
impl Tier {
    pub const ALL: [Tier; 4];
    pub fn id(self) -> &'static str;               // "sprout" ...
    pub fn from_id(s: &str) -> Option<Tier>;       // exact, lowercase only
}

// spellbook.rs
pub const SPELLS_JSON: &str = include_str!("../../../content/spells.json");

/// A formula as content writes it, parsed once.
#[derive(Debug, Clone, PartialEq)]
pub struct Formula { pub text: String, pub equation: formula::Equation }
impl Formula {
    pub fn parse(s: &str) -> Result<Formula, formula::ParseError>;
    pub fn is_reaction(&self) -> bool;             // equation.has_arrow
    pub fn single(&self) -> Option<&formula::Term>; // Some iff no arrow and exactly one term
}
impl<'de> Deserialize<'de> for Formula { /* in the stub: from a string via parse; error text names the input */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind { Reaction, PhaseChange, Register }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role { Runs, AbsorbsHeat, Cites }

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReactionRef { pub kind: RefKind, pub id: String, pub role: Role }

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellDef {
    pub id: String,
    pub slot: u8,
    pub effect: String,
    pub name: String,
    pub label: String,
    pub hotbar_label: String,
    pub cast_caption: String,
    pub formula: Formula,
    pub tier_min: Tier,
    pub reaction: ReactionRef,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpellError {
    Json(String),
    BadId(String),
    DuplicateId(String),
    DuplicateSlot(u8),
    SlotGap,
    BadText { spell: String, field: &'static str },
    UnknownRef { spell: String, id: String },
    Unsourced { spell: String, id: String },
    TierBelowReaction { spell: String },
    RoleMismatch { spell: String, why: String },
}

pub struct SpellBook { pub spells: Vec<SpellDef> }   // sorted by slot
impl SpellBook {
    pub fn parse(json: &str, book: &ReactionBook) -> Result<SpellBook, SpellError>;
    pub fn load(book: &ReactionBook) -> SpellBook;    // parse(SPELLS_JSON, book).expect(..), like ReactionBook::load
    pub fn by_id(&self, id: &str) -> Option<&SpellDef>;
    pub fn by_slot(&self, slot: u8) -> Option<&SpellDef>;
}
```

### Validation rules (the sim lane implements them; tests 3-4 pin them)

In this order, first error wins: JSON and schema (`Json`); id pattern (`BadId`) for `id` and `effect`; unique ids; unique slots; slots exactly `1..=n`; text fields (`BadText`); then for each spell, its reference:

- **Resolve.** `kind: reaction` looks only in `book.reactions`; `kind: phase_change` looks only in `book.phase_changes`. Not found gives `UnknownRef`. A `source` with 10 characters or fewer, after trimming, gives `Unsourced`. `kind: register` is not resolved in sim (sim cannot read the register); its `id` must match `^C-[0-9]{3}$`, otherwise `UnknownRef`. Test 19 resolves it.
- **`runs`** (reactions only): `formula.is_reaction()`, it is balanced, and `formula.equation.to_string() == reaction_equation.to_string()` (same species and the same whole-number coefficients; states ignored).
- **`absorbs_heat`** (phase changes only): `formula.single()` is `Some(t)`, `t.formula == phase.substance`, `t.state` is `Some`, and `phase.latent_heat_kj_per_mol > 0`.
- **`cites`** (register only): the formula must still parse (it always does by this point); no engine check. A `cites` ref is a citation, not engine proof, and B10 counts it separately.
- A role used with the wrong `kind` (including `register` with anything but `cites`) is `RoleMismatch`. `why` names the rule that failed.
- **Tier.** For `kind: reaction`, `Tier::from_id(reaction.tier_min)` must be `Some` (otherwise `Json`), and `spell.tier_min >= it` (otherwise `TierBelowReaction`).

### Client surface (`crates/bevy-client`)

- `chem::Spell::effect_id(self) -> &'static str` and `chem::Spell::from_effect_id(&str) -> Option<chem::Spell>` (in `chem.rs`). Effect ids are engine keys only; they are never saved or sent.
- `spells.rs`:

  ```rust
  pub struct ClientSpells {
      pub reactions: ReactionBook,   // kept for I-028, which reads reaction text
      spells: SpellBook,             // spells.spells[i] is the def for chem::Spell::ALL[i] (checked at init)
  }
  impl ClientSpells {
      pub fn def(&self, s: chem::Spell) -> &SpellDef;  // index of s in Spell::ALL
      pub fn all(&self) -> &[SpellDef];                // slot order
  }
  pub fn book() -> &'static ClientSpells;              // OnceLock; the only caller of the loaders
  ```
- `impl chem::Spell { pub fn def(self) -> &'static SpellDef; pub fn spell_id(self) -> &'static str; pub fn name(self) -> &'static str; pub fn formula(self) -> &'static str; pub fn cast_caption(self) -> &'static str }` in `spells.rs`. **`spell_id()` (= `def().id`: `fire`, `water`, `frost`, `co2`) is the only client method that returns the permanent id.** I-027's `spell_id(i)` swap calls `Spell::ALL[i].spell_id()`; I-028 uses the same name.

### Extending (for phase-2 items)

- New per-spell data is a new **optional** field (`#[serde(default)]`) added to `SpellDef` and `spells.json` in the same PR, with its own validation rule and a test. `deny_unknown_fields` stays, so a typo is still an error.
- A new role is a new `Role` variant with a rule in `SpellBook::parse` and a negative test in the style of test 4. A role must be able to fail for a wrong science claim; a rule that any unrelated substance passes is not accepted (that is why `smothers` was cut).
- `version` stays 1 while changes are additive. A rename or removal bumps it to 2 and migrates I-027's saved ids.
- Any new player-text key is added to the lint's `spells.json` key list in the same PR.

## Lanes

| lane | owns | builds |
|---|---|---|
| contract | the files in "Files the stub creates or changes" | The stub above, then `cargo test --locked` (root), `cargo test -p realgame-sim --no-default-features --locked` and `cargo check --locked` in `crates/bevy-client` all compile. It is the only writer of the two `Cargo.lock`s and the client `Cargo.toml` |
| sim | `crates/sim/src/spellbook.rs` (bodies and tests), `crates/sim/src/tier.rs` (tests), `crates/sim/src/reactions.rs` (the `substance` field and test 5 only), `content/reactions.json` (the `nitrogen_boiling` row only), `.github/workflows/ci.yml` (one step: `cargo test -p realgame-sim --no-default-features --locked` next to `:169`) | Tests 1-8 first (red against the stub's no-op `parse`; test 2 is green once the stub's data is right, and the lane checks it fails on a one-character change). Then the validation rules and the nitrogen row with its fetched NIST value and citation. Must not touch `game.rs`, `game/**`, `grid.rs`, `palette.rs`, `web.rs`, `spell.rs`, `formula.rs`, `known_reactions.json`, any existing reaction row, or `crates/bevy-client/**` |
| client | `crates/bevy-client/src/spells.rs` (new), `crates/bevy-client/src/chem.rs` (delete `name`/`formula`/`cast_caption`; add `effect_id`/`from_effect_id` and their test), `crates/bevy-client/src/main.rs` (only the `mod spells;` line), `docs/loop/items/I-026/evidence/**` | Test 10 first. Then `spells.rs` and the `chem.rs` edit; tests 11, 12 and 13 (size report). Must not touch any other line of `main.rs`, `cues.rs`, `world.rs`, `fx.rs`, `clock.rs`, `motion.rs`, `feel.rs`, `bridge.rs`, `player.rs`, `web/**`, `Cargo.toml`, `Cargo.lock` or tests |
| lint | `test/science-sources.test.ts` (the `spells.json` scan and test 17's cases), `test/spells.test.ts` (new; tests 16 and 19), `content/science-sources.md` (the `where` cells in item 5, and the [S1] enthalpy wording in item 2, only) | Tests 16, 17 and 19 first (red), then the JSON scan (keys `name`, `label`, `hotbar_label`, `cast_caption` of `spells[]` only) and the register edits; logs the step-18 red. Must not change any claim text, verdict or other lint rule |

All three build lanes start together after the contract stub. If any lane finds it needs a dependency, feature or lock change that the contract did not make, it **stops and asks the contract owner**; it never edits a `Cargo.toml` or `Cargo.lock` itself. CI: the sim lane adds the `--no-default-features` step; the client-side seam check is test 19 in the existing `npm test` step (`ci.yml:339`), so no client CI step is added. The **seam** is `content/spells.json` and the `SpellDef` field names: the client reads `name`, `label` and `cast_caption`; the lint reads the same keys plus `hotbar_label`; sim validates them. The seam breaker checks these things:

- The four `effect` ids equal `chem::Spell::ALL.map(effect_id)`, in slot order (also automated in test 19).
- Nothing in the client saves, sends or publishes an `effect_id()`; only `spell_id()` is a permanent key.
- No spell text can break the state JSON, because the `BadText` rule covers every field that `main.rs:1581` interpolates.
- The lint's key list equals the player-text fields of `SpellDef`.
- Test 15 passes on the shared build.
- The client never calls `SpellBook::parse` per frame (`book()` is a `OnceLock`).

After the shared build, the tester runs tests 13, 14 and 15. A **science reviewer** (B6 protocol, fresh model) checks the four reference mappings and the nitrogen row against [S1], [S2], [S9], [S10], [S11], [S14] and [S19], reading the NIST page itself for the 5.57 kJ/mol figure. Each spell's reference must describe the mechanism its caption claims: Fire runs methane combustion; Water absorbs heat (the caption's "below their ignition temperature" is the simplification already noted in C-006 and C-019; latent heat of vaporisation is the real cooling mechanism); Frost absorbs heat by boiling liquid N2; CO2 cites C-035, whose sources say CO2 lowers the oxygen concentration ([S19]) and mainly absorbs heat ([S14]).

## Overlap and merge order

| item | its paths | shared with I-026 | resolution |
|---|---|---|---|
| I-022 (building) | `feel.rs`; `main.rs` `Settings` 91-113, `read_settings` literal 402-412, `aim_and_cast` 952/980/983/987; `hud.js` `setHint`/`hintAllowed`; `analytics.js`; `age.js`; golden append | none. I-026's only `main.rs` change is the `mod` line (22-23) | merge in either order |
| I-024 (building) | `crates/bevy-client/Cargo.toml` **profile sections**; `world.rs` `build`/`Builder`; the `world::build` call in `main.rs:464`; `perf.mjs`, `perf-pairs.mjs` | `crates/bevy-client/Cargo.toml`, different sections (I-026 edits `[dependencies]` only; I-024 must not touch `Cargo.lock`) | **I-024 merges first**, then I-026 rebases (a textual merge with no conflict expected) and re-runs test 13 against the post-I-024 base, so the brotli delta is measured on the final profile. If I-024's P2 adds per-package `[profile.release.package.*]` overrides, `realgame-sim` takes the default |
| I-025 (being specced) | `cues.rs` (append); `main.rs` `Beam`/`spawn_stream`/`SpellMats`/`setup` 507-516/`aim_and_cast` 1018-1136/`projectiles`/format string 1581 and args 1633-1634; `hud.js` `pickHint`; golden `streamsAndFinder` | `main.rs:1098` (`spell.cast_caption()`) sits inside I-025's 1099-1136 hunk neighbourhood, and `1587-1588` sit inside the format-argument list I-025 appends to. **I-026 edits neither line**: they keep compiling because `cast_caption`, `name` and `formula` move to `spells.rs` with the same signatures. If I-025 adds a state field that reads a spell string, it calls the same methods | merge in either order. If I-025 removes `Spell::Frost`'s beam branch (`main.rs:1101`), that is an enum match and unaffected |

The backlog lists I-022 and I-025 as dependencies. With this scope neither is a code dependency, so I-026 can build now in parallel (the row is corrected to "none (merge-order only)"). Its golden run (test 14) is repeated on the shared build at the merge base it actually lands on, with that base's full check count.

### Notes for dependent specs (planner passes these on)

- **I-027:** the permanent id is `chem::Spell::spell_id()` (from `spells.rs`). Do **not** call `effect_id()`; it returns `foam` for CO2. The four ids are `fire`, `water`, `frost`, `co2`, matching I-027's fallback table, so no alias in `decode` is needed.
- **I-028:** same method name. `Tier::ages()` is not provided; if I-028 needs ages per band, it adds them itself. CO2's `reaction` is `{kind: register, id: C-035, role: cites}`, so code that resolves a spell's reaction must handle `RefKind::Register` (no `reactions.json` entry). Fire is not shown at Sprout (decision above).
- **I-030:** a per-spell prediction field, if any, follows "Extending"; its separate `predict.json` is unaffected.
