// The Formula Bench (I-028): build a spell's formula at your age band's
// level. See docs/loop/items/I-028/spec.md (sections 1-3 and Contract).
//
// Loaded by boot.js with a guarded dynamic import, so a missing or broken
// bench.js costs only the bench, never the boot. The game drains
// window.__gameIn.bench (one command string each: `band <tier>`, `open`,
// `pick <slot>`, `try <slot> <text…>`, `close`) and writes
// window.__benchState (a JSON string) only when it changes. crates/sim's
// bench::judge is the only judge: nothing here decides right or wrong.
//
// Contract stub: startBench() only creates the queue, and the pure exports
// return null. The web lane writes the UI and the bodies.

// Sprout: the text for the atoms placed in the tray (recipe elements first,
// in recipe order, then others alphabetically; a count of 1 is written as
// nothing), for example ['H','O','H'] with the water recipe -> 'H2O'.
export function composeSprout(placed, recipe) {
  void placed; void recipe;
  return null;
}

// Seedling, one substance: the text for the per-element tile counts,
// for example {H:2,O:1} -> 'H2O'; all zero -> ''.
export function composeSeedling(recipe, counts) {
  void recipe; void counts;
  return null;
}

// Seedling and Apprentice, a reaction: the equation for the coefficient
// tiles, for example Fire [1,2,1,2] -> 'CH4 + 2O2 -> CO2 + 2H2O'.
export function composeEquation(recipe, coefs) {
  void recipe; void coefs;
  return null;
}

// The balance scale (display only, never the judge): atom counts per pan,
// { left, right, level, tilt } with tilt = sign(right total - left total).
export function scaleOf(recipe, coefs) {
  void recipe; void coefs;
  return null;
}

// The band's reason line for a judged result (Contract, "Reason strings").
// names = { [slot]: { name, formula } } from the published spells list.
export function reasonFor(band, result, names, tries) {
  void band; void result; void names; void tries;
  return null;
}

// The age band: ?band= in the URL, else the age picker's pick on this page
// load, else the bench chooser's; else null (ask). Never stored.
export function bandFrom(search, picked, chosen) {
  void search; void picked; void chosen;
  return null;
}

// The build view on its own (for I-029's worked casts and I-030's trial):
// mountBuilder(root, { band, spell, recipe, prompt, onMake }) -> { destroy }.
export function mountBuilder(root, opts) {
  void root; void opts;
  return null;
}

export function startBench() {
  const gi = window.__gameIn;
  if (gi && !Array.isArray(gi.bench)) gi.bench = [];
}
