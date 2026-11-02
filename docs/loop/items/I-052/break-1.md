# I-052 break, round 1 (wave 15)

Breaker: a fresh detached worktree at `origin/loop/I-052` 8cec098 (integrate round 1). I did not write this code.
Build: `build-web.sh` ok (wasm gzip 5 219 694 bytes). Personas: a 6-year-old, a 13-year-old who plays Minecraft, and
a game-jam judge. Probes were a Playwright script against `web/pkg-out` (`?low&debug`), plus two throwaway native
tests on the generated world. Both tests were deleted afterwards and nothing under `crates/` was changed.

**BLOCKING (for round 2, before the quest listens to matter events): 1 item, B1.**

## Golden (only the sections asked for, `QUERY=?low`)

`ONLY=voxelEngine,blockMatter,valleyStory,formulaBench,spellProgress,workedCasts`: **127/128**, exit 1.
- voxelEngine: E1-E5 pass. E1: 31 chunks meshed 2.8 s after the first state.
- blockMatter: has no checks yet.
- valleyStory: E17 passes; E15 fails.
  - E15 fails in the same way the integrator reported: `early.shown=false`. The harness's "early" key lands more
    than 1 s after the intro shows, at 1.8 fps on this sandbox. This is the known sandbox timing, not a new
    regression.
- formulaBench, spellProgress and workedCasts (classic): all 54 + 12 + 54 pass, including both "a missing
  bench.js / progress.js does not stop the boot" checks.
- No console errors.

These sections have no `ONLY=` entry, so this run did not re-check the integrator's two classic failures
(`clock: cast cadence`, `cast: a real click casts exactly once`).

## Blocking

### B1. The workshop fire (P1) burns itself out in about 45 s, which makes quest step 1 impossible to finish

Native run, `generate(DEFAULT_SEED)` + `matter::World::step` with no emits:
- burning cells in the workshop box: 12 at t = 0, 36 at 1-4 s;
- 0 at about 45-50 s, and 36 cells have turned to ash;
- no `Extinguished` events.

In the browser, `probe 138 36 190` (a workshop wall cell) read `air` a few minutes after load.

Section 7's step 1 is done only when no `BURNING` cell is left **and** at least one was put out by water. So:
- a player who first walks to the Bench to learn Water (spec: 30-60 s per build) finds the fire already out;
- step 1 then can never complete, and the whole story is soft-locked at "1 of 7".

Today nobody sees this, because `story::on_event` does not use matter events yet (Lane C, round 2). It becomes a hard
soft-lock as soon as that wiring lands.

Fix: Lane B, or A for the stamp, owns it. Options:
- make the stamp's fire last until it is doused, as a `gas_vent` does (for example a burning fuel core that does not
  turn to ash);
- or give wood a much longer `burn_s` inside the workshop box;
- and/or let step 1 also complete if the fire burns out. The spec's "at least one put out by water" rule is the
  part that locks.

Add a unit test: after 120 s of steps with no emits, the workshop is still burning.

## Should fix

### S1. Puzzle boxes are too small: every gate except the ice gate can be dug around or built over (spec risk 4)

Checked with `probe` (its `protected` field) and native `can_place` / `can_break`. The y-36 bridge was also done
for real in the browser.
- **P2a ford.**
  - The `FORD` box stops at y 36 (exclusive). I stood on a pillar on the south bank (z 158, outside the box) and
    placed a dirt block at **(128, 36, 157)** over the river with key 6. The game accepted it, and I stood on it
    at y 37 over the water. About 12 dirt blocks bridge the whole river with no chemistry.
  - The box also starts at y 26: `(128, 25, 154)` is unprotected sand under the protected bed, so you can tunnel
    under the river.
  - The bank cells at z 148-149 and 158-159 inside the ford's x range are diggable too, so the ford's water can
    be drained into a ditch.
- **P2a, before wade_push exists.** From the north bank, holding W walked me along the riverbed underwater to
  (128.5, 28, 157.65), against the south bank, with no push back and no caption. The bank at z 158 is diggable,
  so you can dig steps out. E9 / `wade_push` is still to come (Lane B). When it lands it must cover the whole
  ford's water, including the bed.
- **P3 hollow.** The `HOLLOW` box is y 26..34. `(190, 34, 101)` is placeable, so an 11-block bridge at y 34 crosses
  the hollow over lit vents. `(190, 25, 101)` is diggable, so you can tunnel under it.
- **P4 balloon ledge.** `(244, 27..31, 20)` (stone, mud, grass) are all diggable, and the ledge's air at y 25-26
  is directly below. Dig straight down from the Marsh surface and you land on the Kiln Spark's ledge without the
  rig or the balloon.
- **Cliff band and ridge.** These are protected at full height but are only 58-61 / 60-64 tall. A pillar of about
  20 blocks lets you walk over the band, skipping the ice gate. That costs much more dirt, so it matters less.

Suggested fix (Lane A):
- extend the ford, hollow and ledge boxes, from the bedrock or a few blocks below up to the world top
  (`all_y`, the way the canyon does);
- protect the ford's bank columns in z 148-150 and 158-160;
- add one "dig-around" probe to each golden puzzle check, as risk 4 already promises.

V4 should also assert that the cells above and below each box are covered.

### S2. Without WebGL2, the card shows, but Bevy still panics behind it

Test: `getContext('webgl2')` returns null and `navigator.gpu` is undefined.
- `#nogl` shows full screen and is the top element. Good.
- The text is "No 3D here: try Chrome on a recent laptop". The spec says "Your browser can't run 3D here. Try
  Chrome on a recent laptop."
- `init()` still runs and logs two errors, `panicked at bevy_render ... Failed to create wgpu surface ... webgl2 not
  available` and `pageerror: unreachable`.

Fix: skip `init()` when the card is shown, so that console stays clean, and use the spec's wording (Lane C,
`boot.js`).

### S3. Being pushed out of a solid block teleports the player to the top of the whole column

`motion::step` resolves y against each overlapping box in turn, so an embedded player climbs the whole stack:
- `tp 128.5 0.2 205.5` and `tp 128.5 20 205.5` (inside rock) both came out at y 35, the surface;
- a stone or ice block set at head height popped the player 2 m up, on top of it.

On the surface this is harmless. In the roofed Crystal Cave, though, a Frost that freezes water around the player
would lift them through the cave roof onto the Marsh surface. Lane B's freezing should not turn a cell the player
occupies into a solid. Or `walk` should push the player out to the nearest free cell, not the top of the column.

### S4. `?biome=` does not seed the story yet

`?biome=marsh`, `?level=3` and `?level=2` spawn in the right place (`biome` = marsh / frozen; `?biome=cave` spawns in
the cave at y 14). But `quest` is 0 and the pack holds only the Pick, while the spec asks for steps 1-4 done and
coal in the pack (E21, Lane C). `?biome=bogus` falls back to the Village, and `?biome=frozen&classic` gives the
classic courtyard (objs 14). Both are fine.

### S5. Small items

- Digging ice gives nothing, as specced, but no "it melts in your hand" caption shows (section 6). Only a test
  comment says it.
- `materials.json`:
  - `coal_ore` has `flammability 0`, but coal is the game's fuel and burns, igniting at about 400-450 °C (B6);
  - `snow` has `boil_c 100`, a water property on a solid row. It is harmless but odd.
- `story.js`: `SPROUT_FIRST_S` (3) is never used, because the hold is `Math.max(CARD_S, SPROUT_FIRST_S)` = 6.
- `?low` mesh queue: all 140 chunks took **83 s** to mesh on this sandbox, at a 2 ms budget and very slow frames.
  The spawn ring holds single frames of 330-460 ms (CPUFRAME max), while the intro covers it. After that the p50 is
  9-11 ms.
- The 13-year-old route:
  - digging a bank cell next to the river (the canyon outside the ford has `SOURCE` water) gives an endless
    flood into the dug channel;
  - the 4 000-liquid cap bounds the CPU, but nothing stops a flooded Village.
  - Worth one matter test: the flood stays inside the active-chunk caps and `matterMs` stays at 3 ms or less.

## What held up (tried, not broken)

- **Falling through the world.** Collision reads cells, not meshes, so nothing falls through while chunks load.
  Bedrock (y 0) and the 8-block edge ring are protected. `tp -50 200 -50` and `tp 300 -20 300` clamp into the world
  (0.5 / 255.5) and land on the edge mountains. Digging out of bounds is refused by `in_bounds`.
- **Placing a block inside yourself.** I looked at the grass under my own feet with dirt on key 6 and clicked. It
  was refused: the cell stayed air and the pack kept its dirt. Placing on protected cells is refused with "Nothing
  can be built here."
- **Stuck in a block.** Stone set at both feet and head popped the player on top, and W then walked away normally
  (stuck for 0 s).
- **Pack and keys.** If W is held while Tab opens the pack and released inside it, the player does not keep walking
  after Escape (moved 0.00 m). The Pick stops digging while the pack is open (the game is paused).
- **Spells.** Fire on the workshop shows the correct methane caption (CH₄ + 2O₂ → CO₂ + 2H₂O). `activeChunks` and
  `matterMs` (0.3 ms) update. There were no console errors across the core probe run.
- **Edits stall.** 40 `setcell` edits across chunk edges in one go remeshed within 72 frames with no long frame,
  and `meshQ` returned to 0.
- **Classic.** `?classic` and `?level=1` give exactly the 59 classic keys, 14 objs and 3 burning (E2). Every Bench,
  progress and worked check passes on classic.
