# I-044 rust lane, round 2

Branch `loop/I-044`. Answers `break-1.md`. Build: `CARGO_TARGET_DIR=/home/claude/bevy-target-I025`, `build-web.sh` exit 0, wasm gzip 5 044 585 B (round 1: 5 044 475 B).

## Blocking: the gem hid behind the wrist ring (planner option 1 taken)
A y-only root lift moves the gauntlet straight up, so from the eye the gold wrist ring (local z -0.03) slides in front of the gem; below 16.7 deg it covers the gem's centre. Round 2 takes the breaker's recommended option (1): the whole rest pose **orbits about the eye** by an Rx tilt equal to the lift. A rigid turn about the eye only moves the gauntlet's image on screen and does not change which part hides which, so the gem is as clear of the ring as at the default view.

- `feel.rs`: `vm_root_y` (and its private `VM_GEM_DEPTH` / `VM_GEM_RISE`) is replaced by `pub const VM_REST = [0.21, -0.18, -0.42]`, `pub fn vm_tilt_deg(vfov) = VM_GEM_DEG - vm_gem_deg(vfov)` and `pub fn vm_rest(vfov) -> ([f32; 3], f32)` (the translation `Rx(tilt)·VM_REST` and the tilt in radians). `vm_gem_deg` and the three constants are unchanged.
- `main.rs`, the same two sites as round 1 (`setup` root spawn, `viewmodel` per-frame pose): translation `rest + (bx, by + r*0.015, r*0.05)`, rotation `Rx(tilt) · Rx(r*0.35) · Ry(0.1)`. Bob and recoil are unchanged and stay in camera space. Children, gem, `aim_and_cast`, `cues.rs` and I-025 code untouched.
- **Contract change (needs the planner's sign-off in the spec):** "only root y moves" becomes "the rest pose orbits about the eye by `vm_tilt_deg`". `vm` still means the root's camera-space translation with bob and recoil; no new state keys. The gem in camera space is now `vm + Rx(tilt)·Ry(0.1)·(0, 0.0275, -0.0605)`; golden test 7 recomputes `tilt` from the live fov (round-2 web lane).

## Unit tests (plain `rustc --edition 2021 --test`)
Spec tests 1 and 4 named `vm_root_y`, which no longer exists, so they are restated for the orbit, at least as strict:
- test 1 (`vm_gem_matches_today_at_the_default_view`): `vm_gem_deg(58.72) = 17.6 ± 1e-4` as before; the pose is now **exactly** `VM_REST` with tilt 0 (was -0.18 ± 1e-3).
- test 4 (`vm_rest_puts_the_gem_at_its_angle`): the gem's camera-space point under the orbit pose is `vm_gem_deg(v)` ± 0.01 deg below centre for v in {30, 33.4, 46.4, 58.72, 90}.
- new `vm_rest_is_a_rigid_turn_about_the_eye`: root and gem keep their distance from the eye and from each other (1e-5), tilt >= 0, about 5.9 deg at 21:9 / View width 70.
- tests 2 and 3 unchanged.

| file | result |
|---|---|
| feel.rs | 12 passed (7 existing + 5 I-044) |
| cues.rs | 8 passed |
| chem.rs | 19 passed |
| clock.rs | 3 passed |
| motion.rs | 8 passed |

`cargo fmt --check` (crates/bevy-client) clean; `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` clean.

## Pose by view (from `vfov_for`)
| aspect | View width | vfov | gem (deg) | tilt (deg) | root y | root z |
|---|---|---|---|---|---|---|
| 16:9 | 70 | 43.00 | 16.50 | 1.10 | -0.1719 | -0.4234 |
| 16:9 | 75 / 90 | 46.69 / 58.72 | 17.60 | 0 | -0.1800 | -0.4200 |
| 4:3 | 70 / 75 / 90 | 55.4 / 59.8 / 73.7 | 17.60 | 0 | -0.1800 | -0.4200 |
| 21:9 | 70 | 33.41 | 11.70 | 5.90 | -0.1359 | -0.4363 |
| 21:9 | 75 | 36.41 | 13.20 | 4.40 | -0.1473 | -0.4326 |
| 21:9 | 90 | 46.40 | 17.60 | 0 | -0.1800 | -0.4200 |

Golden read the same `vm` live (round-2 web lane): 1680x720 View 70 `[0.2100,-0.1359,-0.4363]`, View 75 `[0.2100,-0.1473,-0.4326]`.

## Should-fix answers
- **`vm_root_y(58.72)` was -0.17992.** Gone with the orbit: the tilt is 0 whenever `vm_gem_deg == VM_GEM_DEG`, so the default pose is exactly `(0.21, -0.18, -0.42)` and the default view is unchanged bit for bit (test 8 reads `[0.2100,-0.1800,-0.4200]`). For the spec: the gem's true default angle is atan(0.1525/0.4805) = 17.61 deg, which `VM_GEM_DEG` rounds to 17.6, so a lifted gem sits about 0.01-0.03 deg lower than `vm_gem_deg` says (golden reads 11.72 for 11.70). Harmless; inside unit test 4's 0.01 on the rustc model and golden's 3 deg limit.
- **16:9 at View width 70 also moves** (gem 16.5 deg, tilt 1.1 deg). True: "default 16:9 unchanged" holds at View width 75 and above, not 70. The stream start (`main.rs` hand `(0.2, -0.15, -0.5)` in `aim_and_cast`) is still not lifted; at 21:9 / View 70 it starts about 5 deg below the gem. Keep the "stream starts at the lifted hand" follow-up on the backlog (out of this lane by the spec).
