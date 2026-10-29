// Deep links for classrooms (DESIGN_V3 §8). Track D owns this file after
// Hour 0. Parsed once at boot by `initRoute()`; unknown or malformed values
// are ignored, never thrown on. `?debug` keeps its v2 behaviour (main.ts
// reads it directly).
//
//   ?tier=sprout|seedling|apprentice   skip the age picker
//   ?mode=story|trials|lab|daily       start in that mode
//   ?room=1-4                          story room
//   ?trial=T1..T7                      Trial
//   ?c=K7QX                            class code (random, no PII)
//   ?demo=1                            projector/demo mode
//   ?test=1                            exposes window.__alch (testhooks.ts)
//
// Analytics stay separate: other modules read only `classCode()` and
// `ageBand()`, and nothing here makes a network call.

import type { Tier } from "./types";

export type Mode = "story" | "trials" | "lab" | "daily";

export interface Route {
  tier: Tier | null;
  mode: Mode | null;
  /** Story room, 1-4. */
  room: number | null;
  /** "T1".."T7". */
  trial: string | null;
  /** Upper-case class code, 4-8 letters/digits. */
  classCode: string | null;
  demo: boolean;
  debug: boolean;
  test: boolean;
}

const EMPTY: Route = {
  tier: null,
  mode: null,
  room: null,
  trial: null,
  classCode: null,
  demo: false,
  debug: false,
  test: false,
};

let current: Route = { ...EMPTY };

const TIERS: readonly string[] = ["sprout", "seedling", "apprentice"];
const MODES: readonly string[] = ["story", "trials", "lab", "daily"];

/** Parses a query string (no network, no storage). Exported for tests. */
export function parseRoute(search: string): Route {
  let q: URLSearchParams;
  try {
    q = new URLSearchParams(search);
  } catch {
    return { ...EMPTY };
  }
  const tier = q.get("tier");
  const mode = q.get("mode");
  const room = Number(q.get("room"));
  const trial = (q.get("trial") ?? "").toUpperCase();
  const c = (q.get("c") ?? "").toUpperCase();
  return {
    tier: tier !== null && TIERS.includes(tier) ? (tier as Tier) : null,
    mode: mode !== null && MODES.includes(mode) ? (mode as Mode) : null,
    room: Number.isInteger(room) && room >= 1 && room <= 4 ? room : null,
    trial: /^T[1-7]$/.test(trial) ? trial : null,
    classCode: /^[A-Z0-9]{4,8}$/.test(c) ? c : null,
    demo: q.get("demo") === "1",
    debug: q.has("debug"),
    test: q.get("test") === "1",
  };
}

/** Reads the page's URL once; call before anything else at boot. */
export function initRoute(search: string = window.location.search): Route {
  current = parseRoute(search);
  return current;
}

export function route(): Readonly<Route> {
  return current;
}

/** The class code from `?c`, or null. Random, never PII. */
export function classCode(): string | null {
  return current.classCode;
}

/** The age band from `?tier` (the same names analytics uses), or null. */
export function ageBand(): Tier | null {
  return current.tier;
}
