# I-052 Lane A: the contract step (spec section 10, items 1-8)

Wave 15. Branch `loop/I-052`, worktree `.claude/worktrees/w14-I-052`, on top of the spec commit 5e9e7f8.

## Order of work

1. **Tests first** (76c9235): `crates/voxel/tests/voxel.rs` (V1-V10 and the contract checks),
   `crates/matter/tests/matter.rs` (M1-M11 and the contract checks), the valley test module in
   `crates/sim/src/valley/mod.rs` (C1-C6 and a contract check), and the `perf.mjs` self-test now expecting
   `classic=1`. On empty crates: `cargo test` failed to compile in all three crates (unresolved imports for
   every contract type), and `perf.mjs --self-test` gave 49 passed, 1 failed (the URL test).
2. **Crates and stubs** (21640b0): items 1-5. Contract checks and V1 are green; the lane tests stay
   `#[ignore]`.
3. **Mode switch, plugins, events, golden** (6ef2fc4): items 6-8, plus `retired-checks.md`.

Lane tests are `#[ignore = "<lane> (<day>): ..."]` with a `todo!()` body, so taking off the `#[ignore]`
without writing the body fails. The lane that owns a feature writes that body first (spec section 12).

## What the contract fixes

- **realgame-voxel**: `Cell` (4 bytes; `level`, `set_level`, `state`, `set_state`, `has`, `set_flag`,
  `temp_k`, `temp_c`, `set_temp_k`; `Cell::BURNING/SOURCE/SETTLED/LIT` = 1/2/4/8), `mat::*` ids in
  section 5.2 order plus `mat::NAMES` and `mat::COUNT`, `CHUNK`, `WORLD`, `CHUNKS`, `World::{new, get, set,
  dirty_chunks, take_dirty, chunk_index, chunk_coords, chunk_origin, cell_index, chunk, put_chunk, protect,
  protected, can_break, can_place, hash}`, `Biome` (ids for `?biome=`), `SPAWN`, `DEFAULT_SEED`, the stamp
  ids and boxes (`stamp::{WORKSHOP, FORD, ICE_GATE, COAL_SEAM, HOLLOW, RIG, SHAFT, LEDGE, KILN, SHORTCUT}`,
  `stamp::ALL`, `stamp::at`), and stub `raycast`, `solid_boxes`, `ice_boxes`, `mesh_chunk`, `generate`
  and `DiffLog`.
  The chunk index is x fastest, then y, then z (the cell index uses the same order). `set` also dirties the
  neighbour chunk when the cell is on a chunk face, so the mesher can redraw the shared face.
- **content/materials.json**: `{"schema": {...}, "rows": [...]}`, one row per Must material in id order.
  The structural fields are filled (`id`, `name`, `phase`, `drops`, `atlas` = id, `transparent`, `solid`).
  The physical fields and `source` are `"todo"` for Lane B, and `hardness_s` is `"todo"` too. `null`
  means "does not apply". `materials::load` parses it, and `materials::todos` lists what is still
  `"todo"` (V10 starts from that).
- **realgame-matter**: `DT`, `FLICKER_S`, `Emit`, `EmitKind::{Water, Frost, Fire, Co2}`,
  `By::{Water, Frost, NoOxygen}`, `MatterEvent` (all 10 variants in the spec), `Gas::{Co2, H2, Ch4, O2}`,
  `World::{new(seed), emit, step(&mut voxel::World, &mut Vec<MatterEvent>), ticks, hash(&voxel::World)}`,
  and `lift_n(gas, volume_m3, payload_kg)`. `step` and `lift_n` are stubs.
- **realgame-sim `valley`**: `Item`, `Stack`, `Pack` (`add`, `take`, `count`, `to_json`), `Recipe`,
  `RECIPES` (empty), `craft` with `CraftError`, `Step` and `STEPS`, `QuestEvent`, `Quest::{new, on, step,
  line(Option<Tier>)}`, `invent::{Invention, judge}`, and `usable`. All bodies are stubs.
- **main.rs**: `Mode` and `mode_from` (section 3.3, with a unit test), `StartBiome`, the `PlayerEvent`
  message and `MatterEvent(realgame_matter::MatterEvent)`, the plugins `voxel_view::plugin`,
  `blockchem::plugin` and `story::plugin` (Voxel only, empty), and `story::on_event`, which already
  receives both messages. Each plugin owns a `Keys` resource. `publish` reads all three through
  `ModeKeys` and appends the voxel keys after `ventsLit` in the section 8 order; classic mode publishes
  exactly today's 59 keys.
  Checked in a browser: the default page and `?level=2` publish 74 keys, with
  `biome` = village / frozen; `?classic=1` and `?level=1` publish 59.
- **golden**: `base` has `classic=1`. `voxelBase` (the default page) goes to the empty `voxelEngine`,
  `blockMatter` and `valleyStory` functions, which are in the `onlyRuns` map and called after
  `workedCasts`, where `frozenGate` and `marsh` used to be.
- **perf.mjs and analytics-e2e.mjs** load `classic=1`.

## Where the contract differs from the spec, and why

1. **Voxel mode still builds the courtyard.** Section 3.3 says Voxel "skips `world::build` and `levels`".
   The contract skips `levels`, `level_visuals` and `level3_visuals` (run only in Classic) but keeps
   `world::build`. The shared systems (`control`, `aim_and_cast`, `projectiles`, `react`, `sync_visuals`,
   `publish`, `bench::run`) still need `Chem` and `Colliders`. Their Mode branches are Lane A's later
   work (section 9, Tuesday), and a voxel page without those resources would panic. So until then, the
   default page shows the courtyard with the voxel keys appended. Nothing in golden reads the default page
   yet.
2. **`matter::World::hash` takes the voxel world** (`hash(&self, vox: &voxel::World)`). The blocks live
   in `realgame_voxel::World`, not in the matter world, and M10 needs both in the hash.
3. **`valley::usable(def: &SpellDef, band: Tier, stage: u8)`**, not `usable(spell, band, book)`.
   Progress (`progress::Book`) lives in the Bevy client, which `realgame-sim` cannot depend on. So the
   stage is passed as a number, using `worked::route`'s numbering (0 Unknown, 1 Learning, 2 Practised,
   3 Mastered). Lane C owns the module and may change it.
4. **`QuestEvent` holds semantic events** (`IntroDone`, `WorkshopOut{by_water}`, `CrossedFord`,
   `GateMelted`, `Holding{item, total}`, `CrossedHollow`, `Invented{relights}`). `realgame-sim` cannot
   see voxel stamp ids or matter events, so `story` will translate `PlayerEvent`s and `MatterEvent`s into
   these. Lane C may add variants.
5. **One golden URL besides `base` changed.** The feel check `feel: blocked storage + reduced motion boots
   with URL sens and no sway` rebuilds its own URL from scratch (`base.replace(/\?.*$/, '') + '?low&debug&
   sens=2'`). Without a change it would load the default page, the block world. It now adds `classic=1`,
   so it stays on the classic courtyard like every other keep check (section 11: "every keep check runs
   unchanged on the classic courtyard"). Its assertions are untouched. At this step it would pass either
   way, because voxel mode still builds the courtyard.
6. **`perf.mjs`'s level-2 and level-3 workload branches are not retired here.** Section 11 lists them,
   but item 8 only adds `classic=1`, and with `classic=1&level=2|3` they still load the old levels.
   `mode_from` keeps `?classic&level=2|3` on the old levels until the follow-up that deletes them.
7. **No `furnace` material row.** Section 6 makes the Furnace a placeable block, but section 5.2's Must
   list has no row for it, so the contract follows 5.2. Lane B or C appends it as id 27 (rows are only
   ever appended).
8. **Window title.** Not changed to "Formula" here: it is not in the contract list, and on the web
   winit may set `document.title`, which the page shell (Lane C) owns.
9. **My own contract test, first version.** `world_and_chunk_sizes_match_the_spec` pins the chunk index
   order x, then y, then z. The first implementation used x, then z, then y, and failed that test. The
   implementation was changed to match the test; the test was not edited.

## Checks (see the round report for the golden numbers)

`cargo fmt --check` (root and client), `cargo clippy --all-targets --locked -D warnings` (root),
`cargo clippy --target wasm32-unknown-unknown --locked -D warnings` (root and client),
`cargo test --locked` (root), `cargo test -p realgame-sim --no-default-features --locked`, standalone
`rustc --test` for `chem.rs` (28 passed) and `worked.rs` (7 passed), `npm test` (97 passed),
`perf.mjs --self-test` (50 passed), and `build-web.sh` (wasm gzip 5 140 472 bytes) are green.
`ONLY=voxelEngine,blockMatter,valleyStory` passed 1/1 (no console errors).

## Full golden (QUERY=?low, built from 6ef2fc4)

**246/247 passed.** The one failure is `cast: a real click casts exactly once` (`casts 1 -> 2 -> 2 (20 frames on)`),
one of the known wall-time flakes on the software GPU. Every check now runs on `classic=1`, and the three empty
I-052 sections ran with no console errors.
