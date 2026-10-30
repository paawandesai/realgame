# I-023: game clock and perf-tool fixes (split from I-015)

based_on: loop/I-015 cd27edc, with main 6435dbf merged

The spec is `../I-015/spec.md` Parts A, C and D plus step B2, unchanged, with step B1 (release opt-level 3) removed. See `../I-015/decision.md` for why.

Acceptance: I-015 tests 1-10 and 13-18 as written. Test 11 (relative CPU cut) and test 12 (TTI bar) move to I-024. Reviews: `../I-015/break-1.md`, `../I-015/break-2.md`.
