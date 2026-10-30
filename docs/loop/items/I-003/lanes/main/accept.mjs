// Acceptance tests for I-003 (tests/e2e/perf.mjs). Runs the perf script as a
// child process and checks its JSON and exit codes against spec.md.
//
//   node docs/loop/items/I-003/lanes/main/accept.mjs [WEB_DIR] [--only t1,t2,...]
//
// Tests: t1 (+t3, t8 from the same full run), t2, t4, t5, t6, t7, t9.
// Writes every child JSON to $ACCEPT_OUT (default /tmp/i003-accept).
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync, readFileSync, readdirSync, cpSync, rmSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync, brotliDecompressSync, brotliCompressSync, constants as Z } from 'node:zlib';
import { tmpdir } from 'node:os';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '../../../../../..');
const perf = join(repo, 'tests/e2e/perf.mjs');
const args = process.argv.slice(2);
const oi = args.indexOf('--only');
const only = oi >= 0 ? args.splice(oi, 2)[1].split(',') : null;
const webDir = resolve(args[0] || join(repo, 'crates/bevy-client/web/pkg-out'));
const out = process.env.ACCEPT_OUT || join(tmpdir(), 'i003-accept');
mkdirSync(out, { recursive: true });
const want = (t) => !only || only.includes(t);

const results = [];
const check = (name, ok, detail = '') => {
  results.push({ name, ok: !!ok });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`);
};
function run(name, env = {}, extra = [], dir = webDir) {
  const t0 = Date.now();
  const r = spawnSync('node', [perf, ...extra, ...(dir === null ? [] : [dir])], {
    env: { ...process.env, ...env }, encoding: 'utf8', maxBuffer: 64 << 20, timeout: 30 * 60 * 1000,
  });
  const secs = (Date.now() - t0) / 1000;
  let json = null;
  try { json = JSON.parse(r.stdout); } catch {}
  writeFileSync(join(out, `${name}.json`), r.stdout || '');
  writeFileSync(join(out, `${name}.stderr`), r.stderr || '');
  console.log(`  ran ${name}: exit=${r.status} in ${secs.toFixed(1)} s`);
  return { code: r.status, json, secs, stderr: r.stderr || '' };
}
const CFGS = ['default@1x', 'default@4x', 'low@1x', 'low@4x'];
const num = (x) => typeof x === 'number' && Number.isFinite(x);
const RAISED = 'b3_brotli_mb=1000,b3_tti_ms=1e9,b2_cpu_p50_ms=1e9';

if (want('t1')) {
  const r = run('t1-full');
  const j = r.json || {};
  check('t1 exit is 0 or 1 (not 2)', r.code === 0 || r.code === 1, `exit=${r.code}`);
  check('t1 JSON parses, schema perf/1', j.schema === 'perf/1');
  const runs = j.runs || {};
  check('t1 all four configurations', CFGS.every((c) => runs[c]), Object.keys(runs).join(','));
  for (const c of CFGS) {
    const x = runs[c] || {};
    check(`t1 ${c} fields`, num(x.tti_ms) && num(x.cpu_ms?.p50) && num(x.cpu_ms?.p95) && x.frames >= 30 &&
      x.workload_ok === true && x.throttle_applied === true && x.console_errors?.count === 0,
      JSON.stringify({ tti: x.tti_ms, cpu: x.cpu_ms, frames: x.frames, wl: x.workload_ok, thr: x.throttle_applied, err: x.console_errors?.count }));
  }
  const v = j.verdict || {};
  const has = (b) => b && 'value' in b && 'threshold' in b && 'pass' in b;
  check('t1 verdicts B3.size, B3.tti, B2.cpu', has(v.B3?.size) && has(v.B3?.tti) && has(v.B2?.cpu));
  const valid = CFGS.every((c) => runs[c]?.valid === true);
  check('t1 pass == AND(verdicts, validity)', j.pass === (!!v.B3?.size?.pass && !!v.B3?.tti?.pass && !!v.B2?.cpu?.pass && valid),
    `pass=${j.pass} size=${v.B3?.size?.pass} tti=${v.B3?.tti?.pass} cpu=${v.B2?.cpu?.pass} valid=${valid}`);
  check('t1 exit matches pass', r.code === (j.pass ? 0 : 1));
  // t3: throttling really applies.
  const l1 = runs['low@1x']?.cpu_ms?.p50, l4 = runs['low@4x']?.cpu_ms?.p50;
  check('t3 low@4x cpu p50 >= 1.5x low@1x', l4 >= 1.5 * l1, `low@1x=${l1} low@4x=${l4} ratio=${(l4 / l1).toFixed(2)}`);
  const d1 = runs['default@1x']?.tti_ms, d4 = runs['default@4x']?.tti_ms;
  check('t3 default@4x tti > default@1x tti', d4 > d1, `1x=${d1} 4x=${d4}`);
  // t6: workload coverage in a full run.
  for (const c of CFGS) {
    const w = runs[c]?.workload || {};
    check(`t6 ${c} each spell cast, burning seen, yaw >= 90`, [1, 2, 3, 4].every((s) => w.casts?.[s] >= 1) && w.burning_max >= 1 && w.yaw_deg >= 90,
      JSON.stringify(w));
  }
  // t8: own hook vs boot.js log.
  for (const c of CFGS) {
    const x = runs[c] || {};
    check(`t8 ${c} cpuframe_log_p50_ms reported`, num(x.cpuframe_log_p50_ms),
      `cpu_ms.p50=${x.cpu_ms?.p50} log=${x.cpuframe_log_p50_ms} ratio=${(x.cpu_ms?.p50 / x.cpuframe_log_p50_ms).toFixed(2)}`);
  }
  // t4 (size) uses this run's size block too.
  if (want('t4')) sizeChecks(j, 't1');
}

function sizeChecks(j, tag) {
  const files = j.size?.files || [];
  check('t4 size block present', files.length > 0 && num(j.size?.brotli_total_bytes) && num(j.size?.gzip_total_bytes));
  // `gzip -9c FILE` stores the file name in the gzip header (FNAME field,
  // name + NUL); node:zlib writes no name. On a tiny file those bytes alone
  // exceed 1%, so a file passes when the gap is within 1% OR is exactly
  // explained by the stored name, and the compressed stream itself is also
  // compared with `gzip -9nc` (no name), which must be within 1%.
  let worst = 0, worstN = 0, okAll = true;
  const rows = [];
  for (const f of files) {
    const cli = +spawnSync('sh', ['-c', 'gzip -9c "$1" | wc -c', 'sh', join(webDir, f.path)], { encoding: 'utf8' }).stdout.trim();
    const cliN = +spawnSync('sh', ['-c', 'gzip -9nc "$1" | wc -c', 'sh', join(webDir, f.path)], { encoding: 'utf8' }).stdout.trim();
    const nameBytes = Buffer.byteLength(f.path.split('/').pop()) + 1;
    const d = Math.abs(f.gzip - cli) / cli, dN = Math.abs(f.gzip - cliN) / cliN;
    const ok = (d <= 0.01 || Math.abs(cli - f.gzip) <= nameBytes) && dN <= 0.01;
    okAll &&= ok;
    worst = Math.max(worst, d); worstN = Math.max(worstN, dN);
    rows.push(`${f.path}: perf=${f.gzip} gzip-9c=${cli} gzip-9nc=${cliN} name=${nameBytes}B ${ok ? 'ok' : 'BAD'}`);
  }
  console.log('  ' + rows.join('\n  '));
  check('t4 gzip bytes match gzip -9 for every file (1%, or the stored file name)', okAll,
    `worst vs -9c ${(worst * 100).toFixed(3)}%, worst vs -9nc ${(worstN * 100).toFixed(3)}%`);
  const wasm = readFileSync(join(webDir, 'bevy-client_bg.wasm'));
  const br = brotliCompressSync(wasm, { params: { [Z.BROTLI_PARAM_QUALITY]: 11, [Z.BROTLI_PARAM_MODE]: Z.BROTLI_MODE_GENERIC, [Z.BROTLI_PARAM_LGWIN]: 24 } });
  const rt = brotliDecompressSync(br);
  const fw = files.find((f) => f.path === 'bevy-client_bg.wasm') || {};
  check('t4 brotli round trip of the .wasm is identical', rt.equals(wasm) && br.length === fw.brotli, `br=${br.length} reported=${fw.brotli}`);
  const mb = fw.brotli / 1e6;
  check('t4 brotli .wasm within 10% of README 3.5 MB', Math.abs(mb - 3.5) / 3.5 <= 0.1, `${mb.toFixed(3)} MB`);
  check('t4 missing_from_dir empty', Array.isArray(j.size?.missing_from_dir) && j.size.missing_from_dir.length === 0, JSON.stringify(j.size?.missing_from_dir));
}

if (want('t2')) {
  let r = run('t2-size', { ONLY: 'low@1x' }, ['--bars', 'b3_brotli_mb=0.1']);
  check('t2 tiny brotli bar fails', r.code === 1 && r.json?.verdict?.B3?.size?.pass === false && r.json?.pass === false, `exit=${r.code}`);
  r = run('t2-cpu', { ONLY: 'low@4x' }, ['--bars', 'b2_cpu_p50_ms=0.01']);
  check('t2 tiny cpu bar fails', r.code === 1 && r.json?.verdict?.B2?.cpu?.pass === false, `exit=${r.code}`);
  r = run('t2-tti', { ONLY: 'default@4x' }, ['--bars', 'b3_tti_ms=1']);
  check('t2 tiny tti bar fails', r.code === 1 && r.json?.verdict?.B3?.tti?.pass === false, `exit=${r.code}`);
  r = run('t2-partial-raised', { ONLY: 'low@1x' }, ['--bars', RAISED]);
  check('t2 subset run with raised bars exits 1, partial', r.code === 1 && r.json?.partial === true && r.json?.pass === false,
    `exit=${r.code} partial=${r.json?.partial} tti=${JSON.stringify(r.json?.verdict?.B3?.tti)}`);
  r = run('t2-full-raised', {}, ['--bars', RAISED]);
  check('t2 full run with raised bars exits 0', r.code === 0 && r.json?.pass === true, `exit=${r.code}`);
}

if (want('t4') && !want('t1')) {
  const r = run('t4-size', { ONLY: 'low@1x' });
  sizeChecks(r.json || {}, 't4');
}

if (want('t5')) {
  let r = run('t5a-nodir', {}, [], '/nonexistent/i003');
  const lines = r.stderr.trim().split('\n').filter(Boolean);
  check('t5a missing WEB_DIR exits 2 with one line', r.code === 2 && lines.length === 1, `exit=${r.code} stderr=${JSON.stringify(lines)}`);
  const broken = join(out, 'broken-pkg');
  rmSync(broken, { recursive: true, force: true });
  cpSync(webDir, broken, { recursive: true });
  rmSync(join(broken, 'bevy-client_bg.wasm'));
  r = run('t5b-nowasm', { ONLY: 'low@1x', TTI_TIMEOUT_S: '30' }, [], broken);
  const x = r.json?.runs?.['low@1x'] || {};
  check('t5b missing wasm exits 1 within 90 s, tti null, errors >= 1', r.code === 1 && r.secs <= 90 && x.tti_ms === null &&
    x.console_errors?.count >= 1 && r.json?.pass === false, `exit=${r.code} secs=${r.secs.toFixed(1)} tti=${x.tti_ms} errs=${x.console_errors?.count}`);
  r = run('t5c-noserver', { BASE_URL: 'http://127.0.0.1:9/', ONLY: 'low@1x' });
  check('t5c BASE_URL with no server exits 2', r.code === 2, `exit=${r.code} ${r.stderr.trim().split('\n').pop()}`);
}

if (want('t6')) {
  const r = run('t6-nocast', { ONLY: 'low@1x', PERF_NO_CAST: '1' });
  const x = r.json?.runs?.['low@1x'] || {};
  check('t6 PERF_NO_CAST makes workload_ok false and exits 1', r.code === 1 && x.workload_ok === false, `exit=${r.code} wl=${JSON.stringify(x.workload)}`);
}

if (want('t7')) {
  const a = run('t7-a', { ONLY: 'low@1x', RUNS: '1' });
  const b = run('t7-b', { ONLY: 'low@1x', RUNS: '1' });
  const ba = a.json?.size?.brotli_total_bytes, bb = b.json?.size?.brotli_total_bytes;
  check('t7 brotli totals byte-identical', num(ba) && ba === bb, `${ba} vs ${bb}`);
  const ca = a.json?.runs?.['low@1x']?.cpu_ms?.p50, cb = b.json?.runs?.['low@1x']?.cpu_ms?.p50;
  check('t7 cpu p50 within 30%', num(ca) && num(cb) && Math.abs(ca - cb) / Math.min(ca, cb) <= 0.3, `${ca} vs ${cb}`);
}

if (want('t9')) {
  const r = run('t9-mac', { MAC_CHANNEL: 'chromium', MAC_HEADLESS: '1', MAC_SAMPLE_S: '10' }, ['--mac']);
  const j = r.json || {}; const m = j.runs?.mac || {};
  check('t9 mac mode end to end', j.mode === 'mac' && num(m.fps_p50) && num(m.fps_p5) && typeof m.renderer === 'string' && m.webgl2 === true &&
    j.verdict?.B2?.mac && 'pass' in j.verdict.B2.mac, `exit=${r.code} fps_p50=${m.fps_p50} fps_p5=${m.fps_p5} renderer=${m.renderer} webgl2=${m.webgl2} verdict=${JSON.stringify(j.verdict?.B2?.mac)}`);
}

const fails = results.filter((r) => !r.ok);
console.log(`\n${results.length - fails.length}/${results.length} passed`);
process.exit(fails.length ? 1 : 0);
