# I-052: retired golden checks

Paawan's rule (2026-11-01, via the coordinator; spec section 11): golden checks that only test the old separate levels (the Frozen Gate, the Marsh, the level-1 layout, the arches) may be retired once the block world replaces those levels, and each retired check is listed in the PR word for word. Checks for the Bench, mastery, worked casts, casting feel, analytics and chemistry rules stay, unchanged, on `classic=1`. CI checks do not change and nothing is loosened.

Removed from `tests/e2e/golden.mjs` by the I-052 contract step: the whole `frozenGate` section (15 checks), the whole `marsh` section (17 checks), their `onlyRuns` entries and call lines, and one worked-casts run that loaded level 2 (1 check). 33 checks in all. Each name below is copied from the source (`check('...')`) of the merge base, not retyped; the column says which new check covers it (spec section 11).

Not retired here: `tests/e2e/perf.mjs`'s level-2 and level-3 workload branches (`--query level=2|3`). The spec lists them for retirement, but the contract step (section 10, item 8) only adds `classic=1`, and with `classic=1` those branches still load the old levels, so they keep working until the follow-up that deletes `level2.rs` and `level3.rs` (proposed I-053).

## The `frozenGate` section (I-035): 15 checks

| # | retired check (word for word) | covered by |
|---|---|---|
| 1 | `frozen gate: the exit stays shut before the win (level 1, exitOpen false)` | retired (arch); no replacement: the block world has no arches |
| 2 | `frozen gate: the level-1 win opens the arch and the banner points to it` | retired (arch); no replacement |
| 3 | `frozen gate: walking into the open arch loads level 2 within 5 s` | retired (level swap); no replacement: one continuous scene |
| 4 | `frozen gate: the liquid stream blocks the walk (stopped on the south bank, z > 8.2)` | ported as E9 |
| 5 | `frozen gate: Fire, Water and CO2 leave the stream liquid; Frost, Water and CO2 leave the gate standing` | ported as M1/M2 and E10/E11 |
| 6 | `frozen gate: Leidenfrost beads after a Frost hit on the liquid stream (>= 4, then 0 within 2 s)` | ported as E14 |
| 7 | `frozen gate: with no band the Apprentice Freezing line shows` | ported as E22 |
| 8 | `frozen gate: three Frost casts freeze the stream 0.34 / 0.68 / 1.00` | ported as M1 and E10 |
| 9 | `frozen gate: the player crosses the frozen stream (z < 3), on ice while on it` | ported as E10 |
| 10 | `frozen gate: Fire melts the gate in 3 hits and walking to the lantern wins` | ported as E11 |
| 11 | `frozen gate: the back arch returns to a fresh courtyard with the exit open` | retired (arch); no replacement |
| 12 | `frozen gate: each band sees its own Freezing line; Sprout shows the freeze picture` | ported as E22 |
| 13 | `frozen gate: level 2 has no more entities than level 1` | replaced by E7 |
| 14 | `frozen gate: 10 level switches leave entities exact and WASM memory flat (± 2 %)` | replaced by E7 (there are no level switches) |
| 15 | `frozen gate: ?level=2 opens the Frozen Gate directly` | replaced by E21 (`?level=2` now opens `?biome=frozen`) |

## The `marsh` section (I-037): 17 checks

| # | retired check (word for word) | covered by |
|---|---|---|
| 1 | `marsh: level 2 forward arch is shut before the lantern (level 2, nextOpen false)` | retired (arch); no replacement |
| 2 | `marsh: the level-2 win opens the forward arch and the banner points to the Marsh` | retired (arch); no replacement |
| 3 | `marsh: walking into the open forward arch loads the Marsh within 5 s` | retired (level swap); no replacement |
| 4 | `marsh: burning gas blocks the walk (stopped south of the dip flames, z > 10)` | ported as E12 |
| 5 | `marsh: Water on a gas vent only knocks it down (7 lit, pool 0, Flicker(Water) line)` | ported as M5 and E12 |
| 6 | `marsh: Frost knocks it down too and Fire changes nothing (7 lit, Frost and Fire lines)` | ported as M5 and E12 |
| 7 | `marsh: CO2 fills the dip (0.5 at a vent, Filling; 1 at the floor, Pooled; 4 lit, still 4 after 5 s)` | ported as M4 and E12 |
| 8 | `marsh: Fire on a smothered vent fizzles (no oxygen), 4 lit` | ported as M5 (Fire on a smothered vent fizzles: no oxygen) |
| 9 | `marsh: the hollow fills in three casts (0.34 / 0.68 / 1.00; 4 lit after two, 0 after three) and both cross` | ported as M4, M4b and E12 |
| 10 | `marsh: walking to the lantern wins ("You cleared the Marsh!") and the walk stops` | retired (level goal); the quest replaces it (E16) |
| 11 | `marsh: the back arch returns to the Frozen Gate with its forward arch open` | retired (arch); no replacement |
| 12 | `marsh: back from the Marsh, level 2 stays solved (stream frozen, gate melted) and the lantern gives no unsolved win` | replaced by the revisit rule (spec section 7) and E16 |
| 13 | `marsh: each band sees its own Filling line; Sprout shows the gas picture` | ported as E22 |
| 14 | `marsh: level 3 has no more entities than level 1` | replaced by E7 |
| 15 | `marsh: 10 switches 1-2-3-2-1 leave entities exact and WASM memory flat (± 2 %)` | replaced by E7 |
| 16 | `marsh: ?level=3 (no debug) opens the Marsh and CO2 can be selected and cast` | replaced by E21 |
| 17 | `marsh: from a direct ?level=3 load, the back arch reaches level 2 with its forward arch open and the gate melted` | retired (arch); no replacement |

## One worked-casts run that loaded level 2 (I-029 / I-035 handoff): 1 check

| # | retired check (word for word) | covered by |
|---|---|---|
| 1 | `worked: handoff: level 2 at Sprout (castmode worked): Fire routes instant, three instant Fire casts melt the gate, and the level is won` | ported as E23 |

The handoff ran inside `run('worked: handoff level 2 at Sprout', ...)`, which reports a failing check under that label if its body throws; that label is removed with it.
