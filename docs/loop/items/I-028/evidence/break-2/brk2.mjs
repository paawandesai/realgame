// I-028 breaker round 2 checks. Usage: node brk.mjs <pkg-out dir>
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

const ready = (page) => page.evaluate(() => !!document.querySelector('.bench-spell'));
const openList = async (page, until) => { await page.click('#benchbtn'); await until(() => page.evaluate(() => document.querySelectorAll('.bench-spell').length > 0)); };
const lastN = async (bs) => (await bs()).result?.n || 0;
const nextResult = async (bs, until, n0, ms = 15000) => until(async () => { const b = await bs(); return b.result && b.result.n > n0 ? b.result : null; }, ms);

// K1. B1 re-run: every equal-total unbalanced Fire set is drawn tilted, marked, with no "more atoms".
if (!ONLY || ONLY.includes('K1')) {
  const { ctx, page, until } = await load('&band=seedling');
  await openList(page, until);
  await page.click('.bench-spell[data-slot="1"]');
  await until(() => page.evaluate(() => !!document.querySelector('.bench-scale')));
  const recipe = (await page.evaluate(() => JSON.parse(window.__benchState))).recipe;
  // All 256 via the module: level <=> tilt 0; and the 9 equal-total sets.
  const all = await page.evaluate(async (recipe) => {
    const m = await import('./bench.js'); const rows = [];
    for (let a = 1; a <= 4; a++) for (let b = 1; b <= 4; b++) for (let c = 1; c <= 4; c++) for (let d = 1; d <= 4; d++) {
      const s = m.scaleOf(recipe, [a, b, c, d]); const sum = (p) => Object.values(p).reduce((x, y) => x + y, 0);
      rows.push({ k: `${a}${b}${c}${d}`, level: s.level, tilt: s.tilt, eq: sum(s.left) === sum(s.right) });
    }
    return rows;
  }, recipe);
  const eqSets = all.filter((r) => r.eq && !r.level).map((r) => r.k);
  check('K1 scaleOf: level iff tilt 0 for all 256 sets', all.every((r) => r.level === (r.tilt === 0)), `n=${all.length} equalTotalUnbalanced=${eqSets.join(',')}`);
  const bad = [];
  for (const k of eqSets) {
    await page.click('.bench-back'); await until(() => ready(page));
    await page.click('.bench-spell[data-slot="1"]');
    await until(() => page.evaluate(() => !!document.querySelector('.bench-scale')));
    for (let i = 0; i < 4; i++) for (let j = 1; j < +k[i]; j++) await page.click(`.bench-coef[data-term="${i}"]`);
    const d = await page.evaluate(() => {
      const s = document.querySelector('.bench-scale'); const b = document.querySelector('.bench-beam');
      const pans = [...document.querySelectorAll('.bench-pan-count')];
      return { coefs: [...document.querySelectorAll('.bench-coef')].map((c) => c.textContent).join(''), level: s.getAttribute('data-level'), tilt: s.getAttribute('data-tilt'), transform: getComputedStyle(b).transform,
        more: [...document.querySelectorAll('.bench-pan-more')].map((e) => e.textContent),
        marked: pans.map((p) => [...p.querySelectorAll('.bench-pan-el[data-ok="false"]')].map((e) => e.getAttribute('data-el')).join('')),
        afterW: pans.map((p) => [...p.querySelectorAll('.bench-pan-el[data-ok="false"]')].map((e) => getComputedStyle(e, '::after').width).join(',')),
        text: pans.map((p) => p.textContent), levelText: document.querySelector('.bench-level')?.textContent };
    });
    const ok = d.coefs === k && d.level === 'false' && d.tilt !== '0' && d.transform !== 'none' && d.transform !== 'matrix(1, 0, 0, 1, 0, 0)' && d.more.every((m) => m === '') && d.marked[0] && d.marked[0] === d.marked[1] && !d.levelText;
    if (!ok) bad.push(d);
    if (k === '1221') check('K1 (detail) 1221 as drawn', true, JSON.stringify(d));
  }
  check('K1 all 9 equal-total sets drawn tilted, marked on both pans, no "more atoms", no Level', bad.length === 0 && eqSets.length === 9, JSON.stringify(bad.slice(0, 2)));
  // The solved pose is still flat and Level.
  await page.click('.bench-back'); await until(() => ready(page));
  await page.click('.bench-spell[data-slot="1"]'); await until(() => page.evaluate(() => !!document.querySelector('.bench-scale')));
  await page.click('.bench-coef[data-term="1"]'); await page.click('.bench-coef[data-term="3"]');
  const solved = await page.evaluate(() => ({ level: document.querySelector('.bench-scale').getAttribute('data-level'), tilt: document.querySelector('.bench-scale').getAttribute('data-tilt'), marks: document.querySelectorAll('.bench-pan-el[data-ok="false"]').length, txt: document.querySelector('.bench-level').textContent }));
  check('K1 1212 is level, flat, unmarked', solved.level === 'true' && solved.tilt === '0' && solved.marks === 0 && solved.txt === 'Level', JSON.stringify(solved));
  await ctx.close();
}

// K2. S1/S2 re-run in the real build: zero subscripts and zero coefficients.
if (!ONLY || ONLY.includes('K2')) {
  const { ctx, page, bs, until } = await load('&band=apprentice');
  await openList(page, until);
  const typed = async (slot, text) => {
    if (!(await page.evaluate((s) => window.__bench.ui().pick === s && !!document.querySelector('#bench-input'), slot))) {
      if (await page.$('.bench-back')) { await page.click('.bench-back'); await until(() => ready(page)); }
      if (await page.$('.bench-again')) { await page.click('.bench-again'); await until(() => ready(page)); }
      await page.click(`.bench-spell[data-slot="${slot}"]`);
      await until(() => page.evaluate(() => !!document.querySelector('#bench-input')));
    }
    const n0 = await lastN(bs);
    await page.fill('#bench-input', text); await page.keyboard.press('Enter');
    const r = await nextResult(bs, until, n0);
    await until(() => page.evaluate(() => !!document.querySelector('.bench-reason, .bench-done-line')), 3000);
    const shown = await page.evaluate(() => (document.querySelector('.bench-done-line') || document.querySelector('.bench-reason'))?.textContent);
    return { text, code: r?.code, ok: r?.ok, bad: r?.bad, diffs: r?.diffs, stage: r?.stage, shown };
  };
  const rows = [];
  for (let i = 0; i < 3; i++) rows.push(await typed(3, 'N2O0(l)'));
  for (const t of ['0N2(l)', '00N2(l)', '0/1N2(l)', '0.0N2(l)', '₀N₂(l)']) rows.push(await typed(3, t));
  rows.push(await typed(4, 'CO2H0(g)'));
  rows.push(await typed(1, '0CH4 + 2O2 -> CO2 + 2H2O'));
  rows.push(await typed(1, 'CH4 + 2O2 → 0CO2 + 2H2O'));
  rows.push(await typed(1, 'CH4 + 2O2 -> CO2 + 2H2O0'));
  for (const r of rows) console.log('   ', JSON.stringify(r));
  check('K2 every zero subscript/coefficient is a fizzle with a non-empty reason, nothing learned', rows.every((r) => r.ok === false && r.code === 'parse' && r.shown && r.shown.trim() && r.stage !== 'learning'), '');
  const st = (await bs()).spells.map((s) => `${s.slot}:${s.stage}`).join(' ');
  check('K2 Frost, CO2 and Fire still unknown after the zero tries', !/[134]:learning/.test(st), st);
  // Positive controls.
  const pos = [await typed(3, '10N2(l)'), await typed(2, 'H2O1(l)')];
  check('K2 controls 10N2(l) and H2O1(l) are right', pos.every((r) => r.ok), JSON.stringify(pos));
  await ctx.close();
  // Sprout and Seedling 0H2O through the command channel.
  for (const band of ['sprout', 'seedling']) {
    const L = await load(`&band=${band}`);
    await L.page.evaluate(() => { window.__bench.open(); window.__bench.cmd('pick 2'); });
    await L.until(async () => (await L.bs()).recipe);
    const n0 = await lastN(L.bs);
    await L.page.evaluate(() => window.__bench.cmd('try 2 0H2O'));
    const r = await nextResult(L.bs, L.until, n0);
    check(`K2 ${band} 0H2O is not ok`, r && !r.ok, JSON.stringify(r));
    await L.ctx.close();
  }
}

// K3. S5 re-run: a URL band fixes the band; odd URL bands do not strand the child.
if (!ONLY || ONLY.includes('K3')) {
  {
    const { ctx, page, bs, until, frames } = await load('&band=sprout');
    await openList(page, until);
    const d = await page.evaluate(() => ({ hidden: document.querySelector('.bench-change').hidden, disp: getComputedStyle(document.querySelector('.bench-change')).display }));
    // Tab through the whole panel: focus never lands on the hidden button.
    const seen = [];
    for (let i = 0; i < 12; i++) { await page.keyboard.press('Tab'); seen.push(await page.evaluate(() => document.activeElement?.className || '')); }
    // A scripted click on the hidden button (a stale reference) still cannot change the band.
    await page.evaluate(() => document.querySelector('.bench-change').click());
    const chooser = await page.evaluate(() => !!document.querySelector('.bench-band'));
    if (chooser) { await page.click('.bench-band[data-band="apprentice"]'); await frames(5); }
    const b = await bs();
    check('K3 ?band=sprout: Change age hidden, never focused, band stays sprout', d.hidden && d.disp === 'none' && !seen.includes('bench-change') && b.band === 'sprout', JSON.stringify({ d, chooserViaScript: chooser, band: b.band, ui: await page.evaluate(() => window.__bench.ui()) }));
    await ctx.close();
  }
  for (const q of ['&band=Sprout', '&band=bogus', '&band=']) {
    const { ctx, page, until, frames } = await load(q);
    await page.click('#benchbtn');
    await until(() => page.evaluate(() => !!document.querySelector('.bench-band, .bench-spell')));
    const first = await page.evaluate(() => ({ chooser: !!document.querySelector('.bench-band'), list: !!document.querySelector('.bench-spell'), change: !document.querySelector('.bench-change').hidden }));
    if (first.chooser) { await page.click('.bench-band[data-band="seedling"]'); await until(() => ready(page)); }
    const ui = await page.evaluate(() => window.__bench.ui());
    check(`K3 ${q || 'none'}: the child reaches a spell list`, ui.view === 'list' && !!ui.band, JSON.stringify({ first, ui }));
    await ctx.close();
  }
}

// K3b. URL band plus the age picker (site copy with age.js; analytics on via ?picker).
if (!ONLY || ONLY.includes('K3b')) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  await ctx.route('**/api/**', (r) => r.fulfill({ status: 204, body: '' }));
  const page = await ctx.newPage();
  await page.goto(`${BASE}?low&debug&picker&band=sprout`);
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState && !!window.__bench));) { if (Date.now() - t0 > 120000) break; await sleep(250); }
  await sleep(1500);
  const shown = await page.evaluate(() => { const a = document.getElementById('age'); return !!a && !a.hidden && a.getBoundingClientRect().width > 0; });
  if (shown) { await page.click('#age button[data-band="apprentice"]'); await sleep(800); }
  await page.evaluate(() => document.getElementById('benchbtn').click());
  await sleep(1500);
  const d = await page.evaluate(() => ({ ui: window.__bench.ui(), rust: JSON.parse(window.__benchState).band, change: !document.querySelector('.bench-change').hidden, cards: document.querySelectorAll('#bench .bench-spell').length, chooser: document.querySelectorAll('#bench .bench-band').length }));
  check('K3b ?band=sprout with the age picker: the bench opens on the sprout list', d.ui.open && d.ui.band === 'sprout' && d.rust === 'sprout' && d.cards > 0 && !d.chooser, JSON.stringify({ pickerShown: shown, ...d }));
  await ctx.close();
}

// K4. Pause card hidden while the bench is open, on every way in and out.
if (!ONLY || ONLY.includes('K4')) {
  const { ctx, page, st, until, frames } = await load('&band=apprentice');
  const vis = () => page.evaluate(() => { const p = document.getElementById('pause'); return { shown: !!p && getComputedStyle(p).display !== 'none' && !p.classList.contains('hidden'), under: !!p && p.classList.contains('bench-under'), paused: window.__game.state().paused }; });
  const rows = [];
  for (const [how, open, close] of [
    ['button/Close', () => page.click('#benchbtn'), () => page.click('.bench-close')],
    ['B/Escape', () => page.keyboard.press('KeyB'), () => page.keyboard.press('Escape')],
    ['api/api', () => page.evaluate(() => window.__bench.open()), () => page.evaluate(() => window.__bench.close())],
  ]) {
    await page.evaluate(() => document.getElementById('game').focus());
    await open(); await frames(3);
    const during = await vis();
    await close(); await frames(3);
    const after = await vis();
    rows.push({ how, during, after });
  }
  // Paused first (Escape pause card showing), then the bench from the button.
  await page.evaluate(() => window.__game.pause(true)); await frames(3);
  const before = await vis();
  await page.click('#benchbtn'); await frames(3);
  const during = await vis();
  await page.click('.bench-close'); await frames(3);
  const after = await vis();
  rows.push({ how: 'paused/button/Close', before, during, after });
  console.log('   ', JSON.stringify(rows));
  check('K4 #pause never shows while open; bench-under always removed on close; paused game gets its card back',
    rows.every((r) => !r.during.shown && !r.after.under) && after.shown && after.paused, '');
  await ctx.close();
}

// N1. New: a digit-zero subscript prefix (N02) or empty brackets judged right.
if (!ONLY || ONLY.includes('N1')) {
  const { ctx, page, bs, until } = await load('&band=apprentice');
  await openList(page, until);
  const rows = [];
  for (const [slot, t] of [[3, 'N02(l)'], [2, 'H02O(l)'], [3, 'N2()(l)'], [3, 'N()N(l)']]) {
    if (await page.$('.bench-again')) { await page.click('.bench-again'); await until(() => ready(page)); }
    if (await page.$('.bench-back')) { await page.click('.bench-back'); await until(() => ready(page)); }
    await page.click(`.bench-spell[data-slot="${slot}"]`);
    await until(() => page.evaluate(() => !!document.querySelector('#bench-input')));
    const n0 = await lastN(bs);
    await page.fill('#bench-input', t); await page.keyboard.press('Enter');
    const r = await nextResult(bs, until, n0);
    await until(() => page.evaluate(() => !!document.querySelector('.bench-reason, .bench-done-line')), 3000);
    rows.push({ t, ok: r?.ok, code: r?.code, learned: r?.learned, shown: await page.evaluate(() => [...document.querySelectorAll('.bench-done-line, .bench-done-extra, .bench-reason')].map((e) => e.textContent).join(' | ')) });
  }
  for (const r of rows) console.log('   ', JSON.stringify(r));
  check('N1 N02(l) (digit zero before 2) is not judged right', rows[0].ok === false, JSON.stringify(rows[0]));
  check('N1 H02O(l) is not judged right', rows[1].ok === false, JSON.stringify(rows[1]));
  check('N1 (info) empty brackets N2()(l) / N()N(l)', true, JSON.stringify(rows.slice(2)));
  await ctx.close();
}

// N2. New: Make, fix the scale, Make again before the first answer: the fix is dropped.
if (!ONLY || ONLY.includes('N2')) {
  const { ctx, page, bs, until, frames } = await load('&band=seedling');
  await openList(page, until);
  await page.click('.bench-spell[data-slot="1"]');
  await until(() => page.evaluate(() => !!document.querySelector('.bench-scale')));
  const n0 = await lastN(bs);
  // One synchronous burst: Make at 1-1-1-1, tap O2 and H2O (now level), Make.
  await page.evaluate(() => { const q = (s) => document.querySelector(s); q('.bench-make').click(); q('.bench-coef[data-term="1"]').click(); q('.bench-coef[data-term="3"]').click(); q('.bench-make').click(); });
  await nextResult(bs, until, n0);
  await frames(30);
  const d = await page.evaluate(() => ({ n: JSON.parse(window.__benchState).result?.n, code: JSON.parse(window.__benchState).result?.code, level: document.querySelector('.bench-scale')?.getAttribute('data-level'), reason: document.querySelector('.bench-reason')?.textContent || '', done: !!document.querySelector('.bench-done-line') }));
  check('N2 a fixed build sent before the first answer is judged (not dropped)', d.n === n0 + 2 || d.done, JSON.stringify({ n0, ...d }));
  check('N2 the panel never says "not level" under a level scale', !(d.level === 'true' && /not level/.test(d.reason)), JSON.stringify(d));
  // Does the child recover? One more Make.
  const n1 = d.n;
  await page.click('.bench-make');
  const r2 = await nextResult(bs, until, n1);
  await until(() => page.evaluate(() => !!document.querySelector('.bench-done-line')), 5000);
  check('N2 one more Make recovers (not stuck)', r2 && r2.ok, JSON.stringify(r2));
  await ctx.close();
}

// N3. New: empty builds and full trays: never a blank reason, never stuck.
if (!ONLY || ONLY.includes('N3')) {
  const rows = [];
  {
    const { ctx, page, bs, until } = await load('&band=sprout');
    await openList(page, until);
    await page.click('.bench-spell[data-slot="2"]');
    await until(() => page.evaluate(() => !!document.querySelector('.bench-make')));
    let n0 = await lastN(bs);
    await page.click('.bench-make');
    let r = await nextResult(bs, until, n0);
    await until(() => page.evaluate(() => !!document.querySelector('.bench-reason')), 3000);
    rows.push({ case: 'sprout empty tray', code: r?.code, reason: await page.evaluate(() => document.querySelector('.bench-reason')?.textContent) });
    // 12 balls (the cap), then 2 more taps, then Undo down to H2O.
    for (let i = 0; i < 14; i++) await page.click('.bench-atom[data-token="H"]');
    const tray = await page.evaluate(() => document.querySelector('.bench-tray').getAttribute('data-count'));
    for (let i = 0; i < 10; i++) await page.click('.bench-undo');
    await page.click('.bench-atom[data-token="O"]');
    n0 = await lastN(bs);
    await page.click('.bench-make');
    r = await nextResult(bs, until, n0);
    await until(() => page.evaluate(() => !!document.querySelector('.bench-done-line, .bench-reason')), 3000);
    rows.push({ case: 'sprout 14 taps, undo to H2 + O', trayAt14: tray, code: r?.code, ok: r?.ok, shown: await page.evaluate(() => (document.querySelector('.bench-done-line') || document.querySelector('.bench-reason'))?.textContent) });
    await ctx.close();
  }
  {
    const { ctx, page, bs, until } = await load('&band=seedling');
    await openList(page, until);
    await page.click('.bench-spell[data-slot="3"]');
    await until(() => page.evaluate(() => !!document.querySelector('.bench-tile')));
    let n0 = await lastN(bs);
    await page.click('.bench-make');
    let r = await nextResult(bs, until, n0);
    await until(() => page.evaluate(() => !!document.querySelector('.bench-reason')), 3000);
    rows.push({ case: 'seedling frost all tiles 0', code: r?.code, reason: await page.evaluate(() => document.querySelector('.bench-reason')?.textContent) });
    // N tile wraps 0..4 -> 0: 7 taps = 2.
    for (let i = 0; i < 7; i++) await page.click('.bench-tile');
    const cnt = await page.evaluate(() => document.querySelector('.bench-tile-count').textContent);
    n0 = await lastN(bs);
    await page.click('.bench-make');
    r = await nextResult(bs, until, n0);
    await until(() => page.evaluate(() => !!document.querySelector('.bench-done-line')), 3000);
    rows.push({ case: 'seedling frost N tile 7 taps (wraps to 2)', cnt, code: r?.code, ok: r?.ok });
    await ctx.close();
  }
  for (const r of rows) console.log('   ', JSON.stringify(r));
  check('N3 empty builds give a non-empty reason; caps and wraps never produce a wrong right or a stuck builder',
    rows[0].reason && rows[2].reason && rows[1].ok === true && rows[3].ok === true && rows[3].cnt === '2', '');
}

await browser.close();
server.close();
const fails = out.filter((r) => !r.ok).length;
console.log(`RESULT ${out.length - fails}/${out.length}`);
process.exit(fails ? 1 : 0);
