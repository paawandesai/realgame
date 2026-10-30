# I-005: Anonymous analytics in the 3D client, and the 3D client as the site root

based_on: `origin/main` a94fc488a8f9d3cb3c9c685b7615ea84a5e3a1be

## Problem

Bar B7 fails today on three counts:

1. **The 3D client sends nothing.** The only analytics code is `web/src/analytics.ts` in the retired 2D game. `crates/bevy-client/web/` has no age picker and no event calls, so the dashboard cannot show 3D players or their age bands. Paawan's "done" definition in `target.md` depends on reading exactly those numbers.
2. **`npm run build:site` packages the wrong game.** `scripts/build-site.sh` builds the 2D Vite game into `web/dist` (the `pages_build_output_dir` in `wrangler.toml`), with the dashboard at `/dashboard/`. The 3D client in `crates/bevy-client/web/pkg-out` is not in the site.
3. **The server has no quest event.** `functions/_lib/analytics.ts`, `migrations/0001_events.sql` and the dashboard know `session_start`, `room_complete`, `trial_complete` and `session_end`. B7 names `quest_complete`. The `events` table's `CHECK (type IN (...))` rejects it, and SQLite cannot change a CHECK in place.

There is also no end-to-end check that events reach a real (local) Pages + D1 stack. The unit tests use `node:sqlite` behind a hand-written D1 shim.

## Bar moved

**B7, Deploy-ready with analytics** (level 1). The pass rule says `npm run build:site` packages the 3D client, local `wrangler pages dev` receives `session_start` (with age band), `quest_complete` and `session_end` from a scripted run, the dashboard query returns them, and the privacy tests still pass. The rule is checked by the `analytics-e2e` script. This item creates that script and makes it pass. No other bar's pass rule changes. B1 (golden path) must stay green; see acceptance test 6.

## Smallest version

One lane. The steps are listed in build order.

### 1. Server: `quest_complete`

- **Shape.** `{"v":1,"type":"quest_complete","band":"<band>","quest":<int>,"id":"<32 hex>"}`, where `quest` is a 0-based integer from 0 to 15 (`MAX_QUEST = 15`). In `validateEvent`, add `quest_complete: [...COMMON_KEYS, "quest"]` to `KEYS_BY_TYPE`. The rule for `quest` is the same as for `room`: an integer, in range, and the only extra key. `GameEvent` is unchanged: `handleEvent` maps `quest` into `room` for storage.
- **Keep the classic events.** `room_complete` and `trial_complete` stay valid, because the 2D game at `/classic/` still sends them.
- **Data class.** `quest` on the wire is a small progress index, stored in the existing `room` column (comment updated: "room or quest index, 0 to 15, set only for `room_complete` and `quest_complete`"). No new column and no new kind of data is stored, so target.md's human-only line "any change to what data is collected" is not crossed. B7 names the event, which authorizes the new type. (Critic revision R1.)
- **Migration.** Add `migrations/0002_quest_complete.sql`. Never edit 0001, which is already deployed. The migration rebuilds `events` in the D1-safe way:
  1. `CREATE TABLE events_new` with the same columns and CHECKs, plus `'quest_complete'` in the type list, still `STRICT`, and `room` still nullable with its 0 to 15 CHECK. No new column.
  2. `INSERT INTO events_new SELECT * FROM events` (same columns, same order).
  3. `DROP TABLE events`, then `ALTER TABLE events_new RENAME TO events`.
  4. Re-create `events_day_id`.

  Every column keeps a purpose comment. No IP, UA, country or time-of-day column is added.
- **Ingest.** `handleEvent` inserts the `quest` value into `room` for `quest_complete`. The cap, the purge and the origin, size and content-type checks stay exactly as they are.
- **Stats.** `handleStats` adds `quest_complete` to the funnel query. In `shapeStats`, `funnel` gains `quests: ({ quest } & BandCounts & { all })[]`, built from `quest_complete` rows' `room` value, dense from 0 to the highest quest seen, the same way `rooms` is built (and `rooms` counts only `room_complete`). The existing fields keep their names and meaning.
- **Dashboard.** `dashboard/dashboard.js` draws the funnel as: started, then each "Quest N complete", then the classic room and trial rows only when they are non-zero. The KPI row gains "quests completed". The page still uses no inline script or style (CSP).

### 2. Client: `crates/bevy-client/web/analytics.js` (new)

This is a plain-JS ES module with JSDoc types, because the root `tsconfig` has `checkJs` and the tests import it. It is a port of the privacy core of `web/src/analytics.ts`, and the behaviour is identical:

- `optedOut(nav)`: DNT `"1"` or `"yes"`, or GPC `true`, means nothing is sent at all.
- `dailyId(storage, now)`: the key `rg.dailyId` holds `"<UTC day>.<32 hex>"` and is replaced on a new UTC day. Every storage access is wrapped in try/catch, and if storage fails the id lives in memory for that page load.
- `durationBucket(ms)`: returns one of the five buckets.
- `send()`: `sendBeacon` first, otherwise `fetch` with keepalive, `credentials: "omit"` and `text/plain`. It never throws.

Pure helpers are exported for tests. The module touches `window` or `document` only inside functions, never at import time.

**On/off switch.** Analytics, and the age picker, run only when the page carries `<meta name="rg-analytics" content="on">`. The source `index.html` ships `content="off"`. `scripts/build-site.sh` flips it to `on` in the `web/dist` copy only. As a result:

- `pkg-out`, which golden, perf and the panel serve from a plain static server with no `/api/event`, sends no request and shows no picker. A beacon to a 404 logs a console error in Chromium; the planner checked this. Those errors would break the "0 console errors" check in B1 and the panel's error count.
- The deployed site, and `wrangler pages dev` on `web/dist`, get analytics.

**Session state** (one session per page load):

- **`session_start {band}`** is sent once, when a band has been chosen and the first game state has arrived (`window.__gameState` is non-null).
- **`quest_complete {band, quest}`** comes from the existing `window.__game.state()`. `hud.js` and `src/` are not edited. A `setInterval` of 500 ms reads the state; this is not done per frame, because of the B2 CPU budget. The event is sent when `outcome` changes from `'playing'` to `'won'`.
  - `quest` is `state.quest` when that is an integer from 0 to 15, otherwise 0 ("Save the Greenhouse" is quest 0).
  - It is sent at most once per quest per session, so a replay after R does not resend.
  - `'lost'` sends nothing.
  - The detection is a pure function, `questEvents(prevState, state, sentSet)`, so it can be unit tested.
- **`session_end {band, dur}`** is sent on `visibilitychange` to hidden, and on `pagehide`, but only when the bucket has grown since the last one sent. This is the same rule as classic.

### 3. Client: `crates/bevy-client/web/age.js` (new): the age picker

- **When it shows.** It shows once per page load, as soon as the page loads, but only when analytics is on and `optedOut` is false. With DNT or GPC set it never shows, because it would collect nothing.
- **Not remembered.** The choice is deliberately not stored. On shared classroom Chromebooks a remembered band would be attributed to the next child. GS-14 covers settings; the age band is not a setting.
- **Loading.** The picker sits over `#stage` and `#loading`. The WASM keeps downloading and booting behind it. `init()` is not gated, so the B3 time to first interactive frame is unchanged.
- **The choices.** There are exactly three `<button>` elements, each with a picture and a short number label:
  - young: "4 to 7" → `sprout`
  - middle: "8 to 10" → `seedling`
  - older: "11 and up" → `apprentice`

  The band values are the ones that already exist on the server, so the band vocabulary does not change.
  - There is no `<input>`, `<select>`, `<textarea>`, text box or birthday field. Nothing typed is accepted.
  - The pictures are inline SVG in `index.html`, or SVG files in `web/`, using presentation attributes only, with no `style=""` (CSP `style-src 'self'`). They show three figures or creatures of clearly different size and age (low-poly toon style). They differ by shape and size, not only by colour (GS-19). The figures are not babyish, so a 13-year-old is fine picking "11 and up" (GS-27). They use no word "kid".
  - The heading is one short line, for example "How old are you?" (GS-21). An age picture beside each number lets a pre-reader pick by picture.
  - Each button has an accessible name, for example `aria-label="4 to 7 years old"`.
- **After a pick.** The picker closes and calls `window.__game.cmd('reset')` once, so the courtyard starts fresh. If the game has not delivered its first state yet (the pick came before boot finished), the reset is queued and runs on the first state, so a click never hits a missing `__game`. (R2.) This command is allowed without `?debug`. Fires spread while the picker is open, and a child must not start the quest behind schedule because of the picker (GS-25). Focus moves to the canvas. Then the existing "Click to look around" flow runs unchanged.
- **Modal.** While open, the picker sits above the HUD and the pause card, and takes focus on its first button, so Enter works at once and clicks and keys do not reach the game behind it. Text scales with browser zoom to 200% without clipping (GS-18). (R3.)
- **No dark patterns** (GS-28). No countdown, no auto-pick, no "come back tomorrow". The picker cannot time out (GS-21: read at your own pace).
- **`boot.js`** imports `analytics.js` and `age.js` and starts them after `startHud()`. If `analytics.js` throws, the game still runs, so the start-up call is wrapped in try/catch.

### 4. `npm run build:site` packages the 3D client

`scripts/build-site.sh` becomes:

1. **3D client.** It runs `bash crates/bevy-client/build-web.sh`, honouring `CARGO_TARGET_DIR` and `PATH` from the environment. It then copies `crates/bevy-client/web/pkg-out/*` to `web/dist/`, so the site root is the 3D client. In `web/dist/index.html` it flips `rg-analytics` to `on`, and it fails the build if the replacement did not happen exactly once.
2. **2D classic.** It runs `npm --prefix web run build -- --base=/classic/ --outDir <tmp>`, which is passed through to `vite build`, so `web/` is not edited. It moves the result to `web/dist/classic/`. The 2D game stays reachable at `/classic/` until the first playtest passes (target.md).
3. **Headers.** It copies `web/public/_headers` to `web/dist/_headers` at the root, so the strict CSP applies to the whole site. It removes the copy Vite put under `classic/`.
4. **Dashboard.** It copies the dashboard to `web/dist/dashboard/`, as it does now.

`scripts/cf-pages-build.sh` (Cloudflare's git-build path) also installs `wasm-bindgen-cli` 0.2.129, pinned to `crates/bevy-client/Cargo.lock`, so the same script works there. `wrangler.toml` and `package.json` do not change.

**`crates/bevy-client/build-web.sh`: one-line change.** Its `cp` list (`index.html style.css boot.js hud.js favicon.svg`) must add `analytics.js age.js`. Without this, `boot.js` imports modules that are missing from `pkg-out` and the 3D client fails to boot, which breaks B1. This file is not in the ownership list the item was given (it belongs to the client-web area). The planner adds it to the lane for that `cp` line only. No other line of that file changes.

### 5. `tests/e2e/analytics-e2e.mjs` (new)

The script is runnable as `node tests/e2e/analytics-e2e.mjs`. Like golden it uses `playwright`, and it needs `web/dist` from `npm run build:site`. Env: `QUERY` (default `?low`) and `SKIP_BUILD=1` (reuse `web/dist`). It runs these steps:

1. **Local stack.** Make a fresh temp dir. Run `npx wrangler d1 migrations apply realgame-analytics --local --persist-to <tmp>`. Then run `npx wrangler pages dev web/dist --port <free> --persist-to <tmp> --binding DASHBOARD_TOKEN=<48 random chars>`. Wait for HTTP 200 on `/`.
2. **Main run.** Open `/<QUERY>&debug`; the debug commands are only for winning fast. Record every request.
   - Assert the picker is visible, with three buttons and no `input`, `select` or `textarea` inside it. Each button is at least 96×96 CSS px at 1366×768 (GS-22).
   - Click the **middle** picture.
   - Win "Save the Greenhouse" with `__game.cmd`. Select Water, `face` each burning bush, then `cast`. Poll until `outcome === 'won'`.
   - Close the page with `page.close({ runBeforeUnload: true })`, which triggers `session_end`. If a headless close does not fire `pagehide`, the script first calls `page.evaluate` to dispatch a `visibilitychange` with the state forced hidden; the check stays real because the beacon is still captured on the wire.
3. **Keyboard run.** On a new page, assert the picker can be used from the keyboard (GS-13): Tab or arrow keys reach the third picture and Enter picks it. Close the page after `session_start` lands (this run may also send one `session_end`, `lt1m`).
4. **Opt-out runs.** Run one new page with `addInitScript` defining `navigator.globalPrivacyControl = true`, and another with `navigator.doNotTrack = "1"`. For each, assert that no picker is shown and that no request goes to `/api/event` within 10 s of the first game state. Then close the page.
5. **D1 assertions.** Use `wrangler d1 execute realgame-analytics --local --persist-to <tmp> --json --command "SELECT ..."`.
   - The main run's id has exactly 1 `session_start` (`seedling`), exactly 1 `quest_complete` (`seedling`, `room = 0`), and 1 or more `session_end` (`seedling`, `dur` a valid bucket; no wall-clock equality, the run is under a minute and would flake).
   - The keyboard run has exactly 1 `session_start` (`apprentice`), no `quest_complete`, and at most 1 `session_end`.
   - No rows with any other `daily_id`, and the opt-out runs left no rows.
   - `PRAGMA table_info(events)` lists exactly `id, day, daily_id, type, band, room, dur` (no new column).
6. **Dashboard assertions.** Run `GET /api/stats` with the bearer token and assert:
   - `players.today === 2`;
   - `players.todayByBand.seedling === 1` and `apprentice === 1`;
   - `funnel.started.seedling === 1`;
   - `funnel.quests[0].seedling === 1`;
   - the sum of `durations[].players` is 1 or more.

   Also assert that `GET /api/stats` without the token returns 401.
7. **Wire privacy.** Assert:
   - every captured `/api/event` body has exactly the keys allowed for its type, with `v === 1` and `id` matching `/^[0-9a-f]{32}$/`;
   - all events in one run share one id;
   - no request in any run goes to an origin other than the page's;
   - the main run has 0 console errors and 0 page errors.
8. **Output.** Print `PASS` or `FAIL` per check and exit 1 if any check fails. Always stop wrangler, including on failure.

### 6. `docs/ANALYTICS.md`

Update the data flow (the client is now `crates/bevy-client/web/analytics.js`), add the `quest_complete` row and example, and describe the age picker. Add the `rg-analytics` switch, `/classic/`, migration 0002, the dashboard's quest funnel, and how to run `analytics-e2e`. Change "Hooking the game up" to describe the 3D wiring, and keep the classic table under a `/classic/` heading. In the suggested privacy-notice text, "which rooms they finish" becomes "which quests or rooms they finish". The notice's legal review stays human-only (target.md); the round notes flag it for Paawan.

## Acceptance tests (each can fail)

Run them from the repo root. Each test can fail for the reason given.

1. **Unit and type checks.** `npm ci && npm run typecheck && npm test` passes. The new tests (in `test/`) assert:
   - **`validateEvent`.**
     - Accepts `quest_complete` with `quest` equal to 0 and to 15.
     - Rejects `quest` values of 16, -1, 1.5 and `"0"`, and rejects a missing `quest`.
     - Rejects `quest` on `session_start`, `room` on `quest_complete`, and any extra key (including a `quest` column name in a stored row, which does not exist).
   - **Migration 0002.** On `node:sqlite`, with rows inserted under 0001 first, 0002 keeps them, accepts a valid `quest_complete` row, and rejects these by CHECK: `room = 16` on a `quest_complete` row, `type = 'quest_completed'`, and a free-text band.
   - **Handlers end to end** (`test/handlers.test.ts` style). A `quest_complete` is stored and shows up in `funnel.quests[0]`, and the classic `room_complete` still works.
   - **`analytics.js` pure functions.**
     - DNT and GPC opt out.
     - The daily id rotates on a UTC day boundary and survives blocked storage.
     - Duration buckets are correct.
     - The payload builder emits exactly the allowed keys.
     - `questEvents` sends once on `playing`→`won`, sends nothing on `lost`, sends nothing on a repeated `won`, and sends nothing after R then a second win in the same session.

   The suite fails if any of these regress. **No existing assertion is removed or loosened.** `git diff origin/main -- test/` shows only additions, plus any edit needed for the renamed or extended `Stats` shape, each justified in the round notes.
2. **Build.** `CARGO_TARGET_DIR=/home/claude/bevy-target PATH=/home/claude/bevy-target/tools/bin:$PATH npm run build:site` exits 0. Then all of the following hold:
   - `web/dist/index.html` contains `<canvas id="game"` and `content="on"` for `rg-analytics`.
   - `web/dist/{boot.js,hud.js,analytics.js,age.js,bevy-client.js,bevy-client_bg.wasm}` exist.
   - `web/dist/classic/index.html` exists, and every `src` or `href` in it starts with `/classic/` or is relative.
   - `web/dist/dashboard/index.html` exists.
   - `web/dist/_headers` exists and contains `connect-src 'self'`.
   - `crates/bevy-client/web/index.html` (the source) still says `content="off"`.
   - `grep -E 'https?://' web/dist/{index.html,boot.js,analytics.js,age.js}` finds no URL outside comments.

   The test fails if any of these is missing.
3. **Classic still reachable.** On `wrangler pages dev web/dist` (or as part of `analytics-e2e`), `GET /classic/` returns 200 with the 2D page, and its JS and WASM assets return 200. It fails if `/classic/` returns 404 or its assets break.
4. **analytics-e2e.** `node tests/e2e/analytics-e2e.mjs` (after test 2, `SKIP_BUILD=1` allowed) exits 0, with every check in Smallest version §5 printing `PASS`. Each of these mutations, applied one at a time and reverted, makes it exit 1. The builder records the failing check name for each in `round-1.md`, and the breaker reruns at least two:
   - (a) the `quest_complete` send is removed from `analytics.js`;
   - (b) `optedOut` always returns false;
   - (c) the picker sends `sprout` for the middle picture;
   - (d) `session_end` is removed;
   - (e) a `ua: navigator.userAgent` key is added to the payload;
   - (f) `handleStats` drops `quests`;
   - (g) `handleEvent` stores `quest` as 0 whatever was sent.
5. **No analytics off-site.** Serve `crates/bevy-client/web/pkg-out` with a plain static server (as golden does) and load it without `?debug`. No picker is visible, and no request is made to `/api/event` within 20 s of the first game state, including after a reset and a win. It fails if the meta switch is ignored.
6. **B1 not regressed.** After the new `build-web.sh`, `QUERY=?low node tests/e2e/golden.mjs` passes with `golden.mjs` unchanged, and shows 0 console errors. It fails if `boot.js` cannot find a module, or if the picker blocks the canvas.
7. **Scope.** `git diff --name-only origin/main...` lists only these paths:
   - `crates/bevy-client/web/index.html`, `boot.js`, `analytics.js` and `age.js`;
   - `crates/bevy-client/build-web.sh`;
   - `functions/**`, `dashboard/**`, `migrations/**`, `scripts/**` and `test/**`;
   - `tests/e2e/analytics-e2e.mjs`;
   - `docs/ANALYTICS.md`;
   - `docs/loop/items/I-005/**`.

   Also, `git diff origin/main -- crates/bevy-client/build-web.sh` changes only the `cp` line. Nothing under `crates/bevy-client/src/`, `crates/bevy-client/web/hud.js`, `crates/bevy-client/web/style.css`, `web/`, `wrangler.toml`, `package.json` or `tests/e2e/golden.mjs` changes. The picker's styles therefore cannot go in `style.css`. They go in a new `crates/bevy-client/web/age.css`, linked from `index.html`, which needs one more name in the `build-web.sh` `cp` line. Alternatively, the styles can be set from `age.js` through CSSOM property assignment, which CSP allows. The builder states which approach it used.

## Gameplay standards touched

The age picker is a new screen. The other parts of this item are server or tooling only.

| GS | Priority | How the spec meets it |
|----|----------|-----------------------|
| GS-13 | P1 | The picker can be used from the keyboard (Tab or arrow keys, then Enter). Checked in e2e step 3. |
| GS-18 | P0 | Picker text is at least 18 px at 1080p, with contrast of at least 4.5:1. The breaker measures it. |
| GS-19 | P0 | The bands differ by picture shape and size and by a number label, not by colour alone. |
| GS-21 | P0 | One short heading, a picture per choice so pre-readers can pick, and no auto-dismiss or timeout. |
| GS-22 | P1 | The buttons are at least 96×96 CSS px, above the 40 px primary-action minimum. Checked in e2e step 2. |
| GS-23 | P0 | One click, then straight into the existing safe first area. It is not a static tutorial gate. |
| GS-25 | P1 | Pausing the clock while the picker is open is avoided; instead the game resets after the pick, so time on the picker costs no quest time. |
| GS-27 | P0 | Neutral, non-babyish figures, and no "kid" wording. |
| GS-28 | P0 | No countdown, no auto-pick, no lures, and no external links. |
| GS-6 | P0 | The picker has no flashing or animation beyond a focus ring. |
| GS-14 | P1 | Not applied, by design: the age band is not a setting and is not stored, for the shared-device reason above. |

GS-1 to GS-5, GS-8 to GS-12, GS-15 to GS-17, GS-20, GS-24, GS-26 and GS-29 to GS-32 are not touched, because the item makes no change to the camera, controls, feedback, HUD, quests or science.

## Out of scope

- Any edit to `hud.js`, `style.css`, `crates/bevy-client/src/**` or `crates/sim/**`. Quest completion is read from `window.__game.state()` only. A multi-quest `state.quest` field belongs to whichever item adds quests.
- Removing the 2D game. It moves to `/classic/` and stays until the first playtest passes, which is human-only.
- `SECURITY.md`, `docs/DEPLOY.md` and the published privacy notice. The suggested wording changes in `ANALYTICS.md` only. Legal review and publishing are human-only. The round notes list the `SECURITY.md` lines that should mention quests, as a follow-up.
- Production deploy, D1 creation and secrets (`npm run deploy` is human-only).
- **Panel harness.** The panel serves `pkg-out` (analytics off), so personas never see the picker. A tester follow-up could add the picker to `tests/e2e/panel/hud.mjs` and run the panel against `web/dist` if the critic wants the picker panel-tested for B4.
- Adding `analytics-e2e` to push CI or to `package.json` scripts, since `package.json` is a shared file.
- Remembering the band, a "change age" control, or teacher class links.

## Contract

none. This is a single lane, and the only shared file is `crates/bevy-client/build-web.sh`, owned by this lane for its `cp` line only.

## Lanes

| lane | owns | builds |
|------|------|--------|
| main | `crates/bevy-client/web/index.html`, `crates/bevy-client/web/boot.js`, `crates/bevy-client/web/analytics.js`, `crates/bevy-client/web/age.js` (plus `age.css` if used), `crates/bevy-client/build-web.sh` (the `cp` line only; see §4), `functions/**`, `dashboard/**`, `migrations/**`, `scripts/**`, `test/**` (analytics tests only), `tests/e2e/analytics-e2e.mjs`, `docs/ANALYTICS.md`, `docs/loop/items/I-005/lanes/main/**` | Smallest version §1 to §6. The test-1 unit tests are written first and fail on a94fc48. The builder records the outputs of tests 2, 4 (with the six mutation results), 5 and 6 in `round-1.md`, and flags the privacy-notice wording for Paawan's review. |
