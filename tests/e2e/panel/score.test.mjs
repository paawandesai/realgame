// Acceptance tests 1-3: panel scoring for B4, B5 and B8 (no browser).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { scorePanel } from './score.mjs';
import { makeResult, panel } from './fixtures.mjs';
import { PERSONA_IDS } from './personas.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const CLI = join(here, 'panel.mjs');

test('1. B4 passes on the spec example and pins median clarity', () => {
  const l = scorePanel(panel());
  assert.equal(l.kind, 'panel');
  assert.equal(l.source, 'panel');
  assert.equal(l.B4.pass, true);
  assert.equal(l.B4.median_clarity, 8);
  assert.equal(l.B4.all_move_10s, true);
  assert.equal(l.B4.all_cast_30s, true);
  assert.equal(l.B4.control_blockers, 0);
  assert.deepEqual(l.personas, PERSONA_IDS);
});

test('1. B4 fails on ttc 31, on ttc null, on a control blocker; text blocker still passes', () => {
  assert.equal(scorePanel(panel({ ttc: [10, 12, 20, 25, 31] })).B4.pass, false);
  assert.equal(scorePanel(panel({ ttc: [10, 12, null, 25, 30] })).B4.pass, false);
  assert.equal(scorePanel(panel({ ttm: [3, 5, 8, 9, 10.1] })).B4.pass, false);
  assert.equal(scorePanel(panel({ ttm: [null, 5, 8, 9, 10] })).B4.pass, false);
  const ctl = scorePanel(panel({ blockers: [[], [{ kind: 'control', what: 'could not turn', turn: 4 }], [], [], []] }));
  assert.equal(ctl.B4.pass, false);
  assert.equal(ctl.B4.control_blockers, 1);
  assert.equal(ctl.per_persona['9yo'].blockers_control, 1);
  const txt = scorePanel(panel({ blockers: [[], [{ kind: 'text', what: 'long words', turn: 4 }], [], [], []] }));
  assert.equal(txt.B4.pass, true);
  assert.equal(scorePanel(panel({ clarity: [8, 7, 9, 7, 10] })).B4.pass, true); // sorted 7,7,8,9,10: median 8
  assert.equal(scorePanel(panel({ clarity: [8, 7, 9, 7, 7] })).B4.median_clarity, 7);
  assert.equal(scorePanel(panel({ clarity: [8, 7, 9, 7, 7] })).B4.pass, false);
});

test('2. B5 median and minimum fun', () => {
  assert.equal(scorePanel(panel({ fun: [8, 8, 8, 9, 6] })).B5.pass, true);
  const low = scorePanel(panel({ fun: [8, 8, 8, 9, 5] }));
  assert.equal(low.B5.pass, false);
  assert.equal(low.B5.min_fun, 5);
  const mid = scorePanel(panel({ fun: [7, 7, 8, 9, 9] }));
  assert.equal(mid.B5.pass, true);
  assert.equal(mid.B5.median_fun, 8);
  assert.equal(mid.B5.min_fun, 7);
  const m7 = scorePanel(panel({ fun: [7, 7, 7, 9, 9] }));
  assert.equal(m7.B5.pass, false);
  assert.equal(m7.B5.median_fun, 7);
});

test('2. B8 uses the 9yo minutes', () => {
  const nine = PERSONA_IDS.indexOf('9yo');
  const mins = (v) => PERSONA_IDS.map((_, i) => (i === nine ? v : 1));
  const a = scorePanel(panel({ minutes: mins(14.9) }));
  assert.equal(a.B8.pass, false);
  assert.equal(a.B8.minutes_9yo, 14.9);
  assert.equal(a.B8.content_check, 'manual');
  assert.equal(scorePanel(panel({ minutes: mins(15.0) })).B8.pass, true);
  assert.equal(typeof a.B8.quests_completed_9yo, 'number');
});

test('3. incomplete, duplicate and mismatched panels', () => {
  const four = scorePanel(panel().slice(0, 4));
  for (const b of ['B4', 'B5', 'B8']) {
    assert.equal(four[b].pass, false);
    assert.equal(four[b].reason, 'incomplete panel');
  }
  const dup = scorePanel(PERSONA_IDS.map(() => makeResult('9yo')));
  for (const b of ['B4', 'B5', 'B8']) { assert.equal(dup[b].pass, false); assert.equal(dup[b].reason, 'incomplete panel'); }
  const sha = panel(); sha[2].artifact_sha = 'def5678';
  const ms = scorePanel(sha);
  for (const b of ['B4', 'B5', 'B8']) { assert.equal(ms[b].pass, false); assert.equal(ms[b].reason, 'mismatched runs'); }
  const q = panel(); q[0].query = '';
  assert.equal(scorePanel(q).B5.reason, 'mismatched runs');
});

test('3. a panel-result/0 is rejected by name (CLI score)', () => {
  const root = mkdtempSync(join(tmpdir(), 'panel-score-'));
  const dirs = panel().map((r, i) => {
    const d = join(root, `r${i}`);
    spawnSync('mkdir', ['-p', d]);
    if (i === 1) r.schema = 'panel-result/0';
    writeFileSync(join(d, 'result.json'), JSON.stringify(r));
    return d;
  });
  const out = spawnSync(process.execPath, [CLI, 'score', '--runs', ...dirs], { encoding: 'utf8' });
  const j = JSON.parse(out.stdout);
  assert.equal(j.ok, true);
  assert.ok(j.rejected.length === 1 && /schema/.test(j.rejected[0].error), JSON.stringify(j.rejected));
  assert.equal(j.line.B4.reason, 'incomplete panel');
  // --append only when asked.
  assert.equal(existsSync(join(root, 'sb.jsonl')), false);
  const good = panel().map((r, i) => { const d = join(root, `g${i}`); spawnSync('mkdir', ['-p', d]); writeFileSync(join(d, 'result.json'), JSON.stringify(r)); return d; });
  const out2 = spawnSync(process.execPath, [CLI, 'score', '--runs', ...good, '--append', join(root, 'sb.jsonl'), '--artifact', 'abc1234'], { encoding: 'utf8' });
  assert.equal(out2.status, 0, out2.stdout + out2.stderr);
  const lines = readFileSync(join(root, 'sb.jsonl'), 'utf8').trim().split('\n');
  assert.equal(lines.length, 1);
  const l = JSON.parse(lines[0]);
  assert.equal(l.kind, 'panel');
  assert.equal(l.B4.pass, true);
  assert.ok(l.ts);
});
