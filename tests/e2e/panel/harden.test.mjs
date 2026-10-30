// I-014 acceptance tests U1-U11 (docs/loop/items/I-014/spec.md), no browser:
// the information barrier, engaged-time minutes for B8, run quality and the
// throttle check in score, the movement rule for time_to_move_s.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { measure } from './measure.mjs';
import { scorePanel } from './score.mjs';
import { panel } from './fixtures.mjs';
import { PERSONA_IDS } from './personas.mjs';
import { authorize, layout } from './barrier.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const CLI = join(here, 'panel.mjs');
const truth = (o = {}) => ({ pos: [0, 0, 0], yaw: 0, pitch: 0, casts: 0, impacts: 0, captionId: 0, caption: '', outcome: 'playing', burning: 3, extinguished: 0, chem: '', spell: 0, locked: false, paused: false, quest: 'Save the Greenhouse', ...o });
const start = () => ({ turn: 0, cmd: 'start', persona: '9yo', throttle: 1, query: '?low', artifact_sha: 'abc1234', truth: truth() });
const obs = (t) => ({ cmd: 'observe', truth: t });
const act = (action, args, t) => ({ cmd: 'act', action, args, truth: t });
const stop = (t) => ({ cmd: 'stop', truth: t, errors: 0, panics: 0 });
const NINE = PERSONA_IDS.indexOf('9yo');
const QA = PERSONA_IDS.indexOf('qa');

function envWithout(k) { const e = { ...process.env }; delete e[k]; return e; }
function runCli(args, env) {
  return new Promise((res) => {
    const c = spawn(process.execPath, [CLI, ...args], { env, stdio: ['ignore', 'pipe', 'pipe'] });
    let out = '';
    c.stdout.on('data', (d) => { out += d; });
    c.on('close', (status) => { let j = null; try { j = JSON.parse(out.trim()); } catch { /* */ } res({ status, j, out }); });
  });
}
async function fakeDaemon() {
  const seen = [];
  const srv = createServer((req, res) => {
    let b = '';
    req.on('data', (c) => { b += c; });
    req.on('end', () => { const m = JSON.parse(b); seen.push(m); res.end(JSON.stringify({ ok: true, got: m })); });
  });
  await new Promise((r) => srv.listen(0, '127.0.0.1', r));
  const run = join(mkdtempSync(join(tmpdir(), 'panel-barrier-')), 'run');
  mkdirSync(run, { recursive: true });
  writeFileSync(join(run, 'session.json'), JSON.stringify({ pid: process.pid, port: srv.address().port, persona: '9yo' }));
  mkdirSync(`${run}.operator`, { recursive: true });
  writeFileSync(join(`${run}.operator`, 'operator.json'), JSON.stringify({ token: 'tok-123' }));
  return { run, seen, close: () => srv.close() };
}

test('U1. observe --truth and debug are refused without PANEL_ROLE=operator, before any request', async () => {
  const f = await fakeDaemon();
  try {
    const env = envWithout('PANEL_ROLE');
    for (const args of [['observe', '--run', f.run, '--truth'], ['debug', '--run', f.run, 'face', '3']]) {
      const r = await runCli(args, env);
      assert.notEqual(r.status, 0, r.out);
      assert.equal(r.j.ok, false);
      assert.match(r.j.error, /PANEL_ROLE/);
    }
    assert.equal(f.seen.length, 0, JSON.stringify(f.seen));
    // A plain observe is a persona command and goes through, without a token.
    const p = await runCli(['observe', '--run', f.run], env);
    assert.equal(p.status, 0, p.out);
    assert.equal(f.seen.length, 1);
    assert.equal(f.seen[0].truth, false);
    assert.equal(f.seen[0].token, undefined);
    // The operator's truth observe carries the token from the operator dir.
    const o = await runCli(['observe', '--run', f.run, '--truth'], { ...env, PANEL_ROLE: 'operator' });
    assert.equal(o.status, 0, o.out);
    assert.equal(f.seen[1].truth, true);
    assert.equal(f.seen[1].token, 'tok-123');
  } finally { f.close(); }
});

test('U1. start is an operator command', () => {
  const run = join(mkdtempSync(join(tmpdir(), 'panel-start-')), 'run');
  const r = spawnSync(process.execPath, [CLI, 'start', '--run', run], { encoding: 'utf8', env: envWithout('PANEL_ROLE') });
  assert.notEqual(r.status, 0);
  assert.match(JSON.parse(r.stdout).error, /PANEL_ROLE/);
  assert.equal(existsSync(join(run, 'session.json')), false);
});

test('U2. the daemon authorizes truth and debug only with the token', () => {
  assert.throws(() => authorize({ cmd: 'observe', truth: true }, 's3'), /token/);
  assert.throws(() => authorize({ cmd: 'observe', truth: true, token: 'nope' }, 's3'), /token/);
  assert.throws(() => authorize({ cmd: 'debug', action: 'face' }, 's3'), /token/);
  assert.throws(() => authorize({ cmd: 'observe', truth: true, token: '' }, ''), /token/);
  assert.doesNotThrow(() => authorize({ cmd: 'observe', truth: true, token: 's3' }, 's3'));
  assert.doesNotThrow(() => authorize({ cmd: 'debug', token: 's3' }, 's3'));
  for (const c of ['observe', 'act', 'stop', 'ping']) assert.doesNotThrow(() => authorize({ cmd: c }, 's3'));
});

test('U3. the operator dir is outside the run dir', () => {
  const l = layout('/x/panel/9yo');
  assert.equal(l.run, '/x/panel/9yo');
  assert.equal(l.op, '/x/panel/9yo.operator');
  assert.equal(layout('/x/run', '/y/op').op, '/y/op');
  assert.equal(layout('/x/run', '/x/runner').op, '/x/runner');
  assert.throws(() => layout('/x/run', '/x/run/op'), /outside/);
  assert.throws(() => layout('/x/run', '/x/run'), /outside/);
  assert.throws(() => layout('/x/run', '/x'), /outside/); // the run dir may not sit inside it either
});

test('U4. idling does not count as play: 90 x wait 10000 and 300 bare observes', () => {
  const t = truth();
  const waits = [start(), ...Array.from({ length: 90 }, () => act('wait', ['10000'], t)), stop(t)];
  const m = measure(waits);
  assert.ok(m.minutes_played < 0.5, `minutes_played=${m.minutes_played}`);
  assert.equal(m.panel_minutes, 15);
  assert.equal(m.engaged_acts, 0);
  const mixed = [start(), ...Array.from({ length: 90 }, () => [obs(t), act('wait', ['10000'], t)]).flat(), stop(t)];
  assert.ok(measure(mixed).minutes_played < 0.5, `minutes_played=${measure(mixed).minutes_played}`);
  const looks = [start(), ...Array.from({ length: 300 }, () => obs(t)), stop(t)];
  assert.equal(measure(looks).minutes_played, 0);
  // Plugged into a panel, B8 fails.
  const p = panel(); p[NINE].measured.minutes_played = m.minutes_played; p[NINE].measured.engaged_acts = m.engaged_acts;
  assert.equal(scorePanel(p).B8.pass, false);
});

test('U5. holding a key that changes nothing is capped at 0.5 s per act', () => {
  const t = truth();
  const log = [start(), ...Array.from({ length: 200 }, () => [obs(t), act('key', ['Shift', '10000'], t)]).flat(), stop(t)];
  const m = measure(log);
  assert.ok(m.minutes_played <= 200 / 60, `minutes_played=${m.minutes_played}`);
  assert.equal(m.minutes_played, Math.round((100 / 60) * 1000) / 1000);
});

test('U6. real play keeps its time: the synthetic session is all engaged (9.6 s)', () => {
  const log = [
    start(),
    obs(truth()),
    act('key', ['w', '1500'], truth({ pos: [0, 0, -1.2] })),
    act('wait', ['2000'], truth({ pos: [0, 0, -1.2] })),
    act('click', ['640', '360'], truth({ pos: [0, 0, -1.2], casts: 1, captionId: 1 })),
    obs(truth({ pos: [0, 0, -1.2], casts: 1, impacts: 1, captionId: 2 })),
    stop(truth({ pos: [0, 0, -1.2], casts: 1, impacts: 1, captionId: 2 })),
  ];
  const m = measure(log);
  assert.equal(m.minutes_played, 0.16);
  assert.equal(m.engaged_acts, 2);
  // A wait after an engaged act is credited at most 3 s, in total.
  const long = [...log.slice(0, 3), act('wait', ['10000'], truth({ pos: [0, 0, -1.2] })), act('wait', ['10000'], truth({ pos: [0, 0, -1.2] })), stop(truth({ pos: [0, 0, -1.2] }))];
  assert.equal(measure(long).minutes_played, Math.round(((3 + 1.5 + 3) / 60) * 1000) / 1000);
  // Turning the view, changing spell and pausing are engaged too.
  const t0 = truth();
  for (const t1 of [truth({ yaw: 10 }), truth({ pitch: -5 }), truth({ spell: 1 }), truth({ paused: true }), truth({ locked: true })]) {
    assert.equal(measure([start(), act('move', ['200', '0'], t1), stop(t1)]).engaged_acts, 1, JSON.stringify(t1));
  }
  assert.equal(measure([start(), act('move', ['1', '0'], truth({ yaw: 0.1 })), stop(t0)]).engaged_acts, 0);
});

test('U7. score fails a panel with bad run quality and names the persona and field', () => {
  const cases = [
    [(p) => { p[QA].throttle = 1; }, /qa.*throttle/],
    [(p) => { p[0].measured.panics = 3; }, /6yo.*panics/],
    [(p) => { p[2].stopped_because = 'error'; }, /13yo.*stopped_because/],
    [(p) => { p[3].measured.turns = 9; }, /teacher.*turns/],
    [(p) => { p[NINE].measured.stalled_turns = 20; }, /9yo.*stalled/],
  ];
  for (const [mut, re] of cases) {
    const p = panel(); mut(p);
    const l = scorePanel(p);
    for (const b of ['B4', 'B5', 'B8']) { assert.equal(l[b].pass, false, b); assert.equal(l[b].reason, 'run quality'); }
    assert.ok(l.problems.some((x) => re.test(x)), JSON.stringify(l.problems));
  }
  const thr = panel(); thr[QA].throttle = 1; thr[QA].cpu_bench_ms = 400;
  assert.equal(scorePanel(thr, { allowThrottle: true }).B5.pass, true);
  const both = panel(); both[QA].throttle = 1; both[0].measured.panics = 1;
  assert.equal(scorePanel(both, { allowThrottle: true }).B5.reason, 'run quality');
  assert.equal(scorePanel(panel()).B5.pass, true);
  assert.deepEqual(scorePanel(panel()).problems, []);
});

test('U8. B4 needs the qa throttle to show: cpu_bench_ms qa / 9yo > 2', () => {
  const at = (qa) => { const p = panel(); if (qa === undefined) delete p[QA].cpu_bench_ms; else p[QA].cpu_bench_ms = qa; return scorePanel(p).B4; };
  assert.equal(at(400).pass, true);
  assert.equal(at(400).throttle_ok, true);
  assert.equal(at(400).throttle_ratio, 4);
  assert.equal(at(200).pass, false);
  assert.equal(at(200).throttle_ok, false);
  assert.equal(at(undefined).pass, false);
  assert.equal(at(undefined).throttle_ok, false);
});

test('U9. time_to_move: a movement key that moved at all counts; other keys and waits do not', () => {
  const near = truth({ pos: [0, 0, -0.3] });
  const w = measure([start(), obs(truth()), act('key', ['w', '1500'], near), stop(near)]);
  assert.equal(w.time_to_move_s, 4.5);
  const chord = measure([start(), obs(truth()), act('keys', ['w+Shift', '1000'], near), stop(near)]);
  assert.equal(chord.time_to_move_s, 4);
  const shift = measure([start(), obs(truth()), act('key', ['Shift', '1500'], near), stop(near)]);
  assert.equal(shift.time_to_move_s, null);
  const wait = measure([start(), obs(truth()), act('wait', ['1500'], near), stop(near)]);
  assert.equal(wait.time_to_move_s, null);
  const still = measure([start(), obs(truth()), act('key', ['w', '100'], truth({ pos: [0, 0, -0.01] })), stop(truth())]);
  assert.equal(still.time_to_move_s, null);
});

test('U10. report refuses a log without a stop entry', () => {
  const d = mkdtempSync(join(tmpdir(), 'panel-nostop-'));
  writeFileSync(join(d, 'log.jsonl'), [start(), obs(truth())].map((e) => JSON.stringify(e)).join('\n') + '\n');
  const a = { model: 'haiku', clarity: 7, fun: 6, blockers: [], would_play_again: 'maybe', best: 'x', worst: 'y', quotes: [], stopped_because: 'quit' };
  writeFileSync(join(d, 'answer.json'), JSON.stringify(a));
  const r = spawnSync(process.execPath, [CLI, 'report', '--run', d, '--file', join(d, 'answer.json')], { encoding: 'utf8' });
  assert.notEqual(r.status, 0, r.stdout);
  assert.match(JSON.parse(r.stdout).error, /stop/);
  assert.equal(existsSync(join(d, 'result.json')), false);
});

test('U11. B8 needs engaged acts: at least 2 per minute played', () => {
  const p = panel(); p[NINE].measured.minutes_played = 16; p[NINE].measured.engaged_acts = 31;
  const l = scorePanel(p);
  assert.equal(l.B8.pass, false);
  assert.equal(l.B8.engaged_acts_9yo, 31);
  p[NINE].measured.engaged_acts = 32;
  assert.equal(scorePanel(p).B8.pass, true);
});
