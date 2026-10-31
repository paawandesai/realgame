import { World } from '@iwsdk/core';
import projectOptions from 'virtual:iwsdk-project';
import { BenchSystem } from './bench.js';
import { loadChemistry } from './chem.js';
import { PanelSystem } from './panel.js';

await loadChemistry();

World.create(document.getElementById('scene-container') as HTMLDivElement, projectOptions).then((world) => {
  world.registerSystem(PanelSystem);
  world.registerSystem(BenchSystem);
  (window as unknown as { bench: unknown }).bench = world.getSystem(BenchSystem);
});
