// I-028 breaker seam checks. Usage: node brk.mjs <pkg-out dir>
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { createRequire } from 'node:module';
const require = createRequire('/home/claude/realgame/package.json');
const { chromium } = require('playwright');
const root = process.argv[2];
const ONLY = process.env.ONLY || '';
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
const server = createServer(async (req, res) => {
  const u = new URL(req.url, 'http://x');
  const path = normalize(decodeURIComponent(u.pathname)).replace(/^[/\\]+/, '') || 'index.html';
  try {
    let body = await readFile(join(root, path));
    if (path === 'index.html' && u.searchParams.has('picker')) body = Buffer.from(body.toString().replace('<meta name="rg-analytics" content="off">', '<meta name="rg-analytics" content="on">'));
    res.writeHead(200, { 'content-type': types[extname(path)] || 'application/octet-stream' });
    res.end(body);
  } catch { res.writeHead(404).end(); }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const BASE = `http://127.0.0.1:${server.address().port}/`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const out = [];
const check = (name, ok, detail = '') => { out.push({ name, ok: !!ok }); console.log(`${ok ? 'PASS' : 'FAIL'} ${name} ${typeof detail === 'string' ? detail : JSON.stringify(detail)}`); };
const browser = await chromium.launch();

async function load(q, ctxOpts = {}) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 720 }, ...ctxOpts });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  await page.goto(`${BASE}?low&debug${q}`);
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState && !!window.__bench && !!window.__benchState));) {
    if (Date.now() - t0 > 120000) throw new Error('no boot');
    await sleep(250);
  }
  const st = () => page.evaluate(() => window.__game.state());
  const bs = () => page.evaluate(() => JSON.parse(window.__benchState));
  const frames = async (n) => { const f0 = (await st()).frame; for (const t0 = Date.now(); (await st()).frame < f0 + n && Date.now() - t0 < 30000;) await sleep(40); };
  const until = async (fn, ms = 15000) => { for (const t0 = Date.now(); Date.now() - t0 < ms; await sleep(50)) { const v = await fn(); if (v) return v; } return null; };
  await frames(3);
  return { ctx, page, errors, st, bs, frames, until };
}

// A. Compositions judged Ok for every learnable spell and band.
if (!ONLY || ONLY.includes('A')) {
  for (const band of ['sprout', 'seedling', 'apprentice', 'adept']) {
    const { ctx, page, bs, until } = await load(`&band=${band}`);
    await page.evaluate(() => window.__bench.open());
    await until(async () => (await bs()).spells);
    const spells = (await bs()).spells;
    const results = [];
    for (const s of spells) {
      await page.evaluate((slot) => window.__bench.cmd(`pick ${slot}`), s.slot);
      const v = await until(async () => { const b = await bs(); return b.pick === s.slot && (band === 'apprentice' || band === 'adept' || b.recipe) ? b : null; });
      const text = await page.evaluate(async ({ v, s, band }) => {
        const m = await import('./bench.js');
        const r = v.recipe;
        if (band === 'apprentice' || band === 'adept') return s.formula;
        if (r.some((t) => t.side === 1)) return m.composeEquation(r, r.map((t) => t.coef));
        const counts = {}; for (const t of r) for (const [el, n] of t.atoms) counts[el] = (counts[el] || 0) + n;
        if (band === 'sprout') { const placed = []; for (const [el, n] of Object.entries(counts).reverse()) for (let k = 0; k < n; k++) placed.push(el); return m.composeSprout(placed, r); }
        return m.composeSeedling(r, { H: 0, O: 0, C: 0, N: 0, ...counts });
      }, { v, s, band });
      const n0 = (v.result || {}).n || 0;
      await page.evaluate(({ slot, text }) => window.__bench.cmd(`try ${slot} ${text}`), { slot: s.slot, text });
      const r = (await until(async () => { const b = await bs(); return b.result && b.result.n > n0 ? b : null; })).result;
      results.push({ id: s.id, text, code: r.code });
    }
    const ids = spells.map((s) => s.id).join(',');
    check(`A ${band}: every composed build is ok (${ids})`, results.every((r) => r.code === 'ok') && (band === 'sprout' ? ids === 'water,frost,co2' : ids === 'fire,water,frost,co2'), results);
    await ctx.close();
  }
}

// B. scaleOf vs judge, 256 Fire combinations at Seedling.
if (!ONLY || ONLY.includes('B')) {
  const { ctx, page, bs, until } = await load('&band=seedling');
  await page.evaluate(() => { window.__bench.open(); window.__bench.cmd('pick 1'); });
  const v = await until(async () => { const b = await bs(); return b.recipe ? b : null; });
  const recipe = v.recipe;
  let bad = [], n = 0, oks = 0;
  for (let a = 1; a <= 4; a++) for (let b = 1; b <= 4; b++) for (let c = 1; c <= 4; c++) for (let d = 1; d <= 4; d++) {
    const coefs = [a, b, c, d];
    const { text, sc } = await page.evaluate(async ({ recipe, coefs }) => { const m = await import('./bench.js'); return { text: m.composeEquation(recipe, coefs), sc: m.scaleOf(recipe, coefs) }; }, { recipe, coefs });
    await page.evaluate((t) => window.__bench.cmd(`try 1 ${t}`), text);
    n++;
    const r = (await until(async () => { const x = await bs(); return x.result && x.result.n === n ? x : null; }, 20000))?.result;
    if (!r) { bad.push({ coefs, why: 'no result' }); continue; }
    if (r.ok) oks++;
    const expDiffs = Object.keys(sc.left).sort().filter((el) => sc.left[el] !== sc.right[el]).map((el) => [el, sc.left[el], sc.right[el]]);
    const agree = sc.level ? r.ok : (r.code === 'unbalanced' && JSON.stringify(r.diffs) === JSON.stringify(expDiffs));
    if (!agree) bad.push({ coefs, sc, r });
  }
  check('B scaleOf agrees with judge for all 256 Fire coefficient sets', bad.length === 0, `n=${n} ok=${oks} bad=${JSON.stringify(bad.slice(0, 3))}`);
  await ctx.close();
}

// C, D, E. Key leak, no re-learn, write counts.
if (!ONLY || ONLY.includes('C')) {
  const { ctx, page, st, bs, frames, until, errors } = await load('&band=apprentice');
  await page.evaluate(() => {
    window.__spy = { doc: 0, canvas: 0, writes: 0 };
    document.addEventListener('keydown', () => window.__spy.doc++);
    document.getElementById('game').addEventListener('keydown', () => window.__spy.canvas++, true);
    let v = window.__benchState;
    Object.defineProperty(window, '__benchState', { configurable: true, get: () => v, set: (x) => { window.__spy.writes++; v = x; } });
  });
  await frames(5);
  await page.evaluate(() => { window.__spy.writes = 0; });
  await frames(60);
  const closedWrites = await page.evaluate(() => window.__spy.writes);
  check('E closed and still: at most 1 __benchState write in 60 frames', closedWrites <= 1, `writes=${closedWrites}`);

  // lock, then B
  await page.mouse.click(640, 360);
  await until(() => page.evaluate(() => !!document.pointerLockElement), 5000);
  const locked = await page.evaluate(() => !!document.pointerLockElement);
  await page.evaluate(() => { window.__spy.writes = 0; window.__spy.doc = 0; window.__spy.canvas = 0; });
  await page.keyboard.press('KeyB');
  await until(async () => (await page.evaluate(() => window.__bench.ui().open)));
  await page.click('.bench-spell[data-slot="3"]');
  await until(() => page.evaluate(() => document.activeElement && document.activeElement.id === 'bench-input'));
  const s0 = await st();
  await page.keyboard.type('Br2 R 3 P 1 4');
  // keys with focus on a button, not the box
  await page.focus('.bench-make');
  for (const k of ['Digit2', 'KeyR', 'KeyP', 'Space', 'KeyE', 'KeyB', 'Digit4']) await page.keyboard.press(k);
  await page.focus('#bench-input');
  await frames(10);
  const s1 = await st();
  const spy = await page.evaluate(() => window.__spy);
  const val = await page.evaluate(() => document.getElementById('bench-input').value);
  check('C no key reaches winit (canvas) or hud.js (document) while open', spy.doc === 0 && spy.canvas === 0 && s1.spell === s0.spell && s1.captionId === s0.captionId && s1.gameTime === s0.gameTime && s1.paused && (await page.evaluate(() => window.__bench.ui().open)), { locked, spy, val, spell: [s0.spell, s1.spell], cap: [s0.captionId, s1.captionId], gt: [s0.gameTime, s1.gameTime] });
  check('C typing b/B in the box does not toggle the bench', val.includes('Br2'), `value=${JSON.stringify(val)}`);
  // write count while open with 3 tries
  await page.fill('#bench-input', '');
  await frames(3);
  await page.evaluate(() => { window.__spy.writes = 0; });
  for (const t of ['N2', 'N2(g)', 'Xx']) { await page.fill('#bench-input', t); await page.keyboard.press('Enter'); await frames(3); }
  await frames(60 - 9);
  const openWrites = await page.evaluate(() => window.__spy.writes);
  check('E open on a build view, 3 tries in 60 frames: at most 3 writes', openWrites <= 3, `writes=${openWrites}`);
  // D. learn frost, then five more
  await page.fill('#bench-input', 'N2(l)'); await page.keyboard.press('Enter');
  await until(async () => (await bs()).result?.ok);
  const rs = [];
  for (let i = 0; i < 5; i++) {
    await page.click('.bench-again'); await page.click('.bench-spell[data-slot="3"]');
    await until(() => page.evaluate(() => !!document.getElementById('bench-input')));
    const n0 = (await bs()).result?.n || 0;
    await page.fill('#bench-input', 'N2(l)'); await page.keyboard.press('Enter');
    const r = (await until(async () => { const b = await bs(); return b.result && b.result.n > n0 ? b.result : null; }));
    rs.push(r && `${r.code}/${r.learned}`);
  }
  await frames(3);
  const p = (await st()).progress[2];
  check('D five more correct builds never re-learn (xp stays 10)', p.st === 'learning' && p.xp === 10 && rs.every((x) => x === 'ok/false'), { p, rs });
  // close via Back to the game: relock asked, unpaused
  await page.click('.bench-leave');
  await frames(10);
  const after = await st();
  check('C closing restores pause state and inert', !after.paused && (await page.evaluate(() => ['stage', 'hud', 'loading', 'pause'].every((id) => !document.getElementById(id).inert))), { paused: after.paused, lock: await page.evaluate(() => !!document.pointerLockElement) });
  check('C no page/console errors', errors.length === 0, errors.slice(0, 3).join(' | '));
  await ctx.close();
}

// F. Age picker shown before pointer lock (analytics on via served meta).
if (!ONLY || ONLY.includes('F')) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  await ctx.route('**/api/**', (r) => r.fulfill({ status: 204, body: '' }));
  const page = await ctx.newPage();
  await page.goto(`${BASE}?low&debug&picker`);
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState && !!window.__bench));) { if (Date.now() - t0 > 120000) break; await sleep(250); }
  await sleep(1500);
  const shown = await page.evaluate(() => { const a = document.getElementById('age'); return !!a && !a.hidden && a.getBoundingClientRect().width > 0; });
  await page.keyboard.press('KeyB');
  await sleep(500);
  const openUnder = await page.evaluate(() => window.__bench.ui().open);
  const btnClick = await page.evaluate(() => { document.getElementById('benchbtn').click(); return window.__bench.ui().open; });
  check('F B and #benchbtn do nothing while the age picker shows', shown && !openUnder && !btnClick, { shown, openUnder, btnClick });
  if (shown) {
    await page.click('#age button[data-band="seedling"]');
    await sleep(800);
    const b0 = await page.evaluate(() => JSON.parse(window.__benchState).band);
    await page.keyboard.press('KeyB');
    await sleep(1500);
    const ui = await page.evaluate(() => ({ ...window.__bench.ui(), chooser: document.querySelectorAll('#bench .bench-band').length, cards: document.querySelectorAll('#bench .bench-spell').length }));
    check('F picker pick reaches Rust closed and the bench opens on Seedling list', b0 === 'seedling' && ui.open && ui.chooser === 0 && ui.cards === 4, { b0, ui });
    // Escape, then lock should still work
    await page.keyboard.press('Escape');
    await sleep(500);
    await page.mouse.click(640, 360);
    await sleep(800);
    check('F after picker + bench, a click still locks the pointer', await page.evaluate(() => !!document.pointerLockElement && !window.__bench.ui().open));
  }
  await ctx.close();
}

// G. Hint gating and H kid playthrough (Seedling Fire with the mouse), then cast.
if (!ONLY || ONLY.includes('G')) {
  const { ctx, page, st, bs, frames, until, errors } = await load('');
  const hint = () => page.evaluate(() => document.getElementById('benchbtn').classList.contains('hint'));
  await frames(10);
  const h0 = await hint();
  // H: open via button, chooser, Seedling, Fire on the scale
  await page.click('#benchbtn');
  const chooser = await until(() => page.evaluate(() => document.querySelectorAll('#bench .bench-band').length));
  await page.click('#bench .bench-band[data-band="seedling"]');
  await until(() => page.evaluate(() => document.querySelectorAll('#bench .bench-spell').length === 4));
  await page.click('.bench-spell[data-slot="1"]');
  await until(() => page.evaluate(() => !!document.querySelector('.bench-scale')));
  const lvl0 = await page.getAttribute('.bench-scale', 'data-level');
  await page.click('.bench-make');
  await until(async () => (await bs()).result);
  const r1 = (await bs()).result;
  const reason = await page.evaluate(() => document.querySelector('.bench-reason')?.textContent);
  await page.click('.bench-coef[data-term="1"]'); await page.click('.bench-coef[data-term="3"]');
  const lvl1 = await page.getAttribute('.bench-scale', 'data-level');
  const pans = await page.evaluate(() => [...document.querySelectorAll('.bench-pan-count')].map((e) => e.textContent));
  await page.click('.bench-make');
  await until(async () => (await bs()).result?.ok);
  await page.waitForSelector('.bench-done-line', { timeout: 10000 }).catch(() => {});
  const done = await page.evaluate(() => document.querySelector('.bench-done-line')?.textContent);
  await page.click('.bench-leave');
  await frames(10);
  const s = await st();
  check('H kid playthrough: button, chooser, Fire unbalanced then level, done, back', chooser === 3 && lvl0 === 'false' && r1.code === 'unbalanced' && lvl1 === 'true' && done === 'CH₄ + 2O₂ → CO₂ + 2H₂O is right! You are learning Fire.' && s.progress[0].st === 'learning' && !s.paused && !(await page.evaluate(() => window.__bench.ui().open)), { lvl0, r1: r1.code, reason, lvl1, pans, done, p: s.progress[0], paused: s.paused });
  await page.evaluate(() => window.__game.cmd('select 1'));
  await frames(3);
  const c0 = (await st()).casts;
  await sleep(400);
  await page.evaluate(() => window.__game.cmd('cast'));
  const c1 = await until(async () => { const x = await st(); return x.casts > c0 ? x.casts : null; });
  await frames(10);
  const h1 = await hint();
  check('H cast Fire after building it', c1 === c0 + 1, `casts ${c0} -> ${c1}`);
  check('G hint: off before any cast, never on once a spell is learning', !h0 && !h1, { h0, h1 });
  check('G/H no page/console errors', errors.length === 0, errors.slice(0, 3).join(' | '));
  await ctx.close();
  // hint appears with casts>=1 and nothing learned
  const L = await load('');
  await L.page.evaluate(() => { localStorage.removeItem('rg.progress.v1'); window.__game.cmd('select 2'); });
  await L.frames(3); await sleep(400);
  await L.page.evaluate(() => window.__game.cmd('cast'));
  const hOn = await L.until(() => L.page.evaluate(() => document.getElementById('benchbtn').classList.contains('hint')), 8000);
  check('G hint: on after the first cast with nothing learned', !!hOn);
  await L.ctx.close();
}

// I. Extra breaks: Change age mid-build; URL band vs Change age; Escape in chooser; reduce.
if (!ONLY || ONLY.includes('I')) {
  const { ctx, page, bs, until, frames } = await load('&band=sprout');
  await page.click('#benchbtn');
  await until(() => page.evaluate(() => document.querySelectorAll('.bench-spell').length));
  await page.click('.bench-change');
  await page.click('.bench-band[data-band="apprentice"]');
  await frames(5);
  const b = await bs();
  check('I (info) with ?band=sprout a child can switch to Apprentice via Change age', true, `rust band=${b.band} cards=${(b.spells || []).length}`);
  await page.keyboard.press('Escape');
  await frames(5);
  await page.keyboard.press('KeyB');
  await frames(5);
  const ui = await page.evaluate(() => window.__bench.ui());
  check('I (info) band after reopen with URL band', true, JSON.stringify(ui));
  // Escape on chooser closes
  await page.click('.bench-change');
  await page.keyboard.press('Escape');
  await frames(5);
  check('I Escape on the chooser closes the bench', !(await page.evaluate(() => window.__bench.ui().open)));
  await ctx.close();
}

await browser.close();
server.close();
const fails = out.filter((r) => !r.ok).length;
console.log(`RESULT ${out.length - fails}/${out.length}`);
process.exit(fails ? 1 : 0);
