// I-053: the synthesised sound library (crates/bevy-client/web/sfx.js), with
// a mocked AudioContext that records every node and every scheduled value.
import assert from "node:assert/strict";
import { test } from "node:test";
import { createSfx, DIG, PLACE, STEP, digMaterial, stepMaterial } from "../crates/bevy-client/web/sfx.js";

class Param {
  value: number;
  events: Array<[string, number, number]> = [];
  constructor(v = 0) { this.value = v; }
  setValueAtTime(v: number, t: number) { this.events.push(["set", v, t]); this.value = v; return this; }
  linearRampToValueAtTime(v: number, t: number) { this.events.push(["lin", v, t]); return this; }
  exponentialRampToValueAtTime(v: number, t: number) { this.events.push(["exp", v, t]); return this; }
  setTargetAtTime(v: number, t: number) { this.events.push(["target", v, t]); this.value = v; return this; }
  cancelScheduledValues(t: number) { this.events.push(["cancel", 0, t]); return this; }
}
class Node {
  kind: string;
  outs: Node[] = [];
  constructor(kind: string) { this.kind = kind; }
  connect(n: Node | Param) { if (n instanceof Node) this.outs.push(n); return n; }
}
class Gain extends Node { gain = new Param(1); constructor() { super("gain"); } }
class Filter extends Node { type = "lowpass"; frequency = new Param(350); Q = new Param(1); constructor() { super("filter"); } }
class Comp extends Node {
  threshold = new Param(-24); knee = new Param(30); ratio = new Param(12); attack = new Param(0.003); release = new Param(0.25);
  constructor() { super("compressor"); }
}
class Source extends Node {
  started = false; stopped = false;
  start() { this.started = true; }
  stop() { this.stopped = true; }
}
class Osc extends Source { type = "sine"; frequency = new Param(440); constructor() { super("osc"); } }
class Buf extends Source { buffer: unknown = null; loop = false; constructor() { super("buffer"); } }

class MockContext {
  sampleRate = 8000;
  currentTime = 1;
  state = "running";
  destination = new Node("destination");
  nodes: Node[] = [];
  private add<T extends Node>(n: T) { this.nodes.push(n); return n; }
  createGain() { return this.add(new Gain()); }
  createBiquadFilter() { return this.add(new Filter()); }
  createDynamicsCompressor() { return this.add(new Comp()); }
  createOscillator() { return this.add(new Osc()); }
  createBufferSource() { return this.add(new Buf()); }
  createBuffer(_ch: number, n: number, sr: number) {
    const data = new Float32Array(n);
    return { duration: n / sr, sampleRate: sr, length: n, getChannelData: () => data };
  }
  resume() { return Promise.resolve(); }
}

const make = () => {
  const ctx = new MockContext();
  const sfx = createSfx({ makeContext: () => ctx as unknown as AudioContext });
  return { ctx, sfx };
};

const REQUIRED = [
  "water_charge", "water_splash", "hiss", "steam", "flame_whoosh", "fire_crackle", "frost_spray",
  "ice_crack", "ice_drip", "co2_puff", "h2_pop", "methane_whoomp", "glass_ting",
  "dig_dirt", "dig_stone", "dig_sand", "dig_wood", "dig_ice",
  "dig_coal", "dig_iron_ore", "dig_crystal", "dig_glass",
  "dig_mud", "dig_leaves", "dig_snow", "dig_iron", "dig_brick", "dig_planks", "place_block",
  "step_grass", "step_stone", "step_sand", "step_snow", "step_wood", "step_ice", "step_crystal",
  "step_mud", "step_leaves",
  "pickup", "bench_success", "bench_wrong", "gate_open", "fanfare",
];

test("every sound in the list exists and plays from generated nodes only", () => {
  const { ctx, sfx } = make();
  for (const n of REQUIRED) assert.ok(sfx.names.includes(n), `missing ${n}`);
  assert.equal(sfx.unlock(), true);
  for (const n of sfx.names) {
    const before = ctx.nodes.length;
    const h = sfx.play(n);
    assert.ok(h && typeof h.stop === "function", `${n} returns a handle`);
    const made = ctx.nodes.slice(before);
    assert.ok(made.some((x) => x instanceof Source && x.started), `${n} starts a generated source`);
    assert.doesNotThrow(() => h.stop(), `${n} stops`);
    // Sources are oscillators or the code-made noise buffers: no samples.
    for (const x of made) if (x instanceof Buf) assert.ok(x.buffer, `${n}: buffer source has a generated buffer`);
  }
  assert.deepEqual(DIG, ["dirt", "stone", "sand", "wood", "ice", "coal", "iron_ore", "crystal", "glass", "mud", "leaves", "snow", "iron", "brick", "planks"]);
  assert.deepEqual(STEP, ["grass", "stone", "sand", "snow", "wood", "ice", "crystal", "mud", "leaves"]);
  assert.deepEqual(PLACE, ["dirt", "stone", "sand", "wood", "ice", "glass", "crystal", "coal", "ore"]);
  for (const m of [...DIG, "ore", "lava"]) assert.ok(sfx.play("dig", { material: m }), `dig ${m}`);
  for (const m of [...STEP, "lava"]) assert.ok(sfx.play("step", { material: m }), `step ${m}`);
  // place_block varies by material and keeps a default; each material
  // builds a different graph from the default one.
  const graph = (o: Record<string, string>) => {
    const before = ctx.nodes.length;
    sfx.play("place_block", o);
    return ctx.nodes.slice(before).map((n) => (n instanceof Filter ? `${n.kind}:${n.type}:${n.frequency.events[0]?.[1]}` : n instanceof Osc ? `osc:${n.frequency.events[0]?.[1]}` : n.kind)).join(",");
  };
  const base = graph({});
  assert.equal(graph({ material: "lava" }), base, "an unknown material is the default");
  const seen = new Set<string>();
  for (const m of PLACE) {
    const g = graph({ material: m });
    assert.notEqual(g, base, `place ${m} differs from the default`);
    seen.add(g);
  }
  assert.equal(seen.size, PLACE.length, "every placed material sounds different");
  assert.equal(graph({ material: "ore" }), graph({ material: "iron_ore" }), "ore is iron ore");
});

test("an unknown name is a no-op, and nothing plays before the first gesture", () => {
  const { ctx, sfx } = make();
  assert.equal(sfx.play("water_splash"), null, "silent before unlock");
  assert.equal(ctx.nodes.length, 0, "no context work before unlock");
  sfx.unlock();
  const before = ctx.nodes.length;
  assert.equal(sfx.play("no_such_sound"), null);
  assert.equal(sfx.play(""), null);
  assert.equal(ctx.nodes.length, before, "an unknown name creates nothing");
  // A second unlock reuses the context.
  sfx.unlock();
  assert.equal(ctx.nodes.length, before);
});

test("volume is clamped to 0..1 and NaN keeps it", () => {
  const { ctx, sfx } = make();
  assert.equal(sfx.setVolume(2), 1);
  assert.equal(sfx.setVolume(-1), 0);
  assert.equal(sfx.setVolume(0.4), 0.4);
  assert.equal(sfx.setVolume(Number.NaN), 0.4);
  assert.equal(sfx.setVolume(Number.POSITIVE_INFINITY), 0.4);
  sfx.unlock();
  const master = ctx.nodes.find((n) => n instanceof Gain) as Gain;
  assert.equal(master.gain.value, 0.4, "the master starts at the chosen volume");
  sfx.setVolume(5);
  assert.equal(master.gain.value, 1);
});

test("output stays at or below -6 dBFS: limiter, warm low-pass and a 0.5 trim", () => {
  const { ctx, sfx } = make();
  sfx.unlock();
  const comp = ctx.nodes.find((n) => n instanceof Comp) as Comp;
  assert.ok(comp, "a master limiter");
  assert.ok(comp.ratio.value >= 20 && comp.threshold.value <= -6);
  const warm = ctx.nodes.find((n) => n instanceof Filter) as Filter;
  assert.equal(warm.type, "lowpass");
  assert.ok(warm.frequency.value <= 8000, "no harsh highs");
  const trim = comp.outs[0] as Gain;
  assert.ok(trim instanceof Gain && trim.gain.value <= 0.5, "trim to -6 dBFS");
  assert.ok(trim.outs.includes(ctx.destination));
  // Every envelope peak at full per-sound volume is at most 1 before the master.
  for (const n of sfx.names) {
    const before = ctx.nodes.length;
    sfx.play(n, { volume: 5 });
    for (const g of ctx.nodes.slice(before)) {
      if (!(g instanceof Gain)) continue;
      for (const [kind, v] of g.gain.events) if (kind !== "cancel") assert.ok(v <= 1, `${n}: gain ${v}`);
    }
  }
});

// The block world's materials (I-052 spec 5.2, content/materials.json): every
// block you can dig has its own dig sound, and every block you can stand on a
// footstep, either directly or through a block that sounds the same.
const DIGGABLE = ["dirt", "grass", "sand", "stone", "cliffstone", "runestone", "wood", "leaves", "planks", "snow", "ice", "mud",
  "coal_ore", "iron_ore", "crystal", "glass", "iron", "brick", "ash"];
const WALKABLE = ["dirt", "grass", "sand", "stone", "cliffstone", "runestone", "wood", "leaves", "planks", "snow", "ice", "mud",
  "coal_ore", "iron_ore", "crystal", "glass", "iron", "brick", "ash"];

test("every block-world material has a dig sound and a footstep", () => {
  for (const m of DIGGABLE) assert.ok(DIG.includes(digMaterial(m)), `dig ${m} -> ${digMaterial(m)}`);
  for (const m of WALKABLE) assert.ok(STEP.includes(stepMaterial(m)), `step ${m} -> ${stepMaterial(m)}`);
  assert.equal(digMaterial("coal_ore"), "coal");
  assert.equal(digMaterial("ore"), "iron_ore");
  assert.equal(stepMaterial("planks"), "wood");
  assert.equal(digMaterial("lava"), "lava", "unknown ids pass through (the sound then falls back)");
  const { ctx, sfx } = make();
  sfx.unlock();
  for (const m of DIGGABLE) {
    const before = ctx.nodes.length;
    assert.ok(sfx.play("dig", { material: m }), `dig ${m}`);
    assert.ok(ctx.nodes.length > before);
  }
  for (const m of WALKABLE) assert.ok(sfx.play("step", { material: m }), `step ${m}`);
});
