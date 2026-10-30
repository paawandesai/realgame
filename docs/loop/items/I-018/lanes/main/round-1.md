# I-018 lane main, round 1 (builder)

Branch `loop/I-018`, based on origin/main a3e50b1. The spec's `based_on` is 855abfc. a3e50b1 is the I-005 merge on top of it, and it does not touch the electrolysis block, `chem.rs`'s tests or row P-015. Earlier builders were killed by container restarts. This round reviewed their uncommitted edits against the spec, kept them, and finished the item.

## What was built

- **`crates/bevy-client/src/chem.rs`**: new `pub mod electrolysis` with no Bevy types, as the spec defines it:
  - constants `TUBE_Y 1.6`, `TUBE_H 1.1`, `TUBE_D 0.25`, `INSET 0.025`, `FILL_D 0.2`, `TUBE_DZ 0.9`, `CATHODE_DZ = -TUBE_DZ`, `ANODE_DZ = TUBE_DZ`, `GAS_O2 0.3`, `GAS_H2 = 2.0 * GAS_O2`;
  - `Piece { y, h, d }`, `Tube { dz, glass, water, gas }`, `tube(dz, gas)` and `tubes()`.
  - The gas sits at the top of each tube, from the inner top down. The water fills from the inner bottom up to the gas. H2 has gas from 1.525 to 2.125 over water from 1.075 to 1.525. O2 has gas from 1.825 to 2.125 over water from 1.075 to 1.825.
  - Tests 1 to 4 are added at the end of the existing `mod tests`. No existing rule or test changed.
- **`crates/bevy-client/src/world.rs`**:
  - New `gas` material next to `glass`: `srgba(1.0, 0.85, 0.35, 0.6)`, `AlphaMode::Blend`, not emissive. It is a warm pale yellow, not near-white, so it stands out against the lavender wall.
  - The electrolysis block is now one `for t in chem::electrolysis::tubes()` loop. It draws glass, water (`liquid[1]`) and gas for each tube.
  - The comment now states 2H2O -> 2H2 + O2, cathode and anode, and points to `chem::electrolysis`.
  - The battery, ball, bench, `Target`, `Obj` and `glass` material are unchanged.
- **`content/science-sources.md`**: only row P-015 changed. Its `where` is now `crates/bevy-client/src/chem.rs, crates/bevy-client/src/world.rs`. Its note gives 0.6 m (H2, cathode side) and 0.3 m (O2), `chem::electrolysis::GAS_H2 = 2 × GAS_O2`, and the test `electrolysis_gas_is_two_to_one`. The Avogadro sentence is kept. The verdict, value and sources are unchanged.
- **`docs/loop/items/I-018/screenshot.png`**: taken from this branch's build with `?low&debug`, `tp 19.6 6.0`, `look 270 -10`. That is 2.2 m from the bench, facing +x. It shows yellow gas columns at the tops of both tubes over blue water, with the left (H2) column twice the right (O2), against the alcove wall.

## Checks

| check | result |
|---|---|
| `rustc --edition 2021 --test src/chem.rs` (the CI step, the crate's unit tests) | 19 passed, including the 4 new `electrolysis_*` tests |
| test 5 greps on the `2H2O` … `name: "lab bench"` range | literal pattern: none. `electrolysis::tubes()`: 1. `.(glass\|water\|gas).`: 6. `cathode`: the comment line. Polarity pattern: none. `H2 twice the O2` in the file: none |
| `npm test` | 50/50 pass (P-015 `where` paths resolve) |
| `cargo fmt --check` (bevy-client) | clean |
| `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` | clean |
| `build-web.sh` | OK, wasm gzip 5,038,422 B |
| brotli q11 of `pkg-out` (all files) | base a3e50b1 3,563,701 B, branch 3,563,339 B (−362 B; ≤ 2 KB growth bar met) |
| `QUERY=?low node tests/e2e/golden.mjs` (under the chromium lock) | 67/67 passed, run on the wasm whose sha1 starts 16ca9b56, which is this branch's build. No check was edited |

## Note for the tester: shared target dir hazard

The first `build-web.sh` this round finished its release step in 0.4 s and shipped a **stale wasm from another worktree**. The uplifted file `/home/claude/bevy-target/wasm32-unknown-unknown/release/bevy-client.wasm` is shared by every worktree using that `CARGO_TARGET_DIR`. This worktree's sources were older than that file, so cargo treated the build as fresh. The stale build showed the old 0.55/0.35 water columns. A golden run on it also passed, and that run is **not** counted above.

The fix was to `touch crates/bevy-client/src/*.rs` before building. The log then shows `Compiling realgame-bevy-client … (…/wf_2e5db624-b19-2/…)`. Check that line, or check the screenshot, before trusting a build of this branch.
