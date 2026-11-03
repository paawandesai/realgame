// Ambience beds per area (submission P13), synthesised with the Web Audio
// API: no recordings. Each area has a steady bed (filtered noise that
// breathes slowly) and occasional details scheduled at random gaps:
//
//   Village        wind and birds
//   Frozen Fields  cold, gusty wind and creaking ice
//   Marsh          bubbling and insects
//   Crystal Cave   drips (with an echo) and a low hum
//   Kiln           the hearth's roar and crackle (the ending, and near it)
//
// It follows `__gameState.biome` (and `ending`), crossfades over 3 s, starts
// on the first key or click in the block world, and is silent when the
// game's Sound setting is off. It sits under the music and the effects.
// `window.__ambience` shows its state for tests.

const STORE_KEY = 'alchemist.settings.v1';
/** Peak level: well under the music (0.18) and the effects. */
const LEVEL = 0.12;
const FADE_S = 3;

/**
 * Per area: the bed (noise filter type, centre Hz, Q, level, how fast and
 * deep it breathes) and the details (name, shortest and longest gap in s).
 * @type {Record<string, { bed: { type: BiquadFilterType, f: number, q: number, level: number, lfo: number, depth: number }, extra?: { type: BiquadFilterType, f: number, q: number, level: number, trem: number }, hum?: number[], details: [string, number, number][] }>}
 */
export const AREAS = {
  village: { bed: { type: 'lowpass', f: 520, q: 0.6, level: 0.55, lfo: 0.09, depth: 0.35 }, details: [['bird', 2.5, 7]] },
  frozen: { bed: { type: 'bandpass', f: 850, q: 0.7, level: 0.75, lfo: 0.16, depth: 0.6 }, details: [['creak', 6, 13]] },
  marsh: {
    bed: { type: 'lowpass', f: 380, q: 0.8, level: 0.35, lfo: 0.07, depth: 0.25 },
    extra: { type: 'bandpass', f: 5200, q: 6, level: 0.12, trem: 28 },
    details: [['bubble', 0.5, 2.2]],
  },
  cave: { bed: { type: 'lowpass', f: 220, q: 0.5, level: 0.25, lfo: 0.05, depth: 0.2 }, hum: [55, 82.5], details: [['drip', 1.2, 3.5]] },
  kiln: { bed: { type: 'lowpass', f: 300, q: 1.0, level: 0.8, lfo: 0.6, depth: 0.25 }, details: [['crackle', 0.08, 0.4]] },
};

/** The next gap (s) before a detail, from `rand` in 0..1. */
export const gap = (/** @type {number} */ lo, /** @type {number} */ hi, /** @type {number} */ rand) => lo + (hi - lo) * rand;

/** The area to play for a game state: the Kiln at the ending, else its biome. */
export function areaFor(/** @type {any} */ s) {
  if (!s) return null;
  if (s.ending) return 'kiln';
  return typeof s.biome === 'string' && AREAS[s.biome] ? s.biome : null;
}

const soundOn = () => {
  try {
    const s = JSON.parse(localStorage.getItem(STORE_KEY) || 'null');
    return !(s && s.sound === false);
  } catch {
    return true;
  }
};

export function startAmbience() {
  const w = /** @type {any} */ (window);
  /** @type {AudioContext | null} */
  let ctx = null;
  /** @type {GainNode | null} */
  let out = null;
  /** @type {AudioBuffer | null} */
  let noise = null;
  /** @type {{ area: string, gain: GainNode, stop: () => void, next: Record<string, number> } | null} */
  let layer = null;
  let want = /** @type {string | null} */ (null);
  const state = { playing: false, area: /** @type {string | null} */ (null), details: 0, muted: !soundOn() };
  w.__ambience = state;

  const ensure = () => {
    if (ctx) return true;
    const AC = w.AudioContext || w.webkitAudioContext;
    if (!AC) return false;
    try { ctx = new AC(); } catch { return false; }
    const c = /** @type {AudioContext} */ (ctx);
    out = c.createGain();
    out.gain.value = LEVEL;
    out.connect(c.destination);
    noise = c.createBuffer(1, c.sampleRate * 2, c.sampleRate);
    const d = noise.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
    return true;
  };

  /** A looping filtered noise into `dest`; returns its stop. */
  const bedNoise = (/** @type {AudioContext} */ c, /** @type {AudioNode} */ dest, type, f, q, level) => {
    const s = c.createBufferSource();
    s.buffer = noise;
    s.loop = true;
    const fl = c.createBiquadFilter();
    fl.type = type;
    fl.frequency.value = f;
    fl.Q.value = q;
    const g = c.createGain();
    g.gain.value = level;
    s.connect(fl);
    fl.connect(g);
    g.connect(dest);
    s.start();
    return { s, g, stop: () => { try { s.stop(); } catch { /* stopped */ } } };
  };

  const startLayer = (/** @type {string} */ area) => {
    const c = /** @type {AudioContext} */ (ctx);
    const a = AREAS[area];
    const gain = c.createGain();
    gain.gain.setValueAtTime(0.0001, c.currentTime);
    gain.gain.linearRampToValueAtTime(1, c.currentTime + FADE_S);
    gain.connect(/** @type {GainNode} */ (out));
    /** @type {(() => void)[]} */
    const stops = [];
    // The bed, breathing with a slow LFO on its level.
    const b = bedNoise(c, gain, a.bed.type, a.bed.f, a.bed.q, a.bed.level);
    const lfo = c.createOscillator();
    lfo.frequency.value = a.bed.lfo;
    const lg = c.createGain();
    lg.gain.value = a.bed.level * a.bed.depth;
    lfo.connect(lg);
    lg.connect(b.g.gain);
    lfo.start();
    stops.push(b.stop, () => { try { lfo.stop(); } catch { /* stopped */ } });
    if (a.extra) {
      // Insects: a high band of noise trembling fast.
      const e = bedNoise(c, gain, a.extra.type, a.extra.f, a.extra.q, a.extra.level);
      const tr = c.createOscillator();
      tr.frequency.value = a.extra.trem;
      const tg = c.createGain();
      tg.gain.value = a.extra.level;
      tr.connect(tg);
      tg.connect(e.g.gain);
      tr.start();
      stops.push(e.stop, () => { try { tr.stop(); } catch { /* stopped */ } });
    }
    for (const f of a.hum || []) {
      const o = c.createOscillator();
      o.frequency.value = f;
      const g = c.createGain();
      g.gain.value = 0.08;
      o.connect(g);
      g.connect(gain);
      o.start();
      stops.push(() => { try { o.stop(); } catch { /* stopped */ } });
    }
    /** @type {Record<string, number>} */
    const next = {};
    for (const [n, lo, hi] of a.details) next[n] = c.currentTime + gap(lo, hi, Math.random());
    return { area, gain, next, stop: () => stops.forEach((s) => s()) };
  };

  // ---- details ----
  const env = (/** @type {AudioContext} */ c, /** @type {AudioNode} */ dest, t, a, d, peak) => {
    const g = c.createGain();
    g.gain.setValueAtTime(0.0001, t);
    g.gain.linearRampToValueAtTime(peak, t + a);
    g.gain.exponentialRampToValueAtTime(0.0001, t + a + d);
    g.connect(dest);
    return g;
  };
  const tone = (c, dest, type, f0, f1, t, dur, peak) => {
    const o = c.createOscillator();
    o.type = type;
    o.frequency.setValueAtTime(f0, t);
    o.frequency.exponentialRampToValueAtTime(Math.max(1, f1), t + dur);
    o.connect(env(c, dest, t, Math.min(0.01, dur / 4), dur, peak));
    o.start(t);
    o.stop(t + dur + 0.05);
  };
  const burst = (c, dest, type, f, q, t, dur, peak) => {
    const s = c.createBufferSource();
    s.buffer = noise;
    const fl = c.createBiquadFilter();
    fl.type = type;
    fl.frequency.value = f;
    fl.Q.value = q;
    s.connect(fl);
    fl.connect(env(c, dest, t, 0.003, dur, peak));
    s.start(t, Math.random());
    s.stop(t + dur + 0.05);
  };
  /** @type {Record<string, (c: AudioContext, dest: AudioNode, t: number) => void>} */
  const DETAILS = {
    // Two or three quick rising chirps.
    bird: (c, d, t) => {
      const n = 2 + Math.floor(Math.random() * 2);
      const f = 2400 + Math.random() * 900;
      for (let k = 0; k < n; k++) tone(c, d, 'sine', f, f * 1.35, t + k * 0.11, 0.07, 0.35);
    },
    // A slow, low groan of ice under strain.
    creak: (c, d, t) => {
      const fl = c.createBiquadFilter();
      fl.type = 'bandpass';
      fl.frequency.value = 420;
      fl.Q.value = 3;
      fl.connect(d);
      tone(c, fl, 'sawtooth', 95, 70, t, 0.6, 0.5);
    },
    bubble: (c, d, t) => tone(c, d, 'sine', 260 + Math.random() * 120, 700, t, 0.07, 0.4),
    // A plink with a short echo.
    drip: (c, d, t) => {
      const f = 1300 + Math.random() * 500;
      tone(c, d, 'sine', f, f * 1.4, t, 0.05, 0.5);
      tone(c, d, 'sine', f, f * 1.4, t + 0.23, 0.05, 0.16);
      tone(c, d, 'sine', f, f * 1.4, t + 0.46, 0.05, 0.06);
    },
    crackle: (c, d, t) => burst(c, d, 'bandpass', 1800 + Math.random() * 1500, 2, t, 0.03, 0.5),
  };

  const schedule = () => {
    if (!ctx || !layer) return;
    const a = AREAS[layer.area];
    for (const [n, lo, hi] of a.details) {
      while (layer.next[n] < ctx.currentTime + 0.3) {
        try { DETAILS[n](ctx, layer.gain, Math.max(ctx.currentTime, layer.next[n])); } catch { /* skip */ }
        state.details++;
        layer.next[n] += gap(lo, hi, Math.random());
      }
    }
  };

  const fadeOut = (/** @type {typeof layer} */ l) => {
    if (!l || !ctx) return;
    const t = ctx.currentTime;
    l.gain.gain.cancelScheduledValues(t);
    l.gain.gain.setValueAtTime(l.gain.gain.value, t);
    l.gain.gain.linearRampToValueAtTime(0.0001, t + FADE_S);
    setTimeout(() => l.stop(), FADE_S * 1000 + 100);
  };

  const sync = () => {
    state.muted = !soundOn();
    const target = state.muted ? null : want;
    if (target === state.area) return;
    if (target && !ensure()) return;
    if (ctx && ctx.state === 'suspended') ctx.resume().catch(() => {});
    fadeOut(layer);
    layer = target ? startLayer(target) : null;
    state.area = target;
    state.playing = !!layer;
  };

  let gesture = false;
  const onGesture = () => { gesture = true; };
  window.addEventListener('keydown', onGesture, { capture: true, passive: true });
  window.addEventListener('pointerdown', onGesture, { capture: true, passive: true });

  let last = 0;
  const tick = (/** @type {number} */ now) => {
    try {
      if (now - last > 250) {
        last = now;
        const raw = w.__gameState;
        const s = typeof raw === 'string' ? JSON.parse(raw) : null;
        if (s && s.mode === 'voxel') {
          want = areaFor(s);
          if (gesture) sync();
        }
      }
      schedule();
    } catch {
      // Ambience must never stop the game.
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}
