# I-029: Worked casts, then instant casts

**based_on:** origin/main d476ac8 (2026-11-01, merge of PR #47, I-028 Formula Bench). I-026, I-027 and I-028 are merged. Every name and line number below was read from d476ac8.

**Revision 3 (wave 13, on d476ac8).** Revision 2 was written against c4832d2, before I-028 existed. It assumed a `bench.js` API (`currentBand`, `chooseBand`, `mountChooser`, a `guided` option) that I-028 never shipped, and it put new keys into `__gameState`, which would break two existing golden checks. Revision 3 uses the merged code as it is. "Changes from revision 2" at the end lists every change. `critique.md` (revision 1) and the planner response there still apply, except where that list says otherwise.

Phase 2, third item of the core loop. `docs/loop/target.md`, "Core loop": "Before mastery every cast is a worked cast (rebuild the formula in a quick overlay, about 3-10 s) ... Then the spell is an instant cast." Backlog row I-029: bars B9, B4, B11. It depends on I-027 and I-028, which are both merged. It runs in parallel with I-035 (level 2, branch `loop/I-035`); see "Overlap and merge order".

Paawan's rule (`config.md`, `visible_each_wave`): every wave ships something a player can see and play. This item ships the overlay, stage badges on the hotbar, and the instant cast once a spell is mastered. It must stay quick and fun, never a chore.

## Facts from d476ac8 this spec relies on

`main.rs` means `crates/bevy-client/src/main.rs`, `hud.js` means `crates/bevy-client/web/hud.js`, `bench.js` means `crates/bevy-client/web/bench.js`, `client bench.rs` means `crates/bevy-client/src/bench.rs`, `sim bench.rs` means `crates/sim/src/bench.rs`, and `golden` means `tests/e2e/golden.mjs`.

**The cast path (`main.rs`)**
- `aim_and_cast` starts at `main.rs:1040`. It has 13 parameters; I-035 adds `level: Res<Level>`, which makes 14. Bevy's limit is 16.
- `let incoming = std::mem::take(&mut web.cmds);` is at `:1149`. `fire` commands are counted into `clicks` at `:1153`.
- While paused, the queue is cleared (`:1156-1157`). Otherwise one queued command pops when `cooldown <= 0` (`:1167-1171`). `cast`/`fire` push `game.aim` (`:1177`) and `tap` pushes a pick along `ray_at` (`:1178-1182`).
- The I-025 held path pushes `game.aim` only when `casts` is empty (`:1201-1209`).
- The cast loop starts after `let ended` (`:1210-1211`). So a gate placed just before `:1210` covers every source: a locked click (`fire`), `cast`, `tap`, and the held button.
- `Aim { point, obj, assisted }` is `Clone, Copy, Default` (`main.rs:137-142`).

**Pause and look (`control`, `main.rs:752`)**
- On wasm, `game.paused = web.paused;` is at `:776`, and `player.look(web.dx, web.dy, settings.sens)` runs unconditionally at `:778`.
- While paused, these all stop: the game clock (`:798`), movement (`:935`), `chem_tick` (`:741`), projectiles and streams (`:1300`, `:1337`) and fx (`:1502`). In `aim_and_cast`, queued casts drop and the held path is off.
- Spell selection comes from three places in `control`: the digit loop (`:803-815`), the native wheel (`:817-820`) and the `select` arm (`:847`). On the web, the wheel sends `select` from `hud.js:371-376`.
- The command filter at `:908-911` keeps only `tap*`, `cast` and `fire` for `aim_and_cast`.
- `debug_only` (`:838-841`) blocks a command's `match` arm without `?debug`. The filter still passes it on afterwards, so a debug check for a command handled in `aim_and_cast` must be made there (`settings.debug`, `main.rs:112`, read at `:457`).

**Other `main.rs` facts**
- `reset` (`main.rs:970-985`) rebuilds `Game` from `default()`, keeping `spell`, `locked`, `paused` and `epoch`. I-035 adds a `level` argument.
- `Progress(progress::Book)` is a `Resource` outside `Game` (`main.rs:147`).

**I-027: progress (`crates/bevy-client/src/progress.rs`)**
- `Book::get(i) -> Option<&SpellProgress>`. `SpellProgress.stage: Stage { Unknown, Learning, Practised, Mastered }` is ordered and has `as_str()`.
- `Book::worked_cast(i, correct)`: on Unknown it does nothing. On Learning, Practised or Mastered it adds `XP_WORKED` = 10 when correct and 0 when not. `settle` moves Learning → Practised at `PRACTISED_XP` = 30.
- `Book::instant_cast(i)` adds `XP_INSTANT` = 2, on Mastered only.
- Mastery is set only by `settle` (3 builds over 2 days plus a prediction; I-030 wires these in play).
- The `prog` commands are handled at `main.rs:848-852` by `progress::apply_cmd`. `prog load <save>` is allowed without `?debug`. The save `p1|water:3:90:3:2:20756:1` is Water Mastered, and `prog clear` needs `?debug`. **This item's test hook for mastery is `prog load`.** It adds no new mastery path; how a spell becomes Mastered stays I-030.

**I-028: the bench (`sim bench.rs`, `realgame_sim::bench`)**
- `judge(spells: &[SpellDef], slot: u8, band: Tier, text: &str) -> Verdict` (`:70`). `Verdict { ok, code, diffs, other, bad, simplest, coef }` (`:37-52`); `Code` serializes snake_case (`atoms_off`, `not_for_band`, ...).
- `learnable(def, band) -> bool` (`:348`) is `band >= def.tier_min`.
- `recipe(def) -> Vec<RecipeTerm>` (`:296`) and `prompt(def, band) -> Option<&str>` (`:353`).
- `SpellCard { slot, id, name, formula, stage, prompt }` (`:363`), `ResultView { n, slot, #[flatten] verdict, learned, stage }` (`:376`), and `BenchView { v, at_bench (atBench), open, band, spells, pick, recipe, result }` (`:390`) with `BenchView::closed(at, band)` and `to_json()`.
- Spell data (`content/spells.json`): slot 1 Fire `tier_min: seedling`; slots 2-4 Water, Frost and CO₂ `tier_min: sprout`. **Fire is not learnable at Sprout.**

**I-028: the bench (`client bench.rs`)**
- `pub struct BenchState { pub open, pub band: Option<Tier>, pub pick, pub result, pub n, pub last_json }` is a `Resource`, inserted by `spawn_station` at Startup. Its doc comment says it is "`pub` so I-029's worked casts and I-030's trial read it (`band` in particular, kept when the bench closes)". `band` is set only by the bench queue's `band <tier>` command (`Cmd::Band`).
- `bench::run` runs at `.after(control).before(publish)` (`main.rs:404`). It drains `web.bench`, takes `ResMut<Progress>`, and publishes `window.__benchState` only when its JSON changes (`bridge::publish_bench`, `bridge.rs:109`).

**I-028: the bench (`bench.js`)**
- It is loaded by `boot.js:31` with a guarded dynamic `import('./bench.js')`, so a broken `bench.js` costs only the bench.
- Exports: `pretty(text, keepState)` (`:57`), `composeSprout`, `composeSeedling`, `composeEquation`, `scaleOf`, `reasonFor(band, result, names, tries)` (`:176`), `bandFrom(search, picked, chosen)` (`:231`), `mountBuilder(root, { band, spell, recipe, prompt, onMake })` (`:343`, which returns `{ destroy }`) and `startBench()` (`:580`).
- **No** `currentBand`, `chooseBand`, `mountChooser` or `guided` exists.
- `mountBuilder` per band:
  - Sprout: a recipe card of balls, `.bench-atom[data-el]` cards, `.bench-undo` and `.bench-make`. Water is H, H, O, Make: 4 inputs.
  - Seedling, one substance: a recipe card with the formula, `.bench-tile[data-el]` count tiles cycling 0-4, and `.bench-make`.
  - Seedling Fire: `.bench-coef[data-term]` coefficient tiles under the balance scale, and `.bench-make`.
  - Apprentice: `#bench-input` with no recipe, Enter or `.bench-make`.
  - Pieces carry `data-token`. Keyboard helpers live on elements as `input._benchMake()` and `tile._benchStep(d)`; the bench's own `onKey` (`bench.js:1057-1087`) calls them, because its window-capture `stopImmediatePropagation` stops the element's own listeners.
- The band resolves in `startBench` (`resolveBand`, `:634-642`): `?band=`, then the age picker's pick (`#age button[data-band]`, shown only where `rg-analytics` is on: the built site, `web/dist`), then the bench chooser. Each new value is pushed as `band <b>` on `__gameIn.bench`, so **Rust's `BenchState.band` is the band** whenever one is known. Golden check "bench: ?band=apprentice reaches the game within 2 s of the first state, without an open" pins this. On a page with no `?band=`, no picker and no bench visit, `BenchState.band` is `None`.
- The bench's key listener is in window capture. While closed it takes only `KeyB`, and only when the target is not editable and no other visible `[aria-modal="true"]` shows (`gateBlocked`). Opening calls `exitPointerLock` and `__game.pause(true)`.
- Test hook `window.__bench = { state(), ui(), open(), close(), cmd(s) }`.

**`hud.js`**
- `startHud()` begins at `:133`, and `cmd` is at `:175`.
- `showPause` is at `:230-244`, and `window.__game = { … }` is at `:245-254`.
- `onMove` is at `:313-336`, with its locked branch at `:316-321`. The locked `pointerdown` listener is at `:365-369`, the wheel listener at `:371-376`, and `pauseNow` and the Escape/P keydown at `:423-440`.
- **The pointer-lock block is out of bounds (I-047 is parked on it):** `pending`/`settle`/`tryLock`/`lock` (`:176-225`) and `syncLock`/`delta`/`movement`/`moveWorks` (`:258-296`). This item does not edit those lines. `delta` clamps a locked move to ±200 px; I-047's bogus post-click jump arrives as a ±200 px move.

**`boot.js` and `build-web.sh`**
- `boot.js:27` calls `startHud()`, then dynamically imports `progress.js` (`:30`) and `bench.js` (`:31`).
- `build-web.sh:21` copies an explicit list: `cp web/index.html web/style.css web/boot.js web/hud.js web/progress.js web/bench.js web/favicon.svg "$OUT/"`.

**Golden**
- `formulaBench(...)` is called at `golden:351`, and `check('no console errors', …)` is at `:353`.
- `clockAndBudget` pins the exact `__gameState` key order (`:1053-1059`). `spellProgress` test 16 pins the tail after `fireInView` to exactly `progress,progRev,progSave` (`:2668-2672`). **So any new `__gameState` key would break two existing checks.** This item adds none (see Contract).

## Problem

After I-027 and I-028, a player can learn a spell at the Bench and the game keeps per-spell progress. But in the field every spell still fires on one click. Nothing in play asks for the formula again, so worked casts do not exist and Learning can never become Practised through play. Mastery buys nothing, because every cast is already instant. B9 ("uses an instant cast") cannot even be observed.

There are two risks:
- **Lag.** A shooter where every click opens a box feels slow.
- **Lock-out (I-035's F1).** Once casting depends on learning, a Sprout child cannot learn Fire at the Bench (`tier_min: seedling`). Level 1 (water on fires) and level 2 (Frost to freeze the stream, then Fire to melt the gate) must stay winnable for a 5-year-old.

**Goal:**
- Before mastery, a click opens a small builder below the crosshair within 2 frames, and the world holds while it is open.
- A correct build fires the spell at the target of the click within 2 frames and earns XP.
- A mastered spell fires on one click.
- No spell is ever locked: every route ends in a cast unless the player cancels.

## The Sprout rule (answers I-035 F1)

**A spell the player's band cannot learn casts instantly, with no build and no XP.** At Sprout that means Fire. Fire then works at Sprout exactly as it does today, so level 2's gate melts for a 5-year-old. I-035 needs no Sprout heat spell, and I-028's bench is not changed.

The same rule covers a page where no band is known yet: every spell casts instantly until the band is known. There is nothing to build at an unknown band, and asking a child's age inside the field overlay would be a second chooser that I-028 does not export.

Reasons:
- I-028's `judge` returns `not_for_band` for every text at a band below `tier_min`. A build there could never succeed, so gating the cast would leave a dead hotbar slot.
- Unlearned but learnable spells are **not** locked either. They open the builder with the answer shown to copy (the "copy" builds below), and earn no XP.
- So casting never depends on learning. Learning changes only what a cast earns, and mastery removes the build.

## Bars moved

- **B9 (mastery works):** enabling.
  - Worked casts earn XP in play, so Learning → Practised now happens through casting.
  - "Uses an instant cast" becomes observable: the hotbar ★ badge, and `routes[i] === 'instant'` for a Mastered spell.
  - B9 needs I-030's trial too, so nothing is reported as moved at merge.
- **B4 (every age can play):** must hold. Predictions for the first paired panel run (same seed, merge base vs this artifact, no `debug`):
  - Every persona still lands a cast within 30 s.
  - A 6-year-old with no band known casts instantly, as today. At `band=sprout`, the first Water cast is a 4-input copy build (H, H, O, Make).
  - (critic r3) On the built site the age picker sets the band, so for real players the **first click of the game** at a known band opens a copy build. The panel run must use a page where the band is known (the picker or `?band=`) for at least the 6-year-old and the 13-year-old; a band-less panel page would measure today's game, not this item.
  - No persona reports "stuck in a box", "the view jumped" or "the mouse stopped working".
  - Median clarity drops by at most 1 against the paired base. If it drops by more, the critic decides; the loop does not tune routes on its own.
- **B5 (fun):** must hold (critic r3). Mastery needs 2 days, so in a first session **every** cast of a learnable spell is a build; the fun risk is the 10th Water build, not the first. Prediction for the paired run: median fun drops by at most 1, and no persona's fun drops by 2 or more. If either fails, or the 13-year-old answers "quiz" to the GS-30 question, the critic decides between merging with a follow-up and the pre-agreed fallback in Out of scope (charged casts); the loop does not tune routes on its own.
- **B11 (more than a shooter):** enabling. Before mastery, every cast at a known band uses the spell's chemistry (GS-30).
- **B1:** stays green. Golden's page passes `debug`, so it is in instant mode (Open decision 2), and no existing check is edited.
- **B2, B3:** no regression. While the overlay is closed it costs one string compare per frame. `worked.js` is under 12 KB unminified.
- **B6:** stays at 0 errors. Formulas and reasons come from `spells.json` and `bench.js`. This item's chrome lines get Not-claims rows.
- **B7:** unchanged. There is no new analytics event and nothing is sent.

## Smallest version

### 1. Routes (Rust, pure: `crates/bevy-client/src/worked.rs`, new; no Bevy types, no `crate::` imports)

A spell's route comes from its stage, whether it is learnable at the band, the cast mode, and whether the overlay is ready. The first matching row wins:

| # | condition | route | what a click does | XP |
|---|---|---|---|---|
| 1 | mode `Instant`, or the overlay not `ready` | `instant` | casts at once (today) | none |
| 2 | stage Mastered | `instant` | casts at once; `instant_cast(i)` | +2 |
| 3 | band not known (`learnable == None`) | `instant` | casts at once | none |
| 4 | not learnable at the band (`Some(false)`): **the Sprout rule** | `instant` | casts at once | none |
| 5 | learnable (`Some(true)`), stage Unknown, Learning or Practised | `worked` | opens the builder | +10 on a correct, unhelped build (`worked_cast(i, true)`); I-027 gives 0 on Unknown |

Rows 3 and 4 add no XP call at all, and row 1 neither.

### 2. The builder is I-028's `mountBuilder`, unchanged

`worked.js` calls `mountBuilder(mount, { band, spell, recipe, prompt, onMake })` with values from `work.view`, a `BenchView` that Rust builds. `onMake(text)` sends `work submit <id> <text>`, and Rust judges the text with `bench::judge`. JS never decides right or wrong.

**Copy builds.** `worked.js` shows one line above the builder, `Copy: <pretty(formula)>` (class `.worked-copy`), when either:
- the spell's stage is `unknown` and the band is not `sprout`; or
- `work.help` is true (two wrong submits in this worked cast, `HELP_AFTER = 2`) and the band is not `sprout`.

At Sprout and Seedling, `mountBuilder` already shows the recipe card, so the answer is always on screen there. At Sprout the line is never shown (it is text). `bench.js` is **not edited**: there is no `guided` option and no `bench-next` class.

**XP rules:**
- A wrong submit fizzles kindly: no cast and no cooldown. It calls `worked_cast(i, false)`, which gives 0.
- A correct submit after help casts and earns 0, because `earns` is false.
- A correct submit with `wrong < 2` earns `worked_cast(i, true)`.

A worked cast never ends without a cast unless the player cancels. Worked casts never call `learn` or `unaided_build`, and the field overlay never offers I-030's trial.

### 3. The band is `BenchState.band`

Rust reads `Res<bench::BenchState>.band`. I-029 adds no band command, no chooser and no storage. `adept` is `Tier::Adept`, which `learnable` and `judge` treat as at least Apprentice.

### 4. Rust wiring (`main.rs`, small hunks only)

**New state**
- `Game` gains two fields, appended: `work: worked::Desk` and `work_aim: Aim`. `reset` rebuilds `Game`, so a reset closes an open builder with no extra code.
- A new resource `Working { mode: worked::CastMode, ready: bool, next_id: u32, done: u32, wrong: u32, routes: [worked::Route; 4], last_json: String }` is inserted in `main()`. `mode` is `worked::mode_from(settings.debug, param("cast").as_deref(), cfg!(target_arch = "wasm32"))`, computed before `.insert_resource(settings)` (`:380`); `next_id` starts at 1. `reset` never touches `Working`. `Settings` is not changed.

**`control` (no new parameter)**
- `:776` becomes `game.paused = web.paused || game.work.is_open();`. This one line makes every existing pause path hold the world while the builder is open.
- `:778` runs `player.look` only when `!game.work.is_open()`, so drag-to-look in point mode cannot turn the view while building.
- The digit loop (`:803-815`), the native wheel (`:817-820`) and the `select` arm (`:847`) do nothing while `game.work.is_open()`.
- The filter at `:910` also keeps commands that start with `work ` and `castmode `.

**`aim_and_cast`** gains one parameter, `wk: WorkCtx`, a `#[derive(SystemParam)] struct WorkCtx<'w> { working: ResMut<'w, Working>, prog: ResMut<'w, Progress>, bench: Res<'w, bench::BenchState> }`. That takes it from 13 to 14 parameters, or 15 with I-035's `level`. Spell definitions come from `crate::spells::book().all()`, and the def for spell index `s` is the one with `slot == s + 1`.

1. **Right after `:1149`**, it takes every `work …` and `castmode …` line out of `incoming`, in order, before `clicks` and the paused clear:
   - `work ready`: `working.ready = true`.
   - `work submit <id> <text…>`: the text is the rest of the line, words joined by single spaces. This does nothing when the id is stale or unknown, or when `bench.band` is `None`. Otherwise:
     - It computes `v = bench::judge(spells, spell + 1, band, text)`, then `desk.submit(id, v.ok)`.
     - **On `Cast { spell, earns }`:**
       - call `prog.worked_cast(spell, earns)` and add 1 to `working.done`;
       - set `game.paused = web.paused` for this frame;
       - push `game.work_aim` to the front of `casts`, marked accepted (it skips the gate below, and it still sets `cooldown`).
     - **On `Wrong { help }`:**
       - call `prog.worked_cast(spell, false)` and add 1 to `working.wrong`;
       - rebuild the view with `result: Some(ResultView { n: work.wrong, slot, verdict: v, learned: false, stage })` and store it with `desk.set_view`.
   - `work cancel <id>`: closes the builder with no cast and no XP.
   - `castmode worked|instant`: sets `working.mode` only when `settings.debug`; otherwise it logs `warn!` (a warning, not an error).
2. **Routes.** Every frame it sets `working.routes[i] = worked::route(stage(i), bench.band.map(|b| bench::learnable(def(i), b)), working.mode, working.ready)`.
3. **The gate**, placed just before `let ended` (`:1210`). For each pending cast that was not accepted:
   - If the route is `Instant`, it casts as today. A Mastered spell also gets `prog.instant_cast(spell)`.
   - If the route is `Worked`, it calls `game.work.request(working.next_id, spell, Route::Worked)` and, when that opens, adds 1 to `next_id`. It then sets `game.work_aim = aim` (the `game.aim`, or the tap's pick, at the click), clears `cast_queue`, and does not cast. While a builder is open, a request is ignored, so one click opens at most one builder.
   - **The accepted cast lands where an instant cast at the click would have landed.** The world held, so the target has not moved.
4. **The view.** On open, the glue builds this and stores `to_json()` with `desk.set_view`:
   `BenchView { v: 1, at_bench: false, open: true, band: Some(band), spells: Some(vec![SpellCard { slot, id, name, formula: def.formula.text, stage, prompt: bench::prompt(def, band).unwrap_or_default() }]), pick: Some(slot), recipe: (band < Tier::Apprentice).then(|| bench::recipe(def)), result: None }`. The glue is a free function `work_view(def, band, stage, result) -> String` in `main.rs`.
5. **Publish.** At the end of `aim_and_cast`, it builds the `__workState` JSON (Contract). When the JSON differs from `working.last_json`, it calls `bridge::publish_work(&json)`, mirroring `bench::run`.

**Native** builds always use `CastMode::Instant`, because `mode_from(_, _, false)` returns `Instant`.

### 5. What never changes

- Aiming: the crosshair, the assist cone and `pick`.
- Movement outside the builder.
- The cooldown, streams and flight times (I-019, I-025).
- The pointer-lock rules: the overlay never requests or releases the lock, and the pointer-lock block in `hud.js` is untouched.
- The game clock (I-023): `clock.rs` and `motion.rs` are not touched, and the hold reuses `paused`.
- The bench and `bench.js`.

### 6. The overlay (web: `crates/bevy-client/web/worked.js`, new)

**Loading**
- `boot.js` gains one line after `:31`: `try { import('./worked.js').then((m) => m.startWorked()).catch(() => {}); } catch {}`.
- `worked.js` statically imports `mountBuilder`, `reasonFor` and `pretty` from `./bench.js`. That is the same module URL, so it shares the bench's module instance.
- `startWorked()` registers with the HUD (`window.__game.overlay(api)`) and then sends `work ready`. **If `worked.js` or `bench.js` fails to load, `ready` stays false, every route is `instant`, and the game is exactly today's game.** There is no state in which the world holds with no panel to close.

**API.** `createWorked({ doc, win, cmd, bench })` returns `{ update(workState), isOpen(), move(dx, dy), press(button), key(e), cancel() }`. `bench` is `{ mountBuilder, reasonFor, pretty }`, so node tests can pass fakes. `startWorked()` wires the real ones and polls `window.__workState` on `requestAnimationFrame`, parsing only when the string changes (as `bench.js` polls `__benchState`).

**Open and close come only from state**
- When `work` has a new `id`, it creates `#worked` (`role="dialog"`, `aria-label` set to the spell name, **no** `aria-modal`, so the bench's gate is not blocked). The panel is appended to `body` and holds, in order:
  - the header: the hotbar icon, the spell name and a stage word ("New", "Learning", "Practised");
  - for a `New` spell, the line "New spell: copy it. Learn it at the Formula Bench (B) to earn XP.";
  - the `.worked-copy` line, when section 2 says so;
  - a mount for `mountBuilder`;
  - `.worked-reason`;
  - a close button `.worked-close` (✕, `aria-label` "Cancel"), which sends `work cancel <id>`.
- When `work.help` turns true, it shows `.worked-copy`; the builder is not remounted.
- When `work.view.result.n` grows (and `ok` is false), it shows `reasonFor(band, result, names, result.n)` in `.worked-reason` with a ✗ mark (`bench-mark cross`) and plays the existing miss sound through `__game` (`lastSound` shows it). `names` is built from `view.spells`.
- When `work` becomes `null` (accepted, cancelled or reset), it destroys the builder and removes `#worked`, then calls `doc.getElementById('game').focus()`.
- **The bench wins.** When `window.__bench?.ui().open` becomes true while the overlay is open, it sends `work cancel <id>`.

**Layout**
- The panel is centred horizontally, at most 560 px wide, with its top at least 24 px below the crosshair's bottom edge. Its bottom stays inside the viewport at 1366 × 768. It may cover the hotbar, since selection is blocked while it is open.
- Styles scoped to `#worked` (appended to `style.css`) may shrink I-028's pieces, but only down to GS-22 minimums inside the panel: Sprout atom cards, Undo and Make at least 64 × 64 px; tiles, coefficient buttons and Make at least 44 × 44 px; text at least 18 px.

**Under pointer lock (the usual case)**
- Locked deltas move a virtual cursor (`#worked .vcursor`, a 24 px arrow), not the camera. It starts at the centre of the panel and is clamped to the panel.
- A move with `|dx| >= 200` or `|dy| >= 200` (`hud.js`'s clamp value, which is how I-047's bogus post-click jump arrives) is ignored by the virtual cursor. `hud.js` is not changed for this.
- A locked left press clicks the element under the virtual cursor (`elementFromPoint`, then the closest `button`, then `.click()`). A locked right press cancels.

**Without lock** (point mode, lock refused, touch): the panel is a normal DOM panel, clicked with the real pointer or a finger.

**Keyboard** (one window capture-phase `keydown` listener, added in `startWorked`). While the overlay is open, for every keydown except `Escape` and `KeyB`:
- It calls `stopImmediatePropagation()`. Winit, `hud.js`'s P/C/R handling and digit selection never see the key.
- It calls `preventDefault()` only on Tab/Shift+Tab, where it moves focus within `#worked` as the bench's `moveFocus` does. Printable keys are never prevented, so typing reaches `#bench-input`.
- Because propagation is stopped, it does what the element's own listener would have done, exactly as `bench.js` `onKey` does:
  - Enter in `#bench-input` calls `t._benchMake()`;
  - ArrowUp or ArrowDown on an element with `_benchStep` calls `t._benchStep(±1)`;
  - Left and Right arrows move focus.
- Enter and Space on a focused button keep their native click.
- `Escape` passes through: `hud.js` pauses, and `showPause(true)` cancels the builder. `KeyB` passes through: the bench opens and the overlay cancels itself.
- `keyup` is never touched.
- On open, focus moves to `#bench-input` (Apprentice) or to the first piece.
- While closed, the listener returns at once.

**Feedback**
- An accepted build closes the panel. The existing cast pulse and sound play when `casts` goes up. There is no separate XP pop.
- When a spell's stage goes from `learning` to `practised` (read from `stages`), `#worked-toast` shows "Water: Practised!" with the ◑ shape for 4 s.
- **Hotbar stage badges.** `worked.js` appends one `span.stage` to each `.slot` (`index.html` is not edited). Each is a shape of at least 20 px: ○ Unknown, ◔ Learning, ◑ Practised, ★ Mastered. Its `aria-label` and `title` are the stage word, so the shape carries the meaning, not colour (GS-19).

**`hud.js` hooks** (five, none inside the pointer-lock block):
1. Add `let overlay = null;` before `window.__game`, and add `overlay: (o) => { overlay = o; }` to the `window.__game` object (`:245-254`).
2. In `onMove`'s locked branch (`:316`), first: `if (overlay && overlay.isOpen()) { overlay.move(dx, dy); return; }`.
3. In the locked `pointerdown` listener (`:365`), after the `!input.locked` check and before the button check: `if (overlay && overlay.isOpen()) { overlay.press(e.button); return; }`.
4. In the wheel listener (`:371`): `if (overlay && overlay.isOpen()) return;`.
5. In `showPause` (`:230`): `if (on && overlay) overlay.cancel();`.

**Packaging.** `build-web.sh:21` gains `web/worked.js` on the `cp` line. There is no new storage key and nothing is sent over the network.

### 7. Science register

Sentence-shaped chrome text from `worked.js` gets rows in `content/science-sources.md` "Not claims", with the reason "UI instruction; states no science". That covers the New line, the toast, "Copy:", "Cancel" and the stage words if the lint flags them. Formulas, prompts and reasons come from `spells.json` and `bench.js` (I-026 and I-028 rows).

## Acceptance tests

Every test below fails on d476ac8, because `worked.rs`, `worked.js`, `window.__workState` and the `work`/`castmode` commands do not exist there. Each test also states how it fails on the contract stub.

### Rust unit tests

Run with `rustc --edition 2021 --test crates/bevy-client/src/worked.rs -o <scratch>/worked && <scratch>/worked`. On the stub, `route` returns `Instant` and `request` never opens.

1. **Route table.**
   - `route(stage, learnable, mode, ready)` for stages 0..=3 × `learnable` ∈ {`None`, `Some(false)`, `Some(true)`} × {(`Worked`, true), (`Worked`, false), (`Instant`, true)} gives exactly the table in Smallest version 1: 36 cases.
   - In particular, `route(0, Some(false), Worked, true)` is `Instant` (the Sprout rule), `route(1, Some(true), Worked, true)` is `Worked`, and `route(3, Some(true), Worked, true)` is `Instant`.
   - `as_str` gives `instant` and `worked`.
2. **Mode.**
   - `mode_from(true, None, true)` is `Instant`, and `mode_from(true, Some("worked"), true)` is `Worked`.
   - `mode_from(false, None, true)` is `Worked`. `mode_from(false, Some("instant"), true)` is `Worked`: `cast=instant` is ignored without `debug`.
   - `mode_from(_, _, false)` is `Instant` for all four combinations.
3. **Open and close.**
   - A fresh `Desk` is closed.
   - `request(1, 1, Worked)` opens with id 1, spell 1 and wrong 0, and `help()` is false.
   - A second `request(2, …)` while open returns false and keeps id 1.
   - `request(_, _, Instant)` never opens, and `spell >= 4` never opens.
   - `cancel(1)` closes it. `cancel(7)` on a closed desk returns false. After that, `request(5, …)` opens with id 5.
4. **Submit.**
   - `submit(id, false)` gives `Wrong { help: false }`, then `Wrong { help: true }`, and the desk stays open.
   - `submit(id, true)` then gives `Cast { spell, earns: false }` and closes.
   - In a new open, a first-try `submit(id, true)` gives `earns: true`. After one wrong submit, a correct one still gives `earns: true`.
   - `submit(stale, true)` gives `Stale` and changes nothing.
5. **`state_json`.**
   - A closed desk gives `null`.
   - An open one gives exactly `{"id":1,"spell":1,"route":"worked","wrong":0,"help":false,"view":null}` before `set_view`.
   - After `set_view(r#"{"v":1,"open":true}"#)`, `view` is that text verbatim.
   - `set_view` on a closed desk does nothing, and a new `request` clears the view.
6. **No panic.** A deterministic LCG drives 50 000 random operations (`request`, `submit`, `cancel` and `set_view`, with any ids, spells 0..6 and both routes) over 20 seeds. No operation panics. `is_open()` is true exactly when the last state change was a successful `request` not yet followed by a `Cast` or a `cancel`.
7. The existing plain-`rustc` tests still pass: `chem.rs`, `cues.rs`, `feel.rs`, `clock.rs`, `motion.rs` and `progress.rs`. `cargo test -p realgame-sim` and the client `bench.rs` tests still pass.

### Node unit tests

These go in `test/bevy-worked.test.ts`. They import `crates/bevy-client/web/worked.js` with a fake `doc` (the minimal DOM shim approach of `test/bench.test.ts` 11b), a fake `cmd` and a fake `bench` whose `mountBuilder` records its options and renders a few buttons and an input. They run with `npm test`. On the stub, `createWorked` returns no-ops, so 8-12 fail.

8. **Open and close follow state.**
   - `update({ work: { id: 3, route: 'worked', help: false, view } })` mounts the builder once, with `band`, `spell` (`view.spells[0]`), `recipe` and `prompt` taken from `view`. The same id again mounts nothing new.
   - `update({ work: null })` destroys the builder and removes `#worked`. `{ id: 4 }` after that mounts again.
   - `onMake('H2O(l)')` pushes exactly `work submit 3 H2O(l)`.
   - `cancel()` pushes `work cancel 3`, and nothing when closed. The `.worked-close` click does the same.
   - A `view.result` with `n: 1, ok: false` calls `reasonFor(band, result, names, 1)`, and its text appears in `.worked-reason` with a ✗. The same `n` again plays no second sound.
   - With a fake `__bench.ui()` returning `{ open: true }` while open, `update` pushes `work cancel 3`.
9. **Virtual cursor.**
   - It starts at the panel centre. `move(10000, -10000)` in 100-px steps clamps it to the top-right corner.
   - `move(200, 0)` and `move(0, -200)` leave it unchanged.
   - `press(0)` clicks the fake button under it, and `press(2)` pushes `work cancel`.
   - `move` while closed returns false.
10. **Keys.**
    - While open: a `keydown` for `Digit2`, `KeyW`, `KeyR`, `KeyP` or `Enter` gets `stopImmediatePropagation` and not `preventDefault`. `Tab` gets both. `Escape` and `KeyB` get neither. A `keyup` for `KeyW` gets neither.
    - Enter with the fake input focused calls its `_benchMake`. ArrowUp on a fake tile calls `_benchStep(1)`.
    - While closed, nothing is stopped.
11. **Copy line.**
    - Band `apprentice` with stage `unknown` shows `.worked-copy` with the text `Copy: ` plus the fake `pretty(formula)`.
    - Stage `learning` shows none until `help: true` arrives on the same id; then it shows.
    - Band `sprout` never shows it.
12. **Badges and toast.**
    - Given 4 fake `.slot`s and `stages: ['unknown','learning','practised','mastered']`, each slot has exactly one `span.stage` with ○ ◔ ◑ ★ and `aria-label` set. A second `update` adds no duplicates.
    - `stages[1]` going from `learning` to `practised` shows `#worked-toast` containing "Practised".
13. **Packaging.**
    - `boot.js` (read as text) dynamically imports `./worked.js`.
    - Every `./*.js` that `boot.js` or `hud.js` imports, statically or dynamically, and every static import of those modules, appears on `build-web.sh`'s `cp` line.
    - On d476ac8, the first assertion fails.

### Golden e2e checks

These go in a new function `workedCasts({ page, st, cmd, until, frames, sleep, base, browser, errors })` at the end of `golden`. It is called on its own line right after `formulaBench(...)` (`golden:351`) and before `no console errors`, so that check covers it. Run it as `QUERY=?low node tests/e2e/golden.mjs`.

**Helpers**
- `ws()` parses `window.__workState`.
- An in-page recorder appends `{ t: performance.now(), frame, casts, paused, workId, open: !!document.querySelector('#worked') }` to `window.__wc` on every `requestAnimationFrame`. "Within N frames" means N increments of `state.frame` after the event. `until` is used only for waits, never for timing.
- Locking uses `formulaBench`'s `lockNow` pattern. If the pointer cannot be locked, every lock check **fails**; none is skipped.

**Setup and teardown**
- The section loads `base + '&band=seedling'`, removes `rg.progress.v1`, reloads and sends `castmode worked`.
- It ends by sending `castmode instant`, removing `rg.progress.v1` and loading `base`.
- Stages are set only with `prog clear`, `prog learn 1` and `prog load …`.
- From W3 on, each check starts with `prog clear`, `reset`, `select 2` (Water) and a relock, unless it says otherwise.

**Building in tests** uses the UI only, following `work.view.recipe`:
- Seedling taps each `.bench-tile[data-el]` up to its recipe count, or `.bench-coef[data-term]` up to the recipe coefficients for Fire, then `.bench-make`.
- Sprout clicks `.bench-atom[data-el]` once per atom, then `.bench-make`.
- Apprentice types `formula.text` from `content/spells.json`, read by the test, then presses Enter.
- Under lock, a click moves the mouse so the virtual cursor reaches the element's centre, then presses left. Each step is under 200 px.
- The wrong builds are: Seedling one H tile short; Apprentice `H2O2`.

**W1. Default mode.**
- In a new context with `base` minus `debug`, `ws().mode === 'worked'` and `ws().ready === true` within 5 s.
- On the main (`debug`) page, `mode === 'instant'`. With `?low&debug&cast=worked`, it is `'worked'`.
- In the no-`debug` context, `cmd('castmode instant')` leaves it `'worked'` and logs a warning, not an error.
- On the stub, `ready` stays false.

**W2. Instant mode is today's game.** On the main page before setup, `routes` are all `instant`, `select 2` then `cast` gives `casts` +1 within 2 frames, and `work` is `null` on every recorded frame.

**W3. The gate.**
- With Water Learning (`prog learn 1`), a locked click (`mouse.down`, 60 ms, `mouse.up`) gives `casts` +0 for 30 frames.
- `work` opens within 2 frames of the press, with `route: 'worked'`. `#worked` shows within 2 frames, and `state.paused === true` while it is open. `clicks` goes up by 1.
- While it is open, `cast`, `fire` and a held button give `casts` +0 and keep the same `work.id`.

**W4. The world holds, and the view does not move.** While the builder is open:
- `gameTime` and every `objs[].t`/`temp` stay unchanged over 1 s.
- `keyboard.down('KeyW')` for 500 ms changes `pos` by 0.
- 20 locked `mouse.move` steps of 40 px leave `yaw` and `pitch` unchanged, while `.vcursor`'s position changes.
- `document.pointerLockElement` is the canvas on every recorded frame.

**W5. No stuck keys.** Hold `KeyW`, click to open the builder, release `KeyW` while it is open, then cancel with a right press. In the next 500 ms `pos` moves less than 0.01 m.

**W6. Wrong builds fizzle kindly.**
- With Water Learning, a wrong Seedling build gives `casts` +0, `wrong` +1 and `work.view.result.ok === false`. `.worked-reason` shows exactly "Count again: H should be 2. You have 1." with a ✗. (critic r3: the contract step confirms this literal against `bench.js` `reasonFor` for that verdict before the lanes start and corrects the spec if it differs; after that it is frozen.) Water `xp` is unchanged.
- A second wrong build sets `work.help === true` and shows `.worked-copy`.
- A correct build then gives `casts` +1, and Water `xp` is still unchanged (helped).

**W7. A correct build fires at the click's target and earns XP.**
- With Water Learning, `face` a burning target and note `aimObj`. Click, then build the answer.
- (arbiter r2) The builder may be opened with `cmd('cast')` instead of a real locked press, because headless Chromium turns the view on a real locked `mouse.down` (break 1). Then the recorder row where `#worked` first shows must have `aimObj === target`, the Make press must stay a real (virtual-cursor) press, and W12's lock count must still open with a real press. The real-press open is covered by W3 and W18, and real-click aim by golden's 'cast: a real click casts exactly once'.
- Within 2 frames of the Make press: `casts` +1, `work === null`, `paused === false` and `#worked` gone.
- The impact lands on that object (`hitKind` `out`).
- Water `xp` +10 and `done` +1.
- (critic r3) For the 30 frames after the accepted cast, `casts` stays at exactly +1 and `work` stays `null`: one Make press never casts twice and the held path does not reopen the builder.

**W8. Practised.** From Water Learning (10 XP), two correct worked casts make Water `practised` (30 XP) in `stages`. `#worked-toast` appears and contains "Practised", and `routes[1]` is still `'worked'`.

**W9. Mastered is instant.**
- `cmd('prog load p1|water:3:90:3:2:20756:1')` makes `routes[1] === 'instant'`.
- A locked click gives `casts` +1 within 2 frames, and `work` is `null` on every frame from the press to 30 frames after.
- Water `xp` +2. Slot 2's `span.stage` is ★, with `aria-label` containing "Mastered".

**W10. The Sprout rule (F1).**
- At `&band=sprout` with `castmode worked` and fresh progress, `routes[0] === 'instant'`.
- `select 1` (Fire, Unknown) and a locked click give `casts` +1 within 2 frames, `work === null` throughout, and Fire `xp` 0.
- `select 2` and a click open a Sprout builder with no `.worked-copy`. Building H, H, O, Make (exactly 4 inputs) gives `casts` +1, and Water `xp` stays 0 (Unknown).
- `select 3` (Frost) opens a builder too: Frost is learnable at Sprout.

**W11. No band known.** Load `base` with no `band` and no `debug` (a new context, `cast=worked` not needed). Before any bench visit, `ws().band === null`, `routes` are all `instant`, and a locked click casts within 2 frames.

**W12. Pause and lock rules.**
- With the builder open, press Escape: the pause card shows, `work === null` within 2 frames, `casts` +0 and Water `xp` unchanged.
- `#resume`: the builder does not reopen, and `gameTime` did not advance while the builder was open.
- The overlay never called `requestPointerLock` or `exitPointerLock`: both are wrapped on the canvas and the document, and calls are counted over a full W7-style worked cast (from the opening press to 30 frames after the accepted cast) and over a W5-style right-press cancel. The count must be 0. (critic r3: the window does not include the Escape press, because `hud.js`'s own pause path may release the lock there, which is not the overlay's doing.)

**W13. Point mode and taps.** In point mode (`#pointmode`), `tap` at a burning target's screen fraction opens the builder. It is clickable with the real cursor (`page.click` on pieces). A correct build casts at the tapped object, even with the crosshair on another object.

**W14. No spell switch while open.** Keys `1`-`4`, two wheel steps and `cmd('select 3')` leave `spell` unchanged. After cancel, `select 3` works.

**W15. Reset closes it.** `cmd('reset')` while open gives `work === null`, `#worked` gone and `paused === false` within 2 frames.

**W16. Typing is not eaten.**
- At `&band=apprentice`, with Water Unknown and the builder open, `#bench-input` has focus, and `.worked-copy` reads "Copy: H₂O(l)".
- `keyboard.type('wasd12rpc')` leaves the input value `wasd12rpc`, with `pos`, `spell`, `captionId` and the pause state unchanged.
- Select all, Backspace, type `H2O(l)`, then Enter: `casts` +1.

**W17. The bench wins.**
- With the builder open (Water Learning), press `KeyB`: `#bench` opens and `work === null` within 2 frames, with `casts` +0.
- With the bench open, `cmd('cast')` and a canvas click give `casts` +0 and `work === null` throughout.

**W18. Time to cast, per band (the lag check).**
- For each band in `['sprout', 'seedling', 'apprentice']`, reload with `&band=<b>` and send `castmode worked`. For each spell in {Water, and Fire where it is learnable}: set the spell to Learning, face a target, click, and build the answer with one input every `PACE = 150 ms`.
- Record `openFrames` (press to `#worked` shown), `inputs`, `castFrames` and `ttcMs` (press to `casts` +1). (arbiter r2) `castFrames` = the game `frame` of the first recorder row with `casts` +1 minus the `frame` of the Make press: the last capture-phase `pointerdown` (or `keydown` Enter at Apprentice) recorded at or before that row. It is not measured from a harness timestamp taken before the virtual-cursor moves. `ttcMs` is printed for the evidence table only.
- **Pass, per row:**
  - `openFrames <= 2` and `castFrames <= 2`;
  - (arbiter r2) the first row with `casts` +1 is also the first row with `work === null` after the Make press (same `frame`): the accept and the cast happen in one update, so any deferred cast (a timer, a second command, an extra hop through JS) fails at any frame rate;
  - (arbiter r2) the wall-time bar `ttcMs - inputs * PACE <= 400` is removed: at 1.5-3 fps headless it measures harness round trips, not the game. The per-band build time is bounded by the computed `estS` tripwires below, and a node test in `test/bevy-worked.test.ts` checks that Make sends `work submit` in the same call as the press (no timer, promise or animation frame first), which covers a JS delay shorter than one headless frame;
  - the CPU frame in which `#worked` first shows is at most 50 ms (`window.__cpuFrames`, GS-5);
  - at 1366 × 768, the panel's top is at least 24 px below the crosshair, its bottom is inside the viewport, and every piece and Make measure at least 64 × 64 px at Sprout and 44 × 44 px otherwise.
- **Input-count tripwire.** Water at Sprout is exactly 4 inputs. `estS = 1.0 + inputs * perInput[band]` (Sprout 1.5 s, Seedling 0.9 s, Apprentice 0.35 s per character; fixed planning constants, not a sourced claim) must be at most 10 for every row.
- **Level-1 chore tripwire (critic r3).** `N1` is the number of casts (`casts` delta) the existing golden level-1 completion makes, which the tester reads from the same run's log. Pass when `N1 * estS(Water, sprout) <= 60` and `N1 * estS(Water, seedling) <= 60`. A failure means a 5-year-old's level 1 would hold more than a minute of building, and it wakes the critic (B5 rule) rather than being tuned away.
- Each row prints `WORKEDTTC {json}`. The tester copies the table into `docs/loop/items/I-029/evidence/ttc.md`.

**W19. Robust when `worked.js` is missing.** In a new no-`debug` context with `page.route('**/worked.js', 404)` and `&band=seedling`:
- the game boots, `ws().ready === false`, and `routes` are all `instant`;
- a locked click casts within 2 frames, and `paused` is never true;
- the 404 is the only console error, and it is excluded the same way `formulaBench`'s missing-`bench.js` check excludes its own.

**W20. Packaging.** In the served `pkg-out`, `fetch('worked.js')` returns 200.

**W21. Teardown.** After the section, `mode === 'instant'`, every stage is `unknown`, and `select 2` then `cast` gives `casts` +1 within 2 frames.

### Golden and perf stay green

- Every existing golden check passes with `QUERY=?low`. There are **no edits to existing checks**: this item adds no `__gameState` key, so the key-order checks in `clockAndBudget` and `spellProgress` stay as they are.
- `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs` runs 3 interleaved base/branch pairs. It passes when the median CPU frame p50 is at most base + 0.5 ms and TTI is at most base + 0.3 s. Perf runs with `debug`, which is instant mode, so this measures the closed overlay's cost.

### Lane hygiene

- `git diff --name-only origin/main...` lists only paths in Lanes.
- `git diff -U0` on `main.rs` touches only:
  - the `mod worked;` line;
  - the `Working` and `WorkCtx` items and `work_view`;
  - the two appended `Game` fields;
  - one `insert_resource` and the `mode` line in `main()`;
  - `control` lines 776, 778, 803-820 (the guards), 847 and 910;
  - `aim_and_cast`'s parameter list, its extraction after 1149, the routes, the gate before 1210 and the publish at its end.
- `git diff` on `hud.js` shows only the five hooks, and no line in `:176-225` or `:258-296` changes.
- `git diff -U0` on `build-web.sh` touches only line 21, and on `boot.js` only the one added line.
- `git diff` on `bench.js`, `client bench.rs`, `sim bench.rs`, `progress.rs` and `progress.js` is empty.

**Panel brief note** (for the tester, who owns `tests/e2e/panel/**`):
- The brief must say: "a small panel may ask you to build the spell; the game waits while it is open".
- The time-to-first-cast metric includes the build.
- As a GS-30 [panel] question, ask: "Did rebuilding the spell before casting feel like part of the game, or like a quiz?" The paired base-versus-artifact run is the decision point.

## GS walk

- **GS-3:** nothing shakes. A wrong build shows ✗ and text.
- **GS-4:** the camera never moves while the builder is open (W4). Locked deltas go to the virtual cursor, and `player.look` is skipped while it is open.
- **GS-5:** the open frame is at most 50 ms (W18).
- **GS-7:** Esc and P pause as before. An open builder cancels on pause (W12).
- **GS-9, GS-13:** no holds and no chords. The builder is fully keyboard-driven with I-028's focus model, and typing is never eaten (W16).
- **GS-11:** ages 4-8 have a mouse-only path: picture cards are clickable with the virtual cursor or the real cursor. A Sprout Water build is 4 clicks.
- **GS-12:** the builder shows within 2 frames, and the cast lands within 2 frames of Make, counted in game frames from the Make press (W3, W7, W18; arbiter r2).
- **GS-15 to GS-17:** right and wrong use an icon, text and sound. Success is the cast itself.
- **GS-18:** panel text is at least 18 px.
- **GS-19:** stages are shapes (○ ◔ ◑ ★), and ✗ marks a wrong build.
- **GS-21:** copy is short, at grade 6 or below. Sprout sees pictures only.
- **GS-22:** pieces are at least 44 px, or 64 px at Sprout, inside the panel at 1366 × 768 (W18).
- **GS-23:** a New spell is a safe copy build: the world waits, the answer is shown, and a mistake costs nothing.
- **GS-24:** the objective line stays visible above the panel.
- **GS-25:** the world holds, so a build carries no time pressure (W4).
- **GS-26:** "New", "Learning" and "Practised" are neutral.
- **GS-27:** no band is assumed. With no band, nothing is built (W11).
- **GS-28:** no streak, lure or countdown.
- **GS-30:** the formula built is the one the engine casts. The per-cast rebuild is mandated by `target.md`; the quiz-feel question goes to the panel.
- **GS-32:** Sprout builds pictures, and every build is followed at once by its effect.
- Every other GS item: no change.

## Out of scope

- Any edit to `bench.js` or the bench, including a `guided` option, a chooser export or a `currentBand`.
- The mastery trial, unaided builds, predict questions and keep-it-sharp (I-030). How a spell becomes Mastered in play is I-030's job.
- A Sprout heat spell. Not needed: the Sprout rule keeps Fire instant at Sprout. A dedicated heat spell with temperature belongs to I-031.
- An age chooser inside the field overlay. With no band known, casts stay instant.
- Shuffled layouts, an XP pop or XP totals (I-039), crafting settings (I-031) and hotbar saving (I-032).
- A player setting to turn worked casts off: `cast=instant` is debug-only. If teachers ask for one, it becomes a GS-26 assist item.
- A compact one-press Sprout mode (a panel-driven follow-up).
- (critic r3) **Charged casts**, the pre-agreed fallback if the B5 rule or the level-1 chore tripwire fails: one correct build loads a few casts of that spell (for example 3) before the next build. It bends `target.md`'s "every cast is a worked cast", so it needs Paawan's yes and is its own item; it is not built here.
- Native builder UI: native stays instant.
- New analytics events.

## Open decisions (defaults taken; the critic may overturn)

1. **The world holds while the builder is open** (GS-25). The cost: worked casts are never risky in combat; mastery still pays in speed and XP.
2. **`debug` means instant casts by default.** Golden, perf and analytics-e2e keep working with no edits; W1 pins the real default.
3. **New (Unknown) learnable spells are copy builds with no XP**, not blocked and not instant. Instant would make learning a spell slow its casting down, which rewards never learning.
4. **The Sprout rule.** A spell above the band casts instantly with no XP (F1). The alternative, a Sprout Fire card, would need I-028 data and judge changes for a reaction a 5-year-old cannot build in pictures.
5. **No band, no build.** With no band known, every cast is instant. The built site has the age picker, and `?band=` links fix the band, so real players at a known band see worked casts. `pkg-out` and band-less links play as today until the bench asks for the age.
6. **Worked state lives in `window.__workState`, not `__gameState`.** This avoids editing two existing key-order checks and mirrors I-028's `__benchState`.

## Contract

A stub lands on the branch base before the lanes start. It has no behaviour, and every lane codes against it.

**`worked.rs`.** It has the public items below, with neutral bodies: `route` returns `Instant`, `request` returns false, `submit` returns `Stale`, `set_view` does nothing, `cancel` returns false and `state_json` returns `null`. `mode_from` is real (three lines), and `HELP_AFTER = 2`.

  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum CastMode { Worked, Instant }
  pub fn mode_from(debug: bool, cast: Option<&str>, wasm: bool) -> CastMode;
  #[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Route { Instant, Worked } // as_str: instant|worked
  impl Route { pub fn as_str(self) -> &'static str }
  pub const HELP_AFTER: u8 = 2;
  pub fn route(stage: u8 /* 0..=3, I-027 Stage order */, learnable: Option<bool> /* None: no band */, mode: CastMode, ready: bool) -> Route;
  #[derive(Clone, Debug, PartialEq, Eq)] pub struct Work { pub id: u32, pub spell: usize, pub route: Route, pub wrong: u8 }
  impl Work { pub fn help(&self) -> bool /* wrong >= HELP_AFTER */ }
  #[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Submit { Stale, Wrong { help: bool }, Cast { spell: usize, earns: bool } }
  #[derive(Default)] pub struct Desk { /* private: Option<Work>, view: String */ }
  impl Desk {
      pub fn request(&mut self, id: u32, spell: usize, route: Route) -> bool;
      pub fn submit(&mut self, id: u32, correct: bool) -> Submit;
      pub fn set_view(&mut self, json: &str); // trusted JSON from BenchView::to_json, emitted verbatim
      pub fn cancel(&mut self, id: u32) -> bool;
      pub fn is_open(&self) -> bool;
      pub fn open(&self) -> Option<&Work>;
      pub fn state_json(&self) -> String;
  }
  ```

  The file has no Bevy types and no `crate::` imports, so it compiles alone. Player-typed text reaches `view` only through `BenchView::to_json()` (serde escapes it there); `Desk` never builds or parses JSON.

**`main.rs` stub wiring**, which lands whole; lanes do not redo it:
- `mod worked;`;
- `Working` (with `ready: false`), inserted in `main()` with the real `mode`;
- `WorkCtx` as `aim_and_cast`'s last parameter, unused apart from the publish;
- the two `Game` fields;
- the `work `/`castmode ` filter pass-through at `:910`;
- `work_view` as a stub returning `"null"`;
- the publish of the neutral `__workState` at the end of `aim_and_cast`.

**`bridge.rs`:** `publish_work(json)` sets `window.__workState`, plus a native no-op and the `pub use` line, mirroring `publish_bench`.

**Web stub:**
- `worked.js` exports `createWorked` (no-op methods; `isOpen()` false, `move()` false) and `startWorked` (does nothing, and **never** sends `work ready`);
- the `boot.js` import line;
- `build-web.sh:21` gains `web/worked.js`;
- `hud.js` hook 1.

On the stub, the built site loads and every route is instant.

**`window.__workState`** is a JSON string, written only when it changes, with keys in this order:

  | key | type | meaning | stub value |
  |---|---|---|---|
  | `v` | 1 | shape version | 1 |
  | `mode` | `"worked"` \| `"instant"` | `Working.mode` | real |
  | `ready` | bool | `worked.js` registered (`work ready`) | false |
  | `band` | `"sprout"` \| `"seedling"` \| `"apprentice"` \| `"adept"` \| null | `BenchState.band` | real |
  | `routes` | 4 route strings | `Working.routes`, `Spell::ALL` order | all `"instant"` |
  | `stages` | 4 stage strings | `Progress` stage per spell | real |
  | `work` | object or null | `Desk::state_json()`: `{id, spell, route, wrong, help, view}`, where `view` is a `BenchView` | null |
  | `done` | u32 | accepted worked casts since boot (not reset by R) | 0 |
  | `wrong` | u32 | wrong submits since boot | 0 |

**Commands** (through `control`'s filter into `aim_and_cast`):
- `work ready`, `work submit <id> <text…>` and `work cancel <id>`, in any build. The text is judged in Rust, so typing it at the console is no cheat.
- `castmode worked|instant`: `?debug` only.

**DOM:** `#worked`, `#worked .vcursor`, `.worked-copy`, `.worked-reason`, `.worked-close`, `#worked-toast` and `.slot .stage[aria-label]`. Inside `#worked` are I-028's own classes: `.bench-atom`, `.bench-tile`, `.bench-coef`, `.bench-make`, `.bench-undo` and `#bench-input`.

**Meanings both sides rely on:**
- `work.id` changes on every open, and `work === null` means closed.
- `__gameState.paused` is true while `work !== null`.
- `routes[state.spell] === 'instant'` means one click casts.

### Overlap and merge order

| file | this item | in-flight | resolution |
|---|---|---|---|
| `main.rs` | the hunks in Lane hygiene | **I-035** (`loop/I-035`): `mod level2;`, `Level` resource, `control` parameter `level` (~`:765`), `debug_only` adds `"level"` (`:840`), `reset(.., level.n)` (`:820`, `:879`), the `level` arm, solids/ice in movement, `aim_and_cast` parameter `level` (after `cam`) and a finder block after `:1108`, `Game` fields appended, `projectiles`/`react`/`publish` hunks | Different lines except the `Game` tail (append both) and `aim_and_cast`'s parameter tail (both append; order is free). This item never edits `reset`, `debug_only` or `publish`. Whoever merges second rebases those two append points |
| `hud.js` | the 5 hooks (`:230`, `:245-254`, `:316`, `:365`, `:371`) | I-035: `:415`, `:573`, `:629`, `:688-735` (banner, objective, finder, `#capicon`); I-047 (parked): the lock block | No shared lines; the lock block is untouched |
| `boot.js`, `build-web.sh` | one line each | none in flight | — |
| `style.css` | appended `#worked …`, `.vcursor`, `.slot .stage`, `#worked-toast` block | I-035: 8 appended lines | append only |
| `golden.mjs` | `workedCasts` and its call line after `:351` | I-035: its own section and call line | call order: … `formulaBench`, `workedCasts`, then I-035's section (or the reverse if I-035 merges first); both before `no console errors` |
| `content/science-sources.md` | Not-claims rows, appended | I-035 rows | append only |
| `bridge.rs` | `publish_work` (about 6 lines) | none | — |
| not touched | `bench.js`, `client bench.rs`, `crates/sim/**`, `progress.rs`, `progress.js`, `chem.rs`, `world.rs`, `level2.rs`, `fx.rs`, `cues.rs`, `feel.rs`, `clock.rs`, `motion.rs`, `player.rs`, `spells.rs`, `index.html`, `age.js`, `analytics.js`, `perf.mjs`, `Cargo.*` | I-035 owns `level2.rs`/`world.rs`/`chem.rs`; I-028 owns the bench | — |

**For I-035:** F1 is answered by the Sprout rule. Level 2 at Sprout casts Fire instantly and Frost through a 3-input picture build (N, N, Make). Whichever item merges second adds one golden check: at `?level=2&band=sprout` with `castmode worked`, `routes[0] === 'instant'` and the gate melts with three instant Fire casts.

**For I-030:** there is no `guided` option in `mountBuilder`, so the trial's `{ band, spell, recipe, prompt, onMake }` call is already the only form. `routes[i]` lives in `window.__workState`, not `__gameState`, so I-030's test 17 (the `castmode worked` line) and test 37 read `__workState.routes`. I-030's refresh step maps this.

## Lanes

| lane | owns | builds |
|---|---|---|
| contract (first, small) | `crates/bevy-client/src/worked.rs` (stub); the `main.rs` stub wiring; `bridge.rs` `publish_work`; `web/worked.js` (stub); `boot.js` (one line); `build-web.sh` (line 21); `hud.js` hook 1 | Native `cargo build`, the wasm build (`CARGO_TARGET_DIR=/home/claude/bevy-target PATH=/home/claude/bevy-target/tools/bin:$PATH bash crates/bevy-client/build-web.sh`) and `cargo clippy --target wasm32-unknown-unknown -D warnings` all pass. Full golden passes on the stub, unchanged. |
| rust | `crates/bevy-client/src/worked.rs`; `main.rs`, only the Lane-hygiene hunks beyond the stub | Unit tests 1-6 first (red on the stub), then the bodies. Then `main.rs`: the paused and look lines, the select guards, the extraction, `work ready`/`submit`/`cancel`/`castmode`, `judge`, `work_view`, the routes, the gate, `instant_cast` on Mastered and the publish. It must not touch `Settings`, `reset`, `debug_only`, `publish`, `bench.rs`, `crates/sim/**`, `progress.rs`, `bridge.rs` (beyond the contract), `web/**` or tests. |
| web | `web/worked.js`; `hud.js` hooks 2-5; `style.css` (append only); `test/bevy-worked.test.ts`; the `workedCasts` function and its call line in `golden.mjs`; Not-claims rows in `content/science-sources.md`; `docs/loop/items/I-029/evidence/**` | Node tests 8-13 first, then `worked.js`, the hooks and the CSS. Golden W1-W21 are written against the stub, then recorded. (critic r3) On the stub these pass as guards and are not evidence of the feature: W2, W11, W19, W20, W21 and W10's first two bullets. Every other W check, including W10's last two bullets, must be seen red on the stub. It must not touch `bench.js`, `boot.js` or `build-web.sh` beyond the contract, `index.html`, `age.js`, `analytics.js`, `progress.js`, `hud.js`'s lock block (`:176-225`, `:258-296`), the panel harness, or any existing golden check. |

**Seam:** the `__workState` JSON (including the `BenchView` in `work.view`) and the `work …` command strings. W3-W19 need both lanes. The shared build runs the full golden (`BASE_URL= QUERY=?low node tests/e2e/golden.mjs`, with the Chromium lock held in the same bash command) plus perf pairs.

**The seam breaker checks:**
- `paused` is true on every frame while `work !== null`, and `gameTime` never advances then.
- One click never casts twice, and the held path cannot reopen the builder right after an accepted cast.
- A stale `work submit` (from a closed builder, a reset or a reload) does nothing.
- For every learnable spell at each band, the text `onMake` sends for a correct build is judged `ok` through `work submit`.
- `keyup` is never swallowed, and printable keys are never prevented while the builder is open.
- `routes`, `stages` and the hotbar badges agree for all four stages, and Fire at Sprout is `instant`.
- `B` opens the bench and closes the builder, and no builder opens while the bench is open.
- `cast=instant` without `debug` is ignored.
- With `worked.js` blocked, nothing ever holds the world.
- A tap's accepted cast uses the target from the click.

## Changes from revision 2

| revision 2 | revision 3 (on d476ac8) | why |
|---|---|---|
| Assumed `currentBand`, `chooseBand`, `mountChooser`, a `guided` option and a `{ destroy }` handle in `bench.js`, with a named fallback commit to add them | `mountBuilder(root, { band, spell, recipe, prompt, onMake })` → `{ destroy }` as merged; no `bench.js` edit at all | I-028 shipped without the four additions; the `destroy` handle exists |
| Band from `currentBand()` via a new `work band <b>` command and `Working.band` | `Res<bench::BenchState>.band`, which `bench.js` already pushes from `?band=`, the age picker or the bench chooser | I-028 made `BenchState.band` `pub` for I-029 |
| Route `choose`: the overlay shows I-028's chooser when no band is known | Removed: with no band known, the cast is instant (row 3) | No exported chooser; smaller; GS-27 still holds |
| Route `guided` with `bench-next` highlights for Unknown spells and help | Route `worked` for every learnable non-Mastered spell; `worked.js` shows a `Copy: …` line (not at Sprout, where the recipe card already shows the answer) | No `guided` option in `mountBuilder`; the Sprout and Seedling recipe cards already show the answer |
| Fire at Sprout casts instantly (Open decision 4) | Kept, and named **the Sprout rule**, answering I-035 F1; W10 tests it | F1 |
| Five new `__gameState` keys, plus an edit to the key-order check | `window.__workState` via `bridge::publish_work`; no existing check edited | `spellProgress` test 16 pins the tail after `fireInView`; a new key would need a second existing-check edit |
| `publish` takes `Working`; format tail extended | `publish` untouched | Follows from the row above |
| `hud.js` statically imports `worked.js` and `bench.js`; 8 hooks including `frame()` and `overUi` | `boot.js` dynamically imports `worked.js`; `work ready` handshake; 5 hooks, none in the pointer-lock block | I-028 loads `bench.js` dynamically so a failure costs only the bench; I-047 is parked on the lock block |
| Keyboard: stop everything except Escape | Stop everything except Escape and `KeyB`; replay `_benchMake`/`_benchStep` as `bench.js` does; `KeyB` opens the bench and cancels the builder | Stopping in window capture also stops the input's own Enter listener; a deterministic bench/overlay rule regardless of listener order |
| `Target { Crosshair, Tap }` in `Desk`; re-pick at accept | `Game.work_aim: Aim`, stored at the click | Smaller; the world held, so the stored aim is exact; drag-to-look cannot move it |
| `reroute` | Removed | No `choose` route |
| Practised toast and header: "Show what you know at the Formula Bench (B) to cast it instantly." | Toast "Water: Practised!"; no bench pointer for mastery yet | I-030 (the trial) has not merged, so the line would point at nothing; I-030 adds it |
| Virtual cursor moves by every locked delta | Ignores moves of ±200 px or more | I-047's bogus post-click jump arrives clamped to ±200 |
| Line numbers from c4832d2 | Re-read on d476ac8 | — |
