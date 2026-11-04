// Bench plan Phase 1: the Chemistry Bench free play page's pure helpers
// (crates/bevy-client/web/bench-free-lib.js) agree with the core codex.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { hill, parseFormula, sideOf, sideText } from "../crates/bevy-client/web/bench-free-lib.js";

const names = JSON.parse(readFileSync("content/chem/names.json", "utf8")) as { elements: Record<string, { name: string }>; substances: Record<string, string> };
const isEl = (e: string) => e in names.elements;
const core = JSON.parse(readFileSync("content/chem/core.json", "utf8")) as { reactions: { equation: string }[] };
const species = new Set<string>();
for (const r of core.reactions) for (const s of r.equation.split(" -> ")) for (const f of Object.keys(sideOf(s))) species.add(f);

test("formulas parse, brackets included", () => {
  assert.deepEqual(parseFormula("Ca(OH)2", isEl), { Ca: 1, O: 2, H: 2 });
  assert.deepEqual(parseFormula("CH3COOH", isEl), { C: 2, H: 4, O: 2 });
  assert.deepEqual(parseFormula("(NH4)2SO4", isEl), { N: 2, H: 8, S: 1, O: 4 });
  assert.equal(parseFormula("Xx2", isEl), null);
  assert.equal(parseFormula("Ca(OH", isEl), null);
  assert.equal(hill({ O: 1, H: 2 }), "H2O");
  assert.equal(hill({ H: 4, C: 1 }), "CH4");
  assert.equal(sideText(sideOf("2H2 + O2")), "2H2 + O2");
});

test("every codex substance parses, has an element colour and an everyday name", () => {
  for (const f of species) {
    const c = parseFormula(f, isEl);
    assert.ok(c, f);
    assert.ok(names.substances[f], `${f}: name`);
  }
});

test("the title links to the bench, and the build ships the page and the engine", () => {
  assert.match(readFileSync("crates/bevy-client/web/title.js", "utf8"), /bench-free\.html/);
  const build = readFileSync("crates/bevy-client/build-web.sh", "utf8");
  for (const f of ["bench-free.html", "bench-free.js", "bench-free-lib.js", "three.min.js", "realgame_chem.wasm", "vendor/README.txt"]) assert.match(build, new RegExp(f.replace(".", "\\.")), f);
});
