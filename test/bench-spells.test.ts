// I-053: discoveries become spells. The shared save (web/formula-save.js),
// the world's sync and hotbar labels (web/discover.js), and the content
// files the Rust rules read. See docs/loop/items/I-053/spec.md.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import {
  addFound,
  cleanSave,
  emptySave,
  FOUND_KEY,
  loadCommands,
  MASTERY_BUILDS,
  mergeSaves,
  PROGRESS_KEY,
  putOnKey,
  readFound,
  readProgress,
  readSpells,
  recordBuild,
  saveSpell,
  SPELLS_KEY,
  writeSpells,
} from "../crates/bevy-client/web/formula-save.js";
import { startDiscover } from "../crates/bevy-client/web/discover.js";
import { mountSpells } from "../crates/bevy-client/web/bench-spells.js";

type Store = { data: Map<string, string>; getItem(k: string): string | null; setItem(k: string, v: string): void };
function memoryStorage(init: Record<string, string> = {}): Store {
  const data = new Map<string, string>(Object.entries(init));
  return { data, getItem: (k) => data.get(k) ?? null, setItem: (k, v) => void data.set(k, v) };
}
const throwing = {
  getItem(): string | null { throw new Error("blocked"); },
  setItem(): void { throw new Error("blocked"); },
};

test("the shared keys: the Bench page's finds and one spell save", () => {
  assert.equal(FOUND_KEY, "formula.bench.found");
  assert.equal(SPELLS_KEY, "formula.spells.v1");
  assert.equal(MASTERY_BUILDS, 2);
  const chem = readFileSync("crates/chem/src/discover.rs", "utf8");
  assert.match(chem, /pub const MASTERY_BUILDS: u8 = 2;/);
  // The Bench page stores finds under the same key (store prefix + "found").
  const page = readFileSync("crates/bevy-client/web/bench-free.js", "utf8");
  assert.match(page, /localStorage\.getItem\('formula\.bench\.' \+ k\)/);
  assert.match(page, /store\.set\('found', found\)/);
  assert.match(page, /saveSpell\(localStorage, r\.id\)/);
});

test("saving a find as a spell, and finds flowing both ways", () => {
  const s = memoryStorage({ [FOUND_KEY]: JSON.stringify(["carbon_burns"]) });
  assert.deepEqual(readFound(s), ["carbon_burns"]);
  addFound(s, ["methane_burns", "carbon_burns", "Bad Id"]);
  assert.deepEqual(readFound(s), ["carbon_burns", "methane_burns"]);
  assert.ok(saveSpell(s, "carbon_burns"));
  assert.ok(!saveSpell(s, "<script>"));
  assert.deepEqual(readSpells(s).saved, { carbon_burns: 0 });
  // The world wrote 2 builds; a later Bench save keeps them.
  writeSpells(s, { v: 1, saved: { carbon_burns: 2 }, slots: ["carbon_burns", null, null, null], quests: ["melt_with_reaction"] });
  saveSpell(s, "carbon_burns");
  saveSpell(s, "chalk_acid");
  const back = readSpells(s);
  assert.deepEqual(back.saved, { carbon_burns: 2, chalk_acid: 0 });
  assert.deepEqual(back.slots, ["carbon_burns", null, null, null]);
  assert.deepEqual(back.quests, ["melt_with_reaction"]);
});

test("the world's write merges with a spell the Bench saved meanwhile", () => {
  const s = memoryStorage();
  saveSpell(s, "zinc_acid");
  writeSpells(s, { v: 1, saved: { carbon_burns: 1 }, slots: [null, null, null, "nope"], quests: [] });
  assert.deepEqual(readSpells(s).saved, { zinc_acid: 0, carbon_burns: 1 });
  assert.deepEqual(readSpells(s).slots, [null, null, null, null], "a slot must hold a saved spell");
  const m = mergeSaves(cleanSave({ saved: { a: 3 } }), cleanSave({ saved: { a: 1, b: 0 }, slots: ["a"] }));
  assert.deepEqual(m.saved, { a: 3, b: 0 });
  assert.deepEqual(m.slots, ["a", null, null, null]);
});

test("corrupt or blocked storage costs only saving", () => {
  const s = memoryStorage({ [SPELLS_KEY]: "{not json", [FOUND_KEY]: "42" });
  assert.deepEqual(readSpells(s), emptySave());
  assert.deepEqual(readFound(s), []);
  assert.deepEqual(readSpells(throwing), emptySave());
  assert.equal(saveSpell(throwing, "carbon_burns"), false);
  assert.deepEqual(cleanSave({ saved: { x: 999, "Y!": 1 }, slots: "no", quests: [1, "q"] }), { v: 1, saved: { x: 255 }, slots: [null, null, null, null], quests: ["q"] });
});

test("load commands carry the save to the game", () => {
  const save = cleanSave({ saved: { carbon_burns: 1 }, slots: ["carbon_burns"], quests: [] });
  assert.deepEqual(loadCommands(save, ["carbon_burns"]), [
    "found carbon_burns",
    "saved carbon_burns:1",
    "slots carbon_burns,-,-,-",
    "quests -",
  ]);
  assert.deepEqual(loadCommands(emptySave(), []), ["found -", "saved -", "slots -,-,-,-", "quests -"]);
});

test("the world loads the save in the block world and writes it back when rev grows", () => {
  const storage = memoryStorage({ [SPELLS_KEY]: JSON.stringify({ v: 1, saved: { carbon_burns: 0 }, slots: [null, null, null, null], quests: [] }) });
  let tick: () => void = () => {};
  const listeners: Record<string, (e: unknown) => void> = {};
  const win: Record<string, unknown> = {
    localStorage: storage,
    __gameState: JSON.stringify({ mode: "voxel" }),
    document: { querySelectorAll: () => [] },
    setInterval: (f: () => void) => { tick = f; },
    addEventListener: (k: string, f: (e: unknown) => void) => { listeners[k] = f; },
  };
  startDiscover(win);
  tick();
  const sent = (win.__gameIn as { disc: string[] }).disc;
  assert.deepEqual(sent.slice(0, 2), ["found -", "saved carbon_burns:0"]);
  // The game equipped it and built it once: rev 2.
  win.__discState = JSON.stringify({ rev: 2, loaded: true, on: [true, false, false, false], names: ["Carbon burns"], kinds: ["fire"], save: { v: 1, saved: { carbon_burns: 1 }, slots: ["carbon_burns", null, null, null], quests: [] } });
  win.__workState = JSON.stringify({ stages: ["learning", "unknown", "unknown", "unknown"] });
  tick();
  assert.deepEqual(readSpells(storage).saved, { carbon_burns: 1 });
  assert.deepEqual(readSpells(storage).slots[0], "carbon_burns");
  assert.deepEqual(readFound(storage), ["methane_burns"], "Fire learned in the world is a Bench find");
  // The Bench page saved in another tab: the save is sent again.
  (win.__gameIn as { disc: string[] }).disc = [];
  listeners.storage?.({ key: SPELLS_KEY });
  assert.equal((win.__gameIn as { disc: string[] }).disc.length, 4);
});

test("not in the classic courtyard", () => {
  let tick: () => void = () => {};
  const win: Record<string, unknown> = {
    localStorage: memoryStorage(),
    __gameState: JSON.stringify({ mode: "classic" }),
    setInterval: (f: () => void) => { tick = f; },
  };
  startDiscover(win);
  tick();
  assert.equal(win.__gameIn, undefined);
});

test("content: bottles, effects and quests are well formed", () => {
  const core = JSON.parse(readFileSync("content/chem/core.json", "utf8")) as { reactions: { id: string }[] };
  const ids = new Set(core.reactions.map((r) => r.id));
  const fx = JSON.parse(readFileSync("content/chem/effects.json", "utf8")) as { kinds: string[]; overrides: Record<string, unknown> };
  assert.deepEqual(fx.kinds, ["fire", "water", "frost", "foam", "cloud", "glow", "wall", "reveal"]);
  for (const id of Object.keys(fx.overrides)) assert.ok(ids.has(id), id);
  const q = JSON.parse(readFileSync("content/chem/quests.json", "utf8")) as {
    quests: { id: string; live: boolean; needs: { runs?: string }; line?: string; line_sprout?: string; hint?: string; hint_sprout?: string; done?: string }[];
  };
  // I-057: five live quests, one per world use, each with its lines; one still specified only.
  const live = q.quests.filter((x) => x.live);
  assert.deepEqual(live.map((x) => x.needs.runs).sort(), ["fire", "foam", "frost", "reveal", "wall"]);
  for (const x of live) for (const k of ["line", "line_sprout", "hint", "hint_sprout", "done"] as const) assert.ok(x[k]?.trim(), `${x.id}: ${k}`);
  assert.ok(q.quests.filter((x) => !x.live).length >= 1);
  // Every bottle says where to find it and how the world gives it (never by digging).
  const b = JSON.parse(readFileSync("content/chem/bottles.json", "utf8")) as { items: { item: string; find: string; from: string }[]; stashes: { items: [string, number][] }[] };
  for (const i of b.items) {
    assert.ok(["stash", "clamp", "rig", "pick", "nowhere"].includes(i.from), i.item);
    assert.ok(i.find.trim() && !/\bdig/i.test(i.find), i.item);
  }
  for (const s of b.stashes) for (const [k] of s.items) assert.ok(b.items.some((i) => i.item === k), k);
  const build = readFileSync("crates/bevy-client/build-web.sh", "utf8");
  assert.match(build, /web\/formula-save\.js/);
  assert.match(build, /web\/discover\.js/);
  assert.match(readFileSync("crates/bevy-client/web/boot.js", "utf8"), /discover\.js/);
});

// I-060: the one Bench counts a free-play build of a saved spell, puts it on
// a key, and reads the world's shipped-spell progress, all through
// formula-save.js.
test("the one Bench: builds, keys and the world's progress in the shared save", () => {
  const s = memoryStorage();
  assert.equal(recordBuild(s, "carbon_burns"), -1, "not saved: nothing counted");
  assert.equal(s.data.get(SPELLS_KEY), undefined);
  saveSpell(s, "carbon_burns");
  assert.equal(recordBuild(s, "carbon_burns"), 1);
  assert.equal(recordBuild(s, "carbon_burns"), MASTERY_BUILDS);
  assert.equal(readSpells(s).saved.carbon_burns, 2);
  // A world write later never lowers it (merge keeps the larger count).
  writeSpells(s, { v: 1, saved: { carbon_burns: 0 }, slots: [null, null, null, null], quests: [] });
  assert.equal(readSpells(s).saved.carbon_burns, 2);
  // Keys: one key per spell; key 0 takes it off; unknown spells and keys refused.
  assert.ok(putOnKey(s, "carbon_burns", 3));
  assert.deepEqual(readSpells(s).slots, [null, null, "carbon_burns", null]);
  assert.ok(putOnKey(s, "carbon_burns", 1));
  assert.deepEqual(readSpells(s).slots, ["carbon_burns", null, null, null]);
  assert.ok(putOnKey(s, "carbon_burns", 0));
  assert.deepEqual(readSpells(s).slots, [null, null, null, null]);
  assert.equal(putOnKey(s, "methane_burns", 1), false);
  assert.equal(putOnKey(s, "carbon_burns", 5), false);
  assert.deepEqual(loadCommands(readSpells(s), [])[1], "saved carbon_burns:2");
  // The world's progress line (progress.rs encode), read only.
  assert.equal(PROGRESS_KEY, "rg.progress.v1");
  assert.match(readFileSync("crates/bevy-client/web/progress.js", "utf8"), /STORE_KEY = 'rg\.progress\.v1'/);
  const p = memoryStorage({ [PROGRESS_KEY]: "p1|water:1:10:0:0:0:0|fire:3:40:3:2:20756:1" });
  assert.deepEqual(readProgress(p), { water: { stage: "learning", xp: 10 }, fire: { stage: "mastered", xp: 40 } });
  assert.deepEqual(readProgress(memoryStorage({ [PROGRESS_KEY]: "nonsense" })), {});
  assert.deepEqual(readProgress(throwing), {});
  assert.equal(recordBuild(throwing, "carbon_burns"), -1);
  assert.equal(typeof mountSpells, "function");
});
