// I-028 breaker round 2 follow-up (N3 harness fix; break-1 section I without a URL band). Usage: node brk.mjs <pkg-out dir>
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

const lastN = async (bs) => (await bs()).result?.n || 0;
const nextResult = async (bs, until, n0, ms = 15000) => until(async () => { const b = await bs(); return b.result && b.result.n > n0 ? b.result : null; }, ms);
// N3 (fixed harness: the Seedling grid has one tile per element, tap the N tile).
{
  const { ctx, page, bs, until } = await load('&band=seedling');
  await page.click('#benchbtn');
  await until(() => page.evaluate(() => document.querySelectorAll('.bench-spell').length > 0));
  await page.click('.bench-spell[data-slot="3"]');
  await until(() => page.evaluate(() => !!document.querySelector('.bench-tile')));
  const tiles = await page.evaluate(() => [...document.querySelectorAll('.bench-tile')].map((t) => t.getAttribute('data-el')).join(','));
  for (let i = 0; i < 7; i++) await page.click('.bench-tile[data-el="N"]');
  const cnt = await page.evaluate(() => document.querySelector('.bench-tile[data-el="N"] .bench-tile-count').textContent);
  let n0 = await lastN(bs);
  await page.click('.bench-make');
  let r = await nextResult(bs, until, n0);
  check('N3 Seedling Frost: N tile tapped 7 times wraps to 2 and is right', cnt === '2' && r && r.ok, JSON.stringify({ tiles, cnt, r }));
  // All five tiles at 4 (20 atoms), then Make: a fizzle with a reason, then back to 0 and Make.
  await page.click('.bench-again').catch(() => {});
  await until(() => page.evaluate(() => !!document.querySelector('.bench-spell')));
  await page.click('.bench-spell[data-slot="3"]');
  await until(() => page.evaluate(() => !!document.querySelector('.bench-tile')));
  await page.evaluate(() => { for (const t of document.querySelectorAll('.bench-tile')) for (let i = 0; i < 4; i++) t.click(); });
  n0 = await lastN(bs);
  await page.click('.bench-make');
  r = await nextResult(bs, until, n0);
  await until(() => page.evaluate(() => !!document.querySelector('.bench-reason')), 3000);
  const reason = await page.evaluate(() => document.querySelector('.bench-reason')?.textContent);
  check('N3 Seedling Frost all tiles at 4: fizzle with a non-empty reason', r && !r.ok && reason && reason.trim(), JSON.stringify({ code: r?.code, diffs: r?.diffs, reason }));
  await ctx.close();
}
// Break-1 section I, adapted: Change age now hidden under a URL band, so run without one.
{
  const { ctx, page, bs, until, frames } = await load('');
  await page.click('#benchbtn');
  await until(() => page.evaluate(() => !!document.querySelector('.bench-band')));
  await page.click('.bench-band[data-band="sprout"]');
  await until(() => page.evaluate(() => document.querySelectorAll('.bench-spell').length > 0));
  await page.click('.bench-spell[data-slot="2"]');
  await until(() => page.evaluate(() => !!document.querySelector('.bench-make')));
  await page.click('.bench-change');
  await page.click('.bench-band[data-band="apprentice"]');
  await frames(5);
  const b = await bs();
  const ui = await page.evaluate(() => window.__bench.ui());
  check('I\' Change age mid-build (no URL band) reaches Rust and shows the new list', b.band === 'apprentice' && ui.band === 'apprentice' && ui.view === 'list', JSON.stringify({ rust: b.band, ui }));
  await page.click('.bench-change');
  await page.keyboard.press('Escape');
  await frames(5);
  check('I\' Escape on the chooser closes the bench', !(await page.evaluate(() => window.__bench.ui().open)));
  await ctx.close();
}
await browser.close();
server.close();
const fails = out.filter((r) => !r.ok).length;
console.log(`RESULT ${out.length - fails}/${out.length}`);
process.exit(fails ? 1 : 0);
