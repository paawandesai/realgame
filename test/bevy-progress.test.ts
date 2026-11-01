// I-027: spell progress saving in the 3D client (crates/bevy-client/web/progress.js).
// Tests 12-15 and 15b of docs/loop/items/I-027/spec.md.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import {
  createSaver,
  localDay,
  MAX_SAVE,
  readSave,
  SAVE_RE,
  startProgress,
  STORE_KEY,
} from "../crates/bevy-client/web/progress.js";

type Store = { data: Map<string, string>; getItem(k: string): string | null; setItem(k: string, v: string): void };
function memoryStorage(init: Record<string, string> = {}): Store {
  const data = new Map<string, string>(Object.entries(init));
  return { data, getItem: (k) => data.get(k) ?? null, setItem: (k, v) => void data.set(k, v) };
}
const throwing = {
  getItem() { throw new Error("blocked"); },
  setItem() { throw new Error("blocked"); },
};
function at(iso: string, offsetMin: number): Date {
  const d = new Date(iso);
  d.getTimezoneOffset = () => offsetMin;
  return d;
}

test("constants", () => {
  assert.equal(STORE_KEY, "rg.progress.v1");
  assert.equal(MAX_SAVE, 512);
  assert.ok(SAVE_RE.test("p1"));
  assert.ok(SAVE_RE.test("p1|water:2:40:2:1:20756:0"));
  assert.ok(!SAVE_RE.test("p1|water:1:-1:0:0:0:0"));
  assert.ok(!SAVE_RE.test("p1|water:1:0:0:0:0:0;"));
});

// 12.
test("12: localDay counts local calendar days", () => {
  assert.equal(localDay(at("2026-10-30T23:30:00Z", 0)), 20756);
  assert.equal(localDay(at("2026-10-30T23:30:00Z", -120)), 20757);
  assert.equal(localDay(at("2026-10-31T03:00:00Z", 420)), 20756);
});

// 13.
test("13: readSave returns only a valid save", () => {
  const ok = "p1|water:1:10:0:0:0:0";
  assert.equal(readSave(memoryStorage({ [STORE_KEY]: ok })), ok);
  assert.equal(readSave(memoryStorage()), null);
  assert.equal(readSave(memoryStorage({ [STORE_KEY]: "garbage" })), null);
  const long = "p1|water:1:10:0:0:0:0" + "0".repeat(513 - ok.length);
  assert.equal(long.length, 513);
  assert.ok(SAVE_RE.test(long));
  assert.equal(readSave(memoryStorage({ [STORE_KEY]: long })), null);
  assert.equal(readSave(throwing), null);
  // It never rewrites a bad save.
  const bad = memoryStorage({ [STORE_KEY]: "garbage" });
  readSave(bad);
  assert.equal(bad.data.get(STORE_KEY), "garbage");
});

// 14.
test("14: createSaver writes once per progRev increase", () => {
  const store = memoryStorage();
  const writes: string[] = [];
  const realSet = store.setItem;
  store.setItem = (k, v) => { writes.push(`${k}=${v}`); realSet(k, v); };
  const seq = [
    { progRev: 0, progSave: "p1" },
    { progRev: 0, progSave: "p1" },
    { progRev: 1, progSave: "p1|water:1:10:0:0:0:0" },
    { progRev: 1, progSave: "p1|water:1:10:0:0:0:0" },
    { progRev: 3, progSave: "p1|water:1:30:0:0:0:0" },
  ];
  let n = 0;
  const saver = createSaver({ storage: store, getState: () => seq[n++] });
  for (let i = 0; i < seq.length; i++) saver.poll();
  assert.deepEqual(writes, [
    `${STORE_KEY}=p1|water:1:10:0:0:0:0`,
    `${STORE_KEY}=p1|water:1:30:0:0:0:0`,
  ]);
});

test("14: createSaver swallows failures and writes nothing bad", () => {
  const good = { progRev: 1, progSave: "p1|water:1:10:0:0:0:0" };
  assert.doesNotThrow(() => createSaver({ storage: throwing, getState: () => good }).poll());
  for (const getState of [() => { throw new Error("x"); }, () => null, () => undefined]) {
    const store = memoryStorage();
    assert.doesNotThrow(() => createSaver({ storage: store, getState }).poll());
    assert.equal(store.data.size, 0);
  }
  const store = memoryStorage();
  createSaver({ storage: store, getState: () => ({ progRev: 2, progSave: "p1|x;y" }) }).poll();
  assert.equal(store.data.size, 0);
});

// 15b.
test("15b: SPELL_IDS in progress.rs match content/spells.json in slot order", () => {
  const book = JSON.parse(readFileSync(new URL("../content/spells.json", import.meta.url), "utf8"));
  const ids = [...book.spells].sort((a, b) => a.slot - b.slot).map((s: { id: string }) => s.id);
  const rs = readFileSync(new URL("../crates/bevy-client/src/progress.rs", import.meta.url), "utf8");
  const line = rs.split("\n").find((l) => l.includes("pub const SPELL_IDS"));
  assert.ok(line, "no SPELL_IDS line");
  const got = [...line.matchAll(/"([^"]*)"/g)].map((m) => m[1]);
  assert.deepEqual(got, ids);
  assert.equal(got.length, 4);
});

// 15.
function fakeWindow(saved: string | null) {
  const cmds: string[] = [];
  const timers: Array<() => void> = [];
  const listeners: Record<string, Array<() => void>> = {};
  const FixedDate = class extends Date {
    constructor() { super("2026-10-30T12:00:00Z"); }
    getTimezoneOffset() { return 0; }
  };
  const win: Record<string, unknown> = {
    __game: { cmd: (c: string) => void cmds.push(c) },
    localStorage: memoryStorage(saved === null ? {} : { [STORE_KEY]: saved }),
    setInterval: (f: () => void) => { timers.push(f); return timers.length; },
    addEventListener: (t: string, f: () => void) => { (listeners[t] ??= []).push(f); },
    document: { visibilityState: "visible", addEventListener: (t: string, f: () => void) => { (listeners[t] ??= []).push(f); } },
    Date: FixedDate,
  };
  return { win, cmds, timers, listeners };
}

test("15: startProgress sends the day, then the save", () => {
  const f = fakeWindow("p1|water:1:10:0:0:0:0");
  startProgress(f.win);
  assert.deepEqual(f.cmds, ["prog day 20756", "prog load p1|water:1:10:0:0:0:0"]);
  assert.equal(f.timers.length, 1);
  assert.ok(f.listeners.pagehide?.length);
  assert.ok(f.listeners.visibilitychange?.length);
});

test("15: startProgress sends only the day without a valid save", () => {
  for (const saved of [null, "garbage"]) {
    const f = fakeWindow(saved);
    startProgress(f.win);
    assert.deepEqual(f.cmds, ["prog day 20756"]);
  }
});

test("15: startProgress never throws", () => {
  const f = fakeWindow("p1");
  delete f.win.__game;
  assert.doesNotThrow(() => startProgress(f.win));
  assert.doesNotThrow(() => startProgress({}));
  const g = fakeWindow("p1");
  g.win.localStorage = throwing;
  assert.doesNotThrow(() => startProgress(g.win));
  assert.deepEqual(g.cmds, ["prog day 20756"]);
});

test("15: startProgress saves from the published state (a JSON string) on poll and pagehide", () => {
  const f = fakeWindow(null);
  startProgress(f.win);
  f.win.__gameState = JSON.stringify({ progRev: 1, progSave: "p1|fire:1:10:0:0:0:0" });
  f.timers[0]!();
  assert.equal((f.win.localStorage as Store).data.get(STORE_KEY), "p1|fire:1:10:0:0:0:0");
  f.win.__gameState = JSON.stringify({ progRev: 2, progSave: "p1|fire:1:20:0:0:0:0" });
  f.listeners.pagehide![0]!();
  assert.equal((f.win.localStorage as Store).data.get(STORE_KEY), "p1|fire:1:20:0:0:0:0");
});

// I-030 N3: the day is sent again after local midnight (the I-027 should-fix).
test("I-030 N3: the poll re-sends prog day once when the local day changes", () => {
  const cmds: string[] = [];
  const timers: Array<() => void> = [];
  let now = Date.parse("2026-10-30T23:59:58Z");
  const Clock = class extends Date {
    constructor() { super(now); }
    getTimezoneOffset() { return 0; }
  };
  const win: Record<string, unknown> = {
    __game: { cmd: (c: string) => void cmds.push(c) },
    localStorage: memoryStorage({ [STORE_KEY]: "p1|water:1:10:0:0:0:0" }),
    setInterval: (f: () => void) => { timers.push(f); return timers.length; },
    addEventListener: () => {},
    document: { visibilityState: "visible", addEventListener: () => {} },
    Date: Clock,
  };
  startProgress(win);
  assert.deepEqual(cmds, ["prog day 20756", "prog load p1|water:1:10:0:0:0:0"], "boot order unchanged");
  assert.equal(timers.length, 1);
  timers[0]!();
  timers[0]!();
  assert.equal(cmds.length, 2, "same day: nothing more");
  now += 5000; // past local midnight
  timers[0]!();
  timers[0]!();
  timers[0]!();
  assert.deepEqual(cmds.slice(2), ["prog day 20757"], "exactly one new day");
  now += 86_400_000;
  timers[0]!();
  assert.deepEqual(cmds.slice(2), ["prog day 20757", "prog day 20758"]);
});
