# I-044: An off-centre cursor test, a gauntlet that stays on screen, and no Esc hint flash

**based_on:** origin/main b8fa548796c4b46b72dab0f2955f83bd5de9b060 (I-022 #36 and I-025 #39 merged). Revision 1-2 were written against origin/loop/I-022 at 7b42fc5; every line number below is now at b8fa548 (see `## Refresh (wave 10)`). If main moves again, the builder re-finds the lines by the code quoted here. `main.rs` means `crates/bevy-client/src/main.rs`, `hud.js` means `crates/bevy-client/web/hud.js`, `golden` means `tests/e2e/golden.mjs`.

**Revision 2** (answers `critique.md`; see its `## Planner response`). Changes: test 6 now puts the target off-centre on both axes and uses two wrong-ray controls; test 6 is stated to pass on 7b42fc5; test 3 pins the 4 deg floor; the Out-of-scope line about the floor is corrected.

Source: the I-022 breaker's follow-ups (a) and (b) and the nit (c).

## Problem

1. **(a) A cursor test that cannot fail.** `review: the cursor ray matches the view at 4:3` (check at `golden:2398`, hover read at `golden:2382-2386`, inside test 10 of `reviewFollowups`) puts the cursor at the screen centre (`centre(960, 720)`) and checks that `hover === lit[0]`. `feel::ray_dir(0.5, 0.5, ..)` is `(0, 0, -1)` for any fov and aspect, so a wrong `fov_y` or aspect in `aim_and_cast` (`main.rs:1050-1053`: `let fov_y = feel::vfov_for(settings.hfov, aspect);` and the `ray_at` closure) would still pass. Point-and-click is the main mode for the youngest players, so a regression here would make clicks miss without any test noticing.
2. **(b) The gauntlet leaves the screen.** The viewmodel root is fixed at camera space `(0.21, -0.18, -0.42)`, scale 0.55 (`main.rs:647` in `setup`: `Transform::from_xyz(0.21, -0.18, -0.42).with_scale(Vec3::splat(0.55))`; again every frame at `main.rs:1637` in `viewmodel`: `t.translation = Vec3::new(0.21 + bx, -0.18 + by + r * 0.015, -0.42 + r * 0.05);`). The spell-colour gem is the `SpellGlow` child at local `(0, 0.05, -0.11)` (`main.rs:678-685`), which puts it at about `(0.21, -0.1525, -0.4805)`, **17.6 deg below the centre**. At 21:9 with View width 70-75, the vertical half-angle is 16.7-18.3 deg (`feel::vfov_for`), so the gem is at or past the bottom edge. The gem is the only "which spell is ready" cue in the world view. At 16:9 and View width 90 (half-angle 29.4 deg) it is fine.
3. **(c) A one-frame hint flash on Esc.** `hintAllowed` (`hud.js:460-461`) lets a hint through when `!input.locked && state.casts === 0`. `frame()` (`hud.js:638-728`) picks the hint at `hud.js:685` with `pickHint({ ..., locked: state.locked, ... })` (`pickHint`, `hud.js:48-54`, returns `AIM_HINT` when `locked && extinguished === 0`). Esc clears `input.locked` at once, but `state.locked` is still `true` for the frame until Bevy writes the next state. In that frame `pickHint` returns `AIM_HINT` and the gate lets it through, so with Hints off the aim hint flashes for one frame.

## Bars moved

- **B4 (every age can play):** intended. Panel bar, nothing reported at merge. Predictions for the next panel run: no persona at a wide window reports "I could not tell which spell was ready", and no persona's point-and-click casts miss more often than on the I-022 build. Median clarity does not fall.
- **B5 (fun):** secondary. Median fun does not fall.
- **B1:** stays green. All new checks go into golden.
- **B2, B3, B6:** no change expected (one pure function, no new entities, no text).

## Smallest version

1. **(b) The gem's angle follows the live vertical fov.** In `feel.rs`, add:
   - `pub const VM_GEM_DEG: f32 = 17.6;` (today's angle), `pub const VM_EDGE_MARGIN_DEG: f32 = 5.0;`, `pub const VM_GEM_MIN_DEG: f32 = 4.0;`.
   - `pub fn vm_gem_deg(vfov_deg: f32) -> f32`: `min(VM_GEM_DEG, vfov_deg/2 - VM_EDGE_MARGIN_DEG)`, then at least `VM_GEM_MIN_DEG`. A non-finite or non-positive input returns `VM_GEM_DEG`.
   - `pub fn vm_root_y(vfov_deg: f32) -> f32`: the root's camera-space y that puts the gem at that angle below centre: `-(0.4805 * tan(vm_gem_deg)) - 0.0275`. It is `-0.18` (within 1e-3) whenever `vm_gem_deg == VM_GEM_DEG`. The constants 0.4805 and 0.0275 are named and commented with where they come from (root z −0.42 plus gem z −0.11 × 0.55; gem y 0.05 × 0.55).
   - In `main.rs`, the `-0.18` at 647 (`setup`, which already has `settings: Res<Settings>`) and 1637 (`viewmodel`, which already has `settings: Res<Settings>`) becomes `feel::vm_root_y(settings.fov_y())` (`Settings::fov_y`, `main.rs:114-116`). The value set in `setup` is overwritten by `viewmodel` on the first frame. Bob and recoil are added on top as today. x, z, scale, rotation and every child stay as they are. The 5 deg margin covers the gem's radius (about 1.6 deg), the bob (under 1 deg) and the glow.
   - Effect: at 16:9 and View width 90 nothing moves. At 21:9 and View width 70, the gem sits about 11.7 deg below centre (about two thirds of the way to the bottom edge).
2. **(a) An off-centre hover check** in `reviewFollowups` test 10 (`// 10. View width holds at 4:3 and 21:9, live.`, `golden:2358-2400`), replacing the centre-only hover read (`golden:2382-2386`, `await centre(960, 720)` ... `hoverOk = pm && h.hover === lit[0]`) and its check `review: the cursor ray matches the view at 4:3` (`golden:2398`), at 4:3 (960×720) **and** 21:9 (1680×720), View width 90, in point mode, with **Aim help off** (so the 4.5 deg hover cone cannot hide a wrong ray), restored afterwards.
3. **(c)** `hintAllowed` reads `state.locked`: `(!!state && !state.locked && state.casts === 0)`. This is the same source `frame()` uses. Unlocking can no longer let through a hint `frame()` picked while locked. On lock, the start line can stay one frame longer, but it was already on screen, so nothing flashes.

## Acceptance tests

**Unit tests** (plain `rustc --edition 2021 --test crates/bevy-client/src/feel.rs`). They fail on b8fa548 because the functions do not exist.

1. `vm_gem_deg(58.72)` = 17.6 ± 1e-4, and `vm_root_y(58.72)` = −0.18 ± 1e-3.
2. For every View width 70..=110 (step 1) and aspect in {4/3, 16/9, 21/9}, with `v = vfov_for(h, a)`: `vm_gem_deg(v) <= v/2 - 5 + 1e-4` whenever `v/2 - 5 >= 4`, and `vm_gem_deg(v) >= 4` always. At 21:9 and View width 70, `vm_gem_deg <= 11.75`.
3. `vm_gem_deg` never decreases as `vfov` rises over 10..=120 deg (step 0.5). The floor holds: `vm_gem_deg(10.0) == 4.0` and `vm_gem_deg(16.0) == 4.0` (each ± 1e-6). NaN, −1 and +∞ return 17.6.
4. `atan((-vm_root_y(v) - 0.0275) / 0.4805)` in degrees equals `vm_gem_deg(v)` ± 0.01 for v in {30, 33.4, 46.4, 58.72, 90}.
5. The existing `feel.rs`, `cues.rs`, `chem.rs`, `clock.rs` and `motion.rs` plain-`rustc` tests still pass.

**Golden checks** (new or replaced checks in `reviewFollowups`, run as `QUERY=?low node tests/e2e/golden.mjs`). The lane runs them on b8fa548 first and records the results. Tests 7 and 9 must fail there (for test 9, the observer check if the flash can be caught, otherwise its source check; see its note). Test 6 **passes** on b8fa548, because I-022 already builds the ray from the live `vfov_for(hfov, aspect)` (`main.rs:1050-1053`). Its evidence that it can fail is its two wrong-ray controls: the lane records each control's distance from `P` and its hover read on b8fa548. If a control lands on the target, the lane picks another pose (below) and does not loosen the check.

6. **`review: the cursor ray matches the view off-centre at 4:3 and 21:9`** (replaces `review: the cursor ray matches the view at 4:3`). For each viewport, 960×720 and 1680×720:
   - Point mode, `#assist` off, View width 90. Pose: `tp 0 5.5`, `face lit[0]` (the bush about 16 m away). Read `yaw0`, `pitch0` and `cam` from state; the target point `T` is `cam` plus 16 m along the direction at `yaw0`, `pitch0` (the point `face` aimed at). Only rotation follows, so `cam` does not move and the exact distance does not change where `T` projects.
   - **Off-centre on both axes.** Move the view so `T` sits near **(0.75W, 0.30H)**. At View width 90 the horizontal half-tan is 1 at every aspect up to 4, so that spot is camera-space direction `(0.5, 0.4·tan(vfov/2), −1)`. Start from `look (yaw0 ∓ atan 0.5) (pitch0 − atan(0.4·tan(vfov/2)/√1.25))` in degrees (use `look`, not `turn`, so mouse sensitivity does not matter; the yaw and pitch signs are whichever put `T` right of and above centre). Then read `yaw`, `pitch`, `cam` and `fov` from state, project `T` to `P` (perspective, `vfov = state.fov`, `aspect = W/H`), and correct the yaw and pitch by the remaining angular error, at most 3 rounds. Require `|P.x − 0.75W| <= 0.05W` and `|P.y − 0.30H| <= 0.05H`.
   - **Hit:** cursor to `P` → within 10 frames, `hover === lit[0]`.
   - **Wrong-ray controls.** Each control finds the pixel `P'` whose ray, built the wrong way, points at `T`, i.e. it inverts `ray_dir(fx, fy, vfov_c, aspect_c)` for `T`'s camera-space direction and maps `(fx, fy)` to pixels:
     - **(i) Stale vfov, live aspect:** `vfov_c = 58.72` (a 16:9 window at View width 90), `aspect_c = W/H`. Expected shift: about 80 px in x and 48 px in y at 4:3, about 100 px in x at 21:9.
     - **(ii) Live vfov, 16:9 aspect:** `vfov_c = state.fov`, `aspect_c = 16/9`. Expected shift: about 60 px in x at 4:3, about 130 px at 21:9.
     - For each control require `|P' − P| >= 25 px`, and with the cursor at `P'`, `hover !== lit[0]` within 10 frames. If a control fails on this pose, try `tp 0 12` with `face lit[2]`, then `face lit[1]`, with the same off-centre placement. Log the pose used. If no pose passes both controls, the check fails.
   - The log line gives the viewport, the fov, the pose, `P`, each `P'` with its distance from `P`, and the hover read at each.
7. **`review: the gauntlet's spell gem stays on screen at 21:9`.** Locked or point mode, `set sway off`, no cast for 1 s (recoil 0) and `streams === 0` (I-025's Water stream is blue and starts at the hand; it lives at most `flight_secs` + `STREAM_AFTER` = about 0.45 s), Water selected, `look Y -25` (ground behind the hand). For each of (1680×720, View width 70), (1680×720, View width 75) and (960×720, View width 70):
   - **Geometry:** with `g = vm + Ry(0.1)·(0, 0.0275, -0.0605)` (the gem in camera space), `atan(-g.y / -g.z)` <= `state.fov/2 - 3` deg.
   - **Pixels:** hide `#hud` and `#pause` with CSSOM and take a screenshot. Project `g` to pixel `G`. In a 15×15 box at `G` (clipped to the viewport), at least 10 pixels have `B >= R + 40 && B >= 120`. In the mirrored box at `(W - G.x, G.y)`, fewer than 10 do (so the blue is the gem, not the scene). Restore the HUD.
   - On b8fa548 at 1680×720 and View width 70, `G` is at or below the bottom edge and the box count is under 10.
8. **`review: the gauntlet does not move at the default view`.** At 1280×720, View width 90, sway off, at rest: `vm` is `(0.21, -0.18, -0.42)` ± 0.002 per axis. This passes on b8fa548 and guards the default look.
9. **`review: Esc with Hints off never flashes the aim hint`.** Hints off, `fresh()`, locked, `casts === 0`, `extinguished === 0`. Install a `MutationObserver` on `#hint` (childList, characterData, subtree, attributes) that records every non-hidden `textContent`. Press Esc (real `keyboard.press('Escape')`, so `pauseNow` runs). Wait 15 frames. Require that `'Aim at a fire and click.'` was never recorded, then restore Hints and the lock.
   - **Note:** if the flash cannot be caught on b8fa548 in 5 tries, the lane records that. It then adds a source check in golden, which reads `hud.js` with `fs` and fails if the `hintAllowed` definition (from `const hintAllowed` to the next `;`) contains `input.locked`. The observer check stays either way.
10. All existing golden checks pass, including I-022's View width, Aim help and Hints checks and I-025's `streamsAndFinder` checks, 3/3 runs. Main's golden has 124 checks; after this item it has 127 (test 6 replaces one, tests 7-9 add three), or 128 if test 9's source check is added.

## Out of scope

- The stream's start point (camera-space hand `(0.2, -0.15, -0.5)` at `main.rs:1204` in `aim_and_cast`, now also the start of I-025's stream rod via `spawn_stream`) is not lifted with the gauntlet. At 21:9 with View width 70 the stream starts about 5 deg below the gem. Named follow-up: "stream starts at the lifted hand".
- Aspects wider than 21:9 (the forearm may leave the screen), portrait windows, and the rune bands (at 21-26 deg below centre they may leave the screen; the gem is the cue).
- Any other hint logic: priority order, `pickHint` (I-025), the idle hint, `note`.
- I-022 lines other than those named here; I-025's merged code (`cues.rs`; in `main.rs` `Stream`, `stream_transform` 251, `spawn_stream` 266, `SpellMats`, the projectile/stream block of `aim_and_cast`, the finder, the state format string; in `hud.js` `pickHint` 48-54, the finder and the rest of `frame()` 638-728; `__game`; golden's `streamsAndFinder`).

- The 4 deg floor in `vm_gem_deg` is a guard only. `vfov_for` clamps aspect to 4 and the smallest View width is 70, so at aspect 4 the gem sits about 4.9 deg below centre and the floor is never reached with supported settings. The builder may keep the floor or drop it; if dropped, unit test 3's floor asserts and test 2's `>= 4` clause go with it.

## Contract

No new state keys. `vm` keeps meaning the viewmodel root's camera-space translation, including bob and recoil. The gem's local offset `(0, 0.05, -0.11)`, the root's scale 0.55 and its rest rotation `Ry(0.1)` stay fixed, so golden's `g = vm + Ry(0.1)·(0, 0.0275, -0.0605)` holds. Golden test 7 relies on this.

## Lanes

| lane | area | owns | builds |
|------|------|------|--------|
| rust | client-core | `crates/bevy-client/src/feel.rs` (append the constants, the two functions and unit tests 1-4), `crates/bevy-client/src/main.rs` (the two `-0.18` sites at 647 and 1637 only) | Unit tests first, then the wiring. Builds native and wasm. Must not touch the gem or children, `aim_and_cast`, `cues.rs`, or any I-025 code |
| web | client-web, tests-e2e | `crates/bevy-client/web/hud.js` (`hintAllowed`, 460-461 only), `tests/e2e/golden.mjs` (inside `reviewFollowups`: the hover lines of its test 10 block, 2382-2386 and 2398, and the spec's new tests 7-9 as a new block after it, before the closing `await fresh()` at 2401) | Golden tests 6-9 first, on b8fa548, recording the failures of 7 and 9 and test 6's control distances and hover reads; then the `hintAllowed` fix. Test 7 goes green on the shared build |

**Seam:** the breaker checks that the gem stays visible at 21:9 with View width 70 while walking (sway on) and right after a cast (recoil). It also checks that the default 16:9 view is unchanged pixel for pixel in the hand region.

## Refresh (wave 10)

Stale check against origin/main b8fa548 (I-022 #36 with the 24 h session-id cap in `analytics.js`; I-025 #39). Main has since moved to 4c7c9a1 with docs-only changes (`docs/loop/backlog.md`, `docs/loop/log.md`), so nothing below differs there. Scope is unchanged; `feel.rs` is unchanged since 7b42fc5.

- `based_on` set to b8fa548796c4b46b72dab0f2955f83bd5de9b060; the "start only after #36 merges" gate is dropped (merged). All "on 7b42fc5" baselines now read "on b8fa548".
- Viewmodel root `-0.18` sites: 585 -> **647** (`setup`) and 1539 -> **1637** (`viewmodel`). Both functions already take `settings: Res<Settings>`; `Settings::fov_y` is at 114-116.
- Gem (`SpellGlow` ball child): 617-623 -> **678-685**. Offset, scale and rest rotation unchanged, so the Contract's `g` formula holds.
- `aim_and_cast` live-fov ray: 988-991 -> **1050-1053**. The hand `(0.2, -0.15, -0.5)` is at **1204** and now also feeds I-025's `spawn_stream` (Out of scope wording updated; follow-up unchanged).
- `hud.js`: `hintAllowed` 444-445 -> **460-461** (code unchanged, bug still present). `frame()` 669-675 -> **638-728**; the hint is now chosen by I-025's `pickHint` (48-54) at **685** with `locked: state.locked`, so the one-frame flash path is the same. The fix is unchanged.
- `golden`: `reviewFollowups` starts at 2205; its test 10 block is 1923-1964 -> **2358-2400**; the hover read is **2382-2386** and the check **2398**; the block's closing `await fresh()` is at 2401. The web lane's new block goes after test 10. Helpers `within`, `centre`, `setCtl`, `vfovAt` are at 2219, 2275, 2210, 2277.
- Golden count: main has 124 checks; this item makes 127 (128 with test 9's source check). Test 10 (all checks) now also names I-025's `streamsAndFinder`.
- Test 7 adds a `streams === 0` precondition, because I-025's blue Water stream starts at the hand next to the gem (it lives about 0.45 s, so the existing 1 s no-cast wait already satisfies it; this only makes it explicit).
- Out of scope / lane bounds: "I-025's ranges" replaced with I-025's merged code by name (`cues.rs`, `Stream`, `stream_transform` 251, `spawn_stream` 266, `SpellMats`, the state format string, `pickHint`, the finder, `streamsAndFinder`).

