// Chemistry Bench teaching content (content/chem/guide.json): every reaction
// id, bottle, condition, family and source it names is real, every band only
// uses what that band can see, and every walkthrough and lesson "try" really
// leads to its reaction when replayed against the engine's matching rules
// (crates/chem/src/lib.rs: Codex::run, Codex::judge, Need::met_by).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { hill, parseFormula, sideOf } from "../crates/bevy-client/web/bench-free-lib.js";

type Reaction = { id: string; family: string; band: string; equation: string; needs: string[]; evidence?: { kind: string } };
type Step = { id: string; say: string; do: Record<string, unknown> };
type Lesson = {
  id: string;
  title: string;
  idea: string;
  try: { reactants: string[]; conditions: string[]; expect: string } | null;
  ngss: string;
  reactions: string[];
  sources: string[];
  /** I-057: the chemistry quest that uses this lesson in the block world. */
  world?: { quest: string; say: string };
  /** I-059: an Adept lesson's AP Chemistry topic (College Board CED). */
  ap?: string;
};
type Goal = { kind: string; family?: string; ids?: string[]; count?: number; reaction?: string };
type Band = {
  walkthrough: Step[];
  walkthrough_sources: string[];
  lessons: Lesson[];
  hints: Record<string, string[]>;
  hint_sources: string[];
  challenges: { id: string; title: string; goal: Goal }[];
};
type Guide = {
  references: Record<string, string>;
  bands: Record<string, Band>;
  where_you_see_it: Record<string, string>;
  where_you_see_it_sources: Record<string, string[]>;
};

const coreFile = JSON.parse(readFileSync("content/chem/core.json", "utf8")) as { families: string[]; reactions: Reaction[] };
/** I-058: the engine plays only reactions with textbook or data evidence (Codex::load hides the rest), so the guide may name only those. */
const playable = (r: Reaction) => r.evidence?.kind === "textbook" || r.evidence?.kind === "data";
const core = { ...coreFile, reactions: coreFile.reactions.filter(playable) };
const names = JSON.parse(readFileSync("content/chem/names.json", "utf8")) as {
  elements: Record<string, unknown>;
  substances: Record<string, string>;
  conditions: [string, string][];
  conditions_young: Record<string, string>;
};
const guide = JSON.parse(readFileSync("content/chem/guide.json", "utf8")) as Guide;
const quests = JSON.parse(readFileSync("content/chem/quests.json", "utf8")) as { quests: { id: string; live: boolean; solved_by?: string[] }[] };
/** The rule tier (rules.json): quests may be solved by a rule reaction too. */
const rules = JSON.parse(readFileSync("content/chem/rules.json", "utf8")) as { reactions: Reaction[] };

const BANDS = ["sprout", "seedling", "apprentice", "adept"];
const rank = (b: string) => BANDS.indexOf(b);
const byId = new Map(core.reactions.map((r) => [r.id, r]));
const CONDS = new Set(names.conditions.map((c) => c[0]));
const isEl = (e: string) => e in names.elements;
/** The bench caps each side's count of one formula at 40 (bench-free.js putOn). */
const SIDE_CAP = 40;
/** Conditions the page hides at Sprout (bench-free.js SPROUT_HIDES). */
const SPROUT_HIDES = new Set(["electricity", "pressure"]);
/** The atom plate holds 16 atoms (bench-free.js addAtom). */
const PLATE_CAP = 16;

// ---------- the engine's matching rules, replayed ----------

type Side = Record<string, number>;
const gcd = (a: number, b: number): number => (b === 0 ? a : gcd(b, a % b));
const key = (s: Side) => JSON.stringify(Object.entries(s).sort(([a], [b]) => (a < b ? -1 : 1)));
/** Both sides divided by one shared divisor, as `reduced` in crates/chem/src/lib.rs. */
function reduced(l: Side, r: Side): [Side, Side] {
  const g = Math.max(1, [...Object.values(l), ...Object.values(r)].reduce(gcd, 0));
  const div = (s: Side) => Object.fromEntries(Object.entries(s).map(([f, n]) => [f, n / g]));
  return [div(l), div(r)];
}
const halves = (eq: string) => eq.split(" -> ").map((s) => sideOf(s)) as [Side, Side];
const KEYS = core.reactions.map((r) => reduced(...halves(r.equation)).map(key) as [string, string]);
/** A stored key's reactant side reduced on its own (Codex::run compares reactant ratios). */
const RUN_KEYS = core.reactions.map((r) => key(reduced(halves(r.equation)[0], {})[0]));
const met = (need: string, have: Set<string>) => have.has(need) || (need === "spark" && have.has("heat"));

type Outcome = { kind: "known" | "needs_condition" | "locked" | "none" | "unbalanced"; id?: string };
function known(i: number, have: Set<string>, band: string): Outcome {
  const r = core.reactions[i]!;
  if (rank(r.band) > rank(band)) return { kind: "locked", id: r.id };
  return r.needs.every((n) => met(n, have)) ? { kind: "known", id: r.id } : { kind: "needs_condition", id: r.id };
}
/** Codex::run: only reactants in the chamber. */
function run(left: Side, have: Set<string>, band: string): Outcome {
  const [l] = reduced(left, {});
  const matches = RUN_KEYS.map((k, i) => [k, i] as const)
    .filter(([k]) => k === key(l))
    .map(([, i]) => i);
  const score = (i: number) => {
    const r = core.reactions[i]!;
    return (r.needs.every((n) => met(n, have)) ? 0 : 2) + (rank(r.band) > rank(band) ? 1 : 0);
  };
  matches.sort((a, b) => score(a) - score(b));
  return matches.length ? known(matches[0]!, have, band) : { kind: "none" };
}
function atoms(s: Side): Side {
  const t: Side = {};
  for (const [f, n] of Object.entries(s)) {
    const c = parseFormula(f, isEl);
    assert.ok(c, `${f} is not a formula`);
    for (const [e, k] of Object.entries(c)) t[e] = (t[e] ?? 0) + k * n;
  }
  return t;
}
/** Codex::judge: a whole equation, typed. */
function judge(eq: string, have: Set<string>, band: string): Outcome {
  const [l, r] = halves(eq.replace(/\s*->\s*/, " -> "));
  if (key(atoms(l)) !== key(atoms(r))) return { kind: "unbalanced" };
  const [kl, kr] = reduced(l, r).map(key);
  const i = KEYS.findIndex(([a, b]) => a === kl && b === kr);
  return i < 0 ? { kind: "none" } : known(i, have, band);
}

// Formula as the codex writes it, by Hill formula (bench-free.js `written`).
const written: Record<string, string> = {};
for (const r of core.reactions) {
  for (const s of r.equation.split(" -> ")) {
    for (const f of Object.keys(sideOf(s))) {
      const c = parseFormula(f, isEl);
      if (c) written[hill(c)] ??= f;
    }
  }
}

const visible = (band: string) => core.reactions.filter((r) => rank(r.band) <= rank(band));
/** The bottle shelf at a band: every reactant of a reaction the band can see (bench-free.js drawShelves). */
const shelf = (band: string) => new Set(visible(band).flatMap((r) => Object.keys(halves(r.equation)[0])));
const tapsOf = (list: string[]) => list.reduce<Side>((s, f) => ((s[f] = (s[f] ?? 0) + 1), s), {});
/** Whether a player at `band` can discover `r`: by bottle taps with every need on, or (Apprentice+) by typing the equation. */
function reachable(band: string, r: Reaction): boolean {
  if (band === "sprout" && r.needs.some((n) => SPROUT_HIDES.has(n))) return false;
  const have = new Set(r.needs);
  const [l] = halves(r.equation);
  const bottles = Object.values(l).every((n) => n <= SIDE_CAP) && run(l, have, band).id === r.id && run(l, have, band).kind === "known";
  const typed = rank(band) >= rank("apprentice") && judge(r.equation, have, band).kind === "known";
  return bottles || typed;
}

/**
 * I-059: Sprout is grades 6 to 7, so its text may show formulas, but only the
 * simple formulas of very familiar substances (MS-PS1-1 level).
 */
const SPROUT_FORMULAS = new Set(["H2", "O2", "H2O", "CO2", "NaCl"]);
/** Formula words in `s` (a symbol with digits, or a formula or element word of two letters or more like NaCl or Fe), coefficients dropped. */
function formulasIn(s: string): string[] {
  const out: string[] = [];
  for (const w of s.match(/[A-Za-z0-9()]+/g) ?? []) {
    // A bracket around a word ("(NaCl)") is prose, not part of the formula.
    let f = /^\(.*\)$/.test(w) ? w.slice(1, -1) : w;
    while ((f.match(/\(/g) ?? []).length > (f.match(/\)/g) ?? []).length && f.startsWith("(")) f = f.slice(1);
    while ((f.match(/\)/g) ?? []).length > (f.match(/\(/g) ?? []).length && f.endsWith(")")) f = f.slice(0, -1);
    f = f.replace(/^\d+/, "");
    if (/^[A-Z][a-z]?\d/.test(f) || /[a-z)][A-Z]/.test(f) || (f.length >= 2 && (f in names.substances || f in names.elements))) out.push(f);
  }
  return out;
}

/**
 * NGSS performance expectations per grade band (I-059): Sprout (grades 6-7)
 * at MS-PS1-1, 2 and 5 level, plus the middle-school energy and life-science
 * ones; Seedling (8-9) middle school to early high school; Apprentice
 * (10-11) high school; Adept (grade 12+) the deepest high-school ones, with
 * an AP Chemistry topic as well.
 */
const NGSS: Record<string, RegExp> = {
  sprout: /^MS-(PS1-[1256]|LS1-[67])$/,
  seedling: /^(MS-(PS1-[1-6]|LS1-[67])|HS-PS1-[27])$/,
  apprentice: /^HS-(PS|LS)\d-\d$/,
  adept: /^HS-PS1-\d$/,
};
/** AP Chemistry Course and Exam Description topics an Adept lesson may cite (R9). */
const AP_TOPICS: Record<string, string> = {
  "1.7": "Periodic Trends",
  "4.2": "Net Ionic Equations",
  "4.5": "Stoichiometry",
  "4.7": "Types of Chemical Reactions",
  "4.9": "Oxidation-Reduction (Redox) Reactions",
  "5.11": "Catalysis",
  "6.7": "Bond Enthalpies",
  "6.8": "Enthalpy of Formation",
  "7.9": "Introduction to Le Chatelier's Principle",
};
/** Performance expectations the guide may cite (NGSS, 2013). */
const NGSS_CODES = new Set([
  "K-LS1-1",
  "2-PS1-1", "2-PS1-2", "2-PS1-3", "2-PS1-4",
  "5-PS1-1", "5-PS1-2", "5-PS1-3", "5-PS1-4", "5-PS3-1", "5-LS1-1",
  "MS-PS1-1", "MS-PS1-2", "MS-PS1-3", "MS-PS1-4", "MS-PS1-5", "MS-PS1-6", "MS-LS1-6", "MS-LS1-7",
  "HS-PS1-1", "HS-PS1-2", "HS-PS1-3", "HS-PS1-4", "HS-PS1-5", "HS-PS1-6", "HS-PS1-7", "HS-LS1-5", "HS-LS1-7",
]);

const checkSources = (where: string, refs: string[]) => {
  assert.ok(Array.isArray(refs) && refs.length > 0, `${where}: no sources`);
  for (const s of refs) assert.ok(s in guide.references, `${where}: unknown source ${s}`);
};
const reaction = (where: string, id: string, band: string) => {
  const r = byId.get(id);
  assert.ok(r, `${where}: no core reaction ${id}`);
  assert.ok(rank(r.band) <= rank(band), `${where}: ${id} is taught at ${r.band}, above ${band}`);
  return r;
};

// ---------- tests ----------

test("the guide has the four bands and every id is unique", () => {
  assert.deepEqual(Object.keys(guide.bands), BANDS);
  const ids = new Set<string>();
  for (const b of Object.values(guide.bands)) {
    for (const x of [...b.walkthrough, ...b.lessons, ...b.challenges]) {
      assert.match(x.id, /^[a-z][a-z0-9_]*$/, x.id);
      assert.ok(!ids.has(x.id), `duplicate id ${x.id}`);
      ids.add(x.id);
    }
  }
  for (const [k, v] of Object.entries(guide.references)) assert.ok(v.trim().length > 10, `reference ${k} is empty`);
});

for (const band of BANDS) {
  const g = guide.bands[band]!;
  const bottles = shelf(band);

  test(`${band}: the walkthrough uses real bottles and conditions and ends in a discovery`, () => {
    assert.ok(g.walkthrough.length >= 4 && g.walkthrough.length <= 6, `${g.walkthrough.length} steps`);
    checkSources(`${band} walkthrough`, g.walkthrough_sources);
    const left: Side = {};
    const have = new Set<string>();
    let outcome: Outcome | null = null;
    g.walkthrough.forEach((step, i) => {
      const where = `${band} walkthrough ${step.id}`;
      assert.ok(step.say.trim(), `${where}: no say`);
      assert.equal(outcome, null, `${where}: only the last step may run the chamber`);
      const keys = Object.keys(step.do);
      assert.equal(keys.length, 1, `${where}: one action`);
      const [k, v] = Object.entries(step.do)[0]!;
      if (k === "tap_bottle") {
        assert.ok(typeof v === "string" && bottles.has(v), `${where}: ${String(v)} is not on the ${band} shelf`);
        left[v] = (left[v] ?? 0) + 1;
      } else if (k === "condition") {
        assert.ok(typeof v === "string" && CONDS.has(v), `${where}: no condition ${String(v)}`);
        assert.ok(!(band === "sprout" && SPROUT_HIDES.has(v)), `${where}: ${v} is hidden at Sprout`);
        if (have.has(v)) have.delete(v);
        else have.add(v);
      } else if (k === "run") {
        assert.equal(v, true, where);
        outcome = run(left, have, band);
      } else if (k === "type") {
        assert.ok(rank(band) >= rank("apprentice"), `${where}: the type box is hidden below Apprentice`);
        assert.ok(typeof v === "string" && v.trim(), where);
        if (/->/.test(v)) outcome = judge(v, have, band);
        else {
          const c = parseFormula(v, isEl);
          assert.ok(c, `${where}: ${v} does not parse`);
          const f = written[hill(c)] ?? v;
          left[f] = (left[f] ?? 0) + 1;
        }
      } else if (k === "open_tab") {
        assert.ok(["codex", "notebook", "guide"].includes(String(v)), `${where}: no tab ${String(v)}`);
      } else assert.fail(`${where}: unknown action ${k}`);
      for (const n of Object.values(left)) assert.ok(n <= SIDE_CAP, `${where}: more than ${SIDE_CAP} of one bottle`);
      if (i === g.walkthrough.length - 1) assert.ok(outcome, `${where}: the last step must run the chamber`);
    });
    const o = outcome as Outcome | null;
    assert.ok(o?.kind === "known" && o.id, `${band} walkthrough ends in ${JSON.stringify(o)}, not a discovery`);
    const r = reaction(`${band} walkthrough`, o.id, band);
    for (const n of r.needs) assert.ok(have.has(n), `${band} walkthrough: ${r.id} needs ${n}, which no step turns on`);
  });

  test(`${band}: every lesson cites real reactions and its try finds the expected one`, () => {
    assert.ok(g.lessons.length >= 6 && g.lessons.length <= 10, `${g.lessons.length} lessons`);
    for (const l of g.lessons) {
      const where = `${band} lesson ${l.id}`;
      assert.ok(l.title.trim() && l.idea.trim(), `${where}: title and idea`);
      const sentences = l.idea.split(/(?<=[.!?])\s+(?=[A-Z(])/).length;
      assert.ok(sentences >= 1 && sentences <= 3, `${where}: ${sentences} sentences`);
      assert.match(l.ngss, NGSS[band]!, `${where}: ${l.ngss} is not a ${band} grade-band code`);
      assert.ok(NGSS_CODES.has(l.ngss), `${where}: unknown NGSS code ${l.ngss}`);
      if (band === "adept") assert.ok(l.ap && l.ap in AP_TOPICS, `${where}: an Adept lesson names its AP Chemistry topic, not ${l.ap}`);
      else assert.equal(l.ap, undefined, `${where}: only Adept cites AP topics`);
      if (l.ap) assert.ok(l.sources.includes("R9"), `${where}: cites the AP course description (R9)`);
      checkSources(where, l.sources);
      assert.ok(l.reactions.length > 0, `${where}: no reactions`);
      for (const id of l.reactions) reaction(where, id, band);
      if (l.try === null) continue;
      const t = l.try;
      const r = reaction(where, t.expect, band);
      assert.ok(l.reactions.includes(t.expect), `${where}: expect ${t.expect} missing from reactions`);
      for (const c of t.conditions) assert.ok(CONDS.has(c), `${where}: no condition ${c}`);
      for (const c of t.conditions) assert.ok(!(band === "sprout" && SPROUT_HIDES.has(c)), `${where}: ${c} is hidden at Sprout`);
      for (const n of r.needs) assert.ok(t.conditions.includes(n), `${where}: ${t.expect} needs ${n}`);
      for (const f of t.reactants) assert.ok(bottles.has(f), `${where}: ${f} is not on the ${band} shelf`);
      const left = tapsOf(t.reactants);
      for (const n of Object.values(left)) assert.ok(n <= SIDE_CAP, `${where}: more than ${SIDE_CAP} taps of one bottle`);
      assert.deepEqual(run(left, new Set(t.conditions), band), { kind: "known", id: t.expect }, where);
    }
  });

  test(`${band}: hints cover every family the band can see`, () => {
    const fams = new Set(visible(band).map((r) => r.family));
    assert.deepEqual(Object.keys(g.hints).sort(), [...fams].sort());
    for (const [fam, lines] of Object.entries(g.hints)) {
      assert.ok(core.families.includes(fam), `${band} hints: no family ${fam}`);
      assert.equal(lines.length, 3, `${band} hints ${fam}: [nudge, idea, answer]`);
      for (const s of lines) assert.ok(s.trim(), `${band} hints ${fam}: empty line`);
    }
    checkSources(`${band} hints`, g.hint_sources);
  });

  test(`${band}: challenges name real families and reactions a player there can reach`, () => {
    assert.ok(g.challenges.length >= 5 && g.challenges.length <= 8, `${g.challenges.length} challenges`);
    const reach = visible(band).filter((r) => reachable(band, r));
    for (const c of g.challenges) {
      const where = `${band} challenge ${c.id}`;
      const goal = c.goal;
      assert.ok(c.title.trim(), where);
      if (goal.kind === "find_family") {
        assert.ok(goal.family && core.families.includes(goal.family), `${where}: no family ${goal.family}`);
        assert.ok(reach.some((r) => r.family === goal.family), `${where}: nothing in ${goal.family} is reachable at ${band}`);
      } else if (goal.kind === "find_any_of") {
        assert.ok(goal.ids?.length, `${where}: no ids`);
        for (const id of goal.ids) reaction(where, id, band);
        assert.ok(goal.ids.some((id) => reach.some((r) => r.id === id)), `${where}: none of these is reachable at ${band}`);
      } else if (goal.kind === "find_all_of") {
        assert.ok(goal.ids && goal.ids.length > 1, `${where}: needs two or more ids`);
        assert.equal(new Set(goal.ids).size, goal.ids.length, `${where}: duplicate ids`);
        for (const id of goal.ids) {
          reaction(where, id, band);
          assert.ok(reach.some((r) => r.id === id), `${where}: ${id} is not reachable at ${band}`);
        }
      } else if (goal.kind === "find_count") {
        assert.ok(Number.isInteger(goal.count) && goal.count! > 0, `${where}: count`);
        if (goal.family !== undefined) assert.ok(core.families.includes(goal.family), `${where}: no family ${goal.family}`);
        // Optional `ids`: only these reactions count.
        if (goal.ids !== undefined) for (const id of goal.ids) reaction(where, id, band);
        const pool = reach.filter((r) => (goal.family === undefined || r.family === goal.family) && (goal.ids === undefined || goal.ids.includes(r.id)));
        assert.ok(goal.count! <= pool.length, `${where}: only ${pool.length} reachable`);
      } else if (goal.kind === "balance") {
        assert.ok(rank(band) >= rank("seedling"), `${where}: the balance scale is hidden at Sprout`);
        const r = reaction(where, goal.reaction ?? "", band);
        assert.equal(judge(r.equation, new Set(r.needs), band).kind, "known", where);
        if (rank(band) < rank("apprentice")) {
          // No type box: reactants come from bottles, products from the atom plate.
          const [l, p] = halves(r.equation);
          for (const [f, n] of Object.entries(l)) assert.ok(bottles.has(f) && n <= SIDE_CAP, `${where}: ${n} ${f} from the shelf`);
          for (const [f, n] of Object.entries(p)) {
            const size = Object.values(parseFormula(f, isEl) ?? {}).reduce((a, b) => a + b, 0);
            assert.ok(size <= PLATE_CAP && n <= SIDE_CAP, `${where}: ${n} ${f} can't be built on the plate`);
          }
        }
      } else assert.fail(`${where}: unknown goal ${goal.kind}`);
    }
  });
}

test("sprout text is grade 6-7 wording: only simple, familiar formulas and short sentences (I-059)", () => {
  const g = guide.bands.sprout!;
  const texts = [
    ...g.walkthrough.map((s) => s.say),
    ...g.lessons.flatMap((l) => [l.title, l.idea, l.world?.say ?? ""]),
    ...Object.values(g.hints).flat(),
    ...g.challenges.map((c) => c.title),
    ...core.reactions.filter((r) => r.band === "sprout").map((r) => guide.where_you_see_it[r.id] ?? ""),
  ];
  // Formulas are shown at Sprout now, but the text names only very familiar ones.
  for (const t of texts) for (const f of formulasIn(t)) assert.ok(SPROUT_FORMULAS.has(f), `Sprout text names ${f}, not a familiar formula: ${t}`);
  // The lessons are no longer picture-only: they use formulas and model words.
  const ideas = g.lessons.map((l) => l.idea).join(" ");
  assert.ok(formulasIn(ideas).length >= 5, "Sprout lessons show simple formulas");
  assert.match(ideas, /\batoms?\b/);
  assert.match(ideas, /\bmolecules?\b/);
  assert.match(ideas, /\bevidence\b/);
  // Sprout has no "out" side, no type box and no balance scale (it never balances by hand).
  for (const t of texts) assert.doesNotMatch(t, /["“]out["”]|\bout side\b|\btype\b|\bscale\b/i, `Sprout has no such control: ${t}`);
  // Short, plain sentences.
  for (const t of texts) {
    assert.ok(t.length <= 200, `too long for Sprout: ${t}`);
    for (const s of t.split(/(?<=[.!?])\s+/)) assert.ok(s.length <= 120, `sentence too long for Sprout: ${s}`);
  }
});

test("sprout is the simplest band: few bottles, core reactions only, no balancing by hand (I-059)", () => {
  const all = [...core.reactions, ...rules.reactions.filter(playable)];
  const sproutAll = all.filter((r) => r.band === "sprout");
  assert.ok(sproutAll.length >= 10 && sproutAll.length <= 16, `${sproutAll.length} Sprout reactions`);
  assert.ok(sproutAll.every((r) => !r.id.startsWith("rule_")), "no rule reaction at Sprout");
  // The page's shelf: every reactant of a reaction the band sees (core and rule tier).
  const shelfAll = (band: string) => new Set(all.filter((r) => rank(r.band) <= rank(band)).flatMap((r) => Object.keys(halves(r.equation)[0])));
  const n = shelfAll("sprout").size;
  assert.ok(n <= 16, `${n} bottles at Sprout`);
  assert.ok(n < shelfAll("seedling").size, "Sprout has fewer bottles than Seedling");
  // Every bottle's formula parses, so the page can show it (no picture-only shelf).
  for (const f of shelfAll("sprout")) assert.ok(parseFormula(f, isEl), `${f} is not a formula`);
  // Each band adds reactions to the one below it.
  for (let i = 1; i < BANDS.length; i++) {
    const at = all.filter((r) => r.band === BANDS[i]).length;
    assert.ok(at > 0, `${BANDS[i]} adds no reactions`);
  }
  assert.ok(!guide.bands.sprout!.challenges.some((c) => c.goal.kind === "balance"), "Sprout never balances by hand");
});

test("every band's walkthrough reaches a discovery that is playable in that band (I-059)", () => {
  for (const band of BANDS) {
    const g = guide.bands[band]!;
    const left: Side = {};
    const have = new Set<string>();
    let outcome: Outcome | null = null;
    for (const step of g.walkthrough) {
      const [k, v] = Object.entries(step.do)[0]!;
      if (k === "tap_bottle") left[String(v)] = (left[String(v)] ?? 0) + 1;
      else if (k === "condition") have.add(String(v));
      else if (k === "run") outcome = run(left, have, band);
      else if (k === "type" && /->/.test(String(v))) outcome = judge(String(v), have, band);
      else if (k === "type") {
        const c = parseFormula(String(v), isEl);
        const f = c ? (written[hill(c)] ?? String(v)) : String(v);
        left[f] = (left[f] ?? 0) + 1;
      }
    }
    const o = outcome as Outcome | null;
    assert.equal(o?.kind, "known", `${band}: the walkthrough ends in ${JSON.stringify(o)}`);
    const r = byId.get(o!.id!);
    assert.ok(r && playable(r), `${band}: ${o!.id} is not playable`);
    assert.ok(rank(r.band) <= rank(band), `${band}: ${r.id} is taught at ${r.band}`);
    assert.ok(reachable(band, r), `${band}: ${r.id} cannot be reached at ${band}`);
  }
});

test("every live quest has a solving reaction playable in every band (I-059)", () => {
  const all = new Map([...core.reactions, ...rules.reactions.filter(playable)].map((r) => [r.id, r]));
  for (const q of quests.quests.filter((x) => x.live)) {
    for (const band of BANDS) {
      const ok = (q.solved_by ?? []).some((id) => {
        const r = all.get(id);
        return !!r && rank(r.band) <= rank(band);
      });
      assert.ok(ok, `${q.id}: no solving reaction is playable at ${band}`);
    }
    // And one is a Sprout reaction, so every band can do every quest.
    assert.ok((q.solved_by ?? []).some((id) => all.get(id)?.band === "sprout"), `${q.id}: no Sprout solver`);
  }
});

test("lessons point at the live chemistry quest that uses them (I-057)", () => {
  const live = new Map(quests.quests.filter((q) => q.live).map((q) => [q.id, q]));
  const linked = new Set<string>();
  for (const band of BANDS) {
    for (const l of guide.bands[band]!.lessons) {
      if (!l.world) continue;
      const where = `${band} lesson ${l.id}`;
      const q = live.get(l.world.quest);
      assert.ok(q, `${where}: no live quest ${l.world.quest}`);
      assert.match(l.world.say, /^Try it in the world: [a-z].*[.!]$/, `${where}: ${l.world.say}`);
      assert.ok(l.world.say.length <= 160, `${where}: too long`);
      linked.add(l.world.quest);
    }
  }
  // Every live quest is linked from at least one lesson, and Sprout links all of them.
  assert.deepEqual([...linked].sort(), [...live.keys()].sort());
  const sprout = new Set(guide.bands.sprout!.lessons.flatMap((l) => (l.world ? [l.world.quest] : [])));
  assert.deepEqual([...sprout].sort(), [...live.keys()].sort());
});

test("sprout and seedling text names conditions by their young labels", () => {
  // The standard labels the young bands replace (Catalyst, Spark), not the ones they keep.
  const young = Object.values(names.conditions_young);
  const swapped = Object.keys(names.conditions_young)
    .map((k) => names.conditions.find((c) => c[0] === k)?.[1] ?? k)
    .filter((label) => !young.includes(label));
  assert.ok(swapped.includes("Catalyst") && swapped.includes("Spark"), swapped.join(", "));
  for (const band of ["sprout", "seedling"]) {
    const g = guide.bands[band]!;
    const texts = [...g.walkthrough.map((s) => s.say), ...g.lessons.flatMap((l) => [l.title, l.idea, l.world?.say ?? ""]), ...Object.values(g.hints).flat(), ...g.challenges.map((c) => c.title)];
    for (const t of texts) for (const label of swapped) assert.ok(!new RegExp(`\\b${label}\\b`).test(t), `${band} names ${label}, not its young label: ${t}`);
  }
});

test("the build ships the guide for the page to fetch", () => {
  assert.match(readFileSync("crates/bevy-client/build-web.sh", "utf8"), /content\/chem\/guide\.json "\$OUT\/chem-guide\.json"/);
  assert.match(readFileSync("crates/bevy-client/web/bench-free.js", "utf8"), /fetch\('chem-guide\.json'\)/);
});

test("every Sprout and Seedling reaction has a sourced everyday line", () => {
  for (const r of core.reactions) if (rank(r.band) <= rank("seedling")) assert.ok(guide.where_you_see_it[r.id]?.trim(), `${r.id}: no where_you_see_it`);
  assert.deepEqual(Object.keys(guide.where_you_see_it_sources).sort(), Object.keys(guide.where_you_see_it).sort());
  for (const [id, line] of Object.entries(guide.where_you_see_it)) {
    assert.ok(byId.has(id), `where_you_see_it: no core reaction ${id}`);
    assert.match(line, /^[A-Z].*[.!]$/, `${id}: one sentence`);
    checkSources(`where_you_see_it ${id}`, guide.where_you_see_it_sources[id]!);
  }
});

test("fixture: the replayed engine matches by ratio and names what is missing", () => {
  assert.deepEqual(run({ CH4: 1, O2: 2 }, new Set(["spark"]), "sprout"), { kind: "known", id: "methane_burns" });
  assert.deepEqual(run({ CH4: 1, O2: 2 }, new Set(["heat"]), "sprout"), { kind: "known", id: "methane_burns" });
  assert.deepEqual(run({ CH4: 1, O2: 2 }, new Set(), "sprout"), { kind: "needs_condition", id: "methane_burns" });
  assert.equal(run({ CH4: 1, O2: 1 }, new Set(["spark"]), "sprout").kind, "none");
  assert.deepEqual(run({ K: 1, H2O: 1 }, new Set(), "seedling"), { kind: "locked", id: "potassium_water" });
  assert.deepEqual(run({ H2O: 1 }, new Set(["electricity"]), "seedling"), { kind: "known", id: "water_splits" });
  assert.deepEqual(run({ CO2: 1, H2O: 1 }, new Set(["light"]), "sprout"), { kind: "known", id: "photosynthesis" });
  // I-059: burning magnesium is a Seedling reaction now (grades 8-9, conservation of mass).
  assert.deepEqual(run({ Mg: 2, O2: 1 }, new Set(["heat"]), "sprout"), { kind: "locked", id: "magnesium_burns" });
  assert.deepEqual(run({ Mg: 2, O2: 1 }, new Set(["heat"]), "seedling"), { kind: "known", id: "magnesium_burns" });
  assert.deepEqual(judge("2K + 2H2O -> 2KOH + H2", new Set(), "seedling"), { kind: "locked", id: "potassium_water" });
  assert.deepEqual(judge("4H2 + 2O2 -> 4H2O", new Set(["spark"]), "adept"), { kind: "known", id: "hydrogen_burns" });
  assert.equal(judge("H2 + O2 -> H2O", new Set(["spark"]), "adept").kind, "unbalanced");
});
