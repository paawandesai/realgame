// Procedural sound effects with WebAudio. Every sound is synthesised from
// oscillators and a noise buffer made at start-up: no audio files are ever
// downloaded. Presentation only, driven by the simulation's FX events, and
// scaled by the same sourced enthalpy that drives shake and particles.

import type { CastOutcome, Fx, Tier } from "./types";

export interface SoundSettings {
  muted: boolean;
  /** 0..1 */
  master: number;
  /** 0..1 */
  sfx: number;
}

const KEY = "sound";
/** Never more than this many voices at once, so a big chain reaction can't clip or stutter. */
const MAX_VOICES = 24;
/** Minimum gap between two sounds of the same kind, ms (keeps repeats calm). */
const MIN_GAP_MS: Partial<Record<Fx["kind"], number>> = {
  arc: 180,
  bubble: 90,
  land: 120,
  hiss: 120,
  splash: 60,
  gain: 400,
};

type Win = Window & { __audioDebug?: number; webkitAudioContext?: typeof AudioContext };

export function loadSoundSettings(tier: Tier): SoundSettings {
  // The youngest players start quieter (they often play on speakers or close headphones).
  const defaults: SoundSettings = { muted: false, master: tier === "sprout" ? 0.45 : 0.7, sfx: 0.8 };
  try {
    const raw = window.localStorage.getItem(KEY);
    if (!raw) return defaults;
    const v = JSON.parse(raw) as Partial<SoundSettings>;
    const unit = (n: unknown, d: number) => (typeof n === "number" && n >= 0 && n <= 1 ? n : d);
    return {
      muted: typeof v.muted === "boolean" ? v.muted : defaults.muted,
      master: unit(v.master, defaults.master),
      sfx: unit(v.sfx, defaults.sfx),
    };
  } catch {
    return defaults;
  }
}

function saveSoundSettings(s: SoundSettings): void {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(s));
  } catch {
    // Storage can be blocked (private windows, school profiles); sound still works.
  }
}

export class Sfx {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private bus: GainNode | null = null;
  private noise: AudioBuffer | null = null;
  private active = 0;
  private lastAt = new Map<string, number>();
  settings: SoundSettings;

  constructor(tier: Tier) {
    this.settings = loadSoundSettings(tier);
    const w = window as Win;
    // Test hook: how many voices have started. A number only; nothing personal.
    w.__audioDebug = w.__audioDebug ?? 0;
  }

  /** Call from a user gesture (browsers only allow audio after one). */
  unlock(): void {
    if (this.settings.muted) return;
    if (!this.ctx) {
      const w = window as Win;
      const Ctor = window.AudioContext ?? w.webkitAudioContext;
      if (!Ctor) return;
      try {
        this.ctx = new Ctor();
      } catch {
        return;
      }
      this.master = this.ctx.createGain();
      this.bus = this.ctx.createGain();
      // A gentle compressor keeps stacked booms from clipping.
      const comp = this.ctx.createDynamicsCompressor();
      comp.threshold.value = -14;
      comp.ratio.value = 4;
      this.bus.connect(this.master);
      this.master.connect(comp);
      comp.connect(this.ctx.destination);
      this.noise = this.makeNoise(this.ctx);
      this.applyVolumes();
    }
    if (this.ctx.state === "suspended") void this.ctx.resume().catch(() => undefined);
  }

  set(next: Partial<SoundSettings>): void {
    this.settings = { ...this.settings, ...next };
    saveSoundSettings(this.settings);
    this.applyVolumes();
    if (!this.settings.muted) this.unlock();
  }

  private applyVolumes(): void {
    if (!this.ctx || !this.master || !this.bus) return;
    const t = this.ctx.currentTime;
    this.master.gain.setTargetAtTime(this.settings.muted ? 0 : this.settings.master, t, 0.02);
    this.bus.gain.setTargetAtTime(this.settings.sfx, t, 0.02);
  }

  private makeNoise(ctx: AudioContext): AudioBuffer {
    const len = Math.floor(ctx.sampleRate * 1.5);
    const buf = ctx.createBuffer(1, len, ctx.sampleRate);
    const data = buf.getChannelData(0);
    for (let i = 0; i < len; i++) data[i] = Math.random() * 2 - 1;
    return buf;
  }

  // ------------------------------------------------------------ dispatch

  handle(events: Fx[], outcomes: Map<number, CastOutcome>): void {
    if (this.settings.muted || events.length === 0) return;
    this.unlock();
    if (!this.ctx) return;
    const now = performance.now();
    for (const e of events) {
      const gap = MIN_GAP_MS[e.kind] ?? 0;
      const last = this.lastAt.get(e.kind) ?? -Infinity;
      if (now - last < gap) continue;
      this.lastAt.set(e.kind, now);
      const out = outcomes.get(e.id);
      switch (e.kind) {
        case "charge":
          this.chirp(e.dh_kj);
          break;
        case "bolt":
          this.whoosh();
          break;
        case "impact":
          // Heat releases go bang; water and gas impacts are covered by splash/bubble.
          if (e.dh_kj < 0) this.boom(e.dh_kj, e.power);
          else if (e.dh_kj > 0) this.frost();
          break;
        case "splash":
          this.splash();
          break;
        case "hiss":
          this.hiss();
          break;
        case "melt":
          this.melt();
          break;
        case "frost":
          this.frost();
          break;
        case "bubble":
          this.plinks(out?.spell === "split" ? 6 : 4);
          break;
        case "arc":
          this.zap();
          break;
        case "fizzle":
          this.fizzle();
          break;
        case "unknown":
          this.shimmer();
          break;
        case "discover":
          this.chime();
          break;
        case "door":
          this.rumble();
          break;
        case "land":
          this.thud(e.power);
          break;
        case "gain":
          this.plinks(1);
          break;
        default:
          break;
      }
    }
  }

  // ------------------------------------------------------------- voices

  /** Starts one enveloped voice if there's room (otherwise drops it silently). */
  private voice(
    start: number,
    dur: number,
    peak: number,
    build: (ctx: AudioContext, out: GainNode) => AudioScheduledSourceNode[],
  ): void {
    const ctx = this.ctx;
    if (!ctx || !this.bus || this.active >= MAX_VOICES) return;
    const t0 = ctx.currentTime + start;
    const env = ctx.createGain();
    env.gain.setValueAtTime(0.0001, t0);
    env.gain.exponentialRampToValueAtTime(peak, t0 + Math.min(0.01, dur / 4));
    env.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
    env.connect(this.bus);
    const sources = build(ctx, env);
    this.active++;
    (window as Win).__audioDebug = ((window as Win).__audioDebug ?? 0) + 1;
    let ended = false;
    for (const s of sources) {
      s.start(t0);
      s.stop(t0 + dur + 0.05);
    }
    const first = sources[0];
    if (first) {
      first.onended = () => {
        if (ended) return;
        ended = true;
        this.active--;
        env.disconnect();
      };
    }
  }

  private osc(ctx: AudioContext, type: OscillatorType, f0: number, f1: number, t0: number, dur: number): OscillatorNode {
    const o = ctx.createOscillator();
    o.type = type;
    const at = ctx.currentTime + t0;
    o.frequency.setValueAtTime(f0, at);
    o.frequency.exponentialRampToValueAtTime(Math.max(1, f1), at + dur);
    return o;
  }

  private noiseSrc(ctx: AudioContext): AudioBufferSourceNode {
    const n = ctx.createBufferSource();
    n.buffer = this.noise;
    n.loop = true;
    return n;
  }

  private filter(ctx: AudioContext, type: BiquadFilterType, f0: number, f1: number, t0: number, dur: number, q = 1): BiquadFilterNode {
    const f = ctx.createBiquadFilter();
    f.type = type;
    f.Q.value = q;
    const at = ctx.currentTime + t0;
    f.frequency.setValueAtTime(f0, at);
    f.frequency.exponentialRampToValueAtTime(Math.max(1, f1), at + dur);
    return f;
  }

  /** Rising chirp while the gauntlet charges; endothermic spells chirp downward. */
  private chirp(dh: number): void {
    const up = dh <= 0;
    this.voice(0, 0.14, 0.18, (ctx, out) => {
      const o = this.osc(ctx, "sine", up ? 420 : 900, up ? 1100 : 380, 0, 0.13);
      o.connect(out);
      return [o];
    });
  }

  private whoosh(): void {
    this.voice(0.05, 0.3, 0.16, (ctx, out) => {
      const n = this.noiseSrc(ctx);
      const lp = this.filter(ctx, "lowpass", 2400, 350, 0.05, 0.3, 0.7);
      n.connect(lp).connect(out);
      const s = this.osc(ctx, "sawtooth", 220, 90, 0.05, 0.3);
      const sg = ctx.createGain();
      sg.gain.value = 0.15;
      s.connect(sg).connect(out);
      return [n, s];
    });
  }

  /** Pop for small releases, deep boom for big ones: pitch falls as |ΔH| grows. */
  private boom(dh: number, power: number): void {
    const mag = Math.min(1, Math.abs(dh) / 900);
    const low = 70 - mag * 40; // 2H2 + O2 (-483.6 kJ) ~ 49 Hz, CH4 (-802.3 kJ) ~ 34 Hz
    const dur = 0.35 + mag * 0.6;
    const peak = 0.35 + 0.4 * Math.min(1, power) * (0.5 + mag);
    this.voice(0, dur, Math.min(0.9, peak), (ctx, out) => {
      const n = this.noiseSrc(ctx);
      const lp = this.filter(ctx, "lowpass", 3000, 200, 0, dur, 0.8);
      n.connect(lp).connect(out);
      const s = this.osc(ctx, "sine", low * 2, low * 0.6, 0, dur);
      const sg = ctx.createGain();
      sg.gain.value = 1.2;
      s.connect(sg).connect(out);
      return [n, s];
    });
  }

  /** Endothermic impact: a thin falling shimmer, like frost. */
  private frost(): void {
    this.voice(0, 0.5, 0.12, (ctx, out) => {
      const o = this.osc(ctx, "triangle", 1600, 500, 0, 0.5);
      o.connect(out);
      return [o];
    });
  }

  private splash(): void {
    this.voice(0, 0.28, 0.3, (ctx, out) => {
      const n = this.noiseSrc(ctx);
      const bp = this.filter(ctx, "bandpass", 1400, 500, 0, 0.28, 1.2);
      n.connect(bp).connect(out);
      return [n];
    });
  }

  private hiss(): void {
    this.voice(0.04, 0.6, 0.14, (ctx, out) => {
      const n = this.noiseSrc(ctx);
      const hp = this.filter(ctx, "highpass", 3500, 6000, 0.04, 0.6, 0.7);
      n.connect(hp).connect(out);
      return [n];
    });
  }

  private melt(): void {
    this.voice(0.05, 0.6, 0.14, (ctx, out) => {
      const o = this.osc(ctx, "sine", 900, 260, 0.05, 0.6);
      o.connect(out);
      return [o];
    });
  }

  private plinks(n: number): void {
    for (let i = 0; i < n; i++) {
      const f = 700 + Math.random() * 700;
      this.voice(i * 0.07, 0.09, 0.12, (ctx, out) => {
        const o = this.osc(ctx, "sine", f, f * 1.6, i * 0.07, 0.08);
        o.connect(out);
        return [o];
      });
    }
  }

  private zap(): void {
    this.voice(0, 0.12, 0.06, (ctx, out) => {
      const o = this.osc(ctx, "square", 1800, 600, 0, 0.12);
      o.connect(out);
      return [o];
    });
  }

  private fizzle(): void {
    this.voice(0, 0.32, 0.1, (ctx, out) => {
      const o = this.osc(ctx, "square", 420, 90, 0, 0.32);
      const lp = this.filter(ctx, "lowpass", 1800, 600, 0, 0.32);
      o.connect(lp).connect(out);
      return [o];
    });
  }

  /** Sprout: a soft, low "bonk" for a card that did not help the goal. */
  bonk(): void {
    this.voice(0, 0.2, 0.12, (ctx, out) => {
      const o = this.osc(ctx, "sine", 260, 130, 0, 0.2);
      o.connect(out);
      return [o];
    });
  }

  private shimmer(): void {
    for (let i = 0; i < 3; i++) {
      this.voice(i * 0.06, 0.22, 0.07, (ctx, out) => {
        const o = this.osc(ctx, "triangle", 1200 + i * 240, 1000 + i * 200, i * 0.06, 0.22);
        o.connect(out);
        return [o];
      });
    }
  }

  /** Major arpeggio (C E G C) on a triangle wave. */
  private chime(): void {
    [523.25, 659.25, 783.99, 1046.5].forEach((f, i) => {
      this.voice(i * 0.09, 0.5, 0.16, (ctx, out) => {
        const o = this.osc(ctx, "triangle", f, f, i * 0.09, 0.5);
        o.connect(out);
        return [o];
      });
    });
  }

  private rumble(): void {
    this.voice(0, 1.4, 0.5, (ctx, out) => {
      const n = this.noiseSrc(ctx);
      const lp = this.filter(ctx, "lowpass", 180, 90, 0, 1.4, 1);
      n.connect(lp).connect(out);
      const s = this.osc(ctx, "sine", 46, 34, 0, 1.4);
      s.connect(out);
      return [n, s];
    });
  }

  private thud(speed: number): void {
    this.voice(0, 0.12, Math.min(0.2, 0.04 * speed), (ctx, out) => {
      const o = this.osc(ctx, "sine", 140, 60, 0, 0.12);
      o.connect(out);
      return [o];
    });
  }
}
