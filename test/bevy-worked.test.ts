// I-029: the worked-cast overlay (crates/bevy-client/web/worked.js).
// Node tests 8-13 of docs/loop/items/I-029/spec.md. A minimal DOM shim (as in
// test/bench.test.ts 11b), a fake `cmd` and a fake `bench` whose
// mountBuilder records its options and renders a few pieces.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { createWorked } from "../crates/bevy-client/web/worked.js";

type Rect = { left: number; top: number; width: number; height: number; right: number; bottom: number };
const rect = (left: number, top: number, width: number, height: number): Rect => ({
  left, top, width, height, right: left + width, bottom: top + height,
});
type Ev = {
  type: string;
  key?: string;
  code?: string;
  shiftKey?: boolean;
  target: unknown;
  stopped: boolean;
  prevented: boolean;
  stopImmediatePropagation(): void;
  preventDefault(): void;
};
type Listener = (e: Ev) => void;

class El {
  tagName: string;
  ownerDocument: Doc;
  parent: El | null = null;
  children: Array<El | string> = [];
  attrs = new Map<string, string>();
  listeners = new Map<string, Listener[]>();
  style: Record<string, string> = {};
  value = "";
  hidden = false;
  clicks = 0;
  _benchMake?: () => void;
  _benchStep?: (d: number) => void;
  constructor(tag: string, doc: Doc) {
    this.tagName = tag.toUpperCase();
    this.ownerDocument = doc;
  }
  get className() { return this.attrs.get("class") ?? ""; }
  set className(v: string) { this.attrs.set("class", v); }
  get classList() {
    const get = () => this.className.split(/\s+/).filter(Boolean);
    return {
      contains: (c: string) => get().includes(c),
      add: (...cs: string[]) => { for (const c of cs) if (!get().includes(c)) this.className = [...get(), c].join(" "); },
      remove: (...cs: string[]) => { this.className = get().filter((x) => !cs.includes(x)).join(" "); },
      toggle: (c: string, on?: boolean) => {
        const want = on ?? !get().includes(c);
        if (want) this.classList.add(c); else this.classList.remove(c);
        return want;
      },
    };
  }
  get id() { return this.attrs.get("id") ?? ""; }
  set id(v: string) { this.attrs.set("id", v); }
  get textContent(): string { return this.children.map((c) => (typeof c === "string" ? c : c.textContent)).join(""); }
  set textContent(v: string) { for (const c of this.children) if (typeof c !== "string") c.parent = null; this.children = v === "" ? [] : [v]; }
  setAttribute(k: string, v: string) { this.attrs.set(k, String(v)); }
  getAttribute(k: string) { return this.attrs.get(k) ?? null; }
  hasAttribute(k: string) { return this.attrs.has(k); }
  removeAttribute(k: string) { this.attrs.delete(k); }
  append(...nodes: Array<El | string>) {
    for (const n of nodes) {
      if (typeof n !== "string") { if (n.parent) n.remove(); n.parent = this; }
      this.children.push(n);
    }
  }
  prepend(...nodes: Array<El | string>) { const rest = this.children; this.children = []; this.append(...nodes); this.children.push(...rest); }
  replaceChildren(...nodes: Array<El | string>) {
    for (const c of this.children) if (typeof c !== "string") c.parent = null;
    this.children = [];
    this.append(...nodes);
  }
  remove() {
    if (!this.parent) return;
    this.parent.children = this.parent.children.filter((c) => c !== this);
    this.parent = null;
  }
  get isConnected(): boolean { let el: El | null = this; while (el.parent) el = el.parent; return el === this.ownerDocument.documentElement; }
  addEventListener(type: string, fn: Listener) { this.listeners.set(type, [...(this.listeners.get(type) ?? []), fn]); }
  removeEventListener(type: string, fn: Listener) { this.listeners.set(type, (this.listeners.get(type) ?? []).filter((f) => f !== fn)); }
  dispatch(type: string) {
    const e = ev(type, {}, this);
    for (let el: El | null = this; el; el = el.parent) for (const fn of el.listeners.get(type) ?? []) fn(e);
  }
  click() { this.clicks += 1; this.dispatch("click"); }
  focus() { this.ownerDocument.activeElement = this; }
  getBoundingClientRect(): Rect { return this.ownerDocument.rectOf(this); }
  closest(sel: string): El | null { for (let el: El | null = this; el; el = el.parent) if (el.tagName !== "#ROOT" && el.matches(sel)) return el; return null; }
  get elements(): El[] {
    const out: El[] = [];
    for (const c of this.children) if (typeof c !== "string") out.push(c, ...c.elements);
    return out;
  }
  matches(sel: string): boolean {
    return sel.split(",").some((one) => {
      const parts = one.trim().split(/\s+/);
      if (!this.matchOne(parts.pop() as string)) return false;
      let el: El | null = this.parent;
      while (parts.length && el) {
        if (el.tagName !== "#ROOT" && el.matchOne(parts[parts.length - 1] as string)) parts.pop();
        el = el.parent;
      }
      return parts.length === 0;
    });
  }
  matchOne(sel: string): boolean {
    const re = /([a-z]+)|#([\w-]+)|\.([\w-]+)|\[([\w-]+)(?:="([^"]*)")?\]/gy;
    let m: RegExpExecArray | null;
    let pos = 0;
    re.lastIndex = 0;
    while ((m = re.exec(sel))) {
      pos = re.lastIndex;
      if (m[1] && this.tagName !== m[1].toUpperCase()) return false;
      if (m[2] && this.id !== m[2]) return false;
      if (m[3] && !this.classList.contains(m[3])) return false;
      if (m[4] && (!this.attrs.has(m[4]) || (m[5] !== undefined && this.attrs.get(m[4]) !== m[5]))) return false;
    }
    if (pos !== sel.length) throw new Error(`shim cannot parse selector ${sel}`);
    return true;
  }
  querySelectorAll(sel: string): El[] { return this.elements.filter((e) => e.matches(sel)); }
  querySelector(sel: string): El | null { return this.querySelectorAll(sel)[0] ?? null; }
}
class Doc {
  documentElement: El;
  body: El;
  activeElement: El | null = null;
  pointerLockElement: El | null = null;
  rects = new Map<El, Rect>();
  constructor() {
    this.documentElement = new El("#root", this);
    this.body = new El("body", this);
    this.documentElement.append(this.body);
    const game = new El("canvas", this);
    game.id = "game";
    this.body.append(game);
  }
  createElement(tag: string) { return new El(tag, this); }
  getElementById(id: string) { return this.documentElement.elements.find((e) => e.id === id) ?? null; }
  querySelector(sel: string) { return this.documentElement.querySelector(sel); }
  querySelectorAll(sel: string) { return this.documentElement.querySelectorAll(sel); }
  // The panel is 560 x 300 at (400, 420); pieces get their own rects below.
  rectOf(el: El): Rect {
    if (el.id === "worked") return rect(400, 420, 560, 300);
    return this.rects.get(el) ?? rect(0, 0, 0, 0);
  }
  // The deepest element (last in document order) whose rect holds the point;
  // pointer-events: none (the virtual cursor) is skipped, as in browsers.
  elementFromPoint(x: number, y: number): El | null {
    let hit: El | null = null;
    for (const e of this.documentElement.elements) {
      if (e.classList.contains("vcursor")) continue;
      const r = this.rectOf(e);
      if (r.width && x >= r.left && x < r.right && y >= r.top && y < r.bottom) hit = e;
    }
    return hit;
  }
}

function ev(type: string, init: { key?: string; code?: string; shiftKey?: boolean }, target: unknown): Ev {
  return {
    type,
    ...init,
    target,
    stopped: false,
    prevented: false,
    stopImmediatePropagation() { this.stopped = true; },
    preventDefault() { this.prevented = true; },
  };
}

type MountOpts = { band: string; spell: { slot: number; id: string; name: string; formula: string; stage: string; prompt: string }; recipe: unknown; prompt: string; onMake: (t: string) => void };

function setup(opts: { benchOpen?: boolean } = {}) {
  const doc = new Doc();
  const sent: string[] = [];
  const mounts: MountOpts[] = [];
  const reasons: unknown[][] = [];
  const sounds: string[] = [];
  let destroyed = 0;
  let pieces: Record<string, El> = {};
  const fakeBench = {
    mountBuilder(root: El, o: MountOpts) {
      mounts.push(o);
      const atom = doc.createElement("button");
      atom.className = "bench-atom";
      atom.setAttribute("data-el", "H");
      atom.setAttribute("data-token", "H");
      const tile = doc.createElement("button");
      tile.className = "bench-tile";
      tile._benchStep = (d: number) => { tile.setAttribute("data-step", String(d)); };
      const input = doc.createElement("input");
      input.id = "bench-input";
      input.setAttribute("data-token", "text");
      input._benchMake = () => { input.setAttribute("data-made", "1"); };
      const make = doc.createElement("button");
      make.className = "bench-make";
      make.addEventListener("click", () => o.onMake("H2O(l)"));
      root.append(atom, tile, input, make);
      doc.rects.set(atom, rect(420, 600, 64, 64));
      doc.rects.set(make, rect(880, 430, 64, 64));
      pieces = { atom, tile, input, make };
      return { destroy: () => { destroyed += 1; root.replaceChildren(); } };
    },
    reasonFor(band: string, result: unknown, names: unknown, tries: number) {
      reasons.push([band, result, names, tries]);
      return "Count again: H should be 2. You have 1.";
    },
    pretty: (t: string) => `P(${t})`,
  };
  const win = {
    __bench: { ui: () => ({ open: !!opts.benchOpen }) },
    __game: { sound: (n: string) => { sounds.push(n); } },
    setTimeout: (_f: () => void, _ms: number) => 0,
    clearTimeout: (_id: number) => {},
  };
  const w = createWorked({
    doc: doc as unknown as Document,
    win: win as unknown as Window,
    cmd: (c: string) => sent.push(c),
    bench: fakeBench as unknown as Parameters<typeof createWorked>[0]["bench"],
  });
  return { doc, win, w, sent, mounts, reasons, sounds, destroyed: () => destroyed, pieces: () => pieces };
}

const WATER = { slot: 2, id: "water", name: "Water", formula: "H2O(l)", stage: "learning", prompt: "Water: build one water molecule from atom tiles." };
const RECIPE = [{ side: 0, coef: 1, formula: "H2O", atoms: [["H", 2], ["O", 1]], state: "l" }];
const view = (band = "seedling", stage = "learning", result: unknown = undefined) => ({
  v: 1, atBench: false, open: true, band, spells: [{ ...WATER, stage }], pick: 2,
  ...(band === "apprentice" ? {} : { recipe: RECIPE }),
  ...(result ? { result } : {}),
});
type WorkState = Parameters<ReturnType<typeof createWorked>["update"]>[0];
const ws = (work: unknown, extra: Record<string, unknown> = {}) => ({
  v: 1, mode: "worked", ready: true, band: "seedling", routes: ["instant", "worked", "worked", "worked"],
  stages: ["unknown", "learning", "unknown", "unknown"], work, done: 0, wrong: 0, ...extra,
}) as unknown as WorkState;
const work = (id: number, v: unknown, more: Record<string, unknown> = {}) => ({ id, spell: 1, route: "worked", wrong: 0, help: false, view: v, ...more });

// 8. Open and close follow state.
test("8: open and close follow __workState; onMake and cancel send work commands", () => {
  const t = setup();
  const v = view();
  t.w.update(ws(work(3, v)));
  assert.equal(t.mounts.length, 1);
  const m = t.mounts[0] as MountOpts;
  assert.equal(m.band, "seedling");
  assert.deepEqual(m.spell, v.spells[0]);
  assert.deepEqual(m.recipe, RECIPE);
  assert.equal(m.prompt, WATER.prompt);
  assert.ok(t.doc.getElementById("worked"), "#worked is in the page");
  assert.equal(t.w.isOpen(), true);
  t.w.update(ws(work(3, v)));
  assert.equal(t.mounts.length, 1, "the same id mounts nothing new");
  m.onMake("H2O(l)");
  assert.deepEqual(t.sent, ["work submit 3 H2O(l)"]);
  t.w.cancel();
  assert.equal(t.sent[1], "work cancel 3");
  (t.doc.querySelector("#worked .worked-close") as El).click();
  assert.equal(t.sent[2], "work cancel 3");
  t.w.update(ws(null));
  assert.equal(t.destroyed(), 1);
  assert.equal(t.doc.getElementById("worked"), null);
  assert.equal(t.w.isOpen(), false);
  assert.equal(t.doc.activeElement?.id, "game", "the canvas gets focus back");
  const n = t.sent.length;
  t.w.cancel();
  assert.equal(t.sent.length, n, "cancel while closed sends nothing");
  t.w.update(ws(work(4, view())));
  assert.equal(t.mounts.length, 2);
});

test("8: a wrong result shows reasonFor's text with a cross and one sound per n", () => {
  const t = setup();
  t.w.update(ws(work(3, view())));
  const result = { n: 1, slot: 2, ok: false, code: "atoms_off", diffs: [["H", 1, 2]], other: 0, bad: "", simplest: "", coef: 0, learned: false, stage: "learning" };
  t.w.update(ws(work(3, view("seedling", "learning", result), { wrong: 1 })));
  assert.equal(t.reasons.length, 1);
  const [band, r, names, tries] = t.reasons[0] as [string, unknown, Record<number, { name: string; formula: string }>, number];
  assert.equal(band, "seedling");
  assert.deepEqual(r, result);
  assert.equal(names[2]?.name, "Water");
  assert.equal(names[2]?.formula, "H2O(l)");
  assert.equal(tries, 1);
  const reason = t.doc.querySelector("#worked .worked-reason") as El;
  assert.ok(reason.textContent.includes("Count again: H should be 2. You have 1."));
  assert.ok(reason.querySelector(".bench-mark.cross"), "a cross mark");
  assert.deepEqual(t.sounds, ["miss"]);
  t.w.update(ws(work(3, view("seedling", "learning", result), { wrong: 1 })));
  assert.deepEqual(t.sounds, ["miss"], "the same n plays no second sound");
  assert.equal(t.mounts.length, 1, "a result never remounts the builder");
});

test("8: the bench wins: an open bench cancels the builder", () => {
  const t = setup({ benchOpen: true });
  t.w.update(ws(work(3, view())));
  assert.ok(t.sent.includes("work cancel 3"));
});

// 9. Virtual cursor.
test("9: the virtual cursor starts at the panel centre, clamps, ignores 200 px jumps, clicks and cancels", () => {
  const t = setup();
  assert.equal(t.w.move(5, 5), false, "move while closed returns false");
  t.w.update(ws(work(3, view())));
  const vc = t.doc.querySelector("#worked .vcursor") as El;
  assert.ok(vc, "a .vcursor");
  const pos = () => [parseFloat(vc.style.left ?? "NaN"), parseFloat(vc.style.top ?? "NaN")];
  assert.deepEqual(pos(), [400 + 280, 420 + 150]);
  // The mousemove fallback moves it until hud.js forwards a real move, then never.
  t.w.move(10, 0, true);
  assert.deepEqual(pos(), [400 + 290, 420 + 150]);
  t.w.move(-10, 0);
  t.w.move(10, 0, true);
  assert.deepEqual(pos(), [400 + 280, 420 + 150], "the fallback is ignored once pointermove works");
  for (let i = 0; i < 100; i++) t.w.move(100, -100);
  assert.deepEqual(pos(), [960, 420], "clamped to the top-right corner");
  const before = pos();
  t.w.move(200, 0);
  t.w.move(0, -200);
  assert.deepEqual(pos(), before, "a +-200 px move is ignored");
  // Down-left onto Make at (880..944, 430..494).
  t.w.move(-50, 30);
  assert.deepEqual(pos(), [910, 450]);
  t.w.press(0);
  assert.equal(t.pieces().make?.clicks, 1);
  assert.equal(t.sent.at(-1), "work submit 3 H2O(l)");
  t.w.press(2);
  assert.equal(t.sent.at(-1), "work cancel 3");
});

// 10. Keys.
test("10: keys: stop everything but Escape and B, prevent only Tab, replay Enter and arrows", () => {
  const t = setup();
  const closed = ev("keydown", { code: "KeyW", key: "w" }, t.doc.body);
  t.w.key(closed as unknown as KeyboardEvent);
  assert.equal(closed.stopped, false, "nothing is stopped while closed");
  t.w.update(ws(work(3, view("apprentice", "learning"), {}), { band: "apprentice" }));
  for (const [code, key] of [["Digit2", "2"], ["KeyW", "w"], ["KeyR", "r"], ["KeyP", "p"], ["Enter", "Enter"]] as const) {
    const e = ev("keydown", { code, key }, t.doc.body);
    t.w.key(e as unknown as KeyboardEvent);
    assert.equal(e.stopped, true, `${code} stopped`);
    assert.equal(e.prevented, false, `${code} not prevented`);
  }
  const tab = ev("keydown", { code: "Tab", key: "Tab" }, t.doc.body);
  t.w.key(tab as unknown as KeyboardEvent);
  assert.equal(tab.stopped && tab.prevented, true, "Tab is stopped and prevented");
  for (const [code, key] of [["Escape", "Escape"], ["KeyB", "b"]] as const) {
    const e = ev("keydown", { code, key }, t.doc.body);
    t.w.key(e as unknown as KeyboardEvent);
    assert.equal(e.stopped || e.prevented, false, `${code} passes`);
  }
  const up = ev("keyup", { code: "KeyW", key: "w" }, t.doc.body);
  t.w.key(up as unknown as KeyboardEvent);
  assert.equal(up.stopped || up.prevented, false, "keyup passes");
  const { input, tile } = t.pieces();
  const enter = ev("keydown", { code: "Enter", key: "Enter" }, input);
  t.w.key(enter as unknown as KeyboardEvent);
  assert.equal(input?.getAttribute("data-made"), "1", "Enter in the input calls _benchMake");
  const arrow = ev("keydown", { code: "ArrowUp", key: "ArrowUp" }, tile);
  t.w.key(arrow as unknown as KeyboardEvent);
  assert.equal(tile?.getAttribute("data-step"), "1", "ArrowUp on a tile calls _benchStep(1)");
});

// 11. Copy line.
test("11: the Copy line: unknown at Apprentice, after help otherwise, never at Sprout", () => {
  const a = setup();
  a.w.update(ws(work(1, view("apprentice", "unknown")), { band: "apprentice" }));
  const copy = a.doc.querySelector("#worked .worked-copy") as El;
  assert.ok(copy && !copy.hidden, "shown");
  assert.equal(copy.textContent, "Copy: P(H2O(l))");
  const s = setup();
  s.w.update(ws(work(2, view("seedling", "learning"))));
  const none = s.doc.querySelector("#worked .worked-copy");
  assert.ok(!none || none.hidden, "not shown before help");
  s.w.update(ws(work(2, view("seedling", "learning"), { wrong: 2, help: true })));
  const shown = s.doc.querySelector("#worked .worked-copy") as El;
  assert.ok(shown && !shown.hidden, "shown after help");
  assert.equal(s.mounts.length, 1, "help does not remount");
  const k = setup();
  k.w.update(ws(work(5, view("sprout", "unknown")), { band: "sprout" }));
  k.w.update(ws(work(5, view("sprout", "unknown"), { wrong: 2, help: true }), { band: "sprout" }));
  const sprout = k.doc.querySelector("#worked .worked-copy");
  assert.ok(!sprout || sprout.hidden, "never at Sprout");
});

// 12. Badges and toast.
test("12: hotbar stage badges and the Practised toast", () => {
  const t = setup();
  const bar = t.doc.createElement("div");
  bar.id = "hotbar";
  for (let i = 0; i < 4; i++) { const b = t.doc.createElement("button"); b.className = "slot"; bar.append(b); }
  t.doc.body.append(bar);
  const stages = ["unknown", "learning", "practised", "mastered"];
  t.w.update(ws(null, { stages }));
  const read = () => t.doc.querySelectorAll(".slot").map((s) => s.querySelectorAll("span.stage"));
  for (const spans of read()) assert.equal(spans.length, 1);
  assert.deepEqual(read().map((s) => s[0]?.textContent), ["○", "◔", "◑", "★"]);
  for (const spans of read()) assert.ok((spans[0]?.getAttribute("aria-label") ?? "").length > 0);
  assert.ok((read()[3]?.[0]?.getAttribute("aria-label") ?? "").includes("Mastered"));
  t.w.update(ws(null, { stages }));
  for (const spans of read()) assert.equal(spans.length, 1, "no duplicates");
  assert.equal(t.doc.getElementById("worked-toast"), null);
  t.w.update(ws(null, { stages: ["unknown", "practised", "practised", "mastered"] }));
  const toast = t.doc.getElementById("worked-toast");
  assert.ok(toast && !toast.hidden && toast.textContent.includes("Practised"), "toast shows Practised");
  assert.ok(toast.textContent.includes("Water"));
});

// Arbiter r2 (W18): a press on Make, and Enter in the Apprentice box, send
// `work submit` before the call returns, with no timer, microtask or frame.
test("14: Make and Enter send work submit synchronously", () => {
  const g = globalThis as unknown as Record<string, unknown>;
  const names = ["setTimeout", "setInterval", "queueMicrotask", "requestAnimationFrame"];
  const saved = names.map((n) => g[n]);
  const deferred: string[] = [];
  const t = setup();
  t.w.update(ws(work(3, view())));
  t.w.move(115, -60);
  t.w.move(115, -60); // onto Make at (910, 450)
  try {
    for (const n of names) g[n] = () => { deferred.push(n); return 0; };
    t.w.press(0);
    assert.equal(t.sent.at(-1), "work submit 3 H2O(l)", "sent before press returns");
  } finally { names.forEach((n, i) => { g[n] = saved[i]; }); }
  const a = setup();
  a.w.update(ws(work(7, view("apprentice", "learning")), { band: "apprentice" }));
  const input = a.pieces().input as El;
  input._benchMake = () => (a.mounts[0] as MountOpts).onMake("H2O(l)");
  try {
    for (const n of names) g[n] = () => { deferred.push(n); return 0; };
    a.w.key(ev("keydown", { code: "Enter", key: "Enter" }, input) as unknown as KeyboardEvent);
    assert.equal(a.sent.at(-1), "work submit 7 H2O(l)", "sent before key returns");
  } finally { names.forEach((n, i) => { g[n] = saved[i]; }); }
  assert.deepEqual(deferred, [], "nothing was deferred");
});

// Break 1 should-fixes 5 and 6: Sprout sees pictures only.
test("15: Sprout: a bench picture on a New panel, and a picture-only Practised toast", () => {
  const k = setup();
  k.w.update(ws(work(5, view("sprout", "unknown")), { band: "sprout" }));
  const cue = k.doc.querySelector("#worked .worked-bench");
  assert.ok(cue, "the bench picture");
  assert.equal(cue?.getAttribute("role"), "img");
  assert.equal(k.doc.querySelector("#worked .worked-new"), null, "no New line at Sprout");
  k.w.update(ws(null, { band: "sprout", stages: ["unknown", "learning", "unknown", "unknown"] }));
  k.w.update(ws(null, { band: "sprout", stages: ["unknown", "practised", "unknown", "unknown"] }));
  const toast = k.doc.getElementById("worked-toast") as El;
  assert.ok(toast && !toast.hidden, "the toast shows");
  assert.doesNotMatch(toast.textContent, /[A-Za-z]/, "no words at Sprout");
  assert.ok(toast.querySelector(".icon.water"), "the spell's picture");
  assert.match(toast.getAttribute("aria-label") ?? "", /Practised/);
  const s = setup();
  s.w.update(ws(work(6, view("seedling", "unknown"))));
  assert.equal(s.doc.querySelector("#worked .worked-bench"), null, "no picture cue above Sprout");
  assert.ok(s.doc.querySelector("#worked .worked-new"), "the New line above Sprout");
});

// Break 1 should-fix 1: pushing the virtual cursor past an edge scrolls.
test("16: the virtual cursor scrolls a tall panel at its top and bottom edges", () => {
  const t = setup();
  t.w.update(ws(work(3, view())));
  const panel = t.doc.getElementById("worked") as El & { scrollTop?: number };
  panel.scrollTop = 0;
  t.w.move(0, 140);
  assert.equal(panel.scrollTop, 0, "inside the panel: no scroll");
  t.w.move(0, 30);
  assert.equal(panel.scrollTop, 20, "20 px past the bottom edge scrolls 20 px");
  for (let i = 0; i < 3; i++) t.w.move(0, -150);
  assert.ok((panel.scrollTop ?? 0) < 0, "past the top scrolls back (the browser clamps at 0)");
});

// 13. Packaging.
test("13: boot.js imports worked.js, and every imported module is on build-web.sh's cp line", () => {
  const dir = new URL("../crates/bevy-client/web/", import.meta.url);
  const boot = readFileSync(new URL("boot.js", dir), "utf8");
  assert.match(boot, /import\(\s*['"]\.\/worked\.js['"]\s*\)/);
  const sh = readFileSync(new URL("../build-web.sh", dir), "utf8");
  const cp = sh.split("\n").find((l) => /^cp web\/index\.html/.test(l)) ?? "";
  const imports = (src: string) => [...src.matchAll(/(?:from\s+|import\(\s*)['"]\.\/([\w-]+\.js)['"]/g)].map((m) => m[1] as string);
  const seen = new Set<string>();
  const queue = ["boot.js", "hud.js"];
  while (queue.length) {
    const f = queue.shift() as string;
    if (seen.has(f)) continue;
    seen.add(f);
    let src = "";
    try { src = readFileSync(new URL(f, dir), "utf8"); } catch { continue; }
    for (const m of imports(src)) queue.push(m);
  }
  // bevy-client.js is wasm-bindgen output; analytics.js and age.js are added
  // by scripts/build-site.sh only where the analytics switch is on.
  for (const f of seen) {
    if (["bevy-client.js", "analytics.js", "age.js"].includes(f)) continue;
    assert.ok(cp.includes(`web/${f}`), `${f} is on the cp line`);
  }
  assert.ok(seen.has("worked.js"));
});

// I-030 N4: practised -> mastered shows a Mastered toast with a star.
test("I-030 N4: a practised -> mastered change shows the Mastered toast; the Practised toast is unchanged", () => {
  const t = setup();
  t.w.update(ws(null, { stages: ["unknown", "learning", "unknown", "unknown"] }));
  t.w.update(ws(null, { stages: ["unknown", "practised", "unknown", "unknown"] }));
  let toast = t.doc.getElementById("worked-toast") as El;
  assert.equal(toast.textContent, "◑P(Water): Practised!", "the old toast text exactly");
  t.w.update(ws(null, { stages: ["unknown", "mastered", "unknown", "unknown"] }));
  toast = t.doc.getElementById("worked-toast") as El;
  assert.ok(!toast.hidden);
  assert.equal(toast.textContent, "★P(Water): Mastered!");
  assert.ok(toast.querySelector(".worked-toast-shape"));
  // A loaded Mastered save (unknown -> mastered) shows no toast change.
  const u = setup();
  u.w.update(ws(null, { stages: ["unknown", "unknown", "unknown", "unknown"] }));
  u.w.update(ws(null, { stages: ["unknown", "mastered", "unknown", "unknown"] }));
  assert.equal(u.doc.getElementById("worked-toast"), null);
  // Sprout: the icon and the star, the words as the label.
  const k = setup();
  k.w.update(ws(work(5, view("sprout", "practised")), { band: "sprout" }));
  k.w.update(ws(null, { band: "sprout", stages: ["unknown", "practised", "unknown", "unknown"] }));
  k.w.update(ws(null, { band: "sprout", stages: ["unknown", "mastered", "unknown", "unknown"] }));
  const st = k.doc.getElementById("worked-toast") as El;
  assert.doesNotMatch(st.textContent, /[A-Za-z]/);
  assert.ok(st.textContent.includes("★"));
  assert.ok(st.querySelector(".icon.water"));
  assert.match(st.getAttribute("aria-label") ?? "", /Mastered!/);
});
