// Free play after the trial: a panel with a "Reset room" button for each
// room (the simulation rebuilds that room's hedge, ice, gas trap or brazier
// from the seed) and the seeded challenge list, ticked as the simulation
// reports them done. Presentation only: the simulation decides when a
// challenge is complete.

import { icon, type IconName } from "./icons";
import type { Tier } from "./types";

export interface Challenge {
  id: string;
  text: string;
  pic: IconName;
  room: number;
  done: boolean;
  unlocks: string | null;
}

export interface FreePlayState {
  challenges: Challenge[];
  unlocked: string[];
}

export class FreePlayPanel {
  readonly el: HTMLElement;
  private readonly toggle: HTMLButtonElement;
  private readonly list: HTMLUListElement;
  private key = "";

  constructor(
    stage: HTMLElement,
    private readonly tier: Tier,
    onReset: (room: number) => void,
    private readonly onSay: (line: string) => void,
  ) {
    this.toggle = document.createElement("button");
    this.toggle.type = "button";
    this.toggle.id = "free-btn";
    this.toggle.hidden = true;
    this.toggle.setAttribute("aria-expanded", "true");
    this.toggle.setAttribute("aria-controls", "free-panel");
    this.toggle.setAttribute("aria-label", "Challenges");
    this.toggle.append(icon("star"));
    this.toggle.addEventListener("click", () => this.setOpen(this.el.hidden));

    this.el = document.createElement("section");
    this.el.id = "free-panel";
    this.el.hidden = true;
    this.el.setAttribute("aria-label", "Free play challenges");
    const title = document.createElement("h2");
    title.textContent = tier === "sprout" ? "" : "Challenges";
    title.append(icon("star"));
    this.list = document.createElement("ul");
    this.list.id = "free-challenges";
    const resets = document.createElement("div");
    resets.className = "free-resets";
    resets.setAttribute("role", "group");
    resets.setAttribute("aria-label", "Reset a room");
    const resetLabel = document.createElement("span");
    resetLabel.className = "free-resets-label";
    resetLabel.textContent = tier === "sprout" ? "" : "Reset room";
    resetLabel.setAttribute("aria-hidden", "true");
    resets.append(resetLabel);
    for (let k = 0; k < 4; k++) {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "free-reset";
      b.dataset.room = String(k);
      const label = `Reset room ${k + 1}`;
      b.setAttribute("aria-label", label);
      const num = document.createElement("span");
      num.className = "free-reset-num";
      num.textContent = String(k + 1);
      b.title = label;
      b.append(icon("restart"), num);
      b.addEventListener("click", () => {
        onReset(k);
        b.blur();
      });
      if (tier === "sprout") b.addEventListener("pointerdown", () => this.onSay(label));
      resets.append(b);
    }
    this.el.append(title, this.list, resets);
    stage.append(this.toggle, this.el);
  }

  show(): void {
    this.toggle.hidden = false;
    this.setOpen(true);
  }

  hide(): void {
    this.toggle.hidden = true;
    this.el.hidden = true;
    this.key = "";
  }

  setOpen(open: boolean): void {
    this.el.hidden = !open;
    this.toggle.setAttribute("aria-expanded", String(open));
  }

  /** Redraws the list when anything changed. */
  render(state: FreePlayState): void {
    const key = JSON.stringify(state.challenges.map((c) => [c.id, c.done, c.text]));
    if (key === this.key) return;
    this.key = key;
    const done = state.challenges.filter((c) => c.done).length;
    this.toggle.dataset.count = `${done}/${state.challenges.length}`;
    this.list.replaceChildren(
      ...state.challenges.map((c) => {
        const li = document.createElement("li");
        li.className = "free-challenge";
        li.dataset.id = c.id;
        li.classList.toggle("done", c.done);
        const pic = document.createElement("span");
        pic.className = "free-pic";
        pic.append(icon(c.pic));
        const text = document.createElement("span");
        text.className = "free-text";
        text.textContent = c.text;
        const tick = document.createElement("span");
        tick.className = "free-tick";
        if (c.done) tick.append(icon("check"));
        li.append(pic, text, tick);
        li.setAttribute("aria-label", `${c.text}${c.done ? " (done)" : ""}`);
        if (this.tier === "sprout") li.addEventListener("pointerdown", () => this.onSay(c.text));
        return li;
      }),
    );
  }
}
