// Acceptance test 5: pkg-out on a plain static server sends no analytics and shows no picker.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { createRequire } from 'node:module';
const repo = process.argv[2];
const require = createRequire(join(repo, 'package.json'));
const { chromium } = require('playwright');
const root = join(repo, 'crates/bevy-client/web/pkg-out');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
const seen = [];
const server = createServer(async (req, res) => {
  const p = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '');
  seen.push(`${req.method} /${p}`);
  try { const b = await readFile(join(root, p || 'index.html')); res.writeHead(200, { 'content-type': types[extname(p || 'index.html')] || 'application/octet-stream' }); res.end(b); }
  catch { res.writeHead(404).end(); }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}/`;
const browser = await chromium.launch();
const page = await browser.newPage();
const reqs = []; const errors = [];
page.on('request', (r) => reqs.push(r.url()));
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
await page.goto(base + '?low&debug'); // debug only to win fast; analytics ignores it
while (!(await page.evaluate(() => !!window.__gameState))) await sleep(250);
const t0 = Date.now();
const picker = await page.evaluate(() => { const a = document.getElementById('age'); return !!a && !a.hidden && a.getBoundingClientRect().height > 0; });
await page.evaluate(() => window.__game.cmd('reset'));
const st = () => page.evaluate(() => window.__game.state());
const until = async (pred, ms) => { const t = Date.now(); let x = await st(); while (!pred(x) && Date.now() - t < ms) { await sleep(150); x = await st(); } return x; };
let s = await until((x) => x.outcome === 'playing' && x.burning === 3 && x.casts === 0, 20000);
const lit = s.objs.filter((o) => o.burning).map((o) => o.i);
await page.evaluate(() => window.__game.cmd('tp 0 5.5'));
await sleep(1000);
for (const i of lit) {
  await page.evaluate((i) => { window.__game.cmd(`face ${i}`); window.__game.cmd('select 2'); }, i);
  const a = await until((x) => x.spell === 1 && x.aimObj === i, 15000);
  await page.evaluate(() => window.__game.cmd('cast'));
  await until((x) => x.impacts > a.impacts, 60000);
}
s = await until((x) => x.outcome === 'won', 60000);
console.log(`${s.outcome === 'won' ? 'PASS' : 'FAIL'} won on pkg-out`);
await page.evaluate(() => { Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'hidden' }); document.dispatchEvent(new Event('visibilitychange')); });
while (Date.now() - t0 < 20000) await sleep(500);
await page.close({ runBeforeUnload: true });
await sleep(1000);
await browser.close(); server.close();
const api = reqs.filter((u) => u.includes('/api/')).concat(seen.filter((s) => s.includes('/api/')));
const mods = seen.filter((s) => /age\.js|analytics\.js/.test(s));
console.log(`${!picker ? 'PASS' : 'FAIL'} no picker on pkg-out`);
console.log(`${api.length === 0 ? 'PASS' : 'FAIL'} no /api/event request within 20 s of first state (after reset and hide) ${JSON.stringify(api)}`);
console.log(`${mods.length === 0 ? 'PASS' : 'FAIL'} age.js/analytics.js never fetched ${JSON.stringify(mods)}`);
console.log(`${errors.length === 0 ? 'PASS' : 'FAIL'} 0 console errors ${JSON.stringify(errors)}`);
process.exit(!picker && api.length === 0 && mods.length === 0 && errors.length === 0 ? 0 : 1);
