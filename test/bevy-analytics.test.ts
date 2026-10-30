// I-005: the 3D client's analytics module (crates/bevy-client/web/analytics.js),
// pure parts only. Ported from web/src/analytics.ts, so the privacy rules match.
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  analyticsOn,
  capped,
  createSession,
  dailyId,
  durationBucket,
  eventBody,
  optedOut,
  questEvents,
  randomId,
  send,
  SESSION_ID_MAX_MS,
} from "../crates/bevy-client/web/analytics.js";

type Store = { data: Map<string, string>; getItem(k: string): string | null; setItem(k: string, v: string): void };
function memoryStorage(): Store {
  const data = new Map<string, string>();
  return { data, getItem: (k) => data.get(k) ?? null, setItem: (k, v) => void data.set(k, v) };
}
const ID = "0123456789abcdef0123456789abcdef";

test("importing the module touches no browser globals", async () => {
  // This file already imported it under plain Node, where window and
  // document do not exist; a fresh import must not throw either.
  const spec = "../crates/bevy-client/web/analytics.js?again";
  const mod = await import(spec);
  assert.equal(typeof mod.start, "function");
});

test("Do Not Track and Global Privacy Control both opt out", () => {
  assert.equal(optedOut({ doNotTrack: "1" }), true);
  assert.equal(optedOut({ doNotTrack: "yes" }), true);
  assert.equal(optedOut({ globalPrivacyControl: true }), true);
  assert.equal(optedOut({ doNotTrack: "0", globalPrivacyControl: false }), false);
  assert.equal(optedOut({ doNotTrack: null }), false);
  assert.equal(optedOut(undefined), true, "no navigator: send nothing");
});

test("randomId is 128 bits of lowercase hex", () => {
  assert.match(randomId(), /^[0-9a-f]{32}$/);
  assert.notEqual(randomId(), randomId());
});

test("the daily id is stable within a UTC day and rotates at the boundary", () => {
  const s = memoryStorage();
  let n = 0;
  const make = () => (++n).toString(16).padStart(32, "0");
  const a = dailyId(s, new Date("2026-10-29T00:00:01Z"), make);
  assert.equal(dailyId(s, new Date("2026-10-29T23:59:59.999Z"), make), a);
  const b = dailyId(s, new Date("2026-10-30T00:00:00Z"), make);
  assert.notEqual(b, a);
  assert.equal(s.data.get("rg.dailyId"), `2026-10-30.${b}`);
  assert.equal(s.data.size, 1);
});

test("blocked storage still yields a day-scoped id without throwing", () => {
  const throwing = {
    getItem(): string | null {
      throw new Error("SecurityError");
    },
    setItem(): void {
      throw new Error("QuotaExceededError");
    },
  };
  const day = new Date("2026-11-02T10:00:00Z");
  const a = dailyId(throwing, day);
  assert.match(a, /^[0-9a-f]{32}$/);
  assert.equal(dailyId(throwing, day), a, "kept in memory for the page load");
  assert.notEqual(dailyId(null, new Date("2026-11-03T10:00:00Z")), a, "and still rotates daily");
  const s = memoryStorage();
  s.data.set("rg.dailyId", "2026-11-04.Ada Lovelace");
  assert.match(dailyId(s, new Date("2026-11-04T09:00:00Z")), /^[0-9a-f]{32}$/);
});

test("duration buckets", () => {
  const min = 60_000;
  assert.equal(durationBucket(0), "lt1m");
  assert.equal(durationBucket(min - 1), "lt1m");
  assert.equal(durationBucket(min), "1to5m");
  assert.equal(durationBucket(5 * min), "5to15m");
  assert.equal(durationBucket(15 * min), "15to30m");
  assert.equal(durationBucket(30 * min), "30mplus");
  assert.equal(durationBucket(10 * 60 * min), "30mplus");
});

test("the payload builder emits exactly the allowed keys", () => {
  const keys = (s: string | null) => (s === null ? null : Object.keys(JSON.parse(s)).sort());
  assert.deepEqual(JSON.parse(eventBody({ type: "session_start", band: "seedling" }, ID) ?? "null"), {
    v: 1,
    type: "session_start",
    band: "seedling",
    id: ID,
  });
  assert.deepEqual(keys(eventBody({ type: "quest_complete", band: "sprout", quest: 0 }, ID)), ["band", "id", "quest", "type", "v"]);
  assert.deepEqual(keys(eventBody({ type: "session_end", band: "apprentice", dur: "lt1m" }, ID)), ["band", "dur", "id", "type", "v"]);
  // Extra or wrong fields never make it out: the event is dropped, not trimmed.
  const loose = { type: "session_start", band: "seedling", ua: "Mozilla/5.0" } as unknown as Parameters<typeof eventBody>[0];
  assert.equal(eventBody(loose, ID), null);
  assert.equal(eventBody({ type: "quest_complete", band: "sprout", quest: 16 }, ID), null);
  assert.equal(eventBody({ type: "quest_complete", band: "sprout", quest: 1.5 }, ID), null);
  assert.equal(eventBody({ type: "quest_complete", band: "sprout" }, ID), null);
  assert.equal(eventBody({ type: "session_end", band: "sprout", dur: "2h" }, ID), null);
  assert.equal(eventBody({ type: "session_start", band: "toddler" }, ID), null);
  assert.equal(eventBody({ type: "room_complete", band: "sprout", room: 1 }, ID), null, "3D sends no classic events");
  assert.equal(eventBody({ type: "session_start", band: "sprout" }, "not-an-id"), null);
});

test("questEvents: once on playing to won, nothing on lost, a repeat or a replay", () => {
  const sent = new Set<number>();
  const playing = { outcome: "playing" };
  const won = { outcome: "won" };
  assert.deepEqual(questEvents(null, won, sent), [], "no previous state: a stale win is not counted");
  assert.deepEqual(questEvents(playing, playing, sent), []);
  assert.deepEqual(questEvents(playing, { outcome: "lost" }, sent), [], "lost sends nothing");
  assert.deepEqual(questEvents(playing, won, sent), [0], "Save the Greenhouse is quest 0");
  assert.deepEqual(questEvents(won, won, sent), [], "a repeated won sends nothing");
  // R (reset) then a second win in the same session.
  assert.deepEqual(questEvents(won, playing, sent), []);
  assert.deepEqual(questEvents(playing, won, sent), [], "at most once per quest per session");
  // A state that names its quest uses it; anything invalid falls back to 0.
  const s2 = new Set<number>();
  assert.deepEqual(questEvents({ outcome: "playing", quest: 3 }, { outcome: "won", quest: 3 }, s2), [3]);
  assert.deepEqual(questEvents({ outcome: "playing", quest: 99 }, { outcome: "won", quest: 99 }, s2), [0]);
  assert.deepEqual(questEvents({ outcome: "playing", quest: 1 }, { outcome: "won", quest: "1" }, new Set()), [0]);
});

test("analyticsOn reads the rg-analytics meta switch", () => {
  const doc = (content: string | null) => ({
    querySelector: (sel: string) => (sel === 'meta[name="rg-analytics"]' && content !== null ? { content } : null),
  });
  assert.equal(analyticsOn(doc("on")), true);
  assert.equal(analyticsOn(doc("off")), false);
  assert.equal(analyticsOn(doc("ON ")), false, "exact value only");
  assert.equal(analyticsOn(doc(null)), false);
  assert.equal(analyticsOn(undefined), false);
});

test("a session sends session_start once after the first state, quests once, and growing session_end buckets", () => {
  const out: Record<string, unknown>[] = [];
  let t = 0;
  let state: Record<string, unknown> | null = null;
  const s = createSession({ band: "seedling", send: (e) => out.push(e), now: () => t, getState: () => state });
  s.hidden();
  s.poll();
  assert.deepEqual(out, [], "nothing before the game's first state");
  state = { outcome: "playing" };
  s.poll();
  s.poll();
  assert.deepEqual(out, [{ type: "session_start", band: "seedling" }]);
  state = { outcome: "won" };
  s.poll();
  s.poll();
  state = { outcome: "playing" };
  s.poll();
  state = { outcome: "won" };
  s.poll();
  assert.deepEqual(out.slice(1), [{ type: "quest_complete", band: "seedling", quest: 0 }]);
  t = 30_000;
  s.hidden();
  s.hidden();
  t = 59_000;
  s.hidden();
  t = 6 * 60_000;
  s.hidden();
  assert.deepEqual(out.slice(2), [
    { type: "session_end", band: "seedling", dur: "lt1m" },
    { type: "session_end", band: "seedling", dur: "5to15m" },
  ]);
});

test("send uses the id it is given (one id per session)", () => {
  const bodies: string[] = [];
  const before = Object.getOwnPropertyDescriptor(globalThis, "navigator");
  Object.defineProperty(globalThis, "navigator", {
    value: { sendBeacon: (_u: string, b: string) => (bodies.push(b), true) },
    configurable: true,
  });
  try {
    assert.equal(typeof (globalThis as { localStorage?: unknown }).localStorage, "undefined");
    send({ type: "session_end", band: "seedling", dur: "lt1m" }, ID);
    assert.equal(bodies.length, 1);
    assert.equal(JSON.parse(bodies[0]!).id, ID);
  } finally {
    if (before) Object.defineProperty(globalThis, "navigator", before);
    else delete (globalThis as { navigator?: unknown }).navigator;
  }
});

test("a session's pinned id stops sending after 24 hours", () => {
  const sent: string[] = [];
  let t = 1_000;
  const f = capped((e: { type: string }) => sent.push(e.type), 1_000, () => t);
  f({ type: "session_start" });
  t = 1_000 + SESSION_ID_MAX_MS;
  f({ type: "quest_complete" });
  t = 1_000 + SESSION_ID_MAX_MS + 1;
  f({ type: "session_end" });
  assert.equal(SESSION_ID_MAX_MS, 24 * 60 * 60 * 1000);
  assert.deepEqual(sent, ["session_start", "quest_complete"]);
});
