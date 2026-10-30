// End-to-end acceptance tests for I-013 (hardening tests/e2e/perf.mjs). Runs
// the perf script as a child process against a real web build and checks its
// JSON and exit codes against docs/loop/items/I-013/spec.md.
//
//   node docs/loop/items/I-013/lanes/main/accept.mjs [WEB_DIR] [--only a1,a2,...]
//
// Child JSON and stderr go to $ACCEPT_OUT (default <tmpdir>/i013-accept).
import { spawnSync, spawn } from 'node:child_process';
import { mkdirSync, writeFileSync, cpSync, rmSync, appendFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { randomBytes } from 'node:crypto';
import { join, dirname, resolve, normalize, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '../../../../../..');
const perf = join(repo, 'tests/e2e/perf.mjs');
const args = process.argv.slice(2);
const oi = args.indexOf('--only');
const only = oi >= 0 ? args.splice(oi, 2)[1].split(',') : null;
const webDir = resolve(args[0] || join(repo, 'crates/bevy-client/web/pkg-out'));
const out = process.env.ACCEPT_OUT || join(tmpdir(), 'i013-accept');
mkdirSync(out, { recursive: true });
const want = (t) => !only || only.includes(t);
const results = [];
const check = (name, ok, detail = '') => {
  results.push({ name, ok: !!ok });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`);
};
const num = (x) => typeof x === 'number' && Number.isFinite(x);
const CFGS = ['default@1x', 'default@4x', 'low@1x', 'low@4x'];

function runAsync(name, env = {}, extra = [], dir = webDir) {
  return new Promise((ok) => {
    const t0 = Date.now();
    const c = spawn('node', [perf, ...extra, ...(dir === null ? [] : [dir])], { env: { ...process.env, ...env } });
    let so = '', se = '';
    c.stdout.on('data', (d) => (so += d));
    c.stderr.on('data', (d) => (se += d));
    c.on('close', (code) => {
      let json = null;
      try { json = JSON.parse(so); } catch {}
      writeFileSync(join(out, `${name}.json`), so);
      writeFileSync(join(out, `${name}.stderr`), se);
      const secs = (Date.now() - t0) / 1000;
      console.log(`  ran ${name}: exit=${code} in ${secs.toFixed(1)} s`);
      ok({ code, json, secs, stderr: se });
    });
  });
}

let clean = null;
if (want('a1') || want('a3')) {
  const r = await runAsync('a1-full');
  const j = (clean = r.json || {});
  check('a1 exit 0 or 1', r.code === 0 || r.code === 1, `exit=${r.code}`);
  check('a1 exit matches pass', r.code === (j.pass ? 0 : 1));
  const runs = j.runs || {};
  check('a1 all four configurations', CFGS.every((c) => runs[c]));
  for (const c of CFGS) {
    const x = runs[c] || {};
    const cal = x.throttle_calibration || {};
    const rate = parseInt(c.split('@')[1], 10);
    check(`a1 ${c} device_pixel_ratio 1`, x.device_pixel_ratio === 1, `${x.device_pixel_ratio}`);
    check(`a1 ${c} visibility sampled, never hidden`, x.visibility?.samples > 0 && x.visibility?.hidden === 0, JSON.stringify(x.visibility));
    check(`a1 ${c} throttle verified, ratio in range`, x.throttle_verified === true && x.throttle_applied === true &&
      (rate === 1 ? cal.ratio <= 1.5 : cal.ratio >= 0.6 * rate && cal.ratio <= 1.6 * rate), JSON.stringify(cal));
    check(`a1 ${c} canvas not blank`, x.canvas && x.canvas.blank === false, JSON.stringify(x.canvas));
    check(`a1 ${c} tti_ms <= tti_raf_tick_ms, tti_bound_ms numeric`, num(x.tti_ms) && num(x.tti_raf_tick_ms) && x.tti_ms <= x.tti_raf_tick_ms && num(x.tti_bound_ms),
      `tti=${x.tti_ms} raf=${x.tti_raf_tick_ms} bound=${x.tti_bound_ms} first_frame=${x.first_frame_log_ms}`);
    const ci = x.cpu_ms?.p50_ci95;
    check(`a1 ${c} cpu p50 CI brackets p50`, Array.isArray(ci) && ci[0] <= x.cpu_ms.p50 && x.cpu_ms.p50 <= ci[1], `p50=${x.cpu_ms?.p50} ci=${JSON.stringify(ci)} frames=${x.frames}`);
    check(`a1 ${c} valid`, x.valid === true, JSON.stringify(x.invalid_reasons));
  }
  check('a1 body_mismatch empty', Array.isArray(j.size?.body_mismatch) && j.size.body_mismatch.length === 0, JSON.stringify(j.size?.body_mismatch));
  check('a1 B3.size uses the requested total', num(j.size?.requested_brotli_mb) && j.verdict?.B3?.size?.value === j.size.requested_brotli_mb,
    `requested=${j.size?.requested_brotli_mb} dir=${j.size?.brotli_total_mb} unrequested=${JSON.stringify(j.size?.unrequested)}`);
  check('a1 B2.cpu carries ci95 and ci_straddles_bar', Array.isArray(j.verdict?.B2?.cpu?.ci95) && typeof j.verdict?.B2?.cpu?.ci_straddles_bar === 'boolean',
    JSON.stringify(j.verdict?.B2?.cpu));
}

const jobs = [];
if (want('a2')) jobs.push((async () => {
  const r = await runAsync('a2-mac', { MAC_CHANNEL: 'chromium', MAC_HEADLESS: '1', MAC_SAMPLE_S: '10', TTI_TIMEOUT_S: '240' }, ['--mac']);
  const x = r.json?.runs?.mac || {};
  check('a2 mac DPR 2', x.device_pixel_ratio === 2, `${x.device_pixel_ratio}`);
  check('a2 mac renderer SwiftShader', /swiftshader/i.test(x.renderer || ''), x.renderer);
  check('a2 mac run invalid for the software renderer', x.valid === false && (x.invalid_reasons || []).some((s) => /software renderer/.test(s)), JSON.stringify(x.invalid_reasons));
  check('a2 exit 1', r.code === 1, `exit=${r.code}`);
})());

if (want('a4')) jobs.push((async () => {
  const alt = join(out, 'a4-served');
  rmSync(alt, { recursive: true, force: true });
  cpSync(webDir, alt, { recursive: true });
  appendFileSync(join(alt, 'hud.js'), '\n// a4: a different build than WEB_DIR\n');
  const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
  const srv = createServer(async (req, res) => {
    try {
      const p = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '') || 'index.html';
      const b = await readFile(join(alt, p));
      res.writeHead(200, { 'content-type': types[extname(p)] || 'application/octet-stream' }).end(b);
    } catch { res.writeHead(404).end(); }
  });
  await new Promise((ok) => srv.listen(0, '127.0.0.1', ok));
  const r = await runAsync('a4-mismatch', { BASE_URL: `http://127.0.0.1:${srv.address().port}/`, ONLY: 'low@1x' });
  srv.close();
  const mm = r.json?.size?.body_mismatch || [];
  check('a4 body_mismatch names hud.js', mm.some((m) => m.path === 'hud.js'), JSON.stringify(mm));
  check('a4 invalid, exit 1', r.json?.valid === false && r.code === 1, `valid=${r.json?.valid} exit=${r.code}`);
})());

if (want('a5')) jobs.push((async () => {
  const r = await runAsync('a5-skip-throttle', { PERF_SKIP_THROTTLE: '1', ONLY: 'low@4x' });
  const x = r.json?.runs?.['low@4x'] || {};
  check('a5 throttle not verified', x.throttle_verified === false, JSON.stringify(x.throttle_calibration));
  check('a5 run invalid, exit 1', x.valid === false && r.code === 1, `exit=${r.code} reasons=${JSON.stringify(x.invalid_reasons)}`);
})());

if (want('a6')) jobs.push((async () => {
  const r = await runAsync('a6-blank', { PERF_BLANK_CANVAS: '1', ONLY: 'low@1x' });
  const x = r.json?.runs?.['low@1x'] || {};
  check('a6 canvas blank', x.canvas?.blank === true, JSON.stringify(x.canvas));
  check('a6 run invalid, exit 1', x.valid === false && r.code === 1, `exit=${r.code} reasons=${JSON.stringify(x.invalid_reasons)}`);
})());

if (want('a7')) {
  const r = spawnSync('node', [perf, webDir], { env: { ...process.env, ONLY: ',' }, encoding: 'utf8' });
  check('a7 ONLY="," exits 2', r.status === 2, `exit=${r.status} ${r.stderr.trim()}`);
}
await Promise.all(jobs);

if (want('a3')) {
  const stray = join(out, 'a3-stray');
  rmSync(stray, { recursive: true, force: true });
  cpSync(webDir, stray, { recursive: true });
  writeFileSync(join(stray, 'stray.bin'), randomBytes(40e6));
  const r = await runAsync('a3-stray', { ONLY: 'low@1x' }, [], stray);
  const s = r.json?.size || {};
  const ref = clean?.size?.requested_brotli_mb;
  check('a3 dir total >= 40 MB', s.brotli_total_mb >= 40, `${s.brotli_total_mb}`);
  check('a3 stray.bin flagged unrequested', (s.unrequested || []).includes('stray.bin'), JSON.stringify(s.unrequested));
  check('a3 B3.size value within 1% of the clean requested total', num(ref) && Math.abs(r.json?.verdict?.B3?.size?.value - ref) / ref <= 0.01,
    `value=${r.json?.verdict?.B3?.size?.value} clean=${ref}`);
  rmSync(stray, { recursive: true, force: true });
}

const f = results.filter((r) => !r.ok).length;
console.log(`# accept: ${results.length - f}/${results.length} passed`);
process.exit(f ? 1 : 0);
