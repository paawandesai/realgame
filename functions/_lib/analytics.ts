// Server side of the anonymous analytics: strict validation of incoming
// events, the stats queries, and the shapes the dashboard reads.
//
// Plain TypeScript with erasable syntax only, so the unit tests run it
// directly with `node --test` (no build step) and wrangler bundles the same
// file for Cloudflare Pages Functions. See docs/ANALYTICS.md.

import type { D1Database } from "./d1.ts";

export const EVENT_TYPES = ["session_start", "room_complete", "trial_complete", "session_end"] as const;
export const BANDS = ["sprout", "seedling", "apprentice"] as const;
/** Session length buckets, in order. Stored as their index (0-4). */
export const DURATIONS = ["lt1m", "1to5m", "5to15m", "15to30m", "30mplus"] as const;
/** Highest room index accepted (0-based). The game has 4-5 rooms; this leaves headroom. */
export const MAX_ROOM = 15;
/** Bodies larger than this are rejected before parsing. A real event is about 100 bytes. */
export const MAX_BODY_BYTES = 512;
/** Events accepted per device per UTC day. A full play-through is about 8. */
export const DAILY_CAP = 120;
/** Raw events older than this many days are deleted. */
export const RETENTION_DAYS = 90;
/** The dashboard's window. */
export const STATS_DAYS = 30;
/** Shortest DASHBOARD_TOKEN the stats endpoint will accept (fails closed below it). */
export const MIN_TOKEN_LENGTH = 32;

export type EventType = (typeof EVENT_TYPES)[number];
export type Band = (typeof BANDS)[number];
export type Duration = (typeof DURATIONS)[number];

export interface GameEvent {
  type: EventType;
  id: string;
  band: Band;
  /** room_complete only. */
  room: number | null;
  /** session_end only: index into DURATIONS. */
  dur: number | null;
}

export type Validation = { ok: true; event: GameEvent } | { ok: false; error: string };

const ID_RE = /^[0-9a-f]{32}$/;
const COMMON_KEYS = ["v", "type", "id", "band"];
const KEYS_BY_TYPE: Record<EventType, readonly string[]> = {
  session_start: COMMON_KEYS,
  trial_complete: COMMON_KEYS,
  room_complete: [...COMMON_KEYS, "room"],
  session_end: [...COMMON_KEYS, "dur"],
};

function isOneOf<T extends string>(list: readonly T[], value: unknown): value is T {
  return typeof value === "string" && (list as readonly string[]).includes(value);
}

/**
 * Accepts exactly one of the four event shapes and nothing else: unknown or
 * missing keys, wrong types, out-of-range rooms, unknown bands and malformed
 * ids are all rejected, so no free text can reach the database.
 */
export function validateEvent(body: string): Validation {
  if (body.length > MAX_BODY_BYTES) return { ok: false, error: "too large" };
  let data: unknown;
  try {
    data = JSON.parse(body);
  } catch {
    return { ok: false, error: "not JSON" };
  }
  if (typeof data !== "object" || data === null || Array.isArray(data)) return { ok: false, error: "not an object" };
  const obj = data as Record<string, unknown>;
  if (obj.v !== 1) return { ok: false, error: "unknown version" };
  if (!isOneOf(EVENT_TYPES, obj.type)) return { ok: false, error: "unknown type" };
  const type = obj.type;
  const allowed = KEYS_BY_TYPE[type];
  const keys = Object.keys(obj);
  if (keys.length !== allowed.length || !keys.every((k) => allowed.includes(k))) {
    return { ok: false, error: "unexpected fields" };
  }
  if (typeof obj.id !== "string" || !ID_RE.test(obj.id)) return { ok: false, error: "bad id" };
  if (!isOneOf(BANDS, obj.band)) return { ok: false, error: "bad band" };
  let room: number | null = null;
  if (type === "room_complete") {
    if (typeof obj.room !== "number" || !Number.isInteger(obj.room) || obj.room < 0 || obj.room > MAX_ROOM) {
      return { ok: false, error: "bad room" };
    }
    room = obj.room;
  }
  let dur: number | null = null;
  if (type === "session_end") {
    if (!isOneOf(DURATIONS, obj.dur)) return { ok: false, error: "bad dur" };
    dur = DURATIONS.indexOf(obj.dur);
  }
  return { ok: true, event: { type, id: obj.id, band: obj.band, room, dur } };
}

/** "YYYY-MM-DD" for the UTC day of `ms`. */
export function utcDay(ms: number): string {
  return new Date(ms).toISOString().slice(0, 10);
}

/** The UTC day `n` days before `day`. */
export function daysBefore(day: string, n: number): string {
  return utcDay(Date.parse(`${day}T00:00:00Z`) - n * 86_400_000);
}

// ----------------------------------------------------------------- responses

const SECURITY_HEADERS: Record<string, string> = {
  // _headers does not apply to Functions responses, so they carry their own.
  "Cache-Control": "no-store",
  "X-Content-Type-Options": "nosniff",
  "Referrer-Policy": "no-referrer",
  "Content-Security-Policy": "default-src 'none'; frame-ancestors 'none'",
  "X-Robots-Tag": "noindex",
};

export function respond(status: number, body?: unknown, extra: Record<string, string> = {}): Response {
  const headers: Record<string, string> = { ...SECURITY_HEADERS, ...extra };
  if (body === undefined) return new Response(null, { status, headers });
  headers["Content-Type"] = "application/json; charset=utf-8";
  return new Response(JSON.stringify(body), { status, headers });
}

// -------------------------------------------------------------------- ingest

export interface Env {
  DB: D1Database;
  DASHBOARD_TOKEN?: string;
}

/** True when the browser says the request came from another site. */
function crossSite(request: Request): boolean {
  const site = request.headers.get("Sec-Fetch-Site");
  if (site !== null && site !== "same-origin") return true;
  const origin = request.headers.get("Origin");
  return origin !== null && origin !== new URL(request.url).origin;
}

/**
 * POST /api/event. 204 when stored, 400 for anything malformed, 429 past the
 * per-device daily cap. Never echoes input back and never logs it.
 */
export async function handleEvent(request: Request, env: Env, now = Date.now()): Promise<Response> {
  if (request.method !== "POST") return respond(405, { error: "POST only" }, { Allow: "POST" });
  if (crossSite(request)) return respond(403, { error: "same-origin only" });
  const type = (request.headers.get("Content-Type") ?? "").split(";")[0]!.trim().toLowerCase();
  if (type !== "application/json" && type !== "text/plain") return respond(415, { error: "JSON only" });
  const declared = Number(request.headers.get("Content-Length") ?? "0");
  if (declared > MAX_BODY_BYTES) return respond(413, { error: "too large" });
  const body = await request.text();
  const result = validateEvent(body);
  if (!result.ok) return respond(result.error === "too large" ? 413 : 400, { error: result.error });

  const e = result.event;
  const day = utcDay(now);
  // One round trip: the insert only happens while this device is under its
  // daily cap, and expired rows are purged on the way (an index range on
  // `day`, so it costs almost nothing when there is nothing to delete).
  const [insert] = await env.DB.batch([
    env.DB.prepare(
      `INSERT INTO events (day, daily_id, type, band, room, dur)
       SELECT ?1, ?2, ?3, ?4, ?5, ?6
       WHERE (SELECT COUNT(*) FROM events WHERE day = ?1 AND daily_id = ?2) < ?7`,
    ).bind(day, e.id, e.type, e.band, e.room, e.dur, DAILY_CAP),
    env.DB.prepare("DELETE FROM events WHERE day < ?1").bind(daysBefore(day, RETENTION_DAYS - 1)),
  ]);
  if (!insert || insert.meta.changes === 0) return respond(429, { error: "daily limit" });
  return respond(204);
}

// --------------------------------------------------------------------- stats

const encoder = new TextEncoder();

/**
 * Constant-time token check. Both sides are hashed first, so the comparison
 * always runs over 32 bytes whatever the input length, and the loop never
 * exits early.
 */
export async function tokenMatches(given: string, expected: string): Promise<boolean> {
  const [a, b] = await Promise.all([
    crypto.subtle.digest("SHA-256", encoder.encode(given)),
    crypto.subtle.digest("SHA-256", encoder.encode(expected)),
  ]);
  const x = new Uint8Array(a);
  const y = new Uint8Array(b);
  let diff = 0;
  for (let i = 0; i < x.length; i++) diff |= x[i]! ^ y[i]!;
  return diff === 0;
}

export interface DayRow {
  day: string;
  players: number;
  sessions: number;
}
export interface DayBandRow {
  day: string;
  band: string;
  players: number;
}
export interface FunnelRow {
  type: string;
  room: number | null;
  band: string;
  players: number;
}
export interface DurationRow {
  dur: number;
  players: number;
}

export type BandCounts = Record<Band, number>;

export interface Stats {
  generatedAt: string;
  today: string;
  windowDays: number;
  retentionDays: number;
  players: { today: number; todayByBand: BandCounts; playerDays: number; playerDaysByBand: BandCounts };
  sessions: { today: number; total: number };
  daily: { day: string; players: number; sessions: number; byBand: BandCounts }[];
  /** Counts are player-days: one device on one UTC day. */
  funnel: {
    started: BandCounts & { all: number };
    rooms: ({ room: number } & BandCounts & { all: number })[];
    trialComplete: BandCounts & { all: number };
  };
  /** Longest session per player-day, by bucket. */
  durations: { bucket: Duration; players: number }[];
}

function zeroBands(): BandCounts {
  return { sprout: 0, seedling: 0, apprentice: 0 };
}

function addBand(target: BandCounts & { all?: number }, band: string, n: number): void {
  if (isOneOf(BANDS, band)) target[band] += n;
  if (target.all !== undefined) target.all += n;
}

/**
 * Turns raw query rows into the dashboard's shape: every day of the window
 * present (zero-filled), oldest first, bands always all three, rooms dense
 * from 0 to the highest seen.
 */
export function shapeStats(
  today: string,
  rows: { days: DayRow[]; dayBands: DayBandRow[]; funnel: FunnelRow[]; durations: DurationRow[] },
  generatedAt: string,
): Stats {
  const byDay = new Map(rows.days.map((r) => [r.day, r]));
  const daily: Stats["daily"] = [];
  for (let i = STATS_DAYS - 1; i >= 0; i--) {
    const day = daysBefore(today, i);
    const r = byDay.get(day);
    daily.push({ day, players: r?.players ?? 0, sessions: r?.sessions ?? 0, byBand: zeroBands() });
  }
  const dayIndex = new Map(daily.map((d, i) => [d.day, i]));
  const playerDaysByBand = zeroBands();
  for (const r of rows.dayBands) {
    const i = dayIndex.get(r.day);
    if (i === undefined) continue;
    addBand(daily[i]!.byBand, r.band, r.players);
    addBand(playerDaysByBand, r.band, r.players);
  }

  const started = { ...zeroBands(), all: 0 };
  const trialComplete = { ...zeroBands(), all: 0 };
  const roomMap = new Map<number, BandCounts & { all: number }>();
  for (const r of rows.funnel) {
    if (r.type === "session_start") addBand(started, r.band, r.players);
    else if (r.type === "trial_complete") addBand(trialComplete, r.band, r.players);
    else if (r.type === "room_complete" && r.room !== null) {
      let entry = roomMap.get(r.room);
      if (!entry) roomMap.set(r.room, (entry = { ...zeroBands(), all: 0 }));
      addBand(entry, r.band, r.players);
    }
  }
  const maxRoom = Math.max(-1, ...roomMap.keys());
  const rooms: Stats["funnel"]["rooms"] = [];
  for (let room = 0; room <= maxRoom; room++) rooms.push({ room, ...(roomMap.get(room) ?? { ...zeroBands(), all: 0 }) });

  const durCounts = new Map(rows.durations.map((r) => [r.dur, r.players]));
  const last = daily[daily.length - 1]!;
  return {
    generatedAt,
    today,
    windowDays: STATS_DAYS,
    retentionDays: RETENTION_DAYS,
    players: {
      today: last.players,
      todayByBand: last.byBand,
      playerDays: daily.reduce((s, d) => s + d.players, 0),
      playerDaysByBand,
    },
    sessions: { today: last.sessions, total: daily.reduce((s, d) => s + d.sessions, 0) },
    daily,
    funnel: { started, rooms, trialComplete },
    durations: DURATIONS.map((bucket, i) => ({ bucket, players: durCounts.get(i) ?? 0 })),
  };
}

/** GET /api/stats, behind `Authorization: Bearer <DASHBOARD_TOKEN>`. */
export async function handleStats(request: Request, env: Env, now = Date.now()): Promise<Response> {
  if (request.method !== "GET") return respond(405, { error: "GET only" }, { Allow: "GET" });
  const expected = env.DASHBOARD_TOKEN ?? "";
  if (expected.length < MIN_TOKEN_LENGTH) {
    return respond(503, { error: `DASHBOARD_TOKEN is not set (or shorter than ${MIN_TOKEN_LENGTH} characters)` });
  }
  const auth = request.headers.get("Authorization") ?? "";
  const given = auth.startsWith("Bearer ") ? auth.slice(7) : "";
  if (!(await tokenMatches(given, expected))) {
    return respond(401, { error: "wrong or missing token" }, { "WWW-Authenticate": "Bearer" });
  }

  const today = utcDay(now);
  const since = daysBefore(today, STATS_DAYS - 1);
  const db = env.DB;
  const [, days, dayBands, funnel, durations] = await db.batch([
    // Retention also runs here, so expired rows go even on days with no events.
    db.prepare("DELETE FROM events WHERE day < ?1").bind(daysBefore(today, RETENTION_DAYS - 1)),
    db
      .prepare(
        `SELECT day, COUNT(DISTINCT daily_id) AS players,
                SUM(type = 'session_start') AS sessions
         FROM events WHERE day >= ?1 GROUP BY day`,
      )
      .bind(since),
    db
      .prepare(
        `SELECT day, band, COUNT(DISTINCT daily_id) AS players
         FROM events WHERE day >= ?1 GROUP BY day, band`,
      )
      .bind(since),
    db
      .prepare(
        `SELECT type, room, band, COUNT(DISTINCT day || daily_id) AS players
         FROM events WHERE day >= ?1 AND type IN ('session_start', 'room_complete', 'trial_complete')
         GROUP BY type, room, band`,
      )
      .bind(since),
    db
      .prepare(
        `SELECT dur, COUNT(*) AS players FROM (
           SELECT MAX(dur) AS dur FROM events
           WHERE day >= ?1 AND type = 'session_end' GROUP BY day, daily_id
         ) GROUP BY dur`,
      )
      .bind(since),
  ]);
  const stats = shapeStats(
    today,
    {
      days: (days?.results ?? []) as unknown as DayRow[],
      dayBands: (dayBands?.results ?? []) as unknown as DayBandRow[],
      funnel: (funnel?.results ?? []) as unknown as FunnelRow[],
      durations: (durations?.results ?? []) as unknown as DurationRow[],
    },
    new Date(now).toISOString(),
  );
  return respond(200, stats);
}
