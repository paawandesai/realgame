# I-007: Feel pass + gameplay-standards P0 fixes (merges I-007 and I-016)

**based_on:** origin/main a94fc488a8f9d3cb3c9c685b7615ea84a5e3a1be (2026-10-30, after I-011 and I-012 merged). This spec replaces the r1 I-007 spec (based on f6504c4 and the unmerged prototype branch). Its CastGate design is dropped because I-012 already built a cast queue: `Game.cast_queue` holds `cast`/`tap`, waits at most 0.3 s, fires one per cooldown, and clears on pause and reset. I-016 ("apply gameplay-standards P0s") is folded in here, and the backlog row for I-016 becomes `merged-into I-007`.

**Facts from a94fc48 this spec relies on** (the planner re-checks them before dispatch; if any has changed, refresh the spec):
- `main.rs`: `const FOV_Y: f32 = 70°` is used by `setup()` for `PerspectiveProjection.fov` and by `ray_at` in `aim_and_cast`, which builds the `tap`/hover ray.
- `player.rs`: `LOOK_PER_PX = 0.0022` rad per CSS px at sens 1. `EYE = 1.6`. `control()` sets the `FpsCam` translation to `player.eye()` and its rotation to `player.rotation()`.
- `Settings { sens, assist (radians, 4° or 0), low, msaa, fxaa, debug }` comes from `read_settings()`, which reads only the URL.
- `pick()`: the aim-assist cone is a fixed `settings.assist` angle out to 60 m. `aim_and_cast` sets `game.cooldown = 0.3` on each cast. `game.recoil` decays at 6/s.
- `viewmodel()` adds a walk bob (`player.stride`) and a recoil kick to the gauntlet. The camera has no bob.
- `projectiles()` increments `game.impacts` when a projectile arrives. It calls `chem.world.apply(i, spell, &mut ev)` only when `p.obj` is `Some`.
- The chemistry on this commit: `Water` on a non-burning flammable object pushes `Soaked`. `Water` on a burning object pushes `Extinguished(Water)`. `Fire` on a wet object pushes `Fizzle(Wet)`. `Fire` on an already-burning object pushes no event.
- There is no audio anywhere in the client.
- `__gameState` has `frame, locked, paused, spell, aimObj, assisted, hover, casts, impacts, yaw, pitch, pos, sens, assist, outcome, objs[]…`. `tap FX FY` takes canvas fractions in 0..1.
- `index.html` (which this item may not edit) holds `#crosshair > span`, `#hint`, `#hotbar .slot`, and `#pause .card` with `#resume` and `#pointmode`. CSP is `style-src 'self'`, so new UI must be built by `hud.js` with `createElement` plus classes. CSSOM writes such as `el.style.setProperty('--cd', …)` are allowed; an HTML `style=""` attribute is not.
- `tests/e2e/panel/hud.mjs` (not owned) reads `#crosshair` and its `on` class. Both must keep their meaning.

## Problem

`gameplay-standards.md` lists P0 failures in the current client that a 4-to-18-year-old in a school lab runs into within the first minute:
- The view is too wide: 70° vertical, about 102° horizontal (GS-1).
- Mouse look is about 2.5× faster than the comfortable range (about 9 cm/360°), and it can only be changed through `?sens=`, which no student will type (GS-8).
- The gauntlet bobs while walking, with no off switch (GS-3).
- Aim assist can only be switched off through the URL (GS-10).
- A cast that lands, one that does nothing and one that misses look almost the same, and there is no sound at all (GS-15, GS-17).
- HUD text is 12 to 16 px, and the hint is white text on a light sky (GS-18).
- A stuck player gets no hint (GS-24).

The I-007 feel items are missing as well. The 0.3 s cooldown is invisible, so holding the button feels like dropped clicks. Nothing at the crosshair says a cast landed.

## Bars moved

- **B4 (every age can play)** and **B5 (fun):** these are the intended movement. Nothing is reported as moved at merge, because both are panel bars. Prediction for the first paired panel run (same seed, a94fc48 artifact vs this one): no persona reports "too fast to look", "couldn't tell if it worked" or "couldn't read it" as a blocker, and median clarity rises at least 1 point.
- **B1 (golden path):** it must stay green. The new checks go into `golden.mjs`, so B1 now also covers these standards.
- **B2, B3:** must not regress (test 21).

## Smallest version

Each bullet names the GS items it meets.

1. **FOV (GS-1, GS-2).** The default is 90° horizontal at 16:9, which is a vertical `fov` of 58.7°. The fixed `FOV_Y` becomes a live value, `Settings.fov_y`. The `PerspectiveProjection` and `ray_at` both read it through one helper, `feel::ray_dir`, so a `tap` lands where the cursor is at any FOV. The settings slider runs from 70 to 110° horizontal in steps of 5 and is converted with `feel::vfov_from_h(h) = 2·atan(tan(h/2)·9/16)`.
2. **Look sensitivity (GS-8).** `LOOK_PER_PX` changes from 0.0022 to **0.00095** rad/px, which is 21.0 cm/360° at 800 CPI (315 px/cm). The sens multiplier still defaults to 1. The slider runs from 0.25 to 3.00 in steps of 0.05, which covers ±50% and reaches 7 cm/360° for trackpad users. `?sens=` keeps working as a multiplier. Check against the golden thresholds: the existing look checks need more than 5° from 200 px and more than 3° from 120 px. At the new constant those give 10.9° and 6.5°, so no existing check is loosened.
3. **Settings block (GS-2, GS-3, GS-8, GS-10, GS-14, GS-18, GS-20 partial, GS-24).** `hud.js` builds a "Settings" block inside `#pause .card`, just before `#resume`. It uses native controls, so it works from the keyboard without extra code (GS-13 partial). Labels are short and not sentences (GS-21).

   | id | label | control | default |
   |----|-------|---------|---------|
   | `#sens` + `#sens-val` | Look speed | range 0.25–3.00, step 0.05 | 1.00 |
   | `#fov` + `#fov-val` | View width | range 70–110, step 5, shows "90°" | 90 |
   | `#sway` | Hand sway | checkbox | on, or off when `prefers-reduced-motion: reduce` and nothing is stored |
   | `#assist` | Aim help | checkbox | on |
   | `#hints` | Hints | checkbox | on |
   | `#sound` | Sound | checkbox | on |
   | `#textsize` | Text size | select 100% / 150% / 200% | 100% |

   - Every control applies live. `hud.js` sends `set sens <x>`, `set hfov <deg>`, `set sway 0|1` and `set assist 0|1`. Text size, hints and sound stay in the HUD only.
   - Settings persist in `localStorage['alchemist.settings.v1']` as `{sens, hfov, sway, assist, hints, sound, text}`. Every storage access is wrapped in try/catch.
   - Precedence is URL (`?sens=`, `?assist=`) > stored value > default. Rust still reads the URL at boot, but only as a provisional value. The HUD sends the effective values once, as soon as the first `__gameState` arrives. Because both apply the URL first, they cannot disagree.
4. **Hand sway (GS-3).** With `sway` off, the viewmodel walk-bob offset is exactly 0. The recoil kick stays: it is one 0.15 s response to the player's own click, it moves only the gauntlet, and it is cast feedback (GS-16). The camera stays exactly at `player.eye()` and `player.rotation()` with no bob, shake or sprint FOV. Test 7 guards this.
5. **Aim assist at runtime, with distance falloff (GS-10).** `set assist 0|1` toggles the 4° cone without a reload. The cone becomes `feel::assist_cone(base, dist)`: the full `base` up to 20 m, falling linearly to 0 at 60 m, and 0 beyond. Assist still bends only the cast, never the view (GS-4).
6. **Cooldown ring and crosshair pulse (I-007).** `#crosshair` gets the class `cooling` while `state.cooldown > 0`. The CSS variable `--cd = clamp(cooldown / cooldownMax, 0, 1)` drives a conic ring that empties as the cooldown runs out. Each change in `casts` adds `pulse`, a scale of 1.0 → 1.35 → 1.0 over 120 ms. The existing `on` state (yellow plus a visible ring, so a shape change as well as colour, GS-15 and GS-19) is unchanged.
7. **Hit markers (GS-15, GS-17, GS-19).** On every impact, Rust publishes `hitKind` and the impact point.
   - `chem::hit_kind(&[Event]) -> HitKind`: any `Extinguished(_)` gives `Out`. Otherwise, any event that is not a `Fizzle` gives `Hit`. Only `Fizzle`s, or no events at all, give `Fizzle`. An impact with no object is `Miss`.
   - `hud.js` injects `#hitmarker` into `#hud`. It has three shapes that differ by more than colour:

     | kind | shape | size, colour | shown for |
     |------|-------|--------------|-----------|
     | `hit` | `.hm-x`, a four-stroke X | 28 px, white | 250 ms |
     | `fizzle` | `.hm-slash`, a hollow circle with a slash | 28 px, grey-blue | 300 ms |
     | `out` | `.hm-star`, a star burst inside a ring | **≥ 96 px**, gold | 700 ms |

     The `out` marker is the big, unmistakable success reaction that GS-17 asks for.
   - A `miss` shows no marker, so it looks different from a hit, and it plays its own sound.
   - The marker sits at the canvas centre while the pointer is locked. When the pointer is free (point-and-click), it sits at the impact's screen point (`hitX`, `hitY` as canvas fractions, from `Camera::world_to_viewport` ÷ the logical viewport size, or −1 when off screen; a −1 falls back to the centre).
   - Area: the `out` marker (96 px) is about 1% of a 1280×720 screen, and `hit` and `fizzle` (28 px) are under 0.1%. The most any can appear is one cast per 0.3 s. That is far inside the GS-6 limits (20% of the screen, 3 per second).
8. **Sound (GS-15, GS-16, GS-17).** `hud.js` synthesises short tones with WebAudio oscillators. There are no asset files, so the download size does not change.

   | sound | when | shape |
   |-------|------|-------|
   | `cast` | `casts` changes | 60 ms soft rising blip |
   | `hit` | `hitKind` is hit | 120 ms two-note up chirp |
   | `fizzle` | `hitKind` is fizzle | 150 ms low falling buzz |
   | `out` | `hitKind` is out | 450 ms three-note rising "cheer" |
   | `miss` | `hitKind` is miss | 80 ms low muffled thud |

   - The `AudioContext` is created and resumed on the first `pointerdown` or `keydown`.
   - `#sound` off mutes everything.
   - Loudness: every tone has a peak gain of at most 0.15 (about −16 dBFS), so a lab of 30 students is not startled. `lastSound()` also returns `peak`.
   - Each sound fires on the HUD frame in which the state change is first seen, the same frame as the marker (GS-16).
   - Test hook: `__game.lastSound()` returns `{name, n, played}`. `n` counts every sound requested. `played` is false when muted or when there is no AudioContext, but `name` and `n` still update.
9. **Readable text (GS-18).**
   - Every visible text element in `#hud` and `#pause` (`.slot .key/.name/.formula`, `#obj-line`, `#hint`, `#caption`, `#banner .small`, `#pause td/p/li/button`, and the new settings labels) is at least 18 px at 100% text size, on viewports of 1366×768 or larger.
   - Sizes are written as `calc(<n>px * var(--ui, 1))`. `#textsize` sets `--ui` on `document.documentElement` to 1, 1.5 or 2 through CSSOM.
   - The hotbar slots widen so the formulas fit on one line at 1366×768. At 150% and 200% they may wrap onto two rows, but they must stay inside the viewport.
   - Contrast: every text element sits on a panel whose background alpha is at least 0.6. That means giving `#hint` the `--panel` background, and darkening `#banner` and `#banner.lost` until white text reaches 4.5:1 even when the panel is composited over pure white.
   - The narrow-screen rule (max-width 760 px) keeps its smaller sizes. Phones are outside the GS-18 viewport.
10. **Idle hint (GS-24, GS-21).** While `outcome` is "playing" and the game is not paused, if 45 s pass (HUD wall clock) with no change in `extinguished` or `impacts`, `#hint` shows *"Look for a fire. Press 2 for Water, aim at the fire and click."* The Water slot also gets the class `nudge`, a slow 1 Hz glow (GS-6 safe; turned off under reduced motion). The glow is the cue for players who cannot read yet. Any progress, or turning off `#hints`, clears both within 2 frames. The idle clock restarts on every pause and resume, so a player coming back from Esc is not told off at once (GS-7). The hint text and the glowing slot are one constant tied to the current (fire) quest. Any later quest item must supply its own idle hint, and the planner files that as a backlog row. A controls reminder is already one key away (Esc).
11. **Reduced motion.** Under `prefers-reduced-motion: reduce`, the `pulse` and `nudge` animations become `animation: none`, the ring has no transition, and `sway` defaults to off. Markers still appear, because they are information.
12. **Science-lint rows.** Every new sentence-shaped player string gets a row in the non-claim table of `content/science-sources.md` (for example the idle-hint line). Settings labels are not sentences and need no row. `npm test` must stay green.

## Acceptance tests

Rust unit tests run with `rustc --edition 2021 --test crates/bevy-client/src/feel.rs -o /tmp/feel && /tmp/feel` and the existing `rustc --edition 2021 --test crates/bevy-client/src/chem.rs -o /tmp/chem && /tmp/chem`. Both modules stay free of Bevy types, as `chem.rs` already is. Every e2e test below is a new `check(...)` in `tests/e2e/golden.mjs`, run as `QUERY=?low node tests/e2e/golden.mjs` on the built artifact. They go in a "feel and standards" section placed after the CPU stats print and before the existing `no console errors` check, so that check also covers them. Existing checks are not edited, loosened or removed.

- "Within N frames" means N increments of `state.frame`.
- Any test that changes a setting restores it before it ends.
- The test asserts `vfov(h)` by computing `2·atan(tan(h/2)·9/16)` itself.
- At 1280×720 (16:9), `hfovOf(v) = 2·atan(tan(v/2)·16/9)`.

**Unit (fail today because the functions do not exist):**
1. `feel::vfov_from_h`: 90 → 58.72 ± 0.05, 70 → 43.0 ± 0.1, 110 → 77.6 ± 0.1. `feel::clamp_hfov`: NaN and ±inf → `None`, 50 → 70, 130 → 110. `feel::DEFAULT_HFOV == 90`. (GS-1, GS-2)
2. `feel::cm_per_360(LOOK_PER_PX, 1.0, 800.0)` lies in [20, 80], with `LOOK_PER_PX` imported from `player.rs`, or re-exported as a `const` the test can see. `feel::clamp_sens`: NaN, ±inf, 0 and negative values → `None`, 5 → 3.0, 0.1 → 0.25. (GS-8)
3. `feel::assist_cone(4°, d)`: d = 5 and d = 20 → 4°; d = 40 → 2° ± 0.01; d = 60 and d = 80 → 0. (GS-10)
4. `feel::ray_dir(fx, fy, fov_y, aspect)` returns a camera-space unit vector. (0.5, 0.5) → (0, 0, −1). For (1.0, 0.5), the angle to −Z equals `hfov/2` within 0.01° when `fov_y = vfov_from_h(90)` and `aspect = 16/9`. Outside [0, 1] → `None`. (GS-1)
5. `chem::hit_kind`:
   - `[Soaked]` → Hit, `[Station]` → Hit, `[Fizzle(Wet)]` → Fizzle, `[]` → Fizzle.
   - `[Fizzle(Wet), Extinguished(Water)]` → Out.
   - `[Ignited]`, `[Froze]` and `[Chilled]` → Hit. (GS-15)

**e2e (each fails on a94fc48: the fields, ids or values are missing or wrong):**

6. **FOV default and slider (GS-1, GS-2).**
   - At boot, `state.fov` is in [55, 62] and `hfovOf(state.fov)` is in [88, 92].
   - `__game.pause(true)`, set `#fov` to 100 and dispatch `input`. Within 10 frames `state.fov` is `vfov(100)` ± 0.2 (67.7), and `#fov-val` reads "100°".
   - `face lit[0]` and move the free cursor to the canvas centre. `state.hover` equals `lit[0]` at hfov 100 as well. This shows that the `tap` ray uses the live FOV, and `ray_dir` (test 4) covers it off-centre.
   - Restore 90.
7. **Camera and hand-sway guard (GS-3).**
   - New state fields: `cam` (camera translation, 3 decimals) and `vm` (viewmodel translation, 4 decimals).
   - `set sway 1` (or click `#sway` on), then `move 0 1 1.5`. Sample every frame, at least 20 samples. Every sample has `cam − pos` = (0, 1.600, 0) ± 0.005. Over the samples, the spread (max − min) of `vm[0]` or `vm[1]` is > 0.002.
   - Turn `#sway` off and repeat. `cam` holds the same way, and the spread of every `vm` component is < 0.0005. No cast happens during sampling.
   - On a94fc48 this fails because there is no `vm` field and no `#sway`.
8. **No camera motion without input (GS-4).** `face lit[0]` with assist on, then 10 s of wall clock with no input and no commands. `yaw` and `pitch` are identical at the start and the end. This is a guard: it passes on a94fc48 when the assertion is limited to the existing fields, and it is kept so it can catch assist ever nudging the view.
9. **Sensitivity (GS-8).**
   - `set sens 1`, `look 90 0`, `turn 100 0` → the yaw change is 5.443° ± 0.05 (100 × 0.00095 rad). On a94fc48 it is 12.6°.
   - From `look 90 0`, the change from `turn 100 0` at sens 2 is twice the change at sens 1, within 1%.
   - In the pause card, set `#sens` to 1.5 and dispatch `input`. Within 10 frames `state.sens` is 1.5 and `#sens-val` reads "1.50". The slider's `min` ≤ 0.5 and `max` ≥ 1.5.
10. **Aim assist runtime toggle (GS-10).**
    - `face b` (a non-burning bush), then `turn` in steps of 9 px (about 0.5°) to the right, up to 40 steps, until `aimObj === b && assisted === true`. If that never happens, the test fails.
    - Uncheck `#assist` and dispatch `change`. Within 10 frames `state.assist` is false and `aimObj` is −1.
    - Check it again. Within 10 frames `assisted` is true and `aimObj === b`.
    - Nothing reloads (`frame` keeps increasing).
11. **Cooldown ring and pulse (I-007, GS-12).**
    - New state fields: `cooldown` (≥ 0, 3 decimals) and `cooldownMax` (0.3).
    - From rest, `cast` at `lit[0]`. Within 2 frames of `casts` changing, `#crosshair` has `pulse`.
    - In the first frame after the cast, `state.cooldown` is in (0, 0.3], `#crosshair` has `cooling`, and the computed `--cd` is in (0, 1].
    - `cooling` is gone within 3 frames of `cooldown` reaching 0.
    - While the pointer is locked and `aimObj ≥ 0`, `#crosshair.on span` has a border colour that is not transparent (the shape cue, GS-15 and GS-19).
12. **Hit markers and sounds (GS-15, GS-16, GS-17, GS-19).** New state field `hitKind` ∈ none, hit, fizzle, out, miss, set on each impact (with `impacts` as its id), plus `hitX` and `hitY`. On a fresh courtyard:
    - Water (`select 2`) on a non-burning bush `b` → `hitKind` "hit". In the same HUD frame where `impacts` changes, `#hitmarker` has `show kind-hit` with `.hm-x` visible, and `lastSound().name` is "hit".
    - Fire (`select 1`) on `b`, which is now wet → "fizzle", `kind-fizzle`, `.hm-slash` visible, sound "fizzle".
    - Water on `lit[0]` → "out", `kind-out`, `.hm-star` visible, bounding box ≥ 80 px wide at 1280×720, sound "out".
    - `look 0 60` then `cast` → "miss". `#hitmarker` does not have `show` within 3 frames of the impact, and the sound is "miss".
    - `show` clears within 1.0 s of being set.
    - With `#sound` off, the next impact has `lastSound().played === false` and `n` increments. Turn sound back on.
13. **Marker position (point-and-click).**
    - Pointer free. In the frame where `show` is set after a `tap` at `lit[1]`, the centre of `#hitmarker`'s bounding box is within 6 px of `(rect.left + hitX·rect.width, rect.top + hitY·rect.height)`, where `rect` is the canvas `getBoundingClientRect()`.
    - When locked, it is within 6 px of the canvas centre.
14. **Latency (GS-12).** Both are measured in-page with a `requestAnimationFrame` loop that sends the command and counts frames.
    - `turn 50 0`: `yaw` changes within 2 frames.
    - `cast` with `cooldown` 0 and `aimObj ≥ 0`: `casts` increments within 2 frames.
    - `set sens 2`: `sens` updates within 2 frames.
15. **Settings persist; storage blocked (GS-14).**
    - After tests 6 to 12 set and restore values, set `#sens` to 1.5 and `#fov` to 100 and turn `#hints` off. `localStorage['alchemist.settings.v1']` parses to `sens 1.5, hfov 100, hints false`.
    - `page.reload()`. Within 10 frames of the first state, `sens` is 1.5, `fov` is `vfov(100)` ± 0.2, and `#hints` is unchecked.
    - Clear the key and reload, then confirm `sens` is 1 and `fov` is 58.7 ± 0.2.
    - A second browser context with an init script that makes `Storage.prototype.getItem` and `setItem` throw, `reducedMotion: 'reduce'`, and URL `?low&debug&sens=2`: the game boots, `sens` is 2 (URL wins), `fov` is 58.7 ± 0.2, `#sway` is unchecked and `state.sway` is false, and that context logs no console errors. After a cast, the computed `animation-name` of `#crosshair` is `none`.
16. **Text size (GS-18).** At viewports of 1920×1080 and 1366×768, with `#pause` shown and the HUD visible:
    - Every visible element under `#hud` or `#pause` that has its own non-whitespace text node has a computed `font-size` of at least 18 px.
    - No `.slot` has `scrollWidth > clientWidth`.
    - Then set `#textsize` to 200%. Each such element's font size is at least 2× its 100% value (at least 36 px). At 1920×1080, every visible `#hud` child's bounding box lies inside the viewport.
    - Every visible `input`, `select` and `button` in `#pause` has a bounding box of at least 24×24 px at 1366×768 (checkboxes are sized by CSS to 28 px, ranges are at least 24 px tall), and `#resume` and `#pointmode` are at least 40 px tall (GS-22).
    - Restore 100% and 1280×720.
    - On a94fc48 this fails: `.slot .key` is 12 px, `.name` 15, `.formula` 14, `#obj-line` 15, `#hint` 16, and there is no `#textsize`.
17. **Contrast (GS-18).**
    - For each element from test 16 at 100%, find the nearest self-or-ancestor (inside `#hud` or `#pause`) whose background alpha is ≥ 0.6. If there is none, the element fails.
    - Composite that background over pure white and over pure black. The WCAG contrast of the text colour against each result must be ≥ 4.5, or ≥ 3.0 when `font-size` is ≥ 36 px.
    - On a94fc48 this fails on `#hint`, which has no panel.
18. **Idle hint (GS-24, GS-21).**
    - Fresh courtyard, unpaused, no input for 50 s of wall clock. `#hint` contains "Look for a fire" and the Water `.slot` has `nudge`, but neither appears before 40 s.
    - Then uncheck `#hints`. Within 2 frames both are gone.
    - Re-check `#hints` and restore.
    - Pause for 30 s of wall clock after 20 s of idle, then resume: no hint appears before another 40 s of unpaused idle.
19. **No flashing from new effects (GS-6).**
    - Over the casts in tests 11 to 12, capture 20 screenshots at fixed frame steps during an `out` marker. For each pair of consecutive frames, the fraction of pixels whose relative luminance changes by ≥ 10% stays under 20% of the frame.
    - A burst of `pulse` and markers from 6 casts within 2 s produces at most 3 luminance flashes over 20% of the screen in any 1 s window.
    - This is a guard on the new HUD effects. It is computed in Node from the PNG buffers with a small inline decoder, or Playwright's `page.screenshot({ type: 'png' })` read through `node:zlib`. No new dependency.
20. **Science lint.** `npm test` passes, and `test/science-sources.test.ts` finds no unlisted sentence-shaped string from `hud.js`.
21. **B2 and B3 guards (tester, no script change).**
    - Build the base artifact (a94fc48, or this item's merge-base) into `<scratchpad>/I-007/base-pkg-out`.
    - Run `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs` on base and on this artifact, interleaved, 3 pairs. The median CPU frame p50 is ≤ base + 1.0 ms and ≤ 16 ms.
    - Brotli of `pkg-out` grows by ≤ 30 KB against the base.
22. **Golden path still green.** All existing `golden.mjs` checks pass 3/3 with `QUERY=?low`, with no edits to their thresholds.

## GS walk (every item, for the critic)

| GS | P | status in this item |
|----|---|---------------------|
| 1 | P0 | **met**: default 90°h (58.7°v); tests 1, 4, 6 |
| 2 | P1 | met: 70–110°h slider, persisted; tests 6, 15 |
| 3 | P0 | **met**: hand-sway off switch; camera has no bob (guard); reduced motion defaults it off; test 7 |
| 4 | P0 | met (guard): assist bends casts, not the view; test 8 |
| 5 | P0 | not touched here. p99/max frame ms is a `perf.mjs` change (I-013, I-015). This item only guards the median (test 21) |
| 6 | P0 | met for new effects: test 19. An audit of existing spell VFX goes to the backlog |
| 7 | P1 | already met (Esc pauses everything); not changed |
| 8 | P0 | **met**: 21 cm/360° at 800 CPI; slider covers ±50%; tests 2, 9 |
| 9 | P1 | deferred: remapping, invert-Y and sprint toggle → new backlog item |
| 10 | P0 | **met**: runtime toggle and distance falloff; tests 3, 10 |
| 11 | P0 | already met (point-and-click mode; golden "click-to-target casts without lock"); not changed |
| 12 | P0 | met (guard): test 14; the marker and sound fire in the same HUD frame as the state change |
| 13 | P1 | partial: native controls are keyboard-operable; full keyboard menu flow deferred |
| 14 | P1 | met: tests 15 |
| 15 | P0 | **met**: marker plus sound, hit and miss differ, `on` uses a ring shape; tests 11, 12 |
| 16 | P1 | partial: same-frame marker and sound, recoil kick, pulse. Hit-stop deferred |
| 17 | P0 | **met**: big `out` star (≥ 96 px), cheer sound, existing caption; test 12. Panel confirms |
| 18 | P0 | **met**: ≥ 18 px, 200% scale, contrast; tests 16, 17 |
| 19 | P0 | met: marker kinds differ by shape; slot icons already differ by shape; crosshair-colour choice (P1) deferred |
| 20 | P1 | partial: master Sound on/off. Separate music/SFX/voice sliders are deferred (there is no music or voice yet) |
| 21 | P0 | met for new copy (one grade-≤ 6 hint, backed by a visual nudge, no auto-dismiss) |
| 22 | P1 | met: new controls are at least 24 px, checked in test 16; slots are already at least 40 px |
| 23 | P0 | **not met, out of scope**: a safe tutorial space is level design → new backlog item "I-0xx Safe first-minute tutorial (GS-23)" |
| 24 | P0 | **met**: idle hint after 45 s, hints toggle, objective numbers already shown; test 18 |
| 25, 26 | P1 | deferred (game-speed / calm assist) |
| 27, 28 | P0 | met (guard): no new content; no streaks, timers, links or auto-advance added |
| 29 | P2 | backlog |
| 30–32 | P0/P1 | not touched (no new quest or notation) |

## Out of scope

- GS-23 tutorial, GS-5 perf-script p99, GS-9 remapping and invert-Y, GS-25 and GS-26 assists, crosshair colour choice, separate volume sliders, hit-stop: each goes to a backlog row.
- Changes to `index.html` (forbidden for this lane). All new DOM is injected by `hud.js`.
- `crates/bevy-client/README.md`: the stated sens constant (0.0022) and the controls table go stale. The planner files a one-line docs follow-up for whoever owns that path.
- A Bevy audio dependency or audio asset files (sound is WebAudio in `hud.js`).
- Analytics: no settings or feel events are sent, and `functions/` and `dashboard/` are not touched.
- Per-spell cooldowns or recoil, damage numbers, a separate viewmodel camera.
- Wiring the `feel.rs` unit tests into CI (an infra item).
- Tuning the defaults from panel data (a follow-up after the first paired panel run).

## Contract

none. The `set` commands, the new `__gameState` fields (`fov`, `cam`, `vm`, `sway`, `cooldown`, `cooldownMax`, `hitKind`, `hitX`, `hitY`) and the DOM ids are written and read only by files the single lane owns. For later items, the ids (`#sens`, `#fov`, `#hitmarker`, `#crosshair.cooling`, `#crosshair.pulse`) match what the I-008 spec expects.

## Lanes

| lane | owns | builds |
|------|------|--------|
| main | `crates/bevy-client/src/**`, `crates/bevy-client/web/hud.js`, `crates/bevy-client/web/style.css`, `tests/e2e/golden.mjs`, `content/science-sources.md` (adding rows for new strings only) | Everything above. Rust: `feel.rs` (+ `mod feel;` in `main.rs`) with tests 1–4; `chem::hit_kind` with test 5; live FOV through `feel::ray_dir`; `LOOK_PER_PX` 0.00095; `set sens/hfov/sway/assist` (not debug-gated); assist falloff; sway gating in `viewmodel()`; `hitKind`/`hitX`/`hitY` in `projectiles()` (adds the `FpsCam` `Camera` + `GlobalTransform` query); new state fields. Web: settings block, persistence and precedence, ring, pulse, marker, WebAudio, text scale, contrast fixes, idle hint, reduced-motion rules. Tests 6–19 in `golden.mjs`. Non-claim rows for new sentence strings. Must not touch `index.html`, `boot.js`, `build-web.sh`, `functions/`, `dashboard/` |
