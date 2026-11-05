// node crates/gfx-lab/bench-preview/shoot.mjs <outDir> [label]
// Serves the repo root and captures the Bench scene: the plate with ethanol,
// and the chamber mid-flame (methane burning).
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { extname, join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const { chromium } = createRequire(import.meta.url)('playwright');
const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const out = process.argv[2]; const label = process.argv[3] || 'shot';
const types = { '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json', '.css': 'text/css' };
const server = createServer(async (req, res) => {
  try { const p = decodeURIComponent(new URL(req.url, 'http://x').pathname); res.writeHead(200, { 'content-type': types[extname(p)] || 'application/octet-stream' }); res.end(await readFile(join(repo, p))); } catch { res.writeHead(404); res.end(); }
});
await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
const base = `http://127.0.0.1:${server.address().port}/crates/gfx-lab/bench-preview/index.html`;
await mkdir(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--ignore-gpu-blocklist'] });
for (const [step, wait, extra] of [['plate', 2500, ''], ['reaction', 9000, ''], ['plate', 2500, '&tier=low']]) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 });
  const logs = []; page.on('console', (m) => { if (/error|warn/i.test(m.type())) logs.push(m.text()); }); page.on('pageerror', (e) => logs.push(String(e)));
  await page.goto(`${base}?step=${step}&wait=${wait}${extra}`);
  await page.waitForFunction(() => window.__ready, null, { timeout: 60000 });
  const name = `${label}-${step}${extra ? '-low' : ''}.png`;
  await page.screenshot({ path: join(out, name) });
  console.log(name, logs.slice(0, 5).join(' | '));
  await page.close();
}
await browser.close(); server.close();
