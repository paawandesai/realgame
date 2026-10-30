# Critique of I-030 (mastery trial and keep-it-sharp), revision 1

**Verdict: revise.** The thing is right. `target.md` defines mastery as 3 unaided correct builds on 2 days plus 1 predict question, and says mastery never fades. The spec makes that reachable in play, keeps it opt-in inside the bench, keeps the world paused, and keeps crafting as the core. It is not `wrong-thing`. But it is bigger than the smallest version, it ships one number that contradicts the game's own data, a few tests cannot fail, and the contract disagrees with the glue text in places. All of this can be fixed on paper.

## 1. Right thing for kids 4-18 and teachers?

Yes, with these concerns.

- **Opt-in and calm.** "Show what you know" is a button the player presses. A wrong build costs nothing and can be retried at once. There is no timer, streak or notification, and nothing is lost by not returning. The GS-28 and GS-30 reasoning is sound. The question is the owner's rule, it is player-chosen, and Back dodges it.
- **The predict question is not crafting.** It is a multiple-choice screen, and GS-30 is the standard it brushes against. Accept it because the owner mandated it. Hold the line that it appears only after a build the player chose.
- **Several questions are trivia, not transfer.** `target.md` asks for "a new situation" that the spell's chemistry explains. These are not derivable from what the learner built or saw: `frost_sprout_2` (the flower freezes hard), `water_seedling_1` (wet log versus dry log) and the Sprout distractors "turns to ice" and "catches fire". A 4-7 year old can pass by elimination. That is acceptable for Sprout. The B9 panel number ("70% answer correctly") will mostly measure guessing, since chance is 33%. Say that in Planner notes so nobody reads 70% as proof of transfer.
- **Two variants make the question a memory test.** A wrong answer shows the right one, and `asked % 2` brings the same question back two counted builds later. After one miss a player can answer from memory. Rotating the choice order does not help, because the labels are unchanged. This is a conscious trade for the smallest version. State it, and do not call it "new situation" evidence. If the bank grows later, the validator's "exactly 2" is the right lever, and the spec already says so.
- **Classroom reality.** The 2-day rule means a single 45-minute class cannot produce a mastered spell. A teacher will see "Counted! Days: 1 of 2" and nothing else. That is the owner's design. Add one line to Planner notes for the teacher page (GS-29, P2) saying that mastery needs two sessions on different days, and that progress lives on that browser.
- **Unaided Seedling Fire is not unaided recall.** `trial_recipe` still shows the four formulas as tiles (`CH4`, `O2`, `CO2`, `H2O`). The player balances but does not recall the equation. At Apprentice they type the whole equation. That is defensible (balancing is the Seedling skill), but "from memory" in the header overstates it. Either reword the Seedling reaction header to "Balance ${name} without the scale", or say in the spec that this is deliberate.
- **XP farming.** Every counted trial build gives +20 XP, even once `builds` is capped at 3, with "no cooldown and no limit". Before `predicted` is true, a player can repeat this without end. XP has no sink until I-039, so it does no harm now, but I-028 test 21 sets a "no farming" precedent. Either award XP only while `builds < 3` (this needs a one-line change to I-027's `unaided_build`, which this spec may not touch) or record it in Out of scope as known and harmless.
- **Sprout misclick.** One submit per attempt is harsh for a 4-year-old who adds one extra atom card. It costs nothing, and the retry is one tap, so it is fine. Confirm that I-028's Sprout builder lets the child remove a card before Make, otherwise "Not counted" becomes the main experience for the youngest players.

## 2. Smallest version

The spec is 616 lines with 30 questions, 22 SVG icons, a new science source, a bank validator, a number parser, a 9-field save format and five lanes. Cut these:

1. **Drop the 8 Adept questions, `kind: number`, `judge_number`, `#trial-number`, `.trial-unit` and test 27.** The spec itself says real players cannot reach Adept (`?band=adept` reads as Apprentice) and that they see Apprentice questions until a follow-up. It is dead code that the science reviewer must still check, and it holds the one wrong number (section 3). Ship 22 questions: 6 Sprout, 8 Seedling, 8 Apprentice. Change the coverage rule to "every band from `tier_min` to Apprentice", and have the Adept follow-up add `number` and its questions. This removes `Number` from `PredictError`, test 3's number half, test 4's arithmetic and one golden test.
2. **Drop `sharp_day`.** Set `last_build_day = day` on a successful keep-it-sharp instead. The ready rule becomes `day >= last_build_day + 3`, and one field disappears. `asked` still needs the 9-field save, so the format change stays, but `sharp_day`'s validation and tests (8, 10) shrink.
3. **Keep-it-sharp is the one other cut candidate.** It is the only thing that tests B9's "after a break a from-memory check still passes", so keep it. But it is the part with the most state (field, save branch, constants, ready rule, button, strings, tests 8, 22, 24). If the lanes run long, split it as I-030b and leave mastery itself in I-030.
4. **Icons.** 22 hand-drawn flat SVGs is a large web-lane chore, and test 12 would pass on an empty `<svg viewBox>`. Keep them, but see section 4.

## 3. Science sourced

- **Must fix: `fire_adept_1` contradicts the game.** It asks for the heat from burning 16.0 g of methane and expects 890 kJ, which is the liquid-water value [S4]. The game's own Fireball is `CH4(g) + 2O2(g) -> CO2(g) + 2H2O(g)` with `delta_h_kj: -802.3` in `content/reactions.json` (`methane_combustion`). A student at Adept would see two heats for the same fire. The stem's note ("the stem rounds it") does not cover the 90 kJ gap. Either use 802 with the same NIST source as the reaction row, or make the stem say liquid water forms and give the value, and register the difference as a `simplified` note. Dropping the Adept questions (section 2) removes the item for now, but the Adept follow-up must carry this fix.
- `fire_seedling_2` cites [S14] (EPA CO2 suppressant review) for a jar flame going out. Cite [S19] or [S6] for oxygen and combustion instead.
- `frost_apprentice_2` is marked `correct`, but liquid air condenses near 77-90 K, so most of a real balloon's shrinkage past the gas-law limit is liquefaction. The stem says "shrinks to a small fraction". Mark it `simplified` with that note (the Adept stem already says "if the air stayed a gas"), or reword the stem to "shrinks a lot as it cools".
- `water_seedling_1` and `water_apprentice_1` are correctly marked `simplified`. Keep their notes in the register rows, not only in the spec.
- **[S21].** I-026 to I-029 introduce no S21 or higher, so the id is free now. Still say "next unused at build time" in the contract, as the spec does. Its fallback for the OpenStax 18.7 quote is sound. The cleaner path is to drop "Nitrogen gas is colourless" from `why` and keep a source-light rewrite, since one extra source costs a fetch and a review for one clause.
- Distractors as Not-claims plus test 33 (the reviewer confirms each is false as worded) is the right protocol. Add `co2_seedling_2` to the reviewer's watch list: an air balloon also sinks slowly, so distractor c ("They drop the same way") is false only in degree. Reword to "The CO₂ balloon floats up" and "Both stay up", which are clearly false, or make c "They drop equally fast".

## 4. Acceptance tests: can each fail?

Mostly yes, and each says how it fails today and on the stub. The exceptions:

- **Test 4** passes on the stub (the spec admits this). Require the mutation run (change one digit, see red) to be recorded in the lane's evidence, and have the checker re-run it.
- **Test 12** passes on `<svg viewBox="0 0 1 1"></svg>`. Add a floor: each icon's SVG string has at least 2 shape elements (`path|circle|rect|ellipse|polygon`) and no two icons have identical markup. Legibility to a 6-year-old stays a panel item, and the panel should be asked about it.
- **Test 15 and 31** use one regex. It misses "see you", "next time" and "return". Add `/see you|next time|return|later|reward|bonus/i`, but not "again", which "try again" legitimately uses. Also scan `rule`, `counted` and `sharp right` explicitly, because those are the strings closest to a lure.
- **Test 35** (+0.5 ms with the bench closed) cannot fail meaningfully, because the trial adds nothing to the frame loop. Keep it as a guard, but the real size risk is the wasm embed of `predict.json`; the 30 KB Brotli bound covers that.
- **Test 5 versus the contract.** Test 5 says `ask` then `on_answer(true)` moves to "Done", but `TrialView.phase` has separate `answer` and `done` values and `AnswerOutcome` has only `Answered` and `Ignored`. Say when the phase is `answer` (every answer, until the player leaves) and when it is `done` (only a final mastery transition), and assert both.
- **Test 17** is the right end-to-end, but it hard-codes the first wrong id. Say how the test finds it (read `bs().trial.question.choices`, pick the id that is not the answer, which the test gets from `predict.json`), or the lane will invent it.
- **Test 23** needs the `Date` shim to run before `progress.js` boots. Say that it is injected with `ctx.addInitScript`, the way golden already does for storage (`golden:941-944`).
- **Add a test:** a Learning spell whose player never opens the trial is still castable and still earns worked XP. I-029 owns this, but the trial must not add a gate. Test 32 covers the normal-build half only.

## 5. Lanes buildable from the contract alone?

Not yet. This is the main reason for `revise` rather than `approve`.

- **Everything assumes unmerged specs.** The spec says so and defers to the contract builder's Refresh note. That is workable, but two assumed names already disagree with I-029's revision 1: I-030 calls `mountBuilder(root, { band, spell, recipe, prompt, onMake })` (I-028's shape), while I-029 mounts it with `{ band, spell, seed, guided, compact, onSubmit }`. The merged builder may carry both. Name which options the trial uses and require the contract builder to verify `recipe: null` is accepted before the web lane starts, not "if I-028's cannot render without a recipe, the web lane makes that one change". A web lane that must edit I-028's `mountBuilder` while I-029's lane also edited it is the one real merge risk. Move hook 5 into the contract step.
- **Glue text versus contract signatures.**
  - Section 5 says `desk.on_answer(c, transitions)`. The contract says `on_answer(slot, correct, mastered_now, given, q)`. Section 5 says `desk.on_build(slot, v.ok)` and then `book.unaided_build(i)` and `desk.ask(q, asked)`, but the contract also has `note_counted(builds, days, xp_gain, mastered_now)`, which section 5 never calls. Rewrite section 5 to call every desk method in order, with the arguments, so the client and sim lanes cannot disagree.
  - `TrialDesk::refuse` exists in the contract, and section 5 says "the view carries `trial.phase = "refused"`". Say that `refuse` is what sets it.
  - `TrialView.phase` is a `String`. Make it an enum with the five values in `serde` snake_case, so a typo cannot ship.
- **The `rule` string has no home.** `Master it: 3 builds from memory on 2 different days, and 1 question.` is in the string table and the GS-28 walk says it is shown with the pips. But section 7, the DOM id list and the tests never place it (no class, no phase, no test). Put it in the list-view strip (`.trial-rule`), add it to the DOM list and test 31's scan.
- **The seam breaker's "recipe atoms" check** needs the Seedling Fire exception said plainly: `atoms` is `[]` and the formulas remain. As written ("never carries `recipe` atoms") it reads right but the golden assertion in test 21 is the one that pins it. Fine.
- **Lane file boundaries** are otherwise clean. The client lane may not touch `web/**` and the web lane may not touch `progress.js`, which matches the files each owns.

## 6. Overlap with I-022, I-024, I-025 and the other phase-2 specs

- **I-022, I-024, I-025:** none of the files they own (`main.rs`, `hud.js`, `feel.rs`, `analytics.js`, `age.js`, `world.rs`, `Cargo.*`, `perf.mjs`, `cues.rs`, `fx.rs`) is touched. Test 36 checks this. Shared appends are `golden.mjs`, `science-sources.md` rows, `build-web.sh:21` and `style.css`, all append-only with the call order stated. Fine.
- **I-026:** consumes `SpellBook`, `Tier`, `tier_min` and `nitrogen_boiling`. Test 4 pins the N2 value to 3 significant figures, which is the right dependency check. With the Adept cut it is no longer needed.
- **I-027:** adds two fields and a 9-field save. I-027's `SAVE_RE` and test 7 allow only the 7-field shape, so a 9-field save has to be shown to match `SAVE_RE` (test 24 does this). Say explicitly that I-030 edits I-027's regex only if it fails, and does so in `progress.rs`, not in `progress.js` (which test 36 forbids touching). If `progress.js` validates the save shape before `prog load`, a 9-field save could be dropped on reload. Check that now, because test 24 would catch it only late.
- **I-028:** adds three command words to `bench::run` and two optional `BenchView` fields. `BenchView` serialisation is pinned by I-028's tests, and I-030 keeps closed-view JSON unchanged, so that is safe. The `bench.js` hooks are small. The `try` branch (route to the trial while one is open) is the one place that can call `learn` by mistake, and the seam breaker lists it.
- **I-029:** worked casts do not count, which is a clean decision. The instant route flips on the next frame after mastery (seam check 6 and test 17).
- **B9 measurement gap.** The spec says the panel needs a "next day" step and files it as a follow-up. B9 cannot be scored without it, since mastery cannot happen in one calendar day. Do not leave this as a Planner note. Add it to the backlog as a numbered item with "Before: first B9 scoring", and put that id in this spec.

## Specific changes (in order of importance)

1. Fix or remove `fire_adept_1` (890 versus the game's 802.3); carry the fix to the Adept follow-up.
2. Cut the Adept questions, `number` kind, `judge_number` and test 27; the bank becomes 22 questions. File the Adept follow-up line.
3. Make section 5 and the contract agree: every `TrialDesk` method called in order with its arguments; `phase` as an enum; define `answer` versus `done`; say what `refuse` does.
4. Place the `rule` string (class, view, DOM list, test 31).
5. Pin the `mountBuilder` options the trial uses, and move hook 5 to the contract step with a check that `recipe: null` renders.
6. Tighten tests 12, 15/31, 17 and 23 as in section 4, and record the test 4 mutation run.
7. Re-mark `frost_apprentice_2` as `simplified` (liquefaction), re-cite `fire_seedling_2`, and fix `co2_seedling_2` distractor c.
8. Cap or record XP farming, and reword the Seedling reaction header ("without the scale").
9. Add the teacher line (two sessions; browser-only) to Planner notes, and a note that 70% correct on a 3-choice question is mostly a guessing floor.
10. Turn the panel day-2 follow-up into a numbered backlog row ordered before B9 scoring.
11. Optional: drop `sharp_day` in favour of `last_build_day`.

After items 1-5 this is an approve.

## Planner response

Spec revised to revision 2. Each change, in the critique's order:

1. **fire_adept_1 (890 vs 802.3):** Accept. Removed with the Adept cut; Planner notes tell the Adept follow-up to use 802 kJ/mol from `methane_combustion` or state liquid water with a `simplified` note.
2. **Cut Adept questions, `number`, `judge_number`, test 27:** Accept. 22 questions, coverage to Apprentice, `pick(.., Adept, ..)` reads Apprentice, `#trial-number`/`.trial-unit`/`check`/`wrong number`/`q adept` gone; test 27 now checks that Adept reads Apprentice.
3. **Section 5 matches the contract; phase enum; answer vs done; refuse:** Accept. Section 5 lists every desk call in order with arguments, `TrialPhase` is a snake_case enum, a phase table defines `done` (attempt over, no open question) vs `answer` (question answered), mastery is the `masteredNow` flag in either phase, and `refuse` is what sets `refused`. `QuestionBook::get` added for the `answer` path.
4. **Place the `rule` string:** Accept. `.trial-rule` in the list view, in the DOM list, and scanned in tests 15 and 31.
5. **Pin `mountBuilder` options; hook 5 into the contract step:** Accept. The trial passes exactly `{ band, spell, recipe, prompt, onMake }`. I-028's current spec (test 11b) already requires `recipe: null` to render, and I-029 uses the same shape plus `guided`, so the claimed `{seed, compact, onSubmit}` mismatch is outdated. The contract builder still checks null recipe and empty-atoms Fire terms, and adds hook 5 only if that check fails. The web lane never edits `mountBuilder`.
6. **Tighten tests 12, 15/31, 17, 23; record test 4 mutation:** Accept. Icon floor (2 or more shapes, no duplicate markup), extra lure regex plus explicit strings, test 17 reads the correct id from `predict.json`, test 23 uses `ctx.addInitScript`, test 4 (now about the numbers stated in the text) has a recorded mutation run that the checker repeats. Test 5 now asserts both phases. New test 37: no trial gate on worked casts. Test 35 kept as a guard, unchanged.
7. **Science fixes:** Accept all. `frost_apprentice_2` stem now says "shrinks a lot as it cools" and is marked `simplified` (liquefaction); `fire_seedling_2` cites S7 S19 S6; `co2_seedling_2` c is "They drop equally fast". Also took the cleaner [S21] path: dropped "Nitrogen gas is colourless", distractor a is "The nitrogen is burning.", and no new source is needed. `simplified` notes must go into the register rows.
8. **XP farming; Seedling reaction header:** Header accepted ("Balance ${name} without the scale: ..."), and the spec now says the visible Fire formulas are deliberate. The XP cap is rejected because it needs a change to I-027's `unaided_build`. It is recorded in Out of scope as known and harmless until I-039 gives XP a sink.
9. **Teacher line; 70% note:** Accept. Both are in Planner notes, and Bars moved says 70% is not evidence of transfer. The 2-variant memory trade-off is stated in the rules.
10. **Panel day-2 as a numbered backlog row:** Accept. Reserved as I-041 (next free id after I-040 on origin/main), with exact row text and "Before: first B9 scoring" in Planner notes. The leader adds the row, because the backlog lives on origin/main and not in this item's files.
11. **Drop `sharp_day`:** Accept. Keep-it-sharp sets `last_build_day`, which only `unaided_build` reads and which does nothing on Mastered spells. The save becomes 7 or 8 fields (`asked` only). I-027's `SAVE_RE` (`^p1(\|[a-z0-9:.]+)*$`) does not count fields, so `progress.js` needs no change. The charset in "Assumed" is corrected to `[a-z0-9:.|]`.

Other section 1 points: Sprout misclick is confirmed fine (I-028's Sprout builder has Undo before Make, now noted in "Assumed from I-028"). Keep-it-sharp stays in I-030; split it into I-030b only if lanes run long.
