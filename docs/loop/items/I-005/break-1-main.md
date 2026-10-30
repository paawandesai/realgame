# I-005 break-1 (main lane), fresh breaker

Verdict: no blocking findings. Ran against 864192c.

## Verified
- Scope: only lane paths changed (index.html, boot.js, analytics.js, age.js, functions, dashboard, migrations, scripts, test, analytics-e2e.mjs, ANALYTICS.md). build-web.sh is NOT changed (deviation from spec section 4, see nit 1; harmless, in lane).
- `npm run typecheck` clean, `npm test` 50/50 (new questEvents, session, validate, migration 0002, handlers, shapeStats tests present; no existing assertion removed).
- `npm run build:site` exits 0. dist has canvas, rg-analytics=on once, boot/hud/analytics/age/bevy-client js+wasm, classic/ (src/href start with /classic/), dashboard, _headers with connect-src 'self'. Source index.html still "off". No http(s) URLs outside comments in index.html, boot.js, analytics.js, age.js.
- `SKIP_BUILD=1 node tests/e2e/analytics-e2e.mjs`: 48/48 PASS.
- Mutations rerun on web/dist: (b) optedOut always false -> GPC and DNT checks FAIL (46/48); (e) `ua: navigator.userAgent` added -> wire, stats and D1 checks FAIL (36/48). Both reverted.
- `QUERY=?low node tests/e2e/golden.mjs`: 31/31, 0 console errors on pkg-out (first run: Chromium "Target crashed" at keyboard.up under software GL, not reproducible on rerun; treated as environmental).
- Privacy: wire bodies carry only v/type/band/(quest|dur)/id; one id per run; different ids across contexts; no cross-origin request; DNT and GPC give no picker and no request; server stores day from server clock only, no IP/UA/country column (PRAGMA = id,day,daily_id,type,band,room,dur); no free-text path (CHECKs and allow-list). Picker: exactly 3 buttons, no input/select/textarea, 168x189 px, 24 px labels, 32 px heading, good contrast, Tab/arrows/Enter work, not remembered, no timer. CSP: styles set via CSSOM, no inline style attrs, connect-src 'self'.

## Blocking
none

## Should-fix
1. functions/_lib/analytics.ts:~351 (`COUNT(DISTINCT daily_id)` over all event types) with crates/bevy-client/web/analytics.js `send` (calls `dailyId` per event): a session that crosses UTC midnight sends its `session_end` under a NEW id. That adds a phantom player on the new day with no session_start. Fix: pin the id at `start()` for the session (still rotates on next page load), or count players from session_start only. Pre-existing in classic, more likely in longer 3D sessions.
2. crates/bevy-client/web/index.html:55 and boot.js:33: the picker is `hidden` in markup and shown only after the async `import('./age.js')` resolves. On a slow link the "Click to look around" card is live first, and a click can take pointer lock before the picker appears; a locked cursor cannot click the picker. Fix: unhide in dist at build time, or call `document.exitPointerLock()` in `startAgePicker` before showing.
3. index.html:55 (`aria-modal="true"`) does not make the game behind it inert for assistive tech; set `inert` on `#stage`, `#hud`, `#loading` while open and remove it on pick.

## Nit
1. Spec section 4 asked for a one-line `cp` change in crates/bevy-client/build-web.sh; the builder instead copies analytics.js and age.js in scripts/build-site.sh:35 (pkg-out stays analytics-free). Behaviour is correct; update the spec or round notes. Side effect: if analytics.js or age.js is ever missing in a deploy, boot.js:33 swallows the import error, so the site silently ships with no picker and no analytics. Only e2e catches it.
2. A reload re-asks the picker; a different band on reload puts one daily id in two bands (`todayByBand` can sum above `players.today`). Acceptable; note it in ANALYTICS.md.
3. A 4-year-old can pick any picture; band is self-reported. Say so in ANALYTICS.md so the dashboard is not read as verified age.
4. analytics.js `optedOut` reads only navigator.doNotTrack and globalPrivacyControl (same as classic).
5. Heading "Pick your age" (index.html:57) vs the spec's question "How old are you?", which reads easier aloud to a pre-reader.
6. `rg.dailyId` stays in localStorage after its day ends until the next visit. Pre-existing.
