# I-052 Lane A, round 1 (wave 15): voxel engine and view

Branch `loop/I-052-A`, from the contract `loop/I-052` at 3315e28. Worktree `.claude/worktrees/w15-I-052-A`.
Scope: the Sunday-night and Monday column of spec section 12 for Lane A.

## Order of work (one commit each)

1. **Tests first** (6c9f702). V2, V5, V6, V7, V8 and V9 lost their `#[ignore]` and got bodies. On the contract stubs
   all six failed (6 passed, 6 failed, 3 ignored). The golden `voxelEngine` section got E1-E5. On the contract build
   E1, E3, E4 and E5 failed. E2 already passed, because the contract kept classic mode whole.
2. **Voxel crate** (6ff9f09): generation with biomes, the greedy mesher with AO, raycast, collision boxes, DiffLog
   and fast material lookups. V1, V2 and V5-V9 pass.
3. **Client** (09e5d49): `voxel_view.rs`, `voxel_tex.rs` and the `main.rs` Mode branches. E1-E5 pass with a working
   pack (see "Not done", item 1).

## Done

- **Generation** (`realgame_voxel::gen`). It builds a column heightmap from seeded value noise for each biome:
  Village 34 (a flat plaza, meadows further out), Kiln terrace 40 with brick steps down to the village (x 124..132,
  z 176..181), Frozen Fields 38-50 with ice ponds that ramp down to the ford, the gate and the pass, Marsh 30-33 with
  1-deep pools, and an edge ring of mountains 62+ high. The fixed landforms are:
  - the river canyon (water z 150..158 at y 28-30 as a SOURCE, cliffstone walls z 148/149 and 158/159 up to y 34);
  - the ford (x 122..134), where the banks step down to y 31 on both sides;
  - the cliff band (z 86..94), the ridge (x 132..138) with the east pass (z 66..74), and the east cliff (x 160..166);
  - the cave tunnels (fixed capsules from the cave mouth down to a hall, the rig room and the shaft) with crystals;
  - seeded trees (pines on snow, oaks on grass), kept off the plaza, the river banks and every stamp.

  Every stamp has a first version, and each writes its whole box from a fixed template: workshop (with BURNING
  planks on the roof), ford (still water, no SOURCE), ice gate, coal seam (coal plus a little iron ore, its top open
  to the sky), hollow (6 vents, a boardwalk, the gap at (196, 28, 101) draining east along a trench into a low
  basin), rig room, shaft, ledge, Kiln (brick dome, chimney to y 60, runestone door, hearth at (128, 41, 169), a sand
  bed), and the shortcut tunnel with its runestone door. The benches are at their section-4 cells, and the well is a
  source.

  **Protected boxes:** the edge ring, the bedrock (y 0), the canyon on both sides of the ford, the cliff band, the
  ridge apart from the pass, the east cliff, both benches, and every puzzle stamp except the coal seam. The hollow
  leaves its gap cell out, using 5 boxes.

  There is no random ore, so coal is found only in the seam behind the ice gate (the progression rule). The crate
  exports `biome_at`, `spawn_for(world, biome)` and `ground_below`.
- **Mesher** (`mesh_chunk`). Greedy meshing per face direction over 32 × 32 slices from a padded 34³ copy. AO uses
  the 3-neighbour rule and is part of the merge key. Liquid tops are lowered by their level. There are two passes,
  opaque and clear (water, ice, glass, crystal, steam). Triangle diagonals flip to follow the AO. The cell below
  y 0 counts as solid, so no bedrock underside is drawn. A buried uniform chunk is skipped. The colour is a toon tint
  per material (`mesh::TINT`) times AO, and UVs are in blocks.
- **Raycast** (Amanatides-Woo, `Solids` / `Fluids`, starts outside the world), **`solid_boxes` / `ice_boxes`**,
  **DiffLog** (version byte, u16 counts, runs of (start, len, cell); decode refuses empty, wrong-version, truncated
  and out-of-range input), and **materials** (`table`, `flags`, `by_name`, `name`, `dig_secs` with gameplay
  defaults until `hardness_s` is filled, `drops`).
- **Client view** (`voxel_view.rs`).
  - It generates the world at startup (the default seed, or `?seed=` with `?debug`) and spawns at `?biome=` or
    `?level=2|3`.
  - It removes the courtyard's entities. `Chem` and `Colliders` stay as inert resources.
  - Chunk entities come in opaque and clear passes, each with its own `Aabb` (frustum culling). `NotShadowCaster`
    is set on the clear pass always and on every chunk at `?low`. Meshes are kept on the GPU only.
  - The mesh queue takes the nearest chunk first. Its budget is 4 ms, or 2 ms at `?low`, with at least 1 chunk per
    frame. It allows 40 ms until the spawn ring is meshed. `chunksMeshed` counts chunks with a non-empty mesh;
    `meshQ` counts chunks waiting.
  - Walking uses `motion`'s rules against the block boxes (ice is solid and is also passed as ice sheets), with a
    one-block step up without jumping and camera easing.
  - **The Pick (key 5):** with the pointer locked, hold the button to dig. A click without the lock (`cast`) or a
    tap digs on by itself. A dig takes the material's dig time, and a block that breaks shows `digT` = 1 for 0.25 s.
    A protected block keeps `digT` at 0 and says "This old runestone will not budge. Try chemistry." once per press.
    A full pack refuses the dig ("Pack full").
  - **Keys 6-9** place the block bound to that key on the face looked at. Placing never goes inside the player,
    never into a protected box, and never onto a solid. The first key 6-9 that has nothing bound gets bound
    automatically to a block the first time it is gathered.
  - It sends `Dug`, `Placed`, `EnteredBiome` and `InBox` events. The keys it publishes are `biome`, `hot`,
    `lookCell` and `digT`.
- **main.rs Mode branches.**
  - Keys 1-4 select a spell and set `hot`; keys 5-9 set `hot`. `select 1..9` and the native wheel cycle all nine
    slots in voxel mode.
  - The fog runs from 120 to 192 m at `?low` and from 180 to 256 m otherwise, with the far plane at 400 m.
  - Voxel aim uses the block raycast (Fluids mode, no assist). The fire finder is off in voxel mode, and no spell is
    cast while a tool (keys 5-9) is in hand.
  - `chem_tick`, `react` and `sync_visuals` run in classic mode only. R and the `tick` and `reset` commands do
    nothing in voxel mode.
  - Voxel `publish` writes `objs` as `[]`, `level` as 0, and every courtyard fire count as 0. Classic mode publishes
    exactly the 59 keys of today.
  - Debug commands: `tp x y z`, `face x y z`, `setcell x y z <name|id>`, and
    `probe x y z` → `probe: {n, at, mat, level, state, tempC, co2, solid, protected}`. The `probe` key goes after
    `ending` once a probe has run.
- **Science register.** The `npm test` science lint flagged the six new instruction lines, so they now have
  Not-claims rows in `content/science-sources.md`, appended at the end of the Not claims table. That file is outside
  Lane A's paths, but these are appended rows only, and they are not in Lane B's "Block world" section. The
  ice-in-the-hand caption was dropped: it states science, and Lane C owns the gathering lines (section 6).

## Test results (head of this branch)

- `cargo test --locked` (root): all pass. Voxel: 13 passed, 3 ignored (V3 and V4 are Tuesday work, V10 is Lane B's).
  The sim tests pass (218), and the matter tests are 3 passed, 13 ignored.
- `cargo fmt --check` (root and client), `cargo clippy --all-targets -D warnings` (root native),
  `cargo clippy --target wasm32-unknown-unknown -D warnings` (root and client): clean.
- `npm test`: 97 pass. `perf.mjs --self-test`: 50 pass. `build-web.sh`: wasm gzip 5 182 144 bytes, up 42 KB from
  the contract's 5 140 472.
- Native timing (`mesh_and_generation_timing_prints`, release-like test profile): generating takes 27 ms, and
  meshing the whole world takes 225 ms (worst chunk 2.0 ms). There are 140 non-empty chunks and 150 array chunks
  (about 19 MB of cells), and 217k triangles.
- **Golden `ONLY=voxelEngine`, `QUERY=?low`, with a working pack: 6/6.**
  - E1: `chunksMeshed` was 20 after 2.0 s.
  - E2: 59 keys and 3 fires on both `?classic` and `?level=1`.
  - E3: walked 10.2 m up the terrace steps, y 35 → 39, and none of 11 samples was inside a solid.
  - E4: dirt +1, the cell turned to air, key 6 was bound, and placing put the dirt back.
  - E5: `digT` stayed 0, the runestone caption showed, and the cell is still cliffstone.

  The pack was a scratch implementation of `Pack::add`/`take`/`count` plus a line in `story::on_event` that fills
  `Keys.pack`, applied only for that run and never committed. On this branch alone, Pack is still the contract stub,
  so E4 fails ("Pack full") until Lane C's C1 lands.
- **Golden on this branch's own build** (`QUERY=?low`, no scratch pack). The full run hit the 30-minute background
  limit inside `reviewFollowups`, after 117 PASS and 0 FAIL. Every section from `reviewFollowups` back to the first
  had passed by then, so the main.rs Mode branches leave the classic checks intact. The rest of the sections then ran
  with `ONLY=spellProgress,formulaBench,workedCasts,voxelEngine,blockMatter,valleyStory` and gave 125/126. The one
  failure is E4, with "pack dirt +0": the stub `Pack::add` refuses, so the dig resets as "Pack full". There were no
  console errors.

## Not done (Tuesday and later, or blocked)

1. **E4 needs Lane C's `Pack`** (`add`, `take`, `count`) and `story` filling `Keys.pack` from `crate::Pack`. Until
   then, digging on this branch says "Pack full" and leaves the block where it is (the spec rule for a full pack).
   Walking, the look cell, the Pick's progress, protected blocks and the mesh all work without it.
2. The atlas is one shared toon tile, not the 256 × 256 per-material atlas. A merged greedy quad cannot repeat one
   atlas tile with a plain `StandardMaterial`, so per-material detail needs a small material extension that wraps
   the UV inside a tile. That is the Wednesday polish item, and the mesher's UVs are already in blocks for it.
   Emissive crystals and torches, sky cover, cave darkness and torch lights (Tuesday) are not done.
3. V3 and V4 are still `#[ignore]` (Tuesday). The stamps and protected boxes exist, but they need checking against
   section 4 cell by cell. Known gaps:
   - the rig has no rig block (no material row for it);
   - the cave ladder exit and its link south of the river are missing;
   - the hollow's boardwalk exits through the gap's column;
   - the Kiln's chimney column runs through the dome's inside.
4. E6-E8 (perf, memory, mesh budget) are not written. Native meshing is about 1.6 ms per chunk on average, so at 4×
   throttle one chunk per frame will break the 6 ms of E8. Spec risk 6 applies: split a chunk's meshing by face
   direction.
5. The courtyard is still built in voxel mode and then despawned, because `Chem` and `Colliders` are still read by
   the shared systems.

## For the other lanes

- **Lane B:**
  - The block world is `ResMut<crate::voxel_view::VoxWorld>` (`.0` is the `realgame_voxel::World`). Use only
    `get` and `set`; every `set` dirties the chunk and its face neighbours, and the view remeshes them.
  - A landed spell arrives as the Bevy message `crate::SpellLanded { spell, pos, normal }`, sent by `projectiles` in
    voxel mode after `cues::flight_secs`. Call `blockchem::on_impact` from it. The aim uses `RayMode::Fluids`, so
    water is hit.
  - `probe` prints `"co2":null`. Tell Lane A where to read the gas fraction (a resource), and it will be wired in.
  - Voxel `burning` publishes 0 until blockchem can supply a count.
  - Useful cells: `gen::HOLLOW_VENTS`, `gen::HOLLOW_GAP`, `gen::KILN_HEARTH`, `gen::WELL`. The river water outside
    the ford has SOURCE set; the ford water does not.
  - `materials::dig_secs` reads `hardness_s` once it is a number.
- **Lane C:**
  - The pack is `ResMut<crate::Pack>` (`.0` is `valley::Pack`). The view calls `add(Item::Block(id), 1)` on a dig
    (a leftover means the pack is full) and `take(Item::Block(id), 1)` on a place. It also writes `pack.hot[k]` to
    bind a key automatically.
  - The `pack` key comes from `story::Keys.pack`. The golden E4 check looks for `item: "dirt"`, so name blocks by
    `realgame_voxel::materials::name(id)`.
  - Grass drops dirt, and ice drops nothing (the caption for it is yours).
  - `hot` 1-9 is in `voxel_view::Keys`. `hud.js` still cycles the wheel over 4 slots; the game accepts `select 1..9`.
  - A protected dig only sets the caption. The dull "tunk" can key off `caption` containing "runestone" plus a press.
  - The courtyard's "Save the Greenhouse" objective box is still shown in voxel mode.
- **Harness note (golden):** under pointer lock, a synthetic `page.mouse.down` in headless Chromium turns the view
  by about 11° in yaw and pitch before the press lands. Small targets therefore get a `fire` command, which is what
  `hud.js` sends for a locked left press.
- **V5 wording:** the spec's "two touching stone blocks give 10 quads" contradicts the same test's "a 32 × 1 × 32
  slab gives exactly 6 quads" under greedy merging; two touching blocks merge to 6 quads. The V5 body checks the
  intent instead: 10 unit faces of area, no face between the blocks, and at most 10 quads. The coordinator should
  confirm, or correct the spec line.
