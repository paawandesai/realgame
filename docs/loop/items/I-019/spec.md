# I-019: Casting you can see

**based_on:** origin/main 3dbd332e2c3941011c3d0b957be4b2d9b1d44cd3 (2026-10-30, after I-013 and I-014 merged; the nightly-1 panel artifact).

**Revision 2** (after critique). The CO2 sinking cloud and `#pausebtn` are cut and become follow-ups. The `fire` filter and the `aim_and_cast` sites are added to the contract. Tests 12, 13 and 16 through 19 are fixed. Planner hygiene: the primary checkout (661b9ba) has neither `docs/loop/gameplay-standards.md` nor `crates/bevy-client/src`. The GS walk uses the worktree copy of the standards, and the facts below come from `git show 3dbd332:...`. Before dispatch, the planner makes sure builders branch from the real origin/main, where both exist.

**Depends on I-007** (feel pass, building on `loop/I-007` at 870ce29, not merged yet). This item builds on I-007's contract and does not redo any of it. Builders branch from origin/main **after I-007 merges**. Before dispatch, the planner moves `based_on` to that merge commit and re-checks the facts below. If I-007's merged contract differs from the list in "Relied on from I-007", the planner refreshes this spec first.

## Facts from 3dbd332 this spec relies on

- **A click under pointer lock is sampled, not latched.**
  - `hud.js` sets `__gameIn.fire = true` on a locked left `pointerdown` and `false` on `pointerup`.
  - `bridge::poll` reads `fire` once per game frame.
  - `aim_and_cast` casts only when `firing && cooldown <= 0` on that frame.
  - So a click whose down and up both fall between two game frames casts nothing. The panel's `click` is a 60 ms press (`daemon.mjs`), and frames at 4× throttle run 35 ms or longer, so many clicks are lost.
  - The golden check `left click casts while locked` passes only because it holds the button down (`page.mouse.down()` with no `up`).
- **One spell that lands puts a fire out at once.** In `chem::apply_inner`, `Water`, `Frost` and `Foam` on a burning object each emit `Extinguished` in that same call. No chemistry needs a hold.
- **Spells are slow to land.** Projectile `dur = dist / speed`, with Water 30, Fire 32, Foam (CO2) 20 and Frost 120 m/s. At 18 m, Water takes 0.6 s and CO2 0.9 s.
  - Game time is `delta_secs().min(0.05)`. On a slow frame the game clock runs behind the wall clock, so at 4× throttle a CO2 cast takes several wall-clock seconds to land.
  - Missed clicks, flight time and the 0.3 s cooldown add up to the panel's "hold 1.5–7 s for water, about 10 s for CO2".
- **Nothing about a spell is visible.**
  - A cast spawns one ball: Water at scale 0.26, CO2 at 0.4.
  - On impact it adds a 14-particle burst at size 0.22 (`fx.burst(.., 14, 3.0, 0.22)`).
  - There is no stream from the hand and no cloud.
  - The particle pool is 160 under `?low` and 320 otherwise. `Fx::emit` caps pending particles at 256 per frame.
- **Fire is the default spell.** `Game` derives `Default`, so `spell = 0`, which is Fire. `reset()` keeps the current spell. Fire on a dry bush sets `counts = false` ("lit by the player"), so putting it out does not count toward the objective.
- **The objective shows two numbers.** `hud.js` writes `Fires put out: ${min(ext, goal)} / ${goal} · burning now: ${burning}`.
  - The win condition is `extinguished >= GOAL_FIRES (3) && burning() == 0`.
  - When a bush the player lit goes out, `extinguished` stays the same, so "0 / 3" can sit next to "burning now: 7".
- **There is no way to find a fire.** The state has no bearing or position for any fire. `objs[]` carries `burning` but no position.
- **Hot bushes glow.** `world::look_of` returns `Look::Hot` for a flammable object above 120 °C. That look uses `glow(materials, srgb(0.75, 0.38, 0.12), 0.35)`, an emissive orange.
  - Heated, unburnt bushes next to a fire therefore glow orange, and players aim at them.
  - The orange glow also implies that wood below 300 °C gives off visible light, which is an implied visual claim.
- **The aim cue cannot tell a fire from a bush.**
  - `#crosshair.on` means "any target", whether or not it is burning.
  - The locked hint reads "Aim at a glowing target: the crosshair turns yellow…".
  - In point mode, `canvas.hover` is set for any hovered target.
- **Escape.** `hud.js` handles `Escape` only when `!input.locked`. When locked, it relies on the browser releasing the lock and `pointerlockchange` then opening the pause screen.
  - Under automation (Playwright `keyboard.press('Escape')`), the lock is not released. The golden Esc check falls back to calling `document.exitPointerLock()` itself.
  - There is no `P` key and no on-screen pause button.
- **Command routing.** In `main.rs` the per-frame command filter keeps only `c.starts_with("tap") || c == "cast"` (line 662), and `aim_and_cast` acts on `a.first() == Some(&"cast")` (line 864). Any other command is dropped. While paused, `game.cast_queue` is cleared (line 847).
- **Turn sign.** `turn dx dy` calls `player.look`, which does `yaw -= dx · LOOK_PER_PX · sens`. With right = `(cos yaw, −sin yaw)`, a **positive** `dx` turns the view **right**, toward a positive `fireBearing`.
- **The hint ignores the banners.** `setHint` runs on every frame without checking `outcome`. While unlocked with `casts == 0`, "Click to look around…" stays on screen under the lose banner.
- **Frame counters.** `chem_tick` steps the world once per rendered frame, so `tick` counts frames during play. `index.html` is off limits because I-005 is editing it. All new DOM is built by `hud.js` with `createElement`. CSP is `style-src 'self'`; CSSOM writes are allowed.

## Relied on from I-007 (its contract; not rebuilt here)

- **State fields:** `fov` (vertical, degrees), `cooldown`, `cooldownMax`, `hitKind`, `hitX`, `hitY`, `hitFrame`.
- **Commands:** `set sens|hfov|sway|assist`.
- **DOM:** `#hitmarker` (`.hm-x`, `.hm-slash`, `.hm-star`), `#crosshair.cooling`, `#crosshair.pulse`, the settings block (`#hints` and the others), and `--ui` text scale.
- **Test hook:** `__game.lastSound()`.
- **Idle hint:** `IDLE_HINT` plus the Water-slot `nudge`, which already hides when `outcome != playing`.
- **Look speed:** `LOOK_PER_PX = 0.00095`, so 150 px ≈ 8.2° at sens 1.
- **Hint order:** I-007's priority order is kept and extended (see 9 below).
- The `out` star marker and the cheer sound are the "it worked" reaction. This item makes the cast reach the target and be seen. I-007 makes the result felt.

## Problem

At 3dbd332, all 5 personas in the nightly panel reported a control blocker (9 in total; B4 median clarity 6). For a 6-year-old the core loop is invisible:
- A single click usually does nothing. When it does cast, a small ball flies for half a second or more and makes a tiny splash.
- The default spell is Fire, so the first cast feeds the fire, and that new fire does not count when it is put out.
- The objective reads 0/3 while "burning now" reads 7.
- Nothing points to the last fire.
- Hot bushes glow like targets.
- Escape does nothing under automation, and there is no other way to pause from the keyboard.
- The start hint stays on screen under the lose banner.

**Goal:** casting is legible to a 6-year-old within a single click. One click on a fire sends a visible stream that reaches it within a quarter of a second and puts it out, and the HUD shows one number going down.

## Bars moved

- **B4 (every age can play):** the intended movement. It is a panel bar, so nothing is reported as moved at merge.
  - Prediction for the first paired panel run (same seed, I-007 merge artifact vs this one): control blockers fall from 9 to at most 2.
  - No persona reports "click does nothing", "could not see the spell", "could not find the fire", "the numbers don't match", or "Esc did not work".
  - Median clarity rises by at least 1, from 6 to 7 or more.
  - `ttc` stays at 30 s or less for all personas.
- **B5 (fun):** secondary. The prediction is that median fun rises by at least 1. Casts that land and show are the main fun lever after control.
- **B1 (golden path):** must stay green. The new checks go into `golden.mjs`.
- **B2, B3:** must not regress (test 21).
- **B6:** must stay at 0 errors. This item adds no Parameters row. It adds and edits not-claims rows, and it removes one implied visual claim (the hot glow).

## Smallest version

1. **Every click casts exactly once (latched click).**
   - On a locked left `pointerdown`, `hud.js` pushes the command `fire`, and still sets `input.fire = true` for holds.
   - Rust treats `fire` like `cast`: it goes into `game.cast_queue`, waits at most 0.3 s through the cooldown, and is dropped while paused.
   - A click is never lost between frames. A click during the cooldown fires when the cooldown ends, if that is within 0.3 s.
   - The held-button path stays as it is, and it is skipped on a frame where a queued cast fired, as today, so a press that is still down after 0.3 s repeats. A short click casts once.
   - The new state field `clicks` counts the `fire` commands received. It is incremented when the command is read, **before** the paused-queue clear. So `clicks` +1 with `casts` +0 while paused is legal.
   - Rust sites: `fire` is added to the line-662 filter (`c.starts_with("tap") || c == "cast" || c == "fire"`) and to the cast branch in `aim_and_cast` (`Some(&"cast") | Some(&"fire")`). That alias is the whole implementation of the latch, because the queue already exists. The Rust lane does not build a second queue.
   - The first click that takes the pointer lock still does not cast. The existing golden check `a click over a target locks the pointer and does not cast` stays.
2. **Spells land within 0.25 s.**
   - New Bevy-free module `crates/bevy-client/src/cues.rs` with `pub fn flight_secs(spell: u8, dist: f32) -> f32 = clamp(dist / speed(spell), 0.03, 0.25)`. The spell index is 0..4 in `Spell::ALL` order, so `cues.rs` compiles alone with `rustc --test`. An index of 4 or more gives 0.03.
   - Speeds stay as they are, and the cap applies to all four spells. Non-finite or non-positive `dist` gives 0.03.
   - `aim_and_cast` uses it for `Projectile.dur`.
3. **A water stream and a splash you can see.**
   - On a Water cast, Rust spawns a **stream**: one entity, a stretched cube or capsule in the water material, from the hand to the moving projectile head.
   - The stream is at least 0.12 m thick at the hand and tapers to the head. It stays until 0.2 s after impact, and it is despawned by an epoch change (reset), like `Beam`.
   - At impact, the Water burst becomes at least 24 particles at size 0.35 or more, plus 8 `Steam` particles when the target was burning.
   - Fire and CO2 casts reuse the same code: one `spawn_stream(material)` helper serves Water, Fire and CO2, not three paths. Fire uses the fire material. CO2 uses the existing foam material (pale), and gets no new particle kind and no cloud. Frost keeps its existing beam, and that beam counts as a stream.
   - New state field `streams`: the number of live stream and beam entities.
   - New state field `fxWater` (u32): the number of live `Water` particles. It is the one count test 12 needs.
   - **Pool budget:** under `?low`, a stream plus a splash uses at most 40 particles per cast. The 160-slot ring overwrites the oldest particles, which is acceptable.
4. *(Cut in revision 2: the sinking CO2 cloud. Follow-up "CO2 cloud that sinks", see Out of scope.)*
5. **Water is the default spell.**
   - On boot, `Game.spell = 1`. Set it in `setup` or with a `Game::new()`, not by changing `Default` semantics elsewhere.
   - `reset()` still keeps the current spell (unchanged).
   - The startup caption in `main.rs:507` changes from "…Press 2 for Water." to `Save the Greenhouse! Put out 3 fires before they spread. Water is ready: aim at a fire and click.`
6. **One fire counter.**
   - `cues::fires_left(goal, extinguished, burning) -> u32 = max(goal.saturating_sub(extinguished), burning)`. It is 0 exactly when the level is won.
   - New state field `firesLeft`.
   - `#obj-line` becomes `Fires to put out: ${state.firesLeft}`, followed by a row of `.fire-pip` flame icons, one per fire left, at most 8, with a `+` element after 8. The icons are the cue for players who cannot read yet (GS-21, GS-32).
   - "burning now" and the `x / y` form are removed. On a win, the line reads `Greenhouse saved!` as today.
   - The rule stays the same, including that a fire the player lit does not count. The counter is honest about it: it goes up when fire spreads, and it never shows 0 while anything burns.
7. **An arrow to the nearest fire.**
   - Rust computes, every frame:
     - `nearFire`: the index of the nearest burning object to the player in the horizontal plane, or −1.
     - `fireBearing`: the signed degrees from the view's forward direction to that object's bounds centre, positive to the right, in (−180, 180], 1 decimal.
     - `fireElev`: the degrees above or below the view centre.
     - `fireOff`: true when the fire is off screen.
   - Pure helpers in `cues.rs`:
     - `bearing_deg(px, pz, yaw, tx, tz)`. Forward is `(-sin yaw, -cos yaw)` and right is `(cos yaw, -sin yaw)`, matching `player.rs`.
     - `off_screen(bearing, elev, hfov, vfov) = |bearing| > hfov/2 − 5 || |elev| > vfov/2 − 5`. Any non-finite input gives false.
     - `hfov` comes from I-007's live `fov` and the camera aspect.
   - `hud.js` injects `#firefinder`: a chevron plus a flame icon, at least 56 px, at the left or right edge, vertically centred.
     - It has the class `right` when `fireBearing > 0`, else `left`. A fire behind the player takes the side of the bearing's sign.
     - It is shown only while `fireOff && nearFire >= 0 && outcome === 'playing' && !paused`.
     - It pulses slowly, scale 1.0 to 1.1 at 1 Hz or slower (GS-6). Under reduced motion it has `animation: none`.
     - It has no text.
8. **The highlight tells a fire from a bush.**
   - `#crosshair` gets the class `fire` while locked and `objs[aimObj].burning`. The HUD derives this from `objs`, and Rust adds nothing for it.
   - `.fire` adds a shape cue on top of I-007's `on`: a second, outer ring drawn with `box-shadow`, flame-orange. It must not look the same in greyscale (GS-19).
   - `on` keeps its meaning. I-007 and the panel `hud.mjs` read it.
   - In point mode, the canvas also gets `hover-fire` when the hovered target is burning. `cursor: pointer` applies only with `hover-fire`. A plain `hover` gets `cursor: crosshair`, and clicking it still casts (soaking a bush is legal).
   - The locked hint becomes `Aim at a fire and click.` It is shown while locked, `extinguished === 0` and `outcome === 'playing'`.
   - The `hot` state material loses its emissive. It becomes a darker, browner, non-emissive base colour, so a heating bush looks scorched and does not glow.
     - The per-look emissive strengths move into `cues::look_emissive(look) -> f32`, with the `Look` enum moved to `cues.rs` and re-exported from `world.rs`.
     - `world.rs` builds the `hot` and `burning` materials from it. `look_of` stays in `world.rs`.
     - `Burning` keeps its current emissive value unchanged, so a fire stays the brightest thing in the courtyard.
9. **Pausing works everywhere.**
   - `Escape` keydown while locked calls `document.exitPointerLock()` and `showPause(true)` in the same handler. While unlocked, `Escape` behaves as today.
   - `KeyP` (by `e.code`, so AZERTY is fine) does the same. While paused, `P` acts like `#resume`: it calls `showPause(false)` and `lock()`, and if the lock is refused the game still unpauses.
   - `hud.js` appends a controls row `Pause` / `Esc or P` to the pause table with `createElement`.
   - The on-screen pause button is deferred (follow-up, see Out of scope). The target is WASD + mouse, and Esc and P fix the reported "Esc did not work".
   - **Hint priority**, highest first:
     1. A visible `#banner`. Every hint hides, and so do `#firefinder` and the nudge.
     2. `note`.
     3. I-007's idle hint.
     4. The locked hint.
     5. The unlocked start hint.
10. **Science register** (`content/science-sources.md`). These are the exact strings:
    - No new Parameters row (P-016 moves to the CO2-cloud follow-up).
    - **Not-claims rows:**
      - `Aim at a fire and click.` (hud.js)
      - `Fires to put out: ${state.firesLeft}` (hud.js)
      - `Save the Greenhouse! Put out 3 fires before they spread. Water is ready: aim at a fire and click.` (main.rs). This replaces the "…Press 2 for Water." row.
    - **Rows removed:** the rows for `Aim at a glowing target…` and the old objective template, now that the strings are gone.
    - `npm test` stays green.

## Acceptance tests

**Unit tests** run with `rustc --edition 2021 --test crates/bevy-client/src/cues.rs -o /tmp/cues && /tmp/cues`. `cues.rs` has no Bevy types. They fail today because the module does not exist.

1. `fires_left`:
   - `(3, 0, 3)` gives 3, `(3, 0, 5)` gives 5, `(3, 1, 2)` gives 2, `(3, 2, 0)` gives 1, `(3, 3, 0)` gives 0, `(3, 5, 1)` gives 1.
   - It never returns 0 when `burning > 0`. Check this for every input in 0..=8³.
2. `bearing_deg` from (0, 0):
   - At yaw 0: target (0, −10) gives 0. (10, −10) gives +45 ± 0.01. (−10, −10) gives −45 ± 0.01. (0, 10) gives |180| ± 0.01.
   - At yaw π/2: (−10, 0) gives 0 ± 0.01, and (0, −10) gives +90 ± 0.01.
   - The test checks agreement with `player.rs`: right = `(cos yaw, −sin yaw)`.
3. `off_screen` with hfov 90 and vfov 58.72:
   - (0, 0) gives false. (39, 0) gives false. (41, 0) gives true. (−170, 0) gives true.
   - (0, 25) gives true. (0, 24) gives false.
   - NaN in any argument gives false.
4. `flight_secs`, for each spell and `d` in {0.5, 1, 18, 60, 120}:
   - The result lies in [0.03, 0.25] and does not decrease as `d` grows.
   - `flight_secs(1, 18) ≤ 0.25` (Water), while 3dbd332 gives 0.6. `flight_secs(7, 18)` gives 0.03.
   - `d` NaN, 0 or −5 gives 0.03.
5. `look_emissive`: `Hot` gives exactly 0.0, and `Burning` gives more than 0.5 and equals the value `world.rs` used at 3dbd332 for the burning material.
6. The existing `rustc --test crates/bevy-client/src/chem.rs` tests, and I-007's `feel.rs` tests, still pass.

**e2e checks** are new `check(...)` calls in `tests/e2e/golden.mjs`. They go in a function `castingYouCanSee(...)` called after I-007's `feelAndStandards(...)` and before `no console errors`, so that check covers them too. They run as `QUERY=?low node tests/e2e/golden.mjs`.
- "Within N frames" means N increments of `state.frame`.
- Existing and I-007 checks are not edited.
- Each test starts from `reset`, except test 7, which reloads, and test 18.
- If the pointer cannot be locked, every test that needs the lock **fails**. It is not skipped.
- `lit` is the starting fire list as golden computes it.
- `hfovOf(v) = 2·atan(tan(v/2)·VW/VH)`.
- The `golden.mjs` code is written against I-007's merged helpers (`face`, `castAt`, the I-007 pose and its `hitKind`, `fov` and `on`-class checks). Tests 8, 15 and 16 depend on them.
- Synthetic events: the `hud.js` `pointerdown` handler must **not** gate on `event.isTrusted`. Tests 8 and 10 dispatch synthetic events.

7. **Water by default.** After `page.reload()`, the first state has `spell === 1` and `.slot[data-spell="2"]` has `sel`. On 3dbd332 `spell` is 0.
8. **A click casts once and puts a fire out (latched edge).**
   - Locked. `tp 0 5.5`, `face lit[0]`, then wait for `aimObj === lit[0] && cooldown === 0 && spell === 1`.
   - In **one** `page.evaluate`, dispatch `pointerdown` and then `pointerup` (`button 0`, `pointerType 'mouse'`, `bubbles`) on the canvas, so both land in the same task, before any game frame.
   - Within 12 frames:
     - `clicks` +1, `casts` exactly +1, `impacts` +1;
     - `objs[lit[0]].burning === false`, `extinguished` +1;
     - `hitKind === 'out'` (I-007);
     - `firesLeft` −1.
   - On 3dbd332 `casts` does not change.
9. **A real click casts exactly once; a hold still repeats.**
   - `face lit[1]`. Call `page.mouse.click(VW/2, VH/2)` with no delay.
   - Within 20 frames, `casts` is exactly +1, `objs[lit[1]].burning === false`, and `casts` is still +1 after 20 more frames.
   - Then `look 0 60` (sky), then `page.mouse.down()`, 1500 ms wall, `page.mouse.up()`. `casts` grows by at least 2 during the hold.
10. **Latency (GS-12).** An in-page `requestAnimationFrame` loop dispatches one `pointerdown`/`pointerup` pair while aimed at a burning target with `cooldown === 0`.
    - `casts` increments within 2 frames.
    - `streams ≥ 1` within 2 frames.
11. **Spells land within a quarter second.**
    - Pose: `tp 0 -20`, then `face` the counted fire nearest z = −2 (Courtyard HALF = 24 m, so this pose exists). If that gives under 18 m on the merged seed, the builder picks another pose and logs it. Either way the test logs the distance and asserts that it is ≥ 18 with `aimObj` on the fire.
    - A Water click gives `impacts` +1 within K frames, where `K = max(10, ceil(0.25 / dtMean) + 2)` and `dtMean` is the mean of `min(0.05, wall-clock frame interval)` over the 10 frames before the click. The test logs both.
    - Reset, then the same with CO2 (`select 4`).
    - On 3dbd332, under the golden's software GPU (dt clamped to 0.05), Water at 18 m needs 12 or more frames and CO2 18 or more.
12. **The stream and splash are visible.** On a Water click at a burning target:
    - `streams ≥ 1` in at least 3 consecutive published frames, starting within 2 frames of `casts` changing.
    - In the frame where `impacts` changes, or the next one, `fxWater ≥ 20`.
    - **Pixel check (race-free).** From the same pose:
      - Take two "before" screenshots and count the pixels in the box x 45–80%, y 45–90% of the viewport with `B > R + 40 && B > 120` (water blue). The noise is the difference between the two counts.
      - Click. An in-page `requestAnimationFrame` watcher calls `__game.pause(true)` in the first frame where `streams ≥ 1`, so the stream is frozen. Then take the "after" screenshot and unpause.
      - The after count must exceed the larger before count by at least noise + 0.3% of the viewport's pixels.
      - Up to 3 attempts (reset between them). One pass is enough, and every attempt's numbers are logged.
    - On 3dbd332, `streams` is undefined.
13. *(Cut in revision 2 along with the CO2 cloud.)*
14. **One fire counter.**
    - From fresh, `#obj-line`'s text, with whitespace collapsed, matches `/^Fires to put out: (\d+)$/` (the pips have no text). The captured number equals `firesLeft`, which equals 3. There are 3 `.fire-pip` elements, and the text has no "burning now" and no "/".
    - `tick 2400`, then wait until `burning > 3`. Within 2 frames, `firesLeft === max(goal − extinguished, burning)`, the obj-line number equals it, and the `.fire-pip` count is `min(firesLeft, 8)`.
    - Cast Water at a counted burning fire. Within 2 frames of `impacts` changing, the number drops by exactly 1.
    - On 3dbd332 the text is "Fires put out: 0 / 3 · burning now: 3".
15. **The highlight tells a fire from a bush.** Locked.
    - Use I-007's pose: `tp 4 2`, then aim at non-burning bush 5 (step-turn until `aimObj === 5`). `#crosshair` has `on` and does not have `fire`.
    - `face lit[0]`, then wait for `aimObj === lit[0]`. `#crosshair` has both `on` and `fire`.
    - The computed `box-shadow` of `#crosshair.fire span` is not `none`, and it differs from the value without `fire`.
    - `#hint` reads `Aim at a fire and click.`
    - Then `__game.pause(true)` and point mode, as golden already does. Hovering lit[2] (cursor at the canvas centre after `face`) gives the canvas `hover-fire`. Hovering bush 5 gives `hover` without `hover-fire`.
    - On 3dbd332 there is no `fire` class.
16. **Following the arrow finds the fire.**
    - From fresh, `tp 0 5.5`, `face lit[0]`, read `yaw`, then `look (yaw + 180) 0`.
    - Within 5 frames: `fireOff === true`, `nearFire ≥ 0`, `objs[nearFire].burning`. `#firefinder` is visible with the class `right` if `fireBearing > 0`, else `left`, and its box is at least 56 px wide.
    - Then step-turn toward the arrow, up to 40 steps. Each step is `turn 150 0` when the class is `right` and `turn -150 0` when it is `left` (a positive `turn` turns the view right; see the facts). After each step, one `page.evaluate` reads both the state and the DOM. In that same read, the class must equal the sign of that read's `fireBearing` (`right` iff `> 0`). Nothing is compared across reads, so a bearing near ±180 that flips between frames is fine. Stop when `#firefinder` is hidden. It must hide within 40 steps.
    - At that point `fireOff === false` and `|fireBearing| ≤ hfovOf(fov)/2`.
    - `__game.pause(true)` hides `#firefinder` within 2 frames.
    - On 3dbd332 there is no `#firefinder`.
17. **Pause under automation.** Locked.
    - `page.keyboard.press('Escape')`. Within 10 frames: `#pause` visible, `state.paused`, `!state.locked`, and `document.pointerLockElement === null`. The test does not call `exitPointerLock`.
    - Click `#resume`, wait for `locked`, then `page.keyboard.press('KeyP')`. Same result.
    - `KeyP` again. Within 10 frames, `state.paused === false`.
    - The pause card has a row containing `Esc or P`.
    - On 3dbd332 the Escape press does not pause. That is why golden falls back.
18. **Nothing under the banner.**
    - Fresh, unlocked (point mode), no casts. Run `tick 2400` until `outcome === 'lost'`.
    - Within 2 frames: `#hint` is hidden or empty, `#firefinder` is hidden, and there is no `.slot.nudge`.
    - Then reset and win with three `cast`s, using golden's `castAt` pattern on the three counted fires. The same holds under the win banner.
    - On 3dbd332, "Click to look around…" stays visible under the lose banner.
19. **No flashing from the new effects (GS-6). Guard: it passes on 3dbd332 and must keep passing.** Use I-007 test 19's in-Node PNG method.
    - Take 20 consecutive-frame screenshots during a Water stream and splash, then 20 during a CO2 stream. The fraction of pixels whose relative luminance changes by 10% or more stays under 20% of the frame for every pair.
20. **Science lint.** `npm test` passes, with the new not-claims rows present and the removed rows gone.
21. **B2/B3 guards (tester, no script change).**
    - Base is the I-007 merge commit, built into `<scratchpad>/I-019/base-pkg-out`.
    - Run `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs`, 3 interleaved pairs. The median CPU frame p50 must be ≤ base + 1.0 ms.
    - Brotli of `pkg-out` grows by at most 30 KB.
22. **Golden still green.** All existing and I-007 checks pass 3/3 with `QUERY=?low`, with no threshold edits. Run golden in isolation (I-017: concurrent Chromiums crash).

## GS walk (screens, controls and effects this item touches)

| GS | P | status |
|----|---|--------|
| 6 | P0 | met for new effects; test 19. The finder pulses at 1 Hz or slower and has none under reduced motion |
| 7 | P1 | met: P and Esc both pause everything (existing pause semantics) |
| 9 | P1 | partial: `KeyP` bound by `code`; no hold is needed to cast any more (a click casts) |
| 10 | P0 | unchanged (I-007 owns assist) |
| 11 | P0 | improved: point mode gets `hover-fire` and Water default; the on-screen pause button is a follow-up |
| 12 | P0 | **met**: test 10 (cast and stream within 2 frames); test 11 (lands within 0.25 s) |
| 13 | P1 | partial: Esc/P pause from the keyboard under every driver |
| 15 | P0 | **met** with I-007: a stream (visual) plus I-007's cast and hit sounds; `fire` ring is a shape cue; test 15 |
| 16 | P1 | improved: impact splash on the impact frame, alongside I-007's marker and sound |
| 17 | P0 | **met** with I-007: a visible stream plus a big splash plus I-007's star and cheer |
| 19 | P0 | met: `fire` vs `on` differ by ring shape; the finder is an arrow shape; pips are icons |
| 21 | P0 | met: new copy is 2 short sentences at grade 2 or below; the counter and finder have icon equivalents |
| 22 | P1 | unchanged: no new click targets (the finder is not clickable); the 44 px pause button is deferred to a follow-up |
| 24 | P0 | **met**: one specific number ("Fires to put out: N") plus icons; a waypoint arrow to the nearest fire |
| 30 | P0 | kept: picking the spell is still the decision. Water default only removes the accidental Fire-first trap; Fire, Frost and CO2 remain one key away |
| 32 | P1 | unchanged: the sinking CO2 cloud is a follow-up |
| 23 | P0 | not met here: the safe first minute is I-020's walk-to-marker plus the GS-23 tutorial row |

## Out of scope

- **A per-fire "putting out" progress ring.** The backlog row asked for one, but one landed spell already puts a fire out (chem facts above). The "7–10 s" came from lost clicks and flight time, which 1 and 2 fix. A ring would show progress that does not exist.
- **Any change to the chemistry rules**, including the rule that a fire the player lit does not count, GOAL_FIRES and spread pacing. `chem.rs` is not edited.
- **The first-30-seconds work** (walk-to-marker, on-screen WASD, age-band captions, explaining spread): that is I-020.
- New sounds beyond I-007's set, frost mist at impact (the Frost beam already shows), FireLight placement, and occlusion-aware finder logic (a fire behind the greenhouse counts as "on screen" when its bearing is in view).
- The browser's own Esc handling in real Chrome: the existing `pointerlockchange` path still covers it.
- `index.html`, `boot.js`, `build-web.sh`, `functions/`, `dashboard/`, and the panel harness (`tests/e2e/panel/**`).
- `crates/bevy-client/README.md`: the controls table will be stale (P, Water default). The planner files a one-line docs follow-up.
- **Follow-ups the planner files in the backlog** (cut from this item in revision 2):
  - "CO2 cloud that sinks": `P::Co2`, `CO2_SINK` and its P-016 science row, a CO2 impact cloud, and a time-based sink test on game seconds with a margin of at least 2× (GS-32).
  - "On-screen pause button": `#pausebtn` ≥ 44×44 px for point-mode and touch players, propagation stopped, hidden under banners (GS-11, GS-22).
- **Panel brief note:** the counter can read, for example, "Fires to put out: 7" while the win needs 3 (fire spread). This is intended and pinned by test 14. The panel brief must tell personas this so that it is not reported as a bug.

## Contract

A stub landed by the contract builder on the branch base, before the lanes start. It has no gameplay behaviour; both lanes code against it.

- **`cues.rs`** with these signatures and `todo!()`-free placeholder bodies that return neutral values. `mod cues;` goes in `main.rs`.
  - `fires_left(goal: u32, ext: u32, burning: u32) -> u32`
  - `bearing_deg(px: f32, pz: f32, yaw: f32, tx: f32, tz: f32) -> f32`
  - `off_screen(bearing: f32, elev: f32, hfov: f32, vfov: f32) -> bool`
  - `flight_secs(spell: u8, dist: f32) -> f32`, where the spell index is 0..4 in `Spell::ALL` order (no copy of `Spell` in `cues.rs`)
  - `pub enum Look { Normal, Burning, Charred, Wet, Frosty, Foamed, Hot }`, re-exported from `world.rs`
  - `look_emissive(look: Look) -> f32`
- **New `__gameState` fields**, published with neutral values by the stub:

  | field | type | neutral value |
  |-------|------|---------------|
  | `clicks` | u32 | 0 |
  | `firesLeft` | u32 | computed for real; it is one line |
  | `nearFire` | i32 | −1 |
  | `fireBearing` | f32, 1 dp | 0 |
  | `fireElev` | f32, 1 dp | 0 |
  | `fireOff` | bool | false |
  | `streams` | u32 | 0 |
  | `fxWater` | u32 | 0 |

- **The `fire` command:** a queued cast. It is not debug-gated. The stub makes the real change, and the Rust lane does not redo it:
  - `fire` is added to the line-662 filter: `c.starts_with("tap") || c == "cast" || c == "fire"`.
  - The cast branch in `aim_and_cast` matches `Some(&"cast") | Some(&"fire")`.
  - `clicks` is incremented when a `fire` command is read, before the paused-queue clear.
  - That alias is the latch, because the existing `cast_queue` already gives the 0.3 s wait and the drop while paused.
- **DOM ids and classes** (web lane only; listed so tests and the panel can rely on them): `#firefinder.left|.right`, `.fire-pip`, `#crosshair.fire`, `canvas.hover-fire`.

## Lanes

| lane | owns | builds |
|------|------|--------|
| rust | `crates/bevy-client/src/**` (`main.rs`, `cues.rs`, `fx.rs`, `world.rs`); `bridge.rs` is not edited | Unit tests 1–5 first. Then: `cues.rs` bodies; `flight_secs` in `aim_and_cast`; one `spawn_stream(material)` for Water, Fire (fire material) and CO2 (existing foam material), despawned on epoch change, and the `streams` count; bigger Water splash and `fxWater`; Water default spell; new startup caption; `firesLeft`; `nearFire`, `fireBearing`, `fireElev` and `fireOff` from the live hfov and aspect; `hot` material made non-emissive through `look_emissive`, `Burning` emissive unchanged. The `fire` routing and `clicks` come from the contract stub. Must not touch `chem.rs` rules, `web/**` or tests |
| web | `crates/bevy-client/web/hud.js`, `crates/bevy-client/web/style.css`, `tests/e2e/golden.mjs`, `content/science-sources.md` (the rows in 10 only) | `fire` push on locked pointerdown (no `isTrusted` gate); `#obj-line` counter and pips; `#firefinder`; `#crosshair.fire`, `canvas.hover-fire` and the new locked hint; Esc-while-locked and `KeyP` handling and the pause-table row; hint priority with banner suppression; reduced-motion rules; science rows; e2e tests 7–12 and 14–19 in `castingYouCanSee`, written against I-007's merged `golden.mjs` helpers. Must not touch `index.html`, `boot.js` or the panel harness |

**Seam:** the web lane's tests 8–12 and 16 need the rust lane's fields. Each lane builds against the contract stub. The shared build after both lanes merge runs the full golden. The seam breaker checks that `fire`, `clicks`, `streams` and `fxWater` mean the same thing on both sides, and that a held press plus a click never casts twice in one frame.
