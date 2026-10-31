# I-035 critique (Spec-critic, wave 13)

**Verdict: revise.** I made the fixes in spec.md myself, each marked "(critic)". With them in, the spec is ready for the contract step. No re-review is needed unless the contract step turns up a contradiction.

Read: target.md, bars.md, areas.md, items/I-035/spec.md (127fd0f). I read no code. The facts the spec cites about code were taken as written. Where a fact looked open to doubt, I made the contract step check it.

## Will a 6-year-old and a 13-year-old both enjoy it and learn from it?
- **6-year-old: yes.** The level takes about 4 minutes and has two clear verbs: make it cold to cross, make it hot to open. Results are big and visible (ice crust grows, the gate melts, beads skate). Sprout picture icons and kind wrong-try lines are in. The level cannot be soft-locked. Frost is a Sprout spell. Fire is castable by every band (the Bench gates no cast), and F1 tracks the gap once I-029 lands.
- **13-year-old: yes, but thin.** The Apprentice lines carry real content: H₂O(l) → H₂O(s), Leidenfrost by name, and 77 K. The learning is the reversal: freeze to cross, melt to open, wrong chemistry fails. The puzzle itself is easy, since the order is forced: 3 Frost, then 3 Fire. This is acceptable for the smallest version. The panel may score the 13-year-old's fun lower (B5). I made no change. A prediction beat ("what happens if you heat the bridge?") belongs to I-034's mastery work.
- **B11:** met. Freezing the bridge and melting the gate are both non-shooting uses, and wrong chemistry visibly fails on the stream and the gate.

## Is it the smallest version?
Yes. It adds no Bench spell and no sim change, gives the client engine one new kind (`Stream`) and one goal mode, and builds level 2 only on entry. Clearing a level lasts one page load. Level select, saving and mastery gates stay in I-034. Three lanes, no shared files.

## Is the science right?
- 77 K: N₂ boils at 77.34 K. 273.15 − 77.34 = 195.8, so "196 K colder" holds. Correct.
- Melting and freezing at 0 °C (273 K) at 1 atm, ΔfusH 6.01 kJ/mol, and "ice stays at 0 °C while it melts": correct.
- Leidenfrost: liquid nitrogen drops on room-temperature or 0 °C water do skate on a vapour film. LN₂ (0.81 g/mL) also floats on water. The Adept line says film boiling insulates the drop, which slows heat flow. That is correct, and it is why LN₂ freezes water poorly, which explains the 3 casts. The spec sends the wording to a fetched source [S24], with a fallback, which is the right pattern.
- What a freeze spell can do: a real LN₂ pour on a stream deep enough that you can't wade freezes a thin surface crust, not a bridge that holds a person. The spec rates this `simplified` (as C-016). The Adept line names the heat-transfer limit. Acceptable.
- Fire on the liquid stream emits `Steam`, which is a real state change (liquid to gas). The lines only say heat cannot *freeze* water, so nothing is false. Fine.
- Air temperature: snow is scenery, `AMBIENT_C` stays 20, and no caption claims the valley is below 0 °C. Correct to forbid that. No line implies LN₂ is safe to touch. Good.
- CO₂ is the extinguisher gas, not dry ice, so "does not freeze water here" is right. If CO₂ ever becomes dry ice (195 K), this line must change.

## Can each acceptance test fail? Can lanes build from the contract alone? (issues found, all fixed)
1. **Arithmetic bug that made tests 1 and 9 fail.** With `STREAM_FREEZE = 0.34`, the second cast gives 0.68, not 0.67. Fixed to 0.34 / 0.68 / 1.0 (capped) in the kid walk-through, test 1 and test 9.
2. **Melting the bridge was inconsistent and could trap the player.** "Solid while ice < 1" makes the stream solid on the *first* Fire hit (0.66), not at `Melted`. A player standing on the bridge would end up inside a collider. Added: push the player out to the nearer bank landing in the same step (core). Added a unit test (test 4) and corrected test 1.
3. **Missing captions for stream Melting/Melted.** Without them, Fire on the bridge would show the gate line "The ice door melted!". Added one table row (2 new strings, which need C- rows).
4. **`Chilled` and the CO₂ no-op event were not in the contract.** The table keys on them, but the contract named only `Freezing`. Also, level 1's rule that Frost sets any water body to ice = 1 could fire on the gate. The contract now defines `Chilled` and `NoChange`, each added only if no existing event already means it. Test 1 pins that Frost on an `IceWall` changes nothing.
5. **Bench pad would vanish.** Tagging the pad `LevelRoot` means `load_level` despawns it, and nothing respawns it on return to level 1. The pad now stays spawned and is hidden in level 2.
6. **`onIce` on the bridge** (test 9) had no contract owner. Added: `on_ice` is true over a `Stream` with ice >= 1.
7. **Test 18 could fail falsely.** WASM linear memory never shrinks. Baseline is now taken after warm-up (switch 2). Entity count must match exactly.
8. **`#capicon`** may need an element in `index.html`. Added to the web lane (only if needed), within client-web.

## Left as is (notes for the builders)
- Test 7's `move 0 1 3` assumes the bridge's existing `move` arguments. The tester should confirm this against bridge.rs.
- The perf noise band (test 16) relies on perf-pairs, which exists. Fine.
