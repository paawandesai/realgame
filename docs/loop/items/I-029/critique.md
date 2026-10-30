# Critique of I-029 (worked casts, then instant casts), revision 1

**Verdict: revise.** The thing is right and `target.md` mandates it: before mastery a cast is a worked cast, the world holds, mastered spells fire at once. The routing idea (one gate before the cast loop, `paused` reused for the hold, no new `control` parameter) is sound and feasible against c4832d2. But the spec was written when I-028 had no spec. I-028 now exists (revision 1) and disagrees with almost every name in "Assumed from I-028". Lanes cannot build from this contract alone. There are also three player-facing gaps and two defects that the acceptance tests would lock in. None is a wrong-thing problem, and all can be fixed on paper.

## 1. Right thing for kids 4-18 and teachers?

Yes, with these concerns.

- **The band is not known when the first click happens.** I-028 deliberately never stores the band. It resolves it from `?band=`, then an age-picker click made on this page load (the picker shows only when analytics is on), then a choice made inside the bench, else it asks. On a fresh page load with no analytics (the `pkg-out` build, a shared classroom link without `?band`), nothing sets I-029's assumed `Band` resource. The first click of the session is the B4 moment (the 6-year-old lands a cast within 30 s), and the spec does not say what the builder shows.
  - Change: when the band is unknown, the overlay shows I-028's picture chooser (three 160 px pictures, one click) and then continues into the build. Do not default to Sprout, because a 13-year-old shown picture cards for a first cast is the GS-27 "talking down" failure. Add a golden check for it. The band stays page-load only, as I-028 requires.
- **The payoff is undiscoverable.** Learning to Practised takes 3 worked casts. After that, Practised casts keep opening a builder and earning +10 XP with no cap and no sink until I-039. Mastery needs I-030's trial at the bench. The Practised toast says "The pieces stay in the same places now", which says nothing about how to get instant casts. B9 needs "masters 3 spells and uses an instant cast within 20 minutes", and a player on the cast path alone will not know the next step.
  - Change: the Practised toast and the header of a Practised build both carry one neutral line, for example "Show what you know at the Formula Bench (B) for instant casts." Icon plus text, no pressure, and mirrored in the not-claims rows. Add an assertion that the line is present (G8).
- **The cost of a click.** I-028's real builders need these inputs:
  - Sprout: tap an atom card per atom, then Make. Water is H, H, O, then Make, which is 4 inputs.
  - Seedling single-substance spells: count tiles cycled 0 to 4 by tapping, then Make.
  - Seedling Fire: a coefficient tile per molecule on the balance scale, then Make.
  - Apprentice: typing.
  - Against the spec's own constants (1.0 s prep, 1.5 s per Sprout pick) that is `estS` about 7 s for Water at Sprout, so G14's "estS <= 4 for Water at Sprout" fails with I-028 as written. The "1 to 2 clicks" the spec promises for the 6-year-old exists only if pieces submit on pick, and I-028 does not do that.
  - Pick one before the contract step. Either:
    - (a) relax the Sprout bound to what I-028 gives (4 inputs, about 7 s, still inside target's 3-10 s) and say plainly that a 5-year-old's first fire fight is 4 taps per cast; or
    - (b) ask I-028 for a `compact` Sprout mode where, on a guided route, the highlighted pieces are the whole build and Make is the only press.
  - I recommend (a) now and (b) as a panel-driven follow-up.
- **GS-30 and the 13-year-old.** The spec flags the quiz-feel risk itself. The risk is that every Learning-stage cast is an interruption. It is mitigated by the world holding, and the 3-cast hop to Practised is short. Keep it, as `target.md` mandates the mechanic. Make the panel brief ask specifically whether a rebuild per cast reads as a quiz (GS-30 [panel]), and keep the paired base-versus-artifact run as the decision point.

## 2. Smallest version

The spec is large for one item: a Rust state machine with 4 routes and seeds, a virtual cursor under pointer lock, badges, a toast, an XP pop, 17 golden checks and a timing table. Cut or defer these:

- **Cut `remembered` and `fresh` as separate routes, and cut the seed.** "Learning shuffles, Practised keeps layout" appears nowhere in `target.md` and has no learning-science source. I-028's `mountBuilder` takes no seed, so it also forces a rewrite of a builder that is not this item's. Replace the table with three routes: `instant`, `worked`, `guided`. Drop `seed`, `Fresh` seeds, test 4, G8's order checks and the "same places" copy. If shuffling is wanted, that is a later pedagogy item with a source.
- **Cut the XP pop.** `workDone` and `xp` already prove the XP, and a stage toast plus a badge cover the feedback. A separate `#xp-pop` is a second effect to get right under GS-6 and GS-3. Keep it only if the panel asks.
- **Keep:**
  - the gate, the hold, the virtual cursor (without it a mouse-only 4 to 7 path under lock does not exist, GS-11), and `keyup` pass-through;
  - the stage badges, because B9's "uses an instant cast" needs them to be observable;
  - help after two misses, which is cheap once the checker returns a reason.
- **Cut `WorkCtx.band` plumbing that duplicates I-028.** See 4.

## 3. Can each test fail?

Mostly yes. The units and G3, G4, G5, G9, G10, G13 and G14 each name how they fail on the stub or at base. Problems:

- **Test 10 locks in a typing bug.** It requires `preventDefault` on `Digit2`, `KeyW` and `Enter` while open. An Apprentice cannot type `H2O` or `2O2` if `Digit2` and `KeyW` are `preventDefault`ed. `stopPropagation` alone keeps Bevy from seeing the key and does not stop the character being inserted. I-028 specifies this correctly (stop, never `preventDefault` on printable keys).
  - Change: while a text input in the builder has focus, `key(e)` stops propagation only and never calls `preventDefault`, except on `Enter` (submit) and `Escape`. In typed bands `data-key` piece picks are disabled. Rewrite test 10 for both modes, and add a golden check that typing `2CO` for Seedling or `CO₂` for Apprentice reaches the input.
- **G14 `estS` cannot meaningfully fail.** It is arithmetic on an input count with invented constants. It checks the representation's input count, which is fine. State that it is a tripwire, not a measurement, and keep the real check on `openFrames`, `castFrames` and the 400 ms overhead. Fix the Sprout expectation per section 1.
- **G9, G7 and G6 lean on the `prog` debug commands** and on `ANSWERS` copied from I-028 fixtures. That is acceptable. Pin the table to I-028's `judge` fixtures by name so a drift fails a test.
- **There is no check for the band-unknown first cast** (section 1) or for exclusivity with the bench (section 4).

## 4. Overlap and lane contract versus I-028 and I-030

**Names that do not match the merged-to-be I-028.** The spec's "If I-028 differs" fallback ("map 1:1, add options in a separate commit") does not cover this much divergence, and it leaves the lanes unbuildable.

| I-029 assumes | I-028 actually specifies |
|---|---|
| `bench::check(spell, band, answer) -> Verdict::{Correct, Wrong(&'static str)}` in client Rust | `realgame_sim::bench::judge(spells, slot, band, text) -> Verdict { ok, code, diffs, other, … }`, with `to_json`. Reasons come from JS `reasonFor(band, result, names, tries)`. |
| `mountBuilder(el, { band, spell, seed, guided, compact, onSubmit })` returning `{ pick, undo, submit, reveal, showReason, pieces, destroy }` | `mountBuilder(root, { band, spell, recipe, prompt, onMake })`. No seed, guided, compact, `data-token`/`data-key`, or handle. |
| `band <id>` command and `Band(Tier)` resource | None. The band is resolved in JS (`bandFrom`), never stored. `adept` is read as `apprentice`. Bench commands are `open`/`pick`/`try`/`close`. |
| A `reason` key `[a-z_]` shown at the band's level | `reasonFor` needs the whole verdict (`diffs`, `other`, `tries`) to say "Count again: H should be 2. You have 1." A bare key loses that. |
| Answer is "a card id, a tile sequence, or typed text" | The answer is a composed formula text (`composeSprout`, `composeSeedling`, `composeEquation`), which is what `judge` takes. Good: use it. |

Changes:
- Rewrite "Assumed from I-028" as a table of exact I-028 names, and move the unmet pieces out of "additive options if needed" and into one of two places. Either ask I-028's author (now, while it is revision 1) to add `seed`-less `guided`, `compact`, `data-token`/`data-key` on pieces, `onMake` returning the text, and the handle. Or make a named contract commit by this item, to `bench.js`, covered by tests and reviewed against I-028's golden section. Do not leave it as an "if" in the web lane, because that is a rewrite of a file I-028 owns.
- `work submit` carries the composed text. Rust calls `judge` through `bench.rs` glue (I-028's `Progress`/`BenchView` already live there). `Desk` stores the verdict JSON, and `state.work.verdict` carries it, so `reasonFor` can run in JS. Replace `set_reason(key)` and the `[a-z_]`-only `reason` field.
- The band comes from I-028's `bandFrom`/chooser, and Rust gets it through an explicit command `band <id>` that `bench.js` already has reason to send (two lines in I-028's contract). Say `adept` maps to Apprentice until the Adept item, and drop `adept` from G14 until then, or run it as an alias row.

**Missing lane ownership.** `build-web.sh:21` copies web files from an explicit list (`index.html style.css boot.js hud.js favicon.svg`). `hud.js` imports `./worked.js` statically, so the built site breaks on a missing module unless the list gains `worked.js`. The spec's lanes and G17 hygiene list do not own `build-web.sh`. I-030's overlap table already assumes I-029 appended `worked.js` there (and I-027 and I-028 do the same for their files). Add `crates/bevy-client/build-web.sh` (the `cp` line only) to the web lane and to G17, and add `worked.js` and `bench.js` presence to a B7/B3 packaging check (`build:site` output).

**Key-handler collision with the bench.** `startBench()` registers its own window capture `keydown` listener (KeyB opens the bench; `stopImmediatePropagation` while open). `createWorked` inside `startHud()` registers earlier. `stopPropagation` on the same target does not stop the bench's listener, so `B` pressed during a worked cast would open the bench over the overlay, and both would mount `mountBuilder` (shared ids and classes).
- Change: state mutual exclusion. While `work` is open the bench cannot open (worked.js uses `stopImmediatePropagation`), and while the bench is open nothing opens a builder (it is paused, so a click is dropped by the existing paused rule). Add a golden check for `B` during a worked cast, and one for a click while the bench is open.

**I-022, I-024, I-025:** the overlap table is accurate. Merge I-025 first (the hold gate at 1069-1077 sits next to this gate) and place the gate after the held-path block. One addition: I-025's "one cast per click" fix touches the same `clicked` and hold-gate lines, so the unit test that "one click casts once" (seam breaker) should run after rebasing on I-025, not on the stub.

**I-026, I-027:** consistent. `learnable = tier_min <= band` is I-028's `NotForBand` rule, so call it the same way instead of reading `spell.def().tier_min` separately. `Book::worked_cast` semantics match I-027 (0 on Unknown, +10 Learning/Practised/Mastered, 0 on a wrong build).

**I-030:** consistent. Worked casts do not call `unaided_build`, `routes[i]` flips to `instant` on mastery, `worked.js` is on the `cp` line, and the `masteryTrial` call goes after `workedCasts`. One shared fix: both specs should say that the practice "Show what you know" trial lives in the bench and the field overlay never offers it, so the Practised-toast pointer in section 1 is the only link between them.

**I-028 mistake to flag back:** I-028 says "stage shown on the hotbar (I-029)" and keeps `KeyB`, band and `mountBuilder` as its own surface. Confirm in I-028's revision that the `guided`/`compact`/key options are listed there, not left to this item.

## 5. Science and standards

- No new science claim is introduced, and chrome text goes to Not-claims rows. That is right. Formulas and reasons come from `spells.json` and `bench.js` (I-026 and I-028 rows). The shuffle and "remembered layout" rationale is unsourced pedagogy, another reason to cut it (section 2).
- GS-3, GS-4, GS-5, GS-6, GS-7, GS-9, GS-11, GS-12, GS-15 to GS-19, GS-24, GS-25 and GS-28 are met as written. GS-21 is met only if the guided line stays below grade 6 (it does). GS-22: the 64 px Sprout target is I-028's, and G14 should assert the measured size in compact mode, not only the breaker at 1366x768.
- GS-23 (learn by playing, in a safe space): the guided Unknown build is a good safe first cast. Say so in the walk.

## Required changes (checklist for revision 2)

1. Replace "Assumed from I-028" with the exact I-028 names (`judge`, `onMake`, `bandFrom`, recipe and prompt). Decide where the missing builder options live (I-028's revision or a named contract commit here). Pass the verdict JSON, not a reason key.
2. Specify the band-unknown first cast: show I-028's chooser once per page load, then build. Add the golden check.
3. Cut `remembered`/`fresh` and the seed (three routes). Drop test 4 and the G8 order checks. Cut the XP pop unless justified.
4. Fix test 10 and the key handling: no `preventDefault` on printable keys when a text input has focus, disable `data-key` picks in typed bands, add a typing golden check.
5. Add the bench pointer to the Practised toast and builder header, and assert it.
6. Settle the Sprout input count and G14's `estS` bound (relax to the real count, or ask I-028 for the compact mode). Label `estS` a tripwire.
7. Add `build-web.sh` (the `cp` line) and a packaging check to the web lane and G17.
8. State bench and worked-cast mutual exclusion (use `stopImmediatePropagation`) and test `B` during a worked cast.
9. Add the GS-30 [panel] question to the brief note and keep the paired panel run as the decision point.
10. Refresh the "I-028 has no spec" lines and the "Open decisions" and "If I-028 differs" text now that I-028 exists.

## Planner response

Spec revised to revision 2 (`spec.md`). Each change:

1. **Exact I-028 names; where missing builder options live; pass the verdict, not a key.** Accept. "Assumed from I-028" is now a table of I-028 names (`judge`, `learnable`, `recipe`, `prompt`, `BenchView`, `mountBuilder(root, { band, spell, recipe, prompt, onMake })`, `reasonFor`, `bandFrom`). Four additions (`currentBand`/`chooseBand`, `mountChooser`, `guided`, a `{ destroy }` handle) are requested of I-028's next revision, with a named contract commit here as the only fallback. `work.view` carries a full I-028 `BenchView` (with `result`), so `reasonFor` gets the whole verdict; `set_reason` and the `[a-z_]` key are gone.
2. **Band-unknown first cast shows I-028's chooser.** Accept. New route `choose` opens the overlay on the chooser, the pick is shared with the bench via `chooseBand` (page load only, never stored), then Rust re-routes the same worked cast. No default band. Golden G18 added.
3. **Cut `remembered`/`fresh`, the seed, test 4, G8's order checks and the XP pop.** Accept. Routes are `instant`, `worked`, `guided` (plus `choose` from change 2); G8 now checks the Practised toast and the bench pointer; `#xp-pop` removed.
4. **Key handling: no `preventDefault` on printable keys; disable `data-key` picks; typing check.** Accept, and go further: per-piece shortcut keys are dropped entirely in favour of I-028's focus model, so there is nothing to disable. Test 10 rewritten (`stopImmediatePropagation`, `preventDefault` on Tab only); G19 types `wasd12` into `#bench-input`. (Seedling has no typing in I-028, so the typing check is Apprentice only.)
5. **Bench pointer in the Practised toast and build header, asserted.** Accept. Both carry "Show what you know at the Formula Bench (B) to cast it instantly."; G8 asserts it.
6. **Settle the Sprout input count; `estS` is a tripwire.** Accept option (a). Water at Sprout is exactly 4 inputs (asserted); `estS <= 10` stays as a labelled tripwire; the `estS <= 4` bound is gone. The 1-press compact mode is listed out of scope as a panel-driven follow-up for I-028.
7. **`build-web.sh` and a packaging check.** Accept. The contract lane owns line 21; G17 covers it; node test 12 (every imported module is on the `cp` line) and golden G21 (`fetch` 200 in `pkg-out`) added.
8. **Mutual exclusion with the bench; test `B` during a worked cast.** Accept. The overlay's capture listener (registered before `startBench`) uses `stopImmediatePropagation`; the overlay also cancels itself if `__benchState.open` turns true (the `#benchbtn` path). G20 added.
9. **GS-30 [panel] question in the brief note; paired run is the decision point.** Accept. Added verbatim to the panel brief note.
10. **Refresh "I-028 has no spec", "Open decisions" and "If I-028 differs".** Accept. All three rewritten; the contract step now records a Refresh note with renames.

Other points:
- **Section 4, band via an I-028 `band <id>` command.** Accept the need, reject the location: Rust gets the band through I-029's own `work band <b>`, which already passes the `work ` filter, so I-028's control-free design is not touched and no I-028 contract line is needed.
- **Drop `adept` from G14.** Accept. G14 runs sprout, seedling and apprentice; `work band adept` is stored and judged as Apprentice by I-028.
- **Pin ANSWERS to I-028 fixtures.** Accept, in a stronger form: golden builds from `work.view.recipe` through the UI, and Apprentice text is read from `content/spells.json`, so a data drift fails the test.
- **Call learnability the I-028 way.** Accept. `bench::learnable(def, band)`; `tier_min` is never read by this item.
- **Seam breaker "one click casts once" after rebasing on I-025.** Accept. Stated in the Overlap table and the breaker list.
- **Field overlay never offers the trial.** Accept. Stated in Smallest version 1 and Out of scope.
- **GS-22 measured sizes; GS-23 in the walk.** Accept. G14 measures pieces inside the panel at 1366 × 768 (64 px Sprout, 44 px otherwise) via `#worked`-scoped CSS, with no `bench.js` edit; GS-23 line added.
- **Planner-originated change (not in the critique):** a spell above the band (Fire at Sprout) now routes `instant` with no XP instead of `guided`, because I-028's `judge` returns `NotForBand` for every text there, so a guided build could never succeed. Recorded as Open decision 4.
