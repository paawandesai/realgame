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
  const keysOk = keys.length === BASE_KEYS.length + 2 + I019_KEYS.length + I025_KEYS.length
    && [...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS, ...I025_KEYS].every((k, i) => keys[i] === k);
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

  // 10. View width holds at 4:3 and 21:9, live.
  {
    const notes = [];
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
      await cmd(`face ${lit[0]}`);
      await centre(960, 720);
      const h = await until((x) => x.hover === lit[0], 15000);
      hoverOk = pm && h.hover === lit[0]; notes.push(`4:3 hover=${h.hover} want ${lit[0]}`);
    } catch (e) {
      notes.push(`error: ${e && e.message}`);
    } finally {
      await page.setViewportSize({ width: VW, height: VH });
      const f0 = (await st()).frame;
      const r = await within(f0, 10, (x) => Math.abs(x.fov - 58.72) <= 0.2);
      back = r.ok; notes.push(`back at ${VW}x${VH}: fov=${fmt(r.x.fov)}`);
      await unpause();
    }
    check('review: View width is 90° wide at 4:3 and 21:9, live', ok43 && ok219, notes.join('; '));
    check('review: View width 100° holds at 4:3 and the label says 100°', ok100, notes.join('; '));
    check('review: the cursor ray matches the view at 4:3', hoverOk, notes.join('; '));
    check('review: the view returns to 58.72° tall at the default size', back, notes.join('; '));
  }
  await fresh();
}
