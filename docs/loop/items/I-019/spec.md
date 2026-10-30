# I-019: Casting you can see

**based_on:** origin/main 265a93e3c53c0046bf2493e52bc85f6d984676fd (2026-10-30, wave 7: after I-007, I-005, I-018 and I-023 merged). Revision 2 was written against 3dbd332; the wave-7 refresh at the end lists every change.

**Revision 2** (after critique), **refreshed for wave 7**. The CO2 sinking cloud and `#pausebtn` are cut and become follow-ups. The `fire` filter and the `aim_and_cast` sites are added to the contract. Tests 12, 13 and 16 through 19 are fixed. Planner hygiene: the primary checkout (661b9ba) still has no `crates/bevy-client/src`. The facts below come from `git show 265a93e:...`, and builders branch from the real origin/main.

**Depended on I-007** (feel pass). I-007 is merged (855abfc), and its contract as merged matches the list in "Relied on from I-007" (checked at 265a93e). This item builds on that contract and does not redo any of it. **I-023** (game clock) is also merged; this item follows its rules (see the clock facts below).

## Facts from 265a93e this spec relies on

Line numbers are at 265a93e. `main.rs` means `crates/bevy-client/src/main.rs`, `hud.js` means `crates/bevy-client/web/hud.js`.

- **A click under pointer lock is sampled, not latched.**
  - A document-level listener (`hud.js:334`) sets `input.fire = true` on a locked left `pointerdown`; `hud.js:335` sets it `false` on `pointerup`. The canvas `pointerdown` handler (`hud.js:344`) returns early while locked.
  - `bridge::poll` reads `fire` once per game frame (`control`, `main.rs:602`).
  - `aim_and_cast` (`main.rs:865`) casts from the held button only when `casts.is_empty() && game.locked && !game.paused && firing && game.cooldown <= 0.0` on that frame (`main.rs:958`).
  - So a click whose down and up both fall between two game frames casts nothing. The panel's `click` is a 60 ms press (`daemon.mjs`), and frames at 4× throttle run 35 ms or longer, so many clicks are lost.
  - The golden check `left click casts while locked` (`golden.mjs:171`) passes only because it holds the button down (`page.mouse.down()` with no `up`).
  - I-023 added a cooldown carry (`main.rs:967–977`): when the cooldown ran out during this frame, the overshoot counts toward the next cooldown. It applies to every cast in the cast loop, queued or held.
- **One spell that lands puts a fire out at once.** In `chem::apply_inner`, `Water`, `Frost` and `Foam` on a burning object each emit `Extinguished` in that same call. No chemistry needs a hold. (`chem.rs` changed since 3dbd332 only by adding `HitKind`/`hit_kind` for I-007.)
- **Spells are slow to land.** Projectile `dur = (dist / speed).max(0.03)` (`main.rs:1015`), with speeds at `main.rs:999–1004`: Water 30, Fire 32, Foam (CO2) 20 and Frost 120 m/s. At 18 m, Water takes 0.6 s and CO2 0.9 s of game time.
  - **Game clock (I-023).** Every per-frame system takes `dt = clock::frame_dt(time.delta_secs())`, clamped to `[0, MAX_FRAME_DT = 0.25]` (`clock.rs`). Bevy's virtual clock, which drives the fixed-step chemistry, has the same ceiling (`main.rs:250–258`). The game clock therefore keeps wall-clock pace down to 4 fps, and a 0.6 s flight takes about 0.6 s of wall time even at 4× throttle.
  - At 3dbd332 the clamp was 0.05, so a CO2 cast took several wall-clock seconds to land under throttle. Missed clicks, flight time and the 0.3 s cooldown added up to the panel's "hold 1.5–7 s for water, about 10 s for CO2". I-023 removed the clock part; lost clicks and the 0.6–0.9 s flight remain.
  - The state publishes `gameTime` (game seconds since the last reset; frozen while paused) and `dt` (this frame's clamped dt) as the last two keys (`main.rs:1413`, `1457–1458`).
- **Nothing about a spell is visible.**
  - A cast spawns one ball: Water at scale 0.26, CO2 at 0.4 (`main.rs:999–1004`).
  - On impact it adds a 14-particle burst at size 0.22 (`fx.burst(burst, p.to, 14, 3.0, 0.22)`, `main.rs:1102`).
  - There is no stream from the hand and no cloud.
  - The particle pool is 160 under `?low` and 320 otherwise (`main.rs:393`). `Fx::emit` caps pending particles at 256 per frame (`fx.rs:71`).
  - **Frost's `Beam` ticks down while paused.** In `projectiles` the beam loop (`main.rs:1042–1049`) runs before the `if game.paused { return; }` at `main.rs:1050`.
- **Fire is the default spell.** `Game` derives `Default` (`main.rs:124–125`), so `spell = 0`, which is Fire. `reset()` (`main.rs:795`) keeps the current spell. Fire on a dry bush sets `counts = false` ("lit by the player"), so putting it out does not count toward the objective. I-005's age picker (shown only when analytics is on, never in `pkg-out`) resets the game once after a pick, and that reset keeps the spell.
- **The objective shows two numbers.** `hud.js:592–594` writes `Fires put out: ${Math.min(state.extinguished, state.goal)} / ${state.goal}  ·  burning now: ${state.burning}`.
  - The win condition is `extinguished >= GOAL_FIRES (3) && burning() == 0`.
  - When a bush the player lit goes out, `extinguished` stays the same, so "0 / 3" can sit next to "burning now: 7".
  - `index.html:21` has the static placeholder `Fires put out: 0 / 3`, which `hud.js` overwrites on the first state.
- **There is no way to find a fire.** The state has no bearing or position for any fire. `objs[]` carries `burning` but no position.
- **Hot bushes glow.** `world::look_of` (`world.rs:137`) returns `Look::Hot` for a flammable object above 120 °C. `state_mats` builds that look as `hot: glow(materials, Color::srgb(0.75, 0.38, 0.12), 0.35)` (`world.rs:325`), an emissive orange. `burning` is `glow(.., srgb(0.55, 0.2, 0.05), 1.2)` (`world.rs:326`). The `Look` enum is at `world.rs:127`.
  - Heated, unburnt bushes next to a fire therefore glow orange, and players aim at them.
  - The orange glow also implies that wood below 300 °C gives off visible light, which is an implied visual claim.
- **The aim cue cannot tell a fire from a bush.**
  - `#crosshair.on` means "any target", whether or not it is burning (`hud.js:577`).
  - The locked hint reads `Aim at a glowing target: the crosshair turns yellow. Left click casts.` while `state.impacts < 3` (`hud.js:584`).
  - In point mode, `canvas.hover` is set for any hovered target (`hud.js:578`), and `style.css:17` gives it `cursor: pointer`.
- **Escape.** `hud.js:385–386` handles `Escape` only when `!input.locked`. When locked, it relies on the browser releasing the lock and `pointerlockchange` then opening the pause screen (`syncLock`, `hud.js:228–248`).
  - Under automation (Playwright `keyboard.press('Escape')`), the lock is not released. The golden Esc check falls back to calling `document.exitPointerLock()` itself (`golden.mjs:297`; the clock section does the same).
  - There is no `P` key and no on-screen pause button. The pause table (`index.html:38–47`) ends with `Free the mouse | Esc`.
- **Command routing.** In `control`, the per-frame command filter keeps only `c.starts_with("tap") || c == "cast"` (`main.rs:735`). In `aim_and_cast`, `incoming = std::mem::take(&mut web.cmds)` (`main.rs:923`); while paused `game.cast_queue` is cleared (`main.rs:924–925`); otherwise commands queue for at most 0.3 s (`main.rs:926–933`). The cast branch acts on `a.first() == Some(&"cast")` (`main.rs:942`). Any other command is dropped.
- **Turn sign.** `turn dx dy` calls `player.look` (`player.rs:67`), which does `yaw -= dx · LOOK_PER_PX · sens`. With right = `(cos yaw, −sin yaw)`, a **positive** `dx` turns the view **right**, toward a positive `fireBearing`.
- **The hint ignores the banners.** The hint block (`hud.js:580–591`) runs on every frame without checking `outcome`. While unlocked with `casts == 0`, "Click to look around…" stays on screen under the lose banner.
- **Counters and DOM.** `chem_tick` runs in `FixedUpdate` at `chem::DT = 1/30 s` of virtual-clock time, so `tick` counts chemistry steps, not frames. The `tick N` debug command steps the chemistry N times at once. `frame` counts published frames. `index.html` stays out of scope for this item (scope unchanged); all new DOM is built by `hud.js` with `createElement`. CSP is `style-src 'self'`; CSSOM writes are allowed.
- **Golden pins the state keys (I-023).** The `clock and budget` section (`golden.mjs:994`) checks `keys.length === BASE_KEYS.length + 2` with `gameTime`, `dt` last (`golden.mjs:1041–1042`), and every frame's key list must stay the same. Any new state field breaks that check unless the check's expected list names it (see Contract).
- **Golden pins `clock::frame_dt` (I-023).** The static check at `golden.mjs:1187–1199` fails on any `min(0.05` in `src/*.rs` and requires `clock::frame_dt(` in `control`, `aim_and_cast`, `projectiles`, `sync_visuals` and `fx::update_fx`.

## Relied on from I-007 (merged at 855abfc; checked at 265a93e; not rebuilt here)

- **State fields:** `fov` (vertical, degrees, `settings.fov_y()`), `cooldown`, `cooldownMax`, `hitKind`, `hitX`, `hitY`, `hitFrame`.
- **Commands:** `set sens|hfov|sway|assist`.
- **DOM:** `#hitmarker` (`.hm-x`, `.hm-slash`, `.hm-star`), `#crosshair.cooling`, `#crosshair.pulse`, the settings block (`#hints` and the others), and `--ui` text scale.
- **Test hook:** `__game.lastSound()`.
- **Idle hint:** `IDLE_HINT` (`hud.js:27`) plus the Water-slot `nudge` (`IDLE_SLOT = 1`), which already hides when `outcome != playing` (`updateIdle`, `hud.js:531`).
- **Look speed:** `feel::LOOK_PER_PX = 0.00095`, so 150 px ≈ 8.2° at sens 1.
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

I-007 and I-023 since fixed the feel and the game clock, but none of the causes above (all still present at 265a93e).

**Goal:** casting is legible to a 6-year-old within a single click. One click on a fire sends a visible stream that reaches it within a quarter of a second of game time and puts it out, and the HUD shows one number going down.

## Bars moved

- **B4 (every age can play):** the intended movement. It is a panel bar, so nothing is reported as moved at merge.
  - Prediction for the first paired panel run (same seed, the 265a93e artifact vs this one): control blockers fall to at most 2.
  - No persona reports "click does nothing", "could not see the spell", "could not find the fire", "the numbers don't match", or "Esc did not work".
  - Median clarity rises by at least 1 over the paired base.
  - `ttc` stays at 30 s or less for all personas.
- **B5 (fun):** secondary. The prediction is that median fun rises by at least 1. Casts that land and show are the main fun lever after control.
- **B1 (golden path):** must stay green. The new checks go into `golden.mjs`.
- **B2, B3:** must not regress (test 21).
- **B6:** must stay at 0 errors. This item adds no Parameters row. It adds and edits not-claims rows, and it removes one implied visual claim (the hot glow).

## Smallest version

1. **Every click casts exactly once (latched click).**
   - On a locked left `pointerdown`, the document listener at `hud.js:334` pushes the command `fire` (via `cmd`), and still sets `input.fire = true` for holds.
   - Rust treats `fire` like `cast`: it goes into `game.cast_queue`, waits at most 0.3 s through the cooldown, and is dropped while paused.
   - A click is never lost between frames. A click during the cooldown fires when the cooldown ends, if that is within 0.3 s.
   - The held-button path stays as it is, and it is skipped on a frame where a queued cast fired (the `casts.is_empty()` guard at `main.rs:958`), so a press that is still down after 0.3 s repeats. A short click casts once. I-023's cooldown carry applies to the queued cast unchanged.
   - The new state field `clicks` counts the `fire` commands received. It is incremented in `aim_and_cast` when `incoming` is taken (`main.rs:923`), **before** the paused-queue clear (`main.rs:924–925`). So `clicks` +1 with `casts` +0 while paused is legal.
   - Rust sites: `fire` is added to the filter at `main.rs:735` (`c.starts_with("tap") || c == "cast" || c == "fire"`) and to the cast branch at `main.rs:942` (`Some(&"cast") | Some(&"fire")`). That alias is the whole implementation of the latch, because the queue already exists. The Rust lane does not build a second queue.
   - The first click that takes the pointer lock still does not cast. The existing golden check `a click over a target locks the pointer and does not cast` (`golden.mjs:118`) stays.
2. **Spells land within 0.25 s of game time.**
   - New Bevy-free module `crates/bevy-client/src/cues.rs` with `pub fn flight_secs(spell: u8, dist: f32) -> f32 = clamp(dist / speed(spell), 0.03, 0.25)`. The spell index is 0..4 in `Spell::ALL` order, so `cues.rs` compiles alone with `rustc --test` (like `clock.rs`, `feel.rs`, `motion.rs`). An index of 4 or more gives 0.03.
   - Speeds stay as they are, and the cap applies to all four spells. Non-finite or non-positive `dist` gives 0.03.
   - `aim_and_cast` uses it for `Projectile.dur` (replacing `(dist / speed).max(0.03)` at `main.rs:1015`). `Projectile.t` keeps advancing by `clock::frame_dt`.
3. **A water stream and a splash you can see.**
   - On a Water cast, Rust spawns a **stream**: one entity, a stretched cube or capsule in the water material, from the hand to the moving projectile head.
   - The stream is at least 0.12 m thick at the hand and tapers to the head. It stays until 0.2 s of **game time** after impact. Its life counts down by `clock::frame_dt` only while not paused (unlike `Beam`, which ticks while paused), so pausing freezes it. It is despawned by an epoch change (reset), like `Beam`.
   - At impact, the Water burst becomes at least 24 particles at size 0.35 or more, plus 8 `Steam` particles when the target was burning.
   - Fire and CO2 casts reuse the same code: one `spawn_stream(material)` helper serves Water, Fire and CO2, not three paths. Fire uses the fire material. CO2 uses the existing foam material (pale), and gets no new particle kind and no cloud. Frost keeps its existing beam, and that beam counts as a stream.
   - Any new per-frame system or function that advances streams takes its dt from `clock::frame_dt(time.delta_secs())` and contains no `min(0.05` (golden clock check 8).
   - New state field `streams`: the number of live stream and beam entities.
   - New state field `fxWater` (u32): the number of live `Water` particles. It is the one count test 12 needs.
   - **Pool budget:** under `?low`, a stream plus a splash uses at most 40 particles per cast. The 160-slot ring overwrites the oldest particles, which is acceptable.
4. *(Cut in revision 2: the sinking CO2 cloud. Follow-up "CO2 cloud that sinks", see Out of scope.)*
5. **Water is the default spell.**
   - On boot, `Game.spell = 1`. Set it in `setup` (`main.rs:376`, which already holds the `Game` resource) or with a `Game::new()`, not by changing `Default` semantics elsewhere.
   - `reset()` still keeps the current spell (unchanged), so I-005's post-pick reset keeps Water.
   - The startup caption at `main.rs:555` changes from "…Press 2 for Water." to `Save the Greenhouse! Put out 3 fires before they spread. Water is ready: aim at a fire and click.` The reset caption (`main.rs:809`) is unchanged.
6. **One fire counter.**
   - `cues::fires_left(goal, extinguished, burning) -> u32 = max(goal.saturating_sub(extinguished), burning)`. It is 0 exactly when the level is won.
   - New state field `firesLeft`.
   - `#obj-line` (written at `hud.js:592–594`) becomes `Fires to put out: ${state.firesLeft}`, followed by a row of `.fire-pip` flame icons, one per fire left, at most 8, with a `+` element after 8. The icons are the cue for players who cannot read yet (GS-21, GS-32).
   - "burning now" and the `x / y` form are removed. On a win, the line reads `Greenhouse saved!` as today. The `index.html` placeholder is left as it is (out of scope); `hud.js` replaces it on the first state.
   - The rule stays the same, including that a fire the player lit does not count. The counter is honest about it: it goes up when fire spreads, and it never shows 0 while anything burns.
7. **An arrow to the nearest fire.**
   - Rust computes, every frame:
     - `nearFire`: the index of the nearest burning object to the player in the horizontal plane, or −1.
     - `fireBearing`: the signed degrees from the view's forward direction to that object's bounds centre (`chem.bounds(i).center()`, as `face` uses), positive to the right, in (−180, 180], 1 decimal.
     - `fireElev`: the degrees above or below the view centre.
     - `fireOff`: true when the fire is off screen.
   - Pure helpers in `cues.rs`:
     - `bearing_deg(px, pz, yaw, tx, tz)`. Forward is `(-sin yaw, -cos yaw)` and right is `(cos yaw, -sin yaw)`, matching `player.rs`.
     - `off_screen(bearing, elev, hfov, vfov) = |bearing| > hfov/2 − 5 || |elev| > vfov/2 − 5`. Any non-finite input gives false.
     - `vfov` is the live `settings.fov_y()` (the state's `fov`), and `hfov = 2·atan(tan(vfov/2)·aspect)` with the same camera `aspect` that `aim_and_cast` computes (`main.rs:892–898`).
   - `hud.js` injects `#firefinder`: a chevron plus a flame icon, at least 56 px, at the left or right edge, vertically centred.
     - It has the class `right` when `fireBearing > 0`, else `left`. A fire behind the player takes the side of the bearing's sign.
     - It is shown only while `fireOff && nearFire >= 0 && outcome === 'playing' && !paused`.
     - It pulses slowly, scale 1.0 to 1.1 at 1 Hz or slower (GS-6). Under reduced motion it has `animation: none`.
     - It has no text.
8. **The highlight tells a fire from a bush.**
   - `#crosshair` gets the class `fire` while locked and `objs[aimObj].burning`, next to the existing `on` toggle (`hud.js:577`). The HUD derives this from `objs`, and Rust adds nothing for it.
   - `.fire` adds a shape cue on top of I-007's `on`: a second, outer ring drawn with `box-shadow` on `#crosshair span`, flame-orange. It must not look the same in greyscale (GS-19).
   - `on` keeps its meaning. I-007 and the panel `hud.mjs` read it.
   - In point mode, the canvas also gets `hover-fire` when the hovered target is burning (next to `hud.js:578`). `cursor: pointer` applies only with `hover-fire`. A plain `hover` gets `cursor: crosshair` (changing `style.css:17`), and clicking it still casts (soaking a bush is legal).
   - The locked hint becomes `Aim at a fire and click.` (replacing the string at `hud.js:584`). It is shown while locked, `extinguished === 0` and `outcome === 'playing'`.
   - The `hot` state material loses its emissive. It becomes a darker, browner, non-emissive base colour, so a heating bush looks scorched and does not glow.
     - The per-look emissive strengths move into `cues::look_emissive(look) -> f32`, with the `Look` enum moved from `world.rs:127` to `cues.rs` and re-exported from `world.rs`.
     - `state_mats` in `world.rs` builds the `hot` and `burning` materials from it. `look_of` stays in `world.rs`.
     - `Burning` keeps its current emissive strength (1.2) unchanged, so a fire stays the brightest thing in the courtyard.
9. **Pausing works everywhere.**
   - `Escape` keydown while locked calls `document.exitPointerLock()` and `showPause(true)` in the same handler (`hud.js:385`). While unlocked, `Escape` behaves as today. The later `pointerlockchange` → `syncLock` → `showPause(true)` is then a no-op.
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
      - `Save the Greenhouse! Put out 3 fires before they spread. Water is ready: aim at a fire and click.` (main.rs). This replaces the "…Press 2 for Water." row (`science-sources.md:89`).
    - **Rows removed:** the rows for `Aim at a glowing target: the crosshair turns yellow. Left click casts.` (`science-sources.md:83`) and the old objective template (`science-sources.md:79`), now that the strings are gone. The point-mode note (`:85`), the `index.html` controls line (`:94`) and `IDLE_HINT` (`:95`) stay, because their strings stay.
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
   - `flight_secs(1, 18) ≤ 0.25` (Water), while 265a93e gives 0.6. `flight_secs(7, 18)` gives 0.03.
   - `d` NaN, 0 or −5 gives 0.03.
5. `look_emissive`: `Hot` gives exactly 0.0, and `Burning` gives exactly 1.2, the strength `world.rs:326` uses for the burning material at 265a93e.
6. The existing plain-`rustc` tests still pass: `chem.rs`, `feel.rs` (I-007), `clock.rs` and `motion.rs` (I-023).

**e2e checks** are new `check(...)` calls in `tests/e2e/golden.mjs`. They go in a function `castingYouCanSee(...)` called after I-007's `feelAndStandards(...)` (`golden.mjs:341`) and before `no console errors` (`golden.mjs:343`), so that check covers them too. (`clockAndBudget(...)` runs before `feelAndStandards` and is not moved.) They run as `QUERY=?low node tests/e2e/golden.mjs`.
- "Within N frames" means N increments of `state.frame`. Durations are measured in `state.gameTime` where the spec says "game seconds".
- Existing and I-007/I-023 checks are not edited, with one exception required by the contract: the expected key list in the clock section's key-order check (see Contract).
- Each test starts from `reset`, except test 7, which reloads, and test 18.
- If the pointer cannot be locked, every test that needs the lock **fails**. It is not skipped.
- `lit` is the starting fire list as golden computes it (`golden.mjs:99`).
- `hfovOf(v) = 2·atan(tan(v/2)·VW/VH)`; golden already defines it at module level (`golden.mjs:359`, 16:9), along with `decodePng`, `lumaOf` and `flashFrac`.
- **Helpers.** `castingYouCanSee` takes the same arguments `feelAndStandards` does (`{ page, st, cmd, until, frames, sleep, idx, lit, locked, base, browser, errors }`). `fresh`, `tp`, `within`, `unpause` and `aimAt` are local to `feelAndStandards`, and `castAt` is local to the main body, so `castingYouCanSee` defines its own copies in the same pattern (or the builder hoists them to module level without changing their behaviour). Tests 8, 15 and 16 use them and the `face`, `tp`, `turn` and `look` debug commands.
- Synthetic events: the `hud.js` `pointerdown` handler must **not** gate on `event.isTrusted`. Tests 8 and 10 dispatch synthetic events (they bubble from the canvas to the document listener).

7. **Water by default.** After `page.reload()`, the first state has `spell === 1` and `.slot[data-spell="2"]` has `sel`. On 265a93e `spell` is 0.
8. **A click casts once and puts a fire out (latched edge).**
   - Locked. `tp 0 5.5`, `face lit[0]`, then wait for `aimObj === lit[0] && cooldown === 0 && spell === 1`.
   - In **one** `page.evaluate`, dispatch `pointerdown` and then `pointerup` (`button 0`, `pointerType 'mouse'`, `bubbles`) on the canvas, so both land in the same task, before any game frame.
   - Within 12 frames:
     - `clicks` +1, `casts` exactly +1, `impacts` +1;
     - `objs[lit[0]].burning === false`, `extinguished` +1;
     - `hitKind === 'out'` (I-007);
     - `firesLeft` −1.
   - On 265a93e `casts` does not change.
9. **A real click casts exactly once; a hold still repeats.**
   - `face lit[1]`. Call `page.mouse.click(VW/2, VH/2)` with no delay.
   - Within 20 frames, `casts` is exactly +1, `objs[lit[1]].burning === false`, and `casts` is still +1 after 20 more frames.
   - Then `look 0 60` (sky), then `page.mouse.down()`, hold until `gameTime` has advanced by at least 1.5 s (capped at 15 s wall), `page.mouse.up()`. `casts` grows by at least 2 during the hold.
10. **Latency (GS-12).** An in-page `requestAnimationFrame` loop dispatches one `pointerdown`/`pointerup` pair while aimed at a burning target with `cooldown === 0`.
    - `casts` increments within 2 frames.
    - `streams ≥ 1` within 2 frames.
11. **Spells land within a quarter second of game time.**
    - Pose: `tp 0 -20`, then `face` the counted fire nearest z = −2 (`world::HALF = 24` m, so this pose exists). If that gives under 18 m on the merged seed, the builder picks another pose and logs it (I-023's 7b pose, (−13.5, 12) to target 0, is 15 m and too short). Either way the test logs the distance and asserts that it is ≥ 18 with `aimObj` on the fire.
    - An in-page `requestAnimationFrame` sampler records `[frame, gameTime, dt, casts, impacts]` for every new `state.frame` (the clock section's `__clk` sampler pattern). A Water click is sent. With `g_c` the `gameTime` of the first row where `casts` changed and `g_i` the `gameTime` of the first row where `impacts` changed, assert `g_i − g_c ≤ 0.25 + d`, where `d` is the largest `dt` in the rows from `g_c` to `g_i` (one frame of resolution). Also assert that `impacts` changed within 60 frames or 15 s wall, whichever is later. The test logs `g_i − g_c`, `d` and the frame count.
    - Reset, then the same with CO2 (`select 4`).
    - On 265a93e, Water at 18 m needs about 0.6 s of game time and CO2 about 0.9 s, so both fail.
12. **The stream and splash are visible.** On a Water click at a burning target:
    - `streams ≥ 1` in at least 3 consecutive published frames, starting within 2 frames of `casts` changing.
    - In the frame where `impacts` changes, or the next one, `fxWater ≥ 20`.
    - **Pixel check (race-free).** From the same pose:
      - Take two "before" screenshots and count the pixels in the box x 45–80%, y 45–90% of the viewport with `B > R + 40 && B > 120` (water blue). The noise is the difference between the two counts.
      - Click. An in-page `requestAnimationFrame` watcher calls `__game.pause(true)` in the first frame where `streams ≥ 1`, so the stream is frozen (its life does not count down while paused; see 3). Then take the "after" screenshot and unpause.
      - The after count must exceed the larger before count by at least noise + 0.3% of the viewport's pixels.
      - Up to 3 attempts (reset between them). One pass is enough, and every attempt's numbers are logged.
    - On 265a93e, `streams` is undefined.
13. *(Cut in revision 2 along with the CO2 cloud.)*
14. **One fire counter.**
    - From fresh, `#obj-line`'s text, with whitespace collapsed, matches `/^Fires to put out: (\d+)$/` (the pips have no text). The captured number equals `firesLeft`, which equals 3. There are 3 `.fire-pip` elements, and the text has no "burning now" and no "/".
    - `tick 2400`, then wait until `burning > 3`. Within 2 frames, `firesLeft === max(goal − extinguished, burning)`, the obj-line number equals it, and the `.fire-pip` count is `min(firesLeft, 8)`.
    - Cast Water at a counted burning fire. Within 2 frames of `impacts` changing, the number drops by exactly 1.
    - On 265a93e the text is "Fires put out: 0 / 3  ·  burning now: 3".
15. **The highlight tells a fire from a bush.** Locked.
    - Use I-007's assist pose (`golden.mjs:576–592`): `tp 4 2`, `b = idx('bush')[5]` (the non-burning bush at (9.6, −10.2)), `face b`, then step-turn (`turn 9 0`, up to 40 steps) until `aimObj === b`. `#crosshair` has `on` and does not have `fire`.
    - `face lit[0]`, then wait for `aimObj === lit[0]`. `#crosshair` has both `on` and `fire`.
    - The computed `box-shadow` of `#crosshair.fire span` is not `none`, and it differs from the value without `fire`.
    - `#hint` reads `Aim at a fire and click.`
    - Then `__game.pause(true)` and point mode, as golden already does. Hovering lit[2] (cursor at the canvas centre after `face`) gives the canvas `hover-fire`. Hovering `b` gives `hover` without `hover-fire`.
    - On 265a93e there is no `fire` class.
16. **Following the arrow finds the fire.**
    - From fresh, `tp 0 5.5`, `face lit[0]`, read `yaw`, then `look (yaw + 180) 0`.
    - Within 5 frames: `fireOff === true`, `nearFire ≥ 0`, `objs[nearFire].burning`. `#firefinder` is visible with the class `right` if `fireBearing > 0`, else `left`, and its box is at least 56 px wide.
    - Then step-turn toward the arrow, up to 40 steps. Each step is `turn 150 0` when the class is `right` and `turn -150 0` when it is `left` (a positive `turn` turns the view right; see the facts). After each step, one `page.evaluate` reads both the state and the DOM. In that same read, the class must equal the sign of that read's `fireBearing` (`right` iff `> 0`). Nothing is compared across reads, so a bearing near ±180 that flips between frames is fine. Stop when `#firefinder` is hidden. It must hide within 40 steps.
    - At that point `fireOff === false` and `|fireBearing| ≤ hfovOf(fov)/2`.
    - `__game.pause(true)` hides `#firefinder` within 2 frames.
    - On 265a93e there is no `#firefinder`.
17. **Pause under automation.** Locked.
    - `page.keyboard.press('Escape')`. Within 10 frames: `#pause` visible, `state.paused`, `!state.locked`, and `document.pointerLockElement === null`. The test does not call `exitPointerLock`.
    - Click `#resume`, wait for `locked`, then `page.keyboard.press('KeyP')`. Same result.
    - `KeyP` again. Within 10 frames, `state.paused === false`.
    - The pause card has a row containing `Esc or P`.
    - On 265a93e the Escape press does not pause. That is why golden falls back.
18. **Nothing under the banner.**
    - Fresh, unlocked (point mode), no casts. Run `tick 2400` until `outcome === 'lost'`.
    - Within 2 frames: `#hint` is hidden or empty, `#firefinder` is hidden, and there is no `.slot.nudge`.
    - Then reset and win with three `cast`s, using golden's `castAt` pattern on the three counted fires. The same holds under the win banner.
    - On 265a93e, "Click to look around…" stays visible under the lose banner.
19. **No flashing from the new effects (GS-6). Guard: it passes on 265a93e and must keep passing.** Use I-007 test 19's in-Node PNG method (`decodePng`, `lumaOf`, `flashFrac`; `golden.mjs:755–788`).
    - Take 20 consecutive-frame screenshots during a Water stream and splash, then 20 during a CO2 stream. The fraction of pixels whose relative luminance changes by 10% or more stays under 20% of the frame for every pair.
20. **Science lint.** `npm test` passes, with the new not-claims rows present and the removed rows gone.
21. **B2/B3 guards (tester, no script change).**
    - Base is the branch base (265a93e, or whatever origin/main the builders branched from), built into `<scratchpad>/I-019/base-pkg-out`.
    - Run `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs`, 3 interleaved pairs (perf.mjs has a machine-load gate since I-023; do not set `PERF_ALLOW_LOAD`). The median CPU frame p50 must be ≤ base + 1.0 ms.
    - Brotli of `pkg-out` grows by at most 30 KB.
22. **Golden still green.** All existing, I-007 and I-023 (`clock and budget`) checks pass 3/3 with `QUERY=?low`, with no threshold edits. Run golden in isolation (I-017: concurrent Chromiums crash).

## GS walk (screens, controls and effects this item touches)

| GS | P | status |
|----|---|--------|
| 6 | P0 | met for new effects; test 19. The finder pulses at 1 Hz or slower and has none under reduced motion |
| 7 | P1 | met: P and Esc both pause everything (existing pause semantics; `gameTime` freezes) |
| 9 | P1 | partial: `KeyP` bound by `code`; no hold is needed to cast any more (a click casts) |
| 10 | P0 | unchanged (I-007 owns assist) |
| 11 | P0 | improved: point mode gets `hover-fire` and Water default; the on-screen pause button is a follow-up |
| 12 | P0 | **met**: test 10 (cast and stream within 2 frames); test 11 (lands within 0.25 s of game time) |
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

- **A per-fire "putting out" progress ring.** The backlog row asked for one, but one landed spell already puts a fire out (chem facts above). The "7–10 s" came from lost clicks, flight time and (at 3dbd332) the slow game clock; I-023 fixed the clock and 1 and 2 fix the rest. A ring would show progress that does not exist.
- **Any change to the chemistry rules**, including the rule that a fire the player lit does not count, GOAL_FIRES and spread pacing. `chem.rs` is not edited.
- **Any change to the game clock.** `clock.rs`, `motion.rs`, the 0.25 s ceiling and the cooldown carry are I-023's and are not edited.
- **The first-30-seconds work** (walk-to-marker, on-screen WASD, age-band captions, explaining spread): that is I-020.
- New sounds beyond I-007's set, frost mist at impact (the Frost beam already shows), FireLight placement, and occlusion-aware finder logic (a fire behind the greenhouse counts as "on screen" when its bearing is in view).
- The browser's own Esc handling in real Chrome: the existing `pointerlockchange` path still covers it.
- `IDLE_HINT` ("…Press 2 for Water…") stays as it is; it is still true with Water selected.
- `index.html` (including its `Fires put out: 0 / 3` placeholder and pause table), `age.js`, `analytics.js`, `boot.js`, `build-web.sh`, `functions/`, `dashboard/`, and the panel harness (`tests/e2e/panel/**`).
- `crates/bevy-client/README.md`: the controls table will be stale (P, Water default). The planner files a one-line docs follow-up.
- **Follow-ups the planner files in the backlog** (cut from this item in revision 2):
  - "CO2 cloud that sinks": `P::Co2`, `CO2_SINK` and its P-016 science row, a CO2 impact cloud, and a time-based sink test on `gameTime` seconds with a margin of at least 2× (GS-32).
  - "On-screen pause button": `#pausebtn` ≥ 44×44 px for point-mode and touch players, propagation stopped, hidden under banners (GS-11, GS-22).
- **Panel brief note:** the counter can read, for example, "Fires to put out: 7" while the win needs 3 (fire spread). This is intended and pinned by test 14. The panel brief must tell personas this so that it is not reported as a bug.

## Contract

A stub landed by the contract builder on the branch base, before the lanes start. It has no gameplay behaviour; both lanes code against it.

- **`cues.rs`** with these signatures and `todo!()`-free placeholder bodies that return neutral values. `mod cues;` goes in `main.rs` next to the other `mod` lines (`main.rs:13–20`).
  - `fires_left(goal: u32, ext: u32, burning: u32) -> u32`
  - `bearing_deg(px: f32, pz: f32, yaw: f32, tx: f32, tz: f32) -> f32`
  - `off_screen(bearing: f32, elev: f32, hfov: f32, vfov: f32) -> bool`
  - `flight_secs(spell: u8, dist: f32) -> f32`, where the spell index is 0..4 in `Spell::ALL` order (no copy of `Spell` in `cues.rs`)
  - `pub enum Look { Normal, Burning, Charred, Wet, Frosty, Foamed, Hot }` (same derives as today: `Clone, Copy, Debug, PartialEq, Eq`), moved from `world.rs:127` and re-exported from `world.rs`
  - `look_emissive(look: Look) -> f32`
- **New `__gameState` fields**, published with neutral values by the stub. They are **appended after `dt`, in exactly this order**, so the base keys and `gameTime`, `dt` keep their positions:

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

- **Golden key-order check (stub makes the change).** In `clockAndBudget`, the stub adds `const I019_KEYS = ['clicks', 'firesLeft', 'nearFire', 'fireBearing', 'fireElev', 'fireOff', 'streams', 'fxWater'];` and changes `keysOk` (`golden.mjs:1041–1042`) to require exactly `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS]` in order. The check's name and its other conditions (objs keys, every frame parses, key list never changes) stay as they are. This is the only edit to an existing check, and the web lane does not redo it.
- **The `fire` command:** a queued cast. It is not debug-gated. The stub makes the real change, and the Rust lane does not redo it:
  - `fire` is added to the filter at `main.rs:735`: `c.starts_with("tap") || c == "cast" || c == "fire"`.
  - The cast branch at `main.rs:942` matches `Some(&"cast") | Some(&"fire")`.
  - `clicks` is incremented for each `fire` in `incoming` (`main.rs:923`), before the paused-queue clear (`main.rs:924–925`).
  - That alias is the latch, because the existing `cast_queue` already gives the 0.3 s wait and the drop while paused, and I-023's carry already applies to every cast in the loop.
- **DOM ids and classes** (web lane only; listed so tests and the panel can rely on them): `#firefinder.left|.right`, `.fire-pip`, `#crosshair.fire`, `canvas.hover-fire`.

## Lanes

| lane | owns | builds |
|------|------|--------|
| rust | `crates/bevy-client/src/**` (`main.rs`, `cues.rs`, `fx.rs`, `world.rs`); `bridge.rs`, `chem.rs`, `clock.rs`, `motion.rs`, `feel.rs` and `player.rs` are not edited | Unit tests 1–5 first. Then: `cues.rs` bodies; `flight_secs` in `aim_and_cast`; one `spawn_stream(material)` for Water, Fire (fire material) and CO2 (existing foam material), its life in game time from `clock::frame_dt` and frozen while paused, despawned on epoch change, and the `streams` count; bigger Water splash and `fxWater`; Water default spell; new startup caption at `main.rs:555`; `firesLeft`; `nearFire`, `fireBearing`, `fireElev` and `fireOff` from the live vfov and the camera aspect; `hot` material made non-emissive through `look_emissive`, `Burning` emissive (1.2) unchanged. No `min(0.05` anywhere. The `fire` routing, `clicks` and the appended state keys come from the contract stub. Must not touch `chem.rs` rules, `web/**` or tests |
| web | `crates/bevy-client/web/hud.js`, `crates/bevy-client/web/style.css`, `tests/e2e/golden.mjs`, `content/science-sources.md` (the rows in 10 only) | `fire` push in the locked `pointerdown` listener at `hud.js:334` (no `isTrusted` gate); `#obj-line` counter and pips; `#firefinder`; `#crosshair.fire`, `canvas.hover-fire` and the new locked hint; Esc-while-locked and `KeyP` handling and the pause-table row; hint priority with banner suppression; reduced-motion rules; science rows; e2e tests 7–12 and 14–19 in `castingYouCanSee`, written against golden at 265a93e (own copies of the `feelAndStandards` helpers). Must not touch `index.html`, `age.js`, `analytics.js`, `boot.js`, the clock section beyond the stub's key list, or the panel harness |

**Seam:** the web lane's tests 8–12 and 16 need the rust lane's fields. Each lane builds against the contract stub. The shared build after both lanes merge runs the full golden, including the `clock and budget` section. The seam breaker checks that `fire`, `clicks`, `streams` and `fxWater` mean the same thing on both sides, that a held press plus a click never casts twice in one frame, that the cooldown carry does not let a click plus a hold cast more often than once per 0.3 s of game time, and that the state key order matches `I019_KEYS`.

## Refresh (wave 7)

Re-checked against origin/main 265a93e (I-007, I-005, I-018, I-023 merged since 3dbd332). Scope unchanged.

- `based_on` moved from 3dbd332 to 265a93e3c53c0046bf2493e52bc85f6d984676fd. The I-007 dependency is satisfied (merged at 855abfc), and its merged contract matches "Relied on from I-007"; that section now cites `hud.js:27`, `IDLE_SLOT = 1`, `updateIdle` and `feel::LOOK_PER_PX`.
- **Line numbers in `main.rs`:** command filter 662 → 735; cast branch 864 → 942; paused-queue clear 847 → 924–925 (with `incoming` at 923); startup caption 507 → 555; held-fire condition now at 958 (with the `casts.is_empty()` guard named); projectile `dur` at 1015; speeds at 999–1004; impact burst at 1102; pool size at 393; `setup` at 376; `reset` at 795; `mod` lines at 13–20. `fx.rs:71` for the 256 cap. `world.rs:127` (`Look`), 137 (`look_of`), 325 (`hot`), 326 (`burning`, 1.2). `hud.js` lines added for the pointerdown listener (334), Escape (385–386), `syncLock` (228–248), `on`/`hover` (577–578), locked hint (584, its condition is `impacts < 3`), obj-line (592–594); `style.css:17`; `index.html:21` and `38–47`; `science-sources.md` rows 79, 83, 85, 89, 94, 95; golden lines 99, 118, 171, 297, 341, 343, 359, 576–592, 755–788, 994, 1041–1042, 1187–1199.
- **Game clock (I-023):** the "spells are slow to land" fact now says dt comes from `clock::frame_dt` (ceiling 0.25 s, virtual clock too), so the game clock keeps wall pace to 4 fps; the panel's numbers are attributed to 3dbd332's 0.05 clamp plus lost clicks and flight time. `gameTime` and `dt` in the state are listed as facts. The cooldown carry (`main.rs:967–977`) is listed and stated to apply to latched casts unchanged; the seam breaker checks it.
- **Timing moved to `gameTime`:** item 2 and the goal say "0.25 s of game time"; test 11 drops the `K = max(10, ceil(0.25 / dtMean) + 2)` frame formula (which used the old 0.05 clamp) for a `gameTime` comparison `g_i − g_c ≤ 0.25 + d` from an in-page sampler; test 9's hold is 1.5 s of `gameTime` instead of 1500 ms wall; the stream's 0.2 s after impact is game time.
- **Stream frozen while paused (new detail, needed by test 12):** `Beam` ticks while paused at main (`main.rs:1042–1050`), so the spec now requires the stream's life to count down only while unpaused, or the pause-freeze pixel check could miss it.
- **New contract item: golden key-order check.** I-023's `clock and budget` check pins `keys.length === BASE_KEYS.length + 2`. The eight new state fields are now appended after `dt` in a fixed order, and the contract stub extends `keysOk` with `I019_KEYS`. This is the one sanctioned edit to an existing check; test 22 and the lanes say so.
- **New rule: `clock::frame_dt` only.** Any new per-frame stream code takes dt from `clock::frame_dt` and has no `min(0.05` (I-023's static check).
- **Fact corrected:** `chem_tick` runs in `FixedUpdate` at 1/30 s (it already did at 3dbd332), so `tick` counts chemistry steps, not frames; no test depended on the old wording.
- **`index.html` reason updated:** I-005 has merged, so it is no longer off limits for that reason; it stays out of scope for this item, now with `age.js` and `analytics.js`. The static `Fires put out: 0 / 3` placeholder is noted as overwritten by `hud.js`.
- **I-005 age picker:** noted that its post-pick reset keeps the spell (so Water stays default) and that it never shows in `pkg-out` (analytics off), so test 7's reload is unaffected.
- **Test 5:** `Burning` is pinned to exactly 1.2 (the value at both 3dbd332 and 265a93e).
- **Test 6:** adds `clock.rs` and `motion.rs` plain-`rustc` tests.
- **Test 15:** the bush is `idx('bush')[5]` with I-007's assist step-turn (`turn 9 0`), not "object 5".
- **e2e helpers:** `castingYouCanSee` takes `feelAndStandards`' argument object and defines its own `fresh`/`tp`/`within`/`unpause`/`aimAt`/`castAt` copies, because those are local at main; `hfovOf`, `decodePng`, `lumaOf`, `flashFrac` are module-level and reused.
- **`fireBearing`/`off_screen` inputs:** the bounds centre is `chem.bounds(i).center()`; hfov is derived from `settings.fov_y()` and the camera aspect `aim_and_cast` already computes.
- **Test 21:** base is the branch base (265a93e), not the I-007 merge commit; perf.mjs's load gate is noted.
- **Test 22 and the seam:** include I-023's `clock and budget` checks.
- **"On 3dbd332" failure notes** in tests 4, 7–18 now say "On 265a93e"; each still fails there for the same reason (test 14's text quoted with its double space).
- **Bars:** the B4 paired-run base is now the 265a93e artifact; blocker and clarity numbers are stated relative to that base.
- **Out of scope:** adds `clock.rs`/`motion.rs`/the clock ceiling/carry, `IDLE_HINT` and `age.js`/`analytics.js`; the rust lane also does not edit `feel.rs` or `player.rs`.
- I-018 (electrolysis) touches no code this spec depends on.

Amended by I-025: hint priority is banner > idle > note > locked > start (item 9 is superseded); a held press repeats after 0.5 s of game time, not 0.3 s (item 1).
