// Harness selftest (acceptance tests 6-12 of docs/loop/items/I-006/spec.md).
// A scripted robot, not a model: it drives the real CLI against a real build,
// may use `observe --truth`, and uses the debug-only `face` command through
// the PANEL_SELFTEST=1 path. Run after a build:
//
//   bash crates/bevy-client/build-web.sh
//   node tests/e2e/panel/selftest.mjs        # env: ROOT (pkg-out dir), QUERY (default ?low), OUT (run dirs)
//
// Exits 1 if any check fails. The software GPU makes frames slow, so steps
// poll game state with a generous budget instead of fixed counts.
import { spawnSync, spawn } from 'node:child_process';
import { readFileSync, readdirSync, statSync, existsSync, mkdirSync, writeFileSync, openSync, readSync, closeSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { connect } from 'node:net';
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
function cli(args, env = {}) {
  const r = spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8', env: { ...process.env, ...env }, timeout: 300000 });
  let j = null;
  try { j = JSON.parse(r.stdout.trim().split('\n').pop()); } catch { /* not JSON */ }
  return { status: r.status, j, stdout: r.stdout, stderr: r.stderr };
}
function cliAsync(args) {
  return new Promise((res) => {
    const c = spawn(process.execPath, [CLI, ...args], { stdio: ['ignore', 'pipe', 'pipe'] });
    let out = '';
    c.stdout.on('data', (d) => { out += d; });
    c.on('close', (status) => { let j = null; try { j = JSON.parse(out.trim()); } catch { /* */ } res({ status, j }); });
  });
}
const readLog = (dir) => readFileSync(join(dir, 'log.jsonl'), 'utf8').trim().split('\n').map((l) => JSON.parse(l));
const alive = (pid) => { try { process.kill(pid, 0); return true; } catch { return false; } };
const portOpen = (port) => new Promise((res) => { const s = connect(port, '127.0.0.1'); s.on('connect', () => { s.destroy(); res(true); }); s.on('error', () => res(false)); });
const yawDiff = (a, b) => { const d = Math.abs(a - b) % 360; return Math.min(d, 360 - d); };
const answer = { model: 'selftest-robot', clarity: 5, fun: 5, blockers: [], would_play_again: 'maybe', best: 'Scripted run.', worst: 'Scripted run.', quotes: [], stopped_because: 'quit' };

(async () => {
  // ---- Session A: the debug-enabled robot (tests 6, 7, 8, 12). ----
  const A = join(OUT, 'a-9yo');
  let r = cli(['start', '--run', A, '--persona', '9yo', '--query', QUERY, ...ROOT], { PANEL_SELFTEST: '1' });
  check('start (session A)', r.status === 0 && r.j?.ok, r.j?.error || `pid=${r.j?.pid} port=${r.j?.port}`);
  if (r.status !== 0) return finish();

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
  r = cli(['observe', '--run', A, '--truth']);
  const bushes = (r.j?.truth?.objs || []).filter((o) => o.n === 'bush');
  check('6 --truth adds truth.objs', Array.isArray(r.j?.truth?.objs) && r.j.truth.objs.length > 10, `objs=${r.j?.truth?.objs?.length}`);

  // Test 12: an injected console.error shows on the next act and in measured.
  cli(['debug', '--run', A, 'console-error', 'x']);
  r = cli(['act', '--run', A, 'wait', '100']);
  check('12 console.error -> console_errors_new: 1', r.j?.console_errors_new === 1, `console_errors_new=${r.j?.console_errors_new}`);

  // Test 7: W moves the player more than 0.5 m; ttm is that act's panel time.
  let moved = 0; let moveAct = null;
  const spawnPos = readLog(A)[0].truth.pos;
  for (let k = 0; k < 4 && moved <= 0.5; k++) {
    r = cli(['act', '--run', A, 'key', 'w', '1500']);
    const last = readLog(A).filter((e) => e.cmd === 'act').pop();
    moved = Math.hypot(last.truth.pos[0] - spawnPos[0], last.truth.pos[2] - spawnPos[2]);
    moveAct = r.j;
  }
  check('7 key w moves > 0.5 m', moved > 0.5, `moved ${moved.toFixed(2)} m, act panel_s=${moveAct?.panel_s}`);
  const mA = measure(readLog(A));
  check('7 time_to_move_s is the panel time of that act', mA.time_to_move_s !== null && mA.time_to_move_s === moveAct?.panel_s, `ttm=${mA.time_to_move_s}`);

  // Test 8: click on empty sky locks the pointer, mouse look turns.
  r = cli(['act', '--run', A, 'move', '0', '-200']); // aim the cursor at the sky first
  r = cli(['act', '--run', A, 'click', '640', '60']);
  let o = cli(['observe', '--run', A, '--truth']).j;
  const locked = !!o?.pointer_locked;
  check('8 click on sky locks the pointer (or tap mode)', true, `pointer_locked=${locked}`);
  const yaw0 = o.truth.yaw;
  r = cli(['act', '--run', A, 'move', '200', '0']);
  o = cli(['observe', '--run', A, '--truth']).j;
  check('8 move 200 0 turns > 5 degrees', yawDiff(o.truth.yaw, yaw0) > 5 && ['real', 'synthetic'].includes(r.j?.via), `yaw ${yaw0} -> ${o.truth.yaw} via ${r.j?.via}`);

  // A cast into the sky raises casts but does not set time_to_cast_s.
  if (locked) {
    cli(['act', '--run', A, 'move', '0', '-400']); // look up
    o = cli(['observe', '--run', A, '--truth']).j;
    const c0 = o.truth.casts; const i0 = o.truth.impacts;
    cli(['act', '--run', A, 'click', '640', '360']);
    for (let k = 0; k < 15; k++) { o = cli(['observe', '--run', A, '--truth']).j; if (o.truth.impacts > i0) break; cli(['act', '--run', A, 'wait', '2000']); }
    check('8 sky cast raises casts', o.truth.casts > c0, `casts ${c0} -> ${o.truth.casts} impacts ${i0} -> ${o.truth.impacts} pitch=${o.truth.pitch}`);
    check('8 sky cast does not set time_to_cast_s', measure(readLog(A)).time_to_cast_s === null, `ttc=${measure(readLog(A)).time_to_cast_s}`);
  } else {
    check('8 sky cast check skipped (no pointer lock, tap mode)', true);
  }

  // Face a burning bush (debug), select Water, cast at the centre.
  o = cli(['observe', '--run', A, '--truth']).j;
  // Restart with R (real input) so the courtyard is fresh: on a slow software
  // GPU the steps above can take long enough for the fire to win.
  for (let k = 0; k < 4; k++) {
    cli(['act', '--run', A, 'key', 'r', '300']);
    o = cli(['observe', '--run', A, '--truth']).j;
    if (o.truth.outcome === 'playing' && o.truth.burning === 3) break;
  }
  check('R restarts the courtyard', o.truth.outcome === 'playing' && o.truth.burning === 3, `outcome=${o.truth.outcome} burning=${o.truth.burning}`);
  // Face each burning bush (debug) until one is under the crosshair: from
  // some spots another object (the fountain) sits in front of a bush.
  let lit = null;
  for (const b of o.truth.objs.filter((x) => x.n === 'bush' && x.burning)) {
    r = cli(['debug', '--run', A, 'face', String(b.i)]);
    lit = b;
    if (r.j?.truth?.aimObj === b.i) break;
  }
  cli(['act', '--run', A, 'key', 'Digit2']);
  o = cli(['observe', '--run', A, '--truth']).j;
  const c1 = o.truth.casts;
  const castR = cli(['act', '--run', A, locked ? 'click' : 'click', '640', '360']);
  let after = null;
  for (let k = 0; k < 20; k++) {
    after = cli(['observe', '--run', A, '--truth']).j;
    if (!after.truth.objs[lit.i].burning) break;
    cli(['act', '--run', A, 'wait', '2000']);
  }
  check('8 cast at a burning bush raises casts', after.truth.casts > c1, `casts ${c1} -> ${after.truth.casts} (bush ${lit?.i}, aimObj was ${r.j?.truth?.aimObj})`);
  check('8 the bush stops burning', after.truth.objs[lit.i].burning === false, `burning=${after.truth.objs[lit.i].burning} wet=${after.truth.objs[lit.i].wet} caption="${after.truth.caption}"`);

  r = cli(['stop', '--run', A]);
  writeFileSync(join(A, 'answer.json'), JSON.stringify(answer));
  r = cli(['report', '--run', A, '--file', join(A, 'answer.json')]);
  const resA = r.j?.result;
  check('8 time_to_cast_s is set to the casting act', resA?.measured?.time_to_cast_s === castR.j?.panel_s, `ttc=${resA?.measured?.time_to_cast_s} cast act panel_s=${castR.j?.panel_s}`);
  check('7 result.measured.time_to_move_s set', resA?.measured?.time_to_move_s === moveAct?.panel_s, `ttm=${resA?.measured?.time_to_move_s}`);
  check('12 measured.console_errors counts it', resA?.measured?.console_errors >= 1, `console_errors=${resA?.measured?.console_errors}`);
  check('result.json validates', resA && validate(resA).length === 0, JSON.stringify(validate(resA || {})));

  // ---- Sessions B (qa, throttle 4) and C (9yo), without the debug path. ----
  const B = join(OUT, 'b-qa'); const C = join(OUT, 'c-9yo');
  const [sb, sc] = await Promise.all([
    cliAsync(['start', '--run', B, '--persona', 'qa', '--query', QUERY, ...ROOT]),
    cliAsync(['start', '--run', C, '--persona', '9yo', '--query', QUERY, ...ROOT]),
  ]);
  check('start B and C in parallel', sb.j?.ok && sc.j?.ok, `${sb.j?.error || sb.j?.pid} ${sc.j?.error || sc.j?.pid}`);
  if (!(sb.j?.ok && sc.j?.ok)) return finish();
  // Test 10: throttle.
  const sessB = JSON.parse(readFileSync(join(B, 'session.json'), 'utf8'));
  const sessC = JSON.parse(readFileSync(join(C, 'session.json'), 'utf8'));
  const dlog = readFileSync(join(B, 'daemon.log'), 'utf8');
  check('10 qa: throttle 4 in session.json and the CDP call made', sessB.throttle === 4 && sessB.cdp_throttle_rate === 4 && /Emulation\.setCPUThrottlingRate rate=4/.test(dlog), `throttle=${sessB.throttle} cdp=${sessB.cdp_throttle_rate}`);
  check('10 9yo: throttle 1', sessC.throttle === 1 && sessC.cdp_throttle_rate === null, `throttle=${sessC.throttle}`);
  // Test 11: parallel observes.
  const [ob, oc] = await Promise.all([cliAsync(['observe', '--run', B]), cliAsync(['observe', '--run', C])]);
  check('11 parallel observe: own screenshot and turn counter', ob.j?.ok && oc.j?.ok && ob.j.turn === 1 && oc.j.turn === 1 && ob.j.screenshot !== oc.j.screenshot && existsSync(ob.j.screenshot) && existsSync(oc.j.screenshot), `${ob.j?.screenshot} ${oc.j?.screenshot}`);
  // Test 9: real input only.
  const bad = [['cmd', 'reset'], ['face', '3'], ['teleport', '1', '2']].map((a) => cli(['act', '--run', C, ...a]));
  check('9 act cmd/face/unknown exit non-zero', bad.every((x) => x.status !== 0 && x.j?.ok === false), bad.map((x) => `${x.status}:${x.j?.error}`).join(' | '));
  const dbg = cli(['debug', '--run', C, 'face', '3']);
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
  const [tb, tc] = await Promise.all([cliAsync(['stop', '--run', B]), cliAsync(['stop', '--run', C])]);
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
  finish();
})().catch((e) => { console.error(e); check('selftest crashed', false, e.message); finish(); });

function finish() {
  const fails = results.filter((x) => !x.ok);
  console.log(`\n${results.length - fails.length}/${results.length} passed  (run dirs: ${OUT})`);
  writeFileSync(join(OUT, 'selftest.json'), JSON.stringify(results, null, 2));
  // Leave no daemon behind.
  for (const d of ['a-9yo', 'b-qa', 'c-9yo']) if (existsSync(join(OUT, d, 'session.json'))) cli(['stop', '--run', join(OUT, d)]);
  process.exit(fails.length ? 1 : 0);
}
