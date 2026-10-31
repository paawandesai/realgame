# I-044 rust lane, round 1

Branch `loop/I-044`, base origin/main 4c7c9a1 (code identical to b8fa548).

## What changed
- `crates/bevy-client/src/feel.rs`: `VM_GEM_DEG = 17.6`, `VM_EDGE_MARGIN_DEG = 5.0`, `VM_GEM_MIN_DEG = 4.0` (floor kept, as a guard), private `VM_GEM_DEPTH = 0.42 + 0.11*0.55` (0.4805) and `VM_GEM_RISE = 0.05*0.55` (0.0275) with their derivation commented; `vm_gem_deg(vfov)` and `vm_root_y(vfov)`; unit tests 1-4.
- `crates/bevy-client/src/main.rs`: the two `-0.18` sites (`setup` root spawn, `viewmodel` per-frame translation) now use `feel::vm_root_y(settings.fov_y())`. Bob, recoil, x, z, scale, rotation and children unchanged. Nothing else touched (no gem, `aim_and_cast`, `cues.rs` or I-025 code).

## Tests first
Unit tests 1-4 were added before the functions. On the base they fail to compile (`error[E0425]: cannot find function vm_gem_deg / vm_root_y`, 10 sites).

## Results (plain `rustc --edition 2021 --test`)
| file | result |
|------|--------|
| feel.rs | 11 passed (7 existing + 4 new) |
| cues.rs | 8 passed |
| chem.rs | 19 passed |
| clock.rs | 3 passed |
| motion.rs | 8 passed |

Other checks: `cargo fmt --check` (root and crates/bevy-client) clean; `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` in crates/bevy-client clean; root `cargo test` 185 passed; web build OK (wasm gzip 5 044 475 B vs 5 044 523 B on the base build).

## Gem angle and root y, before / after (from `vfov_for`, the shipped formula)
| aspect | View width | vfov | half-angle | gem before (deg) | gem after (deg) | root y before | root y after |
|---|---|---|---|---|---|---|---|
| 16:9 | 70 | 43.00 | 21.50 | 17.60 | 16.50 | -0.1800 | -0.1698 |
| 16:9 | 75 | 46.69 | 23.35 | 17.60 | 17.60 | -0.1800 | -0.1799 |
| 16:9 | 90 | 58.72 | 29.36 | 17.60 | 17.60 | -0.1800 | -0.1799 |
| 16:9 | 110 | 77.55 | 38.78 | 17.60 | 17.60 | -0.1800 | -0.1799 |
| 4:3 | 70 | 55.41 | 27.71 | 17.60 | 17.60 | -0.1800 | -0.1799 |
| 4:3 | 75 | 59.84 | 29.92 | 17.60 | 17.60 | -0.1800 | -0.1799 |
| 4:3 | 90 | 73.74 | 36.87 | 17.60 | 17.60 | -0.1800 | -0.1799 |
| 4:3 | 110 | 93.93 | 46.97 | 17.60 | 17.60 | -0.1800 | -0.1799 |
| 21:9 | 70 | 33.41 | 16.70 | 17.60 | 11.70 | -0.1800 | -0.1270 |
| 21:9 | 75 | 36.41 | 18.20 | 17.60 | 13.20 | -0.1800 | -0.1402 |
| 21:9 | 90 | 46.40 | 23.20 | 17.60 | 17.60 | -0.1800 | -0.1799 |
| 21:9 | 110 | 62.94 | 31.47 | 17.60 | 17.60 | -0.1800 | -0.1799 |

Golden test 7 measured the same on the shared build (see `lanes/web/round-1.md`).

## Note for the breaker (seam: default view pixel for pixel)
With the spec's formula and `VM_GEM_DEG = 17.6`, `vm_root_y(58.72)` is -0.17992, not exactly -0.18 (the gem's true default angle is 17.608 deg). The root moves up 0.08 mm at 0.42 m, about 0.17 px at 1280x720; state's `vm` prints -0.1799. Test 8 (±0.002) passes. If the seam check demands bit-identical pixels, an AA edge pixel may differ slightly. Pinning -0.18 exactly would need an offset term that pushes unit test 4's error to 0.009 deg of its 0.01 tolerance, so it was not done.

## Blocker found on the shared build
Golden test 7's pixel half fails at 1680x720 / View width 70 (blue 0) and is weak at View width 75 (45). The lifted gem's centre is hidden behind the gauntlet's own gold wrist ring, which a y-only lift cannot avoid below 16.7 deg. See `lanes/web/round-1.md` for the geometry, screenshots and options. The rust code follows the spec as written. Fixing it needs a Contract or child change, which is outside this lane.
