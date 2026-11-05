# I-056: the "by rule" tier and a world use for every spell (B2)

Phase B2 of the plan "Formula: free-play Bench to spells" (artifact ARwyByPYQG5Lp6e32JGc1a). Branch `loop/bench-b2`, from `loop/bench-play`. The brief called this item I-055; that id was already taken, so it is **I-056**.

## 1. What changes for the player

- The Bench knows about 359 more reactions, **predicted by rule**: acid + base, metal + acid, carbonate + acid, fuel burns, two salts swap (precipitation) and metal swap (displacement). A rule entry says "Predicted by rule (acid + base): An acid and a base cancel each other out, ..." in the codex and on the result card. Checked (core) reactions are unchanged and always win.
- Every saveable spell does something in the world, and the Save button says what ("Save as a spell (a wall spell): it builds a short-lived wall or plugs a gap"). A reaction with nothing safe to do out there is **Bench only**: the Save button is disabled and says why ("Bench only: It makes nitrogen dioxide, a poisonous gas, so it stays on the Bench."). The pack says "In the world: it ..." for each saved spell.
- New world uses: **Wall** (a solid forms: a short-lived wall of blocks), **Reveal** (a colour appears: the beacon stands on a hidden cache nearby for a while), **Lift** (hydrogen: the H2 lift) and **Push** (another gas: the CO2 dash's launch, away from the burst).

## 2. The rule tier (`content/chem/rules.json`)

Generated, never hand-edited:

```
python -I tools/chemdb/build_rules.py content/chem       # rules.json, names.json additions
python -I tools/chemdb/build_show_data.py content/chem   # substances.json, geometry.json (show records)
```

`build_rules.py` holds the rule data with sources: the solubility rules (OpenStax Chemistry 2e Table 4.1, `OPENSTAX-SOL`), the reactivity series from standard reduction potentials (CRC Handbook, `CRC-ECS`; a metal displaces another only when E° differs by at least `margin_v` = 0.25 V, and metals that react with water first, `water_first`, are left out of displacement), the acids (HNO3 is oxidising, so it gives no metal + acid entry), combustion (`OPENSTAX-COMB`) and GHS hazards. It writes the rule text, every entry's name, say and short text, the ion and salt tables the Rust checks re-derive from, and the bottles that make each band's entries makeable.

Each entry: `id` `rule_<rule>_<reactants>`, `tier: "rule"`, `rule`, `family`, `band`, `heat` (from ΔH: little when |ΔH| ≤ 25 kJ), `dh_kj` (from CRC/NBS ΔHf via the show data), `hazard` and `hazard_why`. A show fix per entry (`show`) gives fuels their flame and neutralisations a note and speed. Every new species has states, a sourced ΔHf, a sourced colour where it has one, and geometry in `substances.json` and `geometry.json`.

**Bands.** Seedling: a simple acid + base or metal + acid whose reactants are all Seedling bottles. Adept: anything with a less familiar substance (bromides, phosphates, lithium and barium salts, ammonium carbonate, iron(III), heavier fuels). Apprentice: the rest.

**Left out:** fluorides and HF, cyanides, perchlorates, chromates, mercury salts, and explosive products; lead only for core's lead salts with group-1 or ammonium partners; no silver hydroxide, no iron(III) or aluminium carbonate, no ammonium + hydroxide. A candidate whose reactants (reduced) match a core reaction is dropped (never shadows or duplicates core), as is one missing a sourced ΔHf. 93 were dropped.

**Hazards.** Every rule entry with a corrosive or toxic reactant or product (strong acids and alkalis, silver nitrate, barium, copper and lead salts, ...) has `hazard: lab` and names why. Fuels have `hazard: fire`. (Core does not flag equivalent dilute acid + base reactions; that difference is left for the science review.)

**Counts (359):**

| rule | seedling | apprentice | adept | total |
|---|---|---|---|---|
| acid_base | 4 | 19 | 40 | 63 |
| metal_acid | 1 | 1 | 4 | 6 |
| carbonate_acid | 0 | 27 | 15 | 42 |
| fuel_burns | 0 | 7 | 7 | 14 |
| precipitate | 0 | 58 | 142 | 200 |
| displacement | 0 | 10 | 24 | 34 |
| **total** | 5 | 122 | 232 | 359 |

**Engine.** `Codex::load()` merges core and rules (`CodexTier::Checked` and `Rule`); `run()` prefers a checked reaction, then the band, then the tier. `Codex::load_core()` is core alone. `crates/chem/src/rules.rs` (`RuleData`, `check`) re-derives each entry from the tables: a precipitate is the one insoluble pair of a swap, a displacement follows the series with the margin, a metal + acid has a metal above hydrogen and a non-oxidising acid, a carbonate + acid gives salt, water and CO2, a fuel burns to CO2 and water only.

## 3. Tests (chem crate)

`rules.rs`: the tier loads with its rules; every rule reaction balances and is charge neutral; every one passes its rule's check; the checks reject wrong chemistry; every one is makeable from its band's bottles (core shelf plus rules.json bottles); none duplicates or shadows core; corrosive and toxic ones carry the lab badge; every one has a complete show record (something visible or a note, ΔH present and agreeing with the heat class, a precipitate's colour and grain, bubbles for a gas, a flame for a fuel, a coating for a displacement).

## 4. World uses

| what the reaction does | kind | runs | flies as | in the world |
|---|---|---|---|---|
| burns, or a gas that catches fire | fire / glow | Fire | Fire | burns, melts ice, lights |
| gives off heat in a solution, no colour | glow / water | Water | Water | warm or plain splash |
| takes in heat | frost | Frost | Frost | freezes water, cools |
| gives off only CO2 or N2 | foam | Foam (CO2) | CO2 | smothers fire |
| a solid forms (precipitation) | wall | **Wall** | Frost | a short-lived wall or plug |
| a colour change or a metal coating, in a solution | reveal | **Reveal** | Fire | hidden caches show for a while |
| gives off hydrogen | foam | **Lift** | Water | lifts a player near the landing |
| gives off another harmless gas (O2, steam, ...) | foam | **Push** | CO2 | pushes a player near the landing away |
| gives off a poisonous gas | foam | **Bench only** | - | cannot be saved; says why |

Ordered rules in `content/chem/effects.json`: `kind_rules` (a `reveal` rule sits after the fire-hazard rule, for the solution families with a `coating` or `colour_change` in the show record), `runs`, then `gas_uses` for a fizz that does not smother: a flame → Fire; any of NO, NO2, N2O4, NH3, HCl, HBr, HF, Cl2, Br2, H2S, SO2, CO, CH3Cl → Bench only ("It makes {gas}, a poisonous gas, so it stays on the Bench.", `{gas}` the everyday name from names.json); H2 → Lift; anything else → Push. `Effect` gains `use` (`Runs::world`, plain words) and `bench_only`; `Effect::saveable()` is false only for Bench only.

**Counts:** core 182: fire 69, water 18, frost 30, foam 10, wall 14, reveal 12, lift 14, push 4, none 11. Rule 359: fire 14, water 46, frost 4, foam 38, wall 200, reveal 51, lift 6. All 541: fire 83, water 64, frost 34, foam 48, wall 214, reveal 63, lift 20, push 4, none 11 (none = Bench only).

The 31 former "harmless puffs": Lift 14 (metal + acid, water + metal, steam, ...), Push 4 (peroxide_splits, potassium_chlorate_heat, ethene_hydrogen, ozone_splits), Fire 2 (potassium_water and magnesium_steam: the hydrogen catches fire as it forms), Bench only 11 (haber, hcl_synthesis, hbr_synthesis, hf_synthesis, ostwald_1, no_oxidises, ostwald_3, copper_nitric, hydrogen_sulfur, methane_chlorine, nitrogen_dioxide_dimer).

Other core changes: the 14 core precipitations now run Wall (were Water); 12 core solution reactions with a colour change or coating now run Reveal (were Water): the displacements, the halogen displacements and the coloured neutralisations.

Rule tier by use: acid_base → reveal 17, acid_base → water 46, carbonate_acid → foam 38, carbonate_acid → frost 4, displacement → reveal 34, fuel_burns → fire 14, metal_acid → lift 6, precipitate → wall 200.

## 5. The game side (`crates/bevy-client`)

- `discover.rs`: `runs_of(disc)` and `lands_as_matter(disc)`; a Bench-only id in a save is dropped on load and `put` refuses it. `Data.world` holds the data values.
- `blockchem::drive`, `firebreak::douse`, `late::clamp_fire`: a Wall, Reveal, Lift or Push is not a matter emit, so the shipped spell it flies as does not also burn, douse or light where it lands.
- `world_use.rs` (new): on `SpellLanded` with a discovery whose run is Wall, Reveal, Lift or Push. **Wall** sets blocks like the ice walkway (`build_wall`): the cell in front of the hit face, dropped up to 3 cells to stand on the ground, `width` cells across the line of the throw and `height` up, only air or water cells outside protected stamps; after `seconds` the cells that still hold its material go back to air (`crumble`). **Reveal** (`hidden_near`): the unfound caches (`powers::SITES`, `Powers.found`) within `radius_m` of the landing, nearest first; the objective beacon stands on the nearest for `seconds` (`Uses::revealed`, read by `beacon::place`). **Lift**: a grounded player within `radius_m` rises `blocks` (`Motion::lift`). **Push**: a player within `radius_m` is launched away from the landing at `speed` with the dash's upward kick (`Motion::push`, new, beside `dash`). Each says a caption.
- No shader, material or new mesh: walls are an existing block (ash), the reveal reuses the beacon.

## 6. Data values for the graphics thread (`effects.json` `world`)

Ranges are `[weakest, strongest]`, scaled by the spell's `strength` (0 to 1, `discover::lerp`).

| use | values |
|---|---|
| wall | `material` "ash" (`mat::ASH`, id 21, solid, opaque), `width` [1, 5] blocks across the throw, `height` [1, 3] blocks, `seconds` 15 before it crumbles to air |
| reveal | `radius_m` [24, 64] around the landing, `seconds` 20 the beacon stands on the cache |
| lift | `radius_m` 6 from the landing, `blocks` [3, 8] of rise |
| push | `radius_m` 6, `speed` [8, 20] m/s away from the landing (plus the dash's 5 m/s up) |

Effect kinds: `wall` and `reveal` join fire, water, frost, foam, cloud and glow, each with `kind`, `heat`, `strength` and `tint` as before (a wall's tint is the precipitate's colour; a reveal's is the coloured product's). Flight: Wall as Frost, Reveal as Fire, Lift as Water, Push as CO2 (`Runs::slot`). To find a landing's use: `SpellLanded.disc` → `discover::runs_of`. Live state: `world_use::Uses` (`walls` with their cells and `until`, `reveal`, `reveal_at`, `reveal_until`).

## 7. The Bench page

`codex_json()` adds `rules` (id, family, name, text, source). `bench-free.js`: `ruleHtml` shows "Predicted by rule (name): text" on a found rule entry and on its result card; `saveSpellButton` adds the world use or shows the Bench-only reason on a disabled button; `KIND_WORD` has wall and reveal. `pack.js`: "In the world: it ...". `content/science-sources.md`: Not-claims rows for the new captions and templates.

## 8. Acceptance tests

- chem `rules.rs`: section 3.
- chem `discover.rs`: `every_saveable_spell_has_a_world_use_and_bench_only_ones_say_why` (a saveable spell has a use in plain words and no reason; a Bench-only one has a reason naming its gas; the use agrees with the show record: Wall has a precipitate, Reveal a coating or colour change, Lift hydrogen, Push a gas; every `Runs` value is used; every rule precipitate walls; the data values are in range); `effects_follow_the_chemistry` updated (silver chloride Wall, zinc + acid Lift, iron + copper sulfate Reveal, peroxide Push, NO oxidising Bench only, potassium + water Fire).
- bevy-client `world_use.rs`: a wall stands across the throw and crumbles (leaving a block the player changed); a wall drops to the ground, plugs water and spares stamps; a reveal shows only unfound caches within its radius and ends; hydrogen lifts and a gas burst pushes a player nearby, and not one far away; the world uses come from the codex. `discover.rs`: a Bench-only reaction is never saved or equipped; a hydrogen fizz carries its strength and is not a matter landing.
- `test/bench-spells.test.ts`: the kinds list includes wall and reveal.

## 9. Files

New: `tools/chemdb/build_rules.py`, `content/chem/rules.json`, `crates/chem/src/rules.rs`, `crates/bevy-client/src/world_use.rs`, this spec. Changed: `tools/chemdb/build_show_data.py`, `content/chem/{substances,geometry,names,effects}.json`, `crates/chem/src/{lib,show,discover,web}.rs`, `crates/bevy-client/src/{discover,blockchem,firebreak,late,beacon,powers,main}.rs`, `crates/bevy-client/web/{bench-free.js,bench-free.css,pack.js}`, `content/science-sources.md`, `test/bench-spells.test.ts`, `docs/loop/items/I-053/spec.md`.

## 10. The current mapping table (the 182 core reactions)

Generated from the Rust (`Effects::of`). Rule-tier entries follow section 4 by rule; their full list is `content/chem/rules.json`.

| reaction | family | band | kind | strength | runs | Bench-only reason |
|---|---|---|---|---|---|---|
| `methane_burns` | combustion | sprout | fire | 1 | fire |  |
| `hydrogen_burns` | combustion | sprout | fire | 0.92 | fire |  |
| `water_splits` | electrolysis | seedling | frost | 0.24 | frost |  |
| `carbon_burns` | combustion | sprout | fire | 0.89 | fire |  |
| `iron_rusts` | oxidation | sprout | glow | 1 | fire |  |
| `baking_soda_vinegar` | carbonate_acid | sprout | frost | 0.52 | frost |  |
| `photosynthesis` | life | sprout | frost | 0.25 | frost |  |
| `respiration` | life | sprout | foam | 1 | foam |  |
| `candle_burns` | combustion | sprout | fire | 1 | fire |  |
| `peroxide_splits` | decomposition | sprout | foam | 0.79 | push |  |
| `magnesium_burns` | combustion | sprout | fire | 1 | fire |  |
| `propane_burns` | combustion | sprout | fire | 1 | fire |  |
| `ethanol_burns` | combustion | seedling | fire | 1 | fire |  |
| `fermentation` | life | sprout | foam | 0.69 | foam |  |
| `limewater_test` | precipitation | seedling | wall | 0.65 | wall |  |
| `sodium_water` | water_metal | seedling | foam | 0.88 | lift |  |
| `lithium_water` | water_metal | seedling | foam | 0.91 | lift |  |
| `potassium_water` | water_metal | apprentice | foam | 0.89 | fire |  |
| `calcium_water` | water_metal | seedling | foam | 0.9 | lift |  |
| `barium_water` | water_metal | adept | foam | 0.91 | lift |  |
| `strontium_water` | water_metal | adept | foam | 0.91 | lift |  |
| `magnesium_steam` | water_metal | apprentice | foam | 0.88 | fire |  |
| `salt_from_elements` | synthesis | seedling | glow | 1 | fire |  |
| `potassium_chlorine` | synthesis | apprentice | glow | 1 | fire |  |
| `magnesium_chlorine` | synthesis | apprentice | glow | 0.97 | fire |  |
| `calcium_chlorine` | synthesis | apprentice | glow | 1 | fire |  |
| `aluminium_chlorine` | synthesis | adept | glow | 1 | fire |  |
| `copper_chlorine` | synthesis | apprentice | glow | 0.81 | fire |  |
| `sodium_bromine` | synthesis | adept | glow | 0.98 | fire |  |
| `iron_sulfur` | synthesis | seedling | fire | 0.69 | fire |  |
| `magnesium_sulfur` | synthesis | seedling | glow | 0.87 | fire |  |
| `copper_oxide` | oxidation | seedling | fire | 0.86 | fire |  |
| `zinc_oxide` | oxidation | apprentice | fire | 0.98 | fire |  |
| `aluminium_oxide` | oxidation | apprentice | fire | 1 | fire |  |
| `lithium_oxygen` | oxidation | apprentice | glow | 1 | fire |  |
| `sodium_oxygen` | oxidation | seedling | glow | 1 | fire |  |
| `calcium_oxygen` | oxidation | apprentice | glow | 1 | fire |  |
| `silicon_oxygen` | oxidation | adept | fire | 1 | fire |  |
| `sulfur_burns` | combustion | sprout | fire | 0.85 | fire |  |
| `phosphorus_burns` | combustion | apprentice | fire | 1 | fire |  |
| `co_burns` | combustion | apprentice | fire | 0.95 | fire |  |
| `carbon_little_air` | combustion | apprentice | fire | 0.81 | fire |  |
| `ethane_burns` | combustion | apprentice | fire | 1 | fire |  |
| `butane_burns` | combustion | sprout | fire | 1 | fire |  |
| `pentane_burns` | combustion | apprentice | fire | 1 | fire |  |
| `hexane_burns` | combustion | apprentice | fire | 1 | fire |  |
| `octane_burns` | combustion | apprentice | fire | 1 | fire |  |
| `ethene_burns` | combustion | apprentice | fire | 1 | fire |  |
| `acetylene_burns` | combustion | apprentice | fire | 1 | fire |  |
| `benzene_burns` | combustion | adept | fire | 1 | fire |  |
| `methanol_burns` | combustion | apprentice | fire | 1 | fire |  |
| `propanol_burns` | combustion | adept | fire | 1 | fire |  |
| `ammonia_burns` | combustion | adept | fire | 1 | fire |  |
| `hydrogen_sulfide_burns` | combustion | adept | fire | 1 | fire |  |
| `carbon_disulfide_burns` | combustion | adept | fire | 1 | fire |  |
| `silane_burns` | combustion | adept | fire | 1 | fire |  |
| `haber` | industrial | apprentice | foam | 0.68 | Bench only | It makes ammonia, a poisonous gas, so it stays on the Bench. |
| `ammonia_splits` | decomposition | adept | frost | 0.17 | frost |  |
| `hcl_synthesis` | synthesis | apprentice | foam | 0.78 | Bench only | It makes hydrochloric acid, a poisonous gas, so it stays on the Bench. |
| `hbr_synthesis` | synthesis | adept | foam | 0.7 | Bench only | It makes hydrogen bromide, a poisonous gas, so it stays on the Bench. |
| `hf_synthesis` | synthesis | adept | foam | 0.94 | Bench only | It makes hydrogen fluoride, a poisonous gas, so it stays on the Bench. |
| `contact_so3` | industrial | adept | glow | 0.79 | fire |  |
| `so3_water` | industrial | adept | glow | 0.78 | fire |  |
| `ostwald_1` | industrial | adept | foam | 1 | Bench only | It makes nitric oxide, a poisonous gas, so it stays on the Bench. |
| `no_oxidises` | oxidation | adept | foam | 0.71 | Bench only | It makes nitrogen dioxide, a poisonous gas, so it stays on the Bench. |
| `ostwald_3` | industrial | adept | foam | 0.74 | Bench only | It makes nitric oxide, a poisonous gas, so it stays on the Bench. |
| `lightning_nitrogen` | synthesis | apprentice | frost | 0.19 | frost |  |
| `steam_reforming` | industrial | adept | frost | 0.2 | frost |  |
| `water_gas_shift` | industrial | adept | foam | 0.56 | lift |  |
| `water_gas` | industrial | adept | frost | 0.18 | frost |  |
| `boudouard` | redox | adept | frost | 0.19 | frost |  |
| `methanol_synthesis` | industrial | adept | glow | 0.68 | fire |  |
| `urea_synthesis` | industrial | adept | glow | 0.67 | fire |  |
| `limestone_roast` | thermal | sprout | frost | 0.19 | frost |  |
| `slaking_lime` | synthesis | sprout | fire | 0.63 | fire |  |
| `lime_dries` | thermal | adept | frost | 0.18 | frost |  |
| `copper_carbonate_heat` | thermal | apprentice | frost | 0.14 | frost |  |
| `magnesium_carbonate_heat` | thermal | apprentice | frost | 0.17 | frost |  |
| `baking_soda_heat` | thermal | seedling | frost | 0.18 | frost |  |
| `potassium_chlorate_heat` | thermal | adept | foam | 0.65 | push |  |
| `mercury_oxide_heat` | thermal | adept | frost | 0.21 | frost |  |
| `silver_oxide_heat` | thermal | adept | frost | 0.16 | frost |  |
| `copper_nitrate_heat` | thermal | adept | frost | 0.23 | frost |  |
| `ammonium_chloride_heat` | thermal | apprentice | frost | 0.19 | frost |  |
| `lab_ammonia` | gas_prep | adept | frost | 0.21 | frost |  |
| `salt_electrolysis` | electrolysis | apprentice | frost | 0.25 | frost |  |
| `brine_electrolysis` | electrolysis | adept | frost | 0.23 | frost |  |
| `alumina_electrolysis` | electrolysis | adept | frost | 0.25 | frost |  |
| `copper_chloride_electrolysis` | electrolysis | apprentice | frost | 0.21 | frost |  |
| `hcl_electrolysis` | electrolysis | adept | frost | 0.22 | frost |  |
| `neutralise_hcl_naoh` | neutralisation | seedling | glow | 0.6 | water |  |
| `antacid` | neutralisation | seedling | glow | 0.71 | water |  |
| `neutralise_hcl_koh` | neutralisation | apprentice | glow | 0.6 | water |  |
| `neutralise_hcl_lioh` | neutralisation | apprentice | glow | 0.6 | water |  |
| `neutralise_h2so4_naoh` | neutralisation | apprentice | glow | 0.71 | water |  |
| `neutralise_h2so4_koh` | neutralisation | apprentice | glow | 0.71 | water |  |
| `neutralise_hno3_naoh` | neutralisation | apprentice | glow | 0.6 | water |  |
| `neutralise_hno3_koh` | neutralisation | apprentice | glow | 0.6 | water |  |
| `neutralise_ammonia_hcl` | neutralisation | apprentice | glow | 0.77 | water |  |
| `neutralise_caoh2_hcl` | neutralisation | apprentice | glow | 0.71 | water |  |
| `neutralise_vinegar_naoh` | neutralisation | apprentice | glow | 0.6 | water |  |
| `neutralise_ammonia_h2so4` | neutralisation | adept | glow | 0.7 | water |  |
| `neutralise_ammonia_hno3` | neutralisation | adept | glow | 0.59 | water |  |
| `neutralise_baoh2_h2so4` | precipitation | adept | wall | 0.74 | wall |  |
| `co2_naoh` | neutralisation | apprentice | glow | 0.7 | water |  |
| `cuo_acid` | neutralisation | apprentice | fire | 0.62 | fire |  |
| `mgo_acid` | neutralisation | apprentice | glow | 0.75 | water |  |
| `zinc_acid` | metal_acid | seedling | foam | 0.75 | lift |  |
| `magnesium_acid` | metal_acid | seedling | foam | 0.92 | lift |  |
| `iron_acid` | metal_acid | apprentice | foam | 0.67 | lift |  |
| `aluminium_acid` | metal_acid | apprentice | foam | 1 | lift |  |
| `calcium_acid` | metal_acid | apprentice | foam | 0.94 | lift |  |
| `zinc_sulfuric` | metal_acid | apprentice | foam | 0.75 | lift |  |
| `magnesium_sulfuric` | metal_acid | apprentice | foam | 0.92 | lift |  |
| `iron_sulfuric` | metal_acid | apprentice | foam | 0.67 | lift |  |
| `copper_nitric` | redox | adept | foam | 0.88 | Bench only | It makes nitric oxide, a poisonous gas, so it stays on the Bench. |
| `chalk_acid` | carbonate_acid | sprout | foam | 0.41 | foam |  |
| `washing_soda_acid` | carbonate_acid | seedling | foam | 0.17 | foam |  |
| `baking_soda_hcl` | carbonate_acid | apprentice | frost | 0.52 | frost |  |
| `copper_carbonate_acid` | carbonate_acid | apprentice | foam | 0.45 | foam |  |
| `marble_sulfuric` | carbonate_acid | adept | foam | 0.21 | foam |  |
| `magnesium_carbonate_acid` | carbonate_acid | apprentice | foam | 0.59 | foam |  |
| `iron_copper_sulfate` | displacement | sprout | reveal | 0.75 | reveal |  |
| `zinc_copper_sulfate` | displacement | seedling | reveal | 0.81 | reveal |  |
| `magnesium_copper_sulfate` | displacement | apprentice | reveal | 0.94 | reveal |  |
| `aluminium_copper_sulfate` | displacement | adept | reveal | 1 | reveal |  |
| `magnesium_zinc_sulfate` | displacement | apprentice | reveal | 0.86 | reveal |  |
| `zinc_lead_nitrate` | displacement | adept | reveal | 0.75 | reveal |  |
| `copper_silver_nitrate` | displacement | seedling | reveal | 0.75 | reveal |  |
| `zinc_silver_nitrate` | displacement | apprentice | reveal | 0.88 | reveal |  |
| `thermite` | displacement | apprentice | fire | 1 | fire |  |
| `blast_furnace` | redox | apprentice | foam | 0.49 | foam |  |
| `copper_oxide_carbon` | redox | apprentice | foam | 0.66 | foam |  |
| `copper_oxide_hydrogen` | redox | adept | fire | 0.67 | fire |  |
| `zinc_oxide_carbon` | redox | adept | frost | 0.22 | frost |  |
| `magnesium_co2` | redox | adept | fire | 1 | fire |  |
| `peroxide_iodide` | redox | adept | glow | 0.82 | fire |  |
| `chlorine_bromide` | halogen | apprentice | reveal | 0.64 | reveal |  |
| `chlorine_iodide` | halogen | apprentice | reveal | 0.78 | reveal |  |
| `bromine_iodide` | halogen | apprentice | reveal | 0.7 | reveal |  |
| `bleach_making` | halogen | adept | reveal | 0.69 | reveal |  |
| `iron_chlorine` | synthesis | apprentice | glow | 1 | fire |  |
| `aluminium_iodine` | synthesis | adept | glow | 0.96 | fire |  |
| `zinc_iodine` | synthesis | adept | glow | 0.85 | fire |  |
| `magnesium_nitrogen` | synthesis | adept | fire | 0.92 | fire |  |
| `lithium_nitrogen` | synthesis | adept | glow | 0.87 | fire |  |
| `copper_sulfur` | synthesis | apprentice | fire | 0.66 | fire |  |
| `zinc_sulfur` | synthesis | apprentice | fire | 0.8 | fire |  |
| `hydrogen_sulfur` | synthesis | adept | foam | 0.46 | Bench only | It makes hydrogen sulfide, a poisonous gas, so it stays on the Bench. |
| `sodium_hydride` | synthesis | adept | glow | 0.71 | fire |  |
| `silver_tarnish` | oxidation | apprentice | glow | 0.96 | fire |  |
| `silver_chloride` | precipitation | sprout | wall | 0.63 | wall |  |
| `silver_bromide` | precipitation | apprentice | wall | 0.67 | wall |  |
| `silver_iodide` | precipitation | apprentice | wall | 0.71 | wall |  |
| `golden_rain` | precipitation | apprentice | wall | 0.62 | wall |  |
| `barium_sulfate` | precipitation | apprentice | wall | 0.49 | wall |  |
| `copper_hydroxide` | precipitation | apprentice | wall | 0.6 | wall |  |
| `iron2_hydroxide` | precipitation | adept | wall | 0.45 | wall |  |
| `iron3_hydroxide` | precipitation | adept | wall | 0.67 | wall |  |
| `calcium_carbonate_ppt` | precipitation | apprentice | wall | 0.39 | wall |  |
| `magnesium_hydroxide_ppt` | precipitation | adept | wall | 0.18 | wall |  |
| `lead_chloride` | precipitation | adept | wall | 0.48 | wall |  |
| `zinc_hydroxide` | precipitation | adept | wall | 0.5 | wall |  |
| `co2_water` | synthesis | seedling | water | 0.46 | water |  |
| `so2_water` | synthesis | adept | glow | 0.49 | fire |  |
| `ammonia_water` | synthesis | adept | glow | 0.53 | fire |  |
| `sodium_oxide_water` | synthesis | adept | glow | 0.82 | fire |  |
| `p4o10_water` | synthesis | adept | glow | 0.92 | fire |  |
| `ethene_hydrogen` | organic | adept | foam | 0.74 | push |  |
| `ethene_bromine` | organic | adept | glow | 0.73 | fire |  |
| `ethene_water` | organic | adept | fire | 0.57 | fire |  |
| `esterification` | organic | adept | water | 0.21 | water |  |
| `methane_chlorine` | organic | adept | foam | 0.69 | Bench only | It makes hydrochloric acid, a poisonous gas, so it stays on the Bench. |
| `ethanol_dehydration` | organic | adept | frost | 0.17 | frost |  |
| `cracking` | organic | adept | frost | 0.19 | frost |  |
| `sucrose_hydrolysis` | life | adept | water | 0.39 | water |  |
| `ozone_forms` | synthesis | adept | frost | 0.21 | frost |  |
| `ozone_splits` | decomposition | adept | foam | 0.85 | push |  |
| `nitrogen_dioxide_dimer` | synthesis | adept | foam | 0.6 | Bench only | It makes dinitrogen tetroxide, a poisonous gas, so it stays on the Bench. |
| `glass_making` | industrial | adept | frost | 0.17 | frost |  |
| `cement_clinker` | industrial | adept | frost | 0.17 | frost |  |
| `nitrogen_triiodide` | decomposition | adept | foam | 0.7 | foam |  |
