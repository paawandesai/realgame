# I-025: Streams you cannot miss, an honest arrow, and one cast per click

**based_on:** origin/main a0e2ecbb0d0b6de444f274674fa1cdb8dfba0505 (2026-10-30, after I-019 (#29) and wave-7 state (#30) merged). The code was read there with `git show a0e2ecb:<path>`; the primary checkout (661b9ba) still has no `crates/bevy-client/src`, so builders branch from the real origin/main.

**Revision 2** (answers `critique.md`; see its `## Planner response`). Follow-ups from `docs/loop/items/I-019/break-1.md`: Should-fix 1 (hint priority), 2 (faint stream), 3 (finder points away from a fire under the crosshair), and the nit at `main.rs:1076` (a slow click casts twice). Other nits from that report are out of scope (see Out of scope).

**Runs beside I-022 and I-024.** Both are specced against a0e2ecb and may merge before or after this item. This item stays out of their lanes. "Overlap with I-022 and I-024" under Contract lists every hunk and the line ranges this item must not touch.

## Facts from a0e2ecb this spec relies on

Line numbers are at a0e2ecb. `main.rs` means `crates/bevy-client/src/main.rs`, `hud.js` means `crates/bevy-client/web/hud.js`, `golden` means `tests/e2e/golden.mjs`. `Spell::ALL` is `[Fire, Water, Frost, Foam]` (`chem.rs:67`), so the spell index is Fire 0, Water 1, Frost 2, CO2 (`Foam`) 3, and `select N` picks index N−1.

- **Streams are thin, pale and glowing.**
  - `spawn_stream` (`main.rs:248-271`) spawns one `kit.cone` entity with `stream_transform` (`main.rs:233-245`), scaled `(STREAM_THICK, len, STREAM_THICK)`. `STREAM_THICK = 0.13` (`main.rs:124`). The cone's base is at the hand and its tip at the head, so the far end tapers to nothing.
  - The material is `mats.glow[spell]` (`main.rs:1130`): `SpellMats.glow` (`main.rs:274-277`), built in `setup` (`main.rs:507-516`) as the spell colour plus emissive at 5× that colour. Water is `srgb(0.2, 0.55, 1.0)` and CO2 is `srgb(0.92, 0.95, 1.0)` (`spell_color`, `main.rs:445-452`). The sky is `ClearColor(srgb(0.55, 0.78, 0.98))` (`main.rs:322`), and the camera uses Reinhard tonemapping (`main.rs:543`), with HDR and bloom only when not `?low` (`main.rs:538-540`). A bright, washed-out pale blue or white against a pale blue sky gives very little contrast. The I-019 breaker's shot 05 shows this.
  - **Frost has no stream.** It spawns a `Beam` instead (`main.rs:1101-1116`): a `kit.cube` 0.07 m thick in `glow[2]`. The beam loop (`main.rs:1182-1189`) shrinks it by `× 0.85` per frame (frame-rate dependent) and counts it down while paused. `Beam` (`main.rs:209-213`) is counted with streams in `StreamsAndBeams` (`main.rs:1159-1160`).
  - On impact, Water bursts 24 `Water` particles plus 8 `Steam` on a burning target (`main.rs:1259-1264`). Every other spell bursts 14 particles of its kind at size 0.22 (`main.rs:1266`). The particle kinds and materials are in `fx.rs:104-117`: `P::Frost` is a pale, unlit, growing particle that drifts slowly down (`fx.rs:108`, `197`, `205`).
  - The I-019 pixel check (`golden:1505-1559`) counts pixels with `B > R + 40 && B > 120` in the box x 45–80 %, y 45–90 %. It pauses on the **first** frame with a stream, when the head is still near the hand. It needs only 0.3 % of the viewport above the larger "before" count. The near, thick end of any blue cone passes it. It says nothing about the far end or about contrast with the sky.
- **Where a stream lands on screen.** The hand is at camera-space `(0.2, −0.15, −0.5)` (`main.rs:1099`). With `f = (VH/2) / tan(vfov/2)` (640 px at 1280×720 and the default vfov 58.72°), a stream point at depth `z` m on the line from the hand to a target on the view axis is `r = 0.25·f/z` px from the screen centre, toward the hand's screen point `Hs = (VW/2 + 0.4f, VH/2 + 0.3f)`. A stream `T` m thick is `T·f/z = 4T·r` px wide there. So a 0.2 m stream is `0.8 r` px wide at every point, and the radii from 20 to 110 px cover depths from 8 m down to 1.45 m. These are the numbers test 7 uses.
- **The finder ignores every fire but the nearest.** `aim_and_cast` picks the nearest burning object by horizontal distance (`main.rs:991-998`). It sets `near_fire`, `fire_bearing`, `fire_elev` and `fire_off` (`main.rs:999-1017`). `hud.js:673` shows `#firefinder` while `playing && fireOff && nearFire >= 0 && !paused`. Nothing checks whether some other fire is on screen or under the crosshair.
  - **A concrete case.** From `tp 0 5.5`, `face lit[0]` aims at the bush at (−13.5, −3). The nearest fire is lit[2] at (0, −5), 10.5 m away, 58° to the right, which is off screen at 90° horizontal. So the crosshair shows `on fire` and the arrow points right, away from the fire the child is aiming at (breaker shot 03).
  - **Golden test 16 depends on it** (`golden:1595-1626`). From `tp 0 5.5`, it faces lit[0], turns 180°, then steps toward the arrow until the arrow hides, and requires `fireOff === false && |fireBearing| ≤ hfov/2` (`golden:1617`). On that turn lit[1] (13.5, −3) comes on screen about 40° before lit[2] does. Once a fire on screen hides the arrow, the old `found` condition fails. Contract item 3 changes it.
- **The hold path double-casts on a slow click.** `hud.js:349-354`: a locked left `pointerdown` sets `input.fire = true` and pushes `fire`, and `pointerup` clears `input.fire`. `bridge::poll` reads both in the same frame (`bridge.rs`). In `aim_and_cast`, the `fire` command casts through `cast_queue` (`main.rs:1036-1067`). The held path (`main.rs:1069-1077`) then casts again on the first frame where `cooldown <= 0` while the button is still down.
  - The cooldown is 0.3 s with I-023's carry (`main.rs:1084-1095`), so **any press held for more than about 0.3 s of game time casts twice**. At 4 fps (dt 0.25) a carry can bring the second cast forward to the next frame sample. A child's slow click (200–400 ms) is enough.
  - I-019's spec (item 1) says "a press that is still down after 0.3 s repeats". The breaker filed the double cast as a nit; the brief raises it to fix.
- **Hint priority: the spec and the code disagree.**
  - I-019 spec item 9 says banner > `note` > idle > locked > start.
  - `hud.js:659-671` does banner (`!playing`) > idle > `note` > locked > start.
  - The web lane changed the order on purpose (`docs/loop/items/I-019/lanes/web/round-1.md:39`). I-007's check `feel: idle hint appears after 45 s with no progress` (`golden:859-877`) runs while the point-and-click note from I-007 test 13 is still set, so note-over-idle would fail it.
  - I-022's spec (facts, `hud.js:659-671`) already describes the code's order. I-022 gates `setHint` with `hintAllowed` and does not edit `frame()`.
- **State keys.** The format string is at `main.rs:1581` and its arguments at `main.rs:1582-1634`. Golden's key-order check pins `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS]` (`golden:1045-1047`).
- **Golden layout.** `feelAndStandards(...)` is called at `golden:341` and `castingYouCanSee(...)` at `golden:343`. `no console errors` is at `golden:345`. `castingYouCanSee` is defined at `golden:1243-1758`, after `clockAndBudget` (`golden:996`). Module-level helpers include `decodePng` (`golden:365`), `lumaOf`, `flashFrac` and `hfovOf` (`golden:359-361`). I-019 test 11's 18 m poses are at `golden:1420`.

## Problem

I-019 made casting work: one click casts, the spell lands within 0.25 s, and a stream exists. Its breaker found four things that still confuse a young player:

1. **The stream is hard to see.** Water is a thin, pale, glowing cone that fades into a pale sky and tapers to nothing at the fire. A 7-year-old may not notice it, which is the very thing I-019 was meant to fix. CO2 is a white glow (same problem), and Frost is a 7 cm line. The golden pixel check passes anyway, because it measures only the thick end near the hand.
2. **The arrow says "go somewhere else" while the child is aiming at a fire.** It always points to the *nearest* fire, even when another fire is under the crosshair or on screen.
3. **The hint order is undocumented.** The spec and the code disagree, so the next item cannot tell which one is right.
4. **A slow click casts twice.** Any press longer than about 0.3 s fires the click's cast and then a held cast. Young players click slowly, so they spend twice the spells and see two streams for one click.

**Goal:** every extinguishing spell (Water, CO2, Frost) draws a constant-width stream about 0.2 m thick, along its whole length, that stands out against the sky and the ground. Each stream looks like what it is: Water a liquid jet, CO2 a white cold cloud, Frost a jet in white mist. The arrow appears only when no fire is in view. One click is one cast unless the button is clearly held. The hint order is written down once, and the code follows it.

## Bars moved

- **B4 (every age can play):** the intended movement. It is a panel bar, so nothing is reported as moved at merge.
  - Prediction for the first paired panel run (same seed, a0e2ecb artifact vs this one): no persona reports "could not see the spell", "the stream was hard to see", or "the arrow pointed away from the fire".
  - Median clarity does not fall, and the 6-year-old's clarity rises by at least 1.
  - `ttc` stays at 30 s or less for every persona.
- **B5 (fun):** secondary. The prediction is that median fun does not fall. Streams that read clearly are part of the casting feel.
- **B1 (golden path):** must stay green. The new checks go into golden.
- **B2, B3:** must not regress (test 17). Each stream gains one outline child entity, and Frost and CO2 gain at most 12 mist particles per cast.
- **B6 (science is right):** must stay at 0 errors. The stream looks become sourced `## Parameters` rows (item 6 below). No player-facing text changes.

## Smallest version

1. **One stream helper with per-spell styles, in `cues.rs`.**
   - Add `pub struct StreamStyle { pub thick: f32, pub core: [f32; 3], pub outline: [f32; 3], pub outline_scale: f32, pub mist: u32 }` and `pub fn stream_style(spell: u8) -> StreamStyle`. Colours are sRGB in 0..1. `mist` is the most cold-fog particles one cast may emit. Indices 4 and above return the Water style.
   - Also add `pub fn rel_luminance(rgb: [f32; 3]) -> f32` (WCAG 2 relative luminance of an sRGB colour) and `pub fn contrast_ratio(a: [f32; 3], b: [f32; 3]) -> f32` (`(L1 + 0.05) / (L2 + 0.05)`, lighter over darker), for the unit tests and the lane's tuning.
   - **The styles.** Thickness is in metres. The rust lane may tune values inside these limits only:

     | spell | thick | core | outline | mist | why it is truthful |
     |-------|-------|------|---------|------|--------------------|
     | Fire (0) | 0.18–0.25 | unchanged: the glow material | none (`outline_scale` 1.0, no child) | 0 | out of the brief; it only gets the same thickness |
     | Water (1) | 0.18–0.25 | a saturated blue: B ≥ 0.8, B − R ≥ 0.5, relative luminance 0.175–0.24 | dark navy, relative luminance ≤ 0.025 | 0 | a coherent liquid jet keeps its width; water is drawn in a clearly blue colour code (in thick layers, water is faintly blue) |
     | Frost (2) | 0.18–0.25 | white to pale cyan: every channel ≥ 0.75, B ≥ R | near-black slate, relative luminance ≤ 0.025 | 6–12 `P::Frost` | a liquid-nitrogen jet; the white mist is water vapour from the air condensing in the cold |
     | CO2 (3) | 0.18–0.25 | white: every channel ≥ 0.85, max − min ≤ 0.10 | near-black slate, relative luminance ≤ 0.025 | 6–12 `P::Frost` | a CO2 extinguisher discharges a white cloud of dry-ice snow and condensed vapour |

   - **Why these luminance numbers.** One set of numbers rules both the table and unit test 1. Water core vs sky (L ≈ 0.53) ≥ 2.0 needs core L ≤ 0.24. Outline vs core ≥ 3.0 with outline L ≤ 0.025 needs core L ≥ 0.175. Outline vs a dark-green ground `[0.2, 0.4, 0.2]` (L ≈ 0.105) ≥ 2.0 needs outline L ≤ 0.027. Example Water core that fits: `[0.1, 0.45, 1.0]` (L ≈ 0.20).
   - **Unit tests are not the gate.** They measure the sRGB constants. Pixels on screen go through lighting and Reinhard tonemapping, so e2e tests 7 and 8 are the real gate. The lane does not stop tuning when the unit tests pass.
   - **Materials.** `SpellMats` (`main.rs:274-277`) gains `stream: [Handle<StandardMaterial>; 4]` and `outline: [Handle<StandardMaterial>; 4]`, built in `setup` next to `glow` (`main.rs:507-516`).
     - Water, Frost and CO2 get an opaque core that is not emissive (no bloom haze), lit or `unlit` as the lane's tuning needs.
     - Each outline material is `unlit`, opaque and has `cull_mode: Some(Face::Front)`, so the shell draws as an outline (an inverted hull).
     - Fire's `stream` entry is `glow[0]`, and Fire gets no outline.
     - `aim_and_cast`'s signature does not change.
   - **The mesh.** Use the existing `kit.cyl`, never mutated (I-024 Part R rule), scaled `(thick, len, thick)`: **constant width** from the hand to the head. The cone is not used for Water, Frost or CO2. Perspective already makes the stream look tapered (`4T·r` px wide, see Facts). The stream starts at the hand, as today (a lead-in gap ahead of the hand was considered and rejected; see the GS walk, row 6).
     - For Water, Frost and CO2, the outline is **one child entity** of the stream: the same mesh, scaled `outline_scale` (1.15–1.35) in x and z relative to the core, using `outline[spell]`, with `NotShadowCaster` and `NotShadowReceiver`.
     - The child despawns with its parent. The lane confirms that Bevy 0.20's `despawn` removes children. If it does not, the lane despawns the child explicitly.
     - `STREAM_THICK` (`main.rs:124`) is deleted, and `stream_transform` takes the style.
   - **Frost uses the stream.** The `Beam` spawn (`main.rs:1101-1116`), the `Beam` component (`main.rs:209-213`) and its loop (`main.rs:1182-1189`) are removed. Frost goes through `spawn_stream` like the others (`if spell != Spell::Frost` at `main.rs:1125` goes away). `StreamsAndBeams` becomes a query over `Stream` only, and the `streams` state field keeps its meaning: live stream entities, **not** counting outline children. The game-time life rules stay: `STREAM_AFTER`, `STREAM_MIN_FRAMES`, frozen while paused, and gone on an epoch change.
   - **Mist.** While a Frost or CO2 stream is alive and the game is not paused, it emits `P::Frost` particles at random points on the **far 70 % of its current length that are also at least 2 m from the eye**. If no point meets both rules (a very short stream), it emits nothing that frame and keeps `mist_acc`. This keeps pale, growing blobs away from the camera (GS-6, GS-17). A `mist_acc: f32` in `Stream` accumulates `48 · dt` per frame, with dt from `clock::frame_dt` as `projectiles` already does. The stream emits `floor(mist_acc)` particles and stops at `style.mist` for that stream. The particles have size 0.25–0.4, a life of 0.6–1.0 s and a small random velocity. There is no `fx.rs` change: `P::Frost` already grows and drifts slowly down, which is right for cold fog and for CO2, which is denser than air.
   - **Particle budget under `?low`.** Mist (≤ 12) plus the impact burst (14) plus the trail stays at 40 per cast or fewer, as in I-019's rule.
2. **The arrow hides while any fire is in view.**
   - Add `pub fn fire_in_view(fires: &[(f32, f32)], hfov: f32, vfov: f32, aim_burning: bool) -> bool` in `cues.rs`. It returns `aim_burning || fires.iter().any(|&(b, e)| b.is_finite() && e.is_finite() && !off_screen(b, e, hfov, vfov))`, with I-019's 5° margin unchanged.
   - In `aim_and_cast`, a **new block inserted after the finder's closing `};` (`main.rs:1017`) and before `game.hover = None;` (`main.rs:1018`)**:
     - For every burning object, it computes `(bearing, elev)` exactly as the finder does: `cues::bearing_deg` from the eye, and elevation to `chem.bounds(i).center()` minus pitch.
     - It computes `hfov` from the local `fov_y` and `aspect`. Those lines are **read, not edited**; they belong to I-022.
     - It sets `game.fire_in_view = cues::fire_in_view(&v, hfov, fov_y, game.aim.obj.is_some_and(|i| chem.world.objs[i].burning))`.
     - The existing finder lines (`main.rs:989-1017`) are not edited.
   - New `Game` field `fire_in_view: bool` and new state key `fireInView` (Contract). It is `false` when nothing burns.
   - `hud.js:673`: `findOn` gains `&& !state.fireInView`. Occlusion stays out of scope, as in I-019: a fire behind the greenhouse whose bearing is on screen counts as in view. This is already true at a0e2ecb for the nearest fire (`fire_off` is bearing-only), and fixing it needs the finder lines (989-1017) and an on-screen arrow, so it goes to a named follow-up (see Out of scope). Test 6(d) records today's behaviour for the panel brief.
3. **The hint order is banner > idle > note > locked > start, and it is pinned.**
   - **Decision: keep the code's order.**
     - The idle hint appears only after 45 s with no progress, which means the player is stuck. It is newer and more specific than a `note`, and a note stays from an earlier action until the next lock (the point-and-click note can be minutes old).
     - The idle hint already works in point mode ("aim at the fire and click" fits both modes).
     - I-007's idle check depends on this order, and I-022's spec already describes it.
     - The `note` still shows whenever the idle hint is not due.
   - `hud.js`: the `h` computation in `frame()` (`hud.js:665-671`) moves into a module-level pure function, `function pickHint({ playing, idle, note, locked, extinguished, casts })`. It returns `''`, `IDLE_HINT`, `note`, `AIM_HINT`, `''` or `START_HINT` in exactly today's order. `frame()` calls `setHint(pickHint({...}))`, and the `if (state.locked) note = ''` and `updateIdle(now)` lines stay before the call.
   - `window.__game` (`hud.js:230-238`) gains `pickHint`, for the tests.
   - The comment above the block states the order and cites this spec.
   - The web lane appends an amendment paragraph to the **end** of `docs/loop/items/I-019/spec.md`: "Amended by I-025: hint priority is banner > idle > note > locked > start (item 9 is superseded); a held press repeats after 0.5 s of game time, not 0.3 s (item 1)."
4. **One click is one cast. Holding repeats only after a clear hold.**
   - In `cues.rs`, add `pub const HOLD_DELAY: f32 = 0.5;`, `pub fn held_secs(prev: f32, down: bool, was_down: bool, clicked: bool, dt: f32) -> f32` and `pub fn hold_repeats(held: f32) -> bool`.
     - `held_secs` returns 0 when `!down`, when `!was_down`, or when `clicked`. Otherwise it returns `prev + dt`, where a non-finite or negative `dt` counts as 0.
     - `hold_repeats` returns `held >= HOLD_DELAY`.
   - New `Game` fields: `held: f32` and `was_down: bool`.
   - In `aim_and_cast`, `clicked` is true when `incoming` has a `fire`. It is computed next to the `game.clicks` count (`main.rs:1036-1039`), before `incoming` is moved.
   - After `firing` is known (`main.rs:1070-1075`):
     - When not paused, `game.held = cues::held_secs(game.held, firing && game.locked, game.was_down, clicked, dt)`. When paused, `game.held = 0`.
     - Then `game.was_down = firing && game.locked && !game.paused`.
   - The held-path condition (`main.rs:1076`) gains `&& cues::hold_repeats(game.held)`.
   - **Effect.** The click's own cast comes from the queue as today. A press must be held 0.5 s of game time before the first repeat, and then repeats follow the cooldown (0.3 s, with I-023's carry unchanged).
     - At 4 fps the first repeat comes on the third frame sample the button is down, about 0.5 s.
     - A new `fire` resets the hold timer, so a release and re-press between two frame samples cannot borrow the old press's time.
   - **Native builds** (untested, native dev only, no review time): the lane may treat `mouse.just_pressed(MouseButton::Left)` while locked as `clicked` plus a queued `fire`. Not a requirement; the shipped game is the wasm build.
5. **Nothing else about casting changes.** `flight_secs`, the cooldown, the carry, the latch, the queue, `clicks`, `firesLeft`, the splash and `fxWater` all stay as they are.
6. **Science register** (`content/science-sources.md`, `## Parameters`). Add three rows with the next unused ids, **skipping P-016**, which I-019 reserved for the CO2-cloud follow-up.
   - The rows are the Water, Frost and CO2 stream looks. The `constant` cell is, for example, `stream_style(Water) core colour`. The `value` is **the claim and its limit in the unit-test words**, not the tuned triple, so it cannot go stale: "Water drawn saturated blue (B ≥ 0.8, B − R ≥ 0.5)", "Frost jet drawn white to pale cyan in white mist (all channels ≥ 0.75)", "CO2 discharge drawn white (all channels ≥ 0.85)". The `unit` is `sRGB`, and the `where` is `crates/bevy-client/src/cues.rs, crates/bevy-client/src/main.rs`.
   - **Verdicts.** Water is `simplified` ("a colour code: water is faintly blue only in thick layers; a real jet looks clear or white"). CO2 is `correct` only if the fetched source supports a white cloud of dry-ice particles and condensed vapour; Frost is `correct` only if it supports liquid-nitrogen fog being condensed water vapour.
   - **Sources.**
     - The content work **fetches** each source it cites and records the URL and fetch date. It never cites from memory.
     - Suggested starting points: Braun and Smirnov, "Why is water blue?", J. Chem. Educ. 70(8), 612 (1993), for Water. NIST WebBook CO2 phase data (sublimation at 194.7 K, about −78 °C) plus a fire-safety or extinguisher-maker source on CO2 "snow" and the discharge cloud. A university or national-lab liquid-nitrogen safety sheet on the visible fog being condensed water vapour.
     - If a row cannot be sourced, the lane stops and reports it. It does not merge an unsourced row.
   - No player-facing text is added or removed. `npm test` stays green.

## Acceptance tests

**Unit tests** run with `rustc --edition 2021 --test crates/bevy-client/src/cues.rs -o /tmp/cues && /tmp/cues`. They fail on a0e2ecb because the functions do not exist.

1. **`stream_style`.**
   - For spells 0–3, `thick` is in [0.18, 0.25].
   - For spells 1–3: `outline_scale` is in [1.15, 1.35], and the colour limits in the table above hold.
   - Each outline's `contrast_ratio` is ≥ 3.0 against the sky `[0.55, 0.78, 0.98]` (`main.rs:322`) and ≥ 3.0 against its own core.
   - Each outline's `contrast_ratio` against a dark-green ground sample `[0.2, 0.4, 0.2]` is ≥ 2.0.
   - Water's core `contrast_ratio` against the sky is ≥ 2.0.
   - The Frost and CO2 streams' life follows the same rule as the others (`flight + STREAM_AFTER`, at least `STREAM_MIN_FRAMES`); this is a code-review point, not a unit test. Frost's life is about 0.23 s at minimum flight, against the old beam's 0.25 s.
   - `mist` is 0 for 0 and 1, and in 6..=12 for 2 and 3.
   - Index 9 returns the Water style.
   - `rel_luminance([1,1,1]) = 1 ± 1e-4`, `rel_luminance([0,0,0]) = 0`, and `contrast_ratio([1,1,1],[0,0,0]) = 21 ± 0.01`.
2. **`held_secs` and `hold_repeats`.** A driver steps frames of fixed `dt` over a press from `t_down` to `t_up`. The press is sampled at each frame start, `clicked` is true on the first frame that sees it, and the driver counts the frames where `hold_repeats` is true while the button is down.
   - At dt 1/60: a 0.30 s press and a 0.45 s press give 0 repeat frames. A 0.60 s press gives at least 1.
   - At dt 0.25 (4 fps): a press seen on 2 samples gives 0. A press seen on 3 samples gives at least 1.
   - `clicked` on a frame where `was_down` is true resets to 0. `!down` resets to 0.
   - A `dt` of NaN, −1 or +∞ adds 0.
   - `HOLD_DELAY == 0.5`.
3. **`fire_in_view`** with hfov 90 and vfov 58.72:
   - `([], false)` gives false, and `([], true)` gives true.
   - `[(0, 0)]` gives true. `[(39, 0)]` gives true. `[(41, 0)]` gives false. `[(41, 0), (−10, 3)]` gives true.
   - `[(NaN, 0)]` gives false.
4. **Existing plain-`rustc` tests still pass:** `cues.rs` (I-019's 5), `chem.rs`, `feel.rs`, `clock.rs`, `motion.rs`.

**e2e checks** are new `check(...)` calls in golden, in a new function `streamsAndFinder({ page, st, cmd, until, frames, sleep, idx, lit, locked, base, browser, errors })`. It is the same argument object the other sections take.
- **Placement.**
  - It is **defined immediately before `async function castingYouCanSee`** (`golden:1243`).
  - It is **called on a new line after the `feelAndStandards(...)` call** (`golden:341`), so it runs before `castingYouCanSee`.
  - These positions keep it clear of I-022's `reviewFollowups`, which is defined after `castingYouCanSee` and called after `golden:343`.
- **Exit state.** It ends with `fresh()`, unpaused, Hints and sway restored, and the pointer locked if it was locked on entry.
- **Helpers.** It defines its own copies of `within`, `unpause`, `fresh`, `tp`, `aimAt`, `relock`, `synthClick` and `hudRead`, in the pattern of `castingYouCanSee`. Those helpers are local there.
- **How it runs.** It runs as `QUERY=?low node tests/e2e/golden.mjs`. If the pointer cannot be locked, every test that needs the lock **fails**; none is skipped.
- **Failing first.** The lane runs the new section on a0e2ecb with the contract stub first, and records which checks fail. Tests 5, 6, 7, 9 and 11 must fail there. If test 7 passes on a0e2ecb for Water, the lane stops and reports the numbers to the planner instead of loosening anything.

5. **`fireInView` is a boolean, in place.** From fresh, `typeof state.fireInView === 'boolean'`, and the key order is `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS, 'fireInView']`. The stub makes the key-order check enforce this (Contract).
6. **The arrow stays away while a fire is aimed at or on screen.**
   - **(a) Aimed.** `tp 0 5.5`, `face lit[0]`, then wait for `aimObj === lit[0]`.
     - In one read (`hudRead`) within 5 frames: `fireOff === true`, `nearFire !== lit[0]` (the precondition: the nearest fire is off screen), `fireInView === true`, and `#firefinder` hidden.
     - If the precondition does not hold on this seed, the test tries `face lit[1]` and then `lit[2]`, uses the first one that meets it, and logs which. If none does, it fails.
     - On a0e2ecb the arrow shows (breaker shot 03).
   - **(b) Sweep.** From `tp 0 5.5`, for yaw in 0, 15, …, 345 at pitch 0 (`look Y 0`, then wait 2 frames), each single read must satisfy:
     - the finder is visible exactly when `playing && fireOff && !fireInView && nearFire >= 0 && !paused`;
     - when `objs[aimObj]?.burning`, `fireInView` is true.
   - **(c) Coverage.** Across the sweep, at least one yaw has `fireOff && fireInView` (the suppression case happened), and at least one yaw has the finder visible (the arrow still works).
   - **(d) Occlusion, recorded only.** `tp 0 -30` (north of the greenhouse, x −8..8, z −22..−12), `face lit[2]`, wait 2 frames. Log `aimObj`, `nearFire`, `fireOff`, `fireInView` and whether `#firefinder` is visible. The check passes when the finder rule from (b) holds in that read; the logged values go into the panel brief as a known gap (follow-up: occlusion-aware finder).
7. **Every stream stands out along its length, against the sky.** This is the stricter pixel test.
   - **Setup.** Locked, `set sway off`, `tp 0 5.5`, and `look 0 60` (only sky in view). For each spell in Water (`select 2`), Frost (`select 3`) and CO2 (`select 4`):
   - **Screenshots.** `#hud` and `#pause` get `visibility: hidden` through CSSOM for every screenshot in this test, and are restored afterwards. Take two "before" screenshots while paused, then unpause.
   - **Freeze.** An in-page `requestAnimationFrame` watcher sends one synthetic click. It calls `__game.pause(true)` on the first frame where `impacts` has increased and `streams ≥ 1`, so the stream is at full length, from the hand to its end point, and frozen. Then take the "after" screenshot.
   - **Visible pixel.** An after-pixel is visible when, compared with the same pixel in "before 1", its contrast ratio (`(L1+0.05)/(L2+0.05)` on `lumaOf`'s relative luminance) is ≥ 1.5 **or** its sRGB distance (0–255 per channel) is ≥ 90. A pixel that differs between the two before shots by the same rule is noisy.
   - **Cross-sections.**
     - `C` is the screen centre and `Hs = (VW/2 + 0.4f, VH/2 + 0.3f)`, with `f = (VH/2)/tan(fov/2)` from `state.fov`.
     - For each radius `r` in {20, 30, …, 110} px along C→Hs, take the segment perpendicular to C→Hs at that point, `±(0.4r + 24)` px long.
     - Find the longest run of visible pixels along it. Gaps of 2 px or fewer, and noisy pixels, are bridged but not counted.
     - The cross-section passes when the run is ≥ `0.6 r` px, which is three quarters of the 0.8 r a 0.2 m stream gives (see Facts).
   - **Pass rule.** For each spell, at least 8 of the 10 cross-sections pass, and the after-state has `streams ≥ 1`. Up to 3 attempts per spell, with a reset between them; one pass is enough. Every attempt logs its 10 run lengths and r.
   - **On a0e2ecb.** Water and CO2 fail: the glow against the sky is under both thresholds, or the 0.13 m tapered cone is under 0.6 r. Frost fails: the beam is 0.07 m, or 0.28 r.
8. **…and against the ground.**
   - **Pose.** Use the same method as test 7, from I-019 test 11's pose list (`golden:1420`): the first pose where `aimObj` is a burning fire at least 18 m away. Unpaused with no look change, so the background is ground, bushes and the fire.
   - **Pass rule.** A cross-section where more than 20 % of its pixels are noisy (flames, smoke) is skipped. At least 80 % of the cross-sections that are not skipped pass, with at least 6 not skipped. Each attempt logs the skipped count, so a pose that skips almost everything fails visibly rather than quietly. Test 8 is the only guard for the outline against the rendered ground; unit test 1's green sample is a proxy.
   - It runs for Water, Frost and CO2, with the same attempts and logging as test 7.
9. **A slow click casts once; a hold still repeats.**
   - **Slow click.** Locked, `look 0 60`, wait for `cooldown === 0`. An in-page sampler records `[gameTime, dt, casts]` on every new frame. Call `page.mouse.down()`, poll `state` every 20 ms until `gameTime ≥ g0 + 0.35`, then `page.mouse.up()`.
     - The attempt is valid when the logged `gameTime` span of the press (last sampled frame with the button down minus the first) is below `0.5 − max_dt`, where `max_dt` is the largest sampled `dt` during the press; that guarantees no frame saw `held ≥ 0.5`. An invalid attempt (span too long) is retried, up to 3 times, and the spans are logged.
     - After 20 more frames, `casts` has grown by exactly 1.
     - The exact boundaries (0.45 s vs 0.60 s, 2 vs 3 samples at 4 fps) are proved by unit test 2; this e2e only proves the live wiring.
     - On a0e2ecb it grows by 2, because the held path fires at about 0.3 s.
   - **Hold.** Same pose. An in-page sampler records `[gameTime, casts]` on every new frame. Call `page.mouse.down()`, hold until `gameTime ≥ g0 + 1.6`, then `page.mouse.up()`.
     - `casts` grows by at least 3.
     - The second cast's `gameTime` minus the first's is ≥ `0.5 − d`, where `d` is the largest `dt` between them.
     - I-019's `cast: holding the button keeps casting` (`golden:1376-1384`) also still passes, unedited.
10. **No flashing and no view-blocking mist from the new streams (GS-6, GS-17). Guard.**
    - Use I-019 test 19's method (`golden:1738-1756`): 20 consecutive-frame screenshots during a stream, for **Frost and CO2**, at two poses: far (`tp 0 5.5` aimed at lit[0]) and near (`tp 0 0`, `face lit[2]`, about 5 m; if `aimObj !== lit[2]` there, the lane picks the nearest pose under 6 m that aims at a burning fire and logs it). Four runs in all.
    - The frame-to-frame luminance-change fraction stays under 20 % for every pair.
    - **Coverage.** In every frame, the fraction of the viewport that is near-white mist stays under 15 %. A pixel counts when every channel is ≥ 215 in the frame and not in a "before" shot taken paused at the same pose.
    - I-019's Water and CO2 guard runs unchanged with the new looks.
11. **The hint order is pinned.**
    - `__game.pickHint` exists, and for this table it returns exactly. Fields not listed in a row default to `playing:true, idle:false, note:'', locked:false, extinguished:0, casts:0`.

      | case | result |
      |------|--------|
      | `playing:false` with anything else | `''` |
      | `idle:true, note:'N'` | the idle hint |
      | `idle:false, note:'N', locked:false` | `'N'` |
      | `locked:true, extinguished:0, note:''` | `Aim at a fire and click.` |
      | `locked:true, extinguished:1` | `''` |
      | `locked:false, casts:0, note:''` | the start hint |
      | `locked:false, casts:2, note:''` | `''` |

    - Live: fresh, click `#pointmode` from the pause card. Within 2 frames, `#hint` reads the point-and-click note (idle is not due right after `fresh`).
    - `pickHint` takes no Hints input: the Hints gate stays downstream (`setHint`, I-022's `hintAllowed`). Whichever of I-022 and I-025 merges second, its seam breaker checks live that with Hints off and a locked, no-fire-out state, `pickHint` still returns `Aim at a fire and click.` while `#hint` stays empty.
    - On a0e2ecb `pickHint` does not exist.
12. **The arrow still finds the fire (test 16, edited as sanctioned).** I-019 test 16 passes with the Contract item 3 edit. It logs whether the arrow hid because of `fireOff === false` or because of another fire in view.
13. **Science lint.** `npm test` passes, and the three new `## Parameters` rows are present with fetched sources.
14. **Golden still green.** Every existing check (98 at a0e2ecb), including I-007's section, I-019's section and its blue-pixel check, and I-023's clock section, passes 3/3 with `QUERY=?low`. The only edits to existing checks are the two in Contract (the key list and test 16's `found`). Run golden alone (I-017: concurrent Chromiums crash).
15. **Lane hygiene (I-022 and I-024).**
    - `git diff --name-only origin/main...` lists only paths in Lanes. It lists none of `feel.rs`, `world.rs`, `fx.rs`, `chem.rs`, `clock.rs`, `motion.rs`, `bridge.rs`, `player.rs`, `Cargo.toml`, `Cargo.lock`, `build-web.sh`, `tests/e2e/perf.mjs`, `index.html`, `style.css` or `analytics.js`.
    - `git diff -U0 origin/main... -- crates/bevy-client/src/main.rs` has no hunk that touches a0e2ecb lines 91–113, 402–412, 464, 948–988 or 989–1017.
    - `git diff -U0 ... -- crates/bevy-client/web/hud.js` has no hunk inside 441–444.
16. **Panel brief note.** The streams now have dark outlines, which is a toon convention. The panel brief says so, so that an outline is not reported as a "shadow" bug.
17. **B2/B3 guards (tester, no script change).**
    - Base is a0e2ecb built into `<scratchpad>/I-025/base-pkg-out`.
    - Run `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs` in 3 interleaved pairs (do not set `PERF_ALLOW_LOAD`). The median CPU-frame p50 must be ≤ base + 0.5 ms, and p99 ≤ base + 2 ms.
    - The Brotli size of `pkg-out` grows by at most 5 KB.
    - If I-024's `perf-pairs.mjs` has merged by then, use it instead with the same limits.

## GS walk (screens, controls and effects this item touches)

| GS | P | status |
|----|---|--------|
| 6 | P0 | met: a stream covers at most about 6 % of the frame with its 1.35× outline (about 4–5 % for the core alone; the near end at the hand is about 256 px wide, 345 px with the outline). It is the largest object on screen during a cast, in the lower right, and within GS-6's 20 %. A lead-in gap ahead of the hand was considered and rejected: I-019's first-frame blue-pixel check and GS-12 (stream visible within 2 frames) depend on the near end, and a gap would leave a short stream invisible on its first frames. Mist stays 2 m or more from the eye; test 10 guards flashing and mist coverage (< 15 %) for Frost and CO2, near and far |
| 9 | P1 | improved: a click casts once; holding repeats only after 0.5 s, like keyboard auto-repeat |
| 11 | P0 | unchanged: tap casts in point mode do not use the hold path |
| 12 | P0 | unchanged: the cast and stream still appear within 2 frames (I-019 test 10) |
| 15 | P0 | **met better**: the stream is a visible cast on its own channel, alongside I-007's sound |
| 17 | P0 | **met better**: the stream is visible to a young child against any background (tests 7 and 8) |
| 19 | P0 | met: spells are told apart mainly by the hotbar icon, formula and sound. Stream colour is a secondary cue: Water (blue, no mist) is clearly different; Frost (white to pale cyan) and CO2 (white) look alike, both rods in cold fog. No shape difference is claimed |
| 24 | P0 | **met better**: the arrow appears only when no fire is in view, so it never sends a child away from the fire they are aiming at |
| 21 | P0 | unchanged: no new text |
| 30 | P0 | kept: each stream looks like its chemistry (liquid, white cold cloud, fog), which supports picking the right spell |
| 32 | P1 | improved: the CO2 and Frost "cold" is shown as fog, next to the formula on the hotbar |
| 3 | P0 | unchanged: no shake or bob added (test 7 turns sway off only to make the pixel test stable) |

## Out of scope

- **I-019 breaker nits not in the brief:** `Frosty` keeps emissive 0.25 (a follow-up: cold objects do not glow); the flight cap hiding the 20 vs 30 m/s speed difference; the fire object looking like a peach cone rather than a flame.
- **The sinking CO2 cloud** at impact, `P::Co2`, `CO2_SINK` and P-016: still I-019's follow-up. The mist here is part of the stream, uses `P::Frost`, and makes no rate claim.
- **Occlusion-aware finder logic** (a fire behind the greenhouse counts as "in view" when its bearing is on screen). Named follow-up: **"occlusion-aware finder"**, to be filed with the next free item id. It would use `world::raycast` with `pick`'s line-of-sight rule for both `fire_off` and `fire_in_view`, and needs an arrow that can point at an on-screen but hidden fire. Test 6(d) records the current behaviour so the panel brief can warn about it.
- **Fire's stream look** beyond the shared thickness: no outline, no contrast test.
- **`fx.rs`**: no new particle kind, colour or physics.
- **I-022's work:** the `hintAllowed` gate, `setHint`, `feel.rs`, `Settings`, the aspect and `fov_y` lines and `tap_cone` in `aim_and_cast`, the age picker and analytics.
- **I-024's work:** `Cargo.toml` profiles, `world::build`/`Builder`/the palette and the `world::build` call, `perf.mjs` and `perf-pairs`.
- **Other changes:** `index.html`, `style.css`, `README.md`, the panel harness, the on-screen pause button, and the clock (`clock.rs`, `motion.rs`, the 0.25 s ceiling, the cooldown carry).

## Contract

A stub landed by the contract builder on the branch base, before the lanes start. It has no gameplay behaviour; both lanes code against it.

1. **`cues.rs` additions, with neutral bodies.**
   - `stream_style` returns `{ thick: 0.13, core: spell colour, outline: core, outline_scale: 1.0, mist: 0 }`.
   - `rel_luminance` and `contrast_ratio` are real (one line each).
   - `fire_in_view` returns `false`.
   - `HOLD_DELAY` is 0.5.
   - `held_secs` returns 0, and `hold_repeats` returns `false`, so the stub disables the hold path's gate. The stub must **not** wire `hold_repeats` into `main.rs`; the rust lane does that.
   - No Bevy types, so it compiles alone with `rustc --test`.
2. **New `__gameState` key `fireInView`** (bool, neutral `false`). It is appended after `fxWater`, as the last key, via a `Game.fire_in_view` field the stub adds. The stub extends the format string at `main.rs:1581` and its arguments at `main.rs:1633-1634`.
3. **Golden edits to existing checks (the stub makes both; no lane redoes them).**
   - **Key-order check** (`golden:1045-1047`): add `const I025_KEYS = ['fireInView'];` and require `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS, ...I025_KEYS]` in order. `keys.length` grows by `I025_KEYS.length`. The check's name and its other conditions stay as they are.
   - **I-019 test 16's `found`** (`golden:1617`) becomes `!!h.ff && !h.ff.vis && h.s.fireInView === true && (h.s.fireOff === true || Math.abs(h.s.fireBearing) <= hfovOf(h.s.fov) / 2)`. The arrow may now hide because a nearer-than-arrow fire came into view (see Facts: lit[1] enters view before lit[2] on this turn). The check's name and its other conditions (`consistent`, `steps <= 40`) stay.
   - With the stub's neutral `fireInView = false`, the edited `found` fails until the rust lane lands. That is expected, and the shared build is the gate.
4. **DOM and hooks** (web lane only; listed so tests and the panel can rely on them): `window.__game.pickHint(obj)`. `#firefinder` keeps its id and classes. No new CSS.
5. **Meanings both sides rely on.**
   - `fireInView` means that some burning object's bounds centre is on screen by `off_screen`'s 5° margin, or that the aimed object is burning.
   - `streams` counts `Stream` entities only (Frost now included, outline children excluded).
   - `clicks` is unchanged.

### Overlap with I-022 and I-024 (both specced on a0e2ecb)

| file | I-025 hunks (a0e2ecb lines) | I-022 / I-024 hunks | result |
|------|-----------------------------|---------------------|--------|
| `main.rs` | `Beam` struct 209-213 (removed); `STREAM_THICK` 124 (removed); `stream_transform`/`spawn_stream`/`Stream` 214-271; `SpellMats` 274-277; `Game` new fields 175-185; `setup` materials 507-516; `aim_and_cast`: new block after 1017 (before 1018), `clicked` at 1036-1039, hold state and gate 1069-1077, Frost/stream spawn 1099-1136; `projectiles` 1159-1205 and impact mist; format string 1581 and args 1633-1634; optional native `just_pressed` (untested) | **I-022:** `Settings` 91-106, `fov_y` 108-113, `read_settings` literal 402-412; `aim_and_cast` parameter 952, one new line after 980, lines 983 and 987. **I-024:** the `world::build` call at 464 only | **No shared lines.** The nearest pair is I-022's 987 and I-025's insertion after 1017, with the whole finder (989-1017, unedited) between them. I-025 reads `fov_y` and `aspect` and never edits their lines. After I-022, `fov_y` is `vfov_for(hfov, aspect)`, so `fireInView`'s hfov is the true View width; I-025 needs no change for that. I-025's `setup` hunk (507-516) is 43 lines from I-024's 464 |
| `hud.js` | `__game` 230-238 (+`pickHint`); `frame()` 665-673 (`pickHint` call, `findOn`); new module-level `pickHint` after the constants (after `hud.js:39`, before `export function startHud` at 118; not near 441) | **I-022:** `setHint` + `hintAllowed` 441-444 only | No overlap. I-022's gate sits downstream of `pickHint`, so a hint `pickHint` picks is still gated by Hints |
| `golden.mjs` | call line after 341; new function before 1243; edits at 1045-1047 and 1617 | **I-022:** call line after 343; `reviewFollowups` after 1758. **I-024:** no golden change | Insertions are separated by the unchanged lines 342-343 and by the whole of `castingYouCanSee`. If a textual conflict does appear at the call lines, keep both lines in the order feel, I-025, I-019, I-022 |
| `content/science-sources.md` | 3 new `## Parameters` rows | **I-022:** none. **I-024:** none | none |
| `world.rs`, `fx.rs`, `feel.rs`, `Cargo.toml`, `perf.mjs` | **not touched** | owned by I-024 / I-022 | I-024's Part R rule (kit meshes never mutated) is kept: I-025 scales `kit.cyl` through `Transform` only |

**Pixel checks across items.** I-024 test 12 relies on I-019's stream pixel checks to guard Part R under `?low`. This item keeps those checks and adds stricter ones (tests 7 and 8). If I-024 merges after this item, its golden run includes tests 7 and 8, which also read the rendered picture. I-020 and I-021 are blocked or specced on older bases, and were not checked hunk by hunk.

## Lanes

| lane | owns | builds |
|------|------|--------|
| rust | `crates/bevy-client/src/cues.rs` (append only, below I-019's code), `crates/bevy-client/src/main.rs` (only the hunks in the Overlap table) | Unit tests 1–3 first. Then: `stream_style` values tuned inside the limits; stream and outline materials in `SpellMats`; constant-width `kit.cyl` stream and an outline child for Water, Frost and CO2; Frost through `spawn_stream` with `Beam` removed; mist from `P::Frost` with the game-dt accumulator, the cap and the far-70 %/2 m spawn rule; `fire_in_view` and its new block after the finder; `held_secs`/`hold_repeats` wired at the held path. Builds native (`cargo build`) and wasm. Tunes colours with tests 7 and 8 from the web lane on the shared build. Must not touch the I-022 and I-024 ranges in test 15, `fx.rs`, `world.rs`, `chem.rs`, `clock.rs`, `motion.rs`, `feel.rs`, `bridge.rs`, `player.rs`, `web/**` or tests |
| web | `crates/bevy-client/web/hud.js` (only `pickHint`, the `frame()` lines 665-673 and `__game`), `tests/e2e/golden.mjs` (the new `streamsAndFinder` function and its call line; the two Contract edits come from the stub), `content/science-sources.md` (the three `## Parameters` rows and their sources), `docs/loop/items/I-019/spec.md` (one appended amendment paragraph) | Tests 5–11 first, in `streamsAndFinder`, run on a0e2ecb plus the stub, with the expected failures recorded. Then `pickHint`, the `findOn` change, `__game.pickHint`, the science rows with fetched sources, and the I-019 amendment. Must not touch `setHint` (441-444), `index.html`, `style.css`, `age.js`, `analytics.js`, `boot.js`, the panel harness, or any existing golden check beyond the stub's two edits |

**Seam:** tests 5–9 need the rust lane. The web lane writes them against the contract, and they go green at the shared build after both lanes merge. Test 11 needs only the web lane. The seam breaker checks:

- that `fireInView` means the same thing on both sides (Contract 5), and that the finder rule in `hud.js` is exactly `playing && fireOff && !fireInView && nearFire >= 0 && !paused`;
- that a 0.35 s press gives one cast at the shared build, and that a click plus a hold never casts twice in one frame or more often than once per 0.3 s of game time after the first repeat;
- that `streams` excludes outline children (a Water cast must show `streams === 1`, not 2);
- that the Frost stream freezes while paused and disappears on reset, as the others do, and that golden's stream checks see `streams >= 1` on the first impact frame for Frost (true at a0e2ecb because `projectiles` handles the stream before the impact on the same frame);
- that the three new science rows cite sources whose URLs were fetched (listed in the web lane's round note).
