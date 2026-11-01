# I-037: Level 3, the Marsh (methane flames, CO2 pools low and smothers them)

**based_on:** origin/main dff8336bb47d2528d7c53723f4ddef2e89e96eb7 (merge of PR #49, I-035; I-028 already on main). **Parallel:** I-029 builds on loop/I-029 and owns `main.rs::aim_and_cast`, `bench.rs`, `bench.js`, `progress.*`. This item touches none of them. Level 3 goes in new files plus the smallest hooks.

**Why.** This is backlog row I-037: "Marsh: burning methane, CO2 denser than air pools and smothers". It was level 4 in the old plan and is now the third level, after I-035. It moves **B8** (4 levels: this makes 3) and **B11**: CO2 is a crafted Bench spell (Sprout tier). Filling a hollow with gas is a non-shooting use of chemistry, because you fill a space instead of hitting a fire. Water and Frost fail on this puzzle. Paawan's wave rule applies: the level must be playable, take 2-5 min, be winnable by a 5-year-old (Sprout, aim assist on) and still interest a 13-year-old.

## Facts relied on (dff8336)

- `chem.rs`:
  - `Kind` covers Bush, DryHedge, IceWall, Puddle, SteamVent, LabBench, Greenhouse, Stream, Lantern. `Spell::Foam` is the CO2 spell.
  - CO2 on a flammable object sets `smothered = SMOTHER_S` and puts it out. On Stream or IceWall it gives `NoChange`. On anything else it gives `Smothered`.
  - `Goal::Reach` turns off reseed and the fire win, and `World::reach()` latches `Won`.
  - `Obj` has no CO2 field and no link from one object to another; neighbours are found by footprint gap.
- `main.rs`:
  - `Level { n, cleared1, pending }`. `exit_open` is true in level 2 because the back arch is always open.
  - `levels()` handles the triggers and `load_level`. `?level=2` is parsed at `main.rs:425`. The `level` command takes only 1 or 2 (931-934).
  - `reset(.., level)` picks the start pose by level.
- `level2.rs`:
  - `layout()` is pure: colliders, `chem::World` and picking boxes.
  - `caption(kind, obj, band) -> Option<(text, icon)>`. Constants are `START`, `BACK_ARCH (0, 22.6)` and `LANTERN (0, -17)`.
  - Level 2 has no forward exit.
- `world.rs`: `HALF = 24`, `ARCH_REACH = 1.4`, `LevelRoot` tag, arch door mats.
- `hud.js`:
  - The title, objective and banner are chosen by `state.level === 2` (720-722, 748).
  - `#capicon` accepts `freeze|melt|wrong` (728).
  - The pointer-lock block is not touched.
- Golden:
  - Key order: `I035_KEYS` is last (golden 1072-1074).
  - The tail is checked again at 1347 and 2689 (`slice(-7)`).
  - `ONLY=frozenGate` switch at 94.
- `perf.mjs --query level=2` exists (`isLevel2`, 113).
- Science register: the last ids are **C-072, P-020, [S25]**.

## What a kid does (about 3 minutes)

0. **Level 2 gets a forward exit.**
   - A closed arch stands in the north wall at (0, −22.6). It is solid and dark until the lantern is reached.
   - On the win it lights ice-blue (static), and the banner's small line becomes "Walk to the glowing arch to visit the Marsh. Press R to play again."
   - Feet within `ARCH_REACH` load level 3.
   - `cleared2` keeps the arch open for the rest of the page load. `?level=3` loads level 3 directly, without debug.
1. **0:00 Arrive.**
   - A misty marsh, same size as the courtyard. A plank boardwalk 3 m wide runs from the start (0, 0, 18) north to a lantern on a hummock at (0, −17).
   - Either side is deep reedy water: solid colliders (reed beds) the player cannot enter, so the boardwalk is the only route.
   - The back arch at (0, 22.6) returns to level 2. In level 2 it arrives at the north arch with `cleared2` set.
   - Caption: "The Marsh. Put out the gas flames, cross the boardwalk, reach the lantern."
2. **0:15 The dip.**
   - At z 8..12 the boardwalk sinks into a shallow **dip**. Three marsh-gas vents (x −1, 0, 1, z 10) send bubbles up through the water and burn as blue flames across the planks.
   - While a flame burns it is solid (too hot to pass). Bubbles rise visibly: methane (16 g/mol) is lighter than air.
3. **Wrong chemistry fails kindly.**
   - Water on a vent: the flame shrinks to 40 % for 1.5 s and grows back. It never goes out.
   - Frost does the same.
   - Fire on a burning vent changes nothing.
   - Each try gives a band caption saying why (table) and pointing at CO2.
4. **0:45 Fill the dip.**
   - CO2 aimed at any dip vent or the dip floor adds **0.5** to the dip's pool. A pale ground-hugging haze rises in the dip to that level.
   - At **1.0**, every flame in the dip goes out at once and stays out. The flames lose oxygen; there is no neighbour or decay rule.
   - The planks are clear.
5. **1:30 The sunken hollow.**
   - At z −5..−11 a deeper **hollow** holds 4 vents (x ±1.5, z −7 and −9) across the boardwalk. It needs **3** CO2 casts (0.34, 0.68, 1.0).
   - Nothing goes out before 1.0: a half-full pool is below the flames.
   - Same wrong-spell rules as the dip.
6. **2:30 The lantern.**
   - Within 1.5 m of (0, −17) the player wins: banner "You cleared the Marsh!" with "CO₂ is heavier than air: it filled the hollows and smothered the flames. Walk to the arch to go back."
   - Movement stops on the win: held keys no longer move the player until R or a level change. This fixes level 2's break follow-up. `R` restarts level 3.
7. **No soft-lock.**
   - Fire on a smothered vent fizzles (no oxygen), so a filled hollow can never relight.
   - Pools never drain: real pooled CO2 disperses slowly in moving air, and that is out of scope.
   - The player is never inside a flame's collider when it turns on. Flames only start on level load or R, and both put the player at START.

**Aim rule (I-035 break follow-up).** The hollow and dip are objects with flat, non-solid picking boxes over their floors. A CO2 cast that hits a vent or the floor fills the pool that contains it, so whatever the aim assist picks, the cast does what the reticle shows. The golden check casts at the floor as well as at a vent. **Colliders:** every solid prop has a collider: reed beds, hummock, plinth, lantern post, the arches and the boardwalk rails.

**Stretch S1** (cut first if the lane runs long): one vent on a raised mound off the path at (8, 0). CO2 there gives `Spilled`: "No hollow here: the CO₂ spills away downhill." The vent burns forever and harmlessly, and is not required to win.

## Captions (exact; Sprout lines also set `capIcon`)

The band comes from `BenchState.band`. `None` shows the Apprentice line.

| event (level 3) | Sprout 4-7 (icon) | Seedling 8-10 | Apprentice 11-15 (and None) | Adept 16+ |
|---|---|---|---|---|
| Flicker(Water) | `The fire is on a gas. Water can't stop it. Try CO₂!` (wrong) | `The flame burns gas that bubbles up through the water. Water cannot soak a gas, so the flame grows back.` | `Marsh gas is methane (CH₄). It keeps bubbling up through the water, so a splash only knocks the flame down for a moment.` | `Gas fires are fed from below: water cools the flame but does not cut off the fuel or the oxygen, so the CH₄ keeps burning.` |
| Flicker(Frost) | `Brrr! The fire came back. Try CO₂!` (wrong) | `The cold shrank the flame, but the gas keeps coming up and burning.` | `Liquid nitrogen boils into N₂ gas, which is about as heavy as air, so it drifts away and the methane keeps burning.` | `N₂ (28 g/mol) is nearly as dense as air (29 g/mol): it mixes away instead of pooling, and the CH₄ flame recovers.` |
| Fire on a burning vent | `It is already burning!` (wrong) | `That gas is already burning. Fire cannot put out fire.` (Seedling to Adept) | ← | ← |
| Filling (pool < 1) | `The gas sinks down low. More!` (gas) | `CO₂ is heavier than air, so it sinks and fills the hollow from the bottom.` | `CO₂ (44 g/mol) is denser than air (29 g/mol), so it pools low, like an invisible pond.` | `At the same temperature and pressure, gas density goes with molar mass: CO₂ is about 1.5 times as dense as air, so it settles in low places.` |
| Pooled (pool = 1, flames out) | `All the fires went out!` (gas) | `The CO₂ covered the flames. No oxygen, no fire.` | `The CO₂ pool pushed the air out of the hollow. Without O₂, CH₄ + 2O₂ → CO₂ + 2H₂O cannot happen.` | `Smothered: pooled CO₂ cut off the oxygen to every flame at once. Pooled CO₂ is dangerous for people too, which is why the boardwalk is high.` |
| Fizzle(NoOxygen) on a smothered vent | `No air down there, no fire.` (wrong) | `Fire needs oxygen, and the CO₂ keeps it away.` (Seedling to Adept) | ← | ← |
| Spilled (S1 only) | `It rolled away! Find a low spot.` (wrong) | `No hollow here: the CO₂ spills away downhill.` (Seedling to Adept) | ← | ← |

"Invisible pond": CO2 is colourless. The haze is a game cue and goes in `## Not claims` as a visual convention. The real white fog appears only at discharge ([S23]). No line states a flame colour or the marsh temperature.

## Rules (chem.rs, client-world)

- `Kind::GasVent`:
  - Never runs out of fuel, never chars, starts burning, `flammable()` stays false.
  - Not counted toward `Goal::Fires`. Level 3 uses `Goal::Reach`.
  - Solid for movement while burning (`world::solid_obj`).
- `Kind::Hollow`: inert floor object; picking box only, never solid.
- `Obj.co2: f32` (0..1) on Hollow objects. A vent belongs to the Hollow whose footprint contains its `pos` (pure helper `hollow_of(i)`, computed in `World::with_goal`).
- **Water or Frost on a burning vent:**
  - `Flicker(spell)`, with the flame scale set to 0.4 and back over `FLICKER_S = 1.5` s.
  - `burning` stays true, and `extinguished` does not change.
- **Fire:**
  - Fire on a burning vent gives `NoChange`.
  - Fire on an unlit vent with pool < 1 relights it (`Ignited`). That is reachable only via S1 or a test.
  - Fire on a smothered vent gives `Fizzle(NoOxygen)`.
- **CO2 on a vent or Hollow:**
  - The pool rises by `per_cast` (dip `0.5`, hollow `0.34`, cap 1, snap to 1 within 1e-4), with `Filling`.
  - At 1, every vent of that Hollow gets `burning = false, smothered = f32::MAX`, with `Pooled` (one event, on the Hollow).
  - At 1 already: `Pooled` again, no change.
  - A vent in no Hollow gets `Spilled`.
- **Events and counts:**
  - New events are `Flicker(Spell)`, `Filling`, `Pooled` and `Spilled`. The contract step reuses an existing event only if it already means the same thing, and records which.
  - Level 1/2 behaviour does not change: the existing tests pass unedited.
  - `chem.rs` tests compile alone (`rustc --edition 2021 --test src/chem.rs`): no `crate::` paths.

## Acceptance tests (each fails on main)

**Rust** (`cargo test`, and `rustc --test src/chem.rs` for chem):

1. **`chem` vent rules.**
   - A GasVent burns through 20 000 steps with no BurntOut and no charring.
   - Water and Frost on it give `Flicker(_)` and leave `burning` true and `extinguished` unchanged.
   - Fire on it gives `NoChange`.
2. **`chem` pooling.**
   - In a world with a dip (per_cast 0.5) holding 3 vents, a hollow (0.34) holding 4 vents and 1 vent outside both:
     - CO2 casts give pools of 0.5 → 1.0 and 0.34 → 0.68 → 1.0 (`Filling`, `Filling`, `Pooled`).
     - No vent goes out before its pool reaches 1, and all of that hollow's vents go out at 1.
   - Vents in the other hollow and the outside vent keep burning.
   - Smothered vents stay out through 20 000 steps.
   - Fire on them gives `Fizzle(NoOxygen)`.
   - CO2 on the outside vent gives `Spilled` and changes nothing.
   - The same pool rise happens whether the cast hits a vent or the Hollow.
3. **`chem` win rule.** A `Goal::Reach` world with burning vents never wins, loses or reseeds by itself over 20 000 steps.
4. **`level3` captions.**
   - `caption` returns the exact table strings and icons for every (event, band) pair.
   - `None` gives the Apprentice line.
   - Every string passes the player-text checks (no `"`, `\` or control characters).
5. **`level3` walk.**
   - From START, a collider sweep north to the lantern is blocked by the dip flames.
   - After the dip is pooled it is blocked by the hollow flames.
   - After both are pooled it is clear.
   - Sweeps off the boardwalk on either side hit a reed-bed collider at every z between the start and the hummock.

**Golden** (new section `marsh`, run after `frozenGate`, added to the `ONLY` switch as `ONLY=marsh`; drives the state bridge only):

6. Level 2's forward arch is shut before the lantern: `level 2`, then `tp 0 -21.8` leaves `level === 2` and `nextOpen === false`.
7. Reach the Marsh by playing:
   - Solve level 2 (3 Frost, 3 Fire, walk to the lantern). `nextOpen === true`, and the banner's small line mentions the Marsh.
   - Then `tp 0 -21.8` gives, within 5 s:
     - `level === 3`
     - `objs` with names `dip`, `hollow`, `lantern`, 3 + 4 vents
     - `ventsLit === 7`, `outcome === 'playing'`
     - a caption starting "The Marsh".
8. `?level=3` without `debug` loads level 3.
9. Flames block: `move 0 1 3` from START leaves z > 12.2.
10. Water on a dip vent: in the next frame `ventsLit === 7`, `pools[0] === 0`, and the caption is the Apprentice Flicker(Water) line.
11. Frost on a dip vent gives the same result with the Frost line. Fire on a dip vent leaves `ventsLit === 7`.
12. Dip fills:
    - One CO2 cast at a vent: `pools[0] === 0.5`, `ventsLit === 7`, and the caption is Filling.
    - One CO2 cast at the dip **floor**: `pools[0] === 1`, `ventsLit === 4`, and the caption is Pooled.
    - After 5 s, `ventsLit` is still 4.
13. Fire on a smothered dip vent: `ventsLit === 4`, and the caption is the NoOxygen line.
14. Hollow needs three casts:
    - `pools[1]` goes 0.34 / 0.68 / 1.00.
    - `ventsLit` is 4 after the second cast and 0 after the third.
    - A scripted `move` then crosses both, to z < −12.
15. Win: walking to the lantern gives `outcome === 'won'` and the banner "You cleared the Marsh!". A further `move 0 1 2` leaves `pos` unchanged (the walk stops).
16. Captions per band: for each band (`band <id>` on `__gameIn.bench`), a fresh level 3 CO2 cast at the dip shows that band's Filling line. Sprout gives `capIcon === 'gas'`; no band gives the Apprentice line.
17. Way back: the back arch in level 3 gives `level === 2` and `nextOpen === true`.
18. Key order:
    - `I037_KEYS = ['nextOpen', 'pools', 'ventsLit']` is appended after `I035_KEYS`.
    - The `objs` keys are unchanged.
    - No console errors, no panics, 3/3 runs.

**Pinned golden edits (allowed, must be listed in the PR):** the key-order check at golden 1072-1074 (add `I037_KEYS`), and the tail checks at 1347 and 2689 (they take `slice(-10)` and include the three new keys). These are the same edits I-035 made for its keys. No other existing check changes.

**Perf and lint:**

19. `perf.mjs --query level=3` (extend `isLevel2`-style detection to level 3) at `?low`:
    - Entity count and draw calls are ≤ level 1's on the same build. The break-1 baseline was 633 entities. Flames reuse level 1's fire fx, and bubbles are capped at 12 under `?low`.
    - The median CPU frame at 4x throttle is no worse than level 1 beyond the perf-pairs noise band (5 pairs).
    - Brotli size grows ≤ 0.2 MB.
20. Switching 1→2→3→2→1 ten times leaves the entity count exact against the first load. WASM memory is flat (± 2 %) after the switch-2 warm-up.
21. The science lint passes: every new string has a `C-` row or a `## Not claims` row, and every new constant has a `P-` row.

## Science rows (content/science-sources.md; next ids C-073.., P-021.., [S26]..)

- **Density.**
  - CO2 44.01 [S5], air 28.96 [S8], N2 28.01 [S1].
  - Methane 16.04: use [S4]'s WebBook page if it shows the molecular weight, else a new NIST methane record.
  - 44.01/28.96 = 1.52, which is "about 1.5 times" (as C-035). 28.01 vs 28.96 is "nearly as dense". Gas density goes with molar mass at the same T and p (ideal gas, [S12]).
- **Smothering.** CO2 extinguishes by lowering oxygen [S19] [S14]. Methane combustion [S7] [S4].
- **CO2 pools in low places and is a hazard to people.** [S14] if it says so, else **new [S26]**: USGS Fact Sheet 172-96, Mammoth Mountain CO2 (CO2 accumulating in low areas and killing trees). The content builder fetches and quotes it.
- **Marsh gas is methane made in wetlands.** **New [S27]**: a USGS, NOAA or EPA page that says microbes in waterlogged soils without oxygen make methane.
- **Water is a poor choice on gas fires.** **New [S28]**: OSHA's extinguisher-classes page (Class B covers flammable gases; it says which agents fit and that water does not), or NFPA. Verdict: **simplified** for the Flicker lines. Real water spray can put out a small gas flame. The point that matters is true: water does not remove the gaseous fuel. If no fetched source supports a line, the builder rewords it to what the source says before merging.
- **Simplified verdicts:**
  - The pool rising in fixed steps and never draining.
  - All flames going out at exactly full.
  - A 40 % flicker that always recovers.
- **Parameters:**
  - P-021 `DIP_CO2_PER_CAST` 0.5 [S0]
  - P-022 `HOLLOW_CO2_PER_CAST` 0.34 [S0]
  - P-023 `FLICKER_S` 1.5 s [S0]
  - All are gameplay tuning.
- **Not claims:**
  - The objective line, the arch lines and "Press R".
  - The haze as a visual cue (CO2 is colourless).
  - The blue flame colour: visual only, and no caption states it.
- **Safety.** No line suggests breathing CO2 or standing in a pool. The Adept Pooled line states the hazard. The fresh science reviewer runs `docs/loop/science-review.md` before merge.

## Contract

- **`chem.rs`** (world lane): adds the items under Rules. Existing tests are not edited.
- **`level3.rs`** (new, world lane):
  - `pub fn layout() -> Layout` (pure) and `pub fn build(commands, kit, materials, images) -> (Colliders, Chem)`.
  - Targets are named `dip`, `hollow`, `dip vent` ×3, `hollow vent` ×4 and `lantern`.
  - Constants: `pub const START`, `BACK_ARCH (0, 22.6)`, `LANTERN (0, -17)`, `LANTERN_REACH 1.5`.
  - `pub fn caption(kind, obj, band) -> Option<(&str, &str)>` and `pub fn sync(..)` for the haze heights and flicker scales.
  - Every entity carries `LevelRoot`.
- **`level2.rs`** (world lane): adds the north arch collider and mesh: `NEXT_ARCH (0, -22.6)`, `ArchDoor`. The arch is solid while closed. No caption changes.
- **`main.rs`** (core lane, smallest hooks):
  - `Level` gains `cleared2`, and `exit_open` stays as it is.
  - New `next_open()`: level 1 is the old exit rule; level 2 is won or `cleared2`; level 3 is false.
  - `levels()` handles the level-2 forward arch and the level-3 back arch and lantern.
  - The `level` command and `?level=` accept 3.
  - `reset` sets the level-3 START, and the level-2 arrival from level 3 is the north arch.
  - Captions are looked up in `level3::caption` where level 2's are, in `projectiles` and `react`.
  - The walk stops when `outcome == Won` in level 3.
  - `publish` appends `nextOpen`, `pools` (array of hollow fills, `[]` outside level 3) and `ventsLit`.
  - `aim_and_cast` is not touched: the aim fix lives in the picking boxes.
- **`fx.rs`:** bubble spawns reuse the bead pattern if needed.
- **`hud.js`** (web lane):
  - Title "The Marsh", the objective line and the win banner for `level === 3`.
  - The level-2 banner's small line when `nextOpen`.
  - A `gas` picture for `#capicon` (in `style.css`).
  - The finder arrow points at the forward arch when `nextOpen && outcome === 'won'` in level 2.
  - The pointer-lock block is not touched.

## Lanes (max 3; one owner per file)

| lane | paths |
|---|---|
| world | `crates/bevy-client/src/chem.rs`, `crates/bevy-client/src/level3.rs` (new), `crates/bevy-client/src/level2.rs` (north arch only), `crates/bevy-client/src/world.rs` (only if a shared arch helper is needed), `crates/bevy-client/src/fx.rs` |
| core | `crates/bevy-client/src/main.rs` (hooks above; `mod level3;`), `crates/bevy-client/src/bridge.rs` if needed |
| web | `crates/bevy-client/web/hud.js`, `crates/bevy-client/web/style.css`, `content/science-sources.md`; tester-owned: the golden `marsh` section and pinned key edits, `tests/e2e/perf.mjs` level-3 query |

`crates/sim` does not change. `bench.rs`, `bench.js` and `progress.*` do not change: the bench stays hidden outside level 1 through the existing `BENCH_ON = n == 1`.

## Out of scope

- Level select, saved progress and mastery gates (I-034).
- A fourth level and its forward arch.
- CO2 draining or wind.
- Lose conditions (fire spreading to reeds), creatures (heron), sound beyond the existing tones.
- Level-1 band captions (I-021).
- Any cast or progress change (I-029).
- Flame colour science.

## Follow-ups

- **F1:** pools drain slowly (diffusion) with a relight source. This needs a sourced rate.
- **F2:** a heron on the hummock as the goal.
- **F3:** fold level 3 into I-034's level select.
