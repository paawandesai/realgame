# I-010 spec critique

**Verdict: revise (minor), applied to spec.md.**

- Right thing: yes. B6 measurement hardening, no client change, no drift from the first-person rule.
- Smallest: yes. One lane, three files. Each of the six steps maps to a break finding or an acceptance test.
- Tests can fail: yes. Fixtures, an exact skipped list, four mutations and a scope check are all falsifiable.

## Changes
1. Added a cap: more than 12 newly flagged FPS strings with no row means stop, tighten the prose net and record why. Without it, bulk Not-claims rows could satisfy the lint and hide science.
2. Test 8 limit (c) mutation made concrete (inside and outside strings), so the breaker cannot choose a vacuous one.

Noted, not changed: test 2 line numbers are tied to FPS 6d5463e, which the spec already pins.
