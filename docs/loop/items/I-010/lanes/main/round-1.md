# I-010 lane main, round 1 (builder)

Branch `loop/I-010` from `origin/main` at `eb65a6d`. Client under test: `origin/claude/fps-prototype` at exactly `6d5463e`, extracted with `git archive 6d5463e` into a scratch directory and passed as `SCIENCE_SCAN_ROOT`. Nothing under `crates/` or `.github/` changes.

## Files

- `test/science-sources.test.ts`: the narrowed skip (Rust strings only), the prose net and the new keywords in `isClaim`, the `<n> skipped as machine output` line and the verbose `skipped:` lines, and the new fixtures.
- `content/science-sources.md`: the header sentence on what the lint sees; C-012, C-018 and C-029 re-sourced; [S8] given its location; [S19] and [S20] added; 6 new `## Not claims` rows.
- `docs/loop/science-review.md`: `## Lint limits` (a) to (e); Procedure step 3 extended to the five caption paths and the `skipped:` list; Inputs item 3 keeps the new lines.
- `docs/loop/items/I-010/spec.md`, `critique.md` (copied) and this file.

## Order of work

1. **Tests first.** I replaced the fixture "only {: and leading {{ strings are skipped" with "only {{\" JSON templates and TAG key= log lines are skipped" (test 3, plus the limit (c) inside/outside strings and a JS log line that must not be skipped). I extended "the claim test" (test 4, plus `copper turns green` as a must-not-flag and `Copper turns green.` / `(Copper turns green!)` as must-flag). On `eb65a6d` both failed:
   ```
   ✖ fixture: only {{" JSON templates and TAG key= log lines are skipped as machine output
   ✖ fixture: the claim test
   ```
2. I implemented the skip, the nets and the output lines. On FPS, the lint then flagged exactly the 6 strings the spec predicted (below), and skipped 7 strings, one more than test 2 allows (Deviation 1).
3. I verified the sources (log below) and edited the register.
4. I wrote the protocol changes and ran the mutations.

## Acceptance results

| test | result |
|---|---|
| 1 green on main and on FPS | No client tree: `npm ci --silent; npm run typecheck && npm test` exits 0, 34 tests, 34 pass. `science-sources: 0 skipped as machine output` and `science-sources: 0 client claims, 0 sourced, 15 params, 0 error verdicts, 34 stale-file warnings`. FPS: `SCIENCE_SCAN_ROOT=<fps> npm test` exits 0, 34 pass, `science-sources: 32 client claims, 32 sourced, 15 params, 0 error verdicts, 0 stale-file warnings` (same format, claims equal sourced). |
| 2 skipped list exact | `science-sources: 6 skipped as machine output`. Verbose `skipped:` lines, in order: `STARTUP t={:.0}ms` (main.rs 295), `IMPACT obj={} {:?} -> {:?}` (872), `{{"i":{i},"n":"{}",...` (1142), `{{"frame":{},"tick":{},...` (1149), `FIRST_FRAME t={:.0}ms` (1194), `STATS fps={:.1} fps_p50=...` (1218). Nothing else. |
| 3 narrowed skip fixture | Passes. `Water is H2O: {:?}`, `Ice melts at {:.0} °C`, `note: N2 boils {:?}` are not skipped and are claims; the IMPACT, STATS and `{{"i":...}}` strings are skipped. The old fixture is gone. |
| 4 keyword-free claims fixture | Passes, with all six must-flag and all ten must-not-flag strings from the spec. |
| 5 mutations | Each exits 1 (messages below). |
| 6 foam/CO2 sources | Passages below. |
| 7 [S8] | Passage below. |
| 8 limits | Run on scratch FPS (below). |
| 9 scope | `git diff --name-only origin/main...` lists only the four allowed paths and `docs/loop/items/I-010/**`. |

### Newly flagged FPS strings (6, under the cap of 12)

All six got `## Not claims` rows, none states science:

| string | file | net | row reason |
|---|---|---|---|
| `Save the Greenhouse! Put out 3 fires before they spread. Press 2 for Water.` | main.rs | sentence | game instruction |
| `Save the Greenhouse! Put out 3 fires before they spread.` | main.rs | sentence | game instruction |
| `You saved the greenhouse!` | hud.js | sentence | win banner |
| `${state.extinguished} fires put out. Press R to play again.` | hud.js | sentence | win banner subtitle |
| `The greenhouse caught fire` | hud.js | keyword `caught fire` | lose banner; the why is C-030 |
| `Without locking the mouse you can also click (or tap) a glowing target to cast at it.` | index.html | sentence | game instruction |

The prose net did not need tightening.

### Test 5 failure messages (scratch copy of FPS, each reverted before the next)

```
let _s = "Iron rusts in water.";   (chem.rs, inside pub fn name)
  exit=1  unsourced claim in crates/bevy-client/src/chem.rs: "Iron rusts in water."
game.say("Sodium reacts violently with water.");   (main.rs)
  exit=1  unsourced claim in crates/bevy-client/src/main.rs: "Sodium reacts violently with water."
const z = 'Acids turn litmus red; pH 3.';   (hud.js)
  exit=1  unsourced claim in crates/bevy-client/web/hud.js: "Acids turn litmus red; pH 3."
info!("Water is H2O: {:?}", 1);   (main.rs)
  exit=1  unsourced claim in crates/bevy-client/src/main.rs: "Water is H2O: {:?}"
```

### Test 8 dry run (limits, inside then outside)

```
(a) let _f = "copper turns green";                        exit=0 (invisible)
(a) let _f = "Copper turns green.";                       exit=1 unsourced ... chem.rs: "Copper turns green."
(b) game.say(&format!("{} turns {}", "copper", "green")); exit=0 (invisible)
(b) game.say(&format!("{} turns green.", "Copper"));      exit=0 (invisible: "{}" is not a word, so 2 words)
(c) info!("STATS fps=1 copper turns green");              exit=0 (invisible)
(c) info!("STATS copper turns green.");                   exit=1 unsourced ... main.rs: "STATS copper turns green."
(c) info!("stats fps=1 copper turns green.");             exit=1 unsourced ... main.rs: "stats fps=1 copper turns green."
```

For (b), the "outside" case is the same sentence written as one literal, which is (a)'s outside case. Lint limits (b) says so: only the literal pieces are judged, and a placeholder is not a word.

## Source log (fetched 2026-10-30)

### [S8] U.S. standard sea-level molar mass of air

I fetched NASA SP-398 (https://ntrs.nasa.gov/api/citations/19770003812/downloads/19770003812.pdf) twice and searched it for `28.9644`. It **does** contain the value, so [S8] keeps SP-398, with the location added (the spec allows this). The text is OCR'd, and the unit comes out as `kmol'1` / `kmol"1`. The occurrences:

- Symbols list (front matter): "The standard, sea-level value of M, 28.9644 kg kmol-1"
- Chapter "Lower Boundary Conditions" (Minzner): "the standard six-significant-figure value of the sea-level mean molecular weight, 28.9644 kg kmol-1"; and, in the list of boundary values next to the 86-km values (T = 186.8637 K, M = 28.9522 kg kmol-1): "M0 = 28.9644 kg kmol-1"
- "Definition of Model II", Molecular Nitrogen: "quite close to the mean molecular weight of air, 28.9644 kg kmol-1"

The fetch tool could not reliably give printed page numbers from the OCR (it suggested about p. viii for the Symbols list and pp. 7 to 8 for Lower Boundary Conditions, and flagged them as possibly off by one). So the [S8] entry names the sections, not pages. I also fetched the NTRS record for the *U.S. Standard Atmosphere, 1976* (https://ntrs.nasa.gov/citations/19770009539: NOAA-S/T-76-1562, NASA-TM-X-74335, 1 October 1976). The fetch tool found no `28.9644` in that PDF's OCR text, and said the OCR was "heavily garbled", so I do **not** cite it as the location, only as the standard the value defines. Direct `curl` of NTRS is blocked by the sandbox proxy (403), so I could not grep the raw PDF text myself. The breaker should re-fetch SP-398 and look in the Symbols list and Lower Boundary Conditions.

### C-012 `Fire out! The CO2 foam smothered it: no oxygen, no fire.`

Claim and note, sentence by sentence:

1. *A foam blanket puts fires out by separating the fuel vapour from the oxygen it needs.*
   - [S19] FAA AC 150/5210-6E § 1.2 (p. 1-1), https://www.faa.gov/documentLibrary/media/Advisory_Circular/AC-150-5210-6e-arff-ext-agents.pdf: "They extinguish fire by physically separating the fuel vapors from the heat and oxygen necessary for combustion."
   - [S17] NIST news release, https://www.nist.gov/news-events/news/2023/09/nist-researchers-develop-standards-help-eliminate-forever-chemicals: "the foams not only spread over the fire but prevent it from reigniting by suppressing oxygen flow and fuel vapors."
2. *"No oxygen" is the limiting case: the foam layer excludes oxygen from the fuel surface.*
   - [S20] EPA UNDS Appendix A, AFFF Nature of Discharge, § 2.1, https://www.epa.gov/sites/default/files/2015-08/documents/2007_07_10_oceans_regulatory_unds_tdddocuments_appaafffnod.pdf: "The foam layer effectively excludes oxygen from the surface of the fuel, while the high water content cools the surface."
   - The O2 limits behind "limiting case" are C-004's, [S14].
3. *Firefighting foam is an aggregation of small bubbles made by mixing air into a solution of foam concentrate and water.*
   - [S19] § 1.2: foams "are produced by incorporation of air into a solution of foam concentrate and water."
   - [S19] App. C item 9 (p. C-1), Foam: "An aggregation of small bubbles used to form an air-excluding, vaporsuppressing blanket over the surface" (the definition continues).
4. *A CO2 extinguishing agent discharges carbon dioxide, which is a gas at ambient conditions, and puts fire out by lowering the oxygen concentration.*
   - [S14] EPA430-R-00-002, p. 2, https://www.epa.gov/sites/production/files/2015-06/documents/co2report.pdf: "Carbon dioxide provides its own pressurization for discharge from a storage container"; "liquid carbon dioxide starts to discharge from the nozzle(s) in under 5 seconds"; "It provides three-dimensional protection because it is a gas under ambient conditions."
   - [S19] § 1.4.2 (p. 1-4): "Carbon Dioxide extinguishes fire by reducing the concentration of oxygen, the vapor phase of the fuel, or both" (the sentence continues).
5. *So the game's "CO2 foam" pairs two agents; the smothering the caption describes is how either one works.* This follows from 1, 3 and 4 ([S19] for both mechanisms).

Removed: "'CO2 foam' is not a common modern agent" and "the gas in the bubbles is not what does the work". No fetched source states either. The new note does not call CO2-generating foam fictional. (Britannica Students, "fire extinguisher", https://kids.britannica.com/students/article/fire-extinguisher/274323, describes foam extinguishers in which "Solutions of sodium bicarbonate and aluminum sulfate in separate compartments mix when the extinguisher is inverted." That is the historical chemical foam, but the passage does not say the bubbles are CO2, so I did not cite it.)

[S17] is no longer the only support for any sentence: every sentence that cites it also cites [S19] or [S20]. Verdict stays `simplified`.

### C-018 `CO2 foam covers it: no oxygen can reach it, so it cannot burn.`

Same sentences and passages as C-012: separation ([S19] § 1.2, [S17]), oxygen excluded from the fuel surface ([S20] § 2.1), foam made with air ([S19] § 1.2), CO2 a gas ([S14] p. 2). Removed: "'CO2 foam' is not a common modern agent". Verdict stays `simplified`.

### C-029 `You saved the greenhouse! Water, frost and CO2 foam each stop fire in their own way.`

- Foam stops fire: [S19] § 1.2 (quoted above), and [S17] (quoted above). [S19] was added.
- Frost, meaning liquid nitrogen, is far below ignition temperatures: [S1] NIST WebBook, nitrogen phase-change data, https://webbook.nist.gov/cgi/cbook.cgi?ID=C7727379&Mask=4: "Tboil | 77.34 | K | N/A | Jacobsen, Stewart, et al., 1986 | TRC". This is unchanged from I-004.
- Water: [S9], unchanged. Crossref (https://api.crossref.org/works/10.1016/S0360-1285(99)00012-X) confirms the record: G. Grant, J. Brenton, D. Drysdale, "Fire suppression by water sprays", Progress in Energy and Combustion Science 26(2), 79–130, April 2000. I could not get the abstract through Crossref, Semantic Scholar or ScienceDirect. The water rows' sourcing is outside this item and stays as I-004 left it.

## Deviations and decisions for the critic

1. **Skips apply to Rust strings only.** With the spec's two shapes applied to every file, FPS skipped 7 strings: the 6 in test 2, plus `CPUFRAME n=${s.length} p50_ms=${q(0.5)} p95_ms=${q(0.95)} max_ms=${q(1)}` from `crates/bevy-client/web/boot.js:23` (a `console.log`). Test 2 requires exactly the 6 main.rs strings. Both skip shapes are Rust format-string shapes (`{{` is Rust's escaped brace), so the skip now runs only on `.rs` strings. The JS line is kept, and it is not a claim (no keyword, not a sentence). A fixture asserts this (`extractFileAll("web/boot.js", ...)` skips nothing). Lint limits (c) says "applied to Rust strings only".
2. **The spec's sentence net is implemented as written.** "Tokens containing a letter" uses `\p{L}`, so `{}`, `${...}` and numbers are not words. Trailing characters trimmed before the punctuation check: ASCII and curly quotes, backtick, `)`, `]`, `}`, `>`, `»` and whitespace.
3. **Lint limits (e)** also names inline `<script>`/`<style>` bodies, which fail loudly in the same way. **Limit (d)** lists what falls outside the scan.
4. **[S8] keeps SP-398** (see the source log). The id and the cited statement are unchanged.
