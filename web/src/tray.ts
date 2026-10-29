// Seedling spell building: a tray of picture tiles (molecules, "+" and "->")
// that snap into a spell bar, with a live balance scale that counts the atoms
// on each side and tips until they match. Each molecule in the bar has one
// control for how many: tap its number to count 1, 2, 3, 4 and round again.
//
// The game suggests one spell per room goal. The suggestion goes in once,
// when the goal changes, with a "New spell" pulse; once the player taps any
// tile or number the bar is theirs and is never overwritten (a later
// suggestion waits on the label, to use with a tap).

import { words } from "./device";
import { ATOM_COLOR, atomsOf, molecule } from "./icons";
import { prettyFormula } from "./types";

type Token = { kind: "mol"; formula: string; coef: number } | { kind: "plus" } | { kind: "arrow" };

const MOLECULES: { formula: string; name: string }[] = [
  { formula: "H2", name: "hydrogen" },
  { formula: "O2", name: "oxygen" },
  { formula: "H2O", name: "water" },
  { formula: "CO2", name: "carbon dioxide" },
];

/** Atom totals per side of a token list. */
export function sideAtoms(tokens: Token[]): { left: Record<string, number>; right: Record<string, number>; arrow: boolean } {
  const left: Record<string, number> = {};
  const right: Record<string, number> = {};
  let side = left;
  let arrow = false;
  for (const t of tokens) {
    if (t.kind === "arrow") {
      side = right;
      arrow = true;
    } else if (t.kind === "mol") {
      for (const [el, n] of Object.entries(atomsOf(t.formula))) side[el] = (side[el] ?? 0) + n * t.coef;
    }
  }
  return { left, right, arrow };
}

export function parseSpell(text: string): Token[] {
  const out: Token[] = [];
  const parts = text.replace(/→/g, "->").split("->");
  parts.forEach((part, i) => {
    if (i > 0) out.push({ kind: "arrow" });
    part
      .split("+")
      .map((p) => p.trim())
      .filter(Boolean)
      .forEach((p, j) => {
        if (j > 0) out.push({ kind: "plus" });
        const m = /^(\d*)(.+)$/.exec(p);
        out.push({ kind: "mol", formula: m?.[2] ?? p, coef: m?.[1] ? Number(m[1]) : 1 });
      });
  });
  return out;
}

/** Most molecules a spell can hold (the game's reactions use at most four). */
const MAX_MOLECULES = 5;
/** Largest number the per-molecule counter goes to before wrapping to 1. */
const MAX_COEF = 4;

export class SpellTray {
  private tokens: Token[] = [];
  /** Coefficients a one-tap fix would set, by molecule token index. */
  private fixes = new Map<number, number>();
  private readonly bar: HTMLElement;
  private readonly scale: HTMLElement;
  private readonly beam: HTMLElement;
  private readonly pans: [HTMLElement, HTMLElement];
  private readonly scaleLabel: HTMLElement;
  private readonly molTiles = new Map<string, HTMLButtonElement>();
  /** The "New spell" label over the bar (a button when it offers a swap). */
  private readonly newLabel: HTMLButtonElement;
  private newTimer = 0;
  /** The player has edited the bar since the last suggestion went in. */
  private touched = false;
  /** Which goal the current suggestion is for (-1: none yet). */
  private suggestKey = -1;
  /** A suggestion waiting on the label while the player's own spell stays. */
  private offered = "";

  constructor(
    root: HTMLElement,
    private readonly onCast: (text: string) => void,
    private readonly onSpeak: (caption: string) => void,
  ) {
    const barRow = document.createElement("div");
    barRow.className = "bar-row";
    this.bar = document.createElement("div");
    this.bar.id = "spellbar";
    this.bar.setAttribute("aria-label", "Your spell");
    this.bar.setAttribute("aria-live", "polite");
    const back = button("⌫", "Remove last tile", "tray-key", () => this.backspace());
    const clear = button("Clear", "Clear the spell", "tray-key", () => this.set(""));
    this.newLabel = button("New spell", "New spell", "tray-new", () => {
      if (this.offered) this.set(this.offered);
      this.hideNew();
    });
    this.newLabel.hidden = true;
    const cast = button("Cast", "Cast the spell", "tray-cast primary", () => this.cast());
    cast.id = "tray-cast";
    barRow.append(this.newLabel, this.bar, back, clear, cast);

    // The balance scale: a beam with a pan of atoms at each end.
    this.scale = document.createElement("div");
    this.scale.id = "scale";
    this.scale.setAttribute("role", "img");
    this.beam = document.createElement("div");
    this.beam.className = "beam";
    const panL = document.createElement("div");
    panL.className = "pan left";
    const panR = document.createElement("div");
    panR.className = "pan right";
    this.pans = [panL, panR];
    this.beam.append(panL, panR);
    const post = document.createElement("div");
    post.className = "post";
    this.scaleLabel = document.createElement("div");
    this.scaleLabel.className = "scale-label";
    // What the two sides mean, in words a Seedling knows (each pan's name
    // hangs under it, see fillPan).
    const caption = document.createElement("div");
    caption.className = "scale-caption";
    caption.textContent = "Atoms in = atoms out";
    this.scale.append(caption, this.beam, post, this.scaleLabel);

    const tiles = document.createElement("div");
    tiles.id = "tray-tiles";
    for (const m of MOLECULES) {
      const b = button("", `${m.name}, ${m.formula}`, "tray-tile mol", () => this.addMolecule(m.formula));
      const f = document.createElement("span");
      f.className = "formula";
      f.textContent = prettyFormula(m.formula);
      b.append(molecule(m.formula), f);
      b.dataset.formula = m.formula;
      tiles.append(b);
      this.molTiles.set(m.formula, b);
    }
    tiles.append(
      button("+", "plus", "tray-tile op", () => this.addPlus()),
      button("→", "makes", "tray-tile op", () => this.addArrow()),
    );

    root.append(barRow, this.scale, tiles);
    this.render();
  }

  text(): string {
    let s = "";
    for (const t of this.tokens) {
      if (t.kind === "plus") s += " + ";
      else if (t.kind === "arrow") s += " -> ";
      else s += `${t.coef > 1 ? t.coef : ""}${t.formula}`;
    }
    return s.trim();
  }

  /** The player chose this spell (a fix, a hint's "Use it", Clear): the bar
   * is theirs from now on. */
  set(text: string): void {
    this.touched = true;
    this.offered = "";
    this.tokens = parseSpell(text);
    this.fixes.clear();
    this.render();
  }

  /**
   * The suggested spell for goal `key`. Applied once per goal, in one go,
   * and only while the player hasn't edited the bar; otherwise it waits on
   * the "New spell" label. Returns true if the bar changed.
   */
  suggest(key: number, text: string): boolean {
    if (key === this.suggestKey) return false;
    this.suggestKey = key;
    if (this.touched) {
      if (parseSpellText(text) === this.text()) return false;
      this.offered = text;
      this.showNew(`New spell: ${prettyFormula(text)}`, true);
      return false;
    }
    this.tokens = parseSpell(text);
    this.fixes.clear();
    this.render();
    this.showNew("New spell", false);
    return true;
  }

  /** A fresh start (restart): suggestions apply again. */
  reset(): void {
    this.touched = false;
    this.suggestKey = -1;
    this.offered = "";
    this.hideNew();
  }

  private showNew(text: string, offer: boolean): void {
    this.newLabel.textContent = text;
    this.newLabel.setAttribute("aria-label", offer ? `${text}. Tap to use it.` : text);
    this.newLabel.classList.toggle("offer", offer);
    this.newLabel.hidden = false;
    this.newLabel.classList.remove("pulse");
    void this.newLabel.offsetWidth;
    this.newLabel.classList.add("pulse");
    window.clearTimeout(this.newTimer);
    this.newTimer = window.setTimeout(() => this.hideNew(), offer ? 9000 : 2600);
  }

  private hideNew(): void {
    window.clearTimeout(this.newTimer);
    this.newLabel.hidden = true;
    this.offered = "";
  }

  /** The player edited the bar by hand. */
  private touch(): void {
    this.touched = true;
    if (!this.offered) this.hideNew();
  }

  cast(): void {
    const t = this.text();
    if (t) this.onCast(t);
  }

  /**
   * After a partial cast with a suggested fix, pulse the coefficients that
   * would balance it. Tapping one sets it.
   */
  offerFix(suggestion: string): void {
    const want = parseSpell(suggestion).filter((t) => t.kind === "mol");
    const have = this.tokens.map((t, i) => [t, i] as const).filter(([t]) => t.kind === "mol");
    this.fixes.clear();
    if (want.length !== have.length) return;
    have.forEach(([t, i], k) => {
      const w = want[k]!;
      if (t.kind === "mol" && w.kind === "mol" && w.formula === t.formula && w.coef !== t.coef) this.fixes.set(i, w.coef);
    });
    this.render();
  }

  /** Hint level 2+: glow the tiles a spell needs. */
  setHint(spell: string | null): void {
    const need = new Set<string>();
    if (spell) for (const t of parseSpell(spell)) if (t.kind === "mol") need.add(t.formula);
    for (const [f, b] of this.molTiles) b.classList.toggle("hint", need.has(f));
  }

  private addMolecule(formula: string): void {
    this.touch();
    if (this.tokens.filter((t) => t.kind === "mol").length >= MAX_MOLECULES) {
      this.onSpeak(words("Your spell is full. Tap Clear to start again."));
      return;
    }
    const last = this.tokens[this.tokens.length - 1];
    // Two molecules in a row on one side are joined with "+".
    if (last?.kind === "mol") this.tokens.push({ kind: "plus" });
    this.tokens.push({ kind: "mol", formula, coef: 1 });
    this.changed();
  }

  private addPlus(): void {
    this.touch();
    const last = this.tokens[this.tokens.length - 1];
    if (last?.kind === "mol") this.tokens.push({ kind: "plus" });
    this.changed();
  }

  private addArrow(): void {
    this.touch();
    if (this.tokens.some((t) => t.kind === "arrow")) return;
    const last = this.tokens[this.tokens.length - 1];
    if (last?.kind === "plus") this.tokens.pop();
    this.tokens.push({ kind: "arrow" });
    this.changed();
  }

  private backspace(): void {
    this.touch();
    this.tokens.pop();
    this.changed();
  }

  /** The one "how many" control: this molecule only, 1 -> 2 -> 3 -> 4 -> 1. */
  private cycleCoef(i: number): void {
    const t = this.tokens[i];
    if (t?.kind !== "mol") return;
    this.touch();
    t.coef = (t.coef % MAX_COEF) + 1;
    // A pulsing fix on this molecule is answered (either way) once tapped.
    this.fixes.delete(i);
    this.render();
    if (this.balanced()) this.onSpeak("Balanced!");
  }

  private changed(): void {
    this.fixes.clear();
    this.render();
  }

  balanced(): boolean {
    const { left, right, arrow } = sideAtoms(this.tokens);
    if (!arrow || Object.keys(right).length === 0) return false;
    const els = new Set([...Object.keys(left), ...Object.keys(right)]);
    return [...els].every((e) => (left[e] ?? 0) === (right[e] ?? 0));
  }

  private render(): void {
    this.bar.replaceChildren();
    this.tokens.forEach((t, i) => {
      if (t.kind === "plus") this.bar.append(chip("+", "op"));
      else if (t.kind === "arrow") this.bar.append(chip("→", "op arrow"));
      else {
        const c = document.createElement("span");
        c.className = "bar-term";
        // Always the molecule's own number; a suggested fix only pulses it.
        const coef = button(String(t.coef), `How many ${t.formula}: ${t.coef}. Tap for more.`, "bar-coef", () => this.cycleCoef(i));
        if (this.fixes.has(i)) coef.classList.add("fix");
        if (t.coef === 1) coef.classList.add("one");
        coef.dataset.index = String(i);
        const f = document.createElement("span");
        f.className = "bar-formula";
        f.textContent = prettyFormula(t.formula);
        c.append(coef, molecule(t.formula), f);
        this.bar.append(c);
      }
    });
    if (this.tokens.length === 0) this.bar.append(chip(words("Tap tiles to build a spell"), "empty"));
    this.renderScale();
  }

  private renderScale(): void {
    const { left, right, arrow } = sideAtoms(this.tokens);
    const count = (a: Record<string, number>) => Object.values(a).reduce((x, y) => x + y, 0);
    const [nl, nr] = [count(left), count(right)];
    const balanced = this.balanced();
    let state: string;
    let angle = 0;
    if (!arrow) {
      state = "pour";
      this.scaleLabel.textContent = nl ? "Pour it out" : "";
    } else if (balanced) {
      state = "balanced";
      this.scaleLabel.textContent = "Balanced!";
    } else {
      state = "tipped";
      // The side with more atoms sinks (negative turns the left end down);
      // a mismatch with equal totals still tips a little so it never looks level.
      angle = nl === nr ? 4 : Math.max(-14, Math.min(14, (nr - nl) * 3));
      const extra = Object.keys({ ...left, ...right })
        .filter((e) => (left[e] ?? 0) !== (right[e] ?? 0))
        .map((e) => e);
      this.scaleLabel.textContent = `Not balanced: count the ${extra.join(" and ")}`;
    }
    this.scale.dataset.state = state;
    this.beam.style.transform = `rotate(${angle}deg)`;
    this.scale.setAttribute("aria-label", `Atoms in equal atoms out. ${this.scaleLabel.textContent}. Before ${describe(left)}. After ${describe(right)}.`);
    this.fillPan(this.pans[0], left);
    this.fillPan(this.pans[1], right);
  }

  private fillPan(pan: HTMLElement, atoms: Record<string, number>): void {
    const title = document.createElement("span");
    title.className = "pan-title";
    title.textContent = pan === this.pans[0] ? "Before" : "After";
    pan.replaceChildren(title);
    for (const [el, n] of Object.entries(atoms)) {
      const group = document.createElement("span");
      group.className = "pan-group";
      for (let i = 0; i < Math.min(n, 12); i++) {
        const b = document.createElement("span");
        b.className = `ball el-${el.toLowerCase()}`;
        b.style.background = ATOM_COLOR[el] ?? "#cc79a7";
        group.append(b);
      }
      const label = document.createElement("span");
      label.className = "pan-count";
      label.textContent = `${n} ${el}`;
      group.append(label);
      pan.append(group);
    }
  }
}

/** A spell written the way the bar writes it ("2H2 + O2 -> 2H2O"). */
function parseSpellText(text: string): string {
  let s = "";
  for (const t of parseSpell(text)) {
    if (t.kind === "plus") s += " + ";
    else if (t.kind === "arrow") s += " -> ";
    else s += `${t.coef > 1 ? t.coef : ""}${t.formula}`;
  }
  return s.trim();
}

function describe(a: Record<string, number>): string {
  const parts = Object.entries(a).map(([el, n]) => `${n} ${el}`);
  return parts.length ? parts.join(", ") : "empty";
}

function chip(text: string, cls: string): HTMLSpanElement {
  const s = document.createElement("span");
  s.className = `bar-chip ${cls}`;
  s.textContent = text;
  return s;
}

function button(text: string, aria: string, cls: string, onClick: () => void): HTMLButtonElement {
  const b = document.createElement("button");
  b.type = "button";
  b.className = cls;
  if (text) b.textContent = text;
  b.setAttribute("aria-label", aria);
  b.addEventListener("click", onClick);
  return b;
}
