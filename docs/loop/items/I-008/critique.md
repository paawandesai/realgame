# Critique of I-008 r3 (Quest 2, "Split the Water")

**Critic:** fresh-context spec critic, 2026-10-30. Read: `spec.md` r3, `target.md`, `bars.md`, `areas.md` and `gameplay-standards.md` at origin/main 265a93e, and I-019's spec. Feasibility was checked against `chem.rs`, `world.rs`, `main.rs`, `hud.js` and `golden.mjs` at 265a93e.

## Verdict: REVISE (right thing, wrong in 5 places; no `wrong-thing`)

**It is the right thing.**
- The one-minute B8 problem is real. After the win, the game freezes and the lab says "coming soon".
- The cut from r2 is sound: one quest, existing `Target`, existing `quest_complete`, and a scripted 2:1 lesson with a pop/flare test.
- Science is the mechanic (GS-30): the player must choose Water to fill and Fire to test, and the 2:1 columns are the lesson.
- No auto-continue (GS-28). No timer or fail state (GS-25).
- Almost every fact checks out against 265a93e. Verified: `world.rs` 1084–1130, `Station` at `chem.rs` 134/245/440/1095, `reseed`, `Player::default` at (0,0,18), `Kind::LabBench`, and `objs[].wall`.

**It is not ready to dispatch.** Five items break a P0 or make a stated test impossible to pass. Each has a concrete fix below. The rest are should-fix or trims.

---

## Must-fix

### M1. The post-win quest-1 state breaks pause (GS-7) and hides every hint

**Where:** HUD section 6, "Keep exploring"; Published state section 5.

**Problem:** `hud.js` gates behaviour on `state.outcome`.
- `syncLock` (line 241) shows the pause overlay only when `!(state.outcome !== 'playing')`.
- The warning, the arrow and the hints are all gated on `outcome === 'playing'`.

In quest 1, after the player wins and presses "Keep exploring":
- `outcome` stays `'won'`, but the world and lab stay live.
- So Esc does **not** pause. The lab keeps stepping and casts still land. This breaks GS-7 ("Nothing advances while paused"), and the GS walk wrongly says "pause semantics unchanged".
- The arrow and hint rules that need `outcome === 'playing'` never come back, and I-019's "banner hides hints" rule is keyed to the same field.

**Change:**
- Add a client flag `bannerUp = outcome !== 'playing' && !dismissed`, where `dismissed` is set by "Keep exploring" and cleared when `outcome` changes.
- Say that `syncLock`, the warning, the firefinder, the hints and I-019's banner suppression all use `bannerUp`, not `outcome`.
- Add a test (a new test 15b): after "Keep exploring", Esc gives `paused === true`, and `lab.h2` and `gameTime` do not advance for 30 frames.
- Alternative, smaller: drop "Keep exploring" from this item. Quest 1's banner then only offers R, and this whole class of bugs goes away. Free play is a separate row anyway.

### M2. The banner button cannot be clicked under pointer lock; Enter is the only working route

**Where:** HUD section 6, test 12.

**Problem:** The player is pointer-locked when they win, and nothing calls `exitPointerLock`. With a hidden, locked cursor:
- A 6-year-old with only a mouse (GS-11) cannot click `#banner-next`.
- A locked left `pointerdown` pushes `fire` (I-019), so the click would cast.
- Test 12's `page.click` passes only because headless Chromium is unlocked, so it proves nothing about real play.
- Enter is a second key for the youngest players (GS-11 allows only one movement key).

**Change:**
- Specify that when the Q0 win banner shows with `#banner-next` and the page is locked, `hud.js` calls `document.exitPointerLock()`.
- `syncLock` already skips the pause overlay when `outcome !== 'playing'`, so this does not pause.
- Re-locking happens on the next canvas click, as today.
- Add to test 12: a locked case where `#banner-next` is clicked after the lock was released by the banner, and where Enter pressed under lock leaves `casts` unchanged.
- The seam breaker already lists "Enter never also casts or locks". Extend it to "a mouse click on `#banner-next` never casts".

### M3. Tests 14, 15 and 16 are flaky because the lab steps in real time

**Where:** tests 14 to 16.

**Problem:**
- `chem_tick` runs in `FixedUpdate` at 30 Hz of real time, and the lab steps there too. Every frame the e2e waits (the "within 12 frames" windows, `castAt`, `until`) adds lab ticks.
- At 4x throttle, 12 frames is about 12 to 20 ticks, which is 0.4 to 0.7 mL of H2.
- Test 14 asserts `lab.h2` is 10.0 ± 0.2 and `gasH[0]` is 0.3 ± 0.01 m after `tick 150`. `0.01 m` is about 0.33 mL, which is 5 ticks. These will fail intermittently.
- Test 16 asserts "after filling and `tick 30` (2 mL)". The real-time drift makes it 2.5 to 3 mL or more, not 2. It also asserts `TooLittle`, which needs a value under 5 mL, and that margin is thin.

**Change:**
- Assert deltas and ratios, not absolutes:
  - read `h0 = lab.h2` just before the `tick`, then expect `lab.h2 − h0 = 10.0 ± 0.1`, unless clamped;
  - `gasH[0] / gasH[1]` is 2 ± 0.01 (a ratio is drift-free);
  - `gasH[0] ≈ 0.6 · lab.h2 / 20` read from the same state frame.
- For the `TooLittle` and "just enough" cases, add one debug-gated command, `lab gas <h2_ml>`, which sets `h2_ml` and `o2_ml = h2/2` (3 lines in `main.rs`, rust lane, listed in the Contract under Commands). Use it in tests 15 and 16 so the amount is exact. Do not rely on counting ticks.
- This also removes the 74..=76 window worry: keep it in the pure unit test 3 only.

### M4. The `__gameState` key-order contract is missing; I-019's check will turn red

**Where:** Contract, "New `__gameState` fields".

**Problem:**
- I-019 changes `golden.mjs` `keysOk` to require exactly `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS]` in order.
- I-008 adds `quest`, `questTitle`, `questStep`, `questLine`, `questSpell`, `lab`, `gasH`, `goalObj`, `goalBearing`, `goalElev` and `goalOff`. It gives no order and no edit to that check.
- So the `clock and budget` section (and test 22, "golden still green") fails on the first shared build.

**Change:**
- Add to the Contract: the new keys are appended **after `fxWater`** in the exact order of the table, as `I008_KEYS`.
- The contract stub edits `keysOk` to `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS, ...I008_KEYS]`.
- Name it as the only edit to an existing check (as I-019 did), and say that neither lane redoes it.
- Also state that `lab` is a nested object with the key order `open, level, h2, o2, h2Tested, o2Tested, done`. Say `outcome` and `quest` are in the base position (`outcome` already is).

### M5. The cell pick box is too small for the first thing a 4-to-8-year-old must hit (GS-10, GS-11)

**Where:** section 3, `CELL_PICK_H = (0.75, 0.25, 0.45)`.

**Problem:**
- The whole bench box (1.5 × 2.0 × 3.6 m) shrinks to a 1.5 × 0.5 × 0.9 m slab for the fill step.
- The Fill step is the first lab hurdle, and every other spell is `UseWater`.
- `world::raycast` picks by hit, and assist picks by bounds-centre distance. A smaller box misses more.

**Change:**
- Keep z at `±0.45`. That keeps the 0.15 m gap to the tube boxes (no overlap, as the spec wants).
- Make the box tall: y from about 0.2 to 1.5 (centre y 0.85, half y 0.65). It then covers the bench body under the cell and the battery, and it is much easier to hit.
- Add a test: from `tp 20 6`, a Water cast aimed 2 to 3 degrees off the cell (the GS-10 near-miss pattern) still lands as `Filled` with assist on.

---

## Should-fix

### S1. The 10-second prediction does not match the numbers

- `GasReady` and step 3 happen at 5 mL H2, which is **2.5 s** in. Both tubes are only full at 10 s.
- The bars prediction ("about 10 s of bubbles") and the "Q2 about 2 to 4 minutes" estimate assume the wrong wait.
- At 2.5 s the H2 column is 0.15 m against 0.075 m, which is a weak picture of the 2:1 lesson.
- **Change:** `H2_TEST_ML = 10`, `O2_TEST_ML = 5`, so step 3 starts at 5 s with a clearly visible half-full pair. Update tests 3, 5, 6, 7 and the bars text. The tune is cheap; only the constants and test numbers move.

### S2. Pixel check (test 14) can fail for the wrong reason, and the published `gasH` is not what is drawn

- `gasH` is derived from `Lab::gas_heights`, but the pixels come from a separate system that sets `TubePiece` transforms. A bug in that system passes the state tests.
- The three-attempt pixel test is a noisy substitute.
- **Change:** publish `gasH` from the `TubePiece` transforms actually applied, as the rendered scale. Then a plain state check proves the link. Keep the pixel check as a non-gating logged screenshot, or drop it.

### S3. The calm world still shows the greenhouse warning

- `hud.js` line 605 shows `#warning` ("The greenhouse is heating up … catch fire at 300 °C!") whenever `outcome === 'playing'` and `ghTemp > 60`.
- In calm quest 1 nothing can happen to the greenhouse, so a scary pulsing warning is misleading. It is also a GS-6 pulse the item does not test.
- **Change:** publish `ghDanger = false` and clamp `ghTemp` to 20 in `main.rs` while `calm`, or gate the warning on `quest === 0`. Pick one and test it with a fire near the greenhouse.

### S4. GS walk is missing P0 GS-18, and a few copy points

- GS-18 (P0, 18 px text) is not walked, but this item adds `#banner-next`, the longer `#banner .small`, the quest step line and an icon-led objective.
  - **Change:** require computed `font-size >= 18px` for the new elements and add it to test 11 or 12. The pre-existing 15 px `#obj-line` is another item's fix, but the new line must not widen the gap.
- The `Filled` caption is 25 words with a formula. For GS-21 (grade 6) split it. Example: `Water in! Electricity splits it: 2H₂O → 2H₂ + O₂.` Keep `Na₂SO₄` in the Contract text for science (C-027) only if it stays short. If the check on `Na2SO4` in test 14 moves, update that too.
- GS-31 ("minds-on"): both spells are named in the line and the hint, so there is no prediction.
  - **Change:** accept this for the smallest version. Note it in the GS walk as "partial, deferred: a prediction prompt belongs with the hydrogen/oxygen spells follow-up." Do not bolt on a quiz.

### S5. Science row C-030 should be `simplified`

- The standard oxygen test is a **glowing splint that relights**. The game's Fire cast is a flame that "burns brighter".
- That is true of flames in oxygen, but it is not the school test. A strict reviewer (B6, 0 errors) may flag it.
- **Change:** register C-030 as `simplified` with a note, or reword the caption to name the splint idea. Same for C-029: a pop needs hydrogen mixed with some air in a test tube, which the caption mentions ("oxygen from the air"); keep it, but cite it.

### S6. `reset all` and the return to quest 0

- `reset all` is described as a command, but nothing in the HUD sends it, so a kid cannot get back to quest 0 without a reload.
- That is fine for this step. Say it in "Out of scope" with the README follow-up so nobody expects a menu button.

### S7. HUD arrow rules need a Q1 override sentence

- I-019's `#firefinder` is driven by `fireOff`, `nearFire` and `firesLeft`. In calm quest 1 a burning bush the player lit would still drive it.
- **Change:** say in section 6 that in quest 1 the finder ignores `fireOff`/`nearFire` and uses only `goalOff`/`goalObj`.

---

## Smallest version: trims that stay buildable

These are optional. They remove risk but not value.
- Drop the 3-attempt pixel test (S2). It is noisy and not a gate.
- Drop "Keep exploring" (M1 alternative). The post-win live world is the source of the pause and hint bugs, and free play has its own row.
- Keep the arrow with the `.lab` flask icon. A flame icon pointing at the bench would be wrong, and the arrow is cheap since I-019 provides the maths.
- Keep the per-step idle hint, because GS-24 is P0.

## Lanes: buildable from the contract alone?

Mostly yes. These gaps need a line each:
- **Key order and `keysOk`** (M4).
- **How the rust lane finds the ice wall** for `goalObj` and `wall_standing`: say "the `Kind::IceWall` object" (`Kind::IceWall` exists, `world.rs` 81). The spec uses an index without saying how it is found.
- **Where the strings live:** `questLine` and the captions are produced in `main.rs` (rust lane), and the web lane only displays them. The Contract should say so, so the web lane does not hard-code the step lines.
- **`lab gas` debug command** (M3) belongs in the Contract and in the rust lane list.
- The seam breaker list is good. Add: "a click on `#banner-next` never casts", "after Keep exploring (if kept) Esc pauses", and "`gasH` equals the drawn transform".

## Overlap with I-019

- I-019 is the declared dependency, and this spec builds on its merge, so there is no parallel conflict. They touch the same files in sequence: `hud.js`, `style.css`, `golden.mjs`, the `main.rs` state JSON, and the shared `#firefinder`, `#obj-line`, `#banner` and hint code.
- I-019's rust lane says `chem.rs` is not edited, and I-008 owns it. No clash, provided I-019 merges first. The planner's re-check of the "Relied on from I-019" list is the right guard. Add one item to that list: **`I019_KEYS` and the stub's `keysOk` edit** (M4).
- I-019's `Look` enum moves from `world.rs` to `cues.rs`. I-008 edits `world.rs` after that merge, so rebase on the merged file. The `Look` import is the only place a builder could trip.
- Both items must never be built from the same base at the same time. `based_on` moving to the I-019 merge commit handles it.

## Gameplay-standards walk (items this critic checked)

| GS | status |
|----|--------|
| 6 | met for pop and flare (test 19); the calm warning pulse is not tested (S3) |
| 7 | **broken** by post-win live state (M1) |
| 10, 11 | **at risk**: small cell box (M5); banner button under lock (M2) |
| 13 | Enter works; the mouse route does not under lock (M2) |
| 15, 17 | met |
| 18 | **not walked**: P0 for the new text (S4) |
| 21 | partly: `Filled` is too long (S4) |
| 24 | met (step line, arrow, per-step hint) |
| 25, 27, 28 | met |
| 30 | met (Water to fill, Fire to test, wrong spells explain) |
| 31 | partial (S4) |
| 32 | met |

## Required changes (summary)

1. M1: introduce `bannerUp`; fix pause, hints and arrow after "Keep exploring", with a test (or drop "Keep exploring").
2. M2: exit pointer lock when the quest banner shows; test a locked click and Enter.
3. M3: change tests 14 to 16 to deltas, ratios and a debug `lab gas` command.
4. M4: append `I008_KEYS` after `fxWater`; the stub edits `keysOk`.
5. M5: make the cell pick box tall; add an assisted near-miss test.
6. S1: `H2_TEST_ML = 10`, `O2_TEST_ML = 5`; fix the predictions.
7. S2 to S7 as above.

## Planner response

Planner, 2026-10-30. `spec.md` is revised to r4. Every change is accepted, except one half-sentence in M4.

- **M1: accept, using the smaller alternative.** "Keep exploring" is dropped. A new `quest_outcome()` gates `chem_tick`, `tick` and `aim_and_cast`, so a quest 2 win ends the game exactly as a quest 0 win does. `outcome !== 'playing'` keeps its meaning, and no `bannerUp` flag is needed. Test 15 now proves that nothing advances after the win.
- **M2: accept.** The quest 0 banner calls `exitPointerLock()`. The button click calls `stopPropagation` and `preventDefault`. Test 12 adds the locked mouse-click case and the Enter case, both with `casts` unchanged.
- **M3: accept.** Test 14 now checks deltas and ratios. The new debug command `lab gas` (backed by `Lab::set_gas`) gives exact amounts in tests 15–17. The `GasReady` tick window is checked only in unit test 3.
- **M4: accept, except one point.** `I008_KEYS` is appended after `fxWater` in table order, and the stub makes the single `keysOk` edit. Rejected: "`quest` is in the base position". `quest` is a new key, so it cannot be a base key. It goes first in `I008_KEYS`, and `outcome` stays where it is in the base list.
- **M5: accept.** `CELL_PICK_C` is (0, 0.85, 0) and `CELL_PICK_H` is (0.75, 0.65, 0.45), which keeps the 0.15 m z gap to the tubes. Test 14 adds a 2.5° near miss with assist on.
- **S1: accept.** `H2_TEST_ML` is 10 and `O2_TEST_ML` is 5. Unit tests 3, 5 and 7 and the B8 prediction text are updated to match.
- **S2: accept.** `gasH` is now published from the `DrawnGas` resource, which records the transforms actually applied. The pixel check becomes a non-gating screenshot.
- **S3: accept, gating on quest 0.** The `#warning` check gets `quest === 0`, so the published `ghTemp` stays true. New test 16b requires `ghTemp` to go above 60 with no warning shown.
- **S4: accept.** New text is checked for ≥ 18 px in tests 11 and 12. The `Filled` caption is cut to 14 words and keeps `Na₂SO₄`. GS-31 is marked partial and deferred.
- **S5: accept.** C-030 becomes `simplified`, with a note about the glowing splint. C-029's source must mention the hydrogen and air mixture.
- **S6: accept.** Out of scope now says there is no HUD route back to quest 0, and the README follow-up covers the command.
- **S7: accept.** In quest 1 the finder ignores `fireOff`, `nearFire` and `firesLeft`, and is driven only by the `goal*` fields.
- **Trims: accept.** Pixel test and "Keep exploring" are dropped. The arrow and the per-step hint stay.
- **Lane gaps: accept.** The ice wall is found by its `Kind::IceWall` lookup, all strings are made in `main.rs`, and `lab gas` is in the Contract and the rust lane. The seam breaker gets three new checks.
- **Overlap: accept.** `I019_KEYS` and the `keysOk` edit are added to "Relied on from I-019", and the `Look` move is noted for the rebase.
