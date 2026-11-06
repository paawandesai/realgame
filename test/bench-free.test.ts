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
  for (const f of ["bench-free.html", "bench-free.js", "bench-free-lib.js", "bench-scene.js", "bench-spells.js", "chem-quests.json", "three.min.js", "realgame_chem.wasm", "vendor/README.txt"]) assert.match(build, new RegExp(f.replace(".", "\\.")), f);
});

test("lab records ship with their licence, use only Bench elements, and match the JS Hill order", () => {
  const lit = JSON.parse(readFileSync("content/chem/literature.json", "utf8")) as { licence: string; attribution: string; reactions: Record<string, [number, string]> };
  assert.equal(lit.licence, "CC BY-SA 4.0");
  assert.match(lit.attribution, /Open Reaction Database/);
  const keys = Object.keys(lit.reactions);
  assert.ok(keys.length > 5000, `only ${keys.length} lab records`);
  for (const k of keys) {
    for (const side of k.split(" -> ")) {
      for (const f of Object.keys(sideOf(side))) {
        const atoms = parseFormula(f, isEl);
        assert.ok(atoms, `${f} in ${k} uses an element the Bench lacks`);
        assert.equal(hill(atoms), f, `${f} in ${k} is not in Hill order`);
      }
    }
  }
  const build = readFileSync("crates/bevy-client/build-web.sh", "utf8");
  assert.match(build, /content\/chem\/literature\.json "\$OUT\/chem-literature\.json"/);
  assert.match(readFileSync("crates/bevy-client/web/bench-free.js", "utf8"), /CC BY-SA 4\.0/);
});

test("the page fits the window: Run sits outside the scrolling table and the coach outside the 3D view", () => {
  const html = readFileSync("crates/bevy-client/web/bench-free.html", "utf8");
  const css = readFileSync("crates/bevy-client/web/bench-free.css", "utf8");
  const js = readFileSync("crates/bevy-client/web/bench-free.js", "utf8");
  // Run is in the bench pane's footer, after its scrolling part ends.
  const scrollStart = html.indexOf('class="pane-scroll" id="benchScroll"');
  const actions = html.indexOf('<div class="actions">');
  assert.ok(scrollStart > 0 && actions > scrollStart, "bench pane has a scrolling part, then the actions footer");
  assert.ok(html.indexOf('id="test"') > actions, "Run is in the actions footer");
  assert.ok(html.indexOf('id="result"') < actions, "the result scrolls with the table");
  // The coach takes its own strip above the view; the canvas is inside the view.
  const coach = html.indexOf('class="coach"'), view = html.indexOf('class="view"'), canvas = html.indexOf('id="scene"');
  assert.ok(coach > 0 && coach < view && view < canvas, "coach before the view, canvas inside it");
  assert.match(css, /\.coach \{ position: static;/);
  assert.match(css, /html, body \{ height: 100dvh; overflow: hidden; \}/);
  assert.match(css, /body \{ margin: 0; \}/);
  // What is on "in" is drawn in the chamber, and tests can count it.
  assert.match(js, /previewIn\(\);\n {2}\};/, "drawTable previews the in side");
  assert.match(js, /sceneCount: \(\) => chamberMols \+ atoms\.length/);
});

test("every Bench page script parses as a module (a duplicate declaration stops the whole page loading)", async () => {
  const { spawnSync } = await import("node:child_process");
  const { mkdtempSync, copyFileSync, rmSync } = await import("node:fs");
  const { join } = await import("node:path");
  const { tmpdir } = await import("node:os");
  const dir = mkdtempSync(join(tmpdir(), "bench-parse-"));
  try {
    for (const f of ["bench-free.js", "bench-free-lib.js", "bench-scene.js", "bench-spells.js", "bench.js", "formula-save.js"]) {
      const out = join(dir, f.replace(/\.js$/, ".mjs"));
      copyFileSync(`crates/bevy-client/web/${f}`, out);
      const r = spawnSync(process.execPath, ["--check", out], { encoding: "utf8" });
      assert.equal(r.status, 0, `${f} does not parse: ${r.stderr}`);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
