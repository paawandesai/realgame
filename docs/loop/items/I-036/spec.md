# I-036: Level 4, the Crystal Lab (split water 2:1, hydrogen pops, a hydrogen balloon lifts the crate)

**based_on:** origin/main 5662cd7 (merge of PR #53; I-029 worked casts, I-035 Frozen Gate and I-037 Marsh on main). **Lane:** built on Paawan's Mac as the third build lane (wave 14, 2026-11-01). It touches no file I-030 or I-049 own: `bench.*`, `progress.*` and `worked.*` do not change.

**Why.** This is backlog row I-036: "Crystal Lab: Split Water (electricity, H2:O2 2:1), Spark H2, H2 balloon lift (supersedes I-008)". It moves **B8** (4 levels: this makes the fourth) and **B11**: lifting a crate with a balloon is a non-shooting use of chemistry, and Water is the crafted Bench spell it needs. It follows Paawan's wave rule: playable, 2-5 min, winnable by a 5-year-old (Sprout, aim assist on, no typing) and still interesting to a 13-year-old.

## Facts relied on (5662cd7)

- `chem.rs`: `Kind` ends with `GasVent, Hollow`. `Obj` is built only through `Obj::new` and `Obj::hollow` (no struct literals in tests), so new fields are safe. `Goal::Reach` turns off reseed and the fire win; `World::reach()` latches `Won`. `chem::electrolysis` already defines the Hoffman tube geometry with `GAS_H2 = 2 × GAS_O2` (register P-015).
- `main.rs`: `Level { n, cleared1, cleared2, pending }`; `next_open()` is false for level 3; `levels()` handles triggers and loads; `?level=` and the `level` command accept 1-3; `reset` picks the start pose and line; `projectiles` asks `level2/level3::caption`; `publish` ends with `nextOpen`, `pools`, `ventsLit`; `control` stops the walk on level 3's win.
- `level2.rs`: the north arch pattern (`NEXT_ARCH (0, -22.6)`, `next_door()`, `NextDoor`, `ARRIVE_NORTH (0, 0, -20.4)`, `solved()`), which `level_visuals` already lights while `next_open`.
- `level3.rs`: no forward arch; the lantern clearing (x ±4) runs to the north wall.
- `hud.js`: title, objective and banner by `state.level`; `#capicon` accepts `freeze|melt|wrong|gas`.
- Golden: key order ends `...I035_KEYS, ...I037_KEYS` (golden 1091-1093); the tail check takes `slice(-10)` (1367); sections registered in the `ONLY` map (98-104).
- Science register: last ids **C-096, P-023, [S31]**.

## What a kid does (about 3 minutes)

0. **The Marsh gets a forward arch.** A closed arch in the Marsh's north wall at (0, −22.6), solid and dark until the lantern is reached, then glowing; the level-3 banner's small line then says "Walk to the glowing arch to visit the Crystal Lab." Walking into it loads level 4. `cleared3` keeps it open for the page load; `?level=4` loads level 4 directly.
1. **0:00 Arrive.** A stone lab hall the courtyard's size. A stone partition wall crosses the hall at z −4 with one doorway (2.4 m wide) in the middle, blocked by a heavy iron **crate**. A limp **balloon** is tied to the crate, and a hose runs from it back to the **water splitter** on a bench at (−4, 8): a battery and two glass tubes (hydrogen on the left, oxygen on the right). The lantern stands behind the wall at (0, −16). Back arch (0, 22.6) returns to the Marsh. Caption: "The Crystal Lab. Split water to fill the balloon, lift the crate, reach the lantern."
2. **0:20 Split water.** Water cast at the splitter runs it: both tubes fill with gas bubbles, the hydrogen column always exactly twice the oxygen column, then the hydrogen flows up the hose into the balloon and the oxygen is let out. Each cast adds **0.34** to the balloon (0.34, 0.68, then 1).
3. **1:00 Lift.** At 1 the balloon floats up and lifts the crate out of the doorway onto a ledge on the wall (one `Lifted` event). The doorway is clear.
4. **Pop (the hydrogen test).** Fire on the balloon while it holds hydrogen: a squeaky pop, the hydrogen burns (2H₂ + O₂ → 2H₂O) and the balloon is empty again. Splitting more water refills it. If the crate is already on the ledge it stays there (no soft-lock).
5. **Wrong chemistry fails kindly.** Fire, Frost or CO₂ on the splitter: it needs water. CO₂ on the balloon: heavier than air, it would weigh it down. Water or Frost on the balloon: it needs a light gas. Fire on an empty balloon: nothing to burn. Any spell on the crate: too heavy, fill the balloon. Each gets a band caption.
6. **2:30 The lantern.** Within 1.5 m of (0, −16) the player wins: banner "You lit up the Crystal Lab!" with "Water split 2:1 into hydrogen and oxygen, and light hydrogen lifted the crate. Walk to the arch to go back. Press R to play again." The walk stops on the win (as level 3). `R` restarts level 4.

**Sprout path:** Water is the default spell; aim at the big splitter and click three times, then walk. No formula typing beyond what worked casts (I-029) already ask per band.

## Captions (exact; Sprout lines also set `capIcon`)

The band comes from `BenchState.band`; `None` shows the Apprentice line.

| event (level 4) | Sprout 4-7 (icon) | Seedling 8-10 | Apprentice 11-15 (and None) | Adept 16+ |
|---|---|---|---|---|
| Split (splitter) | `Zap! The water splits into two gases. More water!` (gas) | `Electricity splits water into hydrogen and oxygen. Twice as much hydrogen comes out as oxygen.` | `2H₂O → 2H₂ + O₂: the hydrogen tube fills twice as fast as the oxygen tube.` | `Electrolysis: 2H₂O(l) → 2H₂(g) + O₂(g). Two moles of H₂ for every mole of O₂, and equal moles of gas fill equal volumes, so the gas columns are 2:1.` |
| Lifted (splitter) | `Up it goes! The light gas lifts the crate.` (gas) | `Hydrogen is much lighter than air, so the full balloon floats up and lifts the crate.` | `H₂ (2 g/mol) is about 14 times lighter than air (29 g/mol). The balloon floats, and the crate goes with it.` | `Buoyancy: the air the balloon pushes aside weighs more than the balloon, its H₂ and the crate together, so they rise.` |
| Popped (balloon) | `Pop! Hydrogen burns with a bang.` | `Squeaky pop! That is the test for hydrogen: it burns with oxygen in the air and makes water.` | `Pop! 2H₂ + O₂ → 2H₂O. Hydrogen is flammable, so the balloon burst. Split more water to fill it again.` | `Hydrogen burns in air across a wide range of mixtures (4 to 74 %), which is why hydrogen balloons were a fire risk.` |
| any spell but Water on the splitter (Fizzle(NotFlammable), Chilled, Smothered) | `The machine needs water. Try Water!` (wrong) | `Electricity splits water, so the splitter needs water. Cast Water into it.` (Seedling to Adept) | ← | ← |
| CO₂ on the balloon (NoChange) | `Too heavy! That gas will not float.` (wrong) | `CO₂ is heavier than air, so it would weigh the balloon down. It needs a light gas: hydrogen.` (Seedling to Adept) | ← | ← |
| Water or Frost on the balloon (Soaked, Chilled) | `The balloon needs a light gas. Make some with water!` (wrong) | `The balloon needs a gas lighter than air. Split water to make hydrogen.` (Seedling to Adept) | ← | ← |
| Fire on an empty balloon (Fizzle(NotFlammable)) | `Nothing to burn yet!` (wrong) | `The balloon is empty: no hydrogen to burn yet.` (Seedling to Adept) | ← | ← |
| any spell on the crate (NoChange) | `Too heavy to push. Fill the balloon!` (wrong) | `The crate is too heavy to move. A balloon full of hydrogen can lift it.` (Seedling to Adept) | ← | ← |
| any spell on the lantern | `That is the lantern. Walk up to it!` | ← | ← | ← |

## Rules (chem.rs)

- `Kind::Splitter` (the water splitter), `Kind::Balloon`, `Kind::Crate`. None is flammable; none counts toward `Goal::Fires`. Level 4 uses `Goal::Reach`.
- `Obj` gains `gas: f32` (balloon hydrogen, 0..1), `lifted: bool` (crate on the ledge) and `run: f32` (splitter: seconds left of the current split, for the tube display). Defaults 0 / false / 0.
- **Water on the splitter:** `run = SPLIT_S` (2.5 s); the balloon's `gas` rises by `H2_PER_SPLIT` (0.34, cap 1, snap to 1 within 1e-4). If it reaches 1 and the crate is not lifted: `lifted = true`, event `Lifted` (on the splitter). Otherwise event `Split`. With no balloon in the world: `Split` only.
- **Fire, Frost, CO₂ on the splitter:** `Fizzle(NotFlammable)`, `Chilled`, `Smothered`; nothing changes.
- **Fire on the balloon:** `gas > 0` → `gas = 0`, event `Popped`. `gas == 0` → `Fizzle(NotFlammable)`. The crate never comes down.
- **CO₂ / Water / Frost on the balloon:** `NoChange` / `Soaked` / `Chilled`; nothing changes.
- **Any spell on the crate:** `NoChange`; nothing changes.
- `step` counts the splitter's `run` down to 0; nothing else in the lab changes over time.
- `hit_kind`: `Lifted` and `Popped` are `Hit` (no fire was put out).
- `world::solid_obj`: Crate is solid while `!lifted`; Balloon is never solid; Splitter is solid (the bench).
- Level 1-3 behaviour does not change: the existing tests pass unedited. `chem.rs` tests compile alone (`rustc --test src/chem.rs`): no `crate::` paths.
- `chem::electrolysis::column(run) -> (f32, f32)`: the displayed H₂ and O₂ column heights for a split with `run` seconds left: both grow, hold, then drain together, always `h2 = 2 × o2`, using `GAS_H2`/`GAS_O2`.

## Acceptance tests (each fails on 5662cd7)

**Rust** (`cargo test`; `rustc --test src/chem.rs` for chem):

1. `chem` splitting: in a lab world (splitter, balloon, crate, lantern), three Water casts on the splitter give balloon gas 0.34 / 0.68 / 1.0 with events `Split`, `Split`, `Lifted`; the crate is lifted only after the third; a fourth cast gives `Split` and changes nothing.
2. `chem` wrong spells: Fire, Frost and CO₂ on the splitter give `Fizzle(NotFlammable)`, `Chilled`, `Smothered` and leave gas 0; CO₂, Water and Frost on the balloon give `NoChange`, `Soaked`, `Chilled` and leave gas unchanged; every spell on the crate gives `NoChange`.
3. `chem` pop: Fire on a balloon with gas gives `Popped` and gas 0; on an empty balloon `Fizzle(NotFlammable)`. After the lift, a pop leaves `lifted` true.
4. `chem` lab never wins, loses or reseeds by itself over 20 000 steps; the splitter's `run` counts down to 0.
5. `chem::electrolysis::column`: for every `run` in 0..SPLIT_S, `h2 == 2 × o2` (within 1e-6), columns are 0 at `run = 0` and at `run = SPLIT_S`, and reach `(GAS_H2, GAS_O2)` in the hold phase.
6. `level4` captions: `caption` returns the exact table strings and icons for every (event, object, band) pair; `None` gives the Apprentice line; every string passes the player-text checks (no `"`, `\` or control characters, ≤ 200 bytes).
7. `level4` walk: from START a collider sweep north is stopped south of the partition (z > −4) while the crate is down, along the middle and hugging either doorpost; after the lift it reaches the lantern. Sweeps east and west along the partition's south face never pass it.
8. `level3` forward arch: `level3::solved` fills both hollows (pools 1, 1), puts every vent out and latches `Won`.

**Golden** (new section `crystalLab`, run after `marsh`, in the `ONLY` map as `ONLY=crystalLab`; drives the state bridge):

9. The Marsh's forward arch is shut before its lantern: `level 3`, then `tp 0 -21.8` leaves `level === 3` and `nextOpen === false`.
10. Reach the lab by playing: solve level 3 (2 CO₂ on the dip, 3 on the hollow, walk to the lantern); `nextOpen === true` and the banner's small line mentions the Crystal Lab. Then `tp 0 -21.8` gives, within 5 s, `level === 4`, objs named `water splitter`, `balloon`, `crate`, `lantern`, `balloon === 0`, `crateUp === false`, `outcome === 'playing'` and a caption starting "The Crystal Lab".
11. The crate blocks: `move 0 1 4` from START leaves z > −4.
12. Wrong spells: Fire on the splitter gives the Apprentice "needs water" line and `balloon === 0`; CO₂ on the balloon gives the CO₂ line and `balloon === 0`.
13. Splitting: three Water casts on the splitter give `balloon` 0.34 / 0.68 / 1 (within 1e-3) with the Split, Split and Lifted lines, and `crateUp` false, false, true. While a split runs, a published `tubes` sample has `tubes[1] > 0` and `tubes[0] / tubes[1]` within 1e-3 of 2.
14. Pop: Fire on the balloon gives the Popped line, `balloon === 0` and `crateUp === true`.
15. Win: a scripted walk crosses the doorway (z < −5) and reaching the lantern gives `outcome === 'won'` and the banner "You lit up the Crystal Lab!"; a further `move 0 1 2` leaves `pos` unchanged.
16. Captions per band: for each band, a fresh level 4 Water cast on the splitter shows that band's Split line; Sprout gives `capIcon === 'gas'`.
17. Way back: the back arch gives `level === 3` with `nextOpen === true` and both pools full.
18. `?level=4` without `debug` loads the lab, and Water can be selected and cast.
19. Key order: `I036_KEYS = ['balloon', 'crateUp', 'tubes']` appended after `I037_KEYS`; `objs` keys unchanged; no console errors.

**Pinned golden edits (allowed, listed in the PR):** the key-order check (golden 1091-1093: add `I036_KEYS`) and the tail check (1367: `slice(-13)` including the three new keys). Same kind of edit I-035 and I-037 made. No other existing check changes.

## Science rows (content/science-sources.md; next ids C-097.., P-024.., [S32]..)

- Electrolysis gives H₂ and O₂ 2:1 by amount and so by volume: [S13] (stoichiometric amounts at cathode and anode), [S12] (Avogadro), new **[S36]** Wikipedia "Electrolysis of water" ("the amount of hydrogen generated is twice the amount of oxygen"; pure water conducts poorly, so an electrolyte such as a salt is added). Verdict for the Split lines: **simplified** (the splitter's electrolyte is not shown; real cells add one).
- H₂ 2.016 g/mol: new **[S32]** NIST WebBook Hydrogen (2.01588). Air 28.96 [S8]. 28.96 / 2.016 = 14.4, "about 14 times lighter".
- Gas balloons float because they are filled with a gas lighter than air (hydrogen or helium): new **[S34]** Wikipedia "Gas balloon". Buoyancy (Adept): simplified (one balloon lifting a crate is gameplay scale; the rule that the displaced air must outweigh the load is true).
- Hydrogen burns with oxygen in air to water, 2H₂ + O₂ → 2H₂O, flammable, 4-74 % in air: new **[S35]** Wikipedia "Hydrogen".
- The squeaky pop is the test for hydrogen: new **[S33]** Wikipedia "Splint (laboratory equipment)".
- CO₂ heavier than air: [S5] [S8] [S26].
- Parameters: P-024 `H2_PER_SPLIT` 0.34 [S0] (three casts fill the balloon); P-025 `SPLIT_S` 2.5 s [S0] (tube display time).
- Not claims: the objective line, arch and "Press R" lines, the lantern line, "Too heavy to push. Fill the balloon!" style instructions where they state no science.
- Safety: no line suggests making hydrogen at home; the Adept pop line states the fire risk.

## Contract

- **`chem.rs`**: the items under Rules, plus `SPLIT_S`, `H2_PER_SPLIT`, events `Split`, `Lifted`, `Popped` with engine captions (the Apprentice lines), and `electrolysis::column`.
- **`level4.rs`** (new): `pub fn layout() -> Layout` (pure), `pub fn build(..) -> (Colliders, Chem)`, `pub fn caption(kind, obj, band)`, `pub fn sync(..)` (balloon size, crate path, tube columns), `balloon(w)`, `crate_up(w)`, `tubes(w)`. Targets named `water splitter`, `balloon`, `crate`, `lantern`. Constants `START`, `BACK_ARCH (0, 22.6)`, `LANTERN (0, -16)`, `LANTERN_REACH 1.5`. Every entity carries `LevelRoot`.
- **`level3.rs`**: north arch at `NEXT_ARCH (0, -22.6)` (posts, lintel, `level2::NextDoor` panel so `level_visuals` lights it), `ARRIVE_NORTH`, `solved()`.
- **`main.rs`**: `Level.cleared3`; `next_open` level 3 = won or `cleared3`; `levels()` handles the level-3 forward arch, the level-4 back arch and lantern; `level`/`?level=` accept 4; `reset` for level 4; arriving in level 3 from level 4 uses `level3::solved` and `ARRIVE_NORTH`; captions from `level4::caption`; the walk stops on the level-4 win; the level-3 north door is solid while shut; the finder points at the level-3 forward arch after the win; `publish` appends `balloon`, `crateUp`, `tubes`.
- **`world.rs`**: `solid_obj` arms for the three kinds.
- **`hud.js` / `style.css`**: title "The Crystal Lab", objective and banner for `level === 4`; the level-3 banner line when `nextOpen`; finder toward the forward arch in level 3.
- **Golden**: section `crystalLab` and the pinned key edits.

## Out of scope

- Level select, mastery gates (I-034), saved level progress.
- A fifth level and a forward arch out of the lab.
- A new Hydrogen spell or Bench recipe; the tubes as separate targets; oxygen-relights-a-splint.
- Perf query for level 4 in `perf.mjs` (follow-up).

## Follow-ups

- F1: oxygen tube relights a glowing splint (the oxygen test).
- F2: `perf.mjs --query level=4`.
- F3: fold level 4 into I-034's level select.
