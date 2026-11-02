// Music for the block valley (I-052, Lane C, spec section 8), generated in
// code with the Web Audio API: no audio files. A slow sequencer plays a calm
// pentatonic motif per biome over a soft pad and crossfades over 2 s when the
// biome changes. It starts on the first key or click in the block world, and
// stays silent when the game's Sound setting is off. `window.__music` shows
// its state (for the tests and the pause card).

/** The game's settings key (hud.js). */
const STORE_KEY = 'alchemist.settings.v1';
/** Peak level before the speakers: quieter than the effects. */
const LEVEL = 0.18;
const FADE_S = 2;

/**
 * Per biome: tempo (beats a minute), note length (beats), the motif as
 * semitones over the root (pentatonic), the root (Hz), the voice and the pad.
 * @type {Record<string, { bpm: number, step: number, motif: number[], root: number, voice: OscillatorType, pad: number[], gap: number }>}
 */
const THEMES = {
  // Village: plucked major pentatonic, gentle and walking.
  village: { bpm: 84, step: 1, motif: [0, 4, 7, 9, 7, 4, 2, 4, 0, 2, 4, 7], root: 293.66, voice: 'triangle', pad: [0, 7, 16], gap: 0.12 },
  // Frozen Fields: slow bell tones, high and sparse.
  frozen: { bpm: 60, step: 2, motif: [12, 7, 9, 4, 7, 2], root: 392, voice: 'sine', pad: [0, 7, 14], gap: 0.3 },
  // Marsh: a low drone and a lazy minor pentatonic.
  marsh: { bpm: 66, step: 1.5, motif: [0, 3, 5, 7, 5, 3, 0, -2], root: 196, voice: 'triangle', pad: [-12, 0, 7], gap: 0.25 },
  // Crystal Cave: sparse, echoing notes.
  cave: { bpm: 52, step: 3, motif: [0, 7, 12, 10, 7], root: 261.63, voice: 'sine', pad: [-12, -5], gap: 0.5 },
  // Kiln: a warm full chord (the ending).
  kiln: { bpm: 72, step: 2, motif: [0, 4, 7, 12, 7, 4], root: 261.63, voice: 'triangle', pad: [0, 4, 7, 12], gap: 0.1 },
};

const soundOn = () => {
  try {
    const s = JSON.parse(localStorage.getItem(STORE_KEY) || 'null');
    return !(s && s.sound === false);
  } catch {
    return true;
  }
};

export function startMusic() {
  const w = /** @type {any} */ (window);
  /** @type {AudioContext | null} */
  let ctx = null;
  /** @type {GainNode | null} */
  let out = null;
  /** @type {{ gain: GainNode, pad: OscillatorNode[], biome: string } | null} */
  let layer = null;
  let biome = 'village';
  let next = 0;
  let beat = 0;
  const state = { playing: false, biome, started: false, muted: !soundOn() };
  w.__music = state;

  const ensure = () => {
    if (ctx) return true;
    const AC = w.AudioContext || w.webkitAudioContext;
    if (!AC) return false;
    ctx = new AC();
    out = ctx.createGain();
    out.gain.value = LEVEL;
    const warm = ctx.createBiquadFilter();
    warm.type = 'lowpass';
    warm.frequency.value = 3500;
    out.connect(warm);
    warm.connect(ctx.destination);
    return true;
  };

  /** A soft held pad for `id`, faded in. */
  const startLayer = (/** @type {string} */ id) => {
    if (!ctx || !out) return null;
    const th = THEMES[id] || THEMES.village;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.0001, ctx.currentTime);
    gain.gain.linearRampToValueAtTime(1, ctx.currentTime + FADE_S);
    gain.connect(out);
    const padGain = ctx.createGain();
    padGain.gain.value = 0.12;
    padGain.connect(gain);
    const pad = th.pad.map((st) => {
      const o = /** @type {AudioContext} */ (ctx).createOscillator();
      o.type = 'sine';
      o.frequency.value = (th.root / 2) * 2 ** (st / 12);
      o.connect(padGain);
      o.start();
      return o;
    });
    return { gain, pad, biome: id };
  };

  const stopLayer = (/** @type {{ gain: GainNode, pad: OscillatorNode[] } | null} */ l) => {
    if (!l || !ctx) return;
    const t = ctx.currentTime;
    l.gain.gain.cancelScheduledValues(t);
    l.gain.gain.setValueAtTime(Math.max(0.0001, l.gain.gain.value), t);
    l.gain.gain.linearRampToValueAtTime(0.0001, t + FADE_S);
    for (const o of l.pad) o.stop(t + FADE_S + 0.1);
  };

  /** One motif note at `t`. */
  const note = (/** @type {number} */ t, /** @type {number} */ f, /** @type {number} */ len, /** @type {OscillatorType} */ voice) => {
    if (!ctx || !layer) return;
    const o = ctx.createOscillator();
    const g = ctx.createGain();
    o.type = voice;
    o.frequency.value = f;
    g.gain.setValueAtTime(0.0001, t);
    g.gain.linearRampToValueAtTime(0.5, t + 0.02);
    g.gain.exponentialRampToValueAtTime(0.0001, t + len);
    o.connect(g);
    g.connect(layer.gain);
    o.start(t);
    o.stop(t + len + 0.05);
  };

  const schedule = () => {
    if (!ctx || !layer || !state.playing) return;
    const th = THEMES[layer.biome] || THEMES.village;
    const spb = 60 / th.bpm;
    while (next < ctx.currentTime + 0.4) {
      const st = th.motif[beat % th.motif.length];
      // A rest now and then, so the motif breathes.
      if (beat % (th.motif.length + 2) < th.motif.length && st !== undefined) {
        note(next, th.root * 2 ** (st / 12), spb * th.step * (1 - th.gap) * 1.6, th.voice);
      }
      beat++;
      next += spb * th.step;
    }
  };

  const start = () => {
    if (state.playing) return;
    state.muted = !soundOn();
    if (state.muted || !ensure() || !ctx) return;
    if (ctx.state === 'suspended') ctx.resume().catch(() => {});
    layer = startLayer(biome);
    next = ctx.currentTime + 0.1;
    beat = 0;
    state.playing = true;
    state.started = true;
  };

  const stop = () => {
    if (!state.playing) return;
    state.playing = false;
    stopLayer(layer);
    layer = null;
  };

  /** Crossfades to biome `id`'s theme. */
  const setBiome = (/** @type {string} */ id) => {
    if (!THEMES[id] || id === biome) return;
    biome = id;
    state.biome = id;
    if (!state.playing || !ctx) return;
    stopLayer(layer);
    layer = startLayer(id);
    beat = 0;
    next = ctx.currentTime + 0.2;
  };

  const voxel = () => {
    try { return JSON.parse(w.__gameState || 'null')?.mode === 'voxel'; } catch { return false; }
  };
  const first = () => { if (voxel()) start(); };
  window.addEventListener('pointerdown', first, true);
  window.addEventListener('keydown', first, true);

  setInterval(() => {
    let s = null;
    try { s = JSON.parse(w.__gameState || 'null'); } catch { s = null; }
    if (s && s.mode === 'voxel' && typeof s.biome === 'string') setBiome(s.biome);
    const off = !soundOn();
    if (off && state.playing) stop();
    state.muted = off;
    schedule();
  }, 100);

  return { start, stop, setBiome };
}
