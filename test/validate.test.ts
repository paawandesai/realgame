import assert from "node:assert/strict";
import { test } from "node:test";
import { daysBefore, MAX_BODY_BYTES, shapeStats, tokenMatches, validateEvent } from "../functions/_lib/analytics.ts";

const ID = "0123456789abcdef0123456789abcdef";
const ok = (o: unknown) => validateEvent(JSON.stringify(o));

test("accepts each of the four event shapes", () => {
  assert.deepEqual(ok({ v: 1, type: "session_start", id: ID, band: "sprout" }), {
    ok: true,
    event: { type: "session_start", id: ID, band: "sprout", room: null, dur: null },
  });
  assert.deepEqual(ok({ v: 1, type: "room_complete", id: ID, band: "seedling", room: 3 }), {
    ok: true,
    event: { type: "room_complete", id: ID, band: "seedling", room: 3, dur: null },
  });
  assert.equal(ok({ v: 1, type: "trial_complete", id: ID, band: "apprentice" }).ok, true);
  assert.deepEqual(ok({ v: 1, type: "session_end", id: ID, band: "apprentice", dur: "5to15m" }), {
    ok: true,
    event: { type: "session_end", id: ID, band: "apprentice", room: null, dur: 2 },
  });
});

test("rejects anything outside the allow-list", () => {
  const base = { v: 1, type: "session_start", id: ID, band: "sprout" };
  const bad: [string, unknown][] = [
    ["not JSON", "{"],
    ["array", []],
    ["null", null],
    ["string", "session_start"],
    ["wrong version", { ...base, v: 2 }],
    ["missing version", { type: "session_start", id: ID, band: "sprout" }],
    ["unknown type", { ...base, type: "page_view" }],
    ["unknown band", { ...base, band: "adult" }],
    ["band case", { ...base, band: "Sprout" }],
    ["extra free text", { ...base, name: "Ada" }],
    ["typed formula smuggled in", { ...base, formula: "2H2 + O2 -> 2H2O" }],
    ["room on session_start", { ...base, room: 1 }],
    ["short id", { ...base, id: "abc" }],
    ["uppercase id", { ...base, id: ID.toUpperCase() }],
    ["id with an email", { ...base, id: "kid@example.com000000000000000000" }],
    ["missing id", { v: 1, type: "session_start", band: "sprout" }],
    ["room missing", { v: 1, type: "room_complete", id: ID, band: "sprout" }],
    ["room negative", { v: 1, type: "room_complete", id: ID, band: "sprout", room: -1 }],
    ["room too big", { v: 1, type: "room_complete", id: ID, band: "sprout", room: 16 }],
    ["room fractional", { v: 1, type: "room_complete", id: ID, band: "sprout", room: 1.5 }],
    ["room as string", { v: 1, type: "room_complete", id: ID, band: "sprout", room: "1" }],
    ["dur unknown", { v: 1, type: "session_end", id: ID, band: "sprout", dur: "2h" }],
    ["dur as number", { v: 1, type: "session_end", id: ID, band: "sprout", dur: 3 }],
  ];
  for (const [why, payload] of bad) {
    const r = validateEvent(typeof payload === "string" && why === "not JSON" ? payload : JSON.stringify(payload));
    assert.equal(r.ok, false, why);
  }
  assert.deepEqual(validateEvent(" ".repeat(MAX_BODY_BYTES + 1)), { ok: false, error: "too large" });
});

test("room boundaries 0 and 15 are accepted", () => {
  for (const room of [0, 15]) assert.equal(ok({ v: 1, type: "room_complete", id: ID, band: "sprout", room }).ok, true);
});

test("daysBefore crosses months and years in UTC", () => {
  assert.equal(daysBefore("2026-03-01", 1), "2026-02-28");
  assert.equal(daysBefore("2026-01-01", 1), "2025-12-31");
  assert.equal(daysBefore("2026-10-29", 29), "2026-09-30");
});

test("tokenMatches is exact", async () => {
  const t = "a".repeat(40);
  assert.equal(await tokenMatches(t, t), true);
  assert.equal(await tokenMatches(t.slice(1), t), false);
  assert.equal(await tokenMatches(`${t}b`, t), false);
  assert.equal(await tokenMatches("", t), false);
});

test("shapeStats zero-fills 30 days, all bands and every room up to the highest", () => {
  const s = shapeStats(
    "2026-10-29",
    {
      days: [
        { day: "2026-10-29", players: 3, sessions: 4 },
        { day: "2026-10-01", players: 1, sessions: 1 },
        { day: "2026-09-01", players: 9, sessions: 9 }, // outside the window: ignored
      ],
      dayBands: [
        { day: "2026-10-29", band: "sprout", players: 2 },
        { day: "2026-10-29", band: "apprentice", players: 1 },
        { day: "2026-10-01", band: "seedling", players: 1 },
      ],
      funnel: [
        { type: "session_start", room: null, band: "sprout", players: 2 },
        { type: "session_start", room: null, band: "apprentice", players: 2 },
        { type: "room_complete", room: 0, band: "sprout", players: 2 },
        { type: "room_complete", room: 2, band: "apprentice", players: 1 },
        { type: "trial_complete", room: null, band: "apprentice", players: 1 },
      ],
      durations: [
        { dur: 0, players: 1 },
        { dur: 3, players: 2 },
      ],
    },
    "2026-10-29T12:00:00.000Z",
  );
  assert.equal(s.daily.length, 30);
  assert.equal(s.daily[0]!.day, "2026-09-30");
  assert.equal(s.daily[29]!.day, "2026-10-29");
  assert.equal(s.players.today, 3);
  assert.deepEqual(s.players.todayByBand, { sprout: 2, seedling: 0, apprentice: 1 });
  assert.equal(s.players.playerDays, 4);
  assert.deepEqual(s.players.playerDaysByBand, { sprout: 2, seedling: 1, apprentice: 1 });
  assert.deepEqual(s.sessions, { today: 4, total: 5 });
  assert.deepEqual(s.funnel.started, { sprout: 2, seedling: 0, apprentice: 2, all: 4 });
  assert.deepEqual(
    s.funnel.rooms.map((r) => [r.room, r.all]),
    [
      [0, 2],
      [1, 0],
      [2, 1],
    ],
  );
  assert.equal(s.funnel.trialComplete.all, 1);
  assert.deepEqual(
    s.durations.map((d) => d.players),
    [1, 0, 0, 2, 0],
  );
});

test("shapeStats with no data is all zeros, not an error", () => {
  const s = shapeStats("2026-10-29", { days: [], dayBands: [], funnel: [], durations: [] }, "x");
  assert.equal(s.players.today, 0);
  assert.equal(s.funnel.rooms.length, 0);
  assert.equal(s.daily.every((d) => d.players === 0), true);
});
