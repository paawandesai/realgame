# I-007 lane main, round 1 (builder)

Branch `loop/I-007`, based on origin/main a94fc48. Spec and critique copied from the planner's I-007 folder (they replace the old r1 spec here).

## What was built

- **`src/feel.rs`** (new, no Bevy types): `LOOK_PER_PX = 0.00095`, `DEFAULT_HFOV = 90`, `vfov_from_h`, `clamp_hfov` (70 to 110), `clamp_sens` (0.25 to 3), `cm_per_360`, `assist_cone` (full to 20 m, 0 at 60 m), `ray_dir`. `player::LOOK_PER_PX` re-exports it.
- **`chem::HitKind` + `chem::hit_kind`** as the spec defines them.
- **`main.rs`**:
  - `Settings` gains `hfov` and `sway`. The fixed `FOV_Y` is gone: the projection and the `tap`/hover ray both read `settings.fov_y()`, the ray through `feel::ray_dir`.
  - New `set sens|hfov|sway|assist` commands. They are not debug-only.
  - Assist cone falloff in `pick`.
  - The sway gate in `viewmodel` makes the bob exactly 0 when off. The recoil stays.
  - `projectiles` records `hit_kind` and the impact's screen point. It uses `world_to_viewport` from this frame's eye and rotation, ÷ the logical viewport.
  - New state fields: `fov, sway, cam, vm, cooldown, cooldownMax, hitKind, hitX, hitY, hitFrame`. `pos` now has 3 decimals and `yaw`/`pitch` have 2 (more precision; nothing reads them as strings).
  - URL `?sens=` is now clamped to the slider range of 0.25 to 3 (was 0.05 to 10).
- **`hud.js`**:
  - A settings block in `#pause .card` before `#resume`: `#sens/#sens-val`, `#fov/#fov-val`, `#sway`, `#assist`, `#hints`, `#sound`, `#textsize`.
  - Settings persist in `localStorage['alchemist.settings.v1']`, with every access in try/catch. Precedence is URL > saved > default. The effective values are sent once, on the first state.
  - Crosshair `cooling` + `--cd` ring, and `pulse`.
  - `#hitmarker` (`.hm-x`, `.hm-slash`, `.hm-star`) at the centre when locked and at `hitX/hitY` when free. It carries a `data-frame`.
  - WebAudio tones: cast, hit, fizzle, out, miss. Peak gain is at most 0.15. Test hook `__game.lastSound()` returns `{name, n, played, peak, frame}`.
  - Idle hint and Water-slot `nudge` after 45 s. The idle clock restarts on progress, reset, pause and resume.
  - `--ui` text scale.
- **`style.css`**:
  - All HUD and pause text is `calc(>=18px * var(--ui))`.
  - The hotbar wraps, and slots size to their formula.
  - `#hint` sits on `--panel`. The banners are darker.
  - Controls: checkboxes 28 px, ranges 28 px tall, buttons at least 44 px tall.
  - Reduced motion turns off pulse, nudge and the ring transition.
- **`content/science-sources.md`**: a non-claim row for the idle-hint line.
- **`tests/e2e/golden.mjs`**: a `feelAndStandards` section (spec tests 6 to 19, 36 new checks). It runs after the CPU stats print and before `no console errors`. Existing checks are untouched.

## Tests first

On the base code, before implementing:
- `rustc --test src/feel.rs`: 34 compile errors (functions missing).
- `rustc --test src/chem.rs`: 12 errors (`hit_kind`/`HitKind` missing).

The base build (a94fc48 source, served from the scratchpad) was probed for what the new e2e checks read:

```
missing state fields: fov,cam,vm,sway,cooldown,cooldownMax,hitKind,hitX,hitY
missing ids: sens,sens-val,fov,fov-val,sway,assist,hints,sound,textsize,hitmarker
lastSound hook: undefined
turn 100 px at sens 1: 12.60 deg (new test wants 5.443 +- 0.05)
font sizes: .slot .key=12px .slot .name=15px .slot .formula=14px #obj-line=15px #hint=16px
#hint background: rgba(0, 0, 0, 0)
```

A full golden run on the base build did not reach the new section. The headless renderer crashed (Target crashed, renderer RSS about 5 GB) under pointer lock while the box had a load average of about 20.

## Measured results (final code)

| check | result |
|---|---|
| `rustc --edition 2021 --test src/feel.rs` | `test result: ok. 4 passed; 0 failed` |
| `rustc --edition 2021 --test src/chem.rs` | `test result: ok. 15 passed; 0 failed` (14 existing + `hit_kind_classifies_impacts`) |
| `cargo fmt --check` (crates/bevy-client) | clean |
| `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` | clean |
| `npm test` | `tests 34, pass 34, fail 0` (science lint included) |
| web build (cargo build --release wasm32 + wasm-bindgen 0.2.129, the steps of build-web.sh) | ok; wasm gzip 5,038,443 B vs base 5,035,656 B (+2.8 KB) |
| `QUERY=?low node tests/e2e/golden.mjs` | **67/67 passed**: 31 existing + 36 new, run 04:33 to 04:49 UTC (15 min), exit 0 |

The golden runs, in order:
1. Killed: the box was overloaded, frames took 30 to 40 s, the renderer leaked under lock, and `WASD moves` failed at 0.40 m.
2. 64/67 at a load average of 5 to 8:
   - Test 10: the bush picked from (10, 6) was screened by a lit bush. Changed to (4, 2) and bush 5, checked with a probe.
   - The marker cleared in 1550 ms: the timer ran late behind 400 ms frames. The HUD frame now also expires the marker, rounding to the nearest frame.
   - The burst check sent only 1 cast. The casts now go out on an in-page timer.
3. 67/67. Selected lines:

```
PASS feel: camera stays at the eye; hand sway moves the gauntlet when on camOk=true vm spread=0.0110,0.0040,0.0000
PASS feel: hand sway off holds the gauntlet still camOk=true vm spread=0.00000,0.00000,0.00000
PASS feel: 100 px turns 5.44° at sens 1 turned 5.440°
PASS feel: sens 2 turns twice as far sens1 5.440° sens2 10.890°
PASS feel: Aim help toggles at runtime without a reload off: assist=false aim=-1; on: assisted=true aim=5
PASS feel: putting a fire out shows the big star and the cheer hitKind=out width=96 peak=0.15
PASS feel: the marker clears within 1 s 705 ms
PASS feel: point-and-click marker sits on the impact marker (752.9, 360.0) impact (752.9, 360.0) hitX=0.5882
PASS feel: locked marker sits at the crosshair marker (640.0,360.0) centre (640,360)
PASS feel: input reaches the game within 2 frames turn 1, cast 1, sens 1 frames
PASS feel: the out marker does not flash the screen worst frame-to-frame change 2.2%
PASS feel: a burst of casts flashes at most 3 times a second 6 casts sent, 3 fired, 4 samples, big changes 0, max/s 0
PASS feel: idle hint appears after 45 s with no progress (not before 40 s) shown at 45628 ms, early=false
PASS feel: blocked storage + reduced motion boots with URL sens and no sway sens=2 fov=58.72 sway box=false state=false anim=none errors=
PASS no console errors
67/67 passed
```

Software GPU frame rate during the run: `STATS fps=2.4 p50_ms=405.6`, `CPUFRAME p50_ms=4.70`.

## Deviations and notes for the tester and critic

- **Test 12 miss:** uses `look 180 60` instead of `look 0 60`. Facing −Z from (0, 5.5), a ray 60° up can catch the greenhouse roof, which is a target, so the impact would not be a miss. +Z faces open sky.
- **Test 10:** stands at (4, 2) and uses bush (9.6, −10.2) so the near miss is not screened by another target.
- **Test 13:** the free-pointer marker uses a `tap` 10° right of centre (`fx = 0.5882`), so `hitX ≠ 0.5`.
- **Test 11:** the `#crosshair.on span` ring colour is read by adding `on` for one synchronous read. Lock state is not needed for a CSS check, and the real locked `on` path is unchanged.
- **Test 19 is a guard.** At about 2.4 fps on the software GPU, a 0.3 s game-time cooldown spans several real seconds, so only 3 of the 6 casts fire. Screenshots take about 1 s, so the samples are sparse.
- **"Same HUD frame" (test 12)** is checked with a MutationObserver on `#hitmarker`. It records the state the HUD acted on and `lastSound().frame`, which must equal the marker's `data-frame`, within 1 frame of `hitFrame`.
- **Hints off** removes the idle hint and nudge only. The control hints ("Click to look around", lock notes) still show, because they are needed instructions, not nudges.
- **Look speed:** halving it (0.0022 to 0.00095) changes how far panel personas' drags turn the view. Record this so a clarity dip in the next panel run is not misread.
- **Golden runtime:** the new section adds about 4 to 5 minutes, mostly from the idle-hint waits and the second browser context.
- **Shared machine:** with other lanes running Chromium and cargo on 4 cores, frames slowed to tens of seconds and the renderer leaked under pointer lock. That affects the base build as well. Runs 2 and 3 were started only when the load average was below 5.
- **Out of scope:** `crates/bevy-client/README.md` still states sens 0.0022 and lacks the settings, so a docs follow-up is needed. Spec test 21 (B2/B3 perf pairs) is the tester's.
- `golden.mjs` is tester-owned in areas.md but owned by this single lane per the spec. The tester should review the new checks.
