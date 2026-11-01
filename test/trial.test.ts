// I-030: the Mastery trial view (crates/bevy-client/web/trial.js).
// Node test N2 (and N5's packaging half) of docs/loop/items/I-030/spec.md,
// wave 14 revision. A minimal DOM shim, as in test/bench.test.ts 11b, with
// innerHTML for the inline SVG pictures.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { lineFor, PICTURES, pipStates, renderTrial } from "../crates/bevy-client/web/trial.js";

type Ev = { type: string; key?: string; code?: string; target: El; preventDefault(): void; stopPropagation(): void };
type Listener = (e: Ev) => void;
class El {
  tagName: string;
  ownerDocument: Doc;
  parent: El | null = null;
  children: Array<El | string> = [];
  attrs = new Map<string, string>();
  listeners = new Map<string, Listener[]>();
  style = { props: new Map<string, string>(), setProperty(k: string, v: string) { this.props.set(k, v); } };
  value = "";
  hidden = false;
  disabled = false;
  tabIndex = 0;
  raw = "";
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
      add: (c: string) => { if (!get().includes(c)) this.className = [...get(), c].join(" "); },
      remove: (c: string) => { this.className = get().filter((x) => x !== c).join(" "); },
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
  set textContent(v: string) { this.children = v === "" ? [] : [v]; }
  // Only what trial.js writes: one <svg ...>...</svg> picture.
  set innerHTML(v: string) {
    this.raw = v;
    this.children = [];
    if (/^\s*<svg[\s>]/.test(v)) {
      const svg = new El("svg", this.ownerDocument);
      svg.raw = v;
      this.append(svg);
    } else if (v) this.children = [v];
  }
  get innerHTML() { return this.raw; }
  setAttribute(k: string, v: string) { this.attrs.set(k, String(v)); }
  getAttribute(k: string) { return this.attrs.get(k) ?? null; }
  hasAttribute(k: string) { return this.attrs.has(k); }
  removeAttribute(k: string) { this.attrs.delete(k); }
  append(...nodes: Array<El | string>) {
    for (const n of nodes) {
      if (typeof n !== "string") n.parent = this;
      this.children.push(n);
    }
  }
  replaceChildren(...nodes: Array<El | string>) {
    this.children = [];
    this.append(...nodes);
  }
  remove() {
    if (!this.parent) return;
    this.parent.children = this.parent.children.filter((c) => c !== this);
    this.parent = null;
  }
  addEventListener(type: string, fn: Listener) {
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), fn]);
  }
  dispatch(type: string, extra: { key?: string; code?: string } = {}) {
    const e: Ev = { type, target: this, preventDefault() {}, stopPropagation() {}, ...extra };
    for (let el: El | null = this; el; el = el.parent) for (const fn of el.listeners.get(type) ?? []) fn(e);
  }
  click() { if (!this.disabled) this.dispatch("click"); }
  focus() { this.ownerDocument.activeElement = this; }
  closest(sel: string): El | null { for (let el: El | null = this; el; el = el.parent) if (el.matches(sel)) return el; return null; }
  get elements(): El[] {
    const out: El[] = [];
    for (const c of this.children) if (typeof c !== "string") out.push(c, ...c.elements);
    return out;
  }
  matches(sel: string): boolean {
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
  matchesChain(sel: string): boolean {
    const parts = sel.trim().split(/\s+/);
    if (!this.matches(parts.pop() as string)) return false;
    let el: El | null = this.parent;
    while (parts.length && el) {
      if (el.matches(parts[parts.length - 1] as string)) parts.pop();
      el = el.parent;
    }
    return parts.length === 0;
  }
  querySelectorAll(sel: string): El[] { return this.elements.filter((e) => sel.split(",").some((s) => e.matchesChain(s))); }
  querySelector(sel: string): El | null { return this.querySelectorAll(sel)[0] ?? null; }
}
class Doc {
  activeElement: El | null = null;
  createElement(tag: string) { return new El(tag, this); }
}

const WATER_CARD = { slot: 2, id: "water", name: "Water", formula: "H2O(l)", stage: "practised", prompt: "Make water! Copy the balls in the picture." };
const FIRE_CARD = { slot: 1, id: "fire", name: "Fire", formula: "CH4 + 2O2 -> CO2 + 2H2O", stage: "practised", prompt: "" };
const WATER_RECIPE = [{ side: 0, coef: 1, formula: "H2O", atoms: [["H", 2], ["O", 1]], state: "l" }];
const FIRE_RECIPE = [
  { side: 0, coef: 1, formula: "CH4", atoms: [["C", 1], ["H", 4]], state: null },
  { side: 0, coef: 2, formula: "O2", atoms: [["O", 2]], state: null },
  { side: 1, coef: 1, formula: "CO2", atoms: [["C", 1], ["O", 2]], state: null },
  { side: 1, coef: 2, formula: "H2O", atoms: [["H", 2], ["O", 1]], state: null },
];
const benchView = (band: string, card = WATER_CARD, recipe: unknown = WATER_RECIPE, result: unknown = undefined) => ({
  v: 1, atBench: false, open: true, band, spells: [card], pick: card.slot,
  ...(recipe ? { recipe } : {}),
  ...(result ? { result } : {}),
});
type Trial = Record<string, unknown>;
const trialOf = (over: Trial = {}) => ({
  spell: 1, slot: 2, kind: "master", phase: "build", built: 0, need: 3, misses: 0, asks: 0, right: null, mastered: false, q: null, ...over,
});
const SPROUT_Q = {
  id: "water_sprout",
  prompt: "Lots of water goes on a campfire. What happens?",
  choices: [
    { id: "a", label: "The fire goes out", icon: "fire_out" },
    { id: "b", label: "The fire gets bigger", icon: "fire_big" },
    { id: "c", label: "The fire turns to ice", icon: "ice" },
  ],
  answer: null as string | null,
  why: null as string | null,
};
const ts = (trial: unknown, sharp: number[] = []) => ({ v: 1, sharp, trial });

function setup(band = "sprout") {
  const doc = new Doc();
  const body = doc.createElement("div");
  const sent: string[] = [];
  let closed = 0;
  const ctx = { push: (c: string) => void sent.push(c), band, close: () => { closed += 1; } };
  const render = (state: unknown, view: unknown) =>
    renderTrial(body as unknown as HTMLElement, state as never, view as never, ctx as never);
  return { doc, body, sent, ctx, render, closed: () => closed };
}

// N2.
test("N2: PICTURES has an SVG with at least 2 shapes for every ICONS id in trial.rs", () => {
  const rs = readFileSync(new URL("../crates/bevy-client/src/trial.rs", import.meta.url), "utf8");
  const m = /pub const ICONS: \[&str; \d+\] = \[([\s\S]*?)\];/.exec(rs);
  assert.ok(m, "ICONS in trial.rs");
  const ids = [...(m[1] ?? "").matchAll(/"([a-z_]+)"/g)].map((x) => x[1] as string);
  assert.ok(ids.length >= 7, `ids ${ids}`);
  for (const id of ids) {
    const svg = (PICTURES as Record<string, string>)[id];
    assert.equal(typeof svg, "string", id);
    assert.match(svg as string, /^<svg[\s>]/, id);
    assert.match(svg as string, /<\/svg>$/, id);
    const shapes = (svg as string).match(/<(circle|ellipse|rect|path|polygon|polyline|line)\b/g) ?? [];
    assert.ok(shapes.length >= 2, `${id} has ${shapes.length} shapes`);
    assert.doesNotMatch(svg as string, /<script|on\w+=|href/i, `${id} is a plain picture`);
  }
  for (const id of Object.keys(PICTURES)) assert.ok(ids.includes(id), `${id} is in ICONS`);
});

test("N2: lineFor gives each Contract line exactly", () => {
  assert.equal(lineFor("sprout", 0, "master", "Water"), "Make Water from memory!");
  assert.equal(lineFor("seedling", 0, "master", "Water"), "Build Water from memory. No recipe this time.");
  assert.equal(lineFor("seedling", 0, "master", "Fire", true), "Balance Fire without the scale.");
  assert.equal(lineFor("seedling", 0, "master", "Fire"), "Balance Fire without the scale.");
  assert.equal(lineFor("apprentice", 0, "master", "CO₂"), "Write CO₂ from memory.");
  assert.equal(lineFor("adept", 0, "master", "CO₂"), "Write CO₂ from memory.");
  assert.equal(lineFor("sprout", 1, "master", "Water"), "Again!");
  assert.equal(lineFor("apprentice", 2, "master", "Water"), "Last one!");
  assert.equal(lineFor("sprout", 0, "sharp", "Water"), "Make Water from memory!");
});

test("N2: pipStates", () => {
  assert.deepEqual(pipStates(0), [false, false, false]);
  assert.deepEqual(pipStates(2), [true, true, false]);
  assert.deepEqual(pipStates(3), [true, true, true]);
});

test("N2: a build state mounts an unaided builder with three pips; Make sends try", () => {
  const t = setup("sprout");
  t.render(ts(trialOf({ built: 1 })), benchView("sprout"));
  assert.ok(t.body.querySelector(".trial"), "the trial view");
  assert.equal(t.body.querySelector(".trial-title")?.textContent, "Master Water");
  const pips = t.body.querySelectorAll(".trial-pip");
  assert.deepEqual(pips.map((p) => p.getAttribute("data-on")), ["true", "false", "false"]);
  assert.equal(t.body.querySelector(".trial-line")?.textContent, "Again!");
  assert.ok(t.body.querySelector(".bench-build"), "a builder");
  assert.equal(t.body.querySelector(".bench-recipe"), null, "no recipe card");
  const tok = (x: string) => t.body.querySelector(`[data-token="${x}"]`) as El;
  tok("H").click();
  tok("O").click();
  tok("H").click();
  (t.body.querySelector(".bench-make") as El).click();
  assert.deepEqual(t.sent, ["try 2 H2O"]);
  assert.ok(t.body.querySelector(".trial-back"), "All spells");
  (t.body.querySelector(".trial-back") as El).click();
  assert.equal(t.sent.at(-1), "endtrial");
});

test("N2: Seedling Fire: coefficient tiles, no scale; a wrong build shows the reason, pips stay", () => {
  const t = setup("seedling");
  t.render(ts(trialOf({ spell: 0, slot: 1, built: 2 })), benchView("seedling", FIRE_CARD, FIRE_RECIPE));
  assert.equal(t.body.querySelectorAll(".bench-coef").length, 4);
  assert.equal(t.body.querySelector(".bench-scale"), null);
  assert.equal(t.body.querySelector(".trial-line")?.textContent, "Last one!");
  const miss = { n: 7, slot: 1, ok: false, code: "unbalanced", diffs: [["H", 4, 2]], learned: false, stage: "practised" };
  t.render(ts(trialOf({ spell: 0, slot: 1, built: 2, misses: 1 })), benchView("seedling", FIRE_CARD, FIRE_RECIPE, miss));
  const reason = t.body.querySelector(".trial-feedback")?.textContent ?? "";
  assert.match(reason, /scale is not level/);
  assert.deepEqual(t.body.querySelectorAll(".trial-pip").map((p) => p.getAttribute("data-on")), ["true", "true", "false"]);
});

test("N2: an ask state at Sprout renders 3 picture choices; a click sends answer; digit keys send nothing", () => {
  const t = setup("sprout");
  t.render(ts(trialOf({ phase: "ask", built: 3, q: SPROUT_Q })), benchView("sprout"));
  const choices = t.body.querySelectorAll(".trial-choice");
  assert.equal(choices.length, 3);
  for (const c of choices) assert.ok(c.querySelector("svg"), "a picture");
  assert.deepEqual(choices.map((c) => c.getAttribute("data-choice")), ["a", "b", "c"]);
  assert.equal(t.body.querySelector(".trial-lead")?.textContent, "What happens?");
  assert.equal(t.body.querySelector("[data-correct]"), null, "nothing marked before answering");
  for (const k of ["1", "2", "3"]) t.body.dispatch("keydown", { key: k, code: `Digit${k}` });
  for (const c of choices) c.dispatch("keydown", { key: "2", code: "Digit2" });
  assert.deepEqual(t.sent, []);
  (choices[1] as El).click();
  assert.deepEqual(t.sent, ["answer b"]);
  (choices[2] as El).click();
  assert.deepEqual(t.sent, ["answer b"], "one answer per ask");
});

test("N2: Seedling asks with 3 text buttons", () => {
  const t = setup("seedling");
  const q = {
    id: "water_seedling", prompt: "Water is poured on a pan of hot coals. What will you see?",
    choices: [{ id: "a", label: "Steam rises and the coals cool down", icon: "" }, { id: "b", label: "The coals get hotter", icon: "" }, { id: "c", label: "The water turns to ice", icon: "" }],
    answer: null, why: null,
  };
  t.render(ts(trialOf({ phase: "ask", built: 3, q })), benchView("seedling"));
  const choices = t.body.querySelectorAll(".trial-choice");
  assert.equal(choices.length, 3);
  for (const c of choices) assert.equal(c.querySelector("svg"), null);
  assert.equal(t.body.querySelector(".trial-prompt")?.textContent, q.prompt);
});

test("N2: an answered-wrong state marks the right card and offers Ask me again", () => {
  const t = setup("sprout");
  const q = { ...SPROUT_Q, answer: "a", why: "Water soaks up the fire's heat. The wood cools down and stops burning." };
  t.render(ts(trialOf({ phase: "answered", built: 3, right: false, q })), benchView("sprout"));
  const marked = t.body.querySelectorAll('[data-correct="true"]');
  assert.equal(marked.length, 1);
  assert.equal(marked[0]?.getAttribute("data-choice"), "a");
  assert.match(t.body.querySelector(".trial-wrong")?.textContent ?? "", /Not quite\. It is this one:/);
  assert.equal(t.body.querySelector(".trial-why")?.textContent, q.why);
  const again = t.body.querySelector(".trial-again") as El;
  assert.equal(again.textContent, "Ask me again");
  again.click();
  assert.deepEqual(t.sent, ["again"]);
});

test("N2: a done state with mastered shows the star and the mastered line", () => {
  const t = setup("seedling");
  const q = { ...SPROUT_Q, answer: "a", why: "Because." };
  t.render(ts(trialOf({ phase: "done", built: 3, right: true, mastered: true, q })), benchView("seedling"));
  const done = t.body.querySelector('.trial-done[data-mastered="true"]');
  assert.ok(done, "the done view");
  assert.ok(t.body.querySelector(".trial-star"));
  assert.equal(t.body.querySelector(".trial-mastered")?.textContent, "Water mastered! One click casts it now.");
  (t.body.querySelector(".trial-leave") as El).click();
  assert.equal(t.closed(), 1);
  (t.body.querySelector(".trial-back") as El).click();
  assert.deepEqual(t.sent, ["endtrial"]);
});

test("N2: keep it sharp: title, no pips, the miss line and Still sharp", () => {
  const t = setup("sprout");
  t.render(ts(trialOf({ kind: "sharp" })), benchView("sprout"));
  assert.equal(t.body.querySelector(".trial-title")?.textContent, "Keep Water sharp");
  assert.equal(t.body.querySelectorAll(".trial-pip").length, 0);
  const miss = { n: 3, slot: 2, ok: false, code: "atoms_off", diffs: [["H", 1, 2]], learned: false, stage: "mastered" };
  t.render(ts(trialOf({ kind: "sharp", misses: 1 })), benchView("sprout", WATER_CARD, WATER_RECIPE, miss));
  assert.match(t.body.querySelector(".trial-feedback")?.textContent ?? "", /Not this time\. Water stays mastered\./);
  assert.equal(t.body.querySelector(".trial-retry")?.textContent, "Try again");
  t.render(ts(trialOf({ kind: "sharp", phase: "done", misses: 1 })), benchView("sprout", WATER_CARD, WATER_RECIPE, miss));
  assert.match(t.body.querySelector(".trial-done")?.textContent ?? "", /Still sharp! \+10 XP\./);
});

// N5 (packaging half; the science half is test/science-sources.test.ts).
test("N5: build-web.sh copies web/trial.js and bench.js imports it", () => {
  const dir = new URL("../crates/bevy-client/web/", import.meta.url);
  const sh = readFileSync(new URL("../build-web.sh", dir), "utf8");
  const cp = sh.split("\n").find((l) => /^cp web\/index\.html/.test(l)) ?? "";
  assert.ok(cp.includes("web/trial.js"), cp);
  const bench = readFileSync(new URL("bench.js", dir), "utf8");
  assert.match(bench, /from\s+['"]\.\/trial\.js['"]/);
});
