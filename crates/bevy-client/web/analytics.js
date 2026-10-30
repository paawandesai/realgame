// @ts-check
// Anonymous, first-party play counts for the first-person client. See
// docs/ANALYTICS.md for the full contract and SECURITY.md for the short one.
//
// A port of the privacy core of the classic game's web/src/analytics.ts, with
// the same rules: at most a handful of tiny events per session to same-origin
// /api/event (which age band started, which quests were finished, a coarse
// session-length bucket). Never names, text, cookies, fingerprints or
// location. The only identifier is a random value this device replaces every
// UTC day. Nothing is sent when the browser sends Do Not Track or Global
// Privacy Control, or when the page's rg-analytics switch is not "on".
//
// Every function here is fire-and-forget and never throws. The module reads
// window, document and navigator only inside functions, never at import time,
// so the unit tests can import it under plain Node.

const ENDPOINT = '/api/event';
const STORAGE_KEY = 'rg.dailyId';
/** @type {readonly string[]} */
export const BANDS = ['sprout', 'seedling', 'apprentice'];
/** Highest quest index the server accepts (0-based; "Save the Greenhouse" is 0). */
export const MAX_QUEST = 15;
/** How often the game state is read for quest completion, in ms (not per frame: CPU budget). */
export const POLL_MS = 500;
/** Upper bounds, in minutes, of the session-length buckets (the last is open). */
const DURATION_BUCKETS = /** @type {const} */ ([
  [1, 'lt1m'],
  [5, '1to5m'],
  [15, '5to15m'],
  [30, '15to30m'],
  [Infinity, '30mplus'],
]);
const ID_RE = /^[0-9a-f]{32}$/;

/**
 * @typedef {{ getItem(key: string): string | null, setItem(key: string, value: string): void }} StorageLike
 * @typedef {{ doNotTrack?: string | null, globalPrivacyControl?: boolean }} PrivacySignals
 * @typedef {{ type: 'session_start', band: string }
 *   | { type: 'quest_complete', band: string, quest?: number }
 *   | { type: 'session_end', band: string, dur?: string }
 *   | { type: string, band: string, [key: string]: unknown }} PlayEvent
 * @typedef {{ outcome?: unknown, quest?: unknown } | null | undefined} GameStateLike
 */

/** The extra keys each event type may carry besides v, type, band and id. */
const EXTRA_KEYS = /** @type {Record<string, string[]>} */ ({
  session_start: [],
  quest_complete: ['quest'],
  session_end: ['dur'],
});

/**
 * True when the browser asks not to be tracked (DNT or Global Privacy Control).
 * @param {PrivacySignals | undefined} nav
 */
export function optedOut(nav) {
  if (!nav) return true;
  return nav.doNotTrack === '1' || nav.doNotTrack === 'yes' || nav.globalPrivacyControl === true;
}

/**
 * True only when the page carries `<meta name="rg-analytics" content="on">`.
 * The source page ships "off"; scripts/build-site.sh flips it in web/dist.
 * @param {{ querySelector(sel: string): unknown } | undefined} doc
 */
export function analyticsOn(doc) {
  try {
    const meta = /** @type {{ content?: unknown } | null} */ (doc?.querySelector('meta[name="rg-analytics"]') ?? null);
    return meta?.content === 'on';
  } catch {
    return false;
  }
}

/** "YYYY-MM-DD" in UTC. @param {Date} now */
export function utcDay(now) {
  return now.toISOString().slice(0, 10);
}

/** 32 random lowercase hex characters (128 bits) from the platform CSPRNG. */
export function randomId() {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

/** @type {string | null} */
let memoryId = null;

/**
 * Today's random device id, stored as "<utc day>.<id>" and replaced the first
 * time it is read on a new UTC day. If storage is blocked it lives in memory
 * for this page load only, which can count a device twice in a day but never
 * links days.
 * @param {StorageLike | null} storage
 * @param {Date} now
 * @param {() => string} [makeId]
 */
export function dailyId(storage, now, makeId = randomId) {
  const day = utcDay(now);
  /** @type {string | null} */
  let stored = null;
  try {
    stored = storage?.getItem(STORAGE_KEY) ?? null;
  } catch {
    stored = null;
  }
  const [storedDay, storedId] = stored?.split('.') ?? [];
  if (storedDay === day && storedId && ID_RE.test(storedId)) return storedId;
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

/** The coarse bucket for a session that has lasted `ms` milliseconds. @param {number} ms */
export function durationBucket(ms) {
  const minutes = ms / 60_000;
  for (const [limit, name] of DURATION_BUCKETS) if (minutes < limit) return name;
  return '30mplus';
}

/** @param {unknown} q */
const validQuest = (q) => typeof q === 'number' && Number.isInteger(q) && q >= 0 && q <= MAX_QUEST;

/**
 * The JSON body for one event, with exactly the keys the server allows for
 * its type, or null when anything is off (an unknown type, band or value, or
 * any extra key). Nothing is trimmed: a wrong event is dropped whole.
 * @param {PlayEvent} event
 * @param {string} id
 * @returns {string | null}
 */
export function eventBody(event, id) {
  const extra = EXTRA_KEYS[event.type];
  if (!extra || !BANDS.includes(event.band) || !ID_RE.test(id)) return null;
  const keys = Object.keys(event);
  if (keys.length !== 2 + extra.length || !keys.every((k) => k === 'type' || k === 'band' || extra.includes(k))) return null;
  const e = /** @type {Record<string, unknown>} */ (event);
  if (event.type === 'quest_complete' && !validQuest(e.quest)) return null;
  if (event.type === 'session_end' && !DURATION_BUCKETS.some(([, name]) => name === e.dur)) return null;
  /** @type {Record<string, unknown>} */
  const body = { v: 1, type: event.type, band: event.band };
  for (const k of extra) body[k] = e[k];
  body.id = id;
  return JSON.stringify(body);
}

/**
 * Quest completions between two game states: a quest counts when `outcome`
 * goes from 'playing' to 'won', at most once per quest per session (`sent` is
 * the session's set and is updated). 'lost' counts nothing. The quest is
 * `state.quest` when it is an integer from 0 to 15, otherwise 0.
 * @param {GameStateLike} prev
 * @param {GameStateLike} state
 * @param {Set<number>} sent
 * @returns {number[]}
 */
export function questEvents(prev, state, sent) {
  if (!prev || !state || prev.outcome !== 'playing' || state.outcome !== 'won') return [];
  const quest = validQuest(state.quest) ? /** @type {number} */ (state.quest) : 0;
  if (sent.has(quest)) return [];
  sent.add(quest);
  return [quest];
}

/**
 * One play session (one page load). Pure: the caller supplies the clock, the
 * game state and the sender, and calls poll() on a timer and hidden() when
 * the page is hidden or closed.
 * @param {{ band: string, send: (e: PlayEvent) => void, now: () => number, getState: () => GameStateLike }} opts
 */
export function createSession({ band, send, now, getState }) {
  const startedAt = now();
  let started = false;
  /** @type {GameStateLike} */
  let prev = null;
  const questsSent = new Set();
  let lastBucket = -1;
  return {
    /** Reads the game state: session_start on the first state, then quest completions. */
    poll() {
      /** @type {GameStateLike} */
      let state = null;
      try {
        state = getState();
      } catch {
        state = null;
      }
      if (!state) return;
      if (!started) {
        started = true;
        send({ type: 'session_start', band });
      }
      for (const quest of questEvents(prev, state, questsSent)) send({ type: 'quest_complete', band, quest });
      prev = state;
    },
    /** session_end, only when the length bucket has grown since the last one sent. */
    hidden() {
      if (!started) return;
      const bucket = durationBucket(now() - startedAt);
      const index = DURATION_BUCKETS.findIndex(([, name]) => name === bucket);
      if (index <= lastBucket) return;
      lastBucket = index;
      send({ type: 'session_end', band, dur: bucket });
    },
  };
}

/** @returns {StorageLike | null} */
function storage() {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/**
 * Sends one event: sendBeacon first (it survives the page closing; a string
 * body goes as text/plain, which needs no preflight), otherwise fetch with
 * keepalive. Never throws.
 * @param {PlayEvent} event
 * @param {string} [id] the session's daily id; read from storage when left out
 */
export function send(event, id = dailyId(storage(), new Date())) {
  try {
    if (typeof navigator === 'undefined' || optedOut(/** @type {PrivacySignals} */ (navigator))) return;
    const body = eventBody(event, id);
    if (body === null) return;
    if (typeof navigator.sendBeacon === 'function' && navigator.sendBeacon(ENDPOINT, body)) return;
    void fetch(ENDPOINT, {
      method: 'POST',
      body,
      keepalive: true,
      credentials: 'omit',
      headers: { 'Content-Type': 'text/plain' },
    }).catch(() => undefined);
  } catch {
    // Analytics must never break the game.
  }
}

/** @type {ReturnType<typeof createSession> | null} */
let session = null;

/**
 * Starts the page's one session with the picked band. Reads the game state
 * every POLL_MS through window.__game.state() (web/hud.js) and sends
 * session_end when the page is hidden or closed. Returns false (and does
 * nothing) when analytics is off, the browser opted out, the band is unknown,
 * or a session already started.
 * @param {string} band
 */
export function start(band) {
  try {
    if (session || !BANDS.includes(band)) return false;
    if (!analyticsOn(document) || optedOut(/** @type {PrivacySignals} */ (navigator))) return false;
    const w = /** @type {any} */ (window);
    // One id for the whole session, read once: a session that runs past UTC
    // midnight keeps the id it started with.
    const id = dailyId(storage(), new Date());
    const s = createSession({
      band,
      send: (e) => send(e, id),
      now: () => performance.now(),
      getState: () => (w.__gameState && w.__game ? w.__game.state() : null),
    });
    session = s;
    s.poll();
    setInterval(() => s.poll(), POLL_MS);
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'hidden') s.hidden();
    });
    window.addEventListener('pagehide', () => s.hidden());
    return true;
  } catch {
    return false;
  }
}
