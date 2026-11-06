// Graphics G0 feature matrix: loads gfx-lab once per (API, feature) in
// headless Chromium and records whether the page renders, what Bevy and wgpu
// log, the adapter and frame times, plus one screenshot per run.
//
//   node crates/gfx-lab/run-matrix.mjs [outDir]       (after ./build-web.sh)
//   FEATURES=base,ssao APIS=webgl2 node ...            a subset
//
// The cloud has no GPU: WebGL2 runs on SwiftShader and WebGPU on
// SwiftShader's Vulkan, so times are relative only. Results go to
// <outDir>/matrix.json and <outDir>/<api>-<feature>.png.
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { extname, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// CommonJS resolution, so a globally installed Playwright (NODE_PATH) works.
const { chromium } = createRequire(import.meta.url)('playwright');
const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, 'web', 'pkg-out');
const out = process.argv[2] || join(here, 'matrix-out');
const ALL = ['base', 'msaa', 'fxaa', 'smaa', 'taa', 'bloom', 'grading', 'shadows', 'transmission',
  'clearcoat', 'envmap_cpu', 'envmap_gen', 'depth_prepass', 'ssao', 'contact_shadows', 'ssr',
  'volfog', 'dof', 'distance_fog'];
const FEATURES = (process.env.FEATURES || ALL.join(',')).split(',');
const APIS = (process.env.APIS || 'webgl2,webgpu').split(',');
const TIMEOUT = Number(process.env.TIMEOUT_MS || 90000);

const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm' };
const server = createServer(async (req, res) => {
  try {
    let p = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    if (p === '/') p = '/index.html';
    const body = await readFile(join(root, p));
    res.writeHead(200, { 'content-type': types[extname(p)] || 'application/octet-stream' });
    res.end(body);
  } catch {
    res.writeHead(404); res.end();
  }
});
await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
const base = `http://127.0.0.1:${server.address().port}/`;
await mkdir(out, { recursive: true });

const browser = await chromium.launch({
  args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan,WebGPU', '--use-angle=swiftshader',
    '--use-webgpu-adapter=swiftshader', '--ignore-gpu-blocklist'],
});
const rows = [];
for (const api of APIS) {
  for (const feature of FEATURES) {
    const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
    const logs = [];
    page.on('console', (m) => logs.push(`${m.type()}: ${m.text()}`));
    page.on('pageerror', (e) => logs.push(`pageerror: ${e.message}`));
    const t0 = Date.now();
    let result = null;
    try {
      await page.goto(`${base}?api=${api}&f=${feature}`);
      await page.waitForFunction(() => window.__gfxlab, null, { timeout: TIMEOUT });
      result = JSON.parse(await page.evaluate(() => window.__gfxlab));
    } catch (e) {
      result = { error: `timeout or crash: ${String(e).split('\n')[0]}` };
    }
    await page.screenshot({ path: join(out, `${api}-${feature}.png`) }).catch(() => {});
    const interesting = logs.filter((l) => /error|warn|panic|not supported|lacks|unsupported|requires/i.test(l))
      .map((l) => l.slice(0, 400));
    const row = { api, feature, wall_ms: Date.now() - t0, ...result, issues: [...new Set(interesting)].slice(0, 12) };
    rows.push(row);
    console.log(`${api.padEnd(7)} ${feature.padEnd(16)} ${result.error ? 'FAIL ' + result.error : `p50 ${result.p50_ms} ms`}  issues ${row.issues.length}`);
    await page.close();
  }
}
await browser.close();
server.close();
await writeFile(join(out, 'matrix.json'), JSON.stringify(rows, null, 2));
console.log(`wrote ${join(out, 'matrix.json')}`);
