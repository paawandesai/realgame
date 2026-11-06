// The Chemistry Bench free play page in a real browser (Playwright,
// Chromium): Paawan's laptop playtest. The page fits the window with no page
// scroll at four laptop and desktop sizes in every band and tab, the Run
// button is always on screen, what is on "in" shows in the 3D scene, the
// Guide coach covers no control, and each band's walkthrough can be followed
// step by step to its discovery.
//
// Needs a built web client (chem.js, chem_bg.wasm, vendor/three.min.js...):
// BENCH_PKG=<dir>, or crates/bevy-client/web/pkg-out from build-web.sh. The
// page's own files (bench-free.*, bench-scene.js, bench-free-lib.js) are
// served from the source tree, so the test checks this checkout. Playwright
// comes from the global npm root (NODE_PATH=$(npm root -g)). Without a build
// or Playwright the test is skipped and says why.
import assert from "node:assert/strict";
import { execSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { createRequire } from "node:module";
import { extname, join, resolve } from "node:path";
import { after, before, test } from "node:test";

const SRC = resolve("crates/bevy-client/web");
const PKG = resolve(process.env.BENCH_PKG || join(SRC, "pkg-out"));
const OWN = new Set(["bench-free.html", "bench-free.css", "bench-free.js", "bench-scene.js", "bench-free-lib.js", "bench-spells.js", "bench.js", "formula-save.js"]);
const SIZES: [number, number][] = [[1280, 720], [1440, 800], [1536, 864], [1920, 1080]];
const BANDS = ["sprout", "seedling", "apprentice", "adept"] as const;
const TABS = ["bench", "codex", "notes", "guide", "spells"];

function loadPlaywright(): any {
  const tries = [process.env.NODE_PATH, (() => { try { return execSync("npm root -g", { encoding: "utf8" }).trim(); } catch { return ""; } })()];
  for (const root of tries) {
    if (!root) continue;
    try { return createRequire(join(root, "noop.js"))("playwright"); } catch { /* next */ }
  }
  try { return createRequire(import.meta.url)("playwright"); } catch { return null; }
}

const why = !existsSync(join(PKG, "chem_bg.wasm")) || !existsSync(join(PKG, "vendor", "three.min.js"))
  ? `no built web client at ${PKG} (run crates/bevy-client/build-web.sh or set BENCH_PKG)`
  : "";
const pw = why ? null : loadPlaywright();
const skip = why || (pw ? "" : "Playwright is not installed (npm i -g playwright)");

const TYPES: Record<string, string> = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".wasm": "application/wasm", ".svg": "image/svg+xml" };
let server: Server | null = null;
let base = "";
let browser: any = null;

before(async () => {
  if (skip) return;
  server = createServer((req, res) => {
    const path = decodeURIComponent((req.url || "/").split("?")[0] || "/").replace(/^\/+/, "") || "index.html";
    const file = OWN.has(path) ? join(SRC, path) : join(PKG, path);
    if (!file.startsWith(PKG) && !file.startsWith(SRC)) { res.writeHead(403).end(); return; }
    if (!existsSync(file) || !statSync(file).isFile()) { res.writeHead(404).end(); return; }
    res.writeHead(200, { "content-type": TYPES[extname(file)] || "application/octet-stream" }).end(readFileSync(file));
  });
  await new Promise<void>((ok) => server!.listen(0, "127.0.0.1", ok));
  base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  browser = await pw.chromium.launch({ args: ["--use-gl=swiftshader", "--enable-webgl", "--ignore-gpu-blocklist"] });
});

after(async () => {
  await browser?.close();
  server?.close();
});

/** A fresh page (nothing saved) at a size, bench ready. */
async function open(w: number, h: number, band = "apprentice"): Promise<any> {
  const page = await browser.newPage({ viewport: { width: w, height: h } });
  const errors: string[] = [];
  page.on("pageerror", (e: Error) => errors.push(e.message));
  await page.goto(`${base}/bench-free.html?band=${band}`);
  await page.waitForFunction(() => (window as any).__benchFree && (window as any).__benchFree.guide());
  page.errors = errors;
  return page;
}

type Rect = { x: number; y: number; w: number; h: number; name: string };
const overlaps = (a: Rect, b: Rect) => a.x < b.x + b.w - 0.5 && b.x < a.x + a.w - 0.5 && a.y < b.y + b.h - 0.5 && b.y < a.y + a.h - 0.5;

/** Page scroll sizes, the Run button's box, and the coach against everything it could cover. */
const measure = (page: any) => page.evaluate(() => {
  const box = (el: Element | null, name: string) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return r.width && r.height ? { x: r.left, y: r.top, w: r.width, h: r.height, name } : null;
  };
  const coach = document.getElementById("coach") as HTMLElement;
  const others = [
    ...[...document.querySelectorAll(".guide-target")].map((e) => box(e, "the item it points at")),
    box(document.getElementById("scene"), "the 3D scene"),
    box(document.querySelector(".readout"), "the plate readout"),
    box(document.querySelector(".hint"), "the hint"),
    box(document.getElementById("chamberTag"), "the chamber tag"),
    ...[...document.querySelectorAll("button, input, a.btn")].filter((e) => !coach.contains(e)).map((e) => box(e, `control "${(e as HTMLElement).innerText || (e as HTMLInputElement).placeholder || e.id}"`)),
  ].filter(Boolean);
  return {
    sh: document.documentElement.scrollHeight, sw: document.documentElement.scrollWidth,
    bh: document.body.scrollHeight, iw: innerWidth, ih: innerHeight,
    run: box(document.getElementById("test"), "run"),
    coach: coach.hidden ? null : box(coach, "coach"),
    others,
  };
});

test("the bench fits the window with no page scroll, Run on screen, in every band and tab", { skip: skip || false, timeout: 240_000 }, async () => {
  for (const [w, h] of SIZES) {
    const page = await open(w, h);
    for (const band of BANDS) {
      await page.click(`.band button[data-band="${band}"]`);
      for (const tab of TABS) {
        await page.click(`.tabs button[data-tab="${tab}"]`);
        const m = await measure(page);
        const at = `${w}x${h} ${band} ${tab}`;
        assert.ok(m.sh <= m.ih && m.bh <= m.ih, `${at}: page is ${m.sh} tall in a ${m.ih} window`);
        assert.ok(m.sw <= m.iw, `${at}: page is ${m.sw} wide in a ${m.iw} window`);
        if (tab === "bench") {
          assert.ok(m.run, `${at}: no Run button`);
          assert.ok(m.run.y >= 0 && m.run.y + m.run.h <= m.ih && m.run.x >= 0 && m.run.x + m.run.w <= m.iw, `${at}: Run is off screen (${JSON.stringify(m.run)})`);
        }
      }
    }
    assert.deepEqual(page.errors, [], `${w}x${h}: page errors`);
    await page.close();
  }
});

test("the Guide coach covers no control, the scene, or the item it points at", { skip: skip || false, timeout: 240_000 }, async () => {
  for (const [w, h] of SIZES) {
    const page = await open(w, h, "sprout");
    for (const band of BANDS) {
      // A band not seen yet starts its walkthrough. Walk every step by doing it, checking the coach at each one.
      if (band !== "sprout") await page.click(`.band button[data-band="${band}"]`);
      assert.ok(await page.isVisible("#coach"), `${w}x${h} ${band}: the walkthrough did not start`);
      for (let i = 0; i < 8; i++) {
        const m = await measure(page);
        if (!m.coach) break;
        assert.ok(m.coach.y >= 0 && m.coach.y + m.coach.h <= m.ih, `${w}x${h} ${band}: coach off screen`);
        for (const o of m.others as Rect[]) assert.ok(!overlaps(m.coach, o), `${w}x${h} ${band} step ${i + 1}: the coach covers ${o.name}`);
        if (!(await doStep(page))) break;
      }
    }
    assert.deepEqual(page.errors, [], `${w}x${h}: page errors`);
    await page.close();
  }
});

/** Does what the walkthrough's current step asks, through the page. Returns false when there is no step. */
async function doStep(page: any): Promise<boolean> {
  const step = await page.evaluate(() => {
    const b = (window as any).__benchFree; const w = b.walk(); if (!w) return null;
    return b.guide().bands[w.band].walkthrough[w.i].do;
  });
  if (!step) return false;
  const [k, v] = Object.entries(step)[0] as [string, any];
  if (k === "tap_bottle") await page.click(`.bottle[data-formula="${v}"]`);
  else if (k === "condition") await page.click(`.cond[data-cond="${v}"]`);
  else if (k === "open_tab") await page.click(`.tabs button[data-tab="${v === "notebook" ? "notes" : v}"]`);
  else if (k === "type") { await page.fill("#typeIn", String(v)); await page.press("#typeIn", "Enter"); }
  else if (k === "run") await page.click("#test");
  else throw new Error(`unknown step ${k}`);
  await page.waitForTimeout(50);
  return true;
}

test("each band's walkthrough starts at step 1 and every step advances to the discovery", { skip: skip || false, timeout: 120_000 }, async () => {
  for (const band of BANDS) {
    const page = await open(1440, 800, band);
    const steps: number = await page.evaluate((b: string) => (window as any).__benchFree.guide().bands[b].walkthrough.length, band);
    assert.match(await page.textContent("#coachStep"), new RegExp(`^Step 1 of ${steps}$`), `${band}: walkthrough did not start at step 1`);
    for (let i = 0; i < steps; i++) {
      const at = await page.evaluate(() => (window as any).__benchFree.walk()?.i);
      assert.equal(at, i, `${band}: expected step ${i + 1}`);
      assert.ok(await page.isVisible("#coach"), `${band}: coach hidden at step ${i + 1}`);
      const target = await page.$(".guide-target");
      assert.ok(target, `${band} step ${i + 1}: nothing is lit up to tap`);
      assert.ok(await target.isVisible(), `${band} step ${i + 1}: the lit item is hidden`);
      await doStep(page);
    }
    await page.waitForFunction(() => (window as any).__benchFree.walk() === null, null, { timeout: 5000 });
    assert.match(await page.textContent("#coachSay"), /🎉/, `${band}: no discovery at the end`);
    // Starting again from the Guide tab begins at step 1 with a clean table.
    await page.click('.tabs button[data-tab="guide"]');
    await page.click("#guide .btn.primary");
    assert.match(await page.textContent("#coachStep"), new RegExp(`^Step 1 of ${steps}$`));
    assert.equal(await page.evaluate(() => Object.keys((window as any).__benchFree.left).length), 0, `${band}: restart left the table full`);
    assert.deepEqual(page.errors, [], `${band}: page errors`);
    await page.close();
  }
});

test("bottles on \"in\" show in the 3D scene, follow the counts and clear with the table", { skip: skip || false, timeout: 60_000 }, async () => {
  const page = await open(1440, 800, "seedling");
  const count = () => page.evaluate(() => (window as any).__benchFree.sceneCount());
  const chamber = () => page.evaluate(() => (window as any).__benchFree.chamberCount());
  assert.equal(await count(), 0);
  await page.click('.bottle[data-formula="Na"]');
  await page.click('.bottle[data-formula="Na"]');
  assert.equal(await chamber(), 2, "two sodium on in: two in the chamber");
  assert.ok(await count() > 0);
  assert.match(await page.textContent("#chamberTag"), /2Na/);
  await page.click('#leftSide button[aria-label="One less Na"]');
  assert.equal(await chamber(), 1, "the chamber follows the count");
  await page.click('.bottle[data-formula="NaHCO3"]');
  assert.equal(await chamber(), 2);
  for (let i = 0; i < 9; i++) await page.click('#leftSide button[aria-label="One more NaHCO3"]');
  assert.equal(await chamber(), 7, "capped at six of one substance");
  assert.match(await page.textContent("#chamberTag"), /showing 7/);
  await page.click("#clearTable");
  assert.equal(await count(), 0, "Clear the table empties the chamber");
  assert.ok(await page.isHidden("#chamberTag"));
  // The element shelf still builds on the plate.
  await page.click('.jar[aria-label^="Hydrogen"]');
  assert.equal(await page.evaluate(() => (window as any).__benchFree.plateCount()), 1);
  await page.click("#clear");
  assert.equal(await count(), 0);
  // Sprout (grades 6-7, I-059) sees it too, with formulas, and has the
  // element shelf and plate as models, but no "out" side.
  await page.click('.band button[data-band="sprout"]');
  await page.click('.bottle[data-formula="H2"]');
  await page.click('.bottle[data-formula="H2"]');
  assert.equal(await chamber(), 2);
  assert.match(await page.textContent("#chamberTag"), /2H₂/);
  assert.match(await page.textContent('.bottle[data-formula="H2"]'), /H₂/);
  assert.ok(await page.isVisible("#elementBox"), "Sprout has the element shelf");
  assert.ok(await page.isHidden("#rightSide") && await page.isHidden("#toRight"), "Sprout has no out side");
  assert.deepEqual(page.errors, []);
  await page.close();
});
