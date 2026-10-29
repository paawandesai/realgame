// End-to-end through the real handlers and the real migration, on node:sqlite.
import assert from "node:assert/strict";
import { test } from "node:test";
import { DAILY_CAP, handleEvent, handleStats, type Env } from "../functions/_lib/analytics.ts";
import { memoryD1 } from "./d1-sqlite.ts";

const ORIGIN = "https://realgame.example";
const TOKEN = "t".repeat(48);
const NOW = Date.parse("2026-10-29T12:00:00Z");
const DAY = 86_400_000;
const id = (n: number) => n.toString(16).padStart(32, "0");

function post(body: unknown, headers: Record<string, string> = {}): Request {
  return new Request(`${ORIGIN}/api/event`, {
    method: "POST",
    headers: { "Content-Type": "text/plain;charset=UTF-8", Origin: ORIGIN, "Sec-Fetch-Site": "same-origin", ...headers },
    body: typeof body === "string" ? body : JSON.stringify(body),
  });
}

function stats(token = TOKEN): Request {
  return new Request(`${ORIGIN}/api/stats`, { headers: { Authorization: `Bearer ${token}` } });
}

function env(): Env & { DB: ReturnType<typeof memoryD1> } {
  return { DB: memoryD1(), DASHBOARD_TOKEN: TOKEN };
}

const count = (e: ReturnType<typeof env>) => (e.DB.raw.prepare("SELECT COUNT(*) AS n FROM events").get() as { n: number }).n;

test("valid events are stored with the server's UTC day and nothing else", async () => {
  const e = env();
  const r = await handleEvent(post({ v: 1, type: "room_complete", id: id(1), band: "seedling", room: 2 }), e, NOW);
  assert.equal(r.status, 204);
  const rows = e.DB.raw.prepare("SELECT * FROM events").all();
  assert.deepEqual(
    rows.map((x) => ({ ...x })),
    [{ id: 1, day: "2026-10-29", daily_id: id(1), type: "room_complete", band: "seedling", room: 2, dur: null }],
  );
  assert.equal(r.headers.get("Cache-Control"), "no-store");
});

test("bad payloads, methods, origins and content types are rejected and not stored", async () => {
  const e = env();
  const cases: [Request, number][] = [
    [post({ v: 1, type: "session_start", id: id(1), band: "toddler" }), 400],
    [post({ v: 1, type: "session_start", id: id(1), band: "sprout", name: "Ada" }), 400],
    [post("not json"), 400],
    [post("x".repeat(600)), 413],
    [post({ v: 1, type: "session_start", id: id(1), band: "sprout" }, { Origin: "https://evil.example", "Sec-Fetch-Site": "cross-site" }), 403],
    [post({ v: 1, type: "session_start", id: id(1), band: "sprout" }, { "Sec-Fetch-Site": "same-site" }), 403],
    [post({ v: 1, type: "session_start", id: id(1), band: "sprout" }, { "Content-Type": "application/x-www-form-urlencoded" }), 415],
    [new Request(`${ORIGIN}/api/event`), 405],
  ];
  for (const [req, status] of cases) assert.equal((await handleEvent(req, e, NOW)).status, status);
  assert.equal(count(e), 0);
});

test("each device is capped per day", async () => {
  const e = env();
  const ev = { v: 1, type: "session_start", id: id(7), band: "sprout" };
  for (let i = 0; i < DAILY_CAP; i++) assert.equal((await handleEvent(post(ev), e, NOW)).status, 204);
  assert.equal((await handleEvent(post(ev), e, NOW)).status, 429);
  // Another device, and the same id tomorrow, are unaffected.
  assert.equal((await handleEvent(post({ ...ev, id: id(8) }), e, NOW)).status, 204);
  assert.equal((await handleEvent(post(ev), e, NOW + DAY)).status, 204);
  assert.equal(count(e), DAILY_CAP + 2);
});

test("events older than 90 days are purged", async () => {
  const e = env();
  const ev = { v: 1, type: "session_start", id: id(1), band: "sprout" };
  await handleEvent(post(ev), e, NOW - 90 * DAY); // 2026-07-31: 91 days including today, so purged
  await handleEvent(post(ev), e, NOW - 89 * DAY); // 2026-08-01: the oldest day kept (90 days including today)
  assert.equal(count(e), 2);
  await handleEvent(post({ ...ev, id: id(2) }), e, NOW);
  assert.deepEqual(
    e.DB.raw.prepare("SELECT day FROM events ORDER BY day").all().map((r) => r.day),
    ["2026-08-01", "2026-10-29"],
  );
  // The stats endpoint purges too, so nothing outlives retention on quiet days.
  const later = env();
  await handleEvent(post(ev), later, NOW - 100 * DAY);
  await handleStats(stats(), later, NOW);
  assert.equal(count(later), 0);
});

test("stats need the exact token and fail closed without one", async () => {
  const e = env();
  assert.equal((await handleStats(stats("wrong"), e, NOW)).status, 401);
  assert.equal((await handleStats(new Request(`${ORIGIN}/api/stats`), e, NOW)).status, 401);
  assert.equal((await handleStats(stats(), { DB: e.DB }, NOW)).status, 503);
  assert.equal((await handleStats(stats("short"), { DB: e.DB, DASHBOARD_TOKEN: "short" }, NOW)).status, 503);
  assert.equal((await handleStats(new Request(`${ORIGIN}/api/stats`, { method: "POST" }), e, NOW)).status, 405);
});

test("stats aggregate players, bands, sessions, funnel and durations", async () => {
  const e = env();
  const send = (n: number, extra: Record<string, unknown>, at = NOW) =>
    handleEvent(post({ v: 1, id: id(n), ...extra }), e, at);
  // Today: device 1 (sprout) plays two sessions and clears rooms 0 and 1.
  await send(1, { type: "session_start", band: "sprout" });
  await send(1, { type: "room_complete", band: "sprout", room: 0 });
  await send(1, { type: "room_complete", band: "sprout", room: 0 }); // duplicate: counted once
  await send(1, { type: "room_complete", band: "sprout", room: 1 });
  await send(1, { type: "session_end", band: "sprout", dur: "1to5m" });
  await send(1, { type: "session_start", band: "sprout" });
  await send(1, { type: "session_end", band: "sprout", dur: "5to15m" });
  // Today: device 2 (apprentice) finishes the trial.
  await send(2, { type: "session_start", band: "apprentice" });
  for (const room of [0, 1, 2, 3]) await send(2, { type: "room_complete", band: "apprentice", room });
  await send(2, { type: "trial_complete", band: "apprentice" });
  await send(2, { type: "session_end", band: "apprentice", dur: "15to30m" });
  // Yesterday: device 1 again, under yesterday's id (ids rotate daily).
  await send(3, { type: "session_start", band: "seedling" }, NOW - DAY);
  // 40 days ago: outside the dashboard window.
  await send(4, { type: "session_start", band: "seedling" }, NOW - 40 * DAY);

  const r = await handleStats(stats(), e, NOW);
  assert.equal(r.status, 200);
  const s = await r.json();
  assert.equal(s.today, "2026-10-29");
  assert.equal(s.players.today, 2);
  assert.deepEqual(s.players.todayByBand, { sprout: 1, seedling: 0, apprentice: 1 });
  assert.equal(s.players.playerDays, 3);
  assert.deepEqual(s.players.playerDaysByBand, { sprout: 1, seedling: 1, apprentice: 1 });
  assert.deepEqual(s.sessions, { today: 3, total: 4 });
  assert.equal(s.daily.length, 30);
  assert.deepEqual(s.daily[28], { day: "2026-10-28", players: 1, sessions: 1, byBand: { sprout: 0, seedling: 1, apprentice: 0 } });
  assert.deepEqual(s.funnel.started, { sprout: 1, seedling: 1, apprentice: 1, all: 3 });
  assert.deepEqual(
    s.funnel.rooms.map((x: { all: number }) => x.all),
    [2, 2, 1, 1],
  );
  assert.deepEqual(s.funnel.trialComplete, { sprout: 0, seedling: 0, apprentice: 1, all: 1 });
  assert.deepEqual(
    s.durations.map((d: { bucket: string; players: number }) => [d.bucket, d.players]),
    [
      ["lt1m", 0],
      ["1to5m", 0],
      ["5to15m", 1], // device 1's longest session today
      ["15to30m", 1],
      ["30mplus", 0],
    ],
  );
});
