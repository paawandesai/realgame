// Submission P13: ambience beds per area (crates/bevy-client/web/ambience.js).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { AREAS, areaFor, gap } from "../crates/bevy-client/web/ambience.js";

test("every area has a bed and its own details", () => {
  const want: Record<string, string[]> = { village: ["bird"], frozen: ["creak"], marsh: ["bubble"], cave: ["drip"], kiln: ["crackle"] };
  for (const [area, details] of Object.entries(want)) {
    const a = (AREAS as any)[area];
    assert.ok(a && a.bed.level > 0, area);
    assert.deepEqual(a.details.map((d: [string, number, number]) => d[0]), details, area);
    for (const [, lo, hi] of a.details) assert.ok(lo > 0 && hi >= lo, area);
  }
  assert.ok((AREAS as any).marsh.extra, "insects");
  assert.ok((AREAS as any).cave.hum.length > 0, "the cave hums");
});

test("the area follows the biome, and the Kiln at the ending", () => {
  assert.equal(areaFor({ biome: "marsh" }), "marsh");
  assert.equal(areaFor({ biome: "marsh", ending: true }), "kiln");
  assert.equal(areaFor({ biome: "nowhere" }), null);
  assert.equal(areaFor(null), null);
  assert.equal(gap(2, 4, 0), 2);
  assert.equal(gap(2, 4, 1), 4);
});

test("the biomes the game reports all have a bed, and the build ships it", () => {
  const view = readFileSync("crates/voxel/src/lib.rs", "utf8");
  for (const b of ["village", "frozen", "marsh", "cave", "kiln"]) assert.ok(view.includes(`"${b}"`), b);
  assert.match(readFileSync("crates/bevy-client/build-web.sh", "utf8"), /web\/ambience\.js/);
  assert.match(readFileSync("crates/bevy-client/build-web.sh", "utf8"), /web\/feel\.js/);
});
