// Wednesday polish: the title screen (crates/bevy-client/web/title.js).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { CONTROLS, wantsTitle } from "../crates/bevy-client/web/title.js";

test("players see the title; automated runs skip it unless ?title asks", () => {
  assert.equal(wantsTitle("", false), true);
  assert.equal(wantsTitle("?band=sprout", false), true);
  assert.equal(wantsTitle("", true), false, "goldens start at the intro as before");
  assert.equal(wantsTitle("?title", true), true);
  assert.equal(wantsTitle("?notitle", false), false);
});

test("the controls card names keys the game really uses", () => {
  const keys = CONTROLS.map(([k]) => k);
  for (const k of ["W A S D", "Click", "1 2 3 4", "Space", "E", "B", "Esc"]) assert.ok(keys.includes(k), k);
  // B opens the Bench and E uses things: check the handlers exist.
  const bench = readFileSync("crates/bevy-client/web/bench.js", "utf8");
  assert.match(bench, /e\.code === 'KeyB'/);
  const late = readFileSync("crates/bevy-client/web/late.js", "utf8");
  assert.match(late, /'KeyE'/);
});

test("the intro waits under the title, and the build ships title.js", () => {
  const story = readFileSync("crates/bevy-client/web/story.js", "utf8");
  assert.match(story, /title-open/);
  const build = readFileSync("crates/bevy-client/build-web.sh", "utf8");
  assert.match(build, /web\/title\.js/);
});
