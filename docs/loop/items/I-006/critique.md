# I-006 spec critique

**Verdict: revise** (changes applied to spec.md)

Right thing: B4, B5 and B8 (level 5) cannot be measured at all today, and I-007 and I-008 name the first panel run as evidence. The harness keeps personas to screenshot and visible HUD text, so B4 is not gamed by reading game state. The panel clock makes timings repeatable. One lane, no shared files, no client change: small enough. Tests 1-12 and 14 can fail and are deterministic. First-person rule respected (real keys, mouse, pointer lock).

Problems found:
1. B8 unreachable by construction: 150 turns at about 3 s observe plus a short act is about 11 panel minutes, below the 15 needed.
2. Scoring accepted any five results, for example five copies of one persona, or runs on different builds.
3. Test 13 needs a real model agent; with no way to spawn one, a builder could fake a pass.

## Changes applied
- Turn cap raised from 150 to 400 (20 panel minute cap kept), with the reason written in.
- Test 3: five results must carry the five distinct persona ids and one artifact_sha and query, otherwise pass:false (incomplete panel or mismatched runs).
- Test 13: if no haiku agent can be spawned, record as blocked in round notes; never pass on a scripted run. Lane done-when allows 13 to be blocked, named.
