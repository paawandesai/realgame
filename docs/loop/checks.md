# Checks

| Level | What it checks | Runs when | Command / protocol | Owner | Cost |
|-------|----------------|-----------|--------------------|-------|------|
| 0 Smoke | scripted core flows on the artifact | every lane commit; first each round | config `smoke` | tester | minutes |
| 1 Tests & CI | suite, lint | every change | config `test` | loop | minutes |

| 2 Perf | size, load, CPU frame ms (cloud); fps p5 (Mac via Remote Control) | client changes; nightly | config `perf` + Mac recipe | tester | ~15 min |
| 2 Science review | every in-game claim vs cited source | content/ or chem changes; nightly | science-review agent over `content/science-sources.md` | science reviewer (fresh model) | small |
| 1 Analytics e2e | events reach local D1 | functions/, client analytics changes | config `analytics-e2e` | tester | minutes |
| 5 Persona panel | 5 personas play the artifact | user-facing changes; nightly | I-006 harness | panel | medium |

## Path triggers
- every change -> levels 0 and 1
- `crates/bevy-client/**` -> level 2 perf, level 5 panel
- `content/**`, `crates/bevy-client/src/chem.rs`, `crates/sim/**` -> level 2 science review
- `functions/**`, `dashboard/**`, analytics code -> analytics e2e

## Last run
| Level | Last run | Result | Due next |
|-------|----------|--------|----------|
