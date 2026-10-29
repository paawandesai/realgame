# I-004 spec critique

**Verdict: revise.** This is the right thing, but the spec needs trimming and has a few holes that let a builder pass the tests while missing the point. I did not run anything. I read only target.md, bars.md, areas.md and the spec.

## Right thing for the buyer?
Yes. A chemistry teacher is the buyer and one of the five panel personas. One wrong temperature or an unbalanced equation breaks the "spells are real reactions" claim. B6 names a register and a lint that do not exist, and it has no review procedure. The item stays first-person 3D and changes no game code, so it does not drift from the hard rule.

It does not move B4, B5 or B8, which is fine for a measurement item. It is cheap relative to its effect on B6.

## Smallest version?
Not quite. Cut or demote:

1. **Content rows for `content/reactions.json` and `known_reactions.json`.** This covers the Content table, test 6, `SCIENCE_CONTENT_DIR` and Appendix C.
   - B6 covers "every formula, temperature and outcome shown in game". The spec itself says the 3D client does not load these files today.
   - The 25 equations belong to I-008 (content). Extend the lint when the 3D game loads them.
   - If the item keeps them, make them a non-blocking warning in this item, not a CI gate.
2. **The `<script>` and `<style>` handling, and the JS and HTML extractors in test 8.** Keep only what the 32 real strings need. The rest is speculative.
3. **Fixtures in test 8 for shapes that do not occur in the slice.** Raw strings and `r#"..."#` are examples. Keep them only if the slice uses them.

The core is: register, the client-string lint, the `_C` pin and the protocol. That is enough to move B6 from unmeasured to measured.

## Can the acceptance tests fail?
Mostly yes. Tests 3, 4, 5, 7 and 8 are real mutation tests. Problems:

- **Test 2 contradicts itself.** It says `FPS` is "6d5463e or later" but demands exactly 33 strings. Pin test 2 to `6d5463e` exactly. Later commits get a different expected set, set by I-001's integrate step.
- **S0 loophole.** Any claim can cite `[S0]` ("design choice, no physical claim") and pass the lint. Add a lint rule that S0 is allowed only in `## Parameters` rows whose note is `gameplay tuning, not a physical claim`. Add a mutation test for it: a Claims row citing `[S0]` must fail.
- **No test for a new `_C` constant with no row.** The Contract requires a row for every `_C` constant. Test 5 only covers a changed value and a removed row. Add a test that adds `pub const X_C: f32 = 1.0;` and expects a failure.
- **Test 4 misses `error` rows with an empty note.** The Contract requires a note unless the verdict is `correct`, but the test lists only `simplified`. Add it.
- **The skip rule can hide a claim.** "Backslash-escaped quote means skip" lets a real caption containing `\"fuel\"` escape the lint. Skip only strings that also contain `{:` or start with `{{`. Add a fixture.
- **Test 9 and test 10 are judgement checks.** They are acceptable, but test 9 tells the builder to match the planner's verdicts. That invites rubber-stamping. State that the builder must derive each verdict from the cited source. State that the critic's 8-row spot-check must include at least 2 `simplified` rows and the two sourced-range rows (`ignition_c(DryHedge)` and `FLAME_C`).
- **Sources can be hallucinated.** The lint cannot check that a source is real. The protocol must make the reviewer open or verify each source, and the spec must say so. The suggested sources are marked "builder verifies", but nothing fails if the builder does not.

## Buildable from the contract alone?
Nearly. Gaps a lane would have to guess at:

- **Output format of the verbose inventory (test 2).** It must be one normalized string per line, with a fixed prefix. Otherwise "equal to Appendix A" cannot be compared mechanically.
- **Stale-row rule.** It is not defined for these cases:
  - multi-file `where` cells (stale if any listed file exists and lacks the text, or only if all do?);
  - `P-` rows, which describe literals;
  - `R-` rows, which are synthesized.

  State that the comparison is on normalized text, and that stale checks apply to `C-` rows only.
- **Parameter value syntax.** Say whether U+2212 or ASCII `-` is used in the `value` cell, and that it is parsed as a number. Say also that `-196.0` equals `-196`.
- **Rust string extraction.** Do not strip `//` comments by regex. Say that the extractor must tokenize, because strings can contain `//`. Otherwise this is a likely bug.
- **B6 scoring is unspecified.** The spec says the protocol defines it, but the pass rule is the whole point.
  - State in the spec that `simplified` passes B6 and `error` fails it.
  - Define `error` as "a statement shown to the player that is false" and `simplified` as "true but idealized or incomplete".
  - Add this to the contract so the protocol does not invent it.
- **`checks.md` triggers.** Test 10 depends on `docs/loop/checks.md`, which I was not given. Confirm it exists and lists the level-2 science-review triggers before building.

## Content judgement calls to flag (not blocking)
- `CO2 blanket, no O2` (C-004) is literally false ("no O2"). It is rated `simplified`. A teacher will see it as an error. Say the reviewer may escalate it, and open a backlog item to reword it to "less O2".
- `CO2 foam` (C-012, C-018, C-029) rests on a product that is obsolete. If the fresh reviewer calls it an `error`, B6 fails the item as designed. That is acceptable, but warn the orchestrator that B6 is likely to start at 1 or more `error`, not 0.
- The lint couples every future caption edit to a register edit. That is the intent. The orchestrator must tell I-001 before merge, as the spec already notes. Prefer merging I-004 after I-001.

## Required changes (summary)
1. Pin test 2 to `6d5463e`.
2. Add the S0 restriction and its test.
3. Add tests for a new `_C` constant with no row and for an `error` row with an empty note.
4. Narrow the escaped-quote skip rule.
5. Define the verbose inventory format and the stale-row rules.
6. Define the `simplified` and `error` meanings and the B6 pass rule in the Contract.
7. Drop the content-JSON rows (or make them warnings), and drop the extractor fixtures for shapes the slice does not use.
8. Require the reviewer to verify each cited source, and add test 9's "at least 2 `simplified` rows" to the critic's spot-check.
