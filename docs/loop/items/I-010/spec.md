# I-010: Science lint hardening from the I-004 break report

Base: `origin/main` eb65a6d (I-004 merged). Client under test: `claude/fps-prototype` 6d5463e (not on main; the lint reads it via `SCIENCE_SCAN_ROOT`). Evidence: `docs/loop/items/I-004/break-1-main.md` (Should-fix 1 to 3, Nit on S8).

## Problem

I-004 landed the science register (`content/science-sources.md`), the source lint (`test/science-sources.test.ts`) and the reviewer protocol (`docs/loop/science-review.md`). Its breaker found four gaps that let B6 pass when the science is not actually sourced:

1. **C-012 / C-018 cite a source that does not say what the note says.** Both notes state "CO2 extinguishers discharge gas, and firefighting foams are made with air" (C-018 says "As C-012 ... 'CO2 foam' is not a common modern agent"). The only foam source, [S17] (a 2023 NIST PFAS news release), says foams "suppress oxygen flow and fuel vapors". It says nothing about air-filled foam or CO2 extinguishers. Under the protocol's own rule ("a source that exists but does not support a row does not count for that row"), half of each note is unsourced. C-029 cites [S17] for "CO2 foam ... stop[s] fire", which has the same weakness.
2. **[S8] may not contain the number it is cited for.** The register cites NASA SP-398 (*The 1976 Standard Atmosphere Above 86-km Altitude*) for the sea-level mean molar mass of air, M0 = 28.9644 kg/kmol, which C-008 and C-032 rely on. That value is defined in the *U.S. Standard Atmosphere, 1976* (NOAA-S/T 76-1562, also published as NASA-TM-X-74335). SP-398 covers the atmosphere above 86 km. Nobody has fetched SP-398 to confirm the number is in it.
3. **Keyword-free claims bypass the lint.** `isClaim` only flags strings with a temperature, a formula with a digit or state symbol, or one of 14 keywords. "Iron rusts in water.", "Sodium reacts violently with water." and "Acids turn litmus red; pH 3." each passed the lint with no row when added to the client. "100% of claims sourced" is really "100% of keyword-matching strings sourced", and neither the register nor the protocol says so.
4. **The `{:` skip is a cheap bypass.** `isSkipped` drops any string containing `{:` anywhere, so `"Water is H2O: {:?}"` passes unsourced. Skipped strings are not listed anywhere, so a reviewer cannot see what was dropped.

## Bar

**B6, Science is right** (level 2): "0 errors, 100% of claims sourced in `content/science-sources.md`". This item makes part (a) of the B6 pass rule (the lint's "every client claim sourced") cover more of what the player actually reads. It makes part (c) (no row whose sources are all unverified) honest for C-008, C-012, C-018, C-029 and C-032. It is measurement-track: no client change.

## Smallest version

One lane, three files.

1. **Foam/CO2 sourcing (register).** In the C-012 and C-018 notes (and the C-029 sources), every factual sentence must be supported by a cited source that the builder has fetched and quoted. Do one of these, per sentence:
   - add a source that states it. For "firefighting foam is a mass of air-filled bubbles", for example, an NFPA 11 definition, a U.S. Fire Administration or EPA foam page, or a fire-protection handbook passage. For "CO2 extinguishers discharge CO2 gas (and snow), not foam", [S14] may already say this: quote it if so. Or
   - drop the sentence and keep only what the cited sources support (foam smothers by separating oxygen and fuel vapour, [S17]; "no oxygen" is the limiting case, [S14]).

   The verdicts stay `simplified` unless the new source changes them. Do not invent a passage. If the builder finds that CO2-generating "chemical foam" was a real historical agent, the note may say so with a source. It must not imply that "CO2 foam" is fictional.
2. **[S8].** Fetch the U.S. Standard Atmosphere 1976 (NOAA-S/T 76-1562 / NASA-TM-X-74335, NTRS 19770009539) and find where it states M0 = 28.9644 kg/kmol (a table or equation reference). Replace the [S8] definition text in place with that document, its URL and the location. The id stays [S8], so C-008 and C-032 keep their citations, because it supports the same statement. If SP-398 is fetched and does state 28.9644, the builder may keep it and add the location instead. Either way, the round notes quote the passage. If no copy can be fetched, the builder says so in the round notes and does not call it verified; the breaker escalates.
3. **Narrow the machine-output skip.** Replace `s.includes("{:") || s.startsWith("{{")` with a skip that matches only the two shapes the client actually uses:
   - a JSON template: the decoded string starts with `{{"`;
   - a log line: it matches `^[A-Z][A-Z0-9_]{2,} [a-z_][a-z0-9_]*=` (upper-case tag, a space, then `key=`), for example `STARTUP t={:.0}ms`, `IMPACT obj={} ...`, `STATS fps=...`, `FIRST_FRAME t=...`.

   Every other string, including any containing `{:`, goes through `isClaim` as usual.
4. **List what is skipped.** The lint always prints one extra line, `science-sources: <n> skipped as machine output`. With `SCIENCE_SOURCES_VERBOSE=1` it also prints each one as `skipped: <text>`. The existing summary line keeps its exact format, because the protocol parses it.
5. **Catch keyword-free claims, with two nets.**
   - **Prose net.** A string is also a claim when it reads as a sentence: at least 3 whitespace-separated tokens containing a letter, and it ends (after trimming trailing quotes and brackets) in `.`, `!`, `?` or `…`. This catches all three break examples wherever they appear, plus any future sentence caption.
   - **Keyword additions.** Add to `KEYWORD`: `react`, `\brust`, `corro`, `acid`, `alkal`, `litmus`, `\bpH\b`, `oxidi`, `dissolv`, `evapor`, `condens`, `sublim`, and `ca(?:ught|tch(?:es)?) fire`. These catch short fragments with no sentence punctuation (labels, banners such as "The greenhouse caught fire").

   Against FPS 6d5463e, the new nets flag a few strings with no row today. Expected, but the builder confirms with verbose output: the two `Save the Greenhouse! Put out 3 fires ...` strings (main.rs; `You saved the greenhouse! Water, frost ...` there is already C-029); `'You saved the greenhouse!'`, `` `${state.extinguished} fires put out. Press R to play again.` `` and `'The greenhouse caught fire'` (hud.js); and `Without locking the mouse you can also click (or tap) a glowing target to cast at it.` (index.html). Each gets a `## Not claims` row with a reason, or a `## Claims` row if it states science. A reason of "game instruction" or "win/lose banner" is enough for the non-science ones.
   - **Cap on new flags.** If the new nets flag more than 12 FPS strings with no row (the list above is 6), the builder stops adding rows, tightens the prose net (not the tests) and records why in `round-1.md`. Bulk Not-claims rows would hide science rather than check it.
6. **Document the residual limit.**
   - Register header: one sentence saying that the lint flags strings with a temperature, a formula, a keyword or sentence shape, and that the review protocol covers the rest (link to the section below).
   - `docs/loop/science-review.md`: add a `## Lint limits` section that lists exactly what the lint cannot see after this item:
     - (a) fragments with no temperature, no formula, no keyword and no sentence punctuation;
     - (b) strings built at runtime from fragments (`concat!`, `+`, joined arrays, `format!` with substituted words);
     - (c) the two skip shapes (`{{"` JSON templates, `TAG key=` log lines). Those are console and bridge output, not shown to the player, but a skip-shaped string would hide prose;
     - (d) files outside `crates/bevy-client/src/**/*.rs` and `crates/bevy-client/web/**/*.{html,js}`;
     - (e) Rust raw strings and `\`-newline continuations, which fail loudly rather than silently.
   - Extend Procedure step 3 so the reviewer lists every string literal on the caption paths (`caption()`, `cast_caption()`, `formula()`, `game.say(`, `.textContent =`) and the skipped strings from the verbose output, and confirms each one is a register row (Claims or Not claims) or states no science. Any that states science with no row becomes an `X-` row with verdict `error`.

## Acceptance tests (each can fail)

Run from the repo root. Here "FPS" means a checkout of `origin/claude/fps-prototype` at 6d5463e, for example `git worktree add <scratch>/fps 6d5463e`, passed as `SCIENCE_SCAN_ROOT`.

1. **Green on main and on FPS.** `npm ci && npm run typecheck && npm test` passes on the branch with no client tree, and `SCIENCE_SCAN_ROOT=<fps> npm test` passes. On FPS the summary line still has the form `science-sources: <n> client claims, <n> sourced, <n> params, <n> error verdicts, <n> stale-file warnings`, with claims equal to sourced. It fails if a newly flagged FPS string has no row, or if the summary format changed.
2. **Skipped list is exact on FPS.** On FPS, `science-sources: 6 skipped as machine output` is printed. With `SCIENCE_SOURCES_VERBOSE=1` the 6 `skipped:` lines are exactly the strings at `crates/bevy-client/src/main.rs` lines 295 (`STARTUP`), 872 (`IMPACT`), 1142 and 1149 (the `{{"` JSON templates), 1194 (`FIRST_FRAME`) and 1218 (`STATS`). It fails if any other string is skipped, or if one of these is not.
3. **Fixture: narrowed skip.** A fixture in `test/science-sources.test.ts` asserts that these are not skipped and are flagged as claims: `"Water is H2O: {:?}"`, `"Ice melts at {:.0} °C"` and `"note: N2 boils {:?}"` (lower-case tag). These remain skipped: `"IMPACT obj={} {:?} -> {:?}"`, `"STATS fps={:.1} burning={}"` and `{{"i":{i},"burning":{}}}`. The old fixture "only {: and leading {{ strings are skipped" is replaced, not left contradicting the new rule.
4. **Fixture: keyword-free claims.** The claim-test fixture asserts `isClaim` is true for `Iron rusts in water.`, `Sodium reacts violently with water.`, `Acids turn litmus red; pH 3.`, `The greenhouse caught fire`, `Salt dissolves` and `Water evaporates`. It still asserts false for `Fire`, `ice wall`, `steam vent`, `Click to look around`, `Frost`, `Fires put out: 0 / 3`, `Greenhouse saved!`, `Loading…`, `Click to play` and `Paused`.
5. **Mutation: break examples now fail the lint.** On a scratch copy of FPS, each of the following makes `SCIENCE_SCAN_ROOT=<copy> npm test` fail with a message that names the string and the file. Revert each before the next.
   - Add `let _s = "Iron rusts in water.";` to a non-test fn in `chem.rs`.
   - Add `game.say("Sodium reacts violently with water.");` in `main.rs`.
   - Add `const z = 'Acids turn litmus red; pH 3.';` in `hud.js`.
   - Add `info!("Water is H2O: {:?}", 1);` in `main.rs`.

   The breaker reruns these. The builder records the four failure messages in the round notes.
6. **Foam/CO2 sources hold up.** For C-012, C-018 and C-029, the round notes give, for each factual sentence in the note (and for the claim itself), the source id, URL and a quoted passage the builder actually read. Each passage supports its sentence. Any sentence without such a passage has been deleted from the note. [S17] is no longer the only cited support for a statement it does not contain. The breaker fetches each new source. It fails if any quoted passage is not in the source, or if a remaining sentence has no passage.
7. **[S8] holds up.** The [S8] definition names a document and location (table, page or equation) where M0 = 28.9644 kg/kmol appears. The round notes quote it with the fetch date. The breaker fetches it. It fails if 28.9644 is not at that location, or if the document is SP-398 and the value is not in it.
8. **Limits are documented and true.** `docs/loop/science-review.md` has a `## Lint limits` section listing (a) to (e). The register header points to it. Step 3 names the five caption paths and the `skipped:` list. The breaker checks that the section is accurate. On scratch FPS, it adds one string per limit (a) to (c) that is still invisible, such as `let _f = "copper turns green";`, and confirms the lint stays green. Then it adds a string just outside each limit, such as `"Copper turns green."`, and confirms the lint fails. It fails if a listed limit is wrong in either direction. For (c), the invisible case is a skip-shaped string carrying prose (`"STATS fps=1 copper turns green"`); the outside case is the same prose with a leading lower-case tag or without the `key=` (`"STATS copper turns green."`).
9. **Scope.** `git diff --name-only origin/main...` lists only `content/science-sources.md`, `test/science-sources.test.ts`, `docs/loop/science-review.md` and files under `docs/loop/items/I-010/`. Nothing under `crates/` or `.github/`.

## Out of scope

- Any client change (`crates/**`), including rewording "CO2 foam" in chem.rs or main.rs. If the review escalates C-012, C-018 or C-004 to `error`, a separate "Fix C-0NN" product item does the wording change.
- `docs/loop/checks.md` and extending science-review triggers to `main.rs` and `web/**` (the protocol's "Known gap"). That stays a follow-up.
- Detecting runtime-built claims in the lint (data-flow analysis of `format!`, `concat!` or `+`). It stays reviewer step 3.
- Requiring a row for every player-visible string (full UI inventory). The prose net and keyword net are the bounded version.
- `content/*.json`, `crates/sim/**`, `web/` (2D), CI or `.github/**`.
- Running a scored fresh-model B6 review. That is the scorer's job after merge.

## Contract

none (single lane; no shared files, no cross-lane interface).

## Lanes

| lane | owns | builds |
|------|------|--------|
| main | `content/science-sources.md`, `test/science-sources.test.ts`, `docs/loop/science-review.md`, `docs/loop/items/I-010/lanes/main/**` | Smallest version 1 to 6. Fixtures for tests 3 and 4 are written first and fail on eb65a6d. The new skip and claim rules and the skipped-line output. New FPS Not-claims or Claims rows. C-012, C-018 and C-029 sources or trimmed notes, and the [S8] fix, with fetched passages quoted in `round-1.md`. The register header sentence. `## Lint limits` and the step-3 extension in the protocol. Records the outputs of tests 1, 2 and 5 in `round-1.md`. |


## Critic notes (round 1)
- Verdict: revise (minor). Right thing, one lane, three files, tests can fail.
- Added a cap on new FPS flags (12) so the prose net cannot be satisfied by bulk Not-claims rows.
- Made the limit (c) mutation in test 8 concrete.
