# I-022 web lane, round 1

Branch `loop/I-022`, based on origin/main a0e2ecb. Commits: 54d58af (tests 4, 6, 7-10, 12-14, failing), c49e9a0 (Hints gate in `setHint`, one id per session, players from `session_start`, picker frees the lock and makes the game inert).

## What changed

- `hud.js`: `hintAllowed(text)` beside `setHint`; `setHint` writes `hintAllowed(text) ? text : ''`. Nothing in `frame()` changed.
- `analytics.js`: `send(event, id = dailyId(...))`; `start` reads the id once and passes `send: (e) => send(e, id)`.
- `functions/_lib/analytics.ts`: `players` in the `days` and `dayBands` queries is `COUNT(DISTINCT CASE WHEN type = 'session_start' THEN daily_id END)` (a band with only later events gives a 0 row).
- `age.js`: on the shown path only, `exitPointerLock` if locked, `inert` on `#stage`, `#hud`, `#loading`, `#pause` (previous values kept), a `pointerlockchange` listener that frees a late lock; on pick the listener is removed and `inert` restored before the canvas is focused.
- `docs/ANALYTICS.md`: the one sentence from the spec.

## Node unit tests (tests 4-6)

| | a0e2ecb (new test files copied in) | loop/I-022 |
|---|---|---|
| `npm test` | 50 pass, 2 fail: "send uses the id it is given", "a session across UTC midnight is one player" | 52 pass, 0 fail |
| `npm run typecheck` | - | clean |

Test 6 numbers on loop/I-022: `players.today` 0, `daily[2026-10-29].players` 1, `playerDays` 1, seedling band row for 2026-10-30 `players: 0`. The existing "stats aggregate ..." test passes unedited.

## Golden (tests 7-11), `QUERY=?low`, run alone under the Chromium lock

New checks on loop/I-022 (identical in both runs):

| check | result | measured |
|---|---|---|
| Hints off hides the locked hint, on brings it back | PASS | before "Aim at a fire and click.", off "", on "Aim at a fire and click." |
| Hints off keeps the start line and the point-and-click note | PASS | start line and point-mode note both shown with Hints off |
| Hints is ticked and saved again afterwards | PASS | `hints: true` saved |
| Aim help off turns off the tap and hover cone | PASS | hover 5 -> -1 -> 5 (bush 5, near miss found) |
| An exact point-and-click hit still works with Aim help off | PASS | hover 0 = `lit[0]` |
| View width is 90 deg wide at 4:3 and 21:9, live | PASS | 4:3 fov 73.74 (want 73.74), 21:9 fov 46.40 (want 46.40) |
| View width 100 deg holds at 4:3, label says 100 deg | PASS | fov 83.58 (want 83.58), label `100°` |
| The cursor ray matches the view at 4:3 | PASS | hover 0 = `lit[0]` at 960x720 |
| The view returns to 58.72 deg at the default size | PASS | 58.72 |

Whole suite (107 checks = 98 existing + 9 new):

| build | run | passed | failed checks |
|---|---|---|---|
| loop/I-022 | 1 | 105/107 | "clock: cast cadence ..." (casts 10, need 11, 19 frames); "cast: a real click casts exactly once" (fire still burning 20 frames on) |
| loop/I-022 | 2 | 105/107 | "clock: cast cadence ..." (casts 9, need 10, 18 frames); "cast: a real click casts exactly once" |
| a0e2ecb base (`I-022/base-pkg-out`), same machine, same hour | 1 | 97/98 | "cast: a real click casts exactly once" (same output: casts 1 -> 2 -> 2, lit[1] burning=true) |

Both failures are I-019 checks on the locked cast path, which this item does not touch (locked casts use `game.aim`; the tap cone and the aspect only feed hover and `tap`). The machine ran at about 2-3 fps in these runs (18-19 frames in the 6 s cadence window). At that rate the 0.3 s cooldown spans two frames, so the cadence check lands at 9 or 10 casts against a need of 10 depending on phase: the base run got exactly 10/10. The real-click check fails on the base build too. So the "3/3 green" bar for test 11 is **not met in this session**: it needs a re-run when the machine renders faster (another session's main run at about 16:25 had 98/98).

## Analytics e2e (tests 12-15), `npm run build:site` then `SKIP_BUILD=1 node tests/e2e/analytics-e2e.mjs`, under the Chromium lock

**59/59 checks passed** (48 existing + 11 new). The playwright version is 1.63.0, so `page.clock` is available.

| check | result | measured |
|---|---|---|
| midnight: session_start and session_end carry the same id | PASS | both `0c56fb40...3426b` |
| midnight: session_end still carries a valid duration | PASS | `lt1m` |
| picker: the canvas takes the lock while age.js is held back | PASS | canvas clear, locked, picker still hidden, 60 s cap not hit |
| picker: it frees the lock within 1 s of appearing | PASS | `pointerLockElement === null` |
| picker: #stage, #hud, #loading, #pause inert while open; #age not | PASS | all four `true`; `#age.inert` false, no inert ancestor |
| picker: a pick closes it and sends session_start (sprout) | PASS | |
| picker: after the pick nothing is inert and the canvas has focus | PASS | all four `false`, `activeElement` = `game` |
| plus a second wrangler start, the playwright version check, and 0 console/page errors in both runs | PASS | |

## Lane hygiene (test 17)

`git diff --name-only origin/main...` lists only lane paths and `docs/loop/items/I-022/**`. `golden.mjs` +209/-0, `analytics-e2e.mjs` +137/-0.
