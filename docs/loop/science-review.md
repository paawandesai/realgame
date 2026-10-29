# Science review protocol

The procedure the fresh-model science reviewer follows to score bar B6 ("science is right"). The reviewer is a model that did not write the content or the register. Follow the steps in order; everything you need is named here. Do not ask questions: when something is ambiguous, apply the tolerance rules, record your reasoning in the row's note, and carry on.

## Scope

- Every row in `content/science-sources.md`: the `## Claims` rows (`C-NNN`, strings the 3D client shows the player) and the `## Parameters` rows (`P-NNN`, numbers in the rules engine that drive a shown outcome).
- The client code that produces those strings and outcomes: `crates/bevy-client/src/**/*.rs` and `crates/bevy-client/web/**` (`.html`, `.js`).
- Out of scope: `content/reactions.json`, `content/known_reactions.json` (the 3D client does not load them yet), the retired 2D game (`web/` at the repo root) and `crates/sim/**` (used only by the 2D game). If the 3D client starts loading or depending on any of these, the item that wires it in extends the register and the lint (see Changing the register).
- `## Not claims` rows are checked only for being honestly non-scientific (step 2).

## Inputs

1. The build under review: a checkout of the commit to score (the "build"). Record its full SHA.
2. `content/science-sources.md` at that commit (the "register").
3. The lint output at that commit: run `SCIENCE_SOURCES_VERBOSE=1 node --disable-warning=ExperimentalWarning --test --test-reporter=spec test/science-sources.test.ts` from the repo root. If the client tree is somewhere else (for example a branch not yet on main), add `SCIENCE_SCAN_ROOT=<path to that checkout>`. Keep the `claim:` lines and the summary line `science-sources: <n> client claims, <n> sourced, <n> params, <n> error verdicts, <n> stale-file warnings`.
4. Web access (to open source URLs) or a library catalogue (to confirm bibliographic records).

Nothing else is needed. Do not read the planner's spec or earlier reviews before forming your own verdicts.

## Procedure

For the build:

1. **Lint.** Run the lint command in Inputs. Copy its summary line into the output. If it exits non-zero, list every failure message in the output; any unsourced claim or stale row already fails B6 part (a).
2. **Inventory.** Compare the `claim:` lines with the register. Every line must be the normalized text of a `## Claims` row or a `## Not claims` row. For each `## Not claims` row, confirm it states no science (a counter, a label); if it does, record it as an `error` with note "science claim filed under Not claims".
3. **Runtime-built strings.** The lint cannot see claims assembled at runtime. Open `crates/bevy-client/src/main.rs` and `crates/bevy-client/web/hud.js` and find every place that sets the caption, banner or HUD text (`game.say(`, `format!(`, `.textContent =`). Any player-facing science text that is built from fragments (`concat!`, joined arrays, `+` on strings) and does not match a register row is a finding: record it as an extra row `X-<n>` with verdict `error` and note "unsourced runtime-built claim".
4. **Per `C-` row**, in id order:
   1. **Read the source.** Open each cited source (see Source verification) and find the passage that supports or contradicts the claim. Form your verdict from the source, not from the register's verdict.
   2. **Equations.** For every chemical equation in the claim: count atoms of each element on each side (write the counts, for example `C 1/1, H 4/4, O 4/4`) and check total charge is equal on both sides (write `charge 0/0`). Any imbalance is an `error`.
   3. **State symbols.** Check each `(s)`, `(l)`, `(g)`, `(aq)` is the right phase at the conditions the caption describes (for example `H2O(s)` for ice, `N2(l)` for liquid nitrogen).
   4. **Numbers, units and conditions.** Check every number against the source after converting units (K = °C + 273.15). Phase temperatures are at 101.325 kPa; ΔH° at 298.15 K and 1 bar per mole of reaction as written. Apply the Tolerance rules.
   5. **Outcome matches caption.** Find the `EventKind` whose `caption()` returns the text (in `crates/bevy-client/src/chem.rs`), or the call site in `main.rs` / `hud.js`. Then read the branch of `World::apply` or `World::step` that emits that event, and confirm the code does what the caption says (for example, a caption saying water cooled the fire must come from a branch that lowers the temperature and stops burning). A caption that misdescribes its own outcome is an `error`; one whose outcome is idealized (instant, total) is at most `simplified`.
   6. **Verdict.** Assign `correct`, `simplified` or `error` per Verdicts. Write a note for anything other than `correct`; for `error`, give the correct statement or value with its source.
5. **Per `P-` row:**
   - For a `_C` constant, the lint has already checked the value matches the code. Check the value against the cited source's range.
   - For a described literal (for example `ignition_c(DryHedge)`, `Obj::new(SteamVent)`), find the literal in the named file, confirm the register value equals it, then check it against the source's range. A mismatch between code and register is an `error`.
   - For a row citing `[S0]`, confirm the number makes no physical claim (pure tuning) or is an honest scenario choice. If a player could read it as a physical fact (it is captioned or drives a captioned outcome that states it), it needs a physical source: record `error` with note "physical claim filed as design choice".
   - A parameter that lies outside every cited sourced range is an `error`; one inside a sourced range but standing in for a spread of real values is `simplified`.
6. **Count.** Tally verdicts and unverified sources and apply B6 scoring.

### Worked example (row C-001)

Register row:

```
| C-001 | `CH4 + 2O2 → CO2 + 2H2O` | crates/bevy-client/src/chem.rs, crates/bevy-client/web/index.html | correct | | [S7] |
```

1. Source: open [S7] (OpenStax *Chemistry 2e* § 4.1). It gives the balanced methane combustion equation `CH4 + 2O2 ⟶ CO2 + 2H2O` and defines a balanced equation as having equal numbers of atoms of each element on both sides. Record `[S7] verified, URL fetched <date>`.
2. Equation: C 1/1, H 4/4, O 4/4, charge 0/0. Balanced.
3. State symbols: none given; nothing to check.
4. Numbers: none beyond stoichiometric coefficients.
5. Outcome: the text is `Spell::Fire.formula()` (hotbar label) and the hotbar `<span class="formula">` in `index.html`. Casting Fire on a dry flammable object runs the `Spell::Fire` arm of `World::apply`, which raises the temperature and sets `burning`. A combustion label on the fire spell matches.
6. Verdict: `correct`, no note.

Output row:

```
| C-001 | correct | correct | C 1/1, H 4/4, O 4/4, charge 0/0; label on Fire spell; apply Spell::Fire ignites | |
```

## Source verification

- Open every source cited by any row: fetch its URL, or for a book or paper without a free URL confirm its bibliographic record (authors, title, venue, year, DOI or ISBN) in a named catalogue (a university library catalogue, Crossref, the publisher's page). Do this once per source, not once per row.
- Check that the source says what the rows citing it claim. A source that exists but does not support a row does not count for that row: say so in the row's note and judge the row as if that source were absent.
- Record per source, in the output's `## Sources` table, `verified` (opened or record confirmed, and it supports what is cited) or `unverified` (not found, not reachable, or does not say what is claimed), with how you checked and the date.
- `[S0]` (design choice) needs no verification; mark it `n/a`.
- A row whose only sources are `unverified` counts as unsourced for B6.
- Never invent a source, a page number or a quotation. Quote only what you actually read.

## Tolerance rules

- **Rounding.** A displayed value is correct if the sourced value rounds to it at the displayed precision: −195.8 °C shows as −196 °C; 77.34 K shows as 77 K; 373.17 K (100.02 °C) shows as 100 °C.
- **Conditions.** A phase temperature without stated pressure means 101.325 kPa. An enthalpy without stated conditions means 298.15 K and 1 bar, per mole of reaction as written. A claim that holds only under unstated non-standard conditions is `simplified` at best.
- **Ranges.** A single in-game value for a quantity that the sources give as a range (flame temperatures, ignition temperatures) is `simplified` if it lies inside a cited range, and `error` if it lies outside every cited range.
- **Limiting cases and omissions.** "No oxygen" for "not enough oxygen to burn", "instantly" for a fast process, or naming one mechanism of several, is `simplified` when no reasonable reading is false. If a curious 9-year-old or a chemistry teacher would come away believing something false, it is `error`.
- **Game outcomes not shown as text** (instant equilibrium, clamped temperatures) are judged only through the captions that describe them.
- **Placeholders** (`{who}`, `${...}`) are judged with every value the code can substitute.

## Verdicts

- `correct`: the statement shown to the player is true as stated, at the conditions in the register header, within the tolerance rules.
- `simplified`: the statement is true but idealized or incomplete (it omits a mechanism, rounds, or describes a limiting case), and no reasonable reading of it is false. The note says what is simplified and why it is not false.
- `error`: a statement shown to the player that is false, or a parameter that drives a shown outcome and lies outside every cited sourced range. The note gives the correct statement or value with its source.
- **B6 pass rule:** B6 passes for a build when (a) the lint reports every client claim sourced (`<n> client claims` equals `<n> sourced`), (b) the fresh reviewer's output has 0 `error` verdicts, and (c) no row's sources are all `unverified` in that output. `simplified` verdicts pass. The reviewer has final say on verdicts and may escalate a `simplified` row to `error`.

## Output

Write one Markdown file. For a scored (nightly or triggered) review the path is `docs/loop/science-reviews/<YYYY-MM-DD>-<short sha>.md`; for a dry run, use the path the task gives you. Use exactly these sections, in this order:

```markdown
# Science review <YYYY-MM-DD> <short sha>

build: <full sha>
reviewer: <model name>, fresh (did not write the register or the content)
lint: science-sources: <n> client claims, <n> sourced, <n> params, <n> error verdicts, <n> stale-file warnings
lint exit: <0 or 1>

## Sources

| source | status | how checked |
|---|---|---|
| [S0] | n/a | design choice |
| [S1] | verified | URL fetched 2026-10-29; Tboil 77.34 K |

## Rows

| id | verdict | register verdict | checks | note |
|---|---|---|---|---|
| C-001 | correct | correct | C 1/1, H 4/4, O 4/4, charge 0/0; ... | |

## Lint failures and inventory findings

- none (or one bullet per lint failure, Not-claims finding or X- row)

## Summary

rows: <n> (claims <n>, params <n>, extra <n>)
correct: <n>
simplified: <n>
error: <n>
rows with only unverified sources: <n>
B6: pass | fail (<which of (a), (b), (c) failed>)

## Backlog items

- <one line per error: "Fix C-012: <correct statement> [S<n>]", or "none">
```

- One `## Rows` line per register row (every `C-` and `P-` row) plus any `X-` rows, in id order.
- `checks` records what you actually checked (atom and charge counts, unit conversions, the code branch read).
- `register verdict` is the verdict in the register at that commit, so disagreements are visible.

## B6 scoring

- `correct`: the statement shown to the player is true as stated, at the conditions in the register header, within the tolerance rules.
- `simplified`: the statement is true but idealized or incomplete (it omits a mechanism, rounds, or describes a limiting case), and no reasonable reading of it is false. The note says what is simplified and why it is not false.
- `error`: a statement shown to the player that is false, or a parameter that drives a shown outcome and lies outside every cited sourced range. The note gives the correct statement or value with its source.
- **B6 pass rule:** B6 passes for a build when (a) the lint reports every client claim sourced (`<n> client claims` equals `<n> sourced`), (b) the fresh reviewer's output has 0 `error` verdicts, and (c) no row's sources are all `unverified` in that output. `simplified` verdicts pass. The reviewer has final say on verdicts and may escalate a `simplified` row to `error`.

The scorer reads the `## Summary` block: `B6: pass` only when all three parts hold.

## When it runs

Exactly the level-2 science-review triggers in `docs/loop/checks.md`:

- `content/**`
- `crates/bevy-client/src/chem.rs`
- `crates/sim/**`
- nightly

Known gap: claims also live in `crates/bevy-client/src/main.rs` and `crates/bevy-client/web/**`, and changes there do not trigger a science review (the level-1 lint still catches unsourced or stale strings there on every change). Extending the trigger to those paths is a follow-up item; this protocol does not change `checks.md`.

## Changing the register

- **Who changes verdicts.** The register's verdicts are the builder's, derived from the cited sources. The fresh reviewer's output has final say for scoring: it may escalate `simplified` to `error` or downgrade with a reason. The reviewer does not edit the register; the next item that touches the affected rows (or a dedicated fix item) updates the register to match the accepted review, in its own PR.
- **How an `error` becomes a backlog item.** For each `error` in a review, the orchestrator adds one backlog line: "Fix <row id>: <correct statement or value> [source]", moving bar B6, track product (or content), with the review file as evidence. The fix changes the client string or constant and the register row in the same PR.
- **Rule for other items.** Any item that adds or changes a player-facing science string or a `_C` constant, or that makes the 3D client load `content/*.json` science, must add or update its register rows (and, for content, the lint) in the same PR. In a multi-lane item, the contract step owns `content/science-sources.md`. In a single-lane item, the lane owns it.
- Ids are never reused. When a string leaves the client, its row is deleted (the lint fails on stale rows) and its id stays retired.
