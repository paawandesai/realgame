// I-044 round 2: the age picker shows before pointer lock on a site-like copy
// (pkg-out + age.js + analytics.js, rg-analytics on), as build-site.sh makes it.
import { createServer } from 'node:http';
import { readFileSync, existsSync } from 'node:fs';
import { join, extname } from 'node:path';
import { createRequire } from 'node:module';
const require = createRequire('/home/claude/realgame/.claude/worktrees/w10-I-044/tests/e2e/golden.mjs');
const { chromium } = require('playwright');
const ROOT = process.argv[2];
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
const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 1366, height: 768 } });
const page = await ctx.newPage();
const out = {};
out.dnt = await page.evaluate(() => [navigator.doNotTrack, navigator.globalPrivacyControl]).catch(() => null);
await page.goto(`http://127.0.0.1:${port}/?low&debug`);
await page.waitForSelector('#age', { state: 'visible', timeout: 120000 }).catch(() => undefined);
out.dnt = await page.evaluate(() => [navigator.doNotTrack, navigator.globalPrivacyControl ?? null]);
out.visible = await page.evaluate(() => { const e = document.getElementById('age'); return !!e && !e.hidden && getComputedStyle(e).display !== 'none'; });
out.lockedWhilePicker = await page.evaluate(() => !!document.pointerLockElement);
out.buttons = await page.evaluate(() => [...document.querySelectorAll('#age button')].map((b) => b.dataset.band));
out.focused = await page.evaluate(() => document.activeElement?.dataset?.band ?? document.activeElement?.id);
if (out.visible) {
  await page.click('#age button[data-band="seedling"]');
  out.hiddenAfter = await page.evaluate(() => document.getElementById('age').hidden === true);
  // The game takes the pointer after the pick: click the canvas.
  await page.mouse.click(683, 384);
  await page.waitForTimeout(1500);
  out.lockedAfterClick = await page.evaluate(() => !!document.pointerLockElement || !!(window.__game && window.__game.state().locked));
}
console.log(JSON.stringify(out));
await browser.close(); srv.close();
const ok = out.visible && !out.lockedWhilePicker && out.buttons?.length === 3 && out.hiddenAfter;
console.log(ok ? 'PASS picker shows before pointer lock and closes on a pick' : 'FAIL picker');
process.exit(ok ? 0 : 1);
