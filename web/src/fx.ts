// The feedback layer: a second canvas over the world for particles, the
// spell bolt, impact flashes, leftover orbs and fizzle hints, plus floating
// labels and screen shake. Presentation only: nothing here feeds back into
// the simulation. Strength comes from the sourced enthalpy on each event.

import { words } from "./device";
import { equationBalls } from "./icons";
import { prettyFormula, type CastOutcome, type Fx, type Tier } from "./types";

const POOL_SIZE = 512;
/** Bolts fly 6 cells per tick in the sim; 60 ticks a second. */
const BOLT_CELLS_PER_MS = (6 * 60) / 1000;
const CHARGE_MS = 150;
const FLASH_MS = 160;
const SHAKE_MS = 280;
/** A bolt is always on screen for at least this long, even at point-blank
 * range, so every cast visibly flies; its impact visuals wait for it. */
const MIN_BOLT_MS = 130;
const WAVE_MS = 520;
const LABEL_MS = 2600;
const LABEL_FIX_MS = 7000;
/** The caption stays at least this far (CSS px) from the impact point. */
const LABEL_GAP_PX = 64;

/** Animation progress in [0, 1]: an effect born after `now` (events handled
 * later in the same frame) has not started yet, rather than running backwards. */
export function progress(now: number, born: number, life: number): number {
  const t = (now - born) / life;
  return t > 0 ? (t < 1 ? t : 1) : 0;
}

/** Canvas radii must never be negative: arc() and createRadialGradient() throw. */
function rad(r: number): number {
  return r > 0 ? r : 0;
}

/** Okabe-Ito palette, so colours stay distinguishable for colour-blind players. */
const COLOR = {
  water: "#56b4e9",
  hydrogen: "#0072b2",
  oxygen: "#d55e00",
  heat: "#e69f00",
  spark: "#f0e442",
  frost: "#a6dcf5",
  grey: "#9aa5b1",
  co2: "#cfd6dd",
  gold: "#f0c75e",
  methane: "#7fe0c0",
} as const;

/** Each gauge's colour, for gather beams and refill streams. */
const JAR_COLOR: Record<string, string> = {
  H2O: COLOR.water,
  H2: COLOR.hydrogen,
  O2: COLOR.oxygen,
  CO2: COLOR.co2,
  CH4: "#7fd18b",
};

const ICONS: Record<string, string> = {
  H2O: "💧",
  H2: "🎈",
  O2: "🫧",
  CO2: "☁️",
  CH4: "💨",
  C: "🪵",
  "⚡": "⚡",
};

interface Particle {
  alive: boolean;
  x: number;
  y: number;
  vx: number;
  vy: number;
  /** Per-ms acceleration in cells; negative rises. */
  g: number;
  born: number;
  life: number;
  size: number;
  color: string;
  /** Drawn as a ring (bubble) instead of a dot. */
  ring: boolean;
}

interface Bolt {
  id: number;
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  born: number;
  /** Flight time on screen, ms. */
  dur: number;
  color: string;
}

/** An expanding shockwave ring, sized by the reaction's |ΔH|. */
interface Wave {
  x: number;
  y: number;
  born: number;
  /** Final radius in cells. */
  r: number;
  color: string;
}

interface Timed {
  x: number;
  y: number;
  born: number;
  life: number;
  color: string;
  radius: number;
}

interface Orb {
  x: number;
  y: number;
  tx: number;
  ty: number;
  born: number;
  text: string;
  /** The orb's label as page text, so it can be read (and tested). */
  el: HTMLElement;
}

interface Ghost {
  x: number;
  y: number;
  /** Nearest source, or null when there is none in the world. */
  sx: number | null;
  sy: number | null;
  icon: string;
  born: number;
}

interface Label {
  el: HTMLElement;
  x: number;
  y: number;
  born: number;
  life: number;
  /** Which side of the impact the caption sits on, chosen once. */
  above: boolean | null;
}

/** The overlay never needs more than this many backing pixels per world cell:
 * sharp enough for icons and text, and it keeps per-frame raster cost flat on
 * big screens. */
const MAX_PX_PER_CELL = 4;
/** Glow is soft, so it is drawn at a low resolution and scaled up smoothly. */
const GLOW_PX_PER_CELL = 2;

export class FxLayer {
  readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  /** Soft light around hot things, under the overlay (hidden when the WebGL
   * renderer draws its own bloom). */
  readonly glowCanvas: HTMLCanvasElement;
  private readonly glowCtx: CanvasRenderingContext2D;
  private glowDrawn = false;
  private overlayDrawn = true;
  private readonly pool: Particle[] = [];
  private bolts: Bolt[] = [];
  private charges: Timed[] = [];
  private flashes: Timed[] = [];
  private waves: Wave[] = [];
  /** Impacts whose bolt is still flying on screen. */
  private pending: { e: Fx; out: CastOutcome | undefined; at: number }[] = [];
  /** Brief full-stage tint after a big exothermic blast. */
  private bloom = { born: -Infinity, alpha: 0, color: "" };
  /** For automated checks: the largest shockwave radius drawn, in CSS px. */
  maxWavePx = 0;
  private orbs: Orb[] = [];
  /** Suction beams while gathering (refreshed every sim tick it pulls). */
  private beams: { x0: number; y0: number; x1: number; y1: number; born: number; color: string }[] = [];
  /** For automated checks: when a gather beam was last drawn. */
  lastBeamAt = -Infinity;
  /** Called for every gauge gain with its jar, amount and cause. */
  onGain: (jar: string, n: number, cause: string) => void = () => {};
  private ghosts: Ghost[] = [];
  private labels: Label[] = [];
  private frostBorn = -Infinity;
  private shakeAmp = 0;
  private shakeBorn = -Infinity;
  private hitStop = 0;
  private viewX = 0;
  private scale = 1;
  private readonly reduced: MediaQueryList;
  /** For automated checks (exposed only with ?debug): largest shake offset applied, in px. */
  maxShakePx = 0;
  lastShakePx = 0;
  /** How far the world layers are raised (px) to keep the player above the
   * bottom HUD. Framing, not motion, so it applies under Calm motion too. */
  liftPx = 0;
  /** HUD popups the cast-result bubble must steer clear of (the Grimoire
   * card, Mote's caption, the objective banner). Hidden ones are skipped. */
  avoid: () => (HTMLElement | null)[] = () => [];

  /** Another world-anchored layer (goal markers) that moves with the world. */
  follow(el: HTMLElement): void {
    this.shaken.push(el);
  }

  constructor(
    private readonly stage: HTMLElement,
    private readonly shaken: HTMLElement[],
    private readonly viewW: number,
    private readonly viewH: number,
    private readonly tier: Tier,
    private readonly onFix: (formula: string) => void,
  ) {
    this.glowCanvas = document.createElement("canvas");
    this.glowCanvas.id = "glow";
    this.glowCanvas.setAttribute("aria-hidden", "true");
    this.glowCanvas.width = viewW * GLOW_PX_PER_CELL;
    this.glowCanvas.height = viewH * GLOW_PX_PER_CELL;
    const after = shaken[shaken.length - 1]?.nextSibling ?? null;
    stage.insertBefore(this.glowCanvas, after);
    shaken.push(this.glowCanvas);
    const gctx = this.glowCanvas.getContext("2d");
    if (!gctx) throw new Error("Canvas 2D is not available");
    this.glowCtx = gctx;
    this.canvas = document.createElement("canvas");
    this.canvas.id = "fx";
    this.canvas.setAttribute("aria-hidden", "true");
    stage.insertBefore(this.canvas, this.glowCanvas.nextSibling);
    shaken.push(this.canvas);
    const ctx = this.canvas.getContext("2d");
    if (!ctx) throw new Error("Canvas 2D is not available");
    this.ctx = ctx;
    for (let i = 0; i < POOL_SIZE; i++) {
      this.pool.push({ alive: false, x: 0, y: 0, vx: 0, vy: 0, g: 0, born: 0, life: 0, size: 1, color: "", ring: false });
    }
    this.reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    const resize = () => this.resize();
    new ResizeObserver(resize).observe(stage);
    resize();
  }

  // ------------------------------------------------------------------ glow

  /** Glowing blocks from the sim's last frame: [x, y, kind, strength] groups. */
  private glow: Float32Array = new Float32Array(0);
  /** Prebaked soft radial sprites, one per glow kind (1 hot, 2 crystal, 3 flame). */
  private readonly glowSprites: (HTMLCanvasElement | null)[] = [
    null,
    bakeGlow("255,110,40", 0.6),
    bakeGlow("226,150,255", 0.6),
    bakeGlow("255,190,96", 0.75),
  ];

  setGlow(entries: Float32Array): void {
    this.glow = entries;
  }

  /** Hydrogen and oxygen in view from the sim: [x, y, kind] groups (1 H2, 2 O2). */
  private gas: Float32Array = new Float32Array(0);
  /** Prebaked glowing bubbles: index 1 hydrogen (light blue), 2 oxygen (orange). */
  private readonly gasSprites: (HTMLCanvasElement | null)[] = [null, bakeBubble("150,210,255"), bakeBubble("255,160,70")];

  setGas(entries: Float32Array): void {
    this.gas = entries;
  }

  /** Each gas cell as a big glowing bubble in its jar's colour, wobbling as
   * it rises, so trapped gas reads at a glance (a cell alone is a faint dot). */
  private drawGas(now: number): void {
    const g = this.gas;
    if (g.length === 0) return;
    const ctx = this.ctx;
    const k = this.scale;
    const calm = this.reducedMotion;
    for (let i = 0; i + 2 < g.length; i += 3) {
      const sprite = this.gasSprites[g[i + 2]!];
      if (!sprite) continue;
      const x = g[i]!;
      const y = g[i + 1]!;
      const seed = (x + this.viewX) * 12.9898 + y * 78.233;
      const wob = calm ? 0 : Math.sin(now / 420 + seed) * 0.6;
      const r = (3.6 + 0.7 * Math.sin(seed)) * k;
      ctx.drawImage(sprite, (x + 0.5 + wob) * k - r, (y + 0.5) * k - r, r * 2, r * 2);
    }
  }

  /** Whether the 2D glow layer is drawn (off when WebGL draws bloom instead). */
  set glowLayer(on: boolean) {
    this.glowCanvas.hidden = !on;
    if (!on) this.glow = new Float32Array(0);
  }

  /** Additive light around flames, crystals and anything above 400 °C, on
   * its own small canvas. */
  private drawGlow(now: number): void {
    const g = this.glow;
    const ctx = this.glowCtx;
    if (g.length === 0) {
      if (this.glowDrawn) ctx.clearRect(0, 0, this.glowCanvas.width, this.glowCanvas.height);
      this.glowDrawn = false;
      return;
    }
    ctx.clearRect(0, 0, this.glowCanvas.width, this.glowCanvas.height);
    this.glowDrawn = true;
    const k = GLOW_PX_PER_CELL;
    ctx.globalCompositeOperation = "lighter";
    const calm = this.reducedMotion;
    for (let i = 0; i + 3 < g.length; i += 4) {
      const kind = g[i + 2]!;
      const sprite = this.glowSprites[kind];
      if (!sprite) continue;
      const strength = g[i + 3]!;
      const x = g[i]!;
      const y = g[i + 1]!;
      const base = kind === 3 ? 12 : kind === 2 ? 7 : 6;
      // Flames breathe a little; never faster than a few times a second.
      const flicker = kind === 3 && !calm ? 0.85 + 0.15 * Math.sin(now / 160 + (x + this.viewX) * 0.7) : 1;
      const r = base * (0.8 + 0.4 * strength) * flicker * k;
      ctx.globalAlpha = Math.min(1, 0.7 + 0.3 * strength);
      ctx.drawImage(sprite, x * k - r, y * k - r, r * 2, r * 2);
    }
    ctx.globalAlpha = 1;
    ctx.globalCompositeOperation = "source-over";
  }

  /** Menu setting; null follows the system's reduced-motion preference. */
  reducedOverride: boolean | null = null;

  get reducedMotion(): boolean {
    return this.reducedOverride ?? this.reduced.matches;
  }

  private resize(): void {
    const r = this.stage.getBoundingClientRect();
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const k = Math.min((r.width * dpr) / this.viewW, MAX_PX_PER_CELL);
    this.canvas.width = Math.max(1, Math.round(this.viewW * k));
    this.canvas.height = Math.max(1, Math.round(this.viewH * k));
    this.scale = this.canvas.width / this.viewW;
  }

  /** Frames the simulation should pause for (hit-stop after a big blast). */
  takeHitStop(): boolean {
    if (this.hitStop > 0) {
      this.hitStop--;
      return true;
    }
    return false;
  }

  // ------------------------------------------------------------ particles

  private spawn(n: number, init: (p: Particle, i: number) => void): void {
    const count = Math.round(this.reducedMotion ? n * 0.3 : n);
    let made = 0;
    for (const p of this.pool) {
      if (made >= count) break;
      if (p.alive) continue;
      p.alive = true;
      p.ring = false;
      p.g = 0;
      init(p, made);
      made++;
    }
  }

  private burst(x: number, y: number, n: number, color: string | ((i: number) => string), speed: number, now: number, g = 0.00004): void {
    this.spawn(n, (p, i) => {
      const a = Math.random() * Math.PI * 2;
      const s = speed * (0.3 + Math.random() * 0.7);
      p.x = x;
      p.y = y;
      p.vx = Math.cos(a) * s;
      p.vy = Math.sin(a) * s - speed * 0.3;
      p.g = g;
      p.born = now;
      p.life = 350 + Math.random() * 450;
      p.size = 0.6 + Math.random() * 0.9;
      p.color = typeof color === "function" ? color(i) : color;
    });
  }

  /** Prebaked soft glows for bolt heads, one per colour. */
  private readonly heads = new Map<string, HTMLCanvasElement | null>();
  private headSprite(hex: string): HTMLCanvasElement | null {
    let s = this.heads.get(hex);
    if (s === undefined) {
      const n = parseInt(hex.slice(1), 16);
      s = bakeGlow(`${(n >> 16) & 255},${(n >> 8) & 255},${n & 255}`, 1);
      this.heads.set(hex, s);
    }
    return s;
  }

  /** Big, slow-fading chunks thrown upward that fall back under gravity. */
  private debris(x: number, y: number, n: number, colors: string[], now: number, speed: number): void {
    this.spawn(n, (p, i) => {
      const a = -Math.PI / 2 + (Math.random() - 0.5) * Math.PI * 1.1;
      const s = speed * (0.5 + Math.random() * 0.8);
      p.x = x;
      p.y = y;
      p.vx = Math.cos(a) * s;
      p.vy = Math.sin(a) * s;
      p.g = 0.00025;
      p.born = now;
      p.life = 700 + Math.random() * 500;
      p.size = 1.4 + Math.random() * 1.4;
      p.color = colors[i % colors.length]!;
    });
  }

  private bubbles(x: number, y: number, n: number, color: (i: number) => string, now: number): void {
    this.spawn(n, (p, i) => {
      p.x = x + (Math.random() - 0.5) * 8;
      p.y = y + (Math.random() - 0.5) * 4;
      p.vx = (Math.random() - 0.5) * 0.01;
      p.vy = -0.01 - Math.random() * 0.02;
      p.g = -0.00001;
      p.born = now + i * 25;
      p.life = 900 + Math.random() * 500;
      p.size = 1 + Math.random() * 0.8;
      p.color = color(i);
      p.ring = true;
    });
  }

  // --------------------------------------------------------------- events

  /** Turns simulation events into visuals. `outcomes` holds this session's casts by id. */
  handle(events: Fx[], outcomes: Map<number, CastOutcome>, now: number): void {
    for (const e of events) {
      const out = outcomes.get(e.id);
      switch (e.kind) {
        case "charge":
          this.charges.push({ x: e.x, y: e.y, born: now, life: CHARGE_MS, color: boltColor(out), radius: 6 });
          break;
        case "bolt": {
          const dist = Math.hypot(e.tx - e.x, e.ty - e.y);
          const dur = Math.max(MIN_BOLT_MS, dist / BOLT_CELLS_PER_MS);
          this.bolts.push({ id: e.id, x0: e.x, y0: e.y, x1: e.tx, y1: e.ty, born: now, dur, color: boltColor(out) });
          break;
        }
        case "impact": {
          // The visuals land when the on-screen bolt does.
          const b = this.bolts.find((x) => x.id === e.id);
          const at = b ? b.born + b.dur : now;
          if (at > now) {
            b!.x1 = e.x;
            b!.y1 = e.y;
            this.pending.push({ e, out, at });
          } else this.impact(e, out, now);
          break;
        }
        case "splash":
          this.burst(e.x, e.y, 10, COLOR.water, 0.04, now, 0.0003);
          break;
        case "hiss":
          this.bubbles(e.x, e.y - 2, 12, () => "#e8eef3", now);
          break;
        case "frost":
          // Water meets the ice wall and can't melt it: a cold white puff.
          this.burst(e.x, e.y, 24, (i) => (i % 3 ? COLOR.frost : "#ffffff"), 0.035, now, -0.00001);
          this.frostBorn = now;
          break;
        case "melt":
          this.burst(e.x, e.y, 14, COLOR.frost, 0.04, now, 0.0001);
          break;
        case "bubble":
          if (out?.spell === "split") {
            // Exactly two hydrogen (blue) for each oxygen (red), like the equation.
            const gas = (out.products.H2 ?? 0) + (out.products.O2 ?? 0);
            this.bubbles(e.x, e.y, Math.min(60, gas * 3), (i) => (i % 3 === 2 ? COLOR.oxygen : COLOR.hydrogen), now);
          } else {
            this.bubbles(e.x, e.y, 12, () => (out?.spell === "cloud" ? COLOR.co2 : COLOR.grey), now);
          }
          break;
        case "seep":
          // Big methane bubbles breaking the marsh surface: where to gather.
          this.spawn(this.reducedMotion ? 1 : 3, (p, i) => {
            p.x = e.x + (Math.random() - 0.5) * 6;
            p.y = e.y + 1;
            p.vx = (Math.random() - 0.5) * 0.004;
            p.vy = -0.012 - Math.random() * 0.01;
            p.g = 0;
            p.born = now + i * 140;
            p.life = 1100 + Math.random() * 400;
            p.size = 1.6 + Math.random() * 1.4;
            p.color = COLOR.methane;
            p.ring = true;
          });
          break;
        case "arc":
          this.flashes.push({ x: e.tx, y: e.ty, born: now, life: 160, color: COLOR.frost, radius: 4 });
          this.bolts.push({ id: -1, x0: e.x, y0: e.y, x1: e.tx, y1: e.ty, born: now - 1000, dur: 1, color: COLOR.frost });
          break;
        case "fizzle":
          this.fizzle(e, out, now);
          break;
        case "unknown":
          this.unknown(e, out, now);
          break;
        case "discover":
          this.burst(e.x, e.y - 6, 16, COLOR.gold, 0.06, now, 0.00003);
          break;
        case "door":
          // The gate dissolves: sparks stream up the whole doorway.
          for (let k = 0; k < 6; k++) {
            this.burst(e.x, e.y - 18 + k * 7, 8, (i) => (i % 2 ? COLOR.gold : "#ffffff"), 0.05, now + k * 40, -0.00003);
          }
          this.flashes.push({ x: e.x, y: e.y, born: now, life: 400, color: COLOR.gold, radius: 22 });
          break;
        case "land":
          this.burst(e.x, e.y, Math.min(10, Math.round(e.power * 2)), "#c9b48a", 0.03, now, 0.0001);
          break;
        case "gather": {
          // A suction beam from what is being pulled to the gauntlet.
          const color = JAR_COLOR[e.reaction_id ?? ""] ?? COLOR.water;
          this.beams = this.beams.filter((b) => now - b.born < 40);
          this.beams.push({ x0: e.x, y0: e.y, x1: e.tx, y1: e.ty, born: now, color });
          this.stream(e.x, e.y, e.tx, e.ty, Math.max(2, Math.round(e.power)), color, now, 260);
          break;
        }
        case "gain": {
          const [cause = "", jar = ""] = (e.reaction_id ?? "").split(":");
          const color = JAR_COLOR[jar] ?? COLOR.grey;
          // Where it came from, streaming into the suit: mist from the air,
          // bubbles from the electrolyser, smoke from a flame.
          this.stream(e.x, e.y, e.tx, e.ty, cause === "split" ? 6 : 8, color, now, 420, cause === "split");
          this.onGain(jar, e.power, cause);
          break;
        }
        default:
          break;
      }
    }
  }

  /** Particles that fly from (x, y) to (tx, ty) over about `ms`. */
  private stream(x: number, y: number, tx: number, ty: number, n: number, color: string, now: number, ms: number, ring = false): void {
    this.spawn(n, (p, i) => {
      const life = ms * (0.8 + Math.random() * 0.4);
      p.x = x + (Math.random() - 0.5) * 4;
      p.y = y + (Math.random() - 0.5) * 4;
      p.vx = (tx - p.x) / life;
      p.vy = (ty - p.y) / life;
      p.born = now + i * 18;
      p.life = life;
      p.size = ring ? 1 : 0.7 + Math.random() * 0.6;
      p.color = color;
      p.ring = ring;
    });
  }

  /** Where the last impact landed, in world cells (for automated checks). */
  lastImpact: [number, number] | null = null;

  private impact(e: Fx, out: CastOutcome | undefined, now: number): void {
    this.lastImpact = [e.x, e.y];
    this.bolts = this.bolts.filter((b) => b.id !== e.id);
    const dh = Math.abs(e.dh_kj);
    const warm = e.dh_kj < 0;
    const cold = e.dh_kj > 0;
    const tint = warm ? COLOR.spark : cold ? COLOR.frost : COLOR.water;
    // One bloom flash (never repeated quickly), bigger for more heat.
    this.flashes.push({ x: e.x, y: e.y, born: now, life: FLASH_MS, color: tint, radius: 10 + Math.min(26, dh / 18) });
    this.flashes.push({ x: e.x, y: e.y, born: now, life: FLASH_MS / 2, color: "#ffffff", radius: 5 + Math.min(8, dh / 60) });
    // A shockwave ring: warm for heat out, frost blue for heat in.
    this.waves.push({ x: e.x, y: e.y, born: now, r: 10 + Math.min(40, dh / 12), color: warm ? COLOR.heat : cold ? COLOR.frost : "#d8f0ff" });
    if (warm && dh > 100 && !this.reducedMotion) this.bloom = { born: now, alpha: Math.min(0.3, dh / 1600), color: "255,214,120" };
    const n = Math.max(12, Math.min(60, Math.round(12 + dh / 10)));
    const palette = warm ? [COLOR.spark, COLOR.heat, COLOR.oxygen] : cold ? [COLOR.frost, "#ffffff"] : [COLOR.water, "#d8f0ff"];
    this.burst(e.x, e.y, n, (i) => palette[i % palette.length]!, 0.05 + Math.min(0.08, dh / 6000), now + 40, warm ? 0.00004 : 0.0003);
    // Ember (or ice-chip) debris: chunky, thrown up and falling back.
    this.debris(e.x, e.y, Math.max(6, Math.min(28, Math.round(6 + dh / 25))), warm ? [COLOR.heat, COLOR.oxygen, COLOR.spark] : cold ? [COLOR.frost, "#ffffff"] : [COLOR.water], now, 0.05 + Math.min(0.06, dh / 8000));
    if (cold) this.frostBorn = now;
    // Shake and hit-stop scale with the heat released or absorbed.
    if (!this.reducedMotion) {
      this.shakeAmp = Math.max(1.5, Math.min(10, dh / 50));
      this.shakeBorn = now;
      if (dh > 400) this.hitStop = 3;
    }
    // Unreacted reagent (whole molecules, "½ O2") drifts back to the player.
    const left = out?.unreacted ?? [];
    left.forEach((text, i) => {
      const label = document.createElement("div");
      label.className = "orb-label";
      label.textContent = text;
      label.hidden = true;
      this.stage.append(label);
      this.orbs.push({ x: e.x + i * 4, y: e.y - 4, tx: e.tx, ty: e.ty, born: now + 200 + i * 120, text, el: label });
    });
    if (out) window.setTimeout(() => this.label(out, e.x, e.y), 100);
  }

  private fizzle(e: Fx, out: CastOutcome | undefined, now: number): void {
    this.burst(e.x, e.y, 14, COLOR.grey, 0.03, now, -0.00002);
    const missing = out?.missing[0] ?? "";
    this.ghosts.push({
      // Beside the puff, so the floating label above it never hides it.
      x: e.x + 14,
      y: e.y - 2,
      sx: e.tx >= 0 ? e.tx : null,
      sy: e.ty >= 0 ? e.ty : null,
      icon: ICONS[missing] ?? "?",
      born: now,
    });
    if (out) this.label(out, e.x, e.y);
  }

  private unknown(e: Fx, out: CastOutcome | undefined, now: number): void {
    // A shimmer puff at the gauntlet and a "not known yet" icon.
    this.burst(e.x, e.y, 16, (i) => (i % 2 ? "#cc79a7" : "#f5f5f5"), 0.03, now, -0.00002);
    this.ghosts.push({ x: e.x + 14, y: e.y - 2, sx: null, sy: null, icon: "?", born: now });
    if (out) this.label(out, e.x, e.y);
  }

  // --------------------------------------------------------------- labels

  private label(out: CastOutcome, x: number, y: number): void {
    // A newer cast's label (such as a fizzle hint) is never covered by an older one.
    if (this.labels.some((l) => Number(l.el.dataset.cast) > out.id)) return;
    const el = document.createElement("div");
    el.className = `fx-label ${out.outcome}`;
    el.dataset.cast = String(out.id);
    if (out.outcome !== "unknown") {
      const eq = document.createElement("div");
      eq.className = "fx-eq";
      // The youngest players see atoms as balls, never subscripts.
      if (this.tier === "sprout") eq.append(equationBalls(out.equation));
      // Apprentice sees state symbols, so ΔH for H₂O(g) and H₂O(l) can't look contradictory.
      else eq.textContent = prettyFormula(out.equation_states ?? out.equation);
      el.append(eq);
    }
    // The sim writes short lines; each gets its own row so nothing runs on.
    for (const line of words(out.label).split("\n")) {
      const text = document.createElement("div");
      text.textContent = this.tier === "sprout" ? line : prettyFormula(line);
      el.append(text);
    }
    const unbalanced = out.imbalance.length > 0;
    // What this cast spent and made, from its own coefficients, and where
    // the products went (the suit's own refills show apart, by the gauges).
    // An unbalanced cast already says what reacted.
    if (this.tier !== "sprout" && !unbalanced && out.ledger && out.outcome !== "fizzle" && out.outcome !== "unknown") {
      const ledger = document.createElement("div");
      ledger.className = "fx-ledger";
      ledger.textContent = prettyFormula(out.ledger);
      el.append(ledger);
      if (out.went) {
        const went = document.createElement("div");
        went.className = "fx-went";
        went.textContent = prettyFormula(out.went);
        el.append(went);
      }
    }
    // ΔH always belongs to an equation it matches: the one cast, or (when
    // only part of it ran) the one that actually reacted.
    if (this.tier === "apprentice" && !unbalanced && out.delta_h_kj !== null && out.delta_h_kj !== 0) {
      const dh = document.createElement("div");
      dh.className = "fx-dh";
      const sign = out.delta_h_kj < 0 ? "−" : "+";
      const same = out.reacted === null || out.reacted === out.equation || out.reacted === out.canonical;
      const what = same ? "" : ` for ${prettyFormula(out.reacted ?? "")}`;
      dh.textContent = `ΔH = ${sign}${Math.abs(out.delta_h_kj).toFixed(1)} kJ${what} (${out.source?.split(",")[0] ?? ""})`;
      el.append(dh);
    }
    let life = LABEL_MS;
    if (out.suggestion) {
      const fix = document.createElement("button");
      fix.type = "button";
      fix.textContent = `Fix it: ${prettyFormula(out.suggestion)}`;
      const s = out.suggestion;
      fix.addEventListener("click", () => this.onFix(s));
      el.append(fix);
      life = LABEL_FIX_MS;
    }
    // One label at a time keeps the scene readable.
    for (const l of this.labels) l.el.remove();
    this.labels = [{ el, x, y, born: performance.now(), life, above: null }];
    this.stage.append(el);
    this.placeLabel(this.labels[0]!);
  }

  private placeLabel(l: Label): void {
    // Keep the whole label on screen, whatever its width, and clear of the
    // impact point: the blast is the show, the caption sits beside it.
    const sw = this.stage.clientWidth;
    const sh = this.stage.clientHeight;
    const lw = l.el.offsetWidth;
    const lh = l.el.offsetHeight;
    const half = lw / 2 + 8;
    const x = ((l.x - this.viewX) / this.viewW) * sw;
    const y = (l.y / this.viewH) * sh - this.liftPx;
    const top = sh * 0.17;
    const bottom = sh * 0.86;
    if (l.above === null) l.above = y - LABEL_GAP_PX - lh >= top || y + LABEL_GAP_PX + lh > bottom;
    let ty = l.above ? y - LABEL_GAP_PX - lh : y + LABEL_GAP_PX;
    ty = Math.max(top, Math.min(bottom - lh, ty));
    let tx = Math.max(half, Math.min(sw - half, x));
    // Still over the impact after clamping? Slide sideways instead.
    const overlapsY = ty < y + LABEL_GAP_PX && ty + lh > y - LABEL_GAP_PX;
    if (overlapsY && Math.abs(tx - x) < lw / 2 + LABEL_GAP_PX) {
      tx = x > sw / 2 ? x - lw / 2 - LABEL_GAP_PX : x + lw / 2 + LABEL_GAP_PX;
      tx = Math.max(half, Math.min(sw - half, tx));
    }
    [tx, ty] = this.clearOfPopups(tx, ty, lw, lh, sw, sh);
    // Never past the stage's edges, whatever the steering above did.
    tx = Math.max(lw / 2 + 4, Math.min(sw - lw / 2 - 4, tx));
    ty = Math.max(4, Math.min(sh - lh - 4, ty));
    l.el.style.left = `${tx}px`;
    l.el.style.top = `${ty}px`;
  }

  /** Moves a label (centre `tx`, top `ty`) off any visible HUD popup: below
   * it, above it, or beside it, whichever first stays on the stage. */
  private clearOfPopups(tx: number, ty: number, lw: number, lh: number, sw: number, sh: number): [number, number] {
    const s = this.stage.getBoundingClientRect();
    const rects: DOMRect[] = [];
    for (const el of this.avoid()) {
      if (!el || el.hidden || el.offsetParent === null) continue;
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) continue;
      rects.push(new DOMRect(r.left - s.left, r.top - s.top, r.width, r.height));
    }
    const pad = 8;
    const hits = (x: number, y: number) =>
      rects.find((r) => x - lw / 2 < r.right + pad && x + lw / 2 > r.left - pad && y < r.bottom + pad && y + lh > r.top - pad);
    const fits = (x: number, y: number) => x - lw / 2 >= 0 && x + lw / 2 <= sw && y >= 0 && y + lh <= sh;
    let hit = hits(tx, ty);
    if (!hit) return [tx, ty];
    // Try each way out of each popup in the way, nearest first.
    for (let tries = 0; hit && tries < 4; tries++) {
      const r = hit;
      const options: [number, number][] = [
        [tx, r.bottom + pad + 1],
        [tx, r.top - pad - lh - 1],
        [r.left - pad - lw / 2 - 1, ty],
        [r.right + pad + lw / 2 + 1, ty],
      ];
      options.sort((a, b) => Math.hypot(a[0] - tx, a[1] - ty) - Math.hypot(b[0] - tx, b[1] - ty));
      const ok = options.find(([x, y]) => fits(x, y) && !hits(x, y));
      if (ok) return ok;
      const next = options.find(([x, y]) => fits(x, y));
      if (!next) break;
      [tx, ty] = next;
      hit = hits(tx, ty);
    }
    return [tx, ty];
  }

  // ----------------------------------------------------------------- draw

  /** Draws one frame. `viewX` is the camera's left edge in world cells. */
  draw(now: number, viewX: number): void {
    this.viewX = viewX;
    const ctx = this.ctx;
    const k = this.scale;
    const sx = (x: number) => (x - viewX) * k;
    const sy = (y: number) => y * k;
    this.drawGlow(now);
    // An idle overlay (nothing in flight, nothing fading) is left alone
    // instead of being cleared and redrawn every frame.
    const busy = this.busy(now);
    if (!busy && !this.overlayDrawn) {
      this.placeLabels(now);
      this.applyShake(now);
      return;
    }
    this.overlayDrawn = busy;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    // Impacts whose bolt has now arrived.
    if (this.pending.length) {
      const due = this.pending.filter((p) => p.at <= now);
      this.pending = this.pending.filter((p) => p.at > now);
      for (const p of due) this.impact(p.e, p.out, now);
    }

    this.drawGas(now);

    // Bloom: a soft warm wash over the whole stage for a big blast.
    const bt = (now - this.bloom.born) / 220;
    if (bt >= 0 && bt < 1) {
      ctx.globalAlpha = this.bloom.alpha * (1 - bt);
      ctx.fillStyle = `rgb(${this.bloom.color})`;
      ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
      ctx.globalAlpha = 1;
    }

    // Charge rings at the gauntlet.
    this.charges = this.charges.filter((c) => now - c.born < c.life);
    for (const c of this.charges) {
      const t = progress(now, c.born, c.life);
      ctx.strokeStyle = c.color;
      ctx.globalAlpha = 1 - t;
      ctx.lineWidth = Math.max(1, k * 0.8);
      ctx.beginPath();
      ctx.arc(sx(c.x), sy(c.y), rad((c.radius * (1 - t) + 1) * k), 0, Math.PI * 2);
      ctx.stroke();
    }

    // Bolts: a glowing head with a long fading trail and sparks shed behind.
    this.bolts = this.bolts.filter((b) => (b.id === -1 ? now - b.born < 1400 : now - b.born < b.dur + 400));
    for (const b of this.bolts) {
      const arc = b.id === -1;
      const dist = Math.hypot(b.x1 - b.x0, b.y1 - b.y0) || 1;
      const t = arc ? 1 : progress(now, b.born, b.dur);
      if (!arc && t >= 1) continue;
      const hx = b.x0 + (b.x1 - b.x0) * t;
      const hy = b.y0 + (b.y1 - b.y0) * t;
      const tail = arc ? 0 : Math.max(0, t - 28 / dist);
      const tx = b.x0 + (b.x1 - b.x0) * tail;
      const ty = b.y0 + (b.y1 - b.y0) * tail;
      const grad = ctx.createLinearGradient(sx(tx), sy(ty), sx(hx), sy(hy));
      grad.addColorStop(0, "rgba(255,255,255,0)");
      grad.addColorStop(1, b.color);
      ctx.globalAlpha = arc ? Math.max(0, 1 - (now - b.born - 1000) / 400) : 1;
      ctx.lineCap = "round";
      if (!arc) {
        // Soft outer glow under the core line.
        ctx.globalCompositeOperation = "lighter";
        ctx.strokeStyle = grad;
        ctx.globalAlpha = 0.35;
        ctx.lineWidth = 6 * k;
        ctx.beginPath();
        ctx.moveTo(sx(tx), sy(ty));
        ctx.lineTo(sx(hx), sy(hy));
        ctx.stroke();
        const sprite = this.headSprite(b.color);
        ctx.globalAlpha = 1;
        if (sprite) ctx.drawImage(sprite, sx(hx) - 7 * k, sy(hy) - 7 * k, 14 * k, 14 * k);
        ctx.globalCompositeOperation = "source-over";
        ctx.globalAlpha = 1;
        if (!this.reducedMotion && Math.random() < 0.7) {
          this.spawn(1, (p) => {
            p.x = hx;
            p.y = hy;
            p.vx = (Math.random() - 0.5) * 0.02;
            p.vy = (Math.random() - 0.5) * 0.02;
            p.born = now;
            p.life = 220 + Math.random() * 180;
            p.size = 0.8 + Math.random() * 0.8;
            p.color = b.color;
          });
        }
      }
      ctx.strokeStyle = grad;
      ctx.lineWidth = (arc ? 2 : 3) * k;
      ctx.beginPath();
      ctx.moveTo(sx(tx), sy(ty));
      ctx.lineTo(sx(hx), sy(hy));
      ctx.stroke();
      ctx.fillStyle = "#ffffff";
      ctx.beginPath();
      ctx.arc(sx(hx), sy(hy), rad((arc ? 1.4 : 2.2) * k), 0, Math.PI * 2);
      ctx.fill();
    }

    // Gather beams: a wobbling glow line from the source to the hand.
    this.beams = this.beams.filter((b) => now - b.born < 140);
    for (const b of this.beams) {
      const t = progress(now, b.born, 140);
      this.lastBeamAt = now;
      ctx.globalCompositeOperation = "lighter";
      ctx.strokeStyle = b.color;
      ctx.lineCap = "round";
      for (const [w, a] of [[5, 0.25], [1.6, 0.9]] as const) {
        ctx.globalAlpha = a * (1 - t * 0.6);
        ctx.lineWidth = w * k;
        ctx.beginPath();
        ctx.moveTo(sx(b.x0), sy(b.y0));
        const mx = (b.x0 + b.x1) / 2;
        const my = (b.y0 + b.y1) / 2 + (this.reducedMotion ? 0 : Math.sin(now / 60) * 2);
        ctx.quadraticCurveTo(sx(mx), sy(my), sx(b.x1), sy(b.y1));
        ctx.stroke();
      }
      ctx.globalCompositeOperation = "source-over";
    }
    ctx.globalAlpha = 1;

    // Shockwaves: a ring that races out and thins as it fades.
    this.waves = this.waves.filter((w) => now - w.born < WAVE_MS);
    const css = this.canvas.clientWidth / this.canvas.width;
    for (const w of this.waves) {
      const t = progress(now, w.born, WAVE_MS);
      const ease = 1 - (1 - t) * (1 - t) * (1 - t);
      const r = rad((2 + (w.r - 2) * ease) * k);
      this.maxWavePx = Math.max(this.maxWavePx, r * css);
      ctx.globalAlpha = 0.9 * (1 - t);
      ctx.strokeStyle = w.color;
      ctx.lineWidth = Math.max(1, (3.5 * (1 - t) + 0.5) * k);
      ctx.beginPath();
      ctx.arc(sx(w.x), sy(w.y), rad(r), 0, Math.PI * 2);
      ctx.stroke();
      ctx.globalAlpha = 0.35 * (1 - t);
      ctx.lineWidth = Math.max(1, 1.2 * k);
      ctx.strokeStyle = "#ffffff";
      ctx.beginPath();
      ctx.arc(sx(w.x), sy(w.y), rad(r * 0.82), 0, Math.PI * 2);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;

    // Impact flashes: a single soft fade.
    this.flashes = this.flashes.filter((f) => now - f.born < f.life);
    ctx.globalCompositeOperation = "lighter";
    for (const f of this.flashes) {
      const t = progress(now, f.born, f.life);
      const r = rad(f.radius * k * (0.6 + 0.4 * t));
      const g = ctx.createRadialGradient(sx(f.x), sy(f.y), 0, sx(f.x), sy(f.y), Math.max(0.001, r));
      g.addColorStop(0, f.color);
      g.addColorStop(1, "rgba(0,0,0,0)");
      ctx.globalAlpha = 0.9 * (1 - t);
      ctx.fillStyle = g;
      ctx.fillRect(sx(f.x) - r, sy(f.y) - r, r * 2, r * 2);
    }
    ctx.globalCompositeOperation = "source-over";

    // Particles.
    for (const p of this.pool) {
      if (!p.alive) continue;
      const age = now - p.born;
      if (age < 0) continue;
      if (age > p.life) {
        p.alive = false;
        continue;
      }
      const x = p.x + p.vx * age;
      const y = p.y + p.vy * age + 0.5 * p.g * age * age;
      ctx.globalAlpha = 1 - age / p.life;
      if (p.ring) {
        ctx.strokeStyle = p.color;
        ctx.lineWidth = Math.max(1, k * 0.4);
        ctx.beginPath();
        ctx.arc(sx(x), sy(y), rad(p.size * k), 0, Math.PI * 2);
        ctx.stroke();
      } else {
        ctx.fillStyle = p.color;
        ctx.fillRect(sx(x) - (p.size * k) / 2, sy(y) - (p.size * k) / 2, p.size * k, p.size * k);
      }
    }

    // Leftover orbs drift back to the player.
    this.orbs = this.orbs.filter((o) => {
      if (now - o.born < 1400) return true;
      o.el.remove();
      return false;
    });
    ctx.font = `bold ${Math.round(4 * k)}px system-ui, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (const o of this.orbs) {
      const t = progress(now, o.born, 1400);
      const e = t * t * (3 - 2 * t);
      const x = o.x + (o.tx - o.x) * e;
      const y = o.y + (o.ty - o.y) * e - Math.sin(t * Math.PI) * 10;
      ctx.globalAlpha = t < 0.85 ? 1 : (1 - t) / 0.15;
      ctx.fillStyle = COLOR.oxygen;
      ctx.beginPath();
      ctx.arc(sx(x), sy(y), rad(3 * k), 0, Math.PI * 2);
      ctx.fill();
      o.el.hidden = now < o.born;
      o.el.style.left = `${sx(x) * css}px`;
      o.el.style.top = `${(sy(y) - 6 * k) * css}px`;
      o.el.style.opacity = String(ctx.globalAlpha);
    }

    // Fizzle hints: a ghost of what was missing and an arrow to where to get it.
    this.ghosts = this.ghosts.filter((g) => now - g.born < 2500);
    for (const g of this.ghosts) {
      const t = progress(now, g.born, 2500);
      ctx.globalAlpha = Math.min(1, (1 - t) * 2) * 0.85;
      const gy = sy(g.y) - Math.sin(now / 200) * k;
      // A badge behind the icon keeps it visible even without an emoji font.
      ctx.fillStyle = "rgba(15,23,32,0.75)";
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = Math.max(1, k * 0.5);
      ctx.beginPath();
      ctx.arc(sx(g.x), gy, rad(7 * k), 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = "#ffffff";
      ctx.font = `bold ${Math.round(8 * k)}px system-ui, sans-serif`;
      ctx.fillText(g.icon, sx(g.x), gy + k * 0.5);
      if (g.sx !== null && g.sy !== null) this.arrow(sx(g.x), sy(g.y) + 6 * k, sx(g.sx), sy(g.sy), k);
    }

    // Frost vignette for heat-absorbing (endothermic) reactions.
    const ft = (now - this.frostBorn) / 700;
    if (ft >= 0 && ft < 1) {
      const w = this.canvas.width;
      const h = this.canvas.height;
      const v = ctx.createRadialGradient(w / 2, h / 2, rad(Math.min(w, h) * 0.35), w / 2, h / 2, rad(Math.max(w, h) * 0.7));
      v.addColorStop(0, "rgba(166,220,245,0)");
      v.addColorStop(1, "rgba(166,220,245,0.55)");
      ctx.globalAlpha = 1 - ft;
      ctx.fillStyle = v;
      ctx.fillRect(0, 0, w, h);
    }
    ctx.globalAlpha = 1;

    this.placeLabels(now);
    this.applyShake(now);
  }

  /** Counts of what is on screen right now (for automated checks). */
  stats(): { particles: number; bolts: number; ghosts: number; flashes: number; labels: number; waves: number } {
    let particles = 0;
    for (const p of this.pool) if (p.alive) particles++;
    return {
      particles,
      waves: this.waves.length,
      bolts: this.bolts.filter((b) => b.id !== -1 && performance.now() - b.born < b.dur).length,
      ghosts: this.ghosts.length,
      flashes: this.flashes.length,
      labels: this.labels.length,
    };
  }

  /** Whether anything is drawn on the overlay canvas this frame. */
  private busy(now: number): boolean {
    if (this.beams.length || this.charges.length || this.bolts.length || this.flashes.length || this.orbs.length || this.ghosts.length || this.waves.length || this.pending.length) {
      return true;
    }
    if (now - this.frostBorn < 700 || now - this.bloom.born < 220) return true;
    if (this.gas.length) return true;
    for (const p of this.pool) if (p.alive) return true;
    return false;
  }

  /** Floating labels follow the world as the camera moves. */
  private placeLabels(now: number): void {
    this.labels = this.labels.filter((l) => {
      const age = now - l.born;
      if (age > l.life) {
        l.el.remove();
        return false;
      }
      this.placeLabel(l);
      l.el.style.opacity = age > l.life - 400 ? String((l.life - age) / 400) : "1";
      return true;
    });
  }

  private arrow(x0: number, y0: number, x1: number, y1: number, k: number): void {
    const ctx = this.ctx;
    const w = this.canvas.width;
    const h = this.canvas.height;
    // Off screen: point along the edge instead of drawing to it.
    const margin = 10 * k;
    const cx = Math.max(margin, Math.min(w - margin, x1));
    const cy = Math.max(margin, Math.min(h - margin, y1));
    const len = Math.hypot(cx - x0, cy - y0);
    if (len < 8 * k) return;
    const ux = (cx - x0) / len;
    const uy = (cy - y0) / len;
    const end = Math.min(len, 60 * k);
    const ex = x0 + ux * end;
    const ey = y0 + uy * end;
    ctx.strokeStyle = COLOR.gold;
    ctx.fillStyle = COLOR.gold;
    ctx.lineWidth = 1.5 * k;
    ctx.setLineDash([3 * k, 3 * k]);
    ctx.beginPath();
    ctx.moveTo(x0, y0);
    ctx.lineTo(ex, ey);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.beginPath();
    ctx.moveTo(ex + ux * 4 * k, ey + uy * 4 * k);
    ctx.lineTo(ex - uy * 3 * k, ey + ux * 3 * k);
    ctx.lineTo(ex + uy * 3 * k, ey - ux * 3 * k);
    ctx.closePath();
    ctx.fill();
  }

  private applyShake(now: number): void {
    const t = now - this.shakeBorn;
    let dx = 0;
    let dy = 0;
    if (!this.reducedMotion && t >= 0 && t < SHAKE_MS) {
      const amp = this.shakeAmp * Math.exp((-4 * t) / SHAKE_MS);
      dx = (Math.random() * 2 - 1) * amp;
      dy = (Math.random() * 2 - 1) * amp;
    }
    this.lastShakePx = Math.max(Math.abs(dx), Math.abs(dy));
    this.maxShakePx = Math.max(this.maxShakePx, this.lastShakePx);
    dy -= this.liftPx;
    const transform = dx === 0 && dy === 0 ? "" : `translate(${dx.toFixed(1)}px, ${dy.toFixed(1)}px)`;
    for (const el of this.shaken) {
      if (el.style.transform !== transform) el.style.transform = transform;
    }
  }
}

function boltColor(out: CastOutcome | undefined): string {
  switch (out?.spell) {
    case "water":
      return COLOR.water;
    case "split":
      return COLOR.frost;
    case "cloud":
    case "gas":
      return COLOR.co2;
    case "boom":
    case "fireball":
    case "burn":
      return COLOR.heat;
    default:
      return COLOR.grey;
  }
}

/** A soft radial light on a small offscreen canvas, drawn scaled each frame. */
function bakeGlow(rgb: string, alpha: number): HTMLCanvasElement | null {
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const g = c.getContext("2d");
  if (!g) return null;
  const grad = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  grad.addColorStop(0, `rgba(${rgb},${alpha})`);
  grad.addColorStop(0.25, `rgba(${rgb},${alpha * 0.7})`);
  grad.addColorStop(0.6, `rgba(${rgb},${alpha * 0.25})`);
  grad.addColorStop(1, `rgba(${rgb},0)`);
  g.fillStyle = grad;
  g.fillRect(0, 0, 64, 64);
  return c;
}

/** A glowing gas bubble: soft halo, a bright rim and a white highlight. */
function bakeBubble(rgb: string): HTMLCanvasElement | null {
  const c = document.createElement("canvas");
  c.width = c.height = 48;
  const g = c.getContext("2d");
  if (!g) return null;
  const halo = g.createRadialGradient(24, 24, 8, 24, 24, 24);
  halo.addColorStop(0, `rgba(${rgb},0.45)`);
  halo.addColorStop(1, `rgba(${rgb},0)`);
  g.fillStyle = halo;
  g.fillRect(0, 0, 48, 48);
  const body = g.createRadialGradient(20, 19, 2, 24, 24, 13);
  body.addColorStop(0, `rgba(${rgb},0.35)`);
  body.addColorStop(1, `rgba(${rgb},0.8)`);
  g.fillStyle = body;
  g.beginPath();
  g.arc(24, 24, 13, 0, Math.PI * 2);
  g.fill();
  g.strokeStyle = `rgba(${rgb},1)`;
  g.lineWidth = 2;
  g.stroke();
  g.fillStyle = "rgba(255,255,255,0.85)";
  g.beginPath();
  g.arc(19, 18, 3.5, 0, Math.PI * 2);
  g.fill();
  return c;
}
