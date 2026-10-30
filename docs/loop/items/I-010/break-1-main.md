# I-010 break 1 (main lane)

Verdict: no blocking findings.

Checks run on loop/I-010 @ 3727bda
- npm ci, typecheck, npm test: green (34 pass). With SCIENCE_SCAN_ROOT=FPS 6d5463e: green, `32 client claims, 32 sourced, 15 params, 0 error verdicts, 0 stale-file warnings`, `6 skipped as machine output`; the 6 skipped: lines are STARTUP, IMPACT, two `{{"` templates, FIRST_FRAME, STATS (as spec test 2).
- Mutations (test 5) all fail naming string and file: chem.rs "Iron rusts in water.", main.rs game.say sodium, hud.js acid/pH, main.rs info! "Water is H2O: {:?}".
- Limits (test 8): invisible and still green: "copper turns green" (a), "STATS fps=1 copper turns green" (c). Outside and failing: "Copper turns green.", "STATS copper turns green.". JS log-line shaped string with a claim is flagged (skips are Rust-only, as documented). Doc limits (a) to (c) accurate.
- Sources fetched: S19 (FAA AC 150/5210-6E) quotes at 1.2, 1.4.2, App C present (App C text has "vaporsuppressing" with no hyphen, a PDF line-break artefact). S20 (EPA UNDS AFFF NOD, April 1999, 2.1) quote present. S8 (SP-398) contains 28.9644 in Symbols list, Lower Boundary Conditions text and list. S14 says CO2 "is a gas under ambient conditions".

Should-fix (non-blocking)
1. C-012/C-018 say a CO2 agent "discharges carbon dioxide ... [S14]"; S14 supports only "a gas under ambient conditions" and self-pressurized discharge (it describes liquid discharge from nozzles), not "discharges gas". [S19] 1.4.2 supports the discharge claim better; cite S19 for that clause or reword.
2. S8 text quotes the Symbols entry as "The standard, sea-level value of M"; in the source it is the M0 entry. Cosmetic.
3. Residual (documented, not a defect): skip-shaped prose with a period, e.g. `STATS fps=1 copper turns green.`, is invisible; limit (c) says so.
