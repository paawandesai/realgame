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
  /** The room the player is in (0-based) and which gates are open. */
  room: number;
  gates_open: boolean[];
  goals: Goal[];
  torch: TorchView;
}

/** A room's target, marked on screen (see `Goal` in game/rooms.rs). */
export interface Goal {
  room: number;
  kind: "fire" | "ice" | "gas" | "brazier";
  x: number;
  y: number;
  done: boolean;
  /** "Fire left", 0..1, for the burning hedges. */
  meter: number | null;
  /** The spell that moves this goal on right now. */
  need: "water" | "boom" | "split";
  /** Where to stand (player centre, world cells) to cast it. */
  stand_x: number;
  /** The room's goal in one tier-appropriate line (the objective banner). */
  text: string;
  /** A target area to outline (x0, y0, x1, y1), such as the gas pocket. */
  area: [number, number, number, number] | null;
  /** Where to label the crystal battery (top centre), in room 3. */
  battery: [number, number] | null;
  /** The goal in ordered steps, each ticked when done (room 4). */
  steps: { text: string; done: boolean }[] | null;
}

export interface TorchView {
  x: number;
  y: number;
  lit: boolean;
  carried: boolean;
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
    | "frost"
    | "arc"
    | "bubble"
    | "door"
    | "discover"
    | "land"
    /** The suction beam while gathering (reaction_id is the jar). */
    | "gather"
    /** Methane bubbling up from a marsh vent (no reaction). */
    | "seep"
    /** A gauge went up; reaction_id is "cause:jar" (condense, split, capture, gather). */
    | "gain";
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
  /** Reagent molecules that did not react ("1 H2", "½ O2"). */
  unreacted: string[];
  /** The simplest balanced form, or the substance poured; null if unbalanced. */
  canonical: string | null;
  /** How many times the simplest form was cast (2 for 4H2 + 2O2). */
  multiple: number;
  concept: string | null;
  real_world: string | null;
  delta_h_kj: number | null;
  source: string | null;
  first_discovery: boolean;
  /** Why an "unknown" cast was rejected; null for real casts. */
  reason: "empty" | "parse" | "element" | "not_in_world" | "unbalanced" | "state" | "zero" | null;
  /** Apprentice only: the reaction with state symbols, matching the ΔH shown. */
  equation_states: string | null;
}

/** Turns "2H2 + O2 -> 2H2O" into display text with subscripts and a real arrow. */
export function prettyFormula(text: string): string {
  const subs = "₀₁₂₃₄₅₆₇₈₉";
  return text
    .replace(/->/g, "→")
    .replace(/([A-Za-z)])(\d+)/g, (_, el: string, n: string) => el + [...n].map((d) => subs[Number(d)]).join(""));
}
