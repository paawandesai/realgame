// The reaction book as the simulation ships it (content/reactions.json, read
// from WebAssembly so there is one copy), the spells built on it, and the
// hover cards that show each spell's sourced numbers.

import type { IconName } from "./icons";
import { prettyFormula, type Tier } from "./types";

export interface BookReaction {
  id: string;
  name: string;
  equation: string;
  /** The equation with state symbols at 25 °C, which ΔH refers to. */
  equation_states: string;
  delta_h_kj: number;
  concept: string;
  /** Six words or fewer, for Sprout. */
  short_sprout: string;
  /** Eight words or fewer, for Seedling. */
  short_seedling: string;
  real_world: string;
  /** A real-world safety note, for reactions dangerous outside a game. */
  safety?: string;
  source: string;
}

interface PhaseChange {
  id: string;
  note?: string;
  source: string;
  temperature_c?: number;
  latent_heat_kj_per_mol?: number;
}

export interface Book {
  byId: Map<string, BookReaction>;
  /** Sourced phase changes (melting, boiling): kJ/mol taken in, at °C. */
  phases: Map<string, { kj: number; c: number; source: string }>;
  /** Why steam and liquid water give different ΔH (sourced), if present. */
  vaporNote: { text: string; source: string } | null;
}

export function parseBook(json: string): Book {
  const byId = new Map<string, BookReaction>();
  const phases: Book["phases"] = new Map();
  let vaporNote: Book["vaporNote"] = null;
  try {
    const t = JSON.parse(json) as { reactions?: BookReaction[]; phase_changes?: PhaseChange[] };
    for (const r of t.reactions ?? []) byId.set(r.id, r);
    for (const p of t.phase_changes ?? []) {
      if (typeof p.latent_heat_kj_per_mol === "number" && typeof p.temperature_c === "number") {
        phases.set(p.id, { kj: p.latent_heat_kj_per_mol, c: p.temperature_c, source: p.source });
      }
    }
    const v = (t.phase_changes ?? []).find((p) => p.id === "water_vaporisation_25c");
    if (v?.note) vaporNote = { text: v.note, source: v.source };
  } catch {
    // The table is compiled into the sim; an unreadable copy only loses the cards.
  }
  return { byId, phases, vaporNote };
}

/** The one card that explains why steam and liquid water give different ΔH
 * (making steam vs splitting liquid); shown there only, not on every card. */
const WATER_STATE_REACTIONS = new Set(["water_synthesis"]);

/** The sourced steam-vs-liquid note for a reaction that needs it, else "". */
export function stateNote(id: string, book: Book): string {
  return WATER_STATE_REACTIONS.has(id) && book.vaporNote ? book.vaporNote.text : "";
}

/** A spell the player can cast by hotkey, card or name. */
export interface SpellDef {
  formula: string;
  /** Short everyday name ("Boom"). */
  name: string;
  picture: IconName;
  /** The reaction in the book, or null for a plain pour from the suit. */
  reaction: string | null;
  /** For pours: what it is and where its numbers come from. */
  note?: string;
  noteSource?: string;
}

/** Hotkeys 1-5 for the Apprentice, in the order the trial needs them. */
export const BAR_SPELLS: SpellDef[] = [
  {
    formula: "H2O",
    name: "Water",
    picture: "drop",
    reaction: null,
    note: "No reaction: your suit's condenser collar pulls water out of humid air, and this pours it out.",
  },
  { formula: "2H2 + O2 -> 2H2O", name: "Boom", picture: "boom", reaction: "water_synthesis" },
  { formula: "2H2O -> 2H2 + O2", name: "Bubbles", picture: "bubbles", reaction: "electrolysis_water" },
  { formula: "CH4 + 2O2 -> CO2 + 2H2O", name: "Fireball", picture: "methane", reaction: "methane_combustion" },
  {
    formula: "CO2",
    name: "Cloud",
    picture: "cloud",
    reaction: null,
    note: "No reaction: carbon dioxide is denser than air (1.977 vs 1.293 g/L at 0 °C), so it sinks and smothers flames.",
    noteSource: "CRC Handbook of Chemistry and Physics, gas densities",
  },
];

/** A Grimoire page: a reaction from the book, or one of the suit's pours. */
export interface GrimoirePage {
  id: string;
  name: string;
  picture: IconName;
  /** The room (1-4) that teaches it. */
  room: number;
  /** Where to earn it, naming the room, while that room is still ahead. */
  where: string;
  /** How to earn it once its room is behind you (never names a room). */
  again: string;
  /** Sprout's teaser: pictures and a few words, no room numbers. */
  whereSprout: string;
  /** Tiers that can earn it in this trial (empty: a later trial). */
  tiers: Tier[];
  /** The formula for a pour (no reaction): what the card shows. */
  formula?: string;
  /** Sprout and Seedling words for a pour page. */
  short?: string;
  /** Shown on a page that comes in a later trial (or that a free-play
   * challenge unlocks). */
  soon?: string;
  /** A sourced phase change in reactions.json this page shows (melting). */
  phase?: string;
}

const ALL: Tier[] = ["sprout", "seedling", "apprentice"];

/** The Grimoire's pages, in the order the trial teaches them. Only pages
 * this tier can earn in this trial count; the last is a "coming soon". */
export const GRIMOIRE_PAGES: GrimoirePage[] = [
  {
    id: "pour_water",
    name: "Water",
    picture: "drop",
    room: 1,
    where: "Room 1, Splash Garden: pour water (H2O) on the burning hedge.",
    again: "Cast water (H2O) at a fire.",
    whereSprout: "The water drop.",
    tiers: ALL,
    formula: "H2O",
    short: "Water puts out fire.",
  },
  {
    id: "water_synthesis",
    name: "Boom",
    picture: "boom",
    room: 2,
    where: "Room 2, the Frozen Gate: melt the ice wall with boom (2H2 + O2 -> 2H2O).",
    again: "Cast boom: 2H2 + O2 -> 2H2O.",
    whereSprout: "The boom card.",
    tiers: ALL,
  },
  {
    id: "electrolysis_water",
    name: "Bubbles",
    picture: "bubbles",
    room: 3,
    where: "Room 3, the Crystal Lab: split water (2H2O -> 2H2 + O2) beside the crystal.",
    again: "Cast bubbles: 2H2O -> 2H2 + O2.",
    whereSprout: "The bubbles card.",
    tiers: ALL,
  },
  {
    id: "pour_co2",
    name: "Cloud",
    picture: "cloud",
    room: 4,
    where: "Room 4, the Marsh Door: smother the burning hedge with carbon dioxide (CO2).",
    again: "Cast cloud (CO2) at a fire to smother it.",
    whereSprout: "The cloud card.",
    tiers: ALL,
    formula: "CO2",
    short: "Heavy gas smothers fire.",
  },
  {
    id: "methane_combustion",
    name: "Fireball",
    picture: "methane",
    room: 4,
    where: "Room 4 bonus: put out the hedge, then light the brazier with fireball (CH4 + 2O2 -> CO2 + 2H2O), using methane from the marsh bubbles.",
    again: "Bonus: gather methane from marsh bubbles, then cast fireball at a brazier or a flame: CH4 + 2O2 -> CO2 + 2H2O.",
    whereSprout: "The marsh bubbles.",
    tiers: ["seedling", "apprentice"],
  },
  {
    id: "carbon_combustion",
    name: "Burning wood",
    picture: "log",
    room: 1,
    where: "",
    again: "",
    whereSprout: "More soon!",
    tiers: [],
    soon: "Coming soon, or earn it in free play: smother a flame with CO2. Wood is mostly cellulose (C, H, O); burning makes CO2 and H2O.",
  },
  {
    id: "water_melting",
    name: "Melting ice",
    picture: "ice",
    room: 2,
    where: "",
    again: "",
    whereSprout: "More soon!",
    tiers: [],
    formula: "H2O(s) -> H2O(l)",
    short: "Ice takes in heat to melt.",
    phase: "water_melting",
    soon: "Coming soon, or earn it in free play: melt the ice using under 500 kJ.",
  },
];

/** Pages this tier can earn in this trial. */
export function earnablePages(tier: Tier): GrimoirePage[] {
  return GRIMOIRE_PAGES.filter((p) => p.tiers.includes(tier));
}

/** "−483.6 kJ" with a real minus sign. */
export function formatDh(kj: number): string {
  const sign = kj < 0 ? "−" : "+";
  return `${sign}${Math.abs(kj).toFixed(1)} kJ`;
}

/** The lines a hover card shows for a spell. */
export function spellCardLines(spell: SpellDef, book: Book): { title: string; lines: string[]; source: string } {
  const r = spell.reaction ? book.byId.get(spell.reaction) : undefined;
  if (r) {
    return {
      title: `${spell.name}: ${prettyFormula(r.equation_states ?? r.equation)}`,
      lines: [
        `ΔH = ${formatDh(r.delta_h_kj)} ${r.delta_h_kj < 0 ? "(releases heat)" : "(takes in energy)"}`,
        r.concept,
      ],
      // The steam-vs-liquid note lives on the Grimoire page only.
      source: r.source,
    };
  }
  return {
    title: `${spell.name}: ${prettyFormula(spell.formula)}`,
    lines: [spell.note ?? ""],
    source: spell.noteSource ?? "",
  };
}

/**
 * One floating card shared by every hoverable thing. Shown on pointer hover
 * and on keyboard focus, so it works without a mouse.
 */
export class HoverCard {
  private readonly el: HTMLElement;
  private owner: HTMLElement | null = null;

  constructor(private readonly stage: HTMLElement) {
    this.el = document.createElement("div");
    this.el.id = "hovercard";
    this.el.setAttribute("role", "tooltip");
    this.el.hidden = true;
    stage.append(this.el);
  }

  attach(target: HTMLElement, content: () => { title: string; lines: string[]; source: string } | null): void {
    const show = () => {
      const c = content();
      if (!c) return;
      this.owner = target;
      const title = document.createElement("strong");
      title.textContent = c.title;
      const parts: HTMLElement[] = [title];
      for (const l of c.lines) {
        if (!l) continue;
        const p = document.createElement("p");
        p.textContent = l;
        parts.push(p);
      }
      if (c.source) {
        const s = document.createElement("p");
        s.className = "hc-source";
        s.textContent = `Source: ${c.source}`;
        parts.push(s);
      }
      this.el.replaceChildren(...parts);
      this.el.hidden = false;
      this.place(target);
    };
    const hide = () => {
      if (this.owner === target) this.hide();
    };
    target.addEventListener("pointerenter", (e) => {
      if (e.pointerType === "mouse") show();
    });
    target.addEventListener("pointerleave", hide);
    target.addEventListener("focus", show);
    target.addEventListener("blur", hide);
  }

  hide(): void {
    this.owner = null;
    this.el.hidden = true;
  }

  private place(target: HTMLElement): void {
    const s = this.stage.getBoundingClientRect();
    const r = target.getBoundingClientRect();
    const w = this.el.offsetWidth;
    const h = this.el.offsetHeight;
    let left = r.left - s.left + r.width / 2 - w / 2;
    left = Math.max(8, Math.min(s.width - w - 8, left));
    let top = r.top - s.top - h - 8;
    if (top < 8) top = r.bottom - s.top + 8;
    this.el.style.left = `${left}px`;
    this.el.style.top = `${Math.max(8, Math.min(s.height - h - 8, top))}px`;
  }
}
