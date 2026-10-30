# Analytics

How many children play, in which age band, and how far they get, counted without collecting anything that identifies a child. This file is the contract for the data: what is sent, why, where it is kept, and when it is deleted. `SECURITY.md` has the short version for parents and schools; `docs/DEPLOY.md` covers setting it up.

## The data flow

```
3D game at / (crates/bevy-client/web/analytics.js, age.js)    classic 2D game at /classic/ (web/src/analytics.ts)
  │  POST /api/event, same origin, ~100 bytes of JSON (sendBeacon, or fetch with keepalive)
  ▼
Cloudflare Pages Function (functions/api/event.ts)
  │  strict validation, per-device daily cap, purge of rows older than 90 days
  ▼
Cloudflare D1 database "realgame-analytics", table `events` (migrations/0001_events.sql, 0002_quest_complete.sql)
  ▲
  │  aggregate queries only
Cloudflare Pages Function (functions/api/stats.ts), behind DASHBOARD_TOKEN
  ▲
  │  GET /api/stats with Authorization: Bearer <token>
dashboard (/dashboard/, static files from dashboard/)
```

There is no third-party analytics service, script, pixel or SDK. The browser only ever talks to the game's own origin, so the Content Security Policy stays `connect-src 'self'` (already the setting in `web/public/_headers`; nothing there changed).

## The site

`npm run build:site` (`scripts/build-site.sh`) builds `web/dist`, the folder Cloudflare Pages serves:

- `/` is the first-person 3D game (`crates/bevy-client`), the game children play.
- `/classic/` is the retired 2D game (`web/`), kept reachable until the first playtest passes. It still sends its own `room_complete` and `trial_complete` events.
- `/dashboard/` is the dashboard below.
- `web/public/_headers` is copied to the root, so the strict CSP (`connect-src 'self'`) covers the whole site.

### The `rg-analytics` switch

The 3D client sends events, and shows the age picker, only when its page carries `<meta name="rg-analytics" content="on">`. The source `crates/bevy-client/web/index.html` ships `content="off"`, and `build-site.sh` flips it to `on` in the `web/dist` copy only (the build fails if the flip does not happen exactly once). So `crates/bevy-client/web/pkg-out`, which the golden-path, perf and panel tests serve from a plain static server with no `/api/event`, sends nothing and shows no picker. `boot.js` loads `age.js` and `analytics.js` on demand only when the switch is on, inside a `try`, so a failure there cannot stop the game from booting.

## What is sent

Five event types, and nothing else. The 3D game sends `session_start`, `quest_complete` and `session_end`; the classic game at `/classic/` sends `session_start`, `room_complete`, `trial_complete` and `session_end`. Each is a JSON object with exactly these keys; the server rejects any other key, value or shape.

| Event | Sent when | Fields besides `v`, `type`, `id` |
| --- | --- | --- |
| `session_start` | 3D: an age band was picked and the game's first state has arrived (once per page load). Classic: play begins with a chosen or remembered age band | `band` |
| `quest_complete` | 3D: a quest's outcome changes from playing to won (each quest at most once per session, so a replay after R does not resend; a loss sends nothing) | `band`, `quest` (0-based integer, 0 to 15; "Save the Greenhouse" is 0) |
| `room_complete` | Classic: a room is cleared (each room at most once per session) | `band`, `room` (0-based integer, 0 to 15) |
| `trial_complete` | Classic: the whole trial is finished (once per session) | `band` |
| `session_end` | The tab is hidden or closed, and the session's length bucket has grown since the last one sent | `band`, `dur` |

- `v` is always `1` (the format version).
- `band` is one of `sprout` (ages 4 to 7), `seedling` (8 to 10), `apprentice` (11 and up): the age band the player picked, not an age or birth date.
- `dur` is one of `lt1m`, `1to5m`, `5to15m`, `15to30m`, `30mplus`. The server keeps the longest bucket per device per day.
- `id` is 32 random hex characters (128 bits from `crypto.getRandomValues`) made on the device and replaced the first time it is read on a new UTC day. It lets the server count a day's unique players. Because it changes every day and is derived from nothing about the device, it cannot link one day's play to the next, and it is never shared with anyone.

- `quest` is a small progress index, stored in the existing `room` column (migration 0002 adds no column and no new kind of data).

Examples, in full:

```json
{"v":1,"type":"quest_complete","band":"seedling","quest":0,"id":"9f2c4e0b7a1d4c3e8b6f0a2d4e6c8b1a"}
{"v":1,"type":"room_complete","band":"seedling","room":2,"id":"9f2c4e0b7a1d4c3e8b6f0a2d4e6c8b1a"}
```

The 3D client reads quest completion from the game's existing `window.__game.state()` hook (`web/hud.js`) every 500 ms (not every frame, for the CPU budget); the game code itself is not involved.

## The age picker (3D game)

When analytics is on and the browser has not opted out, the 3D game opens with one short heading, "Pick your age", and three large picture buttons: "4 to 7" (`sprout`), "8 to 10" (`seedling`) and "11 and up" (`apprentice`). Each has a low-poly figure of a clearly different size and shape, so a child who cannot read yet can pick by picture, and an accessible name such as "4 to 7 years old". There is no text box, birthday field or anything else to type. The markup is in `crates/bevy-client/web/index.html` (hidden) and `age.js` styles it through the CSSOM, which the CSP allows.

- It shows once per page load, before play, over the loading screen; the WASM keeps loading behind it. It has no timer, countdown or auto-pick.
- It is modal: it takes focus on its first button, Tab and the arrow keys move between the three pictures, Enter or a click picks, and keys and clicks do not reach the game behind it.
- After a pick it closes, sends nothing by itself, and resets the courtyard once (queued until the game's first state if the pick came before boot finished), so time spent on the picker costs no quest time. Focus goes to the game, and the usual "Click to look around" flow follows.
- The choice is **not stored**. On shared classroom devices a remembered band would be counted for the next child, so a reload asks again.
- With Do Not Track or Global Privacy Control set it never shows, because nothing would be collected.

## What is never collected

Names, nicknames, emails, ages or birth dates, anything typed (formulas, spells, free text), the lab notebook, cookies, IP addresses, user agents, device or browser fingerprints, screen sizes, referrers, precise or coarse location (Cloudflare's `request.cf.country` is deliberately not read), timestamps finer than a UTC day, or any cross-site identifier.

The server stores only: the UTC day it received the event (from its own clock, not the device's), the daily id, the event type, the band, and the room or quest index or the duration bucket. See `migrations/0001_events.sql` and `migrations/0002_quest_complete.sql` (which rebuilds the table to allow the new type, with the same columns); each column there is commented with its purpose.

## On the device

`localStorage` holds one extra key, `rg.dailyId`, with the value `"<UTC day>.<id>"`. It is overwritten each new UTC day, so there is never more than one. All access is wrapped in `try`/`catch`. If storage is blocked (private windows, locked-down school profiles), the id lives in memory for that page load only; such a device may be counted more than once that day, but nothing breaks.

The game works the same when the endpoint is missing or unreachable (local `npm run dev`, offline, a plain static host): every send is fire-and-forget, errors are swallowed, and nothing waits on the network. This was checked in a browser with `/api/event` blocked.

## Do Not Track and Global Privacy Control

**Decision: both are honoured. When `navigator.doNotTrack` is `"1"` or `navigator.globalPrivacyControl` is `true`, nothing is sent at all.**

Why: these counts are first-party, anonymous and aggregate, so neither signal is strictly required by law for them (GPC is legally binding mainly as an opt-out of sale or sharing, which never happens here). But this is a children's product used in schools, and a browser that asks not to be tracked should get exactly that. The cost is undercounting: Firefox with strict settings, Brave, and some managed school browsers send these signals, so the dashboard's numbers are a floor, not a census. To change the decision, edit `optedOut()` in `crates/bevy-client/web/analytics.js` and `web/src/analytics.ts` (the classic game), and this section.

## Server-side checks (functions/_lib/analytics.ts)

- **Method and origin:** POST only. Requests whose `Sec-Fetch-Site` is not `same-origin`, or whose `Origin` is another site, get 403. This keeps other sites from posting through visitors' browsers; it is not authentication, and the cap below bounds what a script can do.
- **Content type and size:** `text/plain` (what `sendBeacon` sends for a string) or `application/json` only; bodies over 512 bytes get 413 before parsing.
- **Shape:** exactly the keys listed above for that event type; `type` and `band` from fixed lists; `room` and `quest` integers from 0 to 15; `dur` from the fixed list; `id` exactly 32 lowercase hex characters. Anything else gets 400 and is not stored. The response never echoes input.
- **Rate limit:** at most 120 events per daily id per UTC day (a full play-through is about 8). The insert and the cap check are one SQL statement, so there is no race. Over the cap: 429.
- **Database constraints:** the table repeats the same rules as `CHECK` constraints in a `STRICT` table, so a bug in the Function still cannot store free text.
- **Logging:** the Functions log nothing. Cloudflare's Workers Logs are off by default for Pages; leave them off.

## Retention

Raw events are kept for **90 days** (today plus the previous 89 UTC days) and then deleted. Cloudflare Pages Functions cannot run on a schedule, so the purge runs inside the requests themselves: every `/api/event` insert and every `/api/stats` read also runs `DELETE FROM events WHERE day < <cutoff>`. That is one index range scan, effectively free when there is nothing to delete. If the game gets no traffic and no one opens the dashboard, no request runs the purge; `npm run analytics:purge` does the same delete by hand. There are no rollups or backups beyond D1's own point-in-time recovery (Time Travel), which keeps 7 days on the free plan, so deleted rows are gone from it within 7 days.

## The dashboard

`/dashboard/` is a static page (in `dashboard/`, copied into `web/dist/dashboard/` by `scripts/build-site.sh`). It asks once for the `DASHBOARD_TOKEN`, keeps it in `sessionStorage` for that tab only, and calls `/api/stats` with it. It shows:

- players today (unique daily ids) and today's split by band
- sessions today and over 30 days
- daily unique players for 30 days, stacked by age band
- age-band totals over 30 days
- quests completed over 30 days (and quest 1 as a share of sessions started)
- the funnel: started, then each "Quest N complete"; the classic game's rooms and trial follow only when they are non-zero (all bands, or one band)
- the longest session per player per day, by bucket
- a table view of the daily numbers

A player is counted on the UTC day their session started; a session that runs past midnight is not counted again on the next day. Counts across days are **player-days** (one device on one UTC day), since no identifier spans days by design. A device that switches bands within a day counts once in the day's total but in each band it played.

The page itself holds no data, so it is not secret; the data is behind `/api/stats`. That Function compares the bearer token with `DASHBOARD_TOKEN` in constant time (both SHA-256 hashed, then compared byte by byte without an early exit), and refuses with 503 if the secret is missing or shorter than 32 characters. It is drawn with hand-written SVG and HTML, with no inline scripts or styles and no chart library, so it runs under the same strict CSP as the game.

For stronger protection, put **Cloudflare Access** in front of `/dashboard/*` and `/api/stats` (steps in `docs/DEPLOY.md`). Access checks identity at Cloudflare's edge (for example, a one-time code to Paawan's email) before any request reaches the page or the Function. The token check stays as a second layer.

## Hooking the game up

### The 3D game (site root)

Nothing in the game code (`crates/bevy-client/src/`, `web/hud.js`) calls analytics. The wiring is:

- `boot.js`, after `startHud()`, checks the `rg-analytics` switch and, only when it is on, imports `age.js`, which imports `analytics.js`.
- `age.js` shows the picker (unless DNT or GPC is set). A pick calls `start(band)` in `analytics.js` and queues one `window.__game.cmd('reset')`.
- `start(band)` creates the page's one session (`createSession`): every 500 ms it reads `window.__game.state()`; the first non-null state sends `session_start`, and `questEvents(prev, state, sent)` turns a playing-to-won change into `quest_complete` (quest `state.quest` when it is an integer from 0 to 15, otherwise 0). `visibilitychange` to hidden and `pagehide` send `session_end` when the bucket has grown.

The pure parts (`optedOut`, `dailyId`, `durationBucket`, `eventBody`, `questEvents`, `createSession`, `analyticsOn`) are unit tested in `test/bevy-analytics.test.ts`. A future multi-quest build only has to put the quest index in the state as `quest`.

### The classic game (/classic/)

The game calls three functions from `web/src/analytics.ts`. Each is safe to call at any time: it never throws, ignores bad input, and deduplicates per session.

```ts
import { trackRoomComplete, trackSessionStart, trackTrialComplete } from "./analytics";
```

**On `main` (the current game), only the session start is wired**, because `main` has no rooms yet. In `web/src/main.ts`, the first line of `start(tier, memory)`:

```ts
function start(tier: Tier, memory: WebAssembly.Memory): void {
  trackSessionStart(tier);
  ...
```

`start()` runs right after a tier is picked on the start screen and also when a remembered tier skips it, so "tier select" and "session start" are the same event.

**On the rewrite (`claude/playable-v2`), re-apply these four calls** after merging:

| Where (`web/src/main.ts` on `claude/playable-v2`) | Call |
| --- | --- |
| First line of `function start(tier: Tier, memory: WebAssembly.Memory)` | `trackSessionStart(tier);` |
| First line of `function restartTrial()` (a replay starts a new session, so rooms count again) | `trackSessionStart(tier);` |
| In the `s.gates_open.forEach((open, i) => { ... })` loop, inside `if (open && gatesSeen[i] === false && !freePlay) { ... }`, next to `showRoomClear(i)` | `trackRoomComplete(i);` (gate `i` is room `i`, 0-based; the dashboard shows it as "Room i+1") |
| In `function showEnd()`, right after `ended = true;` | `trackTrialComplete();` |

Free play is not counted as rooms or trials, since those calls sit behind the existing `!freePlay` checks.

Merging this branch with `claude/playable-v2` conflicts in one place only: the import block at the top of `web/src/main.ts` (checked with `git merge-tree`). Keep the rewrite's imports, add the analytics import line above, then add the calls in the table.

## Checking it end to end

`tests/e2e/analytics-e2e.mjs` runs the built site on a local Pages + D1 stack and checks bar B7:

```sh
CARGO_TARGET_DIR=... npm run build:site           # or SKIP_BUILD=1 to reuse web/dist
QUERY=?low node tests/e2e/analytics-e2e.mjs
```

It applies the migrations to a fresh local D1 directory, starts `wrangler pages dev web/dist` with a random `DASHBOARD_TOKEN`, then drives headless Chromium: a main run (picker shown with three 96 px or larger buttons and nothing to type in, picks "8 to 10", wins "Save the Greenhouse", hides and closes the page), a keyboard-only run (arrow and Tab to "11 and up", Enter), and two opt-out runs (GPC, DNT: no picker, no request). It then checks the rows in D1, `/api/stats` (with and without the token), every event body on the wire (exactly the allowed keys, one daily id per run, no other origin), `/classic/` and its assets, and the table's columns. It prints PASS or FAIL per check and exits 1 on any failure.

## Changing what is collected

Every new field needs a written purpose and a deletion date before it ships (`docs/ENGINEERING.md`). To add one: add a migration (never edit `0001_events.sql` once deployed), extend `validateEvent` and its tests, update this file and `SECURITY.md`, and deploy (`npm run deploy` applies new migrations first).

## Children's privacy law

This is the reasoning, not legal advice; have counsel review the published privacy notice before a wide launch.

- **COPPA (16 CFR Part 312, as amended in 2025; compliance date April 22, 2026).** "Personal information" includes a persistent identifier that can recognise a user over time and across different websites. The daily id is random, first-party, never shared, and replaced every day, so it is designed not to be one. Even if a regulator read it as one, it is used only for the "support for the internal operations" of the site (counting use to maintain and improve the game, 16 CFR 312.2 and 312.5(c)(7)), with no other personal information, no contact, no profiling and no advertising, which is the use the rule allows without parental consent. The amended rule asks operators relying on that exception to say in their online notice which internal operations it serves and how they keep the identifier to that purpose; the suggested notice text below does that. The written data-retention policy the amended rule requires is the Retention section above, and it should also appear in the notice.
- **Third parties.** Cloudflare hosts the site and the database as a service provider. Like any web host, its edge sees each visitor's IP address in transit; this project stores none of it. Name Cloudflare in the privacy notice as the hosting provider.
- **Student privacy (FERPA, state laws such as SOPIPA, the NDPA).** No student records, accounts or identifiers are created; no data is sold, used for advertising, or used to build a profile of a student.

Suggested text for the privacy notice:

> We count how many people play, which age band they pick (4 to 7, 8 to 10, or 11 and up), which quests or rooms they finish, and roughly how long they play, so we can fix and improve the game. To count each day's players without knowing who they are, the game keeps a random number in the browser that it replaces every day. We do not collect names, email addresses, anything typed into the game, cookies, IP addresses or location, and we never sell or share this data or use it for advertising. If your browser sends Do Not Track or Global Privacy Control, the game sends nothing. The counts are stored with our hosting provider, Cloudflare, and deleted after 90 days.
