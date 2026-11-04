# I-052 lane one-engine: the in-world bench judges with the shared codex

**Plan:** Option B, step 1. **Branch:** `loop/one-engine` (not merged before the 2026-11-05 submission deploy; no PR yet).
**Where this note lives:** `docs/loop/items/I-052/spec.md` is the block-world spec, so this lane's note is here instead of replacing it.

## 1. Current flow (origin/main)

- The courtyard bench (`crates/bevy-client/src/bench.rs:210`, `Cmd::Try`) and worked casts (`crates/bevy-client/src/main.rs:2315`, `work submit`) both call `realgame_sim::bench::judge(spells, slot, band, text)`.
- `crates/sim/src/bench.rs` `judge` (was line 70): picks the spell by slot, checks `tier_min` (`learnable`), parses, rejects zero numbers, then
  - a single-substance spell (Water `H2O(l)`, Frost `N2(l)`, CO2 `CO2(g)`) goes to `judge_single` (atoms, then state at Apprentice+);
  - a reaction spell (Fire `CH4 + 2O2 -> CO2 + 2H2O`) goes to `judge_reaction` (was line 229): needs an arrow, the spell's species (else `OtherSpell` / `WrongSpecies`), balanced atoms (else `Unbalanced` with diffs), then **the spell's own formula** as the only reference for "is this the reaction" (`coefs(simplest)` equal, else `WrongSpecies`).
- The reference data is `content/spells.json` (formula, `tier_min`, and a `reaction` ref into `content/reactions.json`: Fire `methane_combustion`, Water `water_boiling`, Frost `nitrogen_boiling` (phase changes), CO2 register `C-035`).
- The free-play Bench page uses a different engine: `crates/chem` (`Codex::load`, `Codex::judge` at `crates/chem/src/lib.rs:225`, `Codex::run`, `Outcome`) over `content/chem/core.json` (182 reviewed reactions), exported by `crates/chem/src/web.rs`.
- `web/bench.js` and `web/worked.js` only draw `__benchState` / `__workState` and never decide right or wrong.

So the two benches had separate references for the same chemistry: methane burning is `methane_combustion` in one and `methane_burns` in the other.

## 2. Proposed flow (implemented)

- `crates/sim/src/bench.rs` gets a small trait, `ReactionCodex::call(eq, band) -> CodexCall { Known(id), Locked(id), Unbalanced, Other }`, plus `NoCodex`, `codex_id(codex, def)` and `judge_with(codex, spells, slot, band, text)`. `judge` is now `judge_with(&NoCodex, ..)`, so every existing caller and test is unchanged.
- In `judge_reaction`, the spell-identity checks stay first (arrow, species, `OtherSpell`, `WrongSpecies`, `Unbalanced` diffs). The last step, "is this the real reaction", now asks the codex when the spell's formula is a codex reaction (`codex_id` is `Some`): the build is right only if the codex says `Known(id)` for the same id at the player's band; `Locked(id)` gives `NotForBand`; anything else is `WrongSpecies`. When the spell is not in the codex, the old formula comparison runs.
- `crates/chem/src/lib.rs` implements `ReactionCodex for Codex` by calling `Codex::judge(eq, &Need::ALL, band)`. All needs are supplied because casting the spell is the spark or heat; the codex holds one entry per equation (tested), so conditions never change which reaction an equation is.
- `crates/bevy-client` depends on `realgame-chem` (no `web` feature). `spells::ClientSpells` loads the codex once next to the spell book, asserts at startup that every reaction spell is a codex reaction, and exposes `ClientSpells::judge`, which both call sites now use.

## 3. Dependency direction

`realgame-chem -> realgame-sim` already exists (formula parsing, `Tier`). The trait lives in sim and chem implements it, so there is no cycle and nothing moves between crates. `realgame-bevy-client -> {realgame-sim, realgame-chem}`. The alternatives were worse: moving the bench judge into chem drags the spell book and `BenchView` with it, and moving the codex into sim mixes the free-play engine and its predictor into the 2D game crate.

## 4. Spells that are not codex reactions

Water, Frost and CO2 are single substances whose references are phase changes (`water_boiling`, `nitrogen_boiling`) or a register citation (`C-035`). `codex_id` returns `None` for any spell without an arrow, so they keep `judge_single` exactly as before. The test `every_reaction_spell_maps_to_a_codex_reaction_it_can_teach` fails if a non-reaction spell's reference kind becomes anything other than a phase change or register entry, or if a reaction spell is missing from the codex.

## 5. What a player sees

Nothing changes for the four shipped spells. Fire still needs `tier_min` Seedling in the world even though the codex teaches `methane_burns` at Sprout; learnability is the spell book's rule and runs first. Wrong-spell messages (`OtherSpell`, `WrongSpecies`) and unbalanced diffs are still worked out by the spell book, because the codex has no notion of slots.

## 6. Risks

- **A codex edit can now change a world verdict.** That is the point, but a codex band raised above a spell's `tier_min` would lock the spell (`NotForBand`). The test asserts `codex band <= tier_min` for every mapped spell; the client also asserts at startup that each reaction spell is in the codex.
- **Wasm size:** the client now embeds `core.json` (96 KB raw) and parses it once at startup. `names.json` is a `const` the client never reads, so it is not linked. `literature.json` is not included by the crate.
- **CI coverage:** the `bevy` path filter in `.github/workflows/ci.yml` only lists `crates/bevy-client/**`, so a change to `crates/chem` or `content/chem` does not re-run the client's wasm clippy and build (the same was already true for `crates/sim`). Follow-up: add `crates/sim/**`, `crates/chem/**` and `content/**` to that filter.
- **Codex id vs reactions.json id:** Fire refers to `methane_combustion` (reactions.json) and maps to `methane_burns` (core.json) by equation. The test checks the two equations reduce to the same thing.

## 7. Acceptance tests

- `crates/sim/src/bench.rs` `a_codex_decides_reaction_spells_only`: with a stand-in codex, `codex_id` is set only for Fire; every test-1 Ok gives the same verdict with or without a codex; a codex that teaches the reaction later gives `NotForBand`; identity and balance codes are unchanged; a codex that names a different reaction gives `WrongSpecies`, never Ok.
- `crates/chem/tests/one_engine.rs`:
  - `every_reaction_spell_maps_to_a_codex_reaction_it_can_teach`: Fire maps to `methane_burns` (reviewed, band <= `tier_min`, same equation as `methane_combustion`); Water, Frost, CO2 map to nothing and are phase changes or register entries.
  - `the_world_judge_and_the_codex_agree_on_every_mapped_spell`: for every mapped spell, every coefficient choice 1..6 on its species and every band, the world's Ok is exactly the codex's `Known { id }`, and the codex's `Unbalanced` is the world's `Unbalanced`.
  - `no_visible_change_for_the_shipped_spells`: for all four spells, all bands, the sim test texts and the sweep, `judge_with(codex)` equals the old `judge`.
- All existing sim, chem and client tests pass unchanged.
