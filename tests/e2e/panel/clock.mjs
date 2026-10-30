// The panel clock (docs/loop/panel.md, "Panel clock"). Wall time on a model's
// turn includes model latency and a slow software GPU, so the harness counts
// panel time instead: the input duration of each act, plus a fixed reading
// cost per observe for the persona. Deterministic for a given action log.
import { PERSONAS } from './personas.mjs';

const TAP_MS = 100; // a click, a mouse move or a wheel notch
export const DEFAULT_KEY_MS = 100;
export const DEFAULT_DRAG_MS = 300;
export const MAX_WAIT_MS = 10000;
export const MAX_HOLD_MS = 10000;

// Panel milliseconds for one log entry. Entries without a cost (start, stop,
// refused actions) count 0.
export function costMs(e, personaId) {
  if (e.cmd === 'observe') return Math.round((PERSONAS[personaId]?.read_s ?? 3) * 1000);
  if (e.cmd !== 'act') return 0;
  const a = e.args || [];
  switch (e.action) {
    case 'key': return a[1] !== undefined ? +a[1] : DEFAULT_KEY_MS;
    case 'keys': return +a[1];
    case 'move': case 'click': case 'wheel': return TAP_MS;
    case 'drag': return a[4] !== undefined ? +a[4] : DEFAULT_DRAG_MS;
    case 'hold': return +a[2];
    case 'wait': return +a[0];
    default: return 0;
  }
}

// Cumulative panel seconds after each entry of the log.
export function panelClock(log, personaId) {
  const p = personaId ?? log.find((e) => e.cmd === 'start')?.persona;
  let ms = 0;
  return log.map((e) => { ms += costMs(e, p); return ms / 1000; });
}
