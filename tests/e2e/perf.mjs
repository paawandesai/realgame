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
// Chromium), MAC_HEADLESS (0), MAC_DPR (2: deviceScaleFactor, the MacBook's
// Retina density). Test-only hooks, each makes a run invalid on purpose:
// PERF_NO_CAST=1 drops casts from the workload, PERF_SKIP_THROTTLE=1 never
// sends the CDP throttle (but reports it as sent), PERF_BLANK_CANVAS=1 paints
// the canvas black with CSS, PERF_GL_COUNT=1 counts WebGL2 calls per frame
// (gl_calls; the wrapper adds CPU, so the run is a diagnostic only).
// PERF_ALLOW_LOAD=1 lets a cloud run count on a busy machine (see below).
//
// Machine load (I-015): every run records load_avg (1/5/15 min) and the
// busiest other processes (busy_procs) at load start and at sampling end. A
// cloud run is invalid (`machine busy: load1=<x>`) when the 1-minute load
// average, less this script's own browser (an estimate kept like the
// kernel's average), is above half the CPU count at either moment; before
// each run the script waits up to 60 s for the machine to go quiet. A run
// invalid only for an environment reason (machine busy, throttle not
// verified) is re-run up to 2 times, each listed under `reruns`; a run with
// an incomplete workload is never re-run. Workload steps wait for their
// state change until both their wall-clock timeout and 8 game frames have
// passed, so a slow software GPU no longer drops a cast the game would fire
// on its next frame; a step with no new frames gives up at 10x its timeout.
// Flags: --query k=v[&k=v] adds parameters to the page URL (I-035: `--query
// level=2` measures level 2 with a stream-and-gate workload; I-037: `--query
// level=3` measures level 3 with a gas-vent workload). --bars k=v,... overrides thresholds (b3_brotli_mb, b3_tti_ms,
// b2_cpu_p50_ms, b2_mac_p5_fps). --self-test runs the unit tests of the pure
// helpers (no browser, no build) and lints docs/loop/mac-perf.md.
//
// Trust checks (docs/loop/items/I-013/spec.md): every run reports
// device_pixel_ratio, visibility (hidden / unfocused samples), a per-run
// throttle calibration (cloud), a screenshot blank-canvas check, the TTI
// uncertainty (tti_bound_ms) and a bootstrap 95% range of the CPU p50. B3.size
// counts only files the page requested, and every served body must match its
// WEB_DIR file in length. A failed check lands in the run's invalid_reasons.
//
// Exit: 0 pass, 1 a bar failed or a run was invalid, 2 usage or environment
// error. Progress goes to stderr. Needs `playwright` (no package.json entry).
import { createServer } from 'node:http';
import { readFile, readdir, stat } from 'node:fs/promises';
import { writeFileSync, readFileSync } from 'node:fs';
import { extname, join, normalize, dirname, resolve, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { brotliCompressSync, gzipSync, inflateSync, constants as Z } from 'node:zlib';
import os from 'node:os';

const require = createRequire(import.meta.url);
const here = dirname(fileURLToPath(import.meta.url));
const log = (...a) => console.error('[perf]', ...a);
const die = (msg) => { console.error(`perf: ${msg}`); process.exit(2); };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const env = process.env;

// ---- pure helpers (unit-tested by --self-test) ------------------------------
// Function declarations, so the self-test can call them before the rest of
// the module runs.
function pct(arr, p) {
  if (!arr.length) return null;
  const s = [...arr].sort((a, b) => a - b);
  return s[Math.min(s.length - 1, Math.round((s.length - 1) * p))];
}
function r2(x) { return x === null || x === undefined ? null : +x.toFixed(2); }

// A workload step gives up only when both its wall-clock timeout and
// `minFrames` game frames have passed since it started (I-015 D1), or in any
// case after `capFactor` times its timeout, so a frozen page (no new frames)
// cannot hold a step forever. Injected into the page with the workload.
function stepTimedOut(elapsedMs, framesSince, ms, minFrames = 8, capFactor = 10) {
  return elapsedMs >= ms * capFactor || (elapsedMs >= ms && framesSince >= minFrames);
}

// The machine counts as quiet when the 1-minute load average (others only)
// is at most half the CPU count (I-015 D2).
function loadGate(load1, cpus) {
  return typeof load1 === 'number' && Number.isFinite(load1) && cpus > 0 && load1 <= cpus * 0.5;
}

// Environment reasons a run may be repeated for; anything else (above all an
// incomplete workload) is a product signal and is never re-run.
const ENV_REASON = /^(machine busy|throttle not verified)/;
// The reason to re-run an invalid run, or null (I-015 D8). `attempt` counts
// the re-runs already made for this repeat.
function rerunPlan(run, attempt, maxReruns = 2) {
  if (!run || run.valid || attempt >= maxReruns || !run.workload_ok) return null;
  const why = run.invalid_reasons || [];
  return why.length && why.every((r) => ENV_REASON.test(r)) ? why.join('; ') : null;
}

// The workload's tp/move/look/turn/face commands work only with `?debug`
// (it gates nothing else), like golden.mjs.
function pageUrl(base, tier, query = '') {
  const extra = String(query || '').replace(/^[?&]+/, '');
  return base + (tier === 'low' ? '?low&debug' : '?debug') + (extra ? `&${extra}` : '');
}
// `--query level=2` (I-035) measures level 2, the Frozen Gate: nothing burns
// there, so the workload casts at the stream and the gate instead of fires
// and does not require a burning object.
function isLevel2(query) { return /(^|&)level=2(&|$)/.test(String(query || '').replace(/^[?&]+/, '')); }
// `--query level=3` (I-037) measures level 3, the Marsh: the workload casts
// every spell at the middle dip vent (burning gas) from the boardwalk.
function isLevel3(query) { return /(^|&)level=3(&|$)/.test(String(query || '').replace(/^[?&]+/, '')); }
function levelOf(query) { return isLevel3(query) ? 3 : isLevel2(query) ? 2 : 1; }

function contextOptions(isMac, { vw, vh, dpr }) {
  return isMac
    ? { viewport: { width: vw, height: vh }, deviceScaleFactor: dpr }
    : { viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 };
}

// Every reason a run cannot count. Empty means valid.
function invalidReasons(r, isMac, { dpr }) {
  const why = [];
  if (r.error_thrown) why.push(`run threw: ${r.error_thrown}`);
  if (r.tti_ms === null || r.tti_ms === undefined) why.push('no interactive frame');
  if (!r.workload_ok) why.push('workload incomplete');
  if (r.console_errors?.count) why.push(`${r.console_errors.count} console error(s)`);
  if (r.panics) why.push(`${r.panics} panic(s)`);
  if (!(r.frames > 0)) why.push('no frames sampled');
  const v = r.visibility;
  if (!v || !(v.samples > 0)) why.push('visibility not sampled');
  else {
    if (v.hidden > 0) why.push(`page hidden in ${v.hidden}/${v.samples} samples`);
    if (isMac && v.unfocused > 0) why.push(`window lost focus in ${v.unfocused}/${v.samples} samples`);
  }
  if (!r.canvas) why.push('canvas not checked');
  else if (r.canvas.blank) why.push('blank canvas');
  if (r.gl_calls) why.push('GL call counting on (diagnostic hook)');
  if (!isMac && r.load_gate && !r.load_gate.ok && !r.load_gate.allowed) why.push(`machine busy: load1=${r.load_gate.load1_busy}`);
  if (isMac) {
    if (r.webgl2 !== true) why.push('not a WebGL2 build');
    if (!r.renderer) why.push('renderer unknown');
    else if (/swiftshader|llvmpipe|softpipe|software/i.test(r.renderer)) why.push(`software renderer: ${r.renderer}`);
    if (!(r.device_pixel_ratio >= dpr)) why.push(`devicePixelRatio ${r.device_pixel_ratio} below ${dpr}`);
  } else {
    if (!r.throttle_applied) why.push('throttle not applied');
    if (!r.throttle_verified) why.push('throttle not verified by calibration');
  }
  return why;
}

// B3 size from what the page requested. files: [{path, raw, brotli}];
// requested: paths; served: Map path -> [body lengths seen].
function sizeAccounting(files, requested, served) {
  const byPath = new Map(files.map((f) => [f.path, f]));
  const req = [...new Set(requested)].sort();
  const reqSet = new Set(req);
  const found = req.filter((p) => byPath.has(p));
  const bytes = found.length ? found.reduce((a, p) => a + byPath.get(p).brotli, 0) : null;
  const body_mismatch = [];
  for (const p of [...served.keys()].sort()) {
    const f = byPath.get(p);
    if (!f) continue;
    const lens = [...new Set([].concat(served.get(p)))];
    if (lens.length !== 1 || lens[0] !== f.raw) body_mismatch.push({ path: p, served: lens, dir: f.raw });
  }
  return {
    requested: req,
    requested_brotli_bytes: bytes,
    requested_brotli_mb: bytes === null ? null : +(bytes / 1e6).toFixed(3),
    missing_from_dir: req.filter((p) => !byPath.has(p)),
    unrequested: files.map((f) => f.path).filter((p) => !reqSet.has(p)).sort(),
    body_mismatch,
    // Requested and in the dir, but no served body was measured: not proven
    // to be the same build.
    body_unverified: found.filter((p) => !served.has(p)),
  };
}

// First time a check passes, and the gap to the last failing check before it
// (how late the stamp can be). Self-contained: its source is injected into
// the page.
function makeTtiTracker() {
  let lastFalse = null;
  const k = {
    tti: null, bound: null,
    check(t, ok) {
      if (k.tti !== null) return true;
      if (ok) { k.tti = t; k.bound = lastFalse === null ? null : t - lastFalse; } else lastFalse = t;
      return ok;
    },
  };
  return k;
}

// I-024 Part T. The `t` of the `STARTUP t=<ms>` line that `setup` logs once
// the wasm is instantiated and the app is built; null when the text has none.
// Anchored, so STARTUPX / XSTARTUP do not match. Self-contained: its source
// is injected into the page.
function startupLogMs(text) {
  const m = /\bSTARTUP t=(\d+(?:\.\d+)?)ms\b/.exec(String(text));
  return m ? +m[1] : null;
}

// I-024 Part T. Splits TTI into four phases (all on the navigation clock):
// fetch (wasm responseEnd), compile_init (responseEnd to STARTUP: compile
// left after the download, instantiate, plugin build), first_frame (STARTUP
// to FIRST_FRAME: Startup systems, pipelines, shaders, the first 3 frames)
// and to_interactive (FIRST_FRAME to TTI). A phase is null when either end
// is. Caveat: with streaming instantiation compile overlaps the download, so
// compile_init is overlap-biased low and fetch includes some compile; read
// the two together. A negative phase is kept as is and named in `warn`.
function ttiPhases(run) {
  const v = (x) => (x === null || x === undefined || !Number.isFinite(x) ? null : x);
  const d = (a, b) => (v(a) === null || v(b) === null ? null : r2(b - a));
  const p = {
    fetch_ms: v(run.wasm_response_end_ms),
    compile_init_ms: d(run.wasm_response_end_ms, run.startup_log_ms),
    first_frame_ms: d(run.startup_log_ms, run.first_frame_log_ms),
    to_interactive_ms: d(run.first_frame_log_ms, run.tti_ms),
  };
  const neg = Object.keys(p).filter((k) => p[k] !== null && p[k] < 0);
  if (neg.length) p.warn = `negative ${neg.join(', ')} (streaming compile overlaps the fetch; read fetch_ms and compile_init_ms together)`;
  return p;
}

// Seeded bootstrap range of a percentile (default: 95% range of the median).
function bootstrapCI(xs, { iters = 2000, seed = 1, p = 0.5, level = 0.95 } = {}) {
  if (!xs.length) return null;
  let s = seed >>> 0 || 1;
  const rnd = () => ((s = (Math.imul(s, 1664525) + 1013904223) >>> 0) / 2 ** 32);
  const n = xs.length, stats = new Array(iters), buf = new Array(n);
  for (let i = 0; i < iters; i++) {
    for (let j = 0; j < n; j++) buf[j] = xs[(rnd() * n) | 0];
    stats[i] = pct(buf, p);
  }
  return [pct(stats, (1 - level) / 2), pct(stats, 1 - (1 - level) / 2)];
}

// Interleaved [throttled_ms, base_ms] pairs -> the median pair ratio, so a
// burst of load from other processes during one measurement does not decide it.
function calibrationRatio(pairs) {
  const rs = pairs.filter(([t, b]) => t > 0 && b > 0).map(([t, b]) => t / b);
  return rs.length ? r2(pct(rs, 0.5)) : null;
}

// The measured slowdown of a fixed busy loop must match the throttle rate.
function throttleOk(rate, ratio) {
  if (typeof ratio !== 'number' || !Number.isFinite(ratio)) return false;
  return rate <= 1 ? ratio <= 1.5 : ratio >= 0.6 * rate && ratio <= 1.6 * rate;
}

// Minimal PNG decoder: 8-bit RGB or RGBA, not interlaced (what Chromium's
// screenshots are).
function decodePng(buf) {
  const sig = [137, 80, 78, 71, 13, 10, 26, 10];
  if (buf.length < 8 || sig.some((b, i) => buf[i] !== b)) throw new Error('not a PNG');
  let off = 8, w = 0, h = 0, ch = 0;
  const idat = [];
  while (off + 8 <= buf.length) {
    const len = buf.readUInt32BE(off), type = buf.toString('latin1', off + 4, off + 8);
    const data = buf.subarray(off + 8, off + 8 + len);
    if (type === 'IHDR') {
      w = data.readUInt32BE(0); h = data.readUInt32BE(4);
      const depth = data[8], ct = data[9], interlace = data[12];
      if (depth !== 8 || (ct !== 2 && ct !== 6) || interlace !== 0) throw new Error(`unsupported PNG (depth ${depth}, color type ${ct}, interlace ${interlace})`);
      ch = ct === 6 ? 4 : 3;
    } else if (type === 'IDAT') idat.push(data);
    else if (type === 'IEND') break;
    off += 12 + len;
  }
  if (!w || !h || !idat.length) throw new Error('PNG has no image data');
  const raw = inflateSync(Buffer.concat(idat));
  const stride = w * ch, out = Buffer.alloc(stride * h);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)], src = y * (stride + 1) + 1, dst = y * stride;
    for (let i = 0; i < stride; i++) {
      const a = i >= ch ? out[dst + i - ch] : 0, b = y ? out[dst - stride + i] : 0, c = i >= ch && y ? out[dst - stride + i - ch] : 0;
      let pred = 0;
      if (f === 1) pred = a; else if (f === 2) pred = b; else if (f === 3) pred = (a + b) >> 1;
      else if (f === 4) { const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c); pred = pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      else if (f !== 0) throw new Error(`bad PNG filter ${f}`);
      out[dst + i] = (raw[src + i] + pred) & 255;
    }
  }
  return { width: w, height: h, channels: ch, data: out, pixel: (x, y) => out.subarray((y * w + x) * ch, (y * w + x) * ch + ch) };
}

// Blank when the luma barely varies or there are fewer than 8 distinct
// colors (5 bits per channel). Samples at most about 250k pixels.
function canvasStats(img) {
  const { width: w, height: h, channels: ch, data } = img;
  const step = Math.max(1, Math.floor(Math.sqrt((w * h) / 250000)));
  let n = 0, sum = 0, sum2 = 0;
  const colors = new Set();
  for (let y = 0; y < h; y += step) for (let x = 0; x < w; x += step) {
    const i = (y * w + x) * ch, r = data[i], g = data[i + 1], b = data[i + 2];
    const l = 0.299 * r + 0.587 * g + 0.114 * b;
    n++; sum += l; sum2 += l * l;
    if (colors.size < 4096) colors.add(((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3));
  }
  const mean = sum / n, sd = Math.sqrt(Math.max(0, sum2 / n - mean * mean));
  return { w, h, luma_mean: r2(mean), luma_stddev: r2(sd), distinct_colors: colors.size, blank: sd < 2 || colors.size < 8 };
}

// docs/loop/mac-perf.md must give the real exit code of the run: the run
// line writes with OUT= and never pipes into tee without pipefail, and jq
// (used to read the result) is in the tools check.
function lintMacRecipe(text) {
  const blocks = [...text.matchAll(/```sh\n([\s\S]*?)```/g)].map((m) => m[1]);
  const problems = [];
  const runBlocks = blocks.filter((b) => /perf\.mjs\s+--mac/.test(b));
  if (!runBlocks.length) problems.push('no `perf.mjs --mac` run line in a sh block');
  for (const b of runBlocks) {
    if (/\|\s*tee\b/.test(b) && !/pipefail/.test(b)) problems.push('run line pipes through tee without pipefail (the exit code is tee\'s)');
    if (!/\bOUT=/.test(b)) problems.push('run line does not write the JSON with OUT=');
  }
  if (!blocks.some((b) => /^\s*(jq --version|command -v jq)\b/m.test(b))) problems.push('jq is used to read the result but not checked in the tools step');
  return problems;
}

// ---- arguments ----------------------------------------------------------
const argv = process.argv.slice(2);
let mac = false, selfTest = false, QUERY = '';
const barOverrides = {};
const positional = [];
for (let i = 0; i < argv.length; i++) {
  const a = argv[i];
  if (a === '--mac') mac = true;
  else if (a === '--self-test') selfTest = true;
  else if (a === '--query' || a.startsWith('--query=')) {
    QUERY = a === '--query' ? argv[++i] : a.slice(8);
    if (!QUERY || !/^[A-Za-z0-9_=&.-]+$/.test(QUERY)) die('--query needs k=v[&k=v...] (letters, digits, _ = & . -)');
  }
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
if (selfTest) process.exit(await runSelfTest());
if (positional.length > 1) die('usage: node tests/e2e/perf.mjs [--mac] [--bars k=v,...] [--query k=v] [WEB_DIR]');
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
if (!Number.isInteger(MIN_FRAMES) || MIN_FRAMES < 1) die('MIN_FRAMES must be an integer >= 1');
const MAX_SAMPLE_S = numEnv('MAX_SAMPLE_S', 150);
const MAC_VW = numEnv('MAC_VW', 1512), MAC_VH = numEnv('MAC_VH', 945);
const MAC_DPR = numEnv('MAC_DPR', 2);
if (MAC_DPR < 1) die('MAC_DPR must be >= 1');
const NO_CAST = env.PERF_NO_CAST === '1';
const SKIP_THROTTLE = env.PERF_SKIP_THROTTLE === '1';
const BLANK_CANVAS = env.PERF_BLANK_CANVAS === '1';
const GL_COUNT = env.PERF_GL_COUNT === '1';
const ALLOW_LOAD = env.PERF_ALLOW_LOAD === '1';
const MAX_RERUNS = 2;
const QUIET_WAIT_S = 60;
const ALL_CFGS = ['default@1x', 'default@4x', 'low@1x', 'low@4x'];
const only = env.ONLY !== undefined && env.ONLY !== '' ? env.ONLY.split(',').map((s) => s.trim()).filter(Boolean) : null;
if (only && !only.length) die(`ONLY lists no configuration: "${env.ONLY}"`);
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
    let path;
    try { path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, ''); }
    catch { res.writeHead(400).end(); return; }
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
// Also catches the FIRST_FRAME console line and records when the game first
// becomes interactive. TTI is checked right after every rAF callback (Bevy
// builds its state inside one) and from a 5 ms timer, so it is not rounded up
// to the next frame; P.tt.bound is the gap to the last failing check.
// P.ttiRaf keeps the old stamp (start of the first tick that sees the state).
// With glCount (PERF_GL_COUNT=1) every WebGL2RenderingContext method is
// wrapped to count calls while P.gl.on (the sampling window).
// I-024: also catches the STARTUP line (startupLog, parsed by startupLogMs).
// Injected as `(pageHook)(makeTtiTracker, glCount, startupLogMs)`.
function pageHook(makeTti, glCount, parseStartup) {
  const P = (window.__perf = { ticks: [], firstFrameLog: null, startupLog: null, tti: null, ttiRaf: null, tt: makeTti(), cur: null, cpuframe: [], maxTicks: 200000 });
  const now = () => performance.now();
  if (glCount && window.WebGL2RenderingContext) {
    const G = (P.gl = { on: false, counts: Object.create(null) });
    const proto = WebGL2RenderingContext.prototype;
    for (const name of Object.getOwnPropertyNames(proto)) {
      const d = Object.getOwnPropertyDescriptor(proto, name);
      if (name === 'constructor' || !d || typeof d.value !== 'function') continue;
      const f = d.value;
      proto[name] = function (...a) { if (G.on) G.counts[name] = (G.counts[name] || 0) + 1; return f.apply(this, a); };
    }
  }
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
  // Parses __gameState only when the string changed, so frequent checks stay cheap.
  let lastStr = null, lastOk = false;
  const interactive = () => {
    if (P.firstFrameLog === null || !window.__gameState) return false;
    const str = window.__gameState;
    if (str === lastStr) return lastOk;
    lastStr = str;
    try {
      const s = JSON.parse(str);
      lastOk = !!(s && Array.isArray(s.objs) && s.objs.length > 0 && s.outcome === 'playing');
    } catch { lastOk = false; }
    return lastOk;
  };
  const checkTti = (t = now()) => { if (P.tt.tti === null && P.tt.check(t, interactive())) P.tti = P.tt.tti; };
  const timer = setInterval(() => { if (P.tt.tti !== null) clearInterval(timer); else checkTti(); }, 5);
  const raf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = (cb) => raf((ts) => {
    const t0 = now();
    if (!P.cur || P.cur.ts !== ts) {
      flush();
      P.cur = { ts, start: t0, cpu: 0 };
      if (P.ttiRaf === null && interactive()) P.ttiRaf = t0;
      if (P.tt.tti === null) checkTti(t0);
    }
    try { cb(ts); } finally {
      P.cur.cpu += now() - t0;
      if (P.tt.tti === null) checkTti();
    }
  });
  const scan = (args) => {
    const t = args.map(String).join(' ');
    if (P.firstFrameLog === null) {
      const m = /FIRST_FRAME t=(\d+(?:\.\d+)?)ms/.exec(t);
      if (m) P.firstFrameLog = +m[1];
    }
    if (P.startupLog === null) P.startupLog = parseStartup(t);
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
function pageWorkload(noCast, timedOut, level = 1) {
  const level2 = level === 2 || level === true, level3 = level === 3;
  const W = (window.__work = { sampling: false, casts: { 1: 0, 2: 0, 3: 0, 4: 0 }, castsTotal: 0, burningMax: 0, yaw: 0, steps: 0, resets: 0,
    vis: { samples: 0, hidden: 0, unfocused: 0, hidden_events: 0 } });
  // A hidden or background window throttles rAF: count it during sampling.
  const visSample = () => {
    if (!W.sampling) return;
    W.vis.samples++;
    if (document.visibilityState !== 'visible') W.vis.hidden++;
    if (!document.hasFocus()) W.vis.unfocused++;
  };
  document.addEventListener('visibilitychange', () => { if (W.sampling && document.visibilityState !== 'visible') { W.vis.hidden_events++; visSample(); } });
  const st = () => { try { return JSON.parse(window.__gameState); } catch { return null; } };
  const cmd = (c) => window.__game.cmd(c);
  let s0 = st();
  const idx = (name) => (level3 && name === 'dip vent' ? (s0.objs.filter((o) => o.n === name)[1] || {}).i : (s0.objs.find((o) => o.n === name) || {}).i);
  const burningAny = (s) => (level3 ? idx('dip vent') : level2 ? idx('ice gate') : (s.objs.find((o) => o.burning && o.n === 'bush') || s.objs.find((o) => o.burning) || {}).i);
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
      const t = level3 ? idx('dip vent') : target(s) ?? idx(level2 ? 'stream' : 'dry hedge');
      cmd(level3 ? 'tp 0 13' : level2 ? 'tp 0 10' : 'tp 0 5.5'); cmd(`face ${t}`); cmd(`select ${spell}`);
      if (!noCast) cmd('cast');
    },
    until: (s) => (noCast ? s.frame > W.f0 + 1 : s.casts > W.c0),
    ms: 6000,
    done: (s) => { if (W.sampling && s.casts > W.c0 && s.spell === spell - 1) W.casts[spell]++; },
  });
  const motion = (...cs) => steps.push({ go: (s) => { W.f0 = s.frame; cs.forEach(cmd); }, until: (s) => s.frame > W.f0, ms: 4000 });
  spellAt(1, () => idx(level2 ? 'stream' : 'dry hedge'));
  motion('move 0 1 0.5', 'turn 400 0');
  spellAt(2, burningAny);
  motion('look 90 10', 'move 1 0 0.4');
  spellAt(3, (s) => (level2 ? idx('stream') : burningAny(s) ?? idx('puddle')));
  motion('turn -400 50', 'move 0 -1 0.4');
  spellAt(4, burningAny);
  motion('look -90 -5');
  let i = 0, t0 = 0, fStart = 0, active = null, lastYaw = null;
  setInterval(() => {
    visSample();
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
      if (!ok && !timedOut(performance.now() - t0, s.frame - fStart, active.ms)) return;
      if (active.done) active.done(s);
      active = null;
      i = (i + 1) % steps.length;
    }
    active = steps[i];
    W.steps++;
    t0 = performance.now();
    fStart = s.frame;
    active.go(s);
  }, 50);
}

// ---- machine load -------------------------------------------------------------
// This script's own processes (node and its browser) are every descendant of
// this process. Their CPU is read from /proc once a second and averaged the
// way the kernel averages load (1-minute exponential decay), so the run's
// own browser can be taken out of the 1-minute load average. Where /proc is
// missing (macOS) the own share is 0 and busy_procs comes from ps alone.
const PROC = (() => { try { readFileSync('/proc/self/stat'); return true; } catch { return false; } })();
const TICK_HZ = 100;
function psRows() {
  try {
    const txt = execFileSync('ps', ['-eo', 'pid=,ppid=,pcpu=,comm='], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout: 5000 });
    return txt.split('\n').map((l) => l.trim().match(/^(\d+)\s+(\d+)\s+([\d.]+)\s+(.*)$/)).filter(Boolean)
      .map((m) => ({ pid: +m[1], ppid: +m[2], pcpu: +m[3], comm: m[4] }));
  } catch { return null; }
}
function ownPids(rows) {
  const own = new Set([process.pid]);
  for (let grew = true; grew;) {
    grew = false;
    for (const r of rows) if (!own.has(r.pid) && own.has(r.ppid)) { own.add(r.pid); grew = true; }
  }
  return own;
}
function procCpuS(pid) {
  try {
    const t = readFileSync(`/proc/${pid}/stat`, 'utf8');
    const f = t.slice(t.lastIndexOf(')') + 2).split(' ');
    return (+f[11] + +f[12]) / TICK_HZ; // utime + stime
  } catch { return null; }
}
const ownLoad = { ema: 0, last: null, cpu: new Map(), timer: null };
function sampleOwnLoad() {
  if (!PROC) return;
  const rows = psRows();
  if (!rows) return;
  const now = Date.now();
  let used = 0;
  const seen = new Map();
  for (const pid of ownPids(rows)) {
    const c = procCpuS(pid);
    if (c === null) continue;
    seen.set(pid, c);
    const prev = ownLoad.cpu.get(pid);
    used += prev === undefined ? 0 : Math.max(0, c - prev);
  }
  ownLoad.cpu = seen;
  if (ownLoad.last !== null) {
    const dt = (now - ownLoad.last) / 1000;
    if (dt > 0) {
      const k = Math.exp(-dt / 60);
      ownLoad.ema = ownLoad.ema * k + (used / dt) * (1 - k);
    }
  }
  ownLoad.last = now;
}
function startOwnLoad() {
  sampleOwnLoad();
  ownLoad.timer = setInterval(sampleOwnLoad, 1000);
  ownLoad.timer.unref();
}
// One moment: load averages, this script's estimated share and the busiest
// other processes (name and %CPU, above 50%, at most 5; null without ps).
function machineLoad() {
  sampleOwnLoad();
  const rows = psRows();
  const own = rows ? ownPids(rows) : new Set();
  const busy = rows ? rows.filter((r) => !own.has(r.pid) && r.pcpu > 50).sort((a, b) => b.pcpu - a.pcpu).slice(0, 5)
    .map((r) => ({ name: r.comm, cpu: r.pcpu })) : null;
  const la = os.loadavg();
  const ownShare = PROC ? r2(ownLoad.ema) : 0;
  return { load_avg: la.map(r2), own: ownShare, others: r2(Math.max(0, la[0] - ownShare)), busy_procs: busy };
}
// The run's load fields and the gate verdict.
function loadRecord(start, end) {
  const cpus = os.cpus().length;
  const at = [start, end].filter(Boolean);
  const busy = at.find((x) => !loadGate(x.others, cpus));
  return {
    load_avg: { start: start?.load_avg ?? null, end: end?.load_avg ?? null },
    load_own_estimate: { start: start?.own ?? null, end: end?.own ?? null },
    busy_procs: { start: start?.busy_procs ?? null, end: end?.busy_procs ?? null },
    load_gate: { limit: cpus * 0.5, ok: at.length > 0 && !busy, allowed: env.PERF_ALLOW_LOAD === '1', load1_busy: busy ? busy.others : null,
      others: { start: start?.others ?? null, end: end?.others ?? null } },
  };
}
// Waits up to QUIET_WAIT_S for the gate to pass before a run.
async function waitForQuiet(cfg) {
  const cpus = os.cpus().length;
  const t0 = Date.now();
  let m = machineLoad();
  let said = false;
  while (!loadGate(m.others, cpus) && Date.now() - t0 < QUIET_WAIT_S * 1000) {
    if (!said) { log(`${cfg}: waiting for quiet machine (load1=${m.load_avg[0]}, own ~${m.own}, limit ${cpus * 0.5})`); said = true; }
    await sleep(2000);
    m = machineLoad();
  }
  return { waited_s: +((Date.now() - t0) / 1000).toFixed(1), quiet: loadGate(m.others, cpus) };
}
// Per-frame GL call summary from the counts of one sampling window.
function glSummary(counts, frames) {
  const n = Math.max(1, frames);
  const total = Object.values(counts).reduce((a, b) => a + b, 0);
  const draws = (counts.drawElementsInstanced || 0) + (counts.drawArraysInstanced || 0);
  const top = Object.fromEntries(Object.entries(counts).sort((a, b) => b[1] - a[1]).slice(0, 10).map(([k, v]) => [k, r2(v / n)]));
  return { frames, per_frame: r2(total / n), draws_per_frame: r2(draws / n), top };
}

// ---- one run ----------------------------------------------------------------
const median = (xs) => { const v = xs.filter((x) => typeof x === 'number'); return v.length ? pct(v, 0.5) : null; };

// Times a fixed busy loop in the page at the throttle that was active during
// the run, then at rate 1, and restores the run's rate. The ratio must match
// the configured rate (throttleOk). Runs on the game page itself, so it checks
// the renderer the run was measured on.
async function calibrateThrottle(page, cdp, rate, sendRate) {
  const time = (n) => page.evaluate((n) => {
    let x = 1;
    const t = performance.now();
    for (let i = 0; i < n; i++) x = (Math.imul(x, 1664525) + 1013904223) | 0;
    window.__perfSink = x;
    return performance.now() - t;
  }, n);
  // Size the loop to about 150 ms under the run's throttle (warms the JIT too).
  let n = 1e5, ms = await time(n);
  while (ms < 150 && n < 1e10) { n = Math.ceil(n * Math.min(8, Math.max(1.5, 160 / Math.max(ms, 1)))); ms = await time(n); }
  // Five interleaved pairs: the run's own throttle state, then rate 1.
  const pairs = [];
  for (let k = 0; k < 5; k++) {
    const throttled = await time(n);
    await cdp.send('Emulation.setCPUThrottlingRate', { rate: 1 });
    const base = await time(n);
    if (sendRate) await cdp.send('Emulation.setCPUThrottlingRate', { rate });
    pairs.push([r2(throttled), r2(base)]);
  }
  const ratio = calibrationRatio(pairs);
  return { rate, iterations: n, pairs_ms: pairs, ratio, ok: throttleOk(rate, ratio) };
}

// Screenshot of the canvas area with every other element hidden, decoded
// here: a black or blank canvas is caught even when state and TTI look fine.
// The overlay is hidden through the CSSOM (an injected <style> would break
// the page's CSP and log a console error), then restored.
async function checkCanvas(page) {
  // One evaluate (no actionability timeouts) hides the overlay and returns
  // the canvas box: on a busy 4x-throttled main thread each round trip can
  // take seconds.
  const box = await page.evaluate(() => {
    const c = document.querySelector('canvas');
    window.__perfHidden = [];
    for (const el of document.body.querySelectorAll('*')) {
      if (el === c || el.contains(c)) continue;
      window.__perfHidden.push([el, el.style.getPropertyValue('visibility'), el.style.getPropertyPriority('visibility')]);
      el.style.setProperty('visibility', 'hidden', 'important');
    }
    if (!c) return null;
    const r = c.getBoundingClientRect();
    return { x: r.x, y: r.y, width: r.width, height: r.height };
  });
  try {
    // page.screenshot with a clip: a locator screenshot first waits for the
    // element to be stable over animation frames, which a loaded software GPU
    // may not deliver in time.
    if (!box || box.width < 1 || box.height < 1) throw new Error('canvas has no box');
    const shot = () => page.screenshot({ type: 'png', timeout: 120000, animations: 'allow', clip: box });
    const png = await shot().catch((e) => { log(`canvas screenshot retry: ${e.message.split('\n')[0]}`); return shot(); });
    return canvasStats(decodePng(png));
  } finally {
    await page.evaluate(() => { for (const [el, v, pr] of window.__perfHidden || []) el.style.setProperty('visibility', v, pr); }).catch(() => {});
  }
}

const sameOriginPath = (u) => decodeURIComponent(u.pathname).replace(/^\/+/, '') || 'index.html';

async function runOne(browser, base, cfg, collect) {
  const isMac = cfg === 'mac';
  const [tier, thr] = isMac ? ['default', '1x'] : cfg.split('@');
  const rate = isMac ? null : parseInt(thr, 10);
  const url = pageUrl(base, tier, QUERY);
  const context = await browser.newContext(contextOptions(isMac, { vw: MAC_VW, vh: MAC_VH, dpr: MAC_DPR }));
  const page = await context.newPage();
  const out = { config: cfg, url, tti_ms: null, tti_bound_ms: null, tti_raf_tick_ms: null, first_frame_log_ms: null, frames: 0,
    cpu_ms: { p50: null, p95: null, p99: null, max: null, p50_ci95: null }, frame_interval_ms: { p50: null, p95: null }, cpuframe_log_p50_ms: null,
    console_errors: { count: 0, first: [] }, panics: 0, workload_ok: false, workload: null, device_pixel_ratio: null, visibility: null,
    canvas: null, throttle_applied: false, throttle_verified: isMac ? null : false, throttle_calibration: null, valid: false, invalid_reasons: [] };
  const errors = [];
  let cpuframeLog = null, statsP5 = null;
  let phases = { boot_ms: null, wasm_response_end_ms: null, startup_log_ms: null };
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
  // Every same-origin request (B3 counts only these) and the length of every
  // served body (it must match the WEB_DIR file, also under BASE_URL).
  const bodies = [];
  page.on('request', (r) => {
    const u = new URL(r.url());
    if (u.origin === origin) collect.requested.add(sameOriginPath(u));
  });
  page.on('response', (resp) => {
    const u = new URL(resp.url());
    if (u.origin !== origin || resp.status() !== 200 || resp.request().method() !== 'GET') return;
    const p = sameOriginPath(u);
    // The body as the page got it; when Playwright cannot hand it over (a
    // streamed .wasm), the same URL fetched again from Node.
    const add = (len) => {
      const l = collect.served.get(p) || [];
      if (!l.includes(len)) l.push(len);
      collect.served.set(p, l);
    };
    bodies.push(resp.body().then((b) => add(b.length), () =>
      fetch(resp.url(), { signal: AbortSignal.timeout(60000) }).then((r) => (r.ok ? r.arrayBuffer() : null))
        .then((b) => { if (b) { add(b.byteLength); collect.refetched.add(p); } }, () => {})));
  });
  let cdp = null;
  try {
    if (!isMac) {
      cdp = await context.newCDPSession(page);
      if (!SKIP_THROTTLE) await cdp.send('Emulation.setCPUThrottlingRate', { rate });
      out.throttle_applied = true;
      out.throttle_rate = rate;
    }
    out.load_start = machineLoad();
    await page.addInitScript({ content: `(${pageHook})(${makeTtiTracker}, ${JSON.stringify(GL_COUNT)}, ${startupLogMs});` });
    log(`${cfg}: loading ${url}`);
    const deadline = Date.now() + TTI_TIMEOUT_S * 1000;
    await page.goto(url, { timeout: TTI_TIMEOUT_S * 1000 }).catch((e) => log(`${cfg}: goto: ${e.message.split('\n')[0]}`));
    let tti = null;
    let ctx = [];
    while (Date.now() < deadline) {
      [tti, ctx] = await page.evaluate(() => [window.__perf.tt.tti, window.__perf.ctx]).catch(() => [null, []]);
      if (tti !== null) break;
      // A WebGPU build can never pass the real-GPU WebGL2 bar: stop early.
      if (isMac && ctx.some((c) => c.startsWith('webgpu'))) break;
      await sleep(200);
    }
    out.contexts = ctx;
    const pf = await page.evaluate(() => {
      const w = performance.getEntriesByType('resource').find((e) => { try { return new URL(e.name).pathname.endsWith('_bg.wasm'); } catch { return false; } });
      return { ff: window.__perf.firstFrameLog, bound: window.__perf.tt.bound, raf: window.__perf.ttiRaf, dpr: window.devicePixelRatio,
        boot: typeof window.__bootStart === 'number' ? window.__bootStart : null, wasmEnd: w ? w.responseEnd : null, startup: window.__perf.startupLog };
    }).catch(() => ({}));
    phases = { boot_ms: r2(pf.boot ?? null), wasm_response_end_ms: r2(pf.wasmEnd ?? null), startup_log_ms: pf.startup ?? null };
    out.first_frame_log_ms = pf.ff ?? null;
    out.device_pixel_ratio = pf.dpr ?? null;
    if (tti === null) {
      log(`${cfg}: no interactive frame${Date.now() >= deadline ? ` within ${TTI_TIMEOUT_S} s` : ' (stopped early: WebGPU build)'}`);
    } else {
      out.tti_ms = r2(tti);
      out.tti_bound_ms = r2(pf.bound ?? null);
      log(`${cfg}: interactive at ${out.tti_ms} ms (bound ${out.tti_bound_ms} ms, FIRST_FRAME t=${out.first_frame_log_ms})`);
      if (BLANK_CANVAS) await page.evaluate(() => document.querySelectorAll('canvas').forEach((c) => { c.style.filter = 'brightness(0)'; }));
      if (isMac) await page.bringToFront();
      await page.evaluate(`(${pageWorkload})(${JSON.stringify(NO_CAST)}, ${stepTimedOut}, ${JSON.stringify(levelOf(QUERY))})`);
      const warm = isMac ? 5 : WARMUP_S, sample = isMac ? MAC_SAMPLE_S : SAMPLE_S;
      await sleep(warm * 1000);
      const a = await page.evaluate(() => {
        window.__work.sampling = true;
        if (window.__perf.gl) { window.__perf.gl.counts = Object.create(null); window.__perf.gl.on = true; }
        return performance.now();
      });
      // The window is SAMPLE_S long, extended (up to MAX_SAMPLE_S) until it
      // holds MIN_FRAMES frames: a software GPU may draw under 2 frames a second.
      const tEnd = Date.now() + sample * 1000, tMax = Date.now() + Math.max(sample, MAX_SAMPLE_S) * 1000;
      while (true) {
        await sleep(250);
        if (Date.now() < tEnd) continue;
        const n = await page.evaluate((a) => window.__perf.ticks.filter((t) => t[0] >= a).length, a);
        if (n >= MIN_FRAMES || Date.now() >= tMax) break;
      }
      const [b, ticks, work, gl] = await page.evaluate(() => {
        window.__work.sampling = false;
        const b = performance.now();
        const g = window.__perf.gl;
        if (g) g.on = false;
        return [b, window.__perf.take(0, b), window.__work, g ? { ...g.counts } : null];
      });
      out.load_end = machineLoad();
      const win = ticks.filter((t) => t[0] >= a && t[0] <= b);
      const cpu = win.map((t) => t[2]);
      const iv = [];
      for (let k = 1; k < win.length; k++) iv.push(win[k][0] - win[k - 1][0]);
      out.frames = win.length;
      out.sample_s = +((b - a) / 1000).toFixed(1);
      const ci = bootstrapCI(cpu, { seed: 1 });
      out.cpu_ms = { p50: r2(pct(cpu, 0.5)), p95: r2(pct(cpu, 0.95)), p99: r2(pct(cpu, 0.99)), max: r2(pct(cpu, 1)), p50_ci95: ci && ci.map(r2) };
      if (gl) out.gl_calls = glSummary(gl, win.length);
      out.frame_interval_ms = { p50: r2(pct(iv, 0.5)), p95: r2(pct(iv, 0.95)) };
      out.cpuframe_log_p50_ms = cpuframeLog;
      out.workload = { casts: work.casts, burning_max: work.burningMax, yaw_deg: r2(work.yaw), steps: work.steps, resets: work.resets };
      out.workload_ok = [1, 2, 3, 4].every((s) => work.casts[s] >= 1) && (work.burningMax >= 1 || isLevel2(QUERY)) && work.yaw >= 90;
      out.visibility = work.vis;
      // The old stamp (start of the first tick that saw the state), for comparison.
      out.tti_raf_tick_ms = r2(await page.evaluate(() => window.__perf.ttiRaf).catch(() => null));
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
      try { out.canvas = await checkCanvas(page); }
      catch (e) { out.canvas_error = e.message.split('\n')[0]; log(`${cfg}: canvas check failed: ${out.canvas_error}`); }
    }
    if (cdp) {
      try {
        out.throttle_calibration = await calibrateThrottle(page, cdp, rate, !SKIP_THROTTLE);
        out.throttle_verified = out.throttle_calibration.ok;
      } catch (e) { out.throttle_calibration = { error: e.message.split('\n')[0] }; }
    }
    if (isMac) {
      // webgl2: the game asked for a webgl2 context and never for WebGPU.
      out.contexts = await page.evaluate(() => window.__perf.ctx).catch(() => out.contexts);
      out.webgl2 = out.contexts.includes('webgl2') && !out.contexts.some((c) => c.startsWith('webgpu'));
      if (!out.webgl2) { out.error = 'not a WebGL2 build'; log(`${cfg}: not a WebGL2 build (contexts: ${out.contexts.join(', ') || 'none'})`); }
    }
  } catch (e) {
    out.error_thrown = e.message.split('\n')[0];
    log(`${cfg}: run threw: ${out.error_thrown}`);
  }
  await Promise.race([Promise.allSettled(bodies), sleep(30000)]);
  out.console_errors = { count: errors.length, first: errors.slice(0, 5) };
  Object.assign(out, loadRecord(out.load_start, out.load_end));
  delete out.load_start; delete out.load_end;
  // I-024 Part T: load phases (appended keys; existing keys keep their order).
  Object.assign(out, phases);
  out.tti_phases = ttiPhases(out);
  out.invalid_reasons = invalidReasons(out, isMac, { dpr: MAC_DPR });
  out.valid = out.invalid_reasons.length === 0;
  log(`${cfg}: frames=${out.frames} cpu_p50=${out.cpu_ms.p50} ci95=${JSON.stringify(out.cpu_ms.p50_ci95)} interval_p50=${out.frame_interval_ms.p50} dpr=${out.device_pixel_ratio} ` +
    `vis=${JSON.stringify(out.visibility)} canvas=${JSON.stringify(out.canvas)} throttle=${JSON.stringify(out.throttle_calibration)} workload=${JSON.stringify(out.workload)} ` +
    `errors=${errors.length} valid=${out.valid}${out.valid ? '' : ` (${out.invalid_reasons.join('; ')})`}`);
  await context.close().catch(() => {});
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
  for (const p of [['tti_ms'], ['tti_bound_ms'], ['tti_raf_tick_ms'], ['first_frame_log_ms'], ['frames'], ['cpuframe_log_p50_ms'], ['fps_p50'], ['fps_p5']]) m[p[0]] = median(reps.map((r) => at(r, p)));
  for (const k of ['p50', 'p95', 'p99', 'max']) m.cpu_ms[k] = median(reps.map((r) => r.cpu_ms[k]));
  // The envelope of the per-repeat ranges (each repeat's p50 lies in its own).
  const cis = reps.map((r) => r.cpu_ms.p50_ci95).filter(Boolean);
  m.cpu_ms.p50_ci95 = cis.length === reps.length ? [Math.min(...cis.map((c) => c[0])), Math.max(...cis.map((c) => c[1]))] : null;
  for (const k of ['p50', 'p95']) m.frame_interval_ms[k] = median(reps.map((r) => r.frame_interval_ms[k]));
  m.valid = reps.every((r) => r.valid);
  m.invalid_reasons = [...new Set(reps.flatMap((r) => r.invalid_reasons))];
  m.workload_ok = reps.every((r) => r.workload_ok);
  m.console_errors = { count: reps.reduce((a, r) => a + r.console_errors.count, 0), first: reps.flatMap((r) => r.console_errors.first).slice(0, 5) };
  m.panics = reps.reduce((a, r) => a + r.panics, 0);
  m.throttle_applied = reps.every((r) => r.throttle_applied);
  m.throttle_verified = reps.every((r) => r.throttle_verified);
  m.canvas = reps.find((r) => !r.canvas || r.canvas.blank)?.canvas ?? reps[0].canvas;
  if (reps.every((r) => r.gl_calls)) m.gl_calls = { ...reps[0].gl_calls, per_frame: median(reps.map((r) => r.gl_calls.per_frame)), draws_per_frame: median(reps.map((r) => r.gl_calls.draws_per_frame)) };
  m.load_gate = { ...reps[0].load_gate, ok: reps.every((r) => r.load_gate?.ok) };
  for (const k of ['boot_ms', 'wasm_response_end_ms', 'startup_log_ms']) m[k] = median(reps.map((r) => r[k]));
  m.tti_phases = {};
  for (const k of ['fetch_ms', 'compile_init_ms', 'first_frame_ms', 'to_interactive_ms']) m.tti_phases[k] = median(reps.map((r) => r.tti_phases?.[k] ?? null));
  if (reps.some((r) => r.tti_phases?.warn)) m.tti_phases.warn = reps.map((r) => r.tti_phases?.warn).filter(Boolean)[0];
  m.repeats = reps;
  return m;
}

// ---- self-test (no browser, no build) -------------------------------------------
// `node tests/e2e/perf.mjs --self-test`: unit tests of the pure helpers above
// and a lint of docs/loop/mac-perf.md. Exit 0 only when every test passes.
async function runSelfTest() {
  const { deflateSync } = await import('node:zlib');
  let passed = 0, failed = 0;
  const t = (name, fn) => {
    try { fn(); passed++; console.log(`ok - ${name}`); }
    catch (e) { failed++; console.log(`not ok - ${name}: ${e && e.message}`); }
  };
  const eq = (a, b, m = '') => { if (JSON.stringify(a) !== JSON.stringify(b)) throw new Error(`${m} expected ${JSON.stringify(b)}, got ${JSON.stringify(a)}`); };
  const ok = (c, m) => { if (!c) throw new Error(m || 'assertion failed'); };

  // 1. Context options.
  t('mac context uses deviceScaleFactor 2 at 1512x945', () => {
    eq(contextOptions(true, { vw: 1512, vh: 945, dpr: 2 }), { viewport: { width: 1512, height: 945 }, deviceScaleFactor: 2 });
  });
  t('cloud context uses deviceScaleFactor 1 at 1280x720', () => {
    eq(contextOptions(false, { vw: 1512, vh: 945, dpr: 2 }), { viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 });
  });

  t('page URL always carries classic=1 (I-052: today\'s courtyard workload) and debug (the workload commands need it), low adds ?low', () => {
    eq([pageUrl('http://h/', 'default'), pageUrl('http://h/', 'low')], ['http://h/?classic=1&debug', 'http://h/?low&classic=1&debug']);
    eq([pageUrl('http://h/', 'low', 'level=2'), pageUrl('http://h/', 'default', '?level=2'), pageUrl('http://h/', 'low', '')], ['http://h/?low&classic=1&debug&level=2', 'http://h/?classic=1&debug&level=2', 'http://h/?low&classic=1&debug']);
    eq([isLevel2('level=2'), isLevel2('a=1&level=2'), isLevel2('level=20'), isLevel2('')], [true, true, false, false]);
    eq([isLevel3('level=3'), isLevel3('low&level=3'), isLevel3('level=30'), isLevel3('level=2')], [true, true, false, false]);
    eq([levelOf('level=3'), levelOf('level=2'), levelOf('')], [3, 2, 1]);
  });

  // 3. Invalid reasons.
  const goodMac = { tti_ms: 1000, workload_ok: true, console_errors: { count: 0 }, panics: 0, frames: 500, webgl2: true,
    renderer: 'ANGLE (Apple, ANGLE Metal Renderer: Apple M5 Pro, Unspecified Version)', device_pixel_ratio: 2,
    visibility: { samples: 100, hidden: 0, unfocused: 0 }, canvas: { blank: false } };
  const goodCloud = { tti_ms: 1000, workload_ok: true, console_errors: { count: 0 }, panics: 0, frames: 40, throttle_applied: true,
    throttle_verified: true, device_pixel_ratio: 1, visibility: { samples: 100, hidden: 0, unfocused: 100 }, canvas: { blank: false } };
  t('clean mac run has no invalid reasons', () => eq(invalidReasons(goodMac, true, { dpr: 2 }), []));
  t('clean cloud run has no invalid reasons (unfocused is fine headless)', () => eq(invalidReasons(goodCloud, false, { dpr: 2 }), []));
  t('SwiftShader renderer invalidates a mac run', () => {
    const r = invalidReasons({ ...goodMac, renderer: 'ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader driver)' }, true, { dpr: 2 });
    ok(r.some((x) => /software renderer/.test(x)), JSON.stringify(r));
  });
  t('llvmpipe renderer invalidates a mac run', () => ok(invalidReasons({ ...goodMac, renderer: 'llvmpipe (LLVM 15.0.7, 256 bits)' }, true, { dpr: 2 }).length > 0));
  t('a missing renderer invalidates a mac run', () => ok(invalidReasons({ ...goodMac, renderer: null }, true, { dpr: 2 }).length > 0));
  t('a hidden sample invalidates a mac run and a cloud run', () => {
    ok(invalidReasons({ ...goodMac, visibility: { samples: 100, hidden: 1, unfocused: 0 } }, true, { dpr: 2 }).some((x) => /hidden/.test(x)));
    ok(invalidReasons({ ...goodCloud, visibility: { samples: 100, hidden: 3, unfocused: 0 } }, false, { dpr: 2 }).some((x) => /hidden/.test(x)));
  });
  t('an unfocused sample invalidates a mac run', () => ok(invalidReasons({ ...goodMac, visibility: { samples: 100, hidden: 0, unfocused: 2 } }, true, { dpr: 2 }).some((x) => /focus/.test(x))));
  t('no visibility samples invalidates a run', () => ok(invalidReasons({ ...goodMac, visibility: { samples: 0, hidden: 0, unfocused: 0 } }, true, { dpr: 2 }).length > 0));
  t('DPR 1 invalidates a mac run', () => ok(invalidReasons({ ...goodMac, device_pixel_ratio: 1 }, true, { dpr: 2 }).some((x) => /devicePixelRatio/.test(x))));
  t('unverified throttle invalidates a cloud run', () => ok(invalidReasons({ ...goodCloud, throttle_verified: false }, false, { dpr: 2 }).some((x) => /throttle/.test(x))));
  t('blank canvas invalidates a run', () => ok(invalidReasons({ ...goodCloud, canvas: { blank: true } }, false, { dpr: 2 }).some((x) => /blank/.test(x))));
  t('missing canvas check invalidates a run', () => ok(invalidReasons({ ...goodCloud, canvas: null }, false, { dpr: 2 }).length > 0));
  t('the old rules still hold (tti, workload, errors, panics, frames, webgl2)', () => {
    for (const bad of [{ tti_ms: null }, { workload_ok: false }, { console_errors: { count: 1 } }, { panics: 1 }, { frames: 0 }, { throttle_applied: false }])
      ok(invalidReasons({ ...goodCloud, ...bad }, false, { dpr: 2 }).length > 0, JSON.stringify(bad));
    ok(invalidReasons({ ...goodMac, webgl2: false }, true, { dpr: 2 }).length > 0);
  });

  // 4/5. Size accounting.
  const files = [
    { path: 'index.html', raw: 3000, brotli: 1000 }, { path: 'app_bg.wasm', raw: 17e6, brotli: 3.5e6 },
    { path: 'stray.bin', raw: 40e6, brotli: 40e6 }, { path: 'favicon.svg', raw: 300, brotli: 200 },
  ];
  t('only requested files count, stray file flagged', () => {
    const a = sizeAccounting(files, ['index.html', 'app_bg.wasm'], new Map([['index.html', [3000]], ['app_bg.wasm', [17e6]]]));
    eq(a.requested_brotli_bytes, 3501000);
    eq(a.requested_brotli_mb, 3.501);
    eq(a.unrequested, ['favicon.svg', 'stray.bin']);
    eq(a.missing_from_dir, []);
    eq(a.body_mismatch, []);
  });
  t('no requests gives a null requested total', () => {
    const a = sizeAccounting(files, [], new Map());
    eq(a.requested_brotli_bytes, null); eq(a.requested_brotli_mb, null);
  });
  t('requested file missing from the dir is listed', () => eq(sizeAccounting(files, ['index.html', 'x.js'], new Map()).missing_from_dir, ['x.js']));
  t('served body length differing from the dir file is a mismatch', () => {
    const a = sizeAccounting(files, ['index.html', 'app_bg.wasm'], new Map([['index.html', [3010]], ['app_bg.wasm', [17e6]]]));
    eq(a.body_mismatch, [{ path: 'index.html', served: [3010], dir: 3000 }]);
  });
  t('a requested dir file with no served length is unverified', () => {
    const a = sizeAccounting(files, ['index.html', 'app_bg.wasm', 'x.js'], new Map([['index.html', [3000]]]));
    eq(a.body_unverified, ['app_bg.wasm']);
    eq(sizeAccounting(files, ['index.html'], new Map([['index.html', [3000]]])).body_unverified, []);
  });
  t('a path served with two lengths is a mismatch', () => {
    const a = sizeAccounting(files, ['index.html'], new Map([['index.html', [3000, 2999]]]));
    eq(a.body_mismatch.length, 1);
  });

  // 6. TTI tracker (the same function is injected into the page).
  t('TTI is the first passing check, bound is the gap to the last failing one', () => {
    const k = makeTtiTracker();
    k.check(100, false); k.check(105, false); k.check(2105, true); k.check(2110, true);
    eq(k.tti, 2105); eq(k.bound, 2000);
  });
  t('TTI stays null with no passing check', () => { const k = makeTtiTracker(); k.check(1, false); eq(k.tti, null); eq(k.bound, null); });
  t('a first check that already passes has a null bound', () => { const k = makeTtiTracker(); k.check(7, true); eq(k.tti, 7); eq(k.bound, null); });

  // 7. Bootstrap CI.
  const lcg = (seed) => () => ((seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0) / 2 ** 32);
  t('bootstrap CI brackets the median and is seeded', () => {
    const r = lcg(1); const xs = Array.from({ length: 31 }, () => 10 + r() * 20);
    const m = pct(xs, 0.5); const ci = bootstrapCI(xs, { seed: 7 });
    ok(ci[0] <= m && m <= ci[1], `${ci} vs ${m}`);
    eq(bootstrapCI(xs, { seed: 7 }), ci);
  });
  t('bootstrap CI narrows with more samples', () => {
    const r = lcg(2); const big = Array.from({ length: 400 }, () => 10 + r() * 20);
    const small = big.slice(0, 20);
    const w = (c) => c[1] - c[0];
    ok(w(bootstrapCI(big, { seed: 3 })) < w(bootstrapCI(small, { seed: 3 })));
  });
  t('bootstrap CI of an empty sample is null', () => eq(bootstrapCI([], { seed: 1 }), null));

  // 8. Throttle check.
  t('throttle check: 4x at 3.9 ok, 1.1 and 9 not; 1x at 1.0 ok, 3.8 not; null not', () => {
    eq([throttleOk(4, 3.9), throttleOk(4, 1.1), throttleOk(4, 9), throttleOk(1, 1.0), throttleOk(1, 3.8), throttleOk(4, null)], [true, false, false, true, false, false]);
  });

  t('calibration ratio is the median of interleaved pair ratios (one noisy pair does not flip it)', () => {
    eq(calibrationRatio([[400, 100], [410, 100], [900, 100], [395, 99], [380, 101]]), 4);
    eq(calibrationRatio([[100, 100], [101, 99], [300, 100], [99, 100], [100, 98]]), 1.02);
    eq(calibrationRatio([]), null);
  });

  // 9. PNG decode and blank detection.
  const crcT = Array.from({ length: 256 }, (_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
  const crc = (b) => { let c = ~0; for (const x of b) c = crcT[(c ^ x) & 255] ^ (c >>> 8); return (~c) >>> 0; };
  const chunk = (type, data) => { const l = Buffer.alloc(4); l.writeUInt32BE(data.length); const td = Buffer.concat([Buffer.from(type), data]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([l, td, c]); };
  const encodePng = (w, h, px /* (x,y)=>[r,g,b,a] */, ch = 4) => {
    const rows = [];
    let prev = Buffer.alloc(w * ch);
    for (let y = 0; y < h; y++) {
      const raw = Buffer.alloc(w * ch);
      for (let x = 0; x < w; x++) { const p = px(x, y); for (let c = 0; c < ch; c++) raw[x * ch + c] = p[c]; }
      const f = y % 5; // exercise all five filter types
      const out = Buffer.alloc(w * ch + 1); out[0] = f;
      for (let i = 0; i < raw.length; i++) {
        const a = i >= ch ? raw[i - ch] : 0, b = prev[i], c = i >= ch ? prev[i - ch] : 0;
        const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c);
        const pr = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
        const pred = [0, a, b, (a + b) >> 1, pr][f];
        out[i + 1] = (raw[i] - pred) & 255;
      }
      rows.push(out); prev = raw;
    }
    const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = ch === 4 ? 6 : 2;
    return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(Buffer.concat(rows))), chunk('IEND', Buffer.alloc(0))]);
  };
  t('PNG decode round-trips every filter type (RGBA and RGB)', () => {
    for (const ch of [4, 3]) {
      const px = (x, y) => [(x * 37 + y) & 255, (y * 11) & 255, (x ^ y) & 255, 255];
      const img = decodePng(encodePng(13, 11, px, ch));
      eq([img.width, img.height], [13, 11]);
      for (const [x, y] of [[0, 0], [12, 10], [5, 7], [3, 9]]) eq(Array.from(img.pixel(x, y)).slice(0, 3), px(x, y).slice(0, 3), `ch=${ch} (${x},${y})`);
    }
  });
  t('a uniform black image is blank', () => ok(canvasStats(decodePng(encodePng(64, 48, () => [0, 0, 0, 255]))).blank === true));
  t('a two-color image is blank (too few colors)', () => ok(canvasStats(decodePng(encodePng(64, 48, (x) => (x < 32 ? [0, 0, 0, 255] : [255, 255, 255, 255])))).blank === true));
  t('a varied image is not blank', () => {
    const r = lcg(5);
    const s = canvasStats(decodePng(encodePng(64, 48, (x, y) => [(x * 4) & 255, (y * 5) & 255, (r() * 255) | 0, 255])));
    ok(s.blank === false && s.luma_stddev > 2 && s.distinct_colors >= 8, JSON.stringify(s));
  });
  t('a non-PNG buffer throws', () => { let threw = false; try { decodePng(Buffer.from('nope')); } catch { threw = true; } ok(threw); });

  // 10. I-015: frame-aware step timeouts, the load gate, p99 and re-runs.
  t('a step does not time out after 6000 ms while fewer than 8 frames have passed', () => {
    eq([stepTimedOut(6000, 7, 6000), stepTimedOut(30000, 0, 6000), stepTimedOut(6000, 8, 6000), stepTimedOut(5999, 100, 6000)], [false, false, true, false]);
  });
  t('a step with no new frames still times out at 10x its timeout (frozen page)', () => {
    eq([stepTimedOut(59999, 0, 6000), stepTimedOut(60000, 0, 6000), stepTimedOut(40000, 0, 4000), stepTimedOut(60000, 7, 6000, 8, 10)], [false, true, true, true]);
  });
  t('load gate: 2.5 on 4 CPUs is busy, 1.0 is quiet, 2.0 is the edge, NaN is busy', () => {
    eq([loadGate(2.5, 4), loadGate(1.0, 4), loadGate(2.0, 4), loadGate(NaN, 4), loadGate(null, 4)], [false, true, true, false, false]);
  });
  t('p99 of 1..100 is 99 (pct)', () => eq(pct(Array.from({ length: 100 }, (_, i) => i + 1), 0.99), 99));
  t('re-run plan: environment reasons re-run at most twice, an incomplete workload never', () => {
    const busy = { valid: false, workload_ok: true, invalid_reasons: ['machine busy: load1=3.1'] };
    const thr = { valid: false, workload_ok: true, invalid_reasons: ['throttle not verified by calibration', 'machine busy: load1=2.4'] };
    eq(rerunPlan(busy, 0), 'machine busy: load1=3.1');
    eq(rerunPlan(thr, 1), 'throttle not verified by calibration; machine busy: load1=2.4');
    eq(rerunPlan(busy, 2), null);
    eq(rerunPlan({ ...busy, workload_ok: false, invalid_reasons: ['workload incomplete', 'machine busy: load1=3'] }, 0), null);
    eq(rerunPlan({ ...busy, invalid_reasons: ['machine busy: load1=3', '1 console error(s)'] }, 0), null);
    eq(rerunPlan({ valid: true, workload_ok: true, invalid_reasons: [] }, 0), null);
  });
  t('a busy load gate and the GL hook invalidate a cloud run; PERF_ALLOW_LOAD lets it count', () => {
    const busyGate = { limit: 2, ok: false, allowed: false, load1_busy: 3.4 };
    ok(invalidReasons({ ...goodCloud, load_gate: busyGate }, false, { dpr: 2 }).includes('machine busy: load1=3.4'));
    eq(invalidReasons({ ...goodCloud, load_gate: { ...busyGate, allowed: true } }, false, { dpr: 2 }), []);
    eq(invalidReasons({ ...goodCloud, load_gate: { limit: 2, ok: true } }, false, { dpr: 2 }), []);
    eq(invalidReasons({ ...goodMac, load_gate: busyGate }, true, { dpr: 2 }), []);
    ok(invalidReasons({ ...goodCloud, gl_calls: { per_frame: 2000 } }, false, { dpr: 2 }).some((x) => /GL call/.test(x)));
  });
  t('GL summary: per-frame totals, instanced draws and the top calls', () => {
    const g = glSummary({ texParameteri: 400, drawElementsInstanced: 30, drawArraysInstanced: 10, bindTexture: 160 }, 10);
    eq([g.per_frame, g.draws_per_frame, Object.keys(g.top)[0], g.top.texParameteri], [60, 4, 'texParameteri', 40]);
  });
  t('load record: the gate fails on the busy moment and names its load', () => {
    const cpus = os.cpus().length, lim = cpus * 0.5;
    const r = loadRecord({ load_avg: [1, 1, 1], own: 0, others: lim - 0.5, busy_procs: [] }, { load_avg: [9, 3, 1], own: 1, others: lim + 1, busy_procs: [{ name: 'rustc', cpu: 390 }] });
    eq([r.load_gate.ok, r.load_gate.load1_busy, r.busy_procs.end[0].name], [false, lim + 1, 'rustc']);
    eq(loadRecord({ load_avg: [0.2, 0, 0], own: 0, others: 0.2, busy_procs: [] }, null).load_gate.ok, true);
  });

  // I-024 Part T: the TTI split.
  t('ttiPhases splits TTI into fetch, compile/init, first frame and to-interactive', () => {
    eq(ttiPhases({ wasm_response_end_ms: 900, startup_log_ms: 2400, first_frame_log_ms: 3100, tti_ms: 3300 }),
      { fetch_ms: 900, compile_init_ms: 1500, first_frame_ms: 700, to_interactive_ms: 200 });
  });
  t('ttiPhases: a null end nulls only the phases that use it', () => {
    eq(ttiPhases({ wasm_response_end_ms: null, startup_log_ms: 2400, first_frame_log_ms: 3100, tti_ms: 3300 }),
      { fetch_ms: null, compile_init_ms: null, first_frame_ms: 700, to_interactive_ms: 200 });
    eq(ttiPhases({ wasm_response_end_ms: 900, startup_log_ms: null, first_frame_log_ms: 3100, tti_ms: 3300 }),
      { fetch_ms: 900, compile_init_ms: null, first_frame_ms: null, to_interactive_ms: 200 });
    eq(ttiPhases({ wasm_response_end_ms: 900, startup_log_ms: 2400, first_frame_log_ms: 3100, tti_ms: null }),
      { fetch_ms: 900, compile_init_ms: 1500, first_frame_ms: 700, to_interactive_ms: null });
    eq(ttiPhases({}), { fetch_ms: null, compile_init_ms: null, first_frame_ms: null, to_interactive_ms: null });
  });
  t('ttiPhases keeps a negative phase and flags it in warn (never clamped)', () => {
    const p = ttiPhases({ wasm_response_end_ms: 2500, startup_log_ms: 2400, first_frame_log_ms: 3100, tti_ms: 3300 });
    eq(p.compile_init_ms, -100);
    eq(p.fetch_ms, 2500);
    ok(typeof p.warn === 'string' && /compile_init_ms/.test(p.warn), JSON.stringify(p));
  });
  t('STARTUP console line: anchored parse', () => {
    eq(startupLogMs('STARTUP t=2412ms'), 2412);
    eq(startupLogMs('INFO realgame_bevy_client: STARTUP t=2412ms'), 2412);
    eq(startupLogMs('STARTUPX t=1ms'), null);
    eq(startupLogMs('XSTARTUP t=1ms'), null);
    eq(startupLogMs('STARTUP t=abc'), null);
  });

  // 2. The Mac recipe gives a real exit code.
  t('mac-perf.md lint passes', () => {
    const text = readFileSync(join(here, '../../docs/loop/mac-perf.md'), 'utf8');
    eq(lintMacRecipe(text), []);
  });
  t('lint catches an unguarded tee, a missing OUT= and a missing jq check', () => {
    const bad = '```sh\nnode -v\n```\n```sh\ncd x && node tests/e2e/perf.mjs --mac dir | tee ~/out.json\n```\n';
    const p = lintMacRecipe(bad);
    ok(p.some((x) => /tee/.test(x)) && p.some((x) => /OUT=/.test(x)) && p.some((x) => /jq/.test(x)), JSON.stringify(p));
  });
  t('lint accepts a pipefail-guarded tee with OUT=', () => {
    const good = '```sh\njq --version\n```\n```sh\nset -o pipefail; OUT=~/o.json node tests/e2e/perf.mjs --mac dir | tee /dev/null; echo "exit=$?"\n```\n';
    eq(lintMacRecipe(good), []);
  });

  console.log(`# self-test: ${passed} passed, ${failed} failed`);
  return failed === 0 && passed > 0 ? 0 : 1;
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
// Requests and served body lengths from every run (a file one run fetches
// lazily still counts).
const collect = { requested: new Set(), served: new Map(), refetched: new Set() };
const reruns = [];
startOwnLoad();
for (const cfg of cfgs) {
  const reps = [];
  for (let k = 0; k < (mac ? 1 : RUNS); k++) {
    let run;
    for (let attempt = 0; ; attempt++) {
      const quiet = mac ? null : await waitForQuiet(cfg);
      run = await runOne(browser, base, cfg, collect);
      if (quiet) run.quiet_wait = quiet;
      const why = mac ? null : rerunPlan(run, attempt, MAX_RERUNS);
      if (!why) break;
      reruns.push({ config: cfg, reason: why });
      log(`${cfg}: re-run ${attempt + 1}/${MAX_RERUNS} (environment: ${why})`);
    }
    reps.push(run);
  }
  runs[cfg] = merge(reps);
}
clearInterval(ownLoad.timer);
Object.assign(size, sizeAccounting(size.files, collect.requested, collect.served));
size.served_lengths = Object.fromEntries([...collect.served.entries()].sort());
size.served_refetched = [...collect.refetched].sort();
log(`requested brotli ${size.requested_brotli_bytes} B (${size.requested_brotli_mb} MB); unrequested: ${size.unrequested.join(', ') || 'none'}; ` +
  `body mismatches: ${size.body_mismatch.length ? JSON.stringify(size.body_mismatch) : 'none'}; unverified: ${size.body_unverified.join(', ') || 'none'}`);

const bv = (value, threshold, cmp, needs) => {
  if (needs && !runs[needs]) return { value: null, threshold, pass: null, skipped: true };
  return { value: value ?? null, threshold, pass: value !== null && value !== undefined && cmp(value, threshold) };
};
const verdict = {};
if (mac) {
  verdict.B2 = { mac: bv(runs.mac.fps_p5, BARS.b2_mac_p5_fps, (v, t) => v >= t, 'mac') };
} else {
  verdict.B3 = {
    // Only what the page requested; the dir total is in size.brotli_total_mb.
    size: bv(size.requested_brotli_mb, BARS.b3_brotli_mb, (v, t) => v <= t),
    tti: bv(runs['default@4x']?.tti_ms ?? null, BARS.b3_tti_ms, (v, t) => v <= t, 'default@4x'),
    tti_low_4x_ms: runs['low@4x']?.tti_ms ?? null,
  };
  verdict.B2 = { cpu: bv(runs['low@4x']?.cpu_ms.p50 ?? null, BARS.b2_cpu_p50_ms, (v, t) => v <= t, 'low@4x'), mac: 'not measured here; see docs/loop/mac-perf.md' };
  // The bootstrap 95% range of that p50: when it straddles the bar, a
  // longer run (MIN_FRAMES, RUNS) could flip the verdict.
  const ci = runs['low@4x']?.cpu_ms.p50_ci95 ?? null;
  if (!verdict.B2.cpu.skipped) Object.assign(verdict.B2.cpu, { ci95: ci, ci_straddles_bar: !!ci && ci[0] <= BARS.b2_cpu_p50_ms && BARS.b2_cpu_p50_ms <= ci[1] });
}
const partial = !mac && !!only && ALL_CFGS.some((c) => !only.includes(c));
const allValid = Object.values(runs).every((r) => r.valid) && size.missing_from_dir.length === 0 &&
  size.body_mismatch.length === 0 && size.body_unverified.length === 0;
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
    min_frames: MIN_FRAMES, max_sample_s: MAX_SAMPLE_S, device_scale_factor: mac ? MAC_DPR : 1, skip_throttle: SKIP_THROTTLE, blank_canvas: BLANK_CANVAS,
    gl_count: GL_COUNT, allow_load: ALLOW_LOAD,
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
  reruns,
};
await browser.close();
if (server) server.close();
const text = JSON.stringify(result, null, 2);
if (env.OUT) writeFileSync(env.OUT, text + '\n');
process.stdout.write(text + '\n');
log(`pass=${pass}${partial ? ' (partial run)' : ''} valid=${allValid} in ${result.elapsed_s} s`);
process.exit(pass ? 0 : 1);
