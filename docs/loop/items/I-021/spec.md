# I-021: Captions at the player's reading level (age bands)

**Revision 3** (after `critique.md` of the roadmap-v2 refresh; see "Planner response" there). Changes: `window.__band` is a hard interface (`get`, `set`, `on`) that I-028 and I-029 must use, and `adept` stays `adept`; cast captions are keyed by spell id (`captionKey = "cast"`, `captionArg = <id>`), so I-021 no longer has to go before I-026 and new spells need no enum variant; 26 caption keys (was 29); seedling cells exist only where the Rust text fails the seedling rule (15 event keys and the Water cast now show the Rust text); a hotbar slot with no `SLOTS` row shows its name only; the Bench uses the `SLOTS` names (`Smother`) at sprout/seedling; the B4 prediction covers only what this item controls; GS-21 is "partial" (no speech yet; follow-up filed); tests 11b/11c are breaker checks, not gates; test 10 trimmed; test 5 gains a banned-word check, test 8 a mutation; test 16's grep is a pre-dispatch planner step with its hits listed; `checkReading` is named as the checker for sprout/seedling text in `content/*.json`.

**Revision 2** (after `critique.md`). Changes: seedling cells equal to the Rust text are no longer copied (they fall back to `state.caption`); caption keys are a Rust enum, so a keyless `say` does not compile; existing e2e and web tests that read HUD text are moved to `?band=apprentice` or `state.caption`; `{who}` counts as two words and the sprout `spread` line is reworded; `COACH_TEXT.apprentice` is cut; sprout coach icons are explicit; `start` keeps the click instruction; test 11 is split in three; notes on `Smother` vs `CO₂`, band-free I-019 strings, `areas.md`, `science-review.md` and the pause-card follow-up.

**Refreshed for roadmap v2** (see "Refresh (roadmap v2)" at the end): four reading levels (Sprout 4-7, Seedling 8-10, Apprentice 11-15, Adept 16+), the band and reading-rule modules made reusable by the Formula Bench (I-028), and the overlap with I-020, I-022, I-024, I-025 and I-026 named with a merge order. Scope did not grow.

**based_on:** origin/main `c4832d2e86c2408443124960b130fc6208a89aab` (2026-10-30, roadmap v2, #31; I-019 merged in #29). `git diff a0e2ecb..c4832d2 -- crates tests functions test` is empty, so the code is what I-019 left. The code was read with `git show c4832d2:<path>`: `crates/bevy-client/src/{main.rs,chem.rs}`, `crates/bevy-client/web/{hud.js,age.js,analytics.js,boot.js,index.html}`, `crates/bevy-client/build-web.sh`, `tests/e2e/golden.mjs`, `content/science-sources.md`, `tests/e2e/panel/personas.mjs`, `crates/sim/src/game.rs` (its `Tier`), and `docs/loop/{target.md,bars.md,backlog.md,areas.md,gameplay-standards.md,science-review.md}`. Files not re-read since a3e50b1 (`scripts/build-site.sh`, `test/science-sources.test.ts`, `tests/e2e/panel/{daemon,panel,score,hud}.mjs`, `result.schema.json`) are unchanged between a3e50b1 and c4832d2 (`git diff --stat`). The I-020 contract comes from its roadmap-v2 refresh (`docs/loop/items/I-020/spec.md`, based_on c4832d2); the I-022, I-024 and I-025 facts come from their specs and, for `age.js`, from the I-022 lane branch (`loop/I-022` at eb7e3c5). The pinned tables of I-020 revision 1 were recovered from the planner's session record, then corrected here (see "Changes from I-020 rev 1").

**Status: blocked on merge order.** I-005, I-007 and I-019 have merged. Do not send this item to lanes until **I-020 has merged**. I-020 itself waits for I-022 (building) and I-025 (being specced), so the order is **I-022, I-025, I-020, then I-021**, and **I-021 before I-028** (the Bench reads `window.__band`). I-021 and I-026 may merge in either order: cast captions are keyed by spell id, and `event_key` matches `EventKind`, which I-026 keeps (see "Overlap" under Contract). Builders branch from origin/main after the I-020 merge. Before dispatch the planner:
1. moves `based_on` to that commit and re-checks the facts below, including every line number (they are c4832d2's and will move with I-022, I-025 and I-020);
2. confirms that `docs/loop/items/I-020/caption-strings.txt` exists on that commit (I-020 writes it), because tests 3 and 6 reuse it;
3. confirms that the register ids of the two moved rows are still C-030 (`Heat spread from bush to bush. Press R to try again.`) and C-033 (the `#warning` text) on that commit, and renumbers them in §6 if I-019 or I-020 changed them;
4. confirms that `docs/loop/science-review.md` exists on that commit (it exists at c4832d2). If it does not, test 15's fallback applies;
5. adds `crates/bevy-client/src/captions.rs` (new, this item), `crates/bevy-client/src/lines.rs` and `onboard.rs` (I-020), if not yet listed, to the `client-core` row of `docs/loop/areas.md`, as a planner change outside this item's diff, so the area check covers them and test 18 is not the only guard (at c4832d2 that row lists only `main.rs`, `player.rs`, `bridge.rs`);
5a. reads the merged golden key-order lines (`golden.mjs:1043-1047` at c4832d2) and writes the exact `I021_KEYS` insertion point into `round-1.md`'s header for the contract lane (after I-025's and I-020's key lists);
6. files the follow-up "pause-card reading-level switch" in `docs/loop/backlog.md` (see Out of scope). It is the only way a player with DNT, GPC or analytics off gets the sprout level. It also files "Adept (16+) in the age picker and analytics bands" as a Paawan decision (a change to what data is collected is human-only), and "Speak sprout captions with I-028's speech helper (sound on, text stays)" (B4, GS-21; blocked by I-028);
7. **edits the sibling specs** before either is dispatched (planner changes outside this item's diff):
   - **I-028 §2 and its test 11:** the Bench reads the band with `window.__band.get()` and follows it with `window.__band.on(cb)`; it adds **no** `click` listener on `#age button[data-band]`; its chooser calls `window.__band.set(b)` (so the HUD changes with it); `?band=adept` stays `adept` in `window.__band`, and the Bench maps `adept` to its Apprentice view itself; `bandFrom(search, picked, chosen)` and its test 11 are replaced by a test that the chooser calls `__band.set`. The Bench chooser is shown only when no band was given by the URL or the picker: `band.js` exports `bandKnown()` for that (see §2). At sprout and seedling, Bench card names for slots 1-4 come from `SLOTS[band][i].name` (so spell 4 is `Smother` on the HUD and the Bench). I-028's "After" cell in `backlog.md` gains I-021;
   - **I-029 line 57:** "I-021's `band.js`, extended by I-028 with `adept`" becomes "I-021's `band.js` (`window.__band`; `adept` already a level)";
   - **I-026:** each spell in `content/spells.json` may carry optional `caption_sprout` and `caption_seedling` (the same pattern as I-028's `prompt_*`). I-026 (or the item that adds spells 5-8) moves the four `CAST_CAPTIONS` entries there, checks them with `checkReading` and gives them register rows. Until then `captions.js` holds them. I-026 replaces `captions::spell_id(i)` with `Spell::ALL[i].spell_id()`;
   - **I-030:** its sprout and seedling question text passes `checkReading` at its level;
8. **re-runs test 16's grep** on the I-020 merge base (every file under `tests/e2e/` and every `crates/bevy-client/web/*.test.mjs`, for text assertions on `#caption`, `#warning`, `.formula`, `#hint`, `#history`) and writes the hits and the fix for each into `round-1.md`'s header, so the contract lane's file list is fixed before dispatch. At c4832d2 the hits are: `golden.mjs:864`, `886-887` and `1310-1318` read `#hint` for I-019's idle and locked hints (band-free; no change); `tests/e2e/panel/hud.mjs:8-13` and `54` are the panel's screen reader, not assertions (no change); `panel/selftest.mjs:94` checks the reader's keys only (no change). So at c4832d2 no existing test needs an edit. I-020's `onboard.mjs` (coach text at the default band, which is unchanged) and any I-022 or I-025 test are added by the re-run.

**Stale check.** The planner refreshes this spec before building if any of the following differ from what is listed under "Relied on":
- I-026 (if it merged first): `Spell::spell_id()` gives `fire`, `water`, `frost`, `co2` for `Spell::ALL` order, `EventKind` still has 21 variants, and `cast_caption()` still has the c4832d2 text;
- I-005 (as changed by I-022): the `age.js` click handler line `start(b.dataset.band ?? '');` (`age.js:158` at c4832d2; about line 175 after I-022, which adds the `inert` restore and a `pointerlockchange` removal above it), the picker's exactly-3-buttons check (`age.js:104`), the band values `sprout` / `seedling` / `apprentice` in `analytics.js` `BANDS` (`analytics.js:20`), the rule that the band is never stored, or the lazy `import('./age.js')` in `boot.js:35` after `startHud()`;
- I-020: `coach.js` exports `COACH` and `coachFor(state, now, doneAt)`; the coach text reaches `#hint` through `setHint` (so I-022's `hintAllowed` gates it); `lines.rs` has `event_lines(...)` and `react()` only applies its results; `caption-strings.txt` and its Rust test exist; `I020_KEYS` is in golden's key-order check;
- I-019 (merged): the boot caption `Save the Greenhouse! Put out 3 fires before they spread. Water is ready: aim at a fire and click.` (`main.rs:636-638`);
- `hud.js` still shows captions through one `showCaption(state.caption)` call with `history.unshift(state.caption)` (`hud.js:682-689`), and still builds the `#warning` text (`hud.js:696`) and the lose banner's `.small` (`hud.js:707`) inline;
- `index.html` hotbar slots still carry `.name` and `.formula` spans (`index.html:28-31`), and the picker still has three buttons (`index.html:59`, `71`, `84`);
- golden still pins the exact state key list (`golden.mjs:981` `BASE_KEYS`, `1043-1047` the appended lists);
- `daemon.mjs` still opens `base + QUERY`, and `score.mjs` still requires one shared `query` across the 5 results.

## Facts from c4832d2 this spec relies on

- **Captions have one level.** Every caption is Rust text with formulas and temperatures, for example `Soaked: it must dry out (boil off at 100 °C) before it can burn.` They come from four places:
  - `chem::Spell::cast_caption` (4 strings, `chem.rs:88-102`);
  - `chem::EventKind::caption` (one per variant, 21 strings, `chem.rs:201-249`, including all 6 `Fizzle` arms, all 4 `Extinguished(_)` arms and `Station`; `EventKind` is `chem.rs:108-136`, `Fizzle` `chem.rs:138-148`). The 122 lines `chem.rs` gained since a3e50b1 are I-018's electrolysis test; no caption text changed;
  - `react()` in `main.rs` (`1292-1349`): `Reseeded` (`1316-1319`), the spread block (`1320-1339`: the greenhouse line, the spread `format!` `Heat from a neighbour raised the {who} to its ignition temperature: it caught fire!`, `k.caption()` for uncaptioned `Melted`/`BurntOut`), and the win line (`1341-1348`);
  - `setup` (I-019's boot caption, `main.rs:636-638`) and `reset()` (`main.rs:892`, `Save the Greenhouse! Put out 3 fires before they spread.`);
  - the cast caption is said in `aim_and_cast` (`main.rs:1098`); spell results in `projectiles` (`main.rs:1275-1278`).
  - `Game::say(&str)` (`main.rs:182-186`) sets `caption`, bumps `caption_id`, resets `caption_age` (fields `main.rs:137-139`). `publish` (`main.rs:1546`) sends `caption` and `captionId` through `esc` (format string `1580-1581`, args `1597-1598`). The publish JSON template starts with `{{"`, so the science lint skips it.
- **The HUD shows the Rust text as is.** `hud.js` calls `showCaption(state.caption)` (`hud.js:398-403`, 8 s) and keeps the last 6 in `history` (`hud.js:142-143`, drawn at `221-227`; `C` key at `424`). Since I-019 the push skips a repeat: `if (state.caption && history[0] !== state.caption)` (`hud.js:685`); the band version compares the band text the same way. It also writes two band-free strings inline:
  - `#warning .text` (`hud.js:696`): `` `The greenhouse is heating up: ${state.ghTemp} °C. Wood can catch fire at about 300 °C!` `` (register C-033), shown while `outcome === 'playing' && (ghDanger || ghTemp > 60)`;
  - lose banner `.small` (`hud.js:707`): `Heat spread from bush to bush. Press R to try again.` (C-030). The banner `.big` texts and the win `.small` (`${state.extinguished} fires put out. Press R to play again.`, a Not-claims row since I-019) have no science.
- **The hotbar labels are static `index.html` text** (`index.html:28-31`): `Fire` / `CH₄ + 2O₂ → CO₂ + 2H₂O`, `Water` / `cools below ignition`, `Frost` / `N₂(l) boils at −196 °C`, `CO₂` / `CO₂ displaces O₂`.
- **The band is known only inside `analytics.js`, and only on the deployed site.**
  - `boot.js` runs `startHud()` first (`boot.js:27`). Only when `<meta name="rg-analytics" content="on">` (the `web/dist` copy) does it lazy-import `age.js` (`boot.js:35`). `pkg-out` (golden, perf, panel, local dev) never shows the picker, and neither does a browser with DNT or GPC.
  - On a pick, `age.js` (click handler `age.js:149-161`) hides the picker, calls `start(b.dataset.band ?? '')` (`158`), then `resetWhenReady()`, then focuses `#game`. `start` keeps the band in a closure. Nothing exposes it, and nothing stores it (by design: shared classroom devices). After I-022 the handler also removes a `pointerlockchange` listener and restores `inert` before hiding the picker; the `start` line and the lines after it are unchanged.
  - `startAgePicker` returns false unless there are **exactly three** `button[data-band]` (`age.js:104`). The third is labelled "11 years old and up" (`index.html:84`) and sends `apprentice`.
  - `analytics.js` exports `BANDS = ['sprout', 'seedling', 'apprentice']` (`analytics.js:20`), and `eventBody` drops any event whose band is not in it. It is not copied into `pkg-out` (`build-site.sh` copies it into `web/dist` only), so `pkg-out` code cannot import it.
- **Roadmap v2 names four reading levels** (`target.md` "Core loop"): Sprout 4-7, Seedling 8-10, Apprentice 11-15, Adept 16+. The picker and the analytics band set have three; `adept` exists nowhere in code yet. The 2D engine's `crates/sim` `Tier` (`game.rs:152-164`) also has three (`sprout`, `seedling`, anything else is `apprentice`).
- **Target names** for `{who}`: `bush`, `dry hedge`, `greenhouse`, `ice wall`, `puddle`, `steam vent`, `lab bench` (`world.rs`). Only `bush` and `dry hedge` can reach the spread line. A greenhouse ignite takes the greenhouse line.
- **The science lint** (`test/science-sources.test.ts`) walks `crates/bevy-client/src/**/*.rs`, `web/**/*.html` and `web/**/*.js` (not `.mjs`). It flags strings that contain a temperature, a formula or a science keyword (`burn`, `heat`, `smother`, `boil`, `melt`, `oxygen`, `caught fire`, ...), and strings with a sentence shape. A flagged string passes when a `## Claims` or `## Not claims` row has the same normalized text, wherever it lives. A `C-` row is **stale** (test fails) when a file in its `where` no longer contains the text. JS template text keeps `${...}` literally.
- **The panel.** `panel.mjs start` spawns `daemon.mjs ... --query <q>`. `daemon.mjs` opens `base + QUERY` and logs `query` in the `start` record and in `session.json`. `score.mjs` marks a panel "mismatched" unless all 5 results share one `query`. `result.schema.json` has `additionalProperties: false` at the root. `personas.mjs` exports `PERSONAS`, `PERSONA_IDS` and `persona(id)`. There is no `personaInfo`.
- **Web unit tests.** There are none under `crates/bevy-client/web/` yet. I-020 adds `*.test.mjs` there, run with `node --test`. The root `npm test` runs only `test/**/*.test.ts`.

## Relied on from I-005, I-019, I-020 and I-022 (their contracts; not rebuilt here)

- **I-005.** The band values above; the picker only on the deployed site with analytics on and no opt-out; the band is not stored; the pick calls `start`, then `resetWhenReady`, then focuses the canvas.
- **I-019 (merged, #29).** The boot caption text; Water at boot; the hint priority (banner > idle > note > locked > start, all through `setHint`); new DOM built from `hud.js`; golden's appended key list `I019_KEYS`.
- **I-022 (building).** `setHint` passes text through `hintAllowed`, so with Hints off every coaching line in `#hint`, the sprout coach text included, is hidden; the `#coach` icon stays (I-020). `age.js`'s picker frees the pointer lock and sets `inert` on the game while open, and restores it in the click handler before `start(...)`.
- **I-020.**
  - `coach.js`: `COACH` (one plain text per step: `walk` `Hold W to walk forward.`, `pick` `Fire will not put out fire. Pick Water: press 2.`, `cast` `Aim at a fire and click to put it out.`, `done` `Nice! Now put out every fire.`) and `coachFor(state, now, doneAt)`. The coach text goes into `#hint` through `hud.js`'s one wrapped expression `setHint(coachText || pickHint({...}))` (I-025's `pickHint`).
  - `lines.rs`: `spread_line_due` and `event_lines(...) -> Vec<String>`, which hold every event-line decision `react()` makes. Spread is always said.
  - `caption-strings.txt` and the Rust test that every snapshot caption is still in `chem.rs` / `main.rs` / `lines.rs` byte for byte.
  - The world hold: `chem_tick` waits until the player walks 0.4 m and lands one cast. `tick N` (debug) bypasses it. `move` and `face` work under `?debug`.

**This item does not change** any Rust caption text (Apprentice and Adept show it unchanged), the chemistry, casting, the coach logic, the hold, the hint priority, settings, or the I-007 / I-019 hint strings.

## Problem

In the nightly panel (artifact 3dbd332), the 6-year-old and 9-year-old read captions written for a chemistry student: `CH₄ + 2O₂ → CO₂ + 2H₂O`, `boil off at 100 °C`, `N₂(l) boils at −196 °C (77 K)`. B4 median clarity was 6 against a bar of 8, and the young personas reported text blockers about formulas and "100 degrees". I-005 now asks the player's age on the deployed site, but the game ignores the answer: the band reaches the analytics beacon and nothing else. A pre-reader gets no picture with a caption, and the hotbar shows formulas to everyone.

**Goal:** every caption, hotbar label, warning and lose line has a version for the player's band, and the coach line has a sprout version. It is short and plain for 4–7, with a picture. It is plain with at most one number for 8–10. It is the full formula text for Apprentice (11-15) and Adept (16+). **Every version stays scientifically true.** Simpler words may leave things out, but must never say something false, and each one has a register row.

## Bars and expected move

- **B4 (every age can play):** this item's movement. It is a panel bar, so nothing is reported as moved at merge. **Panel B4 is measured "if the band is known":** the panel gives each persona its band in the URL (`?band=`), the way a real player gets it from the I-005 picker. Real players with DNT, GPC or analytics off get the default, `seedling`. So the first paired panel after merge (same seed and query, I-020 merge artifact vs this one) also runs **one extra 6yo session with no band** (`--band none`, default `seedling`). That session is scored on its own and is not one of the 5 results. Predictions:
  - neither the 6yo nor the 9yo reports a text blocker about a chemical formula, a subscript or kelvin in a caption, a hotbar label, the warning or the lose line (moved here from I-020). The 6yo also reports no blocker about a temperature in those places. The 9yo may still read one temperature per caption (seedling rule), for example `100 °C`;
  - median clarity is not lower than on the I-020 merge artifact. This item does not claim a number: the hints the 6yo meets first (`START_HINT`, `AIM_HINT`, `IDLE_HINT`, `#obj-line`, the pause-card controls table) stay band-free, and clarity is a model score. The I-020 claim "median clarity 6 to at least 7" is **not** carried here;
  - the no-band 6yo guard also reports no formula text blocker in the places above;
  - the 13yo (apprentice), teacher and qa (adept) see the same caption text as before; their clarity does not fall;
  - `time_to_move_s` and `ttc` do not regress for any persona.
- **B6 (science is right):** must stay at 0 errors with 100% of shown claims sourced. This item adds about 45 rows. Every one of them is reviewed by a fresh science reviewer before merge (test 15).
- **B1:** golden passes 3/3, with only its key-order lines edited (test 16). **B2/B3:** no regression beyond the measured noise (test 17). **B7:** the analytics path still sends `session_start` with the picked band (test 9).
- B4 still needs ≥ 8 clarity and zero control blockers to pass, which depends on I-007, I-019 and I-020 as well.
- **B8 / B9 / B10 / B11 (roadmap v2):** not moved. This item gives the Formula Bench (I-028) the one band store (`window.__band`) and the reading-rule checker (`checkReading`), so the HUD and the Bench never show different bands and the Bench's per-band text is checked by the same rules. Cast captions are keyed by spell id, so spells 5-8 (B8) need no Rust enum change.

## Changes from I-020 rev 1 (the hand-off tables)

The hand-off tables were checked again for truth and for their own reading rules:
- **won, sprout** `You did it! The greenhouse is safe!` broke the rev-1 rule (no word over 8 letters). The sprout rule now exempts exactly one word, `greenhouse`: it is the pictured object and the word in the objective title.
- **pick, sprout** `Fire can't stop fire.` was false in general: backburns do stop wildfires by using up fuel. It is now `Fire can't put out fire. Press 2!`, which is true of a burning object.
- **pick, apprentice** `Fire feeds fire.` (rev 1) is dropped: rev 2 cuts `COACH_TEXT.apprentice`, and apprentice reads I-020's `COACH` text.
- **spread, sprout** `Oh no! Heat lit the next {who}!` read as "the following one". It became `Oh no! Heat spread and lit the {who}!` in rev 1 of this spec, and in rev 2 `Heat spread! The {who} caught fire!` (the rev-1 text reached 9 words with `dry hedge`).
- **`SLOTS.apprentice`** is no longer a copy of the `index.html` strings. `hud.js` captures the `index.html` text at boot and restores it, so the two cannot drift. The test reads `index.html` at test time (critic requirement).
- The **18 unpinned keys** are now pinned too (section 4), so builders do not guess and every row is known before building.
- **`warningFor` and `lostFor`** are pinned with their current strings for apprentice (critic requirement).
- `personaInfo` did not exist. The panel uses `persona(id).band`.
- The `docs/loop/panel.md` paragraph is dropped. `areas.md` does not list `docs/loop/**`, and the planner does not edit `areas.md` in an item. The band rule lives in the `personas.mjs` header and the `panel.mjs` / `daemon.mjs` usage lines (critic requirement: "add it to areas.md or drop the paragraph").

## Smallest version

### 1. Caption keys (Rust: `crates/bevy-client/src/captions.rs`, new, Bevy-free)

- `pub enum CaptionKey` with 26 unit variants (`Start`, `Cast`, `Ignited`, ..., `Won`, in the order below), `#[derive(Clone, Copy, Debug, PartialEq, Eq)]`, `impl Default` giving `Start`, `pub const ALL: [CaptionKey; 26]` in that order, and `pub fn as_str(self) -> &'static str` giving the snake-case string. A wrong or missing key is then a compile error. The enum covers the state and event captions only; **a cast is one key, `cast`, and the spell is named by its id in `captionArg`**, so a new spell (B8) needs no new variant.
- `pub const CAPTION_KEYS: &[&str]` stays as a literal list (the web test reads it from source with a regex), exactly these 26, in this order:
  `"start"`, `"cast"`, `"ignited"`, `"out_water"`, `"out_frost"`, `"out_foam"`, `"out_fire"`, `"burnt_out"`, `"reseeded"`, `"melted"`, `"melting"`, `"froze"`, `"steam"`, `"smothered"`, `"soaked"`, `"chilled"`, `"fizzle_wet"`, `"fizzle_no_oxygen"`, `"fizzle_not_flammable"`, `"fizzle_greenhouse"`, `"fizzle_charred"`, `"fizzle_heated"`, `"station"`, `"spread"`, `"greenhouse_caught"`, `"won"`.
- `pub fn spell_id(spell_index: u8) -> &'static str`: 0 `fire`, 1 `water`, 2 `frost`, 3 `co2` (the `Spell::ALL` order; these are I-026's permanent spell ids); 4 or more gives `""`. If I-026 has merged, this body is `Spell::ALL.get(i).map_or("", |s| s.spell_id())` instead, and if I-021 merges first, I-026 makes that swap (planner step 7).
- `pub fn event_key(kind: &chem::EventKind) -> CaptionKey`: one key per variant, as in the table below. `captions.rs` may `use crate::chem` (`chem.rs` has no Bevy types).

**String → key table.** Every caption the game can say today, with its source and its key. The apprentice text is the current string, unchanged.

| key | source (at c4832d2) | current text (apprentice) | arg |
|-----|---------------------|---------------------------|-----|
| `start` | `setup` boot caption (I-019's text after it merges) **and** `reset()` | `Save the Greenhouse! Put out 3 fires before they spread.` (reset); I-019's boot text | `""` |
| `cast` | `Spell::cast_caption`, said in `aim_and_cast` on each cast | the 4 cast captions (C-034, C-006, C-007, C-035) | the spell id (`fire`, `water`, `frost`, `co2`) |
| `ignited` | `EventKind::Ignited.caption()`, when a **cast** lit the object (spell result in `projectiles`) | `It caught fire: hot enough to reach its ignition temperature, with oxygen around.` | `""` |
| `out_water`, `out_frost`, `out_foam`, `out_fire` | `Extinguished(Water / Frost / Foam / Fire)` | C-010, C-011, C-036, `Fire out!` | `""` |
| `burnt_out` | `BurntOut` (a spell result, or uncaptioned in `react()` via `k.caption()`) | `It burnt out: the fuel is used up.` | `""` |
| `reseeded` | `Reseeded`, always said in `react()` | `Another bush is on fire! Put it out before it spreads.` | `""` |
| `melted` | `Melted` (spell result, or uncaptioned in `react()`) | C-014 | `""` |
| `melting`, `froze`, `steam`, `smothered`, `soaked`, `chilled` | the matching `EventKind` | C-037, C-016, C-017, C-038, C-019, C-020 | `""` |
| `fizzle_wet`, `fizzle_no_oxygen`, `fizzle_not_flammable`, `fizzle_greenhouse`, `fizzle_charred`, `fizzle_heated` | `Fizzle(Wet / NoOxygen / NotFlammable / Greenhouse / Charred / Heated)` | C-021, C-022, C-023, the greenhouse not-claim, C-024, C-025 | `""` |
| `station` | `EventKind::Station`: any spell that lands on the lab bench. It explains the water-splitting station that is coming | `Water-splitting station (coming soon): 2H₂O → 2H₂ + O₂ with electricity.` | `""` |
| `spread` | `react()` / `lines::event_lines`: an uncaptioned `Ignited` (heat from a neighbour) whose target is **not** the greenhouse | `Heat from a neighbour raised the {who} to its ignition temperature: it caught fire!` | the target name (`bush`, `dry hedge`) |
| `greenhouse_caught` | the same branch when the target is the greenhouse. This is the losing event | `Oh no! Heat from the fire reached the greenhouse. Press R to try again.` | `""` |
| `won` | `react()`, `Outcome::Won` | `You saved the greenhouse! Water, frost and CO₂ each stop fire in their own way.` | `""` |

**Wiring in `main.rs` and `lines.rs`:**
- `Game` gains `caption_key: CaptionKey` (default `Start`) and `caption_arg: String`.
- `Game::say(&mut self, key: CaptionKey, text: &str)` replaces `say(&str)`. `Game::say_arg(&mut self, key: CaptionKey, arg: &str, text: &str)` sets the arg. Because the signature changes and the key is an enum, every call site must name a valid key or the build fails.
- Call sites (c4832d2 lines): `say(CaptionKey::Start, …)` in `setup` (`main.rs:636`) and `reset()` (`892`); `say_arg(CaptionKey::Cast, captions::spell_id(game.spell as u8), spell.cast_caption())` in `aim_and_cast` (`1098`, inside the block I-025 reworks; the core lane edits only this call); `say(captions::event_key(&last.kind), last.kind.caption())` in `projectiles` (`1277`, the line I-020 also wraps); `say(CaptionKey::Won, …)` in `react()` (`1343`).
- **`lines.rs`** gains `pub struct Line { pub key: CaptionKey, pub arg: String, pub text: String }` and `pub fn event_lines_keyed(<the same parameters as I-020's event_lines>) -> Vec<Line>`. It holds the decision. I-020's `event_lines` becomes `event_lines_keyed(..).into_iter().map(|l| l.text).collect()`, so I-020's tests and mutations stay valid unchanged. `react()` calls `event_lines_keyed` and passes each line to `say_arg(l.key, &l.arg, &l.text)`.
- `publish` appends `"captionKey":"{}","captionArg":"{}"` (the key through `as_str()`, each through `esc`) **at the end** of the format string (`main.rs:1580-1581`), after I-019's, I-025's and I-020's keys, so the base key order holds. Golden pins the full key list, so the contract lane appends `I021_KEYS = ['captionKey', 'captionArg']` at the key-order lines (`golden.mjs:1043-1047` at c4832d2), the same sanctioned edit I-019, I-025 and I-020 make.
- **No caption text changes.** I-020's `caption-strings.txt` test must still pass (test 3).

### 2. The band on the client (web: `crates/bevy-client/web/band.js`, new)

- `LEVELS = ['sprout', 'seedling', 'apprentice', 'adept']` (roadmap v2: Sprout 4-7, Seedling 8-10, Apprentice 11-15, Adept 16+). Everything that takes a band takes one of these. `PICKER_BANDS = ['sprout', 'seedling', 'apprentice']` (the three the picker and `analytics.js` know; one test checks it equals `analytics.js`'s `BANDS`). `DEFAULT_BAND = 'seedling'`.
- **`adept` in this item reads exactly like `apprentice`** (the full Rust text and the `index.html` slots). The Rust captions already carry formulas, kelvin and conditions, so no separate adept text is written here; a split between the two (for example, ionic or state-symbol detail only for adept) is for the Formula Bench (I-028) or a later caption item. `adept` is reachable only by `?band=adept` (teacher links, the panel). Adding it to the picker means a fourth picker button and a new analytics band value: that is a change to what data is collected, which is human-only (`target.md`), so it is **out of scope** and named as a follow-up for Paawan.
- `bandFromSearch(search)` returns the value of `?band=` when it is exactly one of `LEVELS`, else `null`.
- `createBand({ search, doc })` returns `{ get(), set(b), on(cb), known() }`:
  - it starts at `bandFromSearch(search) ?? DEFAULT_BAND`;
  - `known()` is true when the URL gave a band or `set` accepted one, false while the band is only the default (the Bench uses it to decide whether to ask);
  - `set(b)` ignores anything not in `LEVELS`. A valid band replaces the current one, so the last setter wins: the URL applies at boot, and a later pick overrides it. Setting the current band again calls no callback;
  - each change sets `doc.documentElement.dataset.band` and calls the `on` callbacks once;
  - it touches no storage (I-005's rule: no remembered band on shared devices).
- No DOM access at import.
- `hud.js` creates **the one band** in `startHud()` and exposes `window.__band = { get, set, on, known }`. `startHud()` runs before `boot.js` imports `age.js`, so the hook below always finds it. This is the only band store in the client: every consumer (HUD, Bench I-028, worked casts I-029) reads `get()`, follows `on(cb)` and writes with `set(b)`; none adds its own picker listener or keeps its own copy. `adept` stays `adept` in it; a consumer with no adept view (the Bench, `crates/sim` `Tier`) maps it itself.
- **The I-005 hook (the only change to `age.js`).** In the button `click` handler, one line goes **immediately before** `start(b.dataset.band ?? '');`:
  `window.__band?.set(b.dataset.band ?? '');`
  No other line of `age.js` changes (test 18 checks the diff). The picker only sends `PICKER_BANDS` values, so `set` always accepts them. This hook is the **only** way the picker's answer reaches any module; I-028 does not add a second listener (planner step 7). **Reviewer:** the I-005 main lane's breaker reviews this one line and re-runs `test/bevy-analytics.test.ts` (B7). **I-022 overlap:** I-022 edits the same click handler (the `pointerlockchange` removal and the `inert` restore, above the `start` line). I-022 merges first; the hook still goes immediately before the `start(...)` line, below I-022's lines, and touches none of them.
- **Default when no band is known** (DNT, GPC, `pkg-out`, local dev, no `?band=`): `seedling`, which means plain words, no formulas and at most one temperature. A 6-year-old with DNT still gets far easier text than today. A teacher can link `?band=apprentice` (or `?band=adept`) for the full formulas. A reading-level switch in the pause card is a follow-up.

### 3. Reading rules (web: `checkReading(text, band)` in `captions.js`)

`checkReading` returns a list of violations (empty = pass). Tests and the critic use it, and the I-020 coach texts must pass it too. It is pure (no DOM, no state) and takes any `LEVELS` value, so the Formula Bench (I-028) checks its Sprout picture-card and Seedling tile text with it rather than writing its own rules. A level it does not know returns one violation (`unknown level`), never a pass.
- **Words** are split on whitespace, with leading and trailing punctuation stripped. A `{who}` placeholder counts as **two words of 5 letters** (its longest runtime value is `dry hedge`). A `${…}` placeholder is one word of length 1.
- **`sprout` (4–7):**
  - at most 8 words;
  - no word longer than 8 letters, except the one word `greenhouse`;
  - no formula (see below), no `°`, no kelvin (`\d\s*K\b`), no `→`;
  - no digit except one single key or count digit 1–9 standing alone (so no `0`, no `100`, no `−196`);
  - every caption has an icon from the allowed set (§4).
- **`seedling` (8–10):**
  - at most 14 words;
  - no word longer than 11 letters;
  - no formula, no `→`;
  - at most one temperature (a number or `${…}` followed by `°C`/`°F`, or a number followed by `K`).
- **Formula:** any subscript digit U+2080–U+2089, or any word matching `/[A-Z][a-z]?\d/` (`O2`, `H2O`).
- **These rules are a floor**, not a measure of whether a 5-year-old can read a text: they catch formulas, numbers and long words. Panel B4 is the real measure.
- **Text held in JSON:** any sprout or seedling string in `content/*.json` (I-026's `caption_sprout` / `caption_seedling`, I-028's `prompt_*`, I-030's questions, `reactions.json`'s `short_sprout` / `short_seedling`) is checked with `checkReading` at its level by the item that adds it. The science lint does not scan JSON for reading level.
- **`apprentice` and `adept`** texts are not rule-checked (they are the register-reviewed full text); `checkReading(t, 'apprentice')` and `checkReading(t, 'adept')` return `[]`.

### 4. Captions and labels at each reading level (web: `crates/bevy-client/web/captions.js`, new)

- **`CAPTIONS`** maps each of the 26 keys **except `cast`** (so 25) to `{ sprout: { text, icon }, seedling: { text } }`. `apprentice` and `adept` have no entry: they show `state.caption`, the unchanged Rust text.
- **`CAST_CAPTIONS`** maps each spell id (`fire`, `water`, `frost`, `co2`) to the same shape. It is keyed by id, never by slot index. When I-026 (or the item that adds spells 5-8) carries `caption_sprout` / `caption_seedling` in `content/spells.json`, these four entries move there (planner step 7).
- **Seedling cells exist only where the Rust text fails the seedling rule.** `SAME_AS_RUST` lists the 15 event keys whose c4832d2 Rust text passes `checkReading(…, 'seedling')`: `ignited`, `out_water`, `out_frost`, `out_fire`, `burnt_out`, `reseeded`, `melting`, `soaked`, `chilled`, `fizzle_wet`, `fizzle_not_flammable`, `fizzle_greenhouse`, `fizzle_charred`, `fizzle_heated`, `greenhouse_caught`. Their `CAPTIONS` entry has **no `seedling` field**, and seedling shows `state.caption`. The same applies to `CAST_CAPTIONS.water` (the Water cast text passes). The other 10 keys (`start`, whose boot text is 18 words; `out_foam`, `smothered`, `fizzle_no_oxygen`, `won`, which say `CO₂`; `melted`, `froze`, `steam`, `station`, which have equations; `spread`, which is 15 words with `{who}`) and the Fire, Frost and CO₂ casts have a seedling cell. This keeps `captions.js` free of Rust copies (test 14). Test 6 checks the Rust texts against the seedling rule, and test 3 keeps them from changing.
- **`captionFor(state, band)`:**
  - `apprentice` and `adept` give `state.caption`;
  - for `captionKey === 'cast'`, the entry is `CAST_CAPTIONS[state.captionArg]`; if there is none (a spell with no reading-level text yet), sprout and seedling show **`state.spellName` only** (sprout icon `star`), never the formula caption;
  - otherwise the band's text for `state.captionKey`, with `{who}` replaced by `state.captionArg`;
  - a seedling lookup with no `seedling` field gives `state.caption`;
  - an unknown or missing key gives `state.caption` (test 6 makes this impossible for shipped keys).
- **`ICONS`** = `fire`, `water`, `frost`, `foam`, `steam`, `ice`, `smoke`, `star`, `warn`, `bench`. For `sprout`, `#caption` shows the icon before the text: a `span.cap-icon.icon-<name>` drawn with CSS in `style.css`, at least 32 px, with no text content and `aria-hidden="true"`. The icons differ by shape, not only colour (GS-19). The panel's `#caption` reader still sees only the words.
- **`#caption` and the pause-card `history`** show and store `captionFor(state, band.get())`, so the history stays at the player's level. I-019's repeat rule (`history[0] !== text`, `hud.js:685`) compares the band text. The `C` key replays it.
- **Pinned texts. All are exact.** Each text was checked against the rules in §3 and against the register sources (§6). "Rust" means no seedling field: seedling shows the Rust text.

| key | sprout text | icon | seedling text |
|-----|-------------|------|---------------|
| `start` | `Put out 3 fires! Aim and click!` | fire | `Put out the 3 fires! Aim at a fire and click.` |
| `ignited` | `It is so hot it caught fire!` | fire | Rust |
| `out_water` | `Fire out! Water cooled it.` | water | Rust |
| `out_frost` | `Fire out! The cold stopped it.` | frost | Rust |
| `out_foam` | `Fire out! No air, no fire.` | foam | `Fire out! The gas kept the air away from the fire.` |
| `out_fire` | `Fire out!` | star | Rust |
| `burnt_out` | `All burned up! Nothing left to burn.` | smoke | Rust |
| `reseeded` | `A new fire! Put it out!` | fire | Rust |
| `melted` | `The ice melted. Now it is water!` | ice | `The ice melted at 0 °C. Now it is a puddle.` |
| `melting` | `Melting! Ice needs lots of heat.` | ice | Rust |
| `froze` | `Frozen! The water turned to ice.` | ice | `Frozen! The very cold liquid turned the water into ice.` |
| `steam` | `Hot water turns into steam!` | steam | `Hot water boils at 100 °C and turns into steam.` |
| `smothered` | `Gas covers it. It can't burn now.` | foam | `A gas blanket keeps air away, so it cannot burn.` |
| `soaked` | `Wet now! Wet bushes are hard to burn.` | water | Rust |
| `chilled` | `Brrr! It got very cold.` | frost | Rust |
| `fizzle_wet` | `Too wet to burn!` | steam | Rust |
| `fizzle_no_oxygen` | `No air here, so no fire.` | foam | `The gas blanket keeps air away, so fire cannot start.` |
| `fizzle_not_flammable` | `That does not burn.` | warn | Rust |
| `fizzle_greenhouse` | `Not the greenhouse! Aim at a fire.` | warn | Rust |
| `fizzle_charred` | `Already burned. Nothing left to burn.` | smoke | Rust |
| `fizzle_heated` | `Getting hot! A bit more and it burns.` | fire | Rust |
| `station` | `Coming soon! It will split water.` | bench | `Coming soon: this bench will split water into two gases.` |
| `spread` | `Heat spread! The {who} caught fire!` | fire | `Heat from a fire made the {who} next to it catch fire!` |
| `greenhouse_caught` | `Oh no! The greenhouse caught fire! Press R.` | warn | Rust |
| `won` | `You did it! The greenhouse is safe!` | star | `You saved the greenhouse! Water, Frost and Smother each stop fire.` |

| `CAST_CAPTIONS` id | sprout text | icon | seedling text |
|-----|-------------|------|---------------|
| `fire` | `A hot gas flame!` | fire | `The Fire spell burns gas and makes lots of heat.` |
| `water` | `Water cools the fire down.` | water | Rust |
| `frost` | `Super cold! It cools things fast.` | frost | `Frost is a very cold liquid. It pulls heat away fast.` |
| `co2` | `This gas keeps air away.` | foam | `This gas is heavier than air and pushes the air away.` |

- **`SLOTS`** = `{ sprout: [...], seedling: [...] }`, four `{ name, label }` each. `hud.js` captures the four `index.html` `.name` / `.formula` texts **once, in `startHud()`, before any band-dependent write**, so the first sprout paint never shows formulas for a frame. On boot and on every band change, `hud.js` writes them into `.slot .name` and `.slot .formula` with `textContent`. For `apprentice` and `adept`, `hud.js` restores the texts it **captured from the `index.html` DOM at boot**. No copy of the formula strings lives in `captions.js`. The per-slot choice is the pure `slotText(band, i, captured)` in `captions.js`, which returns `{ name, label }`:
  - `apprentice` / `adept`: `captured` as it is;
  - sprout / seedling with a `SLOTS[band][i]` row: that row;
  - sprout / seedling with **no row** (a slot added later, for spells 5-8): `{ name: captured.name, label: '' }`, so a 6-year-old never sees an uncaptured formula.

| slot | sprout name / label | seedling name / label | apprentice and adept |
|------|---------------------|-----------------------|------------|
| 1 | Fire / `hot flame` | Fire / `burns gas` | as in `index.html` |
| 2 | Water / `cools fire` | Water / `soaks up heat` | as in `index.html` |
| 3 | Frost / `super cold` | Frost / `very cold liquid` | as in `index.html` |
| 4 | Smother / `blocks air` | Smother / `keeps air away` | as in `index.html` (`CO₂`) |

- **`Smother` vs `CO₂` is deliberate.** Sprout and seedling name spell 4 for its mechanism (GS-30, GS-31); apprentice keeps the `index.html` name and the Rust `CO₂` texts (cast, `won`, the I-020 pick line). In a mixed class the name differs between neighbouring screens. The science reviewer and the panel brief should not flag this as a bug. **Within one player's screens the name is the same:** the Formula Bench (I-028) names slots 1-4 from `SLOTS[band][i].name` at sprout and seedling (planner step 7), so the hotbar and the Bench card both say `Smother`. A subscript name (`CO₂`) fails the sprout rule, so the HUD cannot switch to it instead.

- **`warningFor(state, band)`** gives the `#warning .text` line. **`lostFor(band)`** gives the lose banner's `.small`. `hud.js` uses them in place of its inline strings. The banner `.big` texts and the win `.small` stay as they are for every band. Pinned:

| fn | sprout | seedling | apprentice and adept (current text, moved from `hud.js`) |
|----|--------|----------|-----------------------------------------------|
| `warningFor` | `The greenhouse is getting hot!` | `` `The greenhouse is heating up: ${state.ghTemp} °C. Too hot and the wood will burn!` `` | `` `The greenhouse is heating up: ${state.ghTemp} °C. Wood can catch fire at about 300 °C!` `` |
| `lostFor` | `The fire spread. Press R to try again!` | `Heat spread from bush to bush. Press R to try again.` (the same constant as apprentice) | `Heat spread from bush to bush. Press R to try again.` |

  Seedling keeps its own `warningFor` text because the apprentice line has two temperatures (`${state.ghTemp} °C` and `300 °C`) and fails the seedling rule. Seedling `lostFor` is the apprentice constant (no copy).
  The `${state.ghTemp}` expression is written exactly like that (parameter named `state`), so the register text matches what the lint extracts. The `#warning` meter bar stays for every band. It is the visual cue that goes with the sprout line, which has no number.

### 5. The coach at each reading level (web: `crates/bevy-client/web/coach.js`)

- New export **`COACH_TEXT = { sprout: {walk, pick, cast, done} }`**. I-020's `COACH` (text and icons) is not edited. **Seedling, apprentice and adept use I-020's `COACH` text unchanged**, so the 13yo, teacher and qa read the same hint as before.
- `coachFor(state, now, doneAt, band = 'seedling')`: the fourth parameter is new and optional. For `sprout` the returned `text` is `COACH_TEXT.sprout[step]`; for any other band it is `COACH[step].text`. **`icon` (and `step`, `slot`) are I-020's for every band**: a sprout coach line shows the same `#coach` step icon as `COACH[step].icon` (`key-w` for walk, and so on), so nothing in the sprout coach relies on text alone (GS-21). With no band, I-020's calls and tests behave exactly as before. `hud.js` passes `band.get()`.

| step | sprout text | icon (from `COACH`) |
|------|-------------|---------------------|
| walk | `Hold W to walk!` | `COACH.walk.icon` |
| pick | `Fire can't put out fire. Press 2!` | `COACH.pick.icon` |
| cast | `Click a fire to put it out!` | `COACH.cast.icon` |
| done | `Yay! Now put out every fire!` | `COACH.done.icon` |

### 6. Science register (`content/science-sources.md`)

Every new player-facing string in §4, §5 and the slots gets a row, **including short labels the lint does not flag** (`hot flame`, `cools fire`, `super cold`, `blocks air`, `very cold liquid`, `keeps air away`, `Fire out!`). Test 7 checks this, not only the lint. Rows are added with new `C-NNN` ids (never reused). No row is removed.
- **`where` edits:** C-030 and C-033 move from `crates/bevy-client/web/hud.js` to `crates/bevy-client/web/captions.js`, because the strings move there. Without that edit they go stale.
- Texts already in the register (every Rust caption shown at seedling, and `Heat spread from bush to bush.…`) need no new row.
- **Suggested verdicts** for the builder. The reviewer (test 15) decides.
  - **`correct`:**
    - cast `fire` both (S7, S4; S15 for "hot flame");
    - cast `frost` both (S1). The register note names the liquid: liquid nitrogen, as C-007 (a teacher may ask "what liquid?"; apprentice says it);
    - out_water sprout (S9);
    - melted both (S10);
    - melting sprout (S10);
    - steam both (S2, S10);
    - chilled sprout (S1);
    - fizzle_wet sprout (S16);
    - fizzle_heated sprout (S6);
    - station both (S13, S7);
    - spread both (S6): sprout `Heat spread! The {who} caught fire!`;
    - won seedling (S9, S1, S19);
    - soaked sprout (S16, S10);
    - lost sprout (S6);
    - pick sprout coach line (S9, S19);
    - slot labels `hot flame`, `burns gas`, `super cold`, `very cold liquid`, `soaks up heat`.
  - **`simplified`**, with a note on what is left out:
    - cast `water` sprout and `cools fire`: water cools the fuel and flame, and some steam smothers (S9);
    - cast `co2` both, out_foam both, smothered both, fizzle_no_oxygen both, `blocks air` and `keeps air away`: as C-035/C-036, CO₂ lowers oxygen and mainly absorbs heat; "no air" is the limiting case (S19, S14, S5, S8);
    - ignited sprout: it leaves out oxygen (S6);
    - out_frost sprout (S1);
    - burnt_out sprout and fizzle_charred sprout: as C-024 (S18, S7);
    - froze both: as C-016 (S1, S10);
    - warning seedling: as C-033, about 300 °C piloted (S6).
  - **`## Not claims`:**
    - start both;
    - reseeded sprout;
    - fizzle_greenhouse sprout;
    - greenhouse_caught sprout;
    - won sprout;
    - warning sprout (state report, as the `The greenhouse is heating up!` row);
    - `Smother` (spell name, as the `CO2` row);
    - `Fire out!`;
    - every sprout coach line except the pick line.
  - Seedling texts marked "Rust" add no row: they are the existing registered Rust strings.

## Acceptance tests

- **Rust:** `cargo test --manifest-path crates/bevy-client/Cargo.toml --locked` (as I-020).
- **Web unit:** `node --test "crates/bevy-client/web/*.test.mjs"` (`.mjs`, so the science lint does not scan test strings).
- **Panel unit:** `node --test tests/e2e/panel/`.
- **E2E:** `node tests/e2e/bands.mjs <pkg-out dir>` (11a and 12; `--breaker` adds 11b and 11c). It serves the built `pkg-out` and drives headless Chromium at 1280×720 with `?low&debug`, plus the band query of each case. It reads `window.__game.state()` and the DOM. "Within N frames" means N increments of `state.frame`. Objects are found by `objs[].n` and `burning`, never by a fixed index. Before any check that needs the world to run, it ends I-020's hold the way I-020 test 8 does (`move 0 1 0.6`, then a cast that lands).
- Each test says how it fails on based_on, or on the I-020 merge commit where it depends on I-020.

1. **Keys cover every caption** (core).
   - `CAPTION_KEYS` has 26 entries and no duplicates, and contains `cast` and no `cast_*` key.
   - `spell_id(0..=3)` gives `fire`, `water`, `frost`, `co2`, and `spell_id(4)` gives `""`.
   - `event_key` maps every `EventKind` variant to the key in the §1 table. The test lists all 21 variants explicitly (every `Fizzle` and every `Extinguished(_)` arm, plus `Station`), so adding a variant without a key fails to compile.
   - `CaptionKey::ALL.map(|k| k.as_str())` equals `CAPTION_KEYS`, in order. `CaptionKey::default()` is `Start`.
   - No source grep: `say` and `say_arg` take a `CaptionKey`, so a keyless or misspelled call site does not compile.
   - Fails today: `captions.rs` does not exist.
2. **Keyed event lines** (core).
   - Build a `chem::World` in its start state, as I-020 test 2 does. Step it until `step` returns an uncaptioned spread `Ignited` on a non-greenhouse target T. `event_lines_keyed` with `caption_age = 0.0` returns a `Line` with `key == CaptionKey::Spread`, `arg == <T's name>`, and `text` equal to I-020's `event_lines` text for the same input.
   - Keep stepping without casts until the greenhouse ignites. That event gives `key == CaptionKey::GreenhouseCaught` and `arg == ""`.
   - An uncaptioned `BurntOut` with `caption_age = 2.0` gives `key == CaptionKey::BurntOut`.
   - `Reseeded` gives `key == CaptionKey::Reseeded`.
   - Fails today: there is no `event_lines_keyed`.
3. **Apprentice text unchanged** (core). I-020's `caption-strings.txt` test passes unchanged. Every snapshot line is still in `chem.rs`, `main.rs` or `lines.rs` byte for byte. **Mutation:** changing one character of any cast caption fails it. This is a guard: it passes before and after.
4. **`band.js`** (web).
   - `bandFromSearch('?band=sprout')` is `'sprout'`, `'?low&band=apprentice'` gives `'apprentice'`, and `'?band=adept'` gives `'adept'`.
   - `'?band=SPROUT'`, `'?band=adult'`, `'?band='` and `''` each give `null`.
   - `createBand({search:'', doc})`: `get()` is `'seedling'`, and `doc.documentElement.dataset.band` is `'seedling'`.
   - `set('x')` changes nothing and calls no callback. `set('seedling')` (the current band) calls no callback.
   - `set('sprout')` changes it and calls each `on` callback exactly once.
   - Nothing touches `localStorage` or `sessionStorage`: the fake globals throw on any access.
   - `PICKER_BANDS` deep-equals `analytics.js`'s `BANDS` (one assertion); `LEVELS` is exactly `['sprout', 'seedling', 'apprentice', 'adept']`.
   - Importing `band.js` with no `document` or `window` defined does not throw.
   - `known()` is false for `search:''`, true for `'?band=sprout'`, and true after `set('sprout')`.
   - Fails today: no `band.js`.
5. **`captionFor` and the pinned tables** (web).
   - `captionFor({caption:'X', captionKey:'soaked'}, 'apprentice')` is `'X'`, and so is the same call with `'adept'`.
   - With `'sprout'` it is `Wet now! Wet bushes are hard to burn.`
   - `captionFor({captionKey:'spread', captionArg:'dry hedge', caption:'…'}, 'seedling')` is `Heat from a fire made the dry hedge next to it catch fire!`, and with `'sprout'` it is `Heat spread! The dry hedge caught fire!`
   - An unknown key with `'sprout'` gives `state.caption`.
   - For each `SAME_AS_RUST` key, `captionFor({captionKey:k, caption:'R'}, 'seedling')` is `'R'`, and `CAPTIONS[k].seedling` is undefined.
   - Casts: `captionFor({captionKey:'cast', captionArg:'co2', caption:'R'}, 'sprout')` is `This gas keeps air away.`; with `captionArg:'water'` and `'seedling'` it is `'R'`; with `captionArg:'salt', spellName:'Salt'` it is `'Salt'` for sprout and for seedling, and `'R'` for apprentice.
   - **Banned words (a check that is not a copy of the table):** no sprout text in `CAPTIONS`, `CAST_CAPTIONS`, `SLOTS`, `warningFor`, `lostFor` or `COACH_TEXT.sprout` contains (case-insensitive, as a whole word) `oxygen`, `ignition`, `temperature`, `thermal`, `nitrogen`, `methane`, `carbon`, `dioxide`, or any whitespace-separated token of an `index.html` `.formula` text (read at test time) that contains a digit or a subscript digit (for example `CH₄`, `N₂(l)`, `−196`; plain words such as `cools` are not banned). **Mutation:** setting `CAPTIONS.ignited.sprout` to `Hot enough: its ignition point!` fails it.
   - Every cell of the §4 caption and `CAST_CAPTIONS` tables ("Rust" cells excepted, as above), the §4 slot table, the `warningFor` / `lostFor` table (`warningFor({ghTemp:150}, b)`; apprentice gives `The greenhouse is heating up: 150 °C. Wood can catch fire at about 300 °C!`) and the §5 coach table equals the pinned text exactly, and so does every sprout icon.
   - `coachFor({onboard:'walk'}, 0, 0)` (no band) still gives I-020's `Hold W to walk forward.`, and with `'sprout'` it gives `Hold W to walk!`. With `'apprentice'` or `'adept'` it gives `Hold W to walk forward.` (I-020's text).
   - For each step, `coachFor(..., 'sprout').icon` equals `COACH[step].icon`. `COACH_TEXT` has no `apprentice` or `seedling` key.
   - Fails today: no `captions.js`, no `COACH_TEXT`.
6. **Coverage and reading rules** (web).
   - The test reads `crates/bevy-client/src/captions.rs` at test time and extracts the `CAPTION_KEYS` literals with a regex. `CAPTIONS` has exactly those keys minus `cast`, none missing and none extra. Every entry has `sprout`; every entry has `seedling` except exactly the 15 `SAME_AS_RUST` keys. `CAST_CAPTIONS` has exactly `fire`, `water`, `frost`, `co2`; each has `sprout`, and each has `seedling` except `water`.
   - `checkReading` returns no violations for: every sprout and seedling text in `CAPTIONS`, `CAST_CAPTIONS`, `SLOTS`, `warningFor` and `lostFor`; every `COACH_TEXT.sprout` line (sprout rule); I-020's four `COACH` texts (seedling rule); and, under the seedling rule, the 16 Rust texts shown at seedling (the 15 `SAME_AS_RUST` keys and the Water cast). The test holds those 16 as fixtures and first checks that each is a line of `docs/loop/items/I-020/caption-strings.txt` (so a Rust edit that breaks the fixture fails here and in test 3).
   - Under the seedling rule, the Rust texts of the 10 keys with a seedling cell **fail** (for example the boot `start` text and the `steam` text), so a cell is not kept where the Rust text would do and not dropped where it would not.
   - `checkReading('Heat spread! The {who} caught fire!', 'sprout')` counts 7 words, and a 7-word sprout text plus `{who}` that would reach 9 words at runtime fails.
   - Every sprout caption (and every `CAST_CAPTIONS` sprout entry) has an icon in `ICONS`.
   - **Mutations** (each one, applied to a copy of the tables in the test, must make it fail):
     - sprout: `CAPTIONS.soaked.sprout` = `Soaked: it must dry out (boil off at 100 °C) before it can burn.` (too many words, a 100 and a `°`);
     - sprout: `CAPTIONS.won.sprout` = `You did it! The courtyard is safe!` (a 9-letter word that is not `greenhouse`);
     - seedling: `CAPTIONS.steam.seedling` = `Hot water boils at 100 °C (373 K): H₂O(l) → H₂O(g).` (a formula and two temperatures);
     - seedling: `CAST_CAPTIONS.fire.seedling` = `The Fire spell burns gas thermodynamically.` (a 17-letter word).
   - Fails today: no `captions.js`.
7. **Every new text is in the register** (web). The test reads `content/science-sources.md` at test time and collects the backticked text of every `## Claims` and `## Not claims` row (normalized as the lint does: subscripts to digits, `−` to `-`, collapsed whitespace). Every sprout and seedling text in `CAPTIONS`, `CAST_CAPTIONS`, `SLOTS`, `warningFor`, `lostFor` and `COACH_TEXT.sprout` is in that set, whether or not the lint would flag it. `npm test` (the lint, including the stale check after the C-030/C-033 `where` edits) passes. `git diff <based_on>... -- content/science-sources.md` removes no row. Fails today: none of the texts exist.
8. **HUD wiring on a fake DOM** (web). Run `startHud`'s band, caption and slot setup against a fake document whose hotbar is built from the slot spans **read from `crates/bevy-client/web/index.html` at test time** (regex over the `.slot` buttons). The test fails loudly if the regex finds other than exactly four `.slot` buttons, each with one `.name` and one `.formula` span, so an `index.html` edit cannot silently pass.
   - With search `?band=sprout`, the `.formula` texts are `hot flame | cools fire | super cold | blocks air`, and the names are `Fire | Water | Frost | Smother`. A write spy on the fake spans records no formula text written after `startHud()` begins (the capture reads, never writes, the `index.html` text before the first sprout write).
   - `window.__band.set('apprentice')` restores every `.name` and `.formula` to exactly the `index.html` texts read at test time.
   - A fake state `{captionKey:'soaked', caption:'X', captionId:1}` under sprout puts `Wet now! Wet bushes are hard to burn.` in `#caption` and in `history`, with a `.cap-icon.icon-water` child. The same under apprentice puts `X`.
   - A fake state with `outcome:'lost'` under sprout gives the banner `.small` `The fire spread. Press R to try again!`
   - `slotText('sprout', 4, {name:'Salt', formula:'NaCl'})` is `{name:'Salt', label:''}` (a slot with no `SLOTS` row shows its name only).
   - **Mutations** (each must fail the test): deleting the slot capture so `startHud()` writes sprout first; and restoring `.formula` on `set('apprentice')` from a hard-coded string (`CH₄ + 2O₂ → CO₂ + 2H₂O` and the other three) instead of the captured DOM text, with the fake `index.html` slot texts altered at test time so a hard-coded copy cannot match.
   - Fails today: there is no band.
9. **The I-005 hook, on a fake DOM** (web; no by-hand alternative). `age.test.mjs` sets fake `window`, `document` and `navigator` globals:
   - a `meta[name="rg-analytics"]` with content `on`, and `navigator.doNotTrack = '0'`;
   - `#age` with the three `button[data-band]` and a `#game`;
   - a `window.__band` spy, and `window.__gameState` / `window.__game` fakes;
   - `globalThis.setInterval` stubbed to a no-op (restored after), so the session poller does not keep the test alive;
   - a `navigator.sendBeacon` spy;
   - for I-022's picker code (merged before this item): `document.pointerLockElement = null`, a `document.exitPointerLock` stub, `document.addEventListener` / `removeEventListener` that accept `pointerlockchange`, `#stage`, `#hud`, `#loading` and `#pause` fakes with a writable `inert`, and a fake `localStorage` (I-022's `start` reads the daily id from storage once). The lane copies the fake-DOM shape from I-022's own `age`/analytics tests if they have one, rather than inventing another.
   It imports `age.js`, calls `startAgePicker()` (it returns `true`), and clicks the "4 to 7" button.
   - `__band.set` was called exactly once, with `'sprout'`, and **before** the first `sendBeacon` call.
   - `sendBeacon` received a `session_start` body whose `band` is `sprout` (B7 still holds).
   - Clicking again calls neither spy again (`picked` guard).
   - **Mutation:** deleting the hook line fails the test.
   - Fails today: `age.js` has no hook.
10. **Panel band plumbing** (contract; `tests/e2e/panel/band.test.mjs`).
    - `persona('6yo').band` is `'sprout'`, `persona('9yo').band` is `'seedling'`, `persona('13yo').band` is `'apprentice'`, and `'teacher'` and `'qa'` (adults) are `'adept'` (adept reads like apprentice in this item, so their screens do not change).
    - The pure `pageUrl(base, query, band)` in `personas.mjs`: `pageUrl('http://h/', '?low', 'sprout')` is `'http://h/?low&band=sprout'`; `pageUrl('http://h/', '', 'seedling')` is `'http://h/?band=seedling'`; `pageUrl('http://h/', '?low&band=apprentice', 'sprout')` keeps the operator's band (`'http://h/?low&band=apprentice'`); `pageUrl('http://h/', '?low', 'none')` is `'http://h/?low'`.
    - The existing `score`, `report` and `harden` tests stay green.
    - Not unit-tested (the seam breaker runs it once): `panel.mjs start --band <level|none>` (default `persona(id).band`) passes `--band` to `daemon.mjs`, which opens `pageUrl(...)` and adds `band` to its `start` record. `query`, `session.json`, `score.mjs`'s identical-query rule and `result.schema.json` are unchanged.
    - Fails today: there is no `band`.
11. **E2E: sprout on screen** (contract; `?low&debug&band=sprout`). Three separate tests. **Only 11a gates the merge** (with test 12). 11b and 11c need a long tick loop that depends on I-020's hold and the world's determinism, so they are **seam-breaker checks**: the breaker runs them and records the result, and a failure is a finding for the planner, not a merge block. In each, no visible text node in `#hud` matches `/[₀-₉]|°|→|\b[A-Z][a-z]?\d/` at any checkpoint.
    - **11a: slots, hint and caption icon.**
      - `html[data-band]` is `sprout`.
      - The `.slot .formula` texts are `hot flame | cools fire | super cold | blocks air`.
      - At the first state, `#hint` (or I-020's coach text node) is `Hold W to walk!`, and `#coach` is visible with class `step-walk` and its icon (the sprout coach keeps I-020's icon).
      - End the hold, then `face <non-burning bush>`, `select 2`, `cast`. After `impacts` rises, `#caption` text is `Wet now! Wet bushes are hard to burn.` and `#caption .cap-icon.icon-water` is visible with a box of at least 32×32 px.
    - **11b: history line after a spread.** End the hold. Then step `tick 100` repeatedly (at most 60 times) until `state.captionKey === 'spread'`. The world start state is deterministic (I-020 test 2 relies on it), so this is a fixed path, not a branch. The test **fails** if `greenhouse_caught` or the step cap comes first. Then open the pause card: `#history` holds a line matching `/^Heat spread! The (bush|dry hedge) caught fire!$/`.
    - **11c: warning and lose text.** End the hold. Step `tick 200` until `#warning` is visible with `outcome === 'playing'`: its `.text` is `The greenhouse is getting hot!`. Step on until `outcome === 'lost'`: the banner `.small` is `The fire spread. Press R to try again!`. Then run the no-formula scan over the whole `#hud`.
    - Fails on the I-020 merge commit: the slots show formulas, and `#caption` shows the C-019 text.
12. **E2E: apprentice and default** (contract).
    - **`?band=apprentice`** (and the same four checks once more with `?band=adept`, which must give identical text):
      - after a cast, `#caption` equals `state.caption`;
      - the four slot `.name` / `.formula` texts equal those in the served `index.html`, fetched and parsed at test time;
      - the `#warning` text, once visible, matches `/^The greenhouse is heating up: \d+ °C\. Wood can catch fire at about 300 °C!$/`;
      - the first `#hint` is I-020's `Hold W to walk forward.` (apprentice uses I-020's coach text).
    - **No `?band`:**
      - `html[data-band]` is `seedling`;
      - after a Water cast that puts out a fire, `#caption` equals `state.caption` (`out_water` is a `SAME_AS_RUST` key, so seedling shows the Rust text);
      - after a Water cast at a non-burning bush the cast caption is `state.caption` too (`CAST_CAPTIONS.water` has no seedling field), and after a Frost cast it is `Frost is a very cold liquid. It pulls heat away fast.`;
      - the first `#caption` (boot) is `Put out the 3 fires! Aim at a fire and click.`;
      - the first `#hint` is I-020's `Hold W to walk forward.`
    - Tests 11 and 12 run with default settings (Hints on). With Hints off, I-022's `hintAllowed` hides every coach text, sprout included; that is I-022's and I-020's behaviour and is not re-tested here.
    - **Picker hook live:** on the no-band page, `window.__band.known()` is false; `window.__band.set('sprout')` re-labels the slots, switches `#hint` to the sprout coach text within 2 frames, and calls a callback registered with `window.__band.on`; `known()` is then true.
    - Fails on the I-020 merge commit: there is no `data-band`, and the captions are the C- texts.
13. **Captions stay true at every level (lint guard).** `npm test` passes with 0 unsourced client claims and 0 stale rows. Guard: green before and after.
14. **No string drift between Rust and the tables.** `captions.js` has no copy of any Rust caption (apprentice and the `SAME_AS_RUST` seedling cells come from `state.caption`), and no copy of an `index.html` slot formula. The test greps `captions.js` for each `caption-strings.txt` line and each `index.html` `.formula` text and finds none. **Mutation:** adding `seedling: { text: 'That does not burn.' }` to `fizzle_not_flammable` fails it.
15. **Science review gate (model check, before merge).** A fresh science reviewer, a model that wrote none of these texts, follows `docs/loop/science-review.md` over every row this item adds or edits.
    - It is given each **register row** (claim, verdict, note, sources, `where`), not only the string, so a wrong `simplified` note or a wrong source is caught too.
    - It checks that each sprout and seedling text is true as stated, and that each `simplified` note names what is left out and why the text is not false.
    - Pass: 0 `error` verdicts, and every new text sourced.
    - The contract lane records the review in `round-1.md`.
    - **Fallback:** if `docs/loop/science-review.md` is missing on the merge commit, the contract lane writes the reviewer prompt it used (scope, verdict scale, tolerance rules) into `round-1.md` before dispatching the reviewer.
    - A text the reviewer finds false is rewritten under §3 and re-reviewed. It does not ship with an `error` verdict.
16. **B1 and existing tests not regressed.** After `build-web.sh`, `QUERY=?low node tests/e2e/golden.mjs` passes 3 of 3 runs with 0 console errors. Its only edit is `I021_KEYS = ['captionKey', 'captionArg']` appended at the key-order lines (`golden.mjs:1043-1047` at c4832d2, after I-025's and I-020's lists), because golden pins the exact state key list. At c4832d2 golden reads `#hint` (I-019's idle and locked hints, `golden.mjs:864`, `1316`) and `#obj-line`, which this item does not change at the default band; it reads no `#caption`, `#warning`, `.formula` or `#history` text. With no `?band`, golden runs at `seedling`. Golden must assert on `state.caption` (Rust, unchanged), never on HUD text; if any golden check reads `#caption`, `#warning`, `.formula`, `#hint` or `#history` text, the builder stops and reports (golden is not edited).
    - **All other tests:** the planner's pre-dispatch grep (step 8) fixes the list of existing tests that assert on `#caption`, `#warning`, `.formula`, `#hint` or `#history` text, and the fix for each, in `round-1.md`'s header. At c4832d2 the list is empty (the hits are band-free hint reads and the panel's reader). The contract lane applies only the listed fixes (a move to `?band=apprentice`, or a switch to `state.caption`). Every listed test passes after the merge.
    - The seam breaker re-runs the grep on the merge commit and confirms nothing was missed.
17. **B2/B3 guard.** `node tests/e2e/perf.mjs` at 4x throttle with `?low` (no band), following I-020 test 13's method:
    - noise floor from two batches of 3 runs at the I-020 merge artifact (I-024's `perf-pairs.mjs` may drive the runs if it has merged; both artifacts must be built with the same `Cargo.toml` profile, which I-024 may change, so if I-024 merges between the two the planner moves the comparison base first);
    - 3 interleaved runs each of this artifact and that one;
    - limits are max(budget, 2 × noise floor) with budgets median CPU p50 +1.0 ms and TTI +0.5 s. Brotli total is limited to +20 KB (deterministic);
    - every run has `workload_ok`.
    - `band.js` and `captions.js` are ES modules imported by `hud.js` (as I-020's `coach.js` is). The two extra small same-origin module requests (about 4 KB together, Brotli) are accepted and are counted inside the TTI budget above.
18. **Scope.** `git diff --name-only <based_on>...` lists only the paths in the Lanes table, the existing test files listed in `round-1.md`'s header by pre-dispatch step 8, and `docs/loop/items/I-021/**`.
    - `git diff -U0 <based_on>... -- crates/bevy-client/web/age.js` has exactly one `+` line, `window.__band?.set(b.dataset.band ?? '');`, and no `-` line.
    - `index.html`, `analytics.js`, `boot.js`, `chem.rs`, `world.rs`, `cues.rs`, `fx.rs`, `Cargo.toml`, `perf.mjs`, `crates/sim/`, `content/reactions.json`, `functions/`, `dashboard/` and `result.schema.json` do not change. `golden.mjs` changes only at the key-order lines (`I021_KEYS`).
    - In `hud.js`, `setHint`, `hintAllowed` (I-022), `pickHint` and the finder code (I-025) are not edited.
    - I-020's `COACH` table and its tests are unchanged.

## GS walk (screens, texts and effects this item touches)

Source: `docs/loop/gameplay-standards.md` at c4832d2 (unchanged since a3e50b1).

| GS | P | status |
|----|---|--------|
| 6 | P0 | met: the icons are static; no new animation |
| 14 | P1 | not applicable by I-005's rule: the band is never stored (shared devices). The URL and the picker set it each load |
| 17 | P0 | improved: every sprout result caption has a big icon (star on a win and on `out_fire`) |
| 18 | P0 | partial: caption icons are at least 32 px. Text sizes are unchanged (I-007 owns them) |
| 19 | P0 | met: the icons differ by shape. Slot names stay alongside slot icons |
| 20 | P1 | unchanged: captions are not speech, and there is still no audio-only cue |
| 21 | P0 | **partial**: reading levels with enforced rules (test 6); every sprout caption has an icon plus short text; captions stay 8 s, can be replayed with C and are in the pause history. **No speech yet**: a pre-reader still cannot hear the sentence. Follow-up filed (pre-dispatch step 6): speak sprout captions with I-028's speech helper once it merges |
| 24 | P0 | kept: the warning stays specific at every band. The sprout warning has the meter bar in place of a number |
| 26 | P1 | met: the labels describe, never shame. "Yay!" is sprout only. Nothing says "baby" or "kid". The apprentice text is the full science |
| 27 | P0 | met: the copy is friendly. The apprentice text is unchanged for teens |
| 30 | P0 | kept: the spell choice is still the decision. Seedling and sprout name spell 4 for its mechanism (`Smother`) |
| 31 | P1 | improved: meaningful at each age (cools, keeps air away, too wet to burn) |
| 32 | P1 | **met**: no formula or temperature for 4–7, and at most one number for 8–10, each next to the visible effect and its icon |

Not touched: GS-1 to GS-5, GS-7 to GS-13, GS-15, GS-16, GS-22, GS-23, GS-25, GS-28 and GS-29. There is no new control, camera, effect, timer or button.

## Out of scope

- Any change to Rust caption text, chemistry, casting, the coach logic or the hold (I-019 and I-020).
- A pause-card reading-level switch. **Named follow-up, filed by the planner in `backlog.md` at dispatch:** "Pause-card reading-level switch (sprout / seedling / apprentice), not stored" (B4; blocked by I-021). Until then a DNT, GPC or analytics-off player gets `seedling`, and a reload brings the picker back on the deployed site.
- Remembering the band; showing the picker when analytics is off; any change to `age.js` beyond the one line, or to `analytics.js`.
- Band texts for I-007's `IDLE_HINT`, **I-019's `#obj-line` counter and pips** (sprout players will see `Fires to put out: 7` in words, with the pips beside it), I-019's locked and other hints, the pause-card controls table, and the win/lose banner `.big` lines. These stay band-free on purpose. They are short already. This is a follow-up if the panel flags them.
- The apprentice coach text: apprentice and adept use I-020's `COACH` unchanged.
- **Separate adept text**, and **adept in the picker or in analytics**. Adept reads like apprentice here. A fourth picker button and a fourth analytics band value change what data is collected, which is human-only (`target.md`): named follow-up for Paawan, "Adept (16+) in the age picker and analytics bands" (B7, B4), filed by the planner at dispatch. Until then an adult on the deployed site picks "11 years old and up" and gets the same full text.
- The Formula Bench's own per-band texts (I-028). This item only gives it `LEVELS`, the one live band (`window.__band`), `SLOTS` names and `checkReading`. The I-028 / I-029 / I-026 / I-030 spec edits are planner changes (pre-dispatch step 7), not part of this diff.
- Band versions of `state.spellName` / `state.formula`, which nothing in the HUD shows.
- HUD font sizes (I-007), translations, and speaking captions aloud (follow-up, after I-028's speech helper exists; see GS-21).
- `docs/loop/panel.md` and `docs/loop/areas.md` edits in this item's diff. The planner adds `captions.rs` and `lines.rs` to `areas.md` before dispatch (pre-dispatch step 5), and files a follow-up to add one line on bands to `panel.md` once `areas.md` lists it.

## Contract

**New `__gameState` fields** (existing fields unchanged):

| field | type | meaning |
|-------|------|---------|
| `captionKey` | string, a member of `CAPTION_KEYS` | the key of `caption` |
| `captionArg` | string | the target name for `spread`, the spell id (`fire`, `water`, `frost`, `co2`, later I-026's ids) for `cast`, else `""` |

Both are appended after every key that merged before (I-019's, I-025's, I-020's), and golden's key-order check gains `I021_KEYS = ['captionKey', 'captionArg']`.

**Rust.**
- `crates/bevy-client/src/captions.rs` holds `CaptionKey` (with `ALL`, `as_str`, `Default`), `CAPTION_KEYS`, `spell_id` and `event_key` (§1). It is Bevy-free and may `use crate::chem`. `main.rs` adds `mod captions;` next to the other `mod` lines.
- `lines.rs` gains `Line` (key: `CaptionKey`) and `event_lines_keyed`. `event_lines` keeps its signature and output.
- `Game::say(key: CaptionKey, text)` and `Game::say_arg(key: CaptionKey, arg, text)`.

**Web modules** (ES modules, no DOM access at import; `build-web.sh`'s `cp` line adds `band.js captions.js`):
- `band.js`: `LEVELS`, `PICKER_BANDS`, `DEFAULT_BAND`, `bandFromSearch`, `createBand` (`get`, `set`, `on`, `known`);
- `captions.js`: `CAPTIONS`, `CAST_CAPTIONS`, `SAME_AS_RUST`, `SLOTS`, `slotText`, `ICONS`, `captionFor`, `warningFor`, `lostFor`, `checkReading`;
- `coach.js`: adds `COACH_TEXT` (sprout only) and the optional `band` parameter of `coachFor` (text changes for sprout only; icon never changes).

**The band interface (hard; I-028, I-029 and later items use exactly this).**
- `window.__band.get()` gives one of `LEVELS`; `adept` stays `adept` (consumers without an adept view map it themselves; I-029 sends `band adept` to Rust).
- `window.__band.set(b)` is the only writer: the URL at boot, the `age.js` hook, and the Bench chooser (I-028) all call it. Values outside `LEVELS` are ignored.
- `window.__band.on(cb)` calls `cb(band)` on every change. `window.__band.known()` says whether the band came from the URL or a `set` (not just the default).
- No consumer adds its own `#age` click listener or keeps its own band. Pure imports: `LEVELS`, `DEFAULT_BAND`, `bandFromSearch`, `checkReading`, `ICONS`, `SLOTS`.
- **`checkReading` checks every sprout and seedling string**, wherever it is held: the JS tables here, and JSON (`content/spells.json` `caption_sprout` / `caption_seedling` from I-026, I-028's `prompt_*`, I-030's questions, `reactions.json`'s `short_sprout` / `short_seedling`). The item that adds such a string adds the test. Note: the 2D engine's `crates/sim` `Tier` has no `adept` (`game.rs:152-164`, unknown strings fall to `Apprentice`).

**DOM and hooks** (all created by `hud.js`; `index.html` unchanged):
- `html[data-band]` (one of `LEVELS`);
- `#caption .cap-icon.icon-<name>` (sprout only);
- `window.__band = { get, set, on, known }` (the band interface below);
- `age.js` calls `window.__band?.set(band)` on a pick.

**Panel:**
- each `PERSONAS` entry gets `band`;
- `personas.mjs` gets `pageUrl(base, query, band)`;
- `panel.mjs` and `daemon.mjs` take `--band <level|none>`;
- the daemon opens `pageUrl(...)` and records `band` in the `start` record and in `session.json`.

**Contract stub** (landed on the branch base before the lanes start, so all three lanes build against it):
- `captions.rs` with `CaptionKey`, the full 26-entry `CAPTION_KEYS` list and `spell_id` / `event_key` bodies (they are the table; the stub makes them real);
- `captionKey` / `captionArg` published as `"start"` / `""`;
- empty `band.js` / `captions.js` exports with the signatures above.

### Overlap with I-020, I-022, I-024, I-025 and I-026

Line numbers are c4832d2's. Merge order: **I-022, I-025, I-020, I-021**, then I-028 (which reads `window.__band`). I-026 in either order relative to I-021.

| file | this item's hunks | other items' hunks | rule |
|------|------|------|------|
| `age.js` | one `+` line immediately before `start(b.dataset.band ?? '');` (`158`) | **I-022:** in `startAgePicker`, the lock release and `inert` set after `style(root)` (`105`), and in the same click handler the `pointerlockchange` removal and `inert` restore above `start(...)`; `analytics.js` `send`/`start` daily id | I-022 first. The hook sits between I-022's last added line and `start(...)`, and touches no I-022 line. Test 9's fake DOM covers I-022's calls |
| `hud.js` | `showCaption` caller and `history` push (`682-689`), `#warning .text` (`696`), lose `.small` (`707`), slot capture and writes in `startHud()` (`118`), one `import` block, the `coachFor(..., band.get())` argument | **I-022:** `setHint` and `hintAllowed` (`441-444`). **I-025:** `pickHint` (moved out of `665-671`), `findOn` (`673`), the finder, `window.__game.pickHint` (`230-238`). **I-020:** coach DOM, the wrapped `setHint(coachText \|\| pickHint(...))` call, pointer code, first-state branch | No shared lines. This item passes the band into I-020's `coachFor` call and touches nothing else in the hint line |
| `coach.js` | `COACH_TEXT` and `coachFor`'s optional 4th parameter | **I-020:** whole file (new) | I-020 first; `COACH` is not edited |
| `main.rs` | `mod captions;` (`15-23`), `Game` fields (`137-139`), `say` (`182-186`), the 5 call sites (`636`, `892`, `1098`, `1277`, `1343`), `react()` event-line call (`1320-1339`, which I-020 turns into an `event_lines` call), format string end (`1580-1581`) and args | **I-022:** `Settings`, `read_settings`, `aim_and_cast` `952-987`. **I-025:** `Game` fields, stream code, `setup` `507-516`, `aim_and_cast` `1036-1136` (contains the cast `say` at `1098`), format string (`fireInView`). **I-020:** `mod onboard; mod lines;`, `chem_tick`, `projectiles` `1271-1287`, `react()` event lines, format string. **I-024:** `world::build` call (`464`) | Keys append to the format string in merge order. At `1098` this item changes only the `say(...)` arguments wherever I-025 leaves that call. If I-025 splits the cast caption into more than one `say`, each becomes `say_arg(CaptionKey::Cast, spell_id(..), ..)` |
| `lines.rs` | `Line`, `event_lines_keyed`; `event_lines` becomes a thin map | **I-020:** whole file (new) | I-020 first; its tests stay green unchanged |
| `golden.mjs` | `I021_KEYS` at the key-order lines (`1043-1047`) | **I-025:** `I025_KEYS` at the same lines. **I-020:** `I020_KEYS` at the same lines. **I-022:** `reviewFollowups` | Append after I-020's list; nothing else |
| `build-web.sh` | `cp` line `21` (adds `band.js captions.js`) | **I-020:** same line (`coach.js`). **I-024:** none (it edits `Cargo.toml` profiles, `world.rs`, `perf.mjs`) | Append to the line after I-020 |
| `chem.rs` (read only), `captions.rs` | `event_key` matches on `chem::EventKind`; `spell_id(i)` maps the `Spell::ALL` index to the id | **I-026:** moves `name`/`formula`/`cast_caption` from `chem.rs` to `spells.rs` with the same signatures, adds `Spell::spell_id()`, keeps `EventKind`; `main.rs:1098` compiles unchanged | **Either order.** If I-021 first: I-026 swaps `spell_id(i)`'s body for `Spell::ALL[i].spell_id()` and later moves `CAST_CAPTIONS` into `spells.json` (planner step 7). If I-026 first: the core lane writes that body directly. Test 1 lists all 21 `EventKind` variants, so a dropped or added variant fails to compile |
| `content/science-sources.md` | new rows; `where` edits on C-030 and C-033 | **I-020:** new rows (C-040 or next free). **I-022, I-024, I-025:** none planned | Ids are taken at build time from the merged register; add rows, remove none |

## Lanes

The three lanes build against the stub independently. Web tests use fake state and a fake DOM. Core tests are Rust unit tests. The contract e2e is red until core and web merge, and is first run then. Merge order: core, web, contract.

| lane | owns | builds |
|------|------|--------|
| core | `crates/bevy-client/src/main.rs` (`say` / `say_arg`, the call sites, the `publish` fields, `mod captions;`), `crates/bevy-client/src/captions.rs` (new; added to `areas.md` `client-core` by the planner), `crates/bevy-client/src/lines.rs` (`Line`, `event_lines_keyed`, `event_lines` as a thin map) | §1, tests 1 to 3. Must not change any caption string or `chem.rs` |
| web | `crates/bevy-client/web/hud.js`, `style.css`, `band.js`, `captions.js`, `coach.js` (`COACH_TEXT.sprout` and the `band` parameter only), `crates/bevy-client/web/*.test.mjs` (new: `band`, `captions`, `hud-band`, `age`), `crates/bevy-client/web/age.js` (the one hook line only; reviewer: the I-005 main-lane breaker), `crates/bevy-client/build-web.sh` (the `cp` line only), `content/science-sources.md` | §2 to §6, tests 4 to 9 and 13 to 14. The caption icons. The slot capture and restore. `warningFor` / `lostFor` wired into `hud.js`. Every register row and the C-030/C-033 `where` edits |
| contract | `tests/e2e/bands.mjs`, `tests/e2e/panel/personas.mjs`, `tests/e2e/panel/daemon.mjs`, `tests/e2e/panel/panel.mjs` (the `--band` flag and argv helper only), `tests/e2e/panel/band.test.mjs`, and the existing e2e / web test files listed by pre-dispatch step 8 (the band query or `state.caption` read only; never `perf.mjs`; none at c4832d2), and `tests/e2e/golden.mjs` (the key-order lines only: `I021_KEYS`) | tests 10, 11a and 12 (11b and 11c are written here and run by the seam breaker), and the `I021_KEYS` edit. It runs 15 (it dispatches the reviewer), 16 and 17 and records them in `round-1.md` |

**Shared files.** `main.rs`, `hud.js` and `coach.js` are also edited by I-019 (merged), I-022, I-025 and I-020, which all merge first. The lanes rebase onto them and change only the blocks named above (see the overlap table). If the merged I-020 `react()` still holds an event-line condition outside `event_lines`, or the merged `hud.js` builds the caption, warning or lose text somewhere other than one place each, the builder stops and reports in `round-1.md`, and the planner refreshes this spec.

**Seam.** The seam breaker checks:
- every key `main.rs` can publish has a `CAPTIONS` entry. It drives every `EventKind` once in the e2e page, or uses test 1's list against test 6's;
- test 16's grep of existing tests that read HUD text is complete, and each listed test passes;
- tests 11b and 11c (`bands.mjs --breaker`) pass; a failure is reported to the planner;
- `panel.mjs start --band none` (and one banded persona) opens the expected URL and logs `band` in the daemon's `start` record;
- no other module adds a listener on `#age button[data-band]` (grep), and `window.__band` has `get`, `set`, `on`, `known`;
- `captionArg` is only ever a target name;
- a band change mid-play never shows a mixed-band hotbar;
- the deployed `web/dist` (`npm run build:site`, `wrangler pages dev`) shows the sprout slots after the "4 to 7" pick. This is a breaker smoke run on top of test 9, not a replacement for it.

## Critic requirements carried from I-020 (checklist)

| requirement (I-020 critique / hand-off) | where met |
|---|---|
| "string → key" table for every `format!` and `say` in `react()` and `reset()`; `greenhouse_caught` and `station` described | §1 table; tests 1, 2 |
| `warningFor` / `lostFor` pinned with current strings, or dropped | §4 table (pinned; apprentice = current text, moved verbatim); tests 5, 12 |
| I-005 hook tested by a node test on a fake DOM, no by-hand alternative; I-005's lane named reviewer of the `age.js` line | §2; test 9 (with mutation); Lanes (reviewer); test 18 (one-line diff) |
| `SLOTS.apprentice` test reads `index.html` at test time | §4 (captured from the DOM, no copy); tests 8, 12, 14 |
| `docs/loop/panel.md` added to `areas.md`, or the paragraph dropped | dropped (see "Changes from I-020 rev 1"); follow-up filed by the planner |
| panel B4 stated as "if the band is known", plus one 6yo run with no `?band` as a guard | Bars and expected move; `--band none` (test 10) |
| reading-rule mutations for both sprout and seedling | test 6 (two each) |
| I-021 critique rev 1, items 1 to 9 | see "Planner response" in `critique.md`; all accepted |
| I-021 critique of the roadmap-v2 refresh, checklist items 1 to 9 | see "Planner response" in `critique.md`; all accepted except the seedling `warningFor` cut (rejected: the apprentice line fails the seedling rule); speech (item 3) taken as a filed follow-up with GS-21 downgraded to partial |
| claims moved from I-020: median clarity 6 → ≥ 7; no text blocker about formulas or "100 degrees" | Bars and expected move |
| captions stay scientifically true at every level; new texts have register rows | §4 checked texts, §6, tests 7, 13, 15 |

## Refresh (roadmap v2)

Stale check against origin/main c4832d2 (I-019 #29, wave-7 state #30, roadmap v2 #31). `git diff a3e50b1..c4832d2` touches `main.rs`, `hud.js`, `style.css`, `chem.rs` (I-018's test only), `fx.rs`, `player.rs`, `world.rs`, `Cargo.toml`, `golden.mjs`, `perf.mjs`, `content/science-sources.md` and new `cues.rs`, `clock.rs`, `motion.rs`. It does not touch `age.js`, `analytics.js`, `boot.js`, `index.html`, `build-web.sh`, `tests/e2e/panel/**` or `test/**`. Every caption string is unchanged, `EventKind` still has 21 variants, and C-030 / C-033 keep their ids and text. Scope did not grow: no new caption key, no new text row, no new lane. Changes:

1. **based_on** a3e50b1 → c4832d2. Status: I-019 has merged; the item still waits for I-020, which itself waits for I-022 and I-025. Merge order is now stated: I-022, I-025, I-020, I-021, and I-021 before I-026.
2. **Four reading levels (roadmap v2).** Sprout 4-7, Seedling 8-10, Apprentice 11-15, Adept 16+. `band.js` now has `LEVELS` (four) and `PICKER_BANDS` (the three the picker and `analytics.js` know; the old `BANDS` check moved to it). `adept` reads exactly like `apprentice` (full Rust text, `index.html` slots, I-020's coach text), so no new text is written. `?band=adept` works; the picker and analytics stay three-band, because a fourth analytics value changes what is collected (human-only). Named follow-up for Paawan, filed at dispatch (pre-dispatch step 6, Out of scope). `checkReading`, `captionFor`, `coachFor`, the slot and warning/lose tables, tests 4, 5, 10 and 12, and the Contract now name `adept`. The panel's teacher and qa personas move from `apprentice` to `adept`; their screens do not change.
3. **Reusable by the Formula Bench (I-028).** `band.js` and `checkReading` are stated as the shared band store and reading-rule checker. I-028 imports them and reads the live band from `window.__band`; it does not fork them. `checkReading` returns a violation for an unknown level instead of passing. A Contract note records that `crates/sim` `Tier` has no `adept` and that `reactions.json`'s `short_sprout` / `short_seedling` should pass `checkReading`. B9-B11 are named as not moved.
4. **Golden key list.** Golden pins the exact state key list (`golden.mjs:981`, `1043-1047`), so the new `captionKey` / `captionArg` keys would have failed it. The keys are appended at the end of the format string, the contract lane appends `I021_KEYS` (the same sanctioned edit as I-019, I-025 and I-020), and tests 16 and 18 and the Lanes table say so (rev 2 said golden does not change). Pre-dispatch step 5a pins the insertion point.
5. **Overlap with I-022.** I-022 edits `age.js`'s click handler (the `pointerlockchange` removal and the `inert` restore above `start(...)`) and gates `#hint` with `hintAllowed`. The one-line hook still goes immediately before `start(...)` and touches no I-022 line. Test 9's fake DOM gains what I-022's picker code calls (`pointerLockElement`, `exitPointerLock`, `inert`, `localStorage`). Tests 11 and 12 state that they run with Hints on.
6. **Overlap with I-020, I-024, I-025 and I-026.** A new table under Contract names each shared file, the hunks, and the order. The cast `say` at `main.rs:1098` sits inside I-025's `aim_and_cast` rework; only its arguments change. Test 17 notes I-024's `perf-pairs.mjs` and the shared `Cargo.toml` profile. Test 18's must-not-change list adds `world.rs`, `cues.rs`, `fx.rs`, `Cargo.toml`, `crates/sim/`, `reactions.json`, and the I-022/I-025 `hud.js` functions.
7. **I-020 contract.** The coach text reaches `#hint` through `setHint(coachText || pickHint(...))`, so I-022 gates it; I-020 adds `onboard.rs` too, which pre-dispatch step 5 now lists for `areas.md`.
8. **Line numbers and paths** updated to c4832d2: `main.rs` `say` 182-186, fields 137-139, boot caption 636-638, `reset` 892, cast 1098, `projectiles` 1275-1278, `react()` 1292-1349 (spread block 1320-1339, won 1341-1348), `publish` 1546 and format 1580-1581; `chem.rs` `cast_caption` 88-102, `EventKind` 108-136, `Fizzle` 138-148, `caption` 201-249; `hud.js` `startHud` 118, `history` 142-143 / 221-227, `showCaption` 398-403, `C` 424, caption block 682-689, warning 696, lose 707; `age.js` 104, 149-161, 158; `analytics.js:20`; `boot.js` 27, 35; `index.html` 28-31, 59/71/84; `build-web.sh:21`.
9. **Facts added:** I-019's history repeat rule (the band text is compared the same way); the win `.small` now has text (a Not-claims row) and stays band-free; the picker requires exactly three buttons.
10. **Roadmap v2 check.** The item is HUD text in the first-person client; it adds no camera or control and fits the hard rule. It does not touch the core loop (Bench, worked casts, mastery: I-028 to I-030).

