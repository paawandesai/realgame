# I-052: Block valley (voxel world you can dig, build and do chemistry in)

**Game name:** Formula (the window title and the ending card use it; `main.rs` sets `Window::title` to "Formula").

**based_on:** 5662cd7 (origin/main, "Merge pull request #53 from paawandesai/loop/state-w14").
**Track:** product. **Bars:** B1 (golden), B2/B3 (perf and load), B4/B5 (panel), B6/B10 (science), B8 (enough to play), B11 (more than a shooter).
**Due:** Game Week, Option 1 Exploration/Adventure, Thursday 2026-11-05 at midnight. Feature freeze Wednesday 2026-11-04 night.
**Direction (coordinator, 2026-11-01):** Paawan chose a **block world** (Minecraft-style voxel terrain you can dig and build in) instead of the hand-placed valley, and wants it built with the full goal in mind. So the voxel and matter code are **new pure-Rust crates with no Bevy dependency**, which a future headless server and the VR spin-off can reuse. Design doc: https://claude.ai/artifact/Kg2m9K27rvzV8jt9umgaTt (summarised by the coordinator; not fetched). The earlier valley plan (https://claude.ai/artifact/WNHo4L6s1XdkZfrJKHG2q8) still supplies the story, the regions and the puzzles; this spec turns its regions into biomes.
**Hardware target (coordinator, 2026-11-01):** modern laptops (about 4 years old or newer) running Chrome. Chromebooks are nice to have, not the target. B2's cloud rule (4× throttle, `?low`) still applies, because CI does not change.
**Out of scope here:** spell trajectories (I-051 rebuilds them: ballistic water arcs, short fire cone, sinking CO2) and the mastery trial (I-030, being built). This spec only says where a landed spell enters the matter sim.

Paths: `main.rs` means `crates/bevy-client/src/main.rs`; `golden` means `tests/e2e/golden.mjs`; `web/` means `crates/bevy-client/web/` (the 3D client's shell, not the retired top-level `web/`).

---

## 1. Problem

1. **The assignment is not met.** Today the game is three walled 48 m boxes (`world::HALF = 24.0`) joined by arches. `levels` (`main.rs:1233`) despawns every `LevelRoot` entity and builds the next box on each arch walk, so the game is "a level, then a loading cut". There is no world to explore, no gathering, no item that opens a new path, no intro or ending, and no music. Paawan rejected the levels as too simplistic.
2. **The world cannot change shape.** Chemistry acts on about 10 hand-placed `chem::Obj` records per level (`chem.rs:292`), each with its footprint, a `neighbours` list (`NEIGHBOUR_GAP = 2.5` m) and one `Goal` per world (`Goal::Fires` or `Goal::Reach`, `chem.rs:435`). Melting ice, flowing water or a pool of CO2 are flags on one object (`Obj::ice`, `Obj::co2`), not matter that moves. A block world needs matter in cells.
3. **Paawan's direction:** a dig-and-build block world by Thursday, with chemistry acting on the blocks (fire melts ice into water that flows, frost freezes water into ice you can walk on, sand plus heat makes glass, ore plus charcoal plus heat makes iron, CO2 pools in hollows, H2 rises). It must still meet the assignment: a beginning, middle and end; 3 or more connected, revisitable areas; 3 or more real puzzles; progression where an ability or item opens new paths; cohesive generated visuals, generated SFX and music; playable by a stranger in a browser; 10-15 minutes.

## 2. The smallest version that meets the assignment by Wednesday night

**One generated block world, 256 × 96 × 256 blocks (1 block = 1 m), in 8 × 3 × 8 chunks of 32³.** Seed-based terrain with stamped templates. Five biomes. One continuous scene: no loading screen and no level swap after boot.

| Assignment rule | How the Must list meets it |
|---|---|
| Beginning / middle / end | Intro over the cold Kiln, then the quest chain (6 steps), then the relit-Kiln ending card |
| 3+ connected, revisitable areas | Village, Frozen Fields, Marsh, Crystal Cave, Kiln terrace. All are in one world; you can walk back. A shortcut door from the Marsh back to the Village closes the loop |
| 3+ real puzzles | P1 workshop fire, P2 frozen river ford + ice gate, P3 methane hollow (close the gap in the rim, then pool CO2), P4 balloon shaft (choose the gas), P5 invention at the Kiln (combine what you gathered). Each fails with the wrong chemistry or the wrong order (B11) |
| Progression: ability or item opens paths | Frost opens the river; Fire opens the ice gate; CO2 opens the marsh path; **Coal** (gathered behind the ice gate) makes a **Torch**, and the Crystal Cave is too dark to find the rig without one; the **Kiln Spark** (on the cave ledge) unlocks the Kiln door; **O2** (from the cave's electrolysis rig) and **Coal** let you invent the reaction that relights the Kiln |
| Polished generated visuals | A runtime-generated "toon block" atlas in the game's existing palette: flat colour tiles with a thin darker border and a little detail per material (no image downloads), vertex ambient occlusion, distance fog, per-biome sky tint, emissive crystals and torches. Protected runestone blocks have their own carved, faintly glowing tile, so you can see which blocks will not budge |
| Generated SFX and music | Synthesised in code with WebAudio (no audio files; the restricted ElevenLabs connector here cannot make music anyway): the existing `makeSound` in `web/hud.js` plus dig, place, splash, hiss, whoosh and Kiln roar; a procedural music sequencer with one motif per biome (`web/music.js`) |
| A stranger can play in a browser | It is the default page. The intro states the goal, a single objective line and an arrow say where to go next, and every control is on the pause card |
| 10-15 minutes | About 450 m of walking (about 90 s at 5 m/s), 4 Formula Bench builds (about 30-60 s each), 5 puzzles (about 1-2 min each), and the ending |

**Must (feature freeze Wednesday night):** the voxel crate (chunks, cells, material table, generation and stamps, protected zones, raycast, collision boxes, greedy mesher, diff format); the matter crate (heat, fire spread, ice/water/steam, volume-conserving water, the coarse gas field with CO2 sinking and H2/CH4 rising, flame smothering, glass and iron rules, balloon lift, the spell emit API); the client view (chunk meshes, a time-sliced mesh queue, the atlas, break and place, collision); the pack, crafting (Torch, plus a Furnace for glass and iron, added after the critique), quest, invention, intro, objective and ending; music and SFX; golden sections; and `?classic` keeping today's game whole.

**Should (only if Tuesday's plan is green):** saving chunk diffs to IndexedDB, with OPFS when it is available; mesh building in a Web Worker; Sprout picture captions for the new events.

**Won't (this item):** creatures, other players, the WebGPU compute path for matter, an infinite world, crafting settings (I-031), a level-select screen (I-034).

**Mastery and the 15-minute session.** Mastery takes 3 unaided builds over **2 days** (target.md, I-030). Nobody can master a spell in a 15-minute run, and the loop never fakes it. So every gate checks that a spell is **usable**, not that it is mastered. Usable means stage ≥ Learning for a spell your band can learn, and always usable for a spell your band cannot learn (Fire at Sprout, which matches `worked::route`). Worked casts still earn XP, and mastery stays the long-term loop. Section 7 says how the plan's "until mastery" line changes.

## 3. Architecture (what is new, what is reused, what is replaced)

```
crates/sim      (exists)  reaction rules engine: formula, reactions, spellbook, bench, tier.
                           + new module valley/ : pack, craft, quest, invent (pure, no Bevy)
crates/voxel    (new)     cells, chunks, materials table, generation + stamps, protected
                           zones, raycast, collision boxes, greedy mesher (pure), diff log
crates/matter   (new)     block sim on active chunks at 10 steps/s + coarse 2 m gas field;
                           depends on voxel and sim (reaction ids); deterministic
crates/bevy-client        mode switch, chunk view + mesh scheduling, block chemistry glue,
                           story glue; reuses player/motion/feel/cues/fx/bench/progress/worked
```

- Both new crates go in the root workspace (`Cargo.toml`, `members = ["crates/sim"]` today). Their only dependencies are `serde` and `serde_json`, which are already in `Cargo.lock`, so there are no new downloads and `cargo test --locked` covers them in CI. No `noise` crate: value noise is about 40 lines in `voxel::gen`. `bevy-client` (its own workspace) adds them as path dependencies, with `default-features = false` where that applies.
- **Determinism:** fixed iteration order (chunk index, then cell index); no `HashMap` iteration in a step; no `sin`, `exp` or `powf` inside a sim step (use tables and `+ - * /` only, which are IEEE-exact in both wasm and native); a seeded xorshift RNG stored in the world. The same seed plus the same emits give a bit-identical world hash on native and wasm. `matter::World::hash()` is the test hook.

### 3.1 Reused as is
- `player.rs` and `motion.rs`: `Player::step(input, dt, solids: &[Box3], ice_sheets: &[Box3])` (`player.rs:84`) already takes plain boxes. The voxel view passes the solid-block boxes that overlap the player's swept box (at most about 60), and passes ice blocks as `ice_sheets`, so `onIce` and the 4.0 m/s² ice acceleration carry over. `motion.rs` is not edited.
- `feel.rs`, `cues.rs`, `clock.rs`, `fx.rs` (`Fx::burst`, `spawn_beads` for Leidenfrost), the viewmodel, the crosshair, aim assist, pause, settings, the HUD shell, `bench.rs` + `web/bench.js` (Formula Bench), `progress.rs` + `web/progress.js`, `worked.rs` + `web/worked.js`, analytics, and `spells.rs` with `content/spells.json`.
- `chem::electrolysis::tubes()` (`chem.rs:1009`): the Hoffman voltameter geometry with the 2:1 H2:O2 columns. It is drawn again at the cave rig.
- Caption text style and per-band lines: the pattern of `level2::caption(kind, band)` (`level2.rs:235`) is copied for matter events, with every line sourced in `content/science-sources.md`.
- The fire finder arrow (`#firefinder`, `web/hud.js:595`) becomes the objective arrow in voxel mode. It reads a new `objBearing` key, using the same bearing maths (`cues::bearing_deg`).

### 3.2 Replaced in voxel mode (kept, frozen, in classic mode)
- `world.rs`, `level2.rs`, `level3.rs`, `chem.rs`'s `World`, `levels`, `level_visuals`, `level3_visuals`, `Level`, `LevelRoot` and the arches. None of these files is edited or deleted by this item. They run only when `Mode::Classic`.
- Picking in voxel mode is the voxel raycast (DDA), not `pick` over `Chem::targets` (`main.rs:1454`). Spell landing in voxel mode calls `blockchem::on_impact`, not `World::apply`.

### 3.3 Mode switch (main.rs, owned by Lane A)
- `enum Mode { Voxel, Classic }`, a resource read once at startup. **Default `Voxel`.** `Classic` when the URL has `classic` (any value) or `level=1`. The classic courtyard stays for this item as the fixture that the Bench, worked-cast, feel, mastery and analytics checks run on (section 11). `?level=2` and `?level=3` now open the block world at `?biome=frozen` and `?biome=marsh`: the block world replaces those levels, and Paawan approved retiring the checks that tested only them. `level2.rs` and `level3.rs` are left in the tree, still reachable through the classic courtyard's arch, and are deleted in a follow-up (proposed I-053) together with the classic courtyard once the kept checks are ported.
- `Classic` registers today's systems unchanged. `Voxel` registers `voxel_view`, `blockchem` and `story` and skips `world::build` and `levels`. Shared systems (`control`, `aim_and_cast`, `viewmodel`, `fx::update_fx`, `publish`, `bench::run`) branch on `Mode` at the few places where they touch `Chem` or `Colliders`. Section 9 lists those places.
- In classic mode `publish` writes **exactly today's keys**, so the key-order checks pass unedited. Voxel mode appends its keys after `ventsLit` (section 8).
- `?biome=village|frozen|marsh|cave|kiln` (voxel only, works without `?debug` so a judge can jump in): spawn at that biome's spawn point, with every earlier quest step marked done and the pack holding what those steps would have given. Spell stages are not changed, because progress belongs to the player.

## 4. World layout

The world is 256 × 96 × 256 blocks. x points east, z points south, y points up. The sea or river level is y = 30, and the village ground is y = 34. The edge of the world is a ring of tall protected mountains, 8 blocks thick.

| Biome | Extent (x, z) | Ground y | Contents (stamped) |
|---|---|---|---|
| **Village** (spawn) | x 96..160, z 162..230 | 34 | Spawn (128, 35, 205) facing north. **Formula Bench** (124, 35, 200). **Invention Bench** (132, 35, 200). Workshop hut of `Planks` and `Wood` (138..146, 186..194), burning at start (P1). Houses, trees, a well (water source). The Kiln is visible on its terrace to the north |
| **Kiln terrace** | x 112..144, z 162..176 (raised to y 40, steps up from the village) | 40 | The **Great Kiln** (brick dome, chimney to y 60, the landmark). **Kiln door** of protected `Runestone`, which opens only with the Kiln Spark in the pack. Inside: the kiln hearth (the P5 target) and a glass-sand bed |
| **River ford** | the river runs z 150..158 across the world | river surface y 30, banks y 34 | A protected canyon. Its walls are 6+ blocks of `Cliffstone`, except at the ford x 122..134, where the banks step down to y 31 on both sides. Water is 3 deep: too deep to wade (section 5.4). P2a: freeze it |
| **Frozen Fields** | x 8..120, z 40..150 | 38-50 (hills) | Snow, ice, pines. The **ice gate** fills a 4-wide opening in a protected cliff band (z 86..94, x 8..120) at x 58..62 (P2b). Behind it: the **coal seam** (x 40..80, z 60..84, coal ore at y 36-42), and the **east pass** to the Marsh at (118, 44, 70) |
| **Marsh** | x 136..248, z 40..150 | 30-33 | Shallow pools, mud, reeds, boardwalk of `Planks`. **Methane hollow** (P3): a protected boardwalk ravine at x 186..196, z 96..106, floor y 27, with 6 gas vents. The **cave mouth** at (220, 32, 58). The **shortcut door** in the protected cliff between the Marsh and the Village (x 160..166) at z 180. It opens from the Marsh side only (a lever), and from then on in both directions |
| **Crystal Cave** (dungeon) | x 188..248, z 10..70, y 6..26 | cave floor y 8-12 | Underground, roofed, dark. Crystal clusters (emissive, dim). The **electrolysis rig** room (226..236, 24..34, floor y 10). The **balloon shaft** (240, 10..24, 20): a crate on a rope under a 14 m protected shaft, with the **Kiln Spark** on the ledge at y 24 (P4). A ladder exit to (200, 33, 140) near the shortcut door |

**Stamps** (`voxel::stamp`) are hand-authored block templates placed after the noise terrain, at fixed positions: bench plaza, workshop, Kiln, ford, ice gate, coal seam, methane hollow, cave rooms, balloon shaft, shortcut door. Each stamp also writes its **protected box**. **Protected** is a world-level list of boxes (not a cell bit, so the diff log cannot change it). Inside a protected box, `can_break` and `can_place` are false. This stops a player from digging under a puzzle or bridging over it with blocks. The rule applies only to puzzle volumes and the world edge, so everything else can be dug and built. A refused dig plays a dull "tunk" and shows a caption the first time: "This old runestone will not budge. Try chemistry."

**Walking route** (Must path): spawn → workshop 15 m → ford 50 m → ice gate 65 m → coal seam 25 m → east pass 60 m → hollow 75 m → cave mouth 45 m → rig 40 m → shaft 20 m → ladder exit 70 m → shortcut door 40 m → Kiln 60 m. About 565 m. Walking takes about 115 s; with puzzles and builds the run is 10-15 min.

**Biome placement is fixed; noise only shapes the ground inside each biome.** That way every seed gives the same puzzles, and the golden test can use coordinates. The seed is `0x52_47_31` unless the URL has `?seed=` (debug only).

## 5. The voxel and matter model

### 5.1 Cell (4 bytes, `voxel::Cell`, `#[repr(C)]`)
| byte | field | meaning |
|---|---|---|
| 0 | `mat: u8` | material id (row in the material table) |
| 1 | `level_state: u8` | low nibble: liquid level 0-8 (8 = full); high nibble: state bits `BURNING=1, SOURCE=2, SETTLED=4, LIT=8` (LIT: a torch, or a lit kiln) |
| 2-3 | `temp: u16` | temperature in units of 0.05 K, 0..3276 K (0 means "ambient, untouched": read as 293.15 K) |

A chunk is 32³ cells, 128 KB. A chunk that is all one cell value (all air, or all stone) is stored as `Uniform(Cell)` and takes no array. Generation keeps about 90-110 of the 192 chunks as arrays (12-14 MB), which is inside the 96 MB WASM budget that the classic clock check holds the game to.

### 5.2 Materials (`content/materials.json`, loaded and validated by `voxel::materials`)
Each row has: `id`, `name`, `phase` (solid, liquid, gas, none), `density_kg_m3`, `melt_c`, `boil_c`, `heat_capacity_j_kgk`, `conductivity_w_mk`, `flammability` (0-1, with `ignite_c` and `burn_s`), `hardness_s` (dig time), `drops` (item), `atlas` (tile index), `transparent`, `solid`, and a `source` key into `content/science-sources.md`. A validator test fails if any row lacks a source.
Must rows: `air, dirt, grass, sand, stone, cliffstone, runestone, wood, leaves, planks, snow, ice, water, steam, mud, coal_ore, iron_ore, crystal, glass, iron, brick, ash, torch, bench, invention_bench, kiln_hearth, gas_vent`. Gases (CO2, H2, CH4, O2) are not blocks; they live in the gas field (5.5).

### 5.3 Matter step (`matter::World::step`, 10 steps/s, fixed `DT = 0.1 s`)
Each step runs only on **active chunks**: a chunk is active while any of its cells changed in the last 3 s, or while it holds a burning, hot (|T − ambient| > 2 K) or unsettled liquid cell. Each pass has a hard cap per step (below). Work that goes over the cap carries over to the next step, so the world gets slower but never stalls the frame. Order:
1. **Heat:** conduction between face neighbours for cells in the hot list only. ΔT uses the materials' conductivity and heat capacity, and it is clamped so a step cannot overshoot the neighbour. Latent heat is a buffer at melt and boil: ice holds at 0 °C until it has absorbed the latent heat. Cap: 8 000 cell updates per step.
2. **Phase:** ice → water above 0 °C (water level 8, `SOURCE` off); water → ice below 0 °C, for the **top cell of a column first**, so a river freezes from the surface down and the ice is walkable; water → steam above 100 °C (the steam rises one cell per step and condenses or vanishes after 5 s); sand → glass when T ≥ 1700 °C (quartz melts near 1710 °C); iron_ore → iron when T ≥ 1200 °C and a `coal_ore` cell touches it (carbon reduces iron oxide: 2Fe₂O₃ + 3C → 4Fe + 3CO₂; the coal cell is used up). There is no separate charcoal block in this item; "coal" stands for the carbon fuel, sourced as such. **Leidenfrost:** a Frost emit landing on water above 10 °C shows beads (reusing `Fx::spawn_beads`) before it freezes, as level 2 does.
3. **Fire:** a `BURNING` cell burns for `burn_s`, heats its neighbours to the fire temperature (wood fire about 1100 °C, from `grid.rs` `WOOD_FIRE_C`), and lights flammable neighbours that reach `ignite_c` (wood 300 °C). It then becomes `ash`, or `air` for leaves. A fire goes out if a water cell or a water emit touches it, or if its gas-field cell has O2 below 15 % (CO2 above about 28 %, because a flame needs about 15 % O2). A `gas_vent` burns forever (methane keeps bubbling up) unless its gas cell is smothered. Water or Frost on a vent knocks the flame down for `FLICKER_S` = 1.5 s (the value from `chem.rs:52`), and then it relights. Cap: 2 000 burning cells.
4. **Water** (volume-conserving, integer levels): a liquid cell moves as much as it can down into air or a lower liquid level, then shares sideways so its level and its 4 neighbours' levels even out (all integer, with a fixed tie-break order). The sum of levels over the world changes only through sources, drains (the world edge, evaporation to steam) and emits. That is a unit test. `SOURCE` cells (the river, the well) refill to 8. Cells with no change for 2 steps get `SETTLED` and are skipped until a neighbour changes. Cap: 4 000 liquid cells per step.

### 5.4 Deep water and the ford
Water is never solid. A player whose feet are in water 2 or more cells deep is pushed back to the last dry standing position within 0.3 s, with a splash and a shiver sound, frosted screen edges for 1 s, the Sprout snowflake picture (the existing "freeze" icon from I-035), and a caption: "Too deep and icy to wade. Freeze it into a bridge?" (`blockchem::wade_push`, the same job as `level2::bank_push`). Swimming is Won't for this item. Ice cells are solid and slippery.

### 5.5 Coarse gas field (`matter::gas`)
- One gas cell per 2 × 2 × 2 blocks: 128 × 48 × 128 cells, stored per chunk (16³ per chunk, only for active chunks plus a 1-chunk margin). Each gas cell holds the mole fractions of `co2, h2, ch4, o2` as `u8` (in 1/255 steps); the rest is air (N2 + O2 at 21 %).
- A gas cell is closed if 5 or more of its 8 blocks are solid. Each step: **buoyancy:** each species moves up or down one gas cell according to its density against air (CO2 1.98 kg/m³ sinks; H2 0.09 and CH4 0.72 rise; densities from `realgame_sim::species` and the material table's sources); **spread:** it shares sideways with open neighbours (fixed order); **sky vent:** a cell with open sky above loses H2 and CH4; a hollow keeps CO2, which leaks back to air over 60 s. Cap: 6 000 gas cells per step.
- A flame is smothered when its gas cell's O2 falls below 15 %. A CO2 emit is 1 m³ of gas (one gas cell going from 0 to about 1.0 CO2), so a hollow of about 3 × 2 × 3 gas cells fills in about 3 casts. That matches level 3's "fills in three casts" (`HOLLOW_CO2_PER_CAST = 0.34`, `chem.rs:49`).
- **The hollow's gap (P3, added after the critique):** the hollow's rim has one low gap (1 block wide, at the marsh side, x 196, z 101, y 28). The protected box leaves exactly that cell placeable. With the gap open, CO2 spills out through it into the marsh and the vents stay lit; you can watch the mist pour out. Closing it with any block, or Frosting the shallow water in it into an ice dam, lets the hollow fill. So P3 is two steps, and doing them in the wrong order visibly fails.
- **Visible CO2:** pooled CO2 (fraction > 0.3) is drawn as a flat, low, white translucent mist sheet at the top of each pooled gas column (one quad per column, at most 64 in view), like dry-ice fog. Rising H2 is not drawn (it is invisible in reality); the balloon shows it.

### 5.6 Spells enter as matter (the one hook I-051 calls)
`matter::Emit { pos: [f32;3], normal: [i8;3], what: EmitKind, amount: f32 }`, with `EmitKind`:
- **Water** (`water_synthesis` is not the cast; the cast throws liquid water): adds 8 liquid levels at the landing cell. It flows and drains like any water. One cast has a cap: 24 levels in total.
- **Frost** (liquid nitrogen, −196 °C, `chem::LN2_C`): cools a radius of 1.5 blocks around the hit by a set number of joules. One cast freezes the top layer of about a 3 × 3 patch of water. So a 6-wide ford needs about 3 casts per row, close to level 2's three casts.
- **Fire** (methane flame from the gauntlet): heats a radius of 1 block to 1100 °C for 0.5 s, and adds CO2 and H2O to the gas cell, from `methane_combustion`.
- **CO2** (`foam` effect): adds 1 m³ of CO2 to the gas cell at the hit, cold (−20 °C), and it sinks from there.
Client glue: `blockchem::on_impact(spell, hit_pos, hit_normal)` turns a landed spell into one `Emit`. I-051 owns where and when a spell lands. Until I-051 merges, voxel mode lands spells at the voxel raycast hit after `cues::flight_secs` (the delay classic mode uses today).

### 5.7 Balloon lift (P4, `matter::lift`)
`lift_n(gas, volume_m3, payload_kg) = (ρ_air − ρ_gas) · V · g − payload_kg · g`. The cave balloon is 2 m³ and the crate plus the Spark weigh 1.5 kg. Air is 1.20 kg/m³ at 20 °C. H2 gives (1.20 − 0.084) · 2 · 9.81 − 14.7 ≈ +7.2 N, so it rises. O2 gives (1.20 − 1.33) · 2 · 9.81 − 14.7 ≈ −17.3 N, so it does not rise. A test asserts the sign for H2, O2, CO2 and CH4 (CH4 at 0.67 kg/m³ also lifts, which is real, but the cave has no methane to collect). The crate is a client entity (a simple box on a rope) moved by the sign of `lift_n`, at 1 m/s up to the ledge.

### 5.8 Raycast, collision, meshing (`voxel`)
- `raycast(origin, dir, max_t) -> Option<Hit { cell: [i32;3], normal: [i8;3], t }>`: Amanatides-Woo DDA. It skips non-solid cells; it hits liquids only in `RayMode::Fluids`, which Frost and Fire use to target water.
- `solid_boxes(aabb) -> Vec<[f32;6]>` and `ice_boxes(aabb)`, which feed `Player::step`.
- `mesh_chunk(&World, chunk) -> ChunkMesh { opaque: MeshData, clear: MeshData }`: greedy meshing per face direction over 32 × 32 slices. A face is drawn only when the neighbour is air or a transparent material of a **different** id. Quads merge only when material, AO and light match. Per-vertex AO uses the 3-neighbour rule. Water's top face is lowered by `(8 − level)/8`. `MeshData` is plain `Vec<[f32;3]>` positions, `Vec<[f32;3]>` normals, `Vec<[f32;2]>` uvs, `Vec<[f32;4]>` colours (AO and tint), and `Vec<u32>` indices: no Bevy types, so a server or the VR client can reuse it.
- `DiffLog`: for each chunk, a sorted list of `(cell index u16, Cell)` that differ from the seed's generated chunk. It has `encode` and `decode` (version byte, then run-length). Storing it in IndexedDB or OPFS is Should; the format is Must, so the save never needs a migration.

### 5.9 Client view (`bevy-client`)
- One entity per non-empty chunk per pass: `opaque` (one `StandardMaterial` with the atlas, vertex colours on, `perceptual_roughness 0.9`) and `clear` (water, ice, glass and crystal, alpha blend). Each chunk entity has an `Aabb`, so Bevy's frustum culling skips chunks out of view. Chunks have `NotShadowCaster` when `?low` is set.
- **Mesh queue:** dirty chunks are sorted by distance to the player, and the queue has a time budget of **4 ms per frame** (2 ms with `?low`); each frame meshes at least one chunk. Chunk meshing is pure, so the same function later runs in a Web Worker (Should) or a native thread pool. Bevy's `AsyncComputeTaskPool` on wasm without atomics runs on the main thread, so this spec does not count on threads for Thursday.
- **Initial build:** the intro card shows while chunks mesh, spawn ring first. The first frame you can play is when the 3 × 2 × 3 chunks around spawn are meshed. The rest fills in behind the fog.
- **View distance:** 6 chunks (192 m), fog starting at 120 m on low-end, or 8 chunks with the fog at 180 m on the default tier. The world is 256 m wide, so from most places the whole world is inside view distance anyway. The budget below therefore assumes every chunk is meshed, and culling only decides how many are drawn.
- **Atlas:** a 256 × 256 `Image` generated at startup, 16 × 16 tiles, nearest-neighbour sampling, no mipmaps (WebGL2-safe), no asset downloads. Style: "toon block": flat base colours taken from the game's existing palette (`StateMats` colours, the sky `ClearColor`), a 1 px darker border, and one small seeded detail per material (wood rings, stone speckle, ice streaks, crystal facets). Runestone gets its own carved, faintly emissive tile. Noisy photo-like textures are out: they would clash with the low-poly toon courtyard.
- **Dark cave:** the ambient brightness falls with the "sky cover" over the player (a top-down column check in `voxel`), from 700 to 120 (dim, never black; a 6-year-old should not be scared). Crystals glow, brighter at Sprout. A held or placed `torch` adds a `PointLight` (no shadows, at most 6 active, nearest first). Without a torch the rig room is too dark to read the tubes, and the objective says "It's too dark. Make a torch at the Formula Bench (wood + coal)."
- **Break and place** (one click verb, so it is still one control scheme): the hotbar is keys 1-4 for spells (as today, so `keys 1-4 select spells` holds), key 5 the **Pick**, and keys 6-9 four pack slots you choose in the pack. A left click with the Pick held digs the block under the crosshair over `hardness_s`, with a crack overlay. A left click with a block held places it on the face you are looking at. Placing is never needed on the Must path except one block (or an ice dam) in the P3 gap, so a young child can ignore keys 6-9. The Pick has a big hotbar picture, and at Sprout and Seedling the step-4 objective shows the Pick and coal pictures. The wheel cycles 1-9 in voxel mode only. Touch and point-and-click: the same, on the block under the cursor.
- **WebGL2 is the baseline**, as `default = ["bloom", "webgl2"]` already is in `crates/bevy-client/Cargo.toml`. Nothing in this item needs WebGPU: no compute, no storage buffers, no texture arrays. The WebGPU build (`WEBGPU=1 build-web.sh`) stays optional, and the matter compute path is a later item. If WebGL2 is missing, boot shows a plain card ("Your browser can't run 3D here. Try Chrome on a recent laptop.") instead of a blank page. That is a small `web/boot.js` check, owned by Lane C.

## 6. Gathering, pack, crafting, invention (`realgame_sim::valley`)

```rust
pub enum Item { Block(u8 /*material id*/), Pick, Torch, O2Flask, H2Balloon, KilnSpark }
pub struct Stack { pub item: Item, pub n: u16 }          // n <= 64 (Block), 1 for tools/keys
pub struct Pack { pub slots: [Option<Stack>; 20], pub hot: [Option<u8>; 4] /* keys 6-9 -> slot */ }
impl Pack {
    pub fn add(&mut self, item: Item, n: u16) -> u16;     // returns leftover
    pub fn take(&mut self, item: Item, n: u16) -> bool;   // all or nothing
    pub fn count(&self, item: Item) -> u32;
    pub fn to_json(&self) -> String;                      // HUD + state key `pack`
}
pub struct Recipe { pub id: &'static str, pub needs: &'static [(Item, u16)], pub gives: (Item, u16), pub source: &'static str }
pub const RECIPES: &[Recipe] = &[ /* torch = 1 wood + 1 coal_ore -> 4 torch */ ];
pub fn craft(pack: &mut Pack, id: &str) -> Result<(), CraftError>; // NotEnough{item, have, need}
```
- **Gathering:** digging a block adds its `drops` item to the pack (`dirt → dirt`, `coal_ore → coal_ore` shown as "Coal", `wood → wood`, `ice → nothing` because it melts in your hand, with a caption saying so). If the pack is full, the block is not removed and the HUD says "Pack full".
- **Rig items:** the electrolysis rig is a block you use (press E or click while next to it, within 2 m): it takes 1 water (from a placed water block or the rig's basin), shows the tubes filling 2:1 over 4 s (`chem::electrolysis::tubes()`), and gives `H2Balloon` (from the cathode tube) and `O2Flask` (from the anode tube). You choose which one to tie to the crate; the wrong gas does not lift it.
- **Crafting UI:** a "Make things" tab in the Formula Bench overlay (`web/bench.js`), with the same modal rules (aria-modal, pauses the game, Escape closes). Each recipe card shows its inputs (pictures for Sprout, names for Seedling and up) and greys out when you are short. Must recipes: **Torch** (1 wood + 1 coal → 4 torches) and **Furnace** (8 stone → 1 furnace block, added after the critique). A Furnace is a placeable block: put sand or iron ore plus coal in it (use it like the rig), light it with Fire, and the matter sim heats its contents; with an O2 flask it reaches 1700 °C and makes glass, with coal alone it reaches about 1250 °C and makes iron. It reuses matter's glass and iron rules (M8), so it adds a recipe, a block and a small use-panel, not new chemistry. Glass and iron blocks are placeable. The Must path does not need the Furnace; it is there for players who want to make things, and it is what turns the sand-to-glass and ore-to-iron rules into play.
- **Pack UI:** `Tab` or `I` opens `web/pack.js`: 20 slots, drag or click a slot then press 6-9 to bind it, item counts at least 18 px, Sprout pictures. It pauses like the bench does.
- **Invention Bench** (`valley::invent`): you put 1-3 items from the pack on the bench, and the bench builds the reaction from what they are (Coal = C(s), O2Flask = O2(g), H2Balloon = H2(g), wood = C(s), water = H2O(l)). `realgame_sim::reactions` judges it: if a balanced reaction in `content/reactions.json` uses only those reactants and is exothermic, the bench says what it makes and how hot it gets. The Kiln relights only if the reaction's flame temperature in pure O2 is at least 1700 °C, which `carbon_combustion` (C + O2) and `water_synthesis` (2H2 + O2) both reach (`grid.rs` `FLAME_C_IN_O2 = 2800`). Otherwise there is a kind reason: nothing to burn (water), no oxidiser (coal alone: "Coal needs oxygen to burn. Air has only 21 %; the Kiln needs more"), or it is already burnt (CO2). The bench uses the existing formula parser, so Apprentice and Adept can type `C + O2 -> CO2` instead of choosing cards. That is the "invent a new reaction" beat: you have never cast it, and you put it together from what you gathered. At Sprout, once the pack holds a working pair, the bench shows that pair as two big glowing picture cards (she still picks them herself); a wrong mix shows a sad cold Kiln picture, and every line is spoken.

## 7. Gates and progression (`realgame_sim::valley::quest`)

The quest is a pure state machine over events; it never polls the world. Events come from `blockchem`, `voxel_view` and the benches.

| Step | Objective line (Seedling; per-band variants in `quest.rs`) | Done when (event) | Opens |
|---|---|---|---|
| 0 Intro | (intro card) | `IntroDone` | - |
| 1 Workshop fire (P1) | "The workshop is on fire! Learn Water at the Formula Bench, then put it out." | no `BURNING` cell left in the workshop stamp box **and** at least one was put out by water (Fire on it spreads it; Frost cools it but does not put it out fast) | Bench hint for Frost; the elder's line points north |
| 2 Ford (P2a) | "The river is too deep. Learn Frost and freeze a path across." | the player stands at z < 150 inside the ford box | Frozen Fields |
| 3 Ice gate (P2b) | "An ice gate blocks the cliff. Melt it with Fire." | the gate stamp's ice is all gone (water or air) | Coal seam, east pass |
| 4 Coal | "Dig some coal from the seam (hold the Pick, key 5)." Sprout and Seedling also see the Pick and coal pictures | pack has 1 or more `coal_ore` | Torch recipe |
| 5 Methane hollow (P3) | "Burning marsh gas blocks the boardwalk. Learn CO₂, close the gap in the rim, and smother it." | every vent in the hollow is out at the same moment and the player has crossed the hollow | Cave mouth |
| 6 Torch + rig + balloon (P4) | "It's dark. Make a torch, split water at the rig, and lift the crate." | pack has `KilnSpark` | Ladder exit, shortcut door lever |
| 7 Kiln (P5) | "Bring the Kiln Spark home and invent a reaction hot enough to relight the Great Kiln." | Invention Bench accepts a qualifying reaction while the player has `KilnSpark`, `Coal` or `H2Balloon`, and `O2Flask` | Ending |

**Gate rule:** the gates are **physical**, not quest checks. The ford is passable only as ice, the gate only when melted, the hollow only when smothered, the cave readable only with light, the ledge reachable only by balloon, the Kiln door only with the Spark. The quest only reports progress. A spell you have not learnt can still be cast, as today (the bench gates nothing; `bench: an Unknown spell still casts`), but in worked mode its cast opens the worked builder (`worked::route`), so in practice you learn it at the bench. The objective line tells you to learn it there.
`valley::usable(spell, band, book) -> bool` is exported for the HUD hint only: "You need Frost: build it at the Formula Bench."

**Revisit:** the world keeps its state; nothing respawns. The ford stays frozen, because the ford stamp's water is still (no source upstream of the ford row) and the air there is held at −5 °C, so the ice cannot melt back. The ice gate stays melted. The hollow's CO2 leaks away over about 60 s, and the vents then relight, so the hollow must be smothered again to cross it a second time; the gap you closed stays closed, so that takes 3 casts. The shortcut door is why you rarely need to: after the cave, the ladder exit and the shortcut door bring you home through the Village. **Reasons to go back:** the Kiln and both benches are in the Village (you return to learn spells and to finish), the coal seam is behind the ice gate (more coal for torches or the Furnace), and the Furnace in the Village turns sand from the Marsh into glass.

**Change from the plan's "worked casts until mastery, then Invention Bench":** the Invention Bench needs the right materials and a valid reaction, not mastery. Mastery stays the long-term progression, with its star and instant cast, and is a bonus, never a gate in this item. Reason: mastery needs 2 days.

## 8. Intro, objective HUD, ending, audio (state keys)

- **Intro:** 3 cards over slow camera moves around the cold Kiln and the village (a scripted camera path, no player input, about 20 s, skippable with any key or click after 1 s; the first card holds for 3 s at Sprout). Each card has a big generated picture (the cold Kiln, frost on the chimney, the apprentice with the gauntlet). Card text (Seedling): "The Great Kiln kept our valley warm and made all our glass." / "Last night it went cold." / "You are the apprentice. Learn real chemistry at the Formula Bench, explore the valley, and relight the Kiln." Sprout hears it read aloud (the existing `speech.ts` path in the bench), Apprentice and Adept get one extra line about the reaction. The age chooser (bench first-open) moves to before the intro in voxel mode, so the intro text fits the band. It gets a focused **Just play** button (Seedling), so Enter or one click starts. Without a band, the intro uses Seedling text.
- **Objective HUD:** `#objective` (`web/hud.js:603`) shows the current step's line and the step number "3 of 7". The arrow (`#firefinder` reused with a new `objective` class) points to the step's target when it is off screen. At Sprout it is always on: an edge chevron when the target is off screen, and a bobbing marker over the target when it is in view. A biome name toast shows for 2 s when you enter a biome ("The Frozen Fields").
- **Ending:** when step 7 completes: the hearth cell becomes `LIT`, the Kiln roars (SFX and fx bursts), the camera pulls back to show the chimney smoke and the village windows (glass blocks) glowing, and the music resolves. The card says: "You relit the Great Kiln!", the time played, the spells you learnt, and the puzzles you solved, with **Keep exploring** (back to free play, no objective) and **Play again** (reload with a fresh world). It sends analytics `quest_complete` with `quest: "valley"` (this event already exists; no new data).
- **Music** (`web/music.js`): a WebAudio sequencer with a pentatonic motif per biome (village: plucked major; frozen: bell tones, slow; marsh: low drone + frogs; cave: echoing sparse notes; Kiln: warm full chord at the ending). It crossfades over 2 s on biome change. It uses the existing Sound setting and a new Music slider in the pause card. It is ducked while speech plays.
- **SFX:** the existing `makeSound` (`web/hud.js:84`) is extended with: dig crunch (pitched by material), place thud, splash and flow trickle, ice crackle, hiss (a flame put out), whoosh (CO2), bubble (rig), Kiln roar, a "tunk" for protected blocks, and the step-complete chime.
- **State keys** (voxel mode only, appended after `ventsLit` in this order): `mode` ("voxel"), `biome` (id string), `quest` (step 0-8), `questLine` (the objective text), `pack` (JSON array of `{item, n}`), `hot` (selected hotbar slot 1-9), `lookCell` (`[x,y,z,mat]` or `null`), `digT` (0..1), `objBearing` (degrees or `null`), `chunksMeshed`, `meshQ`, `activeChunks`, `matterMs` (last step CPU ms), `gasCO2Max` (max CO2 fraction in the player's chunk), `ending` (bool). In voxel mode `objs` is `[]`, `level` is 0, `burning` counts burning cells in the player's view distance, and `outcome` is "playing" until the ending, then "won".
- **Debug commands** (`?debug`): `tp x y z` (3 numbers, voxel mode; the 2-number form stays for classic), `give <item> <n>`, `quest <n>`, `setcell x y z <mat>`, `probe x y z` (writes `probe: {mat, level, state, tempC, co2}` into the next state), `matter step <n>`, `matter pause|run`.

## 9. Performance budget (one scene, low-end hardware)

Target hardware: a modern laptop (about 4 years old or newer) with Chrome; Chromebooks are nice to have. The gate is still B2's cloud rule (median CPU frame ≤ 16 ms at 4× throttle with `?low`) and B3's TTI ≤ 8 s, because CI checks do not change; those numbers leave headroom on a modern laptop at the default tier.

| Item | Budget (at `?low`, 4× throttle) | How |
|---|---|---|
| Generation | ≤ 1.0 s once, before the first frame | 192 chunks; uniform chunks are not allocated; a column heightmap first, then fills |
| First playable frame (TTI) | ≤ 8 s (B3) | Spawn ring of 18 chunks meshed during the intro; the rest of the chunks are meshed behind the fog |
| Meshing | ≤ 2 ms per frame (`?low`), ≤ 4 ms default | Time-sliced queue; at least 1 chunk per frame; greedy; 32³ chunk ≈ 1-3 ms native, measured by a bench test that prints and is not asserted |
| Draw calls | ≤ 250 (`?low`) | ≤ 2 per chunk × ~110 chunks; frustum culling; no shadows on chunks at `?low` |
| Triangles | ≤ 400 k in view | Greedy meshing; leaves are a solid cube, not cross-quads |
| Matter step | ≤ 3 ms per step at 10 Hz (≤ 0.3 ms per frame on average at 100 fps, ≤ 1 step per frame) | Active chunks only; caps per pass (5.3); carry-over |
| Remesh from matter | ≤ 2 chunks per step | Changes are combined per chunk per step |
| Memory | WASM ≤ 96 MB | ~14 MB of cells + ~4 MB gas + meshes in GPU only (`RenderAssetUsages::RENDER_WORLD`) |
| Download | +≤ 0.3 MB brotli | No new crates, no images, no audio files |

The golden section `voxelEngine` reads `CPUFRAME` lines from `boot.js` and `matterMs`, and fails if the median CPU frame over 20 s of walking and casting is above 16 ms at `?low` (the same rule B2 uses). Paawan's Mac run (`docs/loop/mac-perf.md`) gets a voxel workload after the merge.

**Places in shared systems that branch on `Mode`** (Lane A owns them, so the other lanes call functions): `control` (`tp` with 3 numbers; solids come from `voxel::solid_boxes` instead of `colliders.solid`, `main.rs:1149`; the `level2::bank_push` call becomes `blockchem::wade_push` in voxel mode), `aim_and_cast` (picks with `voxel::raycast`), `projectiles` (landing calls `blockchem::on_impact`; captions from `blockchem::caption`), `react` and `sync_visuals` (skipped in voxel mode), `publish` (the appended keys), `bench::run` (`bench::at_bench` reads the bench site from a new `bench::site()` that returns `STATION` in classic mode and (124, 200) in voxel mode; `BENCH_ON` stays true in voxel mode).

## 10. Lanes, ownership and the contract

### Contract step (Lane A's builder, Monday morning, first merge, about 1.5 h)
It merges before the lanes start and holds stubs that compile and tests marked `#[ignore]` where the body comes later.
1. Root `Cargo.toml` members: add `crates/voxel` and `crates/matter`. `crates/voxel/Cargo.toml`, `crates/matter/Cargo.toml` (deps: `serde`, `serde_json`; matter also `realgame-voxel` and `realgame-sim` with `default-features = false`). `Cargo.lock` updated (no new packages). `crates/bevy-client/Cargo.toml` adds both crates as path dependencies; its `Cargo.lock` is updated.
2. `crates/voxel/src/lib.rs`: `Cell` (5.1) with its accessors, `MatId` constants for every Must row (5.2), `CHUNK = 32`, `WORLD = [256, 96, 256]`, the `World` struct with `get`, `set` (marks the chunk dirty), `dirty_chunks`, `chunk_index`, the protected-box list with `can_break` and `can_place`, and the signatures of `raycast`, `solid_boxes`, `ice_boxes`, `mesh_chunk`, `generate(seed)` and `DiffLog`.
3. `content/materials.json` with every Must row and its schema; values marked `"todo"` are allowed only for the fields Lane B fills on Tuesday, and the validator test is `#[ignore]` until then.
4. `crates/matter/src/lib.rs`: `Emit`, `EmitKind`, `MatterEvent` (`Melted{cell}`, `Froze{cell}`, `Ignited{cell}`, `Extinguished{cell, by}`, `Smothered{vent}`, `Relit{vent}`, `Glass{cell}`, `Iron{cell}`, `Leidenfrost{pos}`, `Flowed{chunk}`), `World::{new, emit, step(&mut voxel::World, &mut Vec<MatterEvent>), hash}`, `lift_n`.
5. `crates/sim/src/valley/mod.rs` (and the `lib.rs` line): `Item`, `Stack`, `Pack`, `Recipe`, `QuestEvent`, `Quest::{new, on(event) -> Option<Step>, step, line(band)}`, `invent::judge(items, band, text) -> Invention`, `usable`.
6. `main.rs`: `Mode`, the `?classic` / `?level` rule, the plugin calls `voxel_view::plugin`, `blockchem::plugin`, `story::plugin` (empty stubs in their own files), the `publish` hook that asks each plugin for its keys, and the event plumbing: `PlayerEvent` and `MatterEvent` are Bevy messages that `story::on_event` (a stub) already receives, so Lane C never waits on Lane A's `main.rs`.
7. `golden`: base URL gets `classic=1` added for every existing section (one change in the `base` setup line). The `frozenGate` and `marsh` sections, their `onlyRuns` entries and call lines, and the `worked: handoff` run are removed (the **retire** list in section 11), so CI stays green from the first merge. Empty `voxelEngine`, `blockMatter` and `valleyStory` functions and their `onlyRuns` entries, placed **after `marsh`** in the order A, B, C, so each lane edits only its own function body.
8. `tests/e2e/perf.mjs` and `tests/e2e/analytics-e2e.mjs`: append `classic=1` to the URL they load, so they keep measuring today's workload until a voxel workload exists.

### Lane A: voxel engine and view
Owns `crates/voxel/**`, `crates/bevy-client/src/voxel_view.rs`, `crates/bevy-client/src/voxel_tex.rs`, `crates/bevy-client/src/main.rs`, the `voxelEngine` function in `golden`, and `Cargo.toml` / `Cargo.lock` (both workspaces).
Delivers: generation and stamps for all biomes (section 4), protected boxes, raycast, collision boxes, greedy mesher with AO, `DiffLog`, the chunk entities and mesh queue, the atlas, sky cover and lighting, break and place with the Pick, the 1-9 hotbar in the Rust state, the `Mode` branches (section 9), `?biome=` spawns, and the debug commands `tp`, `setcell` and `probe`.

### Lane B: matter
Owns `crates/matter/**`, `content/materials.json` (values and sources after the contract), `crates/bevy-client/src/blockchem.rs`, the new rows in `content/science-sources.md` (append-only section "Block world"), and the `blockMatter` function in `golden`.
Delivers: section 5.3-5.7 rules, the gas field, `on_impact`, `wade_push`, per-band captions for matter events (new rows in `blockchem.rs`, same shape as `level2::caption`), gas visuals (a few translucent billboard puffs per gas cell above 0.3, at most 64), flame and steam particles through `Fx`, the rig's tube fill, and the balloon crate entity.

### Lane C: game layer (pack, crafting, benches, story, audio)
Owns `crates/sim/src/valley/**`, `crates/bevy-client/src/story.rs`, `web/pack.js`, `web/story.js`, `web/music.js`, `web/hud.js` (objective, arrow, SFX), `web/bench.js` (the "Make things" tab and the Invention Bench view), `web/index.html`, `web/style.css`, `web/boot.js` (the WebGL2 check and module loads), `crates/bevy-client/src/bench.rs` (`site()`), and the `valleyStory` function in `golden`.
Delivers: section 6-8.

### Contract rules between lanes
- Lane B reads and writes cells **only** through `voxel::World::{get, set}` and never meshes. Lane A never decides what matter does; it remeshes `dirty_chunks()` after each matter step.
- Lane C learns about the world **only** through `MatterEvent`s and the voxel view's `PlayerEvent`s (`Dug{mat}`, `Placed{mat}`, `EnteredBiome{id}`, `InBox{stamp}`, `UsedRig`, `UsedLever`, `UsedKilnDoor`), which `main.rs` passes to `Quest::on`. Quest code never reads cells.
- Stamp ids and boxes (`voxel::stamp::{WORKSHOP, FORD, ICE_GATE, COAL_SEAM, HOLLOW, RIG, SHAFT, LEDGE, KILN, SHORTCUT}`) are fixed in the contract step. Their exact coordinates come from section 4, and Lane A may move a stamp by up to 4 blocks only if it updates the constant and all three golden sections still pass.
- New state keys are added only by `main.rs` `publish` in the order in section 8; a plugin returns its values, not JSON fragments.

## 11. Acceptance

### Unit tests (all fail on main: the crates and modules do not exist)
**voxel** (`cargo test -p realgame-voxel`):
- V1 `cell_packs_into_4_bytes`: `size_of::<Cell>() == 4`; level and state nibbles round-trip; temp 0 reads as 293.15 K.
- V2 `generation_is_deterministic`: `generate(S).hash() == generate(S).hash()`, and a different seed gives a different hash outside the stamps but the **same** stamp cells.
- V3 `stamps_are_where_the_spec_says`: the bench, Invention Bench, Kiln hearth, gas vents (6), rig and ledge cells hold their materials at their section-4 coordinates; spawn has 2 air cells above a solid cell.
- V4 `protected_boxes_refuse_dig_and_place`: `can_break` and `can_place` are false on every cell of each puzzle box and of the edge ring, and true at 5 sample cells in the open.
- V5 `greedy_mesh_counts`: a 32 × 1 × 32 slab of stone in an empty chunk gives exactly 6 quads; two touching stone blocks give 10 quads (no inner faces); a water block next to stone draws the stone's face, and stone does not draw a face against stone; a 3D checkerboard gives at most `6 × 16384` quads (the upper bound).
- V6 `mesh_skips_hidden_faces_between_chunks`: a stone surface across the boundary of two chunks draws no faces on the boundary.
- V7 `raycast_dda`: hits the right cell and normal for axis-aligned and diagonal rays, at a chunk boundary, and returns `None` past `max_t`; `RayMode::Fluids` stops at water while `Solids` passes through it.
- V8 `solid_boxes_around_player`: at the spawn point the result includes the ground under the feet, has at most 64 boxes, and includes ice cells only in `ice_boxes`.
- V9 `difflog_round_trips`: random edits encode and decode to the same world; an unedited world encodes to under 16 bytes.
- V10 `materials_table_is_sourced` (un-ignored on Tuesday by Lane B): every row has a `source` that exists as a heading key in `content/science-sources.md`, and `melt_c < boil_c` where both are set.

**matter** (`cargo test -p realgame-matter`):
- M1 `frost_freezes_top_layer_first` and `ford_bridges_in_three_casts_per_row`: on the generated world, 3 Frost emits along one row of the ford make a walkable line of ice (`solid_boxes` cover the path from bank to bank for a 0.7 m wide body).
- M2 `fire_melts_ice_gate_in_three_casts`: the gate stamp's ice has no ice left after 3 Fire emits and 5 s of steps; the meltwater flows downhill (some water cell is lower than the gate's base after 5 s).
- M3 `water_conserves_volume`: in a closed basin with no sources, the sum of levels is the same after 500 steps; water never moves into a solid cell or upward.
- M4 `co2_pools_in_hollow`: 3 CO2 emits above the methane hollow give O2 < 15 % in every vent's gas cell within 5 s, all 6 vents report `Smothered`, and the hollow's CO2 does not climb over its rim (the gas cells just above the rim stay below 5 % CO2).
- M4b `hollow_gap_spills_until_closed`: with the rim gap open, 3 CO2 emits leave at least 1 vent lit after 5 s and CO2 shows in the gas cells outside the gap; after `set` of one stone (or 1 Frost emit on the gap's water, making ice) the same 3 emits put every vent out.
- M5 `water_and_frost_only_flicker_vents`: a Water or Frost emit on a vent gives `Extinguished` and then `Relit` within `FLICKER_S + 0.2` s; Fire on a vent changes nothing.
- M6 `h2_rises_co2_sinks`: an emit of each into a closed 6 m tall box puts H2 in the top gas layer and CO2 in the bottom layer after 3 s.
- M7 `fire_spreads_through_wood_and_water_puts_it_out`: the burning workshop stamp spreads to a touching wood block within 3 s; a Water emit on each burning cell puts the workshop out; a Fire emit relights it.
- M8 `sand_to_glass_and_ore_to_iron`: sand held at 1720 °C becomes glass and sand at 1600 °C stays sand; iron ore at 1250 °C next to coal becomes iron and uses the coal; without coal it stays ore.
- M9 `lift_sign`: H2 and CH4 positive, O2 and CO2 negative, for the cave balloon (2 m³, 1.5 kg).
- M10 `deterministic_steps`: the same world and emits give the same `hash()` after 300 steps, twice in a run.
- M11 `caps_hold`: with 20 000 burning wood cells, one step updates at most 2 000 burning cells and carries the rest over.

**sim valley** (`cargo test -p realgame-sim valley`):
- C1 `pack_stacks_and_overflows`: 64 stack limit, leftover on full, `take` is all-or-nothing.
- C2 `torch_needs_wood_and_coal`: `craft("torch")` fails with `NotEnough{coal}` and leaves the pack unchanged; with both it gives 4 torches.
- C3 `quest_advances_only_in_order`: events out of order do not skip steps; repeating an event does not complete a step twice.
- C4 `invention_judges_real_reactions`: `{Coal, O2Flask}` → `carbon_combustion`, relights; `{H2Balloon, O2Flask}` → `water_synthesis`, relights; `{Coal}` → fizzle "needs oxygen"; `{water}` → fizzle "nothing to burn"; typed `C + O2 -> CO2` at Apprentice is accepted; typed `C + O2 -> CO` is refused with the existing `atoms_off` code and its reason (it is unbalanced as written; the reason must not call carbon monoxide impossible).
- C5 `usable_follows_band`: Fire at Sprout is usable while unknown; Frost at Seedling is usable only from Learning.
- C6 `quest_lines_exist_per_band`: every step has a line for all four bands and for no band, at most 90 characters.

### Golden (new sections; each fails on main because the default page is not a voxel world)
**`voxelEngine` (Lane A, default URL with `?low`):**
- E1 `voxel: the default page boots the block world` (`mode === 'voxel'`, `chunksMeshed >= 18` within 8 s of the first state, no console errors).
- E2 `voxel: ?classic and ?level=1 boot the courtyard unchanged` (`objs.length > 10`, 3 burning, and the state keys are exactly today's).
- E3 `voxel: WASD walks on the terrain and the feet follow the ground` (move 10 m over a slope; `pos[1]` changes and is never more than 0.1 m inside a solid cell).
- E4 `voxel: the Pick digs a dirt block into the pack and placing puts it back` (`lookCell` dirt, `digT` reaches 1, `pack` dirt +1, the cell is air in `probe`; key 6 bound to dirt, then a click restores it).
- E5 `voxel: a protected cell cannot be dug` (the ford cliff; `digT` stays 0 and the "runestone" caption shows).
- E6 `voxel: the median CPU frame is 16 ms or less over 20 s at ?low` (CPUFRAME lines; walking a fixed route through 3 biomes while casting).
- E7 `voxel: WASM memory stays under 96 MB after visiting every biome` (tp tour).
- E8 `voxel: no frame meshes more than the budget` (`meshMs` max ≤ 6 ms in the `?low` run, logged).
**`blockMatter` (Lane B):**
- E9 `matter: the deep ford pushes the player back` (walk into the ford; position returns to the bank, caption shown).
- E10 `matter: three Frost casts per row bridge the ford and the player crosses on ice` (`onIce` seen, z < 150).
- E11 `matter: Fire melts the ice gate and the meltwater flows` (`probe` of the gate cells reads water or air; a cell below holds water).
- E12 `matter: Water and Frost only flicker a vent; with the gap open CO2 spills; after closing it three CO2 casts smother the hollow` (vents still lit with the gap open; then `gasCO2Max` > 0.28, every vent out, the player crosses).
- E13 `matter: an O2 balloon does not lift the crate, an H2 balloon does` (crate y unchanged for 3 s with O2; reaches the ledge with H2).
- E14 `matter: Leidenfrost beads on a Frost hit on liquid water` (`fxBeads >= 4`, then 0 within 2 s).
**`valleyStory` (Lane C):**
- E15 `story: the intro shows three cards and any key skips it after 1 s` (and `quest` goes 0 → 1).
- E16 `story: the objective line and arrow follow each step` (with `give` and `tp` shortcuts through steps 1-7; `questLine` changes each time; the arrow is shown when the target is behind).
- E17 `story: the pack opens with Tab, pauses, and Escape closes it` (aria-modal, inert rest, the same checks as the bench).
- E18 `story: the torch recipe is greyed without coal and makes 4 torches with it; a placed Furnace with sand, coal and an O2 flask, lit with Fire, makes glass`.
- E19 `story: the Invention Bench refuses coal alone and accepts coal + O2; the ending card shows with Keep exploring` (`ending === true`, `outcome === 'won'`, `quest_complete` with `quest: "valley"` in the analytics queue on the built site).
- E20 `story: music starts after the first input, changes on entering the Frozen Fields, and is silent with Sound off` (`window.__music` state).
- E21 `story: ?biome=marsh spawns in the Marsh with steps 1-4 done and coal in the pack; ?level=2 and ?level=3 open the Frozen Fields and the Marsh` (no `?debug`).

### Existing golden checks: what the move breaks, and retire / port / keep
**Rule (Paawan, 2026-11-01, via the coordinator):** golden checks that only test the old separate levels (Frozen Gate, Marsh, the level-1 layout, the arches) may be retired once the block world replaces those levels, and each retired check is listed in the PR. Checks for the Bench, mastery, worked casts, casting feel, analytics and chemistry rules stay. CI checks do not change, and nothing is loosened.

**Why they break.** With the block world as the default page, every existing section would read level-1 content (`objs`, `burning`, `firesLeft`, `tp 0 5.5`, the greenhouse) from a page that has none. So the contract step adds `classic=1` to `golden`'s `base` (one line) and to `perf.mjs` and `analytics-e2e.mjs`. Every **keep** check then runs unchanged on the classic courtyard. Classic mode publishes exactly today's state keys, so `clock: state JSON keeps the base keys in order, appends gameTime and dt, and parses every frame` and `streams: fireInView is a boolean, the last state key` are unaffected.

Labels: **keep** means unchanged, on `classic=1`. **Port** means kept on `classic=1` **and** a block-world version is added in this item (named after the arrow); the classic one goes only when the classic courtyard is deleted (proposed I-053). **Retire** means removed in this item's PR, because the block world replaces the level it tests; the PR lists each one.

**Retire: the `frozenGate` section (all 15; `?level=2` now opens the Frozen Fields; its chemistry is ported below):**
- retire: `frozen gate: the exit stays shut before the win (level 1, exitOpen false)` (arch)
- retire: `frozen gate: the level-1 win opens the arch and the banner points to it` (arch)
- retire: `frozen gate: walking into the open arch loads level 2 within 5 s` (level swap)
- retire → ported as E9: `frozen gate: the liquid stream blocks the walk (stopped on the south bank, z > 8.2)`
- retire → ported as M1/M2 and E10/E11: `frozen gate: Fire, Water and CO2 leave the stream liquid; Frost, Water and CO2 leave the gate standing`
- retire → ported as E14: `frozen gate: Leidenfrost beads after a Frost hit on the liquid stream (>= 4, then 0 within 2 s)`
- retire → ported as E22 (below): `frozen gate: with no band the Apprentice Freezing line shows`
- retire → ported as M1 and E10: `frozen gate: three Frost casts freeze the stream 0.34 / 0.68 / 1.00`
- retire → ported as E10: `frozen gate: the player crosses the frozen stream (z < 3), on ice while on it`
- retire → ported as E11: `frozen gate: Fire melts the gate in 3 hits and walking to the lantern wins`
- retire: `frozen gate: the back arch returns to a fresh courtyard with the exit open` (arch)
- retire → ported as E22: `frozen gate: each band sees its own Freezing line; Sprout shows the freeze picture`
- retire → replaced by E7: `frozen gate: level 2 has no more entities than level 1`
- retire → replaced by E7: `frozen gate: 10 level switches leave entities exact and WASM memory flat (± 2 %)` (there are no level switches)
- retire → replaced by E21: `frozen gate: ?level=2 opens the Frozen Gate directly` (now `?level=2` opens `?biome=frozen`; E21 asserts that too)

**Retire: the `marsh` section (all 17; `?level=3` now opens the block-world Marsh):**
- retire: `marsh: level 2 forward arch is shut before the lantern (level 2, nextOpen false)` (arch)
- retire: `marsh: the level-2 win opens the forward arch and the banner points to the Marsh` (arch)
- retire: `marsh: walking into the open forward arch loads the Marsh within 5 s` (level swap)
- retire → ported as E12: `marsh: burning gas blocks the walk (stopped south of the dip flames, z > 10)`
- retire → ported as M5 and E12: `marsh: Water on a gas vent only knocks it down (7 lit, pool 0, Flicker(Water) line)`
- retire → ported as M5 and E12: `marsh: Frost knocks it down too and Fire changes nothing (7 lit, Frost and Fire lines)`
- retire → ported as M4 and E12: `marsh: CO2 fills the dip (0.5 at a vent, Filling; 1 at the floor, Pooled; 4 lit, still 4 after 5 s)`
- retire → ported as M5 (Fire on a smothered vent fizzles: no oxygen): `marsh: Fire on a smothered vent fizzles (no oxygen), 4 lit`
- retire → ported as M4, M4b and E12: `marsh: the hollow fills in three casts (0.34 / 0.68 / 1.00; 4 lit after two, 0 after three) and both cross`
- retire: `marsh: walking to the lantern wins ("You cleared the Marsh!") and the walk stops` (level goal; the quest replaces it, E16)
- retire: `marsh: the back arch returns to the Frozen Gate with its forward arch open` (arch)
- retire → replaced by the revisit rule (section 7) and E16: `marsh: back from the Marsh, level 2 stays solved (stream frozen, gate melted) and the lantern gives no unsolved win`
- retire → ported as E22: `marsh: each band sees its own Filling line; Sprout shows the gas picture`
- retire → replaced by E7: `marsh: level 3 has no more entities than level 1`
- retire → replaced by E7: `marsh: 10 switches 1-2-3-2-1 leave entities exact and WASM memory flat (± 2 %)`
- retire → replaced by E21: `marsh: ?level=3 (no debug) opens the Marsh and CO2 can be selected and cast`
- retire: `marsh: from a direct ?level=3 load, the back arch reaches level 2 with its forward arch open and the gate melted` (arch)

**Retire: one worked-casts check that loads level 2:**
- retire → ported as E23 (below): `worked: handoff: level 2 at Sprout (castmode worked): Fire routes instant, three instant Fire casts melt the gate, and the level is won`

**Port (chemistry rules on the courtyard; kept on `classic=1`, plus a block-world version now):**
- port → M7 and E24: `fire spreads to neighbours`, `water puts out fire 1`
- port → M4 (O2 below 15 %): `foam smothers the hedge`, `foam blanket stops re-ignition (no O2)`
- port → M2 and E11: `fire melts the ice wall into a puddle`
- port → M1 and E10: `frost refreezes the meltwater`, `frost freezes the puddle`
- keep (no block-world equivalent; classic-only content): `frost puts out fire 2`, `frost on the steam vent`, `lab bench: water-splitting placeholder`, `fire ignites the dry hedge`, `a fire the player lit does not count toward the objective`

**Keep (on `classic=1`, unchanged; the classic courtyard is the fixture they need until I-053 ports them):**
- level-1 layout and objective, which later keep checks build on: `boots`, `three fires at start`, `unchecked fire reaches the greenhouse (lose)`, `reset`, `hover finds the target under the cursor`, `click-to-target casts without lock`, `Save the Greenhouse win`, `win banner shown` (the analytics `quest_complete` path and `cast: nothing shows under the win banner` depend on this win)
- controls and feel: `a click over a target locks the pointer and does not cast`, `mouse look turns`, `WASD moves`, `jump`, `keys 1-4 select spells`, `wheel switches spell` (voxel mode's wheel cycles 1-9; classic keeps 4), `left click casts while locked`, `aim turns while the fire button is held`, `Esc releases the mouse and pauses`, `drag-to-look turns the view without pointer lock`, every `feel:` check, every `clock:` check, every `streams:` check, every `cast:` check, every `review:` check
- every `progress:`, `bench:` and `worked:` check (except the handoff above). The three pad checks (`bench: standing on the pad lights the button and never opens the bench`, `bench: no collider pushes the player off the pad`, `bench: leaving the pad clears atBench and near`) keep `STATION` (-3, 14.5), because `bench::site()` returns it in classic mode
- `analytics-e2e.mjs` (on `classic=1`; E19 adds the valley's `quest_complete`), and `perf.mjs` default and `?low` modes (on `classic=1`). Retire `perf.mjs`'s level-2 and level-3 workload branches (`perf.mjs:570-583`); the voxel workload is E6 for now, and a `perf.mjs --voxel` mode for the Mac run is a follow-up.

**New block-world checks that replace ported ones** (added to the lists above, owned by Lane B unless stated):
- E22 `matter: each band sees its own Freezing and Filling line; Sprout shows the freeze and gas pictures; with no band the Apprentice line shows`
- E23 `story: at Sprout in worked mode Fire routes instant and three Fire casts melt the ice gate` (Lane C, with `?biome=frozen&band=sprout`)
- E24 `matter: the workshop fire spreads through wood blocks and Water puts each burning block out`

The PR description lists every **retire** line above, word for word, and says which new check covers it.

## 12. Build order

| Day | Lane A (engine) | Lane B (matter) | Lane C (game layer) |
|---|---|---|---|
| **Sun night** | Contract step (section 10) merged; voxel `Cell`, `World`, `generate` with flat biomes; V1, V2 green | Material table values with sources; heat + phase on a test world; M3 skeleton | `valley` pack, recipes, quest table; C1-C3 green; intro and ending text for review |
| **Mon** | Mesher with AO (V5-V6), chunk entities, time-sliced queue, atlas, raycast, collision, break and place, `?classic`/`?biome`; E1-E5 green. **End of Monday: you can walk, dig and build in the generated world in the browser** | Water flow (M3), fire (M7), gas field (M4, M6), vents (M5), `on_impact` with flight delay; M1-M7 green in unit tests | Pack UI, Make tab, objective line and arrow, intro camera path, music skeleton; E15, E17 green against stubs |
| **Tue** | Stamps for all biomes and protected boxes (V3, V4), sky cover and torch light, E6-E8 green, Mode branches complete | `blockchem` in the client: captions, puffs, beads, the rig, the balloon crate, `wade_push`; M8-M11; E9-E14 green | Quest wired to events, Invention Bench, ending, `?biome` pack seeding, SFX; E16, E18-E21 green. **End of Tuesday: the whole run is playable from intro to ending** |
| **Wed** | Visual polish: atlas detail, sky tints, fog per biome, crystal glow; perf pass; Should: worker meshing | Science review of every new caption and material row (B6), tuning so each puzzle takes 1-2 min | Music per biome, SFX mix, objective wording pass, a 15-minute timed playthrough by the panel (6, 9, 13, teacher, QA). **Feature freeze at night** |
| Thu | Bug fixes only; the deploy is Paawan's | | |

**Cut lines (decided now, so nobody argues on the day):**
- **Mon night:** you cannot walk, dig and build in the browser → Lane B and Lane C keep going, and Lane A gets the next builder on meshing; the Furnace is cut.
- **Tue noon:** E9-E12 are not green → the gas field is replaced by a per-hollow counter (level 3's rule: 3 CO2 casts fill the stamp hollow; the gap check stays as a block test), and M6 moves to a follow-up.
- **Wed noon:** the run is not playable from the intro to the ending → the cave becomes one room with the rig and the ledge (no ladder; the shortcut door opens from the cave), and every Should is dropped.
- **Wed panel rule:** the median first run for the 9- and 13-year-old personas must be 10-20 min. If it is over 20, shorten the walking route (move stamps closer), not the puzzles.

## 13. Risks

1. **Scope against 3.5 days.** A block engine, a matter sim and a game layer from scratch in parallel. Mitigation: Monday night is a hard checkpoint (walk, dig and build in the browser). If it is missed, cut the gas field to "CO2 fills a stamp hollow by counting casts" (level 3's rule) and drop iron and the Should list.
2. **Disk space for builds.** The build machine has about 2.7 GB free, and a Bevy wasm target directory is several GB. Three lanes cannot each keep a target directory. Mitigation: one shared `CARGO_TARGET_DIR` for the builders, pure crates tested with `cargo test -p` (small target), and the wasm build is run by one lane at a time.
3. **Software GPU in the cloud.** Golden runs on SwiftShader. A 400 k-triangle scene may be slow there even when a Chromebook is fine, which makes E6 flaky. Mitigation: E6 measures CPU frame time (as B2 does), not fps; `?low` uses a 6-chunk view and no chunk shadows.
4. **Building and digging around puzzles.** Mitigation: protected boxes (V4, E5). Risk left: a puzzle box drawn too small. Each golden puzzle check also tries the dig-around once.
5. **Conflicts with I-051 and I-030.** I-051 rebuilds `aim_and_cast` and `projectiles` in `main.rs`; I-030 edits `bench.js` and `progress.rs`. Mitigation: voxel mode touches those systems only through one call each (`voxel::raycast` for the pick, `blockchem::on_impact` for the landing) behind `Mode::Voxel`, and Lane C's bench.js change is a new tab in its own functions.
6. **Main-thread meshing hitches.** Without wasm threads, a 32³ chunk could take more than 4 ms at 4× throttle. Mitigation: the time budget can split a chunk's meshing across frames (by face direction), and the Worker path is Should.
7. **Kids get lost in an open world.** Mitigation: the objective arrow, the Kiln chimney visible from everywhere, biome toasts, and the idle hint (`hud.js` already shows one after 45 s).
8. **Determinism slips** (f32 order, HashMap iteration). Mitigation: M10, and a rule in the crate docs; any server work later starts from this test.
9. **Science errors in new content** (B6). Mitigation: every material row and caption is sourced, and the Wednesday review is in the plan.

## 14. Revision log
- **r1 (after critique.md):** game name Formula; modern laptops are the target and Chromebooks are nice to have; sound is synthesised in code; the Furnace (glass, iron) moved from Should to Must-light; P3 gets a rim gap you must close before the CO2 can pool (M4b, E12); CO2 is drawn as a mist sheet; a toon-block atlas in the existing palette, with a distinct runestone tile; the cave is dim, not black; Sprout gets picture help for the Pick, the Invention Bench and the push-back, and an always-on objective arrow; the age chooser gets "Just play"; the revisit rules are restated plainly, with reasons to go back; the iron rule and C4 are fixed; the contract step wires the event plumbing; cut lines and a timing rule are added; `?level=2/3` now open the biomes, and the broken-check list is marked retire / port / keep per Paawan's approval.
