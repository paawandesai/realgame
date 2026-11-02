# I-052 break 2 (wave 16): puzzles 1-3 on loop/I-052-r2 (5b4b247)

Breaker: a fresh detached worktree of `origin/loop/I-052-r2` (5b4b247). I did not write this code. Web build:
`build-web.sh` ok, wasm gzip 5,288,234 bytes. Native experiments ran with `CARGO_TARGET_DIR=/home/claude/native-target`.
Golden runs used headless Chromium with `QUERY=?low`, holding `/tmp/claude-0/chromium.lock`.

The experiments are throwaway `#[ignore]` tests. They are saved as `evidence/break-2/sweeps.patch` (apply it to
5b4b247 and run `cargo test --release brk_ -- --ignored --nocapture` in `crates/bevy-client`). The source tree is not
changed.

## BLOCKING

### B1. P1 cannot be finished with worked casts, which is how every web player with a band casts Water

**What a worked cast throws.** A worked Water cast always flies at hold 0, the shortest "tap" arc:
- `voxel_cast::arc_spell` is false while Water's route is `Worked`, so `charge` never runs for it.
- The accepted cast is shot on the frame the builder closes. `game.held` is 0 while paused (`main.rs:2086`).
- So the charge mechanic does nothing for a worked cast, and that includes the Practised one-step confirm.

**What the native test plans instead.** `a_lobbed_water_arc_from_the_plaza_puts_the_workshop_out` puts the fire out
with `tap = 0.08`. It only asserts that the hold-0 aims land somewhere in a loose z window (187.9..190.01). It never
puts the fire out with hold 0. This is the "difference" round 2 could not find.

**Hold 0, the same three aims, from (142.5, 35, 197).** Native, the same `Matter::tick` (trace = burning cells after
each cast plus the gap that follows it):

| hold | sim seconds between casts | casts | burning 30 s after the last cast |
|---|---|---|---|
| 0.08 (the native test) | 1, 2, 3, 5 or 8 | 2 | 0 |
| 0 (a worked cast) | 1 | 11 | 0 |
| 0 | 3 (with 0.6 s flight) | 12, not out | 6 |
| 0 | 5 | 12, not out | 7 |
| 0 | 8 | 12, not out | 6 |

- With hold 0, the first aim (y 40, z 189) lands on the store's front face, `(142,36,189)` with normal +z, at z 190.0.
  The others land on the front row's top edge, at z ≈ 189.8.
- The 2 m soak from there misses part of the back row (z 188).
- Every store block is `SOURCE`, so one block left burning relights the whole store in 3-5 s.

**Why the gaps are long in the browser.** `blockchem::drive` keeps stepping while the worked builder pauses the game.
It reads `Res<Time>` and `frame_dt` and never checks `game.paused`. So the time a player spends in the builder is fire
time. That is a few seconds for an Apprentice who types quickly, and 10 s or more for a Sprout who builds the ball
picture.

**The golden reproduces it** (this run): `burning 14 -> 2, casts [built,built,confirm x6], quest 1`, then
`P1 the workshop stays out: burning 7`. This matches the 5 s and 8 s rows above.

**Aiming where a player would** (hold 0, aims cycled across x 141.5 / 142.5 / 143.5):

| aim | 4 s gaps | 10 s gaps |
|---|---|---|
| crosshair on the flames (y 37.5, z 188.5) | never out (8 burning after 12 casts) | never out |
| from the door or inside the yard, aimed at the store top | 5-6 casts, erratic | out only by luck (casts 5-7), or never |
| y 41, z 188 (about 4° above the pile's top) | 2 casts | 4 casts |
| y 41.5, z 187.5 | 2 casts | — |

So there is a working aim. It is a window a few degrees wide above the fire. Nothing tells the player about it, and the
caption on screen during P1 says "Aim at a fire and click." (`evidence/break-2/p1-after-8-casts.png`).
- A 6-year-old aims at the flames, so the fire never goes out and quest step 1 never completes. This is a dead end on
  puzzle 1 of the assignment.
- A 13-year-old may find the window by accident.
- A judge who aims at the fire sees it flare back every time.

**Fix (any one helps; the first two together should make it robust):**
1. Do not step the matter sim while `game.paused` (the builder, the pack, the pause card).
2. Make one Water cast on the store put out the whole connected burning `SOURCE` cluster, or the store's cells within
   the stamp. A real bucket sheets over the pile, as the source note itself says. Or give soaked cells a wet timer of a
   few seconds before they can relight. The science note (`p1-fuel-store`) already says "the back of the pile is cooled
   with the front and does not relight it". The sim does not do that today for a front-face landing.
3. Let the confirm or builder cast keep the press's charge (remember `ch.held` at the press), or plan P1 for hold 0.
4. Change the native test to put the fire out with hold 0, with sim gaps of 5 s or more between casts. The current
   test proves the wrong input.
5. Change the P1 golden aims to the hold-0 working window (for example y 41, z 188), but only after 1-3. Changing the
   aims alone would hide the 6-year-old dead end.

## Should-fix

1. **The Practised gates can be climbed or dug around** (native probe of the generated world with the walls built):
   - **Marsh gate** (x 132, z 66..74, y 30..49): the frozen-side ground is y 42, and the 8 cells above the wall top
     (y 50) are not protected. Pillaring 7 dug dirt blocks at x 131 and stepping over skips Frost practice. The ridge
     on either side goes to y 61 and is protected, so only the gate is low.
   - **Frozen Fields gate** (z 158, top y 39): the 12 cells above the wall in the ford columns (x 122..133) are
     placeable, and the Village-side ground is y 32-34. Pillaring 7 blocks gets you over, though you still land in the
     deep ford, so Frost is still needed.
   - **Cave gate** (x 218..222, y 30..35): 25 diggable cells sit just under the wall, and the ground beside it is
     y 31. So a short tunnel under the 6-high box gets past it. That gate sits on the late thread's cave mouth, and
     round 2 already notes the ladder-exit bypass.
   - Fix: protect each gate's column up to the protected ridge or cliff height (or the world top), and protect a layer
     under it.
2. **"3 correct worked casts" is not what is counted.** In `main.rs`:
   - `Submit::Cast { earns: false }` (a cast built after the answer was shown) still pushes `(spell, true)` to
     `cast_log`.
   - Every `Route::Instant` cast counts: no band, a spell the band cannot learn, `ready == false`, or Mastered.
   - The saved I-029 XP makes Water Practised after the bench plus 2 casts. This run shows it: `built 2, stage
     practised`, and in P1 the third press was already a confirm.
   - The step 4 check was loosened to accept 2. Paawan's rule is 3 in a run.
   - Fix: count only `earns == true` worked casts (and instant casts only when the band cannot learn the spell, if
     that is intended), and keep the Practised stage at 3 run casts for the gate and the confirm.
3. **Frame time climbs after the ford freeze and never comes back on the same page.**
   - The CPUFRAME p50 is about 10-12 ms up to and during the ford-gate check.
   - It rises to 25-35 ms from the Frost casts on the ford, and stays there through the ice gate, the Marsh gate and P3.
     That is 52 of 132 windows over 16 ms, and the mean p50 is 17.1 ms.
   - It drops back to about 10 ms only on the next page load (voxelCast).
   - My guess, not checked: frost-cooled cells, or the refilling `SOURCE` river next to the ice, keep chunks on the hot
     or active list forever.
   - Log `activeChunks` and `matterMs` across P2. The E6 budget is a 16 ms median.
4. **P3 can be skipped, and the cave gate does not need it.**
   - The hollow is an 11 × 10 sunken box in open marsh at ground y 31-32. You can walk around it to the north
     (z < 96) and to the south (z > 106).
   - The cave gate opens on CO2 Practised, which you can earn anywhere.
   - So a player can reach the cave mouth with the quest stuck on step 5. Spec section 7 says "the hollow only when
     smothered".
   - Either make the hollow the only way through to the cave, or gate the cave on the hollow being smothered as well.
5. **Nothing tells the player to lob in P1.**
   - The P1 line is "...then put it out" and the caption is "Aim at a fire and click."
   - From the door, a level aim drops on the floor, and an aim at the flames hits the store's front face or the front
     wall (`(143,36,193)`).
   - Add a hint after a Water cast that lands in the workshop without putting it out: "Throw it high so it falls onto
     the fire."
6. **The gate sign overlaps the objective HUD.** At the ford, the "Practise Water 3 times to open (0/3)" sign covers the
   end of the objective line ("Learn Frost and freez…") (`evidence/break-2/p2-gate-sign-over-objective.png`).
   - The biome label there reads "The Kiln Terrace" while the player stands on the river bank (z 163.5), because
     `biome_at` puts x 112..144, z < 177 in the Kiln.
7. **The science note claims more than the sim does** (`p1-fuel-store`).
   - The note says soaked blocks "do not relight". In play, soaked store blocks relight from a neighbour within seconds.
   - Real soaked wood stays near 100 °C until the water has boiled off.
   - A wet timer (B1, fix 2) would make the note true.
8. **voxelCast: the 1 s charge lands on the Kiln terrace step.**
   - The check fails on this run: `hold 25.02 m ... p [128.7,36.47,180]`.
   - From spawn, a 25 m throw north reaches the terrace step face at z 180. Either the check's spot or the terrace
     edge needs to move. Round 2 did not check it on the base commit, and neither did I.
9. **The P2 gate golden only proves debug instant casts.** The ford-gate check runs on a page with no band, where every
   instant cast counts. No golden opens a gate with worked casts.
   - The step 4 check makes Water Practised by worked casts, but on a different page from the gate.
   - Add one gate-opening check on a banded page.

## What works (seen this run)

- The intro: both Just play checks pass. The welcome caption shows after the intro, and the intro does not reopen.
- The bench at the plaza lights, and Water is learnt there.
- **P2:**
  - The ford wall holds: the walk stopped at z 159.35.
  - Its sign counts, and 3 casts sink it, with the caption "The runestone gate to The Frozen Fields sinks…".
  - Frost from the bank freezes the ford (first ice at (128,30,153)), the player walks across (z 146.8), and the quest
    goes to step 3.
  - Fire melts the ice gate in 5 presses, and the quest goes to step 4.
  - The Marsh gate's sign reads 0/3 and then 3/3, and the gate opens.
- **P3:** with the gap open, 3 CO2 casts leave 3 vents lit. With dirt in the gap, 3 more casts put out all 6 vents.
  The CO2 maximum was 0.878 and the quest went to step 6.
- No console errors.
- **Soft-locks:**
  - Puzzle blocks are protected, so a player cannot dig out the burning store, bridge the ford with blocks, or dig the
    hollow's rim.
  - The workshop cannot burn out by itself, because `SOURCE` fuel never counts down.
  - Ford ice cannot be pre-melted into a soft-lock: Frost refreezes it.
  - I found no water-drain or CO2-leak soft-lock in P2 or P3. The hollow's CO2 leaks away, but the closed gap stays
    closed.

## Golden results (this run, 5b4b247 build)

- `valleyPuzzles,voxelCast`: 13 pass, 3 fail.
  - The 2 P1 checks fail (B1).
  - voxelCast's tap/charge check fails (should-fix 8).
  - The other 9 valleyPuzzles checks and 4 voxelCast checks pass.
- `formulaBench,workedCasts` (classic): **109 of 109 pass**, exit 0. Round 2 had left two of these open, and both now
  pass on the d12b33e+ build:
  - `worked: R2a` (the classic-only confirm fix in ad96daf holds);
  - `bench: Sprout learns Water with the mouse`.
- Not run by me: `voxelEngine`, `blockMatter`, `valleyStory`. Round 2's E4 (Pick dig), E15 and E17 re-runs are
  still open.
