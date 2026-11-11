// The bench's chemistry comes from the Rust rules engine (crates/bench),
// compiled to WebAssembly. This file only wraps it and adds what the 3D view
// needs: atoms per formula, colours and pretty labels.
import init, { bench_catalog, bench_mix } from './pkg/realgame_bench.js';

export type Bowl = Record<string, number>;
export type Energy = 'spark' | 'zap';
export type Status = 'exact' | 'partial' | 'short' | 'wrongenergy' | 'nothing' | 'empty';

export interface Spell {
  id: string;
  name: string;
  equation: string;
  delta_h_kj: number | null;
  energy: Energy;
  concept: string | null;
  real_world: string | null;
  safety: string | null;
  source: string;
  reactants: [string, number][];
  products: [string, number][];
}

export interface MixResult {
  status: Status;
  spell: Spell | null;
  batches: number;
  made: Bowl;
  left: Bowl;
  message: string;
}

export async function loadChemistry(): Promise<void> {
  await init();
}

export function mix(bowl: Bowl, energy: Energy): MixResult {
  return JSON.parse(bench_mix(JSON.stringify(bowl), energy)) as MixResult;
}

export function catalog(): Spell[] {
  return JSON.parse(bench_catalog()) as Spell[];
}

/** Atoms in a simple formula (`CH4` -> C, H, H, H, H). No brackets needed for the bench's reagents. */
export function atomsOf(formula: string): string[] {
  const out: string[] = [];
  for (const m of formula.matchAll(/([A-Z][a-z]?)(\d*)/g)) {
    const n = m[2] ? parseInt(m[2], 10) : 1;
    for (let i = 0; i < n; i++) out.push(m[1]);
  }
  return out;
}

/** CPK-style colours, the convention chemistry classrooms use. */
const COLORS: Record<string, number> = {
  H: 0xf4f4f4,
  O: 0xe8352e,
  C: 0x3a3a3a,
  N: 0x3f6fe0,
  Cl: 0x3cc43c,
  Na: 0xa368e0,
  Fe: 0xd9772b,
  Mg: 0x2f9e5a,
  Al: 0xb8b8c8,
  Ca: 0x6f6f6f,
  S: 0xf2d33c,
  K: 0x8f40d4,
  Zn: 0x7d80b0,
};

export function atomColor(el: string): number {
  return COLORS[el] ?? 0xff80c0;
}

export function atomRadius(el: string): number {
  return el === 'H' ? 0.0075 : 0.011;
}

const SUB = '₀₁₂₃₄₅₆₇₈₉';

/** `H2O` -> `H₂O`, and coefficients stay normal (`2H2` -> `2H₂`). */
export function pretty(text: string): string {
  return text
    .replace(/([A-Za-z)])(\d+)/g, (_, a: string, d: string) => a + [...d].map((c) => SUB[+c]).join(''))
    .replace(/->/g, '→');
}

/** `{H2: 2, O2: 1}` -> `2 H₂ + O₂`. */
export function bowlText(bowl: Bowl): string {
  const parts = Object.entries(bowl)
    .filter(([, n]) => n > 0)
    .map(([f, n]) => `${n > 1 ? n + ' ' : ''}${pretty(f)}`);
  return parts.length ? parts.join(' + ') : 'empty';
}

/** Friendly names for the shelf. */
export const NAMES: Record<string, string> = {
  H2: 'hydrogen',
  O2: 'oxygen',
  CH4: 'methane',
  C: 'carbon',
  Na: 'sodium',
  Cl2: 'chlorine',
  Fe: 'iron',
  Mg: 'magnesium',
  H2O: 'water',
  N2: 'nitrogen',
};
