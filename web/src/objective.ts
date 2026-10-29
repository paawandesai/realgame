// The room objective: when the player reaches a room (or its goal changes),
// a big banner with the goal's picture and one line shows under the top bar,
// then shrinks into a goal chip (the pill) that stays. Only one copy of the
// goal is ever on screen: the pill steps aside while the banner (or any
// other banner, such as the room-clear stamp) is up, a ticked step only
// glows the pill, and the first key press or tap sends the banner into the
// pill early. Tapping the pill shows (and says) the goal again. Separate
// from Mote's captions, so every tier always sees what the room wants,
// whatever the caption setting. Presentation only.

import { icon, type IconName } from "./icons";
import type { Goal, Tier } from "./types";
import { prettyFormula } from "./types";

const GOAL_ICON: Record<Goal["kind"], IconName> = {
  fire: "fire",
  ice: "ice",
  gas: "bubbles",
  brazier: "bowl",
};

const STEP_ICON: Record<string, IconName> = {
  bubbles: "bubbles",
  boom: "boom",
  fire: "fire",
  bowl: "bowl",
};

/** How long the big banner stays before it shrinks into the chip; with
 * the shrink it is gone within 2.5 s. */
const BANNER_MS = 2150;
/** The shrink into the chip (matches the CSS transition). */
const SHRINK_MS = 300;
/** Input this soon after the banner appears doesn't dismiss it (the tap or
 * step that brought it up is still in flight). */
const INPUT_GRACE_MS = 250;

export class ObjectiveBanner {
  private readonly banner: HTMLElement;
  private readonly bannerPic: HTMLElement;
  private readonly bannerText: HTMLElement;
  private readonly chip: HTMLButtonElement;
  private readonly chipPic: HTMLElement;
  private readonly chipText: HTMLElement;
  private readonly bannerSteps: HTMLElement;
  private readonly chipSteps: HTMLElement;
  private key = "";
  private text = "";
  private timer = 0;
  private holdTimer = 0;
  private holdUntil = 0;
  private shownAt = 0;
  /** Another banner (the room-clear stamp) is up: the pill waits too. */
  private othersUp = false;
  /** The banner is waiting for another banner to finish. */
  private pending = false;
  /** The goal line as the simulation wrote it (for speech). */
  private raw = "";
  /** Called when the player asks for the goal again; the caller speaks it. */
  onAnnounce: ((line: string) => void) | null = null;
  /** "Walk this way": shown in the chip's place while the next goal is
   * still off screen, so nobody casts at an empty field. */
  private readonly walkEl: HTMLButtonElement;
  private readonly walkPic: HTMLElement;
  private walkDir = 0;
  /** The walk prompt was tapped (walk there for me). */
  onWalk: (() => void) | null = null;

  /** `pillRow` holds the goal chip, `bannerRow` the big banner; both live
   * in the top HUD column, so they stack and never overlap. */
  constructor(
    pillRow: HTMLElement,
    bannerRow: HTMLElement,
    private readonly tier: Tier,
  ) {
    this.banner = document.createElement("div");
    this.banner.id = "objective-banner";
    this.banner.setAttribute("role", "status");
    this.banner.hidden = true;
    this.bannerPic = document.createElement("span");
    this.bannerPic.className = "objective-pic";
    this.bannerText = document.createElement("span");
    this.bannerText.className = "objective-text";
    this.bannerSteps = document.createElement("span");
    this.bannerSteps.className = "objective-steps";
    const bannerBody = document.createElement("span");
    bannerBody.className = "objective-body";
    bannerBody.append(this.bannerText, this.bannerSteps);
    this.banner.append(this.bannerPic, bannerBody);

    this.chip = document.createElement("button");
    this.chip.type = "button";
    this.chip.id = "goal-chip";
    this.chip.hidden = true;
    this.chipPic = document.createElement("span");
    this.chipPic.className = "objective-pic";
    this.chipText = document.createElement("span");
    this.chipText.className = "objective-text";
    this.chipSteps = document.createElement("span");
    this.chipSteps.className = "objective-steps";
    this.chip.append(this.chipPic, this.chipText, this.chipSteps);
    this.chip.addEventListener("click", () => this.replay());
    this.walkEl = document.createElement("button");
    this.walkEl.type = "button";
    this.walkEl.id = "walk-prompt";
    this.walkEl.hidden = true;
    this.walkPic = document.createElement("span");
    this.walkPic.className = "walk-pic";
    this.walkPic.setAttribute("aria-hidden", "true");
    this.walkEl.append(this.walkPic);
    if (tier !== "sprout") {
      const words = document.createElement("span");
      words.className = "walk-text";
      words.textContent = "Walk this way";
      this.walkEl.append(words);
    }
    this.walkEl.setAttribute("aria-label", "Walk this way to the next goal");
    this.walkEl.addEventListener("click", () => this.onWalk?.());
    pillRow.append(this.chip, this.walkEl);
    bannerRow.append(this.banner);
    // The first key press or tap after the banner appears tucks it away.
    const onInput = (e: Event) => {
      if (e instanceof KeyboardEvent && e.repeat) return;
      if (this.bannerUp && performance.now() - this.shownAt > INPUT_GRACE_MS) this.shrink();
    };
    window.addEventListener("keydown", onInput, { capture: true });
    window.addEventListener("pointerdown", onInput, { capture: true });
  }

  /** Whether the big banner is on screen (not shrinking away). */
  get bannerUp(): boolean {
    return !this.banner.hidden && !this.banner.classList.contains("shrink");
  }

  /** Another banner is showing or queued (or not): the goal banner waits
   * its turn and the pill steps aside meanwhile. */
  setOthersUp(up: boolean): void {
    if (up === this.othersUp) return;
    this.othersUp = up;
    if (up && !this.banner.hidden) {
      // Make way now; come back once the other banner is gone.
      window.clearTimeout(this.timer);
      this.banner.hidden = true;
      this.pending = true;
    } else if (!up && this.pending) this.show();
    this.sync();
  }

  /** One copy of the goal at a time: the pill hides while any banner is on
   * screen (including the goal banner's own shrink). */
  private sync(): void {
    this.chip.classList.toggle("yield", !this.banner.hidden || this.othersUp);
  }

  /** The current objective line (what the chip says), for other UI. */
  get line(): string {
    return this.text;
  }

  /**
   * Shows the goal for the room the player is in, or the next goal once
   * that room is done. A new goal pops the banner; the same one only keeps
   * the chip up to date.
   */
  /**
   * The next goal is not on screen yet: a big walk arrow takes the chip's
   * place (and the banner waits) until it is. `dir` is 1 for right, -1 for
   * left, 0 to stop walking and show the goal again.
   */
  walk(dir: number): void {
    if (dir === this.walkDir) return;
    this.walkDir = dir;
    this.walkEl.hidden = dir === 0;
    if (dir !== 0) {
      this.walkPic.replaceChildren(icon(dir > 0 ? "right" : "left"));
      this.walkEl.classList.toggle("left", dir < 0);
      // The old goal is done: nothing about it (or the next one) shows yet.
      window.clearTimeout(this.timer);
      this.banner.hidden = true;
      this.pending = false;
      this.chip.hidden = true;
      this.key = "walk";
      this.text = "";
      this.sync();
    }
  }

  /** Whether the walk prompt is up. */
  get walking(): boolean {
    return this.walkDir !== 0;
  }

  update(goal: Goal | null): void {
    if (!goal) {
      this.key = "done";
      this.pending = false;
      this.chip.hidden = true;
      this.banner.hidden = true;
      return;
    }
    const text = this.tier === "sprout" ? goal.text : prettyFormula(goal.text);
    const steps = goal.steps ?? [];
    // Sprout: the trap's fill bar inside the step fills as the gas collects.
    const fill = `${Math.round(Math.max(0, Math.min(1, goal.meter ?? 0)) * 100)}%`;
    for (const bar of this.trapFills) bar.style.width = fill;
    const stepKey = steps.map((s) => (s.done ? "1" : "0")).join("");
    const key = `${goal.room}:${goal.text}:${stepKey}`;
    if (key === this.key) return;
    const ticked = this.key.startsWith(`${goal.room}:${goal.text}:`);
    this.key = key;
    this.text = text;
    this.raw = goal.text;
    for (const pic of [this.bannerPic, this.chipPic]) pic.replaceChildren(icon(GOAL_ICON[goal.kind]));
    // Sprout reads pictures, not words: the words are spoken (and in the
    // aria-label), never printed.
    const shown = this.tier === "sprout" ? "" : text;
    this.bannerText.textContent = shown;
    this.chipText.textContent = shown;
    this.bannerText.hidden = shown === "";
    // The chip is small: a goal in steps shows just its ticked steps.
    this.chipText.hidden = shown === "" || (goal.steps ?? []).length > 0;
    this.trapFills = [];
    // A goal in steps: each step listed with a tick once it is done, so
    // "1. hedge out" visibly comes before "2. light the bowl".
    for (const box of [this.bannerSteps, this.chipSteps]) {
      box.replaceChildren(
        ...steps.map((s) => {
          const el = document.createElement("span");
          el.className = s.done ? "objective-step done" : "objective-step";
          const mark = document.createElement("span");
          mark.className = "step-mark";
          mark.textContent = s.done ? "✓" : "○";
          if (this.tier === "sprout") {
            el.append(mark, this.stepPicture(s.pic, fill));
            return el;
          }
          const words = document.createElement("span");
          words.textContent = prettyFormula(s.text);
          el.append(mark, words);
          return el;
        }),
      );
      box.hidden = steps.length === 0;
    }
    const said = steps.length ? ` Steps: ${steps.map((s) => `${s.text}${s.done ? " (done)" : ""}`).join(", ")}.` : "";
    this.chip.setAttribute("aria-label", `Goal: ${text}.${said} Say it again.`);
    // A step just ticked: the pill already says it, so it only glows (no
    // second, bigger copy of the same words).
    this.chip.classList.toggle("ticked", ticked);
    this.chip.hidden = false;
    if (ticked) {
      this.chip.classList.remove("pop");
      void this.chip.offsetWidth;
      this.chip.classList.add("pop");
      return;
    }
    // Not spoken here: each room already speaks its own entry line.
    this.show();
  }

  /** Fill bars of the "trap" step pictures, kept in step with the meter. */
  private trapFills: HTMLElement[] = [];

  /** A step as a picture: an icon, or for the trap a bar that fills. */
  private stepPicture(pic: string, fill: string): HTMLElement {
    const wrap = document.createElement("span");
    wrap.className = "step-pic";
    if (pic === "trap") {
      const bar = document.createElement("span");
      bar.className = "step-bar";
      const f = document.createElement("span");
      f.style.width = fill;
      bar.append(f);
      this.trapFills.push(f);
      wrap.append(bar);
    } else {
      wrap.append(icon(STEP_ICON[pic] ?? "star"));
    }
    return wrap;
  }

  /** Keeps the big banner back for a moment (a room-clear stamp is showing). */
  holdFor(ms: number): void {
    this.holdUntil = performance.now() + ms;
    if (!this.banner.hidden) this.banner.hidden = true;
    this.sync();
  }

  /** The chip was tapped: big banner again, read aloud again. */
  replay(): void {
    if (!this.text) return;
    this.show();
    this.onAnnounce?.(this.raw);
  }

  private show(): void {
    const wait = this.holdUntil - performance.now();
    window.clearTimeout(this.holdTimer);
    if (wait > 0) {
      this.holdTimer = window.setTimeout(() => this.show(), wait);
      return;
    }
    if (this.othersUp) {
      this.pending = true;
      this.sync();
      return;
    }
    this.pending = false;
    this.banner.hidden = false;
    this.banner.classList.remove("shrink");
    this.chip.classList.remove("pop");
    this.shownAt = performance.now();
    this.sync();
    window.clearTimeout(this.timer);
    this.timer = window.setTimeout(() => this.shrink(), BANNER_MS);
  }

  /** The banner shrinks away and the pill comes back in its place. */
  private shrink(): void {
    window.clearTimeout(this.timer);
    this.banner.classList.add("shrink");
    this.chip.classList.add("pop");
    this.timer = window.setTimeout(() => {
      this.banner.hidden = true;
      this.sync();
    }, SHRINK_MS);
  }
}
