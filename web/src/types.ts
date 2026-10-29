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
  /** What charges the ⚡ battery now. */
  charge_source: "crystal" | "solar";
  /** Gauge changes the suit made by itself since the last state, per jar
   * (condenser, electrolyser, CO2 scrubber), apart from the player's casts. */
  background: Record<string, number>;
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
  /** "Fire left" (hedges), "ice left" (ice wall), or how full the gas trap is (room 3), 0..1. */
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
  /** The goal in ordered steps, each ticked when done (rooms 3 and 4). */
  steps: { text: string; done: boolean; pic: "bubbles" | "trap" | "boom" | "fire" | "bowl" | "methane"; bonus: boolean }[] | null;
  /** Room 3's gauge: [H2 trapped, H2 made, O2 made, H2 needed], in cells. */
  gas?: [number, number, number, number] | null;
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
    | "gain"
    /** A blast threw the player back (x, y: the player; tx, ty: the blast). */
    | "knock"
    // v3 (crates/sim/src/events.rs `fx_kind`; payloads documented there).
    /** H2O2 made foam; power = foam cells. */
    | "foam"
    /** Burning Mg met water; power = 0..1. Flash capped at 3 Hz. */
    | "mg_flare"
    /** An indicator pool's pH moved; power = the new pH (0..14). */
    | "ph_shift"
    /** A salt recoloured a flame; reaction_id = "flame_test_<el>", power = emitter id. */
    | "salt"
    /** Sand smothered flames; power = cells. */
    | "smother"
    /** The catalyst flies back to the suit (x, y -> tx, ty). */
    | "catalyst_return";
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
  spell:
    | "water"
    | "boom"
    | "split"
    | "cloud"
    | "gas"
    | "fireball"
    | "burn"
    | "unknown"
    // v3 spells (crates/sim/src/chem.rs V3_SPELLS).
    | "catalyst"
    | "acid"
    | "base"
    | "sand"
    | "salt";
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
  /** The reaction that actually ran ("H2 + 1/2 O2 -> H2O"); its ΔH is dh_kj. */
  reacted: string | null;
  /** What the cast spent and made: "-2 H2, -1 O2, +2 H2O". */
  ledger: string | null;
  /** Where the products went: "2 H2O released as steam". */
  went: string | null;
  /** The simplest balanced form, or the substance poured; null if unbalanced. */
  canonical: string | null;
  /** How many times the simplest form was cast (2 for 4H2 + 2O2). */
  multiple: number;
  concept: string | null;
  real_world: string | null;
  /** A real-world safety note (the hydrogen + oxygen boom). */
  safety: string | null;
  delta_h_kj: number | null;
  source: string | null;
  first_discovery: boolean;
  /** Why an "unknown" cast was rejected; null for real casts. */
  reason:
    | "empty"
    | "parse"
    | "element"
    | "not_in_world"
    | "no_change"
    | "noble_gas"
    | "unbalanced"
    | "no_products"
    | "state"
    | "zero"
    | null;
  /** Apprentice only: the reaction with state symbols, matching the ΔH shown. */
  equation_states: string | null;
  /** Apprentice only: a teaching note, such as "symbols are case-sensitive". */
  note: string | null;
}

/** Turns "2H2 + O2 -> 2H2O" into display text with subscripts and a real arrow. */
export function prettyFormula(text: string): string {
  const subs = "₀₁₂₃₄₅₆₇₈₉";
  return text
    .replace(/->/g, "→")
    .replace(/([A-Za-z)])(\d+)/g, (_, el: string, n: string) => el + [...n].map((d) => subs[Number(d)]).join(""));
}

/** Seedling's real-world safety note: short, plain, shown with a warning
 * picture (the full sourced note is the Apprentice's). */
export const SEEDLING_SAFETY = "Real hydrogen booms. Only grown-up chemists do this!";

// ---------------------------------------------------------------- v3 contract

/** Species ids (`Species as u8` in crates/sim/src/species.rs), for Lab painting. */
export const SPECIES = {
  air: 0,
  stone: 1,
  wood: 2,
  water: 3,
  ice: 4,
  steam: 5,
  h2: 6,
  o2: 7,
  ch4: 8,
  co2: 9,
  cl2: 10,
  flame: 11,
  door: 12,
  crystal: 13,
  mg: 14,
  mgo: 15,
  sand: 16,
  h2o2: 17,
  foam: 18,
  indicator: 19,
  limewater: 20,
  chalk: 21,
} as const;
export type SpeciesName = keyof typeof SPECIES;

/** A Trial or the Lab (see `LevelSpec` in game/levels.rs); from `WebGame.trial_list(tier)`. */
export interface LevelSpec {
  id: string;
  kind: "trial" | "lab";
  title: string;
  goal: string;
  tiers: Tier[];
  /** Casts of the best scripted solution (Apprentice star 2); 0 until set. */
  par_casts: number;
}

/** How a Trial run went (see `TrialResult` in game/levels.rs); from `WebGame.trial_result()`. */
export interface TrialResult {
  id: string;
  solved: boolean;
  casts: number;
  hints_used: number;
  balanced_all: boolean;
  atoms_wasted: number;
  kj_net: number;
  /** Solution signature: a different sig is a different solution (star 3). */
  sig: number;
  /** Stars this run earned on its own (0-2); star 3 comes from progress.recordSig. */
  stars: number;
}
