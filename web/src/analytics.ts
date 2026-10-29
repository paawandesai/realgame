// Anonymous, first-party play counts. See docs/ANALYTICS.md for the full
// contract, and SECURITY.md for what is (and is not) collected.
//
// Sends at most a handful of tiny events per session to same-origin
// /api/event: which age band started, which rooms were cleared, whether the
// trial was finished, and a coarse session-length bucket. Never names, text,
// formulas, cookies, fingerprints or location. The only identifier is a
// random value that this device replaces every UTC day.
//
// Every function here is fire-and-forget and never throws: the game works
// the same when the endpoint is missing (local dev, offline, a plain static
// host) or when storage is blocked.

const ENDPOINT = "/api/event";
const STORAGE_KEY = "rg.dailyId";
const BANDS = ["sprout", "seedling", "apprentice"] as const;
const MAX_ROOM = 15;
/** Upper bounds, in minutes, of the session-length buckets (the last is open). */
const DURATION_BUCKETS = [
  [1, "lt1m"],
  [5, "1to5m"],
  [15, "5to15m"],
  [30, "15to30m"],
  [Infinity, "30mplus"],
] as const;

export type AgeBand = (typeof BANDS)[number];

/** The parts of Storage this module uses (a stub in tests). */
export interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** The privacy signals checked before anything is sent. */
export interface PrivacySignals {
  doNotTrack?: string | null;
  globalPrivacyControl?: boolean;
}

/** True when the browser asks not to be tracked (DNT or Global Privacy Control). */
export function optedOut(nav: PrivacySignals | undefined): boolean {
  if (!nav) return true;
  return nav.doNotTrack === "1" || nav.doNotTrack === "yes" || nav.globalPrivacyControl === true;
}

/** "YYYY-MM-DD" in UTC. */
export function utcDay(now: Date): string {
  return now.toISOString().slice(0, 10);
}

/** 32 random lowercase hex characters (128 bits) from the platform CSPRNG. */
export function randomId(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

let memoryId: string | null = null;

/**
 * Today's random device id. Stored as "<utc day>.<id>" so it is replaced the
 * first time it is read on a new UTC day. If storage is blocked (private
 * windows, locked-down school profiles) it lives in memory for this page
 * load only, which can count one device twice in a day but never links days.
 */
export function dailyId(storage: StorageLike | null, now: Date, makeId: () => string = randomId): string {
  const day = utcDay(now);
  let stored: string | null = null;
  try {
    stored = storage?.getItem(STORAGE_KEY) ?? null;
  } catch {
    stored = null;
  }
  const [storedDay, storedId] = stored?.split(".") ?? [];
  if (storedDay === day && storedId && /^[0-9a-f]{32}$/.test(storedId)) return storedId;
  if (memoryId?.startsWith(`${day}.`)) return memoryId.slice(day.length + 1);

  const id = makeId();
  memoryId = `${day}.${id}`;
  try {
    storage?.setItem(STORAGE_KEY, memoryId);
  } catch {
    // Storage blocked or full: the in-memory id above is used instead.
  }
  return id;
}

/** The coarse bucket for a session that has lasted `ms` milliseconds. */
export function durationBucket(ms: number): string {
  const minutes = ms / 60_000;
  for (const [limit, name] of DURATION_BUCKETS) if (minutes < limit) return name;
  return "30mplus";
}

function isBand(value: unknown): value is AgeBand {
  return typeof value === "string" && (BANDS as readonly string[]).includes(value);
}

function storage(): StorageLike | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function send(payload: Record<string, unknown>): void {
  try {
    if (typeof navigator === "undefined" || optedOut(navigator as PrivacySignals)) return;
    const body = JSON.stringify({ v: 1, ...payload, id: dailyId(storage(), new Date()) });
    // sendBeacon survives the page closing; a string body goes as text/plain,
    // which needs no CORS preflight and is same-origin anyway (CSP connect-src 'self').
    if (typeof navigator.sendBeacon === "function" && navigator.sendBeacon(ENDPOINT, body)) return;
    void fetch(ENDPOINT, {
      method: "POST",
      body,
      keepalive: true,
      credentials: "omit",
      headers: { "Content-Type": "text/plain" },
    }).catch(() => undefined);
  } catch {
    // Analytics must never break the game.
  }
}

// ------------------------------------------------------------ session state

let band: AgeBand | null = null;
let startedAt = 0;
let roomsSent = new Set<number>();
let trialSent = false;
let lastDurationSent = -1;
let listening = false;

function onHidden(): void {
  if (band === null || document.visibilityState !== "hidden") return;
  const bucket = durationBucket(performance.now() - startedAt);
  const index = DURATION_BUCKETS.findIndex(([, name]) => name === bucket);
  // Sent each time the tab is hidden, but only when the bucket has grown,
  // so a session is at most five of these. The server keeps the longest.
  if (index <= lastDurationSent) return;
  lastDurationSent = index;
  send({ type: "session_end", band, dur: bucket });
}

/**
 * Call once when play begins with a chosen (or remembered) age band. Starts a
 * new session: room and trial events are deduplicated per session.
 */
export function trackSessionStart(tier: string): void {
  if (!isBand(tier)) return;
  band = tier;
  startedAt = typeof performance !== "undefined" ? performance.now() : 0;
  roomsSent = new Set();
  trialSent = false;
  lastDurationSent = -1;
  send({ type: "session_start", band });
  if (!listening && typeof document !== "undefined") {
    listening = true;
    document.addEventListener("visibilitychange", onHidden);
  }
}

/** Call when a room is cleared. `room` is the 0-based room index. */
export function trackRoomComplete(room: number): void {
  if (band === null || !Number.isInteger(room) || room < 0 || room > MAX_ROOM || roomsSent.has(room)) return;
  roomsSent.add(room);
  send({ type: "room_complete", band, room });
}

/** Call when the whole trial is finished. */
export function trackTrialComplete(): void {
  if (band === null || trialSent) return;
  trialSent = true;
  send({ type: "trial_complete", band });
}
