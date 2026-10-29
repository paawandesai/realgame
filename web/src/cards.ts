// Sprout spell cards: big picture buttons in the bottom-right corner. Tap a
// card to cast it (soft auto-aim finds the target); drag it onto the world to
// cast it exactly there. Cards appear as the rooms introduce them.

import { icon, type IconName } from "./icons";
import type { GameState } from "./types";

export type CardSpell = "water" | "boom" | "split" | "cloud";

interface CardDef {
  spell: CardSpell;
  picture: IconName;
  /** Short word under the picture (for emerging readers; never needed). */
  word: string;
  /** Spoken when the card is focused or first appears. */
  name: string;
}

export const CARDS: CardDef[] = [
  { spell: "water", picture: "drop", word: "Water", name: "water drop" },
  { spell: "split", picture: "bubbles", word: "Bubbles", name: "bubbles" },
  { spell: "boom", picture: "boom", word: "Boom", name: "boom" },
  { spell: "cloud", picture: "cloud", word: "Cloud", name: "cloud" },
];

/** How full each card's gauge is, from the suit's jars. */
function charge(spell: CardSpell, s: GameState): number {
  const j = (f: string) => s.jars[f] ?? 0;
  const cap = (f: string) => s.jar_caps[f] ?? 10;
  switch (spell) {
    case "water":
      return j("H2O") / cap("H2O");
    case "split":
      return Math.min(1, j("H2O") / cap("H2O"));
    case "boom":
      return Math.min(1, Math.min(j("H2") / 2, j("O2")) / 3);
    case "cloud":
      return j("CO2") / cap("CO2");
  }
}

export class SproutCards {
  private readonly buttons = new Map<CardSpell, HTMLButtonElement>();
  private readonly gauges = new Map<CardSpell, HTMLElement>();
  private dragGhost: HTMLElement | null = null;
  private suppressClick = false;

  constructor(
    private readonly root: HTMLElement,
    private readonly world: HTMLElement,
    private readonly onCast: (spell: CardSpell, drop: [number, number] | null) => void,
  ) {
    for (const c of CARDS) {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "card";
      b.dataset.spell = c.spell;
      b.setAttribute("aria-label", c.name);
      b.hidden = true;
      const pic = document.createElement("span");
      pic.className = "card-pic";
      pic.append(icon(c.picture));
      const word = document.createElement("span");
      word.className = "card-word";
      word.textContent = c.word;
      const gauge = document.createElement("span");
      gauge.className = "card-gauge";
      const fill = document.createElement("span");
      gauge.append(fill);
      b.append(pic, word, gauge);
      this.bindDrag(b, c);
      b.addEventListener("click", () => {
        if (this.suppressClick) {
          this.suppressClick = false;
          return;
        }
        this.onCast(c.spell, null);
      });
      root.append(b);
      this.buttons.set(c.spell, b);
      this.gauges.set(c.spell, fill);
    }
  }

  private bindDrag(b: HTMLButtonElement, c: CardDef): void {
    let start: [number, number] | null = null;
    let dragging = false;
    b.addEventListener("pointerdown", (e) => {
      start = [e.clientX, e.clientY];
      dragging = false;
      b.setPointerCapture?.(e.pointerId);
    });
    b.addEventListener("pointermove", (e) => {
      if (!start) return;
      if (!dragging && Math.hypot(e.clientX - start[0], e.clientY - start[1]) > 14) {
        dragging = true;
        this.dragGhost = document.createElement("div");
        this.dragGhost.className = "card-drag";
        this.dragGhost.append(icon(c.picture));
        document.body.append(this.dragGhost);
      }
      if (dragging && this.dragGhost) {
        this.dragGhost.style.left = `${e.clientX}px`;
        this.dragGhost.style.top = `${e.clientY}px`;
      }
    });
    const end = (e: PointerEvent, cancelled: boolean) => {
      if (!start) return;
      start = null;
      this.dragGhost?.remove();
      this.dragGhost = null;
      if (!dragging) return;
      dragging = false;
      // A drag never also counts as a tap.
      this.suppressClick = true;
      window.setTimeout(() => (this.suppressClick = false), 0);
      if (cancelled) return;
      const r = this.world.getBoundingClientRect();
      const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
      if (inside) this.onCast(c.spell, [e.clientX, e.clientY]);
    };
    b.addEventListener("pointerup", (e) => end(e, false));
    b.addEventListener("pointercancel", (e) => end(e, true));
  }

  /** Shows the cards the player has met; returns the ones that just appeared. */
  unlock(spells: Iterable<CardSpell>): CardSpell[] {
    const fresh: CardSpell[] = [];
    for (const s of spells) {
      const b = this.buttons.get(s);
      if (b?.hidden) {
        b.hidden = false;
        fresh.push(s);
      }
    }
    return fresh;
  }

  reset(): void {
    for (const b of this.buttons.values()) b.hidden = true;
  }

  update(s: GameState): void {
    for (const [spell, fill] of this.gauges) {
      const c = Math.max(0, Math.min(1, charge(spell, s)));
      fill.style.width = `${Math.round(c * 100)}%`;
      this.buttons.get(spell)?.classList.toggle("empty", c <= 0);
    }
  }

  /** Hint level 2+: make the needed card glow. */
  setHint(spell: CardSpell | null): void {
    for (const [s, b] of this.buttons) b.classList.toggle("hint", s === spell);
  }

  /** The goal is ready for this card (the gas is trapped): pulse it. */
  setReady(spell: CardSpell | null): void {
    for (const [s, b] of this.buttons) b.classList.toggle("ready", s === spell);
  }

  private readonly timers = new Map<string, number>();
  /** Adds a class for `ms`, restarting its animation if it is already on. */
  private flash(b: HTMLButtonElement, cls: string, ms: number): void {
    b.classList.remove(cls);
    void b.offsetWidth;
    b.classList.add(cls);
    const key = `${b.dataset.spell}:${cls}`;
    window.clearTimeout(this.timers.get(key));
    this.timers.set(
      key,
      window.setTimeout(() => {
        b.classList.remove(cls);
        if (cls === "wobble") b.querySelector(".card-x")?.remove();
      }, ms),
    );
  }

  /** Sprout: this card did not help the goal. A wobble and a grey X over its
   * picture; no words. */
  wrong(spell: CardSpell): void {
    const b = this.buttons.get(spell);
    if (!b) return;
    if (!b.querySelector(".card-x")) {
      const x = document.createElement("span");
      x.className = "card-x";
      x.setAttribute("aria-hidden", "true");
      x.append(icon("close"));
      b.append(x);
    }
    this.flash(b, "wobble", 1100);
  }

  /** Sprout: this is the card that helps. It bounces. */
  bounce(spell: CardSpell): void {
    const b = this.buttons.get(spell);
    if (b && !b.hidden) this.flash(b, "bounce", 1800);
  }

  element(spell: CardSpell): HTMLButtonElement | null {
    const b = this.buttons.get(spell);
    return b && !b.hidden ? b : null;
  }

  get rootEl(): HTMLElement {
    return this.root;
  }
}
