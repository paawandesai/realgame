# I-021 spec critique

**Verdict: revise (small, mechanical changes; direction is right).** The item is the right thing for 4-10 year olds and their teachers, and it is the item the critic asked I-020 to split out. Two defects would make the build or the tests fail as written (section 3, items A and B). One scope cut is recommended (item 5). Everything else is a tightening.

Reviewed: `docs/loop/items/I-021/spec.md`, `I-019/spec.md` (lanes and contract), and `target.md`, `bars.md`, `areas.md`, `gameplay-standards.md` from origin/main (a3e50b1). Code facts spot-checked at a3e50b1: the `age.js` click handler and `picked` guard, the `index.html` slot spans and `data-band` buttons (4 to 7, 8 to 10, 11 and up), the `hud.js` `showCaption` / `history` / `.small` sites, and the 21 `EventKind` variants in `chem.rs`.

## 1. Right thing for players? Yes

- It attacks the evidence directly: the panel's 6yo and 9yo text blockers on formulas, "100 degrees" and kelvin, and B4 clarity of 6 against a bar of 8. It stays first-person and adds no control, so the hard rule in `target.md` is untouched.
- GS-21, GS-32, GS-17 and GS-19 are all moved by this item, and the walk table is honest about the partial ones (GS-18).
- Keeping the 11-and-up text byte-for-byte as it is today is correct for teens and teachers. The "if the band is known" wording of the B4 claim, plus the extra no-band 6yo guard, is the honest way to measure it.
- Doubts, none blocking:
  - **Pre-readers still rely on words.** Sprout captions get an icon, but the sprout coach line in `#hint` is not said to get one (GS-21: nothing by text alone). I-020's `COACH` carries icons. State that sprout `COACH_TEXT` lines reuse the step icon from `COACH`, and add one assertion for it in test 11.
  - **The default is `seedling` for anyone with DNT or GPC, or with analytics off.** That includes teens (mild GS-26 risk) and 4 to 7 year olds, who get no sprout text. The spec admits this. It is acceptable for a first version only because a reload brings the picker back on the deployed site. Keep the pause-card switch as a named follow-up item, not just a sentence.
  - **The spell is renamed `Smother` for sprout and seedling, but is `CO₂` for apprentice.** In a mixed class ("press 4, the CO₂ one") the names differ across neighbouring screens. It is defensible (GS-30 wants the mechanism named), but the `station` / `won` / pick lines for apprentice still say "CO₂". Add one line to the spec saying this is deliberate, so the science reviewer and the panel brief do not flag it as a bug.
  - **The sprout `start` caption drops the instruction I-019 just added.** I-019's boot caption ends `Water is ready: aim at a fire and click.` Sprout `Put out the 3 fires!` and seedling `Put out the 3 fires before they spread.` lose "aim and click". The coach covers it only after the walk step. Make the seedling text keep the verb: `Put out the 3 fires! Aim at a fire and click.` (check it against the 14-word and one-temperature rule).

## 2. Smallest version? Mostly; cut one piece

It is large (29 keys, two bands, slots, warning, lose line, coach, panel plumbing, about 60 register rows, three lanes), but each part answers a named blocker, and the band-aware panel and the one-line hook are what make the result measurable. Do not shrink the caption tables.

Cut (item 5 below): `COACH_TEXT.apprentice`. It rewrites the hint the 13yo, the teacher and the QA persona already read and did not complain about. It costs four register rows and one more code path. It also contradicts the bar line "13yo, teacher and qa see the same text as before".

## 3. Can every test fail? Mostly; two tests cannot pass as written, three are brittle

**A. Test 14 contradicts the pinned tables (the build would fail).** It greps `captions.js` for every line of `caption-strings.txt` and expects none. But five seedling cells are byte-identical to the Rust text:
- `reseeded` `Another bush is on fire! Put it out before it spreads.`
- `melting` `Melting... it takes a lot of heat to turn ice into water.`
- `out_fire` `Fire out!`
- `fizzle_not_flammable` `That does not burn.`
- `greenhouse_caught` `Oh no! Heat from the fire reached the greenhouse. Press R to try again.`

The `lostFor` seedling text is likewise a copy of the apprentice text. Fix one of two ways. Either `CAPTIONS` entries may be omitted when the seedling text equals the Rust text, and `captionFor` falls back to `state.caption` (then test 6 asserts the key set as "listed keys plus a named `SAME_AS_RUST` list"). Or test 14 is limited to lines that differ from a pinned seedling text. The first is simpler and removes copies. If chosen, `checkReading` must still be run on those five Rust strings under the seedling rule in test 6, which reads `caption-strings.txt`. Check `melting`, which has `...` and 12 words, against the 14-word cap. `reseeded` has 11 words.

**B. Test 1's source-grep is brittle and has a simpler alternative.** Counting `.say(` calls against string literals in `include_str!("main.rs")` breaks on any reformat, any comment containing `.say(`, or I-019 and I-020 rebases. The compiler can do this: make `say` take a `CaptionKey` enum (or a `&'static str` const from `captions::K_*` constants). A wrong key then fails to compile, and the test reduces to "the enum's `ALL` equals `CAPTION_KEYS`". Keep the 21-variant `event_key` test, which is good. If the grep stays, state that it is a guard against a dropped argument only.

**C. Existing tests of I-019, I-020 and I-007 may read the text that now changes with the default band.** Test 16 only checks `golden.mjs`. I-019's web lane writes e2e tests 7 to 12 and 14 to 19 and I-020 writes a panel and e2e set. Any of them that reads `#caption`, `#warning .text`, a `.slot .formula` text or `#hint` text under no `?band` will go red after this merge, because the default is now `seedling`. Replace "grep golden" with "grep every file under `tests/e2e/` and `crates/bevy-client/web/*.test.mjs` for `#caption`, `#warning`, `.formula`, `#hint` and `#history` text assertions; each is either moved to `?band=apprentice` or reads `state.caption`, and the list is recorded in `round-1.md`". The seam breaker owns the check.

**D. Test 11 is long and order dependent.** It chains hold, face, select, cast, `tick 2400`, history match, warning, lose, plus a no-formula regex over the whole HUD. If any step is flaky the whole contract lane is red with no clue why. Split into 11a (slots, hint, caption icon with size), 11b (history line after a spread) and 11c (warning and lose text and the no-formula scan). Same assertions, three tests. Also confirm `tick 2400` really yields a *spread* ignition on a non-greenhouse target with the I-020 seed. If not, use the Rust test 2 path to pin the seed instead of an `if`.

**E. The sprout reading rules are checked on a placeholder, not on the runtime text.** `Oh no! Heat spread and lit the {who}!` counts `{who}` as one word, but at runtime `dry hedge` makes 9 words (above the 8 cap), and test 5 itself asserts that 9-word line. Either count `{who}` as two words in `checkReading`, or drop a word (`Oh no! Heat lit the {who}!` is not clearer, so prefer `Heat spread! The {who} caught fire!`, 6 or 7 words, 8 with `dry hedge`).

Smaller points:
- Test 8 builds a fake DOM by regex over `index.html`. That is fine, but say the regex must fail loudly if it finds other than four `.slot` buttons, so that an `index.html` edit by I-019 does not silently pass.
- Test 17's `+20 KB` is a deterministic Brotli check. Good. Add that `captions.js` is lazy-loaded or inlined, not a second blocking request, or say that one more 4 KB request is accepted.
- Test 15 (science review) needs a stated fallback: `docs/loop/science-review.md` is named, but confirm it exists on the merge commit. If it does not, the contract lane must write the reviewer prompt into `round-1.md`.
- Test 7 asserts that `git diff` removes no register row. Fine. The planner should also confirm C-030 and C-033 are the current ids on the merge commit, because I-019 and I-020 each add rows.

## 4. Lanes buildable from the contract alone? Yes, with three fixes

- **core:** buildable. The string-to-key table and the 21-variant listing are complete. Fix B above.
- **web:** buildable, and unusually well pinned. Fix A, C and E above, and name the icon for each sprout coach line (item 1 above). The hook line and its test are well specified. One ambiguity: `startHud()` creates the band, but `hud.js` is also edited by I-019 and I-020 for the hint and the counters. The spec says "change only the blocks named above", which is enough. Add that the slot capture happens once, before any band-dependent write, so the first sprout paint never flashes the formulas for a frame (GS-6 is not at risk, but the screenshot in the panel could be taken in that frame).
- **contract:** buildable. `pageUrl`, the `--band` flag and the argv helper are clear. The "keep the operator's `band`" rule and `none` are tested.
- **Overlap with I-019: none that blocks.** I-019 owns `cues.rs`, the `fire` command, `fx.rs`, `hud.js` (counter, finder, crosshair, hints), `style.css`, `golden.mjs` and ten register rows. I-021 touches `main.rs` (`say`), `lines.rs`, `captions.rs`, `hud.js` (caption, history, warning, slots, lose line), `style.css`, `age.js`, `coach.js`, `panel/**`. The overlap is on shared files only, and I-021 is blocked until both I-019 and I-020 merge, so the conflicts are rebase-only. Two real interactions to write down:
  1. I-019 changes the boot caption and the Water-at-boot text, which is why the `start` key sits on a moving target (see item 1 above). The `captions.rs` stub's `start` mapping is correct for either text.
  2. I-019's `#obj-line` counter and pips and its locked hint are band-free strings that this item deliberately leaves out. Say so in "Out of scope" next to the `IDLE_HINT` line, since sprout players will see "Fires to put out: 7" in words.
- `areas.md`: `captions.rs` is a new file in `crates/bevy-client/src/`, which `areas.md` lists file by file (`main.rs`, `chem.rs`, `fx.rs`). It is not listed, nor is `lines.rs` (I-020). Name it as owned by the core lane in this item and ask the planner to add the two files to `areas.md`, so the leak check in test 18 is not the only guard.

## 5. Requested changes, in order

1. Fix test 14 against the five seedling cells that equal the Rust text (3A). Pick the fall-back-to-`state.caption` option.
2. Replace the source-grep in test 1 with a compile-time key type, or demote it to a guard (3B).
3. Replace "grep golden" in test 16 with a grep of all e2e and web tests for text assertions on `#caption`, `#warning`, `.formula`, `#hint`, `#history` (3C).
4. Count `{who}` as two words in `checkReading`, or reword the sprout `spread` line (3E).
5. Cut `COACH_TEXT.apprentice`: apprentice uses I-020's `COACH` text unchanged. Remove its four rows, its test cells and the related contract line.
6. Make sprout coach icons explicit and tested (item 1); keep the click instruction in the `start` text (item 1).
7. Split test 11 into three tests (3D) and pin the seed for the spread step.
8. Add the notes: `Smother` versus `CO₂` is deliberate; I-019's counter and hints stay band-free; `captions.rs` and `lines.rs` are added to `areas.md` by the planner; `science-review.md` exists on the merge commit or the prompt is written into `round-1.md`.
9. Promote the pause-card reading-level switch to a filed follow-up item, since it is the only way a DNT player gets the sprout level.

After items 1 to 4 the spec is ready to build once I-019 and I-020 have merged and the stale check passes.

## Planner response

Spec revised to revision 2 (`spec.md`). All nine requested changes are accepted; the smaller points are accepted too.

1. **Accept** (3A): fall-back option taken. `SAME_AS_RUST` lists the five keys, their `CAPTIONS` entries have no `seedling` field, `captionFor` returns `state.caption`, and test 6 runs `checkReading` (seedling) on the five Rust texts as fixtures checked against `caption-strings.txt` (`melting` 12 words, `greenhouse_caught` 14, both pass). `lostFor` seedling shares the apprentice constant; it is not a Rust string, so test 14 is unaffected.
2. **Accept** (3B): `CaptionKey` enum with `ALL`/`as_str`/`Default`; `say`/`say_arg`/`Line.key` take it, so a keyless call does not compile; the source grep is removed and test 1 asserts `ALL` matches `CAPTION_KEYS`.
3. **Accept** (3C): test 16 now greps all of `tests/e2e/` and `web/*.test.mjs` for HUD-text assertions; hits move to `?band=apprentice` or `state.caption`, are listed in `round-1.md`, the contract lane may edit them (scope test 18 allows them), and the seam breaker re-checks.
4. **Accept** (3E): both fixes. `{who}` counts as two 5-letter words in `checkReading`, and the sprout line is now `Heat spread! The {who} caught fire!` (7 words with `dry hedge`).
5. **Accept**: `COACH_TEXT.apprentice` cut; apprentice and seedling use I-020's `COACH`; its four register rows and test cells removed; test 12 expects `Hold W to walk forward.` under apprentice.
6. **Accept**: sprout coach keeps `COACH[step].icon` (asserted in tests 5 and 11a). `start` keeps the verb: seedling `Put out the 3 fires! Aim at a fire and click.` (10 words), sprout `Put out 3 fires! Aim and click!` (7 words, one digit).
7. **Accept** (3D): test 11 split into 11a/11b/11c. The spread step is a fixed deterministic loop on `state.captionKey === 'spread'` (cap 60 x `tick 100`) that fails, never branches, if the greenhouse or the cap comes first. A Rust seed cannot be used from the e2e page, so this is the pinned path.
8. **Accept**: notes added for `Smother` vs `CO₂` (deliberate, §4), I-019's counter/pips/hints band-free (Out of scope), `captions.rs`/`lines.rs` added to `areas.md` by the planner as a pre-dispatch step outside the item diff (keeping the rule that the item does not edit `areas.md`), `science-review.md` confirmed present at a3e50b1 with a `round-1.md` fallback in test 15.
9. **Accept**: pause-card reading-level switch is a named follow-up the planner files in `backlog.md` at dispatch (pre-dispatch step 6 and Out of scope).

Smaller points: **accept** test 8 failing loudly unless exactly four `.slot` buttons; **accept** test 17 note (two small same-origin module requests accepted, counted in TTI); **accept** test 15 fallback; **accept** C-030/C-033 re-check at dispatch (both confirmed at a3e50b1); **accept** the slot capture once in `startHud()` before any band write, with a write-spy assertion in test 8.
