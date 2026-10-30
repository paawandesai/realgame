# I-020 spec critique

**Verdict: revise.** The direction is right and on target. It is first-person, it moves B4 (the bar that most needs it), and the evidence is specific (3 of 5 personas never pressed a movement key, the 6yo's refused-lock click did nothing). It is not wrong-thing. But it is not the smallest version, several tests cannot fail automatically, and the lanes cannot start today. Make the changes below, then it is approvable.

Reviewed: target.md, bars.md, areas.md, I-020/spec.md. `docs/loop/gameplay-standards.md` does not exist on this branch (it exists only under `.claude/worktrees/*`), so the GS table was checked against the spec's own descriptions, not the source text. The bevy-client sources are not in this checkout, so feasibility was judged from the spec's cited facts only.

## 1. Right thing for players? Yes, with two doubts
- Walk coach, world hold, and click-at-cursor on a refused lock are exactly the 6yo/9yo/teacher blockers. Keep them.
- **Doubt A: the panel is told the band.** `pageUrl` injects `?band=` per persona, so B4 is measured on texts that real players only get after the I-005 picker, which never shows when analytics is off or DNT is set. Real Chromebook players default to `seedling`. State plainly in the spec that panel B4 is therefore "if the band is known". Better: run the 6yo once with no band (default seedling) as a guard, so the default path is measured too.
- **Doubt B: the hold has no escape.** A player who never walks keeps the world frozen forever, which is fine, but nothing tells a player who is in a rush that the hold exists. Say in the spec that this is deliberate, and that the fire counter must not show a countdown or "spreading" cues while held (I-019 owns the counter; add this to the stale check).

## 2. Smallest version? No: it is three items in one
It bundles (a) coach plus world hold, (b) a click fix, and (c) a 3-level caption and label system (band.js, captions.js, 29 keys, SLOTS, warningFor, lostFor, panel URL band plumbing, I-005 hook, about 40 science rows). Only (a) and (b) address "does not move within 10 s". (c) addresses the clarity and text-blocker expectations, and it is the bulk of the risk (science register rows, B3 size, panel harness changes).
- **Required change:** either split (c) into I-021 (band + captions) and keep I-020 as (a)+(b)+the spread-caption rule, or cut (c) to its minimum: `sprout` and `seedling` for only the 11 pinned keys plus `SLOTS`, with `warningFor`, `lostFor` and the other 18 keys falling back to the apprentice text. The pinned table already covers the keys the panel flagged; the rest is speculative.
- The spread fix (`spread_line_due`) is a real defect fix and independent of bands. Keep it in I-020 whichever way the split goes.
- If the split is chosen, the expected-movement claim "clarity median 6 to 7" moves to I-021; I-020 then claims only the move and cast timings.

## 3. Can every acceptance test fail? Not yet
- **Test 3 ("the breaker checks this")** is not a test. Replace with a Rust or node test that greps the diff-free invariant: the existing caption strings listed in a snapshot file are all still present byte-identical in `chem.rs` and `main.rs`.
- **Test 16, I-005 hook** offers "or the breaker checks by hand". Remove the alternative. Require the node test with a fake DOM, so a missing `__band?.set` line fails it.
- **Test 10** branches on whether the lock is granted ("if refused ... if granted ..."). A test with two paths can pass without exercising the one that matters. Force one path: stub `requestPointerLock` to reject in one run (that is test 11) and in this run stub it to resolve and assert the click does not cast, then cast via the steering path. Also "no click before the first key" conflicts with the sweep-then-click step; say that the key-W check happens first, explicitly.
- **Test 9 hold** passes trivially if `held` is reported true but `chem_tick` still runs. It does check `tick` and `temp` unchanged for 5 s, so it can fail; good. Add the converse: pressing R before any walk (`cmd reset`) leaves `held` true, since the spec says reset does not touch `Onboard`.
- **Test 12 mutation (spread)** only fails in the contract lane after merge. Add a Rust unit test of `spread_line_due(true,false,0.0,false)` (already in test 2) **and** extract the `react()` decision so one test drives a spread `Ignited` event with `caption_age = 0.0` and asserts a line is produced. Otherwise a core builder can wire `react()` wrongly and the unit tests stay green.
- **Test 15 thresholds** (+1.0 ms p50, +25 KB, +0.5 s TTI over 3 runs) are inside run-to-run noise on a throttled cloud box. State the noise floor measured at based_on (run base 3 times first) and set the thresholds from that, or the test is flaky in both directions. Also note that the hold reduces chem work during perf's first seconds, which can mask a regression: run perf past `Done`.
- **Test 14 (B1)** says `golden.mjs` is unchanged and still passes. Golden's spread and lose checks depend on `tick N`, which bypasses the hold. But if golden ever waits real time for a spread or a lose, it will hang while held. Add one line stating which golden checks were read for this, or the claim is unverified.
- **Test 8 reading rules** are good and have a mutation. Add a second mutation for seedling (a 12-letter word or a formula) so both bands are shown to fail.

## 4. Are the lanes buildable from the contract alone? Mostly, once these are fixed
- **Not buildable today:** the spec depends on I-005, I-007 and I-019 all being merged, and its facts for those come from branches and a draft spec. The stale check is sound, but the item should be marked blocked and not sent to lanes until `based_on` is moved. Say so in the header.
- **Core lane:** buildable. Missing: the exact source of each key for `format!` strings in `reset()` and `react()` (which key each existing string maps to). A list "string -> key" in the spec, or a snapshot file the core lane writes, prevents two builders guessing differently. `greenhouse_caught` and `station` are in the key list but never described.
- **Web lane:** the 18 unpinned keys are left to the builder, which is fine under the reading rules, but `lostFor`/`warningFor` have no pinned text and no stated current strings, so "apprentice returns the exact current strings" cannot be checked from the contract. Pin them or cut them (see section 2).
- **Contract lane:** the panel `band` plumbing is clear. `docs/loop/panel.md` is not in areas.md; either add it to the tests-e2e area or drop the paragraph.
- **Shared files:** `hud.js` and `main.rs` conflicts with I-007/I-019 are acknowledged; the stop-and-report rule is good. `age.js` belongs to I-005's area and is touched by one line: name I-005's lane as the reviewer for that line.
- `index.html` unchanged while `SLOTS` must restore it "byte for byte": the test must read `index.html` at test time, not a copy of the strings, or the two can drift.

## 5. Requested changes, in order
1. Split or shrink band/captions (section 2). Keep coach, hold, click fix, spread rule.
2. Replace the by-hand checks in tests 3 and 16 with automated tests.
3. Make test 10 single-path.
4. Add the `react()`-level spread unit test and the reset-while-held assertion.
5. Set test 15 thresholds from a measured noise floor, and run perf past `Done`.
6. Add the string-to-key mapping and pin `warningFor`/`lostFor` or drop them.
7. Mark the item blocked on I-005/I-007/I-019 and fix the missing `gameplay-standards.md` reference, or point to its real path.
8. Add a default-band (no `?band`) 6yo panel run to measure the real-player path.

## Planner response

Test numbers below are the critique's (revision 1). In revision 2 the tests are renumbered after the split.

Facts checked for this response at origin/main 3dbd332: `docs/loop/gameplay-standards.md` exists there (added by I-011, 9c4b7e3), and it is missing only from this loop branch. `golden.mjs` lines 191-199 drive spread and lose with `tick 2400` / `tick 9000`, and `w.outcome()` is read from the chem world. `perf.mjs` `pageWorkload` starts each cycle with `tp 0 5.5` (12.5 m from spawn), followed by a cast at a named object.

- **Doubt A (panel told the band):** accepted, moved to I-021. After the split, I-020 does not touch the panel URL, so every persona runs the default path. I-021 inherits the "if the band is known" wording and the no-band 6yo guard run.
- **Doubt B (hold has no cue):** accepted. The spec now says the silent hold is deliberate. It adds to the stale check that I-019's counter shows no countdown or spreading cue while `held`, and the new test 8 asserts this.
- **1. Split or shrink band/captions:** accepted, as a split. §3, §4, `captionKey`/`captionArg`, the panel band plumbing and the `age.js` hook go to I-021. I-020 keeps the coach, the hold, the click fix and the spread rule. The clarity and text-blocker claims move to I-021. The coach has one plain text per step (the old seedling column), checked against the seedling reading rule.
- **2a. Test 3 by hand:** accepted. A Rust test now `include_str!`s a snapshot (`caption-strings.txt`, which the planner writes from `based_on` at dispatch) and asserts that every line is still in `chem.rs`/`main.rs` byte for byte.
- **2b. Test 16 by hand:** accepted, moved to I-021. The `age.js` line leaves I-020. I-021 must use the fake-DOM node test with no by-hand alternative.
- **3. Test 10 single-path:** accepted. The lock must be granted (as golden already relies on), or the test fails. The click must not cast, and the cast is made by steering. Test 11 covers the refused path. The W check is now explicitly done before any click.
- **4a. `react()`-level spread test:** accepted. The decision moves into a Bevy-free `lines::event_lines`, which `react()` only applies. A unit test drives a real spread `Ignited` with `caption_age = 0.0`.
- **4b. Reset while held:** accepted, added to test 9.
- **5. Perf noise floor and running past Done:** accepted. The base is run twice first, and each threshold is max(budget, 2 x the measured spread). Running past Done needs no `perf.mjs` change, because its first workload step teleports 12.5 m and casts at an object, so the hold ends in warm-up. The contract lane confirms `workload_ok`, and the stale check guards this.
- **6. String-to-key map, `warningFor`/`lostFor`:** accepted, moved to I-021, which must pin both or drop them. I-020 emits no keys. Its only string guard is the snapshot.
- **7. Blocked header and gameplay-standards path:** accepted. Status is now **blocked on I-007 and I-019** (no longer I-005, after the split). GS refs point to `docs/loop/gameplay-standards.md` at origin/main 3dbd332.
- **8. No-band 6yo panel run:** accepted, moved to I-021. In I-020 every persona already runs with no band.
- **Test 8 seedling mutation:** accepted. The coach-text rule test has a seedling mutation (a 12-letter word, and a formula). The sprout mutations go to I-021.
- **Test 14 golden reading:** accepted. The spec now names the golden checks that were read (spread and lose use `tick N`; the win comes after landed casts).
- **Lanes: `panel.md` and areas, `age.js` reviewer, `index.html` read at test time:** accepted, moved to I-021, because I-020 no longer touches those files. They are listed in the I-021 hand-off.

**Verdict after revision: approve.** Every change is accepted. The band and caption changes are carried into the I-021 hand-off in spec.md.
