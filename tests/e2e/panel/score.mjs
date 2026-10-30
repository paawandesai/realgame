// The panel line for scoreboard.jsonl (docs/loop/panel.md, "Scoring").
// Bars quoted from docs/loop/bars.md:
//   B4 "each persona moves within 10 s and lands a cast within 30 s. Median
//      clarity >= 8/10. No persona reports a control blocker"
//   B5 "Median fun >= 8/10 across the 5 personas, and each persona's fun >= 6"
//   B8 ">= 15 minutes of guided play for a 9-year-old persona (at least 3
//      quests, 6 spells, a free-play area and a skill-tree screen), measured
//      by panel play time". The content half is not visible to the harness.
// Panel scores are "panel" data, never "player" data.
import { PERSONA_IDS } from './personas.mjs';
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

// results: parsed result.json objects (already validated or not).
export function scorePanel(results, { ts = new Date().toISOString(), artifact } = {}) {
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
  const ids = new Set(valid.map((r) => r.persona));
  if (valid.length !== 5 || ids.size !== 5 || !PERSONA_IDS.every((p) => ids.has(p))) {
    return { ...base, ...failAll('incomplete panel') };
  }
  const shas = new Set(valid.map((r) => r.artifact_sha));
  const queries = new Set(valid.map((r) => r.query));
  if (shas.size !== 1 || queries.size !== 1 || (artifact && !shas.has(artifact))) {
    return { ...base, ...failAll('mismatched runs') };
  }
  const rows = PERSONA_IDS.map((p) => per[p]);
  const allMove = rows.every((x) => x.ttm !== null && x.ttm <= 10);
  const allCast = rows.every((x) => x.ttc !== null && x.ttc <= 30);
  const medClarity = median(rows.map((x) => x.clarity));
  const control = rows.reduce((n, x) => n + x.blockers_control, 0);
  const medFun = median(rows.map((x) => x.fun));
  const minFun = Math.min(...rows.map((x) => x.fun));
  const nine = valid.find((r) => r.persona === '9yo');
  return {
    ...base,
    personas: PERSONA_IDS,
    B4: { pass: allMove && allCast && medClarity >= 8 && control === 0, median_clarity: medClarity, all_move_10s: allMove, all_cast_30s: allCast, control_blockers: control },
    B5: { pass: medFun >= 8 && minFun >= 6, median_fun: medFun, min_fun: minFun },
    B8: { pass: nine.measured.minutes_played >= 15, minutes_9yo: nine.measured.minutes_played, quests_completed_9yo: nine.measured.quests_completed, content_check: 'manual' },
  };
}
