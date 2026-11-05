# I-059: Chemistry Bench bands by school grade

Branch `loop/bench-reband`, from `loop/bench-audit` (I-058 on top of `loop/bench-play`). Paawan decided the Chemistry Bench's four bands should be school grades, not ages: Sprout was too complicated for ages 4 to 7. I-059 was free: `docs/loop/items/` ran to I-058, and no remote branch has an I-059 folder.

The band ids (`sprout`, `seedling`, `apprentice`, `adept`) are unchanged, so everything saved by band name stays valid: the Bench page's `formula.bench.band`, `formula.bench.guide.<band>`, the found list, saved spells, a class link's `?band=`, and the analytics band values.

## 1. The bands

| band | grades | NGSS performance expectations | what it covers | what the Bench shows |
|---|---|---|---|---|
| Sprout | 6–7 | MS-PS1-1 (atoms and molecules as models), MS-PS1-2 (evidence a new substance formed), MS-PS1-5 (atoms are conserved); also MS-PS1-6 and MS-LS1-6/7 | Atoms and molecules as models; "a new substance forms" evidence (gas, colour change, a solid, heat); simple word equations and the simple formulas of very familiar substances (H2O, CO2, O2, NaCl, H2). Balanced equations are shown, never balanced by hand. The most visible, everyday, safe reactions. | 12 reactions, 16 bottles (was 19 and 22). Formulas on every bottle, table row and card, with the everyday name; a word equation under each equation; the element shelf and plate (molecules as models); "Show me how" on a result counts each kind of atom on both sides. No "out" side, no scale, no type box; Electricity and High pressure stay hidden. Tap-to-read stays, as an accessibility option. |
| Seedling | 8–9 | MS-PS1-1 to MS-PS1-6, MS-LS1-6/7, and early high school HS-PS1-2 and HS-PS1-7 | Formulas and conservation of mass; balancing simple equations with help (the scale, colour-coded atom counts, "Show me how"); reaction types: synthesis, decomposition, combustion, simple displacement, with neutralisation and metal + acid; exothermic and endothermic as warm or cold. | 33 reactions (26 core, 7 rule), 32 bottles. Scale and "out" side; no type box. |
| Apprentice | 10–11 | HS-PS1-1, 2, 4, 6, 7 (typical high-school chemistry) | Full balancing (type box), all main reaction types, precipitation by the solubility rules, the reactivity series, conditions (catalyst, pressure, heat), energy changes in words. | 149 reactions (77 core, 72 rule), 85 bottles. States of matter on the result card. |
| Adept | Grade 12+ (AP, college, adults) | HS-PS1-1 to HS-PS1-7, plus an AP Chemistry topic (College Board CED: 1.7, 4.2, 4.5, 4.7, 4.9, 5.11, 6.7, 6.8, 7.9) | Stoichiometry, ΔH in kJ, redox, rule-tier predictions, lab records; later, inventions. | 292 reactions (67 core, 225 rule), 150 bottles. ΔH, flame temperature and gas densities on the result card. |

The labels read "Sprout · Grades 6–7", "Seedling · Grades 8–9", "Apprentice · Grades 10–11" and "Adept · Grade 12+" on the Bench page; `Tier::grades` in `crates/sim/src/tier.rs` holds the same strings. Sentences on the page say "Sprout (grades 6–7)" (`BAND_NAME` in `bench-free.js`).

The world's picker (`index.html` `#age`, now "Pick your grade") and the Formula Bench chooser (`bench.js`, "What grade are you in?", "Change grade") keep three choices: "Sprout · Grades 6–7", "Seedling · Grades 8–9" and "Apprentice · Grade 10+". The world reads Adept as Apprentice (`bandOf` in `bench.js`), and analytics counts three bands (`functions/_lib/analytics.ts`); adding a fourth choice would change what data is collected (I-021), so it is left for a separate decision.

## 2. Reaction counts per band

Each band also plays every reaction of the bands below it. All 486 reactions have textbook or data evidence (I-058), so all are playable.

| band | before (core + rule) | after (core + rule) | up to this band, before | up to this band, after |
|---|---|---|---|---|
| Sprout | 19 (19 + 0) | 12 (12 + 0) | 19 | 12 |
| Seedling | 27 (20 + 7) | 33 (26 + 7) | 46 | 45 |
| Apprentice | 130 (68 + 62) | 149 (77 + 72) | 176 | 194 |
| Adept | 310 (75 + 235) | 292 (67 + 225) | 486 | 486 |

Bottles on the shelf (every reactant of a reaction the band sees): Sprout 22 to 16, Seedling 33 to 32, Apprentice 81 to 85, Adept 150 (unchanged).

## 3. Reactions that moved

### Core (`content/chem/core.json`): 30 moved

| reaction | equation | before | after | why |
|---|---|---|---|---|
| `candle_burns` | C25H52 + 38O2 -> 25CO2 + 26H2O | sprout | seedling | paraffin formula (C25H52) is not a familiar one; combustion with balancing is Seedling |
| `magnesium_burns` | 2Mg + O2 -> 2MgO | sprout | seedling | the classic conservation-of-mass reaction (MS-PS1-5), taught with the balance scale |
| `propane_burns` | C3H8 + 5O2 -> 3CO2 + 4H2O | sprout | seedling | more hydrocarbon combustion: fewer bottles at Sprout |
| `fermentation` | C6H12O6 -> 2C2H5OH + 2CO2 | sprout | seedling | ethanol formula; Sprout keeps respiration and photosynthesis |
| `limewater_test` | Ca(OH)2 + CO2 -> CaCO3 + H2O | seedling | sprout | the everyday test for carbon dioxide (breathing into limewater), simple formulas |
| `sodium_water` | 2Na + 2H2O -> 2NaOH + H2 | seedling | apprentice | alkali metals and the reactivity series are high-school chemistry |
| `lithium_water` | 2Li + 2H2O -> 2LiOH + H2 | seedling | apprentice | as sodium_water |
| `calcium_water` | Ca + 2H2O -> Ca(OH)2 + H2 | seedling | apprentice | metals with water go with the Group 1 and 2 trends |
| `sodium_oxygen` | 4Na + O2 -> 2Na2O | seedling | apprentice | alkali metal, lab only |
| `sulfur_burns` | S + O2 -> SO2 | sprout | seedling | gives a toxic gas (SO2) and needs lab care |
| `butane_burns` | 2C4H10 + 13O2 -> 8CO2 + 10H2O | sprout | seedling | more hydrocarbon combustion: fewer bottles at Sprout |
| `propanol_burns` | 2C3H7OH + 9O2 -> 6CO2 + 8H2O | adept | apprentice | combustion balancing practice |
| `slaking_lime` | CaO + H2O -> Ca(OH)2 | sprout | seedling | a synthesis (Seedling reaction type), strongly exothermic and caustic |
| `copper_carbonate_heat` | CuCO3 -> CuO + CO2 | apprentice | seedling | the standard decomposition example (green to black) |
| `neutralise_ammonia_h2so4` | 2NH3 + H2SO4 -> (NH4)2SO4 | adept | apprentice | acid-base with a familiar fertiliser product |
| `neutralise_ammonia_hno3` | NH3 + HNO3 -> NH4NO3 | adept | apprentice | as above |
| `chalk_acid` | CaCO3 + 2HCl -> CaCl2 + H2O + CO2 | sprout | seedling | needs hydrochloric acid, a lab acid |
| `baking_soda_hcl` | NaHCO3 + HCl -> NaCl + H2O + CO2 | apprentice | seedling | the cold (endothermic) fizz for "warm or cold" |
| `marble_sulfuric` | CaCO3 + H2SO4 -> CaSO4 + H2O + CO2 | adept | apprentice | carbonate and acid, a main reaction type |
| `magnesium_copper_sulfate` | Mg + CuSO4 -> MgSO4 + Cu | apprentice | seedling | simple displacement, a Seedling reaction type |
| `aluminium_copper_sulfate` | 2Al + 3CuSO4 -> Al2(SO4)3 + 3Cu | adept | apprentice | the reactivity series |
| `blast_furnace` | Fe2O3 + 3CO -> 2Fe + 3CO2 | apprentice | adept | redox is an Adept topic (AP 4.9) |
| `copper_oxide_carbon` | 2CuO + C -> 2Cu + CO2 | apprentice | adept | redox |
| `silver_tarnish` | 4Ag + 2H2S + O2 -> 2Ag2S + 2H2O | apprentice | adept | a three-reactant redox |
| `iron2_hydroxide` | FeSO4 + 2NaOH -> Fe(OH)2 + Na2SO4 | adept | apprentice | precipitation by the solubility rules (coloured hydroxides) |
| `iron3_hydroxide` | FeCl3 + 3NaOH -> Fe(OH)3 + 3NaCl | adept | apprentice | as above |
| `magnesium_hydroxide_ppt` | MgCl2 + 2NaOH -> Mg(OH)2 + 2NaCl | adept | apprentice | as above |
| `zinc_hydroxide` | ZnSO4 + 2NaOH -> Zn(OH)2 + Na2SO4 | adept | apprentice | as above |
| `so2_water` | SO2 + H2O -> H2SO3 | adept | apprentice | simple synthesis (acid rain) |
| `sodium_oxide_water` | Na2O + H2O -> 2NaOH | adept | apprentice | simple synthesis (a basic oxide in water) |


New `short` lines (eight words or fewer, required at Seedling and below) for the three that came down to Seedling: `copper_carbonate_heat` "Green powder turns black and gives off gas.", `baking_soda_hcl` "It fizzes and the mix gets colder.", `magnesium_copper_sulfate` "Magnesium pushes copper out. The blue fades."

### Rule tier (`content/chem/rules.json`): 12 moved

`tools/chemdb/build_rules.py`'s band logic is unchanged: a rule reaction's band comes from the band its species' bottles first appear at in core.json (`band_of`, `SHELF`), at least Seedling for the simple acid, metal and carbonate reactions and at least Apprentice for the rest; lead is Adept only, and no rule reaction is Sprout. Regenerating it (`/tmp/v/bin/python -I tools/chemdb/build_rules.py content/chem`) carries the core re-band through:

- to Apprentice, because iron(II), iron(III) and magnesium hydroxides and their salts are Apprentice bottles now: `rule_acid_base_hcl_fe2o3`, `rule_precipitate_agno3_mgcl2`, `rule_precipitate_bacl2_feso4`, `rule_precipitate_fecl3_koh`, `rule_precipitate_fecl3_lioh`, `rule_precipitate_feso4_koh`, `rule_precipitate_mgcl2_koh`, `rule_precipitate_mgcl2_lioh`, `rule_displacement_mg_cucl2`, `rule_displacement_mg_feso4`, `rule_displacement_zn_feso4`;
- to Adept: `rule_displacement_mg_agno3` (Mg + 2AgNO3 -> Mg(NO3)2 + 2Ag), because silver metal's first bottle is now `silver_tarnish`, an Adept redox reaction.

The seven Seedling rule reactions are unchanged (CaO, Ca(OH)2 and Mg(OH)2 with vinegar, CaO with HCl, Mg in vinegar, chalk and washing soda in vinegar). `content/chem/evidence.csv` is regenerated (`scripts/evidence-csv.sh`): only its band column changed, for the 42 reactions above.

### Every band's reactions, after

**sprout** (12): `methane_burns`, `hydrogen_burns`, `carbon_burns`, `iron_rusts`, `baking_soda_vinegar`, `photosynthesis`, `respiration`, `peroxide_splits`, `limewater_test`, `limestone_roast`, `iron_copper_sulfate`, `silver_chloride`

**seedling** (26): `water_splits`, `candle_burns`, `magnesium_burns`, `propane_burns`, `ethanol_burns`, `fermentation`, `salt_from_elements`, `iron_sulfur`, `magnesium_sulfur`, `copper_oxide`, `sulfur_burns`, `butane_burns`, `slaking_lime`, `copper_carbonate_heat`, `baking_soda_heat`, `neutralise_hcl_naoh`, `antacid`, `zinc_acid`, `magnesium_acid`, `chalk_acid`, `washing_soda_acid`, `baking_soda_hcl`, `zinc_copper_sulfate`, `magnesium_copper_sulfate`, `copper_silver_nitrate`, `co2_water`

**apprentice** (77): `sodium_water`, `lithium_water`, `potassium_water`, `calcium_water`, `magnesium_steam`, `potassium_chlorine`, `magnesium_chlorine`, `calcium_chlorine`, `copper_chlorine`, `zinc_oxide`, `aluminium_oxide`, `lithium_oxygen`, `sodium_oxygen`, `calcium_oxygen`, `phosphorus_burns`, `co_burns`, `carbon_little_air`, `ethane_burns`, `pentane_burns`, `hexane_burns`, `octane_burns`, `ethene_burns`, `acetylene_burns`, `methanol_burns`, `propanol_burns`, `haber`, `hcl_synthesis`, `lightning_nitrogen`, `magnesium_carbonate_heat`, `ammonium_chloride_heat`, `salt_electrolysis`, `copper_chloride_electrolysis`, `neutralise_hcl_koh`, `neutralise_hcl_lioh`, `neutralise_h2so4_naoh`, `neutralise_h2so4_koh`, `neutralise_hno3_naoh`, `neutralise_hno3_koh`, `neutralise_ammonia_hcl`, `neutralise_caoh2_hcl`, `neutralise_vinegar_naoh`, `neutralise_ammonia_h2so4`, `neutralise_ammonia_hno3`, `co2_naoh`, `cuo_acid`, `mgo_acid`, `iron_acid`, `aluminium_acid`, `calcium_acid`, `zinc_sulfuric`, `magnesium_sulfuric`, `iron_sulfuric`, `copper_carbonate_acid`, `marble_sulfuric`, `magnesium_carbonate_acid`, `aluminium_copper_sulfate`, `magnesium_zinc_sulfate`, `zinc_silver_nitrate`, `thermite`, `chlorine_bromide`, `chlorine_iodide`, `bromine_iodide`, `iron_chlorine`, `copper_sulfur`, `zinc_sulfur`, `silver_bromide`, `silver_iodide`, `golden_rain`, `barium_sulfate`, `copper_hydroxide`, `iron2_hydroxide`, `iron3_hydroxide`, `calcium_carbonate_ppt`, `magnesium_hydroxide_ppt`, `zinc_hydroxide`, `so2_water`, `sodium_oxide_water`

**adept** (67): `barium_water`, `strontium_water`, `aluminium_chlorine`, `sodium_bromine`, `silicon_oxygen`, `benzene_burns`, `ammonia_burns`, `hydrogen_sulfide_burns`, `carbon_disulfide_burns`, `silane_burns`, `ammonia_splits`, `hbr_synthesis`, `hf_synthesis`, `contact_so3`, `so3_water`, `ostwald_1`, `no_oxidises`, `ostwald_3`, `steam_reforming`, `water_gas_shift`, `water_gas`, `boudouard`, `methanol_synthesis`, `urea_synthesis`, `lime_dries`, `potassium_chlorate_heat`, `mercury_oxide_heat`, `silver_oxide_heat`, `copper_nitrate_heat`, `lab_ammonia`, `brine_electrolysis`, `alumina_electrolysis`, `hcl_electrolysis`, `neutralise_baoh2_h2so4`, `copper_nitric`, `zinc_lead_nitrate`, `blast_furnace`, `copper_oxide_carbon`, `copper_oxide_hydrogen`, `zinc_oxide_carbon`, `magnesium_co2`, `peroxide_iodide`, `bleach_making`, `aluminium_iodine`, `zinc_iodine`, `magnesium_nitrogen`, `lithium_nitrogen`, `hydrogen_sulfur`, `sodium_hydride`, `silver_tarnish`, `lead_chloride`, `ammonia_water`, `p4o10_water`, `ethene_hydrogen`, `ethene_bromine`, `ethene_water`, `esterification`, `methane_chlorine`, `ethanol_dehydration`, `cracking`, `sucrose_hydrolysis`, `ozone_forms`, `ozone_splits`, `nitrogen_dioxide_dimer`, `glass_making`, `cement_clinker`, `nitrogen_triiodide`


## 4. What Sprout is now

Twelve everyday reactions a grade 6 or 7 student can see: hydrogen and methane burning, charcoal burning, iron rusting, baking soda and vinegar fizzing (and getting cold), photosynthesis and respiration, hydrogen peroxide foaming with yeast, limewater turning milky with carbon dioxide, limestone breaking down in strong heat, iron getting a copper coat in copper sulfate, and silver nitrate with salt water making a white solid. Sixteen bottles. Each bottle, table row and result shows the formula with its everyday name, and every equation has a word equation under it ("hydrogen + oxygen → water"). The plate and element shelf are there for building molecules as models, but there is nothing to balance: the result shows the balanced equation, and "Show me how" lists how many of each atom are on each side. The Guide's lessons are written for grade 6 and 7 readers (atoms, molecules, evidence of a new substance), name only very familiar formulas, and keep the "Try it in the world" quest links. Read-aloud buttons stay, for anyone who wants them.

Every live quest has a Sprout solver whose bottles the valley gives: melt (charcoal burning), smother (limestone in strong heat), cool (baking soda and vinegar), wall (silver nitrate and salt), reveal (iron and copper sulfate).

## 5. The Guide (`content/chem/guide.json`)

- **Sprout:** a new walkthrough text (2H2 + O2, A spark or flame, Run) in grade 6-7 words, naming H2, O2 and H2O. Ten lessons: Atoms join into molecules (MS-PS1-1), Burning needs oxygen, Bubbles are evidence, A test for carbon dioxide, A new solid appears, A colour change, Heat breaks it down, Breaking apart, Why iron rusts (MS-PS1-2), Chemistry in living things (MS-LS1-7). Hints for its eight families; seven challenges (no balancing).
- **Seedling:** the sodium and chlorine walkthrough now names the formulas and counts the atoms. Ten lessons: Nothing disappears and Balancing with help (MS-PS1-5), the four reaction types (synthesis, decomposition, combustion, metal swap), Warm or cold (MS-PS1-6), Acids meet bases, Metals that fizz, Tests for gases. Twelve hint families (no `water_metal`: metals in water are Apprentice now); seven challenges including the two balance-scale ones.
- **Apprentice:** the Haber walkthrough is unchanged. Ten lessons on HS-PS1 codes: balancing (HS-PS1-7), precipitates and the solubility rules, the reactivity series, Group 1 metals in water (HS-PS1-1), energy changes in words (HS-PS1-4), conditions in the Haber process (HS-PS1-6), neutralisation, halogen swaps, breaking down with heat, electrolysis. No `redox` hints or challenge (redox is Adept).
- **Adept:** the Contact process walkthrough and the nine lessons are kept, each with its AP Chemistry topic (`ap`, shown as "AP Chemistry 4.5" beside the NGSS code), plus a new lesson, ΔH in kilojoules (HS-PS1-4, AP 6.8). New reference R9 (the AP Chemistry Course and Exam Description).
- New everyday lines for the three reactions that came down to Seedling. Every lesson's reactions are playable at its band, and every live quest is still linked from a Sprout lesson.

The science facts are the reviewed ones the lessons already used, rewritten for the band; new ones are textbook statements with their sources in the lesson (for example: CO2 turns limewater milky; hydrogen burns with a squeaky pop; heating green copper carbonate gives black copper oxide and CO2; most iodides dissolve but lead iodide does not; ΔcH° of methane is about −890 kJ/mol, register [S4]).

## 6. Page and world changes (file by file)

- `bench-free.html`: band buttons "Sprout · Grades 6–7" and so on; the group is "Grade band".
- `bench-free.css`: Sprout shows the element shelf and plate; it still hides "Add to out", the "out" side and its note.
- `bench-free.js`, in separate blocks: `BAND_NAME` with grades; `eqFor` shows formulas at Sprout with a word equation; the shelf, table and chamber tag show formulas at every band; guide prose formats formulas at every band; `showHow` (Sprout's "Show me how" on a result); an Adept lesson's AP topic.
- `index.html` and `age.js`: "Pick your grade", grade labels and accessible names.
- `bench.js`: chooser labels and question, "Change grade", "at your grade band".
- `pack.js`: formulas show at Sprout too; "for a higher grade band".
- No rendering file is touched (`bench-scene.js`, `voxel_view.rs`, `sky.rs`, `mood.rs`, `look.rs`, `terrain-tex.js`, `*.wesl`).
- Docs: `docs/loop/target.md`, `docs/ANALYTICS.md`, `SECURITY.md`, the dashboard's band labels. The Classic (2D) game in `web/` and its docs (README's tier list, `docs/DESIGN_V2.md`) still use ages, because that game's content was written for them; it is retired (target.md). Older item specs are kept as written, as records.
- `content/science-sources.md`: C-171 to C-174 replace C-152, C-153, C-154 and C-156 (the Sprout quest links' new text); C-157, C-161 and C-165 name their new lessons; the Not-claims rows for "What grade are you in?" and "at your grade band" follow the new text. The register holds no NGSS rows; the NGSS codes live in guide.json and are checked by test/bench-guide.test.ts.

## 7. Tests

- `test/bench-guide.test.ts`: NGSS codes per band follow the grades (Sprout MS-PS1-1/2/5/6 and MS-LS1-6/7; Seedling middle school plus HS-PS1-2/7; Apprentice HS; Adept HS-PS1 plus a known AP topic and source R9). The pre-reader test ("no formulas") is replaced by the grade 6-7 one: Sprout text may name only H2, O2, H2O, CO2 and NaCl, the lessons must use formulas and the words atom, molecule and evidence, there is no "out", type box or scale, and sentences are short. New: Sprout is the simplest band (10 to 16 reactions, at most 16 bottles and fewer than Seedling, no rule reactions, every bottle a formula, no balance challenge); every band's walkthrough reaches a discovery playable and reachable in that band; every live quest has a listed solver playable in every band and one at Sprout. The fixture checks that burning magnesium is locked at Sprout and known at Seedling.
- `crates/chem/src/lib.rs` `every_band_has_a_codex_to_finish`: Sprout has 10 to 16 reactions (it had to have at least 15), Seedling at least 30, Apprentice 90, Adept 400, and each band adds reactions.
- `crates/chem/src/discover.rs` `every_band_can_solve_every_quest_with_findable_bottles` (new): at every band some usable reaction solves every live quest with bottles the valley gives, a listed solver is usable, and one listed solver is Sprout.
- `crates/sim/src/tier.rs` `bands_are_labelled_by_grade` (new).
- `crates/chem` `the_teacher_table_is_current` passes with the regenerated evidence.csv.
