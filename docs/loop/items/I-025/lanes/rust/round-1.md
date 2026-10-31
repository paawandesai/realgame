# I-025 rust lane, round 1

Commits on `loop/I-025`: `ccaaa85` (lane), `0668400` (core gain after the shared-build pixel run).

## Unit tests first

`rustc --edition 2021 --test crates/bevy-client/src/cues.rs` on the stub: the three new tests (`stream_styles_stand_out_and_tell_the_truth`, `a_click_casts_once_and_a_hold_repeats`, `fire_in_view_sees_any_fire_on_screen`) FAILED, I-019's 5 passed. After the lane: 8/8 pass. `chem.rs` 19, `feel.rs` 7, `clock.rs` 3, `motion.rs` 8 still pass.

## Styles (`cues::stream_style`) and their unit-test numbers

All streams 0.22 m thick; outline scale 1.3; mist 10 for Frost and CO2, 0 for Fire and Water.

| spell | core (sRGB) | core L | outline | outline L | outline/sky | outline/core | outline/ground | core/sky |
|---|---|---|---|---|---|---|---|---|
| Water | [0.10, 0.45, 1.00] | 0.196 | [0.03, 0.06, 0.16] | 0.0056 | 10.49 | 4.43 | 2.78 | 2.37 |
| Frost | [0.86, 0.96, 1.00] | 0.875 | [0.06, 0.08, 0.11] | 0.0070 | 10.22 | 16.22 | 2.71 | (1.59, no limit) |
| CO2 | [0.95, 0.96, 0.97] | 0.908 | [0.06, 0.08, 0.11] | 0.0070 | 10.22 | 16.81 | 2.71 | (1.64, no limit) |

Limits: thick 0.18-0.25, scale 1.15-1.35, outline L <= 0.025, outline/sky and outline/core >= 3.0, outline/ground >= 2.0, Water core L 0.175-0.24 and core/sky >= 2.0. All met.

## main.rs

- `STREAM_THICK` and `Beam` (struct, spawn, loop) removed. Frost goes through `spawn_stream`. `StreamsAndBeams` is `With<Stream>` only, so `streams` counts streams, not outline children (`Moving` now takes one `Without`).
- `spawn_stream` uses `kit.cyl` (never mutated), scaled `(thick, len, thick)` by `stream_transform(tail, head, style)`: constant width. Water/Frost/CO2 get one outline child (same mesh, local scale `(1.3, 1, 1.3)`, `outline[spell]`, `NotShadowCaster`/`NotShadowReceiver`). Bevy 0.20's `despawn` removes children (relationship despawn), so no explicit child despawn.
- `SpellMats` gains `stream` and `outline`. Fire's `stream` is `glow[0]`. Cores are opaque, `unlit`, no emissive; outlines are opaque, `unlit`, `cull_mode: Some(Face::Front)`.
- **Deviation found on the shared build (tests 7/8 are the gate):** the camera tonemaps (TonyMcMapface with the default `bloom` feature, also under `?low`), so an unlit 0.95 white core rendered as `(177, 191, 197)`, the same luminance as the untonemapped sky `(140, 199, 250)`. Tests 7 and 8 then saw only the outline rims (Frost/CO2 runs 5-17 px, 0/10 sections). The fix is `STREAM_CORE_GAIN = [1, 1, 6, 10]`, a linear gain on the core colour. Measured on screen: gain 4 gave Frost `(228, 236, 239)`, gain 10 gave CO2 `(247, 247, 247)`; Water at gain 1 shows `(46, 112, 214)`. The core is still not emissive, but in the full tier (HDR + bloom) an overbright core blooms somewhat, as the old 5x emissive glow did. Planner: please confirm this reading of "no bloom haze".
- Mist: `Stream` gains `style`, `mist_acc`, `mist_out`. While alive and unpaused, Frost/CO2 add `48 * clock::frame_dt` per frame and emit `P::Frost` (size 0.25-0.4, life 0.6-1.0 s, velocity up to 0.5 m/s per axis) at random points on the far 70 % of the current length and at least 2 m from the eye (4 tries per particle, then the head). If the head is under 2 m away, nothing is emitted and `mist_acc` is kept. At most `style.mist` (10) per cast. Budget under `?low`: 10 mist + 14 burst + trail (0.7 per frame over <= 0.25 s) stays at or under 40 per cast at 60 fps.
- `fire_in_view`: new block after the finder's `};`, before `game.hover = None;`. Same bearing/elevation/hfov formulas as the finder; I-022's lines are only read.
- Hold: `clicked` is computed with the `clicks` count; after `firing`, `game.held = held_secs(...)` (0 while paused) and `game.was_down = firing && locked && !paused`; the held path also needs `hold_repeats(game.held)`.
- Frost's stream life follows the shared rule (`flight + STREAM_AFTER`, at least `STREAM_MIN_FRAMES` frames).
- Optional native `just_pressed` not done.

## Checks

- `cargo fmt --check`: clean. `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings`: clean. Web build: clean, no warnings.
- Native `cargo check --locked`: builds; one pre-existing warning (`bridge.rs` `WebIn` fields never read on native), so native `clippy -D warnings` fails on origin/main too. Not touched (bridge.rs is out of lane).
- Wasm size (brotli q11, node zlib): stub build 3,538,227 B, this branch 3,539,537 B (+1,310 B; the limit is +5 KB). The stub differs from 05d43a5 only by the neutral stub.
- Lane hygiene: `git diff -U0 origin/main -- crates/bevy-client/src/main.rs` has no hunk inside 91-117, 405-416, 468 or 948-1022 (the `fire_in_view` block is an insertion after 1022). One extra one-line hunk at 41 (`use bevy::render::render_resource::Face;`), which the Overlap table did not list.
