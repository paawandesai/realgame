# Critique: I-027 Spell progress (spec revision 1)

**Verdict: revise.** The item is the right thing and about the right size. Two defects would stop it shipping as written (M1, M2). The rest are small.

Read: `spec.md` (rev 1); `target.md`, `bars.md`, `areas.md`, `gameplay-standards.md` from origin/main; code at c4832d2; the specs for I-026, I-028, I-029 and I-030. The old critiques of I-022, I-024 and I-025 were not re-read; the spec's overlap table was checked against the code instead.

## 1. Is it the right thing under the new core loop?

Yes. `target.md` makes learn, master and instant cast the loop, and B9 needs mastery that survives a reload and a break. Nothing on c4832d2 stores per-spell state. One owner of the rules (stage, XP, the 2-day rule, never-fades) prevents I-028, I-029 and I-030 from inventing three versions. It keeps crafting at the centre. It adds no shooting, no camera change and no new input, so the first-person hard rule is not touched.

It is invisible to kids and teachers, so no panel can score it. Today it moves no bar, and the spec says so ("enabling"). The spec bounds the risk: it adds no UI, no text and no gameplay gate.

Two points the spec handles well and should keep. The progress save is keyed by a stable id string, not by index. A save with a bad field is rejected whole, and a corrupt save is not deleted.

Two product concerns need a named owner, not only an "Out of scope" line.

- **Shared classroom devices.** Progress is in one browser profile, so child B inherits child A's Mastered spells. The spec defers the "Start over on this computer" button, and I-028 repeats that the planner must file it. It is still not an item.
  - Change 8: the leader files it before I-028 is built, and I-028 cannot ship to teachers without it. Until then the debug-only `prog clear` is not reachable by a teacher.
- **Midnight loophole in "2 days".** The spec counts distinct local calendar days. Three builds at 11:55 pm and 12:05 am satisfy "spread over 2 days", and so does editing the system clock.
  - The target text says "2 days", so this follows the target. It still weakens the learning claim (the spacing effect).
  - Change 5: state the limit in the spec and in the code comment. Do not add timestamps; they would cost privacy and cannot be validated.
  - I-030 may add a "not within the same session" rule. This item must not block that.

## 2. Smallest version

- **Core is minimal.** A pure model, a save, three state keys and one command. No UI.
- **The held analytics lane inflates this item.**
  - It adds tests 26 to 29, a migration, edits to `functions/`, `analytics.js` and the privacy docs, and it waits on a human.
  - The spec's own text says it "ships without it", but the item's lanes, test list and "done" still carry it.
  - Change 6: move the analytics proposal into its own item (I-0xx, held on ACTION-1) and leave one line here. The main item then closes on tests 1 to 25. Keep the proposal text; it is good.
- XP constants and `instant_cast` are needed by I-029. Keep them.

## 3. Can each acceptance test fail?

Mostly yes, with these exceptions.

- **Test 5 cannot be built as written.** It asks for "a Learning spell with xp 20, builds 3, build_days 2 and predicted (built through the API)". That state is unreachable through the API.
  - Learning needs `xp < 30`, and `learn` gives 10.
  - Any `unaided_build` adds 20 and moves the spell to Practised at once.
  - `predict` adds 20, which does the same.
  - A builder will either fail the test or loosen it.
  - Change 3: build that state with `apply_cmd(["load","p1|water:1:20:3:2:20756:1"])` (decode keeps it as saved), then `worked_cast`. State plainly that the two-transition case cannot arise in play, only from a save.
- **Some Rust tests pass on the stub, though the spec says tests 1 to 11 "fail on the stub".**
  - Tests 10 and 11 pass because `spell_id`, `index_of` and `state_json` are real in the stub.
  - Test 6 (never lowers) passes because the stub does nothing.
  - Change 4: say which tests are red on the stub (1 to 5, 7 to 9). Make test 6 also assert that the random run reaches Mastered at least once per seed set, so a no-op cannot pass it.
- **Test 20 depends on Bevy's `warn!` reaching `console.warn` in the wasm build.** Say how the check recognises it (a message containing the command name), or the check may always pass.
- **No test pins `rev` after a worked cast, an instant cast or a wrong predict.** Add the cases `worked_cast(i,false)` and `predict(i,false)`. Neither may bump `rev`, or the page would write the save for nothing.
- **Golden test 24 and the lane-hygiene test 25 are good and falsifiable.** Add a check for M1 as test 25b.

## 4. Science

No chemistry is shown, so B6 does not apply.

The learning-science claims are unsourced: spaced practice, "predict" questions, and "mastery never fades". `gameplay-standards.md` sources every other rule. Change 5 asks for two citations in the spec (or in `content/science-sources.md`, if the leader prefers one place).

- **Spacing:** Cepeda et al. 2006, Psychological Bulletin 132(3), distributed practice review.
- **Retrieval and prediction:** Roediger & Karpicke 2006, Psychological Science 17(3), the testing effect.

The spec should also say that "Mastered never fades" is a game rule, not a memory claim. The keep-it-sharp bonus in I-030 is the memory check.

## 5. Can each lane be built from the contract alone?

Rust and web: yes, apart from M1 and M2. The stub and signatures are exact. The seam is named: golden tests 17 to 23 need the rust lane.

The analytics lane is clear but is blocked on a human (see 2).

### Must-fix defects

- **M1: the new JS file is not shipped, and the boot can fail.**
  - `build-web.sh:21` copies an explicit list: `index.html style.css boot.js hud.js favicon.svg`.
  - The spec adds `web/progress.js`, imports it statically at the top of `boot.js`, and lists neither `build-web.sh` nor `scripts/build-site.sh` in any lane.
  - In the built output (`pkg-out`, which golden serves, and `dist`) `./progress.js` would be a 404. A static import that 404s fails the whole `boot.js` module. `init()` never runs, and the game does not start. That breaks B1, B3 and B7.
  - The `try {}` around the call does not help, because the failure happens at import time.
  - I-028 already found this about this spec ("`build-web.sh:21` ... I-027 adds `web/progress.js` and its spec does not list `build-web.sh`").
  - Change 1: add `crates/bevy-client/build-web.sh` (one word added to the `cp` line) to the web lane and to the contract stub. Check that `build-site.sh` ships `progress.js` into `dist`.
  - Change 1b: load it with a dynamic import, `import('./progress.js').then(m => m.startProgress()).catch(() => {})`. This is the pattern `age.js` already uses. A missing file then costs only saving.
  - Add a golden or script check that `pkg-out/progress.js` exists and that the page boots with 0 console errors when it is missing.
- **M2: test 5 is unbuildable** (section 3, change 3).

### Smaller defects

- **The save charset is inconsistent.**
  - The Rust save charset includes `;`, but `SAVE_RE` is `[a-z0-9:.]` plus `|`. Pick one. Drop `;`, since the format never uses it.
  - Test 8 includes a `-1` field. Fine in Rust, but note that `SAVE_RE` rejects it on the JS side as well.
- **I-030 extends this item's format.** It adds `sharp_day` and `asked`, and a 9-field entry. It also relies on `decode` rejecting 8 and 10 or more fields.
  - State here: "`decode` field count is exactly 7 in this item; I-030 changes it to 7 or 9."
  - The stage-above-fields rule (Mastered with `predicted = 0` kept) must survive I-030's rewrite. Add a test that I-030 inherits.
- **The overlap table is out of date.** It says `boot.js` is touched by none. I-028 and I-030 also edit `boot.js` (I-028 explicitly, and `build-web.sh`). Those are one-line list edits, but they conflict textually. Name them.

## 6. Overlap with the other specs

- **I-022, I-024, I-025:** the table is accurate. The only shared hunk is the `publish` format tail with I-025, and the merge order is stated. No change needed.
- **I-026:** the id contract matches. I-026 pins `spells[].id` as `fire, water, frost, co2` and says it is "saved by I-027". Both specs agree that `spell_id` is the single swap point. Keep the test that an old save still loads.
- **I-028, I-029, I-030:** they call only `Book` methods, as the contract requires. All three assume `learn`, `worked_cast`, `unaided_build` and `predict` exactly as written here, so test 5's rewrite does not affect them.
- **GS items:** GS-14 holds (try/catch plus tests). GS-28 holds, with no copy added here. The critic note for I-030 ("never says come back tomorrow as a lure") is carried forward. GS-21 and GS-27 are not touched.

## Specific changes (ordered)

1. Add `build-web.sh` (and check `build-site.sh`) to the web lane and the stub, with a test that `progress.js` is shipped.
   - 1b: import `progress.js` dynamically in `boot.js`, with `.catch(() => {})`, and add a test that a missing file still boots.
2. Rewrite test 5 to build its state through `load`.
3. State which tests are red on the stub. Make test 6 reach Mastered at least once per seed set.
4. Add no-`rev`-bump cases for wrong casts and wrong predicts. Say how test 20 recognises the warning.
5. Add the two citations and the note that the 2-calendar-day rule is a proxy for spacing. Say what the midnight case does.
6. Move the analytics proposal to its own item and leave one line here.
7. Fix the charset mismatch (`;`). State the 7-field rule and that I-030 widens it.
8. Make "Start over on this computer" an item, required before I-028 reaches any teacher, and update the overlap table for `boot.js` and `build-web.sh`.

With 1, 2 and 8 done, approve. The rest are quick edits that need no second critic pass.
