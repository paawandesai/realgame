# I-004: Science sources file, source lint and science-review protocol

**Revision:** 2 (applies `critique.md`; see "Critique disposition" at the bottom).
**Moves:** B6. Expected change: B6 goes from *unmeasured* to *measured*. The "100% of claims sourced" half becomes a CI gate: `npm test` fails on any unsourced client claim. The "0 errors" half becomes a repeatable fresh-model review with a fixed output format and a fixed pass rule (Contract § Verdicts and B6 pass rule). The planner's pre-assessment expects 100% sourced, 0 `error` and about 8 `simplified` verdicts for the I-001 slice, **but B6 will likely start at 1 or more `error`**: a fresh reviewer may reasonably escalate C-004 ("no O2") or the CO2-foam rows (C-012, C-018, C-029). That is the measurement working, not the item failing; each `error` becomes a backlog item. B6 itself still cannot pass until a fresh reviewer runs the protocol on the merged slice (scorer, nightly).
**based_on:** `f6504c43dac4f04fbafabdbd754f75f8d59441e7` (origin/main), 2026-10-29. Client strings were read from `origin/claude/fps-prototype` at `6d5463efb80a3eb2af568e76e311cafdfeb968c1` (I-001, not yet on main). `docs/loop/checks.md` was confirmed to exist on main with a level-2 science-review row and path triggers.
**Area:** content (`content/science-sources.md` only), analytics/test (`test/**`), loop docs. Track: measurement. After: none (prefer merging after I-001; see Risks).

## Problem

The pitch to teachers is that "spells *are* real reactions, with correct formulas, temperatures and outcomes". Today nothing checks that:

- The I-001 slice shows 32 player-facing science strings: hotbar formulas, cast captions, event captions and the win and lose lines. They live in `crates/bevy-client/src/chem.rs`, `src/main.rs`, `web/hud.js` and `web/index.html`. None of them cites a source.
- Its rules engine also hard-codes temperatures that drive outcomes: `AMBIENT_C = 20`, `LN2_C = -196`, `FLAME_C = 800`, and ignition at 230 °C and 300 °C. None of these is cited either.
- B6 names `content/science-sources.md` and a "source file lint", but neither exists. There is no written procedure for the science reviewer either, so two reviews of the same build can disagree, and no one can tell whether a claim was checked.

One wrong temperature or unbalanced equation in front of a chemistry teacher (one of our five panel personas, and the buyer) undoes the product's main claim.

`content/reactions.json` and `content/known_reactions.json` also carry unreviewed science, but the 3D client does not load them, so B6 ("shown in game") does not cover them yet. They are out of scope here (see Out of scope).

## Smallest version

Three files, one lane, and no change to game code.

1. **`content/science-sources.md`**: the single register of every in-game science claim. Its machine-readable format is fixed in the Contract below.
   - One row for each player-facing claim string in the 3D client (the 32 claims in Appendix A, as of `6d5463e`). One more flagged string, a status counter, goes in `## Not claims`.
   - One row for each temperature constant in `chem.rs` whose name ends in `_C`.
   - One row for each ignition temperature returned by `Obj::ignition_c`.
   - One row for each scenario value: steam vent 110 °C, ice wall −5 °C.
   - One row for the electrolysis prop's 2:1 gas-volume ratio (`world.rs`).
   - Each row has a verdict (`correct`, `simplified` or `error`) derived from its cited source, a note wherever the verdict is not `correct`, and one or more source ids. Each source id is defined in a `## Sources` list as a checkable reference that the builder actually verified.
2. **`test/science-sources.test.ts`**: a `node:test` file that the existing `npm test` glob (`test/**/*.test.ts`) already picks up, so no CI change is needed.
   - It extracts claim strings from the 3D client source with the fixed heuristic in the Contract, and fails when one of them has no row.
   - It fails when a row is malformed, cites an undefined source, or misuses `[S0]`.
   - It fails when a `_C` constant has no row or its value differs from its row.
   - It fails when a `C-` row points at a client file that exists but no longer contains the row's text (a stale row).
   - It must also pass `npm run typecheck`, because the root tsconfig includes `test/`.
3. **`docs/loop/science-review.md`**: the protocol the fresh-model science reviewer follows. It covers:
   - inputs, step-by-step checks, source verification, tolerance rules, and the verdict definitions and B6 pass rule copied from the Contract (the protocol may not redefine them);
   - the output file and its format;
   - when the review runs (it must match `docs/loop/checks.md` path triggers);
   - who may change verdicts, and how an `error` becomes a backlog item.

## Acceptance tests

Each test runs on the lane branch. "Scratch copy" means a temporary worktree or copied directory under the scratchpad, never a commit to `crates/bevy-client`. `FPS` means a checkout of `origin/claude/fps-prototype` at **exactly** `6d5463e` (for example `git worktree add <scratch>/fps 6d5463e`). Tests 3, 5 and 7 mutate a scratch copy of `FPS`.

1. **Green on main, CI unchanged.** On the lane branch:
   - `npm run typecheck && npm test` exits 0, and the spec reporter lists tests from `test/science-sources.test.ts`.
   - `git diff --name-only origin/main...HEAD` lists only `content/science-sources.md`, `test/science-sources.test.ts`, `docs/loop/science-review.md` and files under `docs/loop/items/I-004/`. Nothing under `.github/`, `crates/` or `package*.json` may appear. (lane: main)
2. **Green against the real slice, inventory exact.** `SCIENCE_SCAN_ROOT=<FPS> node --disable-warning=ExperimentalWarning --test test/science-sources.test.ts` exits 0. With `SCIENCE_SOURCES_VERBOSE=1` it prints the inventory in the format in Contract § Verbose inventory. The set of `claim:` lines must equal the 33 strings in Appendix A (C-001 to C-032 and N-1): same count, same normalized text, checked by a mechanical diff (`grep '^claim: ' | sort` against the appendix list). The extractor must not flag any string listed under "must not flag". This test is pinned to `6d5463e`; for later I-001 commits the expected set is whatever I-001's integrate step records in the register. (lane: main)
3. **Fails on an unsourced claim, naming it.** Each mutation must exit non-zero, and the failure message must contain both the exact offending string (normalized) and the repo-relative path `crates/bevy-client/src/chem.rs`:
   - Add `"Copper melts at 1085 °C."` as a string literal in a non-test function of chem.rs.
   - Change `"Hot water boils at 100 °C"` to `"Hot water boils at 90 °C"`.
   - Add the literal `"Dry \"fuel\" catches fire fast."` (with Rust-escaped quotes) in a non-test function of chem.rs. This checks that an escaped quote alone does not cause a skip.

   (lane: main)
4. **Fails on a broken register.** Make each change below separately on a scratch copy of `content/science-sources.md`. The test reads the register from `SCIENCE_SOURCES_FILE` if that variable is set, and from `content/science-sources.md` otherwise. Each change must make the test exit non-zero with a message that names the row id:
   - delete the row for `Hot water boils at 100 °C and turns into steam: H2O(l) → H2O(g).` (run with `SCIENCE_SCAN_ROOT=<FPS>`);
   - set a row's source cell to empty;
   - cite `[S99]`, which is undefined;
   - set a verdict to `maybe`;
   - give two rows the same id;
   - give a `simplified` row an empty note;
   - give an `error` row an empty note;
   - make a `## Claims` row cite `[S0]` (alone or alongside another source);
   - make a `## Parameters` row cite `[S0]` with a note that starts with neither `gameplay tuning` nor `scenario assumption`.

   (lane: main)
5. **Constants are pinned.** On a scratch copy of `FPS`, each change must exit non-zero and name the constant:
   - change `pub const LN2_C: f32 = -196.0;` to `-195.0`; the message reports `LN2_C`, the source value and the register value;
   - remove the `LN2_C` row from a scratch register (with `SCIENCE_SOURCES_FILE`);
   - add `pub const X_C: f32 = 1.0;` to chem.rs; the message reports `X_C` as having no row;
   - (control) change the register's `LN2_C` value cell from `-196` to `−196.0` (U+2212); this must still pass.

   (lane: main)
6. **Stale rows fail only when their file exists.**
   - On main, where `crates/bevy-client/` is absent, `C-` rows whose files do not exist produce one warning line with the count, not a failure (test 1 covers this).
   - Against a scratch copy of `FPS` where the `Melting...` caption text has been edited, the test fails for that stale row (naming C-015) as well as for the new unsourced string.
   - Against a scratch copy of `FPS` where `CH₄ + 2O₂ → CO₂ + 2H₂O` is removed from `web/index.html` only (chem.rs unchanged), the test fails naming C-001 and `crates/bevy-client/web/index.html`.

   (lane: main)
7. **Extractor unit fixtures** (inline in the test file, no repo files needed). Only shapes that occur in the `6d5463e` slice are required. They must assert all of the following:
   - `//` and `///` comments are ignored, so the doc comment `/// Liquid nitrogen boils at 77 K.` is not a claim; and a string containing `//` (for example `"see http://x.org: CO2 is heavy"`) is still extracted whole. The Rust extractor tokenizes; it does not strip comments by regex.
   - Char literals `'"'` and `'\\'` (as in `main.rs` `s.replace('\\', "\\\\").replace('"', "\\\"")`) do not open or close a string.
   - Strings inside a `#[cfg(test)] mod tests { ... }` block are ignored.
   - Strings containing `{:` or starting with `{{` are skipped as machine output; `"Dry \"fuel\" catches fire fast."` is not skipped and is a claim.
   - HTML text nodes in `web/**/*.html` are extracted, one string per text node (at `6d5463e` each claim sits in one text node, for example inside `<span class="formula">`).
   - JS single-quoted, double-quoted and template literals in `web/**/*.js` are extracted, with `${...}` kept literally as in N-1.
   - Normalization makes `CH₄ + 2O₂ → CO₂ + 2H₂O` equal to `CH4 + 2O2 → CO2 + 2H2O`, and `−196 °C` (U+2212) equal to `-196 °C`.
   - `"Fire"`, `"ice wall"`, `"steam vent"` and `"Click to look around"` are not claims.

   (lane: main)
8. **Register content is scientifically precise and sources are real.** For the rows in `content/science-sources.md`:
   - The builder derives every verdict and note from the cited source, not from Appendix A. Appendix A is a cross-check only. Every row where the builder's verdict differs from the appendix is listed in `docs/loop/items/I-004/lanes/main/round-1.md` with the source passage that decided it.
   - Every numeric claim states its conditions: phase temperatures at 101.325 kPa, enthalpies at 298.15 K and 1 bar, per mole of reaction as written.
   - Every `## Sources` entry is checkable: a URL, or author, title, edition and year. No page or section numbers appear unless the builder actually read them.
   - `round-1.md` has a source log: for each `[S<n>]`, how the builder verified it exists and says what the row claims (URL fetched on a date, or bibliographic record confirmed via a named catalogue). A source the builder could not verify is removed, and rows that depended on it get another source.

   The spec critic or breaker spot-checks at least 8 rows against their cited sources. The 8 must include at least 2 `simplified` rows, `ignition_c(DryHedge)` and `FLAME_C`. Any mismatch, or any source that cannot be found, is blocking. (lane: main)
9. **Protocol is complete and runnable.** `docs/loop/science-review.md` contains these sections: `Scope`, `Inputs`, `Procedure`, `Source verification`, `Tolerance rules`, `Verdicts`, `Output`, `B6 scoring`, `When it runs`, `Changing the register`.
   - `Verdicts` and `B6 scoring` reproduce Contract § Verdicts and B6 pass rule word for word.
   - `When it runs` lists exactly the level-2 science-review path triggers in `docs/loop/checks.md` (`content/**`, `crates/bevy-client/src/chem.rs`, `crates/sim/**`, plus nightly), and then a `Known gap` line noting that claims in `src/main.rs` and `web/**` do not trigger a review and that extending the trigger is a follow-up (see Follow-ups).
   - `Procedure` must include atom balancing per element and charge balancing for every equation; checks of state symbols, units and conditions; and a check that the code outcome matches the caption, by reading the `apply`/`step` branch that emits the caption's event.
   - `Source verification` requires the reviewer to open (URL) or confirm (bibliographic record) every cited source, and to record per source `verified` or `unverified` in the output. A row whose only sources are `unverified` counts as unsourced for B6.
   - It must include a worked example on one real row from the register.
   - A fresh model given only this doc, the register and `FPS` can produce an output file in the stated format without asking a question. The breaker runs this as a dry run and saves the output to `docs/loop/items/I-004/break-1-main.md`. (lane: main)

## Out of scope

- Editing any string, constant or behavior in `crates/bevy-client`. I-001 owns it. Known simplifications and any `error` verdict become new backlog items, not edits here.
- `.github/`, `package.json`, `tsconfig.json`, `docs/loop/checks.md`: no new scripts, no CI jobs, no trigger edits.
- `content/reactions.json` and `content/known_reactions.json`. The 3D client does not load them, so they are not "shown in game". When the 3D client starts loading them, the item that wires them in (I-008 or later) adds a `## Content` table and extends the lint in the same PR, per Contract § Rule for other items.
- The retired 2D game (`web/**` at the repo root) and `crates/sim/**`, which only the 2D game uses. If the 3D client starts depending on `crates/sim`, extending the scan is a follow-up item. Until then, `checks.md` still triggers a review on `crates/sim/**` changes, and that review covers only rows in the register.
- Running the scored B6 review. The scorer does that nightly, per the protocol.
- Teacher-facing citations UI, age-tiered wording, translations.
- Claims built at runtime from fragments (`concat!`, joined arrays) beyond `{name}` placeholders. The protocol tells the reviewer to read `apply`/`step` and caption call sites to catch these by hand.
- Extractor support for shapes the slice does not use: Rust raw strings (`r"..."`, `r#"..."#`), Rust `\`-newline continuations, inline `<script>`/`<style>` bodies (the slice's `index.html` has only an external `<script src>`). If a later client commit introduces one, the extractor must fail loudly (unknown token) rather than silently skip; adding support is then that item's job.

## Contract

Single lane, so no contract builder. The register format, the verdict meanings, the B6 pass rule and the test's environment variables below are the interface that later items (I-001 integrate, I-008 content) build against.

### Verdicts and B6 pass rule

- `correct`: the statement shown to the player is true as stated, at the conditions in the register header, within the tolerance rules.
- `simplified`: the statement is true but idealized or incomplete (it omits a mechanism, rounds, or describes a limiting case), and no reasonable reading of it is false. The note says what is simplified and why it is not false.
- `error`: a statement shown to the player that is false, or a parameter that drives a shown outcome and lies outside every cited sourced range. The note gives the correct statement or value with its source.
- **B6 pass rule:** B6 passes for a build when (a) the lint reports every client claim sourced (`<n> client claims` equals `<n> sourced`), (b) the fresh reviewer's output has 0 `error` verdicts, and (c) no row's sources are all `unverified` in that output. `simplified` verdicts pass. The reviewer has final say on verdicts and may escalate a `simplified` row to `error`.

### `content/science-sources.md` format

- **Header paragraph.** States the standard conditions (phase temperatures at 101.325 kPa; ΔH° at 298.15 K, 1 bar, per mole of reaction as written), the verdict meanings above, and how to add a row.
- **`## Claims` table**, with columns in this exact order:
  `| id | claim | where | verdict | note | sources |`
  - `id`: `C-NNN` for client strings. Unique across the whole file and never reused.
  - `claim`: the exact in-game text wrapped in single backticks. Placeholders such as `{who}` and `${...}` are kept literally. A literal `|` is written `\|`.
  - `where`: repo-relative path of the file that shows it, for example `crates/bevy-client/src/chem.rs`. If the same text appears in several files, list them comma-separated.
  - `verdict`: one of `correct`, `simplified` or `error`.
  - `note`: may be empty only when the verdict is `correct`.
  - `sources`: one or more `[S<n>]` separated by spaces. `[S0]` is not allowed here.
- **`## Parameters` table**, columns `| id | constant | value | unit | where | verdict | note | sources |`.
  - `id`: `P-NNN`.
  - `constant` is the Rust name in backticks (for example `` `LN2_C` ``) or a described literal (for example `` `ignition_c(DryHedge)` ``).
  - `value` is a number. ASCII `-` and U+2212 are both accepted; the cell is normalized and parsed as a decimal number, and compared numerically, so `-196`, `-196.0` and `−196` are equal. `unit` is `°C` for temperatures.
  - The lint requires one row per `[pub] const <NAME>_C: f32 = <v>;` in `crates/bevy-client/src/chem.rs` (non-test code), matched by `constant` equal to `` `<NAME>` ``, and checks `value` against `<v>`.
  - Rows for described literals are checked only by the reviewer.
  - `[S0]` is defined in `## Sources` as "Design choice: gameplay tuning or scenario assumption, no physical claim". A row may cite `[S0]` only in `## Parameters`, and only when its note starts with `gameplay tuning` or `scenario assumption`. Pure tuning numbers (`COOLING`, `SPREAD_HEAT`, `SMOTHER_S`, `NEIGHBOUR_GAP`) are listed with verdict `correct`, note `gameplay tuning, not a physical claim`, and `[S0]`.
- **`## Sources` list.** One bullet per source, in the form `- [S<n>] <checkable reference>`. Ids must be unique.
- **`## Not claims` table** (optional), columns `| text | where | reason |`. It lists strings that the heuristic flags but that are not science claims, for example a status counter. The lint treats a listed string as satisfied. Each `reason` must be non-empty.

### Claim heuristic (shared by the lint and the protocol)

**Scanned files** (all paths relative to `SCIENCE_SCAN_ROOT`, which defaults to the repo root):

- `crates/bevy-client/src/**/*.rs`: string literals found by a tokenizer that understands `//` and `/* */` comments, char literals (including `'"'` and `'\\'`), lifetimes and `"..."` strings with escapes. Strings inside `#[cfg(test)] mod <name> { ... }` blocks are dropped. Comment stripping by regex is not acceptable.
- `crates/bevy-client/web/**/*.html`: text nodes.
- `crates/bevy-client/web/**/*.js`: `'...'`, `"..."` and template literals, with `${...}` kept literally.

**Skips.** A string is skipped as machine output when it contains `{:` or starts with `{{`. Nothing else is skipped.

**Normalization.** Applied before flagging, matching and stale checks, in this order:

1. NFC.
2. Subscript digits ₀–₉ become 0–9.
3. U+2212 becomes `-`.
4. Whitespace collapses to single spaces.
5. Leading and trailing whitespace is trimmed.

Rust escapes (`\"`, `\\`, `\n`) are decoded before normalization; JS escapes likewise.

**Claim test.** A normalized string is a claim if any of the following match:

- a temperature, matching `/-?\d+(\.\d+)?\s*°\s*[CF]\b|\b\d+(\.\d+)?\s*K\b/`;
- a chemical formula token that contains a digit or state symbol, such as `CO2`, `H2O`, `N2(l)` or `O2`. Pattern: `/\b(?:[A-Z][a-z]?\d*)+(?:\((?:s|l|g|aq)\))?/` with at least one digit or state symbol present;
- the case-insensitive keyword pattern `/ignit|combust|burn|fuel|boil|melt|freez|froze|latent|oxygen|nitrogen|electroly|smother|heavier than air|\bheat/`.

A claim is satisfied when its normalized text equals the normalized `claim` cell of any row in `## Claims`, or a `text` in `## Not claims`.

### Stale-row rule

- Stale checks apply to `C-` rows only. `P-` rows are covered by the `_C` pin (for constants) and by the reviewer (for described literals).
- For each file in a `C-` row's `where` cell: if the file does not exist under `SCIENCE_SCAN_ROOT`, it counts toward the single warning line; if it exists and none of the strings extracted from it (after skips and normalization) equals the row's normalized claim, the row is stale for that file and the test fails, naming the row id and the file.
- A multi-file row is therefore stale if any listed file exists and lacks the text.

### Verbose inventory

With `SCIENCE_SOURCES_VERBOSE=1` the test prints, before the summary line, one line per distinct normalized flagged string, sorted by UTF-16 code unit order:
`claim: <normalized text>`
followed by nothing else on the line (no path, no id). Strings satisfied by `## Not claims` are printed too.

### Test environment variables

All are optional.

| variable | default | purpose |
|---|---|---|
| `SCIENCE_SCAN_ROOT` | repo root | where the client tree is read from |
| `SCIENCE_SOURCES_FILE` | `content/science-sources.md` | the register |
| `SCIENCE_SOURCES_VERBOSE` | unset | `1` prints the inventory |

The test always prints one summary line for the scorer:
`science-sources: <n> client claims, <n> sourced, <n> params, <n> error verdicts, <n> stale-file warnings`.
The test does **not** fail on `error` verdicts. Errors are a B6 score input and a backlog trigger, not a CI break. Sourcing gaps, malformed rows, `[S0]` misuse, unpinned or mismatched `_C` constants and stale rows do fail it.

### Rule for other items (to be written into `science-review.md` § Changing the register)

Any item that adds or changes a player-facing science string or a `_C` constant, or that makes the 3D client load `content/*.json` science, must add or update its register rows (and, for content, the lint) in the same PR. In a multi-lane item, the contract step owns `content/science-sources.md`. In a single-lane item, the lane owns it.

## Risks and notes for the orchestrator

- **Coupling with I-001.** If I-004 merges before I-001 and I-001 then changes a caption, I-001's CI will fail on that string. I-001's integrate step must then edit the register, outside its lanes. Prefer merging I-004 **after** I-001 and re-running test 2's command against main (with the expected set taken from the register, not Appendix A). Otherwise tell I-001 before it merges.
- **B6 likely starts at 1+ `error`.** C-004 (`CO2 blanket, no O2`) is literally false as worded; it is pre-assessed `simplified` as a limiting case, but a teacher may read it as an error and the reviewer may escalate it. The CO2-foam rows (C-012, C-018, C-029) rest on an obsolete product. Either outcome is acceptable and becomes a backlog item.

## Follow-ups (backlog candidates, not built here)

- Reword C-004 to "less O2" (or similar) in the client; owner I-001 or a later client item.
- Extend `docs/loop/checks.md` level-2 science-review triggers to `crates/bevy-client/src/main.rs` and `crates/bevy-client/web/**`, where claims also live.
- Add a `## Content` table and lint coverage for `content/reactions.json` / `known_reactions.json` when the 3D client loads them (I-008).

## Lanes

Merge order is the row order.

| lane | owns | builds |
|------|------|--------|
| main | `content/science-sources.md`, `test/science-sources.test.ts`, `docs/loop/science-review.md`, `docs/loop/items/I-004/lanes/**` | register (claims, parameters, sources, not-claims), source lint test with extractor fixtures, science-review protocol |

## Appendix A: client claims at `6d5463e` and planner pre-assessment

The heuristic in the Contract flags exactly **33 distinct normalized strings** in `FPS`. I checked this by running a throwaway prototype of the heuristic over `6d5463e`; the prototype is not part of the deliverable. Of the 33:

- 32 are claims and need a `## Claims` row each (C-001 to C-032 below).
- 1 is a status line that goes in `## Not claims` (N-1 below).

Acceptance test 2 compares the extractor's printed inventory to this table: 33 strings, identical after normalization. If the builder's extractor differs, the builder either fixes the extractor or records the difference and its justification in round notes, and the critic decides.

Verdicts are the planner's cross-check, not the answer. The builder derives each verdict from the cited source and records disagreements (test 8); the fresh reviewer has the final say. Phase temperatures are at 101.325 kPa.

| # | claim (normalized: subscripts to digits, U+2212 to `-`) | where | verdict | note / reasoning |
|---|---|---|---|---|
| C-001 | `CH4 + 2O2 → CO2 + 2H2O` | chem.rs `formula`, web/index.html (as `CH₄ + 2O₂ → CO₂ + 2H₂O`) | correct | Balanced (C 1/1, H 4/4, O 4/4). |
| C-002 | `H2O absorbs heat` | chem.rs | correct | Specific heat about 4.18 J g⁻¹ K⁻¹. ΔvapH = 40.65 kJ/mol at the normal boiling point. |
| C-003 | `N2(l) boils at -196 °C` | chem.rs, web/index.html | correct | Normal boiling point ≈77.35 K = −195.8 °C, which rounds to −196. |
| C-004 | `CO2 blanket, no O2` | chem.rs | simplified | The blanket excludes or dilutes O2 below the level flaming combustion needs (roughly 15 vol% or less for most fuels). "No O2" is the limiting case. |
| C-005 | `Combustion: CH4 + 2O2 → CO2 + 2H2O, and lots of heat.` | chem.rs | correct | ΔcH° = −802.3 kJ/mol with H2O as gas, or −890.4 kJ/mol with H2O as liquid. |
| C-006 | `Water soaks up heat and cools things below their ignition temperature.` | chem.rs | correct | Cooling is the main mechanism. Steam also dilutes O2; the caption leaves this out but does not contradict it. |
| C-007 | `Liquid nitrogen boils at -196 °C (77 K), pulling heat out of anything it touches.` | chem.rs | correct | 77.35 K rounds to 77 K. The Leidenfrost vapour layer slows heat transfer, but the statement is still true. |
| C-008 | `CO2 is heavier than air: it smothers fire by pushing the oxygen away.` | chem.rs | correct | M(CO2) = 44.01 g/mol vs about 28.96 g/mol for dry air, so CO2 is about 1.52 times as dense at the same T and p. |
| C-009 | `It caught fire: hot enough to reach its ignition temperature, with oxygen around.` | chem.rs | correct | Fire triangle: heat, fuel, oxidiser. |
| C-010 | `Fire out! Water cooled it below its ignition temperature.` | chem.rs | correct | |
| C-011 | `Fire out! Liquid nitrogen at -196 °C cooled it far below ignition.` | chem.rs | correct | Boiling N2 also inerts the fire by displacing O2; the caption does not mention this. |
| C-012 | `Fire out! The CO2 foam smothered it: no oxygen, no fire.` | chem.rs | simplified | Modern firefighting foams are mostly air-filled (AFFF). CO2-filled foam is the obsolete "chemical foam" (NaHCO3 + Al2(SO4)3), and CO2 extinguishers discharge gas and snow, not foam. The smothering mechanism is right. |
| C-013 | `It burnt out: the fuel is used up.` | chem.rs | correct | |
| C-014 | `The ice melted at 0 °C: H2O(s) → H2O(l). Now it is a puddle.` | chem.rs | correct | Tfus = 273.15 K at 1 atm. |
| C-015 | `Melting... ice needs a lot of heat to turn into water (latent heat).` | chem.rs | correct | ΔfusH = 6.01 kJ/mol, about 334 J/g. |
| C-016 | `Frozen! Liquid nitrogen at -196 °C (77 K) froze the water: H2O(l) → H2O(s).` | chem.rs | simplified | The text is true. The outcome freezes a whole puddle in one cast, much faster than heat transfer allows. |
| C-017 | `Hot water boils at 100 °C and turns into steam: H2O(l) → H2O(g).` | chem.rs | correct | 99.97 °C at 101.325 kPa (ITS-90), which rounds to 100. |
| C-018 | `CO2 foam covers it: no oxygen can reach it, so it cannot burn.` | chem.rs | simplified | As C-004 and C-012. |
| C-019 | `Soaked: it must dry out (boil off at 100 °C) before it can burn.` | chem.rs | simplified | Water evaporates at any temperature. Under flame heating the wet surface stays near 100 °C until the water is gone, and that part is true. |
| C-020 | `Brrr! Liquid nitrogen boils away at -196 °C as it chills the surface.` | chem.rs | correct | |
| C-021 | `Sizzle! The water must boil away at 100 °C before it can burn.` | chem.rs | simplified | As C-019. |
| C-022 | `No oxygen under the CO2 blanket, so the fire cannot start.` | chem.rs | simplified | As C-004. |
| C-023 | `That does not burn.` | chem.rs | correct | No current object can reach this branch. The row is kept so the string stays sourced. |
| C-024 | `Already burnt: no fuel left.` | chem.rs | simplified | Char is mostly carbon and can still burn by glowing or smouldering combustion. Only ash has no fuel left. |
| C-025 | `Heating up... a little more heat and it will catch fire.` | chem.rs | correct | |
| C-026 | `Water-splitting station (coming soon): 2H2O → 2H2 + O2 with electricity.` | chem.rs | correct | Balanced (H 4/4, O 2/2). ΔH° = +571.7 kJ per reaction as written, with liquid water. |
| C-027 | `Oh no! Heat from the fire reached the greenhouse. Press R to try again.` | main.rs | correct | |
| C-028 | `Heat from a neighbour raised the {who} to its ignition temperature: it caught fire!` | main.rs | correct | Radiant and convective heat transfer. |
| C-029 | `You saved the greenhouse! Water, frost and CO2 foam each stop fire in their own way.` | main.rs | correct | Water and frost cool (frost also inerts), and foam excludes oxygen. |
| C-030 | `Heat spread from bush to bush. Press R to try again.` | web/hud.js | correct | |
| C-031 | `cools below ignition` | web/index.html | correct | |
| C-032 | `CO2 displaces O2` | web/index.html (as `CO₂ displaces O₂`) | correct | It displaces or dilutes O2. |
| N-1 | `` Fires put out: ${Math.min(state.extinguished, state.goal)} / ${state.goal} · burning now: ${state.burning} `` | web/hud.js | not a claim | A status counter. It is flagged only by the keyword `burn`. |

Strings the heuristic must **not** flag:
- `refreezes flat, as a sheet` (inside `#[cfg(test)]`);
- `IMPACT obj={} {:?} -> {:?}` and `STATS fps={:.1}...` (contain `{:`), and the `{{\"i\"...` and `{{\"frame\"...` JSON format strings (start with `{{`);
- the doc comment `/// Liquid nitrogen boils at 77 K.` in chem.rs;
- `ice wall`, `steam vent`, `Fire`, `Frost`, `Fires put out: 0 / 3`.

## Appendix B: parameters (pre-assessment)

The builder derives each verdict from its source; this table is a cross-check only.

| constant | value | verdict | note |
|---|---|---|---|
| `LN2_C` | −196 °C | correct | Normal boiling point 77.35 K. |
| `AMBIENT_C` | 20 °C | correct | Note `scenario assumption: a mild day`. Cite [S0]. |
| `FLAME_C` | 800 °C | simplified | Wood and vegetation flames are about 600–1200 °C depending on position and size. Burning char surfaces are about 500–1000 °C. One value stands in for both. The builder must cite a source whose range contains 800, or change the verdict to `error` and note the sourced range. |
| `ignition_c(Bush/Greenhouse)` | 300 °C | simplified | Wood piloted ignition is about 250–350 °C, depending on heating rate and moisture. |
| `ignition_c(DryHedge)` | 230 °C | simplified | Dry leaf litter ignites lower than solid wood. The builder must find a source that gives a range containing 230, or change the verdict to `error` and note the sourced range. |
| `Obj::new(SteamVent)` | 110 °C | correct | Scenario value: superheated steam is above 100 °C at 1 atm. Needs a physical source ([S2]) for "steam above 100 °C at 1 atm"; `[S0]` alone is allowed only if the note starts `scenario assumption`. |
| `Obj::new(IceWall)` | −5 °C | correct | Ice below its melting point ([S2]). |
| Water clamps to 100 °C; Frost sets −196 °C instantly | — | simplified | These outcomes are not shown as text. The reviewer notes that instant equilibrium is a simplification; it is not a B6 error unless a caption states it. |
| `COOLING`, `SPREAD_HEAT`, `SMOTHER_S`, `NEIGHBOUR_GAP` | — | correct | `gameplay tuning, not a physical claim`, [S0]. |

Electrolysis prop (world.rs comment, shown visually): the H2 tube holds twice the gas volume of the O2 tube. This is correct by Avogadro's law (2:1 mole ratio). It gets a `P-` row with `where = crates/bevy-client/src/world.rs`, checked by the reviewer.

## Suggested sources (the builder verifies each, logs how in round-1.md, and may replace any)

- **[S0]** Design choice: gameplay tuning or scenario assumption, no physical claim.
- **[S1]** NIST Chemistry WebBook, SRD 69, Nitrogen: https://webbook.nist.gov/cgi/cbook.cgi?ID=C7727379 . Phase-change data (Tboil).
- **[S2]** NIST Chemistry WebBook, SRD 69, Water: https://webbook.nist.gov/cgi/cbook.cgi?ID=C7732185 . Tfus, Tboil, ΔfusH, ΔvapH.
- **[S3]** Cox, J. D., Wagman, D. D., Medvedev, V. A., *CODATA Key Values for Thermodynamics*, Hemisphere, 1989. ΔfH° for CO2(g), H2O(l) and H2O(g) (for C-005 and C-026).
- **[S4]** NIST Chemistry WebBook, Methane: https://webbook.nist.gov/cgi/cbook.cgi?ID=C74828 . ΔfH°gas and ΔcH°.
- **[S5]** Drysdale, D., *An Introduction to Fire Dynamics*, 3rd ed., Wiley, 2011. Ignition, extinction by cooling and by oxygen dilution, flame temperatures.
- **[S6]** Babrauskas, V., *Ignition Handbook*, Fire Science Publishers, 2003. Ignition temperatures of wood and vegetation.
- **[S7]** OpenStax, *Chemistry 2e*, 2019, https://openstax.org/details/books/chemistry-2e . Balancing, phase changes, latent heat, Avogadro's law, electrolysis.
- **[S8]** IUPAC standard atomic weights, and dry-air composition (for example the U.S. Standard Atmosphere, 1976). Molar masses for the "heavier than air" claim.

(Revision 1's [S9] RS-25 and [S10] hydrogen flammability supported content-JSON narrative only and are dropped with that scope.)

## Critique disposition

Applied from `critique.md` "Required changes":

1. **Pin test 2 to `6d5463e`.** Applied. `FPS` is now exactly `6d5463e` for all tests; later commits get their expected set from the register via I-001 integrate.
2. **`[S0]` restriction and test.** Applied, adjusted: `[S0]` is allowed only in `## Parameters`, with a note starting `gameplay tuning` **or** `scenario assumption`. The critic's version (only `gameplay tuning, not a physical claim`) would forbid `AMBIENT_C = 20`, which is honestly a scenario choice with no physical source. Two mutations added to test 4.
3. **New `_C` constant with no row; `error` row with empty note.** Applied (tests 5 and 4). Also added a control showing U+2212 / `-196.0` value cells still pass.
4. **Narrow the escaped-quote skip.** Applied: skip only on `{:` or a leading `{{`. Checked against `6d5463e`: every machine-output string in `main.rs` still skips. Fixture and mutation added (tests 3 and 7).
5. **Verbose inventory format and stale-row rules.** Applied: Contract § Verbose inventory (`claim: <text>`, sorted) and § Stale-row rule (C- rows only, normalized text, stale if any listed existing file lacks it). Test 6 gained a multi-file case.
6. **Verdict meanings and B6 pass rule in the Contract.** Applied: Contract § Verdicts and B6 pass rule; the protocol must copy it word for word (test 9).
7. **Drop content-JSON rows; drop unused extractor shapes.** Content rows, old test 6, `SCIENCE_CONTENT_DIR`, Appendix C and sources S9/S10 dropped; coverage moves to the item that makes the 3D client load the JSON (Follow-ups). Raw-string, `\`-continuation and `<script>`/`<style>`-body fixtures dropped, and those shapes are listed as out of scope with a fail-loudly rule. See rejection below for the HTML/JS extractors themselves.
8. **Reviewer verifies each source; spot-check composition.** Applied: protocol § Source verification with `verified`/`unverified` per source feeding the B6 pass rule; builder source log in round-1.md; spot-check must include ≥2 `simplified` rows, `ignition_c(DryHedge)` and `FLAME_C`. Test 8 now says verdicts are derived from sources, with Appendix A as cross-check only.

Also applied from the body of the critique: Rust extractor must tokenize (no regex comment stripping); `value` cell syntax and numeric comparison; confirmation that `docs/loop/checks.md` exists (it does, with level-2 science triggers `content/**`, `crates/bevy-client/src/chem.rs`, `crates/sim/**`); the C-004 escalation and CO2-foam warnings (Risks); a C-004 reword follow-up; preference to merge after I-001.

Rejected or partly rejected:

- **Dropping the JS and HTML extractors (critique "Smallest version" item 2).** Rejected for the extractors, accepted for `<script>`/`<style>` handling. Six of the 33 Appendix A strings live only or also in `web/index.html` (C-001, C-003, C-031, C-032) and `web/hud.js` (C-030, N-1). Without these extractors the lint cannot see them, and "100% sourced" would be false. The fixtures kept are exactly the shapes those strings use (HTML text nodes, JS quoted and template literals).
- **Keeping test 10 "exactly the checks.md triggers" unchanged.** Kept, but with a required `Known gap` line: the checks.md triggers miss `src/main.rs` and `web/**`, which hold claims. This item may not edit `checks.md` (out of its area), so the gap is a follow-up rather than silently ignored. The lint still catches unsourced strings there on every change via level 1.
- **Added beyond the critique:** a char-literal fixture (`'"'`, `'\\'`), because `main.rs` at `6d5463e` contains `s.replace('\\', "\\\\").replace('"', "\\\"")`, which a naive tokenizer would mis-pair; and C-015/C-001 named explicitly in stale-row tests.
