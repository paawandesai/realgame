# I-005 spec critique

Verdict: REVISE (applied to spec.md; no further critic round needed unless the planner disagrees with R1).

Checked against target.md (first-person 3D hard rule, human-only list), bars.md B7, areas.md ownership, gameplay-standards P0/P1, docs/ANALYTICS.md. The item is the right thing: B7 needs the 3D client sending events, and the picker is the only way to get an age band. Scope is one lane and the shared `build-web.sh` edit is called out. Tests can fail: seven named mutations, each tied to a check.

## Changes applied

- R1 (human-only data rule). The spec added a new `quest` column and told itself B7 authorized it. target.md makes "any change to what data is collected" human-only, and ANALYTICS.md asks for a purpose and deletion date per new field. Smaller and safer: keep `quest` on the wire, store it in the existing `room` column, add no column. The migration still rebuilds the table for the CHECK type list. Updated: data class, migration, ingest, stats, migration test, table_info assertion, added mutation (g).
- R2 (bug). The picker could be clicked before the WASM booted, when `window.__game` does not exist, so `reset` would throw or be lost. Reset is now queued to the first state.
- R3 (GS). Picker is modal (above HUD and pause card, so Esc or keys do not act on the game behind it), focuses its first button for GS-13, and scales to 200% for GS-18.
- R4 (flaky test). The keyboard run's close can send `session_end`, which clashed with "no other rows". Now allowed (at most 1). The "dur matches wall-clock" check is dropped; a sub-minute run is always `lt1m`. Added a fallback to force a hidden visibility event if headless close skips `pagehide`.

## GS walk (screens touched: the picker)
GS-6, 13, 18, 19, 21, 22, 23, 25, 27, 28 met as written. GS-14 deferred by design with a sound reason (shared Chromebooks). Others untouched.

## Notes for builder and breaker
- Picker shows on every page load, so a reload re-asks and resets progress. Accepted; the breaker should confirm it reads fine to a 6-year-old (a picture per band).
- Privacy notice wording and SECURITY.md stay human-only; flag in round notes.
