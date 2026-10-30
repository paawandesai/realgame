// Measured fields (docs/loop/panel.md, "Measured fields"), computed from a
// session's log.jsonl only. The persona never reports these.
import { panelClock, costMs } from './clock.mjs';

const r3 = (x) => Math.round(x * 1000) / 1000;
const dist2d = (a, b) => Math.hypot(a[0] - b[0], a[2] - b[2]);
const angle = (a, b) => { const d = Math.abs((a ?? 0) - (b ?? 0)) % 360; return Math.min(d, 360 - d); };

// Captions the game writes on its own, not as the result of a cast
// (src/main.rs `react`): a neighbour catching fire, the greenhouse igniting.
const SPONTANEOUS = /^(Heat from a neighbour|Oh no!)/;

// Did a cast land between two truth samples? The projectile hit something
// (impacts rose) and the game captioned a spell result. Every cast writes one
// caption as it leaves the hand (src/main.rs: `game.say(spell.cast_caption())`),
// so those are subtracted. A cast into the sky raises impacts but writes no
// result caption. A lone spontaneous caption is not a result.
// I-014 kept this caption rule over a chemistry diff: the chem flags are
// coarse (heating a bush that does not ignite flips no flag) and they change
// on their own (spread, drying), which would credit a sky cast.
export function landed(prev, t) {
  if (!(t.impacts > prev.impacts)) return false;
  const results = (t.captionId - prev.captionId) - Math.max(0, t.casts - prev.casts);
  return results >= 2 || (results === 1 && !SPONTANEOUS.test(t.caption || ''));
}

// Engagement (I-014, B8): thresholds for "the player's state changed".
export const MOVE_EPS_M = 0.05;
export const TURN_EPS_DEG = 0.5;
export const IDLE_INPUT_CREDIT_MS = 500; // a non-wait act that changed nothing
export const WAIT_CREDIT_MS = 3000; // waits right after an engaged act, in total
const MOVE_KEY = /^(w|a|s|d|W|A|S|D|KeyW|KeyA|KeyS|KeyD|ArrowUp|ArrowDown|ArrowLeft|ArrowRight)$/;

// Did the player's own state change across an act? Fire spreading, drying
// and captions change on their own, so they are not counted.
export function changed(prev, t) {
  if (!prev || !t) return false;
  if (prev.pos && t.pos && dist2d(prev.pos, t.pos) >= MOVE_EPS_M) return true;
  if (angle(prev.yaw, t.yaw) >= TURN_EPS_DEG || angle(prev.pitch, t.pitch) >= TURN_EPS_DEG) return true;
  if ((t.casts ?? 0) > (prev.casts ?? 0)) return true;
  return prev.spell !== t.spell || !!prev.paused !== !!t.paused || !!prev.locked !== !!t.locked || prev.outcome !== t.outcome;
}

const movementKeyAct = (e) => (e.action === 'key' && MOVE_KEY.test(e.args?.[0] || ''))
  || (e.action === 'keys' && (e.args?.[0] || '').split('+').some((k) => MOVE_KEY.test(k)));

// Engaged panel milliseconds after each log entry (B8 "minutes played"):
//   engaged act (real input other than wait that changed the player's state): its full input time;
//   other non-wait act: at most IDLE_INPUT_CREDIT_MS;
//   wait: only after an engaged act, at most WAIT_CREDIT_MS in total until the next engaged act;
//   observe: its reading cost when the act before or after it is engaged.
export function engagement(log, personaId) {
  const p = personaId ?? log.find((e) => e.cmd === 'start')?.persona;
  const engaged = new Array(log.length).fill(false);
  let prev = null;
  for (let i = 0; i < log.length; i++) {
    const e = log[i];
    if (e.cmd === 'act' && e.action !== 'wait' && changed(prev, e.truth)) engaged[i] = true;
    if (e.truth) prev = e.truth;
  }
  const acts = log.map((e, i) => (e.cmd === 'act' ? i : -1)).filter((i) => i >= 0);
  const prevAct = (i) => { let k = -1; for (const a of acts) { if (a < i) k = a; else break; } return k; };
  const nextAct = (i) => acts.find((a) => a > i) ?? -1;
  let ms = 0; let waitBudget = 0; let count = 0;
  const out = log.map((e, i) => {
    const c = costMs(e, p);
    if (e.cmd === 'act') {
      if (engaged[i]) { ms += c; count++; waitBudget = WAIT_CREDIT_MS; } else if (e.action === 'wait') { const w = Math.min(c, waitBudget); ms += w; waitBudget -= w; } else { ms += Math.min(c, IDLE_INPUT_CREDIT_MS); waitBudget = 0; }
    } else if (e.cmd === 'observe') {
      const a = prevAct(i); const b = nextAct(i);
      if ((a >= 0 && engaged[a]) || (b >= 0 && engaged[b])) ms += c;
    }
    return ms;
  });
  return { activeMs: out, engaged, engagedActs: count };
}

export function measure(log) {
  const start = log.find((e) => e.cmd === 'start');
  if (!start) throw new Error('log has no start entry');
  const clock = panelClock(log, start.persona);
  const eng = engagement(log, start.persona);
  const spawn = start.truth.pos;
  let ttm = null; let ttc = null; let castAct = -1;
  let prev = start.truth;
  let prevOutcome = prev.outcome;
  const quests = new Set();
  let turns = 0; let errors = 0; let panics = 0; let cap = null; let stalled = 0;
  for (let i = 0; i < log.length; i++) {
    const e = log[i];
    if (e.cmd === 'observe' || e.cmd === 'act') turns++;
    if (e.stalled && (e.cmd === 'observe' || e.cmd === 'act')) stalled++;
    if (e.errors !== undefined) errors = e.errors;
    if (e.panics !== undefined) panics = e.panics;
    if (e.cap) cap = e.cap;
    const t = e.truth;
    if (!t || e.cmd === 'start') continue;
    if (e.cmd === 'act') {
      // A movement key that moved the player at all, or more than 0.5 m from spawn.
      const keyMoved = movementKeyAct(e) && prev.pos && t.pos && dist2d(prev.pos, t.pos) >= MOVE_EPS_M;
      if (ttm === null && (keyMoved || dist2d(t.pos, spawn) > 0.5)) ttm = clock[i];
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
    minutes_played: r3((eng.activeMs[eng.activeMs.length - 1] || 0) / 60000),
    quests_completed: quests.size,
    outcome: prev.outcome,
    turns,
    console_errors: errors,
    panics,
    panel_minutes: r3((clock[clock.length - 1] || 0) / 60),
    engaged_acts: eng.engagedActs,
    stalled_turns: stalled,
    cap,
  };
}
