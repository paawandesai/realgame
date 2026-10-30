// The panel line for scoreboard.jsonl (docs/loop/panel.md, "Scoring").
// Bars quoted from docs/loop/bars.md:
//   B4 "each persona moves within 10 s and lands a cast within 30 s. Median
//      clarity >= 8/10. No persona reports a control blocker"
//   B5 "Median fun >= 8/10 across the 5 personas, and each persona's fun >= 6"
//   B8 ">= 15 minutes of guided play for a 9-year-old persona (at least 3
//      quests, 6 spells, a free-play area and a skill-tree screen), measured
//      by panel play time". The content half is not visible to the harness.
// Panel scores are "panel" data, never "player" data.
import { PERSONA_IDS, PERSONAS } from './personas.mjs';
import { validate } from './schema.mjs';

// The middle value of the sorted list (the panel always has 5 entries).
export const median = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  const n = s.length;
  return n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2;
};

const failAll = (reason) => ({
  B4: { pass: false, reason },
  B5: { pass: false, reason },
  B8: { pass: false, reason, content_check: 'manual' },
});

// Run quality (I-014): a run that crashed, errored, barely played, ran at the
// wrong throttle or mostly stalled cannot judge a bar.
export const MIN_TURNS = 20;
export const MAX_STALLED_FRACTION = 0.1;
export const MIN_THROTTLE_RATIO = 2; // qa cpu_bench_ms / 9yo cpu_bench_ms must exceed this
export const ENGAGED_PER_MINUTE = 2; // B8: engaged acts per minute played

export function runProblems(r, { allowThrottle = false } = {}) {
  const out = [];
  const m = r.measured;
  const want = PERSONAS[r.persona]?.throttle;
  if (!allowThrottle && want !== undefined && r.throttle !== want) out.push(`${r.persona}: throttle ${r.throttle}, expected ${want} (score --allow-throttle to accept)`);
  if (m.panics > 0) out.push(`${r.persona}: panics ${m.panics}`);
  if (r.stopped_because === 'error') out.push(`${r.persona}: stopped_because error`);
  if (m.turns < MIN_TURNS) out.push(`${r.persona}: turns ${m.turns} < ${MIN_TURNS}`);
  if ((m.stalled_turns ?? 0) > m.turns * MAX_STALLED_FRACTION) out.push(`${r.persona}: stalled_turns ${m.stalled_turns} of ${m.turns}`);
  return out;
}

// Model routing (config.md): reported, not failed, since the operator assigns models.
function routingWarnings(results) {
  return results.filter((r) => PERSONAS[r.persona] && !String(r.model).toLowerCase().includes(PERSONAS[r.persona].model))
    .map((r) => `${r.persona}: model ${r.model}, routing says ${PERSONAS[r.persona].model}`);
}

// results: parsed result.json objects (already validated or not).
export function scorePanel(results, { ts = new Date().toISOString(), artifact, allowThrottle = false } = {}) {
  const valid = results.filter((r) => validate(r).length === 0);
  const base = {
    ts,
    kind: 'panel',
    source: 'panel',
    artifact_sha: artifact ?? valid[0]?.artifact_sha ?? null,
    query: valid[0]?.query ?? null,
    personas: valid.map((r) => r.persona),
  };
  const per = {};
  for (const r of valid) {
    per[r.persona] = {
      ttm: r.measured.time_to_move_s,
      ttc: r.measured.time_to_cast_s,
      clarity: r.self.clarity,
      fun: r.self.fun,
      minutes: r.measured.minutes_played,
      blockers_control: r.self.blockers.filter((b) => b.kind === 'control').length,
    };
  }
  base.per_persona = per;
  base.problems = valid.flatMap((r) => runProblems(r, { allowThrottle }));
  base.warnings = routingWarnings(valid);
  const ids = new Set(valid.map((r) => r.persona));
  if (valid.length !== 5 || ids.size !== 5 || !PERSONA_IDS.every((p) => ids.has(p))) {
    return { ...base, ...failAll('incomplete panel') };
  }
  const shas = new Set(valid.map((r) => r.artifact_sha));
  const queries = new Set(valid.map((r) => r.query));
  if (shas.size !== 1 || queries.size !== 1 || (artifact && !shas.has(artifact))) {
    return { ...base, ...failAll('mismatched runs') };
  }
  if (base.problems.length) return { ...base, ...failAll('run quality') };
  const rows = PERSONA_IDS.map((p) => per[p]);
  const allMove = rows.every((x) => x.ttm !== null && x.ttm <= 10);
  const allCast = rows.every((x) => x.ttc !== null && x.ttc <= 30);
  const medClarity = median(rows.map((x) => x.clarity));
  const control = rows.reduce((n, x) => n + x.blockers_control, 0);
  const medFun = median(rows.map((x) => x.fun));
  const minFun = Math.min(...rows.map((x) => x.fun));
  const nine = valid.find((r) => r.persona === '9yo');
  const qa = valid.find((r) => r.persona === 'qa');
  // The qa throttle must show in the page (I-014): a fixed JS loop timed at start.
  const ratio = qa.cpu_bench_ms > 0 && nine.cpu_bench_ms > 0 ? Math.round((qa.cpu_bench_ms / nine.cpu_bench_ms) * 100) / 100 : null;
  const throttleOk = ratio !== null && ratio > MIN_THROTTLE_RATIO;
  const engaged9 = nine.measured.engaged_acts ?? 0;
  return {
    ...base,
    personas: PERSONA_IDS,
    B4: { pass: allMove && allCast && medClarity >= 8 && control === 0 && throttleOk, median_clarity: medClarity, all_move_10s: allMove, all_cast_30s: allCast, control_blockers: control, throttle_ok: throttleOk, throttle_ratio: ratio },
    B5: { pass: medFun >= 8 && minFun >= 6, median_fun: medFun, min_fun: minFun },
    B8: { pass: nine.measured.minutes_played >= 15 && engaged9 >= ENGAGED_PER_MINUTE * nine.measured.minutes_played, minutes_9yo: nine.measured.minutes_played, engaged_acts_9yo: engaged9, quests_completed_9yo: nine.measured.quests_completed, content_check: 'manual' },
  };
}
