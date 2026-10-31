// I-046 round 2: the checks the round-1 break could not run, on a site-like
// copy (pkg-out + age.js + analytics.js, rg-analytics on, as build-site.sh
// makes it). At 16:9, 4:3 and 21:9 (1280x720, 960x720, 1680x720):
//   1. the age picker shows before pointer lock, has 3 bands, closes on a pick;
//   2. the start hint shows; while locked, Hints off clears the locked hint
//      and on brings it back; unlocked, the start line stays with Hints off;
//   3. edge aim: in point-and-click mode, a fire turned to about 85 % of the
//      width (near the right edge) is hovered and a real click there casts
//      exactly once, at that fire, with a stream live (screenshot saved).
// Usage: node round2-play.mjs <site dir> <shot dir>
import { createServer } from 'node:http';
import { readFileSync, existsSync, mkdirSync } from 'node:fs';
import { join, extname, dirname } from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(here, '../../../../../../tests/e2e/golden.mjs'));
const { chromium } = require('playwright');
const [ROOT, SHOTS] = process.argv.slice(2);
mkdirSync(SHOTS, { recursive: true });
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.css': 'text/css', '.svg': 'image/svg+xml' };
const srv = createServer((q, r) => {
  let p = decodeURIComponent(q.url.split('?')[0]); if (p.endsWith('/')) p += 'index.html';
  const f = join(ROOT, p);
  if (q.method === 'POST') { r.writeHead(204); return r.end(); }
  if (!existsSync(f)) { r.writeHead(404); return r.end(); }
  r.writeHead(200, { 'content-type': TYPES[extname(f)] || 'application/octet-stream' }); r.end(readFileSync(f));
}).listen(0, '127.0.0.1');
await new Promise((ok) => srv.on('listening', ok));
const port = srv.address().port;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const browser = await chromium.launch();
const results = [];
const check = (name, ok, detail) => { results.push(!!ok); console.log(`${ok ? 'PASS' : 'FAIL'} ${name} :: ${detail}`); };
for (const [W, H, tag] of [[1280, 720, '16:9'], [960, 720, '4:3'], [1680, 720, '21:9']]) {
  const ctx = await browser.newContext({ viewport: { width: W, height: H } });
  const page = await ctx.newPage();
  const errs = []; page.on('pageerror', (e) => errs.push(e.message));
  await page.goto(`http://127.0.0.1:${port}/?low&debug`);
  await page.waitForSelector('#age', { state: 'visible', timeout: 120000 }).catch(() => undefined);
  const visible = await page.evaluate(() => { const e = document.getElementById('age'); return !!e && !e.hidden && getComputedStyle(e).display !== 'none'; });
  const lockedWhile = await page.evaluate(() => !!document.pointerLockElement);
  const bands = await page.evaluate(() => [...document.querySelectorAll('#age button')].map((b) => b.dataset.band));
  await page.screenshot({ path: join(SHOTS, `${W}x${H}-0-picker.png`), timeout: 180000 });
  let hiddenAfter = false;
  if (visible) {
    await page.click('#age button[data-band="seedling"]');
    hiddenAfter = await page.evaluate(() => document.getElementById('age').hidden === true);
  }
  check(`${tag} picker shows before pointer lock and closes on a pick`, visible && !lockedWhile && bands.length === 3 && hiddenAfter,
    `visible=${visible} lockedWhilePicker=${lockedWhile} bands=${bands.join(',')} hiddenAfter=${hiddenAfter}`);
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState));) { if (Date.now() - t0 > 120000) break; await sleep(250); }
  const st = () => page.evaluate(() => window.__game.state());
  const cmd = (c) => page.evaluate((c) => window.__game.cmd(c), c);
  const until = async (pred, ms = 30000) => { const t0 = Date.now(); let x = await st(); while (!pred(x) && Date.now() - t0 < ms) { await sleep(150); x = await st(); } return x; };
  const hint = () => page.evaluate(() => { const h = document.getElementById('hint'); return h && !h.hidden && getComputedStyle(h).display !== 'none' ? h.textContent.trim() : ''; });
  await sleep(2000);
  // Hints gate the locked hint only; the start line and the point-and-click
  // note stay with Hints off by design (golden: "review: Hints off keeps the
  // start line and the point-and-click note").
  const h0 = await hint();
  const pauseShown = () => page.evaluate(() => !document.getElementById('pause').classList.contains('hidden'));
  const setHints = (v) => page.evaluate((v) => { const el = document.getElementById('hints'); el.checked = v; el.dispatchEvent(new Event('change', { bubbles: true })); }, v);
  let locked = false;
  for (let k = 0; k < 6 && !locked; k++) {
    if (!(await pauseShown())) await page.evaluate(() => window.__game.pause(true));
    await page.click('#resume');
    locked = (await until((q) => q.locked, 4000)).locked;
    if (!locked) await sleep(1200);
  }
  await sleep(600);
  const hLocked = await hint();
  await setHints(false); await sleep(600);
  const hOff = await hint();
  await setHints(true); await sleep(600);
  const hOn = await hint();
  await page.evaluate(() => document.exitPointerLock());
  await until((q) => !q.locked, 8000);
  await setHints(false); await page.evaluate(() => window.__game.pause(false)); await sleep(600);
  const hStartOff = await hint();
  await setHints(true);
  check(`${tag} hints: start line shows; locked hint gated by Hints; start line kept with Hints off`,
    h0.length > 0 && locked && hLocked.length > 0 && hOff === '' && hOn.length > 0 && hStartOff.length > 0,
    `start "${h0}"; locked=${locked} hint "${hLocked}"; Hints off "${hOff}"; on "${hOn}"; unlocked + Hints off "${hStartOff}"`);
  // 3. Edge aim in point-and-click mode.
  await page.evaluate(() => window.__game.pause(true));
  await sleep(300);
  await page.click('#pointmode');
  await page.evaluate(() => window.__game.pause(false));
  const pm = await page.evaluate(() => window.__game.mode().pointMode);
  await cmd('reset');
  let s = await until((x) => x.outcome === 'playing' && x.burning === 3 && x.casts === 0, 20000);
  const lit = s.objs.filter((o) => o.burning).map((o) => o.i);
  const ti = lit[2];
  await cmd('select 2');
  await cmd(`face ${ti}`);
  s = await until((x) => x.aimObj === ti, 10000);
  const hfov = 2 * Math.atan(Math.tan((s.fov * Math.PI / 180) / 2) * W / H) * 180 / Math.PI;
  // Turn left so the fire sits at about 85 % of the width.
  const off = Math.atan(0.7 * Math.tan((hfov * Math.PI / 180) / 2)) * 180 / Math.PI;
  await cmd(`look ${s.yaw + off} ${s.pitch}`);
  await sleep(800);
  s = await st();
  let at = null;
  for (let x = Math.round(0.97 * W); x >= Math.round(0.6 * W) && !at; x -= 8) {
    for (const y of [H / 2, H / 2 + 20, H / 2 - 20, H / 2 + 45]) {
      await page.mouse.move(x, y);
      await sleep(60);
      if ((await st()).hover === ti) { at = [x, y]; break; }
    }
  }
  const pre = await st();
  const c0 = pre.casts, wet0 = pre.objs[ti].wet, temp0 = pre.objs[ti].temp;
  let cast = null;
  if (at) {
    await until((x) => x.cooldown === 0, 5000);
    await page.mouse.click(at[0], at[1]);
    cast = await until((x) => x.casts > c0 && x.streams >= 1, 5000);
    await page.screenshot({ path: join(SHOTS, `${W}x${H}-1-edge-cast.png`), timeout: 180000 });
    const after = await until((x) => x.objs[ti].wet > wet0 || !x.objs[ti].burning, 4000);
    cast = { ...cast, casts1: after.casts, hitIt: after.objs[ti].wet > wet0 || !after.objs[ti].burning, wet1: after.objs[ti].wet, temp1: after.objs[ti].temp };
  }
  const frac = at ? at[0] / W : null;
  check(`${tag} edge aim: a click on a fire near the right edge casts once at it`,
    pm && at && frac >= 0.75 && cast && cast.casts1 === c0 + 1 && cast.streams >= 1 && cast.hitIt,
    `pointMode=${pm} yaw ${s.yaw.toFixed(1)} (turned ${off.toFixed(1)}°, hfov ${hfov.toFixed(1)}°) hover@${at ? `${at[0]},${at[1]} (${(frac * 100).toFixed(0)} % of width)` : 'none'}; casts ${c0} -> ${cast ? cast.casts1 : '-'}; streams ${cast ? cast.streams : '-'}; fire ${ti} wet ${wet0} -> ${cast ? cast.wet1 : '-'}, temp ${temp0} -> ${cast ? cast.temp1 : '-'}; errors ${errs.length}`);
  await ctx.close();
}
await browser.close(); srv.close();
const ok = results.every(Boolean);
console.log(ok ? `ALL ${results.length} PASS` : `${results.filter((r) => !r).length} of ${results.length} FAIL`);
process.exit(ok ? 0 : 1);
