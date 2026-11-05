// Drives bench-scene.js with fixed molecules so the graphics work can be
// captured before and after a change, without the chemistry wasm.
// ?step=plate | reaction ; the page sets window.__ready when posed.
import { createBenchScene } from '/crates/bevy-client/web/bench-scene.js';

const names = await (await fetch('/content/chem/names.json')).json();
const EL = {};
for (const [k, x] of Object.entries(names.elements)) EL[k] = { v: x.bonds, c: parseInt(x.color.slice(1), 16), r: x.radius, name: x.name };
const q = new URLSearchParams(location.search);
const s = createBenchScene(document.getElementById('scene'), { elements: EL });
// Plate: ethanol C2H5OH.
const els = ['C', 'C', 'O', 'H', 'H', 'H', 'H', 'H', 'H'];
els.forEach((e) => s.addPlateAtom(e));
s.setPlateBonds([
  { a: 0, b: 1, o: 1 }, { a: 1, b: 2, o: 1 }, { a: 0, b: 3, o: 1 }, { a: 0, b: 4, o: 1 }, { a: 0, b: 5, o: 1 },
  { a: 1, b: 6, o: 1 }, { a: 1, b: 7, o: 1 }, { a: 2, b: 8, o: 1 },
], true);
const ch4 = { els: ['C', 'H', 'H', 'H', 'H'], bonds: [1, 2, 3, 4].map((b) => ({ a: 0, b, o: 1 })) };
const o2 = { els: ['O', 'O'], bonds: [{ a: 0, b: 1, o: 2 }] };
const co2 = { els: ['C', 'O', 'O'], bonds: [{ a: 0, b: 1, o: 2 }, { a: 0, b: 2, o: 2 }] };
const h2o = { els: ['O', 'H', 'H'], bonds: [{ a: 0, b: 1, o: 1 }, { a: 0, b: 2, o: 1 }] };
if (q.get('step') === 'reaction') s.playReaction([ch4, o2, o2], [co2, h2o, h2o], 'flame');
else s.playReaction([ch4, o2, o2], [ch4, o2, o2], 'fizzle');
setTimeout(() => { window.__ready = true; }, Number(q.get('wait') || 3000));
