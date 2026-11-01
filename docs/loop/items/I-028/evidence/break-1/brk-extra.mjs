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


// J. Extra breaks found by reading: a flat beam on an unbalanced Fire; an empty
// Apprentice reason; a zero coefficient judged ok.
{
  const { ctx, page, bs, until, frames, errors } = await load('&band=seedling');
  await page.click('#benchbtn');
  await until(() => page.evaluate(() => document.querySelectorAll('.bench-spell').length === 4));
  await page.click('.bench-spell[data-slot="1"]');
  await until(() => page.evaluate(() => !!document.querySelector('.bench-scale')));
  // Kid taps O2 (1->2), then CO2 by mistake (1->2): CH4 + 2O2 -> 2CO2 + H2O.
  await page.click('.bench-coef[data-term="1"]'); await page.click('.bench-coef[data-term="2"]');
  const sc = await page.evaluate(() => { const s = document.querySelector('.bench-scale'); const b = document.querySelector('.bench-beam'); return { level: s.getAttribute('data-level'), tilt: s.getAttribute('data-tilt'), transform: getComputedStyle(b).transform, pans: [...document.querySelectorAll('.bench-pan-count')].map((e) => e.textContent), more: [...document.querySelectorAll('.bench-pan-more')].map((e) => e.textContent), mark: document.querySelector('.bench-level')?.textContent } });
  await page.click('.bench-make');
  const r = (await until(async () => (await bs()).result))?.result;
  await page.waitForSelector('.bench-reason', { timeout: 5000 }).catch(() => {});
  const reason = await page.evaluate(() => document.querySelector('.bench-reason')?.textContent);
  await page.screenshot({ path: process.env.SHOT || '/tmp/claude-0/flat-beam.png' });
  check('J beam is drawn tilted whenever the scale is not level ([1,2,2,1])', !(sc.level === 'false' && sc.tilt === '0'), { sc, code: r && r.code, reason });
  await ctx.close();
}
{
  const { ctx, page, bs, until } = await load('&band=apprentice');
  await page.click('#benchbtn');
  await until(() => page.evaluate(() => document.querySelectorAll('.bench-spell').length === 4));
  await page.click('.bench-spell[data-slot="3"]');
  await until(() => page.evaluate(() => document.activeElement && document.activeElement.id === 'bench-input'));
  const lines = [];
  for (let i = 0; i < 3; i++) {
    const n0 = (await bs()).result?.n || 0;
    await page.fill('#bench-input', 'N2O0(l)'); await page.keyboard.press('Enter');
    const r = await until(async () => { const b = await bs(); return b.result && b.result.n > n0 ? b.result : null; });
    await until(() => page.evaluate(() => !!document.querySelector('.bench-reason')), 3000);
    lines.push({ code: r?.code, diffs: r?.diffs, reason: await page.evaluate(() => document.querySelector('.bench-reason')?.textContent) });
  }
  check('J N2O0(l) three times: every fizzle has a non-empty reason', lines.every((l) => l.reason && l.reason.trim()), lines);
  const n0 = (await bs()).result?.n || 0;
  await page.fill('#bench-input', '0N2(l)'); await page.keyboard.press('Enter');
  const r0 = await until(async () => { const b = await bs(); return b.result && b.result.n > n0 ? b.result : null; });
  await page.waitForSelector('.bench-done-line, .bench-reason', { timeout: 5000 }).catch(() => {});
  const shown = await page.evaluate(() => document.querySelector('.bench-done-line')?.textContent || document.querySelector('.bench-reason')?.textContent);
  check('J 0N2(l) (zero molecules) is not judged ok', r0 && !r0.ok, { r0, shown });
  await ctx.close();
}
await browser.close();
server.close();
const fails = out.filter((r) => !r.ok).length;
console.log(`RESULT ${out.length - fails}/${out.length}`);
process.exit(fails ? 1 : 0);
