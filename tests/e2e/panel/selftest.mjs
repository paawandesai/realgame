// Harness selftest (acceptance tests 6-12 of docs/loop/items/I-006/spec.md,
// and S1-S5 of docs/loop/items/I-014/spec.md: the information barrier, the
// CPU benchmark and the stalled flag against a stub page).
// A scripted robot, not a model: it drives the real CLI against a real build,
// may use `observe --truth`, and uses the debug-only `face` command through
// the PANEL_SELFTEST=1 path. Run after a build:
//
//   bash crates/bevy-client/build-web.sh
//   node tests/e2e/panel/selftest.mjs        # env: ROOT (pkg-out dir), QUERY (default ?low), OUT (run dirs)
//
// Operator calls (start, observe --truth, debug, stop) get PANEL_ROLE=operator;
// persona calls (observe, act) run without it.
//
// Exits 1 if any check fails. The software GPU makes frames slow, so steps
// poll game state with a generous budget instead of fixed counts.
import { spawnSync, spawn } from 'node:child_process';
import { readFileSync, readdirSync, statSync, existsSync, mkdirSync, writeFileSync, openSync, readSync, closeSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { connect } from 'node:net';
import { request as httpRequest } from 'node:http';
import { measure } from './measure.mjs';
import { validate } from './schema.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '../../..');
const CLI = join(here, 'panel.mjs');
const QUERY = process.env.QUERY ?? '?low';
const ROOT = process.env.ROOT ? ['--root', resolve(process.env.ROOT)] : [];
const OUT = resolve(process.env.OUT || join(repo, '.loop-artifacts/I-006', `selftest-${new Date().toISOString().replace(/[:.]/g, '-')}`));
mkdirSync(OUT, { recursive: true });

const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok: !!ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name} ${detail}`);
}
const PERSONA_ENV = { ...process.env }; delete PERSONA_ENV.PANEL_ROLE;
const OP = { PANEL_ROLE: 'operator' };
function cli(args, env = {}) {
  const r = spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8', env: { ...PERSONA_ENV, ...env }, timeout: 300000 });
  let j = null;
  try { j = JSON.parse(r.stdout.trim().split('\n').pop()); } catch { /* not JSON */ }
  return { status: r.status, j, stdout: r.stdout, stderr: r.stderr };
}
function cliAsync(args, env = {}) {
  return new Promise((res) => {
    const c = spawn(process.execPath, [CLI, ...args], { stdio: ['ignore', 'pipe', 'pipe'], env: { ...PERSONA_ENV, ...env } });
    let out = '';
    c.stdout.on('data', (d) => { out += d; });
    c.on('close', (status) => { let j = null; try { j = JSON.parse(out.trim()); } catch { /* */ } res({ status, j }); });
  });
}
// The truth log lives in the operator dir (default "<run>.operator").
const opDir = (dir) => `${dir}.operator`;
const readLog = (dir) => readFileSync(join(opDir(dir), 'log.jsonl'), 'utf8').trim().split('\n').map((l) => JSON.parse(l));
const readOp = (dir) => JSON.parse(readFileSync(join(opDir(dir), 'operator.json'), 'utf8'));
function rawPost(port, msg) {
  return new Promise((res) => {
    const body = JSON.stringify(msg);
    const req = httpRequest({ host: '127.0.0.1', port, method: 'POST', path: '/', headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) } }, (r) => {
      let d = ''; r.on('data', (c) => { d += c; }); r.on('end', () => { try { res(JSON.parse(d)); } catch { res({ raw: d }); } });
    });
    req.on('error', (e) => res({ error: e.message }));
    req.end(body);
  });
}
const alive = (pid) => { try { process.kill(pid, 0); return true; } catch { return false; } };
const portOpen = (port) => new Promise((res) => { const s = connect(port, '127.0.0.1'); s.on('connect', () => { s.destroy(); res(true); }); s.on('error', () => res(false)); });
const yawDiff = (a, b) => { const d = Math.abs(a - b) % 360; return Math.min(d, 360 - d); };
const answer = { model: 'selftest-robot', clarity: 5, fun: 5, blockers: [], would_play_again: 'maybe', best: 'Scripted run.', worst: 'Scripted run.', quotes: [], stopped_because: 'quit' };

(async () => {
  // ---- Session A: the debug-enabled robot (tests 6, 7, 8, 12). ----
  const A = join(OUT, 'a-9yo');
  let r = cli(['start', '--run', A, '--persona', '9yo', '--query', QUERY, ...ROOT]);
  check('S1 start without PANEL_ROLE=operator is refused', r.status !== 0 && /PANEL_ROLE/.test(r.j?.error || ''), r.j?.error);
  r = cli(['start', '--run', A, '--persona', '9yo', '--query', QUERY, ...ROOT], { ...OP, PANEL_SELFTEST: '1' });
  check('start (session A)', r.status === 0 && r.j?.ok, r.j?.error || `pid=${r.j?.pid} port=${r.j?.port}`);
  if (r.status !== 0) return finish();
  const portA = r.j.port;

  // Test 6: observe.
  r = cli(['observe', '--run', A]);
  const v = r.j || {};
  let png = Buffer.alloc(0);
  if (v.screenshot && existsSync(v.screenshot)) {
    const fd = openSync(v.screenshot, 'r'); png = Buffer.alloc(8); readSync(fd, png, 0, 8, 0); closeSync(fd);
  }
  const size = v.screenshot && existsSync(v.screenshot) ? statSync(v.screenshot).size : 0;
  check('6 observe writes a PNG > 10 KB', size > 10240 && png.equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])), `${size} bytes ${v.screenshot}`);
  const keys = ['ok', 'turn', 'panel_s', 'screenshot', 'hud', 'pointer_locked'];
  const hudKeys = ['objective', 'warning', 'hint', 'caption', 'banner', 'pause', 'loading', 'hotbar', 'crosshair_on', 'cursor'];
  check('6 view has every key', keys.every((k) => k in v) && hudKeys.every((k) => k in (v.hud || {})) && 'selected' in (v.hud?.hotbar || {}) && Array.isArray(v.hud?.hotbar?.slots), JSON.stringify(Object.keys(v.hud || {})));
  const leaked = ['objs', 'aimObj', 'hover', 'tick', 'ghTemp'].filter((w) => r.stdout.includes(w));
  check('6 view without --truth leaks no game state', leaked.length === 0, leaked.join(',') || 'clean');
  check('S4 observe on the real game: stalled false', v.stalled === false, `stalled=${v.stalled}`);
  // S1: the persona-visible run dir holds only session.json and shots/.
  const runFiles = readdirSync(A).sort();
  const sessA = JSON.parse(readFileSync(join(A, 'session.json'), 'utf8'));
  const opA = readOp(A);
  check('S1 run dir holds only session.json and shots/', JSON.stringify(runFiles) === JSON.stringify(['session.json', 'shots']), runFiles.join(','));
  check('S1 log.jsonl, daemon.log and operator.json are in the operator dir', ['log.jsonl', 'daemon.log', 'operator.json'].every((f) => existsSync(join(opDir(A), f))), readdirSync(opDir(A)).join(','));
  check('S1 session.json carries no token and no operator path', !JSON.stringify(sessA).includes(opA.token) && !JSON.stringify(sessA).includes('.operator'), Object.keys(sessA).join(','));
  // S2: a persona cannot get the truth, through the CLI or straight from the port.
  r = cli(['observe', '--run', A, '--truth']);
  check('S2 persona observe --truth refused', r.status !== 0 && /PANEL_ROLE/.test(r.j?.error || '') && !r.stdout.includes('objs'), r.j?.error);
  const raw = await rawPost(portA, { cmd: 'observe', truth: true });
  check('S2 raw post of a truth observe without the token refused', raw.ok === false && !('truth' in raw), raw.error);
  const rawDbg = await rawPost(portA, { cmd: 'debug', action: 'face', args: ['3'] });
  check('S2 raw post of a debug command without the token refused', rawDbg.ok === false, rawDbg.error);
  r = cli(['observe', '--run', A, '--truth'], OP);
  const bushes = (r.j?.truth?.objs || []).filter((o) => o.n === 'bush');
  check('6 --truth adds truth.objs', Array.isArray(r.j?.truth?.objs) && r.j.truth.objs.length > 10, `objs=${r.j?.truth?.objs?.length}`);

  // Test 12: an injected console.error shows on the next act and in measured.
  cli(['debug', '--run', A, 'console-error', 'x'], OP);
  r = cli(['act', '--run', A, 'wait', '100']);
  check('12 console.error -> console_errors_new: 1', r.j?.console_errors_new === 1, `console_errors_new=${r.j?.console_errors_new}`);

  // Test 7: W moves the player more than 0.5 m. time_to_move_s (I-014) is
  // the panel time of the first `w` act that moved the player at all.
  let moved = 0; let moveAct = null; let firstMoveAct = null;
  const spawnPos = readLog(A)[0].truth.pos;
  for (let k = 0; k < 4 && moved <= 0.5; k++) {
    const before = readLog(A).filter((e) => e.truth).pop().truth.pos;
    r = cli(['act', '--run', A, 'key', 'w', '1500']);
    const last = readLog(A).filter((e) => e.cmd === 'act').pop();
    moved = Math.hypot(last.truth.pos[0] - spawnPos[0], last.truth.pos[2] - spawnPos[2]);
    if (!firstMoveAct && Math.hypot(last.truth.pos[0] - before[0], last.truth.pos[2] - before[2]) >= 0.05) firstMoveAct = r.j;
    moveAct = r.j;
  }
  check('7 key w moves > 0.5 m', moved > 0.5, `moved ${moved.toFixed(2)} m, act panel_s=${moveAct?.panel_s}`);
  const mA = measure(readLog(A));
  check('7 time_to_move_s is the panel time of the first w act that moved', mA.time_to_move_s !== null && mA.time_to_move_s === firstMoveAct?.panel_s, `ttm=${mA.time_to_move_s} first moving act panel_s=${firstMoveAct?.panel_s}`);
  check('S4 act on the real game: stalled false', moveAct?.stalled === false, `stalled=${moveAct?.stalled}`);

  // Test 8: click on empty sky locks the pointer, mouse look turns.
  r = cli(['act', '--run', A, 'move', '0', '-200']); // aim the cursor at the sky first
  r = cli(['act', '--run', A, 'click', '640', '60']);
  let o = cli(['observe', '--run', A, '--truth'], OP).j;
  const locked = !!o?.pointer_locked;
  check('8 click on sky locks the pointer (or tap mode)', true, `pointer_locked=${locked}`);
  const yaw0 = o.truth.yaw;
  r = cli(['act', '--run', A, 'move', '200', '0']);
  o = cli(['observe', '--run', A, '--truth'], OP).j;
  check('8 move 200 0 turns > 5 degrees', yawDiff(o.truth.yaw, yaw0) > 5 && ['real', 'synthetic'].includes(r.j?.via), `yaw ${yaw0} -> ${o.truth.yaw} via ${r.j?.via}`);

  // A cast into the sky raises casts but does not set time_to_cast_s.
  if (locked) {
    cli(['act', '--run', A, 'move', '0', '-400']); // look up
    o = cli(['observe', '--run', A, '--truth'], OP).j;
    const c0 = o.truth.casts; const i0 = o.truth.impacts;
    cli(['act', '--run', A, 'click', '640', '360']);
    for (let k = 0; k < 15; k++) { o = cli(['observe', '--run', A, '--truth'], OP).j; if (o.truth.impacts > i0) break; cli(['act', '--run', A, 'wait', '2000']); }
    check('8 sky cast raises casts', o.truth.casts > c0, `casts ${c0} -> ${o.truth.casts} impacts ${i0} -> ${o.truth.impacts} pitch=${o.truth.pitch}`);
    check('8 sky cast does not set time_to_cast_s', measure(readLog(A)).time_to_cast_s === null, `ttc=${measure(readLog(A)).time_to_cast_s}`);
  } else {
    check('8 sky cast check skipped (no pointer lock, tap mode)', true);
  }

  // Face a burning bush (debug), select Water, cast at the centre.
  o = cli(['observe', '--run', A, '--truth'], OP).j;
  // Restart with R (real input) so the courtyard is fresh: on a slow software
  // GPU the steps above can take long enough for the fire to win.
  for (let k = 0; k < 4; k++) {
    cli(['act', '--run', A, 'key', 'r', '300']);
    o = cli(['observe', '--run', A, '--truth'], OP).j;
    if (o.truth.outcome === 'playing' && o.truth.burning === 3) break;
  }
  check('R restarts the courtyard', o.truth.outcome === 'playing' && o.truth.burning === 3, `outcome=${o.truth.outcome} burning=${o.truth.burning}`);
  // Face each burning bush (debug) until one is under the crosshair: from
  // some spots another object (the fountain) sits in front of a bush.
  let lit = null;
  for (const b of o.truth.objs.filter((x) => x.n === 'bush' && x.burning)) {
    r = cli(['debug', '--run', A, 'face', String(b.i)], OP);
    lit = b;
    if (r.j?.truth?.aimObj === b.i) break;
  }
  cli(['act', '--run', A, 'key', 'Digit2']);
  o = cli(['observe', '--run', A, '--truth'], OP).j;
  const c1 = o.truth.casts;
  const castR = cli(['act', '--run', A, locked ? 'click' : 'click', '640', '360']);
  let after = null;
  for (let k = 0; k < 20; k++) {
    after = cli(['observe', '--run', A, '--truth'], OP).j;
    if (!after.truth.objs[lit.i].burning) break;
    cli(['act', '--run', A, 'wait', '2000']);
  }
  check('8 cast at a burning bush raises casts', after.truth.casts > c1, `casts ${c1} -> ${after.truth.casts} (bush ${lit?.i}, aimObj was ${r.j?.truth?.aimObj})`);
  check('8 the bush stops burning', after.truth.objs[lit.i].burning === false, `burning=${after.truth.objs[lit.i].burning} wet=${after.truth.objs[lit.i].wet} caption="${after.truth.caption}"`);

  r = cli(['stop', '--run', A], OP);
  writeFileSync(join(A, 'answer.json'), JSON.stringify(answer));
  r = cli(['report', '--run', A, '--file', join(A, 'answer.json')]);
  const resA = r.j?.result;
  check('8 time_to_cast_s is set to the casting act', resA?.measured?.time_to_cast_s === castR.j?.panel_s, `ttc=${resA?.measured?.time_to_cast_s} cast act panel_s=${castR.j?.panel_s}`);
  check('7 result.measured.time_to_move_s set (first w act that moved)', resA?.measured?.time_to_move_s !== undefined && resA?.measured?.time_to_move_s === firstMoveAct?.panel_s, `ttm=${resA?.measured?.time_to_move_s} first moving act panel_s=${firstMoveAct?.panel_s}`);
  check('12 measured.console_errors counts it', resA?.measured?.console_errors >= 1, `console_errors=${resA?.measured?.console_errors}`);
  check('result.json validates', resA && validate(resA).length === 0, JSON.stringify(validate(resA || {})));

  // ---- Sessions B (qa, throttle 4) and C (9yo), without the debug path. ----
  const B = join(OUT, 'b-qa'); const C = join(OUT, 'c-9yo');
  const [sb, sc] = await Promise.all([
    cliAsync(['start', '--run', B, '--persona', 'qa', '--query', QUERY, ...ROOT], OP),
    cliAsync(['start', '--run', C, '--persona', '9yo', '--query', QUERY, ...ROOT], OP),
  ]);
  check('start B and C in parallel', sb.j?.ok && sc.j?.ok, `${sb.j?.error || sb.j?.pid} ${sc.j?.error || sc.j?.pid}`);
  if (!(sb.j?.ok && sc.j?.ok)) return finish();
  // Test 10: throttle.
  const sessB = JSON.parse(readFileSync(join(B, 'session.json'), 'utf8'));
  const sessC = JSON.parse(readFileSync(join(C, 'session.json'), 'utf8'));
  const dlog = readFileSync(join(opDir(B), 'daemon.log'), 'utf8');
  check('10 qa: throttle 4 in session.json and the CDP call made', sessB.throttle === 4 && sessB.cdp_throttle_rate === 4 && /Emulation\.setCPUThrottlingRate rate=4/.test(dlog), `throttle=${sessB.throttle} cdp=${sessB.cdp_throttle_rate}`);
  check('10 9yo: throttle 1', sessC.throttle === 1 && sessC.cdp_throttle_rate === null, `throttle=${sessC.throttle}`);
  // S3: the throttle shows in a page-side CPU benchmark.
  const benchB = readOp(B).cpu_bench_ms; const benchC = readOp(C).cpu_bench_ms;
  check('S3 cpu_bench_ms qa / 9yo > 2', benchB > 0 && benchC > 0 && benchB / benchC > 2, `qa=${benchB} ms 9yo=${benchC} ms ratio=${(benchB / benchC).toFixed(2)}`);
  // Test 11: parallel observes.
  const [ob, oc] = await Promise.all([cliAsync(['observe', '--run', B]), cliAsync(['observe', '--run', C])]);
  check('11 parallel observe: own screenshot and turn counter', ob.j?.ok && oc.j?.ok && ob.j.turn === 1 && oc.j.turn === 1 && ob.j.screenshot !== oc.j.screenshot && existsSync(ob.j.screenshot) && existsSync(oc.j.screenshot), `${ob.j?.screenshot} ${oc.j?.screenshot}`);
  // Test 9: real input only.
  const bad = [['cmd', 'reset'], ['face', '3'], ['teleport', '1', '2']].map((a) => cli(['act', '--run', C, ...a]));
  check('9 act cmd/face/unknown exit non-zero', bad.every((x) => x.status !== 0 && x.j?.ok === false), bad.map((x) => `${x.status}:${x.j?.error}`).join(' | '));
  const dbg = cli(['debug', '--run', C, 'face', '3'], OP);
  check('9 debug path refused without PANEL_SELFTEST', dbg.status !== 0, dbg.j?.error);
  r = cli(['observe', '--run', C]);
  check('9 session still alive after refusals', r.status === 0 && r.j?.turn === 2, `turn=${r.j?.turn}`);
  const needle = ['__game', 'cmd'].join('.');
  const hits = readdirSync(here).filter((f) => f.endsWith('.mjs')).flatMap((f) => readFileSync(join(here, f), 'utf8').split('\n').map((l, i) => [f, i + 1, l]).filter(([, , l]) => l.includes(needle)));
  const allowed = hits.every(([f, , l]) => f === 'daemon.mjs' && (/'reset'/.test(l) || /`face \$\{i\}`/.test(l)));
  check('9 the game command hook is used only by start (reset) and the selftest debug path', allowed && hits.length === 2, hits.map(([f, n]) => `${f}:${n}`).join(' '));
  // Test 7 (second half): only waiting keeps time_to_move_s null.
  cli(['act', '--run', C, 'wait', '1500']);
  // ---- Stop both and check cleanup. ----
  const [tb, tc] = await Promise.all([cliAsync(['stop', '--run', B], OP), cliAsync(['stop', '--run', C], OP)]);
  const dead = !alive(sessB.pid) && !alive(sessC.pid);
  const closed = !(await portOpen(sessB.port)) && !(await portOpen(sessC.port));
  check('11 stop: daemons gone, ports closed', tb.j?.ok && tc.j?.ok && dead && closed, `dead=${dead} closed=${closed}`);
  const again = cli(['stop', '--run', B]);
  check('11 second stop exits 0', again.status === 0, JSON.stringify(again.j));
  writeFileSync(join(B, 'answer.json'), JSON.stringify(answer));
  writeFileSync(join(C, 'answer.json'), JSON.stringify(answer));
  const rb = cli(['report', '--run', B, '--file', join(B, 'answer.json')]).j;
  const rc = cli(['report', '--run', C, '--file', join(C, 'answer.json')]).j;
  check('10 qa result carries throttle 4', rb?.result?.throttle === 4, `throttle=${rb?.result?.throttle}`);
  check('7 only waiting: time_to_move_s stays null', rc?.result?.measured?.time_to_move_s === null, `ttm=${rc?.result?.measured?.time_to_move_s}`);
  check('S3 result.json carries cpu_bench_ms', rb?.result?.cpu_bench_ms === benchB && rc?.result?.cpu_bench_ms === benchC, `qa=${rb?.result?.cpu_bench_ms} 9yo=${rc?.result?.cpu_bench_ms}`);
  check('S1 result.json is written to the operator dir', rb?.result_path === join(opDir(B), 'result.json') && !existsSync(join(B, 'result.json')), rb?.result_path);

  // ---- Session D: negative control, a stub page whose game never advances (S4). ----
  const STUB = join(OUT, 'stub-root');
  mkdirSync(STUB, { recursive: true });
  const frozenState = { frame: 7, tick: 7, pos: [0, 1.6, 18], yaw: 0, pitch: 0, casts: 0, impacts: 0, captionId: 0, caption: '', outcome: 'playing', burning: 3, extinguished: 0, locked: false, paused: false, spell: 0, objs: [] };
  // The page keeps running JavaScript (an empty animation loop) but the game's
  // frame counter never moves: a hung game, not a dead tab.
  writeFileSync(join(STUB, 'index.html'), `<!doctype html><meta charset="utf-8"><title>stub</title><canvas id="game" width="1280" height="720" style="background:#000"></canvas>
<script>window.__gameState = ${JSON.stringify(JSON.stringify(frozenState))}; window.__game = { cmd: function () {} };
requestAnimationFrame(function loop() { requestAnimationFrame(loop); });</script>`);
  const D = join(OUT, 'd-stub');
  r = cli(['start', '--run', D, '--persona', '9yo', '--query', '', '--root', STUB], OP);
  check('S4 start on the stub page', r.status === 0 && r.j?.ok, r.j?.error || `pid=${r.j?.pid}`);
  if (r.status === 0) {
    const od = cli(['observe', '--run', D]);
    check('S4 stub: observe returns stalled:true', od.status === 0 && od.j?.stalled === true, `status=${od.status} stalled=${od.j?.stalled} ${od.j?.error || ''}`);
    const ad = cli(['act', '--run', D, 'key', 'w', '300']);
    check('S4 stub: act returns ok with stalled:true', ad.status === 0 && ad.j?.ok && ad.j?.stalled === true, `status=${ad.status} stalled=${ad.j?.stalled} ${ad.j?.error || ''}`);
    cli(['stop', '--run', D], OP);
    const md = measure(readLog(D));
    check('S4 stub: measured.stalled_turns counts both turns', md.stalled_turns === 2, `stalled_turns=${md.stalled_turns} turns=${md.turns}`);
  }
  finish();
})().catch((e) => { console.error(e); check('selftest crashed', false, e.message); finish(); });

function finish() {
  const fails = results.filter((x) => !x.ok);
  console.log(`\n${results.length - fails.length}/${results.length} passed  (run dirs: ${OUT})`);
  writeFileSync(join(OUT, 'selftest.json'), JSON.stringify(results, null, 2));
  // Leave no daemon behind.
  for (const d of ['a-9yo', 'b-qa', 'c-9yo', 'd-stub']) if (existsSync(join(OUT, d, 'session.json'))) cli(['stop', '--run', join(OUT, d)], OP);
  process.exit(fails.length ? 1 : 0);
}
