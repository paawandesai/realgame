# I-047 critique (Spec-critic)

**Verdict: revise (minor; edits applied in spec.md, marked "(critic)").** Read: target.md, bars.md, areas.md, spec.md only.

## Right and smallest fix?
Yes. The fix stays in hud.js, keeps the ±200 px clamp, and changes no Rust code or sensitivity. The other obvious fix, dropping every move over 200 px instead of clamping it, would be smaller but would eat real fast flicks from high-DPI mice. The spec's rule needs all three conditions at once (within 50 ms of an edge, |dx| >= 0.4 W, |dy| >= 0.4 H), which is narrower and the better choice. It fits the hard rule (mouse look plus click to cast) and B4 (a click that knocks the aim off the target is a control blocker).

## Can the acceptance tests fail on main?
- T2 is deterministic: on base a synthetic (-W/2, -H/2) is clamped to 200 px, a 10.9° turn, so the test fails. One caveat: if hud.js ignores untrusted events, T2 would pass on base for the wrong reason. The failing-first record (T3 must pass on base) catches that, because T3 also uses synthetic events.
- T1 depends on headless Chromium sending the bogus move from wherever the Puppeteer mouse is. The diagnosis shows client (0,0), so the delta is likely `(mouse - centre)` with the mouse never moved. If `relock()` clicks the canvas centre, the bogus delta could be about (0,0) and T1 would pass on base. The spec's "stop and report" covers this outcome, but the probe needed to measure it (added, see below). It also means the filter's signature may only fit Puppeteer's default (0,0) cursor.

## Could it drop a kid's genuine fast move?
Only a single event (or coalesced part) moving at least 512 x 288 px at 1280x720, on both axes, within 50 ms of a press, release or lock change. At 60-1000 Hz event rates that is over 30,000 px/s in one sample, which is not plausible for a 4-18-year-old on a trackpad or mouse. Even if it happened, the clamp already limits that event to 200 px (10.9°), so the worst-case loss is one sub-11° nudge. T3 (a large diagonal far from any edge) and T4 (a one-axis flick right after an edge) guard against both ways the filter could go wrong. Acceptable.

Remaining risk: the parent event of coalesced moves. Its movement can be the sum of fast parts, so testing "each part and the event itself" could drop a genuine fast sweep because its summed parent looks large. The spec now says to test the unit that is actually summed.

## Changes made (critic)
1. Fix step 2: the probe also records the bogus move's movement and client coordinates with the Puppeteer mouse at (0,0), at the centre and at (100,100). If the delta is not about (-W/2, -H/2) in all three, the lane stops and reports instead of tuning the rule.
2. Fix step 3: test the same unit that is summed (each coalesced part, or the event when it has none). Never drop a whole event because of its summed parent. Edge timing uses the parent's timeStamp.
3. T1: log the Puppeteer mouse position and every raw locked move in the window, so a pass or fail shows whether the bogus move happened.

No change to scope, lanes, thresholds, or T2-T5.
