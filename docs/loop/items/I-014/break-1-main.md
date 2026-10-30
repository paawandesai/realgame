# I-014 break-1 (main breaker)

Built web release from loop/I-014 (cd912f3); `node --test tests/e2e/panel/` 26/26; selftest 47/47 (qa cpu_bench 110.9 ms vs 9yo 33.3 ms, ratio 3.3); manual probes below.

Original I-006 findings: 1 (barrier) fixed against accident, not against intent (documented); 2 (idle B8) fixed for waits and no-change input but a new hole remains (should-fix 1); 3 fixed; 4 fixed (ratio verified in real run); 5 fixed; 6 fixed by documenting the caption rule; 7 fixed (stub negative control); nits fixed.

## Blocking
None.

## Should-fix
1. **B8 is still gameable with toggle input.** `changed()` counts a pause toggle, a spell change or a pointer-lock toggle as engagement and the act then gets its full input time. Probe (measure.mjs on a synthetic log, 90 x `key Escape 10000`, paused flipping each act): `minutes_played` 15, `engaged_acts` 90, so B8 passes (15 min and 90 >= 30). Same with `key 1 10000` alternating spells. Real game confirmed: `key Escape 300` flips `truth.paused` each time (probe: 3 engaged acts of 3), and a paused game does not set `stalled`. U5 only tests `Shift`, which changes nothing. Fix: credit toggle-type changes (pause, lock, spell, outcome) a small fixed amount (for example 0.5 s) instead of the full hold time, or only count movement/look/cast change at full time; and cap credit per act by a plausible value for the state change.
2. **`engaged_acts >= 2 * minutes_played` is a weak floor.** It needs 30 engaged acts for 15 minutes, which is one engaged act per 30 s. With the full-hold credit above, 2 acts per minute of 10 s holds is not "play". Tighten (for example 6 per minute) or tie to the fixes above.

## Nit
- The barrier is by convention against a determined persona: the operator dir default is the guessable sibling `<run>.operator`, its path is on the daemon command line (`ps`, `/proc/<pid>/cmdline`) and the token file is the same user's. panel.md says so honestly; the run recipe puts the operator dir in the repo, which is fine. A persona that reads `ps` is only caught by transcript review.
- `cpu_bench_ms` ratio margin: 3.3 observed vs threshold 2 (fastest-of-7 on a blank page). A loaded box running the five sessions at once could inflate 9yo's bench or shrink the ratio; B4 would then fail on `throttle_ok:false` (a safe failure, but flaky). Consider logging both benches in `score` output when failing.
- `report` is not operator-gated; a persona that knows the operator dir could write result.json. Same convention-level barrier.
