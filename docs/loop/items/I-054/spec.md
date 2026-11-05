# I-054 (Bench B1): reaction show records

**Plan:** "Formula: free-play Bench to spells plan", phase B1 (https://claude.ai/artifact/ARwyByPYQG5Lp6e32JGc1a), section "See: what each reaction looks like". **Branch:** `loop/bench-show`, cut from `loop/bench-spells` (I-053) at 4f37412.

**Note on the item number.** `docs/loop/items/I-054/` already holds `notes.md` and atlas images of the generated block atlas (wave-15-mac, branch `loop/I-054`). This spec was asked for at this path; the two items share the folder until the coordinator renumbers one of them.

**What it is.** Every one of the 182 codex reactions gets a *show record*: what a real lab bench shows, as measured values the renderer computes from, never art settings. The chemistry engine (`crates/chem`) builds the records; the Bench page uses them for the result card and passes them to the 3D scene; the spell effects (I-053) take their tint and strength from them. `bench-scene.js` (graphics thread) is not changed: it gets the record as an optional 4th argument and ignores it today.

**Honesty rules.** A value is either sourced (its source key is in the record or the data file) or flagged (`estimated`, `hex_estimated`, `verified: false`, `illustrative`). Show data is `check: draft` until the science review of the show colours (plan, "Science review").

## 1. Files

| file | what | made by |
|---|---|---|
| `content/chem/substances.json` | every codex species: ΔfH° per phase, state at 298 K, melting and boiling points, density, Shomate heat capacities, colours per phase, precipitate grain, sources | `tools/chemdb/build_show_data.py` (generated; edit the script) |
| `content/chem/geometry.json` | every codex species: elements, positions (Å), bonds with orders, lattice flag, conformer id, source | the same script |
| `content/chem/show.json` | hand corrections: flames and their emitters, glows, media, state overrides, speeds, extra visible changes, notes, each with `why` | by hand |
| `crates/chem/src/show.rs` | the types, the generator (`Shows::build`), the atom story, `frame`, `strength_of`, the tests | |
| `crates/chem/src/discover.rs` | `Effects` now holds the show records; tint and strength come from them | |
| `crates/chem/src/web.rs` | `show_json(id)`, `show_frame_json(id, t)`, `geometry_json(formula)`, `show_sources_json()` | |
| `crates/bevy-client/web/bench-free.js` | the result card's "What you'd see", by band; `playReaction(inMols, outMols, fx, show)` | |

The data is compiled into the wasm with `include_str!` (substances 130 KB, geometry 51 KB, show 24 KB before compression), so `build-web.sh` needs no change: the page asks the engine with `show_json(id)`.

**Rebuilding the data.** `python -m venv <dir outside the repo> && <dir>/bin/pip install chemicals rdkit tblite ase && <dir>/bin/python -I tools/chemdb/build_show_data.py content/chem` (add `--regeometry` to recompute every conformer; without it the existing ones are kept, since the xTB optimisation takes minutes).

## 2. Sources

Machine-readable copies of published tables, read from the `chemicals` package (C. Bell, MIT licence) so every value was copied by a script, not typed:

| key | source | used for |
|---|---|---|
| `CRC-T` | CRC Handbook, "Standard Thermodynamic Properties of Chemical Substances" | ΔfH° of solids, liquids, gases (191 values) |
| `CRC-ION` | CRC Handbook, "Thermodynamic Properties of Aqueous Ions" | ΔfH°(aq) of dissolved strong electrolytes as the sum of their ions (44 values) |
| `CRC-PHYS` | CRC Handbook, "Physical Constants of Inorganic / Organic Compounds" | melting point, boiling point, density |
| `CRC-PC` | CRC Handbook heats of fusion and vaporization | melting on the way to a flame temperature |
| `NIST-SHOMATE` | NIST Chemistry WebBook Shomate coefficients | heat capacities for flame and glow temperatures |
| `API-TDB`, `GHAZ` | API Technical Data Book ΔfH°(g); Ghazerati ΔvapH(298 K) | pentacosane (candle wax), estimated |
| `RDKIT` | RDKit ETKDGv3 conformer, optimised with GFN2-xTB (tblite) | 54 molecules and 40 polyatomic-ion clusters |

Entered by hand, flagged: `NBS82` (Wagman et al. 1982, 21 values, `verified: false`: aqueous molecules such as NH3(aq), CO2(aq), CH3COOH(aq), Cl2(aq), Br2(aq), I2(aq), and Fe(OH)2, Fe(OH)3, Mg3N2, Li3N, P4O10, CaSiO3, Al2(SO4)3); `CRC-PRINTED` (three boiling points: KCl, CaO, ZnS; `verified: false`); three ΔfH° marked `estimated` with how (NI3, CuCO3, C25H52). Colours: `COLOUR-GE` (Greenwood and Earnshaw, cross-checked with each substance's Wikipedia infobox), `COLOUR-FLAME` (flame tests; NIST ASD lines), `COLOUR-HC` (Gaydon, flame spectroscopy), `MIST` (Bohren and Huffman). Speeds: `SPEED`, `DEMO`; air: `AIR`. Every key's full reference is in the files and in `show_sources_json()`.

## 3. The schema, field by field

A record is `show::ShowRecord`, serialised by `show_json(id)`.

| field | type, unit | meaning and rule |
|---|---|---|
| `reaction_id` | string | the codex id |
| `medium` | `water` \| `air` \| `dry` | where it happens. `water`: families neutralisation, displacement, metal_acid, carbonate_acid, precipitation, halogen, water_metal, plus the reactions listed in show.json `aqueous_reactions`; `air`: O2 is a reactant; else `dry` (a dry mix, a melt, pure gases). show.json can override. |
| `species[]` | | every reactant then every product, in equation order |
| `.formula`, `.coef`, `.side` | | `side` is `reactant` or `product` |
| `.state` | `s` \| `l` \| `g` \| `aq` | at the reaction's conditions. Start from the state at 298 K (CRC melting and boiling points). In water: a soluble species with an aq ΔfH° is `aq`, water stays `l`, a gas product stays `g` (it bubbles out), a gas reactant that dissolves is `aq`. A hot reaction (combustion, needs heat or a spark, a flame or glow) not in water: a liquid product boiling at or below 400 K is `g` (water from a flame is steam). show.json `states` override (molten salt for electrolysis, steam for the water-gas reactions, products under reflux). |
| `.molar_mass` | g/mol | from IUPAC standard atomic weights |
| `.colour` | `Colour` | the species' colour in that state, if the table has one (see below) |
| `.conformer` | string | the geometry id, `<formula>#<source>-1` |
| `.lattice` | bool | an ionic, metallic or network solid: the geometry is a representative unit |
| `.at_room` | `condenses` \| `freezes` \| `evaporates` \| `none` | products only: what it does back at 298.15 K. A gas whose boiling point is above room temperature condenses (or freezes, when its melting point is above too: iodine vapour deposits as crystals); a liquid whose melting point is above room temperature freezes (molten iron); a liquid boiling below 360 K evaporates (bromine, pentane); anything else `none` |
| `Colour.described` | string | the colour as the source describes it ("pale yellow-green"); "colourless..." means colourless at bench strength |
| `Colour.hex` | `#rrggbb` sRGB, optional | a rendering of the description, never a measurement; absent when unsure |
| `Colour.hex_estimated` | bool | always true when `hex` is present |
| `Colour.src` | source key | |
| `visible[]` | list of `flame`, `glow`, `bubbles`, `precipitate`, `colour_change`, `smoke`, `frost`, `shimmer`, `condensation` | rules below, plus show.json `add` |
| `note` | string, optional | what there is to see in words, especially when `visible` is empty (a neutralisation: two colourless solutions stay colourless and warm a little) |
| `dh.kj` | kJ per mole of reaction as written, signed | Hess's law over ΔfH° at 298.15 K, 1 bar, with each species in its record state; `null` when a species has no value |
| `dh.terms[]` | | `formula`, signed `coef`, the `state` whose ΔfH° was used, `dhf` (kJ/mol), `src`, `estimated`, `verified` |
| `dh.state_fallbacks[]` | | species whose ΔfH° at the record state is missing, so the standard state's was used (molten iron in thermite uses Fe(s)) |
| `dh.dh_estimated` | bool | `kj` missing or any term estimated |
| `dh.heat` | -1, 0, +1 | from `kj`: 0 when \|ΔH\| ≤ 25 kJ, else its sign (with no `kj`, the codex heat class) |
| `dh.agrees` | bool or null | the sign agrees with the codex heat class (`little`: \|ΔH\| ≤ 25 kJ); null when the codex gives no class |
| `dh.note` | string, optional | show.json `dh_note`: why a disagreement is right (none needed today, see section 7) |
| `speed` | `instant` \| `fast` \| `slow` | instant: under a second; fast: seconds to a minute; slow: minutes or longer. By family (show.json `speed_by_family`) with corrections |
| `sped_up` | bool | true for every slow reaction: the Bench shows it sped up and says so |
| `timescale` | string | how long it really takes ("days to weeks", "seconds") |
| `flame` | optional | `emitter` (key into show.json `emitters`), `described`, `hex`, `lines_nm` (emission lines, nm, NIST ASD), `blackbody` (soot), `src`, `note`, `temp` |
| `glow` | optional | incandescence: `blackbody: true`, `temp` (only when the reaction heats itself, ΔH ≤ −50 kJ; an externally heated glow has no temperature in the record) |
| `temp.k` | K | adiabatic temperature, rounded to 10 K: −ΔH warms the products (plus 3.76 N2 per O2 when it burns in air) from 298.15 K, heat capacities from NIST Shomate (3R per atom where missing, said in `basis`), melting with the CRC heat of fusion, capped at the lowest boiling point of a condensed product. No heat lost and no dissociation, so an **upper bound**: methane in air gives 2330 K against 2226 K for the equilibrium calculation. Above 4000 K with no boiling point to cap at, the record gives no temperature |
| `temp.estimated` | bool | always true |
| `temp.basis` | string | how it was computed |
| `gases[]` | | every species in state `g`: `formula`, `side`, `density_rel_air` (molar mass / 28.96, 3 decimals), `rises` (< 1), `opacity`, `colour`, `condenses_to` (steam: a white mist of droplets, `MIST`) |
| `opacity` | `clear` \| `tinted` \| `dense` | a class, not a number: clear = colourless gas; tinted = coloured gas (NO2, Cl2, Br2, I2); dense = smoke (solid particles) |
| `smoke[]` | | `formula` (absent for soot), `opacity: dense`, `colour`: solid products of a flame or gas-phase reaction, show.json `smoke` (black soot from acetylene and benzene) |
| `precipitates[]` | | in water, every product that stays solid: `formula`, `colour`, `grain` (`cloudy`: fine or gelatinous, settles slowly; `grainy`: crystals or metal deposit, settles fast) |
| `colour_changes[]` | | `formula`, `state`, `appears` (product) or not (reactant), `colour` |
| `atom_story.illustrative` | true | the order and timing picture which bonds change; not a mechanism |
| `atom_story.atoms[]` | | one per reactant atom: `el`, `from` [term, copy, atom] in the reactants, `to` [term, copy, atom] in the products (indices into the equation's terms, the coefficient copy and the geometry's atoms) |
| `atom_story.events[]` | | `kind` (`break` \| `form`), `a`, `b` (story atom indices), `order`, `start`, `duration` (fractions of the show, 0..1). Breaks are spread over 0.10 to 0.40, forms over 0.50 to 0.80, each 0.15 long, so bonds stretch before they break and new bonds pull in after |
| `check` | `draft` | until the science review |

**Visible change rules** (show.rs `build_record`), before show.json `add`:
flame: show.json names an emitter. glow: show.json `glow`. bubbles: a gas product in water. precipitate: in water, a product that stays solid. colour_change: a coloured dissolved, liquid or gaseous species comes or goes, or a solid turns into a solid of another colour (a precipitate that only appears is not a colour change). smoke: a flame with a solid product, or gases only that make a solid. shimmer: a flame or glow. condensation: a hot reaction whose gas product condenses at room temperature. frost: only by hand (no codex reaction makes frost on a bench: baking soda and vinegar cools by a few kelvin).

**Geometry** (`geometry.json`, `show::Geometry`): `id`, `elements[]`, `positions[]` (Å, centred on the centroid), `bonds[]` as [i, j, order] with order 1, 1.5 (aromatic), 2, 3 or **0 for an ionic contact**, `lattice`, `src`, `method`, `spacing_a` (Å: the cube root of the volume per formula unit, or per ion, from the CRC density), `note`. Molecules (54): RDKit ETKDGv3 then GFN2-xTB in the gas phase (H2O O–H 0.96 Å, CO2 C=O 1.14 Å, N2 1.10 Å against 0.958, 1.160, 1.098 measured). Ionic solids (79): a cluster of the formula unit's ions on a simple cubic grid, cations and anions alternating, spacing from the density; polyatomic ions are xTB conformers (40). Metals, carbon, silicon, sulfur, mercury (18): one atom per formula unit (`ATOM`), with a note (sulfur is S8 rings; silica is one SiO2 unit of the network).

## 4. What the renderer gets

- **Per reaction:** `show_json(id)` (the record above). The Bench page passes it as the 4th argument: `scene3d.playReaction(inMols, outMols, fx, show)`; today's scene ignores it.
- **Per frame:** `show_frame_json(id, t)`, `show::frame(record, geometry, effect, t)` → `Frame`:
  - `t` (0..1);
  - `atoms[]`: `el`, `pos` [x, y, z] Å. Reactant units sit in a square grid centred on the origin, products in another, and each atom travels from its reactant place to its product place between the end of its first break and the end of its last form (smoothstep); a breaking bond's atoms are pushed up to 0.4 Å apart during the break and relax over the next 0.1;
  - `bonds[]`: `a`, `b`, `order`, `stretch` (1 at rest; rises to 1.5 as a bond breaks, falls from 1.6 to 1 as one forms); a bond kept from reactants to products stays, its order switching at t = 0.5;
  - `events[]`: the break and form events under way, with `progress` 0..1;
  - `effect`: the spell effect (`reaction_id`, `kind`, `heat`, `strength`, `tint`) and `visible[]` as [change, intensity 0..1]: flame, glow and shimmer rise over t 0.45 to 0.6; the others over 0.55 to 0.9.
- **Geometry:** `geometry_json(formula)`; **sources:** `show_sources_json()`.
- **What to compute, not invent:** flame colour from the emitter (lines in nm, or a blackbody at `temp.k` for soot and glows); gas rise or sink from `density_rel_air`; tint and opacity class of gases; mist from `condenses_to`; precipitate colour and settling from `grain`; heat haze from `shimmer`; slow reactions labelled sped up.

## 5. The Bench result card (bench-free.js)

For a known reaction, below the equation:

- **Sprout and Seedling:** "What you'd see: …" in plain words from the record (a yellow-orange flame, white smoke, bubbles of carbon dioxide, a white solid (silver chloride) clouding the liquid, a colour change: the blue of copper sulfate going, …), the record's note, and "For real this takes days, so the Bench shows it sped up." for slow ones.
- **Apprentice** adds the equation with states, e.g. 2Na(s) + Cl₂(g) → 2NaCl(s), and what the state symbols mean.
- **Adept** adds ΔH (kJ per mole of reaction as written, "(estimated)" when flagged), the estimated upper-bound flame or glow temperature (to 100 K), and each product gas's density against air.

The register rows are C-116 to C-120 in `content/science-sources.md` (sources S43 to S49).

## 6. Spells (I-053 effects) from the show records

- **tint:** an effect that runs Fire takes its flame's emitter colour (sodium yellow-orange, lithium crimson, magnesium brilliant white, a gas burner's blue, soot yellow); otherwise the first product with a rendered colour (a precipitate, a coloured gas or a coloured solution, never water). The old tint tables in effects.json are gone (`tint_rule` says this). 77 reactions have a tint.
- **strength:** from |ΔH| on a documented gameplay scale (`show::strength_of`, `strength_scale` in effects.json): `ln(1 + |ΔH|/kJ) / ln(1 + 800)`, clamped to 0.1..1, rounded to 0.01. A burning (hundreds of kJ) is 1, a neutralisation (55.8 kJ) 0.6, chalk and acid (14.5 kJ) 0.41. A reaction that takes in heat only while it is given heat, light or electricity chills little, so it counts a quarter: limestone roasting (+179 kJ) 0.19 against baking soda and vinegar (+31 kJ, feels cold by itself) 0.52. With no ΔH the old heat-class rules apply (none today). The overrides (thermite 1.0) still win.
- The I-053 tests still pass unchanged except `the_effect_data_is_consistent`, which no longer reads the removed tint tables and now also checks that every tint is a sourced colour of the reaction's show record. The I-053 spec's section 11 table shows the old strengths and tints; the table in section 9 here supersedes it.

## 7. ΔH against the codex heat class

Every reaction with a codex heat class agrees with its computed ΔH sign. Two disagreed while building and were fixed by correcting the record's states (not the codex):

| reaction | first computed | fix | now |
|---|---|---|---|
| `water_gas_shift` | +2.8 kJ with liquid water | the shift runs on steam: H2O(g) | −41.2 kJ (releases) |
| `esterification` | +76.5 kJ with vapour products (the hot rule) | it is heated under reflux, so the ester and water stay liquid | −3.2 kJ (little) |

Eight reactions have no codex heat class but a computed ΔH (the codex could now say "releases", or "little" for the ones within 25 kJ): `golden_rain` −63.4, `copper_hydroxide` −54.6, `iron3_hydroxide` −84.5, `iron2_hydroxide` −19.9, `lead_chloride` −23.3, `zinc_hydroxide` −28.0, `co2_water` −20.4, `so2_water` −26.2 kJ. Left for the science review; the codex is unchanged.

Four records have an estimated ΔH (an estimated term): `candle_burns` (pentacosane), `copper_carbonate_heat` and `copper_carbonate_acid` (CuCO3), `nitrogen_triiodide` (NI3).

Flames with no temperature (no boiling point to bound them above 4000 K): `lithium_oxygen`, `magnesium_sulfur`, `magnesium_nitrogen`. Glows with no temperature: `aluminium_chlorine` (the same reason) and the externally heated ones (`limestone_roast`, `cement_clinker`, `glass_making`, `water_gas`, `boudouard`, `blast_furnace`, `zinc_oxide_carbon`).

## 8. Acceptance tests

`crates/chem/src/show.rs`:

- `every_codex_reaction_has_a_complete_show_record`: 182 records in codex order; every species has a state, a conformer and (products) `at_room`; something visible or a note; a timescale; `sped_up` exactly when slow; every flame has a described colour, every burning a temperature, every temperature estimated and in 900 to 4000 K; every precipitate a grain; every colour change listed; a non-empty, illustrative atom story whose events lie in 0..1 and whose atoms are the reactants' atoms.
- `every_colour_and_enthalpy_has_a_source_or_is_flagged`: every colour's source key exists and every hex is flagged `hex_estimated` and well formed; every ΔfH° has a known source or is `estimated` with how; every species has one; every ΔH term has a source or is estimated, and `dh_estimated` matches.
- `the_enthalpy_sign_agrees_with_the_heat_class`: no unexplained disagreement; methane −802.3 kJ (steam), HCl + NaOH −55.8 kJ, limestone +178.3 kJ, thermite −851.5 kJ.
- `gas_densities_relative_to_air_are_right`: H2 < 0.1 and rises; CO2 1.52 and sinks; O2 > 1; NO2 tinted; steam rises, clear, condenses to mist; Cl2 > 2.4 with a colour.
- `geometry_covers_every_species`: a geometry for every codex species, its atoms match the formula, valid bonds and orders; H2O, CO2, N2, Cl2 bond lengths within 0.03 to 0.06 Å of experiment; NaCl a lattice, CH4 not.
- `the_shows_follow_the_chemistry`: silver chloride a cloudy white precipitate from aq reactants; steam from methane; molten salt in electrolysis; methane flame 2200 to 2400 K; chalk fizzes; copper sulfate changes colour; magnesium smokes; rust slow and sped up; neutralisation instant; sodium's 589.0 and 589.6 nm lines; steam condenses, molten iron freezes; hydrogen burning breaks 3 bonds and forms 4.
- `a_frame_shows_bonds_stretch_break_and_form`: hydrogen burning has 6 atoms and 3 bonds at t = 0, a stretched bond mid-break, 4 O–H bonds at rest and the flame at full at t = 1, each O bonded to two H, continuous motion; every record's frames are finite.
- `strength_follows_the_documented_scale`.

`crates/chem/src/discover.rs` (I-053): all six tests pass; `the_effect_data_is_consistent` checks every tint against the show record.

`test/science-sources.test.ts`: the new card strings have rows C-116 to C-120 and three Not-claims rows.

`show_table` and `show_print` (ignored) print the table below and one record.

## 9. All 182 records

Generated by `cargo test -p realgame-chem show_table -- --ignored --nocapture`. ΔH in kJ per mole of reaction as written ("est." when estimated); T is the flame or glow temperature (K, an estimated upper bound); strength and tint are the spell's.

| reaction | medium | ΔH | codex heat | visible | T | speed | strength | tint |
|---|---|---|---|---|---|---|---|---|
| `methane_burns` | air | -802.5 | releases | flame, shimmer, condensation | 2330 | fast | 1 | #5a7dff |
| `hydrogen_burns` | air | -483.6 | releases | flame, shimmer, condensation | 2520 | instant | 0.92 |  |
| `water_splits` | water | +571.6 | absorbs | bubbles |  | slow (sped up) | 0.24 |  |
| `carbon_burns` | air | -393.5 | releases | glow, shimmer | 2460 | fast | 0.89 |  |
| `iron_rusts` | air | -1648.4 | releases | colour_change |  | slow (sped up) | 1 |  |
| `baking_soda_vinegar` | water | +31.2 | absorbs | bubbles |  | fast | 0.52 |  |
| `photosynthesis` | water | +2935.4 | absorbs | bubbles |  | slow (sped up) | 0.25 |  |
| `respiration` | air | -2802.5 | releases |  |  | slow (sped up) | 1 |  |
| `candle_burns` | air | -15378.5 est. | releases | flame, shimmer, condensation | 2390 | slow (sped up) | 1 | #ffa830 |
| `peroxide_splits` | water | -189.3 | releases | bubbles |  | fast | 0.79 |  |
| `magnesium_burns` | air | -1203.2 | releases | flame, glow, colour_change, smoke, shimmer | 3870 | fast | 1 | #f5f7ff |
| `propane_burns` | air | -2043.9 | releases | flame, shimmer, condensation | 2390 | fast | 1 | #5a7dff |
| `ethanol_burns` | air | -1234.8 | releases | flame, shimmer, condensation | 2290 | fast | 1 |  |
| `fermentation` | water | -101.4 | releases | bubbles |  | slow (sped up) | 0.69 |  |
| `limewater_test` | water | -76.8 | releases | precipitate |  | instant | 0.65 | #f4f4f4 |
| `sodium_water` | water | -368.6 | releases | bubbles |  | fast | 0.88 |  |
| `lithium_water` | water | -445.4 | releases | bubbles |  | fast | 0.91 |  |
| `potassium_water` | water | -393.2 | releases | flame, bubbles, shimmer | 2980 | fast | 0.89 |  |
| `calcium_water` | water | -413.6 | releases | bubbles, precipitate, colour_change |  | fast | 0.9 | #f4f4f4 |
| `barium_water` | water | -426.0 | releases | bubbles |  | fast | 0.91 |  |
| `strontium_water` | water | -434.2 | releases | bubbles |  | fast | 0.91 |  |
| `magnesium_steam` | dry | -359.8 | releases | flame, colour_change, smoke, shimmer | 3490 | fast | 0.88 | #f4f4f4 |
| `salt_from_elements` | dry | -822.4 | releases | flame, colour_change, smoke, shimmer | 1740 | fast | 1 | #ffb21e |
| `potassium_chlorine` | dry | -873.0 | releases | flame, colour_change, smoke, shimmer | 1690 | fast | 1 | #b58fd8 |
| `magnesium_chlorine` | dry | -641.3 | releases | flame, colour_change, smoke, shimmer | 1690 | fast | 0.97 | #f5f7ff |
| `calcium_chlorine` | dry | -795.4 | releases | flame, colour_change, smoke, shimmer | 2210 | fast | 1 | #ff6b2b |
| `aluminium_chlorine` | dry | -1408.4 | releases | glow, colour_change, smoke, shimmer |  | fast | 1 |  |
| `copper_chlorine` | dry | -220.1 | releases | glow, colour_change, smoke, shimmer | 1270 | fast | 0.81 |  |
| `sodium_bromine` | dry | -722.2 | releases | colour_change |  | slow (sped up) | 0.98 |  |
| `iron_sulfur` | dry | -100.0 | releases | glow, colour_change, shimmer | 1670 | fast | 0.69 |  |
| `magnesium_sulfur` | dry | -346.0 | releases | flame, colour_change, smoke, shimmer |  | instant | 0.87 | #f5f7ff |
| `copper_oxide` | air | -314.6 | releases | colour_change |  | fast | 0.86 |  |
| `zinc_oxide` | air | -701.0 | releases | flame, colour_change, smoke, shimmer | 2760 | fast | 0.98 | #7fd1c0 |
| `aluminium_oxide` | air | -3351.4 | releases | flame, colour_change, smoke, shimmer | 3250 | fast | 1 | #f5f7ff |
| `lithium_oxygen` | air | -1195.8 | releases | flame, colour_change, smoke, shimmer |  | fast | 1 | #d6213a |
| `sodium_oxygen` | air | -828.4 | releases | flame, colour_change, smoke, shimmer | 2570 | fast | 1 | #ffb21e |
| `calcium_oxygen` | air | -1269.8 | releases | flame, colour_change, smoke, shimmer | 3120 | fast | 1 | #ff6b2b |
| `silicon_oxygen` | air | -910.7 | releases | glow, colour_change, shimmer | 3220 | fast | 1 |  |
| `sulfur_burns` | air | -296.8 | releases | flame, shimmer | 1970 | fast | 0.85 | #3d5bff |
| `phosphorus_burns` | air | -2984.0 | releases | flame, colour_change, smoke, shimmer | 3080 | fast | 1 |  |
| `co_burns` | air | -566.0 | releases | flame, shimmer | 2660 | fast | 0.95 | #4f6bff |
| `carbon_little_air` | air | -221.0 | releases | glow, shimmer | 1490 | fast | 0.81 |  |
| `ethane_burns` | air | -2856.8 | releases | flame, shimmer, condensation | 2380 | fast | 1 | #5a7dff |
| `butane_burns` | air | -5314.6 | releases | flame, shimmer, condensation | 2400 | fast | 1 | #5a7dff |
| `pentane_burns` | air | -3244.8 | releases | flame, shimmer, condensation | 2390 | fast | 1 | #ffa830 |
| `hexane_burns` | air | -7709.8 | releases | flame, shimmer, condensation | 2390 | fast | 1 | #ffa830 |
| `octane_burns` | air | -10148.2 | releases | flame, shimmer, condensation | 2390 | fast | 1 | #ffa830 |
| `ethene_burns` | air | -1323.0 | releases | flame, shimmer, condensation | 2570 | fast | 1 | #ffa830 |
| `acetylene_burns` | air | -2512.4 | releases | flame, smoke, shimmer, condensation | 2910 | fast | 1 | #ffa830 |
| `benzene_burns` | air | -6271.0 | releases | flame, smoke, shimmer, condensation | 2510 | fast | 1 | #ffa830 |
| `methanol_burns` | air | -1275.8 | releases | flame, shimmer, condensation | 2230 | fast | 1 |  |
| `propanol_burns` | air | -3690.2 | releases | flame, shimmer, condensation | 2330 | fast | 1 |  |
| `ammonia_burns` | air | -1267.2 | releases | flame, shimmer, condensation | 2110 | fast | 1 |  |
| `hydrogen_sulfide_burns` | air | -1036.0 | releases | flame, shimmer, condensation | 2120 | fast | 1 | #3d5bff |
| `carbon_disulfide_burns` | air | -1076.1 | releases | flame, shimmer | 2290 | fast | 1 | #3d5bff |
| `silane_burns` | air | -1428.6 | releases | flame, smoke, shimmer, condensation | 3220 | instant | 1 |  |
| `haber` | dry | -91.8 | releases |  |  | slow (sped up) | 0.68 |  |
| `ammonia_splits` | dry | +91.8 | absorbs |  |  | fast | 0.17 |  |
| `hcl_synthesis` | dry | -184.6 | releases | colour_change |  | instant | 0.78 |  |
| `hbr_synthesis` | dry | -103.5 | releases | colour_change |  | fast | 0.7 |  |
| `hf_synthesis` | dry | -546.6 | releases | colour_change |  | instant | 0.94 |  |
| `contact_so3` | air | -197.8 | releases | condensation |  | slow (sped up) | 0.79 |  |
| `so3_water` | water | -227.8 | releases | smoke |  | instant | 0.81 |  |
| `ostwald_1` | air | -902.0 | releases | glow, shimmer, condensation | 1250 | fast | 1 |  |
| `no_oxidises` | air | -116.2 | releases | colour_change |  | instant | 0.71 | #a2401c |
| `ostwald_3` | water | -137.3 | releases | bubbles, colour_change |  | fast | 0.74 |  |
| `lightning_nitrogen` | air | +182.6 | absorbs |  |  | instant | 0.19 |  |
| `steam_reforming` | dry | +205.9 | absorbs |  |  | slow (sped up) | 0.2 |  |
| `water_gas_shift` | dry | -41.2 | releases |  |  | slow (sped up) | 0.56 |  |
| `water_gas` | dry | +131.3 | absorbs | glow, shimmer |  | slow (sped up) | 0.18 |  |
| `boudouard` | dry | +172.5 | absorbs | glow, shimmer |  | fast | 0.19 |  |
| `methanol_synthesis` | dry | -90.5 | releases | condensation |  | slow (sped up) | 0.68 |  |
| `urea_synthesis` | dry | -89.6 | releases | condensation |  | slow (sped up) | 0.67 |  |
| `limestone_roast` | dry | +179.2 | absorbs | glow, shimmer |  | slow (sped up) | 0.19 | #f4f4f4 |
| `slaking_lime` | air | -64.5 | releases | condensation |  | fast | 0.63 |  |
| `lime_dries` | dry | +108.5 | absorbs | condensation |  | slow (sped up) | 0.18 | #f4f4f4 |
| `copper_carbonate_heat` | dry | +44.2 est. | absorbs | colour_change |  | slow (sped up) | 0.14 | #1c1c1c |
| `magnesium_carbonate_heat` | dry | +100.7 | absorbs |  |  | slow (sped up) | 0.17 | #f4f4f4 |
| `baking_soda_heat` | dry | +135.6 | absorbs | condensation |  | slow (sped up) | 0.18 |  |
| `potassium_chlorate_heat` | dry | -77.6 | releases | bubbles |  | slow (sped up) | 0.65 | #f4f4f4 |
| `mercury_oxide_heat` | dry | +304.4 | absorbs | condensation |  | slow (sped up) | 0.21 |  |
| `silver_oxide_heat` | dry | +62.2 | absorbs | colour_change |  | slow (sped up) | 0.16 | #c0c0c0 |
| `copper_nitrate_heat` | dry | +424.0 | absorbs | colour_change |  | slow (sped up) | 0.23 | #1c1c1c |
| `ammonium_chloride_heat` | dry | +176.2 | absorbs | smoke |  | slow (sped up) | 0.19 |  |
| `lab_ammonia` | dry | +243.2 | absorbs | condensation |  | slow (sped up) | 0.21 | #f4f4f4 |
| `salt_electrolysis` | dry | +822.4 | absorbs | colour_change |  | slow (sped up) | 0.25 | #c8d84a |
| `brine_electrolysis` | water | +446.0 | absorbs | bubbles, colour_change |  | slow (sped up) | 0.23 | #c8d84a |
| `alumina_electrolysis` | dry | +3351.4 | absorbs |  |  | slow (sped up) | 0.25 |  |
| `copper_chloride_electrolysis` | water | +269.6 | absorbs | bubbles, precipitate, colour_change |  | slow (sped up) | 0.21 | #b87333 |
| `hcl_electrolysis` | water | +334.4 | absorbs | bubbles, colour_change |  | slow (sped up) | 0.22 | #c8d84a |
| `neutralise_hcl_naoh` | water | -55.8 | releases |  |  | instant | 0.6 |  |
| `antacid` | water | -114.0 | releases |  |  | instant | 0.71 |  |
| `neutralise_hcl_koh` | water | -55.8 | releases |  |  | instant | 0.6 |  |
| `neutralise_hcl_lioh` | water | -55.8 | releases |  |  | instant | 0.6 |  |
| `neutralise_h2so4_naoh` | water | -111.6 | releases |  |  | instant | 0.71 |  |
| `neutralise_h2so4_koh` | water | -111.6 | releases |  |  | instant | 0.71 |  |
| `neutralise_hno3_naoh` | water | -55.8 | releases |  |  | instant | 0.6 |  |
| `neutralise_hno3_koh` | water | -55.8 | releases |  |  | instant | 0.6 |  |
| `neutralise_ammonia_hcl` | air | -176.2 | releases | smoke |  | instant | 0.77 | #f4f4f4 |
| `neutralise_caoh2_hcl` | water | -111.6 | releases |  |  | instant | 0.71 |  |
| `neutralise_vinegar_naoh` | water | -56.0 | releases |  |  | instant | 0.6 |  |
| `neutralise_ammonia_h2so4` | water | -104.4 | releases |  |  | instant | 0.7 |  |
| `neutralise_ammonia_hno3` | water | -52.2 | releases |  |  | instant | 0.59 |  |
| `neutralise_baoh2_h2so4` | water | -137.9 | releases | precipitate |  | instant | 0.74 | #f4f4f4 |
| `co2_naoh` | water | -109.4 | releases |  |  | instant | 0.7 |  |
| `cuo_acid` | water | -63.7 | releases | colour_change |  | instant | 0.62 |  |
| `mgo_acid` | water | -151.1 | releases |  |  | instant | 0.75 |  |
| `zinc_acid` | water | -153.9 | releases | bubbles |  | fast | 0.75 |  |
| `magnesium_acid` | water | -466.9 | releases | bubbles |  | fast | 0.92 |  |
| `iron_acid` | water | -89.1 | releases | bubbles, colour_change |  | fast | 0.67 | #b8d8a0 |
| `aluminium_acid` | water | -1062.0 | releases | bubbles |  | fast | 1 |  |
| `calcium_acid` | water | -542.8 | releases | bubbles |  | fast | 0.94 |  |
| `zinc_sulfuric` | water | -153.9 | releases | bubbles |  | fast | 0.75 |  |
| `magnesium_sulfuric` | water | -466.9 | releases | bubbles |  | fast | 0.92 |  |
| `iron_sulfuric` | water | -89.1 | releases | bubbles, colour_change |  | fast | 0.67 | #b8d8a0 |
| `copper_nitric` | water | -351.4 | releases | bubbles, colour_change |  | fast | 0.88 | #2f7fd0 |
| `chalk_acid` | water | -14.5 | releases | bubbles |  | fast | 0.41 |  |
| `washing_soda_acid` | water | -2.2 | releases | bubbles |  | fast | 0.17 |  |
| `baking_soda_hcl` | water | +12.7 | absorbs | bubbles |  | fast | 0.39 |  |
| `copper_carbonate_acid` | water | -19.5 est. | releases | bubbles, colour_change |  | fast | 0.45 | #2f7fd0 |
| `marble_sulfuric` | water | +3.1 | little | bubbles, precipitate |  | fast | 0.21 | #f4f4f4 |
| `magnesium_carbonate_acid` | water | -50.4 | releases | bubbles |  | fast | 0.59 |  |
| `iron_copper_sulfate` | water | -153.9 | releases | precipitate, colour_change |  | slow (sped up) | 0.75 | #b8d8a0 |
| `zinc_copper_sulfate` | water | -218.7 | releases | precipitate, colour_change |  | slow (sped up) | 0.81 | #b87333 |
| `magnesium_copper_sulfate` | water | -531.7 | releases | precipitate, colour_change |  | slow (sped up) | 0.94 | #b87333 |
| `aluminium_copper_sulfate` | water | -1256.4 | releases | precipitate, colour_change |  | slow (sped up) | 1 | #b87333 |
| `magnesium_zinc_sulfate` | water | -313.0 | releases | precipitate, colour_change |  | slow (sped up) | 0.86 | #8e9aa6 |
| `zinc_lead_nitrate` | water | -152.2 | releases | precipitate, colour_change |  | slow (sped up) | 0.75 | #5a5f66 |
| `copper_silver_nitrate` | water | -146.4 | releases | precipitate, colour_change |  | slow (sped up) | 0.75 | #2f7fd0 |
| `zinc_silver_nitrate` | water | -365.1 | releases | precipitate, colour_change |  | slow (sped up) | 0.88 | #c0c0c0 |
| `thermite` | dry | -851.5 | releases | glow, colour_change, smoke, shimmer | 2800 | fast | 1 |  |
| `blast_furnace` | dry | -24.8 | releases | glow, colour_change, shimmer |  | slow (sped up) | 0.49 | #6e6e6e |
| `copper_oxide_carbon` | dry | -78.9 | releases | glow, colour_change, shimmer | 1070 | fast | 0.66 | #b87333 |
| `copper_oxide_hydrogen` | dry | -84.5 | releases | glow, colour_change, shimmer, condensation | 1360 | fast | 0.67 |  |
| `zinc_oxide_carbon` | dry | +370.4 | absorbs | glow, shimmer |  | fast | 0.22 |  |
| `magnesium_co2` | dry | -809.7 | releases | flame, colour_change, smoke, shimmer | 3870 | fast | 1 | #f5f7ff |
| `peroxide_iodide` | water | -247.4 | releases | colour_change |  | fast | 0.82 |  |
| `chlorine_bromide` | water | -70.4 | releases | colour_change |  | instant | 0.64 | #e8a33a |
| `chlorine_iodide` | water | -178.0 | releases | colour_change |  | instant | 0.78 | #8b4513 |
| `bromine_iodide` | water | -107.6 | releases | colour_change |  | instant | 0.7 | #8b4513 |
| `bleach_making` | water | -100.1 | releases | colour_change |  | instant | 0.69 |  |
| `iron_chlorine` | dry | -799.0 | releases | glow, colour_change, smoke, shimmer | 590 | fast | 1 |  |
| `aluminium_iodine` | dry | -605.8 | releases | colour_change, smoke |  | fast | 0.96 |  |
| `zinc_iodine` | dry | -208.0 | releases | colour_change |  | slow (sped up) | 0.8 |  |
| `magnesium_nitrogen` | dry | -461.1 | releases | flame, colour_change, smoke, shimmer |  | fast | 0.92 | #f5f7ff |
| `lithium_nitrogen` | dry | -329.1 | releases | colour_change |  | slow (sped up) | 0.87 |  |
| `copper_sulfur` | dry | -79.5 | releases | glow, colour_change, shimmer | 1360 | fast | 0.66 |  |
| `zinc_sulfur` | dry | -206.0 | releases | glow, colour_change, smoke, shimmer | 1460 | instant | 0.8 |  |
| `hydrogen_sulfur` | dry | -20.6 | releases |  |  | fast | 0.46 |  |
| `sodium_hydride` | dry | -112.6 | releases |  |  | fast | 0.71 |  |
| `silver_tarnish` | air | -595.6 | releases | colour_change |  | slow (sped up) | 0.96 |  |
| `silver_chloride` | water | -65.4 | releases | precipitate |  | instant | 0.63 | #f4f4f4 |
| `silver_bromide` | water | -84.4 | releases | precipitate |  | instant | 0.67 | #f1e6b0 |
| `silver_iodide` | water | -112.2 | releases | precipitate |  | instant | 0.71 | #f2df73 |
| `golden_rain` | water | -63.4 | (none) | precipitate |  | instant | 0.62 | #ffd21f |
| `barium_sulfate` | water | -26.3 | releases | precipitate |  | instant | 0.49 | #f4f4f4 |
| `copper_hydroxide` | water | -54.6 | (none) | precipitate, colour_change |  | instant | 0.6 | #3d8fe0 |
| `iron2_hydroxide` | water | -19.9 | (none) | precipitate, colour_change |  | instant | 0.45 | #5b7f4f |
| `iron3_hydroxide` | water | -84.5 | (none) | precipitate, colour_change |  | instant | 0.67 | #a0522d |
| `calcium_carbonate_ppt` | water | +12.3 | absorbs | precipitate |  | instant | 0.39 | #f4f4f4 |
| `magnesium_hydroxide_ppt` | water | +2.4 | little | precipitate |  | instant | 0.18 | #f4f4f4 |
| `lead_chloride` | water | -23.3 | (none) | precipitate |  | instant | 0.48 | #f4f4f4 |
| `zinc_hydroxide` | water | -28.0 | (none) | precipitate |  | instant | 0.5 | #f4f4f4 |
| `co2_water` | water | -20.4 | (none) |  |  | fast | 0.46 |  |
| `so2_water` | water | -26.2 | (none) |  |  | fast | 0.49 |  |
| `ammonia_water` | water | -34.4 | releases |  |  | fast | 0.53 |  |
| `sodium_oxide_water` | water | -240.2 | releases |  |  | fast | 0.82 |  |
| `p4o10_water` | water | -454.6 | releases |  |  | instant | 0.92 |  |
| `ethene_hydrogen` | dry | -136.4 | releases |  |  | slow (sped up) | 0.74 |  |
| `ethene_bromine` | water | -129.0 | releases | colour_change |  | fast | 0.73 |  |
| `ethene_water` | dry | -45.4 | releases | condensation |  | slow (sped up) | 0.57 |  |
| `esterification` | dry | -3.2 | little |  |  | slow (sped up) | 0.21 |  |
| `methane_chlorine` | dry | -99.6 | releases | colour_change |  | fast | 0.69 |  |
| `ethanol_dehydration` | dry | +88.2 | absorbs | condensation |  | slow (sped up) | 0.17 |  |
| `cracking` | dry | +144.8 | absorbs | condensation |  | slow (sped up) | 0.19 |  |
| `sucrose_hydrolysis` | water | -12.5 | releases |  |  | slow (sped up) | 0.39 |  |
| `ozone_forms` | air | +285.4 | absorbs |  |  | fast | 0.21 |  |
| `ozone_splits` | dry | -285.4 | releases |  |  | slow (sped up) | 0.85 |  |
| `nitrogen_dioxide_dimer` | dry | -55.3 | releases | colour_change |  | fast | 0.6 |  |
| `glass_making` | dry | +93.0 | absorbs | glow, bubbles, shimmer |  | slow (sped up) | 0.17 |  |
| `cement_clinker` | dry | +89.9 | absorbs | glow, shimmer |  | slow (sped up) | 0.17 |  |
| `nitrogen_triiodide` | dry | -104.8 est. | releases | colour_change, smoke |  | instant | 0.7 | #8a3fa0 |
