# I-028 break, round 2 (wave 12)

Breaker: fresh model. I did not write this code. Target: `origin/loop/I-028` at **a3bc179** (fix commit 95223ab plus its notes), in the detached worktree `/tmp/claude-0/brk12-I-028b`. I built the wasm from that worktree with the shared target (`build-web.sh` ok, wasm gzip 5 103 485 B). For the age-picker checks I used a built-site copy (`pkg-out` + `age.js` + `analytics.js`, analytics switched on through `?picker`). Every Chromium run held `/tmp/claude-0/chromium.lock` inside the same command. The headless frame rate was 1.2 to 1.5 fps, so one frame took about 700 ms.

Evidence is in `evidence/break-2/`:
- `formulaBench-only.txt`: the golden `formulaBench` section, run alone.
- `brk2.mjs` and `brk2-run.txt`: round-2 re-runs and new breaks.
- `brk2b.mjs` and `brk2b-run.txt`: a harness fix for N3, and break-1 section I adapted.
- `seam-rerun.txt` and `extra-rerun.txt`: the break-1 scripts re-run unchanged.
- `judge-probe-2.txt`: the temporary sim probe, removed after the run.

## Verdict

**No blocking findings. 2 should-fix, 2 minor.** Every break-1 blocking and should-fix repro is fixed in the real build, and nothing on break-1's passed list regressed. I found two new problems. A leading-zero subscript (`N02(l)`) is judged right and teaches the spell. And the new double-click guard drops a corrected build on a slow device, so the panel says "The scale is not level" under a level scale.

## Gates

| gate | result |
|---|---|
| golden `formulaBench` section alone (temporary copy of golden that runs boot and then `formulaBench`; deleted afterwards), `BASE_URL= QUERY=?low` | **57/57** (2 boot checks, all `bench:` checks including the 7 round-2 checks, and no console errors) |
| `cargo test -p realgame-sim --no-default-features --locked bench` | 8/8 |
| `node --test test/bench.test.ts` | 12/12 |
| Full golden | not run, per the task. The fixer's 192/193 stands. Its one FAIL is the known slow-GPU cast flake, outside this item |

## Break-1 repros, re-run

| break-1 item | round 2 result |
|---|---|
| **B1** flat beam at 1221 and the other 8 equal-total sets | **FIXED.** `scaleOf` gives level iff tilt is 0 for all 256 sets. The equal-total unbalanced sets are exactly 1221, 2113, 2122, 2131, 2433, 2442, 3334, 3343 and 4244. In the real DOM, for each of the 9: `data-level="false"`, `data-tilt` is not 0, the computed transform is rotated (1221: `matrix(0.9976, -0.0698, 0.0698, 0.9976, 0, 0)`), no pan says "more atoms", there is no "Level" text, and the mismatched elements are marked on both pans (1221: C, H and O each get a 12 px cross). 1212 is still level, flat, unmarked and says "Level". Break-1 J now passes |
| **S1** zero subscript, blank Apprentice reason | **FIXED.** `N2O0(l)` three times gives `parse`/`bad "0"` each time, with the reason "Use element symbols, numbers, + and an arrow.", which is never blank. `CO2H0(g)`, `N₂O₀(l)` and `… + 2H2O0` behave the same. The sim fuzz now includes 0 and 1, plus the diffs invariant |
| **S2** zero coefficient judged ok | **FIXED.** `0N2(l)`, `00N2(l)`, `0/1N2(l)`, `0.0N2(l)`, `₀N₂(l)` and `0 N2(l)` are all rejected. So are Fire `0CH4 + …` and `CH4 + 2O2 → 0CO2 + 2H2O` (also with `=`, `=>` and `⟶`), and Sprout and Seedling `0H2O`. All are `parse`, and nothing is learned (every stage stays unknown). Controls: `10N2(l)` and `H2O1(l)` are right. `1/0…` and `0/0…` are `parse` and never panic |
| **S3** citations | **FIXED.** I checked [S24] against the live NIST page (`ID=C124389&Mask=4`): Ttriple 216.58 K (Marsh 1987; Angus et al. 1976) and Ptriple 5.185 bar, which match the register line. The C-047 argument holds: the triple point is above 1.013 bar, so CO₂ has no liquid at 1 atm and is a gas at room temperature. C-045 now cites [S10] (OpenStax 10.3) for 273.15 K melting and [S2] for 373.17 K boiling |
| **S4** misses counted every fizzle | **FIXED** as the fixer described (the golden check "state slips do not count toward the third-try help" passes). See the new S-B below for a side effect of the double-click guard |
| **S5** URL band can be overridden | **FIXED.** With `?band=sprout`, "Change age" is `hidden` with `display: none`, and 12 Tab presses never focus it. A scripted `.click()` on the hidden button still opens the chooser, but picking "11 and up" leaves the bench and Rust on sprout. With `?band=sprout&picker` (age picker shown) and "11 and up" picked in the picker, the bench still opens on the 3-card sprout list. With odd URL bands (`?band=Sprout`, `?band=bogus`, `?band=`), the chooser shows and the child reaches a list. No stuck screen |
| **Mac: pause card behind the bench** | **FIXED.** `#pause` is never visible while the bench is open, whether opened and closed by button and Close, B and Escape, or API and API. `bench-under` is always removed on close. When the game was paused first (card showing), the card is back after closing |
| **Mac: one tile click adds one** | Confirmed. A Seedling Frost N tile tapped 7 times wraps to 2 (0→4→0) and is judged right |

## Break-1 passed list, re-run unchanged (`seam-rerun.txt`, `extra-rerun.txt`)

All pass: A (compositions `ok` at every band), B (`scaleOf` against the judge for 256 sets: 2 `ok`, 0 disagreements), C (no key reaches winit or hud.js, `b` in the box does not toggle, close restores pause and `inert`, no errors), D (no re-learn), E (`__benchState` writes 0 closed and 3 for 3 tries), F (age picker gate, pick reaches Rust, lock after), G (hint gating) and H (kid playthrough, then cast 0 → 1).

Section I crashed at its first step (`page.click('.bench-change')` on `?band=sprout`). This is **expected**: the S5 fix hides that button under a URL band, and the script predates the fix. I re-ran the section without a URL band (`brk2b`). Change age in the middle of a build reaches Rust (`apprentice`) and shows the new list, and Escape on the chooser closes the bench. Both pass.

## New breaks (3 tried)

### S-A (should-fix). A leading-zero subscript is judged right and teaches the spell: `N02(l)` gives "N₂(l) is right! You are learning Frost."

`has_zero_number` rejects a digit run only when it is **all** zeros, so `02` passes, and `read_number` reads it as 2. In the real build (`brk2-run.txt`, N1):
- `N02(l)` → `ok`, learned Frost, "N₂(l) is right! You are learning Frost. | Simplest form: N₂(l)".
- `H02O(l)` → `ok`, learned Water.
- The probe also gives `ok` for `H2O01(l)` and `H2O0000000001(l)`.

The child path is the classic slip of typing the digit 0 for the letter O. A student who thinks liquid nitrogen is "NO2" and types `N02(l)` is told they are right and learns Frost. This is the same class as break-1 S2: a judge-only formula that is not chemistry, judged correct. The Simplest form line softens it. Fix in `bench::judge` (sim lane): treat any subscript digit run that starts with `0` as `Parse` with `bad "0"`, as the round-2 rule already does for a run of zeros. Coefficients with a leading zero (`02N2(l)`) can stay as they are or get the same rule. Add `N02(l)`, `H02O(l)` and `H2O01(l)` to test 3b's zero table.

Lower priority, same place: empty brackets are judged right too (`N2()(l)`, `()N2(l)`, `N()N(l)` and `H2O()(l)` are all `ok`, with the simplest form shown). This is not a chemistry error, but `()` is not notation. Consider making an empty group `Parse`.

### S-B (should-fix). The round-2 double-click guard drops a corrected build, and the panel then says "The scale is not level" under a level scale

`onMake` ignores any Make within 2 s while a try is in flight, whatever text it carries. At Seedling Fire, a child taps Make at 1-1-1-1, sees the beam still tilted, taps O₂ and H₂O (the beam goes level) and taps Make again before the first answer lands. On a slow device the first answer takes a frame of 700 ms or more. The second Make is dropped (n stays 1). The result shown is the first try's `unbalanced`: "The scale is not level. Atoms are never made or lost, …", directly under a beam that is drawn **level** with "Level" (`brk2-run.txt`, N2: `{"n":1,"code":"unbalanced","level":"true"}`). It is not a dead end: one more Make gives `ok` and Fire is learned. But on screen it repeats B1 in reverse (the words contradict the scale), and it is a regression from the S4 change. Fix (web lane): drop a second Make only when its text equals the in-flight try's text. Otherwise send it, or queue it and send it as soon as the result arrives. Also consider clearing the fizzle box when a tile or coefficient changes, so an old reason never sits under a new build (that part predates round 2). Add a golden or node check: Make, change a coefficient, Make in one burst, and the last result shown is for the last build.

### N3 (no finding). Empty builds, the 12-ball cap and tile wrap

- Sprout, Make on an empty tray: `empty`, "Tap the balls to build it."
- Sprout, 14 taps on H: the tray stops at 12. Undo to H₂, add O, Make: "You made Water!"
- Seedling Frost, all tiles at 0: `empty`, "Tap the tiles to build it."
- Seedling Frost, N tile at 2 (after wrapping): right.
- Seedling Frost, all tiles at 4: `atoms_off` with four "Count again" lines.

None of these gives a blank reason, a wrong right or a stuck builder. The first N3 run reported a FAIL because my harness tapped the first tile, which is H, not N. `brk2b` taps `[data-el="N"]` and passes.

## Minor

- **M1.** The `parse` reason for a zero ("Use element symbols, numbers, + and an arrow.") does not tell an Apprentice what is wrong with `0N2(l)`, because they did use numbers. The verdict carries `bad "0"`, so a short line could name it (for example "0 means none. Leave out the 0."). That needs a new player string and register row, so it can wait for I-031.
- **M2.** A scripted click on the hidden "Change age" button still opens the chooser under a URL band. The band stays locked and no child can reach the button, so this is info only. A guard `if (urlBand()) return` in the click handler would make it airtight.

## Hygiene

- The temporary sim probe was appended to `crates/sim/src/bench.rs` in the breaker worktree only and restored from a copy right after. The temporary `tests/e2e/fb-brk.tmp.mjs` was deleted. Neither reached a branch.
- `hud.js` was not touched.
- `/home/claude/bevy-target/debug` (from the sim tests) is deleted.
- The worktree `/tmp/claude-0/brk12-I-028b` and the site copy `/tmp/claude-0/brk12-I-028b-site` are removed after the push.
