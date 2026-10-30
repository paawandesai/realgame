# I-020 spec critique (roadmap v2, revision 3 review)

**Verdict: revise.** Not wrong-thing. A first-person WASD-and-look onboarding is what the hard rule asks for, it is the only open item aimed at B4 (0 of 5 personas pass the move half), and the evidence is specific (3 of 5 never pressed a movement key; the 6yo's refused-lock click was dropped). It adds no 2D, top-down or shooter behaviour. But under the new core loop it has one real gap (nobody owns the `cast` step once casts become worked casts), it is still bigger than the smallest version, one test cannot be built as written, and its B4 predictions are only valid before I-029. Make the changes in section 7, then approve.

Reviewed: spec.md; target.md, bars.md, areas.md, gameplay-standards.md at origin/main (c4832d2); the phase-2 specs I-026, I-027, I-028, I-029, I-030; I-022, I-024, I-025, I-021 specs (read for overlap); `hud.js` header and file list at origin/main.

## 1. Right thing for kids 4-18 and teachers under learn / master / instant cast?

Yes for the first 30 seconds, with three caveats.
- **Keep:** the world hold (a 4-year-old cannot lose while learning, GS-23/25), the silent hold, the click-at-cursor fix on a refused lock (GS-11, the 6yo blocker), and "spread is always said". Teachers get a replay button, which is what a classroom needs.
- **Caveat A: the coach teaches shooting, and stops there.** The core loop is learn at the bench, worked casts, mastery, instant cast; "aim and click" is one use of it. After I-028/I-029 the first click on Water opens the builder overlay (Water is Unknown on a fresh save). The `cast` coach line ("Aim at a fire and click to put it out.") then promises something the game no longer does, the builder opens, and `coachFor` returns `null` because I-029 sets `paused` while it is open (I-029 spec line 120). So the player sees the coach vanish mid-step with nothing explaining the overlay. **No spec owns this.** The I-020 spec says "Those items adapt the `cast` step", but a grep of I-026..I-030 finds no mention of `coach`, `onboard` or I-020. I-028 lists "a guided first-bench walk-through (GS-23)" as out of scope, so the first Formula Bench visit is untaught as well. This does not make I-020 wrong; it must not ship as if the loop were finished. Required: see changes 1 and 2.
- **Caveat B: B4's "lands a cast within 30 s" changes meaning after I-029.** The expected numbers (6yo cast within 20 s, today 21.9 s) were measured on instant, un-built casts. A worked cast costs 3-10 s of builder time on top. The spec must say its predictions hold for the merge base (pre-I-027..I-029) only, and that the bar's cast clock for later panels is I-029's problem. Otherwise a later panel regression gets blamed on this item.
- **Caveat C: "Hold W" for ages 4-7.** GS-11 allows at most one movement key, so W is fine, but GS-9 says no held chords and the beat only needs 0.4 m. Say "Press W to walk." (tap works, 0.4 m is about one short press) so a 4-year-old with one finger is not asked to hold.

## 2. Smallest version? Not yet. It is two items and a half

(a) coach + world hold + replay, (b) refused-lock click fix, (c) spread line plus a `lines.rs` refactor of `react()` with a planner-written snapshot file and a snapshot test. Only (a) and (b) move "moves within 10 s" and "no 'clicked and nothing happened'". (c) moves a caption gap that is also inside the hold's blast radius (the hold means spread cannot happen before the first landed cast, so the unexplained spread only matters after `Done`).
- Cut `event_lines`, `caption-strings.txt` and test 3 from this item. Keep only a one-line fix to the spread condition in `react()` (drop `caption_age > 1.5` for the uncaptioned spread `Ignited` case) with `spread_line_due` as a tiny pure fn and tests 2a-2c. Move `event_lines` + snapshot to I-021, which is the item that actually needs them (it already "builds on `lines.rs` and the snapshot"). That also removes a planner-written input from this item's dispatch and shrinks the `main.rs` hunk in the overlap table (`react()` 1316-1339 would be one line, not a function extraction).
- Test 13 is heavier than the change: a noise-floor protocol (two base batches, three interleaved pairs, 2x floor rule) for one resource, one per-frame distance check and a few KB of JS. Replace with: Brotli delta <= +25 KB (deterministic) and one `perf.mjs` run per artifact with `workload_ok` true and CPU p50 within +1 ms. Keep the full protocol for I-024, which is a perf item.
- Test 5 is test 4 restated (both pin the same table rows); fold it in.

## 3. Can each acceptance test fail?

Mostly yes, and the mutations are good (tests 2, 3, 7, 10, 11 name one). Problems:
- **Test 6 cannot be built as written.** It runs "`hud.js`'s coach setup against a fake document". `hud.js` is a classic top-level script (`const $ = document.getElementById`, `location`, `localStorage` at import; the header comment shows it is a script, no exports), and there is no `*.test.mjs` for it today. A web lane cannot import it under `node --test`. Fix: put the DOM builder in `coach.js` as `mountCoach(doc, canvas, send)` returning `{ update(state, now), replayButton }`, and have `hud.js` only call it. Then test 6 targets `mountCoach` with a fake `doc`; the `settings.hints` false assertion becomes "`update` is passed `hintsOn:false`; `#coach` icon stays". Also `canvas.focus()` on first state belongs inside `mountCoach` for the same reason. Without this the e2e (tests 8-10) is the only guard and it is red until all three lanes merge.
- **Test 9 step 1 is a race.** "`document.activeElement` is `#game` at the first state" passes if the page autofocused for another reason; add a mutation (remove the `canvas.focus()` call: the W press in step 2 must fail to move, and `#game` must not be active). The spec names the mutation for test 10 only.
- **Test 8's "no countdown text" regex** (`/spread|[0-9]+ ?s\b|hurry/i`) would also match a legitimate "2 s" in the pause card or a future I-029 builder timer. Scope it to `#objective`, `#firefinder` and `#obj-line` (the places I-019 owns) rather than "every visible text node outside four elements", or it breaks when I-028/I-029 add HUD text.
- **Test 12's "3/3 golden" plus 14's scope check** are guards, not proofs of the feature; fine, but golden's first landed cast already happens before any chem wait, so the real regression risk is the hold making `fire spreads` need `tick`. The spec argues this from reading golden; make test 12 state the expected pre-merge count of checks (98 at a0e2ecb) so a silent skip shows.
- Test 14 (scope) lists `golden.mjs` key-order lines only; I-027/I-029 also edit the same list. Fine with the merge-order rule, but the check should read "only the `I020_KEYS` addition", not "only those lines", or a later rebase on I-027's keys fails it.

## 4. Science sourced?

Adequate. One science statement ("Fire will not put out fire.") gets a `C-` row; the rest are instructions in **Not claims**, as `AIM_HINT` and `IDLE_HINT` do. Two notes: (1) the pick text is the only place the first minute states the chemistry, and it is true but thin (fire needs fuel, oxygen and heat; water removes heat). A sourced row for the full "water cools below ignition" claim already exists (I-019); cite it rather than a new row if one covers it. (2) The Hints-off behaviour hides this science line, because it goes through `setHint`. That is consistent with I-022 and acceptable, but then the `pick` icon alone cannot say "not fire". State this limit in the spec (GS-30 is only met while Hints is on).

## 5. Can the lanes build from the contract alone?

Core and contract: yes (state keys, commands and signatures are pinned). Web: no, for the reasons in 3 (no testable seam for `hud.js`). Other gaps:
- Line numbers are c4832d2's and the spec itself says I-022 and I-025 shift them; fine, but the lanes must not start until the planner's "After" step has run. That is a dispatch dependency on two unmerged items plus a planner-written `caption-strings.txt` (removed if change 3 is taken).
- `coachFor` reads `state.onboard`, `spell`, `paused`, `outcome`. `onboard` and `held` come from the core lane, so the web lane's fake states must match the contract table exactly; add one sample `__gameState` fragment to the Contract so both lanes copy the same shape.
- The `#coach` placement ("must not overlap `#firefinder`, `#objective`, `#hotbar` at two sizes") has no test. Add a bounding-box check to test 9, or drop the sentence.
- Replay: `onboard restart` calls `reset()`, which keeps spell progress (I-027 stores it separately) but, after I-029, may need to close an open builder. Name this: `reset` already closes the builder (I-029 G13), so it holds; add one line saying so.

## 6. Overlap with I-022, I-024, I-025 and the phase-2 specs

- **I-022, I-025:** well handled (merge order, wrap `pickHint` call in `coachText || pickHint(...)`, golden key order appended). One risk: the stale check says "`hintAllowed` is `settings.hints || !text || text === note`"; if I-022 changes that during review, the coach text silently becomes visible or hidden. Keep the stale check.
- **I-024:** no shared hunks. Its perf pairs driver is optional here. OK.
- **I-026:** keeps captions byte-identical; test 3 guards it. If test 3 moves to I-021 (change 3), I-026's byte-identical rule needs its own guard, and I-021 must land it before I-026. Check the order in the backlog.
- **I-027:** appends its own `__gameState` keys and golden key list; keys are appended in merge order, so I-020's `I020_KEYS` position depends on whether I-027 merges first. The spec already says "merge order"; state that the rebase order for golden's list is decided at dispatch and test 12 reads the list from origin/main, not a hard-coded sequence.
- **I-028:** its Formula Bench has a "B" key and an unguided first visit. I-020 does not teach it and says so (out of scope), but the backlog needs a row for the guided first-bench walk-through. Without it nobody teaches the core mechanic (learn).
- **I-029:** the real collision. I-029 holds the world with `paused` while the builder is open; I-020 holds it with `Onboard`. They compose (either holds), but: (a) `coachFor` goes `null` during the builder, with no coach line for it; (b) `onboard === 'cast'` persists while the builder is open, so a replay mid-builder reads oddly; (c) I-029 G15 asserts "existing behaviour: `select 2`, `cast`, `casts` +1 within 2 frames" which the I-020 hold does not break. Add a `Contract` line: "`coachFor` may be extended by I-029 with a `work` step; this item leaves a `STEPS` list and a `null`-while-paused rule that I-029 can override", and add a one-line item to I-029's Overlap table naming `coach.js`.
- **I-030:** no interaction.

## 7. Specific changes

1. **Own the cast step under the new loop.** Add to the spec: `coachFor` takes an optional `state.work` (I-029's builder state); when set, it returns `null` (no coach text over the builder). State that I-029 adds the builder coach copy, and add the matching line to I-029's spec/backlog row. Rewrite "Out of scope" so it says who adapts the `cast` text.
2. **Add the missing backlog item.** A guided first-bench walk-through (I-028 lists it out of scope). Name it in the Hand-off section so the core loop's first-minute teaching has an owner.
3. **Cut to the smallest version.** Move `event_lines`, `caption-strings.txt` and test 3 to I-021. Keep `spread_line_due` and a one-condition edit in `react()`; keep test 2's first three bullets and the e2e spread check (test 11).
4. **Make the web lane testable.** Move the coach DOM, first-state focus and replay button into `coach.js` as `mountCoach(doc, canvas, send)`; test 6 runs against it with a fake `doc`. `hud.js` shrinks to one import, one call and the `coachText || pickHint(...)` wrap.
5. **Soften the walk text** to "Press W to walk." (still at most 14 words, no word over 11 letters) and update tests 4, 5 and the reading-rule mutation text accordingly.
6. **Restate the Expected block.** Say the 10 s and 30 s predictions are for the merge base before I-027..I-029, and that later panels measure the worked-cast route separately.
7. **Tests.** Add the `canvas.focus()` mutation to test 9; scope test 8's regex to `#objective`, `#firefinder`, `#obj-line`; add a bounding-box assertion for overlap or delete the sentence; fold test 5 into 4; make test 14 "only the `I020_KEYS` addition"; replace test 13's noise-floor protocol with Brotli delta plus one paired `perf.mjs` run.
8. **Science row.** Cite I-019's existing cooling claim for the pick line if it covers it, and state that Hints off hides the only science statement in the first minute.
9. **Contract.** Add one sample `__gameState` fragment (`"onboard":"walk","held":true`) and state that golden's key list order follows merge order at dispatch.

After these, the item is the smallest version that moves B4's move half, is buildable from the contract, and does not strand the core loop.

## Planner response

Revision 3 of `spec.md` takes every change except the two partials below. Each line is change, verdict and reason.

1. **Own the cast step under the new loop: accepted.** `coachFor` now returns `null` while `state.work` is set, as well as while paused. The Out of scope section names I-029 as the owner of the builder coach copy. The matching line for I-029's Overlap table is a planner follow-up, listed in Hand-off, because I-029's spec is outside this item's diff.
2. **Missing backlog item: accepted.** Hand-off now names the guided first-bench walk-through (from I-028's out-of-scope list) as an unowned gap that needs a backlog row before I-028 dispatches. The local `backlog.md` predates the item rows, so the planner files that row at the next backlog refresh, not in this diff.
3. **Cut to the smallest version: accepted.** `event_lines`, `caption-strings.txt` and test 3 move to I-021. `lines.rs` keeps only `spread_line_due`, and `react()` changes by one condition. Test 2 keeps bullets 1-3, and its mutation now targets those bullets. Test 11 matches the spread template, which is now pinned in the spec, instead of a snapshot file. The planner's "write the snapshot" dispatch step is gone. I-021 needs a matching refresh, listed in Hand-off.
4. **Make the web lane testable: accepted.** `coach.js` exports `mountCoach(doc, canvas, send)`, which owns the coach DOM, the replay button and the first-state focus. Test 6 runs against it with a fake `doc`. `hud.js` keeps one import, one call and the `coachText || pickHint(...)` wrap. The refused-lock pointer code stays in `hud.js`, and test 10 guards it.
5. **Softer walk text: accepted.** The walk text is now `Press W to walk.` (GS-9: no held key). Tests 4, 7 and 9 are updated.
6. **Restate the Expected block: accepted.** The 10 s and 30 s predictions apply only to panels whose artifact predates I-027..I-029. I-029 owns the cast clock for the worked-cast route.
7. **Tests: accepted, except for one part.**
   - These are taken as written: the `canvas.focus()` mutation in test 9; the test 8 regex scoped to `#objective`, `#firefinder` and `#obj-line`; a bounding-box check in test 9; test 5 folded into test 4; test 14 reads "only the `I020_KEYS` addition"; test 12 states the pre-merge check count.
   - **Test 13 partial:** the noise-floor protocol is cut and the Brotli +25 KB gate stays. The suggested "CPU p50 within +1 ms" gate on one paired run is **rejected as a gate**. I-024 measured a base-against-base spread of 30.4 to 48.2 ms, so a 1 ms gate on one run would fail at random. CPU p50 and TTI are recorded but not gated, as I-029 does. `workload_ok` is gated.
8. **Science row: partly accepted.** The Hints-off limit is now stated: GS-30 is met only while Hints is on, and the icon alone cannot say "not fire". **Citing I-019's cooling row instead of adding a new row is rejected.** The lint matches the string itself, and `Fire will not put out fire.` is not the cooling claim, so it needs its own `C-` row. That row may reuse I-019's sources.
9. **Contract: accepted.** The spec gains a sample `__gameState` fragment. Golden's key-list order follows merge order at dispatch, and test 12 reads it from origin/main. Replay after I-029 is covered: `reset` closes the builder (I-029 G13), and opening pause cancels it.

Caveats A to C are covered by changes 1, 6 and 5. Section 6 is covered: the I-026 caption guard now comes from I-021's snapshot test, and I-021 already goes before I-026.
