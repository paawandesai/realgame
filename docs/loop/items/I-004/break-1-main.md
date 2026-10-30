# I-004 break round 1 (lane main)

Branch loop/I-004 @ 124e933 vs main f6504c4. `npm ci; npm run typecheck && npm test`: green (34 tests). Diff touches only the 3 allowed files. Against FPS 6d5463e the lint is green (32 claims, 32 sourced, 15 params).

## Blocking
None found.

## Should-fix
1. C-012 / C-018 cite [S17] (NIST PFAS news release) for "firefighting foams are made with air; CO2 extinguishers discharge gas". The page only says foams "suppress oxygen flow and fuel vapors"; it says nothing about air-filled foam or CO2 extinguishers. The cited source supports the smothering half only. Add a source for the "CO2 foam is not a real agent" half (e.g. NFPA 11/12 or an EPA/NFPA foam reference), or drop that sentence.
2. Claim heuristic misses real science claims with no keyword: with FPS plus an added literal, "Iron rusts in water.", "Sodium reacts violently with water." and "Acids turn litmus red; pH 3." all pass the lint with no row. Spec-defined, but it caps "100% sourced" at "100% of keyword-matching strings". Suggest the protocol tell the reviewer to grep for all string literals in the apply/step caption paths; note the limit in the register header.
3. Any string containing `{:` anywhere is skipped as machine output ("Water is H2O: {:?}" passes unsourced). Spec-defined, but a cheap bypass. Consider skipping only when the string is purely a format spec, or log skipped strings in verbose output.

## Nit
- Source verification (fetched today): S2 water Tboil 373.17 +/- 0.04 K (avg of 7) OK. S1 N2 Tboil 77.34 K, Jacobsen 1986 OK. S4 methane dcH gas -890.7 +/- 0.4 Pittam and Pilcher 1972 OK. S16 leaf ignition 227-453 C, authors and CST pp.1183-1203 OK. S15 matches: tips 300-600 (avg 450), continuous flame ~900, wildfire peak 927, rack storage 870 OK; note it also says generic tip estimate is 320-400 C, so the P-010 remark "nearer 300 to 600" is fine. S18 NBSIR 84-2895 Ohlemiller OK. S6, S9, S14 and the OpenStax sections are real and well known, not re-fetched. S8 (NASA SP-398 for M0 = 28.9644) NOT re-fetched: that value is usually cited to the US Standard Atmosphere 1976 (NOAA-S/T 76-1562); SP-398 covers above 86 km. Builder should confirm the number appears in SP-398 or swap the source.
- P-003 FLAME_C 800 inside 300-900 range: simplified is fair. P-005 DryHedge 230 inside 227-453: OK, borderline at the low end of the measured range, correctly flagged simplified. P-004 300 OK.
- C-004 "no O2" is arguably an error to a teacher; register marks simplified and says reviewer may escalate. Acceptable per spec.
- Spec test 2 says 33 claim lines (C-001..032 + N-1); summary line reports 32 because N-1 is in Not claims (spec-conformant; verbose inventory includes it).

## Checked and holding (mutations on a scratch FPS copy)
- Added "Copper melts at 1085 C" in chem.rs: fails, names string and path.
- "boils at 90" edit: fails, names string and path.
- LN2_C -195: fails, reports -195 vs register P-002 -196. Added X_C: fails, "no row".
- Edited "Melting..." caption: fails for unsourced string and for stale C-015.
- Removed CH4 formula from index.html only: fails naming C-001 and index.html.
- Rust raw string and `\`-newline continuation: test fails loudly (unsupported syntax), as spec requires.
- concat! fragments: reassembled fragment "Wood bu" etc. still flagged via "°C" piece, so caught only by luck of keywords; the protocol says reviewer reads call sites.

## Protocol dry run
science-review.md has all 10 required sections, worked example (C-001), per-element and charge balancing, Known gap line, triggers matching checks.md, B6 rule verbatim. Not executed end to end by a fresh model in this round.
