# I-028: Formula Bench (Sprout, Seedling, Apprentice) for the four spells

**based_on:** origin/main c4832d2e86c2408443124960b130fc6208a89aab (2026-10-30, merge of PR #31, roadmap v2). All code facts were read with `git show c4832d2:<path>`. The primary checkout (661b9ba) has no `crates/bevy-client/src`, so builders branch from the real origin/main.

**Revision 2** (answers `critique.md`; see "Planner response" there). Phase 2, second item of the core loop (`target.md`, "Core loop"): "You learn it by building its formula at the Formula Bench, at your age band's level."

**Depends on I-026 and I-027 (both specced, neither merged).** This item calls I-026's `SpellBook`/`SpellDef`/`Tier` and I-027's `Progress`/`Book::learn`. Its lanes start only once **both are on origin/main**; the contract builder rebases this spec's line numbers onto that base and records the new sha in a "Refresh" note. The backlog row says "After I-026"; it should read "I-026, I-027" (planner fixes the row).

**Runs beside I-022, I-024 and I-025** with no shared hunk except two one-line list edits (`boot.js`, `build-web.sh`) that I-027 also touches; see "Overlap and merge order". It adds **no `__gameState` key**, so it does not touch the publish format string or golden's key-order check that I-025 and I-027 both edit.

## Facts from c4832d2 this spec relies on

`main.rs` = `crates/bevy-client/src/main.rs`, `hud.js` = `crates/bevy-client/web/hud.js`, `golden` = `tests/e2e/golden.mjs`.

- **Commands and the bridge.** `bridge::poll` (`bridge.rs:41-80`) copies `window.__gameIn` (`dx, dy, locked, paused, fire, px, py, cmds`) into the `WebIn` resource once per frame, then empties `cmds`. `control` (`main.rs:666`) drains `web.cmds`, runs a `match` on the first word (`main.rs:760-812`), then keeps only `tap*`, `cast` and `fire` for `aim_and_cast` (`main.rs:816-819`). Unknown words fall through `_ => {}`. `control` has 13 system parameters (14 native); I-027 adds a 14th. `bridge::publish` writes `window.__gameState` only.
- **Keys reach the game even while paused.** `control` reads `KeyCode::Digit1..4` (spell select) and `KeyCode::KeyR` (**reset**) from Bevy's `ButtonInput` with no pause check (`main.rs:713-735`). Bevy gets keys from winit's DOM listeners, which `init()` registers after `startHud()` (`boot.js`). So any text box over the game must stop key events before winit sees them, or typing `Br` resets the level.
- **Pause.** `chem_tick` returns early while `game.paused` (`main.rs:654-656`); `game.paused` mirrors `__gameIn.paused`. `hud.js` owns it: `showPause(on)` (`hud.js:215`) sets `input.paused` and shows `#pause`. `window.__game.pause` is `showPause` (`hud.js:230-238`). `syncLock` (`hud.js:242-260`) calls `showPause(true)` whenever the pointer lock is released after it was held once. `__game.lock()` asks for the lock again (it needs a user gesture; a refusal right after a release is normal and sets a "Click again" note).
- **Keyboard in `hud.js`.** One `keydown` listener on `document` (`hud.js:413-425`): Escape toggles pause, `KeyP` pauses or resumes, `C` repeats a caption. B and E are unbound everywhere. The pointer-locked `pointerdown` listener on `document` (`hud.js:349`) pushes `fire` only while locked; the canvas `pointerdown` (`hud.js:363`) does nothing while paused.
- **Text size.** `hud.js:483` sets `--ui` (1, 1.5 or 2) on `:root`; `style.css` sizes HUD text as `calc(<px> * var(--ui))` (GS-18).
- **Age band is not known to the game.** `age.js` shows the 3-picture picker (`index.html`, `#age`, buttons `data-band="sprout|seedling|apprentice"`, each with an inline SVG) only when `rg-analytics` is `on` and the browser has not opted out, and it deliberately **does not store** the choice (shared classroom devices). `web/pkg-out` (golden, perf, panel) has the switch off, so it never shows a picker. The SVGs are in `index.html` in every build, hidden.
- **The build copies an explicit file list.** `build-web.sh:21`: `cp web/index.html web/style.css web/boot.js web/hud.js web/favicon.svg "$OUT/"`. A new `web/*.js` file is not served unless it is added there. (I-027 adds `web/progress.js` and its spec does not list `build-web.sh`; see Overlap.)
- **Courtyard.** `world::HALF = 24`. The player starts at (0, 0, 18) facing −z (`player.rs:42`). Near the start: two seat benches at (±6, 14), 2.4 × 0.7 m (`world.rs:736`); the fountain collider at (0, 3), half-size 1.6 (`world.rs:748-751`); a puddle at (7, 11) (`world.rs:1009`). The east alcove "lab bench" (the future water-splitting station, C-026) is behind the ice wall and is **not** reachable at the start. The point (−3.0, 14.5) is clear of every collider by at least 0.6 m and is about 41° left of the start view (on screen at the default 90° view width).
- **crates/sim tier logic.** `formula::parse_equation` (ASCII and Unicode, state symbols, fractions, `MAX_INPUT_LEN = 80`), `ParseError` (`Empty`, `UnknownElement`, `BadCharacter`, ...), `Equation::{is_balanced, imbalance_report, simplest, species_key}` and `side_atoms` are all std-only. The 2D tiers lived partly in sim (`spell::plan(.., auto_balance)` for Sprout, the Seedling "unbalanced fizzles with a fix" rule in `game.rs:1330-1356`, Apprentice states and numbers) and partly in TypeScript (`web/src/cards.ts` Sprout picture cards, `web/src/tray.ts` Seedling tiles and live balance scale). `game.rs` sits behind I-026's `game` feature and is not linked into the 3D client. The 2D `web/` is retired.
- **Science lint.** `test/science-sources.test.ts` scans `crates/bevy-client/src/**.rs` and `crates/bevy-client/web/**.{html,js}`; I-026 teaches it to read player-text keys of `content/spells.json`. It does **not** scan `crates/sim`. So every player-facing sentence must live in the client (JS or client Rust) or in a scanned `spells.json` key, never only in sim.

### Assumed from I-026 and I-027 (their specs at this writing)

- I-026: `realgame_sim::{tier::Tier (Sprout < Seedling < Apprentice < Adept), spellbook::{SpellDef, SpellBook, SpellError, Formula}}`; `SpellDef { id, slot, effect, name, label, hotbar_label, cast_caption, formula: Formula { text, equation }, tier_min, reaction }`; slots 1-4 equal `Spell::ALL` order; `content/spells.json` with ids `fire` (tier_min `seedling`), `water`, `frost`, `co2` (`sprout`); formulas `CH4 + 2O2 -> CO2 + 2H2O`, `H2O(l)`, `N2(l)`, `CO2(g)`; client `spells::book().all() -> &[SpellDef]`; "Extending" rule: a new per-spell field is optional (`#[serde(default)]`), validated, tested, and its player-text keys are added to the lint's key list in the same PR.
- I-027: `#[derive(Resource)] struct Progress(progress::Book)` in `main.rs`; `Book::get(i) -> Option<&SpellProgress>`, `Book::learn(i) -> Vec<Transition>` (Unknown → Learning, +10 XP, no-op otherwise); `Stage::as_str()` gives `unknown|learning|practised|mastered`; index `i` = slot − 1. Progress is saved by `progress.js` and published as `state.progress`.
- If either merged with different names, the contract builder adapts the call sites and records it; the behaviour below does not change.

## Problem

The core loop starts with "build the formula at the Formula Bench". The 3D client has no Bench, no notion of the player's age band, and nothing that turns a correct build into learning. I-027 gives each spell a stage, but nothing in play can move a spell out of Unknown. The engine that knows how to read and check a formula (crates/sim) is linked by I-026 but used only to load spell data.

The bench must work for a 6-year-old who cannot read yet (pictures, spoken prompts, big targets, mouse only), for a 9-year-old (atom tiles and a balance scale) and for a 13-year-old (typed formulas with states), and it must free the mouse from pointer lock and keep the game's keys from firing while the player builds.

**Goal:** a Formula Bench that opens from a lab station in the courtyard, from the `B` key and from a HUD button. At the player's band, the player builds any of the four spells they are old enough for. A correct build is judged by crates/sim and moves the spell from Unknown to Learning. A wrong build fizzles with a kind, band-appropriate reason and costs nothing.

## Bars moved

- **B9 (mastery works):** enabling, first half of "crafts and masters". After this item, a player can move a spell to Learning in play. B9 moves only with I-029 (worked casts) and I-030 (trial). Nothing is reported as moved at merge.
  - **Product limit, stated plainly:** nothing in play changes after a build until I-029 lands. The bench gates nothing (test 21); alone it yields a badge, a line and a tone. B9 and B5 get no credit from this item.
  - **Learning is not knowing.** A Sprout child copies a picture, so Learning proves no recall; mastery needs unaided builds (I-030). No text anywhere in this item calls a bench build "mastered", "knows" or "learned it by heart"; the done line says "You made …" / "… is right!" and the badge says "Learning".
  - **Panel prediction (informational only, first paired run after merge; not a bar claim):** the 6-year-old persona opens the bench and learns Water within 3 minutes of starting, mouse only; the 9-year-old learns Fire with the scale; the 13-year-old learns Frost by typing. No persona reports a control blocker at the bench.
- **B10 (crafting is correct):** first piece. Every bench verdict comes from one sim function (`bench::judge`) that checks against the spell's sourced formula, and every wrong attempt fizzles with a reason (tests 2-4). The full crafting script is I-031.
- **B4:** guard. The golden path and the panel's first minute are unchanged: the bench **never opens by itself**. Walking onto the pad only highlights the button; it opens only on `B` or a click of `#benchbtn` (test 13).
- **B6:** stays at 0 errors. Every new prompt and reason is registered (test 30); the lint learns the three prompt keys.
- **B1:** green, with a new golden section. **B2/B3:** guarded (test 31).

## Smallest version

### 1. Where it opens

The bench never opens on its own (critique M1). It opens only from `B` or `#benchbtn`.

- **The station.** A new Bevy-side module `crates/bevy-client/src/bench.rs` spawns, in its own `Startup` system (not in `world::build`), a landmark pad at `STATION = (−3.0, 14.5)`: a flat disc of radius `RADIUS = 1.2` m (y 0.02, a light lab-blue material with a white rim) and, hovering at y 2.2 above its centre, a flask icon (a sphere and a short cylinder neck, same material, mild emissive, **static**, no animation). Nothing is added to `Colliders` or to the chemistry targets: the player can stand on the pad and spells pass through the flask.
  - `atBench` is true while the player's feet are within `RADIUS` of `STATION` (horizontal distance). It is a landmark, not a trigger: while `atBench` is true, `bench.js` adds the class `near` to `#benchbtn` (a static attention ring, no pulse, no sound) and removes it when false. **Walking onto the pad never opens the bench**, never releases the lock and never pauses.
- **The `B` key** (by `e.code === 'KeyB'`, so AZERTY works) opens the bench from anywhere, locked or not, subject to the **one modal gate**.
- **One modal gate** (critique M5). `B` and `#benchbtn` do nothing while any element matching `[aria-modal="true"]` other than `#bench` is shown (not `hidden`, not `.hidden`, and with a non-zero client rect). Today that is `#age`; I-029's worked-cast overlay and I-030's trial **must** carry `aria-modal="true"` on their panel so the gate covers them (stated here as part of this contract; see Overlap).
- **A HUD button** `#benchbtn` (flask icon plus the key cap `B`, no other text, at least 64 × 64 CSS px, top-right corner, `aria-label="Formula Bench (B)"`). It is created by `bench.js`, is clickable whenever the pointer is free (point mode, lock refused, paused), and opens the bench. While locked it still shows (it reminds of the key) with the class `locked` and `pointer-events: none`.
  - **Attention ring (GS-23, critique S1).** The class `hint` (a static 4 px ring in the HUD accent colour; no animation, no sound, no count, so GS-28 stays met) is on `#benchbtn` while `state.casts >= 1` **and** no spell in `state.progress` is past `unknown`. It goes away for good once any spell reaches Learning. `near` (on the pad) uses the same ring. Both are read from `__gameState`/`__benchState` in `bench.js`'s own `requestAnimationFrame` poll (at most one class write per change).
- **Controls list (GS-24, critique S4).** `startBench()` appends one row `<tr><td>Formula Bench</td><td>B</td></tr>` to `#pause .card table` by DOM (after the "Read the last message again" row if found, else at the end). `index.html` and `hud.js` stay unedited.
- **Opening** (`B` or the button): remember `wasLocked = state.locked` and `wasPaused = state.paused`; release the pointer lock in the same handler (`document.exitPointerLock()`); call `__game.pause(true)`; set `inert` on `#stage`, `#hud` and `#pause`; show `#bench` above everything except `#age` (z-index 900; `#age` is 1000); focus its first control; push `open` (the band was already sent with `band`, section 2; if no band is known yet the chooser shows and `band` is pushed on the choice).
- **Closing** (the Close button, at least 64 px, or `Escape`): push `bench close`; hide `#bench`; remove `inert`; `__game.pause(wasPaused)`; if `wasLocked` and the close came from a click or a non-Escape key, call `__game.lock()` in the same handler (a gesture). If the lock is refused, hud's own "Click again to lock the mouse." note covers it. Focus goes back to the canvas.
- **Modal keys.** `startBench()` runs right after `startHud()`, before `init()`, and registers `keydown` and `keypress` listeners on `window` in the **capture** phase (**not** `keyup`: critique M4). While the bench is open they call `stopImmediatePropagation()` on every `keydown`/`keypress` (so neither `hud.js` nor winit sees a press: no reset on `R`, no spell change on `2`, no `P`). `keyup` always passes through, so a key held when the bench opened (W while walking, the usual case) is released in Bevy's `ButtonInput` when the player lets go; winit ignores a release of a key it never saw pressed, and `hud.js` has no `keyup` listener. They handle `Escape` (close), `Tab`/`Shift+Tab` and arrow keys (focus moves inside the bench only), and **do not** call `preventDefault` on printable keys, so typing still reaches the bench's text box (the same pattern as `age.js`). `Escape` typed while focus is in `#bench-input` closes the bench (it does not clear the box). While closed they only look for `KeyB` (subject to the modal gate). **Capture order with I-029:** the bench's listener is registered first (in `startBench()`, before `init()`) and swallows only while `#bench` is open; I-029's worked-overlay listener acts only while its panel is open; the modal gate means both are never open together.
- The game stays paused while the bench is open: no fire spreads, `gameTime` stops (GS-25, no time pressure).

### 2. The age band

- Resolution order, worked out at `startBench()`, on every age-picker click and each time the bench opens: the URL `?band=sprout|seedling|apprentice` (`adept` reads as `apprentice` until the Adept follow-up); else the band the player picked in the age picker on this page load (`bench.js` adds its own `click` listener to each `#age button[data-band]`; `age.js` is not edited); else the band chosen inside the bench earlier on this page load; else **ask**.
- **The game learns the band as soon as it is known** (critique M3). Whenever the band resolves (on `startBench()` from the URL, on an age-picker click, on a chooser click, on "Change age"), `bench.js` pushes the non-debug command `band <sprout|seedling|apprentice|adept>` on `__gameIn.bench`, even if the bench has never been opened. Rust stores it in `BenchState.band: Option<Tier>` (None until the first `band` command). I-029's worked casts, which happen in play with the bench closed, read this resource; they do not keep their own band. `open` takes no band argument: it uses `BenchState.band` (a `try` or `pick` while `band` is None is ignored with `warn!`).
- **Asking** shows the bench's band chooser: the heading "How old are you?" and three buttons that reuse the age picker's three pictures (`cloneNode(true)` of each `#age button[data-band] svg`), with "4 to 7", "8 to 10" and "11 and up" under them, each at least 160 × 160 CSS px, each with `aria-label` "4 to 7 years old" (and so on).
  - **Spoken (GS-21, critique S2).** When the chooser shows, "How old are you?" is spoken; when a chooser button gets focus or the pointer enters it, its label ("4 to 7", "8 to 10", "11 and up") is spoken, cancelling any line still playing. Same speech guard as the prompts (section 3).
- A small "Change age" button (at least 48 px) in the bench header reopens the chooser. The band is kept for this page load only, **never stored** (same reason as `age.js`). Nothing about it is sent anywhere. "Change age" is covered by test 16; it has no test of its own.
- **Teacher note (GS-29, critique S5).** The chooser asks on every page load while progress persists (I-027); that is deliberate for shared devices. A class link can fix the band with `?band=sprout|seedling|apprentice`; the planner adds this to the teacher-facing notes.

### 3. What the bench shows, per band

The bench has three views: **chooser** (above), **list** (the spells this band can learn), and **build** (one spell), plus a **done** panel after a correct build.

- **List.** One big card per spell with `band >= tier_min` (Sprout sees Water, Frost and CO₂; Seedling and Apprentice see all four). Each card: the spell icon (the hotbar's `.icon.fire|water|frost|foam` shapes), its name, and a stage badge with a shape as well as a word (Unknown: an empty circle and "New"; Learning: a half-filled circle and "Learning"; Practised and Mastered: their own shapes and words, for I-029/I-030). Cards are at least 120 × 120 px for Sprout and 88 × 88 px otherwise. Fire is **not shown** at Sprout (`tier_min` is `seedling`; I-026 left this choice to I-028).
- **Build, Sprout (4-7): picture cards.** The prompt (text plus a speaker button) and a **recipe card**: the atoms the spell needs, drawn as coloured balls with their letter inside (H white, O red, C dark grey, N blue; the letter means colour is never the only cue, GS-19), in formula order and grouped per element (water: two H, one O). Below it, a **build tray** and four **atom cards** (H, O, C, N), each at least 96 × 96 px. Tapping a card adds that atom to the tray; an Undo button (at least 96 px) removes the last; **Make** (at least 96 px, the largest button) sends the build. The recipe is a guided copy task: count and match.
  - **Speech guard** (used for every spoken line in this item): speak with `speechSynthesis` only if the API exists and sound is on in the saved settings (`alchemist.settings.v1`, read only, `try/catch`). **An absent settings key, an absent `sound` field or a parse failure reads as sound on** (matching `hud.js`'s default `sound: true`). If no voices exist (common on school Chromebooks), `speak` is a silent no-op; every view works from the picture and the on-screen text alone.
  - The prompt is spoken when the spell is picked; after each Make, the done or fizzle line is spoken. The speaker button replays the prompt at any time. Text is always on screen too (GS-20).
- **Build, Seedling (8-10): atom tiles and a balance scale.**
  - Single-substance spells (Water, Frost, CO₂): the recipe shows the **formula** with subscripts (`H₂O`). The build is four element tiles (H, O, C, N), each with a count that cycles 0 → 1 → 2 → 3 → 4 → 0 on tap (Enter/Space; ArrowUp/ArrowDown step it), at least 72 × 72 px, as in the 2D tray. The skill is reading subscripts as atom counts. Per element, the tile shows a tick shape when its count matches the recipe and nothing otherwise (no live red marks; it is judged on Make).
  - Fire: the recipe shows the reaction's molecules laid out as `CH₄ + O₂ → CO₂ + H₂O`, each with a coefficient tile that cycles 1 → 4 (start: all 1). Above it, a **balance scale**: the left pan lists atom counts before the arrow, the right pan after, per element, and the beam tilts toward the heavier side (by total atoms) and is level only when every element matches. The scale tilts with an angle step only (no flashing, a 200 ms transition; none under reduced motion).
  - The scale is display arithmetic in JS from the recipe's per-term atoms (see Contract, `recipe`). It is never the judge. Make sends the build; sim judges it. The pans and beam say "more atoms", never "heavier" or "mass": it counts atoms, not mass.
  - **Build order and fallback (critique section 2).** The web lane builds the scale **last**. If it is not done when the rest is green, Fire at Seedling ships with the coefficient tiles and, after Make, the per-element counts in the fizzle panel (the `unbalanced` reason plus `left/right` per element); test 18's scale assertions are then marked skipped with a note in the evidence, and the scale becomes a follow-up row. The leader accepted this fallback.
- **Build, Apprentice (11+): typed formulas.** The prompt names the spell and what it is ("Frost: write the formula of liquid nitrogen, with its state symbol."). A text box (font at least 24 px, `maxlength` 60, `autocomplete=off`, `spellcheck=false`) takes ASCII or Unicode (`N2(l)`, `N₂(l)`, `->` or `→`); a live preview under it shows the text with subscripts. Enter or **Make** sends it. No recipe is shown. Free typing: any formula can be sent, and the reason says what happened.
- **Done panel** after a correct build: the spell icon at 160 px with a single scale-up (from 0.6 to 1.0 over 300 ms, once; none under reduced motion), "You made Water!" (Sprout), "`H₂O` is right!" (Seedling: the target formula with its state stripped, since Seedling never sees state codes) or "`H₂O(l)` is right!" (Apprentice; with "Simplest form: …" when `simplest` is set, and, when `result.coef > 1` on a single substance, the line `A ${coef} in front would mean ${coef} molecules. One molecule is ${formula}.`, for example "A 2 in front would mean 2 molecules. One molecule is H₂O(l)." (a claim, [S7], row in 8)), the new stage badge ("Learning" with its shape) when this build moved it, a short rising tone (WebAudio, peak gain at most 0.15 like `hud.js`, only with sound on), and the done line spoken (speech guard). Buttons: "Build another" (back to the list) and "Back to the game" (closes). GS-17: big icon, words and sound together.
- **Fizzle panel** after a wrong build: the band's reason (table in Contract) next to the build, which stays as it was so the player can fix it. Sprout also gets a picture diff: per element, the balls they have against the balls the recipe wants, with a tick or a cross shape per element. Seedling gets the per-element counts. Apprentice gets per-element counts only from the **third** wrong try on the same spell in this bench session (the 2D Apprentice rule: help after three misses). Nothing is lost; the stage never goes down.

### 4. Judging: crates/sim `bench` module

New `crates/sim/src/bench.rs` (always compiled, std plus `serde`/`serde_json`, no Bevy), exported from `lib.rs`. It is the **only judge**; JS and client Rust never decide right or wrong.

`judge(spells: &[SpellDef], slot: u8, band: Tier, text: &str) -> Verdict`, with the def found by **its `slot` field** (`spells.iter().find(|d| d.slot == slot)`; slots are 1-4, never an index) and the target `T = def.formula.equation`, in this order (first match wins):

1. No def with that slot (0, 5, …) → `NotForBand`. `band < def.tier_min` → `NotForBand`.
2. `parse_equation(text)`: `ParseError::Empty` → `Empty`; `UnknownElement(s)` → `Element` with `bad = s`; any other error → `Parse` with `bad` = the `BadCharacter` char if any.
3. **Target is one substance** (`T` has no arrow and one term `t`; Water, Frost, CO₂):
   - Typed has an arrow or more than one term → `NotSingle`.
   - Typed atoms (one term `u`, coefficient ignored: amount is a crafting setting, I-031) differ from `t`'s: if they equal another spell's single-term atoms → `OtherSpell` with `other = that slot`; else `AtomsOff` with `diffs` = every element in either, `(el, have, want)`, sorted by element, only the ones that differ.
   - Same atoms. For `band >= Apprentice` and `t.state` set: `u.state` missing → `StateMissing` (`bad = t.formula`); different → `WrongState` (`bad = ` the target's state symbol). Sprout and Seedling never see state codes (the recipe gives the state).
   - Otherwise `Ok`. If the typed term's text (coefficient and whitespace removed) differs from `t.formula` with its state, or the typed coefficient is not 1 (for example `OH2(l)` or `2H2O(l)`), `simplest` = the canonical form with its state (`H2O(l)`). `coef` = the typed coefficient when it is greater than 1, else 0. Amount is not judged (a crafting setting, I-031); the done panel explains the coefficient instead (section 3).
   - At Sprout and Seedling, the state is not compared and `simplest`/`coef` stay empty (their UIs compose canonical text without states).
4. **Target is a reaction** (Fire):
   - No arrow, or no products → `NoProducts`.
   - `species_key` differs from `T`'s → `OtherSpell` if it equals another spell's (none today), else `WrongSpecies`.
   - Not balanced → `Unbalanced` with `diffs` = `(el, left, right)` for each element whose counts differ.
   - Balanced and `simplest()` equals `T.simplest()` → `Ok` (a multiple sets `simplest` to `T`'s text, `CH4 + 2O2 -> CO2 + 2H2O`). Balanced but not proportional → `WrongSpecies`. Typed states are not judged (the target has none).
5. `Adept` is judged exactly as `Apprentice` in this item.

`judge` is the one checker for the whole core loop: I-029's worked casts and I-030's trial call it through the same composed-text path (no separate `check`).

`recipe(def: &SpellDef) -> Vec<RecipeTerm>`: one entry per term of `T` in order: `side` (0 before the arrow, 1 after), `coef`, `formula` (ASCII), `atoms` as `(element, count)` in the order they appear in the formula, `state` (symbol or none).

`BenchView` (Serialize) and `BenchView::to_json()`: the one JSON shape `__benchState` carries (Contract). Building the JSON in sim keeps escaping in one tested place: typed text can contain `"` or `\` (for example in `bad`).

### 5. Client glue (`crates/bevy-client/src/bench.rs`, Bevy)

- `BenchState` resource (`pub`, so I-029 and I-030 read it): `open: bool`, `band: Option<Tier>` (set only by the `band` command; kept when the bench closes), `pick: u8` (0 = none), `result: Option<ResultView>`, `n: u32` (bumps per judged try), `last_json: String`. There is no `armed` state and no edge detection anywhere (critique M1, M2).
- One system `bench::run`, added with `.add_systems(Update, bench::run.after(control).before(publish))` (one new line in `main()`, outside the existing chained tuple). Each frame it:
  - reads `web.bench` (the new bench queue, see 6). Commands: `band <sprout|seedling|apprentice|adept>` (open or closed; sets `band`; while open it also clears `pick` and `result`); `open` (sets `open`, clears `pick` and `result`); `pick <slot>` (only while open, `band` set and the spell learnable at the band; sets `pick`, clears `result`; otherwise ignored); `try <slot> <text…>` (the rest of the line, words joined by one space; only while open with `band` set; runs `judge`, so an unlearnable slot gives `not_for_band`); `close`. Anything else, a malformed argument, or a command while closed (other than `band` and `open`), is ignored with `warn!` (never `error!`).
  - on an `Ok` verdict: if the spell's stage is Unknown, calls `progress.0.learn(slot − 1)` and marks `learned: true` in the result. Any other stage: nothing (repeat builds earn no XP here; XP for builds belongs to I-029/I-030).
  - computes `atBench` from `Player.pos` (published; JS only toggles the `near` class from it).
  - builds the `BenchView` (spell list with `stage` from `Progress`, prompts for the band from `SpellDef`, the recipe for the picked spell for Sprout and Seedling only, the last result) and publishes it with `bridge::publish_bench` **only when the JSON string changed**.
- It does not read or write `Game`, `Chem`, `Settings` or the cast queue (no `Game.epoch` read either). It does not cast and does not unpause.

### 6. Bridge (`bridge.rs`)

- `WebIn` gains `pub bench: Vec<String>`. `poll` reads `__gameIn.bench` (an array, if present) the same way it reads `cmds`, then replaces it with an empty array; the native `poll` clears it.
- New `pub fn publish_bench(json: &str)` writes `window.__benchState` (a no-op natively).
- `hud.js` is not edited: `bench.js` creates `window.__gameIn.bench = []` if it is missing (after `startHud()` made `__gameIn`).
- Why a separate queue: `control`'s command `match` and its filter are lines I-019, I-025 and I-027 edit. A separate queue leaves `control` untouched.

### 7. Content: prompts in `content/spells.json` (I-026 "Extending")

Three new optional string fields per spell: `prompt_sprout`, `prompt_seedling`, `prompt_apprentice`. Validation (new `SpellError::Prompt { spell, band }`): a spell has a prompt for exactly the bands from its `tier_min` up to Apprentice (Adept uses Apprentice's); 1-120 chars; I-026's `BadText` character rule. The exact strings:

| id | prompt_sprout | prompt_seedling | prompt_apprentice |
|---|---|---|---|
| fire | (none) | `Fire: methane burns with oxygen. Change the numbers until the scale is level.` | `Fire: write the balanced equation for methane burning in oxygen.` |
| water | `Make water! Copy the balls in the picture.` | `Water: build one water molecule from atom tiles.` | `Water: write the formula of liquid water, with its state symbol.` |
| frost | `Make liquid nitrogen! It is super cold. Copy the balls in the picture.` | `Frost: build one nitrogen molecule. Liquid nitrogen is very, very cold.` | `Frost: write the formula of liquid nitrogen, with its state symbol.` |
| co2 | `Make carbon dioxide! Copy the balls in the picture.` | `CO₂: build one carbon dioxide molecule.` | `CO₂: write the formula of carbon dioxide gas, with its state symbol.` |

The lint's `spells.json` key list gains the three keys.

### 8. Science register

- Claims (new `C-` rows, next free ids; `where` = `content/spells.json` or `crates/bevy-client/web/bench.js`):
  - the four "very cold"/"super cold" liquid-nitrogen prompts: correct, [S1] (Tboil 77.34 K);
  - the two methane prompts: correct, [S7] [S4];
  - the Seedling unbalanced reason `The scale is not level. Atoms are never made or lost, so both sides need the same atoms.`: correct, [S7] (balancing equations; conservation of atoms);
  - **state claims in the Apprentice prompts** (each enforced by the judge): "liquid water" → `H2O(l)` at room conditions, [S2]; "liquid nitrogen" → `N2(l)` at or below its normal boiling point (77.34 K, 1 atm), [S1]; "carbon dioxide gas" → `CO2(g)` at room temperature and 1 atm, [S5] (an extinguisher's CO₂ leaves as gas and snow; I-026 chose `CO2(g)` and this item does not reopen it);
  - the coefficient line `A ${coef} in front would mean ${coef} molecules. One molecule is ${formula}.`: correct, [S7] (a coefficient counts molecules; subscripts are atoms in one molecule).
- Not-claims rows for every other prompt and reason string in Contract (game instructions or counts; flagged by sentence shape). The web lane copies the exact strings.
- The not-claims note for the scale says: it compares **atom counts**, not mass; no text may say "heavier", "weight" or "mass" about it.
- Not a claim and not text: the CPK-style atom colours (a drawing convention). The bench draws **no molecular geometry** (atoms are shown as a recipe of loose balls), so it makes no shape claim.

## Acceptance tests

Each test says how it fails today or on the contract stub (whose `judge` returns `Verdict { ok: false, code: Parse, .. }` for every input and whose `bench.js` builds no UI).

**Sim unit tests** in `crates/sim/src/bench.rs`, run with `cargo test -p realgame-sim --no-default-features --locked bench` (and again with default features). They fail on the stub.

1. **Correct builds per band are `Ok`** (against the shipped `SpellBook::load`):
   - Sprout: water `H2O`, frost `N2`, co2 `CO2` (the texts `bench.js` composes; see test 8).
   - Seedling: the same three, plus fire `CH4 + 2O2 -> CO2 + 2H2O`.
   - Apprentice: `H2O(l)`, `H₂O(l)`, `N2(l)`, `N₂(l)`, `CO2(g)`, `CH4 + 2O2 -> CO2 + 2H2O`, `CH₄ + 2O₂ → CO₂ + 2H₂O`, `CH4(g) + 2O2(g) -> CO2(g) + 2H2O(l)` (each with `simplest == ""` and `coef == 0` where the text is canonical); `2CH4 + 4O2 -> 2CO2 + 4H2O` is `Ok` with `simplest == "CH4 + 2O2 -> CO2 + 2H2O"`; `OH2(l)` is `Ok` with `simplest == "H2O(l)"`, `coef == 0`; `2H2O(l)` is `Ok` with `simplest == "H2O(l)"` and `coef == 2`.
   - Adept gives the same verdicts as Apprentice for every case in this test.
2. **Wrong builds fizzle with the specific code and details** (each asserts the whole `Verdict`):
   - Sprout water `HO` → `AtomsOff`, diffs `[("H", 1, 2)]`; `H2O2` → `[("O", 2, 1)]`; `H2OC` → `[("C", 1, 0)]`; `""` → `Empty`.
   - Sprout frost `H2O` → `OtherSpell`, `other == 2`; Seedling co2 `N2` → `OtherSpell`, `other == 3`.
   - Seedling fire `CH4 + O2 -> CO2 + H2O` → `Unbalanced`, diffs `[("H", 4, 2), ("O", 2, 3)]`.
   - Apprentice frost `N2` → `StateMissing` (`bad == "N2"`); `N2(g)` → `WrongState` (`bad == "l"`); water `H2O(g)` → `WrongState`; `Xx2` → `Element` (`bad == "Xx"`); `H2 ++ O2` → `Parse`; fire `CH4 + 2O2` → `NoProducts`; fire `2H2 + O2 -> 2H2O` → `WrongSpecies`; water `H2O(l) -> H2O(g)` → `NotSingle`.
   - Sprout or Seedling `N2(g)` for frost → `Ok` (states are given by the recipe below Apprentice).
   - Sprout fire (any text) → `NotForBand`; slot 0 and slot 5 → `NotForBand` (no panic: the def is looked up by its `slot` field).
3. **It agrees with the rules engine.** For every `Ok` on Fire, the typed equation `is_balanced()` and has `T`'s `species_key`. For every `Ok` on a single-substance spell, `side_atoms` of the typed term equals the target's. Checked over every `Ok` produced in tests 1 and 4.
3b. **Hand-written chemistry table (independent of the parser).** In `bench.rs` tests, a literal table of 12 inputs that must be `Ok` and 12 that must not, each with a one-line chemical reason in a comment, judged at Apprentice. The not-`Ok` rows include at least: water `H2O2(l)` (hydrogen peroxide, not water), water `HO(l)` (a radical, not water), co2 `NO2(g)` (nitrogen dioxide), co2 `CO(g)` (carbon monoxide), frost `N(l)` (one atom, not the molecule), fire `CH4 + O2 -> CO2 + 2H2O` (O unbalanced), fire `2H2 + O2 -> 2H2O` (hydrogen burning, not methane), fire `CH4 + 2O2 -> CO + 2H2O` (wrong product; unbalanced too), fire `CH4 -> C + 2H2` (decomposition, no oxygen). The `Ok` rows include at least the eight canonical/Unicode forms of test 1. This table is written by hand from [S7]/[S4], not generated with `parse_equation`.
4. **Never panics, never a false `Ok`.** A deterministic LCG builds 20 000 strings per seed over 20 seeds from the alphabet `HONCXx()lgs2 4+->→₂"\` (length 0-90), and judges each against every slot at every band. No panic; every `Ok` satisfies test 3's rule; every result serializes with `to_json` and parses back with `serde_json`.
5. **Recipe.** water → `[{side 0, coef 1, "H2O", [("H",2),("O",1)], Some("l")}]`; frost → `[… "N2", [("N",2)], Some("l")]`; fire → 4 terms, sides `[0,0,1,1]`, coefs `[1,2,1,2]`, formulas `[CH4, O2, CO2, H2O]`, `CH4` atoms `[("C",1),("H",4)]`, no states.
6. **View JSON.** `BenchView::closed(true, None).to_json()` is exactly `{"v":1,"atBench":true,"open":false}`, and `BenchView::closed(false, Some(Tier::Seedling)).to_json()` is exactly `{"v":1,"atBench":false,"open":false,"band":"seedling"}`. Every `SpellCard` carries `formula` equal to its def's ASCII formula text (`H2O(l)`, `CH4 + 2O2 -> CO2 + 2H2O`). An open view with a result whose `bad` is `"\` round-trips through `serde_json` with the same value, and its top-level keys come in the Contract's order.
7. **Prompt validation** (in `spellbook.rs`, I-026's style; each asserts the variant): removing `prompt_seedling` from water → `Prompt { spell: "water", band: Seedling }`; adding `prompt_sprout` to fire → `Prompt { spell: "fire", band: Sprout }`; a prompt with `"` → `BadText`; 121 chars → `BadText`. **Parity:** the 11 shipped prompts equal the table in 7 byte for byte.

**Node unit tests** in `test/bench.test.ts`, importing the pure exports of `crates/bevy-client/web/bench.js` (run by `npm test`). They fail on the stub (exports throw or return null).

8. `composeSprout(placed, recipe)`: water recipe with `['H','O','H']` → `H2O`; frost with `['N','N']` → `N2`; co2 with `['O','C','O']` → `CO2`; water with `['H','O','H','C']` → `H2OC` (recipe elements first, in recipe order, then others alphabetically, count 1 written as nothing); `[]` → `''`.
9. `composeSeedling(recipe, counts)` for single substances (`{H:2,O:1}` → `H2O`; all zero → `''`) and `composeEquation(recipe, coefs)` for fire `[1,2,1,2]` → `CH4 + 2O2 -> CO2 + 2H2O`. `scaleOf(recipe, coefs)`: `[1,2,1,2]` → `{left:{C:1,H:4,O:4}, right:{C:1,H:4,O:4}, level:true, tilt:0}`; `[1,1,1,1]` → `level:false`, `tilt` = sign(right total − left total) (`+1`: right pan heavier).
10. `reasonFor(band, result, names, tries)` (with `names = { [slot]: { name, formula } }` built from the published `spells` list) returns exactly the Contract table's string for every `code × band` (Sprout, Seedling, Apprentice; **all 13 codes**, including `not_for_band` and `ok`), with `${…}` filled; the Seedling `ok` line has no state symbol (`H₂O is right! …`); Apprentice `AtomsOff` names counts only when `tries >= 3`. No reason contains `wrong`, `bad`, `fail`, `error`, `incorrect` or `!` after a fizzle (kind tone), and every Sprout reason has at most 12 words.
11. `bandFrom(search, picked, chosen)`: `?band=sprout` → `sprout`; `?band=adept` → `apprentice`; `?band=toddler` with nothing else → `null`; no URL band and `picked = 'seedling'` → `seedling`; the URL beats `picked`, which beats `chosen`.
11b. **`mountBuilder`** (critique M3, contract gap 5), with a minimal DOM shim (or `linkedom` if already a dev dependency; no new dependency otherwise, in which case this check moves to golden as 28c). Mounted into a fresh `div` with `{ band: 'sprout', spell: water card, recipe: water recipe, prompt, onMake }`: it renders the build view only (no `.bench-spell`, no `.bench-band`, no `.bench-close`, does not touch `document` focus or `inert`); every placeable piece carries `data-token` (`H`, `O`, …; for Fire coefficients `data-token="coef:<term>"`); clicking H, H, O then `.bench-make` calls `onMake('H2O')` exactly once; it returns `{ destroy }`, and `destroy()` empties the `div`. Mounted with `recipe: null` at Seedling (I-030's hidden-help case) and with `band: 'apprentice'`, it renders without throwing (tiles without a recipe card; a text box).

**Golden e2e checks** in a new function `formulaBench({ page, st, cmd, until, frames, sleep, base, browser, errors })` at the end of `golden`, called on its own line after I-027's `spellProgress(...)` call and before `no console errors`. Each starts with `localStorage.removeItem('rg.progress.v1')`, a reload with `QUERY=?low` plus `debug` (and `&band=…` where stated), and a fresh pointer-lock click where a test says "locked". They fail today because `window.__benchState`, `#bench` and `#benchbtn` do not exist; on the stub they fail at test 13.

12. **Closed shape.** With no URL band and no picker, `JSON.parse(window.__benchState)` is `{v:1, atBench:false, open:false}` after boot.
13. **The station is a landmark and never opens the bench by itself** (critique M1).
    - From the start pose, `atan2` from `state.pos`/`state.yaw` to `STATION` gives a bearing whose absolute value is below `hfovOf(state.fov) / 2` (on screen).
    - Locked. `tp -3 14.5`. For 30 frames: `__benchState.atBench === true`, `#benchbtn` has the class `near`, `#bench` stays hidden, `document.pointerLockElement` is still the canvas and `state.paused === false`.
    - `state.pos` after 10 more frames is within 0.01 m of (−3, 14.5) (no collider pushed it).
    - `tp 0 18`: within 10 frames `atBench === false` and `near` is gone.
14. **The `B` key, locked.** Locked, `page.keyboard.press('KeyB')`. Within 10 frames: `#bench` open, lock released, `paused`, `document.elementFromPoint(VW/2, VH/2)` inside `#bench`, and `#stage`, `#hud`, `#pause` have `inert`. `Escape`: closed, `paused === false` within 10 frames, the canvas has focus.
    - **Held key (critique M4).** Locked, `keyboard.down('KeyW')`, 5 frames, `press('KeyB')` (bench opens), `keyboard.up('KeyW')`, close with the Close button, re-lock. Record `state.pos`; 30 frames later it is unchanged (within 0.01 m).
15. **Opened from the pause card, closing returns to it.** `__game.pause(true)`, press `KeyB`, close with Escape: `state.paused` is still true and `#pause` is visible.
16. **Band chooser.** With no `band` in the URL, the first open shows three `[data-band]` buttons, each at least 160 × 160 px, each holding an `svg`. Clicking `seedling` shows the list with 4 cards. Close and reopen: no chooser (kept for this page load). "Change age" shows it again. After a reload, it asks again (not stored: `localStorage` and `sessionStorage` hold no key containing `band`).
17. **A 6-year-old learns Water with the mouse only** (`&band=sprout`; `page.mouse` clicks at element centres only; no keyboard).
    - Before the bench opens, an init script replaces `window.speechSynthesis` with a stub that records `speak(u)` calls (`u.text`).
    - Click `#benchbtn` (unlocked, point mode not needed: the pointer is free after load). The list shows exactly Water, Frost and CO₂ (no Fire), each card at least 120 × 120 px.
    - Click Water. The stub has recorded `Make water! Copy the balls in the picture.` The recipe shows 2 `H` balls and 1 `O` ball.
    - Click atom cards H, O (each at least 96 × 96 px), then Make. Within 10 frames: `__benchState.result.ok === false`, `code === 'atoms_off'`; the fizzle panel shows the Sprout reason and a diff row for H with a cross shape; `state.progress[1].st === 'unknown'`.
    - Click H, then Make. Within 10 frames: `result.ok`, `result.learned`, the done panel shows `You made Water!` and a "Learning" badge; `state.progress[1].st === 'learning'` and `xp === 10`. The stub recorded the done line.
    - The stub also recorded the fizzle line `Look again. Count the balls in the picture.` after the first Make (not only the prompt).
    - With **no** `alchemist.settings.v1` key, picking Water records the prompt (absent = sound on). With `sound:false` saved (reload), picking Water records **no** `speak` call; the prompt text is still on screen.
18. **A 9-year-old learns Fire on the scale** (`&band=seedling`, keyboard and mouse both allowed).
    - Pick Fire. The scale's pans read `C 1, H 4, O 2` and `C 1, H 2, O 3`; the beam is not level (`[data-level="false"]`).
    - Make → `result.code === 'unbalanced'` and the reason names the rule (Contract string); fire stays `unknown`.
    - Tap the O₂ coefficient once (1 → 2) and the H₂O coefficient once: within 2 frames of the last tap the beam is `[data-level="true"]` and both pans read `C 1, H 4, O 4`. Make → `ok`, fire `learning`.
    - Water at Seedling: tap H twice and O once, Make → `ok`.
19. **A 13-year-old learns Frost by typing** (`&band=apprentice`).
    - Pick Frost. No recipe element is in the DOM. Focus is in the text box.
    - Type `N2`, Enter → `code === 'state_missing'`, reason shown. Type to make it `N2(g)`, Enter → `wrong_state`. Then `N₂(l)` via `page.keyboard.insertText`, Enter → `ok`, frost `learning`.
    - **Keys do not leak.** Before typing, read `state.spell`, `state.captionId` and `state.gameTime`. Type `Br2 R 3 P` into the box. `state.spell`, `state.captionId` (a reset bumps it) and `state.gameTime` are unchanged, the bench is still open, and `#pause` did not toggle.
    - **Escape in the text box.** With focus in `#bench-input` holding `N2`, `Escape` closes the bench (`#bench` hidden, `paused` restored within 10 frames); it does not just clear the box.
    - Three wrong single-substance tries on Water (`HO(l)` three times): the first two reasons have no digits; the third includes `H` and `2`.
20. **Keyboard only.** `&band=sprout`, `KeyB` to open, then only `Tab`, `Shift+Tab`, arrow keys, `Enter`, `Space` and `Escape`: reach Water, add H, H, O, Make → `ok`. Focus never leaves `#bench` while it is open (after each key, `document.activeElement.closest('#bench')` is not null). Escape closes and focus is on the canvas.
21. **No farming, no gating.** After test 17's Water is `learning`, five more correct Water builds each give `result.ok`, `learned === false`, and `xp` stays 10. With frost `unknown`, `select 3` and `cast` still cast Frost after closing (`casts` +1): the bench gates nothing in this item. (Deliberately kept true under I-029, which casts Unknown spells through a guided build; I-029 must not delete this check.)
22. **The game stays frozen.** With the bench open for 2 s of wall time: `state.gameTime`, `state.tick` and `state.casts` are unchanged; a `wheel` event and a click inside the bench change neither `spell` nor `casts`.
23. **Progress survives a reload** (I-027's save). After learning Water, reload: the list badge for Water reads "Learning".
24. **Sizes and text (GS-18, GS-22).** At 1280 × 720 and at 1366 × 768, for every band's list and build views: every bench `button` and focusable control is at least 48 × 48 CSS px (Sprout atom cards and Make at least 96, Sprout spell cards at least 120), at least one text element is found per view (an empty view fails), every text element's computed `font-size` is at least 18 px (Sprout prompt at least 24 px), and the bench panel fits the viewport or scrolls inside itself (no page scroll). With text size 2 (`alchemist.settings.v1` `text:2`), font sizes double.
25. **Contrast (GS-18).** Every bench text element's colour against the background of its **nearest ancestor with an opaque computed background** (cards, badges and buttons included) has a contrast ratio of at least 4.5:1.
26. **Reduced motion and no flashing (GS-6).** Positive control first: without reduced motion, the done icon's computed `animation-name` is not `none` and its `animation-iteration-count` is `1`, and every other bench animation also has count 1. With `reducedMotion: 'reduce'`, the done icon and the scale beam have `animation-name: none` and `transition-duration: 0s`. `#benchbtn.hint` and `.near` have `animation-name: none` in both modes.
27. **One modal gate** (critique M5). Unhide `#age` (`hidden = false`) and press `KeyB`: the bench stays closed. Hide it again: `KeyB` opens it; close. Then append to `body` a visible `<div aria-modal="true" role="dialog">` (standing in for I-029's overlay), press `KeyB` and click `#benchbtn` (unlocked): the bench stays closed. Remove it: `KeyB` opens it.
28. **The bench button.** Unlocked: `#benchbtn` is visible, at least 64 × 64, has `aria-label` containing `Formula Bench`, and clicking it opens the bench. Locked: it is still visible, has the class `locked`, and its computed `pointer-events` is `none` (unlocked: `auto`).
28a. **Attention ring and controls row (GS-23, GS-24).** After boot, `#benchbtn` has no `hint`. After one `cast` (`casts >= 1`), within 10 frames it has `hint`. Learn Water at the bench and close: `hint` is gone and does not return after another cast. The pause card's controls table has a row whose cells read `Formula Bench` and `B`.
28b. **The band reaches Rust with the bench closed** (critique M3). Fresh load, no URL band, bench never opened: `__bench.cmd('band seedling')`; within 10 frames `JSON.parse(__benchState)` is `{v:1, atBench:false, open:false, band:'seedling'}` (the closed view carries `band` once it is set; see Contract). With `&band=apprentice` in the URL, it is `apprentice` within 10 frames of boot without any open. At Sprout, `pick 1` is ignored (`pick` stays unset) and `try 1 CH4 + 2O2 -> CO2 + 2H2O` returns `not_for_band`.
29. **Picker band reaches the bench** (`tests/e2e/analytics-e2e.mjs`, append only, after I-022's checks): in the built site (analytics on), pick "8 to 10" in the age picker, press `KeyB`: no chooser, and the list shows 4 cards (Seedling). DNT on (no picker): the chooser shows.

**Lint, perf and hygiene.**

30. **Science lint.** `npm test` is green with the new rows. Before the rows are added (prompts and reasons in place), the lint reports each new string as unsourced: the expected red step, logged by the web lane.
31. **B2/B3 guard (tester).** Base = the branch base, built the same way under `<scratchpad>/I-028/`. `ONLY=low@4x RUNS=3 node tests/e2e/perf.mjs`, 3 interleaved pairs (or I-024's `perf-pairs.mjs` if merged): median CPU frame p50 ≤ base + 1.0 ms with the bench closed. Brotli of `pkg-out` grows by at most 40 KB. Record both in `docs/loop/items/I-028/evidence/perf.md`.
32. **Golden green.** Every existing check passes 3/3 with `QUERY=?low` in isolation (I-017), with **no** edit to any existing check, and the state key-order check unchanged (this item adds no `__gameState` key). That is the full count at the base the branch lands on.
33. **Lane hygiene.** `git diff --name-only origin/main...` lists only paths in Lanes. `git diff -U0` on `main.rs` touches only the `mod bench;` line and the two `add_systems` lines (`Update` and `Startup`): three added lines in all. `hud.js`, `index.html`, `age.js`, `analytics.js`, `progress.js`, `world.rs`, `chem.rs`, `cues.rs`, `fx.rs`, `feel.rs`, `player.rs`, both `Cargo.toml`s and both `Cargo.lock`s are untouched. `golden` has 0 deleted lines.

## GS walk (screens, controls and effects this item touches)

| GS | P | status |
|----|---|--------|
| 6 | P0 | met: one 300 ms scale-up and a 200 ms beam tilt, iteration count 1, none under reduced motion; the station is static (test 26) |
| 7 | P1 | met: the game is paused while the bench is open; closing restores the previous pause state (tests 15, 22) |
| 9 | P1 | met: `KeyB` by `code`; no hold or chord; Seedling counts tap-cycle as in the 2D tray |
| 11 | P0 | met: the bench is reachable by mouse only (the `#benchbtn` button), and Sprout builds by clicking big cards; no drag anywhere (test 17) |
| 13 | P1 | met: the bench is fully keyboard-operable and focus is trapped inside while open (test 20); the pointer lock is released on open; Escape closes, also from the text box (test 19) |
| 14 | P1 | met for reading: sound and text size come from the saved settings; the band is deliberately not saved (privacy, as `age.js`) |
| 15 | P0 | met: correct and fizzle differ in shape (tick or cross, big icon), text and sound |
| 17 | P0 | met: big icon, a spoken and written line, and a tone on success (test 17) |
| 18 | P0 | met: at least 18 px (Sprout prompt 24 px), scales with `--ui`, 4.5:1 contrast (tests 24, 25) |
| 19 | P0 | met: atoms carry their letter as well as colour; stages have shapes as well as words; the scale's level state is a beam angle plus a tick |
| 20 | P1 | met: spoken prompts are always also on screen; sound respects the saved setting |
| 21 | P0 | met: Sprout prompts are under 12 words and spoken; the chooser question and each age button's label are spoken; the recipe is a picture; no auto-dismiss (tests 16, 17) |
| 22 | P1 | met: at least 48 px everywhere; 96 to 160 px for Sprout (test 24) |
| 23 | P0 | met for discoverability: the bench never interrupts the first minute (no walk-on open, test 13); after the first cast and until the first spell is Learning, `#benchbtn` carries one static attention ring, and the pad lights it too (test 28a); building is learned by doing in a safe, paused space. A full guided walk-through stays a follow-up |
| 25 | P1 | met: no timer at the bench; the world is frozen |
| 26 | P1 | met: "Change age" at any time keeps all progress; labels are ages, never "easy" or "baby" |
| 27 | P0 | met: neutral tone; the 13-year-old's view has no pictures of balls and no exclamation-heavy copy |
| 28 | P0 | met: no rewards for returning, no streaks, no countdowns; the attention ring is static, one-time, silent and uncounted (test 26, 28a) |
| 24 | P0 | met: `B` is added to the pause card's controls list by `bench.js` (test 28a); the button shows the key cap |
| 30 | P0 | met: building the real formula is the mechanic; the bench is crafting, not a quiz screen that interrupts play (it opens only on `B` or a click) |
| 31 | P1 | met: minds-on (count, balance, write); one learning goal per view |
| 32 | P1 | met: Sprout never sees a formula; Seedling sees subscripts as counts next to tiles; the scale makes conservation concrete |
| 1-5, 8, 10, 12, 16, 29 | — | unchanged (no camera, aim, cast, objective or settings change) |

## Out of scope

- **Adept** (16+): equations with conditions (temperature, pressure, a catalyst over the arrow, ΔH sign). Follow-up line for the backlog: "I-0xx Phase 2, Adept bench: equations with conditions, reusing `bench::judge`, with sourced condition data". Until then `?band=adept` reads as Apprentice and the chooser offers three bands.
- **Worked casts, instant casts, any cast gating** (I-029), the mastery trial and "predict what happens" (I-030), and stage shown on the hotbar (I-029). The bench only moves Unknown → Learning.
- **XP for repeat builds**, unaided builds and the 2-day rule (I-030 calls I-027's `unaided_build`).
- **Free crafting** of new spells, settings (amount, form, temperature, trigger), energy (I-031). Apprentice typing is free, but only the four spells can be learned.
- **Molecular geometry pictures** (bent water, linear CO₂): a later art item with its own register rows.
- **A guided first-bench walk-through** (GS-23 tutorial) and an idle hint pointing at the station. (The static attention ring on `#benchbtn` is in scope; see section 1.)
- **A "Start over on this computer" button.** Not built here, but a **precondition for any classroom link**, not a note: shared devices will show one child's Learning badges to the next. The planner files it in the backlog **before this item's PR is opened**, and this item's backlog row is marked "do not ship to teachers before <that id>".
- **Analytics** for bench use: none. I-027's held `spell_learned` event (ACTION-1) will fire from this item's `learn` call once approved; nothing here sends anything.
- `hud.js`, `index.html`, `age.js`, `analytics.js`, `progress.js`, the panel harness (the panel brief gains one line about the bench, a follow-up), and `crates/bevy-client/README.md` (one controls row for `B`, a docs follow-up; the in-game controls list is covered in this item by DOM, section 1).

## Contract

Lands first, on a base that has I-026 and I-027 merged, as one commit by the contract builder. It has no behaviour of its own; lanes code against it.

### Files the stub creates or changes

- `crates/sim/src/bench.rs`: every public type and signature below; `judge` returns `Verdict { ok: false, code: Code::Parse, ..Default }`; `recipe` returns `vec![]`; `BenchView::to_json` is real (it is serde). `crates/sim/src/lib.rs`: `pub mod bench;` (always compiled).
- `crates/sim/src/spellbook.rs`: the three optional `prompt_*` fields on `SpellDef` (`#[serde(default)] Option<String>`), the `SpellError::Prompt { spell: String, band: Tier }` variant, and no rule yet. `content/spells.json`: the 11 prompts (the data is part of the contract).
- `crates/bevy-client/src/bridge.rs`: `WebIn.bench`, `poll` reading `__gameIn.bench`, `publish_bench` (complete; it is small).
- `crates/bevy-client/src/bench.rs`: `BenchState`, `STATION`, `RADIUS`, the `Startup` spawn system (empty body) and `run` (publishes `BenchView::closed(false, None)` once). `crates/bevy-client/src/main.rs`: `mod bench;` in the `mod` list and `.add_systems(Update, bench::run.after(control).before(publish))` plus `.add_systems(Startup, bench::spawn_station)` as new lines after the existing `add_systems` calls.
- `crates/bevy-client/web/bench.js`: the exports below with `null`-returning bodies and a `startBench()` that only creates `window.__gameIn.bench`. `crates/bevy-client/web/boot.js`: `import { startBench } from './bench.js';` and `try { startBench(); } catch {}` after `startHud();` (after I-027's `startProgress` lines). `crates/bevy-client/build-web.sh:21`: add `web/bench.js` to the `cp` list.
- `tests/e2e/golden.mjs`: an empty `async function formulaBench(…) {}` at the end and its call line.

### `crates/sim/src/bench.rs`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Code { Ok, #[default] Parse, Empty, Element, NotSingle, AtomsOff, OtherSpell,
                StateMissing, WrongState, NoProducts, WrongSpecies, Unbalanced, NotForBand }

#[derive(Clone, Debug, PartialEq, Default, Serialize)]
pub struct Verdict {
    pub ok: bool,
    pub code: Code,
    /// AtomsOff: (el, have, want). Unbalanced: (el, left, right). Sorted by element.
    pub diffs: Vec<(String, u32, u32)>,
    /// OtherSpell: the slot whose formula this builds; else 0.
    pub other: u8,
    /// Element: the symbol; Parse: the bad character (or ""); StateMissing: the
    /// formula; WrongState: the target state symbol. Player-typed: escaped by serde.
    pub bad: String,
    /// Ok on a multiple or a non-canonical formula: the canonical text; else "".
    pub simplest: String,
    /// Ok on a single substance typed with a coefficient > 1 (Apprentice+): that coefficient; else 0.
    pub coef: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RecipeTerm { pub side: u8, pub coef: u32, pub formula: String,
                        pub atoms: Vec<(String, u32)>, pub state: Option<String> }

pub fn judge(spells: &[SpellDef], slot: u8, band: Tier, text: &str) -> Verdict;
pub fn recipe(def: &SpellDef) -> Vec<RecipeTerm>;
pub fn learnable(def: &SpellDef, band: Tier) -> bool;   // band >= def.tier_min
pub fn prompt(def: &SpellDef, band: Tier) -> Option<&str>; // Adept -> apprentice prompt

#[derive(Serialize)]
pub struct SpellCard { pub slot: u8, pub id: String, pub name: String, pub formula: String /* ASCII, from the def, with state */, pub stage: String, pub prompt: String }
#[derive(Serialize)]
pub struct ResultView { pub n: u32, pub slot: u8, #[serde(flatten)] pub verdict: Verdict, pub learned: bool, pub stage: String }
#[derive(Serialize)]
pub struct BenchView {
    pub v: u8,                       // 1
    #[serde(rename = "atBench")] pub at_bench: bool,
    pub open: bool,
    #[serde(skip_serializing_if = "Option::is_none")] pub band: Option<Tier>,
    #[serde(skip_serializing_if = "Option::is_none")] pub spells: Option<Vec<SpellCard>>, // learnable at band, slot order
    #[serde(skip_serializing_if = "Option::is_none")] pub pick: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub recipe: Option<Vec<RecipeTerm>>, // Sprout and Seedling only
    #[serde(skip_serializing_if = "Option::is_none")] pub result: Option<ResultView>,
}
impl BenchView { pub fn closed(at_bench: bool, band: Option<Tier>) -> BenchView; pub fn to_json(&self) -> String; }
```

Key order in the JSON is the struct order. `code` serializes as snake_case (`atoms_off`, `state_missing`, …).

### Commands (`window.__gameIn.bench`, one string each; never debug-gated)

`band <sprout|seedling|apprentice|adept>`, `open`, `pick <slot>`, `try <slot> <text…>`, `close`. `bench.js` pushes them; tests may push them through `window.__bench.cmd(s)`. `band` is sent whenever the band resolves, open or closed (section 2); I-029 and I-030 read `BenchState.band` and send no band command of their own.

### `window.__benchState` (written by `bridge::publish_bench`, only on change)

Closed: `{"v":1,"atBench":<bool>,"open":false}`, plus `"band":"<tier>"` once a `band` command has set it. Open: the full `BenchView`. `result.n` bumps on every judged `try`. `result.stage` is the spell's stage after the try.

### `crates/bevy-client/web/bench.js` exports

`startBench()`; pure: `composeSprout(placed, recipe)`, `composeSeedling(recipe, counts)`, `composeEquation(recipe, coefs)`, `scaleOf(recipe, coefs)`, `reasonFor(band, result, names, tries)`, `bandFrom(search, picked, chosen)`; and, for I-029's worked-cast overlay and I-030's trial, `mountBuilder(root, { band, spell, recipe, prompt, onMake }) -> { destroy }`: the build view on its own, without the list, chooser, modal handling, focus trap or `inert`; `spell` is a `SpellCard`; `recipe` may be `null` (no recipe card: I-030's hidden help); `onMake(text)` gets the composed text (the same text the bench sends in `try`); every placeable piece carries `data-token` (an element symbol, or `coef:<term>` for Fire's coefficient tiles) in addition to its `data-el`/`data-term`. Its options are the names I-029 must use (I-029's `guided`, `compact`, `seed` and handle methods are not provided; I-029 adds what it needs on top in its own PR). Test 11b covers it.

Test hook: `window.__bench = { state(), ui(), open(), close(), cmd(s) }`, where `ui()` gives `{ open, band, view: 'chooser'|'list'|'build'|'done'|'fizzle', pick }`. **Under `?debug` only**, `window.__bench.fake(view)` renders a given `BenchView` object as if Rust had published it (it does not write `__benchState`), so the web lane can build and screenshot the done and fizzle panels against the stub before the sim and client lanes merge. It is absent without `debug`.

### DOM ids and classes (web lane; for tests, I-029, I-030 and the panel; keep these names exactly)

`#bench` (with `role="dialog"`, `aria-modal="true"`), `#benchbtn` (with `.locked`, `.near`, `.hint`), `[data-token]` on every placeable piece, `.bench-band[data-band]`, `.bench-spell[data-slot]` with `.bench-stage[data-stage]`, `.bench-atom[data-el]`, `.bench-tile[data-el]`, `.bench-coef[data-term]`, `.bench-scale[data-level]`, `.bench-make`, `.bench-undo`, `.bench-say`, `#bench-input`, `.bench-reason`, `.bench-diff[data-el][data-ok]`, `.bench-done`, `.bench-close`. Styles are appended to `style.css` (CSP `style-src 'self'`).

### Reason strings (exact; `reasonFor` returns these)

`names` is `{ [slot]: { name, formula } }` built from the published `spells` list. `${name}` = `names[result.slot].name`; `${other}` = `names[result.other].name`; `${formula}` = `names[result.slot].formula` shown with Unicode subscripts and arrow, **with state symbols stripped at Seedling**; `${el}`, `${have}`, `${want}`, `${left}`, `${right}` come from `result.diffs`; `${stateWord}` from `result.bad` (`l` → `a liquid`, `g` → `a gas`, `s` → `a solid`, `aq` → `a solution in water`).

| code | Sprout | Seedling | Apprentice |
|---|---|---|---|
| ok | `You made ${name}!` | `${formula} is right! You are learning ${name}.` (no state) | `${formula} is right! You are learning ${name}.` (with state) |
| empty | `Tap the balls to build it.` | `Tap the tiles to build it.` | `Write a formula first.` |
| atoms_off | `Look again. Count the balls in the picture.` | `Count again: ${el} should be ${want}. You have ${have}.` (one line per diff) | before 3 tries: `Not yet. Which atoms does ${name} have, and how many?`; from 3: as Seedling |
| other_spell | `That makes ${other}. This one is ${name}.` | same | same |
| unbalanced | (n/a) | `The scale is not level. Atoms are never made or lost, so both sides need the same atoms.` | `Not balanced yet: ${el}: ${left} before, ${right} after.` (one line per diff) |
| state_missing | (n/a) | (n/a) | `Right atoms. Now add its state: (s), (l) or (g).` |
| wrong_state | (n/a) | (n/a) | `Right atoms. This spell needs it as ${stateWord}.` (`a liquid`, `a gas`, `a solid`) |
| not_single | (n/a) | (n/a) | `This spell is one substance. Write just its formula.` |
| no_products | (n/a) | (n/a) | `Add an arrow and what it makes.` |
| wrong_species | (n/a) | (n/a) | `Those are not the substances in this reaction. Check what burns and what it makes.` |
| element | (n/a) | (n/a) | `"${bad}" is not an element symbol.` |
| parse | (n/a) | (n/a) | `Use element symbols, numbers, + and an arrow.` |
| not_for_band | `Pick a spell from the list.` | same | same |

`(n/a)` codes cannot arise from that band's UI; `reasonFor` returns the Apprentice string for them. The Seedling `unbalanced` line is a claim (row in 8); the rest are not-claims.

## Lanes

| lane | owns | builds |
|---|---|---|
| contract (first) | the files in "Files the stub creates or changes" | The stub; root `cargo test --locked`, `cargo test -p realgame-sim --no-default-features --locked`, client native `cargo check --locked` and `./build-web.sh` pass; golden passes unchanged on the stub. Records the post-I-026/I-027 base sha and refreshed line numbers in a "Refresh" note at the end of this spec |
| sim | `crates/sim/src/bench.rs` (bodies, tests 1-6), `crates/sim/src/spellbook.rs` (the prompt rule and test 7 only) | Tests 1-7 first (red on the stub), then `judge`, `recipe`, `learnable`, `prompt` and the prompt rule. Must not touch `formula.rs`, `spell.rs`, `reactions.rs`, `game*`, `web.rs`, any other sim file, `content/**` (the prompt data is the stub's: a wrong string in the stub is the contract builder's bug, fixed by the contract builder only, and the sim lane reports it), or `crates/bevy-client/**` |
| client | `crates/bevy-client/src/bench.rs` | The station meshes and materials, `atBench`, the command loop (including `band`), the `learn` call, the `BenchView` build and change-only publish. Native and wasm builds, `cargo fmt --check`, wasm clippy `-D warnings`. Must not touch `main.rs` beyond the stub's lines, `bridge.rs` beyond the stub, any other `src/*.rs`, `Cargo.*`, `web/**` or tests |
| web | `crates/bevy-client/web/bench.js`, `crates/bevy-client/web/style.css` (one appended block), `test/bench.test.ts` (new), `tests/e2e/golden.mjs` (the `formulaBench` body only), `tests/e2e/analytics-e2e.mjs` (test 29, appended), `content/science-sources.md` (the rows in 8 only), `test/science-sources.test.ts` (the three `prompt_*` keys in the `spells.json` key list only) | Tests 8-11 first, then the modal, chooser, three band views, speech, sound, done and fizzle panels, `#benchbtn`, the key capture, the modal gate, the `near`/`hint` classes, the controls row, `mountBuilder` (test 11b), the debug `__bench.fake`; the Fire scale **last** (fallback in section 3). Until the sim and client lanes merge, the stub judges everything `parse`, so the web lane builds the done and fizzle panels with `__bench.fake(...)` under `?debug` and screenshots them into its evidence. Golden tests 12-28b written against the stub (12 passes on the stub; 13-28 fail until the client and sim lanes merge) and recorded. Must not touch `hud.js`, `index.html`, `age.js`, `analytics.js`, `progress.js`, `boot.js` beyond the stub, the panel harness, or any existing golden or analytics-e2e check |

The sim, client and web lanes run in parallel after the stub. **Seam:** the `BenchView` JSON and the command strings. The seam breaker checks that:

- the texts `composeSprout`, `composeSeedling` and `composeEquation` produce for a correct build are judged `Ok` by `judge` for every learnable spell and band (run the JS compositions through the shared build's bench);
- `scaleOf` agrees with `judge`'s `Unbalanced` diffs for every coefficient combination 1-4 on Fire (256 cases, through `__bench.cmd('try …')`);
- no key event reaches winit or `hud.js` while the bench is open (test 19), including on a native build where it is irrelevant (the native build has no bench UI);
- the pad never opens the bench at all (test 13), and repeated correct builds never re-learn: after five correct builds of one spell, its `xp` is still 10 (test 21);
- no command word or `BenchView` field differs between `bench.js` and `bench.rs` (`band` in particular, open and closed);
- `__benchState` is not rewritten on frames where nothing changed (spy on the property setter for 60 frames with the bench closed and the player still: at most 1 write; and with the bench **open** on a build view for 60 frames with three judged tries: at most one write per try plus one for the open).

After the shared build, the tester runs tests 29, 31 and 32. A **science reviewer** (B6 protocol, fresh model) checks the 11 prompts, the reason table and the new register rows against [S1], [S4] and [S7], and confirms the judge accepts exactly the sourced formulas.

## Overlap and merge order

| file | this item | in-flight item's hunk | resolution |
|---|---|---|---|
| `main.rs` | `mod bench;` (mod list); two new `add_systems` lines after the existing ones | **I-022:** `Settings`, `read_settings`, `aim_and_cast` lines. **I-024:** the `world::build` call (464). **I-025:** `cues`, streams, `aim_and_cast`, the publish format tail. **I-026:** `mod spells;` (mod list). **I-027:** `mod progress;`, `Progress`, `init_resource`, `control` params and arm, `publish` | The `mod` list is a one-line neighbour of I-026's and I-027's `mod` lines: land after both (alphabetical: `bench` goes first in the list). No other shared hunk |
| `bridge.rs` | `WebIn.bench`, `poll`, `publish_bench` | none (I-019, I-022, I-025 and I-027 list it as not edited) | none |
| `boot.js` | one import and one guarded call | **I-027:** one import and one call after `startHud()` | **I-027 first**; this item's lines go after I-027's |
| `build-web.sh` | `web/bench.js` added to the `cp` line (21) | **I-027:** must add `web/progress.js` to the same line (its spec does not list it; the planner tells I-027's builder; otherwise `progress.js` 404s in `pkg-out` and I-027's golden tests fail) | Whoever lands second rebases the one line; both files end up in the list |
| `style.css` | one appended block at the end | none in flight (I-019 merged; I-022, I-025 do not edit it) | none |
| `golden` | `formulaBench` at the end; one call line before `no console errors` | **I-022:** `reviewFollowups`. **I-025:** `streamsAndFinder`, key-order lines. **I-027:** `spellProgress`, key-order lines | Call order: feel, I-025, I-019, I-022, I-027, I-028. This item edits no key-order line |
| `analytics-e2e.mjs` | test 29 appended | **I-022:** appended checks | append after I-022's |
| `index.html` `#age` (read only) | depends on `#age button[data-band]`, its `svg`, and `#age`'s `hidden` | **I-022** may change the picker markup and `age.js` | The contract builder re-reads the merged `index.html` and confirms these selectors (and that `#age` keeps `aria-modal="true"`) before the stub; any rename is applied in the stub and recorded in the Refresh note |
| (I-029 spec) | `bench::judge` over composed text; `BenchState.band` from the `band` command; `mountBuilder(root, { band, spell, recipe, prompt, onMake }) -> { destroy }` with `data-token`; the `aria-modal` gate; capture order (section 1) | **I-029** (specced before this) assumes a Rust `Band` resource and a debug-free `band <id>` it owns, `bench::check(spell, band, answer)`, a richer `mountBuilder` handle, and "I-021's `band.js`" (which does not exist) | **This item settles it.** I-029 must call `bench::judge` through the same text path (no `check`), read `BenchState.band` (no own band command or resource), take option names from the merged I-028 and extend `mountBuilder` in its own PR, put `aria-modal="true"` on its overlay, and keep test 21's "Unknown still casts" check. The planner adds one line to I-029's Overlap table before I-029 is built |
| (I-030 spec) | `judge`, `recipe`, `Band`/`Tier` names; `mountBuilder` with `recipe: null` (test 11b); `.bench-coef` without `.bench-scale` | **I-030** adds `trial` words and view fields to `bench::run` and hooks in `bench.js` | Compatible; keep the class names exactly as in Contract |
| `content/spells.json`, `spellbook.rs`, `test/science-sources.test.ts`, `content/science-sources.md` | prompt fields, prompt rule, three lint keys, new rows | **I-026** creates these | Builds on I-026 merged (a dependency, not a parallel overlap) |
| `world.rs`, `Cargo.toml`, `Cargo.lock`, `perf.mjs`, `hud.js`, `cues.rs`, `fx.rs`, `feel.rs`, `chem.rs` | not touched | I-022, I-024, I-025, I-026 | none. The station is spawned outside `world::build`, so I-024's scenery post-pass never sees it |

**Merge order:** I-026 → I-027 (main) → I-028. I-022, I-024 and I-025 can land before or after I-028 in any order; only the `golden` call line and `analytics-e2e.mjs` append need a trivial rebase.

## ACTIONs and planner notes

- **No human ACTION.** Nothing new is collected or sent; the band is not stored.
- **Planner (not human-only):** fix I-028's backlog "After" cell to `I-026, I-027`; file "Start over on this computer" **before this item's PR is opened** and mark this row "do not ship to teachers before" it; file the Adept bench follow-up; tell I-027's builder about `build-web.sh` (Overlap) before either builds; add the I-029 Overlap line (judge, `BenchState.band`, `mountBuilder` names, `aria-modal`, keep test 21's check); add `?band=` to the teacher-facing notes (GS-29); add one line to the panel brief: "the B key or the flask button (top right) opens the Formula Bench; the lab pad near the start lights the button"; if the Fire scale ships as the fallback, file the scale as a follow-up.
