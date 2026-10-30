// The five panel personas (target.md "Standing assumptions"), their reading
// cost per observe for the panel clock (docs/loop/panel.md), default CPU
// throttle and model (config.md "Model routing").
export const PERSONAS = {
  '6yo': { read_s: 4, throttle: 1, model: 'haiku', label: '6-year-old' },
  '9yo': { read_s: 3, throttle: 1, model: 'haiku', label: '9-year-old' },
  '13yo': { read_s: 2, throttle: 1, model: 'sonnet', label: '13-year-old' },
  teacher: { read_s: 3, throttle: 1, model: 'sonnet', label: 'chemistry teacher' },
  qa: { read_s: 2, throttle: 4, model: 'opus', label: 'QA tester (4x CPU throttle)' },
};
export const PERSONA_IDS = Object.keys(PERSONAS);

// Caps for one session (panel.md): whichever comes first.
export const TURN_CAP = 400;
export const TIME_CAP_S = 20 * 60;

export function persona(id) {
  const p = PERSONAS[id];
  if (!p) throw new Error(`unknown persona "${id}" (expected one of ${PERSONA_IDS.join(', ')})`);
  return p;
}
