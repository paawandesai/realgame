# I-018 break-1 (breaker, wave-6)

Verdict: no blocking findings.

## Blocking
none

## Should-fix
none

## Nit
- crates/bevy-client/src/chem.rs:687-764 (mod electrolysis): the water column is drawn as a separate cylinder under the gas with the tube bottom open in the voltameter sense; physically fine for a static prop. No change needed.
- crates/bevy-client/src/world.rs:1092: comment says cathode/anode but the battery is unmarked, so a player cannot verify which tube is cathode. Spec accepts this (polarity marks out of scope, I-008).
- Brotli growth (<= 2 KB) not measured by this breaker (no base build compared); wasm gzip bytes on branch 5038422.

## Verified
- Changed files are all in lane: content/science-sources.md (P-015 row only), chem.rs, world.rs (gas material + electrolysis block), docs/loop/items/I-018/*.
- Science: GAS_H2 = 2.0*GAS_O2 (0.6 m vs 0.3 m), equal diameters (0.2 m), so volume ratio is exactly 2:1. H2 tube at dz = -0.9 (cathode), O2 at +0.9; world.rs draws only chem::electrolysis::tubes() (no literals). Register P-015 row states 0.6 m / 0.3 m, GAS_H2, test name; where = chem.rs, world.rs; verdict correct.
- rustc --test chem.rs: 19 passed incl. the 4 electrolysis tests.
- Spec test 5 greps: all as required (no literals, 1 tubes() call, 6 piece refs, cathode comment, no polarity words, old comment gone).
- npm test: 50 pass, 0 fail.
- Build-web.sh succeeds. golden.mjs ?low: 67/67 passed, no console errors.
- screenshot.png: two tan/yellow gas columns at tube tops, left (H2) about twice the right, above blue water, clearly distinct from the lavender wall.
