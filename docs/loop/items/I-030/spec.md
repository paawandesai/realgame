# I-030: Mastery trial (3 builds from memory on 2 days, plus 1 predict question) and keep-it-sharp

**based_on:** origin/main c4832d2e86c2408443124960b130fc6208a89aab (2026-10-30, merge of PR #31, roadmap v2). Code was read with `git show c4832d2:<path>`. The primary checkout (661b9ba) has no `crates/bevy-client/src`, so builders branch from the real origin/main.

**Revision 2** (answers `critique.md` revision 1; the per-item decisions are under "Planner response" there). Main changes: the Adept questions and the `number` kind are cut (22 questions, Adept players get the Apprentice questions); `sharp_day` is dropped (keep-it-sharp moves `last_build_day`, and the save gains 1 field, not 2); section 5 now calls every `TrialDesk` method in order; `TrialView.phase` is an enum; the `rule` line has a place; `mountBuilder`'s options are pinned and checked in the contract step; [S21] is no longer needed; three question fixes.

Phase 2, fourth and last item of the core loop (`target.md`, "Core loop": "Mastery = 3 unaided correct builds spread over 2 days + 1 'predict what happens' question in a new situation. Then the spell is an instant cast from the hotbar ... Mastery never fades; an optional keep-it-sharp check only adds a bonus"). Backlog row I-030: bar B9; after I-029.

**Depends on I-026, I-027, I-028 and I-029. All four are specced and none has merged** (their specs at this writing: `docs/loop/items/I-02{6,7,8,9}/spec.md`, revision 1 each). This item adds no new mechanism that those items already own. It calls I-027's progress API, runs inside I-028's Formula Bench, and its payoff is I-029's instant route. **Its lanes start only after I-029 has merged.** The contract builder then rebases on that origin/main, checks every name in "Assumed from ..." below against the merged code, maps any renames 1:1, and records the base sha and the mapping in a "Refresh" note at the end of this spec.

**Runs beside I-022, I-024 and I-025** with no shared hunk. It does not edit `main.rs`, `hud.js`, `feel.rs`, `analytics.js`, `age.js`, `world.rs`, `Cargo.toml`, `perf.mjs`, `cues.rs` or `fx.rs`. See "Overlap and merge order".

## Facts from c4832d2 this spec relies on

`golden` means `tests/e2e/golden.mjs`.

- **None of the loop exists yet.** There is no progress model, no bench, no worked cast and no trial on c4832d2. The four spells are always castable (`chem::Spell::ALL`, `chem.rs:59-67`).
- **Science lint.** `test/science-sources.test.ts` scans only `crates/bevy-client/src/**.rs` and `crates/bevy-client/web/**.{html,js}` (`scanClient`, about lines 476-498). A string is flagged as a claim if it has a temperature, a chemical formula, a science keyword, or a sentence shape (three or more words ending in `.`, `!`, `?` or `…`). A `C-` row is stale if no file in its `where` cell contains its text. A new data file is not scanned until the lint learns to read it. I-026 adds a scan of `content/spells.json` keys, and I-028 adds three keys to that scan.
- **Register.** `content/science-sources.md` defines [S0]-[S19] ([S17] and [S20] are retired; ids are never reused). Every source this item cites already exists, and it adds none: [S1] NIST N2 phase data (Tboil 77.34 K); [S2] NIST water phase data (Tboil 373.17 K); [S4] NIST methane ΔcH°gas −890.7 kJ/mol; [S5] NIST CO2 (M 44.0095); [S6] (ignition and combustion, cited by C-009, C-023, C-025); [S7] OpenStax 4.1 (balancing, conservation of atoms); [S8] US Std Atmosphere (M air 28.9644); [S9] Grant et al., water sprays; [S10] OpenStax 10.3 (phase transitions); [S11] OpenStax 5.1 (energy, specific heat); [S12] OpenStax 9.2 (ideal gas law); [S14] EPA CO2 suppressant review; [S19] FAA AC 150/5210-6E (CO2 lowers the oxygen concentration).
- **`content/reactions.json`** has `water_boiling` (100 °C, 40.65 kJ/mol, CRC). I-026 adds `nitrogen_boiling` (−195.8 °C, ΔvapH copied from [S1], "about 5.57 kJ/mol"; I-026's builder records the exact value).
- **Golden** boots its page with `debug` always added (`golden:62`). It already has a context whose `Storage.prototype.getItem` and `setItem` throw (`golden:941-944`), and it uses `ctx.addInitScript` for that. `playwright` is not in `package.json`; golden needs it installed globally, as today.
- **`build-web.sh:21`** copies an explicit list of web files. A new `web/*.js` file is not served unless it is added there.
- **GS-28** bans daily-reward or streak lures ("come back tomorrow") and fake countdowns. **GS-30** bans quiz screens that interrupt play. **GS-25** bans time pressure the player did not choose. I-027's GS walk asks the critic to check in this item that the trial never says "come back tomorrow" as a lure.

## Assumed from I-027 (spell progress; specced, revision 1)

- `#[derive(Resource)] struct Progress(progress::Book)` in `main.rs`. `progress.rs` is pure Rust with no Bevy types, tested with `rustc --edition 2021 --test`.
- `Stage { Unknown, Learning, Practised, Mastered }` (ordered, `as_str()`), `SpellProgress { stage, xp, builds, build_days, last_build_day, predicted }`, `Book { spells: [SpellProgress; 4], rev, day }`.
- `Book::unaided_build(i)`: on Learning or Practised and only when `day != 0`, it sets `builds = min(builds + 1, 3)`; when `day != last_build_day` it sets `build_days = min(build_days + 1, 2)` and `last_build_day = day`; then +20 XP. `Book::predict(i, correct)`: on Learning or Practised, a correct answer sets `predicted` and adds +20 XP, and a wrong answer changes nothing. `settle` does Learning → Practised at 30 XP, then Practised → Mastered when `builds >= 3 && build_days >= 2 && predicted`. No operation except `clear` lowers a stage or XP.
- `Book::set_day(d)` accepts `19_000..=80_000`, and anything else sets 0 (unknown). **The day is injected:** `progress.js` pushes `prog day <localDay(new Date())>` once at boot, and `prog day <n>` is allowed in every build (not debug-gated). `prog learn|worked|build|predict|clear` are `?debug` only.
- Save: `localStorage['rg.progress.v1']`, format `p1|id:stage:xp:builds:build_days:last_build_day:predicted` (7 fields), at most 512 characters, charset `[a-z0-9:.|]`. `decode` rejects the whole save on any bad field or "too many fields". Saved only when `progRev` grows.
- `progress.js`'s `SAVE_RE = /^p1(\|[a-z0-9:.]+)*$/` (I-027 spec, `progress.js` section) does not count fields, so an 8-field entry (Smallest version 6) passes `readSave` and `createSaver` unchanged. Field-count validation lives only in `progress.rs` `decode`, which this item extends. `progress.js` is not edited (test 36); if the merged `SAVE_RE` does count fields, the contract builder stops and reports it rather than editing `progress.js`.
- `state.progress` is `[{id, st, xp, builds, days, predicted}, ...]` (I-027 test 11 pins the exact JSON for a fresh book).
- I-027 leaves the trial, unaided builds, the predict question and keep-it-sharp to this item, and says the constants may be tuned here, in one place.

## Assumed from I-028 (Formula Bench; specced, revision 1)

- The bench is a modal `#bench`. The game is paused while it is open. Its commands go through `window.__gameIn.bench` (one string each, never debug-gated): `open <band>`, `pick <slot>`, `try <slot> <text…>`, `close`. Tests push them with `window.__bench.cmd(s)`.
- `crates/sim/src/bench.rs` (the only judge): `judge(spells, slot, band, text) -> Verdict { ok, code, diffs, other, bad, simplest }`, `recipe(def) -> Vec<RecipeTerm { side, coef, formula, atoms, state }>`, `learnable(def, band)`, `BenchView { v, atBench, open, band, spells, pick, recipe, result }` (optional fields skipped when `None`), with `to_json()`.
- `crates/bevy-client/src/bench.rs`: `BenchState { open, band, pick, armed, result, n, last_json }` and one system `bench::run`, which reads the queue, judges, calls `progress.0.learn(slot − 1)` on a correct build of an Unknown spell, and publishes `window.__benchState` only when the JSON changed.
- `web/bench.js`: the band views (Sprout picture cards plus a recipe card, Seedling atom tiles plus a balance scale, Apprentice typed), `reasonFor(band, result, names, tries)`, and `mountBuilder(root, { band, spell, recipe, prompt, onMake }) -> { destroy }`. I-028 (contract, test 11b) already requires `recipe: null` to render (tiles without a recipe card). The Sprout builder has an Undo button that removes the last atom before Make, so a misplaced card need not cost an attempt. Test hook: `window.__bench = { state(), ui(), open(), close(), cmd(s) }`, where `ui().view` is one of `chooser|list|build|done|fizzle`.
- `?band=adept` reads as `apprentice` in `bandFrom` (I-028 test 11), and the chooser offers three bands. Rust accepts `open adept` and judges Adept exactly as Apprentice.
- I-028 leaves "XP for repeat builds, unaided builds and the 2-day rule" to this item.

## Assumed from I-029 (worked and instant casts; specced, revision 1)

- `worked::route(stage, learnable, mode)` gives `instant` for every Mastered spell, and `state.routes[i]` publishes it. `castmode worked|instant` is a `?debug` command, and golden's page is `instant` by default.
- Worked casts call only `worked_cast` and `instant_cast`, never `unaided_build`. I-029 leaves "whether a worked cast counts toward the mastery trial" to this item.

- I-029 calls the same `mountBuilder` with one extra option, `guided` (I-029 spec, `mountBuilder` option 3). **The trial uses exactly `{ band, spell, recipe, prompt, onMake }`** and never passes `guided` (so it is false).

**If any of these differ when merged,** the contract builder maps names 1:1 and records it in the Refresh note. Behaviour below does not change. **The contract builder also checks, before any lane starts, that the merged `mountBuilder` renders (a) with `recipe: null` at Sprout, Seedling and Apprentice, and (b) with Seedling Fire terms whose `atoms` are all empty, with no recipe card, no tick and no `.bench-scale`.** If either fails, the contract builder makes that one additive change in `bench.js` as part of the stub (hook 5), so no lane edits `mountBuilder`.

## Problem

After I-027, I-028 and I-029, a player can learn a spell at the bench (Unknown → Learning) and practise it with worked casts (Learning → Practised). Nothing in play can make it Mastered. I-027's `unaided_build` and `predict` exist, but nothing calls them. So the instant cast, the payoff of the whole loop and the thing B9 measures ("uses an instant cast within 20 minutes"), can be reached only with debug commands. There is also no check that a mastered player can still do it later ("after a break a from-memory check still passes").

Two things make this hard:
- **"Unaided" has to mean unaided.** The bench is built to help: a recipe card to copy (Sprout), atom ticks and a balance scale (Seedling), and per-element counts after three misses (Apprentice). A build made with any of these on screen is not evidence of mastery.
- **The question has to be right.** A wrong "correct answer" teaches the wrong science, and so does a distractor that is in fact true. Both are B6 errors.

**Goal:** a player-chosen trial at the bench. A Learning or Practised spell offers "Show what you know". Each trial build is checked by sim with every help hidden. A first-try correct build counts toward 3 builds on 2 different days, and it brings one band-level "predict what happens" question about a situation the game never shows. When all three are done, the spell is Mastered and becomes an instant cast. A Mastered spell offers an optional "Keep it sharp" build every few days for a small XP bonus, and nothing can take mastery away. Every day count comes from one injectable day number.

## Bars moved

- **B9 (mastery works): the item that makes it reachable in play.** After this item, Mastered and the instant cast are reached through the bench with no debug command.
  - B9 is a panel bar, so nothing is reported as moved at merge.
  - **Measurement note for the leader:** B9's "masters 3 spells within 20 minutes" cannot happen in one calendar day, because mastery needs 2 days by design. The panel must simulate the second day. The panel harness (I-006/I-014) needs a "next day" step that pushes `prog day <today + 1>` between two persona sessions (allowed without `debug`). This is a numbered backlog item, **I-041 "Panel day-2 session"** (next free id after I-040 on origin/main; the leader takes the next free id if I-041 is gone and updates this line), with "Before: first B9 scoring" (row text in Planner notes). Without it the panel can only score B9's "predict question" and "from-memory check" halves.
  - **Panel prediction** (first paired run that includes a day-2 session): the 9-year-old persona masters Water and CO₂ at Seedling; at least 70% of the personas who reach the question answer it correctly; no persona reports the trial as "a quiz", "a test I didn't ask for" or "it told me to come back tomorrow". **The 70% is not evidence of transfer.** Chance on a 3-choice question is 33%, several Sprout distractors can be ruled out by common sense, and with 2 variants a player who missed once may answer the repeat from memory. It only checks that the questions are answerable and fair.
- **B6 (science is right):** must stay at 0 errors and 100% sourced. This item adds 22 questions. Every correct answer, every explanation and every stem that states data gets a `C-` row with a source. Every distractor is registered as a Not-claim, and the science reviewer confirms each one is false (test 33).
- **B4:** guard. The trial is reached only from the bench list, which the player opens. The golden path and the first minute are unchanged.
- **B1:** green, with a new golden section. **B2/B3:** guarded (test 35). The bench is closed during play, and the question bank is about 11 KB of text embedded in the wasm.
- **B7:** unchanged. No new analytics event (I-027's held `spell_mastered` fires from the `Transition` this item causes, once ACTION-1 is approved).

## Smallest version

### 1. Rules (one place: `progress.rs` constants and `crates/sim/src/mastery.rs`)

- **What counts as an unaided build:** a **trial build**. That is a bench build started with the trial button, judged by sim, and **correct on the first submit of that attempt**, with every help hidden (2 below).
  - Normal bench builds still only teach (I-028).
  - Worked casts do not count (they carry I-029's guided highlight, fixed layout and help after two misses).
  - A wrong first submit ends the attempt as "not counted". It shows a kind reason without numbers, and nothing is lost. The player may start a new attempt at once. There is no cooldown and no limit.
  - **What "from memory" means per band, deliberately.** Sprout and single-substance Seedling builds show no recipe at all. Seedling Fire still shows the four formulas as coefficient tiles (`CH4`, `O2`, `CO2`, `H2O`) but no atoms and no scale: the Seedling skill is balancing, so the trial tests balancing without help, not recall of the four formulas. Apprentice types the whole equation. The Seedling reaction header says "without the scale" for this reason (Contract strings).
- **Mastery** is unchanged from I-027: `builds >= 3`, `build_days >= 2` (2 different local calendar days) and `predicted`. Each counted trial build calls `unaided_build(i)`, so every one also gives +20 XP. The first one usually moves Learning to Practised (10 + 20 = 30 XP). XP keeps coming after `builds` reaches 3 while `predicted` is false; this is known and harmless for now (Out of scope).
- **The question** comes right after a counted trial build while `predicted` is false and the spell is not Mastered. One question per counted build: the next one comes with the next counted build, on any day.
  - It is chosen from the bank by spell, band and the spell's `asked` count. There are 2 variants per spell and band; the variant is `asked % 2`, and the displayed choice order is rotated left by `asked % 3`. The Adept band uses the Apprentice questions (Smallest version 3).
  - **Known trade-off of the smallest version:** after a wrong answer (which shows the right one), the same question returns two shown questions later, so it can be answered from memory. That repeat is not "new situation" evidence. More variants are the fix (Out of scope).
  - `asked` is bumped when a question is **shown**, so closing the bench to dodge a question still moves to the other variant next time.
  - A correct answer calls `predict(i, true)`. A wrong answer calls `predict(i, false)`, which changes nothing, and then shows the right answer and why. The question is never re-asked straight away, so answers cannot be found by elimination.
- **Keep it sharp** applies to a Mastered spell when `day != 0 && day >= last_build_day + KEEP_SHARP_DAYS`, where `KEEP_SHARP_DAYS = 3`.
  - The bench card then shows "Keep it sharp". A first-try correct trial build (same hiding rules) adds `XP_SHARP = 15` and sets `last_build_day = day`. A wrong build changes nothing. (`last_build_day` is only read by `unaided_build`, which does nothing on a Mastered spell, so reusing it changes no other rule.)
  - When it is not ready, nothing is shown: no countdown, no "in 2 days", no badge.
- **Mastery never fades.** No code path in this item lowers a stage, XP, `builds`, `build_days` or `predicted`. A keep-it-sharp miss and any number of days without play leave Mastered as it is.
- **The day** is I-027's `Book.day`, the one injectable clock. Every rule above reads it through `Book::day()` or takes it as a parameter. Nothing in this item reads `Date`, `Instant` or `SystemTime`. With `day == 0` (unknown or out of range) a trial or keep-it-sharp refuses to start, with a reason (`no_day`), so the 2-day rule cannot be met by accident.

### 2. Hiding every help (enforced in Rust, not in JS)

While a trial or keep-it-sharp attempt is in its build phase, `bench::run` publishes:
- `recipe`: `mastery::trial_recipe(def, band)`.
  - For a single-substance spell (Water, Frost, CO₂) it is `None`, so there is no recipe card, no formula with subscripts and no tile ticks.
  - For a reaction (Fire at Seedling) it is the terms with `formula`, `coef = 1` and `side`, but with `atoms` empty and `state` `None`. The coefficient tiles can still be drawn from the formulas, but `scaleOf` has no atoms, so the balance scale is not drawn.
  - At Apprentice (and Adept, judged as Apprentice) the bench shows no recipe anyway.
- `result` after the submit: the verdict passed through `mastery::strip_help`, which empties `diffs` and keeps `ok`, `code`, `other`, `bad` and `simplest`. So no per-element counts can be shown, whatever the try count.
- No prompt from `spells.json` (the Sprout prompt says "Copy the balls in the picture"). The trial shows its own header line (Contract strings).

### 3. Question bank: `content/predict.json` (new) and `crates/sim/src/mastery.rs` (new)

- **Why a separate file, not a `spells.json` field:** it is 22 nested records with their own kinds and validation, and the lint scans it with its own key list. I-026's "Extending" rule covers flat per-spell fields.
- **Schema, version 1.** Unknown fields at any level are an error.

  ```json
  { "version": 1, "notes": "maintainers only; never shown, never scanned",
    "questions": [
      { "id": "water_sprout_1", "spell": "water", "band": "sprout", "kind": "pick",
        "scene": ["campfire", "bucket"],
        "prompt": "Lots of water goes on a campfire. What happens?",
        "choices": [ {"id": "a", "label": "The fire goes out", "icon": "fire_out", "correct": true},
                     {"id": "b", "label": "The fire gets bigger", "icon": "fire_big"},
                     {"id": "c", "label": "The fire turns to ice", "icon": "ice"} ],
        "why": "Water soaks up the fire's heat. The wood cools down and stops burning.",
        "sources": ["S9", "S11"] } ] }
  ```

  | field | rule |
  |---|---|
  | `id` | `^[a-z0-9_]{1,40}$`, unique |
  | `spell` | an id in I-026's `SpellBook` |
  | `band` | `sprout`, `seedling` or `apprentice`, with `band >= spell.tier_min` (an `adept` question is a `KindBand` error in version 1) |
  | `kind` | `pick` for sprout, `predict` for seedling, `explain` for apprentice (kind and band must match) |
  | `scene` | `pick` only: 1 to 3 ids from `mastery::ICONS` |
  | `prompt` | 1-220 chars; I-026's `BadText` character rule (no `"`, `\` or control characters) |
  | `choices` | exactly 3, ids `a`, `b`, `c` once each, exactly one `correct: true` (`correct` defaults to false); `label` 1-140 chars, same character rule; `icon` required for `pick` (from `ICONS`) and absent otherwise |
  | `why` | 1-300 chars; same character rule |
  | `sources` | non-empty; each `^S[0-9]{1,3}$` (the lint checks they are defined in the register) |

- **Coverage rule:** for every spell and every band from its `tier_min` up to **Apprentice** there are **exactly 2** questions, with ids `<spell>_<band>_1` and `_2`. With today's spells that is 3 × 2 (Sprout) + 4 × 2 × 2 = 22.
- **Adept reads the Apprentice questions.** `pick(spell, Adept, asked)` returns the same question as `pick(spell, Apprentice, asked)`. The Adept follow-up (filed by I-028) adds an Adept kind, its questions and their judge; see Planner notes for what it must carry.
- **Answers** (`judge_choice`, the only judge): correct iff the text, trimmed, is exactly the correct choice's id.
- **`mastery::ICONS`** (Sprout pictures; the web lane draws each as a simple flat SVG, at least 160 px in a choice card): `campfire`, `bucket`, `fire`, `fire_out`, `fire_big`, `ice`, `hot_pan`, `drops`, `steam`, `ln2`, `balloon`, `balloon_small`, `balloon_big`, `flower`, `flower_frozen`, `candle_jar`, `candle_out`, `co2_jug`, `tank`, `bubble`, `bubble_float`, `bubble_sink`.

### 4. The 22 questions (exact text; the contract stub ships them)

`✓` marks the correct choice. **Verdicts** are what the science lane registers (C rows for the ✓ label, the `why`, and every stem that states a fact or a number; Not-claims rows for every other choice and every stem that only asks). The science reviewer checks every verdict and every distractor (test 33).

**Sprout (4-7), `pick`: a picture scene, 3 picture choices; spoken.**

| id | scene | prompt | choices (icon) | why | sources | verdict |
|---|---|---|---|---|---|---|
| water_sprout_1 | campfire, bucket | `Lots of water goes on a campfire. What happens?` | a ✓ `The fire goes out` (fire_out); b `The fire gets bigger` (fire_big); c `The fire turns to ice` (ice) | `Water soaks up the fire's heat. The wood cools down and stops burning.` | S9 S11 | correct |
| water_sprout_2 | hot_pan, drops | `Drops of water land on a very hot pan. What happens?` | a `They turn to ice` (ice); b ✓ `They turn to steam` (steam); c `They catch fire` (fire) | `The pan is hotter than 100 °C, so the water boils into steam.` | S2 S10 | correct |
| frost_sprout_1 | balloon, ln2 | `A balloon goes into liquid nitrogen. What happens?` | a `It gets bigger` (balloon_big); b `It catches fire` (fire); c ✓ `It gets small` (balloon_small) | `The air inside gets very cold. Cold air takes up less space.` | S12 S1 | correct |
| frost_sprout_2 | flower, ln2 | `A flower is dipped in liquid nitrogen. What happens?` | a ✓ `It freezes hard` (flower_frozen); b `It catches fire` (fire); c `Nothing happens` (flower) | `Liquid nitrogen is −196 °C. The water inside the flower freezes solid.` | S1 S10 | correct |
| co2_sprout_1 | candle_jar, co2_jug | `CO₂ is poured into a jar with a candle in it. What happens?` | a `The flame gets bigger` (fire_big); b ✓ `The flame goes out` (candle_out); c `The candle turns to ice` (ice) | `CO₂ is heavier than air. It fills the jar and keeps oxygen away from the flame.` | S5 S8 S19 S14 | simplified (as C-035/C-036: CO₂ lowers the oxygen concentration and also absorbs heat; "keeps oxygen away" is the textbook picture) |
| co2_sprout_2 | bubble, tank | `A soap bubble falls into a tank full of CO₂. What happens?` | a `It drops to the bottom` (bubble_sink); b `It turns to ice` (ice); c ✓ `It floats` (bubble_float) | `CO₂ is heavier than air, so the air bubble floats on top of it.` | S5 S8 | simplified (CO₂ slowly passes into the bubble, which sinks later; the soap film's own weight is small) |

**Seedling (8-10), `predict`: predict an outcome; 3 short text choices.**

| id | prompt | choices | why | sources | verdict |
|---|---|---|---|---|---|
| fire_seedling_1 | `A cold, dry pot lid is held over a methane flame. What will you see on the lid?` | a ✓ `Drops of water`; b `Nothing: it stays dry`; c `Little bits of ice` | `Burning methane makes water: CH₄ + 2O₂ → CO₂ + 2H₂O. The water vapour cools on the lid and turns into drops.` | S7 S10 | correct |
| fire_seedling_2 | `A glass jar is put over a small methane flame. What will happen to the flame?` | a `It keeps burning forever`; b ✓ `It goes out after a short time`; c `It gets bigger and bigger` | `Fire needs oxygen: CH₄ + 2O₂ → CO₂ + 2H₂O. When the oxygen in the jar gets too low, the flame stops.` | S7 S19 S6 | correct |
| water_seedling_1 | `A wet log and a dry log sit next to the same fire. Which will catch fire first?` | a `The wet log`; b `Both at the same time`; c ✓ `The dry log` | `The water in the wet log must boil away at 100 °C before the wood can get hot enough to burn.` | S2 S10 S6 | simplified (as C-019: water also evaporates below 100 °C; a wet surface under a flame stays near 100 °C until its water is gone) |
| water_seedling_2 | `Water is poured on a pan of hot coals. What will you see?` | a ✓ `Steam rises and the coals cool down`; b `The coals get hotter`; c `The water turns to ice` | `The coals heat the water until it boils into steam. That heat leaves the coals, so they cool.` | S9 S2 S11 | correct |
| frost_seedling_1 | `A balloon shrank in liquid nitrogen. It is taken out into a warm room. What will happen?` | a `It stays small forever`; b ✓ `It grows back to its size`; c `It catches fire` | `As the air inside warms up, it takes up more space again.` | S12 | correct |
| frost_seedling_2 | `A little liquid nitrogen is spilled on a warm floor. What will happen to it?` | a `It turns into water`; b `It stays a puddle all day`; c ✓ `It boils away into the air` | `Liquid nitrogen boils at −196 °C. The room is much warmer, so it boils away into nitrogen gas.` | S1 | correct |
| co2_seedling_1 | `CO₂ is poured slowly into a box with three lit candles: low, middle and high. Which goes out first?` | a ✓ `The low candle`; b `The high candle`; c `None of them` | `CO₂ is heavier than air. It fills the box from the bottom up and reaches the low flame first.` | S5 S8 S19 | correct |
| co2_seedling_2 | `Two balloons are the same size. One holds air and one holds CO₂. Both are let go. What will happen?` | a `The CO₂ balloon floats up`; b ✓ `The CO₂ balloon drops faster`; c `They drop equally fast` | `CO₂ is denser than air, so the CO₂ balloon is heavier than the air balloon.` | S5 S8 S12 | correct |

**Apprentice (11-15), `explain`: choose the best explanation; 3 text choices.**

| id | prompt | choices | why | sources | verdict |
|---|---|---|---|---|---|
| fire_apprentice_1 | `A cold, dry beaker is held upside down over a methane flame, and drops form inside it. Which explanation is best?` | a `The heat pulls water out of the glass.`; b ✓ `Combustion makes water, CH₄ + 2O₂ → CO₂ + 2H₂O, and the H₂O(g) condenses on the cold glass.`; c `The flame turns oxygen into water on its own.` | `Every H atom in that water came from the methane. Water vapour turns to liquid when it cools.` | S7 S10 | correct |
| fire_apprentice_2 | `Methane burns inside a sealed flask that sits on a balance. When the flame goes out, the reading is…` | a ✓ `The same: the atoms are rearranged, not made or destroyed.`; b `Lower: the methane was burned away.`; c `Higher: the flame added heat, and heat has mass.` | `CH₄ + 2O₂ → CO₂ + 2H₂O has 1 C, 4 H and 4 O atoms on each side, so the sealed flask keeps its mass.` | S7 | simplified (the heat that leaves takes a mass of E/c², about 10 ng per 890 kJ, far below what a balance can read) |
| water_apprentice_1 | `A paper cup full of water is held over a candle, and the paper does not burn while water is in it. Which explanation is best?` | a `Wet paper is not a fuel.`; b `Flames cannot touch anything that holds water.`; c ✓ `The water takes in the heat and stays at 100 °C or below, so the paper stays far below the temperature it needs to burn.` | `Heat flows from the paper into the water. While the water boils at 100 °C, the paper beside it cannot get much hotter.` | S2 S11 S6 | simplified (paper above the water line can still char; the stem and answer hold where the water touches) |
| water_apprentice_2 | `Firefighters can spray water as a fine mist or as a solid jet. The same amount of water cools hot gases faster as a mist. Which explanation is best?` | a ✓ `Many small drops have much more surface, so they take in heat and evaporate faster.`; b `A mist contains more water than a jet.`; c `A mist is colder than a jet of the same water.` | `Splitting water into fine drops multiplies its surface area. Heat flows in through the surface, and evaporating water takes in a lot of heat.` | S9 S10 | correct |
| frost_apprentice_1 | `Liquid nitrogen poured on a table makes a white fog. Which explanation is best?` | a `The nitrogen is burning.`; b ✓ `The cold turns water vapour in the air into tiny drops.`; c `The table is smoking because it is burning.` | `The fog is water from the air, cooled until it condenses into tiny drops.` | S1 S10 | correct |
| frost_apprentice_2 | `A balloon of air dipped in liquid nitrogen shrinks a lot as it cools. Which explanation is best?` | a `The cold rubber squeezes the air out through its skin.`; b `The air turns into nitrogen.`; c ✓ `Cooling slows the gas particles, so at the same pressure the gas takes up less space (V is proportional to T in kelvin).` | `At constant pressure a gas's volume is proportional to its temperature in kelvin. Going from 293 K to 77 K would shrink a gas to about a quarter.` | S12 S1 | simplified (near 77-90 K much of the air liquefies, so a real balloon shrinks past the gas-law value mostly by condensing; the `why` says "would shrink a gas") |
| co2_apprentice_1 | `CO₂ from an extinguisher sinks and spreads along the floor. Which explanation is best?` | a ✓ `CO₂ (44 g/mol) is denser than air (about 29 g/mol) at the same temperature and pressure.`; b `CO₂ is sticky, so it clings to the floor.`; c `The floor pulls CO₂ down like a magnet.` | `At the same temperature and pressure, equal volumes of gas hold equal numbers of molecules, so density follows molar mass: 44.0 ÷ 29.0 ≈ 1.5.` | S5 S8 S12 | correct |
| co2_apprentice_2 | `A CO₂ extinguisher puts out a small fire. Which explanation is best?` | a `CO₂ reacts with the fuel and makes it fireproof.`; b ✓ `It lowers the oxygen around the flame below what burning needs, and it also takes in heat.`; c `CO₂ freezes the flame solid.` | `CO₂ puts fire out by lowering the oxygen concentration, and it also absorbs heat from the flame.` | S19 S14 | correct |

**Adept (16+):** no questions in this item; Adept reads the Apprentice ones (section 3). The Adept follow-up writes its own, carrying the `fire_adept_1` fix in Planner notes.

Content checks the sim lane pins (test 4): the numbers stated in the text agree with the game's data. `100 °C` agrees with `water_boiling`, `−196 °C` with `nitrogen_boiling` (rounded to the nearest degree), `44 g/mol` with [S5]'s 44.0095 and `about 29 g/mol` with [S8]'s 28.9644 (to 2 significant figures), `1.5` with 44.0 ÷ 29.0 (to 2 significant figures), and "about a quarter" with 77 ÷ 293 (within 0.25 ± 0.02).

### 5. Client glue (`crates/bevy-client/src/bench.rs`, I-028's file, small additive hunks)

- `BenchState` gains `trial: mastery::TrialDesk`.
- The command loop of `bench::run` gains three words (all of them only while the bench is open; anything else is ignored with `warn!`, never `error!`). `i = slot − 1`, `def` is the spell's `SpellDef`, `band` is `BenchState.band`. **The desk calls happen in exactly this order;** the sim lane implements the phase changes in "TrialDesk phases" (Contract), and the client lane makes exactly these calls.
  - `trial <slot>` and `sharp <slot>` (kind `Trial` or `Sharp`):
    1. `desk.close()`;
    2. `r = desk.start(slot, kind, book.spells[i].stage as u8, book.day(), book.sharp_ready(i), bench::learnable(def, band))`;
    3. on `Ok(())`: `pick = slot`, `result = None` (phase `build`);
    4. on `Err(r)`: `desk.refuse(slot, kind, r)` (this is what sets phase `refused` and `refused = r`), `pick = slot`, `result = None`. Nothing in `Book` changes.
  - `try <slot> <text…>` when `desk.open_build(slot)` is true:
    1. `v = bench::judge(spells, slot, band, text)`;
    2. `out = desk.on_build(slot, v.ok)`; `result = Some(strip_help(v))` with `learned: false` (a trial never calls `learn`);
    3. `out == Counted`, kind `Trial`: `xp0 = book.spells[i].xp`; `t = book.unaided_build(i)`; `mastered_now = t` contains a Mastered transition; `desk.note_counted(builds, build_days, xp − xp0, mastered_now)` with the values read back from `book.spells[i]`. Then, only if the stage is not Mastered and `predicted` is false: `asked = book.spells[i].asked`; `q = questions().pick(def.id, band, asked).expect(..)` (coverage guarantees it); `book.note_asked(i)`; `desk.ask(q, asked)`;
    4. `out == Counted`, kind `Sharp`: `book.sharp(i)`; `desk.note_counted(builds, build_days, XP_SHARP, false)`;
    5. `NotCounted` or `Ignored`: nothing more.
    6. Any `Vec<Transition>` returned in 3 or 4 is handled exactly as I-028 handles the one from `learn` (so I-027's held `spell_mastered` sees it).
  - `try <slot> <text…>` while an attempt is open but `open_build(slot)` is false (another slot, or not the build phase): ignored. A `try` with no attempt open is I-028's normal path, unchanged.
  - `answer <slot> <text…>` when `desk.open_question(slot)` is `Some(qid)`: `q = questions().get(qid)`; `c = q.judge(text)`; `t = book.predict(i, c)`; `mastered_now = t` contains a Mastered transition; `desk.on_answer(slot, c, mastered_now, text.trim(), q)`; transitions as in 6. Otherwise ignored.
- `pick`, `open`, `close` and a reset epoch change call `desk.close()`. A closed or abandoned attempt changes nothing it has not already changed: a counted build stays counted, and a shown question stays asked.
- **The view.** It adds `BenchView.trial` (the open attempt, or `None`) and `BenchView.trials` (one `TrialCard` per spell in the list view, `None` otherwise). During the build phase, `recipe` is `trial_recipe(def, band)` (see 2). The change-only publish is unchanged.
- `questions()` is a `OnceLock<QuestionBook>` in `bench.rs`, loaded from `mastery::PREDICT_JSON` against `spells::book()`'s `SpellBook`. A load failure panics with the error. It cannot happen with shipped content, because sim's tests validate the same embedded file.
- No other client file changes: no `main.rs`, `bridge.rs`, `worked.rs` or `hud.js` edit, and no new `__gameState` key.

### 6. Progress additions (`progress.rs`, I-027's file)

- `SpellProgress` gains `pub asked: u8` (default 0). There is no `sharp_day`: keep-it-sharp moves `last_build_day`.
- New constants next to I-027's: `XP_SHARP = 15` and `KEEP_SHARP_DAYS = 3`.
- New `Book` methods:
  - `day(&self) -> u32`;
  - `sharp_ready(&self, i) -> bool` (the rule in 1);
  - `sharp(&mut self, i) -> Vec<Transition>`: when `sharp_ready`, adds `+XP_SHARP` (saturating), sets `last_build_day = day` and bumps `rev`; it never touches `builds`, `build_days` or `predicted`, always returns `[]`, and does nothing otherwise;
  - `note_asked(&mut self, i)`: on Learning or Practised, `asked = asked.saturating_add(1)` and `rev` bumps; on other stages it does nothing.
- **Save, backward compatible.** An entry is written with 8 fields (`…:predicted:asked`) only when `asked != 0`, and with I-027's 7 fields otherwise. So every I-027 expected save string stays byte-identical. `decode` accepts 7 or 8 fields and rejects 6 or fewer and 9 or more; `asked` must be in `0..=255`. I-027's "too many fields" error is the one place in I-027's code this item changes (from "more than 7" to "more than 8"), in `progress.rs` only; `progress.js`'s `SAVE_RE` already accepts it (Assumed from I-027).
- `state_json` is **not** changed (I-027 test 11 pins it). The trial's numbers reach the HUD through `BenchView.trials`.
- `apply_cmd` is not changed. Tests reach these methods through the bench commands and through I-027's debug commands.

### 7. Web (`crates/bevy-client/web/trial.js`, new; small hooks in `bench.js`)

- **List view.** `decorateCards(listEl, trials, send)` appends to each `.bench-spell[data-slot]`:
  - a **trial strip** `.trial-strip`: 3 build pips, 2 day pips and 1 question pip (`.trial-pip[data-kind="build|day|ask"][data-on="true|false"]`), each with a shape (empty or filled circle, a calendar shape for days, a question-mark shape) and an `aria-label` such as "Builds: 2 of 3";
  - the button **`Show what you know`** (`.trial-start`, star-with-tick icon, at least 64 px, 96 px for Sprout) on Learning and Practised spells;
  - on Mastered spells, the star badge, plus **`Keep it sharp`** (`.trial-sharp`) only while `can_sharp`.
  - Unknown spells show neither button.
  - **The rule line** `.trial-rule` (string `rule`): one line above the spell cards, shown once in the list view whenever at least one card has `canTrial` or a Mastered stage. It is the only place the 2-day rule is stated. Nothing else on the list changes.
- **Trial view** (`ui().view === 'trial'`, rendered by `renderTrial(body, state, ctx)` whenever `state.trial` is present):
  - **build phase:** the header line for the band (Contract), and below it `mountBuilder(panel, { band, spell, recipe: state.recipe ?? null, prompt: header, onMake: (t) => send(\`try ${slot} ${t}\`) })`, with exactly these five options. With `recipe` null there is no recipe card and no ticks. With empty `atoms` the scale is not drawn. Sprout headers are spoken (sound on, and the API exists) and have a speaker button.
  - **after a build:** counted shows `Counted! Builds: ${b} of 3. Days: ${d} of 2.` and goes on to the question if there is one. Not counted shows the trial reason (Contract) and `Not counted this time. Practise, then try again whenever you like.`
  - **question phase,** by `question.kind`:
    - `pick`: the scene icons in a row, the prompt (spoken, with a speaker button), and 3 picture cards `.trial-choice[data-choice]` (at least 160 × 160 px, icon plus label).
    - `predict` and `explain`: the prompt and 3 text buttons `.trial-choice` (at least 48 px high, full width).
    - A choice click (or Enter on a focused choice) sends `answer <slot> <id>`.
  - **answer** (phase `answer`): `.trial-answer[data-correct]` shows a tick or cross shape, `Right!` or the wrong-answer line, then `why` (`.trial-why`). After a wrong answer it adds `Your next trial build brings a new question.`
  - **done** (phase `done`: the attempt ended with no question open) shows the counted or not-counted line, or the keep-it-sharp line. **Whenever `masteredNow` is true, in phase `done` or `answer`,** the view adds `.trial-done[data-mastered="true"]`: the spell icon at 160 px with one scale-up (none under reduced motion), `${name} mastered! One click casts it now.` and the rising tone (as in I-028). A keep-it-sharp result shows `Still sharp! +15 XP.` or `Not this time. ${name} stays mastered.`
  - Every phase has **`Back to the list`** (`.trial-close`, sends `pick 0` to cancel, at least 48 px). The bench's own Close and Escape still work.
  - **refused:** `.trial-refused[data-why]` with the string for `no_day` or `not_ready` (Contract), plus Back.
- **Keys.** The bench's capture-phase key handling (I-028) already stops game keys. Inside the trial, digits and letters do nothing, and `1`, `2` and `3` do **not** pick choices (to avoid accidental answers). Tab moves focus among the choices and buttons; Enter or Space activates the focused one.
- **Hooks in `bench.js`** (additive):
  1. `import { renderTrial, decorateCards } from './trial.js'`;
  2. in the render switch, `if (state.trial) return renderTrial(…)` before the build view;
  3. one `decorateCards(…)` call after the list renders;
  4. `ui().view` reports `'trial'`;
  5. **contract step only, and only if the check in "Assumed from I-029" fails:** `mountBuilder` renders with `recipe: null`, or with terms whose `atoms` are empty (no recipe card, no ticks, no scale, and `composeSprout` uses the placement order). The web lane never edits `mountBuilder`.
- `build-web.sh:21` adds `web/trial.js` to the `cp` list.
- **No timer, no countdown, no streak, no notification.** Nothing outside the open bench mentions trials or days.

### 8. Science register and lint

- **Lint:** the lint learns to scan `content/predict.json` keys `questions[].prompt`, `questions[].choices[].label`, and `questions[].why`, the same way I-026 scans `spells.json`. It also checks that every `questions[].sources` id is defined in `## Sources`.
- **Rows:** a `C-` row (next free ids) for each ✓ label, each `why`, and each stem that states a fact or a number, with the verdicts and notes in 4. A Not-claims row for each other flagged string (distractors: "Distractor in predict question `<id>`: offered as a wrong answer and marked wrong after answering; the game never states it as true."; question-only stems: "Question stem: it sets up a situation and asserts no science."). Not-claims rows also cover the trial UI strings in Contract.
- **No new source.** Revision 1's [S21] was only for "Nitrogen gas is colourless"; `frost_apprentice_1` no longer says it.

## Acceptance tests

Each test says how it fails today and on the contract stub. Today none of these files exist. The stub's `QuestionBook::parse` only deserializes, `pick` returns the first question, `judge_choice` returns false, `TrialDesk::start` returns `Err(Refuse::NoDay)`, `strip_help` and `trial_recipe` return their input unchanged, and the new `Book` methods do nothing.

**Sim unit tests** live in `crates/sim/src/mastery.rs` and run with `cargo test -p realgame-sim --no-default-features --locked mastery` (and again with default features).

1. **The shipped bank validates and covers.** `QuestionBook::parse(PREDICT_JSON, &SpellBook::load(..))` is `Ok`. For each spell and band from `tier_min` to Apprentice, `pick(id, band, 0)` and `pick(id, band, 1)` give `<spell>_<band>_1` and `_2`, and `pick(.., 2)` gives `_1` again. `pick(id, Adept, n)` returns the same question as `pick(id, Apprentice, n)` for n = 0, 1. `pick("fire", Sprout, 0)` is `None`. `get("water_seedling_2")` returns that question and `get("nope")` is `None`. There are 22 questions in all. (Fails on the stub: `pick` ignores its arguments.)
2. **Validation rejects bad content.** Each case mutates the shipped JSON and asserts the specific `PredictError` variant:
   - a missing `_2` (`Coverage`); a `fire` sprout question (`Coverage`);
   - `kind: "pick"` on a seedling question (`KindBand`); `band: "adept"` on an apprentice question (`KindBand`);
   - two correct choices, and none (`Choices`); choice ids `a, a, c` (`Choices`);
   - an unknown icon (`Icon`); an `icon` on an explain choice (`Icon`);
   - a `"` in a `why` (`BadText`);
   - `sources: []` and `sources: ["X1"]` (`Sources`);
   - an unknown field, including `answer` (`Json`); an unknown spell id (`UnknownSpell`).

   (Fails on the stub: parse returns `Ok`.)
3. **Judging.** For every question, `judge_choice(q, correct_id)` and `q.judge(correct_id)` are true, as is the same id with surrounding spaces. The two other ids, `""`, `"A"` and `"a b"` are false.

   (Fails on the stub: `judge_choice` returns false.)
4. **The numbers in the text agree with the game's data.** The checks in section 4 ("Content checks"), each written out in the test against the exact substring in `predict.json` (so editing the text without the test fails): `water_boiling` rounds to 100 °C, `nitrogen_boiling` to −196 °C, [S5]'s 44.0095 and [S8]'s 28.9644 round to 44 and 29, 44.0 ÷ 29.0 rounds to 1.5, and 77 ÷ 293 is within 0.25 ± 0.02.

   (Fails today: the file does not exist. On the stub the data is right, so this test passes. **The sim lane's evidence records a mutation run:** change `−196` to `−186` in `frost_seedling_2`, run test 4, see it red, revert. The checker re-runs that mutation.)
5. **The desk.** With stage, day and readiness passed in:
   - `start(slot, Trial, stage 0 | 3, ..)` is `Err(NotLearned | Mastered)`; `day 0` gives `Err(NoDay)`; not learnable gives `Err(NotForBand)`; Learning or Practised with a day gives `Ok` and `view().phase == Build`.
   - `start(.., Sharp, stage 3, sharp_ready false)` gives `Err(NotReady)`. `refuse(slot, Sharp, NotReady)` then gives `phase == Refused`, `refused == Some(NotReady)`, and `on_build` gives `Ignored`.
   - `on_build(slot, true)` on a fresh attempt gives `Counted`, `phase == Done`, `counted == Some(true)`. A second `on_build` gives `Ignored`. On a fresh attempt `on_build(slot, false)` gives `NotCounted`, `phase == Done`, `counted == Some(false)`, and the next `on_build(slot, true)` gives `Ignored` (one submit per attempt). `on_build(other_slot, true)` gives `Ignored`.
   - `note_counted(2, 1, 20, false)` puts `builds 2, days 1, xpGain 20, masteredNow false` in the view without changing the phase. With `mastered_now = true`, `masteredNow` is true and the phase is still `Done`.
   - `ask(q, 0)` after `Counted` gives `phase == Question` and `open_question(slot) == Some(q.id)`. `on_answer(slot, true, false, "a", q)` gives `Answered`, `phase == Answer`, `answer.isRight == true`; a second `on_answer` gives `Ignored`. A fresh run with `on_answer(slot, true, true, ..)` gives `phase == Answer` and `masteredNow == true`. `on_answer` in any other phase, or for another slot, gives `Ignored`.
   - `close` from any phase leaves `view() == None`.
   - `view()` in the question phase never contains a `correct` key or marks the correct choice: its JSON has no `"correct"` substring. `TrialView.phase` serialises as one of `"build"`, `"question"`, `"answer"`, `"done"`, `"refused"`.

   (Fails on the stub: `start` always refuses.)
6. **Help is stripped.** `strip_help` on an `AtomsOff` verdict with diffs gives empty `diffs` and the same `code`, `ok` and `other`. `trial_recipe` gives `None` for water, frost and co2 at every band, and for fire at Seedling gives 4 terms with `atoms` empty, `state` `None`, `coef` 1 and the formulas `CH4`, `O2`, `CO2`, `H2O`. (Fails on the stub: both return their input.)
7. **Rotation.** `QuestionView` for a choice question with `asked = 0, 1, 2` lists the choice ids `a b c`, `b c a` and `c a b` respectively, and shows the label for each id unchanged. (Fails on the stub.)

**Client unit tests** extend `crates/bevy-client/src/progress.rs` and run with `rustc --edition 2021 --test crates/bevy-client/src/progress.rs -o /tmp/progress && /tmp/progress`. **All of I-027's tests 1-11 still pass unchanged.**

8. **Keep it sharp.**
   - A Mastered spell with `last_build_day = 20756` and `set_day(20758)`: `sharp_ready` is false and `sharp` changes nothing.
   - `set_day(20759)`: `sharp_ready` is true. `sharp` gives `[]`, `xp + 15`, `last_build_day = 20759`, the same `builds`, `build_days` and `predicted`, and bumps `rev`. A second `sharp` the same day changes nothing.
   - `set_day(20761)`: not ready. `set_day(20762)`: ready again.
   - `set_day(0)`: never ready.
   - On a Practised spell `sharp_ready` is false.
9. **Mastery never fades (extended).** I-027's test 6 LCG, with `sharp`, `note_asked` and `set_day` (0 to 90 000) added to the operation mix: no operation other than `clear` lowers a stage, XP, `builds`, `build_days` or `predicted`; Mastered stays Mastered for every day value. No operation panics.
10. **Saves stay compatible.**
    - A book with `asked == 0` everywhere encodes byte-identically to I-027 (`p1|water:1:10:0:0:0:0`), including after a `sharp`.
    - `asked = 1` encodes `p1|water:1:10:0:0:0:0:1`, and it decodes back.
    - 6 fields, 9 fields and `asked = 256` are `Err`.
    - Every book from test 9 round-trips through `encode`/`decode`, fits in 512 characters and matches `^p1(\|[a-z0-9:.]+)*$` (I-027's `SAVE_RE`).
11. **`note_asked`** bumps on Learning and Practised (saturating at 255) and does nothing on Unknown or Mastered.

    (Tests 8-11 fail today because the methods and fields do not exist, and on the stub because their bodies do nothing.)

**Node unit tests** go in `test/trial.test.ts` (new) and run with `npm test`. They import `crates/bevy-client/web/trial.js`.

12. `iconSvg(id)` returns an `<svg` string with a `viewBox` for every id in `content/predict.json`'s `scene` and `icon` values, and `null` for `nope`. The test reads the JSON file, so a new icon in content without a drawing fails. **Floor:** each icon's markup has at least 2 shape elements (`path`, `circle`, `rect`, `ellipse` or `polygon`), and no two icons have identical markup. (Whether a 6-year-old can read them is a panel question; the panel brief asks it.)
13. `trialReason(band, result, names)`: `atoms_off` gives `Not quite: the atoms are not right yet.` at every band; `unbalanced` gives `Not balanced yet: count each kind of atom on both sides.`; neither ever contains a digit; `other_spell` gives I-028's `That makes ${other}. This one is ${name}.`; `state_missing` gives I-028's Apprentice string.
14. `pipsOf(card)` gives 3 build, 2 day and 1 ask pips with `on` matching `builds`, `days` and `predicted`, and clamps values above the caps.
15. **No lure text.** Every exported string constant in `trial.js` and every rendered header, button and message for every phase and band (rendered into a fake `document` with each `TrialView` fixture) contains none of `/tomorrow|come back|streak|in \d+ (day|hour)|days? left|hurry|don't lose|expire/i` and none of `/see you|next time|return|later|reward|bonus/i` ("again" is allowed: "try again" is legitimate). The `rule`, `counted`, `sharp right` and `not_ready` strings are named in the test and scanned explicitly.

**Lint tests** (`test/science-sources.test.ts`, new cases):

16. A `predict.json` fixture with an unregistered sentence-shaped `why` fails the lint, and the same with a row passes. A `sources: ["S99"]` fails ("undefined source"). A `C-` row whose `where` is `content/predict.json` and whose text was removed from the file is reported stale. (Fails today: the lint does not read `predict.json`.)

**Golden e2e checks** go in a new function `masteryTrial({ page, st, cmd, until, frames, sleep, base, browser, errors })` at the end of `golden`, called on its own line after I-029's `workedCasts(...)` call and before `no console errors`. They run as `QUERY=?low node tests/e2e/golden.mjs`.
- `bench(s)` is `page.evaluate((s) => window.__bench.cmd(s), s)`, and `bs()` reads `window.__benchState`.
- Each test starts with `localStorage.removeItem('rg.progress.v1')`, a reload, and `prog day 20755` (the injected clock), unless it says otherwise.
- A spell reaches Learning through the bench (`open seedling`, `pick 2`, `try 2 H2O`), as I-028 does.
- They fail today (no bench). On the stub they fail at test 17 (`trial` is refused with `no_day`).

17. **The full path at Seedling, with the injected clock.**
    - Learn Water. `trial 2`: `bs().trial.phase === 'build'`, `bs().recipe` is absent, and the DOM has no recipe card, no `.bench-scale` and no tick.
    - `try 2 H2O`: counted, `builds` 1, `days` 1, `progress[1].st === 'practised'` (xp 30). The question is `water_seedling_1`. Its JSON has no `correct` key.
    - Answer wrong: the test reads the correct id from `content/predict.json` (`water_seedling_1`, the choice with `correct: true`), then picks the first id in `bs().trial.question.choices` that is not it, and clicks that `.trial-choice`. Result: `.trial-answer[data-correct="false"]`, `trial.phase === 'answer'`, the `why` is on screen, and `predicted` is still false.
    - `trial 2`, `try 2 OH2`: counted (`builds` 2, the non-canonical form still counts) and the question is `water_seedling_2`. Answer right: `predicted` true, and no transition yet.
    - `trial 2`, `try 2 H2O`: `builds` 3, `days` 1, no question, still `practised`.
    - `cmd('prog day 20756')`, then `trial 2`, `try 2 H2O`: `trial.masteredNow === true`, `.trial-done[data-mastered="true"]` reads `Water mastered! One click casts it now.`, and `progress[1].st === 'mastered'`.
    - Close the bench, then `castmode worked`: `routes[1] === 'instant'`, and `select 2` then `cast` gives `casts` +1 with `work === null` (I-029's instant cast).
18. **One day is never enough.** Five counted builds and a correct answer, all on day 20755, leave Water `practised` with `days === 1`.
19. **Unknown day refuses.** `cmd('prog day 0')`, then `trial 2`: `trial.phase === 'refused'`, `refused === 'no_day'`, the refusal line is shown, and progress is unchanged.
20. **One submit per attempt; no help.** `trial 2`, `try 2 HO`: not counted, `result.diffs` is `[]`, no `.bench-diff` is in the DOM, and the reason has no digit. A second `try 2 H2O` in the same attempt changes nothing (`builds` unchanged). Starting `trial 2` again and building correctly counts.
21. **Seedling Fire hides the scale.** Fire learned at Seedling. In `trial 1`, `bs().recipe` has 4 terms, all with `atoms` `[]`. `.bench-coef` exists and `.bench-scale` does not. `try 1 CH4 + 2O2 -> CO2 + 2H2O` counts.
22. **Keep it sharp, never fades.**
    - Master Water (I-027 debug commands: `prog learn 1`, `prog worked 1 1` twice, `prog build 1` three times across days 20755 and 20756, `prog predict 1 1`).
    - At day 20758: no `.trial-sharp` on the Water card, and `sharp 2` is refused with `not_ready`.
    - `prog day 20759`: `.trial-sharp` shows. `sharp 2`, `try 2 H2O` gives `xp` +15 and `Still sharp! +15 XP.`. `sharp 2` again is `not_ready`.
    - `prog day 20800`, `sharp 2`, `try 2 HO`: `xp` unchanged, `Not this time. Water stays mastered.`, and still `mastered`.
    - `prog day 79999`: still `mastered`.
23. **The real clock across a reload (no `prog day` command).**
    - A new context with a `Date` shim injected by `ctx.addInitScript` (as golden already does for the throwing storage, `golden:941-944`), so it runs before `progress.js` boots. The shim fixes `Date` at a time read from `localStorage['test.now']`, falling back to the real time.
    - With `test.now = 2026-10-30T12:00:00Z` and timezone `UTC`: learn Water, then 3 counted trial builds and a correct answer give `days === 1`.
    - Set `test.now = 2026-10-31T09:00:00Z` and reload: `trial 2`, `try 2 H2O` gives `mastered`.
    - The context is closed afterwards. This proves `progress.js`'s `localDay` path feeds the rule.
24. **Persistence.** After a shown question (`asked` 1) and a keep-it-sharp, a reload keeps `asked` (the next question is variant 2) and `last_build_day` (not ready again the same day). `localStorage['rg.progress.v1']` has an 8-field Water entry and matches I-027's `SAVE_RE`.
25. **Sprout picture choice.** `open sprout`, learn Frost (`try 3 N2`). In `trial 3`, header `Make Frost from memory.`, no recipe card, `try 3 N2` counts, and the question is `frost_sprout_1`:
    - the scene shows icons `balloon` and `ln2`;
    - there are 3 `.trial-choice` cards, each at least 160 × 160 px, each with an `<svg>` and a label;
    - clicking `c` gives `data-correct="true"`.
    - With sound on, a stubbed `speechSynthesis.speak` was called with the prompt text.
26. **Apprentice explain.** `open apprentice`, learn CO₂ (`try 4 CO2(g)`), `trial 4`, `try 4 CO2(g)`: the question is `co2_apprentice_1` with 3 full-width text choices. Answering `a` is correct.
27. **Adept reads Apprentice.** `bench('open adept')`. Learn Fire with `try 1 CH4 + 2O2 -> CO2 + 2H2O`, then `trial 1` and the same build: the question is `fire_apprentice_1` with 3 full-width text choices. Answering `a` is wrong and shows `Not quite. The answer is: ` plus choice b's label.
28. **Dodging rotates.** After a counted build shows `water_seedling_1`, `pick 0` (Back) without answering. The next counted build shows `water_seedling_2`.
29. **Game keys stay out and the world stays frozen.** In the question phase, with a `.trial-choice` focused, type `R 2 P 3 1`: `trial.phase` is still `question` (no answer was sent), and `state.captionId`, `state.spell` and `state.gameTime` are unchanged and `#pause` did not toggle (I-028's capture handling).
30. **Sizes, contrast, motion (GS-6, GS-18, GS-22).** For every trial phase at each band, at 1280 × 720 and 1366 × 768: every button is at least 48 px (Sprout cards 160, trial button 96 at Sprout); every text element is at least 18 px (Sprout prompt 24); contrast is 4.5:1 or better. Under `reducedMotion: 'reduce'`, `.trial-done` has `animation-name: none`. Without it, every trial animation has iteration count 1.
31. **No lure text in the live DOM.** In every phase of tests 17-28, and in the list view with `.trial-rule` visible, `#bench`'s `innerText` matches none of the test 15 patterns. The list view shows `.trial-rule` with the exact `rule` string once.
32. **Gameplay and the bench unchanged elsewhere.** I-028's `formulaBench` and I-029's `workedCasts` sections pass unchanged. A normal (non-trial) correct build of a Practised spell still gives no XP and no `builds` change.
37. **The trial adds no gate.** Learn Water at the bench and never open a trial. Close the bench, `castmode worked`: `routes[1]` is I-029's worked route (not refused), a worked cast of Water succeeds, and Water's `xp` rises by I-027's worked-cast amount. (Fails only if a lane makes casting depend on trial state.)

**Science, perf, hygiene.**

33. **Science review (B6 protocol, fresh model).** The reviewer checks, against the cited sources:
    - each ✓ label, `why` and fact-stating stem, and its verdict and note;
    - that **every distractor is false** as worded, in the situation of its question;
    - that each question is a situation the game does not show (a "new situation");

    0 errors. `npm test` is green. Before the rows are added (with the JSON already in place), the lint reports each new string as unsourced: this is the expected red step, logged by the science lane.
34. **Golden green.** Every existing check passes 3/3 with `QUERY=?low` in isolation (I-017), with no edit to any existing check. The state key-order check is unchanged, because this item adds no `__gameState` key.
35. **B2/B3 guard (tester).** Against the branch base built the same way: `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs` (or I-024's `perf-pairs.mjs` if merged), 3 interleaved pairs, bench closed. The median CPU frame p50 is at most base + 0.5 ms. Brotli of `pkg-out` grows by at most 30 KB. Recorded in `docs/loop/items/I-030/evidence/perf.md`.
36. **Lane hygiene.**
    - `git diff --name-only origin/main...` lists only paths in Lanes.
    - `main.rs`, `bridge.rs`, `worked.rs`, `hud.js`, `index.html`, `age.js`, `analytics.js`, `boot.js`, `progress.js`, `world.rs`, `chem.rs`, `cues.rs`, `fx.rs`, `feel.rs`, `player.rs`, every `Cargo.toml` and `Cargo.lock`, and `perf.mjs` are untouched.
    - `golden` has 0 deleted lines.
    - `git diff -U0` on `progress.rs` deletes no line of I-027's tests.

## GS walk (screens, controls and effects this item touches)

| GS | P | status |
|----|---|--------|
| 6 | P0 | met: one 300 ms scale-up on mastery, iteration count 1, none under reduced motion (test 30) |
| 7 | P1 | met: the trial lives in the bench, so the game is paused (I-028) |
| 13 | P1 | met: the trial is keyboard operable (Tab between choices, Enter or Space answers), and digits do not pick answers by accident |
| 15, 17 | P0 | met: counted, not counted, right and wrong each differ by shape (tick, cross, pips), text and sound; mastery gets a big icon, a line and a tone |
| 18, 22 | P0/P1 | met: tests 30 and 25 |
| 19 | P0 | met: pips have shapes (circle, calendar, question mark) and labels, not colour alone |
| 20, 21 | P1/P0 | met: Sprout header and prompt are spoken and on screen; Sprout picks pictures, with short labels |
| 25 | P1 | met: no timer anywhere in the trial; answers and builds can take as long as the player likes |
| 26 | P1 | met: changing age keeps all progress, and the band only changes the question's form |
| 27 | P0 | met: neutral tone; teens get explanations, not pictures |
| 28 | P0 | **met, with the reasoning stated for the critic.** The 2-day rule is the owner's learning rule (`target.md`), and it is shown neutrally only inside the open bench, as the line `Master it: 3 builds from memory on 2 different days, and 1 question.` plus pips. There is no "come back tomorrow", no countdown, no streak, no notification, and no reward for returning beyond the rule itself. Nothing is lost by not returning: Learning and Practised spells keep working through worked casts, and mastery never fades. Keep-it-sharp is invisible until it is ready and never says when it will be. Tests 15 and 31 scan for lure wording |
| 30 | P0 | **met, with the reasoning stated for the critic.** The question is not a screen that interrupts play: it appears only after a trial build the player chose, inside the bench. It asks the player to predict a physical outcome of the spell's own chemistry in a new situation (transfer), which is what the owner's mastery rule requires. It never blocks casting, and Practised spells keep their worked casts |
| 31 | P1 | met: minds-on (predict, explain), one goal per view |
| 32 | P1 | met: Sprout questions are pictures with no formula or number in the choices; Seedling formulas appear only inside explanations |
| 1-5, 8-12, 16, 23, 24, 29 | — | unchanged (no camera, aim, cast, HUD or objective change) |

## Out of scope

- **Adept questions.** Cut from this item (critique revision 1). Adept reads the Apprentice questions. The Adept follow-up (filed by I-028) adds the Adept kind, its questions, a number judge and its input, and flips `bandFrom`; Planner notes list what it must carry.
- **XP from repeat trial builds.** Every counted trial build gives I-027's +20 XP, also after `builds` reaches 3 while `predicted` is false, so XP can be farmed there. Known and harmless: XP has no sink until I-039. Capping it needs a change to I-027's `unaided_build`, which this item does not make; I-039 must revisit it before XP buys anything.
- Worked casts counting as unaided builds (decided here: they do not). Any change to I-029's routes, or to I-028's normal bench builds, prompts or reasons.
- More than 2 variants per question, or a question per counted build after `predicted` (later content items may add variants; the validator's "exactly 2" then becomes "at least 2", and the variant becomes `asked % n`).
- Guarding against a midnight crossing or a changed device clock. The day is the local calendar day at boot, as I-027 set it. A player who changes the clock can reach day 2 early; that is the same trust as editing `localStorage`.
- A HUD or hotbar display of trial progress outside the bench (I-039 shrine). Analytics for trials or answers (none; I-027's held `spell_mastered` covers mastery).
- Spaced-repetition scheduling, reminders or notifications of any kind (GS-28).
- Panel harness changes (the day-2 step is a follow-up; see Planner notes). `crates/bevy-client/README.md`.

## Contract

Lands first, on a base that has I-026, I-027, I-028 and I-029 merged, as one commit by the contract builder. It has no behaviour; lanes code against it.

### Files the stub creates or changes

- `content/predict.json`: **the full file**, the 22 questions in 4 (the data is part of the contract).
- `crates/sim/src/mastery.rs`: every public item below, with neutral bodies. `crates/sim/src/lib.rs`: `pub mod mastery;` (always compiled, next to `bench`).
- `crates/sim/src/bench.rs`: `BenchView` gains two trailing optional fields, `#[serde(skip_serializing_if = "Option::is_none")] pub trial: Option<mastery::TrialView>` and `pub trials: Option<Vec<mastery::TrialCard>>`, both `None` wherever I-028 builds a view. The closed-view JSON is unchanged.
- `crates/bevy-client/src/progress.rs`: the `asked` field (`Default` 0), the two constants, the four method signatures with neutral bodies (`day` is real), and nothing else.
- `crates/bevy-client/src/bench.rs`: `BenchState.trial: mastery::TrialDesk` and the `questions()` `OnceLock`. The three command words parse and do nothing.
- `crates/bevy-client/web/trial.js`: exports with `null`-returning bodies. `crates/bevy-client/web/bench.js`: hooks 1-4 in 7, calling the stubs, plus hook 5 only if the `mountBuilder` check in "Assumed from I-029" fails. `crates/bevy-client/build-web.sh:21`: add `web/trial.js`.
- `tests/e2e/golden.mjs`: an empty `async function masteryTrial(…) {}` at the end and its call line.

### `crates/sim/src/mastery.rs`

```rust
pub const PREDICT_JSON: &str = include_str!("../../../content/predict.json");
pub const ICONS: [&str; 22] = [/* the 22 ids in Smallest version 3, in that order */];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind { Pick, Predict, Explain }

#[derive(Clone, Debug, Deserialize)] #[serde(deny_unknown_fields)]
pub struct Choice { pub id: String, pub label: String, #[serde(default)] pub icon: Option<String>, #[serde(default)] pub correct: bool }

#[derive(Clone, Debug, Deserialize)] #[serde(deny_unknown_fields)]
pub struct Question { pub id: String, pub spell: String, pub band: Tier, pub kind: Kind,
    #[serde(default)] pub scene: Vec<String>, pub prompt: String,
    pub choices: Vec<Choice>,
    pub why: String, pub sources: Vec<String> }

#[derive(Debug, Clone, PartialEq)]
pub enum PredictError { Json(String), BadId(String), DuplicateId(String), UnknownSpell(String),
    KindBand(String), Choices(String), Icon(String), BadText { id: String, field: &'static str },
    Sources(String), Coverage { spell: String, band: Tier } }

pub struct QuestionBook { /* private */ }
impl QuestionBook {
    pub fn parse(json: &str, spells: &SpellBook) -> Result<QuestionBook, PredictError>;
    pub fn load(spells: &SpellBook) -> QuestionBook;              // parse(PREDICT_JSON, ..).expect(..)
    pub fn pick(&self, spell: &str, band: Tier, asked: u8) -> Option<&Question>; // variant asked % 2; Adept reads Apprentice
    pub fn get(&self, id: &str) -> Option<&Question>;
    pub fn len(&self) -> usize;
}
impl Question {
    pub fn judge(&self, text: &str) -> bool;                       // judge_choice
    pub fn view(&self, asked: u8) -> QuestionView;                 // choices rotated left by asked % 3; no `correct`
    pub fn answer_view(&self, given: &str, correct: bool) -> AnswerView;
}
pub fn judge_choice(q: &Question, text: &str) -> bool;

pub fn strip_help(v: Verdict) -> Verdict;                         // diffs emptied
pub fn trial_recipe(def: &SpellDef, band: Tier) -> Option<Vec<RecipeTerm>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)] #[serde(rename_all = "lowercase")]
pub enum TrialKind { Trial, Sharp }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum Refuse { NoDay, NotLearned, Mastered, NotReady, NotForBand }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildOutcome { Counted, NotCounted, Ignored }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnswerOutcome { Answered, Ignored }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum TrialPhase { Build, Question, Answer, Done, Refused }

#[derive(Default)] pub struct TrialDesk { /* private: the open attempt, its phase, slot, kind, question id, asked-at-show, last outcome */ }
impl TrialDesk {
    pub fn start(&mut self, slot: u8, kind: TrialKind, stage: u8 /* 0..=3, I-027 order */, day: u32, sharp_ready: bool, learnable: bool) -> Result<(), Refuse>;
    pub fn on_build(&mut self, slot: u8, ok: bool) -> BuildOutcome;
    pub fn ask(&mut self, q: &Question, asked: u8);
    pub fn on_answer(&mut self, slot: u8, correct: bool, mastered_now: bool, given: &str, q: &Question) -> AnswerOutcome;
    pub fn note_counted(&mut self, builds: u8, days: u8, xp_gain: u32, mastered_now: bool); // numbers for the view
    pub fn refuse(&mut self, slot: u8, kind: TrialKind, r: Refuse);
    pub fn open_build(&self, slot: u8) -> bool;
    pub fn open_question(&self, slot: u8) -> Option<&str>;        // question id
    pub fn close(&mut self);
    pub fn view(&self) -> Option<TrialView>;
}

#[derive(Serialize)] pub struct QuestionView { pub id: String, pub kind: Kind, pub prompt: String,
    #[serde(skip_serializing_if = "Vec::is_empty")] pub scene: Vec<String>,
    pub choices: Vec<ChoiceView> }
#[derive(Serialize)] pub struct ChoiceView { pub id: String, pub label: String, #[serde(skip_serializing_if = "Option::is_none")] pub icon: Option<String> }
#[derive(Serialize)] pub struct AnswerView { pub given: String, #[serde(rename = "isRight")] pub is_right: bool,
    pub right: String /* the correct choice id */, #[serde(rename = "rightLabel")] pub right_label: String, pub why: String }
#[derive(Serialize)] pub struct TrialView { pub slot: u8, pub kind: TrialKind,
    pub phase: TrialPhase,
    #[serde(skip_serializing_if = "Option::is_none")] pub refused: Option<Refuse>,
    #[serde(skip_serializing_if = "Option::is_none")] pub counted: Option<bool>,
    pub builds: u8, pub days: u8, #[serde(rename = "xpGain")] pub xp_gain: u32, #[serde(rename = "masteredNow")] pub mastered_now: bool,
    #[serde(skip_serializing_if = "Option::is_none")] pub question: Option<QuestionView>,
    #[serde(skip_serializing_if = "Option::is_none")] pub answer: Option<AnswerView> }
#[derive(Serialize)] pub struct TrialCard { pub slot: u8, pub builds: u8, pub days: u8, pub predicted: bool,
    #[serde(rename = "canTrial")] pub can_trial: bool, #[serde(rename = "canSharp")] pub can_sharp: bool }
```

**TrialDesk phases** (the sim lane implements exactly this; test 5 pins it):

| call | from phase | result |
|---|---|---|
| `start` → `Ok` | any or none (the glue calls `close` first) | `build`, `counted`/`question`/`answer` cleared |
| `refuse` | any or none | `refused`, with `refused = r` |
| `on_build(slot, true)` | `build`, same slot, first submit | `Counted`; `done`, `counted = true` |
| `on_build(slot, false)` | `build`, same slot, first submit | `NotCounted`; `done`, `counted = false` |
| `on_build` | anything else | `Ignored`; unchanged |
| `note_counted` | `done` | sets `builds`, `days`, `xpGain`, `masteredNow`; phase unchanged |
| `ask` | `done` with `counted = true`, kind `Trial` | `question`; stores the question id and `QuestionView` at `asked` |
| `on_answer` | `question`, same slot | `Answered`; `answer`, with `AnswerView` and `masteredNow = mastered_now` |
| `on_answer` | anything else | `Ignored`; unchanged |
| `close` | any | no attempt; `view()` is `None` |

`done` means the attempt ended with no question open (not counted, counted with no question due, a keep-it-sharp result). `answer` means a question was answered; the attempt stays visible until the player leaves. Mastery is shown by `masteredNow`, in either phase, not by a phase of its own.

`AnswerView.is_right` is named so the view's JSON never contains the substring `"correct"` (test 5). `data-correct` in the tests is a DOM attribute set from `isRight`.

### Commands (bench queue; never debug-gated; the answer is judged in Rust)

`trial <slot>`, `sharp <slot>`, `answer <slot> <text…>`, plus I-028's `try` (routed to the attempt when one is open in its build phase) and `pick 0` (Back, which cancels the attempt).

### `progress.rs` additions

`SpellProgress { …, pub asked: u8 }`; `XP_SHARP = 15`, `KEEP_SHARP_DAYS = 3`; `Book::day()`, `Book::sharp_ready(i)`, `Book::sharp(i) -> Vec<Transition>`, `Book::note_asked(i)`. The save format is 7 or 8 fields (Smallest version 6). `state_json` and `apply_cmd` are unchanged.

### `trial.js` exports

`renderTrial(body, state, ctx)`, `decorateCards(listEl, trials, send)`, and the pure functions `iconSvg(id)`, `trialReason(band, result, names)`, `pipsOf(card)`, `headerFor(band, spell, isReaction)`, plus `STRINGS`, the frozen object of every string below.

### DOM ids and classes (web lane; for tests and the panel)

`.trial-rule`, `.trial-strip`, `.trial-pip[data-kind][data-on]`, `.trial-start[data-slot]`, `.trial-sharp[data-slot]`, `.trial`, `.trial-header`, `.trial-say`, `.trial-scene .trial-icon[data-icon]`, `.trial-q[data-kind]`, `.trial-choice[data-choice]`, `.trial-answer[data-correct]`, `.trial-why`, `.trial-done[data-mastered]`, `.trial-refused[data-why]`, `.trial-close`. Styles are appended to `style.css`.

### UI strings (exact; `${name}` is the spell name; all are Not-claims)

| key | text |
|---|---|
| start | `Show what you know` |
| sharp | `Keep it sharp` |
| rule | `Master it: 3 builds from memory on 2 different days, and 1 question.` |
| header sprout | `Make ${name} from memory.` |
| header seedling | `Build ${name} from memory. No recipe this time.` |
| header seedling, reaction | `Balance ${name} without the scale: count each kind of atom on both sides.` |
| header apprentice and adept | `Write ${name} from memory. No help this time.` |
| counted | `Counted! Builds: ${b} of 3. Days: ${d} of 2.` |
| not counted | `Not counted this time. Practise, then try again whenever you like.` |
| reason atoms_off | `Not quite: the atoms are not right yet.` |
| reason unbalanced | `Not balanced yet: count each kind of atom on both sides.` |
| q sprout | `What will happen?` |
| q seedling | `Predict: what will happen?` |
| q apprentice | `Which explanation is best?` |
| right | `Right!` |
| wrong choice | `Not quite. The answer is: ${rightLabel}` |
| next | `Your next trial build brings a new question.` |
| mastered | `${name} mastered! One click casts it now.` |
| sharp right | `Still sharp! +15 XP.` |
| sharp wrong | `Not this time. ${name} stays mastered.` |
| no_day | `Trials need this computer's date, and it looks wrong. Ask an adult to check it.` |
| not_ready | `Your skill is still sharp. Keep playing!` |
| back | `Back to the list` |

The `rule` string is shown once per list view as `.trial-rule` (section 7). The `not_ready` string is shown only when the command is sent directly (tests), because the button is hidden when not ready. It names no time.

## Overlap and merge order

| file | this item | in-flight or earlier item's hunk | resolution |
|---|---|---|---|
| `crates/bevy-client/src/progress.rs` | one field, two constants, four methods, `encode`/`decode` 8-field branch, new tests | **I-027** (owner; merged before this item builds) | additive; I-027's tests are not edited (test 36) |
| `crates/bevy-client/src/bench.rs` | `BenchState.trial`, `questions()`, three command arms, the `try` branch, view fields | **I-028** (owner; merged before) | additive |
| `crates/sim/src/bench.rs`, `crates/sim/src/lib.rs` | two optional `BenchView` fields; one `pub mod` line | **I-028** (`bench`), **I-026** (`spellbook`, `tier`, the `cfg` gates) | additive, after both |
| `crates/bevy-client/web/bench.js` | hooks 1-4 (web lane); hook 5 (contract step, only if the `mountBuilder` check fails) | **I-028** (owner), **I-029** (`guided` option) | after I-029; the web lane never edits `mountBuilder` |
| `crates/bevy-client/build-web.sh:21` | `web/trial.js` on the `cp` line | **I-027** (`progress.js`), **I-028** (`bench.js`), **I-029** (`worked.js`) on the same line | append last |
| `crates/bevy-client/web/style.css` | one appended block | **I-028**, **I-029** appended blocks | append last |
| `tests/e2e/golden.mjs` | `masteryTrial` at the end; one call line | **I-022** `reviewFollowups`; **I-025** `streamsAndFinder` and key list; **I-027** `spellProgress`; **I-028** `formulaBench`; **I-029** `workedCasts` | call order: feel, I-025, I-019, I-022, I-027, I-028, I-029, **I-030**. No key-list edit |
| `content/science-sources.md`, `test/science-sources.test.ts` | rows appended; the `predict.json` scan; no new source | **I-025** Parameters rows; **I-026** `spells.json` scan and `where` cells; **I-028** prompt keys and rows; **I-029** Not-claims rows | append only; take the next free `C-` ids at build time |
| `main.rs`, `hud.js`, `feel.rs`, `analytics.js`, `age.js`, `world.rs`, `Cargo.toml`/`Cargo.lock`, `perf.mjs`, `cues.rs`, `fx.rs`, `worked.rs`, `bridge.rs`, `progress.js`, `boot.js` | **not touched** | **I-022** (`setHint`/`hintAllowed`, `feel.rs`, `Settings`, `aim_and_cast`, `analytics.js`, `age.js`); **I-024** (Cargo profiles, `world::build`/`Builder`, `perf.mjs`); **I-025** (`main.rs` streams, `cues.rs`, `fx.rs`, finder); **I-027**/**I-028**/**I-029** | none |

**Merge order:** I-022, I-024, I-025 (any order) → I-026 → I-027 → I-028 → I-029 → **I-030**. This item cannot build earlier: every file it extends is created by I-027, I-028 or I-029. The spec can be critiqued now. Its contract step re-reads the merged contracts and records the mapping.

## Lanes

| lane | owns | builds |
|---|---|---|
| contract (first) | the files in "Files the stub creates or changes" | The stub. Root `cargo test --locked`, `cargo test -p realgame-sim --no-default-features --locked`, client native `cargo check --locked`, `./build-web.sh` and the wasm clippy pass. Golden passes unchanged on the stub. Runs the `mountBuilder` check ("Assumed from I-029") and adds hook 5 only if it fails. Writes the Refresh note (base sha, any renames from I-026-I-029, the `mountBuilder` check result) |
| sim | `crates/sim/src/mastery.rs` (bodies, tests 1-7) | Tests 1-7 first (red on the stub, except test 4, whose mutation run is recorded in evidence). Then the validator, `pick`, the judges, `view` and rotation, `strip_help`, `trial_recipe` and `TrialDesk`. Must not touch `bench.rs` beyond the stub, `formula.rs`, `spellbook.rs`, `reactions.rs`, `game*`, `web.rs`, `content/**` or `crates/bevy-client/**` |
| client | `crates/bevy-client/src/progress.rs` (bodies beyond the stub, tests 8-11), `crates/bevy-client/src/bench.rs` (the glue in Smallest version 5) | Tests 8-11 first, then the bodies, then the glue. Native and wasm builds, `cargo fmt --check`, wasm clippy `-D warnings`. Must not touch `main.rs`, `bridge.rs`, `worked.rs`, any other `src/*.rs`, `Cargo.*`, `web/**` or tests |
| web | `crates/bevy-client/web/trial.js`, `crates/bevy-client/web/bench.js` (hooks 1-4 only), `crates/bevy-client/web/style.css` (one appended block), `test/trial.test.ts` (new), `tests/e2e/golden.mjs` (the `masteryTrial` body only) | Tests 12-15 first. Then the rule line, the strip, buttons, every phase view, the 22 icons, speech, sound and reduced-motion rules. Golden tests 17-32 and 37 are written against the stub (all fail there at the first `trial`) and recorded. Must not touch `hud.js`, `index.html`, `age.js`, `analytics.js`, `progress.js`, `worked.js`, `boot.js`, the panel harness or any existing golden check |
| science | `content/science-sources.md` (rows), `test/science-sources.test.ts` (the `predict.json` scan, the sources check and test 16), `content/predict.json` (**text fixes only**, after review, with the sim tests re-run) | Test 16 first (red). Then the scan and the rows. The `simplified` notes in section 4 go into the rows' note cells, not only this spec. Runs the reviewer pass for test 33 and fixes any finding in the JSON text and rows. Must not change any existing row, verdict or lint rule |

The sim, client, web and science lanes run in parallel after the stub. **Seam:** the `TrialView`/`TrialCard`/`QuestionView`/`AnswerView` JSON, the command words, and the icon ids. The seam breaker checks that:
- no trial-phase `__benchState` ever carries `recipe` atoms (Seedling Fire keeps its 4 formulas with `atoms: []`, which is allowed), `diffs` or the substring `"correct"`;
- section 5's desk call order is followed exactly (the client calls every method named there, in that order);
- a `try` from a trial can never call `learn` or count twice (one submit per attempt), and a normal bench `try` never calls `unaided_build`;
- `asked` is bumped exactly once per shown question, including when Back is pressed;
- the day used everywhere is `Book.day` (grep: no `Date`, `Instant` or `SystemTime` in `mastery.rs`, `bench.rs` or `progress.rs`);
- every icon id in `predict.json` draws, and every string the UI shows is in `STRINGS` or comes from `predict.json`;
- mastery from a trial flips I-029's `routes[i]` to `instant` on the next frame.

After the shared build, the tester runs tests 34 and 35. The science reviewer runs test 33.

## Planner notes (not human-only)

- **Backlog row to add (leader, before B9 is scored):** `| I-041 | Panel day-2 session: the harness runs a persona's second session after pushing \`prog day <today + 1>\` (allowed without \`debug\`), with the first session's \`rg.progress.v1\` kept, so B9's mastery and from-memory check can be observed | B9 | measurement | I-030; Before: first B9 scoring | proposed | 2026-10-30 |`. If I-041 is taken, use the next free id and update Bars moved. B9 is not measurable without it.
- **Panel brief, one line:** "A spell becomes Mastered at the Formula Bench with 'Show what you know': 3 builds from memory on 2 different days and 1 question. On day 2, the harness has moved the date for you." Add one question for the 4-7 persona: "Could you tell what each picture choice showed?" (icon legibility, test 12's floor does not cover it).
- **Reading the panel's question score:** 70% correct on a 3-choice question is close to a guessing floor (33%, higher with common-sense elimination, and repeats after a miss can be answered from memory). It shows the questions are fair, not that the player transfers the chemistry.
- **Teacher page (GS-29, P2), one line to carry:** "Mastering a spell needs two sessions on different days, so a single lesson will show 'Days: 1 of 2'. Progress is saved in that browser on that computer only."
- The backlog row for I-030 should read "After: I-027, I-028, I-029".
- **The Adept follow-up (filed by I-028) must carry:** flip `bandFrom` and the choosers to offer Adept; add an Adept question kind, 8 questions (2 per spell), a number judge and its input; and **use the game's own Fireball heat**. `content/reactions.json` `methane_combustion` is `CH4(g) + 2O2(g) -> CO2(g) + 2H2O(g)` with `delta_h_kj: -802.3`, so any methane-heat question uses 802 kJ/mol with that row's source, or states that liquid water forms, gives 890 kJ/mol [S4], and registers the difference as a `simplified` note. Revision 1 of this spec had a `fire_adept_1` at 890 kJ; do not reuse it as written.
