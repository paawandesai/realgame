// The panel session daemon: one Chromium page that stays alive between a
// persona's turns (each turn is a separate CLI call, often tens of seconds
// apart). Started detached by `panel.mjs start`; listens on 127.0.0.1 and
// takes one command at a time. See docs/loop/panel.md.
//
//   node daemon.mjs --run <dir> --persona <id> --throttle <n> --query <q>
//                   --vw <w> --vh <h> [--base-url <url>] [--root <dir>]
//                   [--artifact <sha>] [--freeze 1|0]
import { createServer } from 'node:http';
import { readFile, writeFile, appendFile, mkdir, rm } from 'node:fs/promises';
import { extname, join, normalize, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { costMs, MAX_WAIT_MS, MAX_HOLD_MS, DEFAULT_KEY_MS, DEFAULT_DRAG_MS } from './clock.mjs';
import { TURN_CAP, TIME_CAP_S } from './personas.mjs';
import { SEL, STATE_GLOBAL, CONSOLE, readView, truthSummary } from './hud.mjs';

const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const here = dirname(fileURLToPath(import.meta.url));

const argv = process.argv.slice(2);
const opt = (k, d) => { const i = argv.indexOf(`--${k}`); return i >= 0 ? argv[i + 1] : d; };
const RUN = resolve(opt('run'));
const PERSONA = opt('persona', '9yo');
const THROTTLE = +opt('throttle', '1');
const QUERY = opt('query', '');
const VW = +opt('vw', '1280'); const VH = +opt('vh', '720');
const ROOT = resolve(opt('root', join(here, '../../../crates/bevy-client/web/pkg-out')));
const BASE_URL = opt('base-url');
const ARTIFACT = opt('artifact', 'unknown');
// Freeze: pause the page's JavaScript between turns, so the game does not
// run on while a model thinks (see panel.md, "Game clock between turns").
const FREEZE = opt('freeze', '1') === '1';
const SELFTEST = process.env.PANEL_SELFTEST === '1';
const IDLE_EXIT_MS = 60 * 60 * 1000;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (...a) => appendFile(join(RUN, 'daemon.log'), `${new Date().toISOString()} ${a.join(' ')}\n`);
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };

async function serveStatic() {
  const server = createServer(async (req, res) => {
    const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '');
    if (path.startsWith('..')) { res.writeHead(403).end(); return; }
    try {
      const body = await readFile(join(ROOT, path || 'index.html'));
      res.writeHead(200, { 'content-type': types[extname(path || 'index.html')] || 'application/octet-stream' });
      res.end(body);
    } catch {
      res.writeHead(404).end();
    }
  });
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  return server;
}

let browser; let page; let cdp; let staticServer; let api;
let turn = 0; let panelMs = 0; let cap = null;
let errors = 0; let panics = 0; let errorsAtLastTurn = 0;
let lastCpu = null; let lastStats = null;
const mouse = { x: VW / 2, y: VH / 2 };
let lastActivity = Date.now();

// Tracked from CDP events, so a resume is sent exactly when the page is paused.
let pausedNow = false;
async function freeze() {
  if (!FREEZE || pausedNow) return;
  try {
    // The pause lands on the next JavaScript the page runs (the game's frame
    // loop runs constantly); wait for it.
    await cdp.send('Debugger.pause');
    const t0 = Date.now();
    while (!pausedNow && Date.now() - t0 < 8000) await sleep(20);
    if (!pausedNow) await log('freeze: no pause within 8 s');
  } catch (e) { await log('freeze failed', e.message); }
}
async function thaw() {
  if (!FREEZE) return;
  // A pause request that has not landed yet would stop the next evaluate:
  // give it a moment to land, then resume.
  const t0 = Date.now();
  while (!pausedNow && Date.now() - t0 < 300) await sleep(20);
  if (pausedNow) {
    await cdp.send('Debugger.resume').catch((e) => log('resume failed', e.message));
    const t1 = Date.now();
    while (pausedNow && Date.now() - t1 < 5000) await sleep(10);
    if (pausedNow) await log('thaw: still paused after 5 s');
  } else {
    await log('thaw: page was not paused');
  }
}

const state = () => page.evaluate((g) => (window[g] ? JSON.parse(window[g]) : null), STATE_GLOBAL);
const until = async (pred, ms) => {
  const t0 = Date.now();
  let s = await state();
  while (!pred(s) && Date.now() - t0 < ms) { await sleep(100); s = await state(); }
  return s;
};
const view = () => page.evaluate(readView, SEL);
const summary = async () => { const v = await view(); return { v, t: truthSummary(await state(), v.quest) }; };

async function record(entry) {
  await appendFile(join(RUN, 'log.jsonl'), `${JSON.stringify({ ...entry, errors, panics, ts: new Date().toISOString() })}\n`);
}

function capCheck() {
  if (cap) return cap;
  if (turn >= TURN_CAP) cap = 'turn_cap';
  else if (panelMs >= TIME_CAP_S * 1000) cap = 'time_cap';
  return cap;
}

const KEY_RE = /^([a-z0-9]|Digit[0-9]|Key[A-Z]|Arrow(Up|Down|Left|Right)|Space|Shift|Control|Alt|Escape|Enter|Tab|Backspace)$/;
const num = (v, name, lo, hi) => {
  const n = Number(v);
  if (v === undefined || !Number.isFinite(n)) throw new Error(`${name}: expected a number, got ${v}`);
  if ((lo !== undefined && n < lo) || (hi !== undefined && n > hi)) throw new Error(`${name}: must be between ${lo} and ${hi}, got ${n}`);
  return n;
};
const key = (k) => { if (!KEY_RE.test(k || '')) throw new Error(`unknown key "${k}"`); return k; };
const ACTIONS = new Set(['key', 'keys', 'move', 'drag', 'click', 'hold', 'wheel', 'wait']);

// Checks an action's arguments before anything runs (a refused action is not
// a turn and costs no panel time).
function parseAct(action, a) {
  if (!ACTIONS.has(action)) throw new Error(`unknown action "${action}" (real input only: ${[...ACTIONS].join(', ')})`);
  switch (action) {
    case 'key': key(a[0]); if (a[1] !== undefined) num(a[1], 'ms', 1, MAX_HOLD_MS); break;
    case 'keys': (a[0] || '').split('+').forEach(key); num(a[1], 'ms', 1, MAX_HOLD_MS); break;
    case 'move': num(a[0], 'dx', -5000, 5000); num(a[1], 'dy', -5000, 5000); if (a[2] !== undefined) num(a[2], 'steps', 1, 100); break;
    case 'drag': num(a[0], 'x1', 0, VW); num(a[1], 'y1', 0, VH); num(a[2], 'x2', 0, VW); num(a[3], 'y2', 0, VH); if (a[4] !== undefined) num(a[4], 'ms', 1, MAX_HOLD_MS); break;
    case 'click': num(a[0], 'x', 0, VW); num(a[1], 'y', 0, VH); if (a[2] !== undefined && !['left', 'right'].includes(a[2])) throw new Error('button: left or right'); break;
    case 'hold': num(a[0], 'x', 0, VW); num(a[1], 'y', 0, VH); num(a[2], 'ms', 1, MAX_HOLD_MS); break;
    case 'wheel': num(a[0], 'dy', -2000, 2000); break;
    case 'wait': num(a[0], 'ms', 1, MAX_WAIT_MS); break;
    default:
  }
}

const isLocked = () => page.evaluate(() => !!document.pointerLockElement);
async function pointTo(x, y, steps = 1) {
  await page.mouse.move(x, y, { steps });
  mouse.x = x; mouse.y = y;
}

// Performs one act with real input. Returns how it was delivered.
async function perform(action, a) {
  switch (action) {
    case 'key': {
      await page.keyboard.down(a[0]); await sleep(a[1] !== undefined ? +a[1] : DEFAULT_KEY_MS); await page.keyboard.up(a[0]);
      return 'real';
    }
    case 'keys': {
      const ks = a[0].split('+');
      for (const k of ks) await page.keyboard.down(k);
      await sleep(+a[1]);
      for (const k of ks.reverse()) await page.keyboard.up(k);
      return 'real';
    }
    case 'move': {
      const dx = +a[0]; const dy = +a[1]; const steps = a[2] !== undefined ? +a[2] : 10;
      if (await isLocked()) {
        const y0 = (await state()).yaw; const p0 = (await state()).pitch;
        await pointTo(mouse.x + dx, mouse.y + dy, steps);
        const s = await until((x) => Math.abs(x.yaw - y0) > 0.05 || Math.abs(x.pitch - p0) > 0.05, 1500);
        if (dx !== 0 || dy !== 0) {
          if (Math.abs(s.yaw - y0) <= 0.05 && Math.abs(s.pitch - p0) <= 0.05) {
            // Headless Chromium may report movementX = 0 under pointer lock:
            // feed the same document mousemove handler (golden.mjs fallback).
            await page.evaluate(([mx, my]) => document.dispatchEvent(new MouseEvent('mousemove', { movementX: mx, movementY: my })), [dx, dy]);
            return 'synthetic';
          }
        }
        return 'real';
      }
      await pointTo(Math.max(0, Math.min(VW - 1, mouse.x + dx)), Math.max(0, Math.min(VH - 1, mouse.y + dy)), steps);
      return 'real';
    }
    case 'drag': {
      await pointTo(+a[0], +a[1]);
      await page.mouse.down();
      const ms = a[4] !== undefined ? +a[4] : DEFAULT_DRAG_MS;
      const steps = Math.max(2, Math.round(ms / 50));
      for (let k = 1; k <= steps; k++) {
        await pointTo(+a[0] + ((+a[2] - +a[0]) * k) / steps, +a[1] + ((+a[3] - +a[1]) * k) / steps);
        await sleep(ms / steps);
      }
      await page.mouse.up();
      return 'real';
    }
    case 'click': case 'hold': {
      const button = action === 'click' ? (a[2] || 'left') : 'left';
      // Under pointer lock the cursor has no position: press in place (a
      // move first would send a stray movementX and turn the view).
      if (!(await isLocked())) {
        // A hand takes a moment to reach the spot: let the game see the
        // cursor there (hover, hand cursor) before the press, as it would
        // for a person. Wall time only; the panel clock counts the click.
        const f0 = (await state()).frame;
        await pointTo(+a[0], +a[1]);
        await until((x) => x.frame >= f0 + 2, 2000);
      }
      await page.mouse.down({ button });
      await sleep(action === 'hold' ? +a[2] : 60);
      await page.mouse.up({ button });
      return 'real';
    }
    case 'wheel': await page.mouse.wheel(0, +a[0]); return 'real';
    case 'wait': await sleep(+a[0]); return 'real';
    default: throw new Error('unreachable');
  }
}

// Gives the game a moment (wall time, not panel time) to show the input's
// effect: a couple of frames, and a projectile in flight a chance to land.
async function settle(before) {
  const f0 = before.frame;
  let s = await until((x) => x.frame >= f0 + 2, 3000);
  if (s.casts > before.casts && s.impacts < before.impacts + (s.casts - before.casts)) {
    s = await until((x) => x.impacts >= before.impacts + (x.casts - before.casts), 4000);
  }
  return s;
}

async function shot(path) {
  // CDP capture: Playwright's page.screenshot can drop pointer lock in
  // headless Chromium (golden.mjs notes it), which would pause the game.
  const { data } = await cdp.send('Page.captureScreenshot', { format: 'png' });
  await writeFile(path, Buffer.from(data, 'base64'));
}

function consoleNew() {
  const n = errors - errorsAtLastTurn;
  errorsAtLastTurn = errors;
  return n;
}

const handlers = {
  async observe({ truth }) {
    if (capCheck()) throw Object.assign(new Error(`${cap} reached: run stop, then report`), { cap });
    await thaw();
    turn++;
    panelMs += costMs({ cmd: 'observe' }, PERSONA);
    const name = `${String(turn).padStart(4, '0')}.png`;
    await mkdir(join(RUN, 'shots'), { recursive: true });
    await shot(join(RUN, 'shots', name));
    const { v, t } = await summary();
    const out = { ok: true, turn, panel_s: panelMs / 1000, screenshot: join(RUN, 'shots', name), hud: v.hud, pointer_locked: v.pointer_locked, console_errors_new: consoleNew() };
    await record({ turn, cmd: 'observe', panel_s: out.panel_s, view: { hud: v.hud, pointer_locked: v.pointer_locked }, truth: t });
    if (truth) out.truth = { ...(await state()), cpuframe: lastCpu, stats: lastStats };
    return out;
  },
  async act({ action, args = [] }) {
    parseAct(action, args); // throws before the turn counts
    if (capCheck()) throw Object.assign(new Error(`${cap} reached: run stop, then report`), { cap });
    await thaw();
    const before = await state();
    const via = await perform(action, args);
    turn++;
    panelMs += costMs({ cmd: 'act', action, args }, PERSONA);
    await settle(before);
    const { v, t } = await summary();
    const out = { ok: true, turn, panel_s: panelMs / 1000, action: [action, ...args].join(' '), via, console_errors_new: consoleNew() };
    await record({ turn, cmd: 'act', action, args, via, panel_s: out.panel_s, view: { hud: v.hud, pointer_locked: v.pointer_locked }, truth: t });
    return out;
  },
  // Debug-only commands for selftest.mjs (PANEL_SELFTEST=1 when the session
  // was started). Never available to `act`, never counted as a turn.
  async debug({ action, args = [] }) {
    if (!SELFTEST) throw new Error('debug commands need PANEL_SELFTEST=1 at start');
    await thaw();
    if (action === 'face') {
      const i = num(args[0], 'object', 0, 999);
      await page.evaluate((c) => window.__game.cmd(c), `face ${i}`);
      await until((x) => x.aimObj === i || x.hover === i, 8000);
    } else if (action === 'console-error') {
      await page.evaluate((m) => console.error(m), args[0] || 'x');
      await sleep(200);
    } else {
      throw new Error(`unknown debug action "${action}"`);
    }
    const { t } = await summary();
    await record({ turn, cmd: 'debug', action, args, panel_s: panelMs / 1000, truth: t });
    return { ok: true, debug: action, truth: await state() };
  },
  async stop() {
    await thaw();
    let t = null;
    try { t = (await summary()).t; } catch { /* page gone */ }
    await record({ turn, cmd: 'stop', panel_s: panelMs / 1000, truth: t, cap: capCheck() });
    setTimeout(shutdown, 50);
    return { ok: true, stopped: true, turn, panel_s: panelMs / 1000, cap };
  },
  async ping() { return { ok: true, turn, panel_s: panelMs / 1000, frozen: pausedNow }; },
};

let queue = Promise.resolve();
function serial(fn) { const p = queue.then(fn, fn); queue = p.catch(() => {}); return p; }

async function startApi() {
  api = createServer((req, res) => {
    let body = '';
    req.on('data', (c) => { body += c; });
    req.on('end', () => serial(async () => {
      lastActivity = Date.now();
      let out;
      try {
        const msg = JSON.parse(body || '{}');
        const h = handlers[msg.cmd];
        if (!h) throw new Error(`unknown command "${msg.cmd}"`);
        const t0 = Date.now();
        out = await h(msg);
        await log(`cmd ${msg.cmd} ${msg.action || ''} ${(msg.args || []).join(' ')} ok=${out.ok} wall_ms=${Date.now() - t0}`);
      } catch (e) {
        out = { ok: false, error: e.message, ...(e.cap ? { cap: e.cap } : {}) };
      }
      if (!out.stopped) await freeze();
      res.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify(out));
    }));
  });
  await new Promise((r) => api.listen(0, '127.0.0.1', r));
  return api.address().port;
}

let shuttingDown = false;
async function shutdown() {
  if (shuttingDown) return;
  shuttingDown = true;
  try { await thaw(); } catch { /* ignore */ }
  try { await browser?.close(); } catch { /* ignore */ }
  try { staticServer?.close(); } catch { /* ignore */ }
  try { api?.close(); } catch { /* ignore */ }
  await rm(join(RUN, 'session.json'), { force: true });
  await log('daemon exit');
  process.exit(0);
}
process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);

(async () => {
  await mkdir(RUN, { recursive: true });
  await log(`daemon start persona=${PERSONA} throttle=${THROTTLE} query=${QUERY} freeze=${FREEZE} selftest=${SELFTEST}`);
  staticServer = BASE_URL ? null : await serveStatic();
  const base = (BASE_URL || `http://127.0.0.1:${staticServer.address().port}/`) + QUERY;
  // New headless Chromium (channel "chromium"). The default headless shell
  // crashed in this cloud after a pointer-lock click: the renderer grew past
  // 5 GB and was OOM-killed within a minute (round-1 notes). Falls back to
  // the shell if the full build is not installed.
  try {
    browser = await chromium.launch({ channel: 'chromium' });
  } catch (e) {
    await log('channel chromium unavailable, using the headless shell:', e.message.split('\n')[0]);
    browser = await chromium.launch();
  }
  await log(`browser ${browser.version()}`);
  page = await browser.newPage({ viewport: { width: VW, height: VH } });
  cdp = await page.context().newCDPSession(page);
  let cdpThrottle = null;
  if (THROTTLE > 1) {
    await cdp.send('Emulation.setCPUThrottlingRate', { rate: THROTTLE });
    cdpThrottle = THROTTLE;
    await log(`CDP Emulation.setCPUThrottlingRate rate=${THROTTLE}`);
  }
  if (FREEZE) {
    cdp.on('Debugger.paused', () => { pausedNow = true; });
    cdp.on('Debugger.resumed', () => { pausedNow = false; });
    await cdp.send('Debugger.enable');
  }
  page.on('console', (m) => {
    const t = m.text().replace(/%c/g, '').replace(/ color: .*$/, '');
    if (m.type() === 'error') errors++;
    if (CONSOLE.panic.test(t)) panics++;
    if (CONSOLE.cpu.test(t)) lastCpu = t;
    if (CONSOLE.stats.test(t)) lastStats = t;
    if (CONSOLE.firstFrame.test(t)) log(t);
  });
  page.on('pageerror', (e) => { errors++; log('pageerror', e.message); });
  await page.goto(base);
  await page.waitForFunction((g) => window[g], STATE_GLOBAL, { timeout: 180000 });
  // A fresh courtyard for turn 1: loading takes long on a software GPU and the
  // fires spread meanwhile.
  await page.evaluate(() => window.__game.cmd('reset'));
  await until((x) => x && x.casts === 0 && x.outcome === 'playing', 20000);
  await pointTo(VW / 2, VH / 2);
  const { t } = await summary();
  const port = await startApi();
  await record({ turn: 0, cmd: 'start', persona: PERSONA, throttle: THROTTLE, query: QUERY, artifact_sha: ARTIFACT, freeze: FREEZE, truth: t });
  await freeze();
  const session = { pid: process.pid, port, persona: PERSONA, throttle: THROTTLE, cdp_throttle_rate: cdpThrottle, query: QUERY, artifact_sha: ARTIFACT, freeze: FREEZE, vw: VW, vh: VH, base_url: base, started_at: new Date().toISOString() };
  await writeFile(join(RUN, 'session.json'), JSON.stringify(session, null, 2));
  await log(`ready port=${port}`);
  setInterval(() => { if (Date.now() - lastActivity > IDLE_EXIT_MS) { log('idle exit'); shutdown(); } }, 60000).unref();
})().catch(async (e) => {
  await writeFile(join(RUN, 'start-error.json'), JSON.stringify({ ok: false, error: String(e && e.message || e) }));
  await log('start failed', e && e.stack);
  try { await browser?.close(); } catch { /* ignore */ }
  process.exit(1);
});
