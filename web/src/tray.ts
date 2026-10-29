// Seedling spell building: a tray of picture tiles (molecules, "+" and "->")
// that snap into a spell bar, with a live balance scale that counts the atoms
// on each side and tips until they match. Each molecule in the bar has one
// control for how many: tap its number to count 1, 2, 3, 4 and round again.
//
// The game suggests one spell per room goal. The suggestion goes in once,
// when the goal changes, with a "New spell" pulse. Within one goal, once the
// player taps any tile or number the bar is theirs and is never overwritten.
// A new goal loads its spell (balanced, if the player had edited the bar)
// with an "Undo" chip that brings their own bar back; after Undo the
// suggestion waits on the label, to use with a tap.

import { words } from "./device";
import { ATOM_COLOR, atomsOf, icon, molecule } from "./icons";
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

/**
 * The smallest numbers (1-4 each) that balance these molecules, by token
 * index, or null when no such numbers exist. Used to point at the number
 * that needs changing and to offer spells in their balanced form.
 */
export function balancedCoefs(tokens: Token[]): Map<number, number> | null {
  const mols = tokens.map((t, i) => [t, i] as const).filter(([t]) => t.kind === "mol");
  if (!tokens.some((t) => t.kind === "arrow") || mols.length === 0 || mols.length > MAX_MOLECULES) return null;
  let best: number[] | null = null;
  let bestSum = Infinity;
  const coefs = mols.map(() => 1);
  const trial = tokens.map((t) => ({ ...t }));
  for (;;) {
    const sum = coefs.reduce((a, b) => a + b, 0);
    if (sum < bestSum) {
      mols.forEach(([, i], k) => {
        const t = trial[i]!;
        if (t.kind === "mol") t.coef = coefs[k]!;
      });
      const { left, right } = sideAtoms(trial);
      const els = new Set([...Object.keys(left), ...Object.keys(right)]);
      if (Object.keys(right).length > 0 && [...els].every((e) => (left[e] ?? 0) === (right[e] ?? 0))) {
        best = [...coefs];
        bestSum = sum;
      }
    }
    // Next combination (odometer over 1..MAX_COEF).
    let k = 0;
    while (k < coefs.length && coefs[k] === MAX_COEF) coefs[k++] = 1;
    if (k === coefs.length) break;
    coefs[k]!++;
  }
  if (!best) return null;
  const out = new Map<number, number>();
  mols.forEach(([, i], k) => out.set(i, best[k]!));
  return out;
}

/** A spell in its balanced form ("H2O -> H2 + O2" becomes "2H2O -> 2H2 + O2"). */
export function balancedText(text: string): string {
  const tokens = parseSpell(text);
  const fix = balancedCoefs(tokens);
  if (!fix) return text;
  for (const [i, c] of fix) {
    const t = tokens[i];
    if (t?.kind === "mol") t.coef = c;
  }
  return tokensText(tokens);
}

function tokensText(tokens: Token[]): string {
  let s = "";
  for (const t of tokens) {
    if (t.kind === "plus") s += " + ";
    else if (t.kind === "arrow") s += " -> ";
    else s += `${t.coef > 1 ? t.coef : ""}${t.formula}`;
  }
  return s.trim();
}

const COEF_HINT_KEY = "realgame.coefHintSeen";

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
  /** "Undo": puts back the player's own bar that a new goal's spell replaced. */
  private readonly undoChip: HTMLButtonElement;
  /** The bar the last suggestion replaced ("" when there is nothing to undo). */
  private undoText = "";
  /** The suggestion that replaced it (offered again on the label after Undo). */
  private undoOffer = "";
  private readonly barRow: HTMLElement;
  /** The one-time ghost hand on the number to change (null once shown). */
  private coefHint: HTMLElement | null = null;
  private coefHintSeen = false;
  private coefHintTimer = 0;
  /** Reads a line aloud without a caption (the words are already on screen). */
  sayOnly: ((line: string) => void) | null = null;

  constructor(
    root: HTMLElement,
    private readonly onCast: (text: string) => void,
    private readonly onSpeak: (caption: string) => void,
  ) {
    const barRow = document.createElement("div");
    barRow.className = "bar-row";
    this.barRow = barRow;
    try {
      this.coefHintSeen = window.localStorage.getItem(COEF_HINT_KEY) === "1";
    } catch {
      this.coefHintSeen = false;
    }
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
    this.undoChip = button("Undo", "Undo: bring back my spell", "tray-undo", () => this.undo());
    this.undoChip.id = "tray-undo";
    this.undoChip.hidden = true;
    const cast = button("Cast", "Cast the spell", "tray-cast primary", () => this.cast());
    cast.id = "tray-cast";
    barRow.append(this.newLabel, this.undoChip, this.bar, back, clear, cast);

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
    return tokensText(this.tokens);
  }

  /** The player chose this spell (a fix, a hint's "Use it", Clear): the bar
   * is theirs from now on. */
  set(text: string): void {
    this.touched = true;
    this.offered = "";
    this.hideUndo();
    this.tokens = parseSpell(text);
    this.fixes.clear();
    this.render();
  }

  /**
   * The suggested spell for goal `key`, loaded into the bar once when the
   * goal changes. If the player had edited the bar, it goes in balanced
   * (so one Cast works) with an "Undo" chip that restores their bar. Within
   * one goal an edited bar is never overwritten. Returns true if the bar
   * changed.
   */
  suggest(key: number, text: string): boolean {
    if (key === this.suggestKey) return false;
    this.suggestKey = key;
    const mine = this.touched ? this.text() : "";
    const next = this.touched ? balancedText(text) : text;
    this.touched = false;
    this.offered = "";
    this.hideUndo();
    if (mine && parseSpellText(next) === mine) {
      this.hideNew();
      return false;
    }
    this.tokens = parseSpell(next);
    this.fixes.clear();
    this.render();
    this.showNew("New spell", false);
    if (mine) {
      this.undoText = mine;
      this.undoOffer = next;
      this.undoChip.hidden = false;
    }
    return true;
  }

  /** Undo: the player's own bar comes back; the suggestion waits on the label. */
  private undo(): void {
    const offer = this.undoOffer;
    this.tokens = parseSpell(this.undoText);
    this.touched = true;
    this.fixes.clear();
    this.hideUndo();
    this.render();
    if (offer) {
      this.offered = offer;
      this.showNew(`New spell: ${prettyFormula(offer)}`, true);
    }
  }

  private hideUndo(): void {
    this.undoChip.hidden = true;
    this.undoText = "";
    this.undoOffer = "";
  }

  /**
   * The player cast something that cannot move the goal while a suggestion
   * waits: the label flashes as a big button with the ghost hand on it.
   * Returns true if there was an offer to show.
   */
  nudgeOffer(): boolean {
    if (!this.offered) return false;
    const label = this.newLabel;
    label.classList.add("big");
    if (!label.querySelector(".tray-new-hand")) {
      const hand = document.createElement("span");
      hand.className = "tray-new-hand ghost-tap";
      hand.setAttribute("aria-hidden", "true");
      hand.append(icon("hand"));
      label.append(hand);
    }
    label.classList.remove("pulse");
    void label.offsetWidth;
    label.classList.add("pulse");
    return true;
  }

  /** A fresh start (restart): suggestions apply again. */
  reset(): void {
    this.touched = false;
    this.suggestKey = -1;
    this.offered = "";
    this.hideUndo();
    this.hideNew();
  }

  private showNew(text: string, offer: boolean): void {
    this.newLabel.textContent = text;
    this.newLabel.classList.remove("big");
    this.newLabel.setAttribute("aria-label", offer ? `${text}. Tap to use it.` : text);
    this.newLabel.classList.toggle("offer", offer);
    this.newLabel.hidden = false;
    this.newLabel.classList.remove("pulse");
    void this.newLabel.offsetWidth;
    this.newLabel.classList.add("pulse");
    window.clearTimeout(this.newTimer);
    // An offer keeps its slot until it is used or replaced, so walking away
    // never loses it; the plain "New spell" note fades.
    if (!offer) this.newTimer = window.setTimeout(() => this.hideNew(), 2600);
  }

  /** The offer waiting on the chip ("" for none). */
  get offer(): string {
    return this.offered;
  }

  private hideNew(): void {
    window.clearTimeout(this.newTimer);
    this.newLabel.hidden = true;
    this.offered = "";
  }

  /** The player edited the bar by hand. */
  private touch(): void {
    this.touched = true;
    this.hideUndo();
    if (!this.offered) this.hideNew();
    else if (parseSpellText(this.offered) === this.text()) this.hideNew();
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
    this.dropCoefHint();
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
    this.maybeCoefHint();
  }

  /**
   * The first time the scale is unbalanced, a ghost hand taps the number
   * that needs changing: "Tap a number to add more!". Shown once, and only
   * while the tray is on screen (call again when it opens).
   */
  maybeCoefHint(): void {
    if (this.coefHint) {
      if (this.scale.dataset.state === "tipped") this.placeCoefHint();
      else this.dropCoefHint();
      return;
    }
    if (this.coefHintSeen || this.scale.dataset.state !== "tipped") return;
    if (this.barRow.offsetParent === null || this.barRow.getBoundingClientRect().top >= window.innerHeight) return;
    const fix = balancedCoefs(this.tokens);
    let target = -1;
    if (fix) for (const [i, c] of fix) if (target < 0 && this.tokens[i]?.kind === "mol" && (this.tokens[i] as { coef: number }).coef !== c) target = i;
    if (target < 0) target = this.tokens.findIndex((t) => t.kind === "mol");
    if (target < 0) return;
    this.coefHintSeen = true;
    try {
      window.localStorage.setItem(COEF_HINT_KEY, "1");
    } catch {
      // Private windows: the hint may show again next time, which is fine.
    }
    const hint = document.createElement("div");
    hint.className = "coef-hint";
    hint.dataset.index = String(target);
    hint.setAttribute("role", "note");
    const hand = document.createElement("span");
    hand.className = "coef-hand";
    hand.setAttribute("aria-hidden", "true");
    hand.append(icon("hand"));
    const say = document.createElement("span");
    say.className = "coef-say";
    say.textContent = words("Tap a number to add more!");
    hint.append(hand, say);
    this.barRow.append(hint);
    this.coefHint = hint;
    this.placeCoefHint();
    // The bubble shows the words; they are read aloud, not captioned twice.
    (this.sayOnly ?? this.onSpeak)(words("Tap a number to add more!"));
    this.coefHintTimer = window.setTimeout(() => this.dropCoefHint(), 12000);
  }

  private placeCoefHint(): void {
    const hint = this.coefHint;
    if (!hint) return;
    const btn = this.bar.querySelector<HTMLElement>(`.bar-coef[data-index="${hint.dataset.index ?? ""}"]`);
    if (!btn) {
      this.dropCoefHint();
      return;
    }
    const row = this.barRow.getBoundingClientRect();
    const b = btn.getBoundingClientRect();
    hint.style.left = `${Math.round(b.left - row.left + b.width / 2)}px`;
    hint.style.top = `${Math.round(b.top - row.top + b.height / 2)}px`;
  }

  private dropCoefHint(): void {
    window.clearTimeout(this.coefHintTimer);
    this.coefHint?.remove();
    this.coefHint = null;
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
