// Measured fields (docs/loop/panel.md, "Measured fields"), computed from a
// session's log.jsonl only. The persona never reports these.
import { panelClock } from './clock.mjs';

const r3 = (x) => Math.round(x * 1000) / 1000;
const dist2d = (a, b) => Math.hypot(a[0] - b[0], a[2] - b[2]);

// Captions the game writes on its own, not as the result of a cast
// (src/main.rs `react`): a neighbour catching fire, the greenhouse igniting.
const SPONTANEOUS = /^(Heat from a neighbour|Oh no!)/;

// Did a cast land between two truth samples? The projectile hit something
// (impacts rose) and the game captioned a spell result. Every cast writes one
// caption as it leaves the hand (src/main.rs: `game.say(spell.cast_caption())`),
// so those are subtracted. A cast into the sky raises impacts but writes no
// result caption. A lone spontaneous caption is not a result.
export function landed(prev, t) {
  if (!(t.impacts > prev.impacts)) return false;
  const results = (t.captionId - prev.captionId) - Math.max(0, t.casts - prev.casts);
  return results >= 2 || (results === 1 && !SPONTANEOUS.test(t.caption || ''));
}

export function measure(log) {
  const start = log.find((e) => e.cmd === 'start');
  if (!start) throw new Error('log has no start entry');
  const clock = panelClock(log, start.persona);
  const spawn = start.truth.pos;
  let ttm = null; let ttc = null; let castAct = -1;
  let prev = start.truth;
  let prevOutcome = prev.outcome;
  const quests = new Set();
  let turns = 0; let errors = 0; let panics = 0; let cap = null;
  for (let i = 0; i < log.length; i++) {
    const e = log[i];
    if (e.cmd === 'observe' || e.cmd === 'act') turns++;
    if (e.errors !== undefined) errors = e.errors;
    if (e.panics !== undefined) panics = e.panics;
    if (e.cap) cap = e.cap;
    const t = e.truth;
    if (!t || e.cmd === 'start') continue;
    if (e.cmd === 'act') {
      if (ttm === null && dist2d(t.pos, spawn) > 0.5) ttm = clock[i];
      if (t.casts > prev.casts) castAct = i;
    }
    if (ttc === null && castAct >= 0 && landed(prev, t)) ttc = clock[castAct];
    if (t.outcome === 'won' && prevOutcome !== 'won') quests.add(t.quest || 'quest');
    prevOutcome = t.outcome;
    prev = t;
  }
  return {
    time_to_move_s: ttm === null ? null : r3(ttm),
    time_to_cast_s: ttc === null ? null : r3(ttc),
    minutes_played: r3((clock[clock.length - 1] || 0) / 60),
    quests_completed: quests.size,
    outcome: prev.outcome,
    turns,
    console_errors: errors,
    panics,
    cap,
  };
}
