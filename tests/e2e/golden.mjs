// Golden path for the first-person prototype (crates/bevy-client).
//
// Drives headless Chromium through the game: pointer lock (or the
// click-to-target fallback), mouse look, WASD, jump, every spell, the
// chemistry state changes, fire spread and the lose state, Esc to pause, and
// the "Save the Greenhouse" win. Reads state through the page's
// `window.__game` hook (web/hud.js).
//
//   bash crates/bevy-client/build-web.sh        # builds web/pkg-out
//   node tests/e2e/golden.mjs                   # serves web/pkg-out itself
//
// Env: BASE_URL (test a running server instead), QUERY (for example `?low`,
// faster on a software GPU), SHOTS (directory for screenshots; none if
// unset), VW/VH (viewport, default 1280x720), IDLE_MS (idle time before the
// closing STATS/CPUFRAME lines). The page always gets `debug` added (the
// test commands need it). Needs `playwright` with Chromium. Exits 1 if any
// check fails. Frames are slow on a software GPU, so each step waits on game
// state rather than on fixed sleeps. Run by the loop's smoke step, not by
// push CI (too slow for the CI budget).
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '../../crates/bevy-client/web/pkg-out');
const VW = +(process.env.VW || 1280), VH = +(process.env.VH || 720);
const shotDir = process.env.SHOTS ? process.env.SHOTS.replace(/\/?$/, '/') : null;
const screenshot = async (page, name) => { if (shotDir) await page.screenshot({ path: `${shotDir}${name}`, timeout: 180000 }); };

const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm' };
async function serve() {
  const server = createServer(async (req, res) => {
    const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '');
    if (path.startsWith('..')) { res.writeHead(403).end(); return; }
    try {
      const body = await readFile(join(root, path || 'index.html'));
      res.writeHead(200, { 'content-type': types[extname(path || 'index.html')] || 'application/octet-stream' });
      res.end(body);
    } catch {
      res.writeHead(404).end();
    }
  });
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  return server;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok: !!ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name} ${detail}`);
}

(async () => {
  const server = process.env.BASE_URL ? null : await serve();
  // `debug` turns on the test commands (tp, tick, look, turn, face, move).
  const q = (process.env.QUERY || '').replace(/^\?/, '');
  const base = (process.env.BASE_URL || `http://127.0.0.1:${server.address().port}/`) + `?${q ? q + '&' : ''}debug`;
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: VW, height: VH } });
  const logs = [];
  const errors = [];
  page.on('console', (m) => {
    const t = m.text().replace(/%c/g, '').replace(/ color: .*$/, '');
    logs.push(t);
    if (m.type() === 'error') errors.push(t);
    if (/STATS|CPUFRAME|FIRST_FRAME|IMPACT|panicked/.test(t)) console.log('  console:', t.slice(0, 240));
  });
  page.on('pageerror', (e) => { errors.push(e.message); console.log('  pageerror:', e.message); });
  await page.goto(base);
  // Polled with evaluate, not waitForFunction: the page's CSP (no
  // 'unsafe-eval') blocks waitForFunction's string evaluation in some setups.
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState));) {
    if (Date.now() - t0 > 120000) throw new Error('game did not start within 120 s');
    await sleep(250);
  }
  const st = () => page.evaluate(() => window.__game.state());
  const cmd = (c) => page.evaluate((c) => window.__game.cmd(c), c);
  // Polls the game state until pred(state) holds (frames are slow on a software GPU).
  const until = async (pred, ms = 30000) => {
    const t0 = Date.now();
    let x = await st();
    while (!pred(x) && Date.now() - t0 < ms) { await sleep(150); x = await st(); }
    return x;
  };
  const tickNow = async () => (await st()).tick;
  const frames = async (n) => { const f0 = (await st()).frame; await until((x) => x.frame >= f0 + n, 60000); };
  await sleep(2500);
  let s = await st();
  check('boots', s && s.objs.length > 10, `objs=${s.objs.length} burning=${s.burning}`);
  check('three fires at start', s.burning === 3);
  const idx = (n) => s.objs.filter((o) => o.n === n).map((o) => o.i);
  const bushes = idx('bush'); const hedge = idx('dry hedge')[0]; const ice = idx('ice wall')[0];
  const puddle = idx('puddle')[0]; const bench = idx('lab bench')[0]; const vent = idx('steam vent')[0];
  const lit = s.objs.filter((o) => o.burning).map((o) => o.i);

  // Screenshot 1: first-person view toward the greenhouse with the gauntlet.
  await frames(3);
  await screenshot(page, '01-first-person.png');

  // A left click with the cursor right over a burning bush must ask for the
  // pointer lock, not cast at the bush (click-to-target is only for touch and
  // the point-and-click mode).
  await cmd(`face ${lit[2]}`);
  await page.mouse.move(VW / 2 + 1, VH / 2 + 1);
  await page.mouse.move(VW / 2, VH / 2);
  s = await until((x) => x.hover === lit[2], 15000);
  const casts0 = s.casts;
  await page.mouse.click(VW / 2, VH / 2);
  s = await until((x) => x.locked, 8000);
  const locked = s.locked;
  await sleep(500);
  s = await st();
  check('a click over a target locks the pointer and does not cast', locked && s.casts === casts0,
    `hover was ${lit[2]}, locked=${locked}, casts ${casts0} -> ${s.casts}`);
  await cmd('look 0 25');
  s = await until((x) => x.pitch > 20, 10000);

  // Turn with the mouse (raw movementX) when locked, else the turn command.
  const yaw0 = s.yaw;
  let lookVia = 'turn command';
  if (locked) {
    for (let k = 0; k < 10; k++) { await page.mouse.move(VW / 2 + (k + 1) * 20, VH * 0.2); await sleep(16); }
    s = await until((x) => Math.abs(x.yaw - yaw0) > 5, 4000);
    lookVia = 'real mouse movementX';
    if (Math.abs(s.yaw - yaw0) <= 5) {
      // Headless Chromium may report movementX = 0 under pointer lock: feed the
      // same pointermove handler a synthetic event instead.
      await page.evaluate(() => document.getElementById('game').dispatchEvent(
        new PointerEvent('pointermove', { bubbles: true, pointerType: 'mouse', movementX: 200, movementY: 0 })));
      lookVia = 'synthetic pointermove (movementX)';
    }
  } else {
    await cmd('turn 200 0');
  }
  s = await until((x) => Math.abs(x.yaw - yaw0) > 5, 10000);
  console.log('  look via', lookVia);
  check('mouse look turns', Math.abs(s.yaw - yaw0) > 5, `yaw ${yaw0} -> ${s.yaw}`);

  // Walk forward with W (keyboard focus on the canvas).
  await page.evaluate(() => document.getElementById('game').focus());
  const p0 = s.pos;
  await page.keyboard.down('w');
  s = await until((x) => Math.hypot(x.pos[0] - p0[0], x.pos[2] - p0[2]) > 2, 20000);
  await page.keyboard.up('w');
  const moved = Math.hypot(s.pos[0] - p0[0], s.pos[2] - p0[2]);
  check('WASD moves', moved > 1.5, `moved ${moved.toFixed(2)} m`);
  // Jump.
  await page.keyboard.down('Space');
  s = await until((x) => x.pos[1] > 0.2, 10000);
  await page.keyboard.up('Space');
  check('jump', s.pos[1] > 0.2, `y=${s.pos[1]}`);
  await sleep(800);

  // Select each spell with 1-4 and the wheel.
  const sel = [];
  for (const [n, k] of ['1', '2', '3', '4'].entries()) { await page.keyboard.down(k); sel.push((await until((x) => x.spell === n, 10000)).spell); await page.keyboard.up(k); }
  check('keys 1-4 select spells', sel.join() === '0,1,2,3', sel.join());
  if (locked) {
    await page.mouse.wheel(0, 100);
    s = await until((x) => x.spell === 0, 10000);
    check('wheel switches spell', s.spell === 0, `spell=${s.spell}`);
    // A real left click while locked casts the selected spell.
    const c0 = s.casts;
    await page.mouse.down();
    s = await until((x) => x.casts > c0, 15000);
    check('left click casts while locked', s.casts > c0, `casts ${c0} -> ${s.casts}`);
    // Aiming must keep working while the fire button is held (browsers send
    // no mousemove while a button is held after winit's preventDefault on
    // pointerdown, so the game reads pointermove). Real moves first, then a
    // synthetic pointermove with the button held if headless reports
    // movementX = 0 under lock.
    {
      const y0 = s.yaw;
      for (let k = 0; k < 6; k++) { await page.mouse.move(VW / 2 - (k + 1) * 20, VH * 0.2); await sleep(16); }
      s = await until((x) => Math.abs(x.yaw - y0) > 3, 3000);
      if (Math.abs(s.yaw - y0) <= 3) {
        await page.evaluate(() => document.getElementById('game').dispatchEvent(
          new PointerEvent('pointermove', { bubbles: true, pointerType: 'mouse', buttons: 1, movementX: -150, movementY: 0 })));
        s = await until((x) => Math.abs(x.yaw - y0) > 3, 10000);
      }
      check('aim turns while the fire button is held', Math.abs(s.yaw - y0) > 3, `yaw ${y0} -> ${s.yaw}`);
    }
    await page.mouse.up();
  }

  // Heat spreads to neighbours (deterministic fast-forward), then reset.
  let t0 = await tickNow();
  await cmd('tick 2400');
  s = await until((x) => x.tick >= t0 + 2400, 20000);
  check('fire spreads to neighbours', s.burning > 3, `burning=${s.burning} after 80 s of sim`);
  t0 = await tickNow();
  await cmd('tick 9000');
  s = await until((x) => x.outcome !== 'playing' || x.tick >= t0 + 9000, 20000);
  check('unchecked fire reaches the greenhouse (lose)', s.outcome === 'lost', s.outcome);
  await cmd('reset');
  s = await until((x) => x.outcome === 'playing' && x.burning === 3, 20000);
  check('reset', s.burning === 3 && s.outcome === 'playing', `burning=${s.burning}`);

  // A screenshot can drop pointer lock in headless Chromium, which pauses the
  // game (as Esc would): resume through the overlay's button.
  const resume = async () => {
    const x = await st();
    if (!x.paused) return;
    await page.click('#resume');
    await until((q) => !q.paused, 8000);
  };
  const castAt = async (i, spell) => {
    await resume();
    await cmd(`face ${i}`);
    await cmd(`select ${spell}`);
    const a = await until((x) => x.spell === spell - 1 && x.aimObj === i, 15000);
    // Casts at the current aim. The real left-click path is checked once
    // above: under headless pointer lock, a press can arrive with a stray
    // movementX that turns the view, which would make these checks flaky.
    await cmd('cast');
    const z = await until((x) => x.impacts > a.impacts && x.captionId >= a.captionId + 2, 60000);
    if (!(z.impacts > a.impacts)) {
      const pick = (x) => JSON.stringify({ locked: x.locked, paused: x.paused, spell: x.spell, aim: x.aimObj, casts: x.casts, impacts: x.impacts, outcome: x.outcome });
      console.log(`  castAt(${i}, ${spell}) did not land: before ${pick(a)} after ${pick(z)}`);
    }
    return { aim: a.aimObj, s: await st() };
  };
  const tp = async (x, z) => { await cmd(`tp ${x} ${z}`); await until((q) => Math.abs(q.pos[0] - x) < 0.5 && Math.abs(q.pos[2] - z) < 0.5, 10000); };

  // Each section starts from a fresh courtyard: on a software GPU the test
  // runs slowly enough that unattended fires could otherwise win meanwhile.
  const fresh = async () => {
    await cmd('reset');
    await until((x) => x.outcome === 'playing' && x.burning === 3 && x.casts === 0, 20000);
  };

  // Fire on the dry hedge (screenshot 2 as it lands).
  await fresh();
  await resume();
  await tp(-12, 8);
  await cmd(`face ${hedge}`); await cmd('select 1');
  let a0 = await until((x) => x.spell === 0 && x.aimObj === hedge, 15000);
  await cmd('cast');
  await until((x) => x.impacts > a0.impacts, 20000);
  await frames(2);
  await screenshot(page, '02-fire-hits-hedge.png');
  s = await st();
  check('fire ignites the dry hedge', s.objs[hedge].burning, `temp=${s.objs[hedge].temp}`);
  console.log('  caption:', s.caption);
  let r = await castAt(hedge, 4);
  check('foam smothers the hedge', !r.s.objs[hedge].burning && r.s.objs[hedge].smothered > 0, `ext=${r.s.extinguished}`);
  check('a fire the player lit does not count toward the objective', r.s.extinguished === 0, `ext=${r.s.extinguished}`);
  r = await castAt(hedge, 1);
  check('foam blanket stops re-ignition (no O2)', !r.s.objs[hedge].burning, r.s.caption);

  // Ice wall: fire melts it in three hits, frost refreezes the puddle.
  await fresh();
  await tp(10, 6);
  for (let k = 0; k < 3; k++) { r = await castAt(ice, 1); await until((x) => x.casts >= 0, 10); }
  check('fire melts the ice wall into a puddle', !r.s.objs[ice].wall && !r.s.objs[ice].frozen, r.s.caption);
  r = await castAt(ice, 3);
  check('frost refreezes the meltwater', r.s.objs[ice].frozen, r.s.caption);
  // Lab bench behind it, now reachable.
  r = await castAt(bench, 2);
  check('lab bench: water-splitting placeholder', /splitting/.test(r.s.caption), r.s.caption);
  r = await castAt(vent, 3);
  check('frost on the steam vent', r.s.objs[vent].temp < 50, `temp=${r.s.objs[vent].temp}`);

  // Frost on the loose puddle (screenshot 3).
  await fresh();
  await resume();
  await tp(4.5, 15.5);
  await cmd(`face ${puddle}`); await cmd('select 3');
  a0 = await until((x) => x.spell === 2 && x.aimObj === puddle, 15000);
  await cmd('cast');
  await until((x) => x.impacts > a0.impacts, 20000);
  await frames(2);
  await screenshot(page, '03-frost-on-puddle.png');
  s = await st();
  check('frost freezes the puddle', s.objs[puddle].frozen, s.caption);

  // Save the Greenhouse from a fresh start: water and frost on two fires,
  // then click-to-target for the third.
  await cmd('reset');
  await until((x) => x.outcome === 'playing' && x.burning === 3 && x.extinguished === 0, 20000);
  await tp(0, 5.5);
  r = await castAt(lit[0], 2);
  check('water puts out fire 1', !r.s.objs[lit[0]].burning, r.s.caption);
  r = await castAt(lit[1], 3);
  check('frost puts out fire 2', !r.s.objs[lit[1]].burning, r.s.caption);

  // Pointer free: Esc (or exit pointer lock) shows the pause overlay.
  if (locked) {
    await page.keyboard.press('Escape');
    await sleep(300);
    let paused = await page.evaluate(() => !document.getElementById('pause').classList.contains('hidden'));
    if (!paused) { await page.evaluate(() => document.exitPointerLock()); await sleep(300); }
    paused = await page.evaluate(() => !document.getElementById('pause').classList.contains('hidden'));
    s = await st();
    check('Esc releases the mouse and pauses', paused && !s.locked && s.paused, `paused=${paused} locked=${s.locked}`);
    await page.click('#pointmode');
    await sleep(200);
  }
  // Drag-to-look (the fallback when pointer lock is blocked): hold the right
  // button and drag, with the pointer free.
  {
    const y0 = (await st()).yaw;
    await page.mouse.move(VW / 2, VH / 2);
    await page.mouse.down({ button: 'right' });
    for (let k = 1; k <= 8; k++) { await page.mouse.move(VW / 2 + k * 15, VH / 2); await sleep(30); }
    await page.mouse.up({ button: 'right' });
    s = await until((x) => Math.abs(x.yaw - y0) > 3, 15000);
    check('drag-to-look turns the view without pointer lock', !s.locked && Math.abs(s.yaw - y0) > 3, `yaw ${y0} -> ${s.yaw}`);
  }
  await cmd(`face ${lit[2]}`);
  await cmd('select 2');
  await page.mouse.move(VW / 2 + 1, VH / 2 + 1);
  await page.mouse.move(VW / 2, VH / 2);
  s = await until((x) => x.hover === lit[2], 15000);
  check('hover finds the target under the cursor', s.hover === lit[2], `hover=${s.hover}`);
  a0 = s;
  await page.mouse.click(VW / 2, VH / 2);
  s = await until((x) => x.impacts > a0.impacts, 20000);
  s = await until((x) => x.outcome === 'won', 5000);
  check('click-to-target casts without lock', !s.objs[lit[2]].burning && !s.locked, s.caption);
  check('Save the Greenhouse win', s.outcome === 'won', `ext=${s.extinguished} burning=${s.burning}`);
  const banner = await page.evaluate(() => !document.getElementById('banner').classList.contains('hidden') && document.querySelector('#banner .big').textContent);
  check('win banner shown', !!banner, banner);
  await screenshot(page, '04-win.png');

  // Idle and busy CPU numbers.
  await cmd('reset');
  await sleep(+(process.env.IDLE_MS || 8000));
  const canvas = await page.evaluate(() => [document.getElementById('game').width, document.getElementById('game').height, devicePixelRatio]);
  console.log('  canvas', canvas.join(' '));
  const stats = logs.filter((l) => /STATS|CPUFRAME/.test(l)).slice(-6);
  console.log(stats.join('\n'));

  await clockAndBudget({ page, st, cmd, until, frames, sleep, logs });

  await feelAndStandards({ page, st, cmd, until, frames, sleep, idx: (n) => s.objs.filter((o) => o.n === n).map((o) => o.i), lit, locked, base, browser, errors });

  await streamsAndFinder({ page, st, cmd, until, frames, sleep, idx: (n) => s.objs.filter((o) => o.n === n).map((o) => o.i), lit, locked, base, browser, errors });

  await castingYouCanSee({ page, st, cmd, until, frames, sleep, idx: (n) => s.objs.filter((o) => o.n === n).map((o) => o.i), lit, locked, base, browser, errors });

  await reviewFollowups({ page, st, cmd, until, frames, sleep, idx: (n) => s.objs.filter((o) => o.n === n).map((o) => o.i), lit, locked, base, browser, errors });

  check('no console errors', errors.length === 0, errors.slice(0, 3).join(' | '));
  const fails = results.filter((r) => !r.ok);
  console.log(`\n${results.length - fails.length}/${results.length} passed`);
  await browser.close();
  if (server) server.close();
  process.exit(fails.length ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(2); });

// ---------------------------------------------------------------------------
// Feel and standards (docs/loop/items/I-007/spec.md, tests 6-19): field of
// view, look speed, hand sway, aim assist, cooldown ring, hit markers and
// sounds, text size and contrast, the idle hint, settings persistence and
// a flashing guard. Each test restores any setting it changes.
// ---------------------------------------------------------------------------
const DEG = Math.PI / 180;
const vfov = (h) => 2 * Math.atan(Math.tan((h * DEG) / 2) * 9 / 16) / DEG;
const hfovOf = (v) => 2 * Math.atan(Math.tan((v * DEG) / 2) * 16 / 9) / DEG;

// A minimal PNG decoder (8-bit RGB or RGBA, non-interlaced: what Chromium
// screenshots are), so the flashing guard needs no new dependency.
async function decodePng(buf) {
  const { inflateSync } = await import('node:zlib');
  let o = 8; let w = 0; let h = 0; let ct = 0; const idat = [];
  while (o < buf.length) {
    const len = buf.readUInt32BE(o); const type = buf.toString('ascii', o + 4, o + 8);
    const d = buf.subarray(o + 8, o + 8 + len);
    if (type === 'IHDR') { w = d.readUInt32BE(0); h = d.readUInt32BE(4); ct = d[9]; if (d[8] !== 8 || d[12] !== 0) throw new Error('unsupported PNG'); }
    if (type === 'IDAT') idat.push(d);
    if (type === 'IEND') break;
    o += 12 + len;
  }
  const bpp = ct === 6 ? 4 : ct === 2 ? 3 : 0;
  if (!bpp) throw new Error(`unsupported PNG colour type ${ct}`);
  const raw = inflateSync(Buffer.concat(idat)); const stride = w * bpp;
  const out = Buffer.alloc(h * stride);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)]; const src = y * (stride + 1) + 1; const dst = y * stride;
    for (let x = 0; x < stride; x++) {
      const a = x >= bpp ? out[dst + x - bpp] : 0; const b = y ? out[dst - stride + x] : 0;
      const c = x >= bpp && y ? out[dst - stride + x - bpp] : 0; const v = raw[src + x];
      let p;
      if (f === 0) p = 0; else if (f === 1) p = a; else if (f === 2) p = b; else if (f === 3) p = (a + b) >> 1;
      else { const pa = Math.abs(b - c); const pb = Math.abs(a - c); const pc = Math.abs(a + b - 2 * c); p = pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      out[dst + x] = (v + p) & 255;
    }
  }
  return { w, h, bpp, data: out };
}
const LIN = Array.from({ length: 256 }, (_, i) => { const c = i / 255; return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; });
const lumaOf = (img) => {
  const n = img.w * img.h; const L = new Float32Array(n);
  for (let i = 0; i < n; i++) { const k = i * img.bpp; L[i] = 0.2126 * LIN[img.data[k]] + 0.7152 * LIN[img.data[k + 1]] + 0.0722 * LIN[img.data[k + 2]]; }
  return L;
};
// Fraction of pixels whose relative luminance changes by 0.1 or more.
const flashFrac = (a, b) => { let c = 0; for (let i = 0; i < a.length; i++) if (Math.abs(a[i] - b[i]) >= 0.1) c++; return c / a.length; };

async function feelAndStandards({ page, st, cmd, until, frames, sleep, idx, lit, locked, base, browser, errors }) {
  console.log('  -- feel and standards (I-007) --');
  const VW = +(process.env.VW || 1280), VH = +(process.env.VH || 720);
  // Sets a settings control and fires the event a person's change would.
  const setCtl = (sel, v, ev) => page.evaluate(([sel, v, ev]) => {
    const el = document.querySelector(sel);
    if (!el) return false;
    if (el.type === 'checkbox') el.checked = !!v; else el.value = String(v);
    el.dispatchEvent(new Event(ev || (el.type === 'range' ? 'input' : 'change'), { bubbles: true }));
    return true;
  }, [sel, v, ev]);
  const text = (sel) => page.evaluate((sel) => { const e = document.querySelector(sel); return e ? e.textContent : null; }, sel);
  const checked = (sel) => page.evaluate((sel) => { const e = document.querySelector(sel); return e ? e.checked : null; }, sel);
  // Polls until pred holds or more than n frames have passed since frame f0.
  const within = async (f0, n, pred, ms = 30000) => {
    const t0 = Date.now();
    let x = await st();
    while (!pred(x) && x.frame <= f0 + n && Date.now() - t0 < ms) { await sleep(20); x = await st(); }
    return { ok: !!pred(x) && x.frame <= f0 + n, x };
  };
  const fresh = async () => {
    await cmd('reset');
    await until((x) => x.outcome === 'playing' && x.burning === 3 && x.casts === 0, 20000);
  };
  const tp = async (x, z) => { await cmd(`tp ${x} ${z}`); await until((q) => Math.abs(q.pos[0] - x) < 0.5 && Math.abs(q.pos[2] - z) < 0.5, 10000); };
  const unpause = async () => { if ((await st()).paused) { await page.evaluate(() => window.__game.pause(false)); await until((x) => !x.paused, 8000); } };
  const aimAt = async (i, spell) => {
    await unpause();
    await cmd(`face ${i}`);
    if (spell) await cmd(`select ${spell}`);
    return until((x) => (!spell || x.spell === spell - 1) && x.aimObj === i && x.cooldown === 0, 15000);
  };
  const bushes = idx('bush');
  let s = await st();

  // Arms an in-page watcher on #hitmarker: every time it is shown, records
  // the state the HUD acted on (read in the same task, before the next game
  // frame), the sound, the shape shown and where it is.
  const armHit = () => page.evaluate(() => {
    const hm = document.getElementById('hitmarker');
    const w = { shows: [], clears: [], n0: window.__game.lastSound ? window.__game.lastSound().n : -1 };
    window.__hitWatch = w;
    if (window.__hitObs) window.__hitObs.disconnect();
    if (!hm) return false;
    let was = hm.classList.contains('show');
    let wasFrame = hm.dataset.frame;
    const vis = (sel) => { const e = hm.querySelector(sel); if (!e) return false; const r = e.getBoundingClientRect(); return r.width > 0 && getComputedStyle(e).visibility !== 'hidden'; };
    const obs = new MutationObserver(() => {
      const on = hm.classList.contains('show');
      if (on && (!was || hm.dataset.frame !== wasFrame)) {
        const s = window.__game.state();
        const r = hm.getBoundingClientRect();
        w.shows.push({ t: performance.now(), cls: hm.className, frame: +hm.dataset.frame, impacts: s.impacts, hitKind: s.hitKind, hitFrame: s.hitFrame, hitX: s.hitX, hitY: s.hitY, locked: s.locked, sound: window.__game.lastSound(), x: vis('.hm-x'), slash: vis('.hm-slash'), star: vis('.hm-star'), box: [r.left, r.top, r.width, r.height] });
      }
      if (!on && was) w.clears.push(performance.now());
      was = on; wasFrame = hm.dataset.frame;
    });
    obs.observe(hm, { attributes: true, attributeFilter: ['class', 'data-frame'] });
    window.__hitObs = obs;
    return true;
  });
  const hitWatch = () => page.evaluate(() => window.__hitWatch);
  // Casts and waits for the impact; returns the state after it and the watcher.
  const castAndLand = async (c = 'cast', ms = 60000) => {
    const a = await st();
    await armHit();
    await cmd(c);
    const z = await until((x) => x.impacts > a.impacts, ms);
    await frames(3);
    return { a, z, w: await hitWatch() };
  };

  // 6. FOV default and slider (GS-1, GS-2).
  {
    s = await st();
    check('feel: default view is 90° wide (58.7° tall)', s.fov >= 55 && s.fov <= 62 && hfovOf(s.fov) >= 88 && hfovOf(s.fov) <= 92,
      `fov=${s.fov} hfov=${s.fov === undefined ? '?' : hfovOf(s.fov).toFixed(1)}`);
    await page.evaluate(() => window.__game.pause(true));
    const f0 = (await st()).frame;
    const set = await setCtl('#fov', 100);
    const r = await within(f0, 10, (x) => Math.abs(x.fov - vfov(100)) <= 0.2);
    const label = await text('#fov-val');
    check('feel: View width slider sets the FOV live', set && r.ok && label === '100°', `fov=${r.x.fov} want ${vfov(100).toFixed(2)} label=${label}`);
    await unpause();
    await cmd(`face ${lit[0]}`);
    await page.mouse.move(VW / 2 + 1, VH / 2 + 1);
    await page.mouse.move(VW / 2, VH / 2);
    s = await until((x) => x.hover === lit[0], 15000);
    check('feel: the cursor ray uses the live FOV', s.hover === lit[0], `hover=${s.hover} want ${lit[0]} fov=${s.fov}`);
    await setCtl('#fov', 90);
    s = await until((x) => Math.abs(x.fov - vfov(90)) <= 0.2, 10000);
  }

  // 7. Camera and hand-sway guard (GS-3).
  {
    await fresh();
    await tp(0, 14);
    await cmd('look 0 0');
    await sleep(600);
    // Samples every game frame while walking back and forth.
    const sample = () => page.evaluate(() => new Promise((res) => {
      const out = [];
      let last = -1; let k = 0; let dir = 1;
      window.__game.cmd('move 0 1 1.5');
      const step = () => {
        const s = window.__game.state();
        if (s.frame !== last) {
          last = s.frame;
          out.push({ cam: s.cam, pos: s.pos, vm: s.vm, casts: s.casts });
          if (++k % 12 === 0) { dir = -dir; window.__game.cmd(`move 0 ${dir} 1.5`); }
        }
        if (out.length >= 30) { window.__game.cmd('move 0 0 0'); res(out); return; }
        requestAnimationFrame(step);
      };
      requestAnimationFrame(step);
    }));
    const judge = (rows) => {
      const camOk = rows.every((r) => r.cam && Math.abs(r.cam[0] - r.pos[0]) <= 0.005 && Math.abs(r.cam[1] - r.pos[1] - 1.6) <= 0.005 && Math.abs(r.cam[2] - r.pos[2]) <= 0.005);
      const spread = (k) => (rows.every((r) => r.vm) ? Math.max(...rows.map((r) => r.vm[k])) - Math.min(...rows.map((r) => r.vm[k])) : NaN);
      return { camOk, sp: [0, 1, 2].map(spread), casts: new Set(rows.map((r) => r.casts)).size === 1, moved: Math.hypot(rows.at(-1).pos[2] - rows[0].pos[2], rows.at(-1).pos[0] - rows[0].pos[0]) };
    };
    await setCtl('#sway', true);
    await until((x) => x.sway === true, 5000);
    const on = judge(await sample());
    check('feel: camera stays at the eye; hand sway moves the gauntlet when on', on.camOk && on.casts && (on.sp[0] > 0.002 || on.sp[1] > 0.002),
      `camOk=${on.camOk} vm spread=${on.sp.map((v) => v.toFixed(4))}`);
    await setCtl('#sway', false);
    await until((x) => x.sway === false, 5000);
    await sleep(400);
    const off = judge(await sample());
    check('feel: hand sway off holds the gauntlet still', off.camOk && off.casts && off.sp.every((v) => v < 0.0005),
      `camOk=${off.camOk} vm spread=${off.sp.map((v) => v.toFixed(5))}`);
    await setCtl('#sway', true);
    await until((x) => x.sway === true, 5000);
  }

  // 8. No camera motion without input (GS-4).
  {
    await fresh();
    await cmd(`face ${lit[0]}`);
    await sleep(500);
    const a = await st();
    await sleep(10000);
    const b = await st();
    check('feel: the view never moves without input (assist on)', a.assist === true && a.yaw === b.yaw && a.pitch === b.pitch,
      `yaw ${a.yaw} -> ${b.yaw}, pitch ${a.pitch} -> ${b.pitch}`);
  }

  // 9. Sensitivity (GS-8).
  {
    const turnBy = async (sens) => {
      await cmd(`set sens ${sens}`);
      await until((x) => x.sens === sens, 5000);
      await cmd('look 90 0');
      const a = await until((x) => Math.abs(x.yaw - 90) < 0.01 && Math.abs(x.pitch) < 0.01, 5000);
      await cmd('turn 100 0');
      const b = await until((x) => Math.abs(x.yaw - a.yaw) > 0.5, 5000);
      return a.yaw - b.yaw;
    };
    const d1 = await turnBy(1);
    check('feel: 100 px turns 5.44° at sens 1', Math.abs(d1 - 5.443) <= 0.05, `turned ${d1.toFixed(3)}°`);
    const d2 = await turnBy(2);
    check('feel: sens 2 turns twice as far', Math.abs(d2 / d1 - 2) <= 0.02, `sens1 ${d1.toFixed(3)}° sens2 ${d2.toFixed(3)}°`);
    await page.evaluate(() => window.__game.pause(true));
    const f0 = (await st()).frame;
    await setCtl('#sens', 1.5);
    const r = await within(f0, 10, (x) => x.sens === 1.5);
    const label = await text('#sens-val');
    const range = await page.evaluate(() => { const e = document.getElementById('sens'); return e ? [+e.min, +e.max] : [NaN, NaN]; });
    check('feel: Look speed slider sets sens live', r.ok && label === '1.50' && range[0] <= 0.5 && range[1] >= 1.5,
      `sens=${r.x.sens} label=${label} range=${range}`);
    await setCtl('#sens', 1);
    await until((x) => x.sens === 1, 5000);
    await unpause();
  }

  // 10. Aim assist runtime toggle (GS-10).
  {
    await fresh();
    // From (4, 2), bush (9.6, -10.2) is 25° right of straight ahead and the
    // next target to its right is about 15° further on: a clear near miss.
    await tp(4, 2);
    const b = bushes[5];
    let found = false;
    s = await aimAt(b);
    if (s.aimObj === b) {
      for (let k = 0; k < 40 && !found; k++) {
        const f = (await st()).frame;
        await cmd('turn 9 0');
        s = await until((x) => x.frame >= f + 2, 5000);
        found = s.aimObj === b && s.assisted === true;
      }
    }
    check('feel: aim assist catches a near miss', found, `bush ${b} aimObj=${s.aimObj} assisted=${s.assisted}`);
    let f0 = (await st()).frame;
    await setCtl('#assist', false);
    const off = await within(f0, 10, (x) => x.assist === false && x.aimObj === -1);
    f0 = (await st()).frame;
    await setCtl('#assist', true);
    const on = await within(f0, 10, (x) => x.assist === true && x.assisted === true && x.aimObj === b);
    check('feel: Aim help toggles at runtime without a reload', found && off.ok && on.ok && on.x.frame > f0,
      `off: assist=${off.x.assist} aim=${off.x.aimObj}; on: assisted=${on.x.assisted} aim=${on.x.aimObj}`);
  }

  // 11. Cooldown ring and pulse (I-007, GS-12).
  {
    await fresh();
    await tp(0, 5.5);
    await aimAt(lit[0], 2);
    const rec = await page.evaluate(() => new Promise((res) => {
      const cross = document.getElementById('crosshair');
      const s0 = window.__game.state();
      const rows = [];
      window.__game.cmd('cast');
      const step = () => {
        const s = window.__game.state();
        const cd = getComputedStyle(cross).getPropertyValue('--cd').trim();
        rows.push({ f: s.frame, casts: s.casts, cooldown: s.cooldown, max: s.cooldownMax, pulse: cross.classList.contains('pulse'), cooling: cross.classList.contains('cooling'), cd: cd === '' ? null : +cd });
        const done = s.casts > s0.casts && s.cooldown === 0 && !cross.classList.contains('cooling');
        if (rows.length > 600 || (done && rows.length > 3)) { res({ s0, rows }); return; }
        requestAnimationFrame(step);
      };
      requestAnimationFrame(step);
    }));
    const rows = rec.rows;
    const first = rows.find((r) => r.casts > rec.s0.casts);
    const fc = first ? first.f : Infinity;
    const pulse = rows.some((r) => r.pulse && r.f <= fc + 2);
    const ring = rows.some((r) => r.f <= fc + 1 && r.cooling && r.cd > 0 && r.cd <= 1);
    const cdOk = !!first && first.cooldown > 0 && first.cooldown <= 0.3 && first.max === 0.3;
    const zero = rows.find((r) => r.f > fc && r.cooldown === 0);
    const gone = !!zero && rows.some((r) => r.f >= zero.f && r.f <= zero.f + 3 && !r.cooling);
    check('feel: a cast pulses the crosshair', pulse, `cast at frame ${fc}`);
    check('feel: cooldown ring shows and empties', cdOk && ring && gone,
      `cooldown=${first && first.cooldown} max=${first && first.max} ring=${ring} cleared=${gone}`);
    const onRing = await page.evaluate(() => {
      const c = document.getElementById('crosshair');
      const had = c.classList.contains('on');
      c.classList.add('on');
      const col = getComputedStyle(c.querySelector('span')).borderTopColor;
      if (!had) c.classList.remove('on');
      return col;
    });
    check('feel: the on-target crosshair shows a ring (shape cue)', !/rgba\(\d+, \d+, \d+, 0\)|transparent/.test(onRing), onRing);
  }

  // 12. Hit markers and sounds (GS-15, GS-16, GS-17, GS-19).
  let soundOk = false;
  {
    await fresh();
    await tp(10, 6);
    const b = bushes[4]; // (12, -6.5): not lit at the start, in clear view from (10, 6)
    const same = (sh, kind, shape) => !!sh && sh.cls.includes('show') && sh.cls.includes(`kind-${kind}`) && sh[shape]
      && sh.hitKind === kind && sh.sound && sh.sound.name === kind && sh.sound.frame === sh.frame && sh.frame - sh.hitFrame <= 1;
    await aimAt(b, 2);
    let r = await castAndLand();
    let sh = r.w.shows[0];
    check('feel: water on a bush shows the hit marker and sound in the same frame', same(sh, 'hit', 'x') && r.z.hitKind === 'hit',
      `hitKind=${r.z.hitKind} show=${JSON.stringify(sh)}`);
    await aimAt(b, 1);
    r = await castAndLand();
    sh = r.w.shows[0];
    check('feel: fire on the wet bush shows the fizzle marker and sound', same(sh, 'fizzle', 'slash') && r.z.hitKind === 'fizzle',
      `hitKind=${r.z.hitKind} show=${JSON.stringify(sh)}`);
    await tp(0, 5.5);
    await aimAt(lit[0], 2);
    r = await castAndLand();
    await sleep(1200);
    const w = await hitWatch();
    sh = w.shows[0];
    const clearMs = sh && w.clears.length ? w.clears[0] - sh.t : Infinity;
    check('feel: putting a fire out shows the big star and the cheer', same(sh, 'out', 'star') && sh.box[2] >= 80 && sh.sound.peak <= 0.15,
      `hitKind=${r.z.hitKind} width=${sh && sh.box[2]} peak=${sh && sh.sound.peak}`);
    check('feel: the marker clears within 1 s', clearMs <= 1000, `${clearMs.toFixed ? clearMs.toFixed(0) : clearMs} ms`);
    await cmd('look 180 60');
    await until((x) => x.pitch > 55 && x.cooldown === 0, 5000);
    r = await castAndLand('cast', 300000); // 120 m of flight: slow on a software GPU
    const miss = await page.evaluate(() => window.__game.lastSound && window.__game.lastSound());
    check('feel: a miss shows no marker and has its own sound', r.z.hitKind === 'miss' && r.w.shows.length === 0 && miss && miss.name === 'miss',
      `hitKind=${r.z.hitKind} shows=${r.w.shows.length} sound=${miss && miss.name}`);
    await setCtl('#sound', false);
    await aimAt(lit[1], 2);
    const n0 = (await page.evaluate(() => window.__game.lastSound())).n;
    r = await castAndLand();
    const muted = await page.evaluate(() => window.__game.lastSound());
    check('feel: Sound off mutes but still counts', muted.played === false && muted.n > n0, JSON.stringify(muted));
    await setCtl('#sound', true);
    soundOk = true;
  }

  // 13. Marker position (point-and-click and locked).
  {
    await fresh();
    await tp(0, 5.5);
    await aimAt(lit[1], 2);
    // Turn left 10° so the target sits right of centre, then tap on it.
    await cmd(`turn ${-(10 * DEG / 0.00095).toFixed(0)} 0`);
    await frames(2);
    const fx = 0.5 + Math.tan(10 * DEG) / (2 * Math.tan(45 * DEG));
    const rect = await page.evaluate(() => { const r = document.getElementById('game').getBoundingClientRect(); return [r.left, r.top, r.width, r.height]; });
    let r = await castAndLand(`tap ${fx.toFixed(4)} 0.5000`);
    let sh = r.w.shows[0];
    const cx = sh ? sh.box[0] + sh.box[2] / 2 : NaN, cy = sh ? sh.box[1] + sh.box[3] / 2 : NaN;
    const want = sh ? [rect[0] + sh.hitX * rect[2], rect[1] + sh.hitY * rect[3]] : [NaN, NaN];
    check('feel: point-and-click marker sits on the impact', !!sh && !sh.locked && sh.hitX > 0.5 && Math.hypot(cx - want[0], cy - want[1]) <= 6,
      `marker (${cx.toFixed(1)}, ${cy.toFixed(1)}) impact (${want[0].toFixed(1)}, ${want[1].toFixed(1)}) hitX=${sh && sh.hitX}`);
    // Locked: the marker sits at the crosshair.
    let lockedNow = false;
    if (locked) {
      await page.evaluate(() => window.__game.pause(true));
      await page.click('#resume');
      lockedNow = (await until((x) => x.locked, 8000)).locked;
    }
    if (lockedNow) {
      await aimAt(lit[2], 2);
      r = await castAndLand();
      sh = r.w.shows[0];
      const c2 = sh ? [sh.box[0] + sh.box[2] / 2, sh.box[1] + sh.box[3] / 2] : [NaN, NaN];
      const mid = [rect[0] + rect[2] / 2, rect[1] + rect[3] / 2];
      check('feel: locked marker sits at the crosshair', !!sh && sh.locked && Math.hypot(c2[0] - mid[0], c2[1] - mid[1]) <= 6,
        `marker (${c2.map((v) => v.toFixed(1))}) centre (${mid})`);
      await page.evaluate(() => document.exitPointerLock());
      await until((x) => !x.locked, 8000);
      if (await page.evaluate(() => !document.getElementById('pause').classList.contains('hidden'))) await page.click('#pointmode');
      await unpause();
    } else {
      check('feel: locked marker sits at the crosshair', false, 'pointer lock unavailable');
    }
  }

  // 14. Latency (GS-12): frames from command to state change, counted in-page.
  {
    await fresh();
    await tp(0, 5.5);
    await aimAt(lit[0], 2);
    const lat = (c, kind) => page.evaluate(([c, kind]) => new Promise((res) => {
      const s0 = window.__game.state();
      const test = { yaw: (s) => s.yaw !== s0.yaw, casts: (s) => s.casts > s0.casts, sens: (s) => s.sens === 2 }[kind];
      window.__game.cmd(c);
      const step = () => {
        const s = window.__game.state();
        if (test(s)) { res(s.frame - s0.frame); return; }
        if (s.frame - s0.frame > 30) { res(99); return; }
        requestAnimationFrame(step);
      };
      requestAnimationFrame(step);
    }), [c, kind]);
    const c = await lat('cast', 'casts');
    const t = await lat('turn 50 0', 'yaw');
    const v = await lat('set sens 2', 'sens');
    check('feel: input reaches the game within 2 frames', t <= 2 && c <= 2 && v <= 2, `turn ${t}, cast ${c}, sens ${v} frames`);
    await setCtl('#sens', 1);
    await until((x) => x.sens === 1, 5000);
  }

  // 19. No flashing from the new effects (GS-6).
  {
    await fresh();
    await tp(0, 5.5);
    await aimAt(lit[0], 2);
    const shots = [];
    const grab = async () => lumaOf(await decodePng(await page.screenshot({ type: 'png', timeout: 180000 })));
    shots.push(await grab());
    await cmd('cast');
    for (let k = 0; k < 19; k++) { await frames(1); shots.push(await grab()); }
    let worst = 0;
    for (let k = 1; k < shots.length; k++) worst = Math.max(worst, flashFrac(shots[k - 1], shots[k]));
    check('feel: the out marker does not flash the screen', worst < 0.2, `worst frame-to-frame change ${(worst * 100).toFixed(1)}%`);
    // A burst of 6 casts in about 2 s.
    await fresh();
    await tp(0, 5.5);
    await aimAt(lit[1], 2);
    const burst = [];
    let prev = await grab();
    const c0 = (await st()).casts;
    // The six casts go out on an in-page timer (one now, then every 350 ms), so slow
    // screenshots do not stretch the burst; frames are sampled meanwhile.
    await page.evaluate(() => { let k = 0; const id = setInterval(() => { window.__game.cmd('cast'); if (++k >= 5) clearInterval(id); }, 350); window.__game.cmd('cast'); });
    const t0 = Date.now();
    while (Date.now() - t0 < 4000 || burst.length < 4) {
      const cur = await grab();
      burst.push({ t: Date.now() - t0, f: flashFrac(prev, cur) });
      prev = cur;
    }
    const fired = (await st()).casts - c0;
    const big = burst.filter((x) => x.f >= 0.2).map((x) => x.t);
    let maxPerSec = 0;
    for (const t of big) maxPerSec = Math.max(maxPerSec, big.filter((u) => u >= t && u < t + 1000).length);
    check('feel: a burst of casts flashes at most 3 times a second', maxPerSec <= 3, `6 casts sent, ${fired} fired, ${burst.length} samples, big changes ${big.length}, max/s ${maxPerSec}`);
  }

  // 16 and 17. Text size, control size and contrast (GS-18, GS-22).
  {
    await fresh();
    // Every visible element with its own text, under #hud or #pause.
    const audit = () => page.evaluate(() => {
      const parse = (c) => { const m = c.match(/rgba?\(([^)]+)\)/); if (!m) return [0, 0, 0, 0]; const p = m[1].split(',').map((v) => parseFloat(v)); return [p[0], p[1], p[2], p.length > 3 ? p[3] : 1]; };
      const lin = (c) => { c /= 255; return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; };
      const lum = ([r, g, b]) => 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
      const over = ([r, g, b, a], bg) => [r * a + bg[0] * (1 - a), g * a + bg[1] * (1 - a), b * a + bg[2] * (1 - a)];
      const ratio = (x, y) => { const a = lum(x); const b = lum(y); return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05); };
      const visible = (e) => e.getClientRects().length > 0 && getComputedStyle(e).visibility !== 'hidden';
      const out = [];
      for (const root of [document.getElementById('hud'), document.getElementById('pause')]) {
        for (const e of [root, ...root.querySelectorAll('*')]) {
          if (!visible(e)) continue;
          if (![...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim())) continue;
          const cs = getComputedStyle(e);
          const fs = parseFloat(cs.fontSize);
          let p = e; let bg = null;
          while (p && (root.contains(p))) { const c = parse(getComputedStyle(p).backgroundColor); if (c[3] >= 0.6) { bg = c; break; } p = p.parentElement; }
          let contrast = 0;
          if (bg) {
            const fg = parse(cs.color);
            const need = fs >= 36 ? 3 : 4.5;
            contrast = Math.min(...[[255, 255, 255], [0, 0, 0]].map((base) => { const b = over(bg, base); return ratio(over(fg, b), b); })) / need;
          }
          out.push({ id: (e.id ? '#' + e.id : e.tagName.toLowerCase() + (e.className && typeof e.className === 'string' ? '.' + e.className.split(' ').join('.') : '')) + ':' + e.textContent.trim().slice(0, 18), fs, contrast });
        }
      }
      const slotsOver = [...document.querySelectorAll('.slot')].filter((e) => e.scrollWidth > e.clientWidth).length;
      const ctrls = [...document.querySelectorAll('#pause input, #pause select, #pause button')].filter(visible).map((e) => { const r = e.getBoundingClientRect(); return { id: e.id || e.tagName, w: r.width, h: r.height }; });
      const hudOut = [...document.getElementById('hud').children].filter(visible).filter((e) => { const r = e.getBoundingClientRect(); return r.left < -0.5 || r.top < -0.5 || r.right > innerWidth + 0.5 || r.bottom > innerHeight + 0.5; }).map((e) => e.id);
      return { out, slotsOver, ctrls, hudOut };
    });
    await page.evaluate(() => window.__game.pause(true));
    let sizeOk = true; let ctrlOk = true; let scaleOk = true; let contrastOk = true; const notes = [];
    for (const [w, h] of [[1920, 1080], [1366, 768]]) {
      await page.setViewportSize({ width: w, height: h });
      await frames(3);
      const a = await audit();
      const small = a.out.filter((e) => e.fs < 18);
      if (small.length || a.slotsOver || a.out.length < 10) { sizeOk = false; notes.push(`${w}: small=${small.map((e) => `${e.id}=${e.fs}`).slice(0, 4)} slotsOver=${a.slotsOver} n=${a.out.length}`); }
      const low = a.out.filter((e) => e.contrast < 1);
      if (low.length) { contrastOk = false; notes.push(`${w}: low contrast ${low.map((e) => `${e.id}=${(e.contrast * (e.fs >= 36 ? 3 : 4.5)).toFixed(2)}`).slice(0, 4)}`); }
      if (w === 1366) {
        const bad = a.ctrls.filter((c) => c.w < 24 || c.h < 24 || ((c.id === 'resume' || c.id === 'pointmode') && c.h < 40));
        if (bad.length || a.ctrls.length < 8) { ctrlOk = false; notes.push(`controls: ${JSON.stringify(bad.slice(0, 4))} n=${a.ctrls.length}`); }
      }
      await setCtl('#textsize', '2');
      await frames(3);
      const b = await audit();
      const before = new Map(a.out.map((e) => [e.id, e.fs]));
      const shrunk = b.out.filter((e) => before.has(e.id) && (e.fs < 2 * before.get(e.id) - 0.5 || e.fs < 36));
      if (shrunk.length || (w === 1920 && b.hudOut.length) || !(await page.evaluate(() => document.getElementById('textsize') !== null))) { scaleOk = false; notes.push(`${w}@200%: ${shrunk.map((e) => `${e.id}=${e.fs}`).slice(0, 4)} outside=${b.hudOut}`); }
      await setCtl('#textsize', '1');
      await frames(2);
    }
    check('feel: all HUD and pause text is at least 18 px and slots fit', sizeOk, notes.join(' | '));
    check('feel: Text size 200% doubles text and keeps the HUD on screen', scaleOk, notes.join(' | '));
    check('feel: pause controls are at least 24 px (buttons 40 px)', ctrlOk, notes.join(' | '));
    check('feel: text contrast is at least 4.5:1 on its panel', contrastOk, notes.join(' | '));
    await page.setViewportSize({ width: VW, height: VH });
    await frames(3);
    await unpause();
  }

  // 18. Idle hint (GS-24, GS-21).
  {
    await fresh();
    const IDLE = 'Look for a fire';
    const look = () => page.evaluate(() => ({
      hint: document.getElementById('hint').classList.contains('hidden') ? '' : document.getElementById('hint').textContent,
      nudge: !!document.querySelector('.slot[data-spell="2"].nudge'),
    }));
    const t0 = Date.now();
    let early = false; let shown = null;
    while (Date.now() - t0 < 52000) {
      const v = await look();
      const on = v.hint.includes(IDLE) && v.nudge;
      if ((v.hint.includes(IDLE) || v.nudge) && Date.now() - t0 < 40000) early = true;
      if (on && shown === null) shown = Date.now() - t0;
      if (shown !== null && Date.now() - t0 > 46000) break;
      await sleep(500);
    }
    check('feel: idle hint appears after 45 s with no progress (not before 40 s)', !early && shown !== null, `shown at ${shown} ms, early=${early}`);
    // Hints off: frames until both the text and the glow are gone, in-page.
    const gone = await page.evaluate((IDLE) => new Promise((res) => {
      const el = document.getElementById('hints');
      const f0 = window.__game.state().frame;
      if (!el) { res(99); return; }
      el.checked = false;
      el.dispatchEvent(new Event('change', { bubbles: true }));
      const step = () => {
        const h = document.getElementById('hint');
        const off = (h.classList.contains('hidden') || !h.textContent.includes(IDLE)) && !document.querySelector('.slot[data-spell="2"].nudge');
        const f = window.__game.state().frame;
        if (off) { res(f - f0); return; }
        if (f - f0 > 30) { res(99); return; }
        requestAnimationFrame(step);
      };
      step();
    }), IDLE);
    let v = await look();
    check('feel: turning Hints off clears the idle hint within 2 frames', shown !== null && gone <= 2 && !v.hint.includes(IDLE) && !v.nudge, `${gone} frames, now ${JSON.stringify(v)}`);
    await setCtl('#hints', true);
    // Pause after 20 s of idle, wait 30 s, resume: no hint for another 40 s.
    await fresh();
    await sleep(20000);
    await page.evaluate(() => window.__game.pause(true));
    await sleep(30000);
    await unpause();
    const t1 = Date.now();
    early = false;
    while (Date.now() - t1 < 40000) {
      v = await look();
      if (v.hint.includes(IDLE) || v.nudge) { early = true; break; }
      await sleep(1000);
    }
    check('feel: the idle clock restarts after a pause', !early, early ? `hint at ${Date.now() - t1} ms after resume` : 'none for 40 s');
  }

  // 15. Settings persist; storage blocked (GS-14).
  {
    await page.evaluate(() => window.__game.pause(true));
    await setCtl('#sens', 1.5);
    await setCtl('#fov', 100);
    await setCtl('#hints', false);
    const stored = await page.evaluate(() => { try { return JSON.parse(localStorage.getItem('alchemist.settings.v1')); } catch (e) { return null; } });
    check('feel: settings are saved', !!stored && stored.sens === 1.5 && stored.hfov === 100 && stored.hints === false, JSON.stringify(stored));
    const boot = async () => {
      await page.reload();
      for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState));) {
        if (Date.now() - t0 > 120000) throw new Error('game did not restart within 120 s');
        await sleep(100);
      }
      return (await st()).frame;
    };
    let f0 = await boot();
    let r = await within(f0, 10, (x) => x.sens === 1.5 && Math.abs(x.fov - vfov(100)) <= 0.2);
    const hintsBox = await checked('#hints');
    check('feel: saved settings apply after a reload', r.ok && hintsBox === false, `sens=${r.x.sens} fov=${r.x.fov} hints=${hintsBox}`);
    await page.evaluate(() => { try { localStorage.removeItem('alchemist.settings.v1'); } catch (e) { /* blocked */ } });
    f0 = await boot();
    await frames(5);
    s = await st();
    check('feel: with nothing saved the defaults return', s.sens === 1 && Math.abs(s.fov - 58.7) <= 0.2 && (await checked('#hints')) === true, `sens=${s.sens} fov=${s.fov}`);
    // Storage that throws, reduced motion, and a URL sens.
    const ctx = await browser.newContext({ viewport: { width: VW, height: VH }, reducedMotion: 'reduce' });
    await ctx.addInitScript(() => {
      Storage.prototype.getItem = () => { throw new Error('blocked'); };
      Storage.prototype.setItem = () => { throw new Error('blocked'); };
    });
    const p2 = await ctx.newPage();
    const errs2 = [];
    p2.on('console', (m) => { if (m.type() === 'error') errs2.push(m.text()); });
    p2.on('pageerror', (e) => errs2.push(e.message));
    const url2 = base.replace(/\?.*$/, '') + '?low&debug&sens=2';
    await p2.goto(url2);
    for (const t0 = Date.now(); !(await p2.evaluate(() => !!window.__gameState));) {
      if (Date.now() - t0 > 120000) throw new Error('second context did not start within 120 s');
      await sleep(250);
    }
    const st2 = () => p2.evaluate(() => window.__game.state());
    const t2 = Date.now();
    let s2 = await st2();
    while (!(s2.frame > 12) && Date.now() - t2 < 30000) { await sleep(150); s2 = await st2(); }
    const sway2 = await p2.evaluate(() => { const e = document.getElementById('sway'); return e ? e.checked : null; });
    await p2.evaluate(() => window.__game.cmd('cast'));
    for (const t0 = Date.now(); Date.now() - t0 < 20000; await sleep(100)) { if ((await st2()).casts > 0) break; }
    const anim = await p2.evaluate(() => getComputedStyle(document.getElementById('crosshair')).animationName);
    await p2.evaluate(() => { try { localStorage.setItem('x', '1'); } catch (e) { /* expected */ } });
    await sleep(500);
    check('feel: blocked storage + reduced motion boots with URL sens and no sway', s2.sens === 2 && Math.abs(s2.fov - 58.7) <= 0.2 && sway2 === false && s2.sway === false && anim === 'none' && errs2.length === 0,
      `sens=${s2.sens} fov=${s2.fov} sway box=${sway2} state=${s2.sway} anim=${anim} errors=${errs2.slice(0, 2).join(' | ')}`);
    await ctx.close();
  }
  void soundOk; void errors;
}

// ---------------------------------------------------------------------------
// Clock and budget (docs/loop/items/I-015/spec.md, tests 4-9 and 13): game
// time keeps up with the wall clock down to 4 fps (`gameTime`, `dt`), walking
// and casting keep their wall-clock rates, thin walls hold at large dt, no
// 0.05 s clamp is left, memory stays flat and the state JSON only gains the
// two new keys at the end. Written from the spec's Contract alone.
// ---------------------------------------------------------------------------
const MAX_DT = 0.25;
// Keys the merge base (a3e50b1) publishes, in order; I-015 appends gameTime, dt.
const BASE_KEYS = ['frame', 'tick', 'locked', 'paused', 'spell', 'spellName', 'formula', 'aimObj', 'assisted', 'hover',
  'extinguished', 'goal', 'burning', 'ghTemp', 'ghDanger', 'outcome', 'caption', 'captionId', 'casts', 'impacts', 'pos',
  'yaw', 'pitch', 'onIce', 'low', 'sens', 'assist', 'fov', 'sway', 'cam', 'vm', 'cooldown', 'cooldownMax', 'hitKind',
  'hitX', 'hitY', 'hitFrame', 'objs'];
const OBJ_KEYS = ['i', 'n', 'burning', 'temp', 'wet', 'ice', 'wall', 'frozen', 'smothered', 'fuel'];

// Text of `fn <name>` up to the next top-level `fn`/`pub fn` (null if absent).
function fnBody(src, name) {
  const m = new RegExp(`\\n(?:pub )?fn ${name}\\b`).exec(src);
  if (!m) return null;
  const rest = src.slice(m.index + 1);
  const next = rest.slice(1).search(/\n(?:pub )?fn /);
  return next < 0 ? rest : rest.slice(0, next + 1);
}

async function clockAndBudget({ page, st, cmd, until, frames, sleep, logs }) {
  console.log('  -- clock and budget (I-015) --');
  // In-page sampler: one row per new state.frame, [wall ms, frame, gameTime,
  // dt, pos, paused, casts], plus a JSON.parse check of every frame's raw
  // state string (test 13) for as long as the section runs.
  await page.evaluate(() => {
    const S = (window.__clk = { rows: [], on: false, parsed: 0, parseErr: 0, badKeys: [], last: -1, keyOrder: null });
    const step = () => {
      if (S.stop) return;
      const raw = window.__gameState;
      if (raw) {
        let s = null;
        try { s = JSON.parse(raw); } catch { S.parseErr++; }
        if (s && s.frame !== S.last) {
          S.last = s.frame;
          S.parsed++;
          if (!S.keyOrder) S.keyOrder = Object.keys(s);
          else if (Object.keys(s).join() !== S.keyOrder.join() && S.badKeys.length < 3) S.badKeys.push(Object.keys(s).join());
          if (S.on) S.rows.push([performance.now(), s.frame, s.gameTime, s.dt, s.pos, s.paused, s.casts]);
        }
      }
      requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  });
  const startRows = () => page.evaluate(() => { window.__clk.rows = []; window.__clk.on = true; return window.__clk.last; });
  const stopRows = () => page.evaluate(() => { window.__clk.on = false; return window.__clk.rows; });
  // Drops the first row; expectedGame = sum of min(interval, 0.25) over the rest.
  const span = (rows) => {
    let expected = 0, wall = 0;
    for (let k = 1; k < rows.length; k++) { const iv = (rows[k][0] - rows[k - 1][0]) / 1000; expected += Math.min(iv, MAX_DT); wall += iv; }
    return { expected, wall, n: rows.length };
  };
  const unpause = async () => { if ((await st()).paused) { await page.evaluate(() => window.__game.pause(false)); await until((x) => !x.paused, 8000); } };
  const fresh = async () => {
    await unpause();
    await cmd('reset');
    await until((x) => x.outcome === 'playing' && x.burning === 3 && x.casts === 0, 20000);
  };
  const tp = async (x, z) => { await cmd(`tp ${x} ${z}`); await until((q) => Math.abs(q.pos[0] - x) < 0.5 && Math.abs(q.pos[2] - z) < 0.5, 10000); };
  const fmt = (v, d = 3) => (Number.isFinite(v) ? v.toFixed(d) : String(v));

  // 13 (first half). The state JSON keeps every base key, in order, and only
  // appends gameTime and dt; objs keep their keys.
  await fresh();
  const s0 = await st();
  const keys = Object.keys(s0);
  // I-019 appends its keys after dt, in this order (the one sanctioned edit
  // to this check: docs/loop/items/I-019/spec.md, Contract).
  const I019_KEYS = ['clicks', 'firesLeft', 'nearFire', 'fireBearing', 'fireElev', 'fireOff', 'streams', 'fxWater'];
  // I-025 appends fireInView after them (its Contract item 3).
  const I025_KEYS = ['fireInView'];
  // I-027 appends its progress keys after them (its Contract).
  const I027_KEYS = ['progress', 'progRev', 'progSave'];
  const keysOk = keys.length === BASE_KEYS.length + 2 + I019_KEYS.length + I025_KEYS.length + I027_KEYS.length
    && [...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS, ...I025_KEYS, ...I027_KEYS].every((k, i) => keys[i] === k);
  const objKeysOk = s0.objs.every((o) => Object.keys(o).join() === OBJ_KEYS.join());

  // 4. Game time keeps up with the wall clock, and stops while paused. The
  // page is idle (no command) while it samples.
  {
    await fresh();
    await frames(2);
    await startRows();
    await sleep(8000);
    const rows = await stopRows();
    const w = span(rows);
    const dg = rows.length > 1 ? rows.at(-1)[2] - rows[0][2] : NaN;
    const dtOk = rows.every((r) => typeof r[3] === 'number' && r[3] <= 0.2501);
    const ok = rows.length >= 3 && dg >= 0.9 * w.expected && dg <= 1.1 * w.wall && dtOk;
    // Pause with Esc (the pointer is free here), as the golden path does.
    await page.keyboard.press('Escape');
    let p = await until((x) => x.paused, 3000);
    if (!p.paused) { await page.evaluate(() => document.exitPointerLock()); p = await until((x) => x.paused, 3000); }
    await startRows();
    await sleep(3000);
    const prow = (await stopRows()).filter((r) => r[5] === true);
    const dPaused = prow.length > 1 ? prow.at(-1)[2] - prow[0][2] : NaN;
    await unpause();
    check('clock: game time keeps up with the wall clock and stops while paused',
      ok && p.paused && prow.length >= 2 && dPaused <= 0.001,
      `frames=${rows.length} dGame=${fmt(dg)} expected=${fmt(w.expected)} wall=${fmt(w.wall)} maxDt=${Math.max(...rows.map((r) => r[3] ?? NaN))} ` +
      `paused=${p.paused} paused frames=${prow.length} dGame paused=${fmt(dPaused, 4)}`);
  }

  // 5. Walking covers the right distance in wall time. From (-12, 17) facing
  // +x (look 270): z = 17 is clear and flat from x = -17 to x = 17 (benches
  // at z = 14, trees at (-18, 19) and (18.5, 18.5), wall pillars at
  // |x| = 24; no puddle or ice), and a 3 s hold covers at most 15 m.
  // Checked by hand against world.rs.
  {
    await fresh();
    await tp(-12, 17);
    await cmd('look 270 0');
    await until((x) => Math.abs(x.yaw - 270) < 0.5, 5000);
    await frames(2);
    await page.evaluate(() => document.getElementById('game').focus());
    await startRows();
    await page.keyboard.down('w');
    // The window: rows from the first frame after key-down to the last frame
    // before key-up.
    const fDown = (await st()).frame;
    const t0 = Date.now();
    const after = () => page.evaluate((f) => window.__clk.rows.filter((r) => r[1] > f).length, fDown);
    // Hold 3 s, extended until the window has 4 rows (cap 12 s).
    while (Date.now() - t0 < 3000 || ((await after()) < 4 && Date.now() - t0 < 12000)) await sleep(100);
    const fUp = (await st()).frame;
    await page.keyboard.up('w');
    const rows = (await stopRows()).filter((r) => r[1] > fDown && r[1] <= fUp);
    if (rows.length < 4) {
      check('clock: walking covers 5 m/s of game time', false, `invalid: too few frames (${rows.length} in ${Date.now() - t0} ms)`);
    } else {
      const w = span(rows);
      const a = rows[0][4], b = rows.at(-1)[4];
      const dist = Math.hypot(b[0] - a[0], b[2] - a[2]);
      const lo = 0.85 * 5.0 * w.expected - 0.3, hi = 1.1 * 5.0 * w.expected + 0.2;
      check('clock: walking covers 5 m/s of game time', dist >= lo && dist <= hi,
        `dist=${fmt(dist, 2)} m expectedGame=${fmt(w.expected)} s frames=${rows.length} bounds [${fmt(lo, 2)}, ${fmt(hi, 2)}]`);
    }
    await frames(2);
  }

  // 6. No tunnelling through the greenhouse's 0.1 m west pane (x = -8) at
  // large dt: sprinting at it, then a scripted strafe into it. A guard: the
  // old 0.05 s clamp also held; unit test 3b proves the substeps matter.
  {
    const limit = -8 - 0.35 + 0.01;
    await fresh();
    await tp(-12, -17);
    await cmd('look 270 0');
    await until((x) => Math.abs(x.yaw - 270) < 0.5, 5000);
    await page.evaluate(() => document.getElementById('game').focus());
    await startRows();
    await page.keyboard.down('Shift');
    await page.keyboard.down('w');
    await sleep(4000);
    await page.keyboard.up('w');
    await page.keyboard.up('Shift');
    let rows = await stopRows();
    const sprintMax = Math.max(...rows.map((r) => r[4][0]));
    const sprintOk = rows.length >= 2 && sprintMax <= limit;
    // Turned to face north (yaw 0), a scripted strafe right (+x) into the pane.
    await tp(-12, -17);
    await cmd('look 0 0');
    await until((x) => Math.abs(x.yaw) < 0.5 || Math.abs(x.yaw - 360) < 0.5, 5000);
    const g0 = (await st()).gameTime;
    await startRows();
    await cmd('move 1 0 3');
    const t0 = Date.now();
    while (Date.now() - t0 < 4000 || (((await st()).gameTime - g0) < 3.2 && Date.now() - t0 < 40000)) await sleep(150);
    rows = await stopRows();
    const strafeMax = Math.max(...rows.map((r) => r[4][0]));
    check('clock: no tunnelling through the 0.1 m greenhouse pane at large dt', sprintOk && rows.length >= 2 && strafeMax <= limit,
      `sprint max x=${fmt(sprintMax)} strafe max x=${fmt(strafeMax)} (limit ${fmt(limit, 2)})`);
  }

  // 7. Cast cadence in wall time: a cast command every 100 ms for 6 s.
  {
    await fresh();
    await frames(2);
    await cmd('select 2');
    await cmd('face 0');
    await frames(1);
    const c0 = (await st()).casts;
    await startRows();
    await page.evaluate(() => {
      const id = setInterval(() => window.__game.cmd('cast'), 100);
      setTimeout(() => clearInterval(id), 6000);
      window.__game.cmd('cast');
    });
    await sleep(6000);
    const rows = await stopRows();
    const w = span(rows);
    const casts = (await st()).casts - c0;
    const need = Math.max(2, Math.floor(0.8 * w.expected / 0.3) - 1);
    check('clock: cast cadence keeps the 0.3 s cooldown in wall time', casts >= need,
      `casts=${casts} need ${need} expectedGame=${fmt(w.expected)} frames=${rows.length}`);
    await cmd('reset');
  }

  // 7b. Spells land at large dt: water at target 0 (the lit bush at
  // (-13.5, -3)) from (-13.5, 12), 15 m away down a clear line (the steam
  // vent's box ends at x = -14, the path is flat).
  {
    await fresh();
    await tp(-13.5, 12);
    await cmd('face 0');
    await cmd('select 2');
    const a = await until((x) => x.spell === 1 && x.aimObj === 0 && x.cooldown === 0, 15000);
    await cmd('cast');
    const f0 = a.frame;
    const t0 = Date.now();
    let z = await st();
    while (!(z.impacts > a.impacts) && (Date.now() - t0 < 4000 || z.frame < f0 + 8) && Date.now() - t0 < 60000) { await sleep(100); z = await st(); }
    const dist = Math.hypot(a.pos[0] + 13.5, a.pos[2] + 3);
    check('clock: a spell lands on a target 15 m away at large dt', z.impacts === a.impacts + 1 && a.aimObj === 0 && Math.abs(dist - 15) <= 1,
      `impacts ${a.impacts} -> ${z.impacts} after ${z.frame - f0} frames, ${Date.now() - t0} ms; aim=${a.aimObj} dist=${fmt(dist, 2)}`);
    await cmd('reset');
  }

  // 8. No 0.05 s clamp left in the client (static check of the sources).
  {
    const { readdir } = await import('node:fs/promises');
    const dir = join(here, '../../crates/bevy-client/src');
    const src = {};
    for (const f of (await readdir(dir)).filter((f) => f.endsWith('.rs'))) src[f] = await readFile(join(dir, f), 'utf8');
    const clamp = Object.entries(src).filter(([, t]) => /min\(\s*0\.05/.test(t) || /f32::min\([^)]*,\s*0\.05/.test(t)).map(([f]) => f);
    const sites = [['main.rs', 'control'], ['main.rs', 'aim_and_cast'], ['main.rs', 'projectiles'], ['main.rs', 'sync_visuals'], ['fx.rs', 'update_fx']];
    const missing = sites.filter(([f, n]) => { const b = src[f] && fnBody(src[f], n); return !b || !b.includes('clock::frame_dt('); }).map(([, n]) => n);
    const maxDelta = !!src['main.rs'] && src['main.rs'].includes('MAX_FRAME_DT');
    check('clock: no 0.05 s dt clamp left; every per-frame system uses clock::frame_dt',
      clamp.length === 0 && missing.length === 0 && maxDelta && !!src['clock.rs'] && !!src['motion.rs'],
      `clamped in [${clamp}] missing frame_dt in [${missing}] MAX_FRAME_DT=${maxDelta} clock.rs=${!!src['clock.rs']} motion.rs=${!!src['motion.rs']}`);
  }

  // 13 (second half). Every frame's state string parses: at least 200
  // consecutive frames (the sampler has counted since the section began).
  {
    const t0 = Date.now();
    let n = await page.evaluate(() => window.__clk.parsed);
    while (n < 200 && Date.now() - t0 < 240000) { await sleep(500); n = await page.evaluate(() => window.__clk.parsed); }
    const c = await page.evaluate(() => ({ parsed: window.__clk.parsed, err: window.__clk.parseErr, bad: window.__clk.badKeys }));
    check('clock: state JSON keeps the base keys in order, appends gameTime and dt, and parses every frame',
      keysOk && objKeysOk && c.parsed >= 200 && c.err === 0 && c.bad.length === 0,
      `keys ok=${keysOk} (n=${keys.length}, last=${keys.slice(-3)}) objs ok=${objKeysOk} parsed=${c.parsed} errors=${c.err} key changes=${c.bad.length}`);
  }

  // 9. Memory guard: from the game's STATS lines so far (the first 5 lines,
  // about 10 s of startup, are skipped), the live heap grows by at most 8 MB
  // and wasm memory stays at or under 96 MB.
  {
    const rows = logs.filter((l) => /STATS /.test(l)).map((l) => ({
      live: +(/heap_live_mb=([\d.]+)/.exec(l) || [])[1], wasm: +(/wasm_mem_mb=([\d.]+)/.exec(l) || [])[1],
    })).slice(5).filter((r) => Number.isFinite(r.live) && Number.isFinite(r.wasm));
    const first = rows[0], last = rows.at(-1);
    const wasmMax = rows.length ? Math.max(...rows.map((r) => r.wasm)) : NaN;
    check('clock: memory stays flat (heap +8 MB at most, wasm memory at most 96 MB)', rows.length >= 2 && last.live <= first.live + 8 && wasmMax <= 96,
      `${rows.length} STATS lines, heap_live ${first && first.live} -> ${last && last.live} MB, wasm max ${wasmMax} MB`);
  }
  await page.evaluate(() => { window.__clk.on = false; window.__clk.stop = true; });
  await fresh();
}

// ---------------------------------------------------------------------------
// Streams you cannot miss, an honest arrow, one cast per click
// (docs/loop/items/I-025/spec.md, tests 5-11): fireInView in the state, the
// arrow hidden while a fire is in view, outlined streams that stand out
// along their length against the sky and the ground, one cast per slow
// click, no flashing or view-blocking mist, and the hint order. Runs after
// feelAndStandards and before castingYouCanSee; ends fresh, unpaused, with
// Hints and sway restored and the pointer locked if it was on entry.
// ---------------------------------------------------------------------------
async function streamsAndFinder({ page, st, cmd, until, frames, sleep, idx, lit, locked, base, browser, errors }) {
  console.log('  -- streams and finder (I-025) --');
  const VW = +(process.env.VW || 1280), VH = +(process.env.VH || 720);
  void idx; void base; void browser; void errors; void locked;
  const fmt = (v, d = 3) => (Number.isFinite(v) ? v.toFixed(d) : String(v));
  const within = async (f0, n, pred, ms = 30000) => {
    const t0 = Date.now();
    let x = await st();
    while (!pred(x) && x.frame <= f0 + n && Date.now() - t0 < ms) { await sleep(20); x = await st(); }
    return { ok: !!pred(x) && x.frame <= f0 + n, x };
  };
  const unpause = async () => { if ((await st()).paused) { await page.evaluate(() => window.__game.pause(false)); await until((x) => !x.paused, 8000); } };
  const fresh = async () => {
    await unpause();
    await cmd('reset');
    await until((x) => x.outcome === 'playing' && x.burning === 3 && x.casts === 0, 20000);
  };
  const tp = async (x, z) => { await cmd(`tp ${x} ${z}`); await until((q) => Math.abs(q.pos[0] - x) < 0.5 && Math.abs(q.pos[2] - z) < 0.5, 10000); };
  const aimAt = async (i, spell) => {
    await unpause();
    await cmd(`face ${i}`);
    if (spell) await cmd(`select ${spell}`);
    return until((x) => (!spell || x.spell === spell - 1) && x.aimObj === i && x.cooldown === 0, 15000);
  };
  const pauseShown = () => page.evaluate(() => !document.getElementById('pause').classList.contains('hidden'));
  const relock = async () => {
    for (let k = 0; k < 6; k++) {
      if ((await st()).locked) return true;
      if (!(await pauseShown())) await page.evaluate(() => window.__game.pause(true));
      await page.click('#resume');
      if ((await until((q) => q.locked, 4000)).locked) return true;
      await sleep(1200);
    }
    return (await st()).locked;
  };
  const synthClick = () => page.evaluate(() => {
    const c = document.getElementById('game');
    const r = c.getBoundingClientRect();
    const o = { bubbles: true, cancelable: true, button: 0, pointerId: 1, pointerType: 'mouse', isPrimary: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 };
    const s0 = window.__game.state();
    c.dispatchEvent(new PointerEvent('pointerdown', { ...o, buttons: 1 }));
    c.dispatchEvent(new PointerEvent('pointerup', { ...o, buttons: 0 }));
    return s0;
  });
  const hudRead = () => page.evaluate(() => new Promise((res) => {
    const s = window.__game.hudState() || window.__game.state();
    const vis = (e) => !!e && !e.classList.contains('hidden') && e.getClientRects().length > 0 && getComputedStyle(e).display !== 'none' && getComputedStyle(e).visibility !== 'hidden';
    const ff = document.getElementById('firefinder');
    const hint = document.getElementById('hint');
    res({
      s, ffVis: vis(ff), ff: ff ? { vis: vis(ff), right: ff.classList.contains('right'), left: ff.classList.contains('left') } : null,
      hint: vis(hint) ? hint.textContent.trim() : '', hudPaused: !document.getElementById('pause').classList.contains('hidden'),
    });
  }));
  const hudWithin = async (f0, n, pred, ms = 30000) => {
    const t0 = Date.now();
    let h = await hudRead();
    while (!pred(h) && h.s.frame <= f0 + n && Date.now() - t0 < ms) { await sleep(20); h = await hudRead(); }
    return { ok: !!pred(h) && h.s.frame <= f0 + n, h };
  };
  // The finder rule both sides rely on (Contract 5).
  const finderRule = (h) => h.s.outcome === 'playing' && h.s.fireOff && !h.s.fireInView && h.s.nearFire >= 0 && !h.s.paused && !h.hudPaused;
  const lockedOnEntry = (await st()).locked;
  const swayOnEntry = (await st()).sway;
  const lockOk = await relock();

  // 5. fireInView is a boolean, in place.
  {
    await fresh();
    const s0 = await st();
    const keys = Object.keys(s0);
    check('streams: fireInView is a boolean, the last state key',
      // I-027 appends exactly its three progress keys after fireInView (its
      // Contract); fireInView is still the last key before them, after fxWater.
      typeof s0.fireInView === 'boolean' && keys[keys.length - 4] === 'fireInView' && keys[keys.length - 5] === 'fxWater'
        && keys.slice(-3).join() === 'progress,progRev,progSave',
      `fireInView=${s0.fireInView} (${typeof s0.fireInView}) last keys ${keys.slice(-5).join(',')}`);
  }

  // 6. The arrow stays away while a fire is aimed at or on screen.
  {
    // (a) Aimed at a fire while the nearest fire is off screen.
    await fresh();
    await tp(0, 5.5);
    let pick = null; let h = null; const tried = [];
    for (const t of [lit[0], lit[1], lit[2]]) {
      await cmd(`face ${t}`);
      const a = await until((x) => x.aimObj === t, 10000);
      if (a.aimObj !== t) { tried.push(`${t}: not aimed`); continue; }
      const r = await hudWithin(a.frame, 5, (q) => q.s.aimObj === t && q.s.fireOff === true && q.s.nearFire !== t);
      tried.push(`${t}: fireOff=${r.h.s.fireOff} nearFire=${r.h.s.nearFire}`);
      if (r.ok) { pick = t; h = r.h; break; }
    }
    console.log(`  6a: target ${pick} (tried ${tried.join('; ')})`);
    let ok = false;
    if (pick !== null) {
      const r = await hudWithin(h.s.frame, 5, (q) => q.s.aimObj === pick && q.s.fireOff === true && q.s.nearFire !== pick && q.s.fireInView === true && !q.ffVis);
      ok = r.ok; h = r.h;
    }
    check('streams: no arrow while aiming at a fire (the nearest fire off screen)', ok,
      pick === null ? `no fire met the precondition: ${tried.join('; ')}` : `target ${pick} aimObj=${h.s.aimObj} nearFire=${h.s.nearFire} fireOff=${h.s.fireOff} fireInView=${h.s.fireInView} finder=${h.ffVis}`);

    // (b) and (c) A sweep at pitch 0.
    await tp(0, 5.5);
    let ruleOk = true; let aimOk = true; let suppressed = 0; let shown = 0; const bad = [];
    for (let yaw = 0; yaw < 360; yaw += 15) {
      await cmd(`look ${yaw} 0`);
      await frames(2);
      const q = await hudRead();
      const want = finderRule(q);
      if (q.ffVis !== want) { ruleOk = false; bad.push(`${yaw}: finder ${q.ffVis} rule ${want}`); }
      if (q.s.aimObj >= 0 && q.s.objs[q.s.aimObj] && q.s.objs[q.s.aimObj].burning && q.s.fireInView !== true) { aimOk = false; bad.push(`${yaw}: aimed fire ${q.s.aimObj} fireInView=${q.s.fireInView}`); }
      if (q.s.fireOff && q.s.fireInView) suppressed++;
      if (q.ffVis) shown++;
    }
    check('streams: the arrow shows exactly when no fire is in view (sweep)', ruleOk && aimOk,
      `${bad.slice(0, 4).join('; ') || 'all 24 yaws agree'}`);
    check('streams: the sweep both hides the arrow for a fire in view and shows it', suppressed >= 1 && shown >= 1,
      `fireOff&&fireInView at ${suppressed} yaws, finder shown at ${shown} yaws`);

    // (d) Occlusion, recorded only (follow-up: occlusion-aware finder).
    await tp(0, -30);
    await cmd(`face ${lit[2]}`);
    await frames(2);
    const q = await hudRead();
    console.log(`  6d occlusion (known gap): aimObj=${q.s.aimObj} nearFire=${q.s.nearFire} fireOff=${q.s.fireOff} fireInView=${q.s.fireInView} finder=${q.ffVis}`);
    check('streams: behind the greenhouse the finder follows the rule (occlusion recorded)', q.ffVis === finderRule(q),
      `aimObj=${q.s.aimObj} nearFire=${q.s.nearFire} fireOff=${q.s.fireOff} fireInView=${q.s.fireInView} finder=${q.ffVis}`);
  }

  // Screenshots for tests 7, 8 and 10: the HUD and the pause card hidden
  // through the CSSOM, and restored after.
  const hideUi = (on) => page.evaluate((on) => {
    for (const id of ['hud', 'pause']) { const e = document.getElementById(id); if (e) e.style.visibility = on ? 'hidden' : ''; }
  }, on);
  const shot = async () => decodePng(await page.screenshot({ type: 'png', timeout: 180000 }));
  const lum = (d, k) => 0.2126 * LIN[d[k]] + 0.7152 * LIN[d[k + 1]] + 0.0722 * LIN[d[k + 2]];
  // A pixel differs when its contrast ratio is >= 1.5 or its sRGB distance >= 90.
  const differs = (a, b, k) => {
    const la = lum(a.data, k), lb = lum(b.data, k);
    const cr = (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
    if (cr >= 1.5) return true;
    const dr = a.data[k] - b.data[k], dg = a.data[k + 1] - b.data[k + 1], db = a.data[k + 2] - b.data[k + 2];
    return Math.hypot(dr, dg, db) >= 90;
  };
  // Pause on the first frame where the spell has landed and a stream is
  // drawn (full length, frozen), after one synthetic click.
  const castAndFreeze = () => page.evaluate(() => new Promise((res) => {
    const c = document.getElementById('game');
    const r = c.getBoundingClientRect();
    const o = { bubbles: true, cancelable: true, button: 0, pointerId: 1, pointerType: 'mouse', isPrimary: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 };
    const s0 = window.__game.state();
    c.dispatchEvent(new PointerEvent('pointerdown', { ...o, buttons: 1 }));
    c.dispatchEvent(new PointerEvent('pointerup', { ...o, buttons: 0 }));
    const step = () => {
      const s = window.__game.state();
      if (s.impacts > s0.impacts && s.streams >= 1) { window.__game.pause(true); res(s.frame - s0.frame); return; }
      if (s.frame - s0.frame > 60) { res(-1); return; }
      requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }));
  // Cross-sections of the stream along C -> Hs at r = 20, 30, ... 110 px.
  const sections = (after, b1, b2, fov) => {
    const W = after.w, H = after.h;
    const f = (H / 2) / Math.tan((fov * DEG) / 2);
    const cx = W / 2, cy = H / 2;
    const hx = W / 2 + 0.4 * f, hy = H / 2 + 0.3 * f;
    const len = Math.hypot(hx - cx, hy - cy);
    const ux = (hx - cx) / len, uy = (hy - cy) / len; const px = -uy, py = ux;
    const out = [];
    for (let r = 20; r <= 110; r += 10) {
      const half = Math.round(0.4 * r + 24);
      let best = 0; let run = 0; let gap = 0; let noisy = 0; let total = 0;
      for (let t = -half; t <= half; t++) {
        const x = Math.round(cx + ux * r + px * t), y = Math.round(cy + uy * r + py * t);
        if (x < 0 || y < 0 || x >= W || y >= H) { run = 0; gap = 0; continue; }
        const k = (y * W + x) * after.bpp;
        total++;
        if (differs(b1, b2, k)) { noisy++; continue; }
        if (differs(after, b1, k)) { run += 1; gap = 0; best = Math.max(best, run); } else if (++gap > 2) { run = 0; gap = 0; }
      }
      out.push({ r, run: best, need: 0.6 * r, pass: best >= 0.6 * r, noisy: total ? noisy / total : 1 });
    }
    return out;
  };
  // One pixel attempt for a spell at the current pose: two before shots
  // (paused), a cast frozen at full length, and the after shot.
  const pixelTry = async (spell, pose) => {
    await fresh();
    await pose(spell);
    await page.evaluate(() => window.__game.pause(true));
    await until((x) => x.paused, 5000);
    await hideUi(true);
    await frames(1);
    const b1 = await shot(); const b2 = await shot();
    await hideUi(false);
    await unpause();
    await until((x) => x.cooldown === 0, 5000);
    await hideUi(true);
    const caught = await castAndFreeze();
    await until((x) => x.paused, 5000);
    await frames(1);
    const after = await shot();
    const sx = await st();
    await hideUi(false);
    await unpause();
    return { caught, streams: sx.streams, secs: sections(after, b1, b2, sx.fov) };
  };
  const SPELLS = [[2, 'Water'], [3, 'Frost'], [4, 'CO2']];

  // 7. Every stream stands out along its length, against the sky.
  {
    await cmd('set sway off');
    const sky = async (spell) => { await tp(0, 5.5); await cmd(`select ${spell}`); await cmd('look 0 60'); await until((x) => x.pitch > 55 && x.spell === spell - 1, 5000); };
    for (const [spell, name] of SPELLS) {
      let pass = false; const log = [];
      for (let k = 0; k < 3 && !pass; k++) {
        const t = await pixelTry(spell, sky);
        const n = t.secs.filter((c) => c.pass).length;
        pass = lockOk && t.caught >= 0 && t.streams >= 1 && n >= 8;
        log.push(`try ${k + 1}: ${n}/10 caught@${t.caught} streams=${t.streams} runs ${t.secs.map((c) => `${c.r}:${c.run}`).join(' ')}`);
      }
      console.log(`  sky ${name}: ${log.join(' | ')}`);
      check(`streams: the ${name} stream stands out against the sky along its length`, pass, log.join(' | '));
    }
  }

  // 8. ...and against the ground (I-019 test 11's poses: a fire 18 m away).
  {
    const poses = [[-4, 14, lit[0], [-13.5, -3]], [4, 14, lit[1], [13.5, -3]], [-6, 16, lit[0], [-13.5, -3]], [6, 16, lit[1], [13.5, -3]]];
    let chosen = null;
    const ground = async (spell) => {
      for (const p of (chosen ? [chosen] : poses)) {
        await tp(p[0], p[1]);
        const a = await aimAt(p[2], spell);
        const dist = Math.hypot(a.pos[0] - p[3][0], a.pos[2] - p[3][1]);
        if (a.aimObj === p[2] && a.objs[p[2]].burning && dist >= 18) { chosen = p; return; }
      }
    };
    for (const [spell, name] of SPELLS) {
      let pass = false; const log = [];
      for (let k = 0; k < 3 && !pass; k++) {
        const t = await pixelTry(spell, ground);
        const kept = t.secs.filter((c) => c.noisy <= 0.2);
        const n = kept.filter((c) => c.pass).length;
        pass = lockOk && !!chosen && t.caught >= 0 && t.streams >= 1 && kept.length >= 6 && n >= 0.8 * kept.length;
        log.push(`try ${k + 1}: pose ${chosen ? chosen.slice(0, 2).join(',') : 'none'} ${n}/${kept.length} kept (${10 - kept.length} skipped) caught@${t.caught} streams=${t.streams} runs ${t.secs.map((c) => `${c.r}:${c.run}${c.noisy > 0.2 ? 's' : ''}`).join(' ')}`);
      }
      console.log(`  ground ${name}: ${log.join(' | ')}`);
      check(`streams: the ${name} stream stands out against the ground`, pass, log.join(' | '));
    }
    await cmd(`set sway ${swayOnEntry ? 'on' : 'off'}`);
  }

  // 9. A slow click casts once; a hold still repeats.
  {
    // Records [gameTime, dt, casts, button down] on every new frame until stopped.
    const startRec = () => page.evaluate(() => {
      const rec = { rows: [], down: false, stop: false };
      window.__i025rec = rec;
      const dn = (e) => { if (e.button === 0) rec.down = true; };
      const up = (e) => { if (e.button === 0) rec.down = false; };
      document.addEventListener('pointerdown', dn, true);
      document.addEventListener('pointerup', up, true);
      let last = -1;
      const step = () => {
        const s = window.__game.state();
        if (s.frame !== last) { last = s.frame; rec.rows.push([s.gameTime, s.dt, s.casts, rec.down]); }
        if (rec.stop) { document.removeEventListener('pointerdown', dn, true); document.removeEventListener('pointerup', up, true); return; }
        requestAnimationFrame(step);
      };
      requestAnimationFrame(step);
    });
    const stopRec = () => page.evaluate(() => { window.__i025rec.stop = true; return window.__i025rec.rows; });
    await fresh();
    await relock();
    let slow = null; const spans = [];
    for (let k = 0; k < 3 && !slow; k++) {
      await cmd('look 0 60');
      await until((x) => x.pitch > 55 && x.cooldown === 0, 5000);
      await startRec();
      const g0 = (await st()).gameTime;
      await page.mouse.down();
      const t0 = Date.now();
      let z = await st();
      while (z.gameTime < g0 + 0.35 && Date.now() - t0 < 15000) { await sleep(20); z = await st(); }
      await page.mouse.up();
      const f1 = (await st()).frame;
      await until((x) => x.frame >= f1 + 20, 60000);
      const rows = await stopRec();
      const dn = rows.filter((r) => r[3]);
      const span = dn.length ? dn[dn.length - 1][0] - dn[0][0] : 0;
      const maxDt = dn.reduce((m, r) => Math.max(m, r[1]), 0);
      const c0 = rows.length ? rows[0][2] : 0; const c1 = rows.length ? rows[rows.length - 1][2] : 0;
      spans.push(`span ${fmt(span)} max_dt ${fmt(maxDt)} casts +${c1 - c0}`);
      if (dn.length && span < 0.5 - maxDt) slow = { grew: c1 - c0 };
      await until((x) => x.cooldown === 0, 5000);
    }
    check('streams: a slow click (0.35 s) casts once', lockOk && !!slow && slow.grew === 1, spans.join(' | '));

    await cmd('look 0 60');
    await until((x) => x.pitch > 55 && x.cooldown === 0, 5000);
    await startRec();
    const g0 = (await st()).gameTime;
    await page.mouse.down();
    const t0 = Date.now();
    let z = await st();
    while (z.gameTime < g0 + 1.6 && Date.now() - t0 < 30000) { await sleep(50); z = await st(); }
    await page.mouse.up();
    await frames(2);
    const rows = await stopRec();
    const firsts = [];
    for (let i = 1; i < rows.length; i++) if (rows[i][2] > rows[i - 1][2]) firsts.push(i);
    const grew = rows.length ? rows[rows.length - 1][2] - rows[0][2] : 0;
    let gapOk = false; let detail = '';
    if (firsts.length >= 2) {
      const [i1, i2] = firsts;
      const d = rows.slice(i1, i2 + 1).reduce((m, r) => Math.max(m, r[1]), 0);
      const gap = rows[i2][0] - rows[i1][0];
      gapOk = gap >= 0.5 - d;
      detail = `first repeat ${fmt(gap)} s after the click (need >= ${fmt(0.5 - d)})`;
    }
    check('streams: holding repeats, first after half a second', lockOk && grew >= 3 && gapOk, `casts +${grew}; ${detail}`);
  }

  // 10. No flashing and no view-blocking mist (GS-6, GS-17). A guard.
  {
    const near = async (spell) => {
      await tp(0, 0);
      const a = await aimAt(lit[2], spell);
      if (a.aimObj === lit[2]) return { pose: 'tp 0 0 -> lit[2]', ok: true };
      for (const [x, zz, t] of [[0, -1, lit[2]], [1, -1, lit[2]], [-1, -1, lit[2]]]) {
        await tp(x, zz);
        const b = await aimAt(t, spell);
        const c = b.objs[t];
        if (b.aimObj === t && c && c.burning) return { pose: `tp ${x} ${zz} -> ${t}`, ok: true };
      }
      return { pose: 'none', ok: false };
    };
    const far = async (spell) => { await tp(0, 5.5); const a = await aimAt(lit[0], spell); return { pose: 'tp 0 5.5 -> lit[0]', ok: a.aimObj === lit[0] }; };
    const run = async (spell, poseFn) => {
      await fresh();
      const p = await poseFn(spell);
      await page.evaluate(() => window.__game.pause(true));
      await until((x) => x.paused, 5000);
      await hideUi(true);
      await frames(1);
      const before = await shot();
      await hideUi(false);
      await unpause();
      await until((x) => x.cooldown === 0, 5000);
      await hideUi(true);
      const imgs = [await shot()];
      await cmd('fire');
      for (let k = 0; k < 19; k++) { await frames(1); imgs.push(await shot()); }
      await hideUi(false);
      const L = imgs.map(lumaOf);
      let worst = 0; let mist = 0;
      for (let k = 1; k < L.length; k++) worst = Math.max(worst, flashFrac(L[k - 1], L[k]));
      for (const im of imgs) {
        let n = 0;
        for (let i = 0, k = 0; i < im.w * im.h; i++, k += im.bpp) {
          if (im.data[k] >= 215 && im.data[k + 1] >= 215 && im.data[k + 2] >= 215
            && !(before.data[k] >= 215 && before.data[k + 1] >= 215 && before.data[k + 2] >= 215)) n++;
        }
        mist = Math.max(mist, n / (im.w * im.h));
      }
      return { pose: p.pose, poseOk: p.ok, worst, mist };
    };
    const res = [];
    for (const [spell, name] of [[3, 'Frost'], [4, 'CO2']]) {
      for (const [where, fn] of [['far', far], ['near', near]]) {
        const r = await run(spell, fn);
        res.push({ name, where, ...r });
      }
    }
    const txt = res.map((r) => `${r.name} ${r.where} (${r.pose}): flash ${(r.worst * 100).toFixed(1)}% mist ${(r.mist * 100).toFixed(1)}%`).join(' | ');
    console.log(`  mist guard: ${txt}`);
    check('streams: Frost and CO2 streams do not flash the screen', res.every((r) => r.poseOk && r.worst < 0.2), txt);
    check('streams: Frost and CO2 mist covers under 15% of the view', res.every((r) => r.poseOk && r.mist < 0.15), txt);
  }

  // 11. The hint order is pinned (banner > idle > note > locked > start).
  {
    const table = await page.evaluate(() => {
      const p = window.__game.pickHint;
      if (typeof p !== 'function') return null;
      const d = { playing: true, idle: false, note: '', locked: false, extinguished: 0, casts: 0 };
      const at = (o) => p({ ...d, ...o });
      return {
        banner: [at({ playing: false }), at({ playing: false, idle: true, note: 'N', locked: true })],
        idle: at({ idle: true, note: 'N' }),
        note: at({ idle: false, note: 'N', locked: false }),
        aim: at({ locked: true, extinguished: 0, note: '' }),
        aimDone: at({ locked: true, extinguished: 1 }),
        start: at({ locked: false, casts: 0, note: '' }),
        started: at({ locked: false, casts: 2, note: '' }),
      };
    });
    const ok = !!table && table.banner.every((v) => v === '') && /^Look for a fire\./.test(table.idle) && table.note === 'N'
      && table.aim === 'Aim at a fire and click.' && table.aimDone === '' && /^Click to look around\./.test(table.start) && table.started === '';
    check('streams: pickHint follows banner > idle > note > locked > start', ok, JSON.stringify(table));
    // Live: point mode's note shows right after a reset.
    await fresh();
    if ((await st()).locked) {
      await page.evaluate(() => window.__game.pause(true));
      await page.evaluate(() => document.exitPointerLock());
      await until((q) => !q.locked, 8000);
    }
    if (!(await pauseShown())) await page.evaluate(() => window.__game.pause(true));
    await page.click('#pointmode');
    const f0 = (await st()).frame;
    const r = await hudWithin(f0, 2, (h) => /^Point and click:/.test(h.hint));
    check('streams: the point-and-click note shows within 2 frames', r.ok, `hint "${r.h.hint}"`);
  }

  await fresh();
  if (lockedOnEntry) await relock();
  await cmd(`set sway ${swayOnEntry ? 'on' : 'off'}`);
}

// ---------------------------------------------------------------------------
// Casting you can see (docs/loop/items/I-019/spec.md, tests 7-19): Water by
// default, a click that always casts once, spells that land within 0.25 s of
// game time, a visible stream and splash, one fire counter, a crosshair that
// tells a fire from a bush, an arrow to the nearest fire, Esc and P pause
// under automation, nothing under the banners, and a flashing guard. Helpers
// are local copies of the feelAndStandards ones.
// ---------------------------------------------------------------------------
async function castingYouCanSee({ page, st, cmd, until, frames, sleep, idx, lit, locked, base, browser, errors }) {
  console.log('  -- casting you can see (I-019) --');
  const VW = +(process.env.VW || 1280), VH = +(process.env.VH || 720);
  void base; void browser; void errors; void locked;
  const fmt = (v, d = 3) => (Number.isFinite(v) ? v.toFixed(d) : String(v));
  const within = async (f0, n, pred, ms = 30000) => {
    const t0 = Date.now();
    let x = await st();
    while (!pred(x) && x.frame <= f0 + n && Date.now() - t0 < ms) { await sleep(20); x = await st(); }
    return { ok: !!pred(x) && x.frame <= f0 + n, x };
  };
  const unpause = async () => { if ((await st()).paused) { await page.evaluate(() => window.__game.pause(false)); await until((x) => !x.paused, 8000); } };
  const fresh = async () => {
    await unpause();
    await cmd('reset');
    await until((x) => x.outcome === 'playing' && x.burning === 3 && x.casts === 0, 20000);
  };
  const tp = async (x, z) => { await cmd(`tp ${x} ${z}`); await until((q) => Math.abs(q.pos[0] - x) < 0.5 && Math.abs(q.pos[2] - z) < 0.5, 10000); };
  const aimAt = async (i, spell) => {
    await unpause();
    await cmd(`face ${i}`);
    if (spell) await cmd(`select ${spell}`);
    return until((x) => (!spell || x.spell === spell - 1) && x.aimObj === i && x.cooldown === 0, 15000);
  };
  const castAt = async (i, spell) => {
    await unpause();
    await cmd(`face ${i}`);
    await cmd(`select ${spell}`);
    const a = await until((x) => x.spell === spell - 1 && x.aimObj === i, 15000);
    await cmd('cast');
    await until((x) => x.impacts > a.impacts, 60000);
    return { aim: a.aimObj, s: await st() };
  };
  const pauseShown = () => page.evaluate(() => !document.getElementById('pause').classList.contains('hidden'));
  // Takes the pointer lock through the pause card's button (browsers refuse
  // a new lock for about a second after one is released, so it retries).
  const relock = async () => {
    for (let k = 0; k < 6; k++) {
      if ((await st()).locked) return true;
      if (!(await pauseShown())) await page.evaluate(() => window.__game.pause(true));
      await page.click('#resume');
      if ((await until((q) => q.locked, 4000)).locked) return true;
      await sleep(1200);
    }
    return (await st()).locked;
  };
  // One left click on the canvas from synthetic events, both in the same
  // task (before any game frame); returns the state just before it.
  const synthClick = () => page.evaluate(() => {
    const c = document.getElementById('game');
    const r = c.getBoundingClientRect();
    const o = { bubbles: true, cancelable: true, button: 0, pointerId: 1, pointerType: 'mouse', isPrimary: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 };
    const s0 = window.__game.state();
    c.dispatchEvent(new PointerEvent('pointerdown', { ...o, buttons: 1 }));
    c.dispatchEvent(new PointerEvent('pointerup', { ...o, buttons: 0 }));
    return s0;
  });
  // The HUD and the state it last drew (__game.hudState), in one read. (The
  // game and the HUD each run in their own animation-frame callback, so the
  // newest state can be a frame ahead of the DOM.)
  const hudRead = () => page.evaluate(() => new Promise((res) => {
    const read = () => {
      const s = window.__game.hudState() || window.__game.state();
      const hud = document.getElementById('hud');
      const vis = (e) => !!e && !e.classList.contains('hidden') && e.getClientRects().length > 0 && getComputedStyle(e).display !== 'none' && getComputedStyle(e).visibility !== 'hidden';
      const ol = document.getElementById('obj-line');
      const ff = document.getElementById('firefinder');
      const hint = document.getElementById('hint');
      const r = ff ? ff.getBoundingClientRect() : null;
      res({
        s, synced: +hud.dataset.frame === s.frame,
        text: ol.textContent.replace(/\s+/g, ' ').trim(), pips: ol.querySelectorAll('.fire-pip').length,
        ff: ff ? { vis: vis(ff), right: ff.classList.contains('right'), left: ff.classList.contains('left'), w: r.width } : null,
        hint: vis(hint) ? hint.textContent.trim() : '',
        cross: [...document.getElementById('crosshair').classList], canvas: [...document.getElementById('game').classList],
        nudge: !!document.querySelector('.slot.nudge'), banner: vis(document.getElementById('banner')),
      });
    };
    read();
  }));
  // Polls hudRead until pred holds or more than n frames have passed since f0.
  const hudWithin = async (f0, n, pred, ms = 30000) => {
    const t0 = Date.now();
    let h = await hudRead();
    while (!pred(h) && h.s.frame <= f0 + n && Date.now() - t0 < ms) { await sleep(20); h = await hudRead(); }
    return { ok: !!pred(h) && h.s.frame <= f0 + n, h };
  };
  let s;

  // 7. Water by default (after a reload).
  {
    await page.reload();
    for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState));) {
      if (Date.now() - t0 > 120000) throw new Error('game did not restart within 120 s');
      await sleep(50);
    }
    const first = await st();
    await frames(2);
    const slot = await page.evaluate(() => !!document.querySelector('.slot[data-spell="2"].sel'));
    check('cast: Water is the spell selected on boot', first.spell === 1 && slot, `first state spell=${first.spell} water slot sel=${slot}`);
  }
  const lockOk = await relock();
  console.log(`  pointer lock for I-019: ${lockOk}`);

  // 8. A click casts once and puts a fire out (latched edge).
  {
    await fresh();
    await tp(0, 5.5);
    await aimAt(lit[0], 2);
    const a = await synthClick();
    const r = await within(a.frame, 12, (x) => x.casts === a.casts + 1 && x.impacts >= a.impacts + 1 && x.objs[lit[0]].burning === false
      && x.extinguished === a.extinguished + 1 && x.hitKind === 'out' && x.firesLeft === a.firesLeft - 1 && x.clicks === a.clicks + 1);
    check('cast: one synthetic click casts once and puts the fire out', lockOk && a.locked && r.ok,
      `locked=${a.locked} clicks ${a.clicks} -> ${r.x.clicks}, casts ${a.casts} -> ${r.x.casts}, impacts ${a.impacts} -> ${r.x.impacts}, burning=${r.x.objs[lit[0]].burning}, ext ${a.extinguished} -> ${r.x.extinguished}, hitKind=${r.x.hitKind}, firesLeft ${a.firesLeft} -> ${r.x.firesLeft}, frames ${r.x.frame - a.frame}`);
  }

  // 9. A real click casts exactly once; a held button still repeats.
  {
    // The mouse goes to the click spot first, so the click itself carries no
    // movement that could turn the view off the fire. Pointer moves reach
    // the page on the next animation frame, so the aim is taken after it
    // and checked again a frame later.
    await page.mouse.move(VW / 2, VH / 2);
    await frames(2);
    await aimAt(lit[1], 2);
    await frames(1);
    const a = await aimAt(lit[1], 2);
    await page.mouse.click(VW / 2, VH / 2);
    const r = await within(a.frame, 20, (x) => x.casts === a.casts + 1 && x.objs[lit[1]].burning === false);
    await frames(20);
    const later = await st();
    check('cast: a real click casts exactly once', lockOk && a.locked && r.ok && later.casts === a.casts + 1,
      `casts ${a.casts} -> ${r.x.casts} -> ${later.casts} (20 frames on), lit[1] burning=${r.x.objs[lit[1]].burning}`);
    await cmd('look 0 60');
    const h0 = await until((x) => x.pitch > 55 && x.cooldown === 0, 5000);
    await page.mouse.down();
    const t0 = Date.now();
    let z = await st();
    while (z.gameTime - h0.gameTime < 1.5 && Date.now() - t0 < 15000) { await sleep(50); z = await st(); }
    await page.mouse.up();
    check('cast: holding the button keeps casting', lockOk && z.casts - h0.casts >= 2,
      `casts +${z.casts - h0.casts} over ${fmt(z.gameTime - h0.gameTime)} game s (${Date.now() - t0} ms wall)`);
  }

  // 10. Latency (GS-12): an in-page click while aimed at a fire with no cooldown.
  {
    await fresh();
    await tp(0, 5.5);
    await aimAt(lit[0], 2);
    const lat = await page.evaluate(() => new Promise((res) => {
      const c = document.getElementById('game');
      let s0 = null; let castF = null; let streamF = null;
      const step = () => {
        const s = window.__game.state();
        if (!s0) {
          s0 = s;
          const r = c.getBoundingClientRect();
          const o = { bubbles: true, cancelable: true, button: 0, pointerId: 1, pointerType: 'mouse', isPrimary: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 };
          c.dispatchEvent(new PointerEvent('pointerdown', { ...o, buttons: 1 }));
          c.dispatchEvent(new PointerEvent('pointerup', { ...o, buttons: 0 }));
        } else {
          if (castF === null && s.casts > s0.casts) castF = s.frame - s0.frame;
          if (streamF === null && s.streams >= 1) streamF = s.frame - s0.frame;
          if ((castF !== null && streamF !== null) || s.frame - s0.frame > 30) { res({ castF, streamF, aim: s0.aimObj, cd: s0.cooldown, locked: s0.locked }); return; }
        }
        requestAnimationFrame(step);
      };
      requestAnimationFrame(step);
    }));
    check('cast: a click casts and shows a stream within 2 frames', lockOk && lat.locked && lat.castF !== null && lat.castF <= 2 && lat.streamF !== null && lat.streamF <= 2,
      `cast after ${lat.castF} frames, stream after ${lat.streamF} frames (aim ${lat.aim}, cooldown ${lat.cd})`);
  }

  // 11. Spells land within a quarter second of game time, from 18 m or more.
  {
    // tp 0 -20 is inside the greenhouse (x -8..8, z -22..-12), so the pose is
    // picked from these: the first where the fire is aimed at directly.
    const poses = [[-4, 14, lit[0], [-13.5, -3]], [4, 14, lit[1], [13.5, -3]], [-6, 16, lit[0], [-13.5, -3]], [6, 16, lit[1], [13.5, -3]]];
    const flight = async (spell, label) => {
      await fresh();
      let pose = null; let a = null;
      for (const p of poses) {
        await tp(p[0], p[1]);
        a = await aimAt(p[2], spell);
        const dist = Math.hypot(a.pos[0] - p[3][0], a.pos[2] - p[3][1]);
        if (a.aimObj === p[2] && dist >= 18) { pose = { p, dist }; break; }
      }
      if (!pose) return { ok: false, detail: `no pose aims at a fire 18 m away (last aim ${a && a.aimObj})` };
      const rec = await page.evaluate((useClick) => new Promise((res) => {
        const rows = []; let last = -1; const t0 = performance.now();
        const s0 = window.__game.state();
        if (useClick) {
          const c = document.getElementById('game');
          const r = c.getBoundingClientRect();
          const o = { bubbles: true, cancelable: true, button: 0, pointerId: 1, pointerType: 'mouse', isPrimary: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 };
          c.dispatchEvent(new PointerEvent('pointerdown', { ...o, buttons: 1 }));
          c.dispatchEvent(new PointerEvent('pointerup', { ...o, buttons: 0 }));
        } else window.__game.cmd('fire');
        let after = -1;
        const step = () => {
          const s = window.__game.state();
          if (s.frame !== last) {
            last = s.frame;
            rows.push([s.frame, s.gameTime, s.dt, s.casts, s.impacts]);
            if (after < 0 && s.impacts > s0.impacts) after = rows.length;
          }
          const n = s.frame - s0.frame;
          if ((after > 0 && rows.length >= after + 2) || (n > 60 && performance.now() - t0 > 15000)) { res({ s0, rows }); return; }
          requestAnimationFrame(step);
        };
        requestAnimationFrame(step);
      }), lockOk);
      const { s0, rows } = rec;
      const ic = rows.findIndex((r) => r[3] > s0.casts);
      const ii = rows.findIndex((r) => r[4] > s0.impacts);
      if (ic < 0 || ii < 0) return { ok: false, detail: `${label}: cast row ${ic}, impact row ${ii} in ${rows.length} frames (pose ${pose.p.slice(0, 2)}, ${fmt(pose.dist, 1)} m)` };
      const gc = rows[ic][1], gi = rows[ii][1];
      const d = Math.max(...rows.slice(ic, ii + 1).map((r) => r[2]));
      // (The sampler stops after 60 frames or 15 s, whichever is later, so an
      // impact row here is within that limit.)
      const ok = gi - gc <= 0.25 + d;
      return { ok, detail: `${label}: pose (${pose.p[0]}, ${pose.p[1]}) dist ${fmt(pose.dist, 2)} m, g_i - g_c = ${fmt(gi - gc)} s, d = ${fmt(d, 4)}, ${rows[ii][0] - rows[ic][0]} frames cast->impact` };
    };
    const w = await flight(2, 'Water');
    const c = await flight(4, 'CO2');
    console.log(`  ${w.detail}\n  ${c.detail}`);
    check('cast: Water and CO2 land within 0.25 s of game time from 18 m', w.ok && c.ok, `${w.detail}; ${c.detail}`);
  }

  // 12. The stream and the splash are visible.
  {
    await fresh();
    await tp(0, 5.5);
    await aimAt(lit[0], 2);
    const rec = await page.evaluate(() => new Promise((res) => {
      const c = document.getElementById('game');
      const r = c.getBoundingClientRect();
      const o = { bubbles: true, cancelable: true, button: 0, pointerId: 1, pointerType: 'mouse', isPrimary: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 };
      const s0 = window.__game.state();
      c.dispatchEvent(new PointerEvent('pointerdown', { ...o, buttons: 1 }));
      c.dispatchEvent(new PointerEvent('pointerup', { ...o, buttons: 0 }));
      const rows = []; let last = -1;
      const step = () => {
        const s = window.__game.state();
        if (s.frame !== last) { last = s.frame; rows.push([s.frame, s.casts, s.streams, s.impacts, s.fxWater]); }
        if (rows.length >= 40 || (s.frame - s0.frame > 8 && s.impacts > s0.impacts && rows.at(-1)[2] === 0)) { res({ s0, rows }); return; }
        requestAnimationFrame(step);
      };
      requestAnimationFrame(step);
    }));
    const { s0, rows } = rec;
    const ic = rows.findIndex((x) => x[1] > s0.casts);
    let run = 0; let best = 0; let start = -1; let bestStart = -1;
    rows.forEach((x, k) => { if (x[2] >= 1) { if (run === 0) start = k; run++; if (run > best) { best = run; bestStart = start; } } else run = 0; });
    const streamOk = ic >= 0 && best >= 3 && bestStart >= 0 && rows[bestStart][0] - rows[ic][0] <= 2;
    const ii = rows.findIndex((x) => x[3] > s0.impacts);
    const splash = ii >= 0 ? Math.max(rows[ii][4] ?? 0, rows[ii + 1] ? rows[ii + 1][4] ?? 0 : 0) : -1;
    check('cast: a Water stream shows for 3 frames or more from the cast', lockOk && streamOk,
      `cast row ${ic}, longest stream run ${best} frames from row ${bestStart}; rows ${JSON.stringify(rows.slice(0, 12))}`);
    check('cast: the Water splash has 20 particles or more', ii >= 0 && splash >= 20, `fxWater at impact ${splash}`);

    // Pixel check: water-blue pixels in the box x 45-80 %, y 45-90 %, with
    // the game paused on the first frame that has a stream.
    const blue = (img) => {
      let n = 0;
      const x0 = Math.floor(img.w * 0.45), x1 = Math.floor(img.w * 0.8), y0 = Math.floor(img.h * 0.45), y1 = Math.floor(img.h * 0.9);
      for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
        const k = (y * img.w + x) * img.bpp; const R = img.data[k], B = img.data[k + 2];
        if (B > R + 40 && B > 120) n++;
      }
      return n;
    };
    const shot = async () => decodePng(await page.screenshot({ type: 'png', timeout: 180000 }));
    const tries = [];
    let pass = false;
    for (let k = 0; k < 3 && !pass; k++) {
      await fresh();
      await tp(0, 5.5);
      await aimAt(lit[0], 2);
      const b1 = blue(await shot());
      const b2 = blue(await shot());
      const noise = Math.abs(b1 - b2);
      await aimAt(lit[0], 2);
      const caught = await page.evaluate(() => new Promise((res) => {
        const c = document.getElementById('game');
        const r = c.getBoundingClientRect();
        const o = { bubbles: true, cancelable: true, button: 0, pointerId: 1, pointerType: 'mouse', isPrimary: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 };
        const s0 = window.__game.state();
        c.dispatchEvent(new PointerEvent('pointerdown', { ...o, buttons: 1 }));
        c.dispatchEvent(new PointerEvent('pointerup', { ...o, buttons: 0 }));
        const step = () => {
          const s = window.__game.state();
          if (s.streams >= 1) {
            window.__game.pause(true);
            // The pause card would cover the stream: hidden for the
            // screenshot only (the game stays paused).
            document.getElementById('pause').classList.add('hidden');
            res(s.frame - s0.frame);
            return;
          }
          if (s.frame - s0.frame > 30) { res(-1); return; }
          requestAnimationFrame(step);
        };
        requestAnimationFrame(step);
      }));
      await until((x) => x.paused, 5000);
      await frames(1);
      const after = blue(await shot());
      const sx = await st();
      await page.evaluate(() => window.__game.pause(false));
      await until((x) => !x.paused, 8000);
      const need = Math.max(b1, b2) + noise + 0.003 * VW * VH;
      pass = caught >= 0 && sx.streams >= 1 && after >= need;
      tries.push(`before ${b1}/${b2} noise ${noise} after ${after} need ${Math.round(need)} caught@${caught} streams=${sx.streams}`);
    }
    console.log(`  stream pixels: ${tries.join(' | ')}`);
    check('cast: the Water stream shows on screen (blue pixels)', lockOk && pass, tries.join(' | '));
  }

  // 14. One fire counter.
  {
    await fresh();
    let h = await hudRead();
    const m = /^Fires to put out: (\d+)$/.exec(h.text);
    check('cast: the objective shows one number of fires to put out', !!m && +m[1] === h.s.firesLeft && h.s.firesLeft === 3 && h.pips === 3
      && !/burning now|\//.test(h.text), `text "${h.text}" firesLeft=${h.s.firesLeft} pips=${h.pips}`);
    const t0 = h.s.tick;
    await cmd('tick 2400');
    const sp = await until((x) => x.tick >= t0 + 2400 && x.burning > 3, 20000);
    const num = (t) => { const q = /^Fires to put out: (\d+)$/.exec(t); return q ? +q[1] : NaN; };
    let r = await hudWithin(sp.frame, 2, (q) => q.s.firesLeft === Math.max(q.s.goal - q.s.extinguished, q.s.burning)
      && num(q.text) === q.s.firesLeft && q.pips === Math.min(q.s.firesLeft, 8));
    check('cast: the counter follows the fires that spread', sp.burning > 3 && r.ok,
      `burning=${r.h.s.burning} ext=${r.h.s.extinguished} firesLeft=${r.h.s.firesLeft} text "${r.h.text}" pips=${r.h.pips}`);
    // Water on a counted fire that still burns: the number drops by one.
    await tp(0, 5.5);
    s = await st();
    const cands = [...lit, ...s.objs.filter((o) => o.burning && o.n === 'bush').map((o) => o.i)].filter((i, k, all) => s.objs[i].burning && all.indexOf(i) === k);
    let a = null; let target = -1;
    for (const i of cands) { a = await aimAt(i, 2); if (a.aimObj === i && a.objs[i].burning) { target = i; break; } }
    let dropOk = false; let detail = `no counted fire in view (cands ${cands})`;
    if (target >= 0) {
      const b = await hudRead();
      await cmd('cast');
      const z = await until((x) => x.impacts > b.s.impacts, 30000);
      r = await hudWithin(z.frame, 2, (q) => num(q.text) === num(b.text) - 1 && q.s.firesLeft === num(b.text) - 1);
      dropOk = r.ok;
      detail = `target ${target}: ${num(b.text)} -> ${num(r.h.text)} (firesLeft ${r.h.s.firesLeft}, burning ${r.h.s.burning}, ext ${r.h.s.extinguished})`;
    }
    check('cast: putting out a counted fire lowers the number by one', dropOk, detail);
  }

  // 16. Following the arrow finds the fire.
  {
    await fresh();
    await tp(0, 5.5);
    await cmd(`face ${lit[0]}`);
    const f = await until((x) => x.aimObj === lit[0], 10000);
    await cmd(`look ${(f.yaw + 180).toFixed(2)} 0`);
    const f0 = (await st()).frame;
    let r = await hudWithin(f0, 5, (q) => q.s.fireOff === true && q.s.nearFire >= 0 && q.s.objs[q.s.nearFire].burning && q.ff && q.ff.vis
      && q.ff.right === (q.s.fireBearing > 0) && q.ff.left === !(q.s.fireBearing > 0) && q.ff.w >= 56);
    check('cast: a fire behind the player shows the arrow on its side', r.ok,
      `fireOff=${r.h.s.fireOff} nearFire=${r.h.s.nearFire} bearing=${r.h.s.fireBearing} finder=${JSON.stringify(r.h.ff)}`);
    let steps = 0; let consistent = true; let h = r.h; const log = [];
    while (h.ff && h.ff.vis && steps < 40) {
      const f1 = h.s.frame;
      await cmd(`turn ${h.ff.right ? 150 : -150} 0`);
      await until((x) => x.frame >= f1 + 2, 5000);
      h = await hudRead();
      steps++;
      if (h.ff && h.ff.vis && h.ff.right !== (h.s.fireBearing > 0)) consistent = false;
      log.push(`${h.s.fireBearing}${h.ff && h.ff.vis ? (h.ff.right ? 'R' : 'L') : '-'}`);
    }
    // I-025 (Contract item 3): the arrow may also hide because another
    // fire came into view before the nearest one did.
    const found = !!h.ff && !h.ff.vis && h.s.fireInView === true && (h.s.fireOff === true || Math.abs(h.s.fireBearing) <= hfovOf(h.s.fov) / 2);
    const why = h.s.fireOff === false ? 'nearest fire in view' : 'another fire in view';
    check('cast: turning toward the arrow brings the fire into view', found && consistent && steps <= 40,
      `${steps} steps, consistent=${consistent}, bearing=${h.s.fireBearing} fireOff=${h.s.fireOff} fireInView=${h.s.fireInView} hid: ${why} [${log.join(' ')}]`);
    // A fire off screen again, then pause: the arrow hides.
    await cmd(`look ${(f.yaw + 180).toFixed(2)} 0`);
    await hudWithin((await st()).frame, 5, (q) => q.ff && q.ff.vis);
    await page.evaluate(() => window.__game.pause(true));
    const p0 = (await st()).frame;
    r = await hudWithin(p0, 2, (q) => q.ff && !q.ff.vis);
    check('cast: pausing hides the arrow', r.ok, `finder=${JSON.stringify(r.h.ff)} paused=${r.h.s.paused}`);
    await unpause();
  }

  // 17. Pause under automation: Esc and P, with no exitPointerLock fallback.
  {
    await fresh();
    const lk = await relock();
    const state17 = () => page.evaluate(() => ({ s: window.__game.state(), shown: !document.getElementById('pause').classList.contains('hidden'), ple: document.pointerLockElement === null }));
    const pausedNow = async () => {
      const f0 = (await st()).frame;
      let q = await state17();
      while (!(q.shown && q.s.paused && !q.s.locked && q.ple) && q.s.frame <= f0 + 10) { await sleep(20); q = await state17(); }
      return { ok: q.shown && q.s.paused && !q.s.locked && q.ple && q.s.frame <= f0 + 10, q };
    };
    await page.keyboard.press('Escape');
    const e = await pausedNow();
    check('cast: Esc pauses and frees the mouse under automation', lk && e.ok,
      `locked before=${lk} pause shown=${e.q.shown} paused=${e.q.s.paused} locked=${e.q.s.locked} lockElement null=${e.q.ple}`);
    // Back in through #resume, then P.
    const lk2 = await relock();
    await page.keyboard.press('KeyP');
    const p = await pausedNow();
    const f0 = (await st()).frame;
    await page.keyboard.press('KeyP');
    const u = await within(f0, 10, (x) => x.paused === false);
    const row = await page.evaluate(() => [...document.querySelectorAll('#pause tr')].some((tr) => tr.textContent.includes('Esc or P')));
    check('cast: P pauses and P again resumes; the pause card lists Esc or P', lk2 && p.ok && u.ok && row,
      `relocked=${lk2} P: shown=${p.q.shown} paused=${p.q.s.paused} locked=${p.q.s.locked}; P again: paused=${u.x.paused}; row=${row}`);
  }

  // 15. The crosshair tells a fire from a bush (locked), then point mode.
  {
    await fresh();
    const lk = await relock();
    await tp(4, 2);
    const b = idx('bush')[5];
    await cmd(`face ${b}`);
    let x = await until((q) => q.aimObj === b, 5000);
    for (let k = 0; k < 40 && x.aimObj !== b; k++) {
      const f = x.frame;
      await cmd('turn 9 0');
      x = await until((q) => q.frame >= f + 2, 5000);
    }
    let h = (await hudWithin(x.frame, 3, (q) => q.cross.includes('on'))).h;
    const bushOk = h.s.aimObj === b && h.cross.includes('on') && !h.cross.includes('fire');
    // A pause and resume restarts the 45 s idle clock (on a slow software GPU
    // this section can run that long), so the idle hint does not stand in
    // for the locked hint checked below.
    await page.evaluate(() => { window.__game.pause(true); window.__game.pause(false); });
    await cmd(`face ${lit[0]}`);
    x = await until((q) => q.aimObj === lit[0], 10000);
    h = (await hudWithin(x.frame, 3, (q) => q.cross.includes('fire'))).h;
    const fireOk = h.s.aimObj === lit[0] && h.cross.includes('on') && h.cross.includes('fire');
    const shadow = await page.evaluate(() => {
      const c = document.getElementById('crosshair');
      const span = c.querySelector('span');
      const had = c.classList.contains('fire');
      c.classList.add('fire');
      const withFire = getComputedStyle(span).boxShadow;
      c.classList.remove('fire');
      const without = getComputedStyle(span).boxShadow;
      if (had) c.classList.add('fire');
      return { withFire, without };
    });
    const ringOk = shadow.withFire !== 'none' && shadow.withFire !== shadow.without;
    check('cast: the crosshair adds a fire ring only on a burning target', lk && bushOk && fireOk && ringOk,
      `locked=${lk} bush ${b}: ${bushOk}, fire ${lit[0]}: ${fireOk} (${h.cross.join(' ')}), shadow ${JSON.stringify(shadow)}`);
    check('cast: the locked hint says to aim at a fire', h.hint === 'Aim at a fire and click.', `hint "${h.hint}"`);
    // Point mode (pointer free): the cursor over a fire vs over a bush.
    await page.evaluate(() => window.__game.pause(true));
    await page.evaluate(() => document.exitPointerLock());
    await until((q) => !q.locked, 8000);
    if (!(await pauseShown())) await page.evaluate(() => window.__game.pause(true));
    await page.click('#pointmode');
    await unpause();
    await cmd(`face ${lit[2]}`);
    await page.mouse.move(VW / 2 + 1, VH / 2 + 1);
    await page.mouse.move(VW / 2, VH / 2);
    x = await until((q) => q.hover === lit[2], 15000);
    const hf = (await hudWithin(x.frame, 3, (q) => q.canvas.includes('hover-fire'))).h;
    await cmd(`face ${b}`);
    await page.mouse.move(VW / 2 + 1, VH / 2 + 1);
    await page.mouse.move(VW / 2, VH / 2);
    x = await until((q) => q.hover === b, 15000);
    const hb = (await hudWithin(x.frame, 3, (q) => q.canvas.includes('hover') && !q.canvas.includes('hover-fire'))).h;
    check('cast: point mode marks a fire under the cursor apart from a bush',
      hf.s.hover === lit[2] && hf.canvas.includes('hover-fire') && hb.s.hover === b && hb.canvas.includes('hover') && !hb.canvas.includes('hover-fire'),
      `fire: hover=${hf.s.hover} [${hf.canvas.join(' ')}]; bush: hover=${hb.s.hover} [${hb.canvas.join(' ')}]`);
  }

  // 18. Nothing under the banners (pointer free, point mode, no casts).
  {
    await fresh();
    let x = await st();
    for (let k = 0; k < 10 && x.outcome !== 'lost'; k++) {
      const t0 = x.tick;
      await cmd('tick 2400');
      x = await until((q) => q.outcome !== 'playing' || q.tick >= t0 + 2400, 20000);
    }
    let r = await hudWithin(x.frame, 2, (q) => q.banner && q.hint === '' && q.ff && !q.ff.vis && !q.nudge);
    check('cast: nothing shows under the lose banner', x.outcome === 'lost' && !x.locked && x.casts === 0 && r.ok,
      `outcome=${x.outcome} locked=${x.locked} hint "${r.h.hint}" finder=${JSON.stringify(r.h.ff)} nudge=${r.h.nudge}`);
    await fresh();
    await tp(0, 5.5);
    for (const i of lit) await castAt(i, 2);
    x = await until((q) => q.outcome === 'won', 10000);
    r = await hudWithin(x.frame, 2, (q) => q.banner && q.hint === '' && q.ff && !q.ff.vis && !q.nudge);
    check('cast: nothing shows under the win banner', x.outcome === 'won' && r.ok,
      `outcome=${x.outcome} hint "${r.h.hint}" finder=${JSON.stringify(r.h.ff)} nudge=${r.h.nudge}`);
  }

  // 19. No flashing from the new effects (GS-6). A guard.
  {
    const grab = async () => lumaOf(await decodePng(await page.screenshot({ type: 'png', timeout: 180000 })));
    const run = async (spell) => {
      await fresh();
      await tp(0, 5.5);
      await aimAt(lit[0], spell);
      const shots = [await grab()];
      await cmd('fire');
      for (let k = 0; k < 19; k++) { await frames(1); shots.push(await grab()); }
      let worst = 0;
      for (let k = 1; k < shots.length; k++) worst = Math.max(worst, flashFrac(shots[k - 1], shots[k]));
      return worst;
    };
    const w = await run(2);
    const c = await run(4);
    check('cast: the stream and splash do not flash the screen', w < 0.2 && c < 0.2,
      `worst frame-to-frame change: Water ${(w * 100).toFixed(1)}%, CO2 ${(c * 100).toFixed(1)}%`);
  }
  await fresh();
}

// ---------------------------------------------------------------------------
// Review follow-ups (docs/loop/items/I-022/spec.md, tests 7-10): Hints off
// hides the coaching hints but keeps the start line and notes, Aim help off
// turns off the point-and-click tap and hover cone, and View width is the
// real horizontal angle at 4:3 and 21:9. Runs after castingYouCanSee, so its
// Hints and viewport changes cannot touch I-019's checks; each test restores
// what it changes.
// ---------------------------------------------------------------------------
async function reviewFollowups({ page, st, cmd, until, frames, sleep, idx, lit, locked, base, browser, errors }) {
  console.log('  -- review follow-ups (I-022) --');
  const VW = +(process.env.VW || 1280), VH = +(process.env.VH || 720);
  void base; void browser; void errors; void locked; void frames;
  // Copied from feelAndStandards.
  const setCtl = (sel, v, ev) => page.evaluate(([sel, v, ev]) => {
    const el = document.querySelector(sel);
    if (!el) return false;
    if (el.type === 'checkbox') el.checked = !!v; else el.value = String(v);
    el.dispatchEvent(new Event(ev || (el.type === 'range' ? 'input' : 'change'), { bubbles: true }));
    return true;
  }, [sel, v, ev]);
  const checked = (sel) => page.evaluate((sel) => { const e = document.querySelector(sel); return e ? e.checked : null; }, sel);
  // Copied from castingYouCanSee.
  const within = async (f0, n, pred, ms = 30000) => {
    const t0 = Date.now();
    let x = await st();
    while (!pred(x) && x.frame <= f0 + n && Date.now() - t0 < ms) { await sleep(20); x = await st(); }
    return { ok: !!pred(x) && x.frame <= f0 + n, x };
  };
  const unpause = async () => { if ((await st()).paused) { await page.evaluate(() => window.__game.pause(false)); await until((x) => !x.paused, 8000); } };
  const fresh = async () => {
    await unpause();
    await cmd('reset');
    await until((x) => x.outcome === 'playing' && x.burning === 3 && x.casts === 0, 20000);
  };
  const tp = async (x, z) => { await cmd(`tp ${x} ${z}`); await until((q) => Math.abs(q.pos[0] - x) < 0.5 && Math.abs(q.pos[2] - z) < 0.5, 10000); };
  const aimAt = async (i, spell) => {
    await unpause();
    await cmd(`face ${i}`);
    if (spell) await cmd(`select ${spell}`);
    return until((x) => (!spell || x.spell === spell - 1) && x.aimObj === i && x.cooldown === 0, 15000);
  };
  const pauseShown = () => page.evaluate(() => !document.getElementById('pause').classList.contains('hidden'));
  const relock = async () => {
    for (let k = 0; k < 6; k++) {
      if ((await st()).locked) return true;
      if (!(await pauseShown())) await page.evaluate(() => window.__game.pause(true));
      await page.click('#resume');
      if ((await until((q) => q.locked, 4000)).locked) return true;
      await sleep(1200);
    }
    return (await st()).locked;
  };
  // #hint (shown text, or '' when hidden or empty) with the state the HUD
  // last drew, in one read (a cut-down hudRead from castingYouCanSee).
  const hintRead = () => page.evaluate(() => {
    const s = window.__game.hudState() || window.__game.state();
    const e = document.getElementById('hint');
    const vis = !!e && !e.classList.contains('hidden') && e.getClientRects().length > 0 && getComputedStyle(e).display !== 'none' && getComputedStyle(e).visibility !== 'hidden';
    return { s, hint: vis ? e.textContent.trim() : '' };
  });
  const hintWithin = async (f0, n, pred, ms = 30000) => {
    const t0 = Date.now();
    let h = await hintRead();
    while (!pred(h) && h.s.frame <= f0 + n && Date.now() - t0 < ms) { await sleep(20); h = await hintRead(); }
    return { ok: !!pred(h) && h.s.frame <= f0 + n, h };
  };
  // Point mode on, pointer free, pause card closed.
  const pointMode = async () => {
    if ((await st()).locked) {
      await page.evaluate(() => window.__game.pause(true));
      await page.evaluate(() => document.exitPointerLock());
      await until((q) => !q.locked, 8000);
    }
    if (!(await pauseShown())) await page.evaluate(() => window.__game.pause(true));
    await page.click('#pointmode');
    await unpause();
    return page.evaluate(() => window.__game.mode().pointMode);
  };
  const centre = async (w = VW, h = VH) => { await page.mouse.move(w / 2 + 1, h / 2 + 1); await page.mouse.move(w / 2, h / 2); };
  const DEGR = Math.PI / 180;
  const vfovAt = (h, a) => 2 * Math.atan(Math.tan((h * DEGR) / 2) / a) / DEGR;
  const fmt = (v, d = 2) => (Number.isFinite(v) ? v.toFixed(d) : String(v));

  // 7. Hints off hides the locked hint.
  {
    await fresh();
    const lk = await relock();
    let f0 = (await st()).frame;
    const pre = await hintWithin(f0, 2, (h) => h.hint !== '' && h.s.locked && h.s.extinguished === 0);
    const ticked = (await checked('#hints')) === true;
    f0 = (await st()).frame;
    await setCtl('#hints', false);
    const off = await hintWithin(f0, 2, (h) => h.hint === '');
    f0 = (await st()).frame;
    await setCtl('#hints', true);
    const on = await hintWithin(f0, 2, (h) => h.hint !== '');
    check('review: Hints off hides the locked hint, on brings it back',
      lk && ticked && pre.ok && off.ok && on.ok && off.h.s.locked && on.h.s.locked,
      `locked=${lk} ticked=${ticked} before "${pre.h.hint}" (ext=${pre.h.s.extinguished}); off "${off.h.hint}" ok=${off.ok}; on "${on.h.hint}" ok=${on.ok}`);
  }

  // 8. Hints off keeps the start line and the notes (guard).
  {
    await setCtl('#hints', false);
    await page.evaluate(() => document.exitPointerLock());
    await until((q) => !q.locked, 8000);
    await page.evaluate(() => window.__game.pause(false));
    await until((x) => !x.paused, 8000);
    await fresh();
    let f0 = (await st()).frame;
    const start = await hintWithin(f0, 10, (h) => h.hint !== '' && !h.s.locked && h.s.casts === 0);
    const pm = await pointMode();
    f0 = (await st()).frame;
    const note = await hintWithin(f0, 10, (h) => h.hint !== '' && !h.s.locked);
    check('review: Hints off keeps the start line and the point-and-click note',
      (await checked('#hints')) === false && start.ok && pm && note.ok,
      `start "${start.h.hint}" (locked=${start.h.s.locked} casts=${start.h.s.casts}); point mode=${pm} note "${note.h.hint}"`);
    await setCtl('#hints', true);
    const saved = await page.evaluate(() => { try { return JSON.parse(localStorage.getItem('alchemist.settings.v1')); } catch (e) { return null; } });
    check('review: Hints is ticked and saved again afterwards', (await checked('#hints')) === true && (!saved || saved.hints !== false), JSON.stringify(saved));
  }

  // 9. Aim help off: no tap or hover cone. Pose fixed by the spec.
  {
    await fresh();
    await tp(4, 2);
    const bushes = idx('bush');
    const b = bushes[5];
    let found = false;
    let s = await aimAt(b);
    if (s.aimObj === b) {
      for (let k = 0; k < 40 && !found; k++) {
        const f = (await st()).frame;
        await cmd('turn 9 0');
        s = await until((x) => x.frame >= f + 2, 5000);
        found = s.aimObj === b && s.assisted === true;
      }
    }
    const pm = await pointMode();
    await centre();
    const onHover = await until((x) => x.hover === b, 15000);
    let f0 = (await st()).frame;
    await setCtl('#assist', false);
    const off = await within(f0, 10, (x) => x.assist === false && x.hover === -1);
    f0 = (await st()).frame;
    await setCtl('#assist', true);
    const on = await within(f0, 10, (x) => x.assist === true && x.hover === b);
    check('review: Aim help off turns off the tap and hover cone',
      found && pm && onHover.hover === b && off.ok && on.ok,
      `near miss on bush ${b}: ${found}; point mode=${pm}; on: hover=${onHover.hover}; off: hover=${off.x.hover} assist=${off.x.assist}; on again: hover=${on.x.hover}`);
    await setCtl('#assist', false);
    await until((x) => x.assist === false, 5000);
    await cmd(`face ${lit[0]}`);
    await centre();
    const exact = await until((x) => x.hover === lit[0], 15000);
    check('review: an exact point-and-click hit still works with Aim help off', exact.hover === lit[0] && exact.assist === false,
      `hover=${exact.hover} want ${lit[0]} assist=${exact.assist}`);
    await setCtl('#assist', true);
    await until((x) => x.assist === true, 5000);
  }

  // I-044 test 6 helpers. World to camera space for a view at yaw, pitch
  // (degrees; the camera is Ry(yaw)·Rx(pitch), looking down -z).
  const toCam = (yawD, pitchD, v) => {
    const y = yawD * DEGR, p = pitchD * DEGR;
    const x1 = v[0] * Math.cos(y) - v[2] * Math.sin(y);
    const z1 = v[0] * Math.sin(y) + v[2] * Math.cos(y);
    return [x1, v[1] * Math.cos(p) + z1 * Math.sin(p), -v[1] * Math.sin(p) + z1 * Math.cos(p)];
  };
  // Pixel where a camera-space direction lands for a ray built with this
  // vertical fov and aspect (the inverse of feel::ray_dir).
  const toPx = (c, vfovD, aspect, W, H) => {
    const t = Math.tan((vfovD * DEGR) / 2);
    const u = c[0] / -c[2], v = c[1] / -c[2];
    return { x: ((u / (t * aspect)) + 1) / 2 * W, y: (1 - v / t) / 2 * H };
  };
  const angDiff = (a, b) => ((((a - b) % 360) + 540) % 360) - 180;
  const lookTo = async (yawD, pitchD) => {
    await cmd(`look ${yawD.toFixed(4)} ${pitchD.toFixed(4)}`);
    return until((x) => Math.abs(angDiff(x.yaw, yawD)) < 0.02 && Math.abs(x.pitch - pitchD) < 0.02, 10000);
  };
  const hoverAt = async (x, y, pred) => {
    await page.mouse.move(x + 1, y + 1);
    await page.mouse.move(x, y);
    const f0 = (await st()).frame;
    const r = await within(f0, 10, pred);
    return r;
  };
  // Test 6 on one viewport: the target off-centre at about (0.75W, 0.30H),
  // a hit at P, and both wrong-ray controls at least 25 px away and missing.
  const rayOffCentre = async (W, H) => {
    await page.setViewportSize({ width: W, height: H });
    const want = vfovAt(90, W / H);
    await within((await st()).frame, 10, (x) => Math.abs(x.fov - want) <= 0.3);
    const tries = [];
    for (const [px, pz, ti] of [[0, 5.5, lit[0]], [0, 12, lit[2]], [0, 12, lit[1]]]) {
      const pose = `tp ${px} ${pz} face ${ti}`;
      await tp(px, pz);
      let f = (await st()).frame;
      await cmd(`face ${ti}`);
      let s0 = await until((x) => x.frame >= f + 2, 5000);
      const yaw0 = s0.yaw, pitch0 = s0.pitch, cam = s0.cam;
      const d = [-Math.sin(yaw0 * DEGR) * Math.cos(pitch0 * DEGR), Math.sin(pitch0 * DEGR), -Math.cos(yaw0 * DEGR) * Math.cos(pitch0 * DEGR)];
      const T = [cam[0] + 16 * d[0], cam[1] + 16 * d[1], cam[2] + 16 * d[2]];
      const tv = Math.tan((s0.fov * DEGR) / 2);
      const aWant = Math.atan(0.5), bWant = Math.atan2(0.4 * tv, Math.sqrt(1.25));
      let yaw = yaw0 + aWant / DEGR, pitch = pitch0 - bWant / DEGR;
      let s1 = await lookTo(yaw, pitch);
      let c = null, P = null, placed = false;
      for (let k = 0; k <= 3; k++) {
        const rel = [T[0] - s1.cam[0], T[1] - s1.cam[1], T[2] - s1.cam[2]];
        c = toCam(s1.yaw, s1.pitch, rel);
        P = toPx(c, s1.fov, W / H, W, H);
        placed = c[2] < 0 && Math.abs(P.x - 0.75 * W) <= 0.05 * W && Math.abs(P.y - 0.30 * H) <= 0.05 * H;
        if (placed || k === 3) break;
        const a = Math.atan2(c[0], -c[2]), b = Math.atan2(c[1], Math.hypot(c[0], c[2]));
        yaw = s1.yaw - (a - aWant) / DEGR;
        pitch = s1.pitch + (b - bWant) / DEGR;
        s1 = await lookTo(yaw, pitch);
      }
      const fov = s1.fov;
      const hit = placed ? await hoverAt(P.x, P.y, (x) => x.hover === ti) : { ok: false, x: { hover: null } };
      const ctl = [];
      for (const [name, vc, ac] of [['i stale vfov 58.72', 58.72, W / H], ['ii 16:9 aspect', fov, 16 / 9]]) {
        const Q = toPx(c, vc, ac, W, H);
        const dist = Math.hypot(Q.x - P.x, Q.y - P.y);
        const inside = Q.x >= 0 && Q.x <= W - 1 && Q.y >= 0 && Q.y <= H - 1;
        const miss = inside ? await hoverAt(Q.x, Q.y, (x) => x.hover !== ti) : { ok: false, x: { hover: null } };
        ctl.push({ name, Q, dist, inside, ok: inside && dist >= 25 && miss.ok, hover: miss.x.hover });
      }
      const ok = placed && hit.ok && ctl.every((q) => q.ok);
      tries.push(`${W}x${H} fov=${fmt(fov)} pose "${pose}" P=(${fmt(P.x, 0)},${fmt(P.y, 0)}) placed=${placed} hover@P=${hit.x.hover} want ${ti}; `
        + ctl.map((q) => `(${q.name}) P'=(${fmt(q.Q.x, 0)},${fmt(q.Q.y, 0)}) |P'-P|=${fmt(q.dist, 1)} px hover=${q.hover}${q.inside ? '' : ' off-screen'}`).join('; '));
      // Only a control failure moves on to the next pose; a missed hit fails.
      if (ok || !(placed && hit.ok)) return { ok, note: tries.join(' / ') };
    }
    return { ok: false, note: tries.join(' / ') };
  };

  // 10. View width holds at 4:3 and 21:9, live.
  {
    const notes = [];
    const rayNotes = [];
    let ok43 = false, ok219 = false, ok100 = false, hoverOk = false, back = false;
    try {
      await page.setViewportSize({ width: 960, height: 720 });
      let f0 = (await st()).frame;
      let r = await within(f0, 10, (x) => Math.abs(x.fov - vfovAt(90, 4 / 3)) <= 0.3);
      ok43 = r.ok; notes.push(`4:3 fov=${fmt(r.x.fov)} want ${fmt(vfovAt(90, 4 / 3))}`);
      await page.setViewportSize({ width: 1680, height: 720 });
      f0 = (await st()).frame;
      r = await within(f0, 10, (x) => Math.abs(x.fov - vfovAt(90, 21 / 9)) <= 0.3);
      ok219 = r.ok; notes.push(`21:9 fov=${fmt(r.x.fov)} want ${fmt(vfovAt(90, 21 / 9))}`);
      await page.setViewportSize({ width: 960, height: 720 });
      await within((await st()).frame, 10, (x) => Math.abs(x.fov - vfovAt(90, 4 / 3)) <= 0.3);
      await page.evaluate(() => window.__game.pause(true));
      f0 = (await st()).frame;
      await setCtl('#fov', 100);
      r = await within(f0, 10, (x) => Math.abs(x.fov - vfovAt(100, 4 / 3)) <= 0.3);
      const label = await page.evaluate(() => document.getElementById('fov-val')?.textContent ?? null);
      ok100 = r.ok && label === '100°'; notes.push(`4:3 at 100: fov=${fmt(r.x.fov)} want ${fmt(vfovAt(100, 4 / 3))} label=${label}`);
      await setCtl('#fov', 90);
      await until((x) => Math.abs(x.fov - vfovAt(90, 4 / 3)) <= 0.3, 10000);
      await unpause();
      const pm = await page.evaluate(() => window.__game.mode().pointMode) || await pointMode();
      // I-044 test 6: the cursor ray off-centre on both axes, Aim help off
      // (so the hover cone cannot hide a wrong ray), with two wrong-ray
      // controls that must miss.
      await setCtl('#assist', false);
      await until((x) => x.assist === false, 5000);
      let both = pm;
      for (const [W, H] of [[960, 720], [1680, 720]]) {
        const r6 = await rayOffCentre(W, H);
        both = both && r6.ok;
        rayNotes.push(r6.note);
      }
      hoverOk = both;
    } catch (e) {
      notes.push(`error: ${e && e.message}`);
    } finally {
      await setCtl('#assist', true);
      await until((x) => x.assist === true, 5000);
      await page.setViewportSize({ width: VW, height: VH });
      const f0 = (await st()).frame;
      const r = await within(f0, 10, (x) => Math.abs(x.fov - 58.72) <= 0.2);
      back = r.ok; notes.push(`back at ${VW}x${VH}: fov=${fmt(r.x.fov)}`);
      await unpause();
    }
    check('review: View width is 90° wide at 4:3 and 21:9, live', ok43 && ok219, notes.join('; '));
    check('review: View width 100° holds at 4:3 and the label says 100°', ok100, notes.join('; '));
    check('review: the cursor ray matches the view off-centre at 4:3 and 21:9', hoverOk, rayNotes.join(' | '));
    check('review: the view returns to 58.72° tall at the default size', back, notes.join('; '));
  }

  // I-044 tests 7-9: the gauntlet's spell gem stays on screen in short, wide
  // windows; the default look does not move; Esc with Hints off never
  // flashes the aim hint.
  {
    const gemNotes = [], restNotes = [];
    let gemOk = true, restOk = false;
    const swayWas = await checked('#sway');
    // The gem in camera space: the root plus Rx(tilt)·Ry(0.1)·(0, 0.0275,
    // -0.0605) (local (0, 0.05, -0.11) scaled by 0.55; the Contract keeps
    // these fixed). Round 2: the rest pose orbits up about the eye by
    // tilt = 17.6 - clamp(fov/2 - 5, 4, 17.6) deg (feel::vm_tilt_deg),
    // recomputed here from the live fov, not read back from the game.
    const tiltOf = (fov) => (17.6 - Math.max(4, Math.min(17.6, fov / 2 - 5))) * DEGR;
    const rotX = (t, [x, y, z]) => [x, y * Math.cos(t) - z * Math.sin(t), y * Math.sin(t) + z * Math.cos(t)];
    const gemOf = (vm, fov) => {
      const o = rotX(tiltOf(fov), [Math.sin(0.1) * -0.0605, 0.0275, Math.cos(0.1) * -0.0605]);
      return [vm[0] + o[0], vm[1] + o[1], vm[2] + o[2]];
    };
    const setHud = (hide) => page.evaluate((hide) => {
      for (const id of ['hud', 'pause']) { const e = document.getElementById(id); if (e) e.style.setProperty('display', hide ? 'none' : ''); }
    }, hide);
    const blueIn = (img, cx, cy) => {
      let n = 0;
      for (let y = Math.max(0, Math.round(cy) - 7); y <= Math.min(img.h - 1, Math.round(cy) + 7); y++) {
        for (let x = Math.max(0, Math.round(cx) - 7); x <= Math.min(img.w - 1, Math.round(cx) + 7); x++) {
          const k = (y * img.w + x) * img.bpp;
          const R = img.data[k], B = img.data[k + 2];
          if (B >= R + 40 && B >= 120) n++;
        }
      }
      return n;
    };
    try {
      await fresh();
      const pm = await page.evaluate(() => window.__game.mode().pointMode) || await pointMode();
      await setCtl('#sway', false);
      await cmd('select 2');
      const s0 = await until((x) => x.sway === false && x.spell === 1, 5000);
      await lookTo(s0.yaw, -25);
      for (const [W, H, hf] of [[1680, 720, 70], [1680, 720, 75], [960, 720, 70]]) {
        await page.setViewportSize({ width: W, height: H });
        await setCtl('#fov', hf);
        const want = vfovAt(hf, W / H);
        // At rest: fov live, no cast for 1 s (recoil 0), no stream in flight.
        await sleep(1000);
        const x = await until((q) => Math.abs(q.fov - want) <= 0.3 && q.streams === 0 && q.sway === false && q.spell === 1, 10000);
        await frames(3);
        const s1 = await st();
        const g = gemOf(s1.vm, s1.fov);
        const below = Math.atan(-g[1] / -g[2]) / DEGR;
        // The root sits where the orbit puts it (so `g` is where the gem is).
        const vmWant = rotX(tiltOf(s1.fov), [0.21, -0.18, -0.42]);
        const vmOk = s1.vm.every((v, i) => Math.abs(v - vmWant[i]) <= 0.002);
        const geoOk = Math.abs(s1.fov - want) <= 0.3 && vmOk && below <= s1.fov / 2 - 3;
        await setHud(true);
        await frames(2);
        const img = await decodePng(await page.screenshot({ type: 'png', timeout: 180000 }));
        await setHud(false);
        const G = toPx(g, s1.fov, W / H, W, H);
        const sx = img.w / W, sy = img.h / H;
        const nGem = blueIn(img, G.x * sx, G.y * sy), nMirror = blueIn(img, (W - G.x) * sx, G.y * sy);
        const pixOk = nGem >= 10 && nMirror < 10;
        gemOk = gemOk && pm && geoOk && pixOk && s1.streams === 0;
        gemNotes.push(`${W}x${H} hfov ${hf}: fov=${fmt(s1.fov)} vm=[${s1.vm.map((v) => fmt(v, 4)).join(',')}] (orbit [${vmWant.map((v) => fmt(v, 4)).join(',')}] ${vmOk ? 'ok' : 'OFF'}) gem ${fmt(below)}° below (limit ${fmt(s1.fov / 2 - 3)}) G=(${fmt(G.x, 0)},${fmt(G.y, 0)}) blue ${nGem} vs mirror ${nMirror} streams=${x.streams}`);
      }
      // 8. The default view: the gauntlet has not moved.
      await page.setViewportSize({ width: 1280, height: 720 });
      await setCtl('#fov', 90);
      await sleep(400);
      const r = await until((q) => Math.abs(q.fov - 58.72) <= 0.2 && q.sway === false, 10000);
      await frames(3);
      const s2 = await st();
      const rest = [0.21, -0.18, -0.42];
      restOk = Math.abs(r.fov - 58.72) <= 0.2 && s2.vm.every((v, i) => Math.abs(v - rest[i]) <= 0.002);
      restNotes.push(`1280x720 hfov 90: fov=${fmt(s2.fov)} vm=[${s2.vm.map((v) => fmt(v, 4)).join(',')}] want [0.21,-0.18,-0.42] ± 0.002`);
    } catch (e) {
      gemOk = false;
      gemNotes.push(`error: ${e && e.message}`);
    } finally {
      await setHud(false);
      await setCtl('#fov', 90);
      await setCtl('#sway', swayWas !== false);
      await page.setViewportSize({ width: VW, height: VH });
      await until((q) => Math.abs(q.fov - vfovAt(90, VW / VH)) <= 0.3, 10000);
    }
    check("review: the gauntlet's spell gem stays on screen at 21:9", gemOk, gemNotes.join(' | '));
    check('review: the gauntlet does not move at the default view', restOk, restNotes.join(' | '));
  }

  // 9. Esc with Hints off never flashes the aim hint.
  {
    const AIM = 'Aim at a fire and click.';
    let ok = false, note = '';
    try {
      await setCtl('#hints', false);
      await fresh();
      const lk = await relock();
      const s0 = await until((x) => x.locked && x.casts === 0 && x.extinguished === 0, 10000);
      await page.evaluate(() => {
        const e = document.getElementById('hint');
        window.__i044Hints = [];
        const rec = () => { if (!e.classList.contains('hidden')) window.__i044Hints.push(e.textContent.trim()); };
        window.__i044Obs = new MutationObserver(rec);
        window.__i044Obs.observe(e, { childList: true, characterData: true, subtree: true, attributes: true });
      });
      await page.keyboard.press('Escape');
      await frames(15);
      const seen = await page.evaluate(() => { window.__i044Obs.disconnect(); return window.__i044Hints; });
      const s1 = await st();
      ok = lk && s0.locked && s0.casts === 0 && s0.extinguished === 0 && !s1.locked && !seen.includes(AIM);
      note = `locked=${lk} before: casts=${s0.casts} ext=${s0.extinguished}; after Esc locked=${s1.locked} paused=${s1.paused}; shown ${JSON.stringify(seen)}`;
    } catch (e) {
      note = `error: ${e && e.message}`;
    } finally {
      await setCtl('#hints', true);
      await relock();
    }
    check('review: Esc with Hints off never flashes the aim hint', ok, note);
  }
  await fresh();
}

