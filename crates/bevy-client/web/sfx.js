// Synthesised sound effects (I-053): every sound is made in code with the Web
// Audio API (oscillators, a noise buffer, filters and envelopes); there are no
// samples and no audio files. The palette is warm and soft for young players:
// a gentle low-pass on everything, no harsh highs, and a master limiter with a
// trim so nothing peaks above -6 dBFS.
//
//   import { play, setVolume, names } from './sfx.js';
//   play('water_splash');                 // one-shot
//   const c = play('water_charge');       // a held sound: c.stop() on release
//   play('dig', { material: 'stone' });   // per-material sounds
//   setVolume(0.5);                       // 0..1, clamped
//
// The AudioContext is created lazily on the first user gesture (browsers keep
// audio off until then); `play` before that is a silent no-op. `createSfx`
// builds an independent instance (tests pass a mock context).

/** Peak output, linear: -6 dBFS. */
const TRIM = 0.5;
/** Everything goes through this low-pass: warm, no harsh highs. */
const WARM_HZ = 6000;

/** Materials for `dig` and `step` (an unknown one falls back to the first). */
export const DIG = ['dirt', 'stone', 'sand', 'wood', 'ice'];
export const STEP = ['grass', 'stone', 'sand', 'snow', 'wood'];

/**
 * @typedef {{ stop(): void }} Handle
 * @typedef {{ volume?: number, pitch?: number, material?: string, duration?: number }} Opts
 * @typedef {{ ctx: AudioContext, out: AudioNode, t: number, v: number, p: number, noise: AudioBuffer, o: Opts }} Ctx
 */

/**
 * @param {{ makeContext?: () => AudioContext | null }} [deps]
 */
export function createSfx(deps = {}) {
  const make = deps.makeContext || (() => {
    const g = /** @type {any} */ (globalThis);
    const AC = g.AudioContext || g.webkitAudioContext;
    return AC ? new AC() : null;
  });
  /** @type {AudioContext | null} */
  let ctx = null;
  /** @type {GainNode | null} */
  let master = null;
  /** @type {GainNode | null} */
  let output = null;
  /** @type {AudioBuffer | null} */
  let noise = null;
  /** @type {AudioBuffer | null} */
  let crackle = null;
  let volume = 0.8;

  /** Creates the context and the master chain (call from a user gesture). */
  const unlock = () => {
    if (ctx) {
      if (ctx.state === 'suspended' && ctx.resume) ctx.resume().catch(() => {});
      return true;
    }
    let c = null;
    try { c = make(); } catch (_) { c = null; }
    if (!c) return false;
    ctx = c;
    const m = c.createGain();
    m.gain.value = volume;
    const warm = c.createBiquadFilter();
    warm.type = 'lowpass';
    warm.frequency.value = WARM_HZ;
    warm.Q.value = 0.5;
    const limit = c.createDynamicsCompressor();
    limit.threshold.value = -12;
    limit.knee.value = 0;
    limit.ratio.value = 20;
    limit.attack.value = 0.002;
    limit.release.value = 0.12;
    const out = c.createGain();
    out.gain.value = TRIM;
    m.connect(warm);
    warm.connect(limit);
    limit.connect(out);
    out.connect(c.destination);
    master = m;
    output = out;
    noise = makeNoise(c, 1.5);
    crackle = makeCrackle(c, 2.0);
    return true;
  };

  /**
   * Plays sound `name`. Returns a handle (`stop()` ends a held or looping
   * sound early), or null for an unknown name or before the first gesture.
   * @param {string} name
   * @param {Opts} [o]
   * @returns {Handle | null}
   */
  const play = (name, o = {}) => {
    const fn = SOUNDS[name];
    if (!fn || !ctx || !master || !noise) return null;
    const v = clamp01(o.volume ?? 1);
    const p = Number.isFinite(o.pitch) && /** @type {number} */ (o.pitch) > 0 ? /** @type {number} */ (o.pitch) : 1;
    try {
      return fn({ ctx, out: master, t: ctx.currentTime + 0.005, v, p, noise, o }, crackle) || NOOP;
    } catch (_) {
      return null;
    }
  };

  /** Master volume, clamped to 0..1 (NaN keeps the current value). @param {number} v */
  const setVolume = (v) => {
    if (Number.isFinite(v)) volume = clamp01(v);
    if (master && ctx) master.gain.setTargetAtTime(volume, ctx.currentTime, 0.02);
    return volume;
  };

  return {
    play,
    setVolume,
    unlock,
    names: Object.keys(SOUNDS),
    getVolume: () => volume,
    /** The last node before the speakers (for level meters in dev pages). */
    output: () => output,
    context: () => ctx,
  };
}

/** @type {Handle} */
const NOOP = { stop() {} };

/** @param {number} x */
const clamp01 = (x) => (Number.isFinite(x) ? Math.min(1, Math.max(0, x)) : 0);

/** White noise, `secs` long, for every noisy sound. @param {AudioContext} c @param {number} secs */
function makeNoise(c, secs) {
  const n = Math.floor(c.sampleRate * secs);
  const b = c.createBuffer(1, n, c.sampleRate);
  const d = b.getChannelData(0);
  let seed = 12345;
  for (let i = 0; i < n; i++) {
    seed = (seed * 1103515245 + 12345) >>> 0;
    d[i] = (seed / 4294967296) * 2 - 1;
  }
  return b;
}

/** A looping fire crackle: soft rumble plus sparse decaying pops. @param {AudioContext} c @param {number} secs */
function makeCrackle(c, secs) {
  const sr = c.sampleRate;
  const n = Math.floor(sr * secs);
  const b = c.createBuffer(1, n, sr);
  const d = b.getChannelData(0);
  let seed = 777;
  const rnd = () => { seed = (seed * 1664525 + 1013904223) >>> 0; return seed / 4294967296; };
  let low = 0;
  for (let i = 0; i < n; i++) {
    low += (rnd() * 2 - 1 - low) * 0.02; // rumble: very low-passed noise
    d[i] = low * 0.6;
  }
  const pops = Math.floor(secs * 18);
  for (let k = 0; k < pops; k++) {
    const at = Math.floor(rnd() * (n - sr * 0.03));
    const len = Math.floor(sr * (0.004 + rnd() * 0.02));
    const amp = 0.25 + rnd() * 0.5;
    for (let i = 0; i < len; i++) d[at + i] = (d[at + i] ?? 0) + (rnd() * 2 - 1) * amp * Math.exp(-6 * i / len);
  }
  // Fade the ends together so the loop does not click.
  const f = Math.floor(sr * 0.02);
  for (let i = 0; i < f; i++) { const g = i / f; d[i] = (d[i] ?? 0) * g; d[n - 1 - i] = (d[n - 1 - i] ?? 0) * g; }
  return b;
}

// ---- building blocks ----

/**
 * A gain envelope: 0 -> peak over `a`, then down to silence over `d`.
 * @param {Ctx} c @param {number} a @param {number} d @param {number} peak @param {number} [t0]
 */
function env(c, a, d, peak, t0 = c.t) {
  const g = c.ctx.createGain();
  const pk = Math.max(0.0001, peak * c.v);
  g.gain.setValueAtTime(0.0001, t0);
  g.gain.linearRampToValueAtTime(pk, t0 + a);
  g.gain.exponentialRampToValueAtTime(0.0001, t0 + a + d);
  g.connect(c.out);
  return g;
}

/**
 * An oscillator gliding f0 -> f1 into `dest`.
 * @param {Ctx} c @param {OscillatorType} type @param {number} f0 @param {number} f1
 * @param {number} dur @param {AudioNode} dest @param {number} [t0]
 */
function osc(c, type, f0, f1, dur, dest, t0 = c.t) {
  const o = c.ctx.createOscillator();
  o.type = type;
  o.frequency.setValueAtTime(f0 * c.p, t0);
  o.frequency.exponentialRampToValueAtTime(Math.max(1, f1 * c.p), t0 + dur);
  o.connect(dest);
  o.start(t0);
  o.stop(t0 + dur + 0.05);
  return o;
}

/**
 * Filtered noise into `dest` (the filter sweeps f0 -> f1).
 * @param {Ctx} c @param {BiquadFilterType} type @param {number} f0 @param {number} f1 @param {number} q
 * @param {number} dur @param {AudioNode} dest @param {number} [t0] @param {AudioBuffer} [buf] @param {boolean} [loop]
 */
function noisy(c, type, f0, f1, q, dur, dest, t0 = c.t, buf = c.noise, loop = false) {
  const s = c.ctx.createBufferSource();
  s.buffer = buf;
  s.loop = loop || dur > buf.duration;
  const f = c.ctx.createBiquadFilter();
  f.type = type;
  f.Q.value = q;
  f.frequency.setValueAtTime(f0 * c.p, t0);
  f.frequency.exponentialRampToValueAtTime(Math.max(20, f1 * c.p), t0 + dur);
  s.connect(f);
  f.connect(dest);
  s.start(t0, Math.random() * Math.max(0, buf.duration - 0.5));
  if (Number.isFinite(dur)) s.stop(t0 + dur + 0.05);
  return { s, f };
}

/** A soft bell-like note (sine plus a quiet octave). @param {Ctx} c @param {number} f @param {number} t0 @param {number} dur @param {number} peak */
function bell(c, f, t0, dur, peak) {
  const g = env(c, 0.006, dur, peak, t0);
  osc(c, 'sine', f, f, dur, g, t0);
  const g2 = env(c, 0.004, dur * 0.5, peak * 0.25, t0);
  osc(c, 'sine', f * 2, f * 2, dur * 0.5, g2, t0);
}

/**
 * A sound that holds until stopped: `parts` are started now and given the
 * gain `g`; stop() fades out and stops them.
 * @param {Ctx} c @param {GainNode} g @param {AudioScheduledSourceNode[]} parts
 * @returns {Handle}
 */
function held(c, g, parts) {
  let done = false;
  return {
    stop() {
      if (done) return;
      done = true;
      const now = c.ctx.currentTime;
      g.gain.cancelScheduledValues(now);
      g.gain.setValueAtTime(Math.max(0.0001, g.gain.value), now);
      g.gain.exponentialRampToValueAtTime(0.0001, now + 0.12);
      for (const s of parts) { try { s.stop(now + 0.15); } catch (_) { /* already stopped */ } }
    },
  };
}

// ---- the sounds ----

/** Dig sounds per material: [filter type, from Hz, to Hz, Q, length s, thud Hz, peak]. */
/** @type {Record<string, [BiquadFilterType, number, number, number, number, number, number]>} */
const DIG_SHAPE = {
  dirt: ['lowpass', 900, 300, 0.8, 0.18, 110, 0.7],
  stone: ['bandpass', 1800, 900, 2.5, 0.12, 180, 0.6],
  sand: ['highpass', 1500, 700, 0.6, 0.22, 0, 0.45],
  wood: ['bandpass', 700, 500, 4, 0.14, 220, 0.6],
  ice: ['bandpass', 3000, 1800, 5, 0.12, 0, 0.45],
};
/** Footsteps per material: [filter type, Hz, Q, length s, thud Hz, peak]. */
/** @type {Record<string, [BiquadFilterType, number, number, number, number, number]>} */
const STEP_SHAPE = {
  grass: ['lowpass', 1400, 0.7, 0.1, 0, 0.3],
  stone: ['bandpass', 1100, 1.5, 0.07, 140, 0.32],
  sand: ['highpass', 1800, 0.5, 0.12, 0, 0.22],
  snow: ['lowpass', 2200, 0.6, 0.16, 0, 0.26],
  wood: ['bandpass', 500, 3, 0.09, 160, 0.34],
};

/** @param {Ctx} c @param {string} mat */
function dig(c, mat) {
  const [type, f0, f1, q, len, thud, peak] = DIG_SHAPE[mat] || /** @type {any} */ (DIG_SHAPE.dirt);
  noisy(c, type, f0, f1, q, len, env(c, 0.004, len, peak));
  if (thud) osc(c, 'sine', thud, thud * 0.6, len, env(c, 0.003, len, peak * 0.8));
  if (mat === 'ice') bell(c, 2400, c.t, 0.25, 0.08);
}

/** @param {Ctx} c @param {string} mat */
function step(c, mat) {
  const [type, f, q, len, thud, peak] = STEP_SHAPE[mat] || /** @type {any} */ (STEP_SHAPE.grass);
  noisy(c, type, f, f * 0.7, q, len, env(c, 0.008, len, peak));
  if (thud) osc(c, 'sine', thud, thud * 0.7, len, env(c, 0.004, len, peak * 0.6));
}

/** Every sound: (ctx, crackle buffer) -> optional handle. */
/** @type {Record<string, (c: Ctx, crackle: AudioBuffer | null) => Handle | void>} */
const SOUNDS = {
  // Water charging while the button is held: a rising, bubbling swell.
  water_charge: (c) => {
    const dur = Number.isFinite(c.o.duration) ? /** @type {number} */ (c.o.duration) : 1.5;
    const g = c.ctx.createGain();
    g.gain.setValueAtTime(0.0001, c.t);
    g.gain.linearRampToValueAtTime(0.3 * c.v, c.t + Math.min(0.15, dur));
    g.connect(c.out);
    const o = osc(c, 'triangle', 180, 520, dur, g);
    const n = noisy(c, 'bandpass', 400, 1600, 3, dur, g);
    const lfo = c.ctx.createOscillator();
    const depth = c.ctx.createGain();
    lfo.frequency.value = 9;
    depth.gain.value = 60;
    lfo.connect(depth);
    depth.connect(o.frequency);
    lfo.start(c.t);
    lfo.stop(c.t + dur + 0.05);
    return held(c, g, [o, n.s, lfo]);
  },
  water_splash: (c) => {
    noisy(c, 'lowpass', 2600, 400, 0.7, 0.45, env(c, 0.005, 0.45, 0.75));
    osc(c, 'sine', 320, 120, 0.12, env(c, 0.003, 0.12, 0.35));
    for (const [dt, f] of /** @type {[number, number][]} */ ([[0.06, 900], [0.11, 700], [0.17, 1100]])) {
      osc(c, 'sine', f, f * 1.6, 0.06, env(c, 0.003, 0.06, 0.12, c.t + dt), c.t + dt);
    }
  },
  // Water on fire: a fizzing hiss.
  hiss: (c) => {
    noisy(c, 'highpass', 2500, 3500, 0.7, 0.9, env(c, 0.02, 0.9, 0.35));
    noisy(c, 'bandpass', 1200, 700, 1.2, 0.6, env(c, 0.01, 0.6, 0.25));
  },
  steam: (c) => {
    noisy(c, 'bandpass', 1500, 2800, 0.9, 1.2, env(c, 0.15, 1.05, 0.35));
  },
  flame_whoosh: (c) => {
    noisy(c, 'lowpass', 300, 1800, 0.8, 0.4, env(c, 0.06, 0.34, 0.75));
    osc(c, 'sine', 90, 60, 0.4, env(c, 0.05, 0.35, 0.3));
  },
  // A fire crackling until stopped (default 4 s).
  fire_crackle: (c, crackle) => {
    if (!crackle) return;
    const dur = Number.isFinite(c.o.duration) ? /** @type {number} */ (c.o.duration) : 4;
    const g = c.ctx.createGain();
    g.gain.setValueAtTime(0.0001, c.t);
    g.gain.linearRampToValueAtTime(0.55 * c.v, c.t + 0.3);
    g.gain.setValueAtTime(0.55 * c.v, c.t + Math.max(0.3, dur - 0.4));
    g.gain.exponentialRampToValueAtTime(0.0001, c.t + dur);
    g.connect(c.out);
    const n = noisy(c, 'lowpass', 2200, 1800, 0.6, dur, g, c.t, crackle, true);
    return held(c, g, [n.s]);
  },
  frost_spray: (c) => {
    noisy(c, 'highpass', 3000, 2000, 0.6, 0.6, env(c, 0.02, 0.58, 0.32));
    for (const [dt, f] of /** @type {[number, number][]} */ ([[0.05, 2200], [0.15, 2600], [0.27, 2000]])) bell(c, f, c.t + dt, 0.15, 0.05);
  },
  ice_crack: (c) => {
    noisy(c, 'bandpass', 2400, 900, 3, 0.09, env(c, 0.001, 0.09, 0.8));
    noisy(c, 'bandpass', 1600, 500, 4, 0.2, env(c, 0.002, 0.2, 0.45, c.t + 0.05), c.t + 0.05);
    osc(c, 'sine', 140, 80, 0.25, env(c, 0.002, 0.25, 0.35));
  },
  // Melting ice: a few soft drips.
  ice_drip: (c) => {
    for (const [dt, f] of /** @type {[number, number][]} */ ([[0, 900], [0.18, 1150], [0.42, 800]])) {
      const g = env(c, 0.002, 0.12, 0.3, c.t + dt);
      osc(c, 'sine', f, f * 1.8, 0.1, g, c.t + dt);
    }
  },
  // CO2: a soft heavy puff that sinks (falling filter).
  co2_puff: (c) => {
    noisy(c, 'lowpass', 1400, 250, 0.7, 0.7, env(c, 0.04, 0.66, 0.6));
    osc(c, 'sine', 120, 70, 0.5, env(c, 0.03, 0.47, 0.25));
  },
  // Hydrogen: a squeaky pop.
  h2_pop: (c) => {
    osc(c, 'triangle', 1400, 2600, 0.05, env(c, 0.002, 0.05, 0.35));
    noisy(c, 'bandpass', 1800, 600, 1.5, 0.16, env(c, 0.001, 0.16, 0.8));
    osc(c, 'sine', 160, 60, 0.2, env(c, 0.002, 0.2, 0.5));
  },
  // Methane catching: a low, round whoomp.
  methane_whoomp: (c) => {
    osc(c, 'sine', 70, 40, 0.6, env(c, 0.04, 0.55, 0.8));
    noisy(c, 'lowpass', 200, 900, 0.7, 0.6, env(c, 0.05, 0.55, 0.5));
  },
  glass_ting: (c) => {
    bell(c, 1760, c.t, 0.9, 0.3);
    bell(c, 2637, c.t + 0.01, 0.6, 0.1);
  },
  dig: (c) => dig(c, c.o.material || 'dirt'),
  dig_dirt: (c) => dig(c, 'dirt'),
  dig_stone: (c) => dig(c, 'stone'),
  dig_sand: (c) => dig(c, 'sand'),
  dig_wood: (c) => dig(c, 'wood'),
  dig_ice: (c) => dig(c, 'ice'),
  place_block: (c) => {
    osc(c, 'sine', 200, 120, 0.12, env(c, 0.003, 0.12, 0.6));
    noisy(c, 'lowpass', 1200, 500, 0.8, 0.08, env(c, 0.002, 0.08, 0.35));
  },
  step: (c) => step(c, c.o.material || 'grass'),
  step_grass: (c) => step(c, 'grass'),
  step_stone: (c) => step(c, 'stone'),
  step_sand: (c) => step(c, 'sand'),
  step_snow: (c) => step(c, 'snow'),
  step_wood: (c) => step(c, 'wood'),
  pickup: (c) => {
    bell(c, 660, c.t, 0.18, 0.3);
    bell(c, 990, c.t + 0.08, 0.25, 0.3);
  },
  bench_success: (c) => {
    [523.25, 659.25, 783.99, 1046.5].forEach((f, k) => bell(c, f, c.t + k * 0.09, 0.6, 0.28));
  },
  // Wrong: a soft low "uh-uh", never harsh.
  bench_wrong: (c) => {
    for (const dt of [0, 0.2]) {
      const g = env(c, 0.02, 0.16, 0.35, c.t + dt);
      const f = c.ctx.createBiquadFilter();
      f.type = 'lowpass';
      f.frequency.value = 600;
      f.connect(g);
      osc(c, 'square', 196, 185, 0.18, f, c.t + dt);
    }
  },
  gate_open: (c) => {
    noisy(c, 'lowpass', 300, 120, 1.5, 1.2, env(c, 0.15, 1.0, 0.5));
    osc(c, 'sawtooth', 55, 70, 1.2, (() => { const f = c.ctx.createBiquadFilter(); f.type = 'lowpass'; f.frequency.value = 300; f.connect(env(c, 0.2, 1.0, 0.3)); return f; })());
    bell(c, 392, c.t + 1.0, 0.8, 0.22);
    bell(c, 587.33, c.t + 1.12, 0.9, 0.22);
  },
  // The ending: a short rising fanfare in C major.
  fanfare: (c) => {
    /** @type {[number, number][]} */
    const notes = [[0, 392], [0.16, 523.25], [0.32, 659.25], [0.48, 783.99], [0.72, 1046.5]];
    for (const [dt, f] of notes) {
      const g = env(c, 0.01, dt < 0.7 ? 0.35 : 1.3, 0.22, c.t + dt);
      osc(c, 'triangle', f, f, dt < 0.7 ? 0.35 : 1.3, g, c.t + dt);
    }
    for (const f of [261.63, 329.63, 392]) {
      const g = env(c, 0.05, 1.6, 0.1, c.t + 0.72);
      osc(c, 'sine', f, f, 1.6, g, c.t + 0.72);
    }
  },
};

// The game's shared instance: unlocked on the first user gesture.
const shared = createSfx();
export const play = shared.play;
export const setVolume = shared.setVolume;
export const names = shared.names;
export const unlock = shared.unlock;
const w = /** @type {any} */ (globalThis);
if (w.addEventListener) {
  const first = () => {
    if (shared.unlock()) {
      for (const t of ['pointerdown', 'keydown', 'touchstart']) w.removeEventListener(t, first, true);
    }
  };
  for (const t of ['pointerdown', 'keydown', 'touchstart']) w.addEventListener(t, first, true);
}
