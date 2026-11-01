# I-028 round 2 (fixer: sim, client and web lanes)

Base: `origin/loop/I-028` at fed49a9 (break round 1). Inputs: `break-1.md` (1 blocking, 5 should-fix) and
the real-Chrome check on Paawan's Mac (384b910: `#pause` behind the bench; an atom tile click focuses and
adds 1). Paths touched: `crates/sim/src/bench.rs`, `crates/bevy-client/web/bench.js`,
`crates/bevy-client/web/style.css` (inside the bench block), `test/bench.test.ts`, `tests/e2e/golden.mjs`
(`formulaBench` body only, appended; 0 deleted lines), `content/science-sources.md` (two rows and one
source line). `hud.js`, `index.html`, `age.js`, `formula.rs`, `main.rs`, `bridge.rs` and the client
`bench.rs` are untouched. No acceptance test was edited or weakened. Checks were only added.

## Fixes, test first

| finding | red first | fix |
|---|---|---|
| **B1** flat beam on an unbalanced equal-total Fire build | Node: `round 2 (B1)` x2 failed (`scaleOf(1221).tilt` was 0; no per-element marks) | `scaleOf`: if `!level` and the totals are equal, `tilt` = sign(right - left) of the **first mismatched element in recipe order** (1221 tilts right, toward C 2). So `tilt === 0` exactly when `level`. Test 9 still holds: sign(total) is kept whenever the totals differ. The pans now draw each element as `.bench-pan-el[data-el][data-ok]`, and a mismatch gets a dashed underline and a small cross shape (CSS, not colour alone). The pan text is unchanged (`C 1, H 4, O 4`). "more atoms" shows only when the **totals** differ, so at 1221 neither pan claims more atoms. The beam tilts and the marks show which elements differ. Node checks: all 256 sets give level iff tilt 0, with exactly the 9 equal-total cases the breaker listed. A new golden check at 1-2-2-1 covers `data-level="false"`, `data-tilt != 0`, a computed transform that is not identity, no "more atoms", and 6 marks |
| **S1** zero subscript gives `atoms_off` with empty diffs | Sim: the fuzz with `0` and `1` added to the alphabet, plus the new invariant (`AtomsOff`/`Unbalanced` means `diffs` is not empty), failed. Node `round 2 (S1)` failed (blank line) | `judge`: any zero subscript in a parsed formula (a digit run worth 0 in `Term.formula`: `N2O0(l)`, `CO2H0(g)`, `H2O1H0(l)`) is `Parse` with `bad = "0"`. `reasonFor`: `atoms_off` with no diffs falls back to `Not yet. Which atoms does ${name} have, and how many?` (Seedling and Apprentice from 3). `unbalanced` with no diffs at Apprentice falls back to the C-044 line. No new player string was added |
| **S2** zero coefficient judged `ok` | Sim test 3b gained a 12-row zero table, which failed | `judge`: a term whose coefficient is worth 0 (`0`, `00`, `0/1`, `0.0`, from the normalized text, because `formula.rs` reads 0 as 1 and the sim lane may not edit it) is `Parse` with `bad = "0"`. Covered: `0N2(l)`, `00N2(l)`, `0/1N2(l)`, `0.0N2(l)`, Sprout and Seedling `0H2O`, Fire `0CH4 + 2O2 -> CO2 + 2H2O`, `CH4 + 2O2 -> 0CO2 + 2H2O`, and all-zero. Positive controls: `1N2(l)`, `H2O1(l)` and `C1O2(g)` are `ok`, and `10H2O(l)` is `ok` with `coef 10`. Golden: Apprentice `0H2O(l)` is `parse` and Water stays unknown |
| **S3** C-047 and C-045 citations | n/a (the lint passed before too) | New source **[S24]**: NIST WebBook CO₂ phase-change data (Ttriple 216.58 K, Ptriple 5.185 bar; fetched 2026-11-01, values read from the page). C-047 now cites [S24] [S10], with the reasoning in the note: the triple point is above 1 atm, so CO₂ has no liquid at 1 atm, and room temperature is far above 216.58 K. C-045 now cites [S2] [S10]: 273.15 K melting [S10], 373.17 K boiling [S2]. The science lint shows 42 claims, 42 sourced, 0 errors |
| **S4** misses count every fizzle; double-clicked Make | Golden check (new) | `showFizzle` counts a miss only for `atoms_off` and `unbalanced` (the 2D rule, "three misses of the atoms"). `state_missing`, `wrong_state`, `parse` and the rest no longer bring the counts early. `onMake` ignores a second Make while a try is in flight. The guard lapses after 2 s, so a lost result can never lock the button. Test 19's "three `HO(l)` tries" behaves as before. **Spec note for the planner:** section 3 says "third wrong try". This build reads it as "third wrong try of the atoms" |
| **S5** `?band=` overridable by "Change age" | Golden check (new) | `resolveBand` puts the URL band first, ahead of a chooser pick too (spec section 2 order), and "Change age" is `hidden` when the URL sets a valid band. Tests 16 and 28a run without a URL band and are unaffected |
| **Mac: `#pause` behind the bench** | Golden check (new) | `openBench` adds the class `bench-under` to `#pause` and `closeBench` removes it. `style.css` has `#pause.bench-under { display: none; }`. `hud.js` is **not** edited: it still toggles `.hidden` as before, and the extra class only hides the card while the bench is open. Test 15 is unaffected (the card is back after closing a paused game) |
| **Mac minor: a tile click focuses and adds 1** | n/a | **Left as is, by the spec.** Section 3 says a tap on a Seedling tile (and a Sprout atom card) adds one. A click focuses a `<button>` in Chrome and adds exactly one. The focus ring is `:focus-visible` only, so a mouse click draws no ring. Space and Enter fire one native click (the bench does not also handle them), so nothing double-adds. A new golden check covers it: one click on a Seedling tile gives count 1 |

## Results (on 95223ab, the fix commit; this note and its evidence follow it)

| gate | result |
|---|---|
| `cargo fmt --all --check` (root) and `cargo fmt --check` (client) | clean |
| `cargo clippy --all-targets --locked -- -D warnings` (root) | clean |
| `cargo clippy -p realgame-sim --all-targets [--no-default-features] --locked -- -D warnings` | clean, both feature sets |
| `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` (root and `crates/bevy-client`) | clean |
| `cargo test --locked` | 217 passed |
| `cargo test -p realgame-sim --no-default-features --locked` | 61 passed (bench 8/8) |
| `npm test` | 85/85. Science lint: 42 client claims, 42 sourced, 0 error verdicts. `npm run typecheck` clean |
| `build-web.sh` | ok (wasm gzip 5 103 585 B) |
| golden `formulaBench` section alone (a temporary copy of golden that skips the earlier sections; removed afterwards) | **55/55** (48 existing + 7 new round-2 checks): `evidence/round-2/formulaBench-only.txt` |
| full golden, `BASE_URL= QUERY=?low`, under the Chromium lock | **192/193**: `evidence/round-2/golden-full.txt`. The one FAIL is `cast: a real click casts exactly once` (`casts 1 -> 2 -> 2 (20 frames on), lit[1] burning=true`). The cast count is right (one click, one cast). The check also needs the fire out within 20 frames, and this run was at fps 1.3-2.9. That is the known slow-GPU flake (I-019, I-025 break-1, I-044 break-1, the I-028 web-lane stub run), in a section this item does not touch. All 54 `bench:` checks pass |

The new golden checks (all in `formulaBench`, appended after 28b):
`bench: round 2: at 1-2-2-1 the beam is not level and is drawn tilted, no pan says more atoms, the mismatches are marked`,
`… one click on a Seedling tile adds exactly one`,
`… #pause is not visible while the bench is open, and is back after closing a paused game`,
`… with ?band=sprout there is no Change age and the band stays sprout`,
`… a double-clicked Make is judged once`,
`… state slips do not count toward the third-try help`,
`… 0H2O(l) is not judged right`.

Hygiene: `/home/claude/bevy-target/debug` (made by the sim tests and clippy) was deleted after use. The
temporary section script `tests/e2e/fb-only.tmp.mjs` was deleted and never committed.
