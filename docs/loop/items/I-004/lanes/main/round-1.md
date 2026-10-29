# I-004 lane main, round 1 (builder)

Branch `loop/I-004` from `origin/main` at `f6504c4`. Client strings checked against `origin/claude/fps-prototype` at exactly `6d5463e` (extracted with `git archive 6d5463e crates/bevy-client` into a scratch directory; no commit touches `crates/`).

## Files

- `test/science-sources.test.ts`: the source lint, with extractors (a Rust tokenizer, HTML text nodes, JS literals), the register parser, and inline extractor fixtures. `npm test` already picks it up, so CI is unchanged.
- `content/science-sources.md`: the register. It has 32 `C-` rows, 15 `P-` rows, 1 `## Not claims` row (N-1) and 19 sources (`[S0]` to `[S18]`).
- `docs/loop/science-review.md`: the reviewer protocol.
- This file.

## Order of work

1. I wrote the test file first and ran it with no register. It failed (`cannot read register`), and all the fixture tests passed.
2. I ran it against `6d5463e` with `SCIENCE_SOURCES_VERBOSE=1`. The inventory came out at 33 strings, and a mechanical diff against Appendix A (`grep '^claim: ' | LC_ALL=C sort`) showed it identical. With 0 sourced, it failed on 33 unsourced strings.
3. I verified the sources (log below), then wrote the register from them.
4. I wrote the protocol.

## Acceptance results

| test | result |
|---|---|
| 1 green on main, CI unchanged | `npm run typecheck && npm test`: 34 tests pass (15 from science-sources). Summary line: `science-sources: 0 client claims, 0 sourced, 15 params, 0 error verdicts, 34 stale-file warnings`. The diff touches only the four allowed paths. |
| 2 green on the slice, inventory exact | Exit 0. `science-sources: 32 client claims, 32 sourced, 15 params, 0 error verdicts, 0 stale-file warnings`. The 33 `claim:` lines equal Appendix A exactly (diff empty). None of the must-not-flag strings are flagged. |
| 3 unsourced claims | Copper 1085 °C, "boils at 90 °C" and `Dry \"fuel\"`: each exits 1, and each message has the normalized string plus `crates/bevy-client/src/chem.rs`. |
| 4 broken register | All 9 mutations (plus a second `[S0]` variant) exit 1 and name the row id. **Exception:** see Deviation 1. |
| 5 constants | `LN2_C = -195.0` gives `LN2_C: crates/bevy-client/src/chem.rs has -195 but register P-002 has -196`. With the row removed, or a new `X_C`, the message is `... has no row in ## Parameters`. The control cells `−196.0` (U+2212) and `-196.0` both pass. |
| 6 stale rows | On main: one warning line and no failure. With the Melting caption edited, it fails on `C-015 is stale` and on the new unsourced string. With the formula removed from `index.html` only, it fails on `C-001 is stale: crates/bevy-client/web/index.html ...`. |
| 7 fixtures | All inline, all pass: comments vs `//` inside a string, the char literals `'"'` and `'\\'`, `#[cfg(test)] mod`, the `{:`/`{{` skips vs `Dry \"fuel\"`, HTML text nodes, JS literals with `${...}` kept, normalization, non-claims, unsupported shapes failing loudly, and register-parser errors. |

The mutation runner I used for tests 3 to 6 (22 checks, all pass) is a throwaway script in the scratchpad. It copies the `6d5463e` tree, applies each change, runs the test file and checks the exit code and message substrings.

## Deviations and decisions for the critic

1. **Test 4, first bullet (delete the C-017 row).** Once the row is deleted, its id is in no file, so the message cannot name `C-017`. The lint fails with `unsourced claim in crates/bevy-client/src/chem.rs: "Hot water boils at 100 °C and turns into steam: H2O(l) → H2O(g)."`, which names the exact text and file. I rejected gap detection on ids: ids are retired when strings leave the client, so gaps are legitimate.
2. **The verbose inventory and the reporter.** Run without `--test-reporter`, Node's default reporter when stdout is not a TTY is TAP, and TAP prefixes test-file stdout with `# `. The `claim:` lines therefore start at column 0 only with `--test-reporter=spec` (which `npm test` uses) or after `sed 's/^# //'`. The protocol's lint command passes `--test-reporter=spec`.
3. **What the stale-file warning counts.** It counts (row, file) references whose file is missing: 34 on main (32 rows, 2 of them with two files).
4. **One check added beyond the spec.** When `chem.rs` exists, a `P-` row that pins a `<NAME>_C` constant that no longer exists also fails, mirroring the stale-row rule.
5. **Extractor extras that the slice needed or that are cheap.** A JS regex literal is skipped by a prev-token heuristic and holds no text. HTML entities are decoded, including `&minus;`. Rust nested block comments are handled. Rust raw strings, `\`-newline continuations and inline `<script>`/`<style>` bodies throw `UnsupportedSyntax`, which fails the test loudly.
6. **Parameters beyond Appendix B.** I added `P-010`: the `+600` °C step per Fire cast in `apply`, noted as `gameplay tuning`. Its code comment, "A flame is about 900 °C at its tip", is wrong (flame tips are about 300 to 600 °C per [S15]), but the player never sees it. The two outcome rows Appendix B grouped together are split into `P-008` (water clamps to 100) and `P-009` (Frost sets −196).

## Verdicts that differ from Appendix A

- **C-008** `CO2 is heavier than air: it smothers fire by pushing the oxygen away.` Appendix A says `correct`; I say **`simplified`**. Deciding passage, [S14] (EPA430-R-00-002): "Flame extinguishment by carbon dioxide is predominantly by a thermophysical mechanism", and the report calls dilution of the reacting species secondary. "Heavier than air" is right ([S5] 44.0095 vs [S8] 28.9644 g/mol), but displacing oxygen is not the main mechanism.

Every other C- and P- verdict matches Appendix A after checking the sources. Some numbers in the notes differ from the appendix because I used the cited source's value:
- C-005: [S4] gives ΔcH° = −890.7 kJ/mol (with H2O(l)).
- C-017: [S2] gives Tboil = 373.17 ± 0.04 K = 100.02 °C. The appendix had 99.97 °C.
- C-003/C-007 and the other −196 rows: [S1] gives Tboil = 77.34 K.
- C-026: 2 × 285.830 = +571.66 kJ per reaction as written, from [S3].

Two sourced-range checks the spec asked for:
- **`FLAME_C` = 800 °C** is inside [S15]'s measured range: flame tips about 300 to 600 °C, the continuous flame region about 900 °C, wildfire peaks about 927 °C, and burning rack-storage solids averaging 870 °C. Verdict `simplified`.
- **`ignition_c(DryHedge)` = 230 °C** is inside [S16]'s "Average ignition temperatures were species dependent, ranging from 227°C to 453°C" (leaves, flat-flame burner), near the low end. It is also below wood's about 250 °C minimum-flux value in [S6]. Verdict `simplified`. Caveat for the critic: [S16] measured moist live leaves, not dead hedge litter. Dry dead leaves should ignite at least as easily, so I kept `simplified`; a reviewer wanting a dead-litter source may escalate.

The rows most likely to be escalated to `error` by the fresh reviewer are C-004, C-012, C-018 and C-022 ("no oxygen" and "CO2 foam"), as the spec predicted.

## Source log (all checked 2026-10-29)

| source | how verified | what it supports |
|---|---|---|
| [S0] | n/a (design choice) | tuning and scenario rows |
| [S1] NIST WebBook N2 phase data | URL fetched | Tboil 77.34 K (Jacobsen et al. 1986), 77.4 K (Streng 1971) |
| [S2] NIST WebBook water phase data | URL fetched | Tboil 373.17 ± 0.04 K (average of 7). This page has no Tfus, ΔfusH or ΔvapH, so those come from [S10] |
| [S3] NIST WebBook water condensed-phase thermochemistry | URL fetched | ΔfH°liquid −285.830 ± 0.040 kJ/mol (CODATA, Cox, Wagman et al. 1984). Replaces the suggested CODATA book, whose content I could not read |
| [S4] NIST WebBook methane gas thermochemistry | URL fetched | ΔcH°gas −890.7 ± 0.4 kJ/mol (Pittam and Pilcher 1972) |
| [S5] NIST WebBook CO2 | URL fetched | molecular weight 44.0095 |
| [S6] Babrauskas 2002, J. Fire Prot. Eng. 12(3) 163–189 | bibliographic record and abstract confirmed in the BVB (Bavarian library network) catalogue, record cdi_proquest_miscellaneous_18658548 | about 250 °C at minimum flux (glowing); flaming piloted ignition at 300–365 °C. Replaces the suggested *Ignition Handbook*, whose content I could not read |
| [S7] OpenStax Chemistry 2e § 4.1 | URL fetched | `CH4 + 2O2 ⟶ CO2 + 2H2O` balanced; the balance criterion; state symbols; `2H2O ⟶ 2H2 + O2` |
| [S8] Minzner (ed.), NASA SP-398, 1976 | NTRS PDF fetched | M0 = 28.9644 kg/kmol (standard sea-level air). Replaces the suggested "IUPAC + US Standard Atmosphere" |
| [S9] Grant, Brenton, Drysdale 2000, Prog. Energy Combust. Sci. | record and abstract confirmed on data.zhenzhi.com (DOI 10.1016/S0360-1285(99)00012-X, pp. 79–130, University of Edinburgh). Volume 26 is from the journal citation; the record page did not print it | water extracts heat from flames and the fuel surface; steam reduces O2 |
| [S10] OpenStax Chemistry 2e § 10.3 | URL fetched | ΔfusH of ice 6.0 kJ/mol at 0 °C; ΔHvap 40.67 kJ/mol at bp; temperature constant during a change of state; normal bp at 1 atm |
| [S11] OpenStax Chemistry 2e § 5.1 | URL fetched | c(water) 4.184 J/g °C; "Water has a relatively high specific heat." |
| [S12] OpenStax Chemistry 2e § 9.2 | URL fetched | Avogadro's law: V ∝ n at constant T and P |
| [S13] OpenStax Chemistry 2e § 17.7 | URL fetched | `2H2O(l) ⟶ 2H2(g) + O2(g)`, nonspontaneous, driven by a cell voltage > 1.229 V; stoichiometric H2 and O2 |
| [S14] EPA430-R-00-002 (2000) | URL fetched | CO2 extinguishment mainly thermophysical, dilution secondary; O2 at 10 vol% makes fires "impossible"; fuel-specific limits about 12.5–16 %; "Carbon dioxide is heavier than air." |
| [S15] Babrauskas, "Temperatures in flames and fires" (rev. 2006) | URL fetched | flame tips 320–400 °C (300–600 °C for solids), continuous flame about 900 °C, wildfire peaks about 927 °C, rack-storage solids average 870 °C |
| [S16] Fletcher et al. 2007, Combust. Sci. Technol. | USDA Forest Service Treesearch record 30610 fetched | leaf ignition temperatures 227–453 °C, species-averaged |
| [S17] NIST news release, 18 Sep 2023 | URL fetched | foams "prevent it from reigniting by suppressing oxygen flow and fuel vapors" |
| [S18] Ohlemiller, NBSIR 84-2895 (1984) | NIST record and PDF (nvlpubs) fetched | smouldering is heterogeneous oxidation of the fuel surface; char is predominantly carbonaceous, and its oxidation is highly exothermic |

Suggested sources I dropped because I could not check their content:
- Drysdale, *An Introduction to Fire Dynamics*, 3rd ed. The record exists (MTSU catalogue, Wiley 2011, ISBN 9781119975465), but I could not read the passages, so no row cites it.
- The CODATA 1989 book and the *Ignition Handbook*. Rows that would have cited them use [S3], [S6] and [S15] instead.

I could not find an authoritative source saying that "CO2-filled chemical foam" is obsolete. The C-012/C-018 notes therefore rest on [S17] (how foam works) and [S14] (CO2 is discharged as a gas), and leave escalation to the reviewer.

## Follow-ups (backlog candidates)

- Reword C-004/C-022 ("no O2") to "less O2" or similar, and reconsider "CO2 foam" (C-012, C-018, C-029). Owner: I-001 or a later client item.
- Fix the `chem.rs` code comment "A flame is about 900 °C at its tip" (not shown to the player).
- Extend the `checks.md` level-2 science triggers to `crates/bevy-client/src/main.rs` and `crates/bevy-client/web/**`.
- When I-001 merges, re-run test 2's command against main, taking the expected set from the register.
