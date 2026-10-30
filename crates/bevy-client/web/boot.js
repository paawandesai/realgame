// Loaded as an external module so the page works under script-src 'self'.
import init from './bevy-client.js';
import { startHud } from './hud.js';

window.__bootStart = performance.now();

// CPU time of each frame: wrap requestAnimationFrame callbacks (Bevy runs
// its whole update and render inside one). Logged as a CPUFRAME line.
const cpu = [];
window.__cpuFrames = cpu;
const raf = window.requestAnimationFrame.bind(window);
window.requestAnimationFrame = (cb) => raf((t) => {
  const t0 = performance.now();
  try { cb(t); } finally {
    const d = performance.now() - t0;
    if (d > 0.3) { cpu.push(d); if (cpu.length > 600) cpu.shift(); }
  }
});
setInterval(() => {
  if (cpu.length < 10) return;
  const s = [...cpu].sort((a, b) => a - b);
  const q = (p) => s[Math.min(s.length - 1, Math.round((s.length - 1) * p))].toFixed(2);
  console.log(`CPUFRAME n=${s.length} p50_ms=${q(0.5)} p95_ms=${q(0.95)} max_ms=${q(1)}`);
  cpu.length = 0;
}, 2000);

startHud();
init().catch((e) => {
  // Bevy/winit throws a control-flow exception on purpose to hand the
  // event loop to the browser; anything else is a real error.
  if (!String(e).includes('Using exceptions for control flow')) console.error(e);
});
