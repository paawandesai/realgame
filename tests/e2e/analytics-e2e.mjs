// End-to-end check for bar B7: the built site (web/dist) on a local Cloudflare
// Pages + D1 stack (`wrangler pages dev`) receives the 3D client's anonymous
// events, the dashboard query returns them, and the privacy rules hold on
// the wire. See docs/ANALYTICS.md and docs/loop/items/I-005/spec.md (§5).
//
//   npm run build:site                      # builds web/dist (or SKIP_BUILD=1 to reuse it)
//   node tests/e2e/analytics-e2e.mjs
//
// Env: QUERY (default `?low`, faster on a software GPU; `classic=1` is always
// added (I-052) and `debug` for the main run only, to win fast), SKIP_BUILD=1 (reuse web/dist), KEEP=1
// (keep the temp D1 directory). Needs `playwright` with Chromium and the
// repo's wrangler (npm ci). Prints PASS or FAIL per check, exits 1 if any
// check fails, and always stops wrangler.
import { spawn, spawnSync } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const repo = join(dirname(fileURLToPath(import.meta.url)), '../..');
// I-052: the default page is the block world; `classic=1` keeps this check on
// today's courtyard (its win sends quest_complete) until the valley's own
// quest_complete is checked (golden E19).
const QUERY = [(process.env.QUERY ?? '?low').replace(/^\?/, ''), 'classic=1'].filter(Boolean).join('&');
const DB = 'realgame-analytics';
const TOKEN = randomBytes(24).toString('hex'); // 48 characters
const ID_RE = /^[0-9a-f]{32}$/;
const ALLOWED = {
  session_start: ['v', 'type', 'band', 'id'],
  quest_complete: ['v', 'type', 'band', 'quest', 'id'],
  session_end: ['v', 'type', 'band', 'dur', 'id'],
};
const BUCKETS = ['lt1m', '1to5m', '5to15m', '15to30m', '30mplus'];

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok: !!ok });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ` ${detail}` : ''}`);
  return !!ok;
}

function freePort() {
  return new Promise((resolve, reject) => {
    const s = createServer();
    s.on('error', reject);
    s.listen(0, '127.0.0.1', () => {
      const { port } = s.address();
      s.close(() => resolve(port));
    });
  });
}

function wrangler(args, opts = {}) {
  return spawnSync('npx', ['--no-install', 'wrangler', ...args], {
    cwd: repo,
    encoding: 'utf8',
    env: { ...process.env, CI: '1', WRANGLER_SEND_METRICS: 'false' },
    maxBuffer: 64 * 1024 * 1024,
    ...opts,
  });
}

function d1(persist, sql) {
  const r = wrangler(['d1', 'execute', DB, '--local', '--persist-to', persist, '--json', '--command', sql]);
  if (r.status !== 0) throw new Error(`d1 execute failed: ${r.stderr || r.stdout}`);
  const out = JSON.parse(r.stdout);
  return (Array.isArray(out) ? out : [out]).flatMap((x) => x.results ?? []);
}

/** Opens a fresh browser context (fresh storage, so a fresh daily id) and records its traffic. */
async function openRun(browser, base, { init } = {}) {
  const context = await browser.newContext({ viewport: { width: 1366, height: 768 } });
  if (init) await context.addInitScript(init);
  const requests = [];
  const events = [];
  context.on('request', (req) => {
    const url = req.url();
    requests.push(url);
    if (new URL(url).pathname === '/api/event') events.push({ at: Date.now(), body: req.postData() ?? '' });
  });
  const page = await context.newPage();
  const errors = [];
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(m.text());
  });
  page.on('pageerror', (e) => errors.push(e.message));
  return { context, page, requests, events, errors, base };
}

async function firstState(page, ms = 180000) {
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState).catch(() => false));) {
    if (Date.now() - t0 > ms) throw new Error(`game did not start within ${ms / 1000} s`);
    await sleep(250);
  }
  return Date.now();
}

async function waitFor(pred, ms) {
  const t0 = Date.now();
  while (!pred() && Date.now() - t0 < ms) await sleep(200);
  return pred();
}

const parsed = (run) =>
  run.events.map((e) => {
    try {
      return JSON.parse(e.body);
    } catch {
      return { unparseable: e.body };
    }
  });

(async () => {
  let pages = null;
  let browser = null;
  const persist = mkdtempSync(join(tmpdir(), 'rg-analytics-e2e-'));
  try {
    // ---------------------------------------------------------------- build
    if (process.env.SKIP_BUILD !== '1') {
      const b = spawnSync('bash', ['scripts/build-site.sh'], { cwd: repo, stdio: 'inherit' });
      if (b.status !== 0) throw new Error('npm run build:site failed');
    }
    if (!check('web/dist is built', existsSync(join(repo, 'web/dist/index.html')))) throw new Error('no web/dist');

    // ---------------------------------------------------- 1. local stack
    const mig = wrangler(['d1', 'migrations', 'apply', DB, '--local', '--persist-to', persist]);
    check('d1 migrations apply (local)', mig.status === 0, mig.status === 0 ? '' : (mig.stderr || mig.stdout).slice(-600));
    const port = await freePort();
    const origin = `http://127.0.0.1:${port}`;
    pages = spawn(
      'npx',
      ['--no-install', 'wrangler', 'pages', 'dev', 'web/dist', '--ip', '127.0.0.1', '--port', String(port), '--persist-to', persist, '--binding', `DASHBOARD_TOKEN=${TOKEN}`],
      { cwd: repo, env: { ...process.env, CI: '1', WRANGLER_SEND_METRICS: 'false' }, detached: true, stdio: ['ignore', 'pipe', 'pipe'] },
    );
    let wlog = '';
    pages.stdout.on('data', (d) => (wlog += d));
    pages.stderr.on('data', (d) => (wlog += d));
    let up = false;
    for (const t0 = Date.now(); !up && Date.now() - t0 < 120000;) {
      up = await fetch(`${origin}/`).then((r) => r.status === 200).catch(() => false);
      if (!up) await sleep(500);
    }
    if (!check('wrangler pages dev serves / with 200', up, up ? origin : wlog.slice(-1500))) throw new Error('no local stack');

    // Classic game still reachable at /classic/ (acceptance test 3).
    const classic = await fetch(`${origin}/classic/`);
    const classicHtml = await classic.text();
    const assets = [...classicHtml.matchAll(/(?:src|href)="([^"]+)"/g)].map((m) => m[1]);
    const assetStatus = await Promise.all(assets.map(async (a) => [a, (await fetch(new URL(a, `${origin}/classic/`))).status]));
    check(
      'GET /classic/ serves the 2D page and its assets',
      classic.status === 200 && /<html/i.test(classicHtml) && !/id="game"\s+tabindex/.test(classicHtml) && assets.length > 0 &&
        assets.every((a) => a.startsWith('/classic/') || !/^(\/|https?:)/.test(a)) && assetStatus.every(([, s]) => s === 200),
      JSON.stringify(assetStatus),
    );
    const wasmAsset = assets.find((a) => a.endsWith('.js'));
    if (wasmAsset) {
      const js = await (await fetch(new URL(wasmAsset, `${origin}/classic/`))).text();
      const wasm = [...js.matchAll(/["'`](\/classic\/[^"'`]+\.wasm)["'`]/g)].map((m) => m[1]);
      const ws = await Promise.all(wasm.map(async (w) => (await fetch(`${origin}${w}`)).status));
      check('the classic game\'s WASM is served under /classic/', wasm.length > 0 && ws.every((s) => s === 200), JSON.stringify(wasm));
    }

    browser = await chromium.launch();

    // ------------------------------------------------------- 2. main run
    const main = await openRun(browser, origin);
    const { page } = main;
    await page.goto(`${origin}/?${QUERY ? `${QUERY}&` : ''}debug`);
    await page.waitForSelector('#age', { state: 'visible', timeout: 60000 }).catch(() => undefined);
    const picker = await page.evaluate(() => {
      const root = document.getElementById('age');
      if (!root) return null;
      const r = root.getBoundingClientRect();
      return {
        visible: !root.hidden && r.width > 0 && r.height > 0 && getComputedStyle(root).display !== 'none',
        buttons: [...root.querySelectorAll('button')].map((b) => {
          const br = b.getBoundingClientRect();
          return { band: b.dataset.band, w: br.width, h: br.height, label: b.getAttribute('aria-label') };
        }),
        typed: root.querySelectorAll('input, select, textarea, [contenteditable]').length,
        focused: /** @type {HTMLElement} */ (document.activeElement)?.dataset?.band ?? null,
      };
    });
    check('the age picker is shown', picker?.visible, JSON.stringify(picker?.buttons));
    check('the picker has exactly three buttons and nothing to type in', picker?.buttons.length === 3 && picker.typed === 0);
    check('each picker button is at least 96x96 CSS px at 1366x768', picker?.buttons.every((b) => b.w >= 96 && b.h >= 96));
    check('the picker takes focus on its first button', picker?.focused === 'sprout', String(picker?.focused));
    await page.click('#age button[data-band="seedling"]');
    const hiddenAfter = await page.evaluate(() => document.getElementById('age')?.hidden === true);
    check('the picker closes after a pick', hiddenAfter);

    const st = () => page.evaluate(() => window.__game.state());
    const cmd = (c) => page.evaluate((x) => window.__game.cmd(x), c);
    const until = async (pred, ms = 30000) => {
      const t0 = Date.now();
      let x = await st();
      while (!pred(x) && Date.now() - t0 < ms) {
        await sleep(150);
        x = await st();
      }
      return x;
    };
    await firstState(page);
    // The picker queues one reset for the first state; let it land, then
    // start from a fresh courtyard.
    await sleep(3000);
    let s = await until((x) => x.outcome === 'playing' && x.burning === 3 && x.casts === 0, 30000);
    check('the game starts fresh after the pick', s.outcome === 'playing' && s.burning === 3, `burning=${s.burning} casts=${s.casts}`);
    const lit = s.objs.filter((o) => o.burning).map((o) => o.i);
    await cmd('tp 0 5.5');
    await until((q) => Math.abs(q.pos[0]) < 0.5 && Math.abs(q.pos[2] - 5.5) < 0.5, 10000);
    for (const i of lit) {
      await cmd(`face ${i}`);
      await cmd('select 2');
      const a = await until((x) => x.spell === 1 && x.aimObj === i, 15000);
      await cmd('cast');
      await until((x) => x.impacts > a.impacts || x.outcome !== 'playing', 60000);
      await until((x) => !x.objs[i].burning || x.outcome !== 'playing', 15000);
    }
    s = await until((x) => x.outcome === 'won', 60000);
    check('Save the Greenhouse is won', s.outcome === 'won', `ext=${s.extinguished} burning=${s.burning}`);
    const mainEvents = () => parsed(main);
    await waitFor(() => mainEvents().some((e) => e.type === 'quest_complete'), 5000);
    // session_end on hide: force a hidden visibility event (headless pages are
    // never hidden), then close the page, which also fires pagehide.
    await page.evaluate(() => {
      Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'hidden' });
      document.dispatchEvent(new Event('visibilitychange'));
    });
    await waitFor(() => mainEvents().some((e) => e.type === 'session_end'), 5000);
    check('main run: 0 console errors and 0 page errors', main.errors.length === 0, main.errors.slice(0, 5).join(' | '));
    await page.close({ runBeforeUnload: true });
    await sleep(1000);

    // ---------------------------------------------------- 3. keyboard run
    const kb = await openRun(browser, origin);
    await kb.page.goto(`${origin}/?${QUERY}`);
    await kb.page.waitForSelector('#age', { state: 'visible', timeout: 60000 }).catch(() => undefined);
    const focusedBand = () => kb.page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.dataset?.band ?? null);
    // Let the game finish booting behind the picker first (it focuses its
    // canvas when it does): the picker must keep focus and stay open.
    await firstState(kb.page);
    await sleep(1000);
    const f0 = await focusedBand();
    const stillOpen = await kb.page.evaluate(() => document.getElementById('age')?.hidden === false);
    check('keyboard: the picker keeps focus and stays open while the game boots behind it', f0 === 'sprout' && stillOpen, `focus=${f0} open=${stillOpen}`);
    await kb.page.keyboard.press('ArrowRight');
    const f1 = await focusedBand();
    await kb.page.keyboard.press('Tab');
    const f2 = await focusedBand();
    check('keyboard: arrow and Tab reach the third picture', f0 === 'sprout' && f1 === 'seedling' && f2 === 'apprentice', `${f0} -> ${f1} -> ${f2}`);
    await kb.page.keyboard.press('Enter');
    const kbClosed = await kb.page.evaluate(() => document.getElementById('age')?.hidden === true);
    check('keyboard: Enter picks it and closes the picker', kbClosed);
    await waitFor(() => parsed(kb).some((e) => e.type === 'session_start'), 10000);
    await kb.page.close({ runBeforeUnload: true });
    await sleep(1000);

    // ---------------------------------------------------- 4. opt-out runs
    const optOuts = {
      GPC: () => Object.defineProperty(Navigator.prototype, 'globalPrivacyControl', { configurable: true, get: () => true }),
      DNT: () => Object.defineProperty(Navigator.prototype, 'doNotTrack', { configurable: true, get: () => '1' }),
    };
    const optRuns = [];
    for (const [name, init] of Object.entries(optOuts)) {
      const run = await openRun(browser, origin, { init });
      optRuns.push(run);
      await run.page.goto(`${origin}/?${QUERY}`);
      const t = await firstState(run.page);
      const shown = await run.page.evaluate(() => {
        const root = document.getElementById('age');
        return !!root && !root.hidden && root.getBoundingClientRect().height > 0;
      });
      while (Date.now() - t < 10000) await sleep(250);
      check(`${name}: no picker and no /api/event request within 10 s of the first state`, !shown && run.events.length === 0, `picker=${shown} events=${run.events.length}`);
      await run.page.close({ runBeforeUnload: true });
    }
    await sleep(2000);

    // ------------------------------------------------- 7. wire privacy
    const runs = [
      ['main', main],
      ['keyboard', kb],
      ...optRuns.map((r, i) => [`opt-out ${i}`, r]),
    ];
    for (const [name, run] of runs) {
      const evs = parsed(run);
      const shapes = evs.every((e) => {
        const allowed = ALLOWED[e.type];
        const keys = Object.keys(e);
        return allowed && keys.length === allowed.length && keys.every((k) => allowed.includes(k)) && e.v === 1 && ID_RE.test(e.id);
      });
      check(`wire (${name}): every event body has exactly its allowed keys, v=1 and a 32-hex id`, shapes, JSON.stringify(evs).slice(0, 600));
      check(`wire (${name}): all events share one id`, new Set(evs.map((e) => e.id)).size <= 1);
      const foreign = run.requests.filter((u) => !/^(data|blob):/.test(u) && new URL(u).origin !== origin);
      check(`wire (${name}): no request to another origin`, foreign.length === 0, foreign.slice(0, 3).join(' '));
    }
    const mainId = parsed(main)[0]?.id ?? '';
    const kbId = parsed(kb)[0]?.id ?? '';
    check('the two runs have different daily ids', ID_RE.test(mainId) && ID_RE.test(kbId) && mainId !== kbId);

    // --------------------------------------------- 6. dashboard assertions
    const noToken = await fetch(`${origin}/api/stats`);
    check('GET /api/stats without the token is 401', noToken.status === 401, String(noToken.status));
    const statsRes = await fetch(`${origin}/api/stats`, { headers: { Authorization: `Bearer ${TOKEN}` } });
    const stats = statsRes.status === 200 ? await statsRes.json() : null;
    check('GET /api/stats with the token is 200', statsRes.status === 200, String(statsRes.status));
    check('stats: players.today === 2', stats?.players?.today === 2, String(stats?.players?.today));
    check(
      'stats: todayByBand seedling 1, apprentice 1',
      stats?.players?.todayByBand?.seedling === 1 && stats?.players?.todayByBand?.apprentice === 1,
      JSON.stringify(stats?.players?.todayByBand),
    );
    check('stats: funnel.started.seedling === 1', stats?.funnel?.started?.seedling === 1, JSON.stringify(stats?.funnel?.started));
    check('stats: funnel.quests[0].seedling === 1', stats?.funnel?.quests?.[0]?.seedling === 1, JSON.stringify(stats?.funnel?.quests));
    const durSum = (stats?.durations ?? []).reduce((a, d) => a + d.players, 0);
    check('stats: durations sum to 1 or more', durSum >= 1, String(durSum));

    // Server round trip for a non-zero quest index (the game has quest 0
    // only), so a server that stored every quest as 0 would be caught.
    const probeId = randomBytes(16).toString('hex');
    const probe = (quest) =>
      fetch(`${origin}/api/event`, {
        method: 'POST',
        headers: { 'Content-Type': 'text/plain' },
        body: JSON.stringify({ v: 1, type: 'quest_complete', band: 'sprout', quest, id: probeId }),
      }).then((r) => r.status);
    const [p7, p16] = [await probe(7), await probe(16)];
    check('server: quest 7 is accepted, quest 16 is rejected', p7 === 204 && p16 === 400, `${p7} ${p16}`);

    // ------------------------------------------------- 5. D1 assertions
    process.kill(-pages.pid, 'SIGTERM');
    await new Promise((r) => (pages.exitCode !== null ? r() : pages.once('exit', r)));
    pages = null;
    const rows = d1(persist, 'SELECT daily_id, type, band, room, dur FROM events ORDER BY id');
    const of = (id, type) => rows.filter((r) => r.daily_id === id && r.type === type);
    const mStart = of(mainId, 'session_start');
    const mQuest = of(mainId, 'quest_complete');
    const mEnd = of(mainId, 'session_end');
    check('D1 main: exactly 1 session_start (seedling)', mStart.length === 1 && mStart[0].band === 'seedling', JSON.stringify(mStart));
    check('D1 main: exactly 1 quest_complete (seedling, room 0)', mQuest.length === 1 && mQuest[0].band === 'seedling' && mQuest[0].room === 0, JSON.stringify(mQuest));
    check(
      'D1 main: 1 or more session_end (seedling, valid bucket)',
      mEnd.length >= 1 && mEnd.every((r) => r.band === 'seedling' && Number.isInteger(r.dur) && r.dur >= 0 && r.dur < BUCKETS.length),
      JSON.stringify(mEnd),
    );
    const kStart = of(kbId, 'session_start');
    check('D1 keyboard: exactly 1 session_start (apprentice)', kStart.length === 1 && kStart[0].band === 'apprentice', JSON.stringify(kStart));
    check('D1 keyboard: no quest_complete', of(kbId, 'quest_complete').length === 0);
    check('D1 keyboard: at most 1 session_end', of(kbId, 'session_end').length <= 1);
    const probeRows = rows.filter((r) => r.daily_id === probeId);
    check('D1 probe: quest 7 stored as room 7, quest 16 not stored', probeRows.length === 1 && probeRows[0].room === 7, JSON.stringify(probeRows));
    const others = rows.filter((r) => ![mainId, kbId, probeId].includes(r.daily_id));
    check('D1: no rows with any other daily_id (the opt-out runs left none)', others.length === 0, JSON.stringify(others).slice(0, 300));
    const cols = d1(persist, 'PRAGMA table_info(events)').map((r) => r.name);
    check('D1: events has exactly id, day, daily_id, type, band, room, dur', JSON.stringify(cols) === JSON.stringify(['id', 'day', 'daily_id', 'type', 'band', 'room', 'dur']), cols.join(','));

    // ------------------------------- 8. review follow-ups (I-022, 12-14)
    // The D1 checks above stopped the stack; start it again on the same D1
    // directory (a new port) for these runs. Nothing below reads D1.
    const port2 = await freePort();
    const origin2 = `http://127.0.0.1:${port2}`;
    pages = spawn(
      'npx',
      ['--no-install', 'wrangler', 'pages', 'dev', 'web/dist', '--ip', '127.0.0.1', '--port', String(port2), '--persist-to', persist, '--binding', `DASHBOARD_TOKEN=${TOKEN}`],
      { cwd: repo, env: { ...process.env, CI: '1', WRANGLER_SEND_METRICS: 'false' }, detached: true, stdio: ['ignore', 'pipe', 'pipe'] },
    );
    let wlog2 = '';
    pages.stdout.on('data', (d) => (wlog2 += d));
    pages.stderr.on('data', (d) => (wlog2 += d));
    let up2 = false;
    for (const t0 = Date.now(); !up2 && Date.now() - t0 < 120000;) {
      up2 = await fetch(`${origin2}/`).then((r) => r.status === 200).catch(() => false);
      if (!up2) await sleep(500);
    }
    if (!check('wrangler pages dev serves / again for the follow-up runs', up2, up2 ? origin2 : wlog2.slice(-1500))) throw new Error('no local stack (second start)');

    // 12. One id across UTC midnight: the id is read once per session.
    {
      const pwVersion = require('playwright/package.json').version;
      const [maj, min] = pwVersion.split('.').map(Number);
      if (!check('playwright has page.clock (1.45 or newer)', maj > 1 || (maj === 1 && min >= 45), pwVersion)) throw new Error('page.clock missing');
      const run = await openRun(browser, origin2);
      const p = run.page;
      await p.clock.setFixedTime(new Date('2026-10-29T23:59:30Z'));
      await p.goto(`${origin2}/?${QUERY}`);
      await p.waitForSelector('#age', { state: 'visible', timeout: 60000 }).catch(() => undefined);
      await p.click('#age button[data-band="seedling"]');
      await firstState(p);
      await waitFor(() => parsed(run).some((e) => e.type === 'session_start'), 15000);
      await p.clock.setFixedTime(new Date('2026-10-30T00:00:30Z'));
      await p.evaluate(() => window.dispatchEvent(new Event('pagehide')));
      await waitFor(() => parsed(run).some((e) => e.type === 'session_end'), 10000);
      const evs = parsed(run);
      const s0 = evs.find((e) => e.type === 'session_start');
      const e0 = evs.find((e) => e.type === 'session_end');
      check(
        'midnight: session_start and session_end carry the same id',
        !!s0 && !!e0 && ID_RE.test(s0.id) && s0.id === e0.id,
        `start=${s0?.id} end=${e0?.id}`,
      );
      check('midnight: session_end still carries a valid duration (performance.now is not faked)', !!e0 && BUCKETS.includes(e0.dur), String(e0?.dur));
      check('midnight: 0 console errors and 0 page errors', run.errors.length === 0, run.errors.slice(0, 3).join(' | '));
      await run.context.close();
    }

    // 13 and 14. The picker frees a lock taken before it appeared, and the
    // game behind it is inert. age.js is held back until the lock is taken.
    {
      const run = await openRun(browser, origin2);
      const p = run.page;
      let release = () => undefined;
      let capped = false;
      const held = new Promise((r) => {
        release = r;
      });
      const cap = setTimeout(() => {
        capped = true;
        release();
      }, 60000);
      await p.route('**/age.js', async (route) => {
        await held;
        await route.continue();
      });
      await p.goto(`${origin2}/?${QUERY}`);
      await firstState(p);
      const vp = p.viewportSize() ?? { width: 1366, height: 768 };
      const cx = Math.round(vp.width / 2);
      const cy = Math.round(vp.height / 2);
      const clear = async () => {
        for (const t0 = Date.now(); Date.now() - t0 < 30000;) {
          const ok = await p.evaluate(([x, y]) => {
            const l = document.getElementById('loading');
            return (!l || l.classList.contains('hidden')) && document.elementFromPoint(x, y)?.id === 'game';
          }, [cx, cy]);
          if (ok) return true;
          await sleep(200);
        }
        return false;
      };
      const canvasClear = await clear();
      const lockedBefore = async () => {
        await p.mouse.click(cx, cy);
        for (const t0 = Date.now(); Date.now() - t0 < 5000;) {
          if (await p.evaluate(() => document.pointerLockElement?.id === 'game')) return true;
          await sleep(100);
        }
        return false;
      };
      const lk = canvasClear && (await lockedBefore());
      const pickerHeld = await p.evaluate(() => document.getElementById('age')?.hidden === true);
      check('picker: the canvas takes the pointer lock while age.js is held back', canvasClear && lk && pickerHeld && !capped, `clear=${canvasClear} locked=${lk} pickerHidden=${pickerHeld}`);
      clearTimeout(cap);
      release();
      await p.waitForSelector('#age', { state: 'visible', timeout: 60000 }).catch(() => undefined);
      const shownAt = Date.now();
      let freed = false;
      while (!freed && Date.now() - shownAt < 1000) {
        freed = await p.evaluate(() => document.pointerLockElement === null);
        if (!freed) await sleep(50);
      }
      const visible = await p.evaluate(() => document.getElementById('age')?.hidden === false);
      check('picker: it frees the pointer lock within 1 s of appearing', !capped && visible && freed, `visible=${visible} freed=${freed} capped=${capped}`);
      const ids = ['stage', 'hud', 'loading', 'pause'];
      const inertOpen = await p.evaluate((ids) => {
        const age = document.getElementById('age');
        return {
          game: ids.map((id) => [id, document.getElementById(id)?.inert ?? null]),
          age: age?.inert ?? null,
          ageAncestor: age ? age.closest('[inert]') !== null : null,
        };
      }, ids);
      check(
        'picker: #stage, #hud, #loading and #pause are inert while it is open; #age is not',
        inertOpen.game.every(([, v]) => v === true) && inertOpen.age === false && inertOpen.ageAncestor === false,
        JSON.stringify(inertOpen),
      );
      await p.click('button[data-band="sprout"]');
      const closed = await p.evaluate(() => document.getElementById('age')?.hidden === true);
      const startSent = await waitFor(() => parsed(run).some((e) => e.type === 'session_start' && e.band === 'sprout'), 10000);
      check('picker: a pick closes it and sends session_start (sprout)', closed && startSent, `closed=${closed} sent=${startSent}`);
      const after = await p.evaluate((ids) => ({
        game: ids.map((id) => [id, document.getElementById(id)?.inert ?? null]),
        active: document.activeElement?.id ?? null,
      }), ids);
      check(
        'picker: after the pick nothing is inert and the canvas has focus',
        after.game.every(([, v]) => v === false) && after.active === 'game',
        JSON.stringify(after),
      );
      check('picker run: 0 console errors and 0 page errors', run.errors.length === 0, run.errors.slice(0, 3).join(' | '));
      await run.context.close();
    }

    // 15. The age picker's band reaches the Formula Bench (I-028 test 29):
    // "8 to 10" in the picker, then B, shows the Seedling list (4 spells) with
    // no chooser; with DNT (no picker) the bench asks with its own chooser.
    {
      const benchOpen = async (p) => {
        await p.evaluate(() => document.getElementById('game')?.focus());
        await p.keyboard.press('KeyB');
        for (const t0 = Date.now(); Date.now() - t0 < 10000; await sleep(200)) {
          const v = await p.evaluate(() => ({
            open: !!document.getElementById('bench') && !document.getElementById('bench').hidden,
            chooser: document.querySelectorAll('#bench .bench-band[data-band]').length,
            cards: document.querySelectorAll('#bench .bench-spell').length,
          }));
          if (v.open && (v.chooser > 0 || v.cards > 0)) return v;
        }
        return p.evaluate(() => ({ open: !document.getElementById('bench')?.hidden, chooser: -1, cards: -1 }));
      };
      const run = await openRun(browser, origin2);
      const p = run.page;
      await p.goto(`${origin2}/?${QUERY}`);
      await p.waitForSelector('#age', { state: 'visible', timeout: 60000 }).catch(() => undefined);
      await p.click('#age button[data-band="seedling"]');
      await firstState(p);
      await sleep(1000);
      const v = await benchOpen(p);
      check('bench: a picker pick (8 to 10) opens the bench on the Seedling list, no chooser', v.open && v.chooser === 0 && v.cards === 4, JSON.stringify(v));
      check('bench run: 0 console errors and 0 page errors', run.errors.length === 0, run.errors.slice(0, 3).join(' | '));
      await run.context.close();
      const dnt = await openRun(browser, origin2, {
        init: () => Object.defineProperty(Navigator.prototype, 'doNotTrack', { configurable: true, get: () => '1' }),
      });
      await dnt.page.goto(`${origin2}/?${QUERY}`);
      await firstState(dnt.page);
      await sleep(1000);
      const d = await benchOpen(dnt.page);
      check('bench: with DNT (no picker) the bench asks with its chooser', d.open && d.chooser === 3, JSON.stringify(d));
      await dnt.context.close();
    }
  } catch (err) {
    check('run completed without an exception', false, String(err && err.stack ? err.stack : err));
  } finally {
    if (browser) await browser.close().catch(() => undefined);
    if (pages) {
      try {
        process.kill(-pages.pid, 'SIGTERM');
      } catch {
        // already gone
      }
    }
    if (process.env.KEEP !== '1') rmSync(persist, { recursive: true, force: true });
  }
  const failed = results.filter((r) => !r.ok);
  console.log(`\n${results.length - failed.length}/${results.length} checks passed`);
  if (failed.length) {
    console.log(`FAILED: ${failed.map((f) => f.name).join('; ')}`);
    process.exit(1);
  }
  process.exit(0);
})();
