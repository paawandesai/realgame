// Where to go and what to do, without reading: a bobbing picture over each
// room's target (from the simulation's goal list), a "go" arrow at the screen
// edge when the next target is away, a four-dot room progress strip, and the
// hint ladder's pulse and ghost hint. Presentation only.

import { icon, type IconName } from "./icons";
import type { Goal } from "./types";

const GOAL_ICON: Record<Goal["kind"], IconName> = {
  fire: "hedgeFire",
  ice: "ice",
  gas: "bubbles",
  brazier: "bowl",
};

/** The marker's picture: room 4's marker is the burning hedge until the
 * hedge is out (water is what it needs), then the bowl. */
function markerIcon(g: Goal): IconName {
  return g.kind === "brazier" && g.need === "water" ? "hedgeFire" : GOAL_ICON[g.kind];
}

/** Sprout: a small badge on the marker saying which card it needs (a water
 * drop on a burning hedge) or that it is the trial's final goal (a star on
 * the bowl). */
function markerBadge(g: Goal): IconName | null {
  if (g.need === "water") return "drop";
  if (g.kind === "brazier") return "star";
  return null;
}

/** The player is "there" within this many cells of a goal's standing spot. */
export const NEAR_CELLS = 40;
/** Top of the flat ground in world cells (GROUND_Y in game/rooms.rs). */
const GROUND_Y = 148;
/** Standing within this many cells of the spot counts as on it. */
const ON_SPOT_CELLS = 5;

interface Marker {
  el: HTMLElement;
  meter: HTMLElement;
}

export class GoalMarkers {
  readonly layer: HTMLElement;
  private readonly markers: Marker[] = [];
  private readonly go: HTMLButtonElement;
  private readonly pulse: HTMLElement;
  private readonly dots: HTMLElement[] = [];
  private goals: Goal[] = [];
  private hintLevel = 0;
  private readonly area: HTMLElement;
  private readonly battery: HTMLElement;
  private readonly pointer: HTMLElement;
  private readonly stand: HTMLElement;
  /** Room 3: how full the gas trap is, as a bar and "gas 3/4". */
  private readonly trapMeter: HTMLElement;
  private readonly trapFill: HTMLElement;
  /** The oxygen row under the hydrogen fill: half as long, 2:1. */
  private readonly trapO2: HTMLElement;
  private readonly trapText: HTMLElement;
  /** Show a bouncing pointer on the next goal (casts are not moving it on). */
  pointAt = false;
  /** Whether the go arrow is showing (the player should walk first). */
  goVisible = false;
  /** No gold hint ring (Sprout: the one bullseye already sits on the goal). */
  pulseOff = false;
  /** Sprout: meters and tags are pictures (and digits), never words. */
  private pictureOnly = false;

  /** Sprout reads pictures: the trap meter is a filling bar with a boom
   * picture when full, and the battery tag is just its lightning bolt. */
  setPictureOnly(on: boolean): void {
    this.pictureOnly = on;
    this.setBatteryTag(on);
    const stand = this.stand.firstElementChild;
    if (stand) stand.textContent = on ? "⬇" : "Stand here";
    this.trapText.replaceChildren();
    this.trapKey = "";
  }

  private trapKey = "";

  /** The crystal's name tag: a crystal picture, and its name for readers. */
  private setBatteryTag(pictureOnly: boolean): void {
    this.battery.replaceChildren(icon("crystal"));
    if (!pictureOnly) {
      const word = document.createElement("span");
      word.textContent = "Crystal battery";
      this.battery.append(word);
    }
    this.area.classList.toggle("sprout-cloud", pictureOnly);
    this.area.replaceChildren();
    if (pictureOnly) {
      // Sprout: the gas pocket is a glowing cloud of bubbles with a pulsing
      // bullseye, not an outlined box.
      const cloud = document.createElement("span");
      cloud.className = "cloud-pic";
      cloud.append(icon("bubbles"));
      const eye = document.createElement("span");
      eye.className = "cloud-eye";
      this.area.append(cloud, eye);
    }
  }

  constructor(
    private readonly stage: HTMLElement,
    private readonly strip: HTMLElement,
    private readonly viewW: number,
    private readonly viewH: number,
    onGo: () => void,
  ) {
    this.layer = document.createElement("div");
    this.layer.id = "goal-layer";
    this.layer.setAttribute("aria-hidden", "true");
    stage.append(this.layer);

    this.pulse = document.createElement("div");
    this.pulse.id = "hint-pulse";
    this.pulse.hidden = true;
    this.layer.append(this.pulse);

    // Room 3: the gas pocket to fill, and a name tag on the crystal battery.
    this.area = document.createElement("div");
    this.area.id = "goal-area";
    this.area.hidden = true;
    this.battery = document.createElement("div");
    this.battery.id = "battery-tag";
    this.battery.hidden = true;
    this.pointer = document.createElement("div");
    this.pointer.id = "goal-pointer";
    this.pointer.hidden = true;
    this.pointer.append(icon("hand"));
    // Where to stand to cast at the brazier, once the hedge is out of the way.
    this.stand = document.createElement("div");
    this.stand.id = "stand-spot";
    this.stand.hidden = true;
    const standLabel = document.createElement("span");
    standLabel.className = "stand-label";
    standLabel.textContent = "Stand here";
    this.stand.append(standLabel);
    this.trapMeter = document.createElement("div");
    this.trapMeter.id = "trap-meter";
    this.trapMeter.hidden = true;
    const bar = document.createElement("div");
    bar.className = "trap-bar";
    // Two rows on one scale: hydrogen on top, oxygen below, so the 2:1
    // the split makes shows as one bar twice the length of the other.
    this.trapFill = document.createElement("div");
    this.trapFill.className = "trap-h2";
    this.trapO2 = document.createElement("div");
    this.trapO2.className = "trap-o2";
    bar.append(this.trapFill, this.trapO2);
    this.trapText = document.createElement("span");
    this.trapText.className = "trap-text";
    this.trapMeter.append(icon("bubbles"), bar, this.trapText);
    this.layer.append(this.area, this.battery, this.pointer, this.stand, this.trapMeter);
    this.setBatteryTag(false);

    this.go = document.createElement("button");
    this.go.type = "button";
    this.go.id = "go-arrow";
    this.go.setAttribute("aria-label", "Walk to the next goal");
    this.go.hidden = true;
    this.go.addEventListener("click", onGo);
    stage.append(this.go);
  }

  /** The rooms' targets, from the simulation (polled a few times a second). */
  setGoals(goals: Goal[]): void {
    this.goals = goals;
    while (this.markers.length < goals.length) {
      const el = document.createElement("div");
      el.className = "goal-icon";
      const meter = document.createElement("div");
      meter.className = "goal-meter";
      const fill = document.createElement("div");
      meter.append(fill);
      el.append(meter);
      this.layer.append(el);
      this.markers.push({ el, meter });
    }
    goals.forEach((g, i) => {
      const m = this.markers[i]!;
      const pic = markerIcon(g);
      if (m.el.dataset.kind !== g.kind || m.el.dataset.pic !== pic) {
        m.el.dataset.kind = g.kind;
        m.el.dataset.pic = pic;
        // Swap the picture, never stack a second one.
        m.el.querySelector(":scope > .icon")?.remove();
        m.el.prepend(icon(pic));
      }
      const badge = this.pictureOnly ? markerBadge(g) : null;
      if ((m.el.dataset.badge ?? "") !== (badge ?? "")) {
        m.el.querySelector(":scope > .goal-badge")?.remove();
        if (badge) {
          const b = document.createElement("span");
          b.className = `goal-badge badge-${badge}`;
          b.append(icon(badge));
          m.el.append(b);
          m.el.dataset.badge = badge;
        } else {
          delete m.el.dataset.badge;
        }
      }
      m.el.dataset.room = String(g.room);
      m.el.hidden = g.done;
      // The gas trap has its own meter on the trap (see draw).
      const show = g.kind !== "gas" && g.meter !== null && g.meter > 0;
      m.meter.hidden = !show;
      if (show) (m.meter.firstElementChild as HTMLElement).style.width = `${Math.round((g.meter ?? 0) * 100)}%`;
    });
    this.renderStrip();
  }

  /** The first goal not yet reached, or null when the trial is done. */
  get next(): Goal | null {
    return this.goals.find((g) => !g.done) ?? null;
  }

  setHintLevel(level: number): void {
    this.hintLevel = level;
  }

  private renderStrip(): void {
    if (this.dots.length !== this.goals.length) {
      this.strip.replaceChildren();
      this.dots.length = 0;
      for (let i = 0; i < this.goals.length; i++) {
        const d = document.createElement("span");
        d.className = "dot";
        this.strip.append(d);
        this.dots.push(d);
      }
    }
    const next = this.next;
    let done = 0;
    this.goals.forEach((g, i) => {
      const d = this.dots[i]!;
      d.classList.toggle("done", g.done);
      d.classList.toggle("current", g === next);
      if (g.done) done++;
    });
    this.strip.setAttribute("aria-label", `Rooms done: ${done} of ${this.goals.length}`);
    this.strip.dataset.done = String(done);
  }

  private goRight: boolean | null = null;

  /** Repositions everything for this frame. */
  draw(now: number, viewX: number, playerCx: number, reducedMotion: boolean): void {
    const sw = this.stage.clientWidth;
    const sh = this.stage.clientHeight;
    const kx = sw / this.viewW;
    const ky = sh / this.viewH;
    const bob = reducedMotion ? 0 : Math.sin(now / 300) * 4;
    const next = this.next;
    this.goals.forEach((g, i) => {
      const m = this.markers[i]!;
      m.el.classList.toggle("next", g === next);
      if (g.done) return;
      const x = (g.x - viewX) * kx;
      const on = x >= -20 && x <= sw + 20;
      m.el.classList.toggle("offscreen", !on);
      m.el.style.transform = `translate(${x.toFixed(1)}px, ${(g.y * ky + bob).toFixed(1)}px)`;
    });

    // Walk first? The go arrow points at where to stand for the next goal.
    const far = next !== null && Math.abs(next.stand_x - playerCx) > NEAR_CELLS;
    this.goVisible = far;
    this.go.hidden = !far;
    if (far && next) {
      // Point the way to the goal itself: through the gate when it is in a
      // later room, else toward the target (never toward its standing spot,
      // which can sit behind a player who has walked past it).
      // Goals number rooms from 1.
      const here = Math.floor(playerCx / this.viewW) + 1;
      const right = next.room !== here ? next.room > here : next.x > playerCx;
      // Only rebuild the arrow's picture when it turns round.
      if (this.goRight !== right) {
        this.goRight = right;
        this.go.classList.toggle("left", !right);
      }
      // The next goal is through a door: show an open doorway, not a play
      // arrow, so "walk on to the next room" reads at a glance.
      const door = next.room !== here;
      const pic = door ? (right ? "door" : "doorLeft") : right ? "right" : "left";
      if (this.go.dataset.pic !== pic) {
        this.go.dataset.pic = pic;
        this.go.replaceChildren(icon(pic));
        this.go.setAttribute("aria-label", door ? "Go to the next room" : "Walk to the next goal");
      }
      this.go.classList.toggle("hinting", this.hintLevel >= 1);
    }

    // The next goal's target area (the gas pocket) and the battery's name tag.
    const area = next?.area ?? null;
    this.area.hidden = area === null;
    if (area) {
      const [x0, y0, x1, y1] = area;
      const a = this.area.style;
      a.left = `${((x0 - viewX) * kx).toFixed(1)}px`;
      a.top = `${(y0 * ky).toFixed(1)}px`;
      a.width = `${((x1 - x0) * kx).toFixed(1)}px`;
      a.height = `${((y1 - y0) * ky).toFixed(1)}px`;
    }
    // Room 3: the trap's fill meter, on the stone roof above the trap (the
    // pointing hand sits in the trap), in quarters.
    const gas = next?.kind === "gas" ? next : null;
    // The goal's own meter: shown while its trap is on screen (the player
    // splits water a little way off, so no distance limit).
    const trapOnScreen = area !== null && area[2] >= viewX && area[0] <= viewX + this.viewW;
    this.trapMeter.hidden = gas === null || !trapOnScreen;
    if (gas && area) {
      const fill = Math.max(0, Math.min(1, gas.meter ?? 0));
      const quarters = Math.round(fill * 4);
      // Hydrogen trapped fills the top row; oxygen sits under it in the
      // ratio the splits made it (always 2:1), so the O₂ row is half.
      const [, madeH2, madeO2, need] = gas.gas ?? [0, 0, 0, 0];
      const ratio = madeH2 > 0 ? madeO2 / madeH2 : 0.5;
      this.trapFill.style.width = `${Math.round(fill * 100)}%`;
      this.trapO2.style.width = `${Math.round(fill * ratio * 100)}%`;
      const text = fill >= 1 ? "Full! Boom it!" : need > 0 && madeH2 > 0 ? `H₂ ${madeH2} : O₂ ${madeO2}` : `gas ${quarters}/4`;
      const key = `${this.pictureOnly}:${text}`;
      if (key !== this.trapKey) {
        this.trapKey = key;
        if (!this.pictureOnly) this.trapText.textContent = text;
        else if (fill >= 1) this.trapText.replaceChildren(icon("boom"));
        else this.trapText.replaceChildren();
        this.trapText.hidden = this.pictureOnly && fill < 1;
      }
      this.trapMeter.classList.toggle("full", fill >= 1);
      this.trapMeter.setAttribute("aria-label", fill >= 1 ? "Gas trap full" : `Gas trap ${quarters} of 4 full`);
      const [x0, y0, x1] = area;
      this.trapMeter.style.transform = `translate(${(((x0 + x1) / 2 - viewX) * kx).toFixed(1)}px, ${((y0 - 4) * ky - 6).toFixed(1)}px)`;
    }
    // Name tags only for things on screen and near the player (within
    // NEAR_CELLS): the next room's crystal never labels itself from here.
    const inView = (x: number) => x >= viewX && x <= viewX + this.viewW;
    const near = (x: number) => inView(x) && Math.abs(x - playerCx) <= NEAR_CELLS;
    const bat = next?.battery ?? null;
    this.battery.hidden = bat === null || !near(bat[0]);
    if (bat) {
      this.battery.style.transform = `translate(${((bat[0] - viewX) * kx).toFixed(1)}px, ${(bat[1] * ky).toFixed(1)}px)`;
    }
    // Room 4's second step: a glowing pad where the bowl can be reached.
    const showStand =
      next !== null &&
      next.kind === "brazier" &&
      (next.steps?.[0]?.done ?? false) &&
      Math.abs(next.stand_x - playerCx) > ON_SPOT_CELLS &&
      near(next.stand_x);
    this.stand.hidden = !showStand;
    if (showStand && next) {
      this.stand.style.transform = `translate(${((next.stand_x - viewX) * kx).toFixed(1)}px, ${(GROUND_Y * ky).toFixed(1)}px)`;
    }

    // A hand pointing at the target when casts are not moving the goal on.
    const point = this.pointAt && next !== null && !far;
    this.pointer.hidden = !point;
    if (point && next) {
      const tap = reducedMotion ? 0 : Math.abs(Math.sin(now / 250)) * 10;
      this.pointer.style.transform = `translate(${((next.x - viewX) * kx).toFixed(1)}px, ${(next.y * ky - tap).toFixed(1)}px)`;
    }

    // Hint level 1+: a pulsing ring on the target (or on the go arrow).
    const showPulse = !this.pulseOff && this.hintLevel >= 1 && next !== null && !far;
    this.pulse.hidden = !showPulse;
    if (showPulse && next) {
      const x = (next.x - viewX) * kx;
      this.pulse.style.transform = `translate(${x.toFixed(1)}px, ${(next.y * ky).toFixed(1)}px)`;
    }
    this.keepLabelsClear();
  }

  /** The world labels this layer places: name tags, meters and lens readings. */
  private labels(): HTMLElement[] {
    const out: HTMLElement[] = [this.battery, this.trapMeter];
    const stand = this.stand.firstElementChild;
    if (!this.stand.hidden && stand instanceof HTMLElement) out.push(stand);
    for (const el of this.layer.querySelectorAll<HTMLElement>(".lens-spot")) out.push(el);
    return out.filter((el) => !el.hidden);
  }

  /**
   * Keeps every world label inside the visible picture and off the goal
   * markers: a label slides sideways to stay in view, and one that would sit
   * on a marker moves above or below it. Reads all the boxes first, then
   * writes, so a frame costs one layout.
   */
  private keepLabelsClear(): void {
    const labels = this.labels();
    if (labels.length === 0) return;
    const canvas = this.stage.querySelector("canvas");
    const st = this.stage.getBoundingClientRect();
    const cv = canvas?.getBoundingClientRect() ?? st;
    const pad = 4;
    const box = {
      left: Math.max(st.left, cv.left) + pad,
      right: Math.min(st.right, cv.right) - pad,
      top: Math.max(st.top, cv.top) + pad,
      bottom: Math.min(st.bottom, cv.bottom) - pad,
    };
    const markers = this.markers
      .filter((m) => !m.el.hidden && !m.el.classList.contains("offscreen"))
      .map((m) => m.el.getBoundingClientRect());
    const reads = labels.map((el) => [el, el.getBoundingClientRect()] as const);
    for (const [el, now] of reads) {
      const [ox, oy] = [Number(el.dataset.nx ?? 0), Number(el.dataset.ny ?? 0)];
      // Where the label would be with no nudge.
      let l = now.left - ox;
      let t = now.top - oy;
      const wd = now.width;
      const ht = now.height;
      const fitX = (x: number) => Math.max(box.left, Math.min(x, box.right - wd));
      const fitY = (y: number) => Math.max(box.top, Math.min(y, box.bottom - ht));
      l = fitX(l);
      t = fitY(t);
      const hits = (x: number, y: number) =>
        markers.find((m) => x < m.right && m.left < x + wd && y < m.bottom && m.top < y + ht);
      const m = hits(l, t);
      if (m) {
        // Above the marker, else below it, else beside it.
        const tries: [number, number][] = [
          [l, m.top - ht - 2],
          [l, m.bottom + 2],
          [m.right + 2, t],
          [m.left - wd - 2, t],
        ];
        for (const [x, y] of tries) {
          const fx = fitX(x);
          const fy = fitY(y);
          if (!hits(fx, fy)) {
            [l, t] = [fx, fy];
            break;
          }
        }
      }
      const nx = Math.round(l - (now.left - ox));
      const ny = Math.round(t - (now.top - oy));
      if (nx === ox && ny === oy) continue;
      el.dataset.nx = String(nx);
      el.dataset.ny = String(ny);
      // A nudge in its own property, so it adds to whatever places the label.
      el.style.setProperty("--nudge-x", `${nx}px`);
      el.style.setProperty("--nudge-y", `${ny}px`);
    }
  }

  /** Screen position (stage px) of the next goal, for the ghost hand. */
  nextOnStage(viewX: number): [number, number] | null {
    const n = this.next;
    if (!n) return null;
    return [((n.x - viewX) * this.stage.clientWidth) / this.viewW, (n.y * this.stage.clientHeight) / this.viewH];
  }
}
