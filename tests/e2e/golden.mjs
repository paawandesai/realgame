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
  await page.waitForFunction(() => window.__gameState, null, { timeout: 120000 });
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
  check('no console errors', errors.length === 0, errors.slice(0, 3).join(' | '));
  const fails = results.filter((r) => !r.ok);
  console.log(`\n${results.length - fails.length}/${results.length} passed`);
  await browser.close();
  if (server) server.close();
  process.exit(fails.length ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(2); });
