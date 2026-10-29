// The room objective: when the player reaches a room (or its goal changes),
// a big banner with the goal's picture and one line shows in the middle of
// the stage, then shrinks into a goal chip at the top that stays. Tapping
// the chip shows (and says) the goal again. Separate from Mote's captions,
// so every tier always sees what the room wants, whatever the caption
// setting. Presentation only.

import { icon, type IconName } from "./icons";
import type { Goal, Tier } from "./types";
import { prettyFormula } from "./types";

const GOAL_ICON: Record<Goal["kind"], IconName> = {
  fire: "fire",
  ice: "ice",
  gas: "bubbles",
  brazier: "bowl",
};

/** How long the big banner stays before it shrinks into the chip. */
const BANNER_MS = 3200;

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
  /** The goal line as the simulation wrote it (for speech). */
  private raw = "";
  /** Called when the player asks for the goal again; the caller speaks it. */
  onAnnounce: ((line: string) => void) | null = null;

  constructor(
    stage: HTMLElement,
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
    stage.append(this.banner, this.chip);
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
  update(goal: Goal | null): void {
    if (!goal) {
      this.key = "done";
      this.chip.hidden = true;
      this.banner.hidden = true;
      return;
    }
    const text = this.tier === "sprout" ? goal.text : prettyFormula(goal.text);
    const steps = goal.steps ?? [];
    const stepKey = steps.map((s) => (s.done ? "1" : "0")).join("");
    const key = `${goal.room}:${goal.text}:${stepKey}`;
    if (key === this.key) return;
    const ticked = this.key.startsWith(`${goal.room}:${goal.text}:`);
    this.key = key;
    this.text = text;
    this.raw = goal.text;
    for (const pic of [this.bannerPic, this.chipPic]) pic.replaceChildren(icon(GOAL_ICON[goal.kind]));
    this.bannerText.textContent = text;
    this.chipText.textContent = text;
    // The chip is small: a goal in steps shows just its ticked steps.
    this.chipText.hidden = (goal.steps ?? []).length > 0;
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
          const words = document.createElement("span");
          words.textContent = this.tier === "sprout" ? s.text : prettyFormula(s.text);
          el.append(mark, words);
          return el;
        }),
      );
      box.hidden = steps.length === 0;
    }
    const said = steps.length ? ` Steps: ${steps.map((s) => `${s.text}${s.done ? " (done)" : ""}`).join(", ")}.` : "";
    this.chip.setAttribute("aria-label", `Goal: ${text}.${said} Say it again.`);
    // A step just ticked: the chip glows; the banner comes back to show it.
    this.chip.classList.toggle("ticked", ticked);
    this.chip.hidden = false;
    // Not spoken here: each room already speaks its own entry line.
    this.show();
  }

  /** Keeps the big banner back for a moment (a room-clear stamp is showing). */
  holdFor(ms: number): void {
    this.holdUntil = performance.now() + ms;
    if (!this.banner.hidden) this.banner.hidden = true;
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
    this.banner.hidden = false;
    this.banner.classList.remove("shrink");
    this.chip.classList.remove("pop");
    window.clearTimeout(this.timer);
    this.timer = window.setTimeout(() => {
      this.banner.classList.add("shrink");
      this.chip.classList.add("pop");
      this.timer = window.setTimeout(() => (this.banner.hidden = true), 450);
    }, BANNER_MS);
  }
}
