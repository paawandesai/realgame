// Perf script for the first-person client (crates/bevy-client): bars B3
// (download size, time to first interactive frame) and the cloud half of B2
// (median CPU frame ms with `?low` at 4x CPU throttle). `--mac` runs the
// real-GPU half of B2 (p5 fps on a Mac with Chrome); see docs/loop/mac-perf.md.
//
//   bash crates/bevy-client/build-web.sh          # builds web/pkg-out
//   node tests/e2e/perf.mjs [WEB_DIR]             # JSON verdict on stdout
//   node tests/e2e/perf.mjs --mac [WEB_DIR]       # real-GPU run (Mac, Chrome)
//
// Env: BASE_URL (test a running server; size still reads WEB_DIR), SAMPLE_S
// (sampling window per run, 20), WARMUP_S (3), RUNS (repeats per
// configuration, median reported, 1), OUT (also write the JSON there), ONLY
// (comma-separated subset of default@1x,default@4x,low@1x,low@4x),
// TTI_TIMEOUT_S (120), MIN_FRAMES (30: the window is extended past SAMPLE_S
// until it holds this many frames; a software GPU can draw under one frame a
// second) and MAX_SAMPLE_S (150: the cap on that). Mac mode: MAC_SAMPLE_S
// (60), MAC_VW/MAC_VH (1512x945), MAC_CHANNEL (chrome; `chromium` uses Playwright's bundled
// Chromium), MAC_HEADLESS (0). PERF_NO_CAST=1 drops casts from the workload
// (only for the acceptance test that proves the workload check can fail).
// Flags: --bars k=v,... overrides thresholds (b3_brotli_mb, b3_tti_ms,
// b2_cpu_p50_ms, b2_mac_p5_fps).
//
// Exit: 0 pass, 1 a bar failed or a run was invalid, 2 usage or environment
// error. Progress goes to stderr. Needs `playwright` (no package.json entry).
import { createServer } from 'node:http';
import { readFile, readdir, stat } from 'node:fs/promises';
import { writeFileSync } from 'node:fs';
import { extname, join, normalize, dirname, resolve, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { brotliCompressSync, gzipSync, constants as Z } from 'node:zlib';
import os from 'node:os';

const require = createRequire(import.meta.url);
const here = dirname(fileURLToPath(import.meta.url));
const log = (...a) => console.error('[perf]', ...a);
const die = (msg) => { console.error(`perf: ${msg}`); process.exit(2); };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const env = process.env;

// ---- arguments ----------------------------------------------------------
const argv = process.argv.slice(2);
let mac = false;
const barOverrides = {};
const positional = [];
for (let i = 0; i < argv.length; i++) {
  const a = argv[i];
  if (a === '--mac') mac = true;
  else if (a === '--bars' || a.startsWith('--bars=')) {
    const v = a === '--bars' ? argv[++i] : a.slice(7);
    if (!v) die('--bars needs k=v,...');
    for (const kv of v.split(',')) {
      const [k, x] = kv.split('=');
      const n = Number(x);
      if (!k || x === undefined || x.trim() === '' || !Number.isFinite(n)) die(`bad --bars entry: ${kv}`);
      barOverrides[k.trim()] = n;
    }
  } else if (a.startsWith('--')) die(`unknown flag ${a}`);
  else positional.push(a);
}
if (positional.length > 1) die('usage: node tests/e2e/perf.mjs [--mac] [--bars k=v,...] [WEB_DIR]');
const webDir = resolve(positional[0] || join(here, '../../crates/bevy-client/web/pkg-out'));
const BARS = { b3_brotli_mb: 15, b3_tti_ms: 8000, b2_cpu_p50_ms: 16, b2_mac_p5_fps: 55 };
for (const k of Object.keys(barOverrides)) if (!(k in BARS)) die(`unknown bar ${k} (known: ${Object.keys(BARS).join(', ')})`);
Object.assign(BARS, barOverrides);
const numEnv = (k, d) => {
  if (env[k] === undefined || env[k] === '') return d;
  const n = Number(env[k]);
  if (!Number.isFinite(n) || n < 0) die(`${k} must be a non-negative number`);
  return n;
};
const SAMPLE_S = numEnv('SAMPLE_S', 20);
const WARMUP_S = numEnv('WARMUP_S', 3);
const RUNS = Math.max(1, Math.round(numEnv('RUNS', 1)));
const TTI_TIMEOUT_S = numEnv('TTI_TIMEOUT_S', 120);
const MAC_SAMPLE_S = numEnv('MAC_SAMPLE_S', 60);
const MIN_FRAMES = numEnv('MIN_FRAMES', 30);
const MAX_SAMPLE_S = numEnv('MAX_SAMPLE_S', 150);
const MAC_VW = numEnv('MAC_VW', 1512), MAC_VH = numEnv('MAC_VH', 945);
const NO_CAST = env.PERF_NO_CAST === '1';
const ALL_CFGS = ['default@1x', 'default@4x', 'low@1x', 'low@4x'];
const only = env.ONLY ? env.ONLY.split(',').map((s) => s.trim()).filter(Boolean) : null;
if (only) for (const c of only) if (!ALL_CFGS.includes(c)) die(`unknown configuration in ONLY: ${c}`);
const cfgs = mac ? ['mac'] : (only || ALL_CFGS);

// ---- environment checks -------------------------------------------------
try { if (!(await stat(webDir)).isDirectory()) throw 0; } catch { die(`web dir not found: ${webDir} (build it with crates/bevy-client/build-web.sh)`); }
let chromium, pwVersion;
try {
  ({ chromium } = require('playwright'));
  pwVersion = require('playwright/package.json').version;
} catch { die('playwright not installed (npm i --no-save playwright)'); }

// ---- size -----------------------------------------------------------------
async function walk(dir) {
  const out = [];
  for (const e of await readdir(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) out.push(...await walk(p));
    else if (e.isFile()) out.push(p);
  }
  return out;
}
const BR = { params: { [Z.BROTLI_PARAM_QUALITY]: 11, [Z.BROTLI_PARAM_MODE]: Z.BROTLI_MODE_GENERIC, [Z.BROTLI_PARAM_LGWIN]: 24 } };
// gzip -9 size: the system `gzip -9n` when it exists (the reference tool;
// node:zlib's deflate differs from GNU gzip's by about 1% on the .wasm), else
// node:zlib level 9. gzip is informational: the bar is on Brotli.
let gzipTool = 'gzip -9n';
const gzipLen = (buf) => {
  if (gzipTool === 'gzip -9n') {
    try { return execFileSync('gzip', ['-9nc'], { input: buf, maxBuffer: 1 << 30, stdio: ['pipe', 'pipe', 'ignore'] }).length; }
    catch { gzipTool = 'node:zlib level 9'; }
  }
  return gzipSync(buf, { level: 9 }).length;
};
async function sizeOf(dir) {
  const files = [];
  for (const p of (await walk(dir)).sort()) {
    const buf = await readFile(p);
    files.push({
      path: relative(dir, p).split(sep).join('/'),
      raw: buf.length,
      gzip: gzipLen(buf),
      brotli: brotliCompressSync(buf, BR).length,
    });
  }
  const sum = (k) => files.reduce((a, f) => a + f[k], 0);
  return {
    files,
    gzip_tool: gzipTool,
    brotli_params: 'node:zlib quality 11, BROTLI_MODE_GENERIC, lgwin 24',
    raw_total_bytes: sum('raw'),
    gzip_total_bytes: sum('gzip'),
    brotli_total_bytes: sum('brotli'),
    gzip_total_mb: +(sum('gzip') / 1e6).toFixed(3),
    brotli_total_mb: +(sum('brotli') / 1e6).toFixed(3),
    requested: [],
    missing_from_dir: [],
  };
}

// ---- static server --------------------------------------------------------
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
async function serve(root) {
  const server = createServer(async (req, res) => {
    const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '');
    if (path.startsWith('..')) { res.writeHead(403).end(); return; }
    try {
      const body = await readFile(join(root, path || 'index.html'));
      res.writeHead(200, { 'content-type': types[extname(path || 'index.html')] || 'application/octet-stream', 'cache-control': 'no-store' });
      res.end(body);
    } catch {
      res.writeHead(404).end();
    }
  });
  await new Promise((ok, fail) => { server.once('error', fail); server.listen(0, '127.0.0.1', ok); });
  return server;
}

// ---- in-page instrumentation ------------------------------------------------
// Installed before any page script. Wraps requestAnimationFrame before
// boot.js wraps it, so it sees every callback (Bevy's and the HUD's). The CPU
// of one displayed frame is the sum over all callbacks sharing a timestamp.
// Also catches the FIRST_FRAME console line and records the first tick at
// which the game is interactive.
function pageHook() {
  const P = (window.__perf = { ticks: [], firstFrameLog: null, tti: null, cur: null, cpuframe: [], maxTicks: 200000 });
  const now = () => performance.now();
  // Which context types the page asks for (webgl2 for a WebGL2 build, webgpu
  // for a WebGPU one), recorded without creating any context ourselves.
  P.ctx = [];
  const gc = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (type, ...rest) {
    if (!P.ctx.includes(type)) P.ctx.push(type);
    return gc.call(this, type, ...rest);
  };
  if (navigator.gpu) {
    const ra = navigator.gpu.requestAdapter.bind(navigator.gpu);
    navigator.gpu.requestAdapter = (...a) => { if (!P.ctx.includes('webgpu-adapter')) P.ctx.push('webgpu-adapter'); return ra(...a); };
  }
  const flush = () => {
    const c = P.cur;
    if (c && P.ticks.length < P.maxTicks) P.ticks.push([c.ts, c.start, c.cpu]);
  };
  const interactive = () => {
    if (P.firstFrameLog === null || !window.__gameState) return false;
    try {
      const s = JSON.parse(window.__gameState);
      return s && Array.isArray(s.objs) && s.objs.length > 0 && s.outcome === 'playing';
    } catch { return false; }
  };
  const raf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = (cb) => raf((ts) => {
    const t0 = now();
    if (!P.cur || P.cur.ts !== ts) {
      flush();
      P.cur = { ts, start: t0, cpu: 0 };
      if (P.tti === null && interactive()) P.tti = t0;
    }
    try { cb(ts); } finally { P.cur.cpu += now() - t0; }
  });
  const scan = (args) => {
    const t = args.map(String).join(' ');
    if (P.firstFrameLog === null) {
      const m = /FIRST_FRAME t=(\d+(?:\.\d+)?)ms/.exec(t);
      if (m) P.firstFrameLog = +m[1];
    }
  };
  for (const k of ['log', 'info', 'debug', 'warn']) {
    const f = console[k].bind(console);
    console[k] = (...a) => { try { scan(a); } catch {} return f(...a); };
  }
  // Returns ticks with ts in [a, b] (page clock), flushing the current one.
  P.take = (a, b) => P.ticks.filter((t) => t[0] >= a && t[0] <= b);
}

// The fixed golden-path workload, driven from inside the page every 50 ms
// (outside requestAnimationFrame, so it is not counted as frame CPU except
// for what the game itself does with the commands). Deterministic: the same
// command list in the same order every run.
function pageWorkload(noCast) {
  const W = (window.__work = { sampling: false, casts: { 1: 0, 2: 0, 3: 0, 4: 0 }, castsTotal: 0, burningMax: 0, yaw: 0, steps: 0, resets: 0 });
  const st = () => { try { return JSON.parse(window.__gameState); } catch { return null; } };
  const cmd = (c) => window.__game.cmd(c);
  let s0 = st();
  const idx = (name) => (s0.objs.find((o) => o.n === name) || {}).i;
  const burningAny = (s) => (s.objs.find((o) => o.burning && o.n === 'bush') || s.objs.find((o) => o.burning) || {}).i;
  // Step list: each step issues commands, then waits until pred(state) or
  // its timeout (in ms of wall time).
  // Each spell step teleports to the same spot, faces the target, selects
  // the spell and casts, all queued for one frame, then waits for the cast.
  // Each motion step moves and turns, then waits for the next frame. Few
  // frames per cycle, because a software GPU draws only a few per second.
  const steps = [];
  const spellAt = (spell, target) => steps.push({
    go: (s) => {
      W.c0 = s.casts; W.f0 = s.frame;
      const t = target(s) ?? idx('dry hedge');
      cmd('tp 0 5.5'); cmd(`face ${t}`); cmd(`select ${spell}`);
      if (!noCast) cmd('cast');
    },
    until: (s) => (noCast ? s.frame > W.f0 + 1 : s.casts > W.c0),
    ms: 6000,
    done: (s) => { if (W.sampling && s.casts > W.c0 && s.spell === spell - 1) W.casts[spell]++; },
  });
  const motion = (...cs) => steps.push({ go: (s) => { W.f0 = s.frame; cs.forEach(cmd); }, until: (s) => s.frame > W.f0, ms: 4000 });
  spellAt(1, () => idx('dry hedge'));
  motion('move 0 1 0.5', 'turn 400 0');
  spellAt(2, burningAny);
  motion('look 90 10', 'move 1 0 0.4');
  spellAt(3, (s) => burningAny(s) ?? idx('puddle'));
  motion('turn -400 50', 'move 0 -1 0.4');
  spellAt(4, burningAny);
  motion('look -90 -5');
  let i = 0, t0 = 0, active = null, lastYaw = null;
  setInterval(() => {
    const s = st();
    if (!s) return;
    if (lastYaw !== null && W.sampling) {
      let d = Math.abs(s.yaw - lastYaw) % 360;
      if (d > 180) d = 360 - d;
      W.yaw += d;
    }
    lastYaw = s.yaw;
    if (W.sampling) W.burningMax = Math.max(W.burningMax, s.burning);
    if (s.outcome !== 'playing' || s.paused) {
      if (!active || active.reset !== true) { if (s.paused) window.__game.pause(false); cmd('reset'); W.resets++; active = { reset: true }; t0 = performance.now(); }
      else if (performance.now() - t0 > 5000) active = null;
      return;
    }
    if (active && active.reset) active = null;
    if (active) {
      const ok = active.until(s);
      if (!ok && performance.now() - t0 < active.ms) return;
      if (active.done) active.done(s);
      active = null;
      i = (i + 1) % steps.length;
    }
    active = steps[i];
    W.steps++;
    t0 = performance.now();
    active.go(s);
  }, 50);
}

// ---- one run ----------------------------------------------------------------
const pct = (arr, p) => {
  if (!arr.length) return null;
  const s = [...arr].sort((a, b) => a - b);
  return s[Math.min(s.length - 1, Math.round((s.length - 1) * p))];
};
const r2 = (x) => (x === null || x === undefined ? null : +x.toFixed(2));
const median = (xs) => { const v = xs.filter((x) => typeof x === 'number'); return v.length ? pct(v, 0.5) : null; };

async function runOne(browser, base, cfg, size, collectRequests) {
  const isMac = cfg === 'mac';
  const [tier, thr] = isMac ? ['default', '1x'] : cfg.split('@');
  const rate = isMac ? null : parseInt(thr, 10);
  const url = base + (tier === 'low' ? '?low' : '');
  const context = await browser.newContext({ viewport: isMac ? { width: MAC_VW, height: MAC_VH } : { width: 1280, height: 720 } });
  const page = await context.newPage();
  const out = { config: cfg, url, tti_ms: null, first_frame_log_ms: null, frames: 0, cpu_ms: { p50: null, p95: null, max: null },
    frame_interval_ms: { p50: null, p95: null }, cpuframe_log_p50_ms: null, console_errors: { count: 0, first: [] }, panics: 0,
    workload_ok: false, workload: null, throttle_applied: false, valid: false };
  const errors = [];
  let cpuframeLog = null, statsP5 = null;
  page.on('console', (m) => {
    const t = m.text().replace(/%c/g, '').replace(/ color: .*$/, '');
    if (m.type() === 'error') errors.push(t);
    if (/panicked/.test(t)) out.panics++;
    let x = /CPUFRAME n=\d+ p50_ms=([\d.]+)/.exec(t);
    if (x) cpuframeLog = +x[1];
    x = /STATS .*fps_p5=([\d.]+)/.exec(t);
    if (x) statsP5 = +x[1];
  });
  page.on('pageerror', (e) => { errors.push(e.message); if (/panicked/.test(e.message)) out.panics++; });
  const origin = new URL(base).origin;
  if (collectRequests) page.on('request', (r) => {
    const u = new URL(r.url());
    if (u.origin === origin) collectRequests.add(decodeURIComponent(u.pathname).replace(/^\/+/, '') || 'index.html');
  });
  if (!isMac) {
    const cdp = await context.newCDPSession(page);
    await cdp.send('Emulation.setCPUThrottlingRate', { rate });
    out.throttle_applied = true;
    out.throttle_rate = rate;
  }
  await page.addInitScript(pageHook);
  log(`${cfg}: loading ${url}`);
  await page.goto(url, { timeout: TTI_TIMEOUT_S * 1000 });
  const deadline = Date.now() + TTI_TIMEOUT_S * 1000;
  let tti = null;
  let ctx = [];
  while (Date.now() < deadline) {
    [tti, ctx] = await page.evaluate(() => [window.__perf.tti, window.__perf.ctx]).catch(() => [null, []]);
    if (tti !== null) break;
    // A WebGPU build can never pass the real-GPU WebGL2 bar: stop early.
    if (isMac && ctx.some((c) => c.startsWith('webgpu'))) break;
    await sleep(200);
  }
  out.contexts = ctx;
  out.first_frame_log_ms = await page.evaluate(() => window.__perf.firstFrameLog).catch(() => null);
  if (tti === null) {
    log(`${cfg}: no interactive frame${Date.now() >= deadline ? ` within ${TTI_TIMEOUT_S} s` : ' (stopped early: WebGPU build)'}`);
  } else {
    out.tti_ms = r2(tti);
    log(`${cfg}: interactive at ${out.tti_ms} ms (FIRST_FRAME t=${out.first_frame_log_ms})`);
    await page.evaluate(pageWorkload, NO_CAST);
    const warm = isMac ? 5 : WARMUP_S, sample = isMac ? MAC_SAMPLE_S : SAMPLE_S;
    await sleep(warm * 1000);
    const a = await page.evaluate(() => { window.__work.sampling = true; return performance.now(); });
    // The window is SAMPLE_S long, extended (up to MAX_SAMPLE_S) until it
    // holds MIN_FRAMES frames: a software GPU may draw under 2 frames a second.
    const tEnd = Date.now() + sample * 1000, tMax = Date.now() + Math.max(sample, MAX_SAMPLE_S) * 1000;
    while (true) {
      await sleep(250);
      if (Date.now() < tEnd) continue;
      const n = await page.evaluate((a) => window.__perf.ticks.filter((t) => t[0] >= a).length, a);
      if (n >= MIN_FRAMES || Date.now() >= tMax) break;
    }
    const [b, ticks, work] = await page.evaluate(() => { window.__work.sampling = false; const b = performance.now(); return [b, window.__perf.take(0, b), window.__work]; });
    const win = ticks.filter((t) => t[0] >= a && t[0] <= b);
    const cpu = win.map((t) => t[2]);
    const iv = [];
    for (let k = 1; k < win.length; k++) iv.push(win[k][0] - win[k - 1][0]);
    out.frames = win.length;
    out.sample_s = +((b - a) / 1000).toFixed(1);
    out.cpu_ms = { p50: r2(pct(cpu, 0.5)), p95: r2(pct(cpu, 0.95)), max: r2(pct(cpu, 1)) };
    out.frame_interval_ms = { p50: r2(pct(iv, 0.5)), p95: r2(pct(iv, 0.95)) };
    out.cpuframe_log_p50_ms = cpuframeLog;
    out.workload = { casts: work.casts, burning_max: work.burningMax, yaw_deg: r2(work.yaw), steps: work.steps, resets: work.resets };
    out.workload_ok = [1, 2, 3, 4].every((s) => work.casts[s] >= 1) && work.burningMax >= 1 && work.yaw >= 90;
    if (isMac) {
      out.fps_p50 = r2(1000 / out.frame_interval_ms.p50);
      out.fps_p5 = r2(1000 / out.frame_interval_ms.p95);
      out.stats_fps_p5_log = statsP5;
      Object.assign(out, await page.evaluate(() => {
        const c = document.createElement('canvas');
        const gl = c.getContext('webgl2') || c.getContext('webgl');
        const ext = gl && gl.getExtension('WEBGL_debug_renderer_info');
        const renderer = gl ? (ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER)) : null;
        return { renderer };
      }));
    }
  }
  if (isMac) {
    // webgl2: the game asked for a webgl2 context and never for WebGPU.
    out.contexts = await page.evaluate(() => window.__perf.ctx).catch(() => out.contexts);
    out.webgl2 = out.contexts.includes('webgl2') && !out.contexts.some((c) => c.startsWith('webgpu'));
    if (!out.webgl2) { out.error = 'not a WebGL2 build'; log(`${cfg}: not a WebGL2 build (contexts: ${out.contexts.join(', ') || 'none'})`); }
  }
  out.console_errors = { count: errors.length, first: errors.slice(0, 5) };
  out.valid = out.tti_ms !== null && out.workload_ok && errors.length === 0 && out.panics === 0 && out.frames > 0 &&
    (isMac ? out.webgl2 === true : out.throttle_applied);
  log(`${cfg}: frames=${out.frames} cpu_p50=${out.cpu_ms.p50} cpu_p95=${out.cpu_ms.p95} interval_p50=${out.frame_interval_ms.p50} workload=${JSON.stringify(out.workload)} errors=${errors.length} valid=${out.valid}`);
  await context.close();
  return out;
}

async function rendererOf(browser) {
  const ctx = await browser.newContext();
  const page = await ctx.newPage();
  const r = await page.evaluate(() => {
    const c = document.createElement('canvas');
    const gl = c.getContext('webgl2') || c.getContext('webgl');
    const ext = gl && gl.getExtension('WEBGL_debug_renderer_info');
    return gl ? (ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER)) : null;
  }).catch(() => null);
  await ctx.close();
  return r;
}

// Merges repeats: the median of every numeric metric.
function merge(reps) {
  if (reps.length === 1) return reps[0];
  const m = JSON.parse(JSON.stringify(reps[0]));
  const at = (o, p) => p.reduce((x, k) => (x == null ? x : x[k]), o);
  for (const p of [['tti_ms'], ['first_frame_log_ms'], ['frames'], ['cpuframe_log_p50_ms'], ['fps_p50'], ['fps_p5']]) m[p[0]] = median(reps.map((r) => at(r, p)));
  for (const k of ['p50', 'p95', 'max']) m.cpu_ms[k] = median(reps.map((r) => r.cpu_ms[k]));
  for (const k of ['p50', 'p95']) m.frame_interval_ms[k] = median(reps.map((r) => r.frame_interval_ms[k]));
  m.valid = reps.every((r) => r.valid);
  m.workload_ok = reps.every((r) => r.workload_ok);
  m.console_errors = { count: reps.reduce((a, r) => a + r.console_errors.count, 0), first: reps.flatMap((r) => r.console_errors.first).slice(0, 5) };
  m.panics = reps.reduce((a, r) => a + r.panics, 0);
  m.throttle_applied = reps.every((r) => r.throttle_applied);
  m.repeats = reps;
  return m;
}

// ---- main ---------------------------------------------------------------------
let server = null;
if (!env.BASE_URL) {
  try { server = await serve(webDir); } catch (e) { die(`cannot start the static server: ${e.message}`); }
}
const base = env.BASE_URL ? env.BASE_URL.replace(/\/?$/, '/') : `http://127.0.0.1:${server.address().port}/`;
if (env.BASE_URL) {
  try { const r = await fetch(base, { signal: AbortSignal.timeout(10000) }); if (!r.ok) throw new Error(`HTTP ${r.status}`); }
  catch (e) { die(`BASE_URL not reachable: ${base} (${e.cause?.code || e.message})`); }
}
let browser;
try {
  browser = mac
    ? await chromium.launch({ ...((env.MAC_CHANNEL || 'chrome') === 'chromium' ? {} : { channel: env.MAC_CHANNEL || 'chrome' }), headless: env.MAC_HEADLESS === '1' })
    : await chromium.launch();
} catch (e) { if (server) server.close(); die(`cannot launch the browser: ${e.message.split('\n')[0]}`); }

const t0 = Date.now();
log(`sizing ${webDir}`);
const size = await sizeOf(webDir);
log(`brotli total ${size.brotli_total_bytes} B (${size.brotli_total_mb} MB), gzip ${size.gzip_total_bytes} B`);
const renderer = await rendererOf(browser);
const runs = {};
const requested = new Set();
for (const [n, cfg] of cfgs.entries()) {
  const reps = [];
  for (let k = 0; k < (mac ? 1 : RUNS); k++) reps.push(await runOne(browser, base, cfg, size, n === 0 && k === 0 ? requested : null));
  runs[cfg] = merge(reps);
}
const inDir = new Set(size.files.map((f) => f.path));
size.requested = [...requested].sort();
size.missing_from_dir = size.requested.filter((p) => !inDir.has(p));

const bv = (value, threshold, cmp, needs) => {
  if (needs && !runs[needs]) return { value: null, threshold, pass: null, skipped: true };
  return { value: value ?? null, threshold, pass: value !== null && value !== undefined && cmp(value, threshold) };
};
const verdict = {};
if (mac) {
  verdict.B2 = { mac: bv(runs.mac.fps_p5, BARS.b2_mac_p5_fps, (v, t) => v >= t, 'mac') };
} else {
  verdict.B3 = {
    size: bv(size.brotli_total_mb, BARS.b3_brotli_mb, (v, t) => v <= t),
    tti: bv(runs['default@4x']?.tti_ms ?? null, BARS.b3_tti_ms, (v, t) => v <= t, 'default@4x'),
    tti_low_4x_ms: runs['low@4x']?.tti_ms ?? null,
  };
  verdict.B2 = { cpu: bv(runs['low@4x']?.cpu_ms.p50 ?? null, BARS.b2_cpu_p50_ms, (v, t) => v <= t, 'low@4x'), mac: 'not measured here; see docs/loop/mac-perf.md' };
}
const partial = !mac && !!only && ALL_CFGS.some((c) => !only.includes(c));
const allValid = Object.values(runs).every((r) => r.valid) && size.missing_from_dir.length === 0;
const barPass = mac ? verdict.B2.mac.pass === true
  : verdict.B3.size.pass === true && verdict.B3.tti.pass === true && verdict.B2.cpu.pass === true;
const pass = !partial && barPass && allValid;

let basedOn = null;
try { basedOn = execFileSync('git', ['-C', here, 'rev-parse', 'HEAD'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim(); } catch {}
const result = {
  schema: 'perf/1',
  mode: mac ? 'mac' : 'cloud',
  based_on: basedOn,
  web_dir: webDir,
  base_url: env.BASE_URL || null,
  env: {
    node: process.version, chromium: browser.version(), playwright: pwVersion, cpu: os.cpus()[0]?.model || null, cpus: os.cpus().length,
    renderer, gpu: /swiftshader|llvmpipe/i.test(renderer || '') ? 'software' : 'hardware',
    sample_s: mac ? MAC_SAMPLE_S : SAMPLE_S, warmup_s: mac ? 5 : WARMUP_S, runs: mac ? 1 : RUNS, only, no_cast: NO_CAST,
  },
  size,
  runs,
  bars: BARS,
  verdict,
  partial,
  ...(mac && runs.mac.error ? { error: runs.mac.error } : {}),
  valid: allValid,
  pass,
  elapsed_s: +((Date.now() - t0) / 1000).toFixed(1),
};
await browser.close();
if (server) server.close();
const text = JSON.stringify(result, null, 2);
if (env.OUT) writeFileSync(env.OUT, text + '\n');
process.stdout.write(text + '\n');
log(`pass=${pass}${partial ? ' (partial run)' : ''} valid=${allValid} in ${result.elapsed_s} s`);
process.exit(pass ? 0 : 1);
