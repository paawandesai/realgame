# I-018 spec critique

**Verdict: approve with small changes.** It is the right thing and close to the smallest version. Nothing blocks a build. The changes below tighten the gate.

## Checks

- **Right thing for players?** Yes. The chemistry teacher persona is the buyer, and the 2:1 Hoffman voltameter is a standard demo. B6 has a measured error on exactly this row. It is a first-person prop, so no drift from the hard rule.
- **Smallest version?** Mostly. Two extra cylinders, one material, one constants module, one register row. Not smaller: the `gas` material and the "gas at the top" layout are needed, because the geometry has to read as gas. Do not add anything else.
- **Can every test fail?** Tests 1 to 4 fail on 3dbd332 because the module is missing, which is a weak kind of failure. They would also pass if `world.rs` ignored the constants. Test 5 greps partly cover that. See changes 2 and 3.
- **Lanes buildable from the spec alone?** Yes. One lane, and the facts match origin/main 3dbd332. I checked: the comment at line 1084, the `liquid` and `glass` materials, `mod tests` at `chem.rs` line 643 and row P-015 all exist as described. The `gameplay-standards.md` file you listed does not exist in docs/loop, so I could not check against it.

## Specific changes

1. **Rename the radius constants.** `TUBE_R = 0.25` and `FILL_R = 0.2` are the `scale.x`. `kit.cyl` is `Cylinder::new(0.5, 1.0)`, so these values are diameters, not radii. Call them `TUBE_D` and `FILL_D`, or state "scale = diameter" in the spec. Test 2's `FILL_R < TUBE_R` assertion must follow the rename.
2. **Make test 1 and test 2 guard the scene, not just the constants.** Split a pure helper in `chem::electrolysis` that returns the full set of pieces for one tube: glass, water and gas as `(y, height)` pairs. `world.rs` must then draw only what that helper returns. Otherwise the tests can pass while the drawn scene is wrong. Test 5 should name the helper as the only source of y and scale.y in the block.
3. **Replace the fragile grep in test 5.** `with_scale(Vec3::new(0.2, 0.(55|35), 0.2))` breaks if rustfmt reflows the line. Use `grep -nE '0\.55|0\.35' crates/bevy-client/src/world.rs` scoped to the electrolysis block, or a line range found by the `2H2O` comment. State which one the tester runs.
4. **Test 4 is a constant check.** It adds no value beyond test 1. Drop it, or turn it into a rendered-gap check on `columns()` output (`gas_h` difference at least 0.2 m).
5. **Test 8 is not a gate.** It already says so, but "the critic confirms by eye" needs an owner and an artifact. Say that the tester saves `screenshot.png` to the item folder and that its absence fails the item, since B6 was found by looking at the prop. Add that the gas must be visible against the alcove back wall. Check the contrast of 0.55-alpha white against the wall colour. If the wall is pale, the builder picks a different gas tint.
6. **Transparency ordering.** The gas, water and glass are all blend or emissive pieces in the same tube. Say that the gas and water pieces must not overlap or z-fight with the glass (the fill diameter 0.2 sits inside the tube 0.25, which is fine). Say "the glass keeps its current alpha", so the builder does not re-tune it.
7. **Comment rule.** "Must not name a polarity that the scene contradicts" is untestable. Replace it with: the comment may say cathode and anode, because the battery is unmarked, and must not mention "+" or "-".
8. **Register row separator.** Name the exact separator the existing multi-file rows use. Row P-014 and P-015 have one file, so the builder cannot copy the convention from them. If no row has two files, say what `test/science-sources.test.ts` accepts, and run that test locally before commit.
9. **Ordering with I-007.** The spec depends on I-007 merging first and its `based_on` pointing at a commit that does not exist yet. That is a scheduling risk, not a spec error. If I-007 stalls, the planner may rebase this spec onto I-007's branch instead of waiting. The files touched do not overlap with I-007's stated owner area except `world.rs`, so a conflict is possible but small.

## Acceptance for this critique

No `wrong-thing` finding. After changes 1 to 3 and 5, the item is ready to dispatch.

## Planner response

1. **Accept.** `kit.cyl` is `Cylinder::new(0.5, 1.0)`, so scale.x is a diameter; renamed to `TUBE_D` / `FILL_D` and the spec now says "scale = diameter".
2. **Accept.** Added `Piece`/`Tube` and `tubes()` in `chem::electrolysis`, which return glass, water and gas `(y, h, d)` per tube; `world.rs` draws one loop over `tubes()`, and tests 1 to 4 read `tubes()` output.
3. **Accept.** Test 5 now cuts the block from the `2H2O` comment line to `name: "lab bench"` and greps only that range for bare literals, so a rustfmt reflow cannot dodge it; the spec gives the exact commands.
4. **Accept (second option).** Test 4 now checks the drawn gap `h2.gas.h − o2.gas.h ≥ 0.2` from `tubes()`, not the constants.
5. **Accept.** The tester must save `docs/loop/items/I-018/screenshot.png`, and if it is missing the item fails. I checked the alcove wall: it is pale lavender `srgb(0.84, 0.8, 0.92)`, so near-white is banned and a warm pale yellow is the example tint.
6. **Accept.** The spec says the gas and water sit inside the glass (0.2 inside 0.25) and touch only end to end with no overlap (test 2), and that the glass keeps its current material and alpha (`glass` is added to out of scope).
7. **Accept.** The comment may say cathode and anode and must not use +/−, positive or negative for the electrodes; test 5 greps for both.
8. **Accept.** The separator is `, ` (C-001 and others), and `science-sources.test.ts` splits on `,` and trims. The spec names it and says to run `npm test` locally before committing.
9. **Accept, now moot.** I-007 merged as origin/main 855abfc (PR #24). I re-checked every fact against it (the literals are unchanged, the comment is at line 1084, `mod tests` is at line 688, P-015 is unchanged), moved `based_on` to 855abfc and removed the rebase gate.
