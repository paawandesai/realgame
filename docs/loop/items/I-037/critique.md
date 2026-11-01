# I-037 spec critique (Spec-critic, wave 13)

**Verdict: revise** (small edits, made in spec.md and marked "(critic)"). With them the spec is ready to build.

Read: docs/loop/target.md, bars.md, areas.md, items/I-037/spec.md only.

## Right thing, smallest, fun
- Fits the target: first-person, in the chain (Greenhouse, Frozen Gate, ..., Marsh), moves B8 (3rd level) and B11 (CO2 is a crafted spell; filling a hollow is a non-shooting use; Water/Frost/Fire fail with a reason).
- Short: about 5 CO2 casts plus a walk, about 3 min. A 5-year-old can win it: wide picking boxes on the floor plus vents, aim assist, "More!" captions, no lose state, no soft-lock (smothered vents cannot relight, pools never drain).
- Interesting at 13: the dip needs 2 casts and the deeper hollow needs 3 ("a bigger hollow needs more gas"), nothing goes out until the pool is full, wrong spells show different failure reasons, molar-mass captions. The stretch vent on a mound (S1, CO2 spills away) is the best extra idea; keep it if time allows.
- Scope is tight: new level3.rs plus hooks, no sim change, sensible out-of-scope list.

## Science
- CO2 44.01 vs air 28.96 = 1.52, "about 1.5 times": correct. Methane 16 g/mol is lighter than air, bubbles rise: correct. CH4 + 2O2 -> CO2 + 2H2O is balanced; "no oxygen, no fire" is correct. Water on a gas fire is marked simplified, with the true point (water does not remove the gas fuel): fine.
- **Fixed (critic):** the Adept Pooled line said "which is why the boardwalk is high", but the boardwalk sinks into the dip and the player walks through the filled pool. That contradicts the scene and the spec's own safety rule. The line now states the hazard without that claim, and the spec now sets the flames low and the haze at full just over the flame tops, far below head height.
- **Fixed (critic):** the Frost lines said liquid nitrogen boils into N2 "about as heavy as air, so it drifts away". Freshly boiled N2 is very cold and denser than air for a moment, so the lines now say "once it warms up". A simplified verdict is added: a big, sustained N2 flood can inert a space.

## Can the tests fail / be flaky
- Every listed check fails on main (no level 3, no keys, no GasVent).
- **Fixed (critic):** golden 9 asked for `z > 12.2`, which depends on collider size and could fail with correct code. It is now `z > 10` (the flame line), and the contract step must confirm that 3 s of walking would otherwise cover more than 8 m.
- **Fixed (critic):** golden 14 compared f32 pools 0.34 and 0.68 exactly, which can never match once serialised to JSON. It now uses a 1e-3 tolerance.
- **Added (critic):** the hollow's vents at x ±1.5 on a 3 m boardwalk leave a 3 m middle gap unless the colliders are wide. The spec now requires flame colliders to cover the full width (Rust test 5 already enforces it).

## Lanes can build from the contract
- Clear owners and paths, `chem.rs` tests compile alone, events named.
- **Added (critic):** adding `Obj.co2` breaks any existing test that builds `Obj` with a struct literal, which would mean editing a test. The fallback is `World.pools`, and the contract step records which it used.
- **Added (critic):** a fresh `?level=3` load must let the player cast CO2 (the bench is hidden outside level 1), or the level soft-locks. The contract step confirms this without touching I-029 files.

## Avoids I-029
- It does not touch `aim_and_cast`, `bench.rs`, `bench.js` or `progress.*`.
- **Added (critic):** both items may append golden keys after `I035_KEYS`. If I-029 merges first, `I037_KEYS` goes after its keys and the tail `slice(-N)` is resized to match. This is still only the allowed pinning of new keys.
