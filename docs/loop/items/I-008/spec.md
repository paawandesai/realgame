# I-008: Quests 2 and 3, spells 5 and 6, free-play yard, skill-tree screen

**based_on:** main f6504c4, 2026-10-29. Code read from `claude/fps-prototype` at 6d5463e (I-001, not merged yet).
**Revision:** r2, applies `critique.md` (see "Critique: accepted and rejected" at the bottom).
**After:** I-001 must be merged.
**Stale check:** before dispatch, the planner re-reads I-001 as merged and refreshes this spec if any of these names differ: `chem::{Kind, Spell, EventKind, Fizzle, Obj, World, Outcome, DT}`, `World::apply/step/outcome`, `world::{Aabb, Chem, Target, Collider, Colliders, solid_obj, build}`, `player::{Player, RADIUS, HEIGHT, WALK, SPRINT, JUMP, GRAVITY}`, `Player::step(.., jump: bool, ..)`, `Game`, `control`, `aim_and_cast`, `projectiles`, `react`, `sync_visuals`, `publish`, the `__gameState` fields, the `__game.cmd` grammar, the `CPUFRAME` console line in `web/boot.js`, or the `1 << 30` captioned-event flag. Do not dispatch on this text if I-001 landed with different names.
**Overlap with I-007:** both items touch `main.rs`, `hud.js`, `index.html` and `style.css`. Whichever merges second rebases onto the other. If I-007 merges first:
- the core lane adds `Hydrogen => 0.55` and `Oxygen => 0.55` to `feel::recoil_strength`;
- `feel::hit_kind` needs no change, because every new `Fizzle(_)` already maps to `fizzle` and every other new event to `hit`;
- the web lane keeps I-007's DOM ids (`#sens`, `#fov`, `#hitmarker`, `#crosshair.cooling` and `#crosshair.pulse`).

## Problem
Today the slice holds one quest (Save the Greenhouse, about 4 to 6 minutes for a 9-year-old) and 4 spells. It has nothing to do after the win: `chem_tick` stops stepping once `outcome != Playing`, and casting is blocked. B8 needs at least 15 minutes of guided play with at least 3 quests, 6 spells, a free-play area and a skill-tree screen. As it stands B8 cannot pass, whatever the persona panel says. The lab bench says "coming soon", which promises content that does not exist. Fun (B5) also suffers: a first-person game with a single 5-minute objective gives a child no reason to keep playing, nothing to unlock and nowhere to experiment.

## Bars moved and expected movement
- **B8 (Enough to play):** this item moves B8 from "structurally impossible" to "measurable". Prediction for the first I-006 panel run on this artifact: the 9-year-old persona plays at least 15 minutes, split roughly as Q1 4 to 6 min, Q2 3 to 4 min, Q3 3 to 5 min, and the tree plus free play 3 or more min. All four structural parts exist (3 quests, 6 spells, free play, skill tree), so the B8 checklist is satisfied apart from the time measurement.
- **B5 (Fun):** prediction for a paired panel run (same seed, before vs after): median fun rises at least 1 point. No persona reports "nothing to do after the greenhouse". Spells 5 and 6 are not required by any quest in this version: they are rewards to play with in the yard and courtyard. The Q2 reward is therefore a caption, a science point and two hotbar slots. That is enough for B8 but will move B5 only a little; a later item can give them a quest use.
- **B6 (Science):** every new claim is added to `content/science-sources.md` with a citation (test 20), so the item adds 13 claims without adding an unsourced one. The "coming soon" lab caption is removed.
- **B4 guard:** the Q3 step line names the puzzle's clue (the hot spring melts nearby ice), so a 6-year-old is not left to discover it only through a failure hint.
- **Guards:** B1 (no panics or console errors on the new paths, test 19), B2 and B3 (tests 21 and 22).
- **B1 goes red after this merge, and who fixes it.** B1 requires the smoke playthrough to "complete every quest in the slice". After this item the slice has three quests, and I-002's golden path (out of scope here) only knows Q1. So B1 cannot pass until the smoke walks Q2 and Q3. The fix: if I-002 is still unbuilt when I-008 merges, I-002's spec takes Q2 and Q3, reusing the sequences of e2e tests 14 and 15. Otherwise the planner adds a backlog item "Smoke: walk quests 2 and 3 in `tests/e2e/golden.mjs`" directly after I-008, scheduled next. (This spec does not edit the backlog; the planner does.)

## Smallest version

### 1. Map: a south zone through two gates
- The courtyard stays as it is (x and z from −24 to 24).
- The south wall (z = 24.5) gets two 4 m gates: a west gate at x −10..−6 and an east gate at x 6..10. The gates are open to the sky (no lintel).
- A new walled south zone covers x −24..24, z 25..53. A divider wall at x = 0 (x −0.5..0.5, z 24.5..53, no gaps) splits it into the **Frozen Pond** (west) and the **Free-play yard** (east).
- **Wall height:** every new wall (the south zone's west, east and south walls, the divider, and the courtyard south wall's pieces either side of the gates) is at least 3.0 m tall. The highest a player's head can reach is `JUMP²/(2·GRAVITY) + HEIGHT` = 1.0 + 1.75 = 2.75 m, so no jump clears a wall. The pond can only be reached through the west gate, and the south shore only by crossing the pond.
- The new walls come from a pure function `world::south_walls() -> Vec<Aabb>` (no Bevy app; `Aabb` is the existing pure box type), which `build()` turns into plain `Collider { obj: None }` boxes and meshes.
- The player clamp in `player.rs` changes from a ±(24 − r) square to x ∈ [−24 + r, 24 − r] and z ∈ [−24 + r, 53 − r]. The walls still do the real containment.

### 2. Zones by position
One function decides where the player is, used by the `zone` state field, the Q2 and Q3 step logic, and nothing else:

```
pub fn zone_at(p: [f32; 2]) -> Zone   // in quest.rs; p = [x, z]
  if p[1] > 24.5 && p[0] < 0.0                    -> Pond
  if p[1] > 24.5 && p[0] >= 0.0                   -> Yard
  if p[0] >= 17.5 && p[1] >= 3.0 && p[1] <= 9.0   -> Lab
  otherwise                                       -> Courtyard
```

A player standing in a gate opening (z between 24.5 − r and 24.5) is still `Courtyard`; that is intended.

### 3. Quest 2: "Split the Water" (east alcove lab; real electrolysis)
These objects are in zone `Lab`:
- the existing ice wall;
- the lab bench, renamed "water splitter" (`Kind::LabBench` keeps its kind);
- a new "battery switch" (`Kind::Switch`) on the alcove's north wall at (20.0, 3.3);
- two new gas tubes on the bench: "hydrogen tube" (`Kind::GasTube`) at (21.8, 5.1) and "oxygen tube" (`Kind::GasTube`) at (21.8, 6.9).

The steps, in order:
1. **Reach the lab.** Melt the ice wall with Fire, as today. This step also passes if `zone_at(player) == Lab` or the cell is already filled.
2. **Fill the cell.** Water on the water splitter sets `level = 1` and emits `Filled`. Water on an already-full splitter emits `Filled` again and changes nothing. The caption names the electrolyte: sodium sulfate (Na2SO4). It must not be table salt, because NaCl solution gives chlorine at the anode, not oxygen.
3. **Switch on.** Any spell on the battery switch sets `power = true` on the water splitter and emits `Powered`. The switch only turns on; there is no toggle, so a child spamming casts cannot turn it off by accident.
4. **Wait for the gases.** While `power && level > 0`, each tick does `h2 = (h2 + H2_RATE_ML·DT).min(H2_CAP_ML)` on the hydrogen tube and `o2 = (o2 + H2_RATE_ML·DT/2).min(O2_CAP_ML)` on the oxygen tube. The `min` clamp makes the full values exact. From empty, O2 is exactly half of H2 at every tick: equal gas volumes hold equal moles (Avogadro's law), and 2H2O → 2H2 + O2. The first tick on which H2 equals `H2_CAP_ML`, the world emits `TubesFull` once (once per world lifetime until a `Lab` reset). The tube gauges in the world show the H2 column twice as tall as the O2 column.
5. **Test the gases with Fire.**
   - Fire on the hydrogen tube with at least 5 mL emits `Popped` (a squeaky pop: 2H2 + O2 → 2H2O) and empties the tube.
   - Fire on the oxygen tube with at least 2.5 mL emits `Flared` (oxygen makes the flame burn brighter, though oxygen itself does not burn) and empties the tube.
   - Below those amounts, Fire gives `Fizzle(Empty)`.
   - Any other spell on a tube or on the splitter (apart from Water on the splitter) gives `Fizzle(NotHere)`.
   - Electrolysis keeps running, so the tubes refill.

The quest is done when both tests have happened: a `Popped` event whose `obj` is the hydrogen tube's index, and a `Flared` event whose `obj` is the oxygen tube's index, in either order. `Popped` or `Flared` from spells 5 and 6 on other objects do not count. Completion unlocks spells 5 and 6 and gives 1 science point. The old `EventKind::Station` and its "coming soon" caption are deleted.

### 4. Spells 5 and 6: Hydrogen and Oxygen (locked until Q2 is done)
Hotbar order, fixed: 1 Fire, 2 Water, 3 Frost, 4 Foam, 5 Hydrogen, 6 Oxygen. `select N` and `__gameState.spell` use it (`spell` is 0-based).

**Hydrogen (5)** is a slow, floaty bubble projectile (14 m/s). Cast caption: "Hydrogen, H2: the lightest gas. With a flame it burns: 2H2 + O2 → 2H2O."
- On a burning object: emits `Popped`. No state change. The `Popped` caption says a little hydrogen burns in one quick pop, so a child does not expect it to feed the fire.
- On anything else, including the 110 °C steam vent and the 80 °C hot spring: emits `Fizzle(NoFlame)`, whose caption says H2 is about 14 times lighter than air. No state change.

**Oxygen (6)**, 28 m/s. Cast caption: "Oxygen, O2: it does not burn, but it makes fuels burn faster and hotter."
- On a burning flammable object: `fuel -= min(fuel, OXY_BURN_S)` and emits `Flared`. If fuel reaches 0, it also sets `burning = false` and emits `BurntOut`, in that order.
- On anything else: emits `Fizzle(NoFuel)`. No state change.

Keys 5 and 6 and the mouse wheel select only unlocked spells. `select 5` or `select 6` while locked is ignored. The hotbar shows 6 slots, and slots 5 and 6 carry the `locked` class until they unlock.

### 5. Quest 3: "The Frozen Pond" (liquid-nitrogen puzzle)
The pond has three `Kind::Pond` segments in zone `Pond`. Each spans the full zone width (x −24..−0.5):

| name | z range |
|------|---------|
| "pond north" | 31..35 |
| "pond middle" | 35..43 |
| "pond south" | 43..47 |

**Liquid pond.** A pond with `ice ≤ 0` is a solid collider at least 3.0 m tall: the water is too deep to wade. A frozen pond (`ice > 0`) is walkable and slippery: it is passed to `player.step` as an ice sheet, the same as a frozen puddle.

**Freezing and melting.**
- Frost on a pond sets `ice = 1` and emits `Froze`.
- Pond ice melts by itself: each tick `ice = (ice − DT/POND_ICE_S).max(0.0)` with `POND_ICE_S = 40`. `Melted` is emitted on the tick `ice` first becomes 0.
- Fire melts it, as with other water bodies.
- When a pond melts it stays `Kind::Pond`; it never becomes a Puddle.

**Hot spring.**
- A "hot spring" (`Kind::HotSpring`, zone `Pond`) is a solid rock chimney at least 3.0 m tall with footprint centre (−21, 39) and half extents (1, 1). Its footprint overlaps pond middle only; the gap to the other two ponds is ≥ 2 m.
- It sits at `SPRING_C = 80` °C. Frost sets it to 0 °C and emits `Chilled`. It then rewarms with `temp += (80 − temp)·0.05·DT`.
- While its temperature is above 40 °C, every pond whose footprint overlaps it (gap = 0) loses ice at an extra `SPRING_MELT_RATE = 4` per second (on top of the self-melt, same clamp). Pond middle's ice therefore lasts at most 8 ticks.
- After a chill the spring passes 40 °C after about 13.9 s (tick 416), so the player has a 13.9 s window.

**The puzzle.** The player freezes north and south, chills the spring, freezes the middle, then crosses. The step line names the clue (Contract, Q3 s1). Freezing the middle while the spring is hot fails at once and also gives a hint caption.

**Jump check.** Pond middle is 8 m deep. A sprint jump covers at most 2·8.5·6/18 + 2·0.35 ≈ 6.4 m, so the middle cannot be jumped. North and south can be jumped, which is fine, because the spring is the puzzle.

**Melting under the player.** If the player's body overlaps a solid pond after a step (the ice melted under them, or a `tp`), the player moves to (x, 0, 29.0) with zero velocity. The caption reads: "Splash! The ice melted under you. Back to the shore."

**Completion.** The quest is done when the player stands at z ≥ 47.5 and x < −0.5 (the south shore, with a decorative "Frost Vault" arch and no sign text). That gives 1 point.

### 6. Free-play yard (east half of the south zone, zone `Yard`)
- **Objects:** "yard bush 1", "yard bush 2" and "yard bush 3" (a row 3 m apart, gap 1.2 m, so fire spreads), "yard hedge" (DryHedge), "yard puddle 1", "yard puddle 2", "ice block" (`IceWall`, half extents 0.45 × 1.0) and "reset lever" (`Kind::Switch`).
- **No objective and no fail.** Yard fires never count toward `extinguished`.
- **Reset lever:** any spell on it restores every `Yard` object to its initial state and emits `YardReset`.
- **Isolation:** no `Yard` object is within `NEIGHBOUR_GAP` of an object in another zone. Heat must not cross the walls.

### 7. Skill-tree screen
- **Opening:** press `K`, or click the HUD button `#skills-btn`, to open the full-screen `#skills` overlay. It sets `paused` (the chemistry, movement and casting stop) and releases pointer lock **without** showing the pause card. `K` or `Esc` closes it. After closing, the player is in point-and-click mode, and a click on the canvas re-locks.
- **Branches:** three columns, each a spell with an optional upgrade under it.
  - Heat: Fire, Hydrogen, Oxygen.
  - Water and ice: Water with the upgrade **Big Splash**, Frost with the upgrade **Deep Freeze**.
  - Smother: Foam with the upgrade **Thick Foam**.
- **Spell node states:** `learned` for the four start spells, plus Hydrogen and Oxygen once Q2 is done (labelled "From: Split the Water"), and `locked` otherwise.
- **Upgrade node states:** `bought`, `buyable` (points ≥ 1) or `locked`.
- **Points:** the header `#skill-points` shows "Science points: N", where N = quests done − upgrades bought. Clicking a buyable upgrade sends `buy splash|freeze|foam`, and Rust is authoritative.
- **Upgrade effects:** upgrades are pure gameplay and carry no science claims in their text.
  - **Big Splash:** Water on object i also applies Water's effect to every other flammable object j with `gap(i,j) ≤ SPLASH_GAP`, emitting events for j too.
  - **Deep Freeze:** Frost on a water body i also freezes every water body j with `gap(i,j) == 0`.
  - **Thick Foam:** the CO2 blanket from Foam lasts 40 s instead of 25 s.
- There is no Discoveries panel in this item (cut; see the bottom).

### 8. Quest flow
- **Order:** quests are done in any order. The active quest is the lowest incomplete one. Its title and step line drive the HUD objective box.
- **Q1 latch:** Q1 done latches the first time `outcome() == Won`. `Lost` is possible only while Q1 is not done.
- **The world keeps running:** the chemistry steps unless the game is paused, including after the Q1 win and after a loss. Casting is blocked only while paused.
- **Banner:** on a win, the Q1 banner hides itself after 4 s. The objective then switches to the next incomplete quest.
- **R / `reset`:** restarts the active quest only. It restores that quest's zone (Q1: `Courtyard` plus `extinguished = 0`; Q2: `Lab`, including the `TubesFull` latch; Q3: `Pond`), puts the player at the quest's start ((0, 18), (14, 6) or (−12, 28)), and keeps progress, points, unlocks and upgrades. With every quest done, `reset` restores the `Yard`.
- **`reset all`:** a fresh game. Progress, points, upgrades and unlocks are all cleared.

## Acceptance tests
How each kind of test runs:
- **Rust:** `cargo test --manifest-path crates/bevy-client/Cargo.toml --locked`.
- **Web unit tests:** `node --test crates/bevy-client/web/`.
- **e2e:** `node tests/e2e/content.mjs <artifact dir>` serves the built `web/pkg-out` and drives headless Chromium in point-and-click mode (not locked) through `window.__game.cmd` and `.state()`. Objects are always looked up by `objs[].n`, never by index. Each e2e scenario starts with `reset all`. "Cast X at Y" means `select <N of X>` (hotbar order above), `face <i of Y>`, then `cast`, then wait until `impacts` has risen by 1 (at most 3 s). "Cast any spell at Y" uses whatever spell is selected. Every `select` is written out in the steps below.

### Rust (lane: world; in `chem.rs`, `quest.rs`, `world.rs`)
1. **Electrolysis.** Build a world with a splitter, a switch and the two tubes in zone `Lab`.
   - Water on the splitter gives `[Filled]` and `level == 1.0`. A second Water gives `[Filled]` and `level == 1.0`.
   - Any spell on the switch gives `[Powered]` and `power == true`. A second cast gives `[Powered]` again, and `power` stays true.
   - From both tubes empty, at every tick for 300 ticks: `|o2_ml − h2_ml/2| < 1e-4`.
   - After 150 ticks, `h2_ml` is 10.0 ± 0.1.
   - After 600 ticks, `h2_ml == 20.0` and `o2_ml == 10.0` exactly (the `min` clamp), and `TubesFull` has been emitted exactly once.
   - With `power` on and `level == 0`, 300 ticks give `h2_ml == 0`.
2. **Gas tests.**
   - Fire on the hydrogen tube at 20 mL gives `[Popped]` and the tube is at 0. A second Fire gives `[Fizzle(Empty)]`.
   - Fire on the hydrogen tube at 4.9 mL gives `[Fizzle(Empty)]` and the gas is unchanged.
   - Fire on the oxygen tube at 2.5 mL gives `[Flared]` and the tube is at 0.
   - Water, Frost or Foam on a tube gives `[Fizzle(NotHere)]`. Fire on the splitter gives `[Fizzle(NotHere)]`.
   - The string "coming soon" appears nowhere in `chem.rs`.
3. **Hydrogen spell.**
   - On a burning bush: `[Popped]`, and the `Obj` is otherwise unchanged (compare before and after with `burning`, `temp` and `fuel` equal).
   - On a non-burning bush, the steam vent and a hot spring: `[Fizzle(NoFlame)]`, and the `Obj` equals itself before the cast.
4. **Oxygen spell.**
   - On a burning bush with fuel 60: `[Flared]`, fuel 50.0, still burning.
   - On a burning bush with fuel 4: `[Flared, BurntOut]`, fuel 0, not burning.
   - On a non-burning bush, and on a smothered non-burning bush: `[Fizzle(NoFuel)]`, and the `Obj` is unchanged (the smothered one is still smothered).
5. **Pond ice life.** Ice decrements by exactly `DT/POND_ICE_S` per tick, clamped at 0. Nominally it reaches 0 at tick 1200; f32 accumulation may move that by a tick, so the test uses a window.
   - A lone pond starts with `ice == 0`, and `world::solid_obj` returns true for it.
   - After Frost: `[Froze]`, `ice == 1`, and it is not solid.
   - At tick 1190, `ice > 0` and no `Melted` yet. By tick 1210, `ice == 0` with exactly one `Melted` event, emitted on the first tick `ice == 0`, and the kind is still `Pond`.
   - Three Fire casts on a frozen pond give a melted pond whose kind is still `Pond`.
6. **Hot spring.**
   - Ponds north, middle and south plus the spring, using the layout coordinates. The spring at 80 °C: Frost on pond middle leaves `ice == 0` by tick 8. Frost on north leaves `ice > 0.9` after 100 ticks, because the spring only affects ponds it overlaps.
   - Frost on the spring gives `[Chilled]` and `temp == 0`. Its temperature first exceeds 40.0 at a tick in 410..=420.
   - Chill the spring at tick 0 and freeze the middle at tick 1. The middle's ice is > 0 at tick 405 and == 0 by tick 430.
7. **Upgrades.**
   - Big Splash: three bushes in a row at gaps 0.8 m and 1.5 m, with A and B burning. Water on A with `splash` puts out both A and B, emitting two `Extinguished(Water)` events. C is not wet. Without `splash` only A is out.
   - Deep Freeze: Frost on pond north also freezes middle (they touch). South stays liquid, because only direct `gap == 0` neighbours of the target freeze. Frost on middle freezes all three. Without `freeze`, only the target freezes.
   - Thick Foam: a Foam cast gives `smothered == 40.0`. Without it, `smothered == 25.0`.
8. **Zone scoping.**
   - 3 burning `Courtyard` bushes, the greenhouse and 1 burning `Yard` bush (all far apart). Putting out the 3 courtyard bushes gives `outcome() == Won` while the yard bush still burns. Putting out the yard bush leaves `extinguished == 3`.
   - Igniting the greenhouse gives `Lost`.
9. **Layout.** `world::chem_objects() -> Vec<(&'static str, Obj)>` and `world::south_walls() -> Vec<Aabb>` are pure (no Bevy app) and are the sources `build()` uses. Checks (using the `player` constants, which the contract lane makes `pub`):
   - all object names are unique;
   - every name listed in the Contract exists with its kind and zone;
   - every `Yard` object has `gap ≥ NEIGHBOUR_GAP` to every non-`Yard` object;
   - the spring has gap 0 to pond middle and ≥ 2.0 to north and south;
   - the ponds' x extents cover [−24.0, −0.5] within 0.05;
   - pond middle's z extent is ≥ `2·SPRINT·JUMP/GRAVITY + 2·RADIUS + 1.0`;
   - north is 4.0 and south is 4.0 (±0.01);
   - every `south_walls()` box has `max.y ≥ 3.0` and `max.y ≥ JUMP²/(2·GRAVITY) + HEIGHT + 0.25`;
   - the divider boxes (those with `min.x ≥ −0.5` and `max.x ≤ 0.5`) together cover z [24.5, 53.0] with no gap wider than 0.01;
   - the courtyard south wall has exactly two openings, x −10..−6 and 6..10 (±0.05).
10. **Quest flow** (`quest.rs`). Each Q2 row below uses a world from `World::with_names(chem_objects())`, with only the listed fields changed from the fresh layout, then calls `update(w, p, events)` and checks `step(2, w, p)`. Rows run in sequence on one `Quests` value.

    | row | world fields set | player p | events fed | expected |
    |-----|------------------|----------|------------|----------|
    | a | (fresh) | [0, 18] | [] | step 0 |
    | b | ice wall `wall = false` | [0, 18] | [] | step 1 |
    | b2 | (fresh; separate fresh `Quests`) | [19.5, 6] | [] | step 1 (lab by position) |
    | c | + splitter `level = 1` | [19.5, 6] | [Filled on splitter] | step 2 |
    | d | + splitter `power = true`, tubes 0 / 0 | [19.5, 6] | [Powered on switch] | step 3 |
    | e | + h2 = 5.0, o2 = 2.5 | [19.5, 6] | [] | step 4 |
    | f | + h2 = 0.0 | [19.5, 6] | [Popped on a courtyard bush] | step 3, `h2_tested == false` |
    | g | (same) | [19.5, 6] | [Popped on hydrogen tube] | step 4, `h2_tested`, `done[1] == false` |
    | h | + o2 = 0.0 | [19.5, 6] | [Flared on oxygen tube] | `done[1]`, `unlocked()[4..6] == [T, T]`, `points() == 1`, `done_id` +1, notices contain `QuestDone(2)` |

    Also:
    - Fresh state: `active() == Some(1)`, `unlocked() == [T,T,T,T,F,F]`, `points() == 0`.
    - Reversed order (Flared on the oxygen tube, then Popped on the hydrogen tube, from row e) also latches `done[1]`.
    - `buy(Foam)` returns true and sets `world.upgrades.foam`. A second `buy(Foam)` returns false. `buy(Splash)` with 0 points returns false.
    - Q3: `step(3)` is 0 at [−12, 20] and 1 at [−12, 26]. Done latches when `player_xz = [-12.0, 47.6]`, and not at `[-12.0, 47.4]` or `[0.6, 48.0]`.
    - Q1: `step(1)` equals `extinguished.min(3)`. `Lost` while `done[0]` is false is reported. After `done[0]`, a burning greenhouse does not unset `done[0]`, and `quest1_lost()` is false.
    - `zone_at`: [0, 0] Courtyard, [19, 6] Lab, [19, 10] Courtyard, [−3, 24.4] Courtyard, [−3, 25] Pond, [0, 30] Yard, [−0.01, 30] Pond.
    - `reset_all()` restores the fresh state.

### Web unit (lane: web; `crates/bevy-client/web/skilltree.test.mjs` against `skilltree.js`)
11. `treeModel(state)`:
    - The fresh state (`unlocked` [T,T,T,T,F,F], points 0, no upgrades) gives 6 spell nodes, where `hydrogen` and `oxygen` are `locked` and the others are `learned`, and 3 upgrade nodes, all `locked`.
    - With points 1, all three upgrades are `buyable`.
    - With `upgrades.foam` true, `foam-up` is `bought`.
    - `canBuy(state, 'foam')` is false at 0 points and false when already bought.
12. `slotLocked(state, i)` is true for i = 4 while `unlocked[4]` is false, and false once it is true. `pointsText(state)` gives "Science points: 2" for points 2.

### e2e (lane: contract; `tests/e2e/content.mjs`)
13. **Locked spells.** On a fresh game:
    - `unlocked` is `[true,true,true,true,false,false]`.
    - `select 5` leaves `spell == 0`.
    - The `.slot[data-spell="5"]` and `.slot[data-spell="6"]` buttons have class `locked`.
    - `objs` contains every Contract name.
14. **Quest 2, all the way through.**
    - `tp 19.5 6` → `quests[1].step >= 1` and `zone == "lab"`. Q1 is not done, so `quest` stays 1; quests can be completed in any order.
    - Cast Water at the "water splitter" (`select 2`) → `level` is 1.
    - Cast Water at the "battery switch" → `power` is true.
    - `tick 300` → the hydrogen tube's `gas` is 20.0 and the oxygen tube's `gas` is 10.0.
    - Cast Fire at the "hydrogen tube" (`select 1`) → `caption` contains "2H2 + O2 → 2H2O", and the tube's `gas` is 0.
    - Cast Fire at the "oxygen tube" → within 2 s `quests[1].done` is true, `unlocked[4]` and `unlocked[5]` are true, `points == 1`, and `questDoneId` has risen by exactly 1.
    - `select 5` → `spellName == "Hydrogen"` and `formula == "2H2 + O2 → 2H2O"`.
    - `select 6` → `spellName == "Oxygen"`.
    - The slot `locked` classes are gone.
15. **Quest 3, success.**
    - `tp -12 28` → `zone == "pond"`.
    - Cast Frost (`select 3`) at pond north, then pond south, then the hot spring → the spring's `temp` is ≤ 1.
    - Cast Frost at pond middle → `frozen` is true.
    - `look 180 0` and `move 0 1 7` → within 9 s of the move, `quests[2].done` is true, `pos[2] ≥ 47.5`, and no "Splash" caption has appeared.
16. **Quest 3, the spring defeats a naive crossing.**
    - `tp -12 28`. Cast Frost (`select 3`) at north, middle and south (the spring stays hot).
    - `look 180 0` and `move 0 1 7`, sampling every frame for 9 s.
    - `quests[2].done` stays false, and `pos[2]` stays below 35.0 on every sample (the player is stopped at the melted middle's edge, about 34.65).
    - No caption starting "Splash!" appears during the walk.
    - The middle's `frozen` is false within 0.5 s of its cast.
    - Then `tp -12 39` → within 2 frames `pos[2]` is 29.0 ± 0.05, and `caption` starts with "Splash!".
17. **Free play, the divider and the reset lever.**
    - `tp 12 30` → `zone == "yard"`. Save `extinguished`.
    - Cast Fire at "yard bush 1" (`select 1`) → `burning`.
    - Cast Water at it (`select 2`) → `extinguished` is unchanged.
    - Cast Fire at "yard hedge" (`select 1`) → `burning`.
    - Cast any spell at the "reset lever" → within 2 frames every yard object has `burning:false`, the bushes have `fuel` 60.0, and the hedge has fuel 45.0.
    - **Divider:** `tp 3 40`, `look 90 0` (facing −x), `move 0 1 3`, and send `jump` every 10 frames for 3 s, sampling every frame. `pos[1]` exceeds 0.5 on at least one sample (the jumps happen), `pos[0]` stays ≥ 0.8 on every sample, and `zone` is "yard" throughout.
18. **Skill tree.**
    - Press `K` → `#skills` is not hidden, `state.paused` is true, the `#pause` card stays hidden, and a `cast` command leaves `casts` unchanged. `#skill-points` text is "Science points: 0".
    - Click `[data-node="foam-up"]` with 0 points → `upgrades.foam` is false.
    - Press `K` → closed and `paused` false.
    - After the test 14 steps (points 1): open, click `foam-up` → within 2 frames `upgrades.foam` is true, `points` is 0, and the node has class `bought`. Close.
    - `tp 12 30`, cast Foam (`select 4`) at "yard bush 2" → its `smothered` is ≥ 39.0.
19. **Flow and robustness.**
    - Win Q1: `select 2`, then `face` each burning bush and cast, until `quests[0].done`. Then `tick 30` raises `tick` by 30 (the chemistry is not frozen after the win). `#banner` is hidden within 6 s. `#objective .title` text is "Split the Water".
    - Then `tp -12 28`, cast Frost (`select 3`) at pond north, and send `reset`. The objective is now Q2's, so pond north is untouched (still frozen) and `pos` is (14, 0, 6) ± 0.1.
    - `reset all` → `quests` are all not done, `points` is 0, and `unlocked[4]` is false.
    - Across scenarios 13–19: 0 console errors, 0 `pageerror`, and no "panicked" in the console.
20. **Science sources.**
    - `content/science-sources.md` contains a row for every claim id C-I008-01 … C-I008-13 (Contract), each with at least one `https://` URL.
    - Every caption string in the Contract's caption table appears verbatim in `chem.rs` or `quest.rs`. Strings with `{}` are checked as the literal format string, placeholders included.
    - The test reads both files and fails, listing what is missing.
21. **Perf guard (B2).** Same machine, `?low`, headless Chromium at 4x CPU throttle. Take 5 interleaved runs each of base (main at the merge base) and this artifact; each run gives one `CPUFRAME p50_ms` value (the line `web/boot.js` logs). Let `spread` = max − min of the base's 5 spawn values.
    - At spawn (30 s idle): median of the artifact's 5 values ≤ median of the base's + max(1.0 ms, 2·spread).
    - In the south zone (`tp 0 30`, `look 180 0`, 30 s idle): the artifact's median ≤ the base's spawn median + max(2.0 ms, 2·spread).
    - If base does not log `CPUFRAME`, the test fails with "base has no CPUFRAME" rather than passing; if I-003's perf script has merged, use its runner for the measurement with these limits.
22. **Size guard (B3).** Brotli of `pkg-out` (wasm + js + css + html) grows ≤ 60 KB against base. No new binary assets.

## Out of scope
- Saving progress across reloads.
- Quests 4 and later, NPCs and dialogue.
- A Spark or electricity spell.
- Modelling the real moles or the water used up in electrolysis.
- Audio, including the pop sound.
- New art assets or textures (I-009).
- A Discoveries panel (cut from this item; a candidate for a later B5 item).
- A quest that requires spells 5 or 6.
- Editing I-002's smoke script so it walks quests 2 and 3. Tests 14–15 are written to be reused there. B1 is red until that lands (see "Bars moved").
- Analytics events for the new quests. I-005 consumes `questDoneId` and `quests`.
- Touch-specific skill-tree layout beyond tap = click.
- Balancing play time (the I-006 panel measures it).
- Wiring these tests into CI.

## Contract

### Rust types (`crates/bevy-client/src/chem.rs`; no Bevy)
**New `Kind` variants:** `Switch`, `GasTube`, `Pond`, `HotSpring`. Which gas a tube holds comes from its name, through the lab ids below.

**`Spell`:** adds `Hydrogen` and `Oxygen`, with `Spell::ALL: [Spell; 6] = [Fire, Water, Frost, Foam, Hydrogen, Oxygen]` (hotbar order). `name()` returns "Hydrogen" and "Oxygen". `formula()` returns "2H2 + O2 → 2H2O" and "O2 helps fuels burn".

**`EventKind` changes:**
- adds `Filled`, `Powered`, `TubesFull`, `Popped`, `Flared`, `Chilled` and `YardReset`;
- removes `Station`.

**`Fizzle`:** adds `NoFlame`, `NoFuel`, `Empty` and `NotHere`.

**Zones:** `pub enum Zone { Courtyard, Lab, Pond, Yard }`.

**New `Obj` fields:** `zone: Zone`, `level: f32` (splitter, 0..1), `power: bool` (splitter) and `gas_ml: f32` (tubes). `Obj::new` keeps its signature and defaults to `Courtyard`, and `fn in_zone(self, Zone) -> Self` sets the zone.

**New `World` fields:**
- `upgrades: Upgrades { splash, freeze, foam: bool }`;
- `lab: Option<Lab { splitter, switch, h2_tube, o2_tube: usize }>`, which `World::with_names(objs: Vec<(&'static str, Obj)>)` resolves from the names "water splitter", "battery switch", "hydrogen tube" and "oxygen tube" (`World::new` sets `None`);
- `tubes_full_sent: bool`, the `TubesFull` latch, cleared by `reset_zone(Lab)`;
- `initial: Vec<Obj>`, a copy used for zone resets;
- `fn reset_zone(&mut self, Zone)`, which restores that zone's objects from `initial`, and also zeroes `extinguished` for `Courtyard`.

**`extinguished`:** counts `Courtyard` objects only. `burning()` and `outcome()` consider `Courtyard` objects only.

**Constants:**

| constant | value |
|----------|-------|
| `H2_RATE_ML` | 2.0 |
| `H2_CAP_ML` | 20.0 |
| `O2_CAP_ML` | 10.0 |
| `H2_TEST_ML` | 5.0 |
| `O2_TEST_ML` | 2.5 |
| `OXY_BURN_S` | 10.0 |
| `POND_ICE_S` | 40.0 |
| `SPRING_C` | 80.0 |
| `SPRING_REWARM` | 0.05 |
| `SPRING_MELT_C` | 40.0 |
| `SPRING_MELT_RATE` | 4.0 |
| `SMOTHER_S` | 25.0 |
| `SMOTHER_THICK_S` | 40.0 |
| `SPLASH_GAP` | 1.0 |

**Captioned-event flag.** I-001 marks events that already got a caption by OR-ing `1 << 30` into `Event::obj` (`aim_and_cast`, `main.rs` line 876 at 6d5463e) and strips it in `react` (line 887). The contract lane names it `pub const CAPTIONED: usize = 1 << 30;` in `chem.rs`; core replaces the two literals with it. Core strips the flag from each event before passing events to `Quests::update`, so `update` always sees real object indices.

**`world::solid_obj`:**
- `Pond` → `ice <= 0.0`;
- `HotSpring`, `Switch` and `GasTube` → true;
- `Puddle` → false (unchanged);
- `IceWall` → `wall` (unchanged).

`world::chem_objects()` and `world::south_walls()` are pure, as in test 9.

### `player.rs` constants
`WALK`, `SPRINT`, `JUMP` and `GRAVITY` become `pub` (values unchanged: 5.0, 8.5, 6.0, 18.0). The contract lane makes this one edit so the world lane's test 9 compiles before core lands.

### Quest module (`crates/bevy-client/src/quest.rs`; no Bevy)
```
pub enum Upgrade { Splash, Freeze, Foam }
pub enum Notice { QuestDone(u8), Hint(&'static str), Splash }
pub struct Quests { pub done: [bool; 3], pub h2_tested: bool, pub o2_tested: bool,
                    pub bought: [bool; 3], pub done_id: u32 }
pub fn zone_at(p: [f32; 2]) -> Zone;              // section 2 of Smallest version
impl Quests {
  fn active(&self) -> Option<u8>;                 // 1..=3, lowest not done
  fn step(&self, q: u8, w: &World, p: [f32; 2]) -> u8;
  fn title(q: Option<u8>) -> &'static str;        // "Save the Greenhouse" | "Split the Water" | "The Frozen Pond" | "Free play"
  fn line(&self, w: &World, p: [f32; 2]) -> String;
  fn update(&mut self, w: &World, p: [f32; 2], events: &[Event]) -> Vec<Notice>; // events with CAPTIONED already stripped
  fn unlocked(&self) -> [bool; 6];
  fn points(&self) -> u8;                         // done count − bought count
  fn buy(&mut self, w: &mut World, u: Upgrade) -> bool;
  fn quest1_lost(&self, w: &World) -> bool;
  fn reset_all(&mut self);
}
```

**Step rules:**
- **Q1:** `step = extinguished.min(3)`.
- **Q2:** 0 while the ice wall stands, the player is not in `Lab` and `level == 0`; 1 once reached while `level == 0`; 2 while `level > 0 && !power`; 3 while `power` and some untested tube is below its test amount; 4 while `power` and every untested tube is at or above its test amount. After done, step stays 4.
- **Q3:** 0 until `zone_at(p) == Pond`, then 1. After done, step stays 1.

**Step lines (exact; `{}` are rounded to integers):**
- **Q1:** the existing "Fires put out: n / 3  ·  burning now: b".
- **Q2:**
  - s0 "Melt the ice wall (east) to reach the lab"
  - s1 "Fill the water splitter: cast Water at it"
  - s2 "Cast any spell at the battery switch to turn it on"
  - s3 "Splitting water... H2: {} / 20 mL, O2: {} / 10 mL"
  - s4 "Test each gas with Fire: hydrogen tube {}, oxygen tube {}" (each `{}` is ✓ or ○)
- **Q3:**
  - s0 "Go through the south-west gate to the Frozen Pond"
  - s1 "Freeze the pond to cross. The hot spring melts ice near it: cool it with Frost first. Spring: {} °C"
- **All quests done:** "Free play: try every spell on everything in the yard (south-east gate)."

**Hint:** when pond middle melts while the spring is above 40 °C, `Notice::Hint("The 80 °C hot spring melted the ice right away. Cool the spring with Frost first!")`.

### Captions (exact strings; the science review checks these)

| event or text | caption | claim ids |
|---------------|---------|-----------|
| `Filled` | "The cell is full of water with a little sodium sulfate (Na2SO4) dissolved in it, so electricity can flow. Pure water barely conducts." | C-I008-01, 02 |
| `Powered` | "Current on! Water splits: 2H2O → 2H2 + O2. Hydrogen bubbles up at the − electrode, oxygen at the + electrode." | C-I008-03, 04 |
| `TubesFull` | "Tubes full: 20 mL of hydrogen, 10 mL of oxygen. Two H2 for every O2, just like 2H2O → 2H2 + O2." | C-I008-05 |
| `Popped` | "Squeaky pop! A little hydrogen burns in one quick pop: 2H2 + O2 → 2H2O." | C-I008-06 |
| `Flared` | "The flame flares up: more oxygen makes a fuel burn faster and hotter." | C-I008-07 |
| `Chilled` | "Liquid nitrogen cooled the hot spring right down. It will warm up again soon!" | C-I008-12 |
| `Fizzle(NoFlame)` | "No flame here, so the hydrogen just floats up: H2 is about 14 times lighter than air." | C-I008-08 |
| `Fizzle(NoFuel)` | "Oxygen does not burn by itself: it only helps a fuel burn." | C-I008-09 |
| `Fizzle(Empty)` | "Not enough gas in the tube yet. Wait for more bubbles." | none |
| `Fizzle(NotHere)` | "Use Water on the cell, and Fire to test the gases." | none |
| `YardReset` | "The free-play yard is fresh again." | none |
| `Melted` (changed: drops "Now it is a puddle.") | "The ice melted at 0 °C: H2O(s) → H2O(l)." | C-I008-10 |
| Hydrogen cast | "Hydrogen, H2: the lightest gas. With a flame it burns: 2H2 + O2 → 2H2O." | C-I008-11, 06 |
| Oxygen cast | "Oxygen, O2: it does not burn, but it makes fuels burn faster and hotter." | C-I008-09, 07 |
| Q2 done | "You made hydrogen and oxygen from water! New spells: 5 Hydrogen, 6 Oxygen." | C-I008-03 |
| Q3 step s1 | "Freeze the pond to cross. The hot spring melts ice near it: cool it with Frost first. Spring: {} °C" | C-I008-13 |
| Q3 done | "You crossed! Liquid nitrogen (−196 °C) froze the water, and cooling the hot spring kept it frozen." | C-I008-12 |
| Q3 hint | "The 80 °C hot spring melted the ice right away. Cool the spring with Frost first!" | C-I008-13 |
| `Splash` | "Splash! The ice melted under you. Back to the shore." | none |

### Science claims for `content/science-sources.md`
The contract lane creates the file, or adds an `## I-008` section if I-004 created it first, with columns `| id | claim | source(s) |`. The URLs below are **candidates**: the contract lane must open each one and confirm it says what the row claims, or replace it.

| id | claim | candidate sources |
|----|-------|-------------------|
| C-I008-01 | Pure water is a poor conductor; electrolysis needs a dissolved electrolyte such as Na2SO4. NaCl is avoided because chloride is oxidised to Cl2 at the anode. | https://en.wikipedia.org/wiki/Electrolysis_of_water ; https://chem.libretexts.org (Electrolysis of water) |
| C-I008-02 | Na2SO4 does not take part: the products are still H2 and O2. | same |
| C-I008-03 | 2H2O(l) → 2H2(g) + O2(g) with electrical energy. | same ; https://www.bbc.co.uk/bitesize (GCSE electrolysis of aqueous solutions) |
| C-I008-04 | H2 forms at the cathode (−), O2 at the anode (+). | same |
| C-I008-05 | The gas volume ratio H2 : O2 is 2 : 1, because equal volumes of gas at the same T and p hold equal moles (Avogadro). | https://en.wikipedia.org/wiki/Avogadro%27s_law ; Electrolysis of water |
| C-I008-06 | A lit splint makes a small amount of hydrogen burn with a squeaky pop; 2H2 + O2 → 2H2O. | https://www.bbc.co.uk/bitesize (GCSE tests for gases) ; https://edu.rsc.org |
| C-I008-07 | Extra oxygen makes a fuel burn faster and hotter; it relights a glowing splint. | BBC Bitesize tests for gases ; https://en.wikipedia.org/wiki/Oxygen |
| C-I008-08 | H2 density about 0.084 kg/m³ vs air about 1.20 kg/m³ at 20 °C and 1 atm, a ratio of about 14. | https://www.engineeringtoolbox.com/gas-density-d_158.html ; https://webbook.nist.gov |
| C-I008-09 | Oxygen is an oxidiser, not a fuel: it does not burn by itself. | https://en.wikipedia.org/wiki/Oxygen ; https://en.wikipedia.org/wiki/Fire_triangle |
| C-I008-10 | Ice melts at 0 °C at 1 atm: H2O(s) → H2O(l). | https://webbook.nist.gov (water) |
| C-I008-11 | Hydrogen is the lightest gas (lowest molar mass, 2.016 g/mol). | https://en.wikipedia.org/wiki/Hydrogen ; https://pubchem.ncbi.nlm.nih.gov/compound/Hydrogen |
| C-I008-12 | Liquid nitrogen boils at −195.8 °C (77.4 K); it freezes water and cools warmer things on contact. | https://webbook.nist.gov (nitrogen) ; https://en.wikipedia.org/wiki/Liquid_nitrogen |
| C-I008-13 | Water above 0 °C melts ice; hotter water melts it faster. The 80 °C spring is a game value, which the reviewer checks only for plausibility: natural hot springs reach about 40 to 100 °C. | https://en.wikipedia.org/wiki/Hot_spring |

### Commands (`__game.cmd`; unknown values are ignored)
- `select 1..6`: hotbar order 1 Fire, 2 Water, 3 Frost, 4 Foam, 5 Hydrogen, 6 Oxygen; 5 and 6 are ignored while locked.
- `buy splash|freeze|foam`.
- `reset`: restarts the active quest. `reset all`: a fresh game.
- `tick N`: now always steps.
- `jump` (new): passes `jump = true` to `player.step` on the next frame, exactly as holding Space for one frame.
- `tp`, `face`, `look`, `turn`, `move` and `cast` are unchanged, apart from the new clamp.
- Keys: `5` and `6` select spells, `K` opens the tree. The wheel skips locked spells.

### New `__gameState` fields (existing fields keep their meaning)

| field | type | meaning |
|-------|------|---------|
| `unlocked` | bool[6] | spells usable |
| `quest` | 0..3 | active quest, 0 = all done |
| `questTitle`, `questLine` | string | HUD objective box |
| `quests` | [{`id`: "greenhouse" \| "split" \| "frost", `done`: bool, `step`: u8}] ×3 | per-quest status |
| `questDoneId` | u32 | +1 per quest completion |
| `points` | u8 | unspent science points |
| `upgrades` | {`splash`, `freeze`, `foam`: bool} | upgrades bought |
| `zone` | "courtyard" \| "lab" \| "pond" \| "yard" | `zone_at(player xz)` |
| `objs[]` adds | `z` (zone string), `level` (2 dp), `power` (bool), `gas` (mL, 1 dp) | per-object state |

`outcome` stays: "won" once Q1 is done, "lost" only while `quest1_lost`, otherwise "playing". `extinguished`, `goal` and `burning` cover the courtyard only.

### Visuals (core lane applies them in `sync_visuals`; world lane provides the entities)
- `Target` gains `pub gauge: Option<(Entity, f32)>`. It is `Some` only for the hydrogen tube, the oxygen tube and the water splitter, and `None` for every other target. The `Entity` is a gauge mesh built by the world lane with its origin at the bottom (vertices span y 0..h), so scaling y alone grows it upward; the `f32` is h, its full height in metres. Core sets the entity's `Transform.scale.y` to `max(frac, 0.001)` each frame, where `frac` is `gas_ml / cap` for a tube (H2_CAP_ML or O2_CAP_ML by lab id) and `level` for the splitter.
- Ponds reuse `puddle`, which is shown as the ice sheet while `ice > 0`, with the water material otherwise.
- The battery switch uses its existing `mats` normal (off) or hot (on) look.

### Web (`crates/bevy-client/web/skilltree.js`; an ES module with no DOM access)
- **Functions:** `treeModel(state) -> {spells: [{id, status}], upgrades: [{id, status}]}`, `canBuy(state, id)`, `slotLocked(state, i)` and `pointsText(state)`.
- **Ids:** spell ids are `fire`, `water`, `frost`, `foam`, `hydrogen` and `oxygen`. Upgrade ids are `splash-up`, `freeze-up` and `foam-up`, which map to the `buy` argument with "-up" removed.
- **DOM:** `#skills` (with `.hidden`), `#skills-btn`, `[data-node=<id>]` with class `learned`, `locked`, `buyable` or `bought`, `#skill-points`, and `.slot.locked`.

## Lanes
Merge order is the row order. The contract step lands first. world, core and web then build in parallel against the Contract: core and web use the Contract's types and state fields, and web tests use fake state objects.

| lane | owns globs | builds |
|------|-----------|--------|
| contract | `crates/bevy-client/src/chem.rs` and `crates/bevy-client/src/quest.rs` (type and signature stubs only, with bodies that compile and return defaults), `tests/e2e/content.mjs`, `content/science-sources.md`, `package.json`, `package-lock.json`; plus two one-off edits outside its globs: the `mod quest;` line in `crates/bevy-client/src/main.rs` and `pub` on the four constants in `crates/bevy-client/src/player.rs` | Contract types, enums, constants and `CAPTIONED`; `zone_at` stub; `world::south_walls` and `chem_objects` signatures may be stubbed by world instead; e2e tests 13–22 as written; claim rows with verified URLs; a pinned `playwright` devDependency only if I-002 or I-007 has not added one. After the contract step merges, `main.rs` and `player.rs` belong to core, which rebases over those two edits |
| world | `crates/bevy-client/src/chem.rs`, `crates/bevy-client/src/quest.rs`, `crates/bevy-client/src/world.rs` | electrolysis, gas tests, spells 5–6, pond and spring, upgrades, zone scoping, `reset_zone`, the quest state machine and `zone_at`, the south-zone layout (walls via `south_walls()`, gates, pond, spring chimney, vault arch, yard, lever), `chem_objects()`, gauge entities; Rust tests 1–10 |
| core | `crates/bevy-client/src/main.rs`, `crates/bevy-client/src/player.rs`, `crates/bevy-client/src/fx.rs`, `crates/bevy-client/src/bridge.rs`, `crates/bevy-client/src/feel.rs` (only if I-007 merged) | `Quests` resource and wiring in `react` (stripping `CAPTIONED`); chemistry stepping unless paused; casting unless paused; spells 5–6 selection, projectiles and glow materials ([_; 6]); commands including `jump`; new state fields; the new player clamp; frozen ponds as ice sheets; splash relocation; gauges in `sync_visuals`; `reset`/`reset all` semantics |
| web | `crates/bevy-client/web/**` | 6-slot hotbar with locked slots; objective box from `questTitle`/`questLine`; banner auto-hide on the Q1 win; `skilltree.js` with tests 11–12; `#skills` overlay, K/Esc/button, pointer-lock release without the pause card, buy clicks |

## Critique: accepted and rejected
Applied from `critique.md` (r1 → r2):
- **Cut the Discoveries panel** (`DISCOVERIES`, `discovery_of`, the `discovered` field and state, `#disc-*` DOM, old tests 11, 13 and the 19 tail). Tests are renumbered: Rust 1–10, web 11–12, e2e 13–22.
- **Dropped C-I008-14** (ice density). The vault arch has no sign text.
- **Walls ≥ 3.0 m**, pure `south_walls()`, layout checks in test 9, and an e2e jump-at-divider check in test 17 (with a new `jump` command, since the grammar had none).
- **Test 16 (was 17)** now asserts `pos[2] < 35.0` and no Splash caption during the walk.
- **`select` steps** written out in every e2e scenario; hotbar order stated in the Contract.
- **Test 10** has a concrete row table, plus explicit step rules, and states that only `Popped`/`Flared` on the tubes count.
- **Ice and gas arithmetic pinned** (`max(0)` / `min(cap)` clamps, `Melted` on the first zero tick).
- **Q3 s1 line names the clue** and is in the caption table under C-I008-13.
- **Perf test** uses 5 interleaved runs, medians, a limit of max(fixed, 2·spread), and fails loudly without `CPUFRAME`.
- **B1 red** stated, with the fix named.
- **`zone_at`** defined once, used for the state field and the quest steps; `zone` gains "lab".
- **Player constants and `mod quest`:** the contract lane makes both one-off edits; core owns the files afterwards.
- **Water on a full splitter** re-emits `Filled`; **`Target.gauge`** fully specified; **`CAPTIONED`** flag named and located.
- **Spells 5 and 6 not required by a quest:** said so under B5 and Out of scope.
- **Hydrogen on a burning bush:** the `Popped` caption now says a little hydrogen burns in one quick pop (C-I008-06 adjusted).
- Added a `Chilled` caption (it existed as an event but had none) and listed `Chilled` in the `EventKind` additions, where r1 used it without declaring it.

Rejected or changed:
- **Optional split into I-008a and I-008b: rejected.** B8 needs all four parts at once to be measured, so a split adds a merge cycle and a second rebase against I-007 without letting any bar move sooner. With the Discoveries cut, the item is smaller and the lanes stay independent.
- **"State the result at tick 1200" (test 5): changed.** 1200 f32 subtractions of `DT/40` can land on 1199, 1200 or 1201, so an exact tick would make the test fail on rounding rather than on behaviour. The test pins the decrement rule and uses a 1190 / 1210 window, which still fails any wrong rate (a 10% error moves the zero by 120 ticks).
- **Gate lintels:** not added. The gates are open to the sky; the walls either side are ≥ 3.0 m, which is what closes the bypass.
