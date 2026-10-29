# Critique of I-008 spec (spec critic, fresh eyes)

**Verdict: revise**

The item is the right thing. B8 cannot pass without 3 quests, 6 spells, a free-play area and a skill-tree screen, and it keeps the first-person rule. The science is sound on the points a teacher would check (Na2SO4 instead of NaCl, 2:1 volumes, cathode and anode, LN2 at -196 C). The tick arithmetic checks out: 2.0 mL/s at 30 Hz gives 10 mL at tick 150, the spring crosses 40 C at tick 416, and 4 per second of extra ice loss gives 8 ticks. Most acceptance tests can fail. The spec has one real hole (pond bypass), one scope problem, and several lane and test gaps that would stall a lane or let a bad build pass.

## Buyer fit
- Q2 (electrolysis) and Q3 (freeze the pond, chill the spring first) are real-science puzzles that fit "spells are real reactions". Keep both.
- Q3 puts the whole puzzle on a failure hint. A 6-year-old sees "Cross the pond to the south shore. Hot spring: 80 C" and nothing about the spring. Make the step 1 line name the clue, for example "Freeze the pond. The hot spring melts ice nearby: cool it first." The caption table must carry it verbatim, so test 21 checks it. This doubles as a B4 guard.
- Spells 5 and 6 are never required by any quest. They are only toys in the yard and courtyard. This is acceptable for the smallest version, but say so. The reward for Q2 is then only a caption, a point and two hotbar slots. It is fine for B8, and it will not help B5 much.
- Hydrogen on a burning bush gives "Popped, no state change". A child would expect hydrogen to feed the fire. Add one caption clause saying a small pop is what a test tube gives, or make the effect match. This is a teacher-credibility risk and a B6 reviewer risk, not a blocker.

## Smallest version (too big; trim)
- The Discoveries panel (12 discoveries, `discovery_of`, `DISCOVERIES`, `#disc-*` DOM, tests 11, 13b and the 19 tail, the `discovered` bitfield and state) is not needed by B8 or any named bar. Cut it. It is the cheapest big cut and removes work from all three lanes. Move it to a later item.
- Keep the three upgrades. They are what makes the skill-tree screen more than a poster, and each is about 5 lines of rules.
- Remove C-I008-14 (ice density, "if shown"). The spec itself says the sign may not exist. Delete the claim, or require the sign. Do not leave a row test 21 forces to be sourced for text that may not exist.
- The spec is about 37 KB with 23 tests across 4 lanes. If the planner prefers, split into I-008a (Q2, spells 5 and 6, quest module, 6-slot hotbar) and I-008b (map, pond, yard, tree). This is optional. The trims above are required.

## Can the tests fail? Problems found
1. **Pond bypass is not closed (real hole).** The divider wall at x=0 and the south-zone walls have no height. If the divider is low, a sprint jump from the yard reaches the south shore at z >= 47.5. The completion test (x < -0.5) would stop the quest completing from the yard side, but a child could still cross over the divider to the shore and skip the puzzle. Specify that every south-zone wall, the divider and the gates' lintels (if any) are at least 3.0 m tall. Add to test 9 that their top is above the max jump height, using the `player` constants. Add an e2e step that a sprint jump at the divider does not cross it.
2. **Test 17 is weak.** "`pos[2]` never exceeds 43.0" is true for anything short of the far bank. A naive crosser stops at about z=35 minus the radius. Assert `pos[2] < 35.0` and that no `Splash` appeared.
3. **Test 15 omits `select` before casts.** Water at the splitter and switch needs `select 2`. Test 18 needs `select 1` and then `select 2`, and test 19 needs `select 4` for Foam. Write them out, or the scripts will cast Fire at everything and fail confusingly. Also state the hotbar order in the Contract (1 Fire, 2 Water, 3 Frost, 4 Foam, 5 Hydrogen, 6 Oxygen).
4. **Test 10 is under-specified.** "Feeding the events and world states of the Q2 sequence" does not say what the world must look like at each step. Give a table of (event list, world fields, player xz) to expected `step`. Otherwise a lane can only guess, and the test can pass vacuously.
5. **Test 5 is float-fragile.** "Tick 1199 has ice > 0 and tick 1201 has ice == 0" depends on how ice is decremented. Say that ice decrements by exactly `DT/POND_ICE_S` per tick and is clamped at 0, and that `Melted` fires on the tick it crosses 0. State the result at tick 1200. The same applies to `h2_ml == 20.0 exactly` at tick 600: say the add is clamped with `min(cap)`.
6. **Perf test 22 is noisy.** A limit of +1.0 ms median over 3 interleaved runs on a cloud box will be flaky in both directions. Use 5 runs each, compare medians of medians, and set the limit relative to the base's own run-to-run spread (for example max(1.0 ms, 2x base spread)). Say what happens when `CPUFRAME` is not present on main.
7. **B1 goes red.** B1 requires the smoke playthrough to "complete every quest in the slice", and I-002's smoke script is out of scope here. After this merge B1 cannot pass. Say so explicitly in the spec, and name the item that restores it (the planner should schedule it next). The e2e sequences in tests 15 and 16 are good raw material.
8. **Zone-by-position is undefined at boundaries.** `zone` ("courtyard", "pond", "yard") is "by position", but the lab alcove, the gate thresholds and the divider have no cutoffs. Q3 s0 uses "z > 25 with x < 0". Define one function with exact thresholds and use it for both the state field and the quest line.

## Lanes buildable from the contract alone?
Mostly yes. The Contract has exact names, signatures, constants, captions and state fields, and the web lane is fully independent. Gaps:
- **World lane tests need `player` constants (test 9) but player.rs belongs to the core lane.** world and core build in parallel, so world's tests do not compile until core lands. Fix: the contract lane makes the constants `pub` (add `player.rs` to its globs for that one edit), or the test copies the values from the Contract with a comment.
- **`mod quest` in main.rs.** The contract lane makes the one-line edit but does not own main.rs. Say that the contract lane may touch that one line, with core rebasing, or move the line to a shared owner.
- **Test 1 gives no rule for "second cast" on the switch.** It does: `Powered` again, `power` stays true. Fine. But the splitter has no behaviour for Water when `level == 1` already. State it (re-emit `Filled`, or `Fizzle`).
- **The `Target.gauge` design lives in world.rs, applied in main.rs by core.** Two lanes share the interface with only a one-line description. Add the field's exact type and when it is `None`.
- **The 1<<30 event flag** is mentioned for `update` but not defined in the contract. Define it or point to the existing definition by file and line.
- The spec's "based_on" is an unmerged branch (I-001). The stale check is good, but the planner should re-verify the names before dispatch. Do not dispatch on the current text if I-001 landed with different names.

## Required changes (summary)
1. Cut the Discoveries panel and its tests; drop C-I008-14.
2. Specify wall and divider height >= 3.0 m, and add a layout test and an e2e jump-at-divider check.
3. Strengthen test 17 (`pos[2] < 35`); add the missing `select` steps to tests 15, 18 and 19; give test 10 a concrete table; pin the ice and gas arithmetic in tests 1 and 5.
4. Put the spring clue in the Q3 s1 step line (and caption table).
5. Make the perf test statistically sane.
6. Resolve the player-constants ownership and the `quest` module line; define zone-by-position and the event flag.
7. State that B1 is red after this item and who fixes it.
