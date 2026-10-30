# I-021: Captions at the player's reading level (age bands)

**Revision 2** (after `critique.md`). Changes: seedling cells equal to the Rust text are no longer copied (they fall back to `state.caption`); caption keys are a Rust enum, so a keyless `say` does not compile; existing e2e and web tests that read HUD text are moved to `?band=apprentice` or `state.caption`; `{who}` counts as two words and the sprout `spread` line is reworded; `COACH_TEXT.apprentice` is cut; sprout coach icons are explicit; `start` keeps the click instruction; test 11 is split in three; notes on `Smother` vs `CO₂`, band-free I-019 strings, `areas.md`, `science-review.md` and the pause-card follow-up.

**based_on:** origin/main `a3e50b12d25fd96c99d745c4defafb452ea6f6d8` (2026-10-30, after I-005 merged in #25 and I-007 in #24). The code was read there: `crates/bevy-client/src/{main.rs,chem.rs}`, `crates/bevy-client/web/{hud.js,age.js,analytics.js,boot.js,index.html}`, `crates/bevy-client/build-web.sh`, `scripts/build-site.sh`, `test/science-sources.test.ts`, `content/science-sources.md`, `tests/e2e/panel/{personas.mjs,daemon.mjs,panel.mjs,score.mjs,hud.mjs,result.schema.json}` and `docs/loop/gameplay-standards.md`. The I-019 and I-020 contracts come from their specs (`docs/loop/items/I-019/spec.md` rev 2 and `docs/loop/items/I-020/spec.md` rev 2). The pinned tables of I-020 revision 1 were recovered from the planner's session record, then corrected here (see "Changes from I-020 rev 1").

**Status: blocked.** Do not send this item to lanes until **I-019 and I-020 have both merged**. I-005 and I-007 have already merged. Builders branch from origin/main after the I-020 merge. Before dispatch the planner:
1. moves `based_on` to that commit and re-checks the facts below;
2. confirms that `docs/loop/items/I-020/caption-strings.txt` exists on that commit (I-020 writes it), because tests 3 and 6 reuse it;
3. confirms that the register ids of the two moved rows are still C-030 (`Heat spread from bush to bush. Press R to try again.`) and C-033 (the `#warning` text) on that commit, and renumbers them in §6 if I-019 or I-020 changed them;
4. confirms that `docs/loop/science-review.md` exists on that commit (it exists at a3e50b1). If it does not, test 15's fallback applies;
5. adds `crates/bevy-client/src/captions.rs` (new, this item) and `crates/bevy-client/src/lines.rs` (I-020), if not yet listed, to the `client-core` row of `docs/loop/areas.md`, as a planner change outside this item's diff, so the area check covers them and test 18 is not the only guard;
6. files the follow-up "pause-card reading-level switch" in `docs/loop/backlog.md` (see Out of scope). It is the only way a player with DNT, GPC or analytics off gets the sprout level.

**Stale check.** The planner refreshes this spec before building if any of the following differ from what is listed under "Relied on":
- I-005: the `age.js` click handler line `start(b.dataset.band ?? '');`, the band values `sprout` / `seedling` / `apprentice` in `analytics.js` `BANDS`, the rule that the band is never stored, or the lazy `import('./age.js')` in `boot.js` after `startHud()`;
- I-020: `coach.js` exports `COACH` and `coachFor(state, now, doneAt)`; `lines.rs` has `event_lines(...)` and `react()` only applies its results; `caption-strings.txt` and its Rust test exist;
- I-019: the boot caption `Save the Greenhouse! Put out 3 fires before they spread. Water is ready: aim at a fire and click.`;
- `hud.js` still shows captions through one `showCaption(state.caption)` call with `history.unshift(state.caption)`, and still builds the `#warning` text and the lose banner's `.small` inline;
- `index.html` hotbar slots still carry `.name` and `.formula` spans;
- `daemon.mjs` still opens `base + QUERY`, and `score.mjs` still requires one shared `query` across the 5 results.

## Facts from a3e50b1 this spec relies on

- **Captions have one level.** Every caption is Rust text with formulas and temperatures, for example `Soaked: it must dry out (boil off at 100 °C) before it can burn.` They come from four places:
  - `chem::Spell::cast_caption` (4 strings);
  - `chem::EventKind::caption` (one per variant, 21 strings, including all 6 `Fizzle` arms, all 4 `Extinguished(_)` arms and `Station`);
  - `react()` in `main.rs`: the spread `format!` (`Heat from a neighbour raised the {who} to its ignition temperature: it caught fire!`), the greenhouse line, `k.caption()` for uncaptioned `Melted`/`BurntOut`, `Reseeded`, and the win line;
  - `setup` (boot caption, which I-019 changes) and `reset()` (`Save the Greenhouse! Put out 3 fires before they spread.`).
  - `Game::say(&str)` sets `caption`, bumps `caption_id`, resets `caption_age`. `publish` sends `caption` and `captionId` through `esc`. The publish JSON template starts with `{{"`, so the science lint skips it.
- **The HUD shows the Rust text as is.** `hud.js` calls `showCaption(state.caption)` (8 s) and keeps the last 6 in `history` (pause card, `C` key). It also writes two band-free strings inline:
  - `#warning .text`: `` `The greenhouse is heating up: ${state.ghTemp} °C. Wood can catch fire at about 300 °C!` `` (register C-033), shown while `ghDanger || ghTemp > 60`;
  - lose banner `.small`: `Heat spread from bush to bush. Press R to try again.` (C-030). The banner `.big` texts and the win `.small` have no science.
- **The hotbar labels are static `index.html` text:** `Fire` / `CH₄ + 2O₂ → CO₂ + 2H₂O`, `Water` / `cools below ignition`, `Frost` / `N₂(l) boils at −196 °C`, `CO₂` / `CO₂ displaces O₂`.
- **The band is known only inside `analytics.js`, and only on the deployed site.**
  - `boot.js` runs `startHud()` first. Only when `<meta name="rg-analytics" content="on">` (the `web/dist` copy) does it lazy-import `age.js`. `pkg-out` (golden, perf, panel, local dev) never shows the picker, and neither does a browser with DNT or GPC.
  - On a pick, `age.js` hides the picker, calls `start(b.dataset.band ?? '')`, then `resetWhenReady()`, then focuses `#game`. `start` keeps the band in a closure. Nothing exposes it, and nothing stores it (by design: shared classroom devices).
  - `analytics.js` exports `BANDS = ['sprout', 'seedling', 'apprentice']`. It is not copied into `pkg-out` (`build-site.sh` copies it into `web/dist` only), so `pkg-out` code cannot import it.
- **Target names** for `{who}`: `bush`, `dry hedge`, `greenhouse`, `ice wall`, `puddle`, `steam vent`, `lab bench` (`world.rs`). Only `bush` and `dry hedge` can reach the spread line. A greenhouse ignite takes the greenhouse line.
- **The science lint** (`test/science-sources.test.ts`) walks `crates/bevy-client/src/**/*.rs`, `web/**/*.html` and `web/**/*.js` (not `.mjs`). It flags strings that contain a temperature, a formula or a science keyword (`burn`, `heat`, `smother`, `boil`, `melt`, `oxygen`, `caught fire`, ...), and strings with a sentence shape. A flagged string passes when a `## Claims` or `## Not claims` row has the same normalized text, wherever it lives. A `C-` row is **stale** (test fails) when a file in its `where` no longer contains the text. JS template text keeps `${...}` literally.
- **The panel.** `panel.mjs start` spawns `daemon.mjs ... --query <q>`. `daemon.mjs` opens `base + QUERY` and logs `query` in the `start` record and in `session.json`. `score.mjs` marks a panel "mismatched" unless all 5 results share one `query`. `result.schema.json` has `additionalProperties: false` at the root. `personas.mjs` exports `PERSONAS`, `PERSONA_IDS` and `persona(id)`. There is no `personaInfo`.
- **Web unit tests.** There are none under `crates/bevy-client/web/` yet. I-020 adds `*.test.mjs` there, run with `node --test`. The root `npm test` runs only `test/**/*.test.ts`.

## Relied on from I-005, I-019 and I-020 (their contracts; not rebuilt here)

- **I-005.** The band values above; the picker only on the deployed site with analytics on and no opt-out; the band is not stored; the pick calls `start`, then `resetWhenReady`, then focuses the canvas.
- **I-019.** The boot caption text; Water at boot; the hint priority; new DOM built from `hud.js`.
- **I-020.**
  - `coach.js`: `COACH` (one plain text per step: `walk` `Hold W to walk forward.`, `pick` `Fire will not put out fire. Pick Water: press 2.`, `cast` `Aim at a fire and click to put it out.`, `done` `Nice! Now put out every fire.`) and `coachFor(state, now, doneAt)`. The coach text goes into `#hint`.
  - `lines.rs`: `spread_line_due` and `event_lines(...) -> Vec<String>`, which hold every event-line decision `react()` makes. Spread is always said.
  - `caption-strings.txt` and the Rust test that every snapshot caption is still in `chem.rs` / `main.rs` / `lines.rs` byte for byte.
  - The world hold: `chem_tick` waits until the player walks 0.4 m and lands one cast. `tick N` (debug) bypasses it. `move` and `face` work under `?debug`.

**This item does not change** any Rust caption text (the 11-and-up band shows it unchanged), the chemistry, casting, the coach logic, the hold, the hint priority, settings, or the I-007 / I-019 hint strings.

## Problem

In the nightly panel (artifact 3dbd332), the 6-year-old and 9-year-old read captions written for a chemistry student: `CH₄ + 2O₂ → CO₂ + 2H₂O`, `boil off at 100 °C`, `N₂(l) boils at −196 °C (77 K)`. B4 median clarity was 6 against a bar of 8, and the young personas reported text blockers about formulas and "100 degrees". I-005 now asks the player's age on the deployed site, but the game ignores the answer: the band reaches the analytics beacon and nothing else. A pre-reader gets no picture with a caption, and the hotbar shows formulas to everyone.

**Goal:** every caption, hotbar label, warning and lose line has a version for the player's band, and the coach line has a sprout version. It is short and plain for 4–7, with a picture. It is plain with at most one number for 8–10. It is the full formula text for 11 and up. **Every version stays scientifically true.** Simpler words may leave things out, but must never say something false, and each one has a register row.

## Bars and expected move

- **B4 (every age can play):** this item's movement. It is a panel bar, so nothing is reported as moved at merge. **Panel B4 is measured "if the band is known":** the panel gives each persona its band in the URL (`?band=`), the way a real player gets it from the I-005 picker. Real players with DNT, GPC or analytics off get the default, `seedling`. So the first paired panel after merge (same seed and query, I-020 merge artifact vs this one) also runs **one extra 6yo session with no band** (`--band none`, default `seedling`). That session is scored on its own and is not one of the 5 results. Predictions:
  - median clarity rises from 6 to at least 7 (moved here from I-020);
  - neither the 6yo nor the 9yo reports a text blocker about a formula, a subscript, kelvin or "100 degrees" (moved here from I-020);
  - the no-band 6yo guard also reports no formula text blocker, and its clarity is within 1 of the banded 6yo;
  - the 13yo, teacher and qa (apprentice) see the same caption text as before; their clarity does not fall;
  - `time_to_move_s` and `ttc` do not regress for any persona.
- **B6 (science is right):** must stay at 0 errors with 100% of shown claims sourced. This item adds about 60 rows. Every one of them is reviewed by a fresh science reviewer before merge (test 15).
- **B1:** golden passes 3/3 unchanged (test 16). **B2/B3:** no regression beyond the measured noise (test 17). **B7:** the analytics path still sends `session_start` with the picked band (test 9).
- B4 still needs ≥ 8 clarity and zero control blockers to pass, which depends on I-007, I-019 and I-020 as well.

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

- `pub enum CaptionKey` with 29 unit variants (`Start`, `CastFire`, ..., `Won`, in the order below), `#[derive(Clone, Copy, Debug, PartialEq, Eq)]`, `impl Default` giving `Start`, `pub const ALL: [CaptionKey; 29]` in that order, and `pub fn as_str(self) -> &'static str` giving the snake-case string. A wrong or missing key is then a compile error.
- `pub const CAPTION_KEYS: &[&str]` stays as a literal list (the web test reads it from source with a regex), exactly these 29, in this order:
  `"start"`, `"cast_fire"`, `"cast_water"`, `"cast_frost"`, `"cast_foam"`, `"ignited"`, `"out_water"`, `"out_frost"`, `"out_foam"`, `"out_fire"`, `"burnt_out"`, `"reseeded"`, `"melted"`, `"melting"`, `"froze"`, `"steam"`, `"smothered"`, `"soaked"`, `"chilled"`, `"fizzle_wet"`, `"fizzle_no_oxygen"`, `"fizzle_not_flammable"`, `"fizzle_greenhouse"`, `"fizzle_charred"`, `"fizzle_heated"`, `"station"`, `"spread"`, `"greenhouse_caught"`, `"won"`.
- `pub fn spell_key(spell_index: u8) -> CaptionKey`: 0 `cast_fire`, 1 `cast_water`, 2 `cast_frost`, 3 `cast_foam` (the `Spell::ALL` order); 4 or more gives `cast_water`. This never happens, and the test checks only 0..=3.
- `pub fn event_key(kind: &chem::EventKind) -> CaptionKey`: one key per variant, as in the table below. `captions.rs` may `use crate::chem` (`chem.rs` has no Bevy types).

**String → key table.** Every caption the game can say today, with its source and its key. The apprentice text is the current string, unchanged.

| key | source (at a3e50b1) | current text (apprentice) | arg |
|-----|---------------------|---------------------------|-----|
| `start` | `setup` boot caption (I-019's text after it merges) **and** `reset()` | `Save the Greenhouse! Put out 3 fires before they spread.` (reset); I-019's boot text | `""` |
| `cast_fire` .. `cast_foam` | `Spell::cast_caption`, said in `aim_and_cast` on each cast | the 4 cast captions (C-034, C-006, C-007, C-035) | `""` |
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
- Call sites: `say(CaptionKey::Start, …)` in `setup` and `reset()`; `say(captions::spell_key(game.spell as u8), spell.cast_caption())` in `aim_and_cast`; `say(captions::event_key(&last.kind), last.kind.caption())` in `projectiles`; `say(CaptionKey::Won, …)` in `react()`.
- **`lines.rs`** gains `pub struct Line { pub key: CaptionKey, pub arg: String, pub text: String }` and `pub fn event_lines_keyed(<the same parameters as I-020's event_lines>) -> Vec<Line>`. It holds the decision. I-020's `event_lines` becomes `event_lines_keyed(..).into_iter().map(|l| l.text).collect()`, so I-020's tests and mutations stay valid unchanged. `react()` calls `event_lines_keyed` and passes each line to `say_arg(l.key, &l.arg, &l.text)`.
- `publish` adds `"captionKey":"{}","captionArg":"{}"` (the key through `as_str()`), each through `esc`.
- **No caption text changes.** I-020's `caption-strings.txt` test must still pass (test 3).

### 2. The band on the client (web: `crates/bevy-client/web/band.js`, new)

- `BANDS = ['sprout', 'seedling', 'apprentice']` and `DEFAULT_BAND = 'seedling'`. A test checks that `BANDS` equals `analytics.js`'s `BANDS` (band.js cannot import it, because `analytics.js` is not in `pkg-out`).
- `bandFromSearch(search)` returns the value of `?band=` when it is exactly one of `BANDS`, else `null`.
- `createBand({ search, doc })` returns `{ get(), set(b), on(cb) }`:
  - it starts at `bandFromSearch(search) ?? DEFAULT_BAND`;
  - `set(b)` ignores anything not in `BANDS`. A valid band replaces the current one, so the last setter wins: the URL applies at boot, and a later pick overrides it. Setting the current band again calls no callback;
  - each change sets `doc.documentElement.dataset.band` and calls the `on` callbacks once;
  - it touches no storage (I-005's rule: no remembered band on shared devices).
- No DOM access at import.
- `hud.js` creates one band in `startHud()` and exposes `window.__band = { get, set }`. `startHud()` runs before `boot.js` imports `age.js`, so the hook below always finds it.
- **The I-005 hook (the only change to `age.js`).** In the button `click` handler, one line goes **immediately before** `start(b.dataset.band ?? '');`:
  `window.__band?.set(b.dataset.band ?? '');`
  No other line of `age.js` changes (test 18 checks the diff). **Reviewer:** the I-005 main lane's breaker reviews this one line and re-runs `test/bevy-analytics.test.ts` (B7).
- **Default when no band is known** (DNT, GPC, `pkg-out`, local dev, no `?band=`): `seedling`, which means plain words, no formulas and at most one temperature. A 6-year-old with DNT still gets far easier text than today. A teacher can link `?band=apprentice` for the full formulas. A reading-level switch in the pause card is a follow-up.

### 3. Reading rules (web: `checkReading(text, band)` in `captions.js`)

`checkReading` returns a list of violations (empty = pass). Tests and the critic use it, and the I-020 coach texts must pass it too.
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
- **`apprentice`** texts are not rule-checked (they are the register-reviewed full text).

### 4. Captions and labels at each reading level (web: `crates/bevy-client/web/captions.js`, new)

- **`CAPTIONS`** maps each of the 29 keys to `{ sprout: { text, icon }, seedling: { text } }`. `apprentice` has no entry: it shows `state.caption`, the unchanged Rust text.
- **`SAME_AS_RUST`** = `['reseeded', 'melting', 'out_fire', 'fizzle_not_flammable', 'greenhouse_caught']`. For these five keys the seedling text is the Rust text byte for byte, so their `CAPTIONS` entry has **no `seedling` field** and seedling shows `state.caption`. This keeps `captions.js` free of Rust copies (test 14). The Rust texts are checked against the seedling rule in test 6, and test 3 keeps them from changing.
- **`captionFor(state, band)`:**
  - `apprentice` gives `state.caption`;
  - otherwise the band's text for `state.captionKey`, with `{who}` replaced by `state.captionArg`;
  - a seedling lookup for a `SAME_AS_RUST` key (no `seedling` field) gives `state.caption`;
  - an unknown or missing key gives `state.caption` (test 6 makes this impossible for shipped keys).
- **`ICONS`** = `fire`, `water`, `frost`, `foam`, `steam`, `ice`, `smoke`, `star`, `warn`, `bench`. For `sprout`, `#caption` shows the icon before the text: a `span.cap-icon.icon-<name>` drawn with CSS in `style.css`, at least 32 px, with no text content and `aria-hidden="true"`. The icons differ by shape, not only colour (GS-19). The panel's `#caption` reader still sees only the words.
- **`#caption` and the pause-card `history`** show and store `captionFor(state, band.get())`, so the history stays at the player's level. The `C` key replays it.
- **Pinned texts. All are exact.** Each text was checked against the rules in §3 and against the register sources (§6).

| key | sprout text | icon | seedling text |
|-----|-------------|------|---------------|
| `start` | `Put out 3 fires! Aim and click!` | fire | `Put out the 3 fires! Aim at a fire and click.` |
| `cast_fire` | `A hot gas flame!` | fire | `The Fire spell burns gas and makes lots of heat.` |
| `cast_water` | `Water cools the fire down.` | water | `Water soaks up heat and cools the fire.` |
| `cast_frost` | `Super cold! It cools things fast.` | frost | `Frost is a very cold liquid. It pulls heat away fast.` |
| `cast_foam` | `This gas keeps air away.` | foam | `This gas is heavier than air and pushes the air away.` |
| `ignited` | `It is so hot it caught fire!` | fire | `It got hot enough to catch fire, with air around it.` |
| `out_water` | `Fire out! Water cooled it.` | water | `Fire out! Water made it too cool to burn.` |
| `out_frost` | `Fire out! The cold stopped it.` | frost | `Fire out! The very cold liquid made it far too cool to burn.` |
| `out_foam` | `Fire out! No air, no fire.` | foam | `Fire out! The gas kept the air away from the fire.` |
| `out_fire` | `Fire out!` | star | *(Rust text, `SAME_AS_RUST`)* `Fire out!` |
| `burnt_out` | `All burned up! Nothing left to burn.` | smoke | `It burnt out: all of its fuel is used up.` |
| `reseeded` | `A new fire! Put it out!` | fire | *(Rust text, `SAME_AS_RUST`)* `Another bush is on fire! Put it out before it spreads.` |
| `melted` | `The ice melted. Now it is water!` | ice | `The ice melted at 0 °C. Now it is a puddle.` |
| `melting` | `Melting! Ice needs lots of heat.` | ice | *(Rust text, `SAME_AS_RUST`)* `Melting... it takes a lot of heat to turn ice into water.` |
| `froze` | `Frozen! The water turned to ice.` | ice | `Frozen! The very cold liquid turned the water into ice.` |
| `steam` | `Hot water turns into steam!` | steam | `Hot water boils at 100 °C and turns into steam.` |
| `smothered` | `Gas covers it. It can't burn now.` | foam | `A gas blanket keeps air away, so it cannot burn.` |
| `soaked` | `Wet now! Wet bushes are hard to burn.` | water | `Soaked! It has to dry out before it can burn.` |
| `chilled` | `Brrr! It got very cold.` | frost | `Brrr! The very cold liquid boils away as it chills it.` |
| `fizzle_wet` | `Too wet to burn!` | steam | `Sizzle! It is too wet to burn until the water boils away.` |
| `fizzle_no_oxygen` | `No air here, so no fire.` | foam | `The gas blanket keeps air away, so fire cannot start.` |
| `fizzle_not_flammable` | `That does not burn.` | warn | *(Rust text, `SAME_AS_RUST`)* `That does not burn.` |
| `fizzle_greenhouse` | `Not the greenhouse! Aim at a fire.` | warn | `Not the greenhouse! We want to save it. Aim at the fires.` |
| `fizzle_charred` | `Already burned. Nothing left to burn.` | smoke | `Already burnt: there is no fuel left.` |
| `fizzle_heated` | `Getting hot! A bit more and it burns.` | fire | `Heating up... a little more heat and it will catch fire.` |
| `station` | `Coming soon! It will split water.` | bench | `Coming soon: this bench will split water into two gases.` |
| `spread` | `Heat spread! The {who} caught fire!` | fire | `Heat from a fire made the {who} next to it catch fire!` |
| `greenhouse_caught` | `Oh no! The greenhouse caught fire! Press R.` | warn | *(Rust text, `SAME_AS_RUST`)* `Oh no! Heat from the fire reached the greenhouse. Press R to try again.` |
| `won` | `You did it! The greenhouse is safe!` | star | `You saved the greenhouse! Water, Frost and Smother each stop fire.` |

- **`SLOTS`** = `{ sprout: [...], seedling: [...] }`, four `{ name, label }` each. `hud.js` captures the four `index.html` `.name` / `.formula` texts **once, in `startHud()`, before any band-dependent write**, so the first sprout paint never shows formulas for a frame. On boot and on every band change, `hud.js` writes them into `.slot .name` and `.slot .formula` with `textContent`. For `apprentice`, `hud.js` restores the texts it **captured from the `index.html` DOM at boot**. No copy of the formula strings lives in `captions.js`.

| slot | sprout name / label | seedling name / label | apprentice |
|------|---------------------|-----------------------|------------|
| 1 | Fire / `hot flame` | Fire / `burns gas` | as in `index.html` |
| 2 | Water / `cools fire` | Water / `soaks up heat` | as in `index.html` |
| 3 | Frost / `super cold` | Frost / `very cold liquid` | as in `index.html` |
| 4 | Smother / `blocks air` | Smother / `keeps air away` | as in `index.html` (`CO₂`) |

- **`Smother` vs `CO₂` is deliberate.** Sprout and seedling name spell 4 for its mechanism (GS-30, GS-31); apprentice keeps the `index.html` name and the Rust `CO₂` texts (cast, `won`, the I-020 pick line). In a mixed class the name differs between neighbouring screens. The science reviewer and the panel brief should not flag this as a bug.

- **`warningFor(state, band)`** gives the `#warning .text` line. **`lostFor(band)`** gives the lose banner's `.small`. `hud.js` uses them in place of its inline strings. The banner `.big` texts and the win `.small` stay as they are for every band. Pinned:

| fn | sprout | seedling | apprentice (current text, moved from `hud.js`) |
|----|--------|----------|-----------------------------------------------|
| `warningFor` | `The greenhouse is getting hot!` | `` `The greenhouse is heating up: ${state.ghTemp} °C. Too hot and the wood will burn!` `` | `` `The greenhouse is heating up: ${state.ghTemp} °C. Wood can catch fire at about 300 °C!` `` |
| `lostFor` | `The fire spread. Press R to try again!` | `Heat spread from bush to bush. Press R to try again.` (the same constant as apprentice) | `Heat spread from bush to bush. Press R to try again.` |

  The `${state.ghTemp}` expression is written exactly like that (parameter named `state`), so the register text matches what the lint extracts. The `#warning` meter bar stays for every band. It is the visual cue that goes with the sprout line, which has no number.

### 5. The coach at each reading level (web: `crates/bevy-client/web/coach.js`)

- New export **`COACH_TEXT = { sprout: {walk, pick, cast, done} }`**. I-020's `COACH` (text and icons) is not edited. **Seedling and apprentice both use I-020's `COACH` text unchanged**, so the 13yo, teacher and qa read the same hint as before.
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
- Texts already in the register (`Another bush is on fire!…`, `Melting...…`, `That does not burn.`, `Heating up...…`, `Oh no! Heat from the fire reached the greenhouse.…`, `Heat spread from bush to bush.…`) need no new row.
- **Suggested verdicts** for the builder. The reviewer (test 15) decides.
  - **`correct`:**
    - cast_fire both (S7, S4; S15 for "hot flame");
    - cast_frost both (S1);
    - out_water both (S9);
    - melted both (S10);
    - melting sprout (S10);
    - steam both (S2, S10);
    - chilled both (S1);
    - fizzle_wet sprout (S16);
    - fizzle_heated sprout (S6);
    - station both (S13, S7);
    - spread both (S6): sprout `Heat spread! The {who} caught fire!`;
    - won seedling (S9, S1, S19);
    - burnt_out seedling (S7);
    - ignited seedling (S6, S14);
    - out_frost seedling (S1, S6);
    - soaked sprout (S16, S10);
    - lost sprout (S6);
    - pick sprout coach line (S9, S19);
    - slot labels `hot flame`, `burns gas`, `super cold`, `very cold liquid`, `soaks up heat`.
  - **`simplified`**, with a note on what is left out:
    - cast_water both and `cools fire`: water cools the fuel and flame, and some steam smothers (S9);
    - cast_foam both, out_foam both, smothered both, fizzle_no_oxygen both, `blocks air` and `keeps air away`: as C-035/C-036, CO₂ lowers oxygen and mainly absorbs heat; "no air" is the limiting case (S19, S14, S5, S8);
    - ignited sprout: it leaves out oxygen (S6);
    - out_frost sprout (S1);
    - burnt_out sprout and fizzle_charred both: as C-024 (S18, S7);
    - froze both: as C-016 (S1, S10);
    - soaked seedling and fizzle_wet seedling: as C-019/C-021, moist fuel can ignite before it is fully dry (S16, S10);
    - warning seedling: as C-033, about 300 °C piloted (S6).
  - **`## Not claims`:**
    - start both;
    - reseeded sprout;
    - fizzle_greenhouse both;
    - greenhouse_caught sprout;
    - won sprout;
    - warning sprout (state report, as the `The greenhouse is heating up!` row);
    - `Smother` (spell name, as the `CO2` row);
    - `Fire out!`;
    - every sprout coach line except the pick line.

## Acceptance tests

- **Rust:** `cargo test --manifest-path crates/bevy-client/Cargo.toml --locked` (as I-020).
- **Web unit:** `node --test "crates/bevy-client/web/*.test.mjs"` (`.mjs`, so the science lint does not scan test strings).
- **Panel unit:** `node --test tests/e2e/panel/`.
- **E2E:** `node tests/e2e/bands.mjs <pkg-out dir>`. It serves the built `pkg-out` and drives headless Chromium at 1280×720 with `?low&debug`, plus the band query of each case. It reads `window.__game.state()` and the DOM. "Within N frames" means N increments of `state.frame`. Objects are found by `objs[].n` and `burning`, never by a fixed index. Before any check that needs the world to run, it ends I-020's hold the way I-020 test 8 does (`move 0 1 0.6`, then a cast that lands).
- Each test says how it fails on based_on, or on the I-020 merge commit where it depends on I-020.

1. **Keys cover every caption** (core).
   - `CAPTION_KEYS` has 29 entries and no duplicates.
   - `spell_key(0..=3)` gives `cast_fire`, `cast_water`, `cast_frost`, `cast_foam`.
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
   - `bandFromSearch('?band=sprout')` is `'sprout'`, and `'?low&band=apprentice'` gives `'apprentice'`.
   - `'?band=SPROUT'`, `'?band=adult'`, `'?band='` and `''` each give `null`.
   - `createBand({search:'', doc})`: `get()` is `'seedling'`, and `doc.documentElement.dataset.band` is `'seedling'`.
   - `set('x')` changes nothing and calls no callback. `set('seedling')` (the current band) calls no callback.
   - `set('sprout')` changes it and calls each `on` callback exactly once.
   - Nothing touches `localStorage` or `sessionStorage`: the fake globals throw on any access.
   - `BANDS` deep-equals `analytics.js`'s `BANDS`.
   - Importing `band.js` with no `document` or `window` defined does not throw.
   - Fails today: no `band.js`.
5. **`captionFor` and the pinned tables** (web).
   - `captionFor({caption:'X', captionKey:'soaked'}, 'apprentice')` is `'X'`.
   - With `'sprout'` it is `Wet now! Wet bushes are hard to burn.`
   - `captionFor({captionKey:'spread', captionArg:'dry hedge', caption:'…'}, 'seedling')` is `Heat from a fire made the dry hedge next to it catch fire!`, and with `'sprout'` it is `Heat spread! The dry hedge caught fire!`
   - An unknown key with `'sprout'` gives `state.caption`.
   - For each `SAME_AS_RUST` key, `captionFor({captionKey:k, caption:'R'}, 'seedling')` is `'R'`, and `CAPTIONS[k].seedling` is undefined.
   - Every cell of the §4 caption table (seedling cells marked `SAME_AS_RUST` excepted, as above), the §4 slot table, the `warningFor` / `lostFor` table (`warningFor({ghTemp:150}, b)`; apprentice gives `The greenhouse is heating up: 150 °C. Wood can catch fire at about 300 °C!`) and the §5 coach table equals the pinned text exactly, and so does every sprout icon.
   - `coachFor({onboard:'walk'}, 0, 0)` (no band) still gives I-020's `Hold W to walk forward.`, and with `'sprout'` it gives `Hold W to walk!`. With `'apprentice'` it gives `Hold W to walk forward.` (I-020's text).
   - For each step, `coachFor(..., 'sprout').icon` equals `COACH[step].icon`. `COACH_TEXT` has no `apprentice` or `seedling` key.
   - Fails today: no `captions.js`, no `COACH_TEXT`.
6. **Coverage and reading rules** (web).
   - The test reads `crates/bevy-client/src/captions.rs` at test time and extracts the `CAPTION_KEYS` literals with a regex. `CAPTIONS` has exactly those keys, none missing and none extra. Every entry has `sprout`; every entry has `seedling` except exactly the `SAME_AS_RUST` keys.
   - `checkReading` returns no violations for: every sprout and seedling text in `CAPTIONS`, `SLOTS`, `warningFor` and `lostFor`; every `COACH_TEXT.sprout` line (sprout rule); I-020's four `COACH` texts (seedling rule); and, under the seedling rule, the five Rust texts of the `SAME_AS_RUST` keys. The test holds those five as fixtures and first checks that each is a line of `docs/loop/items/I-020/caption-strings.txt` (so a Rust edit that breaks the fixture fails here and in test 3).
   - `checkReading('Heat spread! The {who} caught fire!', 'sprout')` counts 7 words, and a 7-word sprout text plus `{who}` that would reach 9 words at runtime fails.
   - Every sprout caption has an icon in `ICONS`.
   - **Mutations** (each one, applied to a copy of the tables in the test, must make it fail):
     - sprout: `CAPTIONS.soaked.sprout` = `Soaked: it must dry out (boil off at 100 °C) before it can burn.` (too many words, a 100 and a `°`);
     - sprout: `CAPTIONS.won.sprout` = `You did it! The courtyard is safe!` (a 9-letter word that is not `greenhouse`);
     - seedling: `CAPTIONS.steam.seedling` = `Hot water boils at 100 °C (373 K): H₂O(l) → H₂O(g).` (a formula and two temperatures);
     - seedling: `CAPTIONS.cast_water.seedling` = `Water absorbs heat thermodynamically.` (a 17-letter word).
   - Fails today: no `captions.js`.
7. **Every new text is in the register** (web). The test reads `content/science-sources.md` at test time and collects the backticked text of every `## Claims` and `## Not claims` row (normalized as the lint does: subscripts to digits, `−` to `-`, collapsed whitespace). Every sprout and seedling text in `CAPTIONS`, `SLOTS`, `warningFor`, `lostFor` and `COACH_TEXT.sprout` is in that set, whether or not the lint would flag it. `npm test` (the lint, including the stale check after the C-030/C-033 `where` edits) passes. `git diff <based_on>... -- content/science-sources.md` removes no row. Fails today: none of the texts exist.
8. **HUD wiring on a fake DOM** (web). Run `startHud`'s band, caption and slot setup against a fake document whose hotbar is built from the slot spans **read from `crates/bevy-client/web/index.html` at test time** (regex over the `.slot` buttons). The test fails loudly if the regex finds other than exactly four `.slot` buttons, each with one `.name` and one `.formula` span, so an `index.html` edit cannot silently pass.
   - With search `?band=sprout`, the `.formula` texts are `hot flame | cools fire | super cold | blocks air`, and the names are `Fire | Water | Frost | Smother`. A write spy on the fake spans records no formula text written after `startHud()` begins (the capture reads, never writes, the `index.html` text before the first sprout write).
   - `window.__band.set('apprentice')` restores every `.name` and `.formula` to exactly the `index.html` texts read at test time.
   - A fake state `{captionKey:'soaked', caption:'X', captionId:1}` under sprout puts `Wet now! Wet bushes are hard to burn.` in `#caption` and in `history`, with a `.cap-icon.icon-water` child. The same under apprentice puts `X`.
   - A fake state with `outcome:'lost'` under sprout gives the banner `.small` `The fire spread. Press R to try again!`
   - Fails today: there is no band.
9. **The I-005 hook, on a fake DOM** (web; no by-hand alternative). `age.test.mjs` sets fake `window`, `document` and `navigator` globals:
   - a `meta[name="rg-analytics"]` with content `on`, and `navigator.doNotTrack = '0'`;
   - `#age` with the three `button[data-band]` and a `#game`;
   - a `window.__band` spy, and `window.__gameState` / `window.__game` fakes;
   - `globalThis.setInterval` stubbed to a no-op (restored after), so the session poller does not keep the test alive;
   - a `navigator.sendBeacon` spy.
   It imports `age.js`, calls `startAgePicker()` (it returns `true`), and clicks the "4 to 7" button.
   - `__band.set` was called exactly once, with `'sprout'`, and **before** the first `sendBeacon` call.
   - `sendBeacon` received a `session_start` body whose `band` is `sprout` (B7 still holds).
   - Clicking again calls neither spy again (`picked` guard).
   - **Mutation:** deleting the hook line fails the test.
   - Fails today: `age.js` has no hook.
10. **Panel band plumbing** (contract; `tests/e2e/panel/band.test.mjs`).
    - `persona('6yo').band` is `'sprout'`, `persona('9yo').band` is `'seedling'`, and `'13yo'`, `'teacher'` and `'qa'` are `'apprentice'`.
    - The pure `pageUrl(base, query, band)` in `personas.mjs`:
      - `pageUrl('http://h/', '?low', 'sprout')` is `'http://h/?low&band=sprout'`;
      - `pageUrl('http://h/', '', 'seedling')` is `'http://h/?band=seedling'`;
      - `pageUrl('http://h/', '?low&band=apprentice', 'sprout')` keeps the operator's `band` as it is (`'http://h/?low&band=apprentice'`);
      - `pageUrl('http://h/', '?low', 'none')` is `'http://h/?low'`.
    - `panel.mjs start` accepts `--band <sprout|seedling|apprentice|none>`, defaults to `persona(id).band`, and passes `--band` to `daemon.mjs`. The test checks the daemon argv that `panel.mjs` builds, through an exported pure helper.
    - The daemon's `start` record and `session.json` keep `query` exactly as passed, and add `band`. `score.mjs`'s identical-query rule is unchanged, and the results do not carry `band` (`result.schema.json` is not edited).
    - The existing `score`, `report` and `harden` tests stay green.
    - Fails today: there is no `band`.
11. **E2E: sprout on screen** (contract; `?low&debug&band=sprout`). Three separate tests with the same assertions, so one flaky step does not hide the others. In each, no visible text node in `#hud` matches `/[₀-₉]|°|→|\b[A-Z][a-z]?\d/` at any checkpoint.
    - **11a: slots, hint and caption icon.**
      - `html[data-band]` is `sprout`.
      - The `.slot .formula` texts are `hot flame | cools fire | super cold | blocks air`.
      - At the first state, `#hint` (or I-020's coach text node) is `Hold W to walk!`, and `#coach` is visible with class `step-walk` and its icon (the sprout coach keeps I-020's icon).
      - End the hold, then `face <non-burning bush>`, `select 2`, `cast`. After `impacts` rises, `#caption` text is `Wet now! Wet bushes are hard to burn.` and `#caption .cap-icon.icon-water` is visible with a box of at least 32×32 px.
    - **11b: history line after a spread.** End the hold. Then step `tick 100` repeatedly (at most 60 times) until `state.captionKey === 'spread'`. The world start state is deterministic (I-020 test 2 relies on it), so this is a fixed path, not a branch. The test **fails** if `greenhouse_caught` or the step cap comes first. Then open the pause card: `#history` holds a line matching `/^Heat spread! The (bush|dry hedge) caught fire!$/`.
    - **11c: warning and lose text.** End the hold. Step `tick 200` until `#warning` is visible with `outcome === 'playing'`: its `.text` is `The greenhouse is getting hot!`. Step on until `outcome === 'lost'`: the banner `.small` is `The fire spread. Press R to try again!`. Then run the no-formula scan over the whole `#hud`.
    - Fails on the I-020 merge commit: the slots show formulas, and `#caption` shows the C-019 text.
12. **E2E: apprentice and default** (contract).
    - **`?band=apprentice`:**
      - after a cast, `#caption` equals `state.caption`;
      - the four slot `.name` / `.formula` texts equal those in the served `index.html`, fetched and parsed at test time;
      - the `#warning` text, once visible, matches `/^The greenhouse is heating up: \d+ °C\. Wood can catch fire at about 300 °C!$/`;
      - the first `#hint` is I-020's `Hold W to walk forward.` (apprentice uses I-020's coach text).
    - **No `?band`:**
      - `html[data-band]` is `seedling`;
      - after a Water cast that puts out a fire, `#caption` is `Fire out! Water made it too cool to burn.`;
      - the first `#caption` (boot) is `Put out the 3 fires! Aim at a fire and click.`;
      - the first `#hint` is I-020's `Hold W to walk forward.`
    - **Picker hook live:** on the no-band page, `window.__band.set('sprout')` re-labels the slots, and switches `#hint` to the sprout coach text, within 2 frames.
    - Fails on the I-020 merge commit: there is no `data-band`, and the captions are the C- texts.
13. **Captions stay true at every level (lint guard).** `npm test` passes with 0 unsourced client claims and 0 stale rows. Guard: green before and after.
14. **No string drift between Rust and the tables.** `captions.js` has no copy of any Rust caption (apprentice and the `SAME_AS_RUST` seedling cells come from `state.caption`), and no copy of an `index.html` slot formula. The test greps `captions.js` for each `caption-strings.txt` line and each `index.html` `.formula` text and finds none. **Mutation:** adding `seedling: { text: 'That does not burn.' }` to `fizzle_not_flammable` fails it.
15. **Science review gate (model check, before merge).** A fresh science reviewer, a model that wrote none of these texts, follows `docs/loop/science-review.md` over every row this item adds or edits.
    - It checks that each sprout and seedling text is true as stated, and that each `simplified` note names what is left out and why the text is not false.
    - Pass: 0 `error` verdicts, and every new text sourced.
    - The contract lane records the review in `round-1.md`.
    - **Fallback:** if `docs/loop/science-review.md` is missing on the merge commit, the contract lane writes the reviewer prompt it used (scope, verdict scale, tolerance rules) into `round-1.md` before dispatching the reviewer.
    - A text the reviewer finds false is rewritten under §3 and re-reviewed. It does not ship with an `error` verdict.
16. **B1 and existing tests not regressed.** After `build-web.sh`, `QUERY=?low node tests/e2e/golden.mjs` passes 3 of 3 runs, unchanged, with 0 console errors. With no `?band`, golden runs at `seedling`. Golden must assert on `state.caption` (Rust, unchanged), never on HUD text; if any golden check reads `#caption`, `#warning`, `.formula`, `#hint` or `#history` text, the builder stops and reports (golden is not edited).
    - **All other tests:** the contract lane greps every file under `tests/e2e/` and every `crates/bevy-client/web/*.test.mjs` on the merge base (I-007, I-019 and I-020 tests included) for text assertions on `#caption`, `#warning`, `.formula`, `#hint` and `#history`. Each hit is either moved to `?band=apprentice` (an e2e query change only) or switched to read `state.caption`, with no other change to the test. The list of hits and the fix for each is recorded in `round-1.md`. Every such test passes after the merge.
    - The seam breaker owns this check: it re-runs the grep and the listed tests and confirms nothing was missed.
17. **B2/B3 guard.** `node tests/e2e/perf.mjs` at 4x throttle with `?low` (no band), following I-020 test 13's method:
    - noise floor from two batches of 3 runs at the I-020 merge artifact;
    - 3 interleaved runs each of this artifact and that one;
    - limits are max(budget, 2 × noise floor) with budgets median CPU p50 +1.0 ms and TTI +0.5 s. Brotli total is limited to +20 KB (deterministic);
    - every run has `workload_ok`.
    - `band.js` and `captions.js` are ES modules imported by `hud.js` (as I-020's `coach.js` is). The two extra small same-origin module requests (about 4 KB together, Brotli) are accepted and are counted inside the TTI budget above.
18. **Scope.** `git diff --name-only <based_on>...` lists only the paths in the Lanes table, the existing test files listed in `round-1.md` under test 16, and `docs/loop/items/I-021/**`.
    - `git diff -U0 <based_on>... -- crates/bevy-client/web/age.js` has exactly one `+` line, `window.__band?.set(b.dataset.band ?? '');`, and no `-` line.
    - `index.html`, `analytics.js`, `boot.js`, `chem.rs`, `golden.mjs`, `perf.mjs`, `functions/`, `dashboard/` and `result.schema.json` do not change.
    - I-020's `COACH` table and its tests are unchanged.

## GS walk (screens, texts and effects this item touches)

Source: `docs/loop/gameplay-standards.md` at a3e50b1.

| GS | P | status |
|----|---|--------|
| 6 | P0 | met: the icons are static; no new animation |
| 14 | P1 | not applicable by I-005's rule: the band is never stored (shared devices). The URL and the picker set it each load |
| 17 | P0 | improved: every sprout result caption has a big icon (star on a win and on `out_fire`) |
| 18 | P0 | partial: caption icons are at least 32 px. Text sizes are unchanged (I-007 owns them) |
| 19 | P0 | met: the icons differ by shape. Slot names stay alongside slot icons |
| 20 | P1 | unchanged: captions are not speech, and there is still no audio-only cue |
| 21 | P0 | **met**: three reading levels with enforced rules (test 6). Every sprout caption has an icon. Captions stay 8 s, can be replayed with C and are in the pause history (no forced auto-dismiss) |
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
- The apprentice coach text: apprentice uses I-020's `COACH` unchanged.
- Band versions of `state.spellName` / `state.formula`, which nothing in the HUD shows.
- HUD font sizes (I-007), voice-over, translations.
- `docs/loop/panel.md` and `docs/loop/areas.md` edits in this item's diff. The planner adds `captions.rs` and `lines.rs` to `areas.md` before dispatch (pre-dispatch step 5), and files a follow-up to add one line on bands to `panel.md` once `areas.md` lists it.

## Contract

**New `__gameState` fields** (existing fields unchanged):

| field | type | meaning |
|-------|------|---------|
| `captionKey` | string, a member of `CAPTION_KEYS` | the key of `caption` |
| `captionArg` | string | the target name for `spread`, else `""` |

**Rust.**
- `crates/bevy-client/src/captions.rs` holds `CaptionKey` (with `ALL`, `as_str`, `Default`), `CAPTION_KEYS`, `spell_key` and `event_key` (§1). It is Bevy-free and may `use crate::chem`. `main.rs` adds `mod captions;` next to the other `mod` lines.
- `lines.rs` gains `Line` (key: `CaptionKey`) and `event_lines_keyed`. `event_lines` keeps its signature and output.
- `Game::say(key: CaptionKey, text)` and `Game::say_arg(key: CaptionKey, arg, text)`.

**Web modules** (ES modules, no DOM access at import; `build-web.sh`'s `cp` line adds `band.js captions.js`):
- `band.js`: `BANDS`, `DEFAULT_BAND`, `bandFromSearch`, `createBand`;
- `captions.js`: `CAPTIONS`, `SAME_AS_RUST`, `SLOTS`, `ICONS`, `captionFor`, `warningFor`, `lostFor`, `checkReading`;
- `coach.js`: adds `COACH_TEXT` (sprout only) and the optional `band` parameter of `coachFor` (text changes for sprout only; icon never changes).

**DOM and hooks** (all created by `hud.js`; `index.html` unchanged):
- `html[data-band]`;
- `#caption .cap-icon.icon-<name>` (sprout only);
- `window.__band = { get, set }`;
- `age.js` calls `window.__band?.set(band)` on a pick.

**Panel:**
- each `PERSONAS` entry gets `band`;
- `personas.mjs` gets `pageUrl(base, query, band)`;
- `panel.mjs` and `daemon.mjs` take `--band <b|none>`;
- the daemon opens `pageUrl(...)` and records `band` in the `start` record and in `session.json`.

**Contract stub** (landed on the branch base before the lanes start, so all three lanes build against it):
- `captions.rs` with `CaptionKey`, the full `CAPTION_KEYS` list and `spell_key` / `event_key` bodies (they are the table; the stub makes them real);
- `captionKey` / `captionArg` published as `"start"` / `""`;
- empty `band.js` / `captions.js` exports with the signatures above.

## Lanes

The three lanes build against the stub independently. Web tests use fake state and a fake DOM. Core tests are Rust unit tests. The contract e2e is red until core and web merge, and is first run then. Merge order: core, web, contract.

| lane | owns | builds |
|------|------|--------|
| core | `crates/bevy-client/src/main.rs` (`say` / `say_arg`, the call sites, the `publish` fields, `mod captions;`), `crates/bevy-client/src/captions.rs` (new; added to `areas.md` `client-core` by the planner), `crates/bevy-client/src/lines.rs` (`Line`, `event_lines_keyed`, `event_lines` as a thin map) | §1, tests 1 to 3. Must not change any caption string or `chem.rs` |
| web | `crates/bevy-client/web/hud.js`, `style.css`, `band.js`, `captions.js`, `coach.js` (`COACH_TEXT.sprout` and the `band` parameter only), `crates/bevy-client/web/*.test.mjs` (new: `band`, `captions`, `hud-band`, `age`), `crates/bevy-client/web/age.js` (the one hook line only; reviewer: the I-005 main-lane breaker), `crates/bevy-client/build-web.sh` (the `cp` line only), `content/science-sources.md` | §2 to §6, tests 4 to 9 and 13 to 14. The caption icons. The slot capture and restore. `warningFor` / `lostFor` wired into `hud.js`. Every register row and the C-030/C-033 `where` edits |
| contract | `tests/e2e/bands.mjs`, `tests/e2e/panel/personas.mjs`, `tests/e2e/panel/daemon.mjs`, `tests/e2e/panel/panel.mjs` (the `--band` flag and argv helper only), `tests/e2e/panel/band.test.mjs`, and the existing e2e / web test files found by test 16's grep (the band query or `state.caption` read only; never `golden.mjs` or `perf.mjs`) | tests 10 to 12. It runs 15 (it dispatches the reviewer), 16 and 17 and records them in `round-1.md` |

**Shared files.** `main.rs`, `hud.js` and `coach.js` are also edited by I-019 and I-020, which merge first. The lanes rebase onto them and change only the blocks named above. If the merged I-020 `react()` still holds an event-line condition outside `event_lines`, or the merged `hud.js` builds the caption, warning or lose text somewhere other than one place each, the builder stops and reports in `round-1.md`, and the planner refreshes this spec.

**Seam.** The seam breaker checks:
- every key `main.rs` can publish has a `CAPTIONS` entry. It drives every `EventKind` once in the e2e page, or uses test 1's list against test 6's;
- test 16's grep of existing tests that read HUD text is complete, and each listed test passes;
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
| claims moved from I-020: median clarity 6 → ≥ 7; no text blocker about formulas or "100 degrees" | Bars and expected move |
| captions stay scientifically true at every level; new texts have register rows | §4 checked texts, §6, tests 7, 13, 15 |
