// I-005: the quest_complete event, migration 0002, and the quest funnel.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { DatabaseSync } from "node:sqlite";
import { test } from "node:test";
import { handleEvent, handleStats, MAX_QUEST, shapeStats, validateEvent, type Env } from "../functions/_lib/analytics.ts";
import { memoryD1 } from "./d1-sqlite.ts";

const ID = "0123456789abcdef0123456789abcdef";
const ok = (o: unknown) => validateEvent(JSON.stringify(o));
const quest = (q: unknown) => ({ v: 1, type: "quest_complete", id: ID, band: "seedling", quest: q });

// ------------------------------------------------------------ validateEvent

test("quest_complete is accepted for quest 0 and quest 15, and maps into room", () => {
  assert.equal(MAX_QUEST, 15);
  assert.deepEqual(ok(quest(0)), {
    ok: true,
    event: { type: "quest_complete", id: ID, band: "seedling", room: 0, dur: null },
  });
  assert.deepEqual(ok(quest(15)), {
    ok: true,
    event: { type: "quest_complete", id: ID, band: "seedling", room: 15, dur: null },
  });
});

test("quest_complete rejects bad quest values, a missing quest and extra keys", () => {
  const bad: [string, unknown][] = [
    ["quest 16", quest(16)],
    ["quest -1", quest(-1)],
    ["quest 1.5", quest(1.5)],
    ["quest as string", quest("0")],
    ["quest null", quest(null)],
    ["quest missing", { v: 1, type: "quest_complete", id: ID, band: "seedling" }],
    ["room on quest_complete", { v: 1, type: "quest_complete", id: ID, band: "seedling", room: 0 }],
    ["room and quest", { ...quest(0), room: 0 }],
    ["extra free text", { ...quest(0), name: "Ada" }],
    ["user agent", { ...quest(0), ua: "Mozilla/5.0" }],
    ["quest on session_start", { v: 1, type: "session_start", id: ID, band: "seedling", quest: 0 }],
    ["quest on session_end", { v: 1, type: "session_end", id: ID, band: "seedling", dur: "lt1m", quest: 0 }],
    ["quest on room_complete", { v: 1, type: "room_complete", id: ID, band: "seedling", room: 0, quest: 0 }],
    ["near-miss type", { ...quest(0), type: "quest_completed" }],
    ["bad band", { ...quest(0), band: "adult" }],
  ];
  for (const [why, payload] of bad) assert.equal(ok(payload).ok, false, why);
});

test("the classic room_complete and trial_complete events stay valid", () => {
  assert.equal(ok({ v: 1, type: "room_complete", id: ID, band: "sprout", room: 2 }).ok, true);
  assert.equal(ok({ v: 1, type: "trial_complete", id: ID, band: "sprout" }).ok, true);
});

// --------------------------------------------------------------- migration

const migration = (name: string) => readFileSync(new URL(`../migrations/${name}`, import.meta.url), "utf8");

test("migration 0002 keeps 0001 rows and adds quest_complete with the same CHECKs", () => {
  const db = new DatabaseSync(":memory:");
  db.exec(migration("0001_events.sql"));
  const ins = db.prepare("INSERT INTO events (day, daily_id, type, band, room, dur) VALUES (?, ?, ?, ?, ?, ?)");
  ins.run("2026-10-28", ID, "session_start", "sprout", null, null);
  ins.run("2026-10-28", ID, "room_complete", "sprout", 3, null);
  ins.run("2026-10-29", ID, "session_end", "apprentice", null, 2);
  // 0001 rejects the new type.
  assert.throws(() => ins.run("2026-10-29", ID, "quest_complete", "sprout", 0, null));
  const before = db.prepare("SELECT * FROM events ORDER BY id").all().map((r) => ({ ...r }));

  db.exec(migration("0002_quest_complete.sql"));
  const after = db.prepare("SELECT * FROM events ORDER BY id").all().map((r) => ({ ...r }));
  assert.deepEqual(after, before, "rows survive the table rebuild unchanged");

  const ins2 = db.prepare("INSERT INTO events (day, daily_id, type, band, room, dur) VALUES (?, ?, ?, ?, ?, ?)");
  ins2.run("2026-10-29", ID, "quest_complete", "seedling", 0, null);
  ins2.run("2026-10-29", ID, "quest_complete", "seedling", 15, null);
  assert.throws(() => ins2.run("2026-10-29", ID, "quest_complete", "seedling", 16, null), /CHECK/);
  assert.throws(() => ins2.run("2026-10-29", ID, "quest_completed", "seedling", 0, null), /CHECK/);
  assert.throws(() => ins2.run("2026-10-29", ID, "quest_complete", "my name is Ada", 0, null), /CHECK/);
  assert.throws(() => ins2.run("2026-10-29", "short", "quest_complete", "seedling", 0, null), /CHECK/);
  assert.throws(() => ins2.run("2026-10-29", ID, "session_end", "seedling", null, 5), /CHECK/);
  // STRICT is kept: a text room is a type error, not a silent coercion.
  assert.throws(() => ins2.run("2026-10-29", ID, "quest_complete", "seedling", "zero", null));

  const cols = db.prepare("PRAGMA table_info(events)").all().map((r) => r.name);
  assert.deepEqual(cols, ["id", "day", "daily_id", "type", "band", "room", "dur"], "no new column");
  const idx = db.prepare("PRAGMA index_list(events)").all().map((r) => r.name);
  assert.ok(idx.includes("events_day_id"), "the day/id index is re-created");
  const strict = db.prepare("SELECT sql FROM sqlite_schema WHERE name = 'events'").get() as { sql: string };
  assert.match(strict.sql, /\)\s*STRICT\s*$/);
});

// ---------------------------------------------------------------- handlers

const ORIGIN = "https://realgame.example";
const TOKEN = "t".repeat(48);
const NOW = Date.parse("2026-10-29T12:00:00Z");
const id = (n: number) => n.toString(16).padStart(32, "0");
const post = (body: unknown) =>
  new Request(`${ORIGIN}/api/event`, {
    method: "POST",
    headers: { "Content-Type": "text/plain;charset=UTF-8", Origin: ORIGIN, "Sec-Fetch-Site": "same-origin" },
    body: JSON.stringify(body),
  });
const stats = () => new Request(`${ORIGIN}/api/stats`, { headers: { Authorization: `Bearer ${TOKEN}` } });

test("quest_complete is stored in the room column and shows in funnel.quests", async () => {
  const e: Env & { DB: ReturnType<typeof memoryD1> } = { DB: memoryD1(), DASHBOARD_TOKEN: TOKEN };
  const send = (n: number, extra: Record<string, unknown>) => handleEvent(post({ v: 1, id: id(n), ...extra }), e, NOW);
  assert.equal((await send(1, { type: "session_start", band: "seedling" })).status, 204);
  assert.equal((await send(1, { type: "quest_complete", band: "seedling", quest: 0 })).status, 204);
  assert.equal((await send(1, { type: "quest_complete", band: "seedling", quest: 0 })).status, 204); // counted once
  assert.equal((await send(2, { type: "session_start", band: "apprentice" })).status, 204);
  assert.equal((await send(2, { type: "quest_complete", band: "apprentice", quest: 2 })).status, 204);
  // A classic room_complete still works and does not leak into quests.
  assert.equal((await send(3, { type: "room_complete", band: "sprout", room: 1 })).status, 204);
  assert.equal((await send(1, { type: "quest_complete", band: "seedling", quest: 16 })).status, 400);

  const rows = e.DB.raw.prepare("SELECT daily_id, type, band, room FROM events WHERE type = 'quest_complete' ORDER BY id").all();
  assert.deepEqual(
    rows.map((r) => ({ ...r })),
    [
      { daily_id: id(1), type: "quest_complete", band: "seedling", room: 0 },
      { daily_id: id(1), type: "quest_complete", band: "seedling", room: 0 },
      { daily_id: id(2), type: "quest_complete", band: "apprentice", room: 2 },
    ],
  );

  const r = await handleStats(stats(), e, NOW);
  assert.equal(r.status, 200);
  const s = await r.json();
  assert.deepEqual(s.funnel.quests, [
    { quest: 0, sprout: 0, seedling: 1, apprentice: 0, all: 1 },
    { quest: 1, sprout: 0, seedling: 0, apprentice: 0, all: 0 },
    { quest: 2, sprout: 0, seedling: 0, apprentice: 1, all: 1 },
  ]);
  assert.deepEqual(
    s.funnel.rooms.map((x: { room: number; all: number }) => [x.room, x.all]),
    [
      [0, 0],
      [1, 1],
    ],
  );
  assert.equal(s.funnel.started.all, 2);
});

test("shapeStats builds quests dense from 0 and keeps rooms to room_complete only", () => {
  const s = shapeStats(
    "2026-10-29",
    {
      days: [],
      dayBands: [],
      funnel: [
        { type: "quest_complete", room: 1, band: "sprout", players: 3 },
        { type: "room_complete", room: 0, band: "sprout", players: 1 },
      ],
      durations: [],
    },
    "x",
  );
  assert.deepEqual(
    s.funnel.quests.map((q) => [q.quest, q.sprout, q.all]),
    [
      [0, 0, 0],
      [1, 3, 3],
    ],
  );
  assert.deepEqual(
    s.funnel.rooms.map((q) => [q.room, q.all]),
    [[0, 1]],
  );
  const empty = shapeStats("2026-10-29", { days: [], dayBands: [], funnel: [], durations: [] }, "x");
  assert.deepEqual(empty.funnel.quests, []);
});
