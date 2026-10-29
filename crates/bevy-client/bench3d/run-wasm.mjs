// node run-wasm.mjs <path to voxel_bench3d.wasm>
import { readFileSync } from 'node:fs';
const { instance } = await WebAssembly.instantiate(readFileSync(process.argv[2]), {});
for (const chunks of [1, 8, 32]) {
  instance.exports.bench(chunks, 2);
  const steps = 200;
  const t = performance.now();
  instance.exports.bench(chunks, steps);
  const ms = (performance.now() - t) / steps;
  console.log(`wasm: ${String(chunks).padStart(2)} chunks: ${ms.toFixed(3)} ms/step, ${(chunks / ms).toFixed(1)} chunk-steps per ms`);
}
