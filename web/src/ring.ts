// The formula ring: tiles to build an equation, coefficient dials, and a text
// box for players who would rather type.

import { words } from "./device";
import { icon, molecule, type IconName } from "./icons";
import { prettyFormula, type Tier } from "./types";

interface Term {
  formula: string;
  coef: number;
}

interface TileDef {
  formula: string;
  name: string;
  /** A picture icon, or null to draw the molecule as atom balls. */
  picture: IconName | null;
}

// Pictures are drawn, not emoji: methane gets its own molecule icon, never a fire.
const TILES: TileDef[] = [
  { formula: "H2O", name: "Water", picture: "drop" },
  { formula: "H2", name: "Hydrogen", picture: null },
  { formula: "O2", name: "Oxygen", picture: null },
  { formula: "CH4", name: "Methane", picture: "methane" },
  { formula: "CO2", name: "Carbon dioxide", picture: "cloud" },
];

export class FormulaRing {
  private left: Term[] = [];
  private right: Term[] = [];
  private onRight = false;
  private readonly root: HTMLElement;
  private readonly input: HTMLInputElement;
  private readonly equation: HTMLElement;
  private readonly arrowBtn: HTMLButtonElement;

  constructor(
    tier: Tier,
    private readonly onCast: (formula: string) => void,
  ) {
    this.root = byId("ring");
    this.input = byId<HTMLInputElement>("formula");
    this.equation = byId("equation");
    this.arrowBtn = byId<HTMLButtonElement>("arrow-btn");

    const tiles = byId("tiles");
    for (const t of TILES) {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "tile";
      b.title = t.name;
      b.setAttribute("aria-label", `${t.name}, ${t.formula}`);
      const pic = document.createElement("span");
      pic.className = "pic";
      pic.append(t.picture ? icon(t.picture) : molecule(t.formula));
      const label = document.createElement("span");
      label.textContent = tier === "sprout" ? t.name : prettyFormula(t.formula);
      b.append(pic, label);
      b.addEventListener("click", () => this.addTile(t.formula));
      tiles.append(b);
    }

    // The youngest players build with pictures only; typing comes later.
    this.input.hidden = tier === "sprout";
    this.input.addEventListener("input", () => this.fromText(this.input.value));
    this.input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        this.cast();
      } else if (e.key === "Escape") {
        this.toggle(false);
      }
      // Typing letters like Q or N must not trigger game shortcuts.
      e.stopPropagation();
    });
    this.arrowBtn.addEventListener("click", () => {
      this.onRight = !this.onRight;
      this.sync();
    });
    byId("clear-btn").addEventListener("click", () => this.clear());
    byId("cast-btn").addEventListener("click", () => this.cast());
    this.sync();
  }

  get isOpen(): boolean {
    return !this.root.hidden;
  }

  toggle(open = !this.isOpen): void {
    this.root.hidden = !open;
    if (open && !this.input.hidden) this.input.focus();
    if (!open) this.input.blur();
  }

  text(): string {
    const side = (ts: Term[]) => ts.map((t) => (t.coef > 1 ? `${t.coef}${t.formula}` : t.formula)).join(" + ");
    if (this.right.length === 0 && !this.onRight) return side(this.left);
    return `${side(this.left)} -> ${side(this.right)}`;
  }

  set(formula: string): void {
    this.fromText(formula);
    this.input.value = formula;
    this.sync();
  }

  cast(): void {
    const t = this.text().trim();
    if (t) this.onCast(t);
  }

  private clear(): void {
    this.left = [];
    this.right = [];
    this.onRight = false;
    this.input.value = "";
    this.sync();
  }

  private addTile(formula: string): void {
    const side = this.onRight ? this.right : this.left;
    const existing = side.find((t) => t.formula === formula);
    if (existing) existing.coef = Math.min(existing.coef + 1, 9);
    else side.push({ formula, coef: 1 });
    this.input.value = this.text();
    this.sync();
  }

  private fromText(text: string): void {
    const [l = "", r] = text.replace(/→/g, "->").split("->");
    const parse = (s: string): Term[] =>
      s
        .split("+")
        .map((p) => p.trim())
        .filter(Boolean)
        .map((p) => {
          const m = /^(\d*)(.+)$/.exec(p);
          return { coef: m?.[1] ? Number(m[1]) : 1, formula: m?.[2] ?? p };
        });
    this.left = parse(l);
    this.right = r === undefined ? [] : parse(r);
    this.onRight = r !== undefined;
    this.sync(false);
  }

  /** Redraws the equation as chips with up/down dials. */
  private sync(writeInput = true): void {
    if (writeInput && document.activeElement !== this.input) this.input.value = this.text();
    this.arrowBtn.textContent = this.onRight ? "Back to left side" : "Add arrow →";
    this.equation.replaceChildren();
    const addSide = (terms: Term[], label: string) => {
      if (terms.length === 0) {
        const hint = document.createElement("span");
        hint.className = "hint";
        hint.textContent = label;
        this.equation.append(hint);
      }
      terms.forEach((t, i) => {
        if (i > 0) this.equation.append(textSpan(" + ", "op"));
        const chip = document.createElement("span");
        chip.className = "chip";
        const down = dial("−", `Fewer ${t.formula}`, () => {
          t.coef -= 1;
          if (t.coef < 1) terms.splice(terms.indexOf(t), 1);
          this.sync();
        });
        const up = dial("+", `More ${t.formula}`, () => {
          t.coef = Math.min(t.coef + 1, 9);
          this.sync();
        });
        chip.append(down, textSpan(`${t.coef > 1 ? t.coef : ""}${prettyFormula(t.formula)}`, "term"), up);
        this.equation.append(chip);
      });
    };
    addSide(this.left, words("Tap substances to start"));
    if (this.onRight || this.right.length > 0) {
      this.equation.append(textSpan(" → ", "op arrow"));
      addSide(this.right, "What do they make?");
    }
  }
}

function dial(label: string, aria: string, onClick: () => void): HTMLButtonElement {
  const b = document.createElement("button");
  b.type = "button";
  b.className = "dial";
  b.textContent = label;
  b.setAttribute("aria-label", aria);
  b.addEventListener("click", onClick);
  return b;
}

function textSpan(text: string, cls: string): HTMLSpanElement {
  const s = document.createElement("span");
  s.className = cls;
  s.textContent = text;
  return s;
}

export function byId<T extends HTMLElement = HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Missing #${id}`);
  return el as T;
}
