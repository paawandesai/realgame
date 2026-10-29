// Small picture icons drawn as inline SVG with DOM calls (no innerHTML, no
// image files, no emoji fonts needed), so a child who can't read still sees
// a drop, bubbles, a burst or a cloud on every device.

const NS = "http://www.w3.org/2000/svg";

type Shape = [tag: string, attrs: Record<string, string | number>];

function svg(shapes: Shape[], viewBox = "0 0 48 48"): SVGSVGElement {
  const root = document.createElementNS(NS, "svg");
  root.setAttribute("viewBox", viewBox);
  root.setAttribute("aria-hidden", "true");
  root.setAttribute("focusable", "false");
  root.classList.add("icon");
  for (const [tag, attrs] of shapes) {
    const el = document.createElementNS(NS, tag);
    for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
    root.append(el);
  }
  return root;
}

const DROP: Shape[] = [
  ["path", { d: "M24 4 C24 4 9 22 9 31 a15 15 0 0 0 30 0 C39 22 24 4 24 4 Z", fill: "#56b4e9", stroke: "#e6f6ff", "stroke-width": 2 }],
  ["ellipse", { cx: 18, cy: 30, rx: 3, ry: 6, fill: "#e6f6ff", opacity: 0.7 }],
];

const BUBBLES: Shape[] = [
  ["circle", { cx: 17, cy: 30, r: 10, fill: "none", stroke: "#0072b2", "stroke-width": 4 }],
  ["circle", { cx: 33, cy: 34, r: 7, fill: "none", stroke: "#0072b2", "stroke-width": 4 }],
  ["circle", { cx: 30, cy: 14, r: 7, fill: "none", stroke: "#d55e00", "stroke-width": 4 }],
];

const BOOM: Shape[] = [
  [
    "path",
    {
      d: "M24 2 L29 16 L44 10 L34 22 L46 30 L31 31 L32 46 L24 34 L15 46 L16 31 L2 30 L14 22 L4 10 L19 16 Z",
      fill: "#e69f00",
      stroke: "#f0e442",
      "stroke-width": 2,
      "stroke-linejoin": "round",
    },
  ],
  ["circle", { cx: 24, cy: 25, r: 6, fill: "#f0e442" }],
];

const CLOUD: Shape[] = [
  [
    "path",
    {
      d: "M13 38 a9 9 0 0 1 0-18 a12 12 0 0 1 23-3 a9 9 0 0 1 1 21 Z",
      fill: "#cfd6dd",
      stroke: "#ffffff",
      "stroke-width": 2,
    },
  ],
  ["path", { d: "M14 42 l-2 4 M24 42 l-2 4 M34 42 l-2 4", stroke: "#9aa5b1", "stroke-width": 3, "stroke-linecap": "round" }],
];

const FIRE: Shape[] = [
  ["path", { d: "M24 4 C30 14 38 18 38 30 a14 14 0 0 1 -28 0 C10 22 16 18 18 10 C20 16 22 18 24 18 C24 12 22 8 24 4 Z", fill: "#e69f00" }],
  ["path", { d: "M24 24 C28 28 31 31 31 35 a7 7 0 0 1 -14 0 C17 31 21 29 24 24 Z", fill: "#f0e442" }],
];

const ICE: Shape[] = [
  ["rect", { x: 8, y: 8, width: 32, height: 32, rx: 4, fill: "#a6dcf5", stroke: "#ffffff", "stroke-width": 2 }],
  ["path", { d: "M14 16 l8 -4 M14 26 l14 -8 M20 36 l14 -8", stroke: "#ffffff", "stroke-width": 3, "stroke-linecap": "round" }],
];

const BOWL: Shape[] = [
  ["path", { d: "M8 26 h32 a16 14 0 0 1 -32 0 Z", fill: "#8a7f73", stroke: "#e8e2da", "stroke-width": 2 }],
  ["path", { d: "M24 4 C28 10 32 13 32 19 a8 8 0 0 1 -16 0 C16 14 20 12 24 4 Z", fill: "#e69f00", opacity: 0.85 }],
];

const ARROW_R: Shape[] = [["path", { d: "M12 8 L38 24 L12 40 Z", fill: "currentColor" }]];
const ARROW_L: Shape[] = [["path", { d: "M36 8 L10 24 L36 40 Z", fill: "currentColor" }]];
// An open stone doorway with a chevron through it: "on to the next room".
const DOOR: Shape[] = [
  ["path", { d: "M8 44 V16 a16 14 0 0 1 32 0 V44 Z", fill: "#8a8f98", stroke: "#fff7d6", "stroke-width": 2 }],
  ["path", { d: "M14 44 V18 a10 9 0 0 1 20 0 V44 Z", fill: "#1d2733" }],
  ["path", { d: "M19 22 L30 31 L19 40 Z", fill: "#f0c75e" }],
];
const DOOR_L: Shape[] = [
  DOOR[0]!,
  DOOR[1]!,
  ["path", { d: "M29 22 L18 31 L29 40 Z", fill: "#f0c75e" }],
];
const JUMP: Shape[] = [["path", { d: "M24 6 L42 30 H31 V42 H17 V30 H6 Z", fill: "currentColor" }]];

const GEAR: Shape[] = [
  [
    "path",
    {
      d: "M20 4 h8 l1.5 6 a15 15 0 0 1 4 2.3 l6 -1.8 l4 7 l-4.5 4.2 a15 15 0 0 1 0 4.6 l4.5 4.2 l-4 7 l-6 -1.8 a15 15 0 0 1 -4 2.3 L28 44 h-8 l-1.5 -6 a15 15 0 0 1 -4 -2.3 l-6 1.8 l-4 -7 l4.5 -4.2 a15 15 0 0 1 0 -4.6 L4.5 17.5 l4 -7 l6 1.8 a15 15 0 0 1 4 -2.3 Z",
      fill: "currentColor",
    },
  ],
  ["circle", { cx: 24, cy: 24, r: 6, fill: "#16212c" }],
];

const SPEAKER: Shape[] = [
  ["path", { d: "M6 18 h8 l10 -9 v30 l-10 -9 h-8 Z", fill: "currentColor" }],
  ["path", { d: "M30 16 a10 10 0 0 1 0 16 M35 10 a17 17 0 0 1 0 28", fill: "none", stroke: "currentColor", "stroke-width": 3, "stroke-linecap": "round" }],
];

const HAND: Shape[] = [
  [
    "path",
    {
      d: "M18 44 C12 40 8 32 8 28 c0-3 4-4 6-1 l3 4 V8 a3.5 3.5 0 0 1 7 0 v14 a3.5 3.5 0 0 1 7 0 v2 a3.5 3.5 0 0 1 7 0 v4 a3 3 0 0 1 6 0 v6 c0 6-4 10-8 10 Z",
      fill: "#ffffff",
      stroke: "#0f1720",
      "stroke-width": 2,
      "stroke-linejoin": "round",
    },
  ],
];

const SPROUT: Shape[] = [
  ["path", { d: "M24 44 V24", stroke: "#4ade80", "stroke-width": 4, "stroke-linecap": "round" }],
  ["path", { d: "M24 26 C14 26 8 20 8 12 C18 12 24 18 24 26 Z", fill: "#4ade80" }],
  ["path", { d: "M24 22 C30 22 38 18 38 8 C30 8 24 12 24 22 Z", fill: "#86efac" }],
];

const SAPLING: Shape[] = [
  ["path", { d: "M24 46 V20", stroke: "#a16207", "stroke-width": 4, "stroke-linecap": "round" }],
  ["circle", { cx: 24, cy: 16, r: 12, fill: "#22c55e" }],
  ["circle", { cx: 14, cy: 24, r: 8, fill: "#16a34a" }],
  ["circle", { cx: 34, cy: 24, r: 8, fill: "#16a34a" }],
];

const HAT: Shape[] = [
  ["path", { d: "M24 2 L36 36 H12 Z", fill: "#6d28d9", stroke: "#c4b5fd", "stroke-width": 2, "stroke-linejoin": "round" }],
  ["ellipse", { cx: 24, cy: 38, rx: 20, ry: 6, fill: "#5b21b6", stroke: "#c4b5fd", "stroke-width": 2 }],
  ["path", { d: "M22 14 l2 -4 l2 4 l4 1 l-3 3 l1 4 l-4 -2 l-4 2 l1 -4 l-3 -3 Z", fill: "#f0c75e" }],
];

const STAR: Shape[] = [
  ["path", { d: "M24 4 L30 18 L45 19 L33 29 L37 44 L24 36 L11 44 L15 29 L3 19 L18 18 Z", fill: "#f0c75e", stroke: "#fff7d6", "stroke-width": 2, "stroke-linejoin": "round" }],
];

/** Methane: one carbon with four hydrogens, rising from the marsh in a bubble. */
const METHANE: Shape[] = [
  ["circle", { cx: 24, cy: 24, r: 21, fill: "#0b3b2e", stroke: "#009e73", "stroke-width": 3 }],
  ["path", { d: "M24 24 L24 9 M24 24 L11 32 M24 24 L37 32 M24 24 L30 36", stroke: "#e6f6ff", "stroke-width": 2.5 }],
  ["circle", { cx: 24, cy: 9, r: 5, fill: "#56b4e9" }],
  ["circle", { cx: 11, cy: 32, r: 5, fill: "#56b4e9" }],
  ["circle", { cx: 37, cy: 32, r: 5, fill: "#56b4e9" }],
  ["circle", { cx: 30, cy: 37, r: 4, fill: "#56b4e9" }],
  ["circle", { cx: 24, cy: 24, r: 7, fill: "#3b3b3b", stroke: "#e6f6ff", "stroke-width": 1.5 }],
];

/** Burning wood: a log with a small flame (carbon burning). */
const LOG: Shape[] = [
  ["rect", { x: 6, y: 32, width: 36, height: 10, rx: 5, fill: "#8a5a2b", stroke: "#e8d2b0", "stroke-width": 2 }],
  ["circle", { cx: 37, cy: 37, r: 3, fill: "#e8d2b0" }],
  ["path", { d: "M22 6 C27 13 32 17 32 24 a8 8 0 0 1 -16 0 C16 18 20 15 22 6 Z", fill: "#e69f00" }],
  ["path", { d: "M24 18 C26 21 28 23 28 26 a4 4 0 0 1 -8 0 C20 23 22 21 24 18 Z", fill: "#f0e442" }],
];

/** The Grimoire: a closed book with a star on the cover. */
const BOOK: Shape[] = [
  ["path", { d: "M10 6 h26 a4 4 0 0 1 4 4 v30 a4 4 0 0 1 -4 4 h-26 Z", fill: "#7c3aed", stroke: "#e9ddff", "stroke-width": 2 }],
  ["path", { d: "M10 6 v38", stroke: "#e9ddff", "stroke-width": 3 }],
  ["path", { d: "M25 14 l2.5 5 l5.5 .8 l-4 3.9 l1 5.5 l-5 -2.6 l-5 2.6 l1 -5.5 l-4 -3.9 l5.5 -.8 Z", fill: "#f0c75e" }],
];

/** Gather: an open hand scooping. */
const SCOOP: Shape[] = [
  ["path", { d: "M6 22 h36 a18 16 0 0 1 -36 0 Z", fill: "none", stroke: "currentColor", "stroke-width": 4, "stroke-linejoin": "round" }],
  ["path", { d: "M24 4 v12 M18 10 l6 6 l6 -6", fill: "none", stroke: "currentColor", "stroke-width": 4, "stroke-linecap": "round", "stroke-linejoin": "round" }],
];

/** Cast: a spark burst from a wand tip. */
const WAND: Shape[] = [
  ["path", { d: "M8 42 L28 22", stroke: "currentColor", "stroke-width": 5, "stroke-linecap": "round" }],
  ["path", { d: "M34 4 l3 8 l8 3 l-8 3 l-3 8 l-3 -8 l-8 -3 l8 -3 Z", fill: "#f0c75e" }],
];

/** Write a spell: a pencil. */
const PENCIL: Shape[] = [
  ["path", { d: "M10 38 L12 30 L32 10 L38 16 L18 36 Z", fill: "none", stroke: "currentColor", "stroke-width": 4, "stroke-linejoin": "round" }],
  ["path", { d: "M28 14 L34 20 M8 42 h32", stroke: "currentColor", "stroke-width": 3, "stroke-linecap": "round" }],
];

const ICONS = {
  pencil: PENCIL,
  methane: METHANE,
  log: LOG,
  book: BOOK,
  scoop: SCOOP,
  wand: WAND,
  drop: DROP,
  bubbles: BUBBLES,
  boom: BOOM,
  cloud: CLOUD,
  fire: FIRE,
  ice: ICE,
  bowl: BOWL,
  right: ARROW_R,
  left: ARROW_L,
  jump: JUMP,
  door: DOOR,
  doorLeft: DOOR_L,
  gear: GEAR,
  speaker: SPEAKER,
  hand: HAND,
  sprout: SPROUT,
  sapling: SAPLING,
  hat: HAT,
  star: STAR,
} as const;

export type IconName = keyof typeof ICONS;

export function icon(name: IconName): SVGSVGElement {
  return svg(ICONS[name]);
}

/** Replaces an element's children with an icon. */
export function setIcon(el: Element, name: IconName): void {
  el.replaceChildren(icon(name));
}

/** Atom colours (Okabe-Ito), the same on balls, tiles and the scale. */
export const ATOM_COLOR: Record<string, string> = {
  H: "#56b4e9",
  O: "#d55e00",
  C: "#3b3b3b",
};

/** Counts atoms in a formula such as "H2O" or "CO2". */
export function atomsOf(formula: string): Record<string, number> {
  const out: Record<string, number> = {};
  for (const m of formula.matchAll(/([A-Z][a-z]?)(\d*)/g)) {
    const el = m[1]!;
    out[el] = (out[el] ?? 0) + (m[2] ? Number(m[2]) : 1);
  }
  return out;
}

/** A molecule drawn as touching balls ("two blue and one red"), never subscripts. */
export function molecule(formula: string): HTMLSpanElement {
  const m = document.createElement("span");
  m.className = "molecule";
  m.setAttribute("aria-hidden", "true");
  for (const [el, n] of Object.entries(atomsOf(formula))) {
    for (let i = 0; i < n; i++) {
      const b = document.createElement("span");
      b.className = `ball el-${el.toLowerCase()}`;
      b.style.background = ATOM_COLOR[el] ?? "#cc79a7";
      m.append(b);
    }
  }
  return m;
}

/** "2H2 + O2 -> 2H2O" drawn as groups of balls with + and an arrow. */
export function equationBalls(equation: string): HTMLSpanElement {
  const root = document.createElement("span");
  root.className = "eq-balls";
  const sides = equation.replace(/→/g, "->").split("->");
  sides.forEach((side, i) => {
    if (i > 0) root.append(sep("→"));
    side
      .split("+")
      .map((t) => t.trim())
      .filter(Boolean)
      .forEach((term, j) => {
        if (j > 0) root.append(sep("+"));
        const m = /^(\d*)(.+)$/.exec(term);
        const n = Math.min(6, m?.[1] ? Number(m[1]) : 1);
        for (let k = 0; k < n; k++) root.append(molecule(m?.[2] ?? term));
      });
  });
  return root;
}

function sep(text: string): HTMLSpanElement {
  const s = document.createElement("span");
  s.className = "eq-sep";
  s.setAttribute("aria-hidden", "true");
  s.textContent = text;
  return s;
}
