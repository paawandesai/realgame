# I-024 render lane, round 1: Part R (one shared material for palette scenery under `?low`)

**Outcome: reverted.** Test 8's hard check failed in three of three paired GL counts: texParameteri fell by 21-23%, not the required 30%. Per the spec, the counts are recorded, the code is reverted, and no atlas of textured materials is attempted. R was therefore not taken to a `perf-pairs` verdict (test 10); there is no `evidence/R-pairs.json`.

- Base: origin/main c4832d2 (code-identical to a0e2ecb: c4832d2 only adds `docs/loop/` files), built once into `<scratchpad>/I-024/base-pkg-out`.
- R build: commit de4648f (`world::build(.., low)`, `LowShare`, end-of-build post-pass skipping every `Target` entity, vertex-colour copies of kit meshes, shared white `toon`). Incremental build 300 s, wasm raw 17 694 046 B vs base 17 679 322 B (+14.7 KB).
- Revert: the branch's `world.rs` and `main.rs` are back to origin/main. The implementation stays in history at de4648f for the follow-up, and the evidence script stays at `lanes/render/pixels.mjs`.

## Test 8: GL calls per frame, `PERF_GL_COUNT=1 ONLY=low@1x RUNS=1`

Same box, serial under the Chromium lock, alternating base and R (`evidence/R-gl-count.txt`). Other agents were building on the box during these runs, but GL counts per frame do not depend on load.

| run | side | frames | GL calls/frame | draws/frame | texParameteri | bindSampler | bindTexture |
|---|---|---|---|---|---|---|---|
| 1 | base | 43 | 2 109.2 | 45.51 | 501.1 | 267.3 | 251.6 |
| 1 | R | 55 | 1 880.0 | 43.05 | 387.2 | 210.2 | 194.6 |
| 2 | base | 56 | 2 107.6 | 45.68 | 504.1 | 268.6 | 253.1 |
| 2 | R | 54 | 1 903.8 | 43.83 | 399.3 | 216.1 | 200.7 |
| 3 | base | 57 | 2 093.3 | 45.39 | 499.5 | 266.3 | 250.8 |
| 3 | R | 56 | 1 869.1 | 42.88 | 387.0 | 210.0 | 194.6 |

| check | limit | measured (median of 3) | result |
|---|---|---|---|
| R texParameteri / base | ≤ 0.70 | 387.2 / 501.1 = **0.773** (runs: 0.773, 0.792, 0.775) | **fail** |
| R draws vs base | within 5% | 43.05 vs 45.51 = **−5.4%** (runs: −5.4, −4.0, −5.5) | fail (marginal) |
| GL calls/frame (recorded) | expect ~1 300 | 1 880 (−10.8%) | — |
| `LOW_SHARED_MAT` once at `?low` | `colours=10`, `meshes ≤ 40` | `colours=10 pieces=91 meshes=14 candidates=91 target_skipped=0` | pass |
| no line at the default tier | absent | absent | pass |

Re-measured base on this tree (post-I-019): 501 texParameteri and 45.5 draws per frame, against I-015's pre-I-019 440 and 39.8. The gate was taken against the re-measured figure, as the spec requires.

**Why the hypothesis fell short.** The 91 swapped palette pieces removed about 114 texParameteri per frame. The other ~387 most likely come from material switches the spec left alone by design (not separately counted here): every chemistry object has its own `StateMats` (one material per object and state), plus the textured grass and walls, `glow` materials and the transparent glass. No post-pass entity was referenced by a `Target` (`target_skipped=0`): chem objects are spawned with their own `StateMats.normal` handles, not with the shared palette handles. So the remaining bind-group changes are probably dominated by per-object chem materials. A follow-up that wants the other 70% would have to share `StateMats` across objects of the same kind and state. That is a runtime look change, out of this item's scope.

## Test 9: picture unchanged (guard, recorded for the follow-up)

`node docs/loop/items/I-024/lanes/render/pixels.mjs <base> <R> <tier> <out>`, 960×540, game paused, all DOM above the canvas hidden. Mean absolute difference per channel (0-255):

| pose | `?low` base vs R (≤ 0.5) | default base vs R (≤ 0.2) | `?low` base vs base (noise) |
|---|---|---|---|
| spawn | 0.0437 | 0.0721 | 0.0343 |
| greenhouse | 0 | 0.0223 | 0 |
| lab-bench | 0 | 0.0141 | 0 |
| hedge-row | 0 | 0.0097 | 0 |
| courtyard-corner | 0.0418 | 0.0533 | 0.0311 |

State round trip at `?low` on R (bush object 4 at (12, −6.5), framed with the greenhouse behind it): Normal vs Wet 0.2557 (the Wet look registers), Normal vs back to Normal **0.0252** (≤ 0.5, pass). `pieces` (91) = palette pieces (91) − Target-referenced (0). No console errors on either side. Raw output: `evidence/R-pixels-{low,default,noise}.json`.

Test 9 passed: the vertex-colour × white construction kept the picture the same. The part failed on the size of the GL saving, not on the picture.

## Tests 10 and 11

Not run for R. The spec gates R on test 8 ("If test 8 shows no 30% drop, it reverts and records"), so R is not a candidate in the final combined set.
