# I-005 lane main, round 1 (builder)

Branch `loop/I-005`, from `origin/main` a94fc48. Bar moved: B7 (deploy-ready with analytics). Everything below was measured in this worktree on 2026-10-30. Headless Chromium used a software GPU.

## What was built

- **Server.** `quest_complete {band, quest}`, with `quest` an integer from 0 to 15 (`MAX_QUEST`). It is stored in the existing `room` column, so there is no new column and no new kind of data (R1). The classic `room_complete` and `trial_complete` still work. `handleStats` and `shapeStats` add `funnel.quests` (dense from 0, the same way as `rooms`), and `rooms` counts only `room_complete`.
- **Migration** `migrations/0002_quest_complete.sql`. It rebuilds `events` with the same columns, CHECKs and STRICT, adds `'quest_complete'` to the type list, copies the rows, renames the table and re-creates `events_day_id`. 0001 is not edited.
- **Dashboard.** The funnel is: started, each "Quest N complete", then the classic room and trial rows, but only when they are non-zero. A new KPI shows "Quests completed, 30 days". The existing trials KPI is relabelled "Classic trials finished". There is still no inline script or style.
- **Client** `crates/bevy-client/web/analytics.js`. It is a port of the privacy core: `optedOut`, `dailyId`, `durationBucket`, `eventBody` (exactly the allowed keys, otherwise null), `questEvents`, `createSession`, `analyticsOn`, `send` and `start`. It touches no browser globals at import time. It polls `window.__game.state()` every 500 ms. `hud.js` and `src/` are not edited.
- **Picker** `crates/bevy-client/web/age.js`, with markup and inline SVG in `index.html` (hidden by default). It has three `<button>`s: "4 to 7" → `sprout`, "8 to 10" → `seedling`, "11 and up" → `apprentice`. Each has a low-poly figure that differs in size and shape (small sprout-headed figure; medium figure with two leaves; tall figure with a pointed hat and wand), and an aria-label. There is nothing to type into. The picker is modal: it focuses its first button, traps focus (the game focuses its canvas when it finishes booting), and Tab and the arrow keys move between the buttons. Keys never reach the game or HUD while it is open. A pick queues one `reset` until the first state arrives (R2), then focuses the canvas. The choice is not stored, and there is no timer. **Styling approach: CSSOM property assignment from `age.js`** (no `age.css`, no `style.css` change).
- **Switch.** The source `index.html` ships `<meta name="rg-analytics" content="off">`. `build-site.sh` flips it to `on` in `web/dist` only, and fails unless exactly one flip happens.
- **`npm run build:site`.** The 3D client goes at the root, the classic game at `/classic/` (via `vite build --base=/classic/ --outDir <tmp>`; `web/` is unchanged), `_headers` at the root (the copy under `classic/` is removed), and the dashboard at `/dashboard/`. `cf-pages-build.sh` installs `wasm-bindgen-cli` 0.2.129.
- **`tests/e2e/analytics-e2e.mjs`** and **`docs/ANALYTICS.md`** follow spec §5 and §6.

### Deviation from spec §4: `build-web.sh` is NOT changed

The workflow's scope list for this lane does not include `crates/bevy-client/build-web.sh`, so the change was made in a way that does not need it. `boot.js` loads `age.js` (which imports `analytics.js`) with a dynamic `import()`, and only when the `rg-analytics` switch is `on`. `build-site.sh` copies `analytics.js` and `age.js` into `web/dist` itself.

As a result, `pkg-out` never references the two modules. Golden, perf and the panel cannot hit a missing module or a 404. The off-site check below confirms that neither file is ever fetched from `pkg-out`. `git diff origin/main -- crates/bevy-client/build-web.sh` is empty.

The import sits after `startHud()` inside `try` with `.catch`, so a failure cannot stop boot. `init()` is not gated on it.

### Deviation: picker heading

The heading is **"Pick your age"** instead of the spec's example "How old are you?". The existing science-claims lint (`test/science-sources.test.ts`) treats any three-word sentence ending in `?` in client HTML as a claim that needs a row in `content/science-sources.md`, and that file is outside this lane's scope. "Pick your age" is one short line with no end punctuation (GS-21), so the lint was not weakened.

## Test-first

The new tests are `test/quest.test.ts` (validateEvent, migration 0002, handlers, shapeStats) and `test/bevy-analytics.test.ts` (the client's pure functions). On a94fc48 they failed before any implementation:

```
Error [ERR_MODULE_NOT_FOUND]: Cannot find module '.../crates/bevy-client/web/analytics.js'
SyntaxError: The requested module '../functions/_lib/analytics.ts' does not provide an export named 'MAX_QUEST'
✖ test/bevy-analytics.test.ts   ✖ test/quest.test.ts
```

`git diff origin/main -- test/` contains only the two new files. No existing test was edited.

## Results

### 1. Unit and type checks

```
npm test        → ℹ tests 50  ℹ pass 50  ℹ fail 0
npm run typecheck (tsc -p tsconfig.json, includes analytics.js through the test import) → clean
age.js: tsc --allowJs --checkJs --strict (standalone) → clean
```

### Crate checks (crates/bevy-client)

```
cargo fmt --check                                                        → ok
cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings     → Finished, 0 warnings
rustc --edition 2021 --test src/chem.rs && run                           → 14 passed; 0 failed
cargo test --locked                                                      → 14 passed; 0 failed
```

### 2. Build

`CARGO_TARGET_DIR=/home/claude/bevy-target PATH=/home/claude/bevy-target/tools/bin:… npm run build:site` exited 0. Output of `lanes/main/build-checks.sh`:

```
wasm gzip bytes: 5035656
build-site: web/dist is ready (18M): 3D client at /, classic game at /classic/, dashboard at /dashboard/
canvas: 1
switch on: 1
web/dist/{_headers,age.js,analytics.js,bevy-client.js,bevy-client_bg.wasm,boot.js,classic/index.html,dashboard/index.html,hud.js} all present
connect-src self in _headers: 1
source switch off: 1
pkg-out switch off: 1
classic refs: src="/classic/assets/index-CEClhHPn.js" href="/classic/assets/index-D4gTiRfD.css"
urls in root files (index.html, boot.js, analytics.js, age.js): none
classic/_headers present: no
```

### 3. Classic still reachable (inside analytics-e2e, on `wrangler pages dev web/dist`)

```
PASS GET /classic/ serves the 2D page and its assets [["/classic/assets/index-CEClhHPn.js",200],["/classic/assets/index-D4gTiRfD.css",200]]
PASS the classic game's WASM is served under /classic/ ["/classic/assets/realgame_sim_bg-Cas2zIK-.wasm"]
```

### 4. analytics-e2e

The command was `SKIP_BUILD=1 QUERY=?low node tests/e2e/analytics-e2e.mjs`. It exited 0 with **48/48 checks passed**. The full PASS list covers:

- the picker: shown, 3 buttons each 168×189 px, nothing to type, focus on the first button, closes after a pick;
- the game: fresh start after the pick, then won;
- 0 console errors and 0 page errors;
- keyboard: focus kept while the game boots, arrow and Tab reach "11 and up", Enter picks;
- GPC and DNT: no picker and 0 requests in 10 s;
- wire shape, single id per run and same origin, for all 4 runs;
- `/api/stats`: 401 without the token, then today=2, seedling=1, apprentice=1, started.seedling=1, quests[0].seedling=1, durations ≥ 1;
- D1: main 1 start, 1 quest at room 0, 1 or more ends; keyboard 1 start, 0 quests, at most 1 end; probe quest 7 stored as 7; no other ids; columns exactly `id, day, daily_id, type, band, room, dur`.

**The test adds one check beyond spec §5.** The game has only quest 0, so mutation (g) ("store quest as 0") is invisible to a real play-through. The script therefore POSTs `quest_complete {quest: 7}` under a random probe id. It asserts 204, then 400 for quest 16, then that D1 stored room 7. The probe id is excluded from the "no other ids" check and is posted after the `/api/stats` assertions, so `players.today === 2` still holds.

**Mutations.** Each was applied to the served copy (or to `functions/_lib/analytics.ts`) one at a time, then reverted. The script is `lanes/main/mutate.py`. Every mutation made the test exit 1:

| Mutation | Exit | Failing checks |
|---|---|---|
| (a) `quest_complete` send removed | 1 | `stats: funnel.quests[0].seedling === 1`; `D1 main: exactly 1 quest_complete (seedling, room 0)` |
| (b) `optedOut` always false | 1 | `GPC: no picker and no /api/event request…` (picker=true); `DNT: …` (picker=true) |
| (c) middle picture sends `sprout` | 1 | `stats: todayByBand seedling 1, apprentice 1`; `stats: funnel.started.seedling === 1`; `stats: funnel.quests[0].seedling === 1`; `D1 main: exactly 1 session_start (seedling)`; `D1 main: exactly 1 quest_complete…`; `D1 main: 1 or more session_end…` |
| (d) `session_end` removed | 1 | `stats: durations sum to 1 or more`; `D1 main: 1 or more session_end (seedling, valid bucket)` |
| (e) `ua: navigator.userAgent` added | 1 | `main run: 0 console errors` (400s); `wire (main)/(keyboard): every event body has exactly its allowed keys…`; all `stats:` and `D1 main/keyboard` row checks (the server rejected every event) |
| (f) `handleStats` drops `quests` | 1 | `stats: funnel.quests[0].seedling === 1` |
| (g) `handleEvent` stores quest as 0 | 1 | `D1 probe: quest 7 stored as room 7, quest 16 not stored` |

### 5. No analytics off-site (pkg-out on a plain static server)

The script is `lanes/main/offsite-check.mjs`. It serves `pkg-out` the same way golden does, loads `?low&debug` (`debug` only to win fast), resets, wins, forces a hidden event, and waits 20 s or more from the first state:

```
PASS won on pkg-out
PASS no picker on pkg-out
PASS no /api/event request within 20 s of first state (after reset and hide) []
PASS age.js/analytics.js never fetched []
PASS 0 console errors []
```

### 6. B1 golden path not regressed

`golden.mjs` is unchanged. The command was `QUERY=?low node tests/e2e/golden.mjs`, and it exited 0:

```
PASS no console errors
31/31 passed
```

### 7. Scope

`git diff --name-only origin/main...` lists:

- `crates/bevy-client/web/{index.html,boot.js,analytics.js,age.js}`
- `dashboard/{dashboard.js,index.html}`
- `docs/ANALYTICS.md`
- `functions/_lib/analytics.ts`
- `migrations/0002_quest_complete.sql`
- `scripts/{build-site.sh,cf-pages-build.sh}`
- `test/{quest,bevy-analytics}.test.ts`
- `tests/e2e/analytics-e2e.mjs`
- `docs/loop/items/I-005/**`

Nothing under `crates/bevy-client/src/`, `hud.js`, `style.css`, `build-web.sh`, `web/`, `wrangler.toml`, `package.json` or `golden.mjs` changed.

## For Paawan (human-only)

- **Privacy notice wording.** In the suggested notice text in `docs/ANALYTICS.md`, "which rooms they finish" is now "which quests or rooms they finish". The legal review and publishing of the notice stay with you.
- **SECURITY.md follow-up** (not edited; out of scope). Its event list should name `quest_complete` (quest index 0 to 15, stored in the existing room column), the age picker (three pictures, not stored, never shown under DNT or GPC), and that the 3D client sends only from the built site (`rg-analytics` switch).
- **Deploy.** `npm run deploy` applies migration 0002 (a table rebuild) before the new Functions go live. The deploy itself is yours.

## Notes for the breaker

- The picker shows on every page load (not stored, by design), so a reload asks again and resets progress.
- The main run in e2e clicks the picker **before** the WASM has booted, which exercises the queued reset (R2). The keyboard run waits for boot first, which exercises the focus trap.
- Each e2e run takes about 4 to 5 minutes on the software GPU. The mutation script takes about 35 minutes for all seven.
