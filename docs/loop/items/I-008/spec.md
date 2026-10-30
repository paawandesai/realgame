# I-008: Quest 2, "Split the Water", at the lab bench

**based_on:** origin/main 265a93e3c53c0046bf2493e52bc85f6d984676fd (2026-10-30, merge of PR #27 `loop/I-023`; I-005, I-007, I-018 and I-023 are all merged).

**Revision 3 (refresh).** r2 (2026-10-29, critiqued) was written against the unmerged I-001 branch. It had 3 quests, 6 spells, a south zone, a free-play yard and a skill tree in one item. Since then these merged: the first-person client, I-007 (feel), I-005 (analytics with `quest_complete`), I-018 (`chem::electrolysis` tubes) and I-023 (game clock). The panel also found that kids could not see their own casting, which I-019 fixes and which is building now. r3 cuts the item to the **smallest next content step**: one quest at the existing lab bench, using the existing electrolysis prop, counted by the existing `quest_complete` event, and built on I-019's stream and visibility contract. Everything else from r2 moves to the follow-up rows listed under "Out of scope". r2's critique points that still apply are kept: an exact step table, pinned gas arithmetic, Na2SO4 rather than NaCl, the golden path walking every quest, no auto-continue, and sources verified by the builder.

**Revision 4 (2026-10-30, planner response to the r3 critique).** Every must-fix is taken. M1 is fixed the smaller way: "Keep exploring" is dropped, and a quest 2 win ends the game like a quest 0 win. Other changes: the cell pick box is tall (M5), tests use deltas, ratios and a debug `lab gas` command (M3), the `I008_KEYS` order and the `keysOk` edit are in the Contract (M4), pointer lock is released for the banner button (M2), test amounts are 10/5 mL (S1), `gasH` comes from the drawn transforms (S2), plus S3 to S7 and the lane gaps. See the "Planner response" section in `critique.md`.

**Depends on I-019** (casting you can see; critiqued, building). Builders branch from origin/main **after I-019 merges**. Before dispatch, the planner moves `based_on` to that merge commit and re-checks the facts below and the "Relied on from I-019" list. If I-019's merged contract differs from that list, the planner refreshes this spec first. Do not dispatch on this text otherwise.

## Facts from 265a93e this spec relies on

- **The lab is already built, but it is a placeholder.**
  - The east alcove (`world.rs` line 768) is closed by the ice wall at x 17.6 (half 0.45, z 3..9). Three Fire casts melt the wall (`ice -= 0.34` per cast).
  - The lab bench is a `Target` named `"lab bench"`, `Kind::LabBench`, at (21.8, 6.0). Its pick box is centred at (21.8, 1.2, 6.0) with half extents (0.75, 1.0, 1.8). That box encloses both tubes.
  - `World::apply_inner` answers **any** spell on a `LabBench` with `EventKind::Station`. Its caption is `"Water-splitting station (coming soon): 2H₂O → 2H₂ + O₂ with electricity."` (register row C-026).
- **The electrolysis prop is static.** `world.rs` (lines 1084–1110) draws `chem::electrolysis::tubes()` once at build time: glass, water and gas cylinders at a fixed 0.6 m (H2) and 0.3 m (O2) gas height. `tube(dz, gas)` already takes the gas height as an argument. I-018's test 5 greps that block for numeric literals, so the block must stay free of them.
- **The game ends at the first win.**
  - `World` latches `ended` on the first Won or Lost.
  - `apply` does nothing once ended. `chem_tick` and the `tick` command step only while `outcome() == Playing`. `aim_and_cast` casts nothing once ended (`let ended = chem.world.outcome() != Outcome::Playing`).
  - So after "You saved the greenhouse!" the only things left to do are look around and press R. The win banner reads `"{n} fires put out. Press R to play again."`
- **`reseed`** relights the bush farthest from the greenhouse whenever nothing burns and `extinguished < GOAL_FIRES`. A calm courtyard would therefore relight on its own unless reseed is turned off.
- **Analytics already handles more than one quest.**
  - `analytics.js` `questEvents(prev, state, sent)` sends `quest_complete` when `outcome` goes from `'playing'` to `'won'`. The quest index is `state.quest` when it is an integer from 0 to 15, otherwise 0. Each quest is sent at most once per session.
  - The server accepts quest 0..15 and stores it in `room`. `/api/stats` returns `funnel.quests[q]`.
  - `__gameState` has no `quest` field today, so every completion counts as quest 0. Publishing `quest` is the whole client-side analytics change. **`analytics.js` and `functions/**` are not edited.** The data collected does not change: quest indexes 0–15 are already in the documented contract, so this is not a human-only privacy change.
  - `tests/e2e/analytics-e2e.mjs` main run (with `?debug`) wins quest 0 with `tp`/`face`/`select`/`cast` and asserts "D1 main: exactly 1 quest_complete (seedling, room 0)" and `funnel.quests[0].seedling === 1`.
- **The golden path pokes the bench.** `golden.mjs` line 264 casts Water at `lab bench` during the fire quest and checks `/splitting/` in the caption.
- **Game clock (I-023).** `clock::frame_dt` clamps dt to 0.25 s. The chemistry is a fixed 30 Hz step (`chem::DT`). `gameTime` is published.
- **Hint plumbing (I-007).** `hud.js` has one idle hint, `IDLE_HINT` ("Look for a fire. Press 2 for Water…"), and `IDLE_SLOT = 1`. The comment at line 24 says "a new quest brings its own idle hint and slot". `MARK_MS` has `hit`, `fizzle` and `out`, and `out` gets the star marker and the cheer.
- **DOM.** `index.html` has `#objective .title` ("Save the Greenhouse"), `#obj-line` and `#banner .big/.small`. The banner has no button. `chem.rs` tests run in CI with `rustc --edition 2021 --test src/chem.rs`.
- **B8 today:** nightly 1 measured about 1 minute of play. The 9-year-old ran out of things to do after the single quest.

## Relied on from I-019 (its contract; not rebuilt here)

- The **`fire` command** (latched click), so every click on a lab target casts once.
- **`cues::flight_secs`**, so spells land within 0.25 s, and **`spawn_stream(material)`**, a visible Water or Fire stream from the hand. Lab casts get both for free, and this item adds no stream code.
- **`cues::bearing_deg(px, pz, yaw, tx, tz)`** and **`cues::off_screen(bearing, elev, hfov, vfov)`**.
- **`#firefinder`** (`.left` / `.right`, at least 56 px, slow pulse, `animation: none` under reduced motion, hidden while paused or under a banner).
- The **hint priority** (a visible `#banner` hides every hint, `#firefinder` and the nudge), and **Water as the default spell**.
- `#obj-line` showing `Fires to put out: N` plus `.fire-pip` icons in the fire quest, and the `firesLeft` field.
- Esc/P pause, and the `castingYouCanSee(...)` section of `golden.mjs`.
- **`I019_KEYS`** and the I-019 stub's edit to `golden.mjs` `keysOk`, which requires exactly `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS]` in order (I-019's last key is `fxWater`). This item appends `I008_KEYS` (Contract).
- I-019 moves the `Look` enum from `world.rs` to `cues.rs`. The rust lane rebases its `world.rs` edits on the merged file.

If I-019 merges without any of these, the planner refreshes this spec. Lanes do not rebuild them.

## Problem

The slice has one 1-to-5-minute quest. After the win it freezes: casting is blocked, nothing steps, and the only option is R. Nightly 1 measured about 1 minute for B8, and the 13-year-old's fun was 4. Meanwhile the lab bench, which kids can already reach by melting the ice wall, says "coming soon", which promises content that does not exist. The electrolysis prop shows a correct 2:1 that nobody can do anything with.

**Goal:** after saving the greenhouse, the player *chooses* to start a second quest. In it they melt their way into the lab, fill the water splitter with Water, watch hydrogen and oxygen bubble into the two tubes at 2:1, and test each gas with Fire: a squeaky pop for hydrogen, a brighter flame for oxygen. Finishing it is counted as `quest_complete` with quest 1.

## Bars moved and expected movement

- **B8 (enough to play):** the intended movement. It is a panel bar, so nothing is reported as moved at merge.
  - Prediction for the first paired panel run (same seed, I-019 merge artifact vs this one): the 9-year-old's measured play time rises by **at least 2 minutes**, with Q2 taking about 2 to 4 minutes: walk to the alcove, melt the wall if it is still standing, fill, watch about 5 s of bubbles until both tubes are half full, test both tubes.
  - B8 **stays red**: it needs 15 minutes, 3 quests, 6 spells, free play and a skill tree. This item takes it from 1 quest to 2, and the follow-up rows carry the rest.
- **B5 (fun):** secondary. Prediction: median fun rises by at least 0.5. No persona reports "nothing to do after the greenhouse" or "the lab says coming soon". The pop and flare are the new "wow" moments, sized for under-6s (GS-17).
- **B7 (analytics):** `analytics-e2e` now proves a real `quest_complete` with quest 1, from a scripted run, through to D1 and `/api/stats` (test 17).
- **B1 (golden path):** B1 requires the smoke run to "complete every quest in the slice". This item adds Q2 to `golden.mjs` in the same PR (tests 10–16), so **B1 does not go red**.
- **B6 (science):** stays at 0 errors and 100% sourced. It adds 5 claim rows (C-027 to C-031) with builder-verified sources, re-pins P-015 to the live columns, retires C-026 ("coming soon") and adds not-claims rows.
- **B2, B3:** must not regress (tests 20 and 21). The item adds 2 pick targets, 4 per-frame transform updates, and no assets.
- **B4:** guard. Q1 is unchanged, and the Q2 objective line, the arrow and the slot nudge keep it legible.

## Smallest version

### 1. The lab state machine (`chem::lab`, pure, in `chem.rs`)

A new `pub mod lab` inside `chem.rs`, next to `pub mod electrolysis`, with no Bevy types. CI's existing `rustc --test src/chem.rs` step therefore covers it with no CI change.

- **Constants:**

  | constant | value |
  |----------|-------|
  | `H2_RATE_ML` | 2.0 (mL/s) |
  | `O2_RATE_ML` | `H2_RATE_ML / 2.0` |
  | `H2_CAP_ML` | 20.0 |
  | `O2_CAP_ML` | 10.0 |
  | `H2_TEST_ML` | 10.0 |
  | `O2_TEST_ML` | 5.0 |

  The test amounts are half of each tube, so step 3 starts 5 s after the fill with a clearly visible 0.3 m vs 0.15 m pair.

- `pub enum Part { Cell, H2Tube, O2Tube }`
- `pub enum LabEvent { Closed, Filled, AlreadyFull, GasReady, Popped, Flared, TooLittle, UseWater, UseFire, Done }`, with `fn caption(self) -> &'static str` (exact strings in the Contract).
- `#[derive(Clone, Debug, Default, PartialEq)] pub struct Lab { pub open: bool, pub level: f32, pub h2_ml: f32, pub o2_ml: f32, pub h2_tested: bool, pub o2_tested: bool, pub done: bool, pub ready_sent: bool }`. `Lab::default()` is closed. `Lab::opened()` is a default with `open = true`.
- **`apply(&mut self, part: Part, spell: Spell, ev: &mut Vec<LabEvent>)`:**
  - Closed lab: any part and spell gives `[Closed]`, with no state change.
  - `Cell`:
    - Water at `level == 0` sets `level = 1` and gives `[Filled]`.
    - Water at `level == 1` gives `[AlreadyFull]`.
    - Any other spell gives `[UseWater]`.
  - `H2Tube` / `O2Tube`:
    - Fire with gas ≥ the test amount empties that tube (`ml = 0`), sets its `*_tested`, and gives `[Popped]` (H2) or `[Flared]` (O2). If both are now tested and `!done`, it also sets `done = true` and appends `Done`.
    - Fire below the test amount gives `[TooLittle]`.
    - Water, Frost or Foam gives `[UseFire]`.
    - After `done`, the pure model still pops and flares, and `Done` is never emitted again. (In the game no cast reaches the lab after `done`, because a quest 2 win ends the game; see section 4.)
- **`step(&mut self, ev: &mut Vec<LabEvent>)`**, one `chem::DT`. While `open && level > 0`:
  - `h2_ml = (h2_ml + H2_RATE_ML·DT).min(H2_CAP_ML)` and `o2_ml = (o2_ml + O2_RATE_ML·DT).min(O2_CAP_ML)`.
  - From empty tubes this keeps `o2 = h2/2` on every tick (2H2O → 2H2 + O2, with equal volumes for equal moles), and both tubes fill in the same 10 s.
  - The first tick on which `h2_ml ≥ H2_TEST_ML`, it emits `GasReady` once (`ready_sent`).
- **`set_gas(&mut self, h2_ml: f32)`** sets `h2_ml = h2_ml.clamp(0, H2_CAP_ML)` and `o2_ml = h2_ml / 2`, and leaves `ready_sent` alone. It backs the debug `lab gas` command, so e2e tests set exact amounts instead of counting real-time ticks.
  - Electrolysis keeps running, so a tested tube refills.
- **`gas_heights(&self) -> (f32, f32)`** = `(electrolysis::GAS_H2 · h2_ml / H2_CAP_ML, electrolysis::GAS_O2 · o2_ml / O2_CAP_ML)`. These are the drawn column heights, in metres. They are 2:1 whenever both tubes filled from empty.
- **`step_index(&self, wall_standing: bool, in_lab: bool) -> u8`:**

  | step | when | meaning |
  |------|------|---------|
  | 0 | `level == 0 && wall_standing && !in_lab` | reach the lab |
  | 1 | `level == 0` (wall melted or `in_lab`) | fill the cell |
  | 2 | `level > 0 && !done` and some untested tube is below its test amount | wait for gas |
  | 3 | `level > 0 && !done` and every untested tube is at or above its test amount | test with Fire |
  | 4 | `done` | done |

- `in_lab` is `x ≥ 18.1 && 3.0 ≤ z ≤ 9.0`, defined once as `pub fn in_lab(x: f32, z: f32) -> bool` in `chem::lab`.

### 2. A calm courtyard for quest 2 (`chem::World`)

- New field `pub calm: bool` (`Default` false, `World::new` false).
- While `calm`:
  - `outcome()` returns `Playing`;
  - `latch` latches nothing;
  - `reseed` returns at once.
  - Spread, melting, freezing and every spell rule are unchanged. A fire the player lights can still spread, but it can no longer win or lose anything.
- `pub fn calmed(&self) -> World`: a clone with `calm = true` and `ended = None`. Every burning object gets `burning = false`, `temp = AMBIENT_C` and `counts = false`. Nothing else changes, so the ice wall, puddles and fuel keep their state.
- `apply_inner`'s `LabBench` branch returns with **no** event, because `main.rs` routes lab hits to `chem::lab` (section 4). `EventKind::Station` and its "coming soon" caption are **deleted**. The one existing test that names `Station` (`hit_kind(&ev(&[EventKind::Station]))`, line 1095) switches to `EventKind::Soaked`, with the same expected `HitKind::Hit`. That is the only edit to an existing test.

### 3. The tubes become live, and two tubes become targets (`world.rs`)

- `electrolysis::tube(dz, gas)` must accept `gas = 0.0`: `gas.h == 0`, and the water fills the inner tube. `tubes()` is unchanged, as the full 2:1 reference for P-015.
- The electrolysis block keeps its single `for t in chem::electrolysis::tubes()` loop and its I-018 comment. Its water and gas pieces get a component `TubePiece { tube: 0|1, gas: bool }` (tube 0 = H2 on the `CATHODE_DZ` side, tube 1 = O2) so that `main.rs` can update them. The glass stays static. I-018's test 5 greps must still pass. Any new number (pick boxes) lives in `chem::electrolysis` as a named constant, not in the block.
- **Pick targets** (appended **after** `"lab bench"`, so every existing object index is unchanged):
  - `"lab bench"` keeps its name and `Kind::LabBench`. Its pick box becomes `CELL_PICK_C = (0.0, 0.85, 0.0)` offset from the bench, with half extents `CELL_PICK_H = (0.75, 0.65, 0.45)`: y 0.2 to 1.5, covering the bench body under the cell and the battery between the tubes. It is tall so the first lab target is easy to hit (GS-10, GS-11). It no longer encloses the tubes.
  - `"hydrogen tube"` and `"oxygen tube"`: `Kind::LabBench`, `Obj::new(Kind::LabBench, [21.8, 6.0 + dz], [0.3, 0.3])`. Their pick boxes are centred at `(21.8, TUBE_Y, 6.0 + dz)` with half extents `TUBE_PICK_H = (0.3, TUBE_H/2, 0.3)`, where `dz` is `CATHODE_DZ` or `ANODE_DZ`. They have no collider of their own, because the bench block already blocks the player.
  - No pick box overlaps another: the cell spans z 5.55–6.45, and the tubes span 4.8–5.4 and 6.6–7.2 (a 0.15 m gap on each side).
- The ice wall is found as **the object with `Kind::IceWall`** (`world.rs` line 81; there is exactly one). Both `goalObj` and `wall_standing` (its `wall` flag) use that lookup, never a hard-coded index.
- `Chem` gains `pub lab: chem::lab::Lab` (`Default`: closed).

### 4. Quest flow (`main.rs`)

- `Game` gains `quest: u8` (0 = Save the Greenhouse, 1 = Split the Water). `reset` carries it over, the way it carries `spell`.
- **Choosing quest 2 (GS-28: no auto-continue):** the command `quest next` (not debug-gated) acts only when `quest == 0 && chem.world.outcome() == Won`. Otherwise it is ignored with a `warn!`. It does all of the following:
  - sets `quest = 1`;
  - replaces `chem.world` with `chem.world.calmed()`, keeping the courtyard the player saved;
  - sets `chem.lab = Lab::opened()`;
  - leaves the player where they are;
  - says `"Quest 2: Split the Water! The lab is behind the ice wall, to the east."`
- **R / `reset`** restarts the *active* quest:
  - Quest 0: exactly as today.
  - Quest 1: `chem.world = chem.initial.calmed()` (the ice wall stands again), `chem.lab = Lab::opened()`, the player at (8.0, 6.0) facing +x (`yaw = −π/2`), a new epoch, and the quest 2 start caption.
  - **`reset all`** (new) is a fresh game at quest 0, the same as today's `reset` with `quest = 0` and `lab = Lab::default()`.
  - Existing callers of `reset` all run in quest 0, so their behaviour does not change.
- **Routing hits:** in the projectile impact path, an impact on an object of kind `LabBench` calls `chem.lab.apply(part, spell, &mut lev)` and **not** `world.apply`. `part` comes from the target name: `"lab bench"` is `Cell`, `"hydrogen tube"` is `H2Tube`, `"oxygen tube"` is `O2Tube`.
  - The last lab event's caption is said.
  - `hit_kind` for the hit marker and sound: `Popped`, `Flared` and `Done` give `out` (I-007's star and cheer, the "it worked" reaction). `Filled` and `GasReady` give `hit`. All others give `fizzle`.
- **Quest end gating (one rule for both quests).** Define `fn quest_outcome(&self) -> Outcome` in `main.rs`: quest 0 gives `chem.world.outcome()`, and quest 1 gives `Won` iff `chem.lab.done`, else `Playing`. Every place that today checks `chem.world.outcome() != Playing` to stop the game uses `quest_outcome()` instead: `chem_tick`, the `tick` command and `aim_and_cast` (`let ended = ...`). So after the quest 2 win, exactly as after the quest 0 win, casting is blocked, the courtyard and the lab stop stepping, and the only things left are to look around and press R. Pause therefore never has a live world behind a banner (GS-7).
- **Stepping:** `chem_tick` also calls `chem.lab.step` when `quest == 1`, not paused, and `quest_outcome() == Playing`. The `tick N` command steps the lab N times too under the same condition. That is the existing loop, plus the lab.
- **`lab gas <h2_ml>`** (debug-gated, like `tp`): in quest 1 calls `chem.lab.set_gas(h2_ml)`; otherwise it is ignored with a `warn!`.
- **Effects** (in `react`, from lab events; existing `P` kinds only, no `fx.rs` change):
  - `Filled`: `P::Water` burst, 20 particles, at the bench top.
  - `Popped`: `P::Fire` 18 plus `P::Spark` 10, at the H2 tube top.
  - `Flared`: `P::Fire` 28 at the O2 tube top, size at least 0.35.
  - While gas forms: one `P::Steam` bubble per tube every 0.25 s game time, rising inside the tube (8 particles/s at most, within I-019's `?low` pool budget).
  - All of these stay within GS-6 (test 19).
- **Live columns:** a system (in `sync_visuals` or one next to it) sets each `TubePiece` transform from `electrolysis::tube(dz, h)`, where `h` comes from `chem.lab.gas_heights()`. Water pieces are `Visibility::Hidden` while `lab.level == 0`. A gas piece is hidden while its height is below 0.001.
- **Published `gasH` is what is drawn:** the same system records, per tube, the gas height it actually applied (the gas piece's `Transform` y scale times the unit cylinder height, or 0 while the piece is hidden) into a `Resource` `DrawnGas([f32; 2])`, and the publisher reads `gasH` from that resource, not from `gas_heights()`. A plain state check on `gasH` therefore proves the render link.
- **Strings live in `main.rs`.** `questTitle`, `questLine` and every caption are produced by the rust lane and published. The web lane only displays them and never hard-codes a step line.
- **Greenhouse line:** the `"Oh no! Heat from the fire reached the greenhouse. Press R to try again."` caption is said only in quest 0. In a calm world the generic neighbour line is used.
- **The quest 2 objective target (reusing I-019's finder maths):** every frame, `goalObj` is:
  - step 0: the index of the `Kind::IceWall` object;
  - step 1: the lab bench;
  - step 2: −1 (watch, no arrow);
  - step 3: the first untested tube (H2 before O2);
  - step 4, and in quest 0: −1.

  `goalBearing`, `goalElev` and `goalOff` come from `cues::bearing_deg` and `cues::off_screen` against that object's bounds centre, with the live hfov and aspect, exactly as I-019 computes `fireBearing`.

### 5. Published state

`outcome` becomes **per active quest**, from `quest_outcome()`:
- quest 0: from the world, as today;
- quest 1: `"won"` iff `lab.done`, otherwise `"playing"`.

Because a won quest stops the game (section 4), `outcome !== 'playing'` keeps meaning "a banner is up and nothing is live" in both quests. `hud.js`'s existing `outcome` gates (`syncLock`, warning, hints, I-019's banner suppression) stay correct with no new flag.

`quest` is published, so `analytics.js` sends `quest_complete` with quest 1 unchanged. See the Contract for every new field.

### 6. HUD (`hud.js`, `style.css`; all new DOM via `createElement`)

- **`#objective .title`** is set from `state.questTitle`.
- **`#obj-line`:**
  - quest 0: I-019's counter and pips, unchanged;
  - quest 1: `state.questLine` as text, preceded by one spell icon (`.obj-icon.fire` or `.obj-icon.water`, reusing the hotbar icon classes) when `questSpell ≥ 0`. The icon is the pre-reader cue (GS-21, GS-32).
- **Win banner, quest 0:**
  - `.big` stays `You saved the greenhouse!`.
  - `.small` becomes `${extinguished} fires put out. Next: the water lab. Press Enter, or R to play again.`
  - A new button `#banner-next` reads `Next quest` with a flask icon. It is at least 40 px tall (GS-22), its click calls `stopPropagation` and `preventDefault`, and it sends `quest next`.
  - **Pointer lock is released for the button (GS-11).** When this banner first shows with `#banner-next` and `document.pointerLockElement` is set, `hud.js` calls `document.exitPointerLock()` once. `syncLock` already skips the pause overlay while `outcome !== 'playing'`, so this does not pause. A mouse-only child sees the cursor and clicks the button. Re-locking happens on the next canvas click, as today. A click on `#banner-next` never reaches the canvas, so it never casts and never re-locks.
  - **Enter** (`e.code === 'Enter'`) sends `quest next` while this banner shows, locked or not (GS-13). It is a keyboard shortcut, not the main route. Nothing advances by itself.
- **Win banner, quest 1:**
  - `.big` reads `You split water!` and `.small` reads `Hydrogen went pop. Oxygen made the flame brighter. Press R to do it again.`
  - There is **no** `#banner-next` (it is hidden), and Enter does nothing. The game is ended exactly as after quest 0 (section 4). Free play is a separate backlog row.
- **Greenhouse warning:** `#warning` is shown only when `state.quest === 0` (plus today's `outcome === 'playing' && ghTemp > 60`). In calm quest 2 nothing can happen to the greenhouse, so the pulsing warning would mislead (GS-6).
- **Arrow:** in quest 1, `#firefinder` ignores `fireOff`, `nearFire` and `firesLeft` and is driven only by `goalObj`/`goalOff`. It is shown iff `goalOff && goalObj ≥ 0 && outcome === 'playing' && !paused`, with the class `lab` (a flask icon in place of the flame) and the side class from the sign of `goalBearing`. A bush the player lit in quest 2 never drives it. I-019's banner suppression still applies. In quest 0, I-019's rule is unchanged.
- **Text size (GS-18):** `#banner-next`, `#banner .small`, the quest 1 `#obj-line` text and `#objective .title` all compute to `font-size ≥ 18px`. The pre-existing 15 px quest 0 `#obj-line` belongs to another item; the quest 1 line must not use it.
- **Idle hint (GS-24):** in quest 1, `updateIdle` uses a per-step hint and slot:
  - step 0: `Melt the ice wall with Fire. Press 1, aim, click.` and slot 0;
  - step 1: `Press 2 for Water and click the lab bench.` and slot 1;
  - step 2: no idle hint (the bubbles are progress);
  - step 3: `Press 1 for Fire and click a gas tube.` and slot 0.

  A change in `questStep` or `lab.h2`/`lab.o2` counts as progress and restarts the idle clock. Quest 0 keeps `IDLE_HINT` and `IDLE_SLOT` as today.

## Acceptance tests

**Unit tests** go in `chem.rs`'s existing `mod tests` and run with `rustc --edition 2021 --test crates/bevy-client/src/chem.rs -o /tmp/chem && /tmp/chem` (CI's command). Tests 1 to 8 fail on the base because `chem::lab`, `World::calm` and `calmed` do not exist.

1. **`lab_closed`.** `Lab::default()`: every `Part` × every `Spell` gives exactly `[Closed]`, and the `Lab` equals its value before the cast.
2. **`lab_fill`.** With `Lab::opened()`:
   - Fire, Frost or Foam on `Cell` gives `[UseWater]` with `level == 0`.
   - Water gives `[Filled]` with `level == 1`.
   - A second Water gives `[AlreadyFull]` with `level == 1`.
3. **`lab_gas_two_to_one`.** Opened and filled, then step 600 times:
   - On every tick, `|o2_ml − h2_ml/2| < 1e-4`, and while `h2_ml > 0`, `|gas_heights().0 / gas_heights().1 − 2| < 1e-3`.
   - After 150 ticks, `h2_ml` is 10.0 ± 0.1.
   - After 600 ticks, `h2_ml == 20.0` and `o2_ml == 10.0` exactly (the `min` clamp).
   - `GasReady` is emitted exactly once, on a tick in 149..=151 (10 mL at 2 mL/s is 150 ticks at 30 Hz; the window allows for f32 accumulation). This window is checked only here, never in e2e.
   - `set_gas(4.0)` gives `h2_ml == 4.0`, `o2_ml == 2.0`; `set_gas(99.0)` gives 20.0 and 10.0.
4. **`lab_no_gas_without_water`.** Opened and not filled: 300 steps leave `h2_ml == 0`, `o2_ml == 0` and no events.
5. **`lab_tests`.** Opened, filled, then set `h2_ml = 9.9`:
   - Fire on `H2Tube` gives `[TooLittle]` and the gas is unchanged.
   - With `h2_ml = 10.0`, Fire gives `[Popped]`, `h2_ml == 0` and `h2_tested`.
   - Water, Frost or Foam on either tube gives `[UseFire]`.
   - With `o2_ml = 5.0`, Fire on `O2Tube` gives `[Flared, Done]` and `done`.
   - Refill and pop H2 again: `[Popped]` only, with no second `Done`.
6. **`lab_either_order`.** Oxygen tested first, then hydrogen: `Done` exactly once, on the second test.
7. **`lab_step_index`.** Each of these is checked:
   - (wall standing, outside the lab) gives 0;
   - wall melted gives 1;
   - `in_lab` with the wall standing gives 1;
   - filled gives 2;
   - `h2 = 10, o2 = 5` gives 3;
   - `h2_tested` with `o2 = 1.0` gives 2;
   - `done` gives 4.

   Also `in_lab`: (18.0, 6) false, (18.2, 6) true, (20, 2.9) false, (20, 9.1) false.
8. **`world_calmed`.** Build the test courtyard used by `courtyard_pacing` with its 3 starting fires, then `calmed()`:
   - `burning() == 0`, `outcome() == Playing` and `ended == None`.
   - Step 3000 ticks: no `Reseeded` event, and `outcome()` is still `Playing`.
   - Then ignite the greenhouse with `ignite(gh)` and step once: `outcome()` is still `Playing`.
   - On an uncalmed copy, the same ignite gives `Lost` (guard).
9. **`electrolysis_tube_empty_and_partial`.** `tube(dz, 0.0)`: `gas.h == 0`, and `water.h == TUBE_H − 2·INSET` within 1e-5. For `g` in {0.1, 0.3, 0.6}: `water.h + gas.h == TUBE_H − 2·INSET` within 1e-5, and the gas top equals the inner top. I-018's tests 1–4 still pass unchanged.
10. **`apply_ignores_lab_bench`.** `World::apply` on a `LabBench` object, with every spell, pushes no event and leaves the object unchanged. `EventKind` has no `Station` variant: the string `coming soon` is absent from `chem.rs` (`grep -c 'coming soon' crates/bevy-client/src/chem.rs` gives 0).

**e2e checks** are new `check(...)` calls in `tests/e2e/golden.mjs`. They go in a function `labQuest(...)` called **after** I-019's `castingYouCanSee(...)` and before `no console errors`, so that check covers them too. They run as `QUERY=?low node tests/e2e/golden.mjs`, in isolation (I-017). The code uses golden's existing `castAt`, `until` and `frames` helpers. Objects are looked up by `objs[].n`. Existing, I-007 and I-019 checks are not edited; the line-264 bench check still passes because the `Closed` caption contains "splitting". The section starts with `reset all`.

11. **Quest 2 is offered, not forced (GS-28).**
    - Win Q1 with three Water `castAt`s on `lit`.
    - Within 10 frames:
      - `outcome === 'won'` and `quest === 0`;
      - `#banner-next` is visible, its box is at least 40 px tall, and its text contains `Next quest`;
      - `#banner .small` contains `Enter`;
      - computed `font-size ≥ 18px` for `#banner-next` and `#banner .small` (GS-18).
    - Wait 300 frames: `quest` is still 0 and the banner still shows.
    - On the base, `#banner-next` does not exist.
12. **Enter starts quest 2.**
    - `page.keyboard.press('Enter')`. Within 10 frames:
      - `quest === 1`, `outcome === 'playing'`, `questTitle === 'Split the Water'`;
      - `#objective .title` reads `Split the Water`;
      - `lab.open === true`;
      - the banner is hidden;
      - `caption` starts with `Quest 2: Split the Water!`;
      - computed `font-size ≥ 18px` for `#objective .title` and `#obj-line` (GS-18).
    - Then `reset`: `quest` is still 1, `pos` is (8, 0, 6) ± 0.1, and the `Kind::IceWall` object has `wall === true`.
    - Then `reset all`: `quest === 0`, `burning === 3`, `lab.open === false`.
    - **Mouse route under lock (GS-11).** Win Q1 again while pointer-locked (lock with a canvas click first; if the headless run cannot hold a real lock, the check records `pointerLockElement` and asserts the code path by spying on `document.exitPointerLock`). Within 10 frames of the win: `exitPointerLock` was called once (or `document.pointerLockElement === null`). Record `c0 = casts`, then **click** `#banner-next` with `page.click`: `quest === 1`, `casts === c0`, and `document.pointerLockElement === null` (the click did not re-lock).
    - **Enter under lock.** `reset all`, win Q1 again while locked (or with the lock spy in place), record `c0 = casts`, press Enter: `quest === 1` and `casts === c0`.
13. **Step 0 to 1, and the arrow points to the wall.**
    - In quest 1 from `reset` (wall standing), `tp 0 6`, then `look 90 0` (facing −x, away from the alcove). Within 5 frames:
      - `questStep === 0`, `goalObj` is the ice wall's index, `goalOff === true`;
      - `#firefinder` is visible with the class `lab`, and with `right` iff `goalBearing > 0`.
    - `castAt(iceWall, 1)` three times, then `until(questStep === 1)`. `questLine` is `Fill the water splitter: cast Water at it.`, `questSpell === 1`, and `#obj-line .obj-icon.water` exists.
14. **Fill, then gas at 2:1 that you can see.** (The lab steps in real time at 30 Hz, so this test asserts deltas and ratios, never absolutes that depend on frame counts.)
    - **Assisted near miss (GS-10).** `tp 20 6`, `face` the bench, then turn the yaw 2.5° off the cell's centre (`look` by the bench bearing ± 2.5) and cast Water with assist on (the default). Within 12 frames:
      - `lab.level === 1`, `questStep === 2`, `hitKind === 'hit'`, and the caption is the `Filled` caption;
      - `streams ≥ 1` was seen between the cast and the impact (I-019);
      - the caption contains `Na2SO4` or `Na₂SO₄`.
    - Read `h0 = lab.h2`, then `tick 150`, and read one state frame. In that frame:
      - `lab.h2 − h0` is 10.0 ± 0.1 (unless `lab.h2` is 20, the clamp);
      - `|lab.o2 − lab.h2 / 2| ≤ 0.1`;
      - `gasH[0] / gasH[1]` is 2 ± 0.01, and `gasH[0]` is `0.6 · lab.h2 / 20` ± 0.01 (published `gasH` is the drawn transform, section 4, so this is the render check);
      - `until(questStep === 3)` within 30 frames, and `questLine` starts with `Test each gas with Fire`.
    - **Screenshot (non-gating):** from `tp 19.6 6` facing the H2 tube, after `__game.pause(true)`, save one screenshot to the run's artifacts and log the path. It is not a check.
15. **Test both gases; quest 2 completes, and the game ends cleanly.**
    - `lab gas 12` (exactly 12 mL H2 and 6 mL O2, both above the test amounts). `castAt(h2Tube, 1)`. Within 12 frames:
      - the caption matches `/squeaky pop/i`, `lab.h2Tested`, `lab.h2 < 1`, `hitKind === 'out'`;
      - `#hitmarker` shows the star (I-007).
    - `lab gas 12` again (so the O2 amount is exact), then `castAt(o2Tube, 1)`. Within 12 frames:
      - `lab.done`, `outcome === 'won'`, `quest === 1`, `questStep === 4`;
      - `#banner .big` reads `You split water!`, and `#banner-next` is hidden.
    - **Ended, like quest 0 (GS-7).** Record `h = lab.h2`, `t = gameTime`, `c = casts`. Over 30 frames: `lab.h2 === h`. Press Enter and cast Fire at the H2 tube: `quest === 1`, `outcome === 'won'`, `lab.h2Tested` unchanged, and no new lab caption (casting is blocked). Then press R: `outcome === 'playing'`, `questStep === 0` and the banner is hidden.
16. **Wrong spells explain and do not progress.**
    - From quest 1 `reset`, `tp 20 6` with the wall melted by `castAt` × 3. Then:
      - `castAt(bench, 1)` gives a caption of `Use Water on the cell.` and `lab.level === 0`;
      - `castAt(h2Tube, 2)` gives `Test a gas tube with Fire.`;
      - after filling, `lab gas 4` (exactly 4 mL H2, well under 10), then `castAt(h2Tube, 1)` within the same few frames gives `Not enough gas yet. Watch the bubbles.` with `h2Tested === false`.
    - Each of these has `hitKind === 'fizzle'`.
16b. **No greenhouse warning in calm quest 2 (S3).** In quest 1, cast Fire at the bush nearest the greenhouse and at the greenhouse until `ghTemp > 60` (or 900 frames). Assert `ghTemp > 60` was reached (the check fails, not skips, if it never is), and on every frame where it held, `#warning` is hidden and `outcome === 'playing'`.
17. **Analytics counts quest 1** (`tests/e2e/analytics-e2e.mjs`, main run).
    - After `Save the Greenhouse is won`, press Enter. Then use the same `tp`/`face`/`select`/`cast` pattern the run uses for quest 0:
      - melt the wall;
      - Water on the bench;
      - `lab gas 12` before each Fire test (the run already loads `?debug`);
      - Fire on each tube.
    - Wait for `outcome === 'won' && quest === 1`.
    - Checks:
      - the run's captured events include `quest_complete` with `quest: 1`, exactly once;
      - **edited check:** "D1 main: exactly 2 quest_complete (seedling, rooms 0 and 1)";
      - **new check:** `stats.funnel.quests[1].seedling === 1`.
    - The two existing quest-0 checks are replaced only by these, and every other check is untouched.
    - On the base, quest 1 is never sent.
18. **Science lint.** `npm test` passes:
    - rows C-027 to C-031 are present (Contract);
    - row C-026 is removed (its id is not reused);
    - row P-015's note describes the live columns and names `lab_gas_two_to_one`;
    - every new HUD or caption string the lint flags has a claim or not-claims row.
19. **No flashing from pop and flare (GS-6).** Use I-019 test 19's in-Node PNG method. Take 20 consecutive-frame screenshots through a `Popped` and 20 through a `Flared`, from 2 m. The fraction of pixels whose relative luminance changes by 10% or more stays under 20% of the frame for every pair.
20. **B2 guard (tester, no script change).**
    - Base is the I-019 merge commit, built into `<scratchpad>/I-008/base-pkg-out`.
    - Run `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs`, 3 interleaved pairs. The median CPU-frame p50 must be ≤ base + 1.0 ms.
    - Also run one pair standing in the lab during step 2 (bubbles plus live columns) with the same limit, measured on the same machine.
21. **B3 guard.** Brotli of `pkg-out` grows by at most 15 KB. No new binary assets.
22. **Golden still green.** All existing, I-007, I-019 and new checks pass 3/3 with `QUERY=?low`, with no threshold edits. The `clock and budget` section's `keysOk` passes with the Contract's `I008_KEYS` order (its one edit is the contract stub's, below).

## GS walk (screens, controls and effects this item touches)

| GS | P | status |
|----|---|--------|
| 6 | P0 | met for pop, flare and bubbles (test 19). The finder pulse is I-019's. The greenhouse warning pulse is off in calm quest 2 (test 16b) |
| 7 | P1 | met: quest 2 starts only on a choice, and each quest end is a clean stopping point. A won quest stops the world and the lab in both quests (`quest_outcome`), so no live world sits behind an unpausable banner (test 15) |
| 10 | P0 | met: the cell pick box is tall (y 0.2–1.5), and an assisted 2.5° near miss still fills (test 14) |
| 11 | P0 | met: the whole quest needs only mouse look and click plus one movement key (or point mode). The banner releases pointer lock so `#banner-next` is clickable with a mouse, and the click never casts (test 12). Enter is an optional shortcut |
| 12 | P0 | kept via I-019 (latched click, flight ≤ 0.25 s) |
| 13 | P1 | met for the new UI: Enter drives `#banner-next`, which is a focusable `<button>` |
| 15 | P0 | met: each lab result has a caption, a particle burst, I-007's marker shape (star for success, slash for fizzle) and a sound |
| 17 | P0 | met: the pop and flare are large bursts with the star and cheer. The tubes visibly fill. The quest end is a big banner |
| 18 | P0 | met for the new text: `#banner-next`, `#banner .small`, `#objective .title` and the quest 1 `#obj-line` compute to ≥ 18 px (tests 11, 12). The pre-existing 15 px quest 0 `#obj-line` is another item's fix |
| 19 | P0 | met: the gas tint differs from the water by luminance, not only hue (I-018). The arrow `.lab` uses a flask *shape*. The objective icon is a shape |
| 21 | P0 | met: new lines are short; the longest caption (`Filled`) is split down to 14 words with one formula. Each step has an icon (objective icon, slot nudge, arrow). Captions do not auto-dismiss faster than today |
| 22 | P1 | met: `#banner-next` is at least 40 px tall |
| 24 | P0 | **met**: a specific step line with numbers ("hydrogen 10 mL, oxygen 5 mL"), an arrow to the next object, and a per-step idle hint plus slot nudge |
| 25 | P1 | met: quest 2 has no timer and no fail |
| 27 | P0 | met: lab glassware only, and the tone is not babyish |
| 28 | P0 | **met**: no auto-continue (test 11 waits 300 frames). No lure copy |
| 30 | P0 | **met**: the player must pick Water to fill and Fire to test. Other spells explain and do nothing (test 16). The 2:1 columns and the pop-vs-flare result *are* the lesson |
| 31 | P1 | **partial, deferred**: meaningful (water, fire) and discussable (2:1, pop test), but both spells are named in the step line and the hint, so there is no prediction. A prediction prompt belongs with the hydrogen/oxygen spells follow-up; no quiz is bolted on here |
| 32 | P1 | met: H2/O2 numbers sit next to visibly growing columns |
| 23 | P0 | unchanged: the safe first minute is I-020's |
| 2, 8, 9, 14, 20, 26 | P1 | unchanged (no new settings or controls beyond Enter) |

## Out of scope

- **Follow-up rows the planner files in the backlog** (the rest of r2, re-scoped against the merged client):
  - **"Spells 5 and 6 (Hydrogen, Oxygen) unlocked by Split the Water, plus a 6-slot hotbar with locked slots"** (B8, B5). Hydrogen on a flame pops; Oxygen makes a burning fuel flare. Reuse r2's Contract rows for `Spell::Hydrogen`/`Oxygen` and claims C-I008-08/09/11 (H2 about 14× lighter than air, O2 an oxidiser, H2 the lightest gas), and give them a quest use.
  - **"Quest 3, The Frozen Pond: liquid-nitrogen bridge puzzle with a hot spring"** (B8, B5). It needs the south zone (walls ≥ 3.0 m, divider, gates) and r2's pond, spring and step tables. **Science that must stay correct:**
    - LN2 boils at 77 K (−196 °C), per [S1].
    - Where the game shows LN2 meeting warmer water or ground, the **Leidenfrost effect** applies: a vapour layer forms under the liquid nitrogen and insulates it, so droplets skate and the first contact transfers heat slowly. Any "freezes instantly" outcome must be registered as `simplified` (as C-016 is), never as `correct`.
    - The 80 °C spring is a game value, checked for plausibility.
  - **"Free-play yard with reset lever"** (B8, B5) and **"Skill-tree screen with three upgrades and science points"** (B8, B5), from r2 sections 6 and 7.
  - **"Smoke and panel walk quests 3+"**: each content item adds its own golden section, as this one does.
  - A one-line docs follow-up: `crates/bevy-client/README.md` controls (Enter for the next quest, `reset all`).
- Saving progress across reloads, so quest 2 must be reached again after a reload.
- **Free play after the quest 2 win** ("Keep exploring" with a live lab). A won quest 2 ends the game like quest 0; free experimenting belongs to the free-play yard row.
- **A HUD route back to quest 0.** `reset all` is a command only; nothing in the HUD sends it, so a child returns to quest 0 by reloading. A menu button is not expected here; the README follow-up documents the command.
- Polarity marks, wires or a switch on the battery. The circuit closes when the cell is filled, which is true of the prop as drawn. The prop stays unlabeled, as I-018 left it.
- Modelling the water used up: 20 mL of H2 at room conditions comes from about 0.015 mL of water, so the cell level is not changed.
- A greenhouse that catches fire in the calm quest-2 courtyard (from a fire the player lit) has no consequence. That is acceptable for this step, and the panel brief notes it.
- New sounds (I-007's set covers it), new art (I-009), and age-band caption variants (I-021).
- `analytics.js`, `functions/**`, `dashboard/**`, `migrations/**`, `index.html` (all new DOM is built by `hud.js`), the panel harness (`tests/e2e/panel/**`) and `.github/**`.
- **Panel brief note:** the panel brief must say the slice now has two quests, and that quest 2 starts from the win banner ("Next quest" button, or Enter).

## Contract

A stub, landed by the contract builder on the branch base (after I-019 merges) before the lanes start. It has no gameplay behaviour. Both lanes code against it.

### Rust (stub signatures with neutral bodies; no `todo!()`)

- `chem::lab`: `Part`, `LabEvent`, `Lab` (fields as in section 1), `Lab::opened()`, `apply`, `step`, `set_gas`, `gas_heights`, `step_index`, `in_lab`, the six constants, and `LabEvent::caption` returning the exact strings below.
- `chem::World::calm: bool` and `World::calmed(&self) -> World`.
- `chem::electrolysis::{CELL_PICK_C, CELL_PICK_H, TUBE_PICK_H}` with the section 3 values.
- `world::Chem::lab: chem::lab::Lab`, and `world::TubePiece { tube: u8, gas: bool }` (a `Component`).
- `Game::quest: u8`, `quest_outcome()` (stub: returns `chem.world.outcome()`), and the `DrawnGas([f32; 2])` resource. The commands `quest next`, `reset all` and `lab gas <h2_ml>` (debug-gated) are parsed. In the stub, `quest next` and `lab gas` are no-ops with a `warn!`, and `reset all` behaves like today's `reset`.
- The publisher emits every `I008_KEYS` field (below) with its neutral value, in order.

### `golden.mjs` key order (the stub's one edit to an existing check)

The contract stub changes `keysOk` in `golden.mjs`'s `clock and budget` section from `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS]` to `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS, ...I008_KEYS]`, with

```js
const I008_KEYS = ['quest', 'questTitle', 'questStep', 'questLine', 'questSpell',
                   'lab', 'gasH', 'goalObj', 'goalBearing', 'goalElev', 'goalOff'];
```

appended after I-019's last key (`fxWater`). `outcome` stays in its base position. `lab` is a nested object with the key order `open, level, h2, o2, h2Tested, o2Tested, done`. This is the only edit to an existing check, made once by the stub; neither lane redoes it.

### `LabEvent` captions (exact; the science review checks these)

| event | caption | register |
|-------|---------|----------|
| `Closed` | `The water-splitting lab opens after you save the greenhouse.` | not-claims |
| `Filled` | `Water in! A little Na₂SO₄ lets electricity split it: 2H₂O → 2H₂ + O₂.` | C-027 |
| `AlreadyFull` | `The cell is already full of water.` | not-claims |
| `GasReady` | `Bubbles! Hydrogen fills one tube twice as fast as oxygen fills the other.` | C-028 |
| `Popped` | `Squeaky pop! Hydrogen burns with oxygen from the air: 2H₂ + O₂ → 2H₂O.` | C-029 |
| `Flared` | `The flame burns brighter: oxygen helps things burn, but it does not burn by itself.` | C-030 |
| `TooLittle` | `Not enough gas yet. Watch the bubbles.` | not-claims |
| `UseWater` | `Use Water on the cell.` | not-claims |
| `UseFire` | `Test a gas tube with Fire.` | not-claims |
| `Done` | `You split water into hydrogen and oxygen!` | C-031 |
| quest 2 start | `Quest 2: Split the Water! The lab is behind the ice wall, to the east.` | not-claims |

### Quest 2 step lines (`questLine`; exact, `{}` rounded to integers)

- 0: `Melt the ice wall (east) with Fire to reach the lab.`
- 1: `Fill the water splitter: cast Water at it.`
- 2: `Splitting water... hydrogen {} mL, oxygen {} mL`
- 3: `Test each gas with Fire: hydrogen {}, oxygen {}` (each `{}` is `✓` or `○`)
- 4: `Water split! Press R to do it again.`

All five are not-claims rows. The step 2 numbers are the live gas.

### Science rows (`content/science-sources.md`)

The builder opens every source and confirms it says what the row claims, or replaces it. New sources take the next free `[S<n>]` ids.

| id | claim (exact text above) | where | verdict | sources |
|----|--------------------------|-------|---------|---------|
| C-027 | `Filled` caption: Na2SO4 is an inert electrolyte that lets water conduct, and 2H2O → 2H2 + O2 under current. NaCl is deliberately not used, because chloride gives Cl2 at the anode | crates/bevy-client/src/chem.rs | correct | [S13] [S7], plus a source that names Na2SO4 (or another sulfate) as the electrolyte for water electrolysis, for example the RSC "Electrolysis of water" practical |
| C-028 | `GasReady` caption: H2 forms at twice the volume rate of O2 | crates/bevy-client/src/chem.rs | correct | [S13] [S12] |
| C-029 | `Popped` caption: a lit splint gives a squeaky pop with hydrogen (which burns with oxygen from the air mixed into the tube), and 2H2 + O2 → 2H2O | crates/bevy-client/src/chem.rs | correct | [S7], plus a new source for the hydrogen pop test that mentions the air/oxygen mixture (for example BBC Bitesize GCSE "Tests for gases" or the RSC) |
| C-030 | `Flared` caption: oxygen supports combustion (a flame burns brighter in it) but is not itself a fuel | crates/bevy-client/src/chem.rs | **simplified** | a new source (for example OpenStax *Chemistry 2e* § 18.10 on oxygen, or the oxygen test for gases). Note: the school test is a glowing splint that relights; the game uses a flame that burns brighter, which is true of flames in oxygen but is not the standard test |
| C-031 | `Done` caption: electrolysis splits water into hydrogen and oxygen | crates/bevy-client/src/chem.rs | correct | [S13] |

- **P-015:** the value (2) and verdict (`correct`) are unchanged. `where` stays `crates/bevy-client/src/chem.rs, crates/bevy-client/src/world.rs`. The note is rewritten to say: "the gas columns are drawn live from `chem::lab::gas_heights` (H2 rate = 2 × O2 rate, equal tubes), so from empty tubes the H2 column is twice the O2 column at every tick (test `lab_gas_two_to_one`). After one gas is tested, that tube refills from zero while the other keeps its gas, as in a real voltameter after venting one tube." Keep the Avogadro sentence.
- **C-026** is removed.

### New `__gameState` fields (existing fields keep their meaning, except `outcome`, below)

| field | type | stub (neutral) value | meaning |
|-------|------|----------------------|---------|
| `quest` | u8 | 0 | 0 = Save the Greenhouse, 1 = Split the Water |
| `questTitle` | string | `"Save the Greenhouse"` | the HUD title |
| `questStep` | u8 | 0 | quest 1: `lab.step_index(..)`. Quest 0: `min(extinguished, 3)` |
| `questLine` | string | `""` | quest 1 step line. `""` in quest 0 (the HUD keeps I-019's counter) |
| `questSpell` | i32 | −1 | 0-based spell for this step (quest 1: steps 0 and 3 give 0, step 1 gives 1), else −1 |
| `lab` | `{open, level, h2, o2, h2Tested, o2Tested, done}` | closed and zeros | `h2`/`o2` in mL, 1 dp |
| `gasH` | [f32; 2], 3 dp | [0, 0] | gas column heights in m as actually applied to the `TubePiece` transforms (`DrawnGas`), H2 then O2 |
| `goalObj` | i32 | −1 | the quest 1 object to act on (section 4) |
| `goalBearing`, `goalElev` | f32, 1 dp | 0 | as I-019's `fireBearing`/`fireElev`, for `goalObj` |
| `goalOff` | bool | false | as I-019's `fireOff`, for `goalObj` |

The table order is the `I008_KEYS` order. `outcome` is per active quest (section 5). In quest 0 it is identical to today.

### Commands (`__game.cmd`)

- `quest next`: starts quest 2, only from a quest 0 win. It is not debug-gated.
- `reset`: restarts the active quest.
- `reset all`: a fresh game at quest 0.
- `tick N`: also steps the lab in quest 1 (only while `quest_outcome() == Playing`).
- `lab gas <h2_ml>`: debug-gated. In quest 1 sets `h2_ml` (clamped to 0..20) and `o2_ml = h2_ml / 2`. Used by tests 15–17 so amounts are exact.

### DOM (web lane; listed so tests and the panel can rely on them)

`#banner-next` (a `<button>`, shown only on the quest 0 win banner), `#firefinder.lab`, and `#obj-line .obj-icon.fire|.water`. `#warning` is gated on `quest === 0`.

## Lanes

| lane | owns | builds |
|------|------|--------|
| rust | `crates/bevy-client/src/chem.rs` (`pub mod lab`, `World::calm`/`calmed`, `Station` removal, `apply_inner` LabBench branch, `electrolysis` constants and the `tube(.., 0)` support, unit tests 1–10, and the one `Station`→`Soaked` test edit); `crates/bevy-client/src/world.rs` (the two tube targets, the cell pick box, `TubePiece`, `Chem::lab`); `crates/bevy-client/src/main.rs` (quest flow, `quest_outcome` gating, routing, stepping, `tick`, `reset`/`reset all`/`quest next`/`lab gas`, `react` effects and captions, every published string, live columns and `DrawnGas`, `goal*` fields via the `Kind::IceWall` lookup, publish in `I008_KEYS` order) | Unit tests 1–10 first, then the rest. Must keep I-018's test 5 greps clean and every existing object index unchanged. Must not touch `cues.rs` (it uses I-019's functions as they are), `fx.rs`, `player.rs`, `bridge.rs`, `web/**`, `tests/**` or `content/**` |
| web | `crates/bevy-client/web/hud.js`, `crates/bevy-client/web/style.css`, `tests/e2e/golden.mjs` (`labQuest`, tests 11–16, 16b and 19; not the `keysOk` edit, which is the stub's), `tests/e2e/analytics-e2e.mjs` (test 17 only), `content/science-sources.md` (the rows in the Contract only) | Title from `questTitle`; quest 1 `#obj-line` with icon (text from `questLine`, never hard-coded); `#banner-next`, pointer-lock release and Enter on the quest 0 banner; the quest 1 banner without a button; `#warning` gated on `quest === 0`; `#firefinder.lab` driven only by `goal*` in quest 1; ≥ 18 px for the new text; per-step idle hint and slot; e2e tests written against the Contract fields; science rows with verified sources. Must not touch `index.html`, `boot.js`, `analytics.js`, `age.js` or the panel harness |

**Seam:** the web lane's tests 11–17 need the rust lane's fields. Each lane builds against the contract stub, and the shared build after both lanes merge runs the full golden and analytics-e2e (isolated). The seam breaker checks:
- `outcome`/`quest` transitions give exactly one `quest_complete` per quest: Q1 won, then `quest next`, then Q2 won. R and replays send nothing new.
- Enter on the quest 0 banner never also casts or locks.
- A mouse click on `#banner-next` never casts and never re-locks.
- After the quest 2 win, Esc/P and R behave as after the quest 0 win, and nothing (lab gas, `gameTime`-driven bubbles, casts) advances behind the banner.
- Published `gasH` equals the drawn `TubePiece` gas heights.
- No lab hit ever reaches `World::apply`.
