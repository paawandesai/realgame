// Acceptance tests 4 and 5: self-report validation, measured fields from the
// log, and the panel clock (no browser).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { panelClock, costMs } from './clock.mjs';
import { measure } from './measure.mjs';
import { validate, loadSchema } from './schema.mjs';
import { makeResult } from './fixtures.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const CLI = join(here, 'panel.mjs');

const truth = (o = {}) => ({ pos: [0, 0, 0], yaw: 0, casts: 0, impacts: 0, captionId: 0, caption: '', outcome: 'playing', burning: 3, extinguished: 0, chem: '', quest: 'Save the Greenhouse', ...o });

// A small synthetic session log: start, observe, key w, wait, click, observe, stop.
function syntheticLog() {
  return [
    { turn: 0, cmd: 'start', persona: '9yo', throttle: 1, query: '?low', artifact_sha: 'abc1234', truth: truth() },
    { turn: 1, cmd: 'observe', truth: truth() },
    { turn: 2, cmd: 'act', action: 'key', args: ['w', '1500'], truth: truth({ pos: [0, 0, -1.2] }) },
    { turn: 3, cmd: 'act', action: 'wait', args: ['2000'], truth: truth({ pos: [0, 0, -1.2] }) },
    // Each cast writes its own caption as it leaves the hand.
    { turn: 4, cmd: 'act', action: 'click', args: ['640', '360'], truth: truth({ pos: [0, 0, -1.2], casts: 1, captionId: 1, caption: 'Water: soaks up heat.' }) },
    { turn: 5, cmd: 'observe', truth: truth({ pos: [0, 0, -1.2], casts: 1, impacts: 1, captionId: 2, caption: 'The water cooled the bush below its ignition temperature.', chem: '0' }) },
    { turn: 5, cmd: 'stop', truth: truth({ pos: [0, 0, -1.2], casts: 1, impacts: 1, captionId: 2, chem: '0' }), errors: 0, panics: 0 },
  ];
}

test('5. panel clock: 3 + 1.5 + 2 + 0.1 + 3 = 9.6, deterministic', () => {
  const log = syntheticLog();
  const c = panelClock(log, '9yo');
  assert.equal(c[c.length - 1], 9.6);
  assert.deepEqual(panelClock(syntheticLog(), '9yo'), c);
  assert.equal(costMs({ cmd: 'observe' }, '6yo'), 4000);
  assert.equal(costMs({ cmd: 'observe' }, '13yo'), 2000);
  assert.equal(costMs({ cmd: 'act', action: 'hold', args: ['640', '360', '600'] }, '9yo'), 600);
  assert.equal(costMs({ cmd: 'act', action: 'keys', args: ['w+Shift', '2000'] }, '9yo'), 2000);
  assert.equal(costMs({ cmd: 'act', action: 'move', args: ['200', '0'] }, '9yo'), 100);
});

test('measure: time to move, time to cast, minutes, quests', () => {
  const m = measure(syntheticLog());
  assert.equal(m.time_to_move_s, 4.5); // end of `key w 1500`: 3 + 1.5
  assert.equal(m.time_to_cast_s, 6.6); // end of the click that cast: 3 + 1.5 + 2 + 0.1
  assert.equal(m.minutes_played, 0.16);
  assert.equal(m.turns, 5);
  assert.equal(m.quests_completed, 0);
  assert.equal(m.outcome, 'playing');
});

test('measure: a cast into the sky does not set time_to_cast; no movement keeps ttm null', () => {
  const log = syntheticLog();
  for (const e of log) { e.truth.pos = [0, 0, 0]; }
  log[5].truth.captionId = 1; log[5].truth.caption = 'Water: soaks up heat.'; log[5].truth.chem = '';
  log[6].truth.captionId = 1;
  const m = measure(log);
  assert.equal(m.time_to_move_s, null);
  assert.equal(m.time_to_cast_s, null);
});

test('measure: a sky cast whose cast caption and impact fall in one sample is not a landed cast', () => {
  const log = syntheticLog();
  log[4].truth = truth({ pos: [0, 0, -1.2], casts: 1, impacts: 1, captionId: 1, caption: 'Water: soaks up heat.' });
  log[5].truth = { ...log[4].truth };
  log[6].truth = { ...log[4].truth };
  assert.equal(measure(log).time_to_cast_s, null);
  // The same sample with a result caption on top is a landed cast.
  log[4].truth = { ...log[4].truth, captionId: 2, caption: 'The water cooled the bush.' };
  assert.equal(measure(log).time_to_cast_s, 6.6);
});

test('measure: a neighbour-ignition caption during a sky cast is not a landed cast', () => {
  const log = syntheticLog();
  log[5].truth.caption = 'Heat from a neighbour raised the bush to its ignition temperature: it caught fire!';
  assert.equal(measure(log).time_to_cast_s, null);
});

test('the schema file is draft 2020-12 and validates a good result', () => {
  const s = loadSchema();
  assert.equal(s.$schema, 'https://json-schema.org/draft/2020-12/schema');
  assert.deepEqual(validate(makeResult('9yo')), []);
  assert.match(validate({ ...makeResult('9yo'), schema: 'panel-result/0' })[0], /schema/);
});

function runDir() {
  const d = mkdtempSync(join(tmpdir(), 'panel-report-'));
  writeFileSync(join(d, 'log.jsonl'), syntheticLog().map((e) => JSON.stringify(e)).join('\n') + '\n');
  return d;
}
const answer = (o = {}) => ({ model: 'haiku', clarity: 7, fun: 6, blockers: [{ kind: 'goal', what: 'I did not know which bush', turn: 3 }], would_play_again: 'maybe', best: 'The water spell.', worst: 'Walking was slow.', quotes: ['I want to put the fire out'], stopped_because: 'quit', ...o });
function report(d, a) {
  const f = join(d, 'answer.json');
  writeFileSync(f, JSON.stringify(a));
  return spawnSync(process.execPath, [CLI, 'report', '--run', d, '--file', f], { encoding: 'utf8' });
}

test('4. report rejects bad answers and names the field', () => {
  const cases = [
    [answer({ clarity: 11 }), /clarity/],
    [answer({ fun: 0 }), /fun/],
    [(() => { const a = answer(); delete a.blockers; return a; })(), /blockers/],
    [answer({ blockers: [{ kind: 'other', what: 'x', turn: 1 }] }), /blockers.*kind|kind/],
  ];
  for (const [a, re] of cases) {
    const d = runDir();
    const r = report(d, a);
    assert.notEqual(r.status, 0, r.stdout);
    const j = JSON.parse(r.stdout);
    assert.equal(j.ok, false);
    assert.match(j.error, re);
    assert.equal(existsSync(join(d, 'result.json')), false);
  }
});

test('4. a valid answer writes result.json with measured from the log; answer values lose', () => {
  const d = runDir();
  const r = report(d, answer({ measured: { time_to_cast_s: 1 }, time_to_cast_s: 1 }));
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stderr, /warning/i);
  const res = JSON.parse(readFileSync(join(d, 'result.json'), 'utf8'));
  assert.equal(res.schema, 'panel-result/1');
  assert.equal(res.persona, '9yo');
  assert.equal(res.measured.time_to_cast_s, 6.6);
  assert.equal(res.measured.time_to_move_s, 4.5);
  assert.equal(res.self.clarity, 7);
  assert.equal(res.query, '?low');
  assert.equal(res.artifact_sha, 'abc1234');
  assert.equal(res.time_to_cast_s, undefined);
  assert.deepEqual(validate(res), []);
});
