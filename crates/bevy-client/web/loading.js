// The loading screen (#loading): real progress for the three parts of a
// start. 0-80 %: the wasm download, from the bytes read so far over the
// response's length (or the last known size when the server compresses
// it). 80-85 %: compiling. 85-100 %: the world, from the game's state
// (`chunksMeshed` of the 18 spawn chunks). It hides once the spawn ring is
// meshed, or 4 s after the first state at the latest.

const RING = 18;
// The wasm's size when the server sends no length (bytes, about right).
const FALLBACK_BYTES = 17_000_000;

const $ = (id) => document.getElementById(id);

function show(pct, step) {
  const el = $('loading');
  if (!el) return;
  const p = Math.max(0, Math.min(100, Math.round(pct)));
  el.setAttribute('aria-valuenow', String(p));
  const fill = $('load-fill');
  if (fill) fill.style.width = `${p}%`;
  const s = $('load-step');
  if (s && step) s.textContent = step;
  // The title screen (title.js) covers this one for players: its Play
  // button, disabled until the game runs, shows the same progress.
  const play = document.querySelector('#title .title-play:disabled');
  if (play) play.textContent = `Loading… ${p}%`;
}

const mb = (n) => (n / 1_048_576).toFixed(1);

/** Fetches the wasm with progress; resolves to its bytes, or null to let
 * the loader fetch it itself. */
export async function loadWasm(url) {
  try {
    show(2, 'Downloading the valley…');
    const res = await fetch(url);
    if (!res.ok || !res.body) return null;
    const enc = res.headers.get('content-encoding');
    const len = Number(res.headers.get('content-length')) || 0;
    const total = len && !enc ? len : FALLBACK_BYTES;
    const reader = res.body.getReader();
    const parts = [];
    let got = 0;
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      parts.push(value);
      got += value.length;
      show(2 + 78 * Math.min(1, got / total), `Downloading the valley… ${mb(got)} of ${mb(Math.max(total, got))} MB`);
    }
    const bytes = new Uint8Array(got);
    let at = 0;
    for (const p of parts) { bytes.set(p, at); at += p.length; }
    window.__wasmBytes = got;
    show(80, 'Starting the engine…');
    return bytes;
  } catch {
    return null;
  }
}

/** Follows the world's generation and meshing until the spawn is ready. */
export function watchWorld() {
  const t0 = performance.now();
  let firstState = null;
  const tick = () => {
    const el = $('loading');
    if (!el) return;
    const raw = window.__gameState;
    let s = null;
    try { s = raw ? JSON.parse(raw) : null; } catch { s = null; }
    if (s && firstState == null) {
      firstState = performance.now();
      // The game is up: never block a click on what is behind.
      el.style.pointerEvents = 'none';
    }
    if (!s) {
      show(80 + Math.min(5, (performance.now() - t0) / 400), 'Building the valley…');
    } else if (s.mode === 'voxel' && typeof s.chunksMeshed === 'number') {
      const n = Math.min(RING, s.chunksMeshed);
      show(85 + 15 * (n / RING), `Shaping the ground… ${n} of ${RING}`);
      if (n >= RING || s.meshQ === 0 || performance.now() - firstState > 4000) {
        el.classList.add('hidden');
        return;
      }
    } else {
      el.classList.add('hidden');
      return;
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}
