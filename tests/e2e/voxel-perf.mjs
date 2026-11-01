// Perf and feel harness for the block world (I-055). Loads the default page
// (the voxel world), walks a fixed route with debug commands (spawn, the
// Village workshop and bench plaza, the Kiln terrace, the Frozen Fields edge
// across the ford, the Marsh edge), and records per frame CPU time (each
// requestAnimationFrame callback, where Bevy runs its update and render)
// and frame interval, hitches (frames over 33 ms), JS heap, wasm memory
// (the game's STATS lines), the chunk queue (`chunksMeshed`, `meshQ`) and
// the time to the first frame and the first playable frame. Budgets come
// from section 9 of docs/loop/items/I-052/spec.md. Writes JSON and a short
// Markdown report.
//
//   bash crates/bevy-client/build-web.sh                 # web/pkg-out
//   node tests/e2e/voxel-perf.mjs                        # headless (cloud): CPU time, ?low, 4x throttle
//   node tests/e2e/voxel-perf.mjs --headed               # this Mac: real Chrome, real GPU, no throttle
//   node tests/e2e/voxel-perf.mjs --classic              # smoke test on the classic courtyard
//   node tests/e2e/voxel-perf.mjs --self-test            # helpers and budget parsing only
//
// Flags: --headed, --classic, --throttle N (CPU slowdown; default 4
// headless, 1 headed), --low / --no-low (default: low headless, default
// tier headed), --query k=v&k=v (extra URL parameters), --out DIR (default
// docs/loop/items/I-055/runs), --dwell S (seconds at each stop, default 6).
// Env: BASE_URL (test a running server instead of serving WEB_DIR), WEB_DIR
// (default crates/bevy-client/web/pkg-out), CHANNEL (headed: chrome),
// TTI_TIMEOUT_S (how long to wait for the spawn ring, 120).
// Exit: 0 every budget met, 1 a budget failed, 2 setup error. Needs
// `playwright` (no package.json entry).
import { createServer } from 'node:http';
import { readFileSync, writeFileSync, mkdirSync, existsSync, statSync } from 'node:fs';
import { join, extname, dirname, resolve, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const SPEC = join(ROOT, 'docs/loop/items/I-052/spec.md');

// ---------- pure helpers (covered by --self-test) ----------

/** Nearest-rank percentile of `xs` (0..1). */
export function pct(xs, p) {
  if (!xs.length) return null;
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.min(s.length - 1, Math.max(0, Math.ceil(p * s.length) - 1))];
}

/** Summary of a list of frame times (ms). */
export function summary(xs, hitchMs = 33) {
  const r = (v) => (v == null ? null : Math.round(v * 100) / 100);
  return {
    n: xs.length,
    p50: r(pct(xs, 0.5)),
    p95: r(pct(xs, 0.95)),
    p99: r(pct(xs, 0.99)),
    max: r(xs.length ? Math.max(...xs) : null),
    hitches: xs.filter((v) => v > hitchMs).length,
  };
}

/**
 * The budgets from the spec's section 9 text: CPU median (ms), TTI (ms),
 * meshing per frame (ms, low and default), WASM memory (MB), draw calls.
 * Falls back to the numbers the spec had on 2026-11-01 for any it cannot
 * read, and says which.
 */
export function readBudgets(md) {
  const sec = (md.split(/^## 9\./m)[1] || '').split(/^## /m)[0];
  const num = (re) => { const m = re.exec(sec); return m ? Number(m[1]) : null; };
  const got = {
    cpu_p50_ms: num(/median CPU frame ≤ (\d+(?:\.\d+)?) ms/),
    tti_ms: (() => { const s = num(/First playable frame \(TTI\) \| ≤ (\d+(?:\.\d+)?) s/); return s == null ? null : s * 1000; })(),
    mesh_low_ms: num(/Meshing \| ≤ (\d+(?:\.\d+)?) ms per frame \(`\?low`\)/),
    mesh_default_ms: num(/Meshing \|[^|]*≤ (\d+(?:\.\d+)?) ms default/),
    wasm_mb: num(/WASM ≤ (\d+) MB/),
    draw_calls: num(/Draw calls \| ≤ (\d+)/),
  };
  const fallback = { cpu_p50_ms: 16, tti_ms: 8000, mesh_low_ms: 2, mesh_default_ms: 4, wasm_mb: 96, draw_calls: 250 };
  const defaulted = Object.keys(fallback).filter((k) => got[k] == null);
  for (const k of defaulted) got[k] = fallback[k];
  return { ...got, defaulted };
}

/** Parses a STATS console line into numbers (fps, wasm_mem_mb, ...). */
export function parseStats(line) {
  if (!/STATS /.test(line)) return null;
  const o = {};
  for (const m of line.matchAll(/(\w+)=(-?[\d.]+)/g)) o[m[1]] = Number(m[2]);
  return o;
}

/** The fixed route: [name, voxel tp (x y z), classic tp (x z), yaw]. */
export const ROUTE = [
  ['spawn', [128, 36, 205], [0, 6], 0],
  ['village workshop', [142, 36, 196], [8, 2], 300],
  ['bench plaza', [124, 36, 200], [-6, 4], 0],
  ['kiln terrace', [128, 42, 170], [0, -6], 0],
  ['frozen edge (ford)', [112, 44, 146], [-12, -10], 120],
  ['marsh edge', [150, 36, 152], [12, -10], 240],
];

// ---------- self-test ----------

function selfTest() {
  const fails = [];
  const eq = (name, a, b) => { if (JSON.stringify(a) !== JSON.stringify(b)) fails.push(`${name}: ${JSON.stringify(a)} != ${JSON.stringify(b)}`); };
  eq('pct p50', pct([5, 1, 3, 2, 4], 0.5), 3);
  eq('pct p99 of 100', pct([...Array(100).keys()].map((i) => i + 1), 0.99), 99);
  eq('pct empty', pct([], 0.5), null);
  eq('summary hitches', summary([10, 20, 34, 50]).hitches, 2);
  eq('stats', parseStats('INFO STATS fps=60.0 wasm_mem_mb=34.2 heap_live_mb=25.1')?.wasm_mem_mb, 34.2);
  eq('stats none', parseStats('CPUFRAME n=1'), null);
  const b = readBudgets(readFileSync(SPEC, 'utf8'));
  eq('budgets read from the spec', b.defaulted, []);
  eq('budget values', [b.cpu_p50_ms, b.tti_ms, b.mesh_low_ms, b.mesh_default_ms, b.wasm_mb, b.draw_calls], [16, 8000, 2, 4, 96, 250]);
  eq('budgets fallback', readBudgets('nothing').defaulted.length, 6);
  for (const f of fails) console.error('FAIL', f);
  console.log(fails.length ? `${fails.length} self-test failures` : 'self-test ok');
  process.exit(fails.length ? 1 : 0);
}

// ---------- the run ----------

function args(argv) {
  const o = { headed: false, classic: false, throttle: null, low: null, query: '', out: join(ROOT, 'docs/loop/items/I-055/runs'), dwell: 6 };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--headed') o.headed = true;
    else if (a === '--classic') o.classic = true;
    else if (a === '--low') o.low = true;
    else if (a === '--no-low') o.low = false;
    else if (a === '--throttle') o.throttle = Number(argv[++i]);
    else if (a === '--query') o.query = argv[++i];
    else if (a === '--out') o.out = resolve(argv[++i]);
    else if (a === '--dwell') o.dwell = Number(argv[++i]);
    else if (a === '--self-test') o.self = true;
    else { console.error(`unknown flag ${a}`); process.exit(2); }
  }
  if (o.throttle == null) o.throttle = o.headed ? 1 : 4;
  if (o.low == null) o.low = !o.headed;
  return o;
}

const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.css': 'text/css', '.json': 'application/json', '.png': 'image/png' };

function serve(dir) {
  return new Promise((ok) => {
    const s = createServer((req, res) => {
      let p = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^([/\\])+/, '');
      if (!p || p.endsWith('/')) p += 'index.html';
      const f = join(dir, p);
      if (!f.startsWith(dir) || !existsSync(f) || !statSync(f).isFile()) { res.writeHead(404); res.end(); return; }
      res.writeHead(200, { 'content-type': TYPES[extname(f)] || 'application/octet-stream', 'cache-control': 'no-store' });
      res.end(readFileSync(f));
    });
    s.listen(0, '127.0.0.1', () => ok(s));
  });
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function main() {
  const o = args(process.argv.slice(2));
  if (o.self) return selfTest();
  const budgets = readBudgets(readFileSync(SPEC, 'utf8'));
  const require = createRequire(import.meta.url);
  let chromium;
  try { ({ chromium } = require('playwright')); } catch { console.error('needs playwright (NODE_PATH)'); process.exit(2); }

  let server = null;
  let base = process.env.BASE_URL;
  if (!base) {
    const dir = resolve(process.env.WEB_DIR || join(ROOT, 'crates/bevy-client/web/pkg-out'));
    if (!existsSync(join(dir, 'index.html'))) { console.error(`no build at ${dir}: run build-web.sh`); process.exit(2); }
    server = await serve(dir);
    base = `http://127.0.0.1:${server.address().port}/`;
  }
  const q = ['debug', o.low ? 'low' : '', o.classic ? 'classic=1' : '', o.query].filter(Boolean).join('&');
  const url = `${base}${base.includes('?') ? '&' : '?'}${q}`;

  const browser = await chromium.launch(o.headed ? { headless: false, channel: process.env.CHANNEL || 'chrome' } : {});
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: o.headed ? 2 : 1 });
  const page = await ctx.newPage();
  // Per frame: the CPU time of all rAF callbacks in that frame (Bevy runs
  // its update and render in one; the HUD's are small) and the interval
  // since the last frame, tagged with the current route segment. Callbacks
  // of one frame share its timestamp. Installed before boot.js, which wraps
  // rAF again for its own CPUFRAME lines.
  await page.addInitScript(() => {
    const w = window;
    w.__vp = { seg: 'boot', frames: [], t: null, cpu: 0, prev: null, t0: performance.now() };
    const raf = w.requestAnimationFrame.bind(w);
    w.requestAnimationFrame = (cb) => raf((t) => {
      const v = w.__vp;
      if (v.t !== t) {
        if (v.t != null) v.frames.push([v.seg, v.cpu, v.prev == null ? null : v.t - v.prev]);
        v.prev = v.t;
        v.t = t;
        v.cpu = 0;
      }
      const a = performance.now();
      try { cb(t); } finally { v.cpu += performance.now() - a; }
    });
  });
  const logs = [];
  const errors = [];
  page.on('console', (m) => { const t = m.text().replace(/%c/g, '').replace(/ color: .*$/, ''); logs.push([Date.now(), t]); if (m.type() === 'error') errors.push(t); });
  page.on('pageerror', (e) => errors.push(e.message));
  const cdp = await ctx.newCDPSession(page);
  if (o.throttle > 1) await cdp.send('Emulation.setCPUThrottlingRate', { rate: o.throttle });

  const t0 = Date.now();
  await page.goto(url);
  const st = () => page.evaluate(() => (window.__gameState ? JSON.parse(window.__gameState) : null));
  const cmd = (c) => page.evaluate((c) => window.__game && window.__game.cmd(c), c);
  const setSeg = (s) => page.evaluate((s) => { window.__vp.seg = s; }, s);
  const now = () => page.evaluate(() => performance.now() - window.__vp.t0);

  // First frame (the game's FIRST_FRAME line) and first playable frame: the
  // first state in the asked mode with the spawn ring meshed (voxel:
  // chunksMeshed >= 18; classic: any state).
  let firstState = null;
  let playable = null;
  const ringNeeded = 18;
  for (;;) {
    const s = await st().catch(() => null);
    if (s && firstState == null) firstState = await now();
    // A build without the chunk keys (the contract stub) counts its first
    // voxel state, and the report says there is no terrain.
    const ring = s && s.mode === 'voxel' && (s.chunksMeshed === undefined || s.chunksMeshed >= ringNeeded);
    if (s && (o.classic || ring)) { playable = await now(); break; }
    if (Date.now() - t0 > Number(process.env.TTI_TIMEOUT_S || 120) * 1000) break;
    await sleep(100);
  }
  const ff = logs.map(([, t]) => /FIRST_FRAME t=(\d+)ms/.exec(t)).find(Boolean);
  const mode = (await st())?.mode || (o.classic ? 'classic' : 'unknown');
  // Close the intro and pause card if they are up, so the route measures play.
  await page.evaluate(() => { try { window.__gameIn = window.__gameIn || {}; (window.__gameIn.story = window.__gameIn.story || []).push('intro_done'); window.__game.pause(false); } catch {} });

  const segs = [];
  for (const [name, vtp, ctp, yaw] of ROUTE) {
    await setSeg(name);
    const before = await st();
    const tLog = Date.now();
    await cmd(o.classic ? `tp ${ctp[0]} ${ctp[1]}` : `tp ${vtp.join(' ')}`);
    await cmd(`look ${yaw} 0`);
    // Settle, then sweep the view round once, then walk forward.
    const dwell = o.dwell * 1000;
    await sleep(dwell * 0.25);
    for (let k = 1; k <= 12; k++) { await cmd(`look ${yaw + k * 30} ${k % 2 ? -10 : 5}`); await sleep(dwell * 0.5 / 12); }
    await cmd('move 0 1 1.5');
    await sleep(dwell * 0.25);
    const after = await st();
    const heap = await page.evaluate(() => (performance.memory ? performance.memory.usedJSHeapSize / 1048576 : null));
    const stats = logs.filter(([t]) => t >= tLog).map(([, l]) => parseStats(l)).filter(Boolean);
    segs.push({
      name,
      pos: after?.pos,
      biome: after?.biome ?? null,
      chunks_meshed: after?.chunksMeshed ?? null,
      mesh_q: after?.meshQ ?? null,
      matter_ms: after?.matterMs ?? null,
      moved: before && after ? Math.hypot(after.pos[0] - before.pos[0], after.pos[2] - before.pos[2]) : null,
      js_heap_mb: heap == null ? null : Math.round(heap * 10) / 10,
      wasm_mb: stats.length ? Math.max(...stats.map((s) => s.wasm_mem_mb ?? 0)) : null,
    });
  }
  const frames = await page.evaluate(() => window.__vp.frames);
  const errs = [...errors];
  await browser.close();
  if (server) server.close();

  const play = frames.filter((f) => f[0] !== 'boot');
  for (const s of segs) {
    const f = play.filter((x) => x[0] === s.name);
    s.cpu = summary(f.map((x) => x[1]));
    s.interval = summary(f.map((x) => x[2]).filter((x) => x != null));
  }
  const cpu = summary(play.map((f) => f[1]));
  const interval = summary(play.map((f) => f[2]).filter((x) => x != null));
  const wasmMax = Math.max(0, ...segs.map((s) => s.wasm_mb || 0)) || null;
  const heapMax = Math.max(0, ...segs.map((s) => s.js_heap_mb || 0)) || null;
  const tti = playable;
  const checks = [
    ['median CPU frame', cpu.p50, budgets.cpu_p50_ms, 'ms'],
    ['first playable frame', tti == null ? null : Math.round(tti), budgets.tti_ms, 'ms'],
    ['WASM memory', wasmMax, budgets.wasm_mb, 'MB'],
  ].map(([name, got, max, unit]) => ({ name, got, max, unit, ok: got != null && got <= max }));
  const meshing = segs.map((s) => s.mesh_q).some((v) => v != null);
  const result = {
    when: new Date().toISOString(),
    url,
    mode,
    profile: o.headed ? 'mac-headed (real GPU)' : 'headless (CPU frame time only)',
    throttle: o.throttle,
    low: o.low,
    budgets,
    first_state_ms: firstState == null ? null : Math.round(firstState),
    first_frame_ms: ff ? Number(ff[1]) : null,
    first_playable_ms: tti == null ? null : Math.round(tti),
    cpu,
    interval: o.headed ? interval : { ...interval, note: 'headless: frame interval is not meaningful' },
    hitches_cpu_over_33ms: cpu.hitches,
    js_heap_max_mb: heapMax,
    wasm_max_mb: wasmMax,
    voxel_keys_present: meshing,
    segments: segs,
    console_errors: errs.slice(0, 5),
    checks,
    pass: checks.every((c) => c.ok) && errs.length === 0,
  };

  mkdirSync(o.out, { recursive: true });
  const stamp = result.when.replace(/[:.]/g, '-');
  const tag = `${o.classic ? 'classic' : 'voxel'}-${o.headed ? 'mac' : 'headless'}`;
  const jsonPath = join(o.out, `${tag}-${stamp}.json`);
  writeFileSync(jsonPath, JSON.stringify(result, null, 2));
  const md = report(result);
  const mdPath = join(o.out, `${tag}-${stamp}.md`);
  writeFileSync(mdPath, md);
  console.log(md);
  console.error(`wrote ${jsonPath}\nwrote ${mdPath}`);
  process.exit(result.pass ? 0 : 1);
}

function report(r) {
  const f = (v, u = '') => (v == null ? 'n/a' : `${v}${u}`);
  const lines = [
    `# Voxel perf: ${r.mode}, ${r.profile}`,
    '',
    `${r.when}. ${r.url.replace(/^http:\/\/127\.0\.0\.1:\d+/, '')}, CPU throttle ${r.throttle}x, ${r.low ? '?low' : 'default tier'}.`,
    '',
    `**${r.pass ? 'PASS' : 'FAIL'}** against section 9 of the I-052 spec${r.budgets.defaulted.length ? ` (defaulted: ${r.budgets.defaulted.join(', ')})` : ''}.`,
    '',
    '| Check | Got | Budget | |',
    '|---|---|---|---|',
    ...r.checks.map((c) => `| ${c.name} | ${f(c.got, ' ' + c.unit)} | ≤ ${c.max} ${c.unit} | ${c.ok ? 'ok' : 'over'} |`),
    '',
    `- CPU per frame: p50 ${f(r.cpu.p50)} ms, p95 ${f(r.cpu.p95)} ms, p99 ${f(r.cpu.p99)} ms, max ${f(r.cpu.max)} ms over ${r.cpu.n} frames; ${r.hitches_cpu_over_33ms} over 33 ms.`,
    r.profile.startsWith('mac') ? `- Frame interval: p50 ${f(r.interval.p50)} ms, p95 ${f(r.interval.p95)} ms, p99 ${f(r.interval.p99)} ms; ${r.interval.hitches} hitches over 33 ms.` : '- Frame interval: not measured headless.',
    `- First state ${f(r.first_state_ms, ' ms')}, first frame ${f(r.first_frame_ms, ' ms')}, first playable ${f(r.first_playable_ms, ' ms')}.`,
    `- JS heap max ${f(r.js_heap_max_mb, ' MB')}, WASM max ${f(r.wasm_max_mb, ' MB')}.`,
    r.voxel_keys_present ? '' : '- The page publishes no chunk keys (`chunksMeshed`, `meshQ`): no voxel terrain in this build.',
    r.first_playable_ms == null && r.mode === 'voxel' ? '- The spawn ring (18 chunks) never meshed before the timeout: no playable terrain in this build.' : '',
    r.console_errors.length ? `- Console errors: ${r.console_errors.join(' | ')}` : '- No console errors.',
    '',
    '| Stop | CPU p50 / p95 / p99 ms | Hitches | Interval p95 | Chunks meshed | Queue | Matter ms | Moved m | JS heap MB | WASM MB |',
    '|---|---|---|---|---|---|---|---|---|---|',
    ...r.segments.map((s) => `| ${s.name} | ${f(s.cpu.p50)} / ${f(s.cpu.p95)} / ${f(s.cpu.p99)} | ${s.cpu.hitches} | ${r.profile.startsWith('mac') ? f(s.interval.p95) : 'n/a'} | ${f(s.chunks_meshed)} | ${f(s.mesh_q)} | ${f(s.matter_ms)} | ${s.moved == null ? 'n/a' : s.moved.toFixed(1)} | ${f(s.js_heap_mb)} | ${f(s.wasm_mb)} |`),
    '',
  ];
  return lines.filter((l, i, a) => !(l === '' && a[i - 1] === '')).join('\n');
}

main().catch((e) => { console.error(e); process.exit(2); });
