// Shapes of the JSON the Rust simulation returns (see crates/sim/src/game.rs).

export type Tier = "sprout" | "seedling" | "apprentice";

export interface Objective {
  id: string;
  text: string;
  done: boolean;
}

export interface GameState {
  tier: Tier;
  jars: Record<string, number>;
  charge: number;
  objectives: Objective[];
  messages: string[];
  player_x: number;
  player_y: number;
  discovered: string[];
  trial_complete: boolean;
  aim_temp_c: number;
  aim_substance: string;
  aim_x: number;
  aim_y: number;
  /** Camera left edge in world cells. */
  view_x: number;
}

/** A presentation-only event from the simulation (see `Fx` in game.rs). */
export interface Fx {
  kind: "land";
  x: number;
  y: number;
  power: number;
}

export interface CastResult {
  ok: boolean;
  kind: "release" | "reaction" | "fizzle" | "error";
  message: string;
  equation: string;
  suggestion: string | null;
  imbalance: string[];
  reaction_id: string | null;
  concept: string | null;
  real_world: string | null;
  delta_h_kj: number | null;
  source: string | null;
  first_discovery: boolean;
}

/** Turns "2H2 + O2 -> 2H2O" into display text with subscripts and a real arrow. */
export function prettyFormula(text: string): string {
  const subs = "₀₁₂₃₄₅₆₇₈₉";
  return text
    .replace(/->/g, "→")
    .replace(/([A-Za-z)])(\d+)/g, (_, el: string, n: string) => el + [...n].map((d) => subs[Number(d)]).join(""));
}
