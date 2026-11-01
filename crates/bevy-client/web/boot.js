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
// Spell progress saved on this device (I-027). Dynamic, so a missing or
// broken progress.js costs only saving, never the boot.
try { import('./progress.js').then((m) => m.startProgress()).catch(() => {}); } catch {}
try { import('./bench.js').then((m) => m.startBench()).catch(() => {}); } catch {}
try { import('./worked.js').then((m) => m.startWorked()).catch(() => {}); } catch {}
// Anonymous analytics and the age picker (docs/ANALYTICS.md), only where the
// page's rg-analytics switch is "on" (the built site, web/dist). Loaded on
// demand, so web/pkg-out (switch "off") never fetches them, and wrapped so a
// failure here can never stop the game from booting. init() below is not
// gated on it: the WASM keeps downloading behind the picker.
try {
  if (document.querySelector('meta[name="rg-analytics"]')?.content === 'on') {
    import('./age.js').then((m) => m.startAgePicker()).catch(() => {});
  }
} catch {
  // Analytics must never break the game.
}
init().catch((e) => {
  // Bevy/winit throws a control-flow exception on purpose to hand the
  // event loop to the browser; anything else is a real error.
  if (!String(e).includes('Using exceptions for control flow')) console.error(e);
});
