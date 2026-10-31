# I-027 web lane, round 1

Commits: 798caa0 (progress.js bodies, `test/bevy-progress.test.ts`, golden `spellProgress`).
Paths: `crates/bevy-client/web/progress.js`, `test/bevy-progress.test.ts`, `tests/e2e/golden.mjs`
(`spellProgress` at the end of the file and its call line right after `reviewFollowups(...)`,
before `no console errors`). `boot.js`, `build-web.sh` unchanged beyond the contract stub.

## Node tests (12-15, 15b)

On the stub the file fails to load (`progress.js` does not provide `createSaver`), so all fail.
With the bodies: `npm test` 73/73 pass; the new ones:

| test | stub | bodies |
|---|---|---|
| constants (STORE_KEY, MAX_SAVE, SAVE_RE rejects `-1` and `;`) | fail | pass |
| 12 localDay (UTC, UTC+2, UTC-7) | fail | pass |
| 13 readSave (valid, missing, garbage, 513 chars, throwing getItem; never rewrites) | fail | pass |
| 14 createSaver writes exactly twice for revs 0,0,1,1,3 | fail | pass |
| 14 throwing setItem / getState, null state, bad progSave: no write, no throw | fail | pass |
| 15b SPELL_IDS line == spells.json ids in slot order | fail (import) | pass |
| 15 startProgress pushes `prog day 20756`, `prog load p1|water:1:10:0:0:0:0` | fail | pass |
| 15 only `prog day ...` without a valid save | fail | pass |
| 15 never throws (no `__game`, `{}`, throwing storage) | fail | pass |
| 15 saves from `__gameState` (JSON string) on poll and pagehide | fail | pass |

`npx tsc -p tsconfig.json`: clean (JSDoc types on progress.js).

## Golden (QUERY=?low)

| check (spec test) | on the stub | final |
|---|---|---|
| 16 fresh shape, keys in Contract order | PASS | PASS |
| 17 `prog learn 1` saved within 2 s | FAIL | PASS (605 ms) |
| 18 survives R and reload, progRev 0 after load | FAIL | PASS |
| 19 path to mastery by prog commands, kept on reload | FAIL | PASS |
| 23a water mastered: select 3 + cast casts Frost | FAIL | PASS |
| 23b all unknown: select 3 + cast casts Frost | PASS | PASS |
| 22 corrupt save boots fresh, kept until first change | FAIL | PASS |
| 25a progress.js served == web/progress.js | PASS | PASS |
| end: save removed, fresh on reload | PASS | PASS |
| 20 refused without ?debug (text match, no error) | FAIL | PASS |
| 21 blocked storage, 0 errors | FAIL | PASS |
| 25b 404 progress.js still boots and casts; only the resource-load error | PASS | PASS |
| whole run | 130/139 | **139/139, EXIT 0** |

Stub-run failures outside this lane: `streams: fireInView is a boolean, the last state key`
(fixed in the contract lane, see contract/round-1.md) and `cast: a real click casts exactly once`
(known box-load flake on main; passed in the final run). Final run: none of the three known
wall-time flakes failed. Only one final run was done (the 3/3 of test 24 is for the shared build;
the Chromium lock was held by I-046 for about two hours of this session).

Final golden progress lines:

```
PASS clock: state JSON keeps the base keys in order, appends gameTime and dt, and parses every frame keys ok=true (n=52, last=progress,progRev,progSave) objs ok=true parsed=202 errors=0 key changes=0
PASS streams: fireInView is a boolean, the last state key fireInView=true (boolean) last keys fxWater,fireInView,progress,progRev,progSave
PASS progress: fresh state has 4 unknown spells, progRev 0, progSave p1, keys in order fire:unknown:0,water:unknown:0,frost:unknown:0,co2:unknown:0 rev=0 save=p1 tail=progress,progRev,progSave
PASS progress: prog learn 1 is saved within 2 s fire:unknown:0,water:learning:10,frost:unknown:0,co2:unknown:0 rev=1 save=p1|water:1:10:0:0:0:0 stored=p1|water:1:10:0:0:0:0 after 605 ms
PASS progress: survives reset and a reload (progRev 0 after load) after R fire:unknown:0,water:learning:10,frost:unknown:0,co2:unknown:0 rev=1 save=p1|water:1:10:0:0:0:0; after reload fire:unknown:0,water:learning:10,frost:unknown:0,co2:unknown:0 rev=0 save=p1
PASS progress: worked casts, 3 builds over 2 days and a prediction reach mastered, kept on reload prog worked 1 1 -> learning:20; prog worked 1 1 -> practised:30; prog build 1 -> practised:50; prog build 1 -> practised:70; prog build 1 -> practised:90; prog pr
PASS progress: with water mastered, select 3 + cast still casts Frost water=mastered casts +1 spell=2
PASS progress: with all spells unknown, select 3 + cast still casts Frost fire:unknown:0,water:unknown:0,frost:unknown:0,co2:unknown:0 rev=0 save=p1 casts +1 spell=2
PASS progress: a corrupt save boots fresh, is kept until the first change, then replaced fire:unknown:0,water:unknown:0,frost:unknown:0,co2:unknown:0 rev=0 save=p1 stored before=garbage after=p1|fire:1:10:0:0:0:0 new errors=
PASS progress: progress.js is served and matches web/progress.js served 3187 bytes, source 3187 bytes
PASS progress: the save is removed at the end (fresh on reload) fire:unknown:0,water:unknown:0,frost:unknown:0,co2:unknown:0 rev=0 save=p1
PASS progress: prog learn is refused without ?debug (needs ?debug, not an error) frost=unknown warned=true error msgs with prog=0
PASS progress: blocked storage boots, progress works in the page, no errors water=learning errors=
PASS progress: a missing progress.js does not stop the boot booted=true casts +1 spell=2 pageerrors=0 console errors=1 (other: )
139/139 passed
```

## 25a (shipping)

- `pkg-out/progress.js` sha256 6b469bef... == `web/progress.js` (after `build-web.sh`).
- `scripts/build-site.sh` step 1 (`cp -R pkg-out/. dist/`) simulated into a temp dir: `progress.js`
  identical. The full `build-site.sh` (needs wasm-pack for the classic game) is left to the shared build.

## Lane hygiene (test 25)

`git diff --name-only origin/main...HEAD`: `crates/bevy-client/build-web.sh`, `src/main.rs`,
`src/progress.rs`, `web/boot.js`, `web/progress.js`, `docs/loop/items/I-027/**`,
`test/bevy-progress.test.ts`, `tests/e2e/golden.mjs`. No forbidden path. `build-web.sh` changes
line 21 only. golden deletions: the two key-order lines and the fireInView last-key line
(contract deviation, documented).

## Privacy

progress.js makes no network request; it only reads/writes `localStorage['rg.progress.v1']`
and pushes `prog day` / `prog load` commands. No analytics change.
