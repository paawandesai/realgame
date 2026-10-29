// The feedback layer: a second canvas over the world for particles, the
// spell bolt, impact flashes, leftover orbs and fizzle hints, plus floating
// labels and screen shake. Presentation only: nothing here feeds back into
// the simulation. Strength comes from the sourced enthalpy on each event.

import { words } from "./device";
import { ATOM_COLOR, icon, type IconName } from "./icons";
import { parseSpell, sideAtoms } from "./tray";
import { prettyFormula, type CastOutcome, type Fx, type Tier } from "./types";

const POOL_SIZE = 512;
/** Live particles at most (at full quality): a burst past it recycles the
 * oldest, so spam never piles up overdraw. */
export const MAX_LIVE_PARTICLES = 160;
/** Gas beads drawn at most per frame (evenly sampled past it). */
const MAX_GAS_BEADS = 140;
/** Gas beads fade out over this many cells under the top of the world, so
 * escaped gas never leaves a band of bubbles along the sky. */
const GAS_CEILING_FADE = 14;
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
  /** When it was last docked (layout reads are throttled). */
  placed?: number;
  /** The opacity last written, so unchanged frames write nothing. */
  shownAlpha?: string;
}

/** The overlay never needs more than this many backing pixels per world cell:
 * sharp enough for icons and text, and it keeps per-frame raster cost flat on
 * big screens. */
const MAX_PX_PER_CELL = 4;
/** On a device too slow for full-size overlays: half the pixels each way. */
const LOW_PX_PER_CELL = 2;
/** Sprout's cast card shows the spell's picture instead of an equation. */
const SPROUT_PICTURE: Partial<Record<CastOutcome["spell"], IconName>> = {
  water: "drop",
  split: "bubbles",
  boom: "boom",
  cloud: "cloud",
  gas: "methane",
  fireball: "fire",
  burn: "fire",
};

/** The player sprite (6 x 12 cells) plus a one-cell outline on every side. */
const SPRITE_BOX_W = 8;
const SPRITE_BOX_H = 14;
/** Drawn size of the player for Sprout (the collision box stays the same). */
const SPROUT_PLAYER_SCALE = 1.5;
/** Dark outline and a light halo around it: one reads on fire, one on night sky. */
export const PLAYER_OUTLINE_RGB: [number, number, number] = [14, 16, 26];
const OUTLINE = `rgb(${PLAYER_OUTLINE_RGB.join(",")})`;
const HALO = "rgba(255, 246, 214, 0.9)";
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
  /** When the overlay was last redrawn. */
  private overlayAt = -Infinity;
  /** When the glow layer was last redrawn. */
  private glowAt = -Infinity;
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

  /** Apprentice: whether a cast card may show the balanced fix yet (the
   * hint ladder's last rung). Other tiers always may. */
  fixAllowed: () => boolean = () => true;

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
    // The player on its own layer above glow, bloom and effects, so fire
    // light never swallows them.
    this.playerCanvas = document.createElement("canvas");
    this.playerCanvas.id = "player-layer";
    this.playerCanvas.setAttribute("aria-hidden", "true");
    stage.insertBefore(this.playerCanvas, this.canvas.nextSibling);
    shaken.push(this.playerCanvas);
    const pctx = this.playerCanvas.getContext("2d");
    if (!pctx) throw new Error("Canvas 2D is not available");
    this.playerCtx = pctx;
    this.spriteCanvas = document.createElement("canvas");
    this.spriteCanvas.width = SPRITE_BOX_W;
    this.spriteCanvas.height = SPRITE_BOX_H;
    this.spriteCtx = this.spriteCanvas.getContext("2d");
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

  /** Gas beads drawn last frame (for automated checks). */
  private gasDrawn = 0;

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
    const beads = Math.floor(g.length / 3);
    // Past the cap, every n-th bead stands for its neighbours.
    const step = beads > MAX_GAS_BEADS ? beads / MAX_GAS_BEADS : 1;
    this.gasDrawn = 0;
    for (let b = 0; b < beads; b += step) {
      const i = Math.floor(b) * 3;
      const sprite = this.gasSprites[g[i + 2]!];
      if (!sprite) continue;
      const x = g[i]!;
      const y = g[i + 1]!;
      const fade = Math.min(1, y / GAS_CEILING_FADE);
      if (fade <= 0.05) continue;
      const seed = (x + this.viewX) * 12.9898 + y * 78.233;
      const wob = calm ? 0 : Math.sin(now / 420 + seed) * 0.6;
      const r = (3.6 + 0.7 * Math.sin(seed)) * k;
      ctx.globalAlpha = fade;
      ctx.drawImage(sprite, (x + 0.5 + wob) * k - r, (y + 0.5) * k - r, r * 2, r * 2);
      this.gasDrawn++;
    }
    ctx.globalAlpha = 1;
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
    // Soft light flickers slowly: about 30 Hz is enough, and halves the
    // canvas commits.
    if (this.glowDrawn && now - this.glowAt < 30) return;
    this.glowAt = now;
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

  // ---------------------------------------------------------------- player

  readonly playerCanvas: HTMLCanvasElement;
  private readonly playerCtx: CanvasRenderingContext2D;
  /** The sprite and its outline at one pixel per cell, scaled up when drawn. */
  private readonly spriteCanvas: HTMLCanvasElement;
  private readonly spriteCtx: CanvasRenderingContext2D | null;
  private spriteKey = "";
  /** Last drawn player box on the player layer, in canvas px (to clear). */
  private playerBox: [number, number, number, number] | null = null;
  /** CSS px per overlay canvas px. */
  private cssPerPx = 1;
  /** For automated checks: the player's drawn box in CSS px relative to the stage. */
  playerScreenBox: { x: number; y: number; w: number; h: number } | null = null;

  /** Draws the player from `WebGame.player_sprite` above every glow layer,
   * with a dark outline and a soft light halo so it reads on fire and on
   * night sky. Sprout sees it 1.5 times bigger, anchored at the feet. The
   * sim's collision box is not touched. */
  drawPlayer(sprite: Int32Array, viewX: number): void {
    const ctx = this.playerCtx;
    const k = this.scale;
    const sc = this.spriteCtx;
    if (!sc || sprite.length < 5) {
      this.playerCanvas.hidden = true;
      return;
    }
    this.playerCanvas.hidden = false;
    let key = "";
    for (let i = 2; i < sprite.length; i++) key += sprite[i]!.toString(36) + ",";
    // The small player canvas is redrawn only when the sprite changes.
    const redraw = key !== this.spriteKey || this.playerBox === null;
    if (key !== this.spriteKey) {
      this.spriteKey = key;
      sc.clearRect(0, 0, SPRITE_BOX_W, SPRITE_BOX_H);
      // Outline first: every empty neighbour (8-way) of a sprite cell.
      sc.fillStyle = OUTLINE;
      for (let i = 2; i + 2 < sprite.length; i += 3) {
        const dx = sprite[i]! + 1;
        const dy = sprite[i + 1]! + 1;
        sc.fillRect(dx - 1, dy - 1, 3, 3);
      }
      for (let i = 2; i + 2 < sprite.length; i += 3) {
        const rgb = sprite[i + 2]!;
        sc.fillStyle = `rgb(${(rgb >> 16) & 255},${(rgb >> 8) & 255},${rgb & 255})`;
        sc.fillRect(sprite[i]! + 1, sprite[i + 1]! + 1, 1, 1);
      }
    }
    const big = this.tier === "sprout" ? SPROUT_PLAYER_SCALE : 1;
    const cell = k * big;
    const w = SPRITE_BOX_W * cell;
    const h = SPRITE_BOX_H * cell;
    // Anchor at the feet, centred on the collision box.
    const footX = (sprite[0]! - viewX + 3) * k;
    const footY = (sprite[1]! + 12) * k;
    const x = Math.round(footX - w / 2);
    const y = Math.round(footY + cell - h);
    // The player has its own canvas just big enough for the sprite, its
    // outline and halo, moved with the CSS translate property (the shake
    // uses transform), so a frame commits a few hundred pixels, not a
    // full-stage layer.
    const pad = Math.ceil(k * 3);
    const bw = Math.ceil(w) + pad * 2;
    const bh = Math.ceil(h) + pad * 2;
    const css = this.cssPerPx;
    if (this.playerCanvas.width !== bw || this.playerCanvas.height !== bh) {
      this.playerCanvas.width = bw;
      this.playerCanvas.height = bh;
      this.playerCanvas.style.width = `${(bw * css).toFixed(2)}px`;
      this.playerCanvas.style.height = `${(bh * css).toFixed(2)}px`;
      this.playerBox = null;
    }
    if (redraw || this.playerBox === null) {
      ctx.clearRect(0, 0, bw, bh);
      ctx.imageSmoothingEnabled = false;
      ctx.save();
      ctx.shadowColor = HALO;
      ctx.shadowBlur = Math.max(2, k * 1.5);
      ctx.drawImage(this.spriteCanvas, pad, pad, w, h);
      ctx.restore();
    }
    this.playerBox = [x - pad, y - pad, bw, bh];
    const at = `${((x - pad) * css).toFixed(1)}px ${((y - pad) * css).toFixed(1)}px`;
    if (this.playerCanvas.style.translate !== at) this.playerCanvas.style.translate = at;
    this.playerScreenBox = { x: x * css, y: y * css, w: w * css, h: h * css };
  }

  private resize(): void {
    const r = this.stage.getBoundingClientRect();
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const k = Math.min((r.width * dpr) / this.viewW, this.pxCap);
    this.canvas.width = Math.max(1, Math.round(this.viewW * k));
    this.canvas.height = Math.max(1, Math.round(this.viewH * k));
    this.scale = this.canvas.width / this.viewW;
    this.cssPerPx = r.width > 0 ? r.width / this.canvas.width : 1;
    // The player canvas is sized on its next draw (to the sprite box).
    this.playerCanvas.width = 0;
    this.playerBox = null;
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
    const cap = this.liveCap;
    const count = Math.min(cap, Math.round((this.reducedMotion ? n * 0.3 : n) * this.quality));
    if (count <= 0) return;
    const free: Particle[] = [];
    const live: Particle[] = [];
    for (const p of this.pool) (p.alive ? live : free).push(p);
    // Past the cap, the oldest particles make way for the new burst.
    const recycle = Math.max(0, live.length + count - cap);
    if (recycle > 0) {
      live.sort((a, b) => a.born - b.born);
      for (let i = 0; i < recycle; i++) live[i]!.alive = false;
    }
    const slots = recycle > 0 ? [...live.slice(0, recycle), ...free] : free;
    for (let i = 0; i < count && i < slots.length; i++) {
      const p = slots[i]!;
      p.alive = true;
      p.ring = false;
      p.g = 0;
      init(p, i);
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
        case "knock": {
          // The blast threw the player back: it lands with the on-screen blast.
          const at = Math.max(now, ...this.pending.map((p) => p.at));
          if (at > now) this.pending.push({ e, out, at });
          else this.knock(e, now);
          break;
        }
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

  /** The player was thrown back by a blast: a hard shake and a dust puff. */
  private knock(e: Fx, now: number): void {
    this.burst(e.x, e.y + 5, 14, (i) => (i % 2 ? "#c9b48a" : COLOR.spark), 0.05, now, 0.0002);
    this.flashes.push({ x: e.x, y: e.y, born: now, life: 180, color: "#ffffff", radius: 6 });
    this.knocks++;
    if (this.reducedMotion) return;
    this.shakeAmp = Math.max(this.shakeAmp * Math.exp((-4 * (now - this.shakeBorn)) / SHAKE_MS), 6 + 6 * e.power);
    this.shakeBorn = now;
  }

  /** The boom safety note was shown on a cast card this visit. */
  private safetyShown = false;

  /** For automated checks: how many knockbacks were shown. */
  knocks = 0;

  /**
   * Share of each particle request actually spawned, 0.3..1. The page lowers
   * it while frames run slow (a rolling frame time over 12 ms) and raises it
   * again when they are fast, so a slow device keeps its frame rate.
   */
  private quality = 1;
  /** Live particles at most, scaled with `quality`. */
  private liveCap = MAX_LIVE_PARTICLES;
  /**
   * `frameMs` is the smoothed work per frame; `p95Ms` (when known) the 95th
   * percentile of recent frame-to-frame times.
   */
  setLoad(frameMs: number, now = performance.now(), p95Ms = 0): void {
    const want = frameMs > 12 ? Math.max(0.3, this.quality - 0.05) : frameMs < 8 ? Math.min(1, this.quality + 0.02) : this.quality;
    this.quality = want;
    this.liveCap = Math.max(16, Math.round(MAX_LIVE_PARTICLES * want));
    // Slow frames (p95 over 25 ms), or still slow with the fewest particles:
    // the overlay and player layers drop to half resolution (a quarter of
    // the pixels to clear, draw and composite every frame). Back to full
    // only after 10 s of fast frames.
    if (this.pxCap === MAX_PX_PER_CELL && (p95Ms > 25 || (want <= 0.3 && frameMs > 12))) {
      this.pxCap = LOW_PX_PER_CELL;
      this.fastSince = -1;
      this.resize();
    } else if (this.pxCap === LOW_PX_PER_CELL) {
      if (frameMs >= 4 || p95Ms > 20) this.fastSince = -1;
      else if (this.fastSince < 0) this.fastSince = now;
      else if (now - this.fastSince > 10000) {
        this.pxCap = MAX_PX_PER_CELL;
        this.resize();
      }
    }
  }
  /** Overlay pixels per cell at most (lowered on slow devices). */
  private pxCap = MAX_PX_PER_CELL;
  private fastSince = -1;

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
    // Seedlings never see half molecules: their orbs are whole atoms ("+1 O").
    const left =
      this.tier === "seedling" && out
        ? Object.entries(out.leftovers)
            .filter(([, n]) => n > 0)
            .map(([el, n]) => `+${n} ${el}`)
        : (out?.unreacted ?? []);
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

  /** A small sparkle puff at a world cell: every Sprout tap answers, even
   * on empty sky. */
  puff(x: number, y: number, now = performance.now()): void {
    this.burst(x, y, 10, (i) => (i % 2 ? "#ffffff" : COLOR.spark), 0.03, now, -0.00002);
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
    // One card element, refilled for each cast instead of rebuilt.
    const el = this.cardEl ?? document.createElement("div");
    this.cardEl = el;
    el.replaceChildren();
    el.removeAttribute("style");
    el.className = `fx-label cast-card ${out.outcome}`;
    el.dataset.cast = String(out.id);
    // Sprout's card is a picture and a few words: no equation row.
    const pic = SPROUT_PICTURE[out.spell];
    if (this.tier === "sprout" && pic && out.outcome !== "unknown") {
      const p = document.createElement("div");
      p.className = "fx-pic";
      p.append(icon(pic));
      el.append(p);
    }
    // Seedling, unbalanced: one short line and a picture scale; the detail
    // (the equation, the counts, what reacted) waits behind "Why?".
    if (this.tier === "seedling" && out.imbalance.length > 0) {
      this.seedlingCard(el, out);
      for (const l of this.labels) if (l.el !== el) l.el.remove();
      this.labels = [{ el, x, y, born: performance.now(), life: LABEL_FIX_MS }];
      if (el.parentNode !== this.stage) this.stage.append(el);
      this.placeLabel(this.labels[0]!);
      return;
    }
    if (out.outcome !== "unknown" && this.tier !== "sprout") {
      const eq = document.createElement("div");
      eq.className = "fx-eq";
      // Apprentice sees state symbols, so ΔH for H₂O(g) and H₂O(l) can't look contradictory.
      eq.textContent = prettyFormula(out.equation_states ?? out.equation);
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
    // The first boom of the visit carries the real-world safety note
    // (Sprout: a "Stand back!" picture instead of words).
    if (out.safety && !this.safetyShown && (out.outcome === "full" || out.outcome === "partial")) {
      this.safetyShown = true;
      if (this.tier === "sprout") {
        const sb = document.createElement("div");
        sb.className = "fx-standback";
        // Pictures only: the boom, and an arrow pointing away from it.
        sb.append(icon("boom"), icon("left"));
        sb.setAttribute("aria-label", "Stand back");
        el.append(sb);
      } else {
        const note = document.createElement("div");
        note.className = "fx-safety";
        note.textContent = `⚠ ${prettyFormula(out.safety)}`;
        el.append(note);
      }
    }
    if (this.tier === "apprentice" && out.note) {
      const note = document.createElement("div");
      note.className = "fx-note";
      note.textContent = out.note;
      el.append(note);
    }
    let life = LABEL_MS;
    if (out.suggestion && (this.tier !== "apprentice" || this.fixAllowed())) {
      const fix = document.createElement("button");
      fix.type = "button";
      // An arrow pointing at nothing gets a spell to use, not a fix.
      // Fix an unbalanced spell; otherwise (a lone gas, a mix) it is a spell to try.
      const verb = out.reason === "no_products" ? "Use it" : out.imbalance.length > 0 ? "Fix it" : "Try";
      fix.textContent = `${verb}: ${prettyFormula(out.suggestion)}`;
      const s = out.suggestion;
      fix.addEventListener("click", () => this.onFix(s));
      el.append(fix);
      life = LABEL_FIX_MS;
    }
    // One label at a time keeps the scene readable.
    for (const l of this.labels) if (l.el !== el) l.el.remove();
    this.labels = [{ el, x, y, born: performance.now(), life }];
    if (el.parentNode !== this.stage) this.stage.append(el);
    this.placeLabel(this.labels[0]!);
  }

  /** The Seedling card for an unbalanced cast: one line, a scale, Why?, Fix it. */
  private seedlingCard(el: HTMLElement, out: CastOutcome): void {
    el.classList.add("seedling-card");
    const [first = "", ...rest] = words(out.label).split("\n");
    const line = document.createElement("div");
    line.className = "fx-line";
    line.textContent = prettyFormula(first);
    el.append(line, miniScale(out.equation));
    const why = document.createElement("div");
    why.className = "fx-why";
    why.hidden = true;
    const eq = document.createElement("div");
    eq.className = "fx-eq";
    eq.textContent = prettyFormula(out.equation);
    why.append(eq);
    for (const r of rest) {
      const d = document.createElement("div");
      d.textContent = prettyFormula(r);
      why.append(d);
    }
    const row = document.createElement("div");
    row.className = "fx-buttons";
    const whyBtn = document.createElement("button");
    whyBtn.type = "button";
    whyBtn.className = "fx-why-btn";
    whyBtn.textContent = "Why?";
    whyBtn.setAttribute("aria-expanded", "false");
    whyBtn.addEventListener("click", () => {
      why.hidden = !why.hidden;
      whyBtn.setAttribute("aria-expanded", String(!why.hidden));
      // Reading takes time: the card stays while the detail is open.
      const l = this.labels.find((x) => x.el === el);
      if (l) l.life = Math.max(l.life, performance.now() - l.born + 12000);
      this.placeLabel(l ?? { el, x: 0, y: 0, born: 0, life: 0 });
    });
    row.append(whyBtn);
    if (out.suggestion) {
      const fix = document.createElement("button");
      fix.type = "button";
      fix.className = "fx-fix";
      fix.textContent = "Fix it";
      fix.setAttribute("aria-label", `Fix it: ${prettyFormula(out.suggestion)}`);
      const s = out.suggestion;
      fix.addEventListener("click", () => this.onFix(s));
      row.append(" ", fix);
    }
    el.append(row, why);
  }

  private placeLabel(l: Label): void {
    l.placed = performance.now();
    this.dockEl(l.el);
  }

  /** The cast card's element, reused from cast to cast. */
  private cardEl: HTMLElement | null = null;

  /** Where the last docked card went, so it stays put while it still fits. */
  private docked = new WeakMap<HTMLElement, { x: number; y: number; top: number }>();

  /**
   * Docks a transient card (the cast card, a "New Grimoire page" card, a
   * room stamp) top-centre under the goal pill, then steps it sideways (or
   * down) off anything it must never cover: goal markers, gates, meters,
   * the pill and Mote's caption. The element is centred on `left`.
   */
  dockEl(el: HTMLElement): void {
    const sw = this.stage.clientWidth;
    const sh = this.stage.clientHeight;
    const lw = el.offsetWidth;
    const lh = el.offsetHeight;
    if (lw === 0 || lh === 0) return;
    const s = this.stage.getBoundingClientRect();
    const top = Math.max(4, Math.round(this.dockTop()));
    const rects: DOMRect[] = [];
    for (const o of this.keepClear()) {
      if (!o || o === el || o.hidden || o.offsetParent === null) continue;
      const r = o.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) continue;
      rects.push(new DOMRect(r.left - s.left, r.top - s.top, r.width, r.height));
    }
    const pad = 8;
    const hits = (x: number, y: number) =>
      rects.some((r) => x - lw / 2 < r.right + pad && x + lw / 2 > r.left - pad && y < r.bottom + pad && y + lh > r.top - pad);
    const fits = (x: number, y: number) => x - lw / 2 >= 4 && x + lw / 2 <= sw - 4 && y >= 4 && y + lh <= sh - 4;
    const last = this.docked.get(el);
    let best: [number, number] | null = null;
    if (last && last.top === top && fits(last.x, last.y) && !hits(last.x, last.y)) best = [last.x, last.y];
    if (!best) {
      const cx = sw / 2;
      const xs = [cx, lw / 2 + 4, sw - lw / 2 - 4];
      const ys = [top];
      for (const r of rects) {
        xs.push(r.left - pad - lw / 2 - 1, r.right + pad + lw / 2 + 1);
        ys.push(r.bottom + pad + 1);
      }
      let score = Infinity;
      for (const x of xs) {
        for (const y of ys) {
          if (y < top || !fits(x, y) || hits(x, y)) continue;
          // Sideways is cheap; every pixel down covers more of the world.
          const d = Math.abs(x - cx) + 3 * (y - top);
          if (d < score) {
            score = d;
            best = [x, y];
          }
        }
      }
    }
    const [x, y] = best ?? [sw / 2, Math.min(top, sh - lh - 4)];
    this.docked.set(el, { x, y, top });
    el.classList.add("docked");
    el.style.left = `${Math.round(x)}px`;
    el.style.top = `${Math.round(y)}px`;
  }

  /** Stage-relative y (px) where docked cards start: under the goal pill. */
  dockTop: () => number = () => 80;
  /** What a docked card must never cover. */
  keepClear: () => (HTMLElement | null)[] = () => [];

  /** Whether a cast card is on screen. */
  get labelUp(): boolean {
    return this.labels.length > 0;
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
    // Only gas beads on the overlay: they wobble slowly, so redrawing (and
    // committing the whole canvas) at about 30 Hz is plenty.
    if (busy && this.overlayDrawn && now - this.overlayAt < 30 && !this.busyBesidesGas(now)) {
      this.placeLabels(now);
      this.applyShake(now);
      return;
    }
    this.overlayAt = now;
    this.overlayDrawn = busy;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    // Impacts whose bolt has now arrived.
    if (this.pending.length) {
      const due = this.pending.filter((p) => p.at <= now);
      this.pending = this.pending.filter((p) => p.at > now);
      for (const p of due) {
        if (p.e.kind === "knock") this.knock(p.e, now);
        else this.impact(p.e, p.out, now);
      }
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
  stats(): {
    particles: number;
    cap: number;
    gas: number;
    bolts: number;
    ghosts: number;
    flashes: number;
    labels: number;
    waves: number;
    quality: number;
    pxPerCell: number;
    knocks: number;
  } {
    let particles = 0;
    for (const p of this.pool) if (p.alive) particles++;
    return {
      particles,
      cap: this.liveCap,
      gas: this.gasDrawn,
      quality: this.quality,
      pxPerCell: this.scale,
      knocks: this.knocks,
      waves: this.waves.length,
      bolts: this.bolts.filter((b) => b.id !== -1 && performance.now() - b.born < b.dur).length,
      ghosts: this.ghosts.length,
      flashes: this.flashes.length,
      labels: this.labels.length,
    };
  }

  /** Whether anything is drawn on the overlay canvas this frame. */
  private busy(now: number): boolean {
    return this.gas.length > 0 || this.busyBesidesGas(now);
  }

  /** Whether anything but the slow gas beads is on the overlay. */
  private busyBesidesGas(now: number): boolean {
    if (this.beams.length || this.charges.length || this.bolts.length || this.flashes.length || this.orbs.length || this.ghosts.length || this.waves.length || this.pending.length) {
      return true;
    }
    if (now - this.frostBorn < 700 || now - this.bloom.born < 220) return true;
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
      // Docking reads layout: a few times a second is enough to follow
      // the HUD, not every frame.
      if (now - (l.placed ?? -Infinity) >= 250) this.placeLabel(l);
      const alpha = age > l.life - 400 ? ((l.life - age) / 400).toFixed(2) : "1";
      if (alpha !== l.shownAlpha) {
        l.shownAlpha = alpha;
        l.el.style.opacity = alpha;
      }
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

/**
 * A picture-only balance scale for a cast card: the atoms before and after
 * as coloured balls on two pans, tipped toward the heavier side.
 */
function miniScale(equation: string): HTMLElement {
  const { left, right } = sideAtoms(parseSpell(equation));
  const count = (a: Record<string, number>) => Object.values(a).reduce((x, y) => x + y, 0);
  const [nl, nr] = [count(left), count(right)];
  const scale = document.createElement("div");
  scale.className = "card-scale";
  scale.setAttribute("role", "img");
  const list = (a: Record<string, number>) =>
    Object.entries(a)
      .map(([e, n]) => `${n} ${e}`)
      .join(", ") || "nothing";
  scale.setAttribute("aria-label", `Before: ${list(left)}. After: ${list(right)}.`);
  const beam = document.createElement("div");
  beam.className = "card-beam";
  const angle = nl === nr ? 4 : Math.max(-12, Math.min(12, (nr - nl) * 3));
  beam.style.transform = `rotate(${angle}deg)`;
  for (const side of [left, right]) {
    const pan = document.createElement("div");
    pan.className = "card-pan";
    for (const [el, n] of Object.entries(side)) {
      for (let i = 0; i < Math.min(n, 10); i++) {
        const b = document.createElement("span");
        b.className = "card-ball";
        b.style.background = ATOM_COLOR[el] ?? "#cc79a7";
        pan.append(b);
      }
    }
    beam.append(pan);
  }
  const post = document.createElement("div");
  post.className = "card-post";
  scale.append(beam, post);
  return scale;
}
