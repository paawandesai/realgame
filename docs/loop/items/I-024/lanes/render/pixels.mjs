// I-024 render lane, tests 8 (log line) and 9 (picture unchanged).
//
//   node docs/loop/items/I-024/lanes/render/pixels.mjs <WEB_DIR_A> <WEB_DIR_B> <low|default> <out-dir> [--roundtrip]
//
// Loads A then B at the given tier (`?low&debug` or `?debug`), pauses the
// game, hides every DOM layer above the canvas (#hud, #pause, splash), and
// screenshots 5 fixed poses (spawn, greenhouse, lab bench, hedge row,
// courtyard corner). Prints the mean absolute per-channel difference (0-255)
// of B against A for each pose, and every LOW_SHARED_MAT console line each
// load printed. With --roundtrip it also, on B only: frames a plain-material
// chem object (the bush at (12, -6.5) with the greenhouse frame behind it),
// shoots it, drives it to Wet with a Water cast, back to Normal with `reset`,
// re-frames the same pose and shoots again; the difference of the two shots
// is the StateMats round-trip figure. JSON summary on stdout; PNGs in out-dir.
// Run under the Chromium lock.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { writeFileSync, mkdirSync } from 'node:fs';
import { extname, join, normalize, resolve } from 'node:path';
import { inflateSync } from 'node:zlib';
import { createRequire } from 'node:module';

const require = createRequire(join(process.cwd(), 'package.json'));
const { chromium } = require('playwright');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const [dirA, dirB, tier, outDir, flag] = process.argv.slice(2);
if (!dirA || !dirB || !['low', 'default'].includes(tier) || !outDir) {
  console.error('usage: pixels.mjs <WEB_DIR_A> <WEB_DIR_B> <low|default> <out-dir> [--roundtrip]');
  process.exit(2);
}
const roundtrip = flag === '--roundtrip';
mkdirSync(outDir, { recursive: true });

// Yaw 0 faces -z (toward the greenhouse); +90 faces -x.
const POSES = [
  ['spawn', ['tp 0 18', 'look 0 0']],
  ['greenhouse', ['tp 0 5.5', 'look 0 5']],
  ['lab-bench', ['tp 14 6', 'look -90 -10']],
  ['hedge-row', ['tp -14 6', 'look 90 0']],
  ['courtyard-corner', ['tp 18 18', 'look 45 10']],
];

function decodePng(buf) {
  let p = 8, w = 0, h = 0, ct = 0, bd = 0;
  const idat = [];
  while (p < buf.length) {
    const len = buf.readUInt32BE(p), type = buf.toString('ascii', p + 4, p + 8), d = buf.subarray(p + 8, p + 8 + len);
    if (type === 'IHDR') { w = d.readUInt32BE(0); h = d.readUInt32BE(4); bd = d[8]; ct = d[9]; if (d[12]) throw new Error('interlaced PNG'); }
    else if (type === 'IDAT') idat.push(d);
    else if (type === 'IEND') break;
    p += 12 + len;
  }
  if (bd !== 8 || (ct !== 6 && ct !== 2)) throw new Error(`PNG colour type ${ct}/${bd} not handled`);
  const bpp = ct === 6 ? 4 : 3, stride = w * bpp, raw = inflateSync(Buffer.concat(idat));
  const out = Buffer.alloc(w * h * 4), prev = Buffer.alloc(stride), cur = Buffer.alloc(stride);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)];
    raw.copy(cur, 0, y * (stride + 1) + 1, (y + 1) * (stride + 1));
    for (let i = 0; i < stride; i++) {
      const a = i >= bpp ? cur[i - bpp] : 0, b = prev[i], c = i >= bpp ? prev[i - bpp] : 0;
      let v = cur[i];
      if (f === 1) v += a; else if (f === 2) v += b; else if (f === 3) v += (a + b) >> 1;
      else if (f === 4) { const pp = a + b - c, pa = Math.abs(pp - a), pb = Math.abs(pp - b), pc = Math.abs(pp - c); v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      cur[i] = v & 255;
    }
    for (let x = 0; x < w; x++) for (let k = 0; k < 4; k++) out[(y * w + x) * 4 + k] = k < bpp ? cur[x * bpp + k] : 255;
    cur.copy(prev);
  }
  return { w, h, data: out };
}

// Mean absolute difference per RGB channel, 0-255.
function meanDiff(a, b) {
  const A = decodePng(a), B = decodePng(b);
  if (A.w !== B.w || A.h !== B.h) throw new Error('size mismatch');
  let s = 0;
  for (let i = 0; i < A.data.length; i += 4) for (let k = 0; k < 3; k++) s += Math.abs(A.data[i + k] - B.data[i + k]);
  return +(s / (A.w * A.h * 3)).toFixed(4);
}

const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
function serve(root) {
  const server = createServer(async (req, res) => {
    const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '');
    try {
      const body = await readFile(join(root, path || 'index.html'));
      res.writeHead(200, { 'content-type': types[extname(path || 'index.html')] || 'application/octet-stream' }).end(body);
    } catch { res.writeHead(404).end(); }
  });
  return new Promise((r) => server.listen(0, '127.0.0.1', () => r(server)));
}

async function session(browser, dir, tag, doRoundtrip) {
  const server = await serve(resolve(dir));
  const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
  const lowLines = [], errors = [];
  page.on('console', (m) => {
    const t = m.text().replace(/%c/g, '').replace(/ color: .*$/, '');
    if (/LOW_SHARED_MAT/.test(t)) lowLines.push(t.slice(t.indexOf('LOW_SHARED_MAT')));
    if (m.type() === 'error' || /panicked/.test(t)) errors.push(t);
  });
  const url = `http://127.0.0.1:${server.address().port}/?${tier === 'low' ? 'low&' : ''}debug`;
  await page.goto(url);
  const st = () => page.evaluate(() => { try { return JSON.parse(window.__gameState); } catch { return null; } });
  const until = async (pred, ms) => { const t0 = Date.now(); for (;;) { const s = await st(); if (s && pred(s)) return s; if (Date.now() - t0 > ms) throw new Error(`timeout (${tag})`); await sleep(150); } };
  const cmd = (c) => page.evaluate((c) => window.__game.cmd(c), c);
  const frames = async (n) => { const f0 = (await st()).frame; await until((x) => x.frame >= f0 + n, 120000); };
  await until((s) => Array.isArray(s.objs) && s.objs.length > 0, 180000);
  await frames(5);
  const hide = () => page.evaluate(() => {
    for (const el of document.body.querySelectorAll('*')) if (el.tagName !== 'CANVAS' && !el.querySelector('canvas')) el.style.visibility = 'hidden';
  });
  const pause = async (on) => { await page.evaluate((on) => window.__game.pause(on), on); await until((x) => x.paused === on, 10000); };
  const shoot = async (name, cmds) => {
    for (const c of cmds) await cmd(c);
    await frames(6);
    await hide();
    await frames(2);
    const png = await page.screenshot({ timeout: 180000 });
    writeFileSync(join(outDir, `${tag}-${name}.png`), png);
    return png;
  };
  await pause(true);
  const shots = {};
  for (const [name, cmds] of POSES) shots[name] = await shoot(name, cmds);
  let rt = null;
  if (doRoundtrip) {
    const s0 = await st();
    const bush = s0.objs.filter((o) => o.n === 'bush')[4].i; // (12, -6.5): not lit at the start
    const pose = ['tp 10 6', `face ${bush}`];
    const a = await shoot('rt-normal', pose);
    await pause(false);
    await cmd('select 2'); // Water
    await frames(2);
    await cmd('cast');
    const wet = await until((x) => x.objs[bush].wet > 0, 120000);
    await frames(6);
    await pause(true);
    const w = await shoot('rt-wet', pose);
    await pause(false);
    await cmd('reset');
    await until((x) => x.objs[bush].wet === 0 && !x.objs[bush].burning, 30000);
    await frames(6);
    await pause(true);
    const b = await shoot('rt-back', pose);
    rt = { bush, wet_seen: wet.objs[bush].wet, normal_vs_wet: meanDiff(a, w), normal_vs_back: meanDiff(a, b) };
  }
  await page.close();
  server.close();
  return { shots, lowLines, errors, rt };
}

const browser = await chromium.launch();
const A = await session(browser, dirA, 'A', false);
const B = await session(browser, dirB, 'B', roundtrip);
await browser.close();
const diffs = {};
for (const [name] of POSES) diffs[name] = meanDiff(A.shots[name], B.shots[name]);
const out = { tier, a: resolve(dirA), b: resolve(dirB), mean_abs_diff: diffs, max: Math.max(...Object.values(diffs)),
  low_shared_mat: { a: A.lowLines, b: B.lowLines }, errors: { a: A.errors.slice(0, 5), b: B.errors.slice(0, 5) }, roundtrip: B.rt };
console.log(JSON.stringify(out, null, 2));
