# I-021 spec critique (revision 3 review, roadmap v2)

**Verdict: revise.** Right thing, wrong seams. The goal (captions, hotbar, warning and lose text at the player's reading level) still serves kids 4-10 and teachers, and it moves B4. But the spec now claims to be the shared band store and reading-rule checker for the core loop, and the phase-2 specs that were written afterwards do not use it. Two of those conflicts will produce a mixed-band game. One decision (closed 29-key enum for cast captions) will be thrown away by I-026 and cannot reach B8's 8 spells. The item is also bigger than its smallest version. None of this needs a rewrite; it needs the changes below.

Reviewed: this spec, `critique.md` (previous), `target.md`, `bars.md`, `areas.md`, `gameplay-standards.md` from origin/main (c4832d2), and `docs/loop/items/I-02{0,2,4,5,6,7,8,9}`, `I-030` specs. Not re-verified against code: line numbers (the spec already says the planner re-checks them after I-022/I-025/I-020).

## 1. Right thing for players and teachers?

Yes for Phase 1 text, with three cautions.

- **Under the new core loop it is support work, not core.** Crafting and mastery (I-026 to I-030) are the loop; this item makes the words around them readable. It earns its place because B4 clarity (6 vs 8) is a panel failure on the youngest personas and the Bench will show the same captions after a cast. Keep it, but do not let it gate I-026. Its "before I-026" rule is the problem (item 3).
- **A 4-5 year old still cannot read "Fire out! Water cooled it."** GS-21 says pre-readers are not blocked by text. The sprout captions carry an icon, but the sentence is silent. I-028 already speaks sprout prompts with `speechSynthesis` (sound on, `try/catch`, text always on screen too). Say in this spec whether sprout captions use the same helper. Either add it (about 10 lines in `hud.js`, one test with a `speechSynthesis` stub, respecting the saved sound setting) or name it as a filed follow-up and stop claiming GS-21 "**met**" in the GS walk (change to "partial: icon plus short text; no speech").
- **Band-free text the sprout player still hits first:** `START_HINT` (`Click to look around. Esc shows the controls and point-and-click mode.`), `AIM_HINT`, `IDLE_HINT`, the pause-card controls table, `#obj-line`. They are explicitly out of scope, but B4's prediction "median clarity 6 to at least 7" and "no text blocker" rests on the 6yo not meeting them. Soften the prediction to what the item controls (no formula, no `°`, no kelvin blocker; clarity not lower), or add the two hint constants to `captions.js` (two keys, same pattern as `warningFor`). Panel clarity is a model score; do not state a number the item cannot cause.
- **Default `seedling` for DNT/GPC/analytics-off.** Acceptable for a first cut because a reload re-shows the picker on the deployed site. But `pkg-out`/local/teacher links never show it, and a teacher handing 5-year-olds a plain link gets seedling. The pause-card switch is the real fix; it is filed as a follow-up only. With I-028 adding its own chooser (item 2) the switch is nearly free. See item 2.

## 2. Overlap and conflicts with the phase-2 specs (must fix)

- **I-028 does not use `band.js` and keeps a second band store.** This spec says I-028 "imports `LEVELS`, `bandFromSearch`, `checkReading`, `ICONS` and follows `window.__band`". I-028 §2 does not: it has its own `bandFrom(search, picked, chosen)`, adds its own `click` listener on `#age button[data-band]`, shows its own "How old are you?" chooser, keeps the answer for the page load, and maps `adept` to `apprentice`. Result: a 6yo with no picker who chooses "4 to 7" in the Bench still reads seedling captions on the HUD, and a `?band=adept` link is `adept` in the HUD and `apprentice` in the bench. This is a real GS-21 and B4 break, not style.
  Change: (a) in this spec's Contract replace the "Reuse by I-028" paragraph with a hard interface: `window.__band.get()`, `.set(b)`, `.on(cb)` (move `on` into the exposed hook now; I-029 also reads the band), and `LEVELS`; (b) add to Out of scope / Planner actions: "I-028 must read `window.__band` (not its own picker listener) and its chooser must call `window.__band.set(...)`; the planner edits I-028's §2 and test 11 before dispatch"; (c) state that `adept` stays `adept` in `window.__band` and consumers map it, since I-029 sends `band adept` to Rust. Order: I-021 before I-028 (backlog "After" cell).
- **I-029 assumes "I-021's `band.js`, extended by I-028 with `adept`".** I-021 already has `adept`. Fix I-029's sentence at the same planner step; no change needed here beyond item 2a.
- **I-026 makes the closed cast-caption enum a dead end.** I-026 deletes `Spell::cast_caption()` and moves cast text to `content/spells.json`, and B8 needs 8 craftable spells. Here `CaptionKey` has exactly four `cast_*` variants keyed by `Spell::ALL` index, so every new spell needs a Rust enum variant, a web table row and 2 register rows by hand. Change: keep the enum for `EventKind` results only (25 keys), and key the cast caption by spell id: `captionKey = "cast"` plus `captionArg = <spell id>`; the sprout and seedling cast texts live in a small `CAST_CAPTIONS` table keyed by id in `captions.js` now, and I-026's spec is told to move them to two optional `spells.json` fields (`caption_sprout`, `caption_seedling`, the same pattern as I-028's `prompt_*`) with the `checkReading` test over them. This also cuts test 1's `spell_key` and shrinks the table the planner must renumber.
- **I-026 and the hotbar.** I-026 leaves the hotbar DOM alone and says `hud.js`/`index.html` are not edited; this item rewrites slot text from `SLOTS` by index. Good, but when I-026/I-028 add spells 5-8 to the hotbar, `SLOTS` has no row and the spell shows the formula to a 6yo. State the fallback: for a slot with no `SLOTS` entry at sprout/seedling, show the spell name only (no `.formula` text). One assertion in test 8.
- **I-022, I-024, I-025, I-020:** the overlap table is accurate in structure (no shared lines; `age.js` hook after I-022's lines; keys append in merge order; `perf` guard). Agreed. Two small asks: (a) the `age.js` hook is a one-line edit to the I-005 file that I-028's separate listener would make redundant once item 2 lands; keep it (B7 path) but delete the "I-028 may add its own listener" ambiguity; (b) I-024 changes the profile, so test 17's two-batch noise floor must be re-run on the I-020 merge artifact if I-024 lands between; the spec already says the planner moves the base. Fine.
- **I-027, I-030:** no overlap. I-030 asks 4 bands of question text; it may wish to run its sprout `prompt` text through `checkReading`. Add one sentence to Contract naming `checkReading` as the checker for any sprout/seedling string in `content/*.json` too; today it only checks JS tables, and the science lint does not scan JSON.

## 3. Smallest version

The spec is 563 lines with 3 lanes, 18 tests, about 60 register rows, a Rust enum, a panel plumbing change and a second stage of docs. For a text change that is too much. Cuts that keep every promise:

1. **Drop seedling cells for keys that already fit.** Many seedling texts are the Rust text with a word swapped (`Soaked! It has to dry out before it can burn.` vs the Rust one). Five are already `SAME_AS_RUST`; apply the rule to every key where the Rust text already passes the seedling rule (14 words, one temperature, no formula). Run `checkReading` over `caption-strings.txt` and write seedling cells only for the failures. Expect about half the rows and half the science review.
2. **Cut `PICKER_BANDS` vs `BANDS` drift test and `LEVELS` split work to one line** (a constant plus one test), not a design section.
3. **Cut `bands.mjs` test 11 into 11a only for the gate; keep 11b/11c as non-blocking breaker checks.** They need a 60-step tick loop that depends on I-020's hold and the world's determinism; a flaky path blocks merge. Make `11a` and `12` the gate, run `11b`/`11c` in the seam breaker.
4. **Cut warning/lose to sprout-only variants** and keep the seedling and apprentice text for both as the existing constant. Seedling `warningFor` is a rewrite of an apprentice string for no measurable gain.
5. **Defer panel `--band` plumbing detail** (test 10) to: `persona.band` plus `pageUrl`. The argv-helper export and `session.json` field are nice to have; one test is enough.

## 4. Can each acceptance test fail?

Mostly yes (mutations are listed for 6, 9, 14; "Fails today" is stated). Gaps:

- **Tests 3 and 13 are guards** that pass before and after; fine, labelled. 
- **Test 5** compares the table to itself (the pinned strings are in the spec and in the code). Real value is only in the `checkReading` rules (test 6) and in the register match (test 7). Add one cross-check that cannot be a copy: every sprout text must contain no word from a banned list built from `index.html` formulas and `caption-strings.txt` science nouns (`oxygen`, `ignition`, `temperature`, `thermal`). Cheap and catches a drifted edit.
- **Test 6 / `checkReading`:** the sprout rule "no word over 8 letters except `greenhouse`" is easy to satisfy and says nothing about whether a 5-year-old reads it. Fine as a floor, but say so in the spec; panel B4 is the real measure. Also pin the word-count rule for `{who}` as the spec does; test 6's `{who}` case covers it.
- **Test 8 and 9** need big fake DOMs; the risk is a test that passes because the fake is lenient. Test 9's mutation (delete the hook line) is right. Add one more mutation to test 8: restore `.formula` from a hard-coded string instead of the captured DOM text and require failure.
- **Test 12 "within 2 frames"** counts `state.frame`; fine. 
- **Test 15 (science gate)** is a model check, and the pass rule is "0 `error` verdicts and every text sourced". Say who dispatches and where the verdicts are recorded; the spec does (round-1.md). Add: the reviewer sees the register row, not only the string, so a `simplified` note can be wrong.
- **Test 17** (perf guard) is well specified; keep.

## 5. Science sourced?

The register approach (a row per shown string, `correct` / `simplified` / not-claim) is right and B6 stays honest. Spot-check of the pinned texts:

- `cast_foam`: sprout "This gas keeps air away." and seedling "heavier than air and pushes the air away" are true for CO₂ (44 vs about 29 g/mol); marked `simplified` correctly (CO₂ also absorbs heat). `won` seedling "Water, Frost and Smother each stop fire" uses a spell name the game calls `CO₂` in I-026's data. Apprentice `Smother` vs `CO₂` is deliberate and noted; but **I-028/I-026 cards show `CO₂` to sprouts** (spell name from `spells.json`). The hotbar says `Smother`, the Bench card says `CO₂`. Pick one for sprout/seedling: either the Bench uses `SLOTS[band][i].name`, or the hotbar keeps `CO₂` at seedling. I recommend the first, and an added line to I-028's planner edit.
- `out_water` "too cool to burn" and `fizzle_wet` "too wet to burn" are fine (S9, S16). `warning` seedling "Too hot and the wood will burn" is the piloted-ignition simplification already noted. `steam` "boils at 100 °C" is correct at 1 atm; fine for a greenhouse game.
- `cast_frost`: "Frost is a very cold liquid" hides the liquid nitrogen. True, but a teacher reading the HUD may ask what liquid. Add the name in the register note and keep apprentice as is.
- `fizzle_charred` / `burnt_out`: "Nothing left to burn" is `simplified` (charcoal can still glow). Acceptable.
- Not-claims rows for coach and state lines are fine; "every sprout coach line except the pick line" being a Not-claim is correct only if the pick line carries the claim row (it does).

No false statements found. Requirement: the new `caption_*` fields in I-026's data (item 2) get the same register rows and `checkReading`.

## 6. Can the lanes build from the contract alone?

Yes, with these fixes:

- **Three lanes, one seam that is red until the end** (the e2e). That is accepted, but the contract lane owns 3 existing tests found by a grep at merge time (test 16). Make that grep a **pre-dispatch planner step** and list the hits in the spec, so lane scope is fixed before dispatch (right now the lane's file list is discovered after dispatch).
- **The stub** (captions.rs with real `event_key` bodies) is fine, but with item 2's enum change the stub shrinks; update the "29" counts everywhere (`CAPTION_KEYS`, test 1, test 6, seam).
- **`areas.md`:** the spec adds `captions.rs`, `lines.rs`, `onboard.rs` to `client-core` at dispatch, and adds new web files (`band.js`, `captions.js`) to `client-web` by the existing `crates/bevy-client/web/**` glob. Fine. The panel files (`personas.mjs`, `panel.mjs`, `daemon.mjs`) are under `tests/e2e/**`, owned by tests-e2e; the spec assigns them to the contract lane. Fine.
- **`science-sources.md`** edits by the web lane while I-020 and others also add rows: ids are taken from the merged register at build time; fine.

## 7. Changes required before dispatch (checklist)

1. Replace "Reuse by I-028" with a hard interface (`window.__band` with `get`, `set`, `on`; `LEVELS`; `adept` stays `adept`); planner edits I-028 §2 and I-029 line 57 to use it; backlog "After" for I-028 gains I-021. (Item 2)
2. Change cast captions from four enum keys to `captionKey = "cast"` plus `captionArg = <spell id>`; tell I-026 to carry `caption_sprout` / `caption_seedling` in `spells.json`. Add the "no `SLOTS` row means name only" fallback. (Item 2)
3. Decide on speech for sprout captions (add using I-028's helper, or file the follow-up and downgrade GS-21 to partial). (Item 1)
4. Resolve the `Smother` vs `CO₂` name for sprout/seedling across HUD and Bench. (Item 5)
5. Soften the B4 prediction (clarity and band-free hints) or add `START_HINT` / `AIM_HINT` to `captions.js`. (Item 1)
6. Apply the smallest-version cuts: seedling cells only where the Rust text fails the rule; 11b/11c non-gating; seedling `warningFor` dropped; trim test 10. (Item 3)
7. Add the test 5 banned-word cross-check and the test 8 hard-coded-restore mutation. (Item 4)
8. Move test 16's grep of existing tests to a pre-dispatch planner step with the hits listed. (Item 6)
9. Mention `checkReading` as the checker for JSON-held sprout/seedling text (I-026, I-028, I-030). (Item 2)

After these, expect **approve**. No part of the item is `wrong-thing`: it adds no camera, control or 2D rendering and does not touch the core loop mechanics.

## Planner response

Revision 3 of `spec.md` applies these. Checklist items (section 7) first, then the other points.

1. Hard `window.__band` interface (`get`, `set`, `on`), `LEVELS`, `adept` stays `adept`; planner edits I-028 §2 / test 11 and I-029, and adds I-021 to I-028's "After": **accept**. A second band store gives a mixed-band game, so this is a real B4/GS-21 break.
2. Cast captions keyed by spell id (`captionKey = "cast"`, `captionArg = <id>`), `CAST_CAPTIONS` by id, I-026 told to carry `caption_sprout` / `caption_seedling`; "no `SLOTS` row means name only" fallback: **accept**. B8 needs 8 spells. Since I-021 can merge before I-026, `captions.rs` holds a 4-entry `spell_id(i)` (`fire`, `water`, `frost`, `co2`, the ids I-026 uses) that I-026 replaces with `Spell::spell_id()`. A spell id with no entry falls back to `state.spellName` at sprout/seedling, never to the formula caption. That removes the "I-021 before I-026" rule.
3. Speech for sprout captions: **accept the follow-up path**. I-021 merges before I-028, so I-028's helper does not exist yet, and writing a second speech helper here is the duplication this critique warns against. GS-21 is downgraded to "partial" and the follow-up "Speak sprout captions with I-028's speech helper" is filed at dispatch.
4. `Smother` vs `CO₂`: **accept the recommendation**. The Bench uses `SLOTS[band][i].name` at sprout/seedling, and that is added to I-028's planner edit. A subscript name fails the sprout rule, so the HUD cannot use `CO₂`.
5. Soften the B4 prediction: **accept softening**. The prediction now covers only what the item controls: no formula, subscript or kelvin blocker, and clarity not lower. Hint constants stay out of scope (I-007/I-019 strings; adding them grows the item).
6. Smallest-version cuts:
   - seedling cells only where the Rust text fails the rule: **accept**. I checked the rule against the c4832d2 strings. 15 of 25 non-cast keys and the Water cast pass (`spread` is 15 words with `{who}`, so it keeps its cell), so they now use the Rust text, which cuts 16 seedling texts and their register rows.
   - one-line `PICKER_BANDS`: **accept**.
   - 11b/11c non-gating: **accept**. They move to the seam breaker.
   - seedling `warningFor` dropped: **reject**. The apprentice warning has two temperatures (`${state.ghTemp} °C` and `300 °C`), so it fails the seedling rule. `lostFor` seedling already is the apprentice constant.
   - trim test 10: **accept**. It now covers `persona.band` and `pageUrl` only. The argv helper and the `session.json` field are dropped, and the `--band` flag stays.
7. Test 5 banned-word cross-check and test 8 hard-coded-restore mutation: **accept**.
8. Test 16 grep as a pre-dispatch planner step with hits listed: **accept**. The c4832d2 hits (golden `#hint` reads of band-free I-019 hints; `panel/hud.mjs` is a reader, not an assertion) are listed. The planner re-runs the grep on the I-020 merge base before dispatch.
9. `checkReading` named as the checker for JSON-held sprout/seedling text (I-026, I-028, I-030): **accept**.

Other points:
- Do not gate I-026 on this item: **accept**. This follows from 2.
- Delete the "I-028 may add its own listener" ambiguity and keep the `age.js` hook: **accept**.
- Default `seedling` with no pause switch: **accept as is**. The follow-up stays filed, and I-028's chooser calling `__band.set` covers part of it.
- Test 6: state that the sprout rule is a floor and that panel B4 is the real measure: **accept**.
- Test 15: the reviewer sees the register row as well as the string: **accept**.
- `cast_frost` register note names liquid nitrogen: **accept**.
- Test 17 rerun if I-024 lands between: **already in spec**, no change.
- Update "29" counts everywhere: **accept**. The count is now 26 keys: 25 event/state keys plus `cast`.
