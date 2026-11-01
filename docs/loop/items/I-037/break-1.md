# I-037 break 1: level 3, the Marsh

Breaker, wave 13. I did not write this code. I tested origin/loop/I-037 at 2afe39c in a fresh detached worktree
(`/tmp/claude-0/brk13-I-037`, removed afterwards). The wasm came from `build-web.sh` (ok, gzip 5132553 B). Headless
Chromium `?low` held the shared lock for every run.

## Verdict

**BLOCKING: 1** (B1 below). It is a level-2 regression that this item introduced: coming back from the Marsh wins
the Frozen Gate without solving it, and the win banner then describes something that never happened.
Everything else in the Marsh held up: no way through a burning flame line, aim equal to what gets hit, captions
that match the state, a working walk-stop, and no leaks or console errors.

## Runs

| run | result |
|---|---|
| golden `ONLY=marsh` | **16/16** |
| golden `ONLY=frozenGate` | run 1 15/16. `Leidenfrost beads ... then 0 within 2 s` failed (beads gone after 3679 ms) at load average 7.3 and 0.4-2 fps. Run 2 (load 3.8): **16/16**. This is a wall-clock flake, the same one the builder recorded |
| golden `formulaBench` (a local, uncommitted copy of golden.mjs whose only change is an `ONLY=formulaBench` branch; no check edited) | **55/55** |
| `rustc --edition 2021 --test src/chem.rs` | 28 passed |
| `npm test` (science lint included) | 85 passed |
| breaker probes (real key presses plus state bridge, below) | no console errors, no panics |

## Blocking

### B1. Returning from the Marsh wins level 2 for free, and the banner says the gate was opened

`levels()` reloads level 2 fresh, with the stream liquid and the ice gate standing (`ice 1, wall true`). It then
puts the player at `level2::ARRIVE_NORTH (0, -20.4)`, which is on the lantern side of the gate, 3.4 m from the
lantern, facing it. One step forward triggers `chem.world.reach()`.

What I saw:
- Probe: win level 2, enter the Marsh, take the back arch, walk forward 1.5 s.
- Result: `outcome: won` at z -17.75, while the gate object still reads `ice 1, wall true, frozen true`.
- The banner said "You opened the Frozen Gate!" with the subtitle "Freeze to cross, melt to open: water changed
  state both ways." The caption said the same.

Why this blocks:
- It hits two of the Level-2 breaker follow-ups this wave was told not to repeat: winning without solving, and
  captions that do not match the actual state.
- A 5-year-old coming back through the arch gets a win and a science line about melting a gate that is still
  frozen, and is still standing behind them.

The `?level=3` path is worse: `cleared2` is false there, so the north arch shows dark and solid (see S1). The only
thing to do on arrival is walk to the lantern for the free win.

Possible fixes (the builder's choice):
- (a) When arriving from level 3, load level 2 already solved: stream frozen (`ice 1`), gate melted
  (`wall false, ice 0`), and `outcome` left at `playing`, with the lantern reach skipped for that load. Or latch
  the outcome as `won` without any new banner or caption.
- (b) Skip `reach()` while `from == 3` until the player has crossed south of the ridge and come back.
- (c) Arrive at level 2's START instead (the simplest, but then the walk back to the Marsh means solving level 2
  again).

Option (a) matches the story best. Whatever the fix, a golden check should pin it: back arch, walk to the
lantern, then `outcome` must not be `won` with the gate standing, or the gate must read as melted.

## Should fix

- **S1. Arriving from a direct `?level=3` load leaves level 2's north arch shut.**
  - The spec (step 1) says the arrival from level 3 comes "with `cleared2` set". `levels()` only sets `cleared2`
    when level 2 is won.
  - Probe: `?level=3`, back arch, then `nextOpen: false`. The player stands 2.2 m from a dark, solid door they
    just walked through, and walking into it stops at z -22.45.
  - Fix: set `level.cleared2 = true` when `n == 2 && from == 3`. B1's fix may cover this.
- **S2. The Sprout Pooled line "All the fires went out!" is false after the dip.**
  - When the dip fills, the four hollow flames still burn straight ahead, and 5-year-olds are the band that reads
    this line.
  - C-089's own notes admit "the lantern's way may still have flames in the other hollow".
  - Suggest "These fires went out!" or "The fires here went out!", with C-089 updated to match. The Seedling to
    Adept lines say "the flames" or "every flame", which is fine because those words read as the hollow's flames.
- **S3. Science-review flag: the game's lesson goes against the builder's own source [S31].**
  - [S31] (NSW RFS) says: "Do not extinguish burning gas without first cutting off the leaking gas supply."
  - Separately, real methane keeps bubbling up after the flames go out. It is lighter than air ([S31]), so it would
    rise out of a CO2 pool rather than stay smothered.
  - The game shows the bubbles stopping once a vent is out (`level3::sync` only emits bubbles for burning vents).
    That visual says the gas stopped, which is not true.
  - This is acceptable as a simplification, but the fresh science reviewer should decide whether:
    - the bubbles should keep rising from smothered vents, which is more honest and costs nothing, and
    - C-076, C-091 or a Not-claims row should record the "do not put out a gas fire without shutting off the
      supply" caveat as `simplified`.
  - The Adept safety line already covers breathing CO2.
- **S4. Standing on a rail walks you over every flame (debug only).**
  - `tp 1.3 14` puts the body inside the 1.2 m rail. `motion::step` resolves the y axis last and lifts the player
    onto the rail top (`pos y = 1.2`).
  - From there a walk north passes over both burning flame lines (vents are also 1.2 m tall) to z -15.9 with
    `ventsLit 7`.
  - A real player cannot get into that overlap:
    - jumping against either rail with W+A or W+D stops at x ±1.15, z 10.75, and
    - the jump apex is 1.0 m, below the 1.2 m tops.
  - So this does not block. If any future teleport or arrival can land in a prop (the I-034 level select, new
    arrival points), the rails and vents should be taller than `HEIGHT`, or `tp` should refuse solid spots.
- **S5. Minor HUD and copy:**
  - Level 3's win banner has no "Press R to play again" (level 1 and 2 have it), though R works.
  - Hitting the lantern from the start or the dip is impossible because the hollow vents block the ray, so the
    lantern caption only shows from inside the hollow or beyond. That is fine; just noting it.

## What I tried that held (with evidence from the probes)

- **Flame lines block the walk.**
  - Walking straight in stops at z 10.75.
  - So does hugging either rail with real W+A or W+D held and Space pressed 10 times.
  - So do sprint-jumps at the dip seam (x 0.5), diagonals at yaw ±40 and ±80 (stopped at x ±1.15, z 10.75), and
    jumping through a flame knocked down by Water to 40 % (still z 10.75).
  - Walking sideways stops at x ±1.15 (rails).
- **Aim equals what gets hit.**
  - From z 13 at pitch -40 the reticle is on `dip` (0), and the cast fills the dip.
  - At -8 to -30 it is on a dip vent, and the cast fills the dip.
  - At -3 to -5 it reaches over the dip flames to a hollow vent or the `hollow` floor. A CO2 cast there filled
    `pools[1]` to 0.34 with the Filling line, so what the reticle shows is what fills.
  - Filling the hollow before the dip is allowed and consistent; it gives older players a shortcut to find.
- **Captions match the state.**
  - On the pooled dip floor: Water gives "These flames are already out.", CO2 repeats Pooled with `ventsLit` still
    4, and Fire gives the NoOxygen line.
  - Water on a hollow vent while aiming at the lantern behind it shows the Flicker line (the vent was hit).
- **The hollow fills in three casts:** 0.34, 0.68, 1. `ventsLit` goes 4, 4, then 0. The win shows "You cleared the
  Marsh!", the objective reads "Marsh cleared!" and the bench stays hidden in level 3.
- **Walk-stop.**
  - Holding W at the win leaves `pos` unchanged for 2 s.
  - Releasing, turning and pressing W again walks back toward the arch, as the banner says.
  - R after the win puts the player at START with 7 vents lit, pools `[0,0]`, `playing`, and walking works again.
  - R mid-level resets the pools and the flames.
- **Switching levels.**
  - L1 to L2 to L3 to L2 to L1 by arches and by `level N` changed state correctly: `exitOpen`/`nextOpen` were right,
    `pools` is `[]` outside level 3, and level 1 came back fresh with 3 fires.
  - In the golden: 10 cycles of 1-2-3-2-1 left the entity count unchanged at 633 and wasm memory flat at 33.6 MB.
    Level 3 has 522 entities, under level 1's 633.
- **Formula Bench and level 2** pass unchanged: formulaBench 55/55 and frozenGate 16/16 on the rerun.
