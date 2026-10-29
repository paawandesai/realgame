// One popup at a time. The Grimoire's "new page" card, the room-clear stamp
// and the Apprentice room score all queue here, so they never pile up on
// each other (or on the cast-result bubble, which steers around whatever is
// showing). Each popup leaves after its time, on the next input when it
// asks for that, or when it is dropped (a new room, the end screen).
// Presentation only.

export interface Popup {
  /** What it is, so it can be dropped by name ("discover", "room-clear", "score"). */
  kind: string;
  el: HTMLElement;
  /** How long it stays, ms. */
  ms: number;
  /** Leave on the next key press or tap after it appears. */
  dismissOnInput?: boolean;
}

/** Inputs within this long of a popup appearing don't dismiss it (the tap
 * that caused it is still in flight). */
const INPUT_GRACE_MS = 250;

export class PopupQueue {
  private queue: Popup[] = [];
  private current: Popup | null = null;
  private shownAt = 0;
  private timer = 0;

  constructor() {
    const onInput = (e: Event) => {
      if (e instanceof KeyboardEvent && e.repeat) return;
      const c = this.current;
      if (c?.dismissOnInput && performance.now() - this.shownAt > INPUT_GRACE_MS) this.next();
    };
    window.addEventListener("keydown", onInput, { capture: true });
    window.addEventListener("pointerdown", onInput, { capture: true });
  }

  /** The popup on screen now, if any. */
  get active(): HTMLElement | null {
    return this.current?.el ?? null;
  }

  /** Whether a popup of one of these kinds is showing or waiting its turn. */
  has(...kinds: string[]): boolean {
    return (this.current !== null && kinds.includes(this.current.kind)) || this.queue.some((q) => kinds.includes(q.kind));
  }

  /** Shows `p` now if nothing is showing, else after what is queued. A popup
   * of the same kind already waiting or showing is replaced, not repeated. */
  push(p: Popup): void {
    this.queue = this.queue.filter((q) => q.kind !== p.kind);
    if (this.current?.kind === p.kind) {
      this.queue.unshift(p);
      this.next();
      return;
    }
    this.queue.push(p);
    if (!this.current) this.next();
  }

  /** Drops every popup of these kinds (all kinds when none are given). */
  drop(...kinds: string[]): void {
    const hit = (p: Popup) => kinds.length === 0 || kinds.includes(p.kind);
    this.queue = this.queue.filter((q) => !hit(q));
    if (this.current && hit(this.current)) this.next();
  }

  private next(): void {
    window.clearTimeout(this.timer);
    // Hidden at once (it fades in, never out), so two are never on screen.
    if (this.current) this.current.el.hidden = true;
    const p = this.queue.shift() ?? null;
    this.current = p;
    if (!p) return;
    p.el.hidden = false;
    // Restart the fade-in.
    p.el.classList.remove("popup-in");
    void p.el.offsetWidth;
    p.el.classList.add("popup-in");
    this.shownAt = performance.now();
    this.timer = window.setTimeout(() => this.next(), p.ms);
  }
}

/** Whether two rects overlap (touching edges don't count). */
export function intersects(a: DOMRect, b: DOMRect): boolean {
  return a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;
}
