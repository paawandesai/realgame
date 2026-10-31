# I-035: Level 2, the Frozen Gate (freeze a bridge, melt a gate)

**based_on:** origin/main dc1d3364979e5451790c52b8a1438571df4e45f3 (merge of PR #44, I-027). **after: I-028** (origin/loop/I-028 head 384b910, about to merge). Lanes start only once I-028 is on origin/main; the contract step re-reads the line numbers below on that base and records the sha in a "Refresh" note. Code facts were read at dc1d336 and, for I-028 files, with `git show origin/loop/I-028:<path>`.

**Why now.** Paawan (2026-10-31): "start building out more levels and ways to play the game and learn more from the game", and config.md's wave rule: every wave ships something a player can see and play. This item merges three backlog rows into one playable slice: **I-035** (Frozen Gate: states of matter, 77 K, Leidenfrost), the **freeze/melt half of I-033** (freeze water into a bridge, melt an ice door), and the **smallest part of I-034** (an exit from level 1 after its fires are out, and a way back). Level select, mastery gates at exits and NGSS mapping stay in I-034; H2 balloon and oil fire stay in I-033.

## Facts this spec relies on (dc1d336 unless noted)

- The 3D rules engine is `crates/bevy-client/src/chem.rs` (no Bevy, deterministic, 30 Hz). It already models the two state changes this level is built on: Fire on an `IceWall` removes 0.34 ice per hit and turns it into a `Puddle` at 0 (`chem.rs:426-441`, events `Melting`/`Melted`); Frost on any water body sets `ice = 1` in one cast (`chem.rs:~520`, event `Froze`); Water on water is `Soaked`; Fire on liquid water is `Steam`. A frozen puddle is slippery (`player.on_ice`, `main.rs:~925`). Puddles are never solid; an `IceWall` is solid while `wall` (`world.rs:60-84`).
- `World::outcome` wins when `extinguished >= GOAL_FIRES (3)` and nothing burns; `reseed` (`chem.rs:~640`) relights a bush or, with no bush left, **declares the level won**. A world with no bushes would win on its first step: level 2 needs the win rule switched off (Contract).
- `world::build` returns `(kit, Colliders, Chem)` and is called once from `setup` (`main.rs:513-516`); `reset` (`main.rs:967-982`) clones `chem.initial` and says the level-1 line. Commands are matched in `control` (`main.rs:~840-897`); debug-only words are refused without `?debug`.
- `publish` (`main.rs:1678-1774`) writes `__gameState` with an ordered key list; golden checks that order (`golden.mjs:1044-1058`, `I027_KEYS` last). `objs[i].n` is the target's name; golden finds targets by name.
- Level-1 courtyard: `HALF = 24`; start (0, 0, 18) facing −z; greenhouse north; Formula Bench pad at (−3, 14.5) (I-028). The south wall behind the start is clear.
- I-028: `bench::BenchState { band: Option<Tier>, .. }` is a `pub` resource set by the `band` command, kept while the bench is closed, meant to be read by in-play systems. `bench::at_bench(pos)` lights `#benchbtn`; the pad is spawned by `bench::spawn_station` at Startup. Bench spells: Fire (tier_min seedling), Water, Frost, CO2 (sprout). The bench gates no cast (I-028 test 21).
- Science lint (`test/science-sources.test.ts`) scans `crates/bevy-client/src/**.rs` and `crates/bevy-client/web/**.{html,js}`, not `crates/sim`. Last register ids on I-028: `C-048`, `P-019`, `[S23]`.
- `tests/e2e/perf.mjs` builds its URL with `pageUrl(base, tier)` (`perf.mjs:105`) and takes no extra query yet.

## What a kid does (about 4 minutes)

0. **Level 1 ends differently.** The win banner keeps "You saved the greenhouse!" and its small line becomes "Walk to the glowing arch behind you to visit the Frozen Gate. Press R to play again." The **exit arch** at the south wall, centre (0, 22.6), until now a dark closed arch, lights up (emissive ice-blue, static, no flashing) and the fire finder arrow points at it. Walking into it (feet within 1.4 m of its centre) loads level 2. Before the win the arch is closed and solid; walking into it does nothing.
1. **0:00 Arrive.** A snowy valley, same size as the courtyard. Caption: "The Frozen Gate. Cross the stream, melt the gate, reach the lantern." A second arch behind the start, labelled with a greenhouse picture, leads back to the courtyard at any time.
2. **0:20 The stream.** 10 m ahead a stream crosses the whole valley, 4 m wide, too deep to wade (solid while liquid). Rock banks leave one landing on each side at x −2..2. Its water ripples (a slow scrolling texture; static under reduced motion).
3. **0:40 Freeze a bridge.** Frost on the stream: each cast adds a third of the ice; the ice crust visibly grows from the banks inward (0.34, 0.68, 1.0; (critic) 2 × 0.34 is 0.68, and the third cast caps at 1). On each Frost hit, 4 to 6 small white beads skate across the water for about 1 s and shrink, inside white mist: liquid nitrogen gliding on its own vapour (the **Leidenfrost effect**). At ice 1.0 the stream is a solid, slippery ice sheet and stops blocking. The kid crosses.
4. **1:30 Wrong chemistry fails kindly.** Fire on the liquid stream: steam, no change. Water on it: no change. CO2: no change. Each gives a band caption saying what would work.
5. **2:00 The gate.** A rock ridge crosses the valley at z = −8 with one opening, filled by a thick ice gate. Fire melts it in 3 hits (existing rule) into a puddle; Frost on the gate only chills it; Water and CO2 do nothing.
6. **2:40 The lantern.** Behind the gate, a glowing lantern on a plinth at (0, −17). Walking within 1.5 m wins: banner "You opened the Frozen Gate!" with "Freeze to cross, melt to open: water changed state both ways. Walk to the arch to go back." `R` restarts level 2.
7. **Way back.** The back arch at (0, 22.6) in level 2 returns to the courtyard, fresh (fires and all), and the courtyard exit stays lit for the rest of the page load, so the kid can come back without winning again.

If the bridge melts again (Fire on it), Frost re-freezes it: the level cannot be soft-locked. (critic) The stream turns solid again on the first Fire hit (ice < 1), so a player standing on it at that moment is moved to the nearer bank landing (z 8.4 or z 3.6, x clamped to −2..2) in the same step; they are never left inside a collider.

## What they learn, shown per band

The band is `BenchState.band`; when it is `None` (no picker, no bench yet: golden, perf, panel builds) the **Apprentice** line shows, which matches level 1's captions. Sprout lines also set `capIcon` so the HUD shows a large picture (snowflake for freeze, flame-on-drop for melt, cross for a wrong try) next to the words (GS-17, GS-20). Lines are exact; each is a register row unless it reuses one.

| event (level 2 only) | Sprout 4-7 | Seedling 8-10 | Apprentice 11-15 (and None) | Adept 16+ |
|---|---|---|---|---|
| Freezing (stream, ice < 1) | `Cold! The water is turning to ice.` | `See the drops skate? Liquid nitrogen boils so fast it rides on a cushion of gas.` | `Freezing: H₂O(l) → H₂O(s). The nitrogen drops glide on their own gas (the Leidenfrost effect), so it takes a few casts.` | `Film boiling (Leidenfrost): a layer of N₂ gas insulates each 77 K drop and slows the heat flow. Freezing water must lose 6.01 kJ/mol.` |
| Froze (stream solid) | `Ice bridge! Now walk across.` | `The water froze into solid ice. A solid keeps its shape, so it holds you up.` | C-016 (reused) | `Frozen: H₂O(l) → H₂O(s) at 0 °C (273 K). Liquid nitrogen at 77 K is 196 K colder, so heat kept flowing out.` |
| Melting (gate) | `Hot! The ice is melting.` | `Heat is melting the ice into water.` | C-015 (reused, `Melting... it takes a lot of heat to turn ice into water.`) | `Ice stays at 0 °C while it melts: the heat (6.01 kJ/mol) breaks up the solid instead of warming it.` |
| Melted (gate) | `The ice door melted! Go through.` | `The ice melted: solid ice became liquid water.` | C-014 (reused) | `Melted at 0 °C (273 K): H₂O(s) → H₂O(l). Same molecules, new state.` |
| (critic) Melting / Melted (stream, Fire on the bridge) | `The bridge melted! Make it cold again.` | `The ice melted back into liquid water. Freeze it again to cross.` (Seedling, Apprentice, Adept) | ← | ← |
| Frost on ice (Chilled) | `Brrr! Cold makes ice colder. Try heat!` | `Cold cannot melt ice. Ice needs heat to turn into water.` (Seedling, Apprentice, Adept) | ← | ← |
| Fire on liquid stream (Steam) | `Hot water is still water. Try cold!` | `Heat cannot freeze water. To make a solid, take heat away.` | ← | ← |
| Water on stream or gate (Soaked) | `More water! Try cold or heat.` | `Adding water does not change its state. Heat melts ice, cold freezes water.` | ← | ← |
| CO2 on stream or gate | `That is a gas. Try cold or heat.` | `CO₂ keeps oxygen away. That does not melt ice or freeze water here.` | ← | ← |

Level 1 captions do not change in this item (band captions for level 1 are I-021).

## Spells: no new Bench spell

Freeze is the existing **Frost** (liquid nitrogen, `N2(l)`) and Melt is the existing **Fire** (heat). Both are already learnable at the Bench (Fire from Seedling, Frost from Sprout) and castable by every band, so the Bench needs no change. Known gap, filed as follow-up **F1**: once I-029 makes casting depend on learning, a Sprout child cannot learn Fire, so the gate needs a Sprout heat route (a Sprout "Warm" spell or a band rule) before I-029 merges. A dedicated heat spell with a temperature setting belongs to I-031.

## Acceptance tests (each can fail)

Rust unit tests (`cargo test`):
1. `chem`: a `Stream` freezes in exactly 3 Frost casts (`ice` 0.34, 0.68, 1.0 (critic), events `Freezing`, `Freezing`, `Froze`); it is solid for movement while `ice < 1` and not solid at 1; Fire on a frozen stream removes 0.34 per hit and it is solid again from the first Fire hit (ice 0.66), with `Melted` at 0 (critic: not only at `Melted`); Water, Fire (liquid) and CO2 never change its `ice`. (critic) Frost on an `IceWall` emits `Chilled` and leaves `ice` and `wall` unchanged (it does not take level 1's water-body "set ice = 1" path); CO2 on a `Stream` or `IceWall` emits `NoChange` and changes nothing.
2. `chem`: a `World` built with `Goal::Reach` never wins or reseeds by itself over 20 000 steps; `reach()` latches `Won`; `apply` after that does nothing. Level 1 (`Goal::Fires`) behaviour is unchanged (existing tests pass unedited).
3. `level2`: the caption table returns the exact strings above for each (event, band) pair; `None` gives the Apprentice line; every string passes `is_player_text`-style checks (no `"`, `\`, control chars).
4. `level2`: from the level-2 start, a straight walk to the lantern is blocked by the liquid stream and by the standing gate (collider sweep), and is clear once both are changed. (critic) A player standing on the frozen stream when one Fire hit lands ends that step on a bank landing, outside every collider.

Golden (`tests/e2e/golden.mjs`, new section `frozenGate`, run last before "no console errors"; drives the state bridge only):
5. **Exit stays shut before the win:** `tp 0 21.8` with fires burning leaves `level === 1`, `exitOpen === false`.
6. **Reach level 2 by playing:** win level 1 the existing way, then `tp 0 21.8` → within 5 s `level === 2`, `objs` contains `stream`, `ice gate`, `lantern`, `burning === 0`, `outcome === 'playing'`, the caption starts "The Frozen Gate".
7. **Stream blocks:** `move 0 1 3` from the start toward −z leaves the player at z > 8.2 (south bank of the stream, z 4..8) while the stream is liquid.
8. **Wrong chemistry fails:** Fire, Water, CO2 on the stream leave `ice === 0`; Frost on the gate leaves `wall === true`; Water and CO2 on the gate leave `wall === true`.
9. **Freeze a bridge and cross:** three Frost casts → `ice` 0.34/0.68/1.00 (critic) and `frozen`; a scripted `move` carries the player to z < 3 (across), `onIce` true while on it.
10. **Melt the gate:** three Fire casts → `wall === false`; walking to the lantern sets `outcome === 'won'` and the banner reads "You opened the Frozen Gate!".
11. **Captions per band:** for each band (pushed as `band <id>` on `__gameIn.bench`, I-028's command) a fresh level 2 Frost on the stream shows exactly that band's Freezing line, and Sprout sets `capIcon === 'freeze'`; with no band the Apprentice line shows.
12. **Way back:** `tp 0 21.8` in level 2 → `level === 1` with 3 fires and `exitOpen === true`.
13. **Leidenfrost beads are visible:** in the frame after a Frost hit on the liquid stream, `fxBeads >= 4`, and 0 within 2 s.
14. **Key order:** the new `__gameState` keys are appended after `I027_KEYS` in the order `level, exitOpen, capIcon, fxBeads` (`I035_KEYS`, the one sanctioned edit to the key-order check); `objs` keys unchanged.
15. **No console errors and no panics** over the whole run including both transitions, 3/3 runs.

Perf and lint:
16. **Perf at `?low`:** `perf.mjs --query level=2` (new flag) median CPU frame at 4x throttle is not worse than level 1's on the same build by more than the noise band perf-pairs reports (5 pairs); entity count and draw calls in level 2 ≤ level 1's. Brotli size grows ≤ 0.2 MB. TTI unchanged (level 2 builds only on entry).
17. **Science lint** passes: every string in the table and the banners has a `C-` row or a `## Not claims` row; every new constant has a `P-` row.
18. **Switching levels 10 times** (golden loop of `level 2` / `level 1` debug commands) leaves entity count and WASM memory flat (± 2 %): nothing leaks. (critic) WASM linear memory never shrinks, so the baseline is taken after switch 2 (warm-up) and compared with the value after switch 10; entity count is compared with the first level-1 load exactly (± 0).

## Science rows (content/science-sources.md, next ids C-049.., P-020..)

- One `C-` row per new string in the table and banners, citing [S1] (N2 boils 77.34 K = −196 °C), [S10] (OpenStax 10.3: melting/freezing at 0 °C, ΔfusH of water 6.01 kJ/mol, solids keep their shape), and a **new Leidenfrost source [S24]** (Quéré, D., "Leidenfrost Dynamics", Annu. Rev. Fluid Mech. 45:197-215, 2013, doi:10.1146/annurev-fluid-011212-140709; the content builder fetches it and quotes the passage on drops levitating on their own vapour; if no fetched source supports drops of liquid nitrogen gliding on **water**, the Seedling/Apprentice Leidenfrost lines say "on a cushion of their own gas" without "on the water", and the bead fx stays).
- Expected verdicts: the 3-casts-to-freeze rule and the whole stream freezing are `simplified` (as C-016: real freezing is heat-transfer-limited; film boiling makes LN2 a poor freezer, which is what the Adept line says). 196 K difference: 273.15 − 77.34 = 195.8 K, rounds to 196 (tolerance rules). "Same molecules, new state": correct for a physical change.
- `P-020` stream ice per Frost cast `0.34` (gameplay tuning, [S0]); `P-021` level-2 air temperature: **none added**. The engine keeps `AMBIENT_C = 20` and has no melting by air, so no string in level 2 states an air temperature; the snow is scenery only (the reviewer should flag any caption that says the valley is below 0 °C).
- Not claims: the objective line, "Walk to the glowing arch…", "Press R…", arch labels.
- Safety: no line says liquid nitrogen is safe to touch. The fresh science reviewer runs `docs/loop/science-review.md` on the branch before merge (B6 stays 0 errors).

## Out of scope

Level select, saving which levels are cleared (cleared lasts one page load; `?level=2` loads level 2 directly for teachers and tests), mastery-trial gates at exits, NGSS mapping (all I-034); H2 balloon, oil fire (I-033); a new heat spell or temperature setting (I-031); level-1 band captions (I-021); creatures, sound design beyond reusing the existing hit/out tones; level 2 art beyond the shared low-poly kit (snow is a white ground material and white-capped walls).

## Contract (the contract step writes stubs for these before lanes start)

- `chem.rs`: `Kind::Stream` (water body; `is_water()` true; solid while `ice < 1.0`), `EventKind::Freezing`; (critic) `EventKind::Chilled` (Frost on `IceWall`) and `EventKind::NoChange` (CO2 on `Stream`/`IceWall`), each added only if `chem.rs` has no event that already means it, which the contract step checks and records; Frost on a `Stream` adds `STREAM_FREEZE = 0.34` ice (cap 1) and emits `Freezing` or `Froze`; other kinds unchanged. `pub enum Goal { Fires, Reach }`, `World::with_goal(objs, Goal)`; `World::new` stays `Goal::Fires`; under `Reach`, `reseed` and the fire win are off; `pub fn reach(&mut self)` latches `Won`. `world::solid_obj` treats `Stream` as solid while `ice < 1`. (critic) `player.on_ice` is true over a `Stream` with `ice >= 1` (same rule as a frozen puddle); the bank push-out when a stream turns solid under the player lives in `main.rs` (core).
- `level2.rs` (new, client-world): `pub fn build(commands, meshes, materials, images) -> (Colliders, Chem)` with targets named exactly `stream`, `ice gate`, `lantern` (lantern is a `LabBench`-like inert kind or `Kind::Lantern`; spells on it say nothing new), `pub const START: Vec3`, `BACK_ARCH: (f32, f32) = (0.0, 22.6)`, `LANTERN: (f32, f32) = (0.0, -17.0)`; `pub fn caption(kind: EventKind, obj: Kind, band: Option<Tier>) -> Option<(&'static str, &'static str)>` returning (text, capIcon). All level-2 entities carry `LevelRoot`.
- `main.rs` (client-core): `#[derive(Resource)] struct Level { n: u8, cleared1: bool }`; `world::build` entities also get `LevelRoot`; `fn load_level(n)` despawns `LevelRoot`, builds the level, swaps `Colliders`/`Chem`, places the player; triggers: exit arch (level 1, won or `cleared1`), back arch and lantern (level 2); `reset` reloads the current level; commands `level <1|2>` (debug-only) and URL `?level=2` (not debug); `react` asks `level2::caption` first when `Level.n == 2`. `__gameState` appends `level` (1|2), `exitOpen` (bool), `capIcon` ("" | "freeze" | "melt" | "wrong"), `fxBeads` (live bead count). `bench.rs`: (critic) the pad is spawned once at Startup and is **not** tagged `LevelRoot` (despawning it would lose it, since `load_level(1)` does not rerun `spawn_station`); in level 2 it is `Visibility::Hidden` and `atBench` is false (one guard), and it shows again in level 1.
- `fx.rs`: `spawn_beads(pos, n)` (4 under `?low`, 6 otherwise), a count resource for `fxBeads`.
- `hud.js`: banner text per level and outcome, objective per level ("Cross the stream, melt the gate, reach the lantern."), finder arrow points at the exit when `exitOpen && level === 1 && outcome === 'won'` (reuse `#firefinder` with a class `exit`), Sprout picture `#capicon` from `capIcon`.

## Lanes (max 3; one owner per file)

| lane | area | paths |
|---|---|---|
| world | client-world, client-fx | `crates/bevy-client/src/chem.rs`, `crates/bevy-client/src/world.rs` (LevelRoot tags, exit arch), `crates/bevy-client/src/level2.rs` (new), `crates/bevy-client/src/fx.rs` |
| core | client-core | `crates/bevy-client/src/main.rs`, `crates/bevy-client/src/bridge.rs` (if needed), `crates/bevy-client/src/bench.rs` (one guard) |
| web | client-web, content, tests | `crates/bevy-client/web/hud.js`, `crates/bevy-client/web/style.css`, `crates/bevy-client/web/index.html` (critic: only if `#capicon` needs a new element), `content/science-sources.md`, `test/science-sources.test.ts` (only if a new path needs scanning); golden `frozenGate` section and `perf.mjs --query` are tester-owned (tests-e2e) |

**sim (`crates/sim`): no change.** The 3D level rules live in the client engine `chem.rs`, and player text must live where the science lint scans. Shared files: none (no new crates or JS files; `level2.rs` joins `main.rs`'s `mod` list, owned by core).

## Follow-ups filed by this spec

- **F1** Sprout heat route for the gate before I-029 gates casts on learning.
- **F2** Save cleared levels and a level-select screen (fold into I-034).
- **F3** Band captions for level 1 (already I-021).
