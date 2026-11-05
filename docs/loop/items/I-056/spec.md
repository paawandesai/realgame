# I-056: the "by rule" tier and a world use for every spell (B2)

Phase B2 of the plan "Formula: free-play Bench to spells" (artifact ARwyByPYQG5Lp6e32JGc1a). Branch `loop/bench-b2`, from `loop/bench-play`. The brief called this item I-055; that id was already taken, so it is **I-056**.

## 1. What changes for the player

- The Bench knows about 304 more reactions, **predicted by rule**: acid + base, metal + acid, carbonate + acid, fuel burns, two salts swap (precipitation) and metal swap (displacement). A rule entry says "Predicted by rule (acid + base): An acid and a base cancel each other out, ..." in the codex and on the result card. Checked (core) reactions are unchanged and always win.
- Every saveable spell does something in the world, and the Save button says what ("Save as a spell (a wall spell): it builds a short-lived wall or plugs a gap"). A reaction with nothing safe to do out there is **Bench only**: the Save button is disabled and says why ("Bench only: It makes nitrogen dioxide, a poisonous gas, so it stays on the Bench."). The pack says "In the world: it ..." for each saved spell.
- New world uses: **Wall** (a solid forms: a short-lived wall of blocks), **Reveal** (a colour appears: the beacon stands on a hidden cache nearby for a while), **Lift** (hydrogen: the H2 lift) and **Push** (another gas: the CO2 dash's launch, away from the burst).

## 2. The rule tier (`content/chem/rules.json`)

Generated, never hand-edited:

```
python -I tools/chemdb/build_rules.py content/chem       # rules.json, names.json additions
python -I tools/chemdb/build_show_data.py content/chem   # substances.json, geometry.json (show records)
```

`build_rules.py` holds the rule data with sources: the solubility rules (OpenStax Chemistry 2e Table 4.1, `OPENSTAX-SOL`), the reactivity series from standard reduction potentials (CRC Handbook, `CRC-ECS`; a metal displaces another only when E° differs by at least `margin_v` = 0.25 V, and metals that react with water first, `water_first`, are left out of displacement), the acids (HNO3 is oxidising, so it gives no metal + acid entry), combustion (`OPENSTAX-COMB`) and GHS hazards. It writes the rule text, every entry's name, say and short text, the ion and salt tables the Rust checks re-derive from, and the bottles that make each band's entries makeable.

Each entry: `id` `rule_<rule>_<reactants>`, `tier: "rule"`, `rule`, `family`, `band`, `heat` (from ΔH: little when |ΔH| ≤ 25 kJ), `dh_kj` (from CRC/NBS ΔHf via the show data), `per` and `dh_per_kj`, `needs`, `hazard` and `hazard_why`. A show fix per entry (`show`) gives fuels their flame and neutralisations a note and speed. Every new species has states, a sourced ΔHf, a sourced colour where it has one, and geometry in `substances.json` and `geometry.json`.

**Bands.** Seedling: a simple acid + base, metal + acid or carbonate + acid (chalk, washing soda or baking soda in vinegar) whose species are all Seedling-familiar. Adept: anything with a bromide or lead (the spec; lead nitrate is a category 1A reproductive toxicant), or a bottle that is not on the core shelf and not listed in `APPRENTICE_NEW` (a core product is not a familiar bottle). Apprentice: the rest.

**Left out:** fluorides and HF, cyanides, perchlorates, chromates, mercury salts, and explosive products; lead only for core's lead salts with group-1 or ammonium partners; no silver hydroxide, no iron(III) or aluminium carbonate, no ammonium + hydroxide. After the science review: no aluminium except with chloride or bromide (`passivated`: its oxide skin stops it in sulfate and nitrate solutions and in cold dilute sulfuric acid); no carbonate or ammonia with an acid whose last proton they cannot take (`base_pka` against each acid's `anion_pka`: carbonate + phosphoric acid and ammonia + phosphoric acid stop at the hydrogenphosphate); no copper or zinc carbonate precipitates (`basic_carbonate`: basic carbonates and CO2 come out); no ammonium carbonate with silver, copper or zinc salts (`ammine` complexes; ammoniacal silver can deposit explosive silver nitride); no aluminium bromide or iodide solutions (not real lab reagents); no sugar (it chars). A candidate whose reactants (reduced) match a core reaction is dropped (never shadows or duplicates core), as is one missing a sourced ΔHf.

**Heat (review S1).** `heat` and the spell strength use ΔH per mole of the rule's key species (`per`: CO2 for carbonate + acid, H2O (or NH3) for acid + base, H2 for metal + acid, the solid for a precipitate, the displaced metal, the fuel), so they do not depend on how the equation is scaled; `dh_kj` stays per equation and `dh_per_kj` is per mole of `per`. A carbonate powder is a solid in the acid (show fix `states`), so every hydrogencarbonate in acid takes in heat, as core baking soda does. The show record carries `dh.per` and `dh.kj_per`.

**Fuels (review S6).** What lights a fuel follows its flash point (`fuels`, PubChem/NIOSH): a spark at or below 25 °C (heptane, cyclohexane, pentene and the gases), otherwise heat or a wick (nonane, decane, undecane, dodecane in kerosene, hexadecane in diesel, butanol, pentanol).

**Hazards (review S8): one policy for both tiers.** Bench bottles are dilute (acids up to 1 M, alkalis up to 0.4 M: low hazard in CLEAPSS Hazcards). The lab badge goes to any reaction, core or rule, with lead, soluble barium (not BaSO4), mercury, silver nitrate or quicklime (`lab_policy`), or that makes a poisonous gas or vapour (the Bench-only list). Burnings have the fire badge; nothing else in the rule tier has a badge. Core gained the lab badge on 14 entries to match (sulfur_burns, carbon_disulfide_burns, lightning_nitrogen, limestone_roast, slaking_lime, lime_dries, ammonium_chloride_heat, lab_ammonia, neutralise_baoh2_h2so4, copper_nitric, zinc_silver_nitrate, silver_bromide, silver_iodide, ozone_forms). golden_rain stays Apprentice (with the lab badge): the Apprentice Guide lesson ap_evidence teaches it.

**Counts (304):**

| rule | seedling | apprentice | adept | total |
|---|---|---|---|---|
| acid_base | 4 | 11 | 47 | 62 |
| metal_acid | 1 | 0 | 4 | 5 |
| carbonate_acid | 2 | 21 | 15 | 38 |
| fuel_burns | 0 | 7 | 6 | 13 |
| precipitate | 0 | 19 | 139 | 158 |
| displacement | 0 | 4 | 24 | 28 |
| **total** | 7 | 62 | 235 | 304 |

**Engine.** `Codex::load()` merges core and rules (`CodexTier::Checked` and `Rule`); `run()` prefers a checked reaction, then the band, then the tier. `Codex::load_core()` is core alone. `crates/chem/src/rules.rs` (`RuleData`, `check`) re-derives each entry from the tables: a precipitate is the one insoluble pair of a swap, a displacement follows the series with the margin, a metal + acid has a metal above hydrogen and a non-oxidising acid, a carbonate + acid gives salt, water and CO2, a fuel burns to CO2 and water only; and the review's rules: aluminium's oxide skin, the pKa limits, basic carbonates, ammine complexes, and a fuel's needs from its flash point.

## 3. Tests (chem crate)

`rules.rs`: the tier loads with its rules; every rule reaction balances and is charge neutral; every one passes its rule's check; the checks reject wrong chemistry (including the review's cases: Al with copper sulfate, zinc nitrate or sulfuric acid; carbonate or hydrogencarbonate making Na3PO4; (NH4)3PO4; CuCO3; silver with ammonium carbonate; a spark for dodecane); every one is makeable from its band's bottles (core shelf plus rules.json bottles); none duplicates or shadows core; `the_lab_badge_follows_one_policy_in_both_tiers`; `aluminium_needs_its_oxide_skin_broken` (M3, including core aluminium_copper_sulfate: slow, with the oxide-skin note); `the_heat_class_does_not_depend_on_how_the_equation_is_scaled` (S1); every one has a complete show record (something visible or a note, ΔH present and agreeing with the heat class, a precipitate's colour and grain, bubbles for a gas, a flame for a fuel, a coating for a displacement).

## 4. World uses

| what the reaction does | kind | runs | flies as | in the world |
|---|---|---|---|---|
| burns, or a gas that catches fire | fire / glow | Fire | Fire | burns, melts ice, lights |
| gives off heat in a solution, no colour | glow / water | Water | Water | warm or plain splash |
| takes in heat by itself (no heat, light or electricity driving it) | frost | Frost | Frost | freezes water, cools |
| gives off only CO2 or N2 | foam | Foam (CO2) | CO2 | smothers fire |
| a solid forms (precipitation) | wall | **Wall** | Frost | a short-lived wall or plug |
| a colour change or a metal coating, in a solution | reveal | **Reveal** | Fire | hidden caches show for a while |
| gives off hydrogen | foam | **Lift** | Water | lifts a player near the landing |
| gives off another harmless gas (O2, steam, ...) | foam | **Push** | CO2 | pushes a player near the landing away |
| makes a poisonous gas or vapour (any kind) | any | **Bench only** | - | cannot be saved; says why |

Ordered rules in `content/chem/effects.json`: `kind_rules` (frost only with `needs_any: false`: a reaction driven by heat, light or electricity only soaks up what drives it, review M5; a `reveal` rule sits after the fire-hazard rule, for the solution families with a `coating` or `colour_change` in the show record), `runs`, then `gas_uses` for a fizz that does not smother: a flame → Fire; H2 → Lift; anything else → Push. Last, `poisons` (review M1) is checked on every product of every reaction, whatever its kind: NO, NO2, N2O4, NH3, HCl, HBr, HF, Cl2, Br2, F2, H2S, SO2, CO, CH3Cl, Hg and O3 make it Bench only, "It makes {gas}, which is poisonous to breathe, so it stays on the Bench.", with `{gas}` the gas or vapour's own name (`poisons.names`: hydrogen chloride, not hydrochloric acid; mercury vapour; review M2). `Effect` gains `use` (`Runs::world`, plain words) and `bench_only`; `Effect::saveable()` is false only for Bench only.

**Counts:** core 182: fire 65, water 19, frost 2, foam 16, wall 14, reveal 11, lift 16, push 9, none 30. Rule 304: fire 13, water 45, frost 8, foam 30, wall 158, reveal 45, lift 5. All 486: fire 78, water 64, frost 10, foam 46, wall 172, reveal 56, lift 21, push 9, none 30 (none = Bench only).

**Bench only (30, all core):** sulfur_burns (sulfur dioxide); carbon_little_air (carbon monoxide); hydrogen_sulfide_burns (sulfur dioxide); carbon_disulfide_burns (sulfur dioxide); haber (ammonia); hcl_synthesis (hydrogen chloride); hbr_synthesis (hydrogen bromide); hf_synthesis (hydrogen fluoride); ostwald_1 (nitric oxide); no_oxidises (nitrogen dioxide); ostwald_3 (nitric oxide); lightning_nitrogen (nitric oxide); steam_reforming (carbon monoxide); water_gas (carbon monoxide); boudouard (carbon monoxide); mercury_oxide_heat (mercury vapour); copper_nitrate_heat (nitrogen dioxide); ammonium_chloride_heat (ammonia); lab_ammonia (ammonia); salt_electrolysis (chlorine); brine_electrolysis (chlorine); copper_chloride_electrolysis (chlorine); hcl_electrolysis (chlorine); copper_nitric (nitric oxide); zinc_oxide_carbon (carbon monoxide); chlorine_bromide (bromine); hydrogen_sulfur (hydrogen sulfide); methane_chlorine (chloromethane); ozone_forms (ozone); nitrogen_dioxide_dimer (dinitrogen tetroxide).

Core reactions that were Frost but are driven by heat, light or electricity now take their gas use (Foam when the only gas is CO2, Lift for hydrogen, Push for another gas, Bench only for a poisonous one) or splash (Water). Only baking_soda_vinegar and baking_soda_hcl stay Frost in core. The 14 core precipitations run Wall; 11 core solution reactions with a colour change or coating run Reveal (chlorine_bromide is Bench only: bromine).

Rule tier by use: acid_base → reveal 17, acid_base → water 45, carbonate_acid → foam 30, carbonate_acid → frost 8, displacement → reveal 28, fuel_burns → fire 13, metal_acid → lift 5, precipitate → wall 158.

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
- chem `discover.rs` (review regressions): `every_poisonous_product_keeps_a_reaction_on_the_bench` (M1), `bench_only_reasons_name_the_gas_not_the_acid` (M2), `frost_only_from_reactions_that_feel_cold_by_themselves` (M5).
- chem `discover.rs`: `every_saveable_spell_has_a_world_use_and_bench_only_ones_say_why` (a saveable spell has a use in plain words and no reason; a Bench-only one has a reason naming its gas; the use agrees with the show record: Wall has a precipitate, Reveal a coating or colour change, Lift hydrogen, Push a gas; every `Runs` value is used; every rule precipitate walls; the data values are in range); `effects_follow_the_chemistry` updated (silver chloride Wall, zinc + acid Lift, iron + copper sulfate Reveal, peroxide Push, NO oxidising Bench only, potassium + water Fire).
- bevy-client `world_use.rs`: a wall stands across the throw and crumbles (leaving a block the player changed); a wall drops to the ground, plugs water and spares stamps; a reveal shows only unfound caches within its radius and ends; hydrogen lifts and a gas burst pushes a player nearby, and not one far away; the world uses come from the codex. `discover.rs`: a Bench-only reaction is never saved or equipped; a hydrogen fizz carries its strength and is not a matter landing.
- `test/bench-spells.test.ts`: the kinds list includes wall and reveal.

## 9. Science review (outcomes)

An independent review of B2 (5 must-fix, 12 should-fix, 9 nits) is applied: M1 poisons on every product, with Hg, O3 and F2; M2 gas names; M3 aluminium only with chloride or bromide (core aluminium_copper_sulfate is slow, with an oxide-skin note); M4 no carbonate + phosphoric acid; M5 Frost only without needs. S1 per-mole heat and solid powders; S2 no CuCO3/ZnCO3 precipitates; S3 no ammonium carbonate with Ag, Cu, Zn; S4 no (NH4)3PO4; S5 no Al + H2SO4; S6 flash-point needs, no sugar; S7 CuO and Fe2O3 need warm acid (slow), quicklime "gets hot"; S8 one hazard policy with a test over both tiers; S9 bands (carbonate + vinegar at Seedling, bromides and lead Adept in the rule tier (core golden_rain reviewed and kept at Apprentice for its Guide lesson, with the lab badge), core products not familiar bottles); S10 no AlBr3/AlI3 solutions; S11 chemical names first ("magnesium hydroxide (as in milk of magnesia)", "calcium carbonate (chalk)"); S12 Al(OH)3 and Zn(OH)2 note that more alkali dissolves them. Nits: N1 iron(II) with silver note; N2 Mg in copper or zinc salts fizzes a little (note); N3 "copper bromide", not "copper(II)"; N4 rule texts; N5 "neutralises"; N6 "dodecane (in kerosene)", "hexadecane (in diesel)"; N8 via S1 (`dh_per_kj`). Not done: N7 (a gelatinous grain; a look change) and N9 (core silver_chloride stays Sprout, now with the lab badge, outside B2).

## 10. Files

New: `tools/chemdb/build_rules.py`, `content/chem/rules.json`, `crates/chem/src/rules.rs`, `crates/bevy-client/src/world_use.rs`, this spec. Changed: `tools/chemdb/build_show_data.py`, `content/chem/{substances,geometry,names,effects}.json`, `crates/chem/src/{lib,show,discover,web}.rs`, `crates/bevy-client/src/{discover,blockchem,firebreak,late,beacon,powers,main}.rs`, `crates/bevy-client/web/{bench-free.js,bench-free.css,pack.js}`, `content/science-sources.md`, `test/bench-spells.test.ts`, `docs/loop/items/I-053/spec.md`.

## 11. The current mapping table (the 182 core reactions)

Generated from the Rust (`Effects::of`). Rule-tier entries follow section 4 by rule; their full list is `content/chem/rules.json`.

| reaction | family | band | kind | strength | runs | Bench-only reason |
|---|---|---|---|---|---|---|
| `methane_burns` | combustion | sprout | fire | 1 | fire |  |
| `hydrogen_burns` | combustion | sprout | fire | 0.92 | fire |  |
| `water_splits` | electrolysis | seedling | foam | 0.24 | lift |  |
| `carbon_burns` | combustion | sprout | fire | 0.89 | fire |  |
| `iron_rusts` | oxidation | sprout | glow | 1 | fire |  |
| `baking_soda_vinegar` | carbonate_acid | sprout | frost | 0.52 | frost |  |
| `photosynthesis` | life | sprout | foam | 0.25 | push |  |
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
| `sulfur_burns` | combustion | sprout | fire | 0.85 | Bench only | It makes sulfur dioxide, which is poisonous to breathe, so it stays on the Bench. |
| `phosphorus_burns` | combustion | apprentice | fire | 1 | fire |  |
| `co_burns` | combustion | apprentice | fire | 0.95 | fire |  |
| `carbon_little_air` | combustion | apprentice | fire | 0.81 | Bench only | It makes carbon monoxide, which is poisonous to breathe, so it stays on the Bench. |
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
| `hydrogen_sulfide_burns` | combustion | adept | fire | 1 | Bench only | It makes sulfur dioxide, which is poisonous to breathe, so it stays on the Bench. |
| `carbon_disulfide_burns` | combustion | adept | fire | 1 | Bench only | It makes sulfur dioxide, which is poisonous to breathe, so it stays on the Bench. |
| `silane_burns` | combustion | adept | fire | 1 | fire |  |
| `haber` | industrial | apprentice | foam | 0.68 | Bench only | It makes ammonia, which is poisonous to breathe, so it stays on the Bench. |
| `ammonia_splits` | decomposition | adept | foam | 0.17 | lift |  |
| `hcl_synthesis` | synthesis | apprentice | foam | 0.78 | Bench only | It makes hydrogen chloride, which is poisonous to breathe, so it stays on the Bench. |
| `hbr_synthesis` | synthesis | adept | foam | 0.7 | Bench only | It makes hydrogen bromide, which is poisonous to breathe, so it stays on the Bench. |
| `hf_synthesis` | synthesis | adept | foam | 0.94 | Bench only | It makes hydrogen fluoride, which is poisonous to breathe, so it stays on the Bench. |
| `contact_so3` | industrial | adept | glow | 0.79 | fire |  |
| `so3_water` | industrial | adept | glow | 0.78 | fire |  |
| `ostwald_1` | industrial | adept | foam | 1 | Bench only | It makes nitric oxide, which is poisonous to breathe, so it stays on the Bench. |
| `no_oxidises` | oxidation | adept | foam | 0.71 | Bench only | It makes nitrogen dioxide, which is poisonous to breathe, so it stays on the Bench. |
| `ostwald_3` | industrial | adept | foam | 0.74 | Bench only | It makes nitric oxide, which is poisonous to breathe, so it stays on the Bench. |
| `lightning_nitrogen` | synthesis | apprentice | foam | 0.19 | Bench only | It makes nitric oxide, which is poisonous to breathe, so it stays on the Bench. |
| `steam_reforming` | industrial | adept | foam | 0.2 | Bench only | It makes carbon monoxide, which is poisonous to breathe, so it stays on the Bench. |
| `water_gas_shift` | industrial | adept | foam | 0.56 | lift |  |
| `water_gas` | industrial | adept | foam | 0.18 | Bench only | It makes carbon monoxide, which is poisonous to breathe, so it stays on the Bench. |
| `boudouard` | redox | adept | foam | 0.19 | Bench only | It makes carbon monoxide, which is poisonous to breathe, so it stays on the Bench. |
| `methanol_synthesis` | industrial | adept | glow | 0.68 | fire |  |
| `urea_synthesis` | industrial | adept | glow | 0.67 | fire |  |
| `limestone_roast` | thermal | sprout | foam | 0.19 | foam |  |
| `slaking_lime` | synthesis | sprout | glow | 0.63 | fire |  |
| `lime_dries` | thermal | adept | water | 0.18 | water |  |
| `copper_carbonate_heat` | thermal | apprentice | foam | 0.14 | foam |  |
| `magnesium_carbonate_heat` | thermal | apprentice | foam | 0.17 | foam |  |
| `baking_soda_heat` | thermal | seedling | foam | 0.18 | foam |  |
| `potassium_chlorate_heat` | thermal | adept | foam | 0.65 | push |  |
| `mercury_oxide_heat` | thermal | adept | foam | 0.21 | Bench only | It makes mercury vapour, which is poisonous to breathe, so it stays on the Bench. |
| `silver_oxide_heat` | thermal | adept | foam | 0.16 | push |  |
| `copper_nitrate_heat` | thermal | adept | foam | 0.23 | Bench only | It makes nitrogen dioxide, which is poisonous to breathe, so it stays on the Bench. |
| `ammonium_chloride_heat` | thermal | apprentice | foam | 0.19 | Bench only | It makes ammonia, which is poisonous to breathe, so it stays on the Bench. |
| `lab_ammonia` | gas_prep | adept | foam | 0.21 | Bench only | It makes ammonia, which is poisonous to breathe, so it stays on the Bench. |
| `salt_electrolysis` | electrolysis | apprentice | foam | 0.25 | Bench only | It makes chlorine, which is poisonous to breathe, so it stays on the Bench. |
| `brine_electrolysis` | electrolysis | adept | foam | 0.23 | Bench only | It makes chlorine, which is poisonous to breathe, so it stays on the Bench. |
| `alumina_electrolysis` | electrolysis | adept | foam | 0.25 | push |  |
| `copper_chloride_electrolysis` | electrolysis | apprentice | foam | 0.21 | Bench only | It makes chlorine, which is poisonous to breathe, so it stays on the Bench. |
| `hcl_electrolysis` | electrolysis | adept | foam | 0.22 | Bench only | It makes chlorine, which is poisonous to breathe, so it stays on the Bench. |
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
| `copper_nitric` | redox | adept | foam | 0.88 | Bench only | It makes nitric oxide, which is poisonous to breathe, so it stays on the Bench. |
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
| `zinc_oxide_carbon` | redox | adept | foam | 0.22 | Bench only | It makes carbon monoxide, which is poisonous to breathe, so it stays on the Bench. |
| `magnesium_co2` | redox | adept | fire | 1 | fire |  |
| `peroxide_iodide` | redox | adept | glow | 0.82 | fire |  |
| `chlorine_bromide` | halogen | apprentice | reveal | 0.64 | Bench only | It makes bromine, which is poisonous to breathe, so it stays on the Bench. |
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
| `hydrogen_sulfur` | synthesis | adept | foam | 0.46 | Bench only | It makes hydrogen sulfide, which is poisonous to breathe, so it stays on the Bench. |
| `sodium_hydride` | synthesis | adept | glow | 0.71 | fire |  |
| `silver_tarnish` | oxidation | apprentice | glow | 0.96 | fire |  |
| `silver_chloride` | precipitation | sprout | wall | 0.63 | wall |  |
| `silver_bromide` | precipitation | apprentice | wall | 0.67 | wall |  |
| `silver_iodide` | precipitation | apprentice | wall | 0.71 | wall |  |
| `golden_rain` | precipitation | adept | wall | 0.62 | wall |  |
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
| `methane_chlorine` | organic | adept | foam | 0.69 | Bench only | It makes chloromethane, which is poisonous to breathe, so it stays on the Bench. |
| `ethanol_dehydration` | organic | adept | foam | 0.17 | push |  |
| `cracking` | organic | adept | foam | 0.19 | push |  |
| `sucrose_hydrolysis` | life | adept | water | 0.39 | water |  |
| `ozone_forms` | synthesis | adept | foam | 0.21 | Bench only | It makes ozone, which is poisonous to breathe, so it stays on the Bench. |
| `ozone_splits` | decomposition | adept | foam | 0.85 | push |  |
| `nitrogen_dioxide_dimer` | synthesis | adept | foam | 0.6 | Bench only | It makes dinitrogen tetroxide, which is poisonous to breathe, so it stays on the Bench. |
| `glass_making` | industrial | adept | foam | 0.17 | foam |  |
| `cement_clinker` | industrial | adept | foam | 0.17 | foam |  |
| `nitrogen_triiodide` | decomposition | adept | foam | 0.7 | foam |  |
