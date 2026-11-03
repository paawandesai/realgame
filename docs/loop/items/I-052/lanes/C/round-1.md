# I-052 Lane C, round 1 (wave-15-mac, Paawan's Mac)

Base: `origin/loop/I-052` at 3315e28 (the contract). Branch: `loop/I-052-C`.
Goal for the round (section 12, Sun night and Mon): valley pack, recipes and quest table (C1-C3), intro and ending text, pack UI, Make tab, objective line and arrow, intro camera path, music skeleton, E15 and E17 green against stubs; plus Paawan's partial-mastery rule in the quest model.

## Commits

1. e928590 tests first: bodies for C1, C2, C3, C5, C6 exactly as section 11 words them, and two partial-mastery tests. Against the stubs: `cargo test -p realgame-sim valley` 1 passed, 7 failed, 1 ignored (C4, Tuesday).
2. 5abbfde `valley`: pack, recipes, craft, quest table and per-band lines, `usable`, `practice`. 8 passed, 1 ignored.
3. 849f0be tests first: golden `valleyStory` E15 (intro) and E17 (pack). Against the contract build: 1/3 (both fail; the console check passes).
4. 613b2d5 story glue and the page: `story.rs`, `story.js`, `pack.js`, `music.js`, `sfx.js` (from loop/I-053 at 80fa3a0), the Make section in `bench.js`, the objective in `hud.js`, the WebGL2 card in `boot.js`, the title "Formula", styles. valleyStory 3/3.

## What is in

- **`realgame_sim::valley`:** `Pack` (64 a block, 1 a tool or key; fills matching slots first; `take` all or nothing; frees emptied slots; `to_json`, `entries`), `Item::{key, from_key, label}`, `mat` ids and names (the voxel table plus `furnace` as row 27; a client test checks they match `realgame_voxel::mat`), `RECIPES` (torch: 1 wood + 1 coal -> 4; furnace: 8 stone), `craft` (`NotEnough`, `UnknownRecipe`, `PackFull`), `can_craft`, the quest (`on` advances only on the event that finishes the current step; `at(n)` for `quest <n>` and `?biome=`), `LINES` per step and band (C6: 90 characters at most), `INTRO` cards per band, `biome_name`, `usable` (follows `bench::learnable`).
- **Partial mastery (Paawan):** `valley::practice::RunPractice`: 3 correct worked casts of a spell in a run make it Practised (`record`, `practised`); `stage(spell, saved)` raises the route stage to Practised (2), never lowers it and never makes it Mastered (3, still the 2-day rule). `gate_spell`/`biome_open`: the Frozen Fields need Water Practised, the Marsh Frost, the Crystal Cave CO2. Tested (2 tests).
- **`story.rs`:** the quest, pack (starts with the Pick) and practice as a resource. Its own queue `window.__gameIn.story`: `intro_done`, `craft <id>`, `bind <slot> <key 6-9>`, `worked_ok <spell>`, and with `?debug` `give <item> <n>` and `quest <n>`. `PlayerEvent::Dug` puts the block's drop in the pack (ice melts in your hand; grass gives dirt) and tells the quest; `Placed` takes it out; `InBox{ford}` at z < 150 is `CrossedFord`. A slow camera pan while the intro shows. It fills `story::Keys` (quest, per-band line, pack, `objBearing` to the step's target) and writes `window.__storyState` (intro text for the band, biome name, recipes with have/need, slots with labels, hot keys, practice, last craft) only when it changes. 3 client tests.
- **Page:** `story.js` (3-card aria-modal intro with generated CSS pictures, card dots and "Just play"; keys and clicks are ignored for 1 s, then skip it; Sprout's first card holds 3 s; then `intro_done`; a chime on each completed step and a pickup sound on pack gains), `pack.js` (Tab or I; 20 slots; click a slot then 6-9 binds it; Escape, Tab or I closes; the bench's modal rules), `music.js` (Web Audio sequencer, no files: a calm pentatonic motif per biome over a soft pad, 2 s crossfade on biome change, starts on the first input in the block world, silent with Sound off, `window.__music`), the "Make things" section under the spell list in voxel mode (greyed when short; a click sends `craft <id>`), the objective (biome name, quest line, "n of 7") and the objective arrow (the fire finder with an `objective` class, shown while the target is more than 35° off to the side), the WebGL2 card, `<title>Formula</title>`.

## Checks (this Mac, headless real Chrome for golden, `?low`, served no-store on :8089)

- `cargo fmt --check` (root and client), `cargo clippy --all-targets -D warnings` (root), `cargo clippy --target wasm32-unknown-unknown -D warnings` (root and client): green.
- `cargo test --locked` (root): green, 226 in realgame-sim with 1 ignored (C4). Client `cargo test --locked`: 101 passed.
- `npm test`: 97/97 (science lint 0 unsourced).
- `build-web.sh`: wasm gzip 5 139 746 bytes.
- Golden `valleyStory`: 3/3 (E15, E17, console). `formulaBench`: 55/55, three runs in a row. `workedCasts`: 53/56 with three fails:
  - `worked: W5 W released while the builder is open is released in the game`
  - `worked: W16 then H2O(l) and Enter casts`
  - Both also fail on main 5662cd7 in this environment (measured earlier today on the I-051 lane), so they are not from this lane.

## Deviations and things to know

1. **`crates/bevy-client/build-web.sh` changed (outside Lane C's paths).** It copies a fixed list of web files, so `story.js`, `pack.js`, `music.js` and `sfx.js` would not reach `pkg-out`. I added those four names to the `cp` line (no other change). Please confirm, or have Lane A own the line.
2. **Player text lives in `crates/sim/src/valley`** (quest lines, intro cards, biome names, item labels), as the spec puts the quest lines there. The science lint scans only `crates/bevy-client`, so these strings are not linted today. **Request for Lane B:** register rows for them in the "Block world" section, or extend the lint's scan to `crates/sim/src/valley`. The client-side JS keeps no sentence-shaped text (labels only), so the lint stays green without touching the register.
3. **E17's pause check waits up to 10 frames** for the published state (the bench's own check does the same with `withinFrames(…, 10)`). The page pauses at once; the state JSON follows 1-3 frames later.
4. **A flaky bench check, now steady.** `bench: round 2: #pause is not visible while the bench is open, ...` failed once in two runs. Cause: `story.js` imported `sfx.js`, whose shared instance unlocks audio on the first key, so in the classic courtyard the first B press also created an AudioContext. `story.js` now loads `sfx.js` only once the block world is seen; 3 runs in a row are 55/55.
5. **Class names:** the bench already uses `.bench-recipe` and `.bench-make`; the Make section uses `.bench-crafts` / `.bench-craft*` so classic bench layouts and their checks are untouched.
6. **Shared CARGO_TARGET_DIR between worktrees is unsafe:** a path dependency (`realgame-sim`) got the same build hash in two worktrees and Cargo reused the other worktree's stale artifact ("Fresh"). Use one target dir per worktree.

## Needs from other lanes

- **Lane A (`main.rs`, `voxel_view`):**
  - Send `PlayerEvent::InBox{stamp}` for the ford box (CrossedFord), and the `Dug`/`Placed` events as section 10 says.
  - For partial mastery to count, push `worked_ok <spell 0-3>` on `window.__gameIn.story` (or call `story` directly) when a worked cast is accepted, and pass `story.practice.stage(spell, saved)` into `worked::route` so a Practised-in-run spell gets the one-step confirm.
  - Biome gates: `valley::practice::biome_open(biome, &story.practice)` is ready for a gate hook (or a HUD hint at the biome edge).
- **Lane B (`blockchem`):** the matter events the quest waits for (Tuesday): workshop out by water, gate melted, hollow crossed. `story::on_event` maps them once they exist.

## Not done this round (next)

- The age chooser before the intro in voxel mode (bench first-open), Sprout's always-on arrow with a bobbing marker, the biome toast, the Music slider on the pause card and ducking under speech.
- E16, E18-E21, E23 (Tuesday), C4 and the Invention Bench view, the ending card, `?biome=` pack seeding.
