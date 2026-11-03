# I-052 round 2 (wave 16): the workshop fire, P2, P3 and the Practised gates

Branch `loop/I-052-r2` (worktree `.claude/worktrees/w16-I-052`), from `origin/loop/I-052-preview` (926f2e2), with
`origin/loop/I-052` merged first (b7eafcc). There is no `break-1.md`, so there were no BLOCKING items to fix. The other
cloud thread owns the Crystal Cave, the Kiln, the Invention Bench and the ending (`loop/I-052-late`). This round did not
touch them, with one exception: the cave gate's wall sits at that thread's cave mouth (see "Known gaps").

## Commits

| sha | step |
|---|---|
| 9b13cbc | World and matter for P1-P3: the fuel-store workshop, the flowing river, the ice gate trench and the hollow pond (tests first) |
| 88b9926 | Client: fire billboards, burning keys and the intro fix (P1); P2/P3 quest progress; Practised gates and the one-step confirm (step 4) |
| f9ac236 | P1: a Water cast soaks burning blocks within 2 m (`WATER_SOAK_RADIUS`), sourced; science register rows for the new strings |
| 29f7c06 | The `valleyPuzzles` golden section |
| c4bfcdd | P1 with real spell physics (a client test plans the Water arc from the plaza); golden fixes |
| a70ee99 | Golden: P1 from the planned spot; jumps held long enough for slow frames |
| 88482b1 | P1: a Water arc runs through a puddle onto the block under it (test first) |
| ab3ca0b | P1: three lobbed aims (front, back left, back right); `land()` is test-only |
| ad96daf | Step 4: the one-step confirm is the block world's only, and classic keeps I-029's builder (test first; the full golden's classic `R2a` caught it) |
| d12b33e | Intro: only a click or tap on Just play skips the 1 s guard, and keys skip as before (the full golden's E15 caught it) |
| (notes) | This file and the evidence screenshots |

## What works

**P1, the workshop fire.**
- The workshop is a brick-walled yard with an open roof. In the middle is a fuel store: 3×2×2 wood blocks flagged
  `BURNING|SOURCE`, at x141-143, y35-36, z188-189. Beside it is a plank workbench. The door faces the plaza.
- `SOURCE` on a flammable block means a fuel store: its burn time never counts down, so it burns until water cools it
  below ignition, or until it is smothered.
- This is sourced in `content/science-sources.md` under `p1-fuel-store` (S9, S37, S6, S2, S14, S0). Its last sentence
  covers the soak radius.
- A Water cast puts out every burning block within 2 m of where it lands. A cast is litres of water, which sheets
  over a pile rather than wetting one block.
- Flames and smoke are billboards from fixed pools (`fire_fx.rs`: 48 flames, 24 smoke puffs, 32 steam puffs). They go
  on the burning cells and lit vents nearest the player.
- Steam puffs and the hiss (story.js, when `burning` drops) mark water putting a fire out.
- `burning` is now the voxel count: burning blocks within 192 m. `workshop_burning` counts the burning cells inside the
  workshop stamp.
- Quest step 1 completes when water has put out a workshop cell and `workshop_burning` reaches 0.
- Water flies a gravity arc (I-051). Aimed level through the door, a tap drops onto the floor in front of the door.
  Lobbed over the 2-high wall into the open roof, it lands on the store. A client test proves this on the generated
  world with the client's own arc planner: `voxel_cast::workshop_arc::a_lobbed_water_arc_from_the_plaza_puts_the_workshop_out`.
- A Water arc now passes through water, so the last cast's puddle on the store cannot catch the next cast. Fire, Frost
  and CO2 still stop at water.
- The intro:
  - Just play closes the intro at once, both with a real mouse click and with a Playwright click.
  - The intro never reopens: story.js re-sends `intro_done` while the quest is still at 0.
  - The welcome caption is hidden while the intro is open (`body.intro-open`). It is said again when the quest moves
    from 0 to 1.
  - Golden: both intro checks pass.

**P2, the frozen ford and the ice gate.**
- The river outside the ford is `SOURCE` (flowing) water and does not freeze from Frost. The ford's water is still and
  freezes from the top down. Sourced under `p2-river-flows`.
- The look cell stops at water while a spell is in hand, so Frost can be aimed from the bank.
- Deep water cannot be waded: `too_deep` checks whether the feet cell and the cell above or below are water, and
  `wade_push` pushes the player back. The caption reads "Too deep and icy to wade. Freeze it into a bridge?" (claims
  row C-097).
- Crossing is a position check (north bank, ford columns) and gives quest step 3.
- The ice gate is 4 wide and 3 tall at z93. In front of it is a trench one block down, so the meltwater runs away.
  Melting all its ice gives quest step 4.
- Golden: the gate check, the crossing (in one run of three; see "Golden") and the fire-melts-the-gate check pass.

**P3, the marsh hollow.**
- The boardwalk now stops short of the rim gap.
- The east rim is cliffstone. The single gap is at (196,28,101), and its cell is the only unprotected cell in the rim.
- East of the rim is a sunken pond, so poured CO2 spills out through the gap.
- Placing a block in the gap closes it. CO2 then pools, the six vents go out, and with the player in the hollow the
  objective moves to step 6.
- Golden: the P3 check passes. With the gap open, 3 CO2 casts leave 3 vents lit. After a dirt block is placed in the
  gap with a real press from the hollow floor, 3 more casts put out every vent. The CO2 maximum is 0.886 and the quest
  reaches step 6.

**Step 4, Practised gates and the one-step confirm.**
- A correct worked cast, or a cast with nothing to build, is logged in `Working::cast_log`. `story::practice_sync`
  feeds it to `RunPractice`.
- Three casts in a run make a spell Practised, with the caption "{name}: Practised! Its cast is now one quick confirm."
- `Working::stage` raises the stage to Practised for routing, `work_view` and the published stages. worked.js then shows
  "You know this one: H₂O(l)" and one Cast button that sends the formula.
- `gates.rs` places three runestone walls at startup, each a protected box:
  - the Frozen Fields gate (the ford mouth) needs Water;
  - the Marsh gate (the east pass) needs Frost;
  - the Crystal Cave gate needs CO2.
- Each wall has an in-world sign that the page projects ("Practise Water 3 times to open (n/3)", shown within 40 m).
  When a gate's spell becomes Practised, the wall sinks with "The runestone gate to X sinks into the ground!".
- A saved stage of Practised or above opens the gate too.
- The confirm is for the block world only (`mode` is `voxel` in `__gameState`). The classic courtyard keeps I-029's
  builder for Practised, unchanged.
- Golden: three checks pass:
  - "correct worked Water casts (three at most) make Water Practised and the next press is one confirm";
  - "the ford is gated until Water is Practised; its sign counts the casts; three casts open it";
  - "the Marsh gate … the ford's Frost casts open it".

## What does not work, or is not proven (honest)

- **P1 in the browser does not finish.** The golden check "P1 water on the workshop puts it out with real presses and
  the objective moves on" fails, and so does "P1 the workshop stays out".
  - In the last full run, the burning count went from 14 to 4 after 8 worked casts (2 built, 6 one-step confirms),
    the quest stayed at step 1, and 7 were burning 5 s later.
  - In the debug runs, held-0 casts from the planned spot showed the pattern. The first lobbed cast takes the workshop
    from 14 burning to 4, and later casts leave 2 to 4 burning, which relight neighbours between casts.
  - The native client test plans the same arcs from the same spot with the client's own planner and the same matter
    sim, and it goes out in 2 to 3 casts.
  - I did not find the difference before the deadline. Suspects:
    - at about 2.5 fps headless the matter sim takes one 0.1 s step per frame, and it keeps stepping while a worked
      cast's builder pauses the game (`blockchem::drive` uses wall time, not game time);
    - the arc landing in the browser may not match the native plan.
  - So: a player can see the fire, learn Water, and knock it down to a few burning blocks, but in the headless build
    the workshop is not proven to go fully out, and quest step 1 is not proven to complete in the browser.
- The P2 crossing was flaky headless. Before the fix it passed in one of three runs. In the other two the player
  stopped on the last ice row because the jump up the far bank's one-block step was missed (a 90 ms Space press at
  0.4 s frames). With the jump held for 450 ms it passed in the last full run (one run).
- The saved XP rule from I-029 can make Water Practised after the bench and two worked casts, before the run's three.
  The step 4 check accepts two or three.
- The P2 golden teleports. It goes to the ford bank and to the ice gate's trench, and it does not walk over the Frozen
  Fields' hills. P1 walks to the workshop, then teleports onto the exact spot the aims are planned from.
- The P3 golden uses a debug `give dirt 2` instead of digging the block (digging is E4's check). The quest steps for P2
  and P3 are set with the debug `quest <n>` story command.
- The cave gate's wall ([218,30,56]-[223,36,61]) is at the late thread's cave mouth, and that thread may move the mouth.
  The late thread's ladder exit can bypass the gate.
- Tests first: the matter, rules and voxel tests and the two client arc tests were red first (output below). The rest
  were written with the code or after it:
  - the client tests for fire_fx, gates, story progress, wade and the bench;
  - the JS test for the one-step confirm;
  - the golden section.
- Pre-existing native `--all-targets` clippy errors in files this round did not touch (progress.rs, worked.rs,
  bridge.rs) are still there. The required checks (clippy for root native and wasm, and for the client's wasm build)
  pass.

## Red first (test output before the code)

- `the_workshop_burns_on_until_water_puts_it_out` panicked at crates/matter/tests/matter.rs:406 (the workshop burnt out
  by itself).
- `fixture_a_fuel_store_burns_until_water_cools_it` panicked at rules.rs:105, left 21 (ash), right wood.
- `the_workshop_is_a_burning_fuel_store_open_to_the_sky` panicked at crates/voxel/tests/voxel.rs:681.
- `fixture_flowing_river_water_does_not_freeze_but_the_still_ford_does`: the flowing river stays liquid (left 11 ice,
  right 12 water).
- `the_workshop_goes_out_with_six_casts_from_the_door`: out after 6 casts from the door, left 10, right 0. The back row
  relit the front. Fixed by `WATER_SOAK_RADIUS`.
- `a_water_arc_lands_through_a_puddle_on_the_store`: left Some([142, 37, 190]), right Some([142, 36, 189]). The arc
  landed on the last cast's water. Fixed by `stops_water`.

## Test results

- `cargo fmt --check` (root and client): ok.
- Clippy `-D warnings`: root native ok, root wasm32 ok, client wasm32 ok. The client's native `--all-targets` clippy
  still has 8 errors, all in files this round did not touch (progress.rs, worked.rs, bridge.rs).
- `cargo test --locked` (root): every test passes.
  - sim: 226 passed.
  - matter: 17 passed, 1 ignored (`lift_sign`, the known spec error).
  - matter rules: 11 passed.
  - voxel: 15 passed, 2 ignored (V3/V4, Lane A).
  - Others: 7 and 2 passed.
- Client native tests (`crates/bevy-client`, `cargo test --locked`): 138 passed, 1 ignored.
- `npm test`: 98 pass, 0 fail.
- `build-web.sh`: ok, wasm gzip 5 287 926 bytes.
- `git push`'s pre-push hook (fmt, clippy native and wasm, cargo test, typecheck, npm test, the web build): passed.
- Golden, last full run (`QUERY=?low`, `ONLY=formulaBench,workedCasts,voxelEngine,blockMatter,valleyStory,voxelCast,valleyPuzzles`):
  **120 pass, 8 fail**, no console errors. The web build was rebuilt during this run, after P2's crossing, for the
  two fixes below. Its only changes were the classic-only confirm gate and the intro's key handling. The P2 gate,
  Marsh and P3 checks ran on the new build.
  - `valleyPuzzles`: 9 of 11 pass. These fail:
    - `puzzles: P1 water on the workshop puts it out with real presses and the objective moves on` (burning 14 -> 4,
      quest 1);
    - `puzzles: P1 the workshop stays out` (burning 7).
  - `formulaBench` and `workedCasts` (classic): 102 pass, 2 fail.
    - `worked: R2a a click right after an accepted Make casts nothing more and opens no builder for 10 frames`.
      **Caused by this round**: a Practised Water opened the one-step confirm on the courtyard. Fixed in ad96daf
      (test first). Re-run pending (see below).
    - `bench: Sprout learns Water with the mouse` (error: no `#bench .bench-again`). bench.js was not touched. Not
      yet shown whether this is this round's doing or a timing flake. Re-run pending.
  - `voxelEngine`, `valleyStory`, `voxelCast`: these fail:
    - `voxel: the Pick digs a dirt block into the pack and placing puts it back` (digT max 0.00). It passed in
      integrate-1.
    - `story: the intro shows three cards and any key skips it after 1 s`. **Caused by this round**: Just play has
      focus, so the new Just play branch took every key. Fixed in d12b33e. Before this round it already failed on
      this sandbox for timing (integrate-1).
    - `story: the pack opens with Tab, pauses, and Escape closes it`. It ran right after E15 on the same page, with
      the intro still open.
    - `voxelCast: Water tapped lands 10-13.5 m away and a full 1 s charge lands 23-29 m away, on the ground at the
      feet`. The 1 s charge landed on the Kiln terrace's step face at (128.7, 36.47, 180), which is terrain this round
      did not change. I did not check it on the base commit.
  - Re-run of `formulaBench,workedCasts,voxelEngine,valleyStory,voxelCast` with both fixes: see the last section.

## Shared-file edits

- `crates/bevy-client/src/main.rs`:
  - `mod fire_fx; mod gates;`.
  - `Working` gains `cast_log` and `practised` plus `fn stage` (Practised raise), used for routes, `work_view` and the
    published stages.
  - The instant route and `Submit::Cast` push to `cast_log`; `publish_work` caps it.
  - In voxel mode, `burning` publishes the matter count, and `ventsLit` is published safely (the count starts at
    `u32::MAX`).
- `crates/bevy-client/src/blockchem.rs`:
  - `Keys` gains `burning`, `workshop_burning`, `vents_lit` and `gate_ice` (manual `Default`).
  - New: `VIEW_M`, `count_near`, `workshop_burning`, `count_in`.
  - `drive` is now `pub(crate)`.
  - The plugin adds `fire_fx` and the `Wade` resource with `wade_push`; `WADE_LINE` and `too_deep` are new.
- `crates/bevy-client/src/story.rs`:
  - New fields: `water_on_workshop` and `gates_json`, published as the `gates` key.
  - New: `fire_progress`, `crossing_progress`, `hollow_progress`, `practice_sync`. The welcome line is said again when
    the quest moves from 0 to 1. The plugin adds `gates`.
- `crates/bevy-client/web/hud.js`: not touched.

Other files changed:
- the client: `fire_fx.rs` (new), `gates.rs` (new), `voxel_cast.rs` (water passes water; tests), `voxel_view.rs`
  (`WELCOME_LINE`, look cell at water), `bench.rs` (the block world's bench station);
- the web page: `web/story.js`, `web/worked.js`, `web/style.css`;
- the root crates: `crates/matter`, `crates/voxel`;
- content and tests: `content/science-sources.md`, `test/bevy-worked.test.ts`, `tests/e2e/golden.mjs`.

No golden checks were retired this round, and none were weakened. The new step 4 check accepts two or three casts
because of the existing saved XP rule (above).

## Evidence

Screenshots from the last golden run are in `docs/loop/items/I-052/evidence/round-2/`:
- `p1-01-start.png`, `p1-02-fire.png`, `p1-03-out.png`: the start, the fire, and the fire after the casts;
- `p4-01-practised.png`: the one-step confirm;
- `p2-01-gate.png`, `p2-02-across.png`, `p2-03-gate-melted.png`: the ford gate and its sign, across, the gate melted;
- `p3-02-hollow.png`, `p3-03-smothered.png`: the hollow, and the vents smothered.

## Re-run with the two fixes

**Not run.** The re-run of `formulaBench,workedCasts,voxelEngine,valleyStory,voxelCast` waited on the shared Chromium
lock for more than 80 minutes, because the other cloud thread's goldens held it, and it was stopped when this round
ended. So these are still open:
- `worked: R2a`: fixed in ad96daf. The JS test is red first and then green, but the golden has not re-run.
- `story: the intro shows three cards and any key skips it after 1 s`: fixed in d12b33e. The golden has not re-run.
  Before this round it already failed on this sandbox for timing.
- `story: the pack opens with Tab, pauses, and Escape closes it`: most likely a knock-on of E15. Not re-run.
- `voxel: the Pick digs a dirt block into the pack and placing puts it back`: cause not found. Not re-run.
- `bench: Sprout learns Water with the mouse`: cause not found. Not re-run.
- `voxelCast: Water tapped lands 10-13.5 m away and a full 1 s charge lands 23-29 m away, on the ground at the feet`:
  terrain this round did not change. Not checked on the base commit.

The next smoke step should run these sections and `valleyPuzzles` on d12b33e or later.

## What a player can do, from the intro on

- Just play closes the intro at once. The welcome line shows after it, not under it, and the intro does not come back.
- The workshop burns with flames and smoke from the start, and keeps burning.
- At the Formula Bench by the plaza, the player learns Water.
- Lobbing Water over the workshop's low wall onto the burning store knocks the fire down with steam and a hiss. In the
  headless browser it was not shown to go fully out, so whether quest step 1 completes in play is unproven.
- The ford north is shut by a runestone wall with a sign, "Practise Water 3 times to open (n/3)". Three correct Water
  casts open it, and Water's worked cast becomes one Cast press.
- From the bank, Frost freezes the still ford into ice that the player walks across. The flowing river beside it does
  not freeze, and the deep water cannot be waded.
- Those Frost casts also open the Marsh's east-pass gate.
- Fire melts the ice gate in the cliff.
- In the methane hollow, CO2 poured with the rim gap open spills out. A block in the gap lets the CO2 pool and smother
  every vent, and the objective moves on.
- The Crystal Cave gate needs CO2 Practised. Everything past the cave mouth is the other thread's.
