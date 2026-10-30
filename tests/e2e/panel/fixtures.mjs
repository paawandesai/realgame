// Shared fixtures for the panel unit tests (no browser).
import { PERSONA_IDS } from './personas.mjs';

export function makeResult(persona, o = {}) {
  return {
    schema: 'panel-result/1',
    persona,
    model: 'haiku',
    artifact_sha: o.artifact_sha ?? 'abc1234',
    query: o.query ?? '?low',
    throttle: persona === 'qa' ? 4 : 1,
    measured: {
      time_to_move_s: o.ttm === undefined ? 3 : o.ttm,
      time_to_cast_s: o.ttc === undefined ? 10 : o.ttc,
      minutes_played: o.minutes ?? 16,
      quests_completed: o.quests ?? 1,
      outcome: o.outcome ?? 'won',
      turns: o.turns ?? 120,
      console_errors: 0,
      panics: 0,
    },
    self: {
      clarity: o.clarity ?? 8,
      fun: o.fun ?? 8,
      blockers: o.blockers ?? [],
      would_play_again: 'yes',
      best: 'Putting out the fire.',
      worst: 'Nothing.',
      quotes: [],
    },
    stopped_because: o.stopped_because ?? 'quit',
    ...(o.schema ? { schema: o.schema } : {}),
  };
}

// Five results, one per persona, with per-persona arrays of overrides.
export function panel({ ttm = [3, 5, 8, 9, 10], ttc = [10, 12, 20, 25, 30], clarity = [8, 8, 9, 7, 10], fun = [8, 8, 8, 9, 8], minutes = [16, 16, 16, 16, 16], blockers = [[], [], [], [], []] } = {}) {
  return PERSONA_IDS.map((p, i) => makeResult(p, { ttm: ttm[i], ttc: ttc[i], clarity: clarity[i], fun: fun[i], minutes: minutes[i], blockers: blockers[i] }));
}
