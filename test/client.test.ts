// The browser module's pure parts: daily id rotation, opt-out, buckets.
import assert from "node:assert/strict";
import { test } from "node:test";
import { dailyId, durationBucket, optedOut, randomId, type StorageLike } from "../web/src/analytics.ts";

function memoryStorage(): StorageLike & { data: Map<string, string> } {
  const data = new Map<string, string>();
  return { data, getItem: (k) => data.get(k) ?? null, setItem: (k, v) => void data.set(k, v) };
}

test("randomId is 128 bits of lowercase hex", () => {
  const a = randomId();
  assert.match(a, /^[0-9a-f]{32}$/);
  assert.notEqual(a, randomId());
});

test("the id is stable within a UTC day and replaced on the next", () => {
  const s = memoryStorage();
  let n = 0;
  const make = () => (++n).toString(16).padStart(32, "0");
  const morning = dailyId(s, new Date("2026-10-29T00:00:01Z"), make);
  assert.equal(dailyId(s, new Date("2026-10-29T23:59:59Z"), make), morning);
  const tomorrow = dailyId(s, new Date("2026-10-30T00:00:00Z"), make);
  assert.notEqual(tomorrow, morning);
  assert.equal(s.data.get("rg.dailyId"), `2026-10-30.${tomorrow}`);
  assert.equal(s.data.size, 1, "only one key, overwritten daily");
});

test("blocked or broken storage still yields a day-scoped id without throwing", () => {
  const throwing: StorageLike = {
    getItem() {
      throw new Error("SecurityError");
    },
    setItem() {
      throw new Error("QuotaExceededError");
    },
  };
  const day = new Date("2026-11-02T10:00:00Z");
  const a = dailyId(throwing, day);
  assert.match(a, /^[0-9a-f]{32}$/);
  assert.equal(dailyId(throwing, day), a, "in-memory for the page load");
  assert.notEqual(dailyId(null, new Date("2026-11-03T10:00:00Z")), a);
});

test("a tampered stored value is replaced", () => {
  const s = memoryStorage();
  s.data.set("rg.dailyId", "2026-10-29.Ada Lovelace");
  assert.match(dailyId(s, new Date("2026-10-29T12:00:00Z")), /^[0-9a-f]{32}$/);
});

test("Do Not Track and Global Privacy Control both opt out", () => {
  assert.equal(optedOut({ doNotTrack: "1" }), true);
  assert.equal(optedOut({ globalPrivacyControl: true }), true);
  assert.equal(optedOut({ doNotTrack: "0", globalPrivacyControl: false }), false);
  assert.equal(optedOut({ doNotTrack: null }), false);
});

test("duration buckets", () => {
  const min = 60_000;
  assert.equal(durationBucket(0), "lt1m");
  assert.equal(durationBucket(59_999), "lt1m");
  assert.equal(durationBucket(min), "1to5m");
  assert.equal(durationBucket(14 * min), "5to15m");
  assert.equal(durationBucket(29 * min), "15to30m");
  assert.equal(durationBucket(3 * 60 * min), "30mplus");
});
