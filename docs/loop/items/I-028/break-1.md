# I-028 break, round 1 (wave 12)

Breaker: fresh model. I did not write this code. Target: `origin/loop/I-028` at **384b910**, on origin/main dc1d336 (I-027 merged), in the detached worktree `/tmp/claude-0/brk12-I-028`. The previous breaker was cut off. Its worktree was already at 384b910, and its `web/pkg-out` matches every web file at HEAD byte for byte. The wasm in it was built at 22:28 from the shared target, and no Rust file changed after b8cf7fe. I reused that build. All Chromium runs held `/tmp/claude-0/chromium.lock` inside the same command.

Evidence: `evidence/break-1/`. It holds `seam-run.txt` (seam script run), `extra-run.txt` (the picker gate on a built-site copy, plus the three new breaks), `brk-seam.mjs` and `brk-extra.mjs` (the scripts; usage `node <script> <pkg-out dir>`), `judge-probe.rs.txt` (the temporary sim probe, removed after the run) and `flat-beam-1221.png`.

## Verdict

**Not ready: 1 blocking, 5 should-fix.** Every seam check in the Lanes list passes. The blocking finding is in the Seedling Fire scale, the item's main teaching device for 8 to 10 year olds. Two judge holes let a typed formula either get no reason or be judged correct when it is wrong.

## Seam checks (spec, Lanes) — all pass

| check | result |
|---|---|
| JS compositions (`composeSprout`, `composeSeedling`, `composeEquation`, and the def formula at Apprentice) judged `ok` by the shared build for every learnable spell at every band | PASS. Sprout lists water, frost, co2. Seedling, Apprentice and Adept list all four. Every build is `ok` |
| `scaleOf` against the judge's `unbalanced` diffs, all 256 Fire coefficient sets, through `__bench.cmd('try …')` | PASS. n=256, 2 `ok` (1212 and 2424), 0 disagreements |
| No key reaches winit (canvas capture spy) or `hud.js` (document spy) while open; started locked, opened with `B`, typed `Br2 R 3 P 1 4` in the box, then `2 R P Space E B 4` with focus on Make | PASS. doc=0 and canvas=0. `spell`, `captionId` and `gameTime` are unchanged. The bench stays open and the box keeps `Br2…` (a typed `B` never toggles it) |
| Five more correct builds never re-learn | PASS. Frost stays `learning` at xp 10, and every repeat gives `ok/false` |
| `__benchState` writes (spy on the setter) | PASS. Closed and still for 60 frames: 0 writes. Open on a build view with 3 judged tries in 60 frames: exactly 3 writes |
| Command words and `BenchView` fields match between `bench.js` and `bench.rs` | PASS (read both). The five words `band open pick try close` match. `band` is pushed open or closed, and the closed view carries `band` (test F below). `try` text is re-joined with single spaces, and an empty `try 2` reaches Rust as `Empty`. Field names `atBench`, `spells[].{slot,id,name,formula,stage,prompt}`, `pick`, `recipe[].{side,coef,formula,atoms,state}` and `result.{n,slot,ok,code,diffs,other,bad,simplest,coef,learned,stage}` are read by `bench.js` under the same names |
| Closing via "Back to the game" restores the pause state and `inert`, and asks for the lock again | PASS. Unpaused, no `inert` left, lock re-acquired. 0 console or page errors |
| Age picker before pointer lock (built-site copy: `pkg-out` + `age.js` + `analytics.js`, switch on) | PASS. With the picker showing, `B` and a scripted `#benchbtn.click()` both leave the bench shut. Picking "8 to 10" puts `band:"seedling"` in the closed view, `B` then opens the 4-card list with no chooser, and after Escape one click locks the pointer again. (The first run used plain `pkg-out`, which ships no `age.js`, so it showed no picker. That run was a harness miss, not a product failure.) |
| Hint gating | PASS. No `hint` before any cast. `hint` appears after the first cast with nothing learned. It never shows once a spell is Learning (also after a later cast) |
| Kid playthrough: `#benchbtn`, chooser "8 to 10", Fire, Make at 1-1-1-1, tap O₂ and H₂O, Make, "Back to the game", then cast Fire | PASS. Chooser has 3 buttons. Beam `false`, then `unbalanced` with the Contract reason. Beam `true` with pans `C 1, H 4, O 4` on both sides. Done line `CH₄ + 2O₂ → CO₂ + 2H₂O is right! You are learning Fire.` Fire `learning` xp 10. Unpaused and closed, and the cast lands (casts 0 → 1) |
| Escape on the chooser closes the bench | PASS |
| `cargo test -p realgame-sim --no-default-features --locked bench` | 8/8 pass |
| Lane hygiene (test 33) | PASS. Changed paths are all in Lanes. `main.rs` has exactly 3 added lines. `boot.js` has +1 line and `build-web.sh` +1 word. `golden` and `analytics-e2e` have 0 deleted lines. `cues.rs`, `feel.rs` and the stream code are untouched |

## Blocking

### B1. The Seedling scale draws a perfectly level beam on 9 unbalanced Fire builds, while the reason says "The scale is not level"

`scaleOf` sets `tilt = sign(right total − left total)`, and the CSS rotates the beam by `tilt * 4deg` (`style.css:238`). When both pans hold the **same total number of atoms** but different elements, `tilt` is 0, so the beam is flat. Only the small "Level" label and its tick are missing (`data-level="false"`). Enumerating 5a+2b = 3c+3d without balance gives 9 of the 256 coefficient sets: **1221, 2113, 2122, 2131, 2433, 2442, 3334, 3343, 4244**.

The first is two taps from the start, and it is an easy mistake for a 9-year-old: tap O₂ (correct), then tap CO₂ instead of H₂O. That gives `CH4 + 2O2 -> 2CO2 + H2O`, pans `C 1, H 4, O 4` against `C 2, H 2, O 5`, both 9 atoms. Confirmed in the real build (`extra-run.txt`, `flat-beam-1221.png`): `data-tilt="0"`, computed transform `matrix(1, 0, 0, 1, 0, 0)` (no rotation), no "more atoms" label on either pan. After Make the panel says "The scale is not level. Atoms are never made or lost, …" directly under a beam that is drawn level.

Why this blocks:
- It breaks the spec, section 3: "the beam … is level only when every element matches."
- The prompt (C-042) says "Change the numbers until the scale is level." The child does exactly that, and the game tells them they are wrong.
- It teaches the wrong conservation idea: that equal *total* atom counts are enough. The scale was the item's concrete picture of per-element conservation (GS-32).
- The seam check (`scaleOf.level` against the judge) cannot catch it, because `level` itself is right. Only the drawn beam is wrong. Test 18 checks only the start pose and the solved pose.

Fix suggestion (web lane, `bench.js` + its `style.css` block): never draw a flat beam unless `level` is true. For example, when `!level && tilt === 0`, tilt toward the side of the first mismatched element in recipe order (or use a fixed "wobble" angle, static under reduced motion). Also show a per-element mismatch mark on the pans (the spec's Seedling single-substance tiles already use a tick/no-tick per element). Then add a golden or node check for 1221: `data-level="false"` must imply `data-tilt != 0` (or whatever the fix's visible cue is).

## Should-fix

### S1. A zero subscript gives `atoms_off` with **empty** `diffs`, and from the third try the Apprentice reason is a blank line

`N2O0(l)` for Frost and `CO2H0(g)` for CO₂ parse to atom maps holding an `O:0` or `H:0` entry. The judge compares the maps, sees them differ and returns `AtomsOff`, but the diff filter keeps only elements whose counts differ, so `diffs == []`. At Apprentice the first two tries show "Not yet. Which atoms does Frost have, and how many?". **The third shows `.bench-reason` with an empty string** (`reasonFor` joins zero `Count again` lines), so the fizzle panel is a red cross with no words (confirmed in the real build, `extra-run.txt`). This also breaks the `Verdict` doc ("AtomsOff: (el, have, want) … only the ones that differ"), because the verdict says atoms are off and names none. Fix in `crates/sim/src/bench.rs`: drop zero counts before comparing (then `N2O0(l)` is `ok` with `simplest = "N2(l)"`), or treat a 0 subscript as `Parse`. Also make `reasonFor` fall back to the before-3 line when `diffs` is empty. Test 4's fuzz alphabet (`HONCXx()lgs2 4+->→₂"\`) has no `0` or `1`, which is why it never found this. Add `0` and `1` to the alphabet, plus an invariant: `AtomsOff`/`Unbalanced` ⇒ `diffs` not empty.

### S2. A zero coefficient is judged `ok`: `0N2(l)` "is right! You are learning Frost."

The probe shows `0H2O` (Sprout), `0N2(l)`, `00N2(l)` and `0/1N2(l)` are all `ok` with `coef == 0` and no `simplest`. Fire `0CH4 + 2O2 -> CO2 + 2H2O` is also `ok`. `0CH4 + 0O2 -> 0CO2 + 0H2O` comes back `unbalanced` with the diffs of 1-1-1-1, which shows the parser reads a 0 coefficient as 1. In the real build, `0N2(l)` **learned Frost** and showed `N₂(l) is right! You are learning Frost.` (`extra-run.txt`). Chemically, "zero molecules of nitrogen" is not the formula of liquid nitrogen, and C-048's coefficient line is never shown for it. The cause is in `formula.rs` (not this item's file), but the judge is the item's only checker (B10), and I-029/I-030 will reuse it. Fix in `bench::judge`: reject any term whose coefficient text is 0 (a `Parse` with `bad = "0"`, or a new reason). Add the cases to test 3b.

### S3. C-047 (and the CO₂ state claim) cites [S5], whose register entry states only a molecular weight

C-047 says "carbon dioxide is a gas at room temperature and 1 atm", sourced to [S5] "NIST Chemistry WebBook, Carbon dioxide (molecular weight 44.0095)". The claim is true: CO₂ sublimes at 194.7 K at 1 atm and has no liquid below 5.18 bar (triple point 216.6 K). But the cited source line does not say it. Fix (web lane, `content/science-sources.md`): point C-047 at the WebBook's CO₂ phase-change page (`…ID=C124389&Mask=4`, sublimation and triple-point data) as a new or extended source line. In the same way, C-045's "liquid between 273.15 K and 373.17 K" cites [S2], which gives only Tboil. Either drop the 273.15 K bound or cite [S10] for melting.

### S4. `misses` counts every fizzle, not only `atoms_off`, so the Apprentice "counts from the third try" help comes early or late

`showFizzle` bumps `misses[slot]` on any wrong code. A student who types `N2` (state_missing) and then `N2(g)` (wrong_state) gets per-element counts on their first `atoms_off` try. A double-clicked Make also counts twice, because each `try` is judged. That matches the spec's literal "third wrong try on the same spell", so this is low severity. Still, the 2D rule it copies was "help after three misses of the atoms". Consider counting only `atoms_off` and `unbalanced`, or note the choice in the lane notes.

### S5. `?band=` from a class link can be overridden by a child with "Change age"

`resolveBand` lets an explicit chooser pick (via "Change age") beat the URL band (`explicit ? bandOf(chosen) : bandFrom(...)`). With `?band=sprout`, a child can switch to Apprentice, and Rust then gets `band apprentice` (seam run, section I). The spec's resolution order puts the URL first, and its teacher note says a class link "can fix the band". Either honour the URL (hide "Change age", or make it a no-op, when `?band=` is set) or record in the spec and teacher notes that the link only sets the starting band.

## Science review (B6, against [S1], [S2], [S4], [S5], [S7])

- **11 prompts** (spells.json, byte-equal to the spec table): correct.
  - Frost Sprout and Seedling ("super cold", "very, very cold"; C-040, C-041): N₂ Tboil 77.34 K [S1]. Correct.
  - Methane prompts (C-042, C-043): CH₄ + 2O₂ → CO₂ + 2H₂O is the textbook example in [S7] §4.1, with ΔcH° in [S4]. Correct.
  - State claims: H₂O(l) [S2] and N₂(l) at ≤ 77.34 K [S1] are correct. CO₂(g) is correct but under-sourced (S3).
  - The not-claim prompts (Water and CO₂ Sprout and Seedling) state no property. Correct as not-claims.
- **Reason table:** every string in `reasonFor` matches the Contract table byte for byte (read against the spec). The one claim, C-044 "Atoms are never made or lost, so both sides need the same atoms.", is correct for chemical reactions [S7] §4.1. C-048 (a coefficient counts molecules) is correct [S7]. The scale's only labels are "Before the arrow", "After the arrow", "more atoms" and "Level". No "heavier", "weight" or "mass" appears in `bench.js`. Its "more atoms" compares totals, which is true as stated. But see B1: totals are not what conservation is about, and the flat beam on equal totals contradicts C-044 on screen.
- **Register rows C-040 to C-048 and the 24 not-claim rows:** ids are contiguous from the next free id, and every `where` path is correct. Fix the sources in S3.
- **Judge accepts exactly the sourced formulas?** Mostly. It accepts H₂O(l), N₂(l) and CO₂(g), in ASCII or Unicode, with spaces and any atom order (`OH2(l)`, `O2C(g)`, `HHO(l)` are `ok` with `simplest`), plus methane combustion and its multiples and fractions. Typed states on Fire are not judged (by design), so `CH4 + 2O2 → CO2 + 2H2O(g)` is `ok`. It rejects the reverse reaction (`CO2 + 2H2O -> CH4 + 2O2` → `wrong_species`), hydrogen burning, decomposition, H₂O₂, HO, NO₂, CO, single N, dimers like `C2O4(g)`, spectators (`+ N2`), and `(aq)`/`(s)` states. The exceptions are S1 (zero subscript) and S2 (zero coefficient). Noted, not a finding: a reordered methane formula `H4C + 2O2 -> …` is `wrong_species` (species_key compares formula text), while a reordered single substance `OH2(l)` is `ok`. That is consistent with the spec's rules, but the Apprentice reason for `H4C` ("Those are not the substances…") is a little harsh. I-031 can revisit it.

## Not run

- No full golden (35-90 min). The integrator's full run plus its section run after the fix stand. All of my runs used the scripts above against the shared build.
- Test 31 perf: the integrator's evidence stands.

## Hygiene

The temporary sim probe was appended to `crates/sim/src/bench.rs` in the breaker worktree only and restored right after (`git status` clean). It never reached a branch. Sim tests reused `/home/claude/bevy-target/debug` (the shared target), which is deleted afterwards. The worktree `/tmp/claude-0/brk12-I-028` is removed after the push.
