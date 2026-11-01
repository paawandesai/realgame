# I-052 Lane B (matter), round 1: Sunday night and Monday

Wave 15. Branch `loop/I-052-B`, worktree `.claude/worktrees/w15-I-052-B`, from the contract at 3315e28.

## Done

1. **Tests first** (d87fecd). M3, M5, M6 and M8-M11 got bodies and lost their `#[ignore]`. M1a, M1b, M2, M4,
   M4b and M7 say "on the generated world", so each check is written once in `crates/matter/tests/common/mod.rs`
   and runs twice:
   - on `voxel::generate` in `tests/matter.rs`, still `#[ignore]` because the generator is Lane A's stub (all air);
   - on hand-built fixtures of the stamps in `tests/rules.rs`. The fixtures sit inside the same stamp boxes, at
     the section 4 coordinates. These are green now.

   The checks find the water, ice, vents and burning cells inside the stamp boxes, so they do not depend on the
   fixtures' exact shapes. Against the stub crate the tests did not compile: `touch`, `gas_at`, `o2_at`,
   `add_gas`, `stats`, `burn_left` and `burn_full` were all missing.
2. **Matter rules** (0c17806), in `crates/matter/src/{lib,gas,props}.rs`:
   - **Heat.** Conduction between face neighbours (clamped, so a step never overshoots), plus a pull back to
     each material's resting temperature. Latent buffers hold ice at 0 °C until it has taken the heat of fusion,
     and water at 100 °C until it boils.
   - **Phase.** Ice → water. Water → ice from the top of a column down. Water → steam (it rises, then vanishes
     after 5 s). Sand → glass at 1713 °C. Iron ore next to coal → iron at 1200 °C, using up the coal. Leidenfrost
     beads on a Frost hit on water above 10 °C.
   - **Fire.** Spreads by heating touching blocks to their ignition temperature. Burns out to ash (leaves to air).
     Goes out when touched by water, when its gas cell's O2 drops below 15 %, or when Frost cools it below
     ignition (rarely, as the spec says).
   - **Water.** Integer and volume-conserving: down first, then sideways. A last single level runs off an edge
     it can fall from. A cell settles after 2 idle steps.
   - **Gas field.** 2 m cells. CO2 and O2 sink, H2 and CH4 rise and leave through the top of the world.
     Sideways spread, and a slow leak of CO2 back to air: 1/60 of a cell's CO2 every 5 s, so a 3-cast pool's
     vents relight after about 50 s.
   - **Vents.** Smothered below 15 % O2 and relit above it. Water or Frost knocks the flame down for
     `FLICKER_S`; Fire does nothing.
   - **`lift_n`.** Uses the species molar masses and the ideal gas law.
   - **Caps.** Heat 8000, fire 2000, water 4000 and gas 6000 per step, with carry-over cursors.
3. **Material values with sources** (fc3d811). `content/materials.json` has no `"todo"` left, and every row's
   `source` is a `### mat-*` heading in an appended `## Block world` section of `content/science-sources.md`.
   That commit also adds P-024 to P-039 for the matter constants and sources S32 to S39 (all fetched
   2026-11-01). V10's body, which the spec gives to Lane B, is written: it failed on the contract's table with
   every value still `"todo"` and passes now.
4. **`blockchem.rs`** (a44c199):
   - `on_impact(spell, hit_pos, hit_normal) -> Emit` (Foam → `EmitKind::Co2`).
   - `Matter::queue`, which holds a cast for `cues::flight_secs`, the delay classic mode uses.
   - `Matter::tick`, which hands landed emits to the sim, runs at most one 0.1 s step per frame, and fills
     `activeChunks`, `matterMs` and `gasCO2Max`.
   - The `Matter` resource is registered in the plugin.

## Not done (this round)

- `blockchem` captions per band (Tuesday), gas puffs, beads, the rig, the balloon crate and `wade_push`.
- The `blockMatter` golden function is still empty, because E9-E14 need the voxel view and the Mode branches
  (Lane A, Tuesday).
- The generated-world runs of M1, M2, M4, M4b, M5 and M7 wait on Lane A's stamps.

## Test results (native, `CARGO_TARGET_DIR=/home/claude/native-target`)

- `cargo test -p realgame-matter`: 19 passed, 8 ignored. In the lib: 2 passed (glass point equals sand's
  `melt_c`; key round trip). In `tests/matter.rs`: 9 passed (the 3 contract checks, M3, M5, M6, M8, M10 and
  M11) and 7 ignored (M1a, M1b, M2, M4, M4b and M7 on the generated world, and M9; see below). In
  `tests/rules.rs`: 8 passed (fixture M1a, M1b, M2, M4, M4b with stone, M4b with an ice dam, M7, and a guard
  that two CO2 casts are not enough) and 1 ignored (M5 on the generated world).
- `cargo test --locked` (root): all pass. `cargo test -p realgame-sim --no-default-features`: pass.
  `realgame-voxel`: V10 now passes.
- `cargo fmt --check` (root and client), `cargo clippy --all-targets -D warnings` (root), and `cargo clippy
  --target wasm32-unknown-unknown -D warnings` (root and client) are clean. `npm test`: 97 pass.

## Where I depart from the spec, and why

1. **M9 is wrong as specced, so it stays ignored with its body unedited.** Section 5.7 and M9 say CH4 lifts the
   cave balloon. It cannot: (1.20 − 0.67) kg/m³ × 2 m³ × 9.81 m/s² = 10.4 N, which is less than the payload's
   1.5 kg × 9.81 = 14.7 N, so `lift_n` is about −4 N. The H2, O2 and CO2 signs and the section 5.7 numbers
   (+7.2 N, −17.3 N) all hold. The fix belongs to the spec owner: either CH4 is expected to be negative for this
   payload, or the payload drops to 1.0 kg or less (10.4 − 9.8 > 0).
2. **CO2 emit size: 23 m³ (`CO2_EMIT_M3`), not 1 m³.** Section 5.5 contradicts itself: 1 m³ of CO2 in an 8 m³
   gas cell is 12 %, not "0 to about 1.0", and a 3 × 2 × 3 hollow cannot fill in 3 casts of one cell each. I
   tuned it so that 3 casts smother the fixture hollow (21 open floor gas cells; M4) and 2 do not (the guard
   test). P-036 records it as gameplay tuning.
3. **Gas connectivity.** The spec's rule (a gas cell is closed if 5 or more of its 8 blocks are solid) cannot
   see a one-block gap: the gap block and the wall it sits in share one gas cell. So, in addition, two gas cells
   exchange gas only when some pair of blocks facing each other across their shared face are both open. Water
   of level 6 or more blocks gas; shallower water does not, so the shallow water in the gap lets CO2 out until
   it is frozen into an ice dam (M4b).
4. **M4 closes the gap first.** M4 (all 6 vents out after 3 casts) and M4b (with the gap open, at least one vent
   stays lit) contradict each other on one world, so M4 places a stone in the gap and then casts. M4b covers
   the open gap.
5. **Emit radius.** Water, Frost and Fire act on the cells whose centres are within 1.5 m of the hit point. For
   a hit on a face centre, that is the 3 × 3 patch in the hit plane plus the cell behind. Section 5.6 says
   "radius of 1 block" for Fire; this is the same 3 × 3 patch, and it is what makes 3 casts cover the gate.
6. **M1b checks cells, not `solid_boxes`.** Ice is in `ice_boxes`, not `solid_boxes` (V8), and both are still
   Lane A stubs. The fixture run checks that every cell under the 0.7 m wide path from bank to bank is ice. The
   generated-world run also checks `ice_boxes` along the path.
7. **Untouched temperatures.** `Cell::temp == 0` reads as each material's resting temperature in the sim, not
   20 °C: ice and snow −5 °C, water 12 °C, everything else 20 °C (P-029, P-030). Otherwise every generated ice
   block would read 20 °C and melt. Heat lives in the sim's own map: when the sim first sees a cell with a
   stored `temp`, it takes the value and writes the field back to 0. So heat alone never dirties a chunk.
8. **Resting temperature replaces "air held at −5 °C" for the ford.** Ice made by Frost relaxes to −5 °C and
   stays frozen anywhere. Fire, Water and conduction still change it.

## What the other lanes must know

**Lane A (voxel, view, main.rs):**

- **Wiring.**
  - The cast (`aim_and_cast`, voxel branch) calls `Matter::queue(game_time, spell, eye, hit_point, normal)`
    with the voxel raycast hit. Frost and Fire use `RayMode::Fluids`.
  - Once per frame, after player edits and before remeshing, call
    `matter.tick(&mut vox, game_time, dt, player_pos, &mut blockchem_keys, &mut events)`, send each event as
    `crate::MatterEvent`, then remesh `dirty_chunks()`.
- **`touch`.** After every dig, place or `setcell`, call `matter.world.touch(&vox, cell)`, so water next to a
  dug block flows, a placed block that blocks gas or water is noticed, and a burning block placed by `setcell`
  starts burning. Gas connectivity is read live, so a block placed in the hollow's gap works even without
  `touch`.
- **`probe`.** Use `matter.world.temp_c(&vox, p)` (not `Cell::temp_c`, which reads 0 as 20 °C) and
  `gas_at(p)[0]` for `co2`.
- **Drawing.** A vent is lit, and a block is burning, when its cell has `Cell::BURNING`. Draw flames from that.
  Steam is the `steam` material.
- **The first `step` scans the whole world once,** for burning, unsettled water, vents, steam and stored
  temperatures. Uniform chunks are skipped. **Generate still water (river, pools, the ford, the well) with
  `Cell::SETTLED` set**, or the first steps will chew through every water cell. The river's source cells need
  `SOURCE`.
- **What the generated stamps need for M1, M2, M4, M4b and M7 to pass there** (the fixtures in `common/mod.rs`
  show one way):
  - **Ford:** still water 3 deep at y 27-29 inside `FORD`, with banks whose top block is at y 30 or 31 at both
    ends of each x column.
  - **Ice gate:** ice cells in `ICE_GATE` with air on their south (+z) side; the opening is 4 wide × 3 tall ×
    1 deep in the fixture. South of the gate, the ground must fall away within 12 blocks, so meltwater ends
    below the gate's base (M2).
  - **Hollow:** exactly 6 `gas_vent` cells in `HOLLOW`, on the floor (vent at y 28 on a floor block at y 27),
    lit (`BURNING`). The rim's top block at y 31. The interior x 187..196, z 97..105, y 28..31 air.
  - **The gap:** at (196, 28, 101), in a one-block east wall at x 196, holding air or shallow water (level < 6,
    `SETTLED`). Directly east of that wall there must be open air that drops lower than the hollow's floor. The
    fixture has a sunken marsh pool at x 197..216, z 96..106, air from y 22 up, open to the sky. Spilled CO2
    keeps sinking into it, so the gap drains the hollow fast enough that 3 casts leave a vent lit. If the far
    side is closed or level, the gas cell (98, 14, 50) fills up instead and M4b fails.
  - **Workshop:** some `planks` or `wood` cells in `WORKSHOP` with `BURNING` set, each touching another wood or
    planks block.
- **Shared target directory hazard.** `realgame-voxel` builds to the same artifact from every worktree (path
  dependencies get the same metadata hash), and it `include_str!`s `content/materials.json`. After another lane
  builds, my tests ran against that lane's table, still all `"todo"`, until I ran `touch
  crates/voxel/src/materials.rs`. Any lane that sees odd material values should touch that file and rebuild.

**Lane C (game layer):**

- Events for the quest: workshop fire put out → `Extinguished { by: By::Water }` (Frost gives `By::Frost`, no
  oxygen gives `By::NoOxygen`). Gate → a run of `Melted`. Hollow → one `Smothered { vent }` per vent, then
  `Relit` when the CO2 leaks away (measured: the first vent relights about 50 s after a 3-cast fill, with the
  gap closed). Step 5's "every vent out at the same moment"
  is best tracked as a count of lit vents from `Smothered`/`Relit`.
- The rig and balloon can call `matter.world.add_gas(&vox, pos, Gas::H2 | Gas::O2, m3)`. `lift_n(gas, 2.0,
  1.5)` decides whether the crate rises. With CH4 it is negative (see departure 1).
- **Register ids I took:** P-024 to P-039 and S32 to S39. Start yours at S40 and P-040. No new C- rows yet (the
  last is C-096).

**Gameplay note for Wednesday's tuning:** fire spreads to a touching block in about 2 s, and planks burn for
45 s. Measured on the fixture hut, which has about 200 wood and planks blocks: 8 burning at 5 s, 58 at 20 s, a
peak of 112 at about 40 s, then burning out. Each Water cast puts out the 3 × 3 patch it hits.
If the burning hut is too big for a 6-year-old, make the stamp's walls mostly stone with a planks roof (Lane A),
or raise `BURN_J`'s ignition time (Lane B).
