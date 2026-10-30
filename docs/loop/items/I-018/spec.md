# I-018: Electrolysis prop shows H2:O2 at exactly 2:1

**based_on:** origin/main 855abfc7e8af33b056b33fc9effdb9b46a969b44 (2026-10-30, merge of PR #24 `loop/I-007`). The earlier base, 3dbd332, came before I-007. The planner checked 855abfc against every fact below: the electrolysis block in `world.rs` has the same literals (its comment is at line 1084), `chem.rs`'s `mod tests` is now at line 688, and row P-015 is unchanged. I-007 has merged, so the build no longer waits on it.

**Backlog row:** `I-018 | Science fix (nightly 1 B6 error P-015): electrolysis prop tubes must show H2:O2 gas columns at exactly 2:1; re-pin register row and fix code comment | B6 | product | — | proposed`.

## Facts from 855abfc this spec relies on

- `world.rs` lines 1073 to 1129 build the lab bench in the alcove at `(x, z) = (21.8, 6.0)`. The electrolysis prop is in it:
  - The comment reads `// Electrolysis: two tubes (H2 twice the O2) over a battery.` (line 1084).
  - Battery: a `cube` in `metal`, at `(x, 1.15, z)`, scale `(0.6, 0.3, 0.5)`. It has no terminals or polarity marks.
  - Two glass tubes: `cyl` in `glass` (`srgba(0.75, 0.95, 1.0, 0.22)`, `AlphaMode::Blend`, line 507), at `(x, 1.6, z ∓ 0.9)`, scale `(0.25, 1.1, 0.25)`. They are the same size and span y 1.05 to 2.15.
  - Two fills, both `cyl` in `liquid[1]` (emissive blue, srgb 0.4, 0.7, 1.0):
    - at `z − 0.9`: centre y 1.35, scale `(0.2, 0.55, 0.2)`, so y 1.075 to 1.625;
    - at `z + 0.9`: centre y 1.25, scale `(0.2, 0.35, 0.2)`, so y 1.075 to 1.425.
  - `kit.cyl` is `Cylinder::new(0.5, 1.0)` (line 309): radius 0.5, height 1.0. So `scale.y` is the height in metres and `scale.x` (= `scale.z`) is the **diameter** in metres.
- The alcove walls use `wall_short`, a stone texture tinted `srgb(0.84, 0.8, 0.92)` (pale lavender, line 483). The ceiling is `stone_dark` `srgb(0.56, 0.52, 0.68)`. A pale white gas will have low contrast against this wall.
- So the two visible columns are 0.55 and 0.35 m, a ratio of **1.571, not 2**. Both start at the tube bottom and are drawn in the "water" blue, so the prop reads as two water columns, not as gas. Nothing marks which tube is H2.
- `content/science-sources.md` row **P-015** (`electrolysis prop H2:O2 tube volume ratio`, value 2, where `crates/bevy-client/src/world.rs`, verdict `correct`) claims "the H2 tube holds twice the O2". Nightly 1 scored this as B6's only error.
- Multi-file `where` cells use a **comma and a space** as the separator (for example C-001: `crates/bevy-client/src/chem.rs, crates/bevy-client/web/index.html`). `test/science-sources.test.ts` splits `where` on `,` and trims each path. It fails when a path does not exist. Its stale-text check applies only to `C-` rows.
- `chem.rs` has no Bevy dependency. CI runs its tests with `rustc --edition 2021 --test src/chem.rs` (`.github/workflows/ci.yml`, step "Chemistry rules tests"). `world.rs` already has `use crate::chem::{self, Kind, Obj};` (line 14).

## Problem

The only B6 error in nightly 1 is a visual science claim that the prop gets wrong. Water splitting, 2H2O(l) → 2H2(g) + O2(g), gives 2 volumes of H2 for every volume of O2 at the same temperature and pressure (Avogadro). The prop shows 0.55 : 0.35 (1.57 : 1), shows liquid where the gas should be, and the code comment says "H2 twice the O2", which the geometry does not do. A chemistry teacher (a panel persona and the buyer) would spot this, because the 2:1 Hoffman voltameter is a standard classroom demo.

## Bar moved, and by how much

- **B6 (science is right):** errors go from **1 to 0**. P-015 goes from error to `correct`, re-pinned to geometry that a unit test checks. The next science review must report no error on P-015. B6 also needs 100% sourced, which is already 33/33 and must stay that way.
- **B1, B2, B3:** must not regress. The change adds two small cylinder meshes and one material, so the size and frame-time effect is nil (test 7).

## Smallest version

1. **Geometry in a Bevy-free place.** Add `pub mod electrolysis` inside `crates/bevy-client/src/chem.rs`, with no Bevy types, so the existing CI step tests it with no CI change. It holds plain `f32` constants and one function, which is the **only source** of the prop's tube and column positions and sizes:
   - `TUBE_Y = 1.6`, `TUBE_H = 1.1`, `TUBE_D = 0.25` (glass diameter = glass `scale.x`), `INSET = 0.025`, `FILL_D = 0.2` (water and gas diameter = their `scale.x`).
   - `TUBE_DZ = 0.9`. `CATHODE_DZ = -TUBE_DZ` (H2 tube, the `z − 0.9` side) and `ANODE_DZ = TUBE_DZ` (O2 tube).
   - `GAS_O2 = 0.3` and `GAS_H2 = 2.0 * GAS_O2` (0.6), in metres of column height. Both tubes have the same diameter, so the height ratio is the volume ratio.
   - `pub struct Piece { pub y: f32, pub h: f32, pub d: f32 }` (centre y, height, diameter) and `pub struct Tube { pub dz: f32, pub glass: Piece, pub water: Piece, pub gas: Piece }`.
   - `pub fn tube(dz: f32, gas: f32) -> Tube`, and `pub fn tubes() -> [Tube; 2]`, which returns `[tube(CATHODE_DZ, GAS_H2), tube(ANODE_DZ, GAS_O2)]`. Inside `tube`:
     - glass = `Piece { y: TUBE_Y, h: TUBE_H, d: TUBE_D }`;
     - inner bottom = `TUBE_Y − TUBE_H/2 + INSET` (1.075); inner top = `TUBE_Y + TUBE_H/2 − INSET` (2.125);
     - the gas sits at the **top** of the tube, from `inner_top − gas` to `inner_top`, as gas collects in a real Hoffman tube;
     - the water fills from the inner bottom up to the gas.
   - With these values, H2 is gas 1.525 to 2.125 over water 1.075 to 1.525, and O2 is gas 1.825 to 2.125 over water 1.075 to 1.825.
2. **`world.rs` draws only what `tubes()` returns.** The electrolysis block becomes one loop: `for t in chem::electrolysis::tubes()`, which draws the glass, water (`liquid[1]`, as today) and a **new gas** cylinder for each tube. Each piece's transform is `Transform::from_xyz(x, p.y, z + t.dz).with_scale(Vec3::new(p.d, p.h, p.d))`. No numeric literal for any tube or column y, height, diameter or z offset stays in the block.
   - The gas pieces use one new material, `gas`, built next to `glass`. It is translucent (`AlphaMode::Blend`) and non-emissive. It must read clearly as different from the blue water, so the gas columns, not the water, read as "the stuff made". It must also stand out against the pale lavender alcove wall (`srgb(0.84, 0.8, 0.92)`). A 0.55-alpha white is too close to that wall, so the builder does not use near-white. A warm pale yellow is one option, for example `srgba(1.0, 0.92, 0.55, 0.6)`. The exact colour is the builder's choice, and the screenshot (test 7) must show it.
   - **Transparency.** The gas and water pieces sit inside the glass (diameter 0.2 inside 0.25) and touch only end to end, with no overlap (test 2), so they do not z-fight with each other or with the glass. **The glass keeps its current material and alpha.** The builder does not re-tune it.
   - The battery, the green ball, the bench, the `Target` and its `Obj` are unchanged.
3. **Comment fixed.** Replace the comment with one that is true, for example `// Electrolysis (Hoffman voltameter): 2H2O -> 2H2 + O2, so the H2 gas column (cathode side) is exactly twice the O2 column (anode side). Geometry and the 2:1 test live in chem::electrolysis.` The comment may say cathode and anode, because the battery is unmarked. It must not contain the characters `+` or `-` as polarity signs: no "+", "−", "positive" or "negative" for the electrodes. The `->` arrow in the equation is allowed.
4. **Register row re-pinned.** Edit only row P-015 in `content/science-sources.md`:
   - `where` becomes `crates/bevy-client/src/chem.rs, crates/bevy-client/src/world.rs` (comma and space, as in C-001);
   - the note says the gas columns at the top of equal tubes are 0.6 m (H2, cathode side) and 0.3 m (O2), set by `chem::electrolysis::GAS_H2 = 2 × GAS_O2` and checked by the test `electrolysis_gas_is_two_to_one`. Keep the existing Avogadro sentence;
   - verdict stays `correct`; value stays 2; sources unchanged.
   - The builder runs `npm test` locally before committing.

## Acceptance tests

Unit tests go in `chem.rs`'s existing `#[cfg(test)] mod tests` (line 688), run with `rustc --edition 2021 --test crates/bevy-client/src/chem.rs -o /tmp/chem && /tmp/chem` (the same command CI runs). Tests 1 to 4 fail on 855abfc because `chem::electrolysis` does not exist. Because `world.rs` must draw only what `tubes()` returns (test 5 checks this), tests 1 to 4 guard the drawn scene, not just constants.

1. **`electrolysis_gas_is_two_to_one`.** From `let [h2, o2] = tubes();`, assert `|h2.gas.h / o2.gas.h − 2.0| < 1e-5`.
2. **`electrolysis_columns_fit_tube`.** For each tube `t` in `tubes()`:
   - `t.water.h > 0.05` and `t.gas.h > 0.05`;
   - water bottom (`water.y − water.h/2`) ≥ glass bottom (`glass.y − glass.h/2`), and gas top (`gas.y + gas.h/2`) ≤ glass top, each within 1e-5;
   - water top equals gas bottom within 1e-5 (no gap, no overlap);
   - `water.h + gas.h` equals `glass.h − 2·INSET` within 1e-5;
   - `water.d < glass.d` and `gas.d < glass.d`.
3. **`electrolysis_h2_at_cathode`.** With `let [h2, o2] = tubes();`: `h2.dz < 0`, `o2.dz > 0`, `h2.dz == -o2.dz`, and `h2.gas.h > o2.gas.h`.
4. **`electrolysis_difference_readable`.** With `let [h2, o2] = tubes();`: `h2.gas.h − o2.gas.h ≥ 0.2`. This is the drawn gap, which a player can see from the bench about 2 m away.
5. **No literals left in the prop (grep, run by the tester on the branch).** The tester finds the block as the lines from the line holding `2H2O` down to the line holding `name: "lab bench"`, and runs each check on that range only:
   ```sh
   F=crates/bevy-client/src/world.rs
   A=$(grep -n '2H2O' $F | head -1 | cut -d: -f1); B=$(grep -n 'name: "lab bench"' $F | cut -d: -f1)
   sed -n "${A},${B}p" $F > /tmp/elec_block.rs
   grep -nE '0\.(55|35|25|9)\b|1\.(1|25|35|6)\b' /tmp/elec_block.rs   # must print nothing
   grep -c 'electrolysis::tubes()' /tmp/elec_block.rs                  # must be 1
   grep -cE '\.(glass|water|gas)\.' /tmp/elec_block.rs                 # must be at least 3
   grep -n 'cathode' /tmp/elec_block.rs                                # must print the comment line
   grep -nE 'positive|negative|\(\+\)|\(-\)|\(−\)' /tmp/elec_block.rs  # must print nothing
   grep -n 'H2 twice the O2' $F                                        # must print nothing
   ```
   The battery (`1.15`, `0.6, 0.3, 0.5`) and the ball (`1.2`, `0.3`) keep their literals. The first pattern does not match them. If it matches a battery or ball line, the tester reports it and does not fail the item for it.
6. **Science lint.** `npm test` passes. Row P-015's `where` is exactly `crates/bevy-client/src/chem.rs, crates/bevy-client/src/world.rs`, and its note contains `GAS_H2` and `0.6`.
7. **Build, golden path and screenshot.**
   - `crates/bevy-client/build-web.sh` builds. `QUERY=?low node tests/e2e/golden.mjs` passes 3/3 with no check edited. Brotli of `pkg-out` grows by ≤ 2 KB against the base.
   - **Screenshot (required).** With `?debug`, the tester teleports to face the lab bench from about 2 m (the alcove at x 21.8, z 6.0) and saves `docs/loop/items/I-018/screenshot.png`. **If the file is missing, the item fails.** The critic then checks by eye that two gas columns are visible at the tube tops against the alcove wall, one about twice the other, above blue water. If the gas does not stand out from the wall, the critic returns the item to the builder for a different tint.

## Out of scope

- Polarity labels, wires or "+"/"−" marks on the battery, and bubbles or animation of the gas forming. The prop stays static. A later quest item (I-008, water-splitting station) can add these and must keep using `chem::electrolysis`.
- Any change to the lab bench `Target`, its `Obj`, the chemistry rules, quests, or the `glass` material.
- Editing any other register row, or `ci.yml` (no CI change is needed, because the tests live in `chem.rs`).
- Every other `world.rs` prop.

## Contract

none. `chem::electrolysis` is new, and only `world.rs` (same lane) reads it.

## Lanes

| lane | owns | builds |
|------|------|--------|
| main | `crates/bevy-client/src/chem.rs` (adds `pub mod electrolysis` and tests 1–4 only; no change to existing rules or tests), `crates/bevy-client/src/world.rs` (the lab-bench electrolysis block and the one new `gas` material only), `content/science-sources.md` (row P-015 only), `docs/loop/items/I-018/screenshot.png` (tester) | Everything above, on a branch from origin/main 855abfc. Must not touch `main.rs`, `player.rs`, `feel.rs`, `web/**`, `tests/**` or `.github/**` |
