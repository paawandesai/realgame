# I-035 break round 1 (Breaker, wave 13)

**Verdict: no blocking defects.** Level 2 plays from start to finish. I found no way to get stuck in it and no way to win without solving it. Level 1 and the Formula Bench still work after switching levels.

Build: detached worktree at origin/loop/I-035 edef893, `build-web.sh` (wasm32, release). Every browser run held the chromium lock, with `?low`. Evidence is in `evidence/break-1/`: `probe.mjs` (my probes), `probe-run.txt`, `golden-formulaBench-frozenGate.txt`, and two screenshots.

## What I ran

| check | result |
|---|---|
| golden `formulaBench` + `frozenGate` sections only (a local, uncommitted `ONLY=benchGate` switch that calls the two sections unchanged), `QUERY=?low` | **63/63**, no console errors |
| breaker probes (`probe.mjs`, 10 groups) | 0 console errors, 0 panics |

## Tries that failed to break it (good)

- **Stuck in the stream.**
  - Walking into the liquid stream stops at z 8.35. So do jump spam while running at it and a diagonal at the landing edge (x 2.05).
  - The rock banks east and west stop me at z 10.15, the wall corners included.
  - Freeze the bridge, slide sideways on the ice to x −14.9, then cast Fire straight down: the stream goes solid at ice 0.66 and I am moved to the landing (−2, 8.4), outside every collider. Walking −z is blocked again and walking +z works.
  - Standing mid-bridge at z 5.9 when Fire lands moves me to the north landing (0, 3.6).
- **Behind the gate.** From the north landing the gate stops me at z −7.15. The gate edge and the ridge's east corner stop me at z −7.05. I found no way around the ridge.
- **Win without solving.** The lantern can be reached only through the melted gate. After the win, casts change nothing and R gives a fresh level 2 (stream ice 0, gate standing, start pose).
- **Wrong chemistry.** Fire, Water and CO2 on the stream leave ice at 0. Frost on a melted gate makes a frozen puddle, which is still walkable. Nothing soft-locks.
- **Between levels.** Switching levels with a Fire projectile in flight, and with a Water stream held down, gives no panic and no errors, and leaves 0 streams. `?level=2` without `debug` loads level 2. `?level=3`, `?level=abc` and `?level=` all load level 1. The `level` command is refused without `?debug`.
- **Formula Bench.** In level 2, standing on the pad's spot gives `atBench` false. Back in level 1 the pad gives `atBench` true. The bench's golden section passes in the same run as the new section.
- **Captions per band.** I checked Sprout (Freezing, Melting/Melted on the stream, Steam, NoChange, Soaked, each with the right icon), Seedling and Adept. Every line matches the spec table. Science: nothing wrong or scary. "196 K colder", "6.01 kJ/mol" and "0 °C (273 K)" are all correct, and no line says liquid nitrogen is safe to touch.
- **Perf at `?low`** (idle STATS, same build, one sample each):

  | | level 1 | level 2 |
  |---|---|---|
  | entities | 633 | 458 |
  | CPU p50 (ms) | 5.0 | 5.6 |
  | wasm memory (MB) | 33.6 | 33.6 |

  This matches the builder's 5-pair result (within the noise band).

## Should-fix (not blocking)

1. **The hint tells you to find a fire in level 2.** While the mouse is locked, level 2 shows "Aim at a fire and click." (`pickHint` in hud.js: `locked && extinguished === 0` gives `AIM_HINT`, and `extinguished` is always 0 in level 2). There is no fire in level 2. A 6-year-old reading it is sent looking for something that is not there. The builder already blanks the idle hint for level 2; the AIM hint needs the same, or a level-2 line ("Aim at the stream and click."). This line is in `frame()`/`pickHint`, outside the pointer-lock block.
2. **The branch no longer has the latest I-028.** I-028 merged to origin/main (PR #47) with its round-2 fixes, and loop/I-035 holds only I-028 fed49a9. Merging origin/main conflicts in `content/science-sources.md`: the C-045 and C-047 rows were reworded on main. After resolving it, the bench and frozenGate sections must be re-run on the merged build.
3. **The win screen is jammed into the lantern.** If W is still held at the win, the player walks on into the plinth, and the camera sits inside the lantern ball and roof (`evidence/break-1/p6-won.png`). Possible fixes: stop the walk on the win, or give the lantern a collider about 1 m wide instead of 0.8 m.
4. **The Sprout melt line is wrong on a part-frozen stream.** Fire on a stream that is only a third frozen says "The bridge melted! Make it cold again." No bridge was there yet. A softer line ("The ice melted! Make it cold again.") would fit both cases.
5. **The aim picks the lantern through the standing gate.** `face lantern` reports `aimObj` = lantern, but the cast lands on the gate (Fire then melts it). The kid sees the lantern targeted while the gate takes the hit. It is harmless, even helpful, but confusing. Level 1 may pick targets the same way already.
6. **Water on the lantern implies it can burn.** By code reading, Water on the inert lantern gives the level-1 Soaked line, "it must dry out ... before it can burn". The spec says spells on the lantern say nothing new. A `(Soaked, Lantern)` row, or a "That does not burn." line, would fix it.
7. **Snow mounds and boulders have no colliders.** You can walk through the mounds at (7, 12) and (−8, 19), and the camera ends up inside a white ball. This is cosmetic.
8. **`?level=2`, then the back arch, leaves the courtyard exit shut** (`exitOpen` false until level 1 is won). This matches the spec ("cleared lasts one page load"). Teachers who share `?level=2` links should know it.
9. **Holding S at the arch may bounce between levels.** Arrival at z 18 is only 3.2 m from the arch and you arrive facing away from it. A kid holding S could go back and forth every 0.7 s. A scripted `move` stops at the switch, so I could not reproduce this with real keys. A short arrival grace time, about 1 s before an arch triggers, would rule it out.
10. **The Leidenfrost source is Wikipedia.** [S24] cites Wikipedia rather than the spec's Quéré review, because the proxy refused doi.org and annualreviews.org. The science reviewer should accept or replace it.
