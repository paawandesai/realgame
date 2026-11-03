// Submission P7/P13: game feel (crates/bevy-client/web/feel.js).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { HIT_SOUND, landVolume, place, STEP_M, stepsBetween } from "../crates/bevy-client/web/feel.js";

test("a footstep every STEP_M metres walked, none standing still", () => {
  assert.equal(stepsBetween(0, STEP_M - 0.01), 0);
  assert.equal(stepsBetween(STEP_M - 0.01, STEP_M + 0.01), 1);
  assert.equal(stepsBetween(10, 10), 0);
  assert.equal(stepsBetween(10, 9), 0, "a reset never plays steps");
});

test("a landing to the right pans right, behind-left pans left, and far is quieter", () => {
  // Yaw 0 faces -z; right is +x.
  const r = place([0, 0, 0], 0, [5, 0, -5]);
  assert.ok(r.pan > 0.4, `${r.pan}`);
  const l = place([0, 0, 0], 0, [-5, 0, 5]);
  assert.ok(l.pan < -0.4, `${l.pan}`);
  const ahead = place([0, 0, 0], 0, [0, 0, -10]);
  assert.ok(Math.abs(ahead.pan) < 1e-6);
  // Turned 90° left (yaw +90: forward is -x), +z is now on the left.
  assert.ok(place([0, 0, 0], 90, [0, 0, 5]).pan < -0.4);
  assert.ok(place([0, 0, 0], 0, [0, 0, -40]).volume < place([0, 0, 0], 0, [0, 0, -4]).volume);
  assert.ok(place([0, 0, 0], 0, [0, 0, -400]).volume > 0, "never silent");
});

test("harder landings are louder; every spell has a landing sound the library has", () => {
  assert.ok(landVolume(12) > landVolume(5));
  assert.ok(landVolume(3.5) >= 0.35 && landVolume(30) <= 1);
  const sfx = readFileSync("crates/bevy-client/web/sfx.js", "utf8");
  for (const n of [...HIT_SOUND, "land", "ui_confirm", "step"]) assert.match(sfx, new RegExp(`\\n  ${n}: \\(c`), n);
  assert.equal(HIT_SOUND.length, 4);
});
