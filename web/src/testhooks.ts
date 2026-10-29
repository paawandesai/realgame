// Read-only hooks for the Playwright golden paths (DESIGN_V3 §12.5). Track E
// owns this file after Hour 0.
//
// `window.__alch` exists only with `?test=1` or `?debug`. Every getter
// returns a fresh copy, so tests can never change game state through it;
// the only actions are `cast` and `aim`, which go through the same paths a
// player's input does. (`window.realgame` under `?debug` is the older v2
// hook and stays as it is.)

import type { Screen } from "./bus";
import type { CastOutcome, Goal, Tier, TrialResult } from "./types";

/** What main.ts hands over. Getters, so values are read when asked. Any
 * left out reads as empty (null, 0 or []), e.g. on the start screen before
 * a game exists; later calls to `installTestHooks` replace the source. */
export interface TestHookSource {
  screen(): Screen | null;
  tier(): Tier | null;
  /** Story room, 1-based; 0 in a Trial or the Lab. */
  room(): number;
  goals(): Goal[];
  lastCast(): CastOutcome | null;
  trialResult(): TrialResult | null;
  /** Grimoire card ids found so far. */
  cards(): string[];
  /** Casts as if typed and sent. */
  cast(text: string): void;
  /** Aims at world cell (x, y) as a pointer would. */
  aim(x: number, y: number): void;
}

export interface AlchHooks {
  readonly screen: Screen | null;
  readonly tier: Tier | null;
  readonly room: number;
  readonly goals: Goal[];
  readonly lastCast: CastOutcome | null;
  readonly trialResult: TrialResult | null;
  readonly cards: string[];
  cast(text: string): void;
  aim(x: number, y: number): void;
}

declare global {
  interface Window {
    __alch?: AlchHooks;
  }
}

/** True when the page was opened for automated tests. */
export function testHooksWanted(search: string = window.location.search): boolean {
  const q = new URLSearchParams(search);
  return q.get("test") === "1" || q.has("debug");
}

function copy<T>(v: T): T {
  return v === null || v === undefined ? v : (JSON.parse(JSON.stringify(v)) as T);
}

const EMPTY: TestHookSource = {
  screen: () => null,
  tier: () => null,
  room: () => 0,
  goals: () => [],
  lastCast: () => null,
  trialResult: () => null,
  cards: () => [],
  cast: () => {},
  aim: () => {},
};

let source: TestHookSource = EMPTY;

/** Defines `window.__alch` on the first call (only under `?test=1` or
 * `?debug`); later calls just swap in a fuller source. */
export function installTestHooks(src: Partial<TestHookSource>): void {
  if (!testHooksWanted()) return;
  const first = source === EMPTY;
  source = { ...EMPTY, ...src };
  if (!first) return;
  const hooks = {} as AlchHooks;
  const getters: [keyof AlchHooks, () => unknown][] = [
    ["screen", () => source.screen()],
    ["tier", () => source.tier()],
    ["room", () => source.room()],
    ["goals", () => copy(source.goals())],
    ["lastCast", () => copy(source.lastCast())],
    ["trialResult", () => copy(source.trialResult())],
    ["cards", () => [...source.cards()]],
  ];
  for (const [name, get] of getters) Object.defineProperty(hooks, name, { get, enumerable: true });
  Object.defineProperty(hooks, "cast", { value: (text: string) => source.cast(String(text)), enumerable: true });
  Object.defineProperty(hooks, "aim", {
    value: (x: number, y: number) => source.aim(Math.round(Number(x)), Math.round(Number(y))),
    enumerable: true,
  });
  Object.defineProperty(window, "__alch", { value: Object.freeze(hooks), configurable: true });
}
