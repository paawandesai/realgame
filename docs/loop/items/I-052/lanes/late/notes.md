# I-052 late lane: Crystal Cave (P4), Kiln finale (P5) and the ending

Branch: `claude/project-thread-21govy` (this session can push only there; it stands in for `loop/I-052-late`).
Base: `origin/loop/I-052-preview` at 926f2e2. The main thread (`loop/I-052-r2`) owns P1-P3, the Practised gates
and the round-1 break fixes, and merges both branches.

## What is in

- **`realgame_sim::valley::invent::judge` (C4, un-ignored, body written test first).** Items on the bench become
  species (coal, wood and planks = C, O2 flask = O2, H2 balloon = H2, water = H2O). The hottest exothermic reaction
  in `content/reactions.json` that uses only those reactants wins; the Kiln relights at a flame of 1700 °C or more
  (fuel in pure O2: 2800 °C, the same value as `grid::FLAME_C_IN_O2`, which is behind the `game` feature). Kind
  reasons for coal alone ("needs oxygen ... Air has only 21 %"), hydrogen alone, O2 alone ("needs a fuel"), water
  ("nothing to burn") and non-reactants. Apprentice and Adept may type the equation: parse, arrow, balance, then
  the book (refused as `Unbalanced` with the atom counts when unbalanced; `C + O2 -> CO` is never called
  impossible), and the typed reactants must be in the pack.
  - **Deviation:** the spec says the unbalanced refusal uses `atoms_off`; the Formula Bench's own reaction judge
    uses `Unbalanced` for an unbalanced equation (`bench.rs` `judge_reaction`), so this follows the bench.
- **M9 `lift_sign` un-ignored and corrected:** CH4 gives 10.4 N of buoyancy against a 14.7 N payload, so it cannot
  lift the 1.5 kg crate (Lane B's note). The test now asserts CH4 < 0 for the crate and > 0 for an empty balloon.
- **`crates/bevy-client/src/late.rs` (new, Voxel mode only).** Pure model `Late` (unit tested natively) plus the
  Bevy side:
  - **Dark cave:** `covered` (solid rock within 30 blocks above the eye) eases ambient light 700 → 120 and the sun
    to 8 % over about a second. A torch in the pack lights a warm point light at the player; the nearest placed
    torch within 12 blocks gets one too (scanned twice a second, only underground).
  - **Rig (P4a):** E within 3 m of the rig's glass cell. Too dark without light ("Make a torch at the Formula Bench
    (wood + coal)"). With light: 4 s of electrolysis; two tubes fill 2:1 (40 mL H2, 20 mL O2); gives an H2 balloon
    and an O2 flask. It will not run again while you carry both.
  - **Balloon crate (P4b):** E at the crate opens a chooser (H2 or O2). The sign of `realgame_matter::lift_n` (2 m³,
    1.5 kg) decides: O2 sags for 3 s, then you untie it and keep the flask; H2 rises at 1.5 m/s to the ledge, the
    Spark rolls in, the rope lowers it at 2.5 m/s, and E takes the Kiln Spark (quest step 6 → 7 through
    `QuestEvent::Holding`).
  - **Kiln door:** E at the runestone door with the Spark turns its runestone cells to air.
  - **Invention Bench (P5):** E at the bench opens the panel (pack items that react, up to 3; a typed equation for
    Apprentice and Adept). A relighting reaction with the Spark in the pack burns one of each reactant and the
    Spark, sends `QuestEvent::Invented { relights: true }` (step 7 → 8), opens the door, sets the hearth `LIT`,
    shows a roaring fire light and chimney smoke, and steps the player back onto the terrace facing the Kiln.
  - **Ending:** after 1.5 s the card: "You relit the Great Kiln!", time played, the five puzzles, **Keep exploring**
    (closes it, play goes on) and **Play again** (reload). `outcome` turns "won" (see main.rs below), so the
    existing analytics `quest_complete` fires once with quest 8.
- **`web/late.js` (new):** the E prompt chip, the message line, the gas chooser, the Invention Bench panel and the
  ending card, with the bench's modal rules (aria-modal, page behind inert, Escape closes, focus back to the canvas).
  Styles appended to `web/style.css` under a `late-` prefix.
- **Golden `valleyLate`** (`ONLY=valleyLate`): L1 dark rig room needs a torch; L2 the rig fills 2:1 and gives both
  gases; L3 (E13) O2 does not lift the crate, H2 does and brings the Spark down; L4 the Kiln door opens with the
  Spark; L5 (E19) coal alone fizzles, coal + O2 lights the Kiln, `ending`, `outcome: won`, card with Keep exploring;
  L6 Keep exploring closes the card.

## Shared files touched (kept minimal)

| File | Change |
|---|---|
| `crates/bevy-client/src/main.rs` | `mod late;`, `late::plugin` in the Voxel plugin list, and in `publish` voxel mode's `outcome` is "won" once `story::Keys::ending` is true (spec section 8), else "playing" |
| `crates/bevy-client/web/hud.js` | the courtyard's win banner stays hidden in voxel mode (`state.mode === 'voxel'`); otherwise the ending showed "You saved the greenhouse!" behind the card |
| `crates/bevy-client/build-web.sh` | `web/late.js` added to the `cp` line |
| `crates/bevy-client/web/boot.js` | one `import('./late.js')` line after `music.js` |
| `crates/bevy-client/web/style.css` | appended `late-*` rules at the end |
| `crates/voxel/src/gen.rs` | one protected box in `protect()`: the shaft and ledge plus 2-3 blocks around, up to the sky |
| `crates/sim/src/valley/mod.rs` | `mat::PLANKS`, `mat::WATER`; `invent` body and C4 body |
| `crates/matter/tests/matter.rs` | M9 corrected and un-ignored |
| `tests/e2e/golden.mjs` | `valleyLate` appended, one `onlyRuns` entry, one call after `voxelCast` |

Not touched: `story.rs`, `blockchem.rs`, `voxel_view.rs`. `late.rs` reads `story::Story` (its public
`quest`) and `crate::Pack`, and orders its systems after `story::on_event`.

## Open / for the merge

- Round 2 (main thread's asks): `?biome=cave` starts at step 6 with 2 coal and 1 wood (enough for a torch, no torch);
  `?biome=kiln` at step 7 with coal, a torch, an O2 flask, an H2 balloon and the Kiln Spark (`late::seed`, golden L0
  and L7). The cave route from the mouth to the rig room and into the shaft is checked by a walker search on the
  generated world (`the_cave_route_is_walkable`). The shaft, the ledge and the rock around and above them up to the
  sky are now one protected box in `voxel::gen::protect` (lane A's file), so the Spark cannot be dug down to
  (`the_ledge_cannot_be_dug_or_built_around`; break finding from the main thread: (244, 27..31, 20)).
- The golden still teleports between places; a real playtest of the walk is worth doing.

## Checks (cloud sandbox, headless Chromium, `QUERY=?low`)

- Golden `ONLY=valleyStory,valleyLate`: **8/9**. All six `valleyLate` checks and the console check pass. The one
  fail is `story: the intro shows three cards and any key skips it after 1 s` (E15), which already fails on this
  sandbox before this lane (integrate-1.md, "E15 on this machine": the harness cannot press inside the first second
  at about 1.6 fps).
- Unit tests: `late` 6/6 (client), `valley` C4 and M9 green. Full results in the push commit message.
- Evidence: `evidence/late/` (rig filling, balloon lifting the crate, the ending card, the relit Kiln with smoke).
