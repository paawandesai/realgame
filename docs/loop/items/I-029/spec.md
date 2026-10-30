# I-029: Worked casts, then instant casts

**based_on:** origin/main c4832d2e86c2408443124960b130fc6208a89aab (2026-10-30, merge of PR #31, roadmap v2; code as at a0e2ecb plus I-019). Code was read with `git show c4832d2:<path>`. The primary checkout (661b9ba) has no `crates/bevy-client/src`, so builders branch from the real origin/main.

**Revision 2** (answers `critique.md` revision 1; the planner's response is at the end of that file). Phase 2, third item of the core loop (`target.md`, "Core loop": "Before mastery every cast is a worked cast (rebuild the formula in a quick overlay, about 3-10 s) ... Then the spell is an instant cast"). Backlog row I-029: bars B9, B4, B11; after I-027 and I-028.

**Depends on I-027 (spell progress) and I-028 (Formula Bench).** Both are specced at revision 1 (`docs/loop/items/I-027/spec.md`, `docs/loop/items/I-028/spec.md`). This spec uses I-028's names exactly ("Assumed from I-028"). It needs four small additions to `bench.js` that I-028 revision 1 does not list; they are requested of I-028's next revision, and if I-028 merges without them, this item's contract step adds them in one named commit (see "Requested of I-028"). **This item does not start building until both have merged.** Its contract step rebases on that origin/main and re-reads every line number below.

**Runs after I-022, I-024 and I-025** in merge order. It touches none of their hunks; "Overlap and merge order" lists each one.

## Facts from c4832d2 this spec relies on

`main.rs` means `crates/bevy-client/src/main.rs`, `hud.js` means `crates/bevy-client/web/hud.js`, `golden` means `tests/e2e/golden.mjs`.

- **One cast path.** Every cast comes through `aim_and_cast` (`main.rs:948`). Its sources are:
  - a locked left click: `hud.js:350-353` pushes `fire` and sets `input.fire = true`;
  - the held button (`firing`, `main.rs:1076`);
  - the `cast` command (tests, and the lock-refused click at `hud.js:392`);
  - `tap x y` (touch and point mode, `hud.js:369` and `387`).
  - `control` keeps only `tap*`, `cast` and `fire` for `aim_and_cast` (`main.rs:816-819`). `aim_and_cast` counts `fire` into `clicks` (`main.rs:1039`). It clears the queue while paused (`main.rs:1040-1041`) and pops one queued command when `cooldown <= 0` (`main.rs:1051-1056`). The held path runs only when nothing was queued (`main.rs:1076`). The cast loop starts at `main.rs:1079-1080`.
  - So one gate placed just before the cast loop covers every source.
- **Pause freezes the world.** On wasm, `control` sets `game.paused = web.paused` every frame (`main.rs:689`), before any other use. While paused the game clock stops (`main.rs:711`, I-023), movement stops (`main.rs:843`), `chem_tick` skips (`main.rs:655`), projectiles and streams freeze (`main.rs:1196`, `1208`), fx slow to 0 (`main.rs:1373`), queued casts drop and the held path is off. Native builds read pause from no one, so `paused` is always false there.
- **Spell selection** comes from four places: keys 1-4 read by Bevy (`main.rs:715-727`), the native wheel (`main.rs:730-733`), the `select N` command (`main.rs:760`, sent by the hotbar, `hud.js:434`), and the web wheel (`hud.js:355-360`).
- **Locked mouse input** is read only in `hud.js`: `onMove` (`hud.js:297`, window capture) adds locked deltas to `input.dx/dy`; the locked `pointerdown` listener (`hud.js:350`) handles clicks; `overUi` (`hud.js:317`) excludes `#hotbar, #pause` from hover; `syncLock` (`hud.js:242-260`) shows the pause card when an earned lock is lost; Esc under lock calls `pauseNow` (`hud.js:413-417`).
- **`reset`** (`main.rs:878-893`) rebuilds `Game` from `default()`, keeping `spell`, `locked`, `paused` and `epoch`.
- **System parameter counts.** `aim_and_cast` has 13 parameters. I-027 takes `control` to 14 (15 on native); Bevy's limit is 16. **This item adds no parameter to `control`.**
- **`settings.debug`** is `param("debug")` (`main.rs:411`). Golden (`golden:62`), perf (`perf.mjs:105`) and analytics-e2e (`:172`) always add `debug`. The persona panel never does (`panel/daemon.mjs:383`).
- **Hotbar.** Static HTML (`index.html:27-31`, four `.slot` buttons).
- **The build copies an explicit file list.** `build-web.sh:21`: `cp web/index.html web/style.css web/boot.js web/hud.js web/favicon.svg "$OUT/"`. A new `web/*.js` that is not on that line 404s in `pkg-out`, and a static `import` of it breaks the HUD.

## Assumed from I-027 (specced, revision 1)

Used exactly as specified:
- `Progress(progress::Book)`, a `Resource`; index `i` = slot − 1.
- `Book::get(i) -> Option<&SpellProgress>`, whose `stage` is `Stage { Unknown, Learning, Practised, Mastered }`, ordered, with `as_str()`.
- `Book::worked_cast(i, correct)`: 0 on Unknown, +10 XP when correct on Learning, Practised or Mastered, 0 when not correct; Learning → Practised at 30 XP.
- `Book::instant_cast(i)`: +2, on Mastered only.
- The debug `prog …` commands, used by tests to set stages.
- The `progress`, `progRev` and `progSave` state keys.

This item calls only those `Book` methods and never writes fields.

## Assumed from I-028 (specced, revision 1)

| I-029 uses | I-028 name (exact) |
|---|---|
| the judge | `realgame_sim::bench::judge(spells: &[SpellDef], slot: u8, band: Tier, text: &str) -> Verdict`, with `Verdict { ok, code, diffs, other, bad, simplest }`; `slot` = spell index + 1; the spells slice is `spells::book().all()`, as I-028's `bench::run` passes it |
| "learnable at this band" | `realgame_sim::bench::learnable(def, band)` (`band >= def.tier_min`, the same rule `judge` uses for `NotForBand`). I-029 never reads `tier_min` itself |
| the recipe and prompt | `bench::recipe(def)`, `bench::prompt(def, band)` |
| the JSON shape the builder reads | `BenchView` / `SpellCard` / `ResultView`, serialized with `BenchView::to_json()` |
| the builder | `bench.js` `mountBuilder(root, { band, spell, recipe, prompt, onMake })`; `onMake(text)` receives the composed formula text (`composeSprout`, `composeSeedling` or `composeEquation`), which is what `judge` takes |
| the reason text | `bench.js` `reasonFor(band, result, names, tries)`, given the whole verdict |
| the band | `bench.js` `bandFrom(search, picked, chosen)`; `adept` reads as `apprentice`; the band is kept for the page load only and never stored |
| the chooser | I-028's three-picture chooser ("How old are you?", buttons at least 160 × 160 px, `.bench-band[data-band]`) |
| the keyboard model of a build | Tab/Shift+Tab and arrows move focus, Enter/Space press, arrows step a Seedling tile, typing goes to `#bench-input`; no `preventDefault` on printable keys |
| DOM classes | `.bench-atom`, `.bench-tile`, `.bench-coef`, `.bench-make`, `.bench-undo`, `#bench-input`, `.bench-reason` |
| the bench's own key capture | `startBench()` registers window capture `keydown`/`keyup` listeners after `startHud()`; while closed they look only for `KeyB` |
| the bench state | `window.__benchState` (`open` is true while the bench is open) |

I-029 does not use I-028's `open/pick/try/close` queue: a worked cast is not a bench session, and must not call `learn`.

### Requested of I-028 (four additions to `bench.js`, each small)

1. **`currentBand()`** returns the band `bandFrom(location.search, picked, chosen)` resolves now, or `null`. **`chooseBand(b)`** sets the same page-load value the bench chooser sets (so a child asked once by either surface is not asked again, and "Change age" in the bench reaches the field).
2. **`mountChooser(root, onPick)`**: the chooser view on its own (same pictures, labels and sizes), calling `chooseBand(b)` then `onPick(b)`.
3. **`mountBuilder` option `guided: boolean`** (default false). When true, the build shows the answer to copy: Sprout and Seedling mark the next correct piece with the class `bench-next` (an outline, plus a 1 Hz pulse unless reduced motion); Apprentice shows the target text above the input as "Copy: H₂O(l)". The builder still judges nothing.
4. **`mountBuilder` returns `{ destroy() }`.**

If I-028 merges without them, this item's contract lane adds them to `bench.js` in one commit titled "bench.js: builder options for I-029 (currentBand, chooseBand, mountChooser, guided)", with node tests in `test/bench.test.ts` for each, reviewed against I-028's golden section (I-028's tests 16-20 must still pass). That is the only edit to `bench.js` this item may make.

## Problem

`target.md` makes the core loop "craft and master real-chemistry spells". After I-027 and I-028, a player can learn a spell at the Bench and the game keeps per-spell progress. But in the field every spell still fires on one click, exactly as at c4832d2. Nothing in play asks for the formula again. Worked casts, which the loop says earn XP before mastery, do not exist, so Learning can never become Practised in play. Mastery buys nothing, because every cast is already instant. B9 ("uses an instant cast within 20 minutes") cannot even be observed.

The risk is lag. A shooter where every click opens a box feels slow. Bad versions of this feature would grab the mouse, drop pointer lock (the browser's "Press Esc" banner on every cast), let fires spread while a 5-year-old builds, or move the camera after the box closes.

**Goal:** before mastery, a click opens a small builder below the crosshair within 2 frames. The world holds while it is open. A correct rebuild at the player's band fires the spell at the crosshair within 2 frames and earns XP. A mastered spell fires on one click, as today. Golden measures the time to cast per band.

## Bars moved

- **B9 (mastery works):** enabling, and the first visible part of it.
  - Worked casts earn XP in play, so Learning → Practised happens through casting.
  - "Uses an instant cast" becomes observable: the hotbar star and `routes[i] === 'instant'`.
  - The Practised toast and the Practised build header point to the next step (the bench), so a player on the cast path alone can find mastery.
  - B9 is a panel bar and also needs I-030's trial, so nothing is reported as moved at merge.
- **B4 (every age can play):** must hold. Predictions for the first paired panel run (same seed, merge base vs this artifact, no `debug`):
  - every persona still lands a cast within 30 s. The 6-year-old's first cast is: one picture click in the chooser (when no band is known), then a guided Sprout build of Water, which is **4 inputs** (H, H, O, Make);
  - no persona reports "stuck in a box", "the view jumped" or "the mouse stopped working";
  - median clarity drops by at most 1 against the paired base.
  - If clarity drops by more than 1, the critic decides whether to keep the change or revisit the routes. The loop does not tune the routes on its own.
- **B11 (more than a shooter):** enabling. Every cast before mastery now uses the chemistry (GS-30). B11 is scored per level from I-034 on.
- **B1:** stays green. Existing checks run in instant mode (Contract, "Cast mode") and are not edited, except the key-order list.
- **B2, B3:** no regression (G16). The overlay costs nothing per frame while it is closed. `worked.js` is loaded with the HUD and is under 12 KB unminified.
- **B6:** stays at 0 errors. Formulas and reasons shown come from `spells.json` and `bench.js` (I-026 and I-028 rows). This item adds Not-claims rows for its own chrome text (Smallest version 5).
- **B7:** unchanged. No new analytics event; worked casts are not sent.

## Smallest version

1. **Routes (Rust, pure: `crates/bevy-client/src/worked.rs`, new, no Bevy types, no `crate::` imports).** Each spell's route is decided from its stage, whether it is learnable at the band (unknown when no band is known yet) and the cast mode. First match wins:

   | mode | stage (I-027) | learnable at band? | route | what a click does | XP |
   |---|---|---|---|---|---|
   | `Instant` | any | any | `instant` | casts at once (today's behaviour) | none |
   | `Worked` | Mastered | any | `instant` | casts at once; `instant_cast(i)` | +2 |
   | `Worked` | Unknown, Learning, Practised | band not known yet | `choose` | opens the overlay with I-028's band chooser; after the pick it continues as the matching row below (or casts, for a non-learnable spell) | per that row |
   | `Worked` | Unknown, Learning, Practised | no | `instant` | casts at once: the spell is above this band and there is nothing to build at it (I-028 `judge` returns `NotForBand` for every text). Today's behaviour | none |
   | `Worked` | Learning, Practised | yes | `worked` | opens the builder | +10 (`worked_cast(i, true)`) on a first-try correct build |
   | `Worked` | Unknown | yes | `guided` | opens the builder with `guided: true` (copy the answer); never blocks play | 0 (`worked_cast` is a no-op on Unknown) |

   - **The builder is I-028's `mountBuilder`**, unchanged apart from the requested `guided` option. The pieces are I-028's: a Sprout Water build is H, H, O, Make (4 inputs); a Seedling single-substance build taps count tiles, then Make; Seedling Fire sets coefficients on the scale, then Make; Apprentice types the formula, then Enter or Make.
   - **Help after two misses.** After `HELP_AFTER = 2` wrong submits in one worked cast, the overlay remounts the builder with `guided: true` (the current pieces are cleared; the answer is shown to copy). A correct submit still casts. A helped build calls `worked_cast(i, false)` (0 XP). A worked cast never ends without a cast unless the player cancels.
   - **A wrong submit fizzles kindly.** No cast, no cooldown. The overlay shows `reasonFor(band, result, names, tries)` with a ✗ icon, the text and the existing `miss` sound. The builder stays mounted so the player can fix the build. `worked_cast(i, false)` is called (0 XP, nothing lost).
   - Worked casts do **not** call `unaided_build` and do not count toward I-030's trial. The field overlay never offers "Show what you know"; the trial lives in the bench (I-030).

2. **The band.** The band is I-028's page-load band, never stored.
   - `worked.js` reads `currentBand()` every frame and, whenever it differs from the last band it sent (including the first non-null value), pushes `work band <sprout|seedling|apprentice>`. Rust keeps it in `Working.band: Option<Tier>` (initially `None`; not reset by R; gone on reload, as I-028 requires).
   - **Band unknown at the first click** (the `pkg-out` build, a shared classroom link without `?band=`, analytics off): the click opens the overlay with route `choose`. The overlay shows `mountChooser` (three 160 px pictures, one click). The pick calls `chooseBand(b)` (so the bench will not ask again this page load) and sends `work band <b>`. Rust then re-routes the open worked cast: `worked` or `guided` keeps it open and the overlay mounts the builder; `instant` (Fire at Sprout) closes it and casts at the stored target with no XP. There is no default band: a 13-year-old is never shown picture cards without being asked (GS-27).
   - The band word `adept` never reaches Rust from JS (I-028 maps it to `apprentice`). `work band adept` is accepted and stored as `Tier::Adept`, which I-028's `judge` treats as Apprentice.

3. **Rust wiring (`main.rs`, small hunks only).**
   - **Desk in `Game`.** `Game` gains `work: worked::Desk`. `reset` rebuilds `Game`, so a reset closes an open builder with no extra code.
   - **Cast mode and band.** A new resource `Working { mode, band, next_id, done, wrong }` (Contract) is inserted in `main()` with `mode` from `worked::mode_from(settings.debug, param("cast").as_deref())`. `Settings` is **not** changed (I-022 owns it).
   - **`control` (no new parameter).**
     - `main.rs:689` becomes `game.paused = web.paused || game.work.is_open();`. This one line makes every existing pause path hold the world while the builder is open.
     - The digit loop (`715-727`), the native wheel (`730-733`) and the `select` arm (`760`) are skipped while `game.work.is_open()`.
     - The filter at `816-819` also keeps commands that start with `work ` and `castmode `.
     - `castmode` is added to the `debug_only` list (`751-754`). It is handled in `aim_and_cast`, which has `Working`.
   - **`aim_and_cast`** takes one new parameter: `#[derive(SystemParam)] struct WorkCtx { working: ResMut<Working>, prog: ResMut<Progress> }`, which counts as one parameter (13 → 14). Spell definitions come from `spells::book().all()` (no resource).
     - **Before the paused-queue clear (`1040`)** it takes every `work …` and `castmode …` command out of `incoming`, in order.
     - `work band <b>`: parses the band (an unknown word is ignored with `warn!`) and sets `working.band`. If a worked cast is open with route `choose`, it computes the new route and calls `desk.reroute(route)`. On `Some(Cast { … })` (route `instant`), it casts as below with no XP.
     - `work submit <id> <text…>`: the text is the rest of the line, words joined by single spaces.
       - A stale or unknown id, or an open route of `choose`, does nothing.
       - Otherwise `v = bench::judge(spells, spell + 1, band, text)`, then `desk.submit(id, v.ok)`.
       - On `Cast { target, earns }`: `prog.worked_cast(spell, earns)` (skipped for `guided`); `working.done += 1`; `game.paused = web.paused` for this frame; push the target's aim to the front of `casts`. Crosshair targets use the current `game.aim`; tap targets use `pick` on `ray_at(tap)` with `tap_assist`. This cast skips the gate and the cooldown check, and still sets `cooldown`.
       - On `Wrong { .. }`: `prog.worked_cast(spell, false)`; `working.wrong += 1`; the view is rebuilt with `result` = `ResultView { n: wrong count, slot, verdict: v, learned: false, stage }` and stored with `desk.set_view(json)`.
     - `work cancel <id>`: closes the builder with no cast and no XP.
     - `castmode worked|instant`: sets `working.mode` (debug only, by the list above).
     - **The view.** On open (and on re-route or band change while open) the glue builds a `BenchView { v: 1, at_bench: false, open: true, band: Some(band), spells: Some(vec![the one SpellCard for this spell, with prompt from bench::prompt]), pick: Some(slot), recipe: Some(bench::recipe(def)) for Sprout and Seedling else None, result }` and stores `view.to_json()` with `desk.set_view`. With route `choose`, the view is not set (`null`).
     - **The gate** goes just before the cast loop (`1079`), after I-025's held-path block. For each pending cast that is not an accepted worked cast, it reads `route = worked::route(stage, learnable, mode)`, where `learnable = working.band.map(|b| bench::learnable(def, b))`.
       - `Instant`: cast as today; a Mastered spell also gets `prog.instant_cast(spell)`.
       - Otherwise: `game.work.request(working.next_id, spell, route, target)` (bump `next_id` when it opens), clear `cast_queue`, and do not cast. The target is `Tap(x, y)` for a `tap`, or `Crosshair`.
       - A request while a builder is already open is ignored. One click opens at most one builder.
     - The held path cannot reopen the builder after it closes: the HUD clears `input.fire` on open and on close (5 below).
   - **`publish`** takes `working: Res<Working>` and reuses I-027's `prog`. It appends the keys in Contract after the last key at the branch base.
   - **Native** builds always use `CastMode::Instant` (no overlay): `mode_from` returns `Instant` when `cfg!(not(target_arch = "wasm32"))`.

4. **What never changes.** Aiming (crosshair, assist cone, `pick`; a worked cast lands where an instant one would have). Movement, outside the builder. The cooldown, streams and flight times (I-019, I-025). The pointer-lock rules: the overlay never requests or releases the lock. The game-clock rules (I-023): `clock.rs` and `motion.rs` are not touched; the hold reuses `paused`.

5. **The overlay (web: `crates/bevy-client/web/worked.js`, new).**
   - `createWorked({ stage, cmd, sound, bench })` returns `{ update(state), isOpen(), move(dx, dy), press(button), key(e), cancel() }`. `bench` is `{ currentBand, chooseBand, mountChooser, mountBuilder, reasonFor }` imported from `./bench.js` by `hud.js` and passed in (so node tests can pass fakes).
   - **Open and close come only from state.** When `state.work` has a new `id`, it creates `#worked`, a panel inside `#stage`:
     - route `choose`: `bench.mountChooser(panel, (b) => cmd(\`work band ${b}\`))`;
     - route `worked` or `guided`: `bench.mountBuilder(panel, { band, spell, recipe, prompt, guided: route === 'guided' || work.help, onMake: (t) => cmd(\`work submit ${id} ${t}\`) })`, with `band`, `recipe` and `prompt` read from `work.view`.
     - When the same id changes route (`choose` → `worked`/`guided`) or `help` turns true, it destroys the mounted view and mounts the builder (guided when helped).
     - When `work.view.result` changes (`n` up) and `ok` is false, it shows `bench.reasonFor(band, result, names, result.n)` in `#worked .reason` with a ✗ icon and plays `miss`.
     - When `state.work` becomes `null` (accepted, cancelled, re-routed to a cast or reset), it destroys the panel.
     - On open and close it sets `input.fire = false`, through the HUD hook below.
     - **Exclusion with the bench.** If `window.__benchState.open` is true while the overlay is open (for example `#benchbtn` clicked in point mode), the overlay sends `work cancel <id>`. While the bench is open the game is paused, so a click is dropped by the existing paused rule and no builder opens.
   - **Layout.** The panel is centred horizontally, at most 560 px wide, its top at least 24 px below the crosshair and its height at most `calc(50vh - 24px)`, so the crosshair stays visible above it. Styles scoped to `#worked` set I-028's pieces to their GS-22 minimums inside the panel: Sprout atom cards, Undo and Make at least 64 × 64 px; Seedling tiles, coefficient buttons and Make at least 44 × 44 px; text at least 18 px. `bench.js` is not edited for this.
   - **Under pointer lock (the usual case).** Locked mouse deltas move a **virtual cursor**, a 24 px arrow drawn in the panel that starts at the panel centre and is clamped to it. They no longer turn the camera. A locked left press clicks the element under the virtual cursor (`document.elementFromPoint`, then `.click()`); a locked right press cancels.
   - **Without lock** (point mode, lock refused, touch), the panel is a normal DOM panel clicked with the real pointer or a finger.
   - **Keyboard (GS-13).** One window capture-phase `keydown` listener, registered in `startHud()` and therefore before `startBench()`'s listeners. While the overlay is open, for every `keydown` except `Escape` it calls `stopImmediatePropagation()` (so `hud.js`, winit and the bench's `KeyB` handler never see it). It calls `preventDefault()` only on `Tab`/`Shift+Tab` (to keep focus inside `#worked`), never on printable keys, so typing reaches `#bench-input`. Every other key does what I-028's build view does with focus: arrows and Enter/Space on pieces, typing in the input, Enter submits. There are no per-piece shortcut keys. On open, focus moves to the first piece (or `#bench-input` in typed bands, or the first chooser button).
     - Esc behaves as today: under lock the browser drops the lock and the game pauses; unlocked, it toggles pause. Either way the HUD cancels the builder.
     - **`keyup` is never stopped**, so a key held when the panel opened is released in Bevy and the player does not keep walking afterwards.
     - While closed, nothing is stopped.
   - **Feedback.**
     - Header: the spell's hotbar icon, its name and a stage word: "New" (guided), "Learning", or for Practised "You know this one". A Practised build's header carries a second line: "Show what you know at the Formula Bench (B) to cast it instantly." Route `choose` shows I-028's heading only.
     - A wrong submit shows ✗ plus the reason, plays `miss`, and shakes nothing (GS-3).
     - An accepted build closes the panel. The existing cast pulse and sound play when `casts` goes up. There is no separate XP pop.
     - On a stage change to Practised, `#worked-toast` shows for 4 s with icon and text: "Water: Practised! Show what you know at the Formula Bench (B) to cast it instantly." Guided builds show the line "New spell: copy the glowing pieces. Learn it at the Formula Bench (B) to earn XP."
   - **Hotbar stage badges.** `worked.js` appends one `span.stage` to each `.slot` (`index.html` is not edited): a shape icon of at least 20 px (○ Unknown, ◔ Learning, ◑ Practised, ★ Mastered) with `aria-label` and `title` set to the stage word. The shape carries the meaning, not colour alone (GS-19).
   - **Hooks in `hud.js`** (small, listed in Lanes):
     1. Import `createWorked` from `./worked.js` and the five bench functions from `./bench.js`; create `worked` in `startHud()` after `cmd` (`hud.js:160`).
     2. In `onMove` (`hud.js:297`): `if (input.locked && worked.isOpen()) { worked.move(dx, dy); return; }` before the `input.dx` add.
     3. In the locked `pointerdown` listener (`hud.js:350`): `if (worked.isOpen()) { worked.press(e.button); return; }` first.
     4. In the wheel listener (`hud.js:355`): return while `worked.isOpen()`.
     5. One window capture-phase `keydown` listener that calls `worked.key(e)`.
     6. `overUi` (`hud.js:317`) adds `#worked`.
     7. In `showPause(true)` (`hud.js:215`): `worked.cancel()`, which sends `work cancel <id>`.
     8. In `frame()` (`hud.js:616`): one call, `worked.update(state)`, after `state` is parsed, outside I-019's, I-022's and I-025's hint lines.
   - **Packaging.** `build-web.sh:21` gains `web/worked.js` on the `cp` line (I-028 adds `web/bench.js`, I-027 `web/progress.js`).
   - **No new storage key**, and nothing sent over the network.

6. **Science register.** Sentence-shaped chrome text from `worked.js` goes into `content/science-sources.md` "Not claims", with the reason "UI instruction; states no science": the toast, the Practised header line, the guided line and the header words if the lint flags them. Formulas, prompts and reasons come from `spells.json` and `bench.js` (I-026 and I-028 rows). This item adds no claim row.

## Acceptance tests

**Rust unit tests** run with `rustc --edition 2021 --test crates/bevy-client/src/worked.rs -o <scratch>/worked && <scratch>/worked`. They fail today because the file does not exist, and on the contract stub, whose `route` always returns `Instant` and whose `request` never opens.

1. **Route table.**
   - `route(stage, learnable, Worked)` for stages 0..=3 × `learnable` in {`None`, `Some(false)`, `Some(true)`} gives exactly the table in Smallest version 1 (12 cases). `Instant` mode gives `Instant` for all 12. `as_str` gives `instant`, `worked`, `guided` and `choose`.
   - `mode_from(true, None)` is `Instant`; `mode_from(true, Some("worked"))` is `Worked` on wasm; `mode_from(false, None)` is `Worked` on wasm; `mode_from(false, Some("instant"))` is `Worked` (`cast=instant` is ignored without `debug`, so a shared link cannot skip the loop). Under native `cfg`, every combination is `Instant` (the test branches on `cfg!(target_arch = "wasm32")`).
2. **Open and close.**
   - A fresh `Desk` is closed. `request(1, 1, Worked, Crosshair)` opens with `id` 1, `spell` 1, `wrong` 0, `help` false. A second `request(2, …)` while open returns false and leaves the id at 1.
   - `request(_, _, Instant, _)` never opens. `spell >= 4` never opens.
   - `cancel(1)` closes; `cancel(7)` on a closed desk does nothing. After it, `request(5, …)` opens with `id` 5.
3. **Submit.**
   - `submit(id, false)` gives `Wrong { help: false }`, then `Wrong { help: true }` (`HELP_AFTER = 2`); the builder stays open. `submit(id, true)` then gives `Cast { spell, target, earns: false }` (helped) and closes.
   - In a new worked cast, a first-try correct submit gives `earns: true` for `Worked` and `earns: false` for `Guided`.
   - `submit(stale_id, true)` gives `Stale` and changes nothing. `submit(id, true)` on an open `Choose` gives `Stale`.
   - A `Tap(0.25, 0.75)` target comes back unchanged in `Cast`.
4. **Re-route.** An open `Choose` desk: `reroute(Worked)` returns `None`, stays open with route `worked` and the same id; `reroute(Instant)` returns `Some(Cast { earns: false, target: <the stored target> })` and closes. `reroute` on a non-`Choose` open desk or a closed desk returns `None` and changes nothing.
5. **`state_json`.**
   - A closed desk gives `null`.
   - An open one gives exactly `{"id":1,"spell":1,"route":"worked","wrong":0,"help":false,"view":null}` before `set_view`, and parses as JSON. After `set_view(r#"{"v":1,"open":true}"#)`, `view` is that text verbatim. `set_view` on a closed desk does nothing; a new `request` clears the view.
6. **No panic.** A deterministic LCG drives 50 000 random ops (`request`, `submit`, `cancel`, `reroute`, `set_view`, any ids, spells 0..6, all routes) over 20 seeds. No op panics. `is_open()` is true exactly when the last state change was a successful `request` not yet followed by a `Cast` or `cancel`.
7. The existing plain-`rustc` tests still pass: `chem.rs`, `cues.rs`, `feel.rs`, `clock.rs`, `motion.rs` and `progress.rs` (I-027).

**Node unit tests** go in `test/bevy-worked.test.ts` and import `web/worked.js` with a fake `stage`, `cmd` and `bench` (whose `mountBuilder` and `mountChooser` record their options and render a few buttons, including a fake `input`). They run with `npm test` and fail today because the module does not exist.

8. **Open and close follow state.**
   - `update({work: {id: 3, route: 'worked', view: {...}}})` mounts the builder once, with `band`, `spell`, `recipe` and `prompt` from `view` and `guided: false`. The same id again mounts nothing new. `route: 'guided'` passes `guided: true`; `help: true` on the same id remounts with `guided: true`.
   - `route: 'choose'` mounts the chooser; its pick pushes `work band seedling`; the same id with `route: 'worked'` then destroys the chooser and mounts the builder.
   - `update({work: null})` destroys the panel. `update({work: {id: 4}})` after id 3 remounts.
   - `onMake('H2O(l)')` pushes exactly `work submit 3 H2O(l)`. `cancel()` pushes `work cancel 3`, and nothing when closed.
   - A `view.result` with `n: 1, ok: false` calls `reasonFor(band, result, names, 1)` and puts its text in `.reason` with a ✗; the same `n` again does not replay the sound.
   - With `window.__benchState = { open: true }` while open, `update` pushes `work cancel <id>`.
   - The fake `currentBand()` going `null` → `'sprout'` pushes `work band sprout` once; unchanged frames push nothing.
9. **Virtual cursor.** It starts at the panel centre; `move(10000, -10000)` clamps it to the top-right corner; `press(0)` clicks the fake element at the cursor; `press(2)` sends `work cancel`; `move` while closed returns false.
10. **Keys.**
    - While open, a `keydown` for `Digit2`, `KeyW`, `KeyB` and `Enter` gets `stopImmediatePropagation` and **not** `preventDefault`. `Tab` gets both. `Escape` gets neither. A `keyup` for `KeyW` gets neither.
    - While open with the fake `input` focused, dispatching `keydown` + the native default for `2` and `w` leaves the input's value ending `2w` (nothing prevented the default).
    - While closed, nothing is stopped.
11. **Badges.** Given 4 fake `.slot`s and stages `unknown, learning, practised, mastered`, each slot has exactly one `span.stage` with icons `○ ◔ ◑ ★` and `aria-label` set. A second `update` adds no duplicates.
12. **Packaging.** Every `./*.js` module statically imported by `web/hud.js` or `web/boot.js` (read as text) appears on `build-web.sh`'s `cp` line. This fails if `worked.js` (or `bench.js`) is imported but not copied.

**Golden e2e checks** go in a new function `workedCasts({ page, st, cmd, until, frames, sleep, idx, lit, locked, base, browser, errors })` at the end of `golden`, called on its own line after I-028's `formulaBench(...)` call and before `no console errors`, so `no console errors` covers it. Run as `QUERY=?low node tests/e2e/golden.mjs`, on its own (I-017). They fail today because `castMode`, `routes`, `work`, `workDone` and `workWrong` do not exist. On the stub they fail from G3 on.

- **Frame-exact timing.** The section installs an in-page recorder. On every `requestAnimationFrame` it appends `{ t: performance.now(), frame, casts, workId, open: !!document.querySelector('#worked:not([hidden])') }` from `window.__gameState` and the DOM to `window.__wc`. "Within N frames" means N increments of `state.frame` after the event's `performance.now()` stamp. `until` (150 ms polling) is used only for waits, never for timing.
- **Setup.** The section starts with `fresh()`, then loads `base + '&band=seedling'`, sends `castmode worked`, removes `rg.progress.v1`, reloads, sends `castmode worked` again and relocks. It ends by sending `castmode instant`, removing `rg.progress.v1` and loading `base`, so later sections start as they always have. Stages are set only with I-027's `prog` debug commands. If the pointer cannot be locked, every lock test **fails**; none is skipped. Each test from G3 on starts with `prog clear`, `reset`, `select 2` (Water) and a relock, except where it says otherwise.
- **Building in tests** follows the recipe in `work.view`, through the UI only: Sprout clicks `.bench-atom[data-el]` once per atom in the recipe, then `.bench-make`; Seedling taps each `.bench-tile[data-el]` up to its recipe count (or `.bench-coef[data-term]` to the recipe coefficients for Fire), then `.bench-make`; Apprentice types the spell's `formula.text`, read by the test from `content/spells.json` (so a drift in I-026's data fails the test), then presses Enter. Under lock, a "click" is: move the mouse so the virtual cursor reaches the element's centre, then a left press. Wrong builds: Seedling one H tile short; Apprentice `H2O2`.

G1. **Default mode.** In a new context with the base URL minus `debug` (as in `golden:949`), `castMode === 'worked'`. On the main page (`debug`), the first state had `castMode === 'instant'`, recorded before setup. With `?low&debug&cast=worked`, `castMode === 'worked'`. In the no-`debug` context, `cmd('castmode instant')` leaves it `worked`, and a warning (not an error) is logged.
G2. **Instant mode is today's game.** On the main page before setup, with fresh progress, `select 2` then `cast` gives `casts` +1 within 2 frames, `work === null` on every recorded frame, and `routes` all `instant`.
G3. **The gate.** With fresh progress (Water Unknown) and Water selected, a locked click (`mouse.down`, 60 ms, `mouse.up`) gives `casts` +0 for 30 frames. `work` opens within 2 frames of the press with `route: 'guided'`; `#worked` is visible within 2 frames; `state.paused === true` while it is open; `clicks` +1; at least one `.bench-next` piece. `cast`, `fire` and a held button while open give `casts` +0 and the same `work.id`.
G4. **The world holds, and the view does not move.** While open: `gameTime` and every `objs[].t` are unchanged over 1 s of wall time; `keyboard.down('KeyW')` for 500 ms changes `pos` by 0; 20 `mouse.move` steps of 40 px under lock leave `yaw` and `pitch` unchanged while the virtual cursor's `style.transform` changes; `document.pointerLockElement` is the canvas on every recorded frame (including after focus moved into the panel).
G5. **No stuck keys.** Hold `KeyW`, click to open the builder (Water Learning), release `KeyW` while it is open, cancel with a right press. In the following 500 ms `pos` moves less than 0.01 m.
G6. **Wrong builds fizzle kindly.** With Water Learning (`prog learn 1`), open the builder and make a wrong build. Expect `casts` +0, `workWrong` +1, `work.view.result.ok === false`, `#worked .reason` visible with the exact text `reasonFor` gives for that verdict (Seedling `atoms_off`: "Count again: H should be 2. You have 1.") and a ✗ icon, and Water `xp` unchanged. A second wrong build sets `work.help === true` and at least one `.bench-next` piece. A correct build then gives `casts` +1 and Water `xp` still unchanged (helped).
G7. **A correct build fires at the crosshair and earns XP.** With Water Learning, `face` a burning target `t` and note `aimObj`. Open the builder and build the correct answer. Within 2 frames of the Make press: `casts` +1, `work === null`, `paused === false`, `#worked` hidden. The impact lands on `t` (`hitKind` `out` for a burning bush, as I-007 defines it). Water `xp` +10 and `workDone` +1. **Fail-first:** with `castmode instant`, the click itself casts, so G3's "casts +0 for 30 frames" fails; on the stub, `work` never opens.
G8. **Practised, and the pointer to mastery.** With Water Learning, two correct worked casts make Water `practised` (10 + 10 + 10 = 30 XP). `#worked-toast` appears and contains "Formula Bench (B)". `routes[1] === 'worked'`. The next click's builder header contains "Formula Bench (B)" and the word "You know this one".
G9. **Mastered is instant.** Drive Water to `mastered` with I-027's test-19 command path. A locked click gives `casts` +1 within 2 frames; `work` is `null` on every frame from the press to 30 frames after; Water `xp` +2. Slot 2's badge is ★ with `aria-label` containing "Mastered".
G10. **Pause and lock rules.** With the builder open, press Escape (the `pauseNow` path): pause card visible, `work === null` within 2 frames, `casts` +0, Water `xp` unchanged. Resume (`#resume`): the builder does not reopen; `gameTime` did not advance while the builder was open or across the pause. The overlay never called `requestPointerLock` or `exitPointerLock`: the section wraps both on the canvas and the document and counts calls while it is open, and expects 0.
G11. **Point mode and taps.** In point mode (`#pointmode`), `tap` at a burning target's screen fraction opens the builder, which is clickable with the real cursor (`page.click` on pieces). A correct build casts at the tapped target: the impact's object is the tapped one, even with the crosshair on another object.
G12. **No spell switch while open.** Keys `1`-`4`, two wheel steps and `cmd('select 3')` while open leave `spell` unchanged. After the builder closes, `select 3` works.
G13. **Reset closes it.** `cmd('reset')` while open gives `work === null`, `#worked` hidden and `paused === false` within 2 frames.
G14. **Time to cast, per band (the lag check).**
  - For each band in `['sprout', 'seedling', 'apprentice']` (reloading with `&band=<b>` and `castmode worked`), and each spell in {Water, and Fire where learnable}: set the spell to Learning, face a target, click, and build the correct answer with one input every `PACE = 150 ms`. Then the same with the spell Practised.
  - Record `openFrames` (press to `#worked` visible), `inputs` (pieces, taps or characters, plus Make or Enter), `castFrames` (final input to `casts` +1) and `ttcMs` (wall time from press to `casts` +1).
  - Pass when, for every row: `openFrames <= 2` and `castFrames <= 2`; `ttcMs - inputs * PACE <= 400` (the game's own overhead at `?low`); the CPU frame in which `#worked` first shows is at most 50 ms (`window.__cpuFrames`, GS-5); and, at 1366 × 768, the panel's top is at least 24 px below the crosshair and every piece and the Make button measure at least 64 × 64 px at Sprout and 44 × 44 px otherwise (`getBoundingClientRect`, GS-22).
  - **Input-count tripwire (not a measurement).** `inputs` for Water at Sprout is exactly 4 (H, H, O, Make). Every row also computes `estS = 1.0 + inputs * perInput[band]` with fixed planning constants (`perInput` = Sprout 1.5 s per pick, Seedling 0.9 s per tap, Apprentice 0.35 s per character), which are not a sourced claim; pass when `estS <= 10` for every row. This only trips if a band's representation grows past the target's 3-10 s; the panel measures real players.
  - Every row is printed as one line `WORKEDTTC {"band":…,"spell":…,"route":…,"inputs":…,"openFrames":…,"castFrames":…,"ttcMs":…,"estS":…}`. The tester copies the table into `docs/loop/items/I-029/evidence/ttc.md`.
  - **Fail-first:** today there is no builder, so `openFrames` is never reached.
G15. **Teardown.** After the section, `castMode === 'instant'`, `progress` is all `unknown`, and `select 2`, `cast` gives `casts` +1 within 2 frames.
G18. **Band unknown at the first click.** Load `base` with no `band` (and analytics off, as in `pkg-out`), `castmode worked`, fresh progress, Water selected, relock. A locked click opens `#worked` within 2 frames with `work.route === 'choose'` and three `.bench-band[data-band]` buttons, each at least 160 × 160 px and holding an `svg`; no `.bench-atom` is shown. Clicking `sprout` (virtual cursor) gives, within 2 frames of the state change, `work.route === 'guided'` with the same `work.id` and four `.bench-atom` cards. Build H, H, O, Make: `casts` +1. Then `select 1` (Fire) and click: `casts` +1 within 2 frames and `work === null` throughout (Fire is not learnable at Sprout). Press `KeyB`: the bench opens with no chooser (band shared for this page load). Reload: the next worked click shows the chooser again, and `localStorage` and `sessionStorage` hold no key containing `band`.
G19. **Typing is not eaten.** At `&band=apprentice`, Water Learning, open the builder. `#bench-input` has focus. `keyboard.type('wasd12')`: the input's value is `wasd12`, `pos` is unchanged and `spell` is unchanged. Select all and Backspace, type `H2O(l)`, press Enter: `casts` +1.
G20. **Exclusion with the bench.** With the builder open (Water Learning), press `KeyB`: `#bench` stays closed and `work.id` is unchanged for 30 frames. Cancel, press `KeyB` to open the bench (lock released, `paused`), then `cmd('cast')` and a click on the canvas: `work === null` and `casts` +0 throughout. Close the bench. In point mode with the builder open, click `#benchbtn`: `work === null` within 2 frames of the bench opening.
G21. **Packaging.** In the built `pkg-out` page, `fetch('worked.js')` and `fetch('bench.js')` both return status 200.

**Golden and perf stay green.**

G16.
- Every existing check passes 3/3 with `QUERY=?low`. No threshold edits and no edits to existing checks except the key-order list (Contract).
- `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs` (or I-024's pairs driver if merged) runs 3 interleaved base/branch pairs. Pass when the median CPU frame p50 is at most base + 0.5 ms and TTI is at most base + 0.3 s. Perf runs with `debug`, so instant mode: this measures the closed overlay's cost.

G17. **Lane hygiene.** `git diff --name-only origin/main...` lists only paths in Lanes. `git diff -U0` on `main.rs` touches only: the `mod` line; the `Working` and `WorkCtx` items; `Game.work`; one `insert_resource`; `control` lines 689, 715-733 (the guard), 751-754 (the `debug_only` list), 760 and 816-819; `aim_and_cast`'s parameter list, its `work`/`castmode` extraction before 1040, and the gate before 1079; `publish`'s parameter list, format tail and argument tail. `git diff -U0` on `build-web.sh` touches only line 21. `git diff` on `bench.js` is empty, or exactly the one named commit in "Requested of I-028".

**Panel brief note (for the tester, who owns `tests/e2e/panel/**`).** Personas now see the builder on their first click unless the spell is Mastered. The brief must say: "a small panel may ask you to build the spell; the game waits while it is open". The time-to-first-cast metric includes the build. The brief also asks each persona, as a **GS-30 [panel]** question: "Did rebuilding the spell before casting feel like part of the game, or like a quiz?" The paired base-vs-artifact run is the decision point for the mechanic's feel; the loop does not tune it from one persona's answer.

## GS walk (screens, controls and effects this item touches)

- **GS-3:** nothing shakes or bobs. A wrong build shows ✗ and text only.
- **GS-4:** the camera never moves while the builder is open (G4), and nothing moves it after it closes, because deltas went to the virtual cursor.
- **GS-5:** the CPU frame when the builder opens is at most 50 ms (G14).
- **GS-6:** the guided pulse is 1 Hz on the pieces only, and static with reduced motion (I-028's `guided` option).
- **GS-7:** Esc and P pause as before. An open builder cancels on pause, and the world was already held (G10).
- **GS-9 and GS-13:** no holds and no chords. The builder works fully from the keyboard with I-028's focus model (Tab, arrows, Enter/Space, typing, Esc); typing is never eaten (G19).
- **GS-11:** ages 4 to 8 have a mouse-only path: the chooser and picture cards are clickable with the virtual cursor under lock or the real cursor in point mode. A Sprout Water build is 4 clicks (H, H, O, Make), plus one chooser click on the first cast of a page load with no band.
- **GS-12:** the builder shows within 2 frames of the click, and the cast lands within 2 frames of the submit (G3, G7, G14).
- **GS-15, GS-16 and GS-17:** right and wrong use icon, text and sound. Success is the cast itself (existing pulse, sound and stream).
- **GS-18:** panel text is at least 18 px.
- **GS-19:** stages are shapes (○ ◔ ◑ ★), and right and wrong are ✓ and ✗, so nothing relies on colour alone.
- **GS-21:** copy is short and at grade 6 or below. Sprout content is pictures (I-028's cards), and the guided highlight shows what to do without reading.
- **GS-22:** pieces and buttons are at least 44 px, or 64 px for Sprout, measured inside the panel at 1366 × 768 (G14); chooser pictures are 160 px (G18).
- **GS-23:** the guided Unknown build is a safe first cast: the world waits, the answer is shown to copy, a mistake costs nothing, and the spell then fires.
- **GS-24:** the objective line stays visible above the panel.
- **GS-25:** the world holds while the builder is open, so a build carries no time pressure (G4).
- **GS-26:** no label shames the player. "New", "Learning" and "You know this one" are neutral.
- **GS-27:** no band is assumed: the player is asked once per page load (G18).
- **GS-28:** no streak, lure or countdown. The pointer to the bench is one neutral line, and instant casting is earned by mastery, never sold.
- **GS-30:** the build *is* the spell: the formula built is the one the engine casts, and nothing is quizzed that the cast does not use. The rebuild per cast before mastery is mandated by `target.md`; whether it reads as a quiz to the 13-year-old is the panel question above.
- **GS-32:** Sprout builds pictures, not formulas, and every build is followed at once by the visible effect.
- Every other GS item: no change.

## Out of scope

- The Formula Bench, the band representations, the checker and the reason texts (I-028), apart from the four requested `bench.js` additions. The mastery trial, unaided builds, predict questions and keep-it-sharp (I-030); the field overlay never offers them.
- Shuffled or "remembered" piece layouts. If a shuffle is wanted for retrieval practice, it is a later pedagogy item with a source.
- An XP pop or XP totals (I-039). Only stage badges and the Practised toast are shown here.
- Crafting settings (I-031). Saving tuned spells to the hotbar (I-032). A hotbar rendered from data, or more than 4 slots.
- A setting that turns worked casts off for players. `cast=instant` is debug-only on purpose. If teachers ask for one, it becomes a GS-26 assist item.
- A compact Sprout mode in which Make is the only press on a guided route (a panel-driven follow-up for I-028 if B4 needs it).
- The Adept band. `adept` reads as Apprentice until the Adept bench follow-up.
- New analytics events or any change to collected data. Native (desktop) builder UI; native stays instant.
- `crates/bevy-client/README.md` controls table: a one-line docs follow-up for the leader to file.

## Open decisions (defaults taken; the critic may overturn)

1. **The world holds while the builder is open.** Letting it run puts unchosen time pressure on 4 to 7-year-olds (GS-25). The cost is that worked casts are never risky in combat; mastery still pays off in speed and XP.
2. **`debug` means instant casts by default** (a test hook). Golden, perf and analytics-e2e keep working with no edits to existing checks. The panel and real players never pass `debug`. G1 pins the real default.
3. **Unknown spells cast through a guided build, with no XP**, rather than being blocked. This keeps the first 30 seconds playable (B4, GS-11) before the player finds the bench.
4. **A spell above the player's band casts instantly with no XP** (Fire at Sprout). I-028's `judge` cannot accept any build for it at that band, and I-028 hides it from the Sprout bench, so there is nothing to rebuild. The alternative, blocking the cast, would leave a dead hotbar slot for a 5-year-old.
5. **The Sprout build is I-028's 4 inputs**, not a 1-press compact mode (see Out of scope).

## Contract

A stub lands on the branch base before the lanes start. It has no behaviour, and every lane codes against it.

- **`worked.rs`**, with these public items and neutral bodies: `route` returns `Instant`, `request` returns false, `submit` returns `Stale`, `reroute` returns `None`, `set_view` does nothing, `state_json` returns `null`, `mode_from` is real (three lines), and `HELP_AFTER = 2`.

  ```rust
  pub enum CastMode { Worked, Instant }
  pub fn mode_from(debug: bool, cast: Option<&str>) -> CastMode;
  pub enum Route { Instant, Worked, Guided, Choose }   // as_str: instant|worked|guided|choose
  impl Route { pub fn as_str(self) -> &'static str }
  pub fn route(stage: u8 /* 0..=3 in I-027 Stage order */, learnable: Option<bool> /* None: band unknown */, mode: CastMode) -> Route;
  pub enum Target { Crosshair, Tap(f32, f32) }
  pub struct Work { pub id: u32, pub spell: usize, pub route: Route, pub target: Target, pub wrong: u8, pub help: bool }
  pub enum Submit { Stale, Wrong { help: bool }, Cast { spell: usize, target: Target, earns: bool } }
  #[derive(Default)] pub struct Desk { /* private: Option<Work>, view: String */ }
  impl Desk {
      pub fn request(&mut self, id: u32, spell: usize, route: Route, target: Target) -> bool; // id from Working::next_id
      pub fn submit(&mut self, id: u32, correct: bool) -> Submit;
      pub fn reroute(&mut self, route: Route) -> Option<Submit>; // only from Choose
      pub fn set_view(&mut self, json: &str); // trusted JSON from BenchView::to_json, emitted verbatim
      pub fn cancel(&mut self, id: u32) -> bool;
      pub fn is_open(&self) -> bool;
      pub fn open(&self) -> Option<&Work>;
      pub fn state_json(&self) -> String;
  }
  ```

  Derives: `Clone, Copy, Debug, PartialEq` on the enums (`Eq` where possible). No Bevy types and no `crate::` imports, so it compiles alone with `rustc --test`. The `view` string comes only from `BenchView::to_json()` (serde, so player-typed text in `bad` is escaped there); `Desk` never builds or parses it.
- **`main.rs` stub wiring**, which lands whole; lanes do not redo it:
  - `mod worked;`;
  - `#[derive(Resource)] struct Working { mode: worked::CastMode, band: Option<Tier>, next_id: u32, done: u32, wrong: u32 }`, inserted in `main()`, with `band: None` and `next_id` starting at 1, never reset;
  - the `WorkCtx` `SystemParam` as an `aim_and_cast` parameter, unused in the stub;
  - `Game.work: worked::Desk`;
  - the `castmode` command and the `work `/`castmode ` filter pass-through (real, each one line);
  - the new state keys with neutral values.
  - The gate, the paused line, the select guards, `work band`, the submit handling and the view are the rust lane's.
- **`build-web.sh:21`**: `web/worked.js` added to the `cp` line, and an empty `web/worked.js` exporting `createWorked` that returns no-op methods (`isOpen()` false, `move()` false), plus the `hud.js` import and create line (hook 1), so the built site loads on the stub.
- **New `__gameState` keys**, appended after the last key at the branch base, in this order:

  | key | type | meaning | neutral value |
  |-----|------|---------|---------------|
  | `castMode` | `"worked"` \| `"instant"` | the `Working` mode | real |
  | `routes` | array of 4 route strings | `route(stage(i), learnable(i), mode)` in `Spell::ALL` order | `["instant","instant","instant","instant"]` |
  | `work` | object or `null` | `Desk::state_json()`: `{id, spell, route, wrong, help, view}` where `view` is an I-028 `BenchView` or `null` | `null` |
  | `workDone` | u32 | accepted worked casts since boot (not reset by R) | 0 |
  | `workWrong` | u32 | wrong submits since boot | 0 |

  `Desk` lives in `Game`, so `reset` clears `work`. The ids, the band and the two counters live on `Working`, which `reset` never touches. Ids are unique for the page load, and a stale `work submit <id>` sent before an R can never match the next builder.
- **Golden key-order check (the stub makes the change).** In `clockAndBudget`, add `const I029_KEYS = ['castMode', 'routes', 'work', 'workDone', 'workWrong'];` and append it at the end of the required order and the length sum, after `I027_KEYS` (I-028 adds no `__gameState` key). Nothing else in that check changes.
- **Commands** (all through `control`'s filter into `aim_and_cast`):
  - `work band <sprout|seedling|apprentice|adept>`, `work submit <id> <text…>` and `work cancel <id>`: any build. The text is judged in Rust by I-028's `judge`, so typing it at the console is no cheat.
  - `castmode worked|instant`: `?debug` only.
- **DOM:** `#worked` (the panel; hidden or removed when closed); `#worked .vcursor`; `#worked .reason`; `#worked-toast`; `.slot .stage[aria-label]`. Inside `#worked`, I-028's classes (`.bench-band`, `.bench-atom`, `.bench-tile`, `.bench-coef`, `.bench-make`, `#bench-input`) and `bench-next` for the guided highlight.
- **Meanings both sides rely on.** `work.id` changes on every open; `work === null` means closed; `paused` is true while `work !== null`; `routes[state.spell] === 'instant'` means one click casts.

### Overlap and merge order

| file | this item's hunks | in-flight item's hunks | resolution |
|------|-------------------|------------------------|------------|
| `main.rs` | `mod` line; `Working`, `WorkCtx`; `Game.work`; `insert_resource` in `main()`; `control` 689, the guards at 715-733 and 760, `debug_only` 751-754, filter 816-819; `aim_and_cast` parameter list end (after `cam`, ~961), extraction before 1040, gate before 1079; `publish` parameters, format tail, argument tail | **I-022:** `Settings` 91-113, `read_settings` 402-412, `aim_and_cast` parameter 952, one line after 980, 983, 987. **I-024:** the `world::build` call at 464. **I-025:** `Game` fields 175-185, `aim_and_cast` block after 1017, `clicked` 1036-1039, the hold gate 1069-1077, the cast-loop body 1099-1136, `projectiles`, format tail. **I-026:** `mod spells;`. **I-027:** `mod progress;`, `Progress`, `control` parameter and `prog` arm, `publish` parameter and tail. **I-028:** `mod bench;` and two `add_systems` lines after the existing ones | **Merge I-025 first.** This item places its gate after I-025's held-path block and before `let ended`. `Game` field additions and the format tail are append-only: I-029 goes last. I-022's 952 edit and this item's new last parameter are different lines. I-028's lines are separate. The seam breaker's "one click casts once" check runs on the I-025-rebased base, not on the stub |
| `hud.js` | `startHud` imports and create; `onMove` (one line); locked `pointerdown` (one line); wheel (one line); one new capture `keydown` listener; `overUi` selector; `showPause` (one line); `frame()` (one call after the parse at ~619) | **I-022:** `setHint` and `hintAllowed` (441-444). **I-025:** `pickHint`, `frame()` 665-673, `__game`. **I-028:** not edited | No shared lines |
| `build-web.sh` | line 21: `web/worked.js` added to the `cp` list | **I-027:** `web/progress.js`; **I-028:** `web/bench.js`, same line | Whoever lands later rebases the one line; all files end up in the list |
| `golden.mjs` | `workedCasts` at the end; one call line; key-order lines | I-022 `reviewFollowups`; I-025 `streamsAndFinder` and its key list; I-027 `spellProgress` and `I027_KEYS`; I-028 `formulaBench` | Call order: feel, I-025, I-019, I-022, I-027, I-028, I-029. Key lists: `I019_KEYS`, `I025_KEYS`, `I027_KEYS`, `I029_KEYS` |
| `style.css` | new `#worked …`, `.vcursor`, `.slot .stage`, `#worked-toast` rules, appended after I-028's block | I-028 (one appended block) | append only; `#worked`-scoped rules override bench piece sizes only inside the panel |
| `web/bench.js` | none, or the one named commit in "Requested of I-028" | I-028 (merged before this item builds) | after I-028 |
| `content/science-sources.md` | Not-claims rows, appended | I-025 Parameters rows; I-026 `where` cells; I-028 rows | append only |
| `Cargo.toml`, `Cargo.lock`, `world.rs`, `fx.rs`, `cues.rs`, `feel.rs`, `clock.rs`, `motion.rs`, `chem.rs`, `bridge.rs`, `player.rs`, `src/bench.rs`, `crates/sim/**`, `perf.mjs`, `index.html`, `age.js`, `analytics.js`, `boot.js`, `progress.rs`, `progress.js` | **not touched** | I-022, I-024, I-025, I-026, I-027, I-028 | none |

**Merge order:** I-022, I-024 and I-025 (any order among them) → I-026 → I-027 → I-028 → I-029. This item's lanes start only after I-028 merges. Its contract step rebases on that origin/main, re-reads every line number above, checks every name in "Assumed from I-028" against the merged code, and records the base sha and any 1:1 renames in a "Refresh" note at the end of this spec.

## Lanes

| lane | owns | builds |
|------|------|--------|
| contract (first, small) | `crates/bevy-client/src/worked.rs` (stub); the `main.rs` stub wiring in Contract; `crates/bevy-client/build-web.sh` (line 21 only); `crates/bevy-client/web/worked.js` (stub) and `hud.js` hook 1; the golden key-order lines; `crates/bevy-client/web/bench.js` and `test/bench.test.ts` **only** for the named commit in "Requested of I-028", if I-028 merged without those additions | The stub, the five neutral keys, `I029_KEYS`, `castmode`, the `cp` line. Native `cargo build`, the wasm build, `./build-web.sh` and `cargo clippy --target wasm32-unknown-unknown -D warnings` pass. Golden passes on the stub (keys present and neutral; the main page is instant either way). The Refresh note |
| rust | `crates/bevy-client/src/worked.rs`; `crates/bevy-client/src/main.rs`, only the hunks in the Overlap table beyond the stub | Unit tests 1-6 first (red on the stub). Then the bodies. Then `main.rs`: the paused line, the select guards, extraction, `work band`, submit handling via `bench::judge`, the `BenchView` build, the gate, `instant_cast` on Mastered, `routes` from `Progress` + `Working.band` + `bench::learnable`. Builds native and wasm. Must not touch `Settings`, `read_settings`, `src/bench.rs`, `bridge.rs`, `crates/sim/**`, `chem.rs`, `cues.rs`, `clock.rs`, `motion.rs`, `feel.rs`, `player.rs`, `world.rs`, `fx.rs`, `progress.rs`, `Cargo.*`, `web/**` or tests |
| web | `crates/bevy-client/web/worked.js`; `crates/bevy-client/web/hud.js` (only hooks 2-8 in Smallest version 5); `crates/bevy-client/web/style.css` (append only); `test/bevy-worked.test.ts` (new); `tests/e2e/golden.mjs` (the `workedCasts` function and its call line); `content/science-sources.md` (Not-claims rows only); `docs/loop/items/I-029/evidence/**` | Node tests 8-12 first. Then `worked.js`, the hooks and the CSS. Golden G1-G15 and G18-G21 written against the stub (G1, G2 and G21 pass on the stub; the rest fail until the rust lane lands) and recorded. Must not touch `bench.js`, `build-web.sh`, `index.html`, `age.js`, `analytics.js`, `boot.js`, `progress.js`, `setHint`/`hintAllowed`, `pickHint`, I-019's `frame()` hint, objective, crosshair and finder lines, the panel harness, or any existing golden check |

The rust and web lanes run in parallel once the stub is in. **Seam:** the `work` JSON (including the `BenchView` in `view`) and the `work …` command strings; G3-G20 need both lanes. The shared build runs the full golden plus G16 (perf pairs). The tester writes `evidence/ttc.md` from the `WORKEDTTC` lines.

The seam breaker checks:
- that `paused` is true on every frame while `work !== null`, and that `gameTime` never advances then;
- that one click can never cast twice (once instantly and once from a worked submit), and that the held path cannot reopen the builder right after an accepted cast (run after rebasing on I-025);
- that a stale `work submit` from a closed builder, a reset or a reload does nothing, and that `work submit` on a `choose` route does nothing;
- that the text `onMake` sends for a correct build at each band is judged `ok` through `work submit` for every learnable spell (the same compositions I-028's seam breaker checks through the bench);
- that `keyup` is never swallowed, and printable keys are never `preventDefault`ed while open;
- that `routes` and the HUD badges agree for all four stages, and `routes` shows `choose` for non-Mastered spells until a band is known;
- that `B` never opens the bench over the overlay, and no builder opens while the bench is open;
- that `cast=instant` without `debug` is ignored in a release build;
- that the accepted cast uses the target from the open, not the one from the submit, for taps (including after a `choose` re-route to `instant`).
