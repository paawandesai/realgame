// I-028: the Formula Bench's pure helpers (crates/bevy-client/web/bench.js).
// Tests 8-11 and 11b of docs/loop/items/I-028/spec.md.
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  bandFrom,
  composeEquation,
  composeSeedling,
  composeSprout,
  mountBuilder,
  reasonFor,
  scaleOf,
} from "../crates/bevy-client/web/bench.js";

type Term = { side: number; coef: number; formula: string; atoms: Array<[string, number]>; state: string | null };
const WATER: Term[] = [{ side: 0, coef: 1, formula: "H2O", atoms: [["H", 2], ["O", 1]], state: "l" }];
const FROST: Term[] = [{ side: 0, coef: 1, formula: "N2", atoms: [["N", 2]], state: "l" }];
const CO2: Term[] = [{ side: 0, coef: 1, formula: "CO2", atoms: [["C", 1], ["O", 2]], state: "g" }];
const FIRE: Term[] = [
  { side: 0, coef: 1, formula: "CH4", atoms: [["C", 1], ["H", 4]], state: null },
  { side: 0, coef: 2, formula: "O2", atoms: [["O", 2]], state: null },
  { side: 1, coef: 1, formula: "CO2", atoms: [["C", 1], ["O", 2]], state: null },
  { side: 1, coef: 2, formula: "H2O", atoms: [["H", 2], ["O", 1]], state: null },
];
// As built from the published `spells` list.
const NAMES = {
  1: { name: "Fire", formula: "CH4 + 2O2 -> CO2 + 2H2O" },
  2: { name: "Water", formula: "H2O(l)" },
  3: { name: "Frost", formula: "N2(l)" },
  4: { name: "CO₂", formula: "CO2(g)" },
};
type R = { slot: number; code: string; diffs?: Array<[string, number, number]>; other?: number; bad?: string };
const res = (r: R) => ({ n: 1, ok: r.code === "ok", diffs: [], other: 0, bad: "", simplest: "", coef: 0, learned: false, stage: "unknown", ...r });

// 8.
test("8: composeSprout puts recipe elements first, then others alphabetically", () => {
  assert.equal(composeSprout(["H", "O", "H"], WATER), "H2O");
  assert.equal(composeSprout(["N", "N"], FROST), "N2");
  assert.equal(composeSprout(["O", "C", "O"], CO2), "CO2");
  assert.equal(composeSprout(["H", "O", "H", "C"], WATER), "H2OC");
  assert.equal(composeSprout(["N", "H", "O", "C"], WATER), "HOCN");
  assert.equal(composeSprout(["H", "H"], WATER), "H2");
  assert.equal(composeSprout([], WATER), "");
});

// 9.
test("9: composeSeedling, composeEquation and scaleOf", () => {
  assert.equal(composeSeedling(WATER, { H: 2, O: 1 }), "H2O");
  assert.equal(composeSeedling(WATER, { H: 2, O: 1, C: 0, N: 0 }), "H2O");
  assert.equal(composeSeedling(WATER, { H: 0, O: 0, C: 0, N: 0 }), "");
  assert.equal(composeSeedling(CO2, { H: 0, O: 2, C: 1, N: 0 }), "CO2");
  assert.equal(composeEquation(FIRE, [1, 2, 1, 2]), "CH4 + 2O2 -> CO2 + 2H2O");
  assert.equal(composeEquation(FIRE, [1, 1, 1, 1]), "CH4 + O2 -> CO2 + H2O");
  assert.equal(composeEquation(FIRE, [2, 4, 2, 4]), "2CH4 + 4O2 -> 2CO2 + 4H2O");
  assert.deepEqual(scaleOf(FIRE, [1, 2, 1, 2]), { left: { C: 1, H: 4, O: 4 }, right: { C: 1, H: 4, O: 4 }, level: true, tilt: 0 });
  const s = scaleOf(FIRE, [1, 1, 1, 1]);
  assert.deepEqual(s.left, { C: 1, H: 4, O: 2 });
  assert.deepEqual(s.right, { C: 1, H: 2, O: 3 });
  assert.equal(s.level, false);
  assert.equal(s.tilt, -1); // 6 atoms after the arrow, 7 before
  assert.equal(scaleOf(FIRE, [1, 4, 1, 1]).tilt, -1);
  assert.equal(scaleOf(FIRE, [1, 1, 3, 1]).tilt, 1);
});

// 10.
const TABLE: Record<string, [string, string, string]> = {
  ok: ["You made Water!", "H₂O is right! You are learning Water.", "H₂O(l) is right! You are learning Water."],
  empty: ["Tap the balls to build it.", "Tap the tiles to build it.", "Write a formula first."],
  atoms_off: [
    "Look again. Count the balls in the picture.",
    "Count again: H should be 2. You have 1.",
    "Not yet. Which atoms does Water have, and how many?",
  ],
  other_spell: ["That makes Frost. This one is Water.", "That makes Frost. This one is Water.", "That makes Frost. This one is Water."],
  state_missing: ["Right atoms. Now add its state: (s), (l) or (g).", "Right atoms. Now add its state: (s), (l) or (g).", "Right atoms. Now add its state: (s), (l) or (g)."],
  wrong_state: ["Right atoms. This spell needs it as a liquid.", "Right atoms. This spell needs it as a liquid.", "Right atoms. This spell needs it as a liquid."],
  not_single: [
    "This spell is one substance. Write just its formula.",
    "This spell is one substance. Write just its formula.",
    "This spell is one substance. Write just its formula.",
  ],
  no_products: ["Add an arrow and what it makes.", "Add an arrow and what it makes.", "Add an arrow and what it makes."],
  wrong_species: [
    "Those are not the substances in this reaction. Check what burns and what it makes.",
    "Those are not the substances in this reaction. Check what burns and what it makes.",
    "Those are not the substances in this reaction. Check what burns and what it makes.",
  ],
  element: ['"Xx" is not an element symbol.', '"Xx" is not an element symbol.', '"Xx" is not an element symbol.'],
  parse: [
    "Use element symbols, numbers, + and an arrow.",
    "Use element symbols, numbers, + and an arrow.",
    "Use element symbols, numbers, + and an arrow.",
  ],
  not_for_band: ["Pick a spell from the list.", "Pick a spell from the list.", "Pick a spell from the list."],
};
const BANDS = ["sprout", "seedling", "apprentice"] as const;

test("10: reasonFor returns the Contract table's string for every code and band", () => {
  const detail: Record<string, Partial<R>> = {
    atoms_off: { diffs: [["H", 1, 2]] },
    other_spell: { other: 3 },
    wrong_state: { bad: "l" },
    state_missing: { bad: "H2O" },
    element: { bad: "Xx" },
  };
  for (const [code, want] of Object.entries(TABLE)) {
    BANDS.forEach((band, i) => {
      const got = reasonFor(band, res({ slot: 2, code, ...detail[code] }), NAMES, 1);
      assert.equal(got, want[i], `${code} at ${band}`);
    });
  }
  // unbalanced (Fire): Seedling names the rule; Apprentice one line per diff; Sprout (n/a) = Apprentice.
  const unb = res({ slot: 1, code: "unbalanced", diffs: [["H", 4, 2], ["O", 2, 3]] });
  const seed = "The scale is not level. Atoms are never made or lost, so both sides need the same atoms.";
  const app = "Not balanced yet: H: 4 before, 2 after.\nNot balanced yet: O: 2 before, 3 after.";
  assert.equal(reasonFor("seedling", unb, NAMES, 1), seed);
  assert.equal(reasonFor("apprentice", unb, NAMES, 1), app);
  assert.equal(reasonFor("sprout", unb, NAMES, 1), app);
  assert.equal(reasonFor("adept", unb, NAMES, 1), app);
  // Every code is covered: 13.
  assert.equal(Object.keys(TABLE).length + 1, 13);
  // Seedling ok has no state symbol; Fire at Apprentice has subscripts and an arrow.
  assert.equal(reasonFor("seedling", res({ slot: 3, code: "ok" }), NAMES, 0), "N₂ is right! You are learning Frost.");
  assert.equal(reasonFor("apprentice", res({ slot: 1, code: "ok" }), NAMES, 0), "CH₄ + 2O₂ → CO₂ + 2H₂O is right! You are learning Fire.");
  // Other states.
  assert.equal(reasonFor("apprentice", res({ slot: 4, code: "wrong_state", bad: "g" }), NAMES, 1), "Right atoms. This spell needs it as a gas.");
  assert.equal(reasonFor("apprentice", res({ slot: 4, code: "wrong_state", bad: "s" }), NAMES, 1), "Right atoms. This spell needs it as a solid.");
  // Several diffs at Seedling: one line each.
  assert.equal(
    reasonFor("seedling", res({ slot: 2, code: "atoms_off", diffs: [["C", 1, 0], ["H", 1, 2]] }), NAMES, 1),
    "Count again: C should be 0. You have 1.\nCount again: H should be 2. You have 1.",
  );
});

test("10: Apprentice atoms_off names counts only from the third wrong try", () => {
  const r = res({ slot: 2, code: "atoms_off", diffs: [["H", 1, 2]] });
  for (const tries of [1, 2]) assert.ok(!/\d/.test(reasonFor("apprentice", r, NAMES, tries)), `try ${tries}`);
  for (const tries of [3, 4]) assert.equal(reasonFor("apprentice", r, NAMES, tries), "Count again: H should be 2. You have 1.");
  assert.equal(reasonFor("adept", r, NAMES, 3), "Count again: H should be 2. You have 1.");
});

test("10: fizzle reasons are kind, and Sprout reasons are short", () => {
  const unkind = /\b(wrong|bad|fail|error|incorrect)/i;
  const detail: Record<string, Partial<R>> = {
    atoms_off: { diffs: [["H", 1, 2]] },
    unbalanced: { slot: 1, diffs: [["O", 2, 3]] },
    other_spell: { other: 3 },
    wrong_state: { bad: "l" },
    element: { bad: "Xx" },
  };
  for (const code of [...Object.keys(TABLE), "unbalanced"]) {
    for (const band of BANDS) {
      for (const tries of [1, 3]) {
        const line = reasonFor(band, res({ slot: 2, code, ...detail[code] }), NAMES, tries);
        assert.ok(line.length > 0, `${code} at ${band}`);
        if (code === "ok") continue;
        assert.ok(!unkind.test(line), `${code} at ${band}: ${line}`);
        assert.ok(!line.includes("!"), `${code} at ${band}: ${line}`);
        // The Sprout column's own strings ((n/a) codes return the Apprentice string).
        if (band === "sprout" && ["empty", "atoms_off", "other_spell", "not_for_band"].includes(code)) {
          assert.ok(line.split(/\s+/).length <= 12, `${code} at sprout has more than 12 words: ${line}`);
        }
      }
    }
  }
  const sproutOk = reasonFor("sprout", res({ slot: 2, code: "ok" }), NAMES, 0);
  assert.ok(sproutOk.split(/\s+/).length <= 12);
});

// 11.
test("11: bandFrom: URL, then the age picker, then the bench chooser", () => {
  assert.equal(bandFrom("?band=sprout", null, null), "sprout");
  assert.equal(bandFrom("?low&band=seedling&debug", null, null), "seedling");
  assert.equal(bandFrom("?band=adept", null, null), "apprentice");
  assert.equal(bandFrom("?band=toddler", null, null), null);
  assert.equal(bandFrom("", null, null), null);
  assert.equal(bandFrom("", "seedling", null), "seedling");
  assert.equal(bandFrom("?band=apprentice", "seedling", "sprout"), "apprentice");
  assert.equal(bandFrom("", "seedling", "sprout"), "seedling");
  assert.equal(bandFrom("", null, "sprout"), "sprout");
  assert.equal(bandFrom("?band=toddler", "nonsense", "apprentice"), "apprentice");
});

// ---------------------------------------------------------------------------
// 11b. mountBuilder, with a minimal DOM shim (no new dependency).
// ---------------------------------------------------------------------------

type Listener = (e: { type: string; key?: string; target: El; preventDefault(): void }) => void;
class El {
  tagName: string;
  ownerDocument: Doc;
  parent: El | null = null;
  children: Array<El | string> = [];
  attrs = new Map<string, string>();
  listeners = new Map<string, Listener[]>();
  style = { props: new Map<string, string>(), setProperty(k: string, v: string) { this.props.set(k, v); } };
  value = "";
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
  addEventListener(type: string, fn: Listener) {
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), fn]);
  }
  dispatch(type: string, extra: { key?: string } = {}) {
    const e = { type, target: this, preventDefault() {}, ...extra };
    for (let el: El | null = this; el; el = el.parent) for (const fn of el.listeners.get(type) ?? []) fn(e);
  }
  click() { this.dispatch("click"); }
  get elements(): El[] {
    const out: El[] = [];
    for (const c of this.children) if (typeof c !== "string") out.push(c, ...c.elements);
    return out;
  }
  matches(sel: string): boolean {
    // Compound selectors only: tag, #id, .class, [attr] and [attr="v"].
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
  // A selector list of compound selectors, each optionally after ancestors ("a b").
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
  focusCalls = 0;
  createElement(tag: string) { return new El(tag, this); }
}

const WATER_CARD = { slot: 2, id: "water", name: "Water", formula: "H2O(l)", stage: "unknown", prompt: "Make water! Copy the balls in the picture." };
const FIRE_CARD = { slot: 1, id: "fire", name: "Fire", formula: "CH4 + 2O2 -> CO2 + 2H2O", stage: "unknown", prompt: "" };

test("11b: mountBuilder renders the build view on its own and calls onMake with the composed text", () => {
  const doc = new Doc();
  const div = doc.createElement("div");
  const made: string[] = [];
  const handle = mountBuilder(div as unknown as HTMLElement, {
    band: "sprout",
    spell: WATER_CARD,
    recipe: WATER,
    prompt: WATER_CARD.prompt,
    onMake: (t: string) => made.push(t),
  });
  assert.equal(typeof handle.destroy, "function");
  for (const sel of [".bench-spell", ".bench-band", ".bench-close"]) assert.equal(div.querySelector(sel), null, sel);
  assert.equal(doc.focusCalls, 0);
  const pieces = div.querySelectorAll("[data-token]");
  assert.deepEqual(pieces.map((p) => p.getAttribute("data-token")), ["H", "O", "C", "N"]);
  // The recipe shows two H balls and one O ball.
  assert.equal(div.querySelectorAll('.bench-recipe .bench-ball[data-el="H"]').length, 2);
  assert.equal(div.querySelectorAll('.bench-recipe .bench-ball[data-el="O"]').length, 1);
  const tok = (t: string) => div.querySelector(`[data-token="${t}"]`) as El;
  tok("H").click();
  tok("H").click();
  tok("O").click();
  assert.equal(div.querySelectorAll(".bench-tray .bench-ball").length, 3);
  (div.querySelector(".bench-make") as El).click();
  assert.deepEqual(made, ["H2O"]);
  // Undo removes the last atom.
  (div.querySelector(".bench-undo") as El).click();
  (div.querySelector(".bench-make") as El).click();
  assert.deepEqual(made, ["H2O", "H2"]);
  handle.destroy();
  assert.equal(div.children.length, 0);
});

test("11b: Seedling tiles and Fire coefficient tiles carry data-token and compose", () => {
  const doc = new Doc();
  const div = doc.createElement("div");
  const made: string[] = [];
  mountBuilder(div as unknown as HTMLElement, { band: "seedling", spell: WATER_CARD, recipe: WATER, prompt: "p", onMake: (t: string) => made.push(t) });
  const tile = (el: string) => div.querySelector(`.bench-tile[data-el="${el}"]`) as El;
  tile("H").click();
  tile("H").click();
  tile("O").click();
  assert.equal(tile("H").getAttribute("data-match"), "true");
  assert.equal(tile("C").getAttribute("data-match"), "false");
  (div.querySelector(".bench-make") as El).click();
  // The 0 -> 4 -> 0 cycle.
  for (let k = 0; k < 5; k++) tile("N").click();
  assert.equal(tile("N").getAttribute("data-token"), "N");
  (div.querySelector(".bench-make") as El).click();
  assert.deepEqual(made, ["H2O", "H2O"]);

  const fdiv = doc.createElement("div");
  const fmade: string[] = [];
  mountBuilder(fdiv as unknown as HTMLElement, { band: "seedling", spell: FIRE_CARD, recipe: FIRE, prompt: "p", onMake: (t: string) => fmade.push(t) });
  const coefs = fdiv.querySelectorAll(".bench-coef");
  assert.deepEqual(coefs.map((c) => c.getAttribute("data-token")), ["coef:0", "coef:1", "coef:2", "coef:3"]);
  const scale = fdiv.querySelector(".bench-scale") as El;
  assert.equal(scale.getAttribute("data-level"), "false");
  const pans = fdiv.querySelectorAll(".bench-pan-count").map((p) => p.textContent);
  assert.deepEqual(pans, ["C 1, H 4, O 2", "C 1, H 2, O 3"]);
  coefs[1]?.click();
  coefs[3]?.click();
  assert.equal(scale.getAttribute("data-level"), "true");
  assert.deepEqual(fdiv.querySelectorAll(".bench-pan-count").map((p) => p.textContent), ["C 1, H 4, O 4", "C 1, H 4, O 4"]);
  (fdiv.querySelector(".bench-make") as El).click();
  assert.deepEqual(fmade, ["CH4 + 2O2 -> CO2 + 2H2O"]);
});

test("11b: mountBuilder renders without a recipe (Seedling) and at Apprentice", () => {
  const doc = new Doc();
  const a = doc.createElement("div");
  mountBuilder(a as unknown as HTMLElement, { band: "seedling", spell: WATER_CARD, recipe: null, prompt: "p", onMake: () => {} });
  assert.equal(a.querySelector(".bench-recipe"), null);
  assert.equal(a.querySelectorAll(".bench-tile").length, 4);
  const b = doc.createElement("div");
  const made: string[] = [];
  mountBuilder(b as unknown as HTMLElement, { band: "apprentice", spell: WATER_CARD, recipe: null, prompt: "p", onMake: (t: string) => made.push(t) });
  const input = b.querySelector("#bench-input") as El;
  assert.ok(input);
  assert.equal(input.getAttribute("maxlength"), "60");
  assert.equal(b.querySelector(".bench-recipe"), null);
  input.value = "  N2(l) ";
  input.dispatch("input");
  assert.equal((b.querySelector(".bench-preview") as El).textContent, "  N₂(l) ");
  input.dispatch("keydown", { key: "Enter" });
  assert.deepEqual(made, ["N2(l)"]);
});

// ---------------------------------------------------------------------------
// Round 2 (break-1 B1 and S1).

test("round 2 (B1): the scale's beam is never drawn level unless every element matches", () => {
  // 1221 is two taps from the start: equal totals (9 and 9), different elements.
  const s = scaleOf(FIRE, [1, 2, 2, 1]);
  assert.deepEqual(s.left, { C: 1, H: 4, O: 4 });
  assert.deepEqual(s.right, { C: 2, H: 2, O: 5 });
  assert.equal(s.level, false);
  assert.notEqual(s.tilt, 0);
  // Every coefficient set 1-4: level <=> tilt 0; unequal totals keep sign(right - left).
  let flat = 0;
  for (let a = 1; a <= 4; a++) for (let b = 1; b <= 4; b++) for (let c = 1; c <= 4; c++) for (let d = 1; d <= 4; d++) {
    const q = scaleOf(FIRE, [a, b, c, d]);
    const sum = (p: Record<string, number>) => Object.values(p).reduce((x, y) => x + y, 0);
    const diff = sum(q.right) - sum(q.left);
    assert.equal(q.tilt === 0, q.level, `${a}${b}${c}${d}`);
    if (diff !== 0) assert.equal(q.tilt, Math.sign(diff), `${a}${b}${c}${d}`);
    else if (!q.level) flat++;
  }
  assert.equal(flat, 9); // 1221, 2113, 2122, 2131, 2433, 2442, 3334, 3343, 4244
});

test("round 2 (B1): at 1-2-2-1 the drawn beam tilts, no pan says 'more atoms', and the mismatched elements are marked", () => {
  const doc = new Doc();
  const div = doc.createElement("div");
  mountBuilder(div as unknown as HTMLElement, { band: "seedling", spell: FIRE_CARD, recipe: FIRE, prompt: "p", onMake: () => {} });
  const coefs = div.querySelectorAll(".bench-coef");
  coefs[1]?.click();
  coefs[2]?.click();
  const scale = div.querySelector(".bench-scale") as El;
  assert.equal(scale.getAttribute("data-level"), "false");
  assert.notEqual(scale.getAttribute("data-tilt"), "0");
  const beam = div.querySelector(".bench-beam") as El;
  assert.notEqual(beam.style.props.get("--tilt"), "0");
  assert.deepEqual(div.querySelectorAll(".bench-pan-count").map((p) => p.textContent), ["C 1, H 4, O 4", "C 2, H 2, O 5"]);
  assert.deepEqual(div.querySelectorAll(".bench-pan-more").map((p) => p.textContent), ["", ""]);
  const marks = div.querySelectorAll(".bench-pan-el").map((e) => `${e.getAttribute("data-el")}:${e.getAttribute("data-ok")}`);
  assert.deepEqual(marks, ["C:false", "H:false", "O:false", "C:false", "H:false", "O:false"]);
  // Balanced: every element is marked ok and the beam is level.
  coefs[2]?.click(); // 1 2 3 1
  coefs[2]?.click(); // 1 2 4 1
  coefs[2]?.click(); // 1 2 1 1
  coefs[3]?.click(); // 1 2 1 2
  assert.equal(scale.getAttribute("data-level"), "true");
  assert.equal(scale.getAttribute("data-tilt"), "0");
  assert.ok(div.querySelectorAll(".bench-pan-el").every((e) => e.getAttribute("data-ok") === "true"));
});

test("round 2 (S1): reasonFor never returns an empty line when diffs are missing", () => {
  const r = { slot: 3, code: "atoms_off", diffs: [] as Array<[string, number, number]> };
  for (const band of ["seedling", "apprentice", "adept"]) {
    for (const tries of [1, 3, 5]) {
      const line = reasonFor(band, r, NAMES, tries);
      assert.equal(line, "Not yet. Which atoms does Frost have, and how many?", `${band} ${tries}`);
    }
  }
  const u = { slot: 1, code: "unbalanced", diffs: [] as Array<[string, number, number]> };
  for (const band of ["seedling", "apprentice"]) assert.ok(reasonFor(band, u, NAMES, 1).trim().length > 0, band);
});

// ---------------------------------------------------------------------------
// I-030 N1: mountBuilder's `unaided` option (the Mastery trial builds from memory).

test("I-030 N1: unaided Sprout has no recipe card, and HHO + Make still sends H2O", () => {
  const doc = new Doc();
  const div = doc.createElement("div");
  const made: string[] = [];
  mountBuilder(div as unknown as HTMLElement, { band: "sprout", spell: WATER_CARD, recipe: WATER, prompt: "p", unaided: true, onMake: (t: string) => made.push(t) });
  assert.equal(div.querySelector(".bench-recipe"), null);
  assert.equal(div.querySelectorAll(".bench-ball").length > 0, true, "the atom cards still show their balls");
  assert.equal(div.querySelectorAll(".bench-tray .bench-ball").length, 0);
  const tok = (t: string) => div.querySelector(`[data-token="${t}"]`) as El;
  tok("H").click();
  tok("H").click();
  tok("O").click();
  (div.querySelector(".bench-make") as El).click();
  assert.deepEqual(made, ["H2O"]);
});

test("I-030 N1: unaided Seedling water has no formula card and no data-match ticks", () => {
  const doc = new Doc();
  const div = doc.createElement("div");
  const made: string[] = [];
  mountBuilder(div as unknown as HTMLElement, { band: "seedling", spell: WATER_CARD, recipe: WATER, prompt: "p", unaided: true, onMake: (t: string) => made.push(t) });
  assert.equal(div.querySelector(".bench-formula"), null);
  assert.equal(div.querySelector(".bench-recipe"), null);
  assert.equal(div.querySelectorAll(".bench-tile").length, 4);
  assert.equal(div.querySelectorAll("[data-match]").length, 0);
  const tile = (el: string) => div.querySelector(`.bench-tile[data-el="${el}"]`) as El;
  tile("O").click();
  tile("H").click();
  tile("H").click();
  assert.equal(div.querySelectorAll("[data-match]").length, 0, "still no ticks after taps");
  (div.querySelector(".bench-make") as El).click();
  assert.deepEqual(made, ["H2O"]);
});

test("I-030 N1: unaided Seedling Fire has coefficient tiles but no scale, and Make sends the equation", () => {
  const doc = new Doc();
  const div = doc.createElement("div");
  const made: string[] = [];
  mountBuilder(div as unknown as HTMLElement, { band: "seedling", spell: FIRE_CARD, recipe: FIRE, prompt: "p", unaided: true, onMake: (t: string) => made.push(t) });
  const coefs = div.querySelectorAll(".bench-coef");
  assert.equal(coefs.length, 4);
  assert.equal(div.querySelector(".bench-scale"), null);
  assert.equal(div.querySelector(".bench-pan"), null);
  assert.equal(div.querySelectorAll(".bench-op").length, 3, "the + and arrow row stays");
  coefs[1]?.click();
  coefs[3]?.click();
  (div.querySelector(".bench-make") as El).click();
  assert.deepEqual(made, ["CH4 + 2O2 -> CO2 + 2H2O"]);
});

test("I-030 N1: without unaided (or with unaided: false) the DOM is exactly as before", () => {
  const shape = (el: El): unknown => ({
    tag: el.tagName,
    attrs: [...el.attrs.entries()].sort(),
    kids: el.children.map((c) => (typeof c === "string" ? c : shape(c))),
  });
  const cases: Array<[string, typeof WATER_CARD, Term[] | null]> = [
    ["sprout", WATER_CARD, WATER],
    ["seedling", WATER_CARD, WATER],
    ["seedling", FIRE_CARD, FIRE],
    ["seedling", WATER_CARD, null],
    ["apprentice", WATER_CARD, null],
  ];
  for (const [band, spell, recipe] of cases) {
    const doc = new Doc();
    const a = doc.createElement("div");
    const b = doc.createElement("div");
    mountBuilder(a as unknown as HTMLElement, { band, spell, recipe, prompt: "p", onMake: () => {} });
    mountBuilder(b as unknown as HTMLElement, { band, spell, recipe, prompt: "p", unaided: false, onMake: () => {} });
    assert.deepEqual(shape(b), shape(a), `${band} ${spell.id}`);
  }
  // The 11b expectations still hold with the option absent.
  const doc = new Doc();
  const s = doc.createElement("div");
  mountBuilder(s as unknown as HTMLElement, { band: "sprout", spell: WATER_CARD, recipe: WATER, prompt: "p", onMake: () => {} });
  assert.equal(s.querySelectorAll('.bench-recipe .bench-ball[data-el="H"]').length, 2);
  const f = doc.createElement("div");
  mountBuilder(f as unknown as HTMLElement, { band: "seedling", spell: FIRE_CARD, recipe: FIRE, prompt: "p", onMake: () => {} });
  assert.ok(f.querySelector(".bench-scale"));
  const w = doc.createElement("div");
  mountBuilder(w as unknown as HTMLElement, { band: "seedling", spell: WATER_CARD, recipe: WATER, prompt: "p", onMake: () => {} });
  assert.ok(w.querySelector(".bench-formula"));
  assert.equal(w.querySelectorAll("[data-match]").length, 4);
});

test("I-030: bench.js exports speak and rise for the trial", async () => {
  const m = await import("../crates/bevy-client/web/bench.js");
  assert.equal(typeof (m as Record<string, unknown>).speak, "function");
  assert.equal(typeof (m as Record<string, unknown>).rise, "function");
});
