// The feedback layer: a second canvas over the world for particles, the
// spell bolt, impact flashes, leftover orbs and fizzle hints, plus floating
// labels and screen shake. Presentation only: nothing here feeds back into
// the simulation. Strength comes from the sourced enthalpy on each event.

import { prettyFormula, type CastOutcome, type Fx, type Goal, type Tier } from "./types";

const POOL_SIZE = 512;
/** Bolts fly 6 cells per tick in the sim; 60 ticks a second. */
const BOLT_CELLS_PER_MS = (6 * 60) / 1000;
const CHARGE_MS = 150;
const FLASH_MS = 80;
const SHAKE_MS = 120;
const LABEL_MS = 2600;
const LABEL_FIX_MS = 7000;

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
} as const;

/** Goal markers: an emoji on a badge (the badge keeps it visible without an emoji font). */
const GOAL_ICONS: Record<Goal["kind"], string> = {
  fire: "🔥",
  ice: "🧊",
  gas: "🫧",
  brazier: "🏮",
};

const ICONS: Record<string, string> = {
  H2O: "💧",
  H2: "🎈",
  O2: "🫧",
  CO2: "☁️",
  CH4: "💨",
  C: "🪵",
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
}

export class FxLayer {
  readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly pool: Particle[] = [];
  private bolts: Bolt[] = [];
  private charges: Timed[] = [];
  private flashes: Timed[] = [];
  private orbs: Orb[] = [];
  private ghosts: Ghost[] = [];
  private labels: Label[] = [];
  private goals: Goal[] = [];
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

  constructor(
    private readonly stage: HTMLElement,
    private readonly shaken: HTMLElement[],
    private readonly viewW: number,
    private readonly viewH: number,
    private readonly tier: Tier,
    private readonly onFix: (formula: string) => void,
  ) {
    this.canvas = document.createElement("canvas");
    this.canvas.id = "fx";
    this.canvas.setAttribute("aria-hidden", "true");
    stage.insertBefore(this.canvas, shaken[0]?.nextSibling ?? null);
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

  private get reducedMotion(): boolean {
    return this.reduced.matches;
  }

  private resize(): void {
    const r = this.stage.getBoundingClientRect();
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    this.canvas.width = Math.max(1, Math.round(r.width * dpr));
    this.canvas.height = Math.max(1, Math.round(r.height * dpr));
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
        case "bolt":
          this.bolts.push({ id: e.id, x0: e.x, y0: e.y, x1: e.tx, y1: e.ty, born: now, color: boltColor(out) });
          break;
        case "impact":
          this.impact(e, out, now);
          break;
        case "splash":
          this.burst(e.x, e.y, 10, COLOR.water, 0.04, now, 0.0003);
          break;
        case "hiss":
          this.bubbles(e.x, e.y - 2, 12, () => "#e8eef3", now);
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
        case "arc":
          this.flashes.push({ x: e.tx, y: e.ty, born: now, life: 160, color: COLOR.frost, radius: 4 });
          this.bolts.push({ id: -1, x0: e.x, y0: e.y, x1: e.tx, y1: e.ty, born: now - 1000, color: COLOR.frost });
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
        case "land":
          this.burst(e.x, e.y, Math.min(10, Math.round(e.power * 2)), "#c9b48a", 0.03, now, 0.0001);
          break;
        default:
          break;
      }
    }
  }

  private impact(e: Fx, out: CastOutcome | undefined, now: number): void {
    this.bolts = this.bolts.filter((b) => b.id !== e.id);
    const dh = Math.abs(e.dh_kj);
    const warm = e.dh_kj < 0;
    const cold = e.dh_kj > 0;
    // One soft flash, never repeated quickly.
    this.flashes.push({
      x: e.x,
      y: e.y,
      born: now,
      life: FLASH_MS,
      color: warm ? COLOR.spark : cold ? COLOR.frost : COLOR.water,
      radius: 6 + Math.min(14, dh / 40),
    });
    const n = Math.max(10, Math.min(60, Math.round(10 + dh / 10)));
    const palette = warm ? [COLOR.spark, COLOR.heat, COLOR.oxygen] : cold ? [COLOR.frost, "#ffffff"] : [COLOR.water, "#d8f0ff"];
    window.setTimeout(() => this.burst(e.x, e.y, n, (i) => palette[i % palette.length]!, 0.04 + Math.min(0.08, dh / 6000), performance.now(), warm ? 0.00004 : 0.0003), 50);
    if (cold) this.frostBorn = now;
    // Shake and hit-stop scale with the heat released or absorbed.
    if (!this.reducedMotion) {
      this.shakeAmp = Math.max(1, Math.min(8, dh / 100));
      this.shakeBorn = now;
      if (dh > 400) this.hitStop = 3;
    }
    // Leftover atoms drift back to the player as labelled orbs.
    const left = Object.entries(e.leftovers);
    left.forEach(([el, count], i) => {
      this.orbs.push({ x: e.x + i * 4, y: e.y - 4, tx: e.tx, ty: e.ty, born: now + 200 + i * 120, text: `+${count} ${el}` });
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
      eq.textContent = prettyFormula(out.equation);
      el.append(eq);
    }
    const text = document.createElement("div");
    text.textContent = out.label;
    el.append(text);
    if (this.tier === "apprentice" && out.delta_h_kj !== null && out.delta_h_kj !== 0) {
      const dh = document.createElement("div");
      dh.className = "fx-dh";
      const sign = out.delta_h_kj < 0 ? "−" : "+";
      dh.textContent = `ΔH = ${sign}${Math.abs(out.delta_h_kj).toFixed(1)} kJ (${out.source?.split(",")[0] ?? ""})`;
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
    this.labels = [{ el, x, y, born: performance.now(), life }];
    this.stage.append(el);
    this.placeLabel(this.labels[0]!);
  }

  private placeLabel(l: Label): void {
    // Keep the whole label on screen, whatever its width.
    const sw = this.stage.clientWidth;
    const sh = this.stage.clientHeight;
    const half = l.el.offsetWidth / 2 + 8;
    const x = ((l.x - this.viewX) / this.viewW) * sw;
    const y = (l.y / this.viewH) * sh;
    l.el.style.left = `${Math.max(half, Math.min(sw - half, x))}px`;
    l.el.style.top = `${Math.max(sh * 0.2, Math.min(sh * 0.8, y))}px`;
  }

  /** The rooms' targets, from the simulation (polled a few times a second). */
  setGoals(goals: Goal[]): void {
    this.goals = goals;
  }

  /** Bobbing icons over each open goal, a "fire left" meter over burning
   * hedges, and an edge arrow toward the next goal when it is off screen. */
  private drawGoals(now: number, k: number, viewX: number): void {
    const ctx = this.ctx;
    const next = this.goals.find((g) => !g.done);
    const bob = this.reducedMotion ? 0 : Math.sin(now / 300) * 1.5;
    for (const g of this.goals) {
      if (g.done) continue;
      const x = (g.x - viewX) * k;
      const y = (g.y + bob) * k;
      const onScreen = g.x >= viewX - 8 && g.x <= viewX + this.viewW + 8;
      if (!onScreen) {
        if (g === next) {
          const edge = g.x < viewX ? 10 * k : this.canvas.width - 10 * k;
          const dir = g.x < viewX ? -1 : 1;
          ctx.globalAlpha = 0.6 + 0.4 * Math.abs(Math.sin(now / 400));
          ctx.fillStyle = COLOR.gold;
          ctx.beginPath();
          ctx.moveTo(edge + dir * 4 * k, y);
          ctx.lineTo(edge - dir * 2 * k, y - 4 * k);
          ctx.lineTo(edge - dir * 2 * k, y + 4 * k);
          ctx.closePath();
          ctx.fill();
          ctx.globalAlpha = 1;
        }
        continue;
      }
      ctx.globalAlpha = 0.92;
      ctx.fillStyle = "rgba(15,23,32,0.7)";
      ctx.strokeStyle = g === next ? COLOR.gold : "#ffffff";
      ctx.lineWidth = Math.max(1, k * 0.5);
      ctx.beginPath();
      ctx.arc(x, y, 6 * k, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = "#ffffff";
      ctx.font = `${Math.round(7 * k)}px system-ui, sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(GOAL_ICONS[g.kind] ?? "★", x, y + k * 0.5);
      if (g.meter !== null && g.meter > 0) {
        // "Fire left": a bar that empties as the fire goes out.
        const w = 24 * k;
        const h = 2.5 * k;
        const top = y - 10 * k;
        ctx.fillStyle = "rgba(15,23,32,0.8)";
        ctx.fillRect(x - w / 2 - k * 0.5, top - k * 0.5, w + k, h + k);
        ctx.fillStyle = COLOR.oxygen;
        ctx.fillRect(x - w / 2, top, w * g.meter, h);
      }
    }
    ctx.globalAlpha = 1;
  }

  // ----------------------------------------------------------------- draw

  /** Draws one frame. `viewX` is the camera's left edge in world cells. */
  draw(now: number, viewX: number): void {
    this.viewX = viewX;
    const ctx = this.ctx;
    const k = this.scale;
    const sx = (x: number) => (x - viewX) * k;
    const sy = (y: number) => y * k;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    this.drawGoals(now, k, viewX);

    // Charge rings at the gauntlet.
    this.charges = this.charges.filter((c) => now - c.born < c.life);
    for (const c of this.charges) {
      const t = (now - c.born) / c.life;
      ctx.strokeStyle = c.color;
      ctx.globalAlpha = 1 - t;
      ctx.lineWidth = Math.max(1, k * 0.8);
      ctx.beginPath();
      ctx.arc(sx(c.x), sy(c.y), (c.radius * (1 - t) + 1) * k, 0, Math.PI * 2);
      ctx.stroke();
    }

    // Bolts with a fading trail.
    this.bolts = this.bolts.filter((b) => now - b.born < 1400);
    for (const b of this.bolts) {
      const dist = Math.hypot(b.x1 - b.x0, b.y1 - b.y0) || 1;
      const t = Math.min(1, ((now - b.born) * BOLT_CELLS_PER_MS) / dist);
      const hx = b.x0 + (b.x1 - b.x0) * t;
      const hy = b.y0 + (b.y1 - b.y0) * t;
      const tail = Math.max(0, t - 18 / dist);
      const tx = b.x0 + (b.x1 - b.x0) * tail;
      const ty = b.y0 + (b.y1 - b.y0) * tail;
      const grad = ctx.createLinearGradient(sx(tx), sy(ty), sx(hx), sy(hy));
      grad.addColorStop(0, "rgba(255,255,255,0)");
      grad.addColorStop(1, b.color);
      ctx.globalAlpha = b.id === -1 ? Math.max(0, 1 - (now - b.born - 1000) / 400) : 1;
      ctx.strokeStyle = grad;
      ctx.lineWidth = 2 * k;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.moveTo(sx(tx), sy(ty));
      ctx.lineTo(sx(hx), sy(hy));
      ctx.stroke();
      ctx.fillStyle = "#ffffff";
      ctx.beginPath();
      ctx.arc(sx(hx), sy(hy), 1.4 * k, 0, Math.PI * 2);
      ctx.fill();
    }

    // Impact flashes: a single soft fade.
    this.flashes = this.flashes.filter((f) => now - f.born < f.life);
    ctx.globalCompositeOperation = "lighter";
    for (const f of this.flashes) {
      const t = (now - f.born) / f.life;
      const r = f.radius * k * (0.6 + 0.4 * t);
      const g = ctx.createRadialGradient(sx(f.x), sy(f.y), 0, sx(f.x), sy(f.y), r);
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
        ctx.arc(sx(x), sy(y), p.size * k, 0, Math.PI * 2);
        ctx.stroke();
      } else {
        ctx.fillStyle = p.color;
        ctx.fillRect(sx(x) - (p.size * k) / 2, sy(y) - (p.size * k) / 2, p.size * k, p.size * k);
      }
    }

    // Leftover orbs drift back to the player.
    this.orbs = this.orbs.filter((o) => now - o.born < 1400);
    ctx.font = `bold ${Math.round(4 * k)}px system-ui, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (const o of this.orbs) {
      const t = Math.max(0, (now - o.born) / 1400);
      const e = t * t * (3 - 2 * t);
      const x = o.x + (o.tx - o.x) * e;
      const y = o.y + (o.ty - o.y) * e - Math.sin(t * Math.PI) * 10;
      ctx.globalAlpha = t < 0.85 ? 1 : (1 - t) / 0.15;
      ctx.fillStyle = COLOR.oxygen;
      ctx.beginPath();
      ctx.arc(sx(x), sy(y), 3 * k, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#ffffff";
      ctx.fillText(o.text, sx(x), sy(y) - 6 * k);
    }

    // Fizzle hints: a ghost of what was missing and an arrow to where to get it.
    this.ghosts = this.ghosts.filter((g) => now - g.born < 2500);
    for (const g of this.ghosts) {
      const t = (now - g.born) / 2500;
      ctx.globalAlpha = Math.min(1, (1 - t) * 2) * 0.85;
      const gy = sy(g.y) - Math.sin(now / 200) * k;
      // A badge behind the icon keeps it visible even without an emoji font.
      ctx.fillStyle = "rgba(15,23,32,0.75)";
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = Math.max(1, k * 0.5);
      ctx.beginPath();
      ctx.arc(sx(g.x), gy, 7 * k, 0, Math.PI * 2);
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
      const v = ctx.createRadialGradient(w / 2, h / 2, Math.min(w, h) * 0.35, w / 2, h / 2, Math.max(w, h) * 0.7);
      v.addColorStop(0, "rgba(166,220,245,0)");
      v.addColorStop(1, "rgba(166,220,245,0.55)");
      ctx.globalAlpha = 1 - ft;
      ctx.fillStyle = v;
      ctx.fillRect(0, 0, w, h);
    }
    ctx.globalAlpha = 1;

    // Floating labels follow the world as the camera moves.
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

    this.applyShake(now);
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
