// Shapes of the JSON the Rust simulation returns (see crates/sim/src/game.rs).

export type Tier = "sprout" | "seedling" | "apprentice";

export interface Objective {
  id: string;
  text: string;
  done: boolean;
}

export interface GameState {
  tier: Tier;
  /** Suit gauges in moles (one water cast is 1 H2O). */
  jars: Record<string, number>;
  jar_caps: Record<string, number>;
  crystal_kj: number;
  crystal_max_kj: number;
  near_crystal: boolean;
  state_version: number;
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

export type Outcome = "full" | "partial" | "fizzle" | "unknown";

/** Element symbol -> atom count. */
export type Atoms = Record<string, number>;

/** A presentation-only event from the simulation (see `Fx` in game.rs). */
export interface Fx {
  kind:
    | "charge"
    | "bolt"
    | "impact"
    | "fizzle"
    | "unknown"
    | "splash"
    | "hiss"
    | "melt"
    | "arc"
    | "bubble"
    | "door"
    | "discover"
    | "land";
  x: number;
  y: number;
  tx: number;
  ty: number;
  dh_kj: number;
  outcome: Outcome | null;
  leftovers: Atoms;
  reaction_id: string | null;
  /** The cast this event belongs to (0 when no cast caused it). */
  id: number;
  power: number;
}

/** What a cast did (see `CastOutcome` in game.rs). Every cast returns one. */
export interface CastOutcome {
  id: number;
  outcome: Outcome;
  spell: "water" | "boom" | "split" | "cloud" | "gas" | "fireball" | "burn" | "unknown";
  input: string;
  equation: string;
  reaction_id: string | null;
  consumed: Atoms;
  from_air: Atoms;
  products: Record<string, number>;
  leftovers: Atoms;
  missing: string[];
  source_x: number | null;
  source_y: number | null;
  power: number;
  dh_kj: number;
  label: string;
  suggestion: string | null;
  imbalance: string[];
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
