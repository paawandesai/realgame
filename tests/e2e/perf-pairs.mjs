// Paired, interleaved perf verdict for one candidate build against one base
// build (I-024 Part T; docs/loop/items/I-024/spec.md, "Keep rule").
//
//   node tests/e2e/perf-pairs.mjs --base <WEB_DIR> --cand <WEB_DIR> [--rounds 5]
//     [--configs low@4x,default@4x] [--noise <noise-pairs.json>] [--out <file.json>]
//     [--keep-runs <dir>]
//   node tests/e2e/perf-pairs.mjs --self-test
//
// For each round and config it runs tests/e2e/perf.mjs as a child (WEB_DIR as
// its argument, ONLY=<config>, RUNS=1, OUT=<tmp>) once per side, in the order
// pairOrder() gives: base first in odd rounds (1, 3, 5), candidate first in
// even ones. An invalid run (perf.mjs's own `valid`) is re-run once in the
// same slot; still invalid means exit 2 and no verdict. It never builds.
//
// Keep rule (keepRule below; every limit is the spec's):
//   1. median low@4x CPU p50 <= 0.92 x base median, and the candidate below
//      base in at least 4 of 5 pairs (scaled: ceil(0.8 x rounds));
//   2. median paired default@4x TTI difference <= +1000 ms;
//   3. median paired low@4x TTI difference <= +500 ms, or, when the noise
//      run's low@4x spread is above 300 ms, that spread rounded up to the next
//      100 ms (never below 500, capped at 1000);
//   4. requested brotli <= base + 0.50 MB;
//   5. median low@4x p99 <= 1.10 x base median p99.
// (Rule 6, golden green at ?low, is run separately: it needs the Chromium
// lock to itself.)
//
// Output: one line per run on stderr, the verdict JSON on stdout (and in
// --out). With --base equal to --cand, tti_spread_ms is the noise figure that
// --noise reads back. Exit: 0 kept, 1 not kept, 2 usage or environment error
// (including a run still invalid after its re-run). --keep-runs <dir> keeps
// every perf.mjs result as <dir>/<config>-r<round>-<side>-a<attempt>.json and
// reuses one that is already there, so a verdict killed half way (container
// restart) resumes from its last finished run with the same schedule.
// --self-test: 0 when every
// unit test passes, 1 otherwise.
//
// Run it under the Chromium lock, alone on the box:
//   bash -c 'exec 9>/tmp/claude-0/chromium.lock; flock 9; node tests/e2e/perf-pairs.mjs ...'
import { spawn } from 'node:child_process';
import { readFileSync, writeFileSync, mkdtempSync, mkdirSync, rmSync, statSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const SIDES = ['base', 'cand'];
export const LIMITS = { cpu_ratio: 0.92, win_frac: 0.8, default_tti_ms: 1000, low_tti_ms: 500, noise_floor_ms: 300, noise_cap_ms: 1000, brotli_mb: 0.5, p99_ratio: 1.1 };

// ---- pure helpers ------------------------------------------------------------
export function median(xs) {
  const v = xs.filter((x) => typeof x === 'number' && Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return null;
  const m = v.length >> 1;
  return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
}
const round = (x, d) => (x === null ? null : +x.toFixed(d));

// Order of the two sides in each round: base first in odd rounds (1-based).
export function pairOrder(rounds) {
  const out = [];
  for (let r = 1; r <= rounds; r++) out.push(r % 2 ? ['base', 'cand'] : ['cand', 'base']);
  return out;
}

// The low@4x TTI limit: 500 ms, or the noise run's low@4x spread rounded up
// to the next 100 ms when that spread is above 300 ms (at least 500, at most
// 1000). `noise` is a noise-pairs.json verdict ({tti_spread_ms: {...}}) or a
// plain {low_spread_ms}.
export function lowTtiLimit(noise) {
  const s = noise == null ? null : (noise.tti_spread_ms?.['low@4x'] ?? noise.low_spread_ms ?? null);
  if (typeof s !== 'number' || s <= LIMITS.noise_floor_ms) return LIMITS.low_tti_ms;
  return Math.min(LIMITS.noise_cap_ms, Math.max(LIMITS.low_tti_ms, Math.ceil(s / 100) * 100));
}

// verdictInput: {cpu: {base: [per-round low@4x CPU p50], cand: [...]},
//   p99: {base: [...], cand: [...]}, tti_diff_ms: {'low@4x': n, 'default@4x': n}
//   (median paired differences, cand - base), brotli_mb: {base, cand}}.
export function keepRule(input, noise) {
  const reasons = [];
  const low_tti_limit_ms = lowTtiLimit(noise);
  const cb = input.cpu?.base || [], cc = input.cpu?.cand || [];
  const mb = median(cb), mc = median(cc);
  if (mb === null || mc === null) reasons.push('no low@4x cpu');
  else {
    const ratio = round(mc / mb, 4);
    if (ratio > LIMITS.cpu_ratio) reasons.push(`cpu ${ratio.toFixed(3)} > ${LIMITS.cpu_ratio}`);
    const n = Math.min(cb.length, cc.length);
    let wins = 0;
    for (let i = 0; i < n; i++) if (cc[i] < cb[i]) wins++;
    const need = Math.ceil(LIMITS.win_frac * n - 1e-9);
    if (wins < need) reasons.push(`wins ${wins}/${n}`);
  }
  const td = input.tti_diff_ms || {};
  const lt = td['low@4x'];
  if (typeof lt !== 'number') reasons.push('no low@4x tti');
  else if (lt > low_tti_limit_ms) reasons.push(`low@4x tti +${round(lt, 0)} > ${low_tti_limit_ms}`);
  const dt = td['default@4x'];
  if (typeof dt === 'number' && dt > LIMITS.default_tti_ms) reasons.push(`default@4x tti +${round(dt, 0)} > ${LIMITS.default_tti_ms}`);
  const bb = input.brotli_mb?.base, bc = input.brotli_mb?.cand;
  if (typeof bb !== 'number' || typeof bc !== 'number') reasons.push('no brotli size');
  else {
    const d = round(bc - bb, 3);
    if (d > LIMITS.brotli_mb) reasons.push(`brotli +${d.toFixed(2)} > ${LIMITS.brotli_mb.toFixed(2)}`);
  }
  const pb = median(input.p99?.base || []), pc = median(input.p99?.cand || []);
  if (pb === null || pc === null) reasons.push('no low@4x p99');
  else {
    const r = round(pc / pb, 4);
    if (r > LIMITS.p99_ratio) reasons.push(`p99 ${r.toFixed(2)} > ${LIMITS.p99_ratio.toFixed(2)}`);
  }
  return { keep: reasons.length === 0, reasons, low_tti_limit_ms };
}

// Reads the fields a pair line needs from one perf.mjs result.
export function runFields(result, config) {
  const r = result?.runs?.[config];
  if (!r) return { cpu_p50_ms: null, p99_ms: null, tti_ms: null, tti_phases: null, valid: false, brotli_mb: result?.size?.requested_brotli_mb ?? null, invalid_reasons: ['no run in perf output'] };
  return {
    cpu_p50_ms: r.cpu_ms?.p50 ?? null,
    p99_ms: r.cpu_ms?.p99 ?? null,
    tti_ms: r.tti_ms ?? null,
    tti_phases: r.tti_phases ?? null,
    valid: r.valid === true,
    brotli_mb: result?.size?.requested_brotli_mb ?? null,
    invalid_reasons: r.invalid_reasons || [],
  };
}

// Builds the verdict from the run list (pure).
export function buildVerdict({ base, cand, rounds, configs, runs, noise }) {
  const PH = ['fetch_ms', 'compile_init_ms', 'first_frame_ms', 'to_interactive_ms'];
  const pick = (side, cfg) => runs.filter((x) => x.side === side && x.config === cfg).sort((a, b) => a.round - b.round);
  const medians = {};
  for (const side of SIDES) {
    medians[side] = {};
    for (const cfg of configs) {
      const rs = pick(side, cfg);
      const ph = {};
      for (const k of PH) ph[k] = median(rs.map((x) => x.tti_phases?.[k] ?? null));
      medians[side][cfg] = { cpu_p50_ms: median(rs.map((x) => x.cpu_p50_ms)), tti_ms: median(rs.map((x) => x.tti_ms)), p99_ms: median(rs.map((x) => x.p99_ms)), tti_phases: ph };
    }
  }
  const paired_tti_diff_ms = {}, tti_spread_ms = {}, wins = {};
  for (const cfg of configs) {
    const b = pick('base', cfg), c = pick('cand', cfg);
    const diffs = [];
    let w = 0;
    for (let i = 0; i < Math.min(b.length, c.length); i++) {
      if (typeof b[i].tti_ms === 'number' && typeof c[i].tti_ms === 'number') diffs.push(c[i].tti_ms - b[i].tti_ms);
      if (typeof b[i].cpu_p50_ms === 'number' && typeof c[i].cpu_p50_ms === 'number' && c[i].cpu_p50_ms < b[i].cpu_p50_ms) w++;
    }
    paired_tti_diff_ms[cfg] = round(median(diffs), 2);
    tti_spread_ms[cfg] = diffs.length ? round(Math.max(...diffs) - Math.min(...diffs), 2) : null;
    wins[cfg] = w;
  }
  const brotli_mb = { base: median(runs.filter((x) => x.side === 'base').map((x) => x.brotli_mb)), cand: median(runs.filter((x) => x.side === 'cand').map((x) => x.brotli_mb)) };
  const low = 'low@4x';
  const k = keepRule({
    cpu: { base: pick('base', low).map((x) => x.cpu_p50_ms), cand: pick('cand', low).map((x) => x.cpu_p50_ms) },
    p99: { base: pick('base', low).map((x) => x.p99_ms), cand: pick('cand', low).map((x) => x.p99_ms) },
    tti_diff_ms: paired_tti_diff_ms,
    brotli_mb,
  }, noise);
  return {
    base, cand, rounds, configs,
    runs: runs.map(({ config, round, side, cpu_p50_ms, p99_ms, tti_ms, tti_phases, valid, rerun }) => ({ config, round, side, cpu_p50_ms, p99_ms, tti_ms, tti_phases, valid, rerun })),
    medians, paired_tti_diff_ms, tti_spread_ms, low_tti_limit_ms: k.low_tti_limit_ms, brotli_mb, wins, keep: k.keep, reasons: k.reasons,
    ...(base === cand ? { noise_run: true } : {}),
  };
}

export function runLine(x) {
  const p = x.tti_phases || {};
  return `${x.config} round ${x.round} ${x.side.padEnd(4)} cpu_p50=${x.cpu_p50_ms} p99=${x.p99_ms} tti=${x.tti_ms} ` +
    `fetch=${p.fetch_ms ?? null} compile_init=${p.compile_init_ms ?? null} first_frame=${p.first_frame_ms ?? null} to_interactive=${p.to_interactive_ms ?? null}` +
    `${p.warn ? ' (warn)' : ''} valid=${x.valid}${x.rerun ? ' rerun' : ''}`;
}

// Drives the schedule. `runOnce(side, config, round)` resolves to runFields().
// Returns {code: 2, verdict: null, error} when a run is still invalid after
// its one re-run, else {code: 0|1, verdict}.
export async function drivePairs({ base, cand, rounds, configs, noise, runOnce, onLine = () => {} }) {
  const order = pairOrder(rounds);
  const runs = [];
  for (let r = 1; r <= rounds; r++) {
    for (const config of configs) {
      for (const side of order[r - 1]) {
        let x = await runOnce(side, config, r);
        let rerun = false;
        if (!x.valid) {
          onLine(runLine({ ...x, config, round: r, side, rerun }) + ` (${(x.invalid_reasons || []).join('; ')}); re-running once`);
          x = await runOnce(side, config, r);
          rerun = true;
        }
        const rec = { ...x, config, round: r, side, rerun };
        onLine(runLine(rec) + (x.valid ? '' : ` (${(x.invalid_reasons || []).join('; ')})`));
        if (!x.valid) return { code: 2, verdict: null, error: `${config} round ${r} ${side}: still invalid after its re-run (${(x.invalid_reasons || []).join('; ')})` };
        runs.push(rec);
      }
    }
  }
  const verdict = buildVerdict({ base, cand, rounds, configs, runs, noise });
  return { code: verdict.keep ? 0 : 1, verdict };
}

// Runs `run(file)` for one slot attempt unless `file` already holds a parsed
// perf.mjs result (a resumed verdict); returns {result, reused}.
export async function cachedRun(file, run) {
  if (file) {
    try { return { result: JSON.parse(readFileSync(file, 'utf8')), reused: true }; } catch {}
  }
  return { result: await run(file), reused: false };
}

// ---- self-test --------------------------------------------------------------
async function selfTest() {
  let passed = 0, failed = 0;
  const t = async (name, fn) => {
    try { await fn(); passed++; console.log(`ok - ${name}`); }
    catch (e) { failed++; console.log(`not ok - ${name}: ${e && e.message}`); }
  };
  const eq = (a, b, m = '') => { if (JSON.stringify(a) !== JSON.stringify(b)) throw new Error(`${m} expected ${JSON.stringify(b)}, got ${JSON.stringify(a)}`); };
  const ok = (c, m) => { if (!c) throw new Error(m || 'assertion failed'); };
  const five = (x) => [x, x, x, x, x];
  const input = (o = {}) => ({
    cpu: { base: [40, 43, 48, 42, 45], cand: [35, 38, 41, 37, 40] },
    p99: { base: five(100), cand: five(105) },
    tti_diff_ms: { 'low@4x': 200, 'default@4x': 200 },
    brotli_mb: { base: 3.557, cand: 3.757 },
    ...o,
  });

  await t('keep: median 38 <= 0.92 x 43, 5/5 wins, TTI +200, p99 +5%, brotli +0.2', () => {
    const k = keepRule(input());
    eq(k.keep, true, JSON.stringify(k.reasons));
    eq(k.reasons, []);
    eq(k.low_tti_limit_ms, 500);
  });
  await t('not kept: median passes but only 2 wins in 5', () => {
    const k = keepRule(input({ cpu: { base: [30, 43, 48, 44, 45], cand: [31, 39, 49, 38, 45] } }));
    eq(k.keep, false);
    eq(k.reasons, ['wins 2/5']);
  });
  await t('not kept: cpu 0.930 > 0.92 with 5/5 wins', () => {
    const k = keepRule(input({ cpu: { base: [41, 42, 43, 44, 45], cand: [38, 39, 40, 41, 42] } }));
    eq(k.keep, false);
    eq(k.reasons, ['cpu 0.930 > 0.92']);
  });
  await t('low@4x TTI +501 with no noise input: not kept', () => {
    const k = keepRule(input({ tti_diff_ms: { 'low@4x': 501, 'default@4x': 200 } }));
    eq(k.keep, false);
    eq(k.reasons, ['low@4x tti +501 > 500']);
  });
  await t('default@4x TTI +1001: not kept; +900: allowed', () => {
    const k = keepRule(input({ tti_diff_ms: { 'low@4x': 200, 'default@4x': 1001 } }));
    eq(k.keep, false);
    eq(k.reasons, ['default@4x tti +1001 > 1000']);
    eq(keepRule(input({ tti_diff_ms: { 'low@4x': 200, 'default@4x': 900 } })).keep, true);
  });
  await t('noise spread sets the low@4x limit: 420 -> 500, 640 -> 700 (+650 allowed), 1400 -> 1000', () => {
    const n = (s) => ({ tti_spread_ms: { 'low@4x': s, 'default@4x': 800 } });
    eq(keepRule(input(), n(420)).low_tti_limit_ms, 500);
    eq(keepRule(input({ tti_diff_ms: { 'low@4x': 501, 'default@4x': 0 } }), n(420)).keep, false);
    const k = keepRule(input({ tti_diff_ms: { 'low@4x': 650, 'default@4x': 0 } }), n(640));
    eq(k.low_tti_limit_ms, 700);
    eq(k.keep, true, JSON.stringify(k.reasons));
    eq(keepRule(input(), n(1400)).low_tti_limit_ms, 1000);
    eq(keepRule(input(), n(250)).low_tti_limit_ms, 500);
  });
  await t('median p99 +11%: not kept', () => {
    const k = keepRule(input({ p99: { base: five(100), cand: five(111) } }));
    eq(k.keep, false);
    eq(k.reasons, ['p99 1.11 > 1.10']);
    eq(keepRule(input({ p99: { base: five(100), cand: five(110) } })).keep, true);
  });
  await t('brotli +0.51 MB: not kept; +0.50 allowed', () => {
    const k = keepRule(input({ brotli_mb: { base: 3.2, cand: 3.71 } }));
    eq(k.keep, false);
    eq(k.reasons, ['brotli +0.51 > 0.50']);
    eq(keepRule(input({ brotli_mb: { base: 3.2, cand: 3.7 } })).keep, true);
  });
  await t('pairOrder(5) alternates, base first in rounds 1, 3 and 5', () => {
    eq(pairOrder(5), [['base', 'cand'], ['cand', 'base'], ['base', 'cand'], ['cand', 'base'], ['base', 'cand']]);
  });

  // The driver, with a fake perf runner.
  const fake = (over = () => null) => {
    const calls = [];
    const runOnce = async (side, config, round) => {
      calls.push([side, config, round]);
      const o = over(side, config, round, calls.length);
      if (o) return o;
      const cpu = side === 'base' ? 40 : 34;
      return { cpu_p50_ms: cpu, p99_ms: 60, tti_ms: 9000 + round, tti_phases: { fetch_ms: 100, compile_init_ms: 1000, first_frame_ms: 2000, to_interactive_ms: 100 }, valid: true, brotli_mb: side === 'base' ? 3.5 : 3.6, invalid_reasons: [] };
    };
    return { calls, runOnce };
  };
  await t('driver: an invalid run still invalid after its re-run exits 2 with no verdict', async () => {
    const f = fake((side, config, round) => (side === 'cand' && round === 2 && config === 'low@4x' ? { valid: false, invalid_reasons: ['machine busy: load1=3'] } : null));
    const r = await drivePairs({ base: 'a', cand: 'b', rounds: 5, configs: ['low@4x', 'default@4x'], runOnce: f.runOnce });
    eq(r.code, 2);
    eq(r.verdict, null);
    ok(/still invalid/.test(r.error), r.error);
  });
  await t('driver: one invalid run is re-run once in the same slot and then counts', async () => {
    let n = 0;
    const f = fake((side, config, round) => (side === 'base' && round === 3 && config === 'default@4x' && n++ === 0 ? { valid: false, invalid_reasons: ['throttle not verified'] } : null));
    const r = await drivePairs({ base: 'a', cand: 'b', rounds: 5, configs: ['low@4x', 'default@4x'], runOnce: f.runOnce });
    eq(r.code, 0, JSON.stringify(r.verdict?.reasons));
    eq(f.calls.length, 21);
    eq(r.verdict.runs.filter((x) => x.rerun).length, 1);
    eq(r.verdict.runs.length, 20);
    eq(r.verdict.wins['low@4x'], 5);
    eq(r.verdict.paired_tti_diff_ms['low@4x'], 0);
  });
  await t('driver: the run order alternates by round for every config', async () => {
    const f = fake();
    await drivePairs({ base: 'a', cand: 'b', rounds: 5, configs: ['low@4x', 'default@4x'], runOnce: f.runOnce });
    const lowSides = f.calls.filter((c) => c[1] === 'low@4x').map((c) => c[0]);
    eq(lowSides, ['base', 'cand', 'cand', 'base', 'base', 'cand', 'cand', 'base', 'base', 'cand']);
    const defSides = f.calls.filter((c) => c[1] === 'default@4x').map((c) => c[0]);
    eq(defSides, lowSides);
  });
  await t('verdict: base vs base gives the TTI spread per config (the noise figure)', async () => {
    const f = fake((side, config, round) => ({ cpu_p50_ms: 40, p99_ms: 60, tti_ms: side === 'cand' ? 9000 + round * 100 : 9000, tti_phases: {}, valid: true, brotli_mb: 3.5, invalid_reasons: [] }));
    const r = await drivePairs({ base: 'a', cand: 'a', rounds: 5, configs: ['low@4x', 'default@4x'], runOnce: f.runOnce });
    eq(r.verdict.tti_spread_ms, { 'low@4x': 400, 'default@4x': 400 });
    eq(r.verdict.noise_run, true);
    eq(r.code, 1);
    for (const k of ['base', 'cand', 'rounds', 'configs', 'runs', 'medians', 'paired_tti_diff_ms', 'tti_spread_ms', 'low_tti_limit_ms', 'brotli_mb', 'wins', 'keep', 'reasons'])
      ok(k in r.verdict, `verdict lacks ${k}`);
  });
  await t('cachedRun reuses a kept result and runs a missing one', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'perf-pairs-test-'));
    try {
      const f = join(dir, 'low@4x-r1-base-a1.json');
      writeFileSync(f, JSON.stringify({ size: {}, runs: { 'low@4x': { valid: true } } }));
      let ran = 0;
      const a = await cachedRun(f, async () => { ran++; return null; });
      eq([a.reused, ran, a.result.runs['low@4x'].valid], [true, 0, true]);
      const b = await cachedRun(join(dir, 'missing.json'), async () => { ran++; return { x: 1 }; });
      eq([b.reused, ran, b.result], [false, 1, { x: 1 }]);
    } finally { rmSync(dir, { recursive: true, force: true }); }
  });
  await t('runFields reads a perf.mjs result', () => {
    const res = { size: { requested_brotli_mb: 3.557 }, runs: { 'low@4x': { cpu_ms: { p50: 41.2, p99: 70 }, tti_ms: 9100, tti_phases: { fetch_ms: 1 }, valid: true, invalid_reasons: [] } } };
    eq(runFields(res, 'low@4x'), { cpu_p50_ms: 41.2, p99_ms: 70, tti_ms: 9100, tti_phases: { fetch_ms: 1 }, valid: true, brotli_mb: 3.557, invalid_reasons: [] });
    eq(runFields(res, 'default@4x').valid, false);
  });

  console.log(`# self-test: ${passed} passed, ${failed} failed`);
  return failed === 0 && passed > 0 ? 0 : 1;
}

// ---- CLI --------------------------------------------------------------------
function usage(msg) {
  console.error(`perf-pairs: ${msg}\nusage: node tests/e2e/perf-pairs.mjs --base <WEB_DIR> --cand <WEB_DIR> [--rounds 5] [--configs low@4x,default@4x] [--noise <noise-pairs.json>] [--out <file.json>] [--keep-runs <dir>] | --self-test`);
  process.exit(2);
}

// One perf.mjs child; resolves to its parsed result, or {exit} with no output.
// With `keep` (a file path) the result is written there and kept.
function perfChild(webDir, config, keep) {
  const dir = keep ? null : mkdtempSync(join(tmpdir(), 'perf-pairs-'));
  const out = keep || join(dir, 'run.json');
  return new Promise((res) => {
    const env = { ...process.env, ONLY: config, RUNS: '1', OUT: out };
    delete env.BASE_URL;
    const ch = spawn(process.execPath, [join(here, 'perf.mjs'), webDir], { env, stdio: ['ignore', 'ignore', 'inherit'] });
    ch.on('close', (code) => {
      let result = null;
      try { result = JSON.parse(readFileSync(out, 'utf8')); } catch {}
      if (dir) rmSync(dir, { recursive: true, force: true });
      res(result || { exit: code });
    });
  });
}

async function perfSlot(webDir, config, keep) {
  const { result, reused } = await cachedRun(keep, (f) => perfChild(webDir, config, f));
  if (!result || !result.runs) return { valid: false, invalid_reasons: [`perf.mjs exit ${result?.exit} with no output`] };
  if (reused) console.error(`[pairs] reusing ${keep}`);
  return runFields(result, config);
}

async function main() {
  const argv = process.argv.slice(2);
  if (argv.includes('--self-test')) process.exit(await selfTest());
  const opt = { rounds: 5, configs: ['low@4x', 'default@4x'] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const val = () => { const v = argv[++i]; if (v === undefined) usage(`${a} needs a value`); return v; };
    if (a === '--base') opt.base = val();
    else if (a === '--cand') opt.cand = val();
    else if (a === '--rounds') { opt.rounds = Number(val()); if (!Number.isInteger(opt.rounds) || opt.rounds < 1) usage('--rounds must be a positive integer'); }
    else if (a === '--configs') opt.configs = val().split(',').map((s) => s.trim()).filter(Boolean);
    else if (a === '--noise') opt.noise = val();
    else if (a === '--out') opt.out = val();
    else if (a === '--keep-runs') opt.keepRuns = val();
    else usage(`unknown argument ${a}`);
  }
  if (!opt.base || !opt.cand) usage('--base and --cand are required');
  for (const c of opt.configs) if (!['default@1x', 'default@4x', 'low@1x', 'low@4x'].includes(c)) usage(`unknown config ${c}`);
  if (!opt.configs.includes('low@4x')) usage('--configs must include low@4x (the CPU rule)');
  for (const k of ['base', 'cand']) {
    opt[k] = resolve(opt[k]);
    try { if (!statSync(opt[k]).isDirectory()) throw 0; } catch { usage(`--${k} is not a directory: ${opt[k]}`); }
  }
  let noise = null;
  if (opt.noise) {
    try { noise = JSON.parse(readFileSync(opt.noise, 'utf8')); } catch (e) { usage(`cannot read --noise: ${e.message}`); }
  }
  const dirOf = { base: opt.base, cand: opt.cand };
  if (opt.keepRuns) mkdirSync(opt.keepRuns, { recursive: true });
  const attempts = new Map();
  const slotFile = (side, config, round) => {
    if (!opt.keepRuns) return null;
    const key = `${config}-r${round}-${side}`;
    const n = (attempts.get(key) || 0) + 1;
    attempts.set(key, n);
    return join(opt.keepRuns, `${key}-a${n}.json`);
  };
  const r = await drivePairs({
    base: opt.base, cand: opt.cand, rounds: opt.rounds, configs: opt.configs, noise,
    runOnce: (side, config, round) => perfSlot(dirOf[side], config, slotFile(side, config, round)),
    onLine: (l) => console.error(`[pairs] ${l}`),
  });
  if (r.code === 2) { console.error(`perf-pairs: ${r.error}; no verdict`); process.exit(2); }
  if (noise) r.verdict.noise_input = { file: opt.noise, tti_spread_ms: noise.tti_spread_ms ?? null };
  const text = JSON.stringify(r.verdict, null, 2);
  if (opt.out) writeFileSync(opt.out, text + '\n');
  process.stdout.write(text + '\n');
  console.error(`[pairs] keep=${r.verdict.keep} reasons=${JSON.stringify(r.verdict.reasons)} low@4x tti limit ${r.verdict.low_tti_limit_ms} ms; tti spread ${JSON.stringify(r.verdict.tti_spread_ms)}`);
  process.exit(r.code);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
