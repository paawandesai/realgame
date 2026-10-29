// The v3 event bus (DESIGN_V3 §12.5). main.ts emits these from its existing
// code paths; v3 modules (screens, trials, lab, progress, grimoire, predict,
// demo, test hooks) listen instead of reaching into main.ts.
//
// Frozen after Hour 0: adding an event is fine, changing a payload needs
// every track's agreement. Listeners run synchronously in the order they
// subscribed; one that throws is reported and skipped, so a v3 module can
// never break the game loop.

import type { CastOutcome, TrialResult } from "./types";

/** Every screen the shell can show. `story` covers the v2 game and its free play. */
export type Screen = "start" | "story" | "end" | "trials" | "trial" | "lab" | "grimoire";

/** Where a Predict card belongs: "story:1".."story:4" or "trial:T1".."trial:T7". */
export type PredictKey = `story:${number}` | `trial:T${number}`;

export interface BusEvents {
  /** The visible screen changed. `prev` is null for the first screen. */
  "screen:change": { screen: Screen; prev: Screen | null };
  /** The player walked into a story room (1-based, as the HUD shows it). */
  "room:enter": { room: number };
  /** A Trial (or "lab") started. */
  "trial:enter": { id: string };
  /** A Trial's goal was met. */
  "trial:solved": { result: TrialResult };
  /** Every cast, after the sim answered. */
  "cast:result": { outcome: CastOutcome };
  /** A Predict card opened (the sim is paused while it shows). */
  "predict:open": { key: PredictKey };
  /** The player (or the class, in Demo) picked an option. */
  "predict:answer": { key: PredictKey; choice: number; correct: boolean };
  /** The key cast happened and the "why" line showed. */
  "predict:reveal": { key: PredictKey; correct: boolean };
  /** A Grimoire card was found (any reaction, phase change or light card id). */
  discover: { id: string };
}

export type BusEvent = keyof BusEvents;
type Handler<K extends BusEvent> = (payload: BusEvents[K]) => void;

const handlers: { [K in BusEvent]?: Handler<K>[] } = {};

/** Subscribes; returns the unsubscribe function. */
export function on<K extends BusEvent>(event: K, handler: Handler<K>): () => void {
  const list = (handlers[event] ??= []) as Handler<K>[];
  list.push(handler);
  return () => off(event, handler);
}

/** Subscribes for the next event only. */
export function once<K extends BusEvent>(event: K, handler: Handler<K>): () => void {
  const stop = on(event, (payload) => {
    stop();
    handler(payload);
  });
  return stop;
}

export function off<K extends BusEvent>(event: K, handler: Handler<K>): void {
  const list = handlers[event] as Handler<K>[] | undefined;
  if (!list) return;
  const i = list.indexOf(handler);
  if (i >= 0) list.splice(i, 1);
}

export function emit<K extends BusEvent>(event: K, payload: BusEvents[K]): void {
  const list = handlers[event] as Handler<K>[] | undefined;
  if (!list) return;
  for (const h of [...list]) {
    try {
      h(payload);
    } catch (err) {
      console.error(`bus: a "${event}" listener failed`, err);
    }
  }
}

/** The last value of each event, for late subscribers such as the test hooks. */
const last: { [K in BusEvent]?: BusEvents[K] } = {};
for (const k of [
  "screen:change",
  "room:enter",
  "trial:enter",
  "trial:solved",
  "cast:result",
  "predict:open",
  "predict:answer",
  "predict:reveal",
  "discover",
] as const) {
  on(k, (p) => {
    (last as Record<string, unknown>)[k] = p;
  });
}

export function lastOf<K extends BusEvent>(event: K): BusEvents[K] | undefined {
  return last[event] as BusEvents[K] | undefined;
}

export const bus = { on, once, off, emit, lastOf };
