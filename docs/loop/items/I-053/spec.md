# I-053: discoveries become spells, the world supplies bottles, chemistry quests, one save

**Plan:** the approved Chemistry Bench plan, Option B, steps 3 to 6. **Branch:** `loop/bench-spells`, cut from `loop/one-engine` (I-052 lane one-engine, step 1) with `main` merged in. Paawan plays the branch on the play build before it merges (coordinator, 2026-11-04), so the branch is playable end to end: save a Bench find as a spell, pick up a bottle, finish a chemistry quest.

**Graphics consistency:** no new shaders, materials or post-processing. Every new spell reuses an existing spawner (the Fire, Water, Frost or CO2 projectile, trail and impact, `spell_fx.rs`) with its existing parameters. The new look is carried as data (`kind`, `heat`, `strength`, `tint`) that the renderer can style later (section 8).

## 1. Current code (before this branch)

- **Spells are four fixed slots.** `crates/bevy-client/src/chem.rs:85` `enum Spell { Fire, Water, Frost, Foam }`, `Spell::ALL` in hotbar order; `content/spells.json` gives each its text, formula and `tier_min`. Everything downstream indexes by slot 0-3: `game.spell`, the glow materials, `powers.rs`, `Working::routes`, the progress book (`progress.rs`, saved as `rg.progress.v1` by `web/progress.js:10`).
- **Selecting:** `main.rs:1299` (`control`), keys 1-4 set `game.spell` and voxel `hot`; key 5 is the Pick, 6-9 pack slots (`voxel_view.rs`, `story.rs:218` `bind`).
- **Casting:** `main.rs:1928` `aim_and_cast` computes a route per slot (`worked::route`, worked.rs:58: build first while Learning, instant once Practised or Mastered, or when the band cannot learn it), opens the worked builder (`game.work.request`) or casts. In the block world a cast goes to `voxel_cast::Charge::shoot` (voxel_cast.rs:53), flies, and lands as `SpellLanded` (`main.rs:172`). `blockchem::drive` (blockchem.rs:333) turns it into one `matter::Emit` (`on_impact`, blockchem.rs:112) with `amount: 1.0`; `spell_fx.rs` draws the impact; `firebreak`, `late` and `steps` read the same message.
- **Judging a build:** `main.rs:2286` `work_cmd` judges `work submit` with `spells::book().judge` = `realgame_sim::bench::judge_with` (bench.rs:126) and the shared codex (lane one-engine: `crates/chem` implements `ReactionCodex`).
- **Mastery:** run practice (`valley::practice`, valley/mod.rs:755, `PRACTISE_CASTS = 3`) makes a shipped spell Practised (instant) in a run; Mastered is the I-030 two-day rule.
- **The codex:** `crates/chem/src/lib.rs`, `content/chem/core.json`, 182 reviewed reactions with `family`, `band`, `needs`, `heat` (releases, absorbs, little, or null) and `hazard` (fire, lab or none). `content/chem/names.json` lists which substances are gases.
- **The pack:** `valley::Pack` (valley/mod.rs:41), 20 slots; digging puts `story::drop_of(mat)` (story.rs:174) in it (coal ore, wood, sand, snow, iron ore, ...). The Invention Bench already maps a few items to species (`invent::species`, valley/mod.rs:545: coal, wood and charcoal are C, the flask O2, the balloon H2).
- **The Bench page:** `web/bench-free.js` stores its finds in `localStorage['formula.bench.found']` (`store`, bench-free.js:43; written at the Known result) and its band in `formula.bench.band`. Nothing reads them in the world.

## 2. Design in one paragraph

A reaction found on the Bench page gets a **Save as a spell** button (on the result and on every found codex entry). Saved spells live in one shared save, `formula.spells.v1`, that both pages read. In the block world, the pack (Tab) lists them; a **Put on key: 1 2 3 4** picker puts one on any spell key the player chooses (each spell on one key; by default the key of the shipped spell whose rule it runs: fire-type on 1, splash on 2, cold on 3, fizz or puff on 4), and pressing that key again switches between the shipped spell and the discovery. On any key a discovery flies and lands by its own rule (`Disc::fly`; Paawan's playtest 2, "Does this have to be put on key 1?"). A discovered spell is **built twice** (worked cast, judged by the codex against its own equation) and then casts instantly; a Sprout casts it without building, as Fire at Sprout does. Each cast needs a **bottle** of every reactant the air does not supply (coal or charcoal for C, snow for H2O, ...) and uses one up. A cast flies and lands like the shipped spell whose rule it runs, with its emit scaled by its **strength**. One **chemistry quest** runs: melt ice with a discovered reaction that gives off heat.

## 3. Effects: every codex reaction maps to one effect (step 3)

> **Changed by I-056 (B2, `loop/bench-b2`):** `runs` has four more values and **none** now means **Bench only**. A precipitation (kind `wall`) runs **Wall**; a solution reaction with a colour change or a metal coating (kind `reveal`) runs **Reveal**; a fizz that does not smother runs by its gas (`gas_uses`): a flame → Fire, hydrogen → **Lift**, any other gas → **Push**; any reaction with a poisonous product (`poisons`) is Bench only with a reason, and **Frost** needs a reaction that takes in heat by itself (no `needs`). Wall, Reveal, Lift and Push are not matter emits: `world_use.rs` carries them out. No reaction lands as a puff with amount 0 any more. See `docs/loop/items/I-056/spec.md` sections 4 to 6.

> **Changed by I-054 (B1, `loop/bench-show`):** an effect's `strength` and `tint` now come from the reaction's show record: strength from |ΔH| on a log scale, tint from the flame emitter or the first coloured product. See `docs/loop/items/I-054/spec.md` section 6. The strength and tint rules below and in effects.json describe this branch before I-054.

`content/chem/effects.json` + `crates/chem/src/discover.rs` (`Effects::of`). Ordered rules, first match wins, on data only (family, heat, hazard, whether a product is a gas in `names.json`, whether it has needs); one override (`thermite`).

| | kind rule (look) | strength rule (0 to 1) |
|---|---|---|
| 1 | family `combustion` → **fire** | family `combustion` → 1.0 |
| 2 | family `precipitation` → **cloud** | heat releases and hazard fire → 0.8 |
| 3 | heat `absorbs` → **frost** | heat releases → 0.5 |
| 4 | a gas product → **foam** | heat absorbs, no needs (feels cold by itself) → 0.6 |
| 5 | heat releases and hazard fire → **fire** | heat absorbs, driven by heat, light or electricity → 0.3 |
| 6 | heat releases → **glow** | anything else (little or unknown heat) → 0.4 |
| 7 | anything else → **water** | |

`heat` is the codex's heat as -1 (absorbs), 0 (little or unknown) or +1 (releases). Strength is principled by heat class: a burning is the shipped Fire (1.0); nothing discovered is stronger than a shipped spell.

**runs** (the world rule it reuses): fire → Fire, glow → Fire, frost → Frost, water → Water, cloud → Water (a precipitate forms in a solution: it splashes), foam → CO2 only when every gas it gives smothers (`smothers`: CO2, N2), else **none** (a hydrogen or oxygen fizz must not put out a fire; it lands as a puff with emit amount 0). A glow from a solution family (`aqueous_families`: neutralisation, displacement, metal and carbonate with acid, precipitation, halogen) runs Water (warm liquid, not a flame). The emit's `amount` is the strength (0 for none); `matter` scales Fire's heat per step and CO2 by `amount` (Water and Frost already did). `amount` 1.0 is unchanged behaviour.

**tint** (optional sRGB hex): for an effect that runs Fire, the flame colour of a reactant that is a bare metal element (Li, Na, K, Ca, Sr, Ba, Mg; flame tests); otherwise the colour of the first product that is a coloured precipitate or gas (AgCl, PbI2, Cu(OH)2, Fe(OH)3, Cl2, NO2, ...). Sources are in the file; `check: draft` until the science review.

**Totals over the 182 reactions:** fire 43, glow 51 (24 run Fire, 27 run Water), frost 30, foam 41 (10 run CO2, 31 are puffs), cloud 14, water 3.

The full table is in section 11 (generated from the Rust, so it is exactly what the game does).

## 4. Mastery (step 3)

`discover::MASTERY_BUILDS = 2`: a discovered spell opens the formula builder until it has been built correctly twice, then casts instantly. The build is judged by the same judge as the shipped spells (`judge_with` with the codex, against `discover::spell_def`, whose formula is the codex equation and whose `tier_min` is the reaction's band, at least Seedling). A Sprout never builds (its builder makes single substances only) and casts its discoveries instantly, like Fire at Sprout. A saved reaction above the player's band does not cast ("That reaction is for an older band."). Builds of a discovery never count toward a shipped spell's practice or progress. The builder's "Build it N more times" uses the discovery's own count (`__workState.disc`).

Note: the shipped spells' run practice is 3 builds (`PRACTISE_CASTS`); the brief says "twice". Discoveries use 2 as briefed; the shipped spells are unchanged.

## 5. Bottles: the world supplies them (step 4)

`content/chem/bottles.json`: the air supplies O2 and N2 free; pack items supply the rest.

| item | species | used up | why |
|---|---|---|---|
| coal_ore | C | yes | coal is mostly carbon |
| charcoal | C | yes | charcoal is mostly carbon |
| snow | H2O | yes | snow is frozen water |
| iron | Fe | yes | an iron block |
| iron_ore | Fe2O3 | yes | the valley's iron ore is haematite |
| sand | SiO2 | yes | sand is mostly silica |
| o2_flask | O2 | no | a cast takes a little |
| h2_balloon | H2 | no | a cast takes a little |

`Bottles::plan` says what a cast needs and takes; a cast with a reactant missing does not fire and says which items it needs and where to get them (each item's `find` line, in plain words; the pack shows the same per saved spell). Bottles are pack items. **End to end now:** the charcoal clamp (cast Fire into the mound's vent, then E) gives charcoal, and coal also works, which is the C bottle for **Carbon burns** (C + O2 → CO2, Sprout band, fire, strength 1), the reaction the live quest is built around. The pack shows the bottles held, with their formula (not for Sprout). Free play (the Bench page) keeps unlimited bottles.

Reactions the world can supply today: hydrogen_burns, water_splits, carbon_burns, iron_rusts, carbon_little_air, haber, lightning_nitrogen, water_gas, ozone_forms.

**Specified, not built:** new materials and pickups: limestone (CaCO3; makes limestone_roast, chalk_acid with an acid), salt water (NaCl + H2O at the shore), vinegar or an acid spring (HCl/CH3COOH for the carbonate and metal reactions), copper ore near the Kiln (Cu, CuO); a Bench-side "bottle shelf" in the in-world Formula Bench showing the same `bottles` state.

## 6. Chemistry quests (step 5)

`content/chem/quests.json`; a quest completes when a discovered spell whose effect matches `needs` lands and the world reports `event` within `radius_m` of the landing within 2.5 s.

- **Live: Melt ice with a reaction** (`melt_with_reaction`): needs heat +1 and runs Fire; event `melted` (`MatterEvent::Melted`) within 3 m. Base Fire does not count; a fizz or a cold reaction does not count. Done: caption C-115, a ✓ in the pack, saved in `formula.spells.v1`. Shown in the pack (C-113, Sprout C-114).
- Specified: **Smother a fire with a reaction's gas** (`smother_with_co2`: runs CO2, e.g. chalk + acid; event a fire out by low O2, 4 m).
- Specified: **Cool a hot steam vent with a reaction that takes in heat** (`cool_the_vent`: heat -1; event a vent cooled).
- Specified: **Make a cloud in the well** (`cloud_in_the_well`: kind cloud, a precipitation; event landed in water, 2 m).

## 7. One save (step 6)

`crates/bevy-client/web/formula-save.js` is the only code that reads or writes the shared keys; the Bench page and the world both import it.

| key | written by | read by | content |
|---|---|---|---|
| `formula.bench.found` | Bench page (finds); world adds `methane_burns` once Fire is learned there | both | JSON array of codex ids |
| `formula.spells.v1` | Bench page (Save as a spell); world (`__discState.save` when its `rev` grows, merged with what is stored) | both | `{v:1, saved:{id: builds}, slots:[id\|null x4], quests:[id]}` |
| `rg.progress.v1` | world (unchanged) | world | shipped spells' progress |
| `formula.bench.band` | Bench page (unchanged) | Bench page | not shared: the world asks the age band each visit |

The world loads the save into the game with `found`, `saved`, `slots`, `quests` commands on `window.__gameIn.disc`, and again when the Bench page writes in another tab (`storage` event). A load never lowers a build count; merges keep the larger count and every saved spell.

## 8. What the graphics thread needs to know

> **Changed by I-056:** two more kinds (`wall`, `reveal`) and four more runs (`wall`, `reveal`, `lift`, `push`); the effect value also carries `use` (what it does in the world, plain words) and `bench_only` (why it cannot be saved, or null). A Wall flies as Frost, a Reveal as Fire, a Lift as Water and a Push as CO2 (`Runs::slot`). The data values for walls and reveals are in I-056 spec section 6.

> **Changed by I-054:** `tint` and `strength` are computed from the show record (`show_json(id)`), which the Bench now also passes to `playReaction` as a 4th argument. See I-054 spec section 4.

- **The effect value** (`realgame_chem::discover::Effect`, serialised in `effects_json()` for the Bench page and in `__discState.saved[].effect`): `reaction_id` (codex id), `kind` (fire, water, frost, foam, cloud, glow), `heat` (-1, 0, +1), `strength` (0 to 1), optional `tint` (`#rrggbb`), and `runs` (the gameplay rule). Hit position and normal come from the cast (`SpellLanded.pos`, `.normal`).
- **How to find it at the impact:** `SpellLanded` has two new fields: `amount` (the emit scale) and `disc: Option<u16>`, the codex index of a discovered spell. `crate::discover::entry(i)` returns `(Entry, Effect)`. Shipped spells have `disc: None`, `amount: 1.0`.
- **Today:** a discovered spell is drawn by the spawner of the shipped spell on its slot, unchanged: kind fire and glow-that-runs-Fire use Fire's; cloud and glow-that-runs-Water use Water's; frost Frost's; foam (both CO2 and puffs) CO2's. No new shader, material or post-processing; `spell_fx.rs`, `fire_fx.rs`, `voxel_cast.rs` visuals are untouched.
- **Later (renderer's choice):** style by `kind` (cloud: a white or tinted settling cloud; glow: a warm glow without flame), use `tint` for the impact and trail colour, scale particle counts by `strength`, and use `heat` for warm or cold colour grading. Every value is in data; no gameplay change is needed.
- Hotbar: a slot with a discovery on gets the class `disc-slot` and `data-kind` (HUD CSS outline only).

## 9. Files

- New: `content/chem/effects.json`, `bottles.json`, `quests.json`; `crates/chem/src/discover.rs` (+ `effects_json` in `web.rs`, `pub use serde_json`); `crates/bevy-client/src/discover.rs`; `web/formula-save.js`, `web/discover.js`; `test/bench-spells.test.ts`.
- Changed: `crates/matter/src/lib.rs` (Fire and CO2 scale with `amount`); `crates/bevy-client/src/main.rs` (SpellLanded fields, key switch, routes, bottles, worked builds, `__workState.disc`), `voxel_cast.rs` (cast tag), `blockchem.rs` (amount, puffs); `web/pack.js` (Spells from the Bench, bottles, quest), `web/worked.js` (discovery build count), `web/bench-free.js` (Save as a spell), `web/boot.js`, `web/style.css`, `build-web.sh`; `content/science-sources.md` (C-113 to C-115 and Not-claims rows).

## 10. Acceptance tests

- `crates/chem/src/discover.rs`: `every_codex_reaction_maps_to_an_effect` (kind, heat, strength in 0..1, reaction_id, tint format, heat agrees with the codex); `the_effect_data_is_consistent`; `effects_follow_the_chemistry` (methane fire 1.0; chalk + acid foam runs CO2; zinc + acid foam runs none; silver chloride a white cloud; magnesium a white flame; a neutralisation glows and splashes; thermite fire); `bottles_name_codex_substances_and_supply_carbon_burning` (every live quest has a Sprout reaction the world supplies); `a_discovered_spell_is_judged_by_the_codex` (every one of the 182 equations is Ok against its own spell; wrong products fail; a higher band is NotForBand); `mastery_is_two_builds_and_sprout_never_builds`.
- `crates/bevy-client/src/discover.rs`: the save loads and drops unknown ids; equip puts a spell on its rule's slot and the key switches; two builds master it and a puff's amount is 0 (I-056: a hydrogen fizz now lifts, with its strength, and a Bench-only reaction is never saved or equipped); a cast needs its bottles and uses them; the melt quest needs a hot discovered spell nearby and in time.
- `test/bench-spells.test.ts`: the shared keys, saving and merging, corrupt or blocked storage, load commands, the world's sync (load, write back, `storage` reload, Fire becomes a find), content shape.
- Playable check (Playwright on the release build, section 12 of the report): save on the Bench page, the pack lists it, dig coal, cast twice with builds, then instantly, melt ice, quest done.

## 11. The mapping table (all 182 codex reactions)

> **Stale after I-056:** the `kind` and `runs` columns below are pre-I-056 for precipitations, reveals and the 31 former puffs. The current table is in `docs/loop/items/I-056/spec.md` section 10.

> **Stale after I-054:** the `strength` and `tint` columns below are the pre-I-054 values. The current ones are in `docs/loop/items/I-054/spec.md` section 10.

`world` = every reactant can come from the air or a bottle item today.

| reaction | family | band | kind | heat | strength | runs | tint | world |
|---|---|---|---|---|---|---|---|---|
| `methane_burns` | combustion | sprout | fire | +1 | 1 | fire |  |  |
| `hydrogen_burns` | combustion | sprout | fire | +1 | 1 | fire |  | yes |
| `water_splits` | electrolysis | seedling | frost | -1 | 0.3 | frost |  | yes |
| `carbon_burns` | combustion | sprout | fire | +1 | 1 | fire |  | yes |
| `iron_rusts` | oxidation | sprout | glow | +1 | 0.5 | fire |  | yes |
| `baking_soda_vinegar` | carbonate_acid | sprout | frost | -1 | 0.6 | frost |  |  |
| `photosynthesis` | life | sprout | frost | -1 | 0.3 | frost |  |  |
| `respiration` | life | sprout | foam | +1 | 0.5 | foam |  |  |
| `candle_burns` | combustion | sprout | fire | +1 | 1 | fire |  |  |
| `peroxide_splits` | decomposition | sprout | foam | +1 | 0.5 | none |  |  |
| `magnesium_burns` | combustion | sprout | fire | +1 | 1 | fire | #f5f7ff |  |
| `propane_burns` | combustion | sprout | fire | +1 | 1 | fire |  |  |
| `ethanol_burns` | combustion | seedling | fire | +1 | 1 | fire |  |  |
| `fermentation` | life | sprout | foam | +1 | 0.5 | foam |  |  |
| `limewater_test` | precipitation | seedling | cloud | +1 | 0.5 | water | #f4f4f4 |  |
| `sodium_water` | water_metal | seedling | foam | +1 | 0.5 | none |  |  |
| `lithium_water` | water_metal | seedling | foam | +1 | 0.5 | none |  |  |
| `potassium_water` | water_metal | apprentice | foam | +1 | 0.5 | none |  |  |
| `calcium_water` | water_metal | seedling | foam | +1 | 0.5 | none |  |  |
| `barium_water` | water_metal | adept | foam | +1 | 0.5 | none |  |  |
| `strontium_water` | water_metal | adept | foam | +1 | 0.5 | none |  |  |
| `magnesium_steam` | water_metal | apprentice | foam | +1 | 0.5 | none |  |  |
| `salt_from_elements` | synthesis | seedling | glow | +1 | 0.5 | fire | #ffb21e |  |
| `potassium_chlorine` | synthesis | apprentice | glow | +1 | 0.5 | fire | #b58fd8 |  |
| `magnesium_chlorine` | synthesis | apprentice | glow | +1 | 0.5 | fire | #f5f7ff |  |
| `calcium_chlorine` | synthesis | apprentice | glow | +1 | 0.5 | fire | #ff6b2b |  |
| `aluminium_chlorine` | synthesis | adept | glow | +1 | 0.5 | fire |  |  |
| `copper_chlorine` | synthesis | apprentice | glow | +1 | 0.5 | fire |  |  |
| `sodium_bromine` | synthesis | adept | glow | +1 | 0.5 | fire | #ffb21e |  |
| `iron_sulfur` | synthesis | seedling | fire | +1 | 0.8 | fire |  |  |
| `magnesium_sulfur` | synthesis | seedling | glow | +1 | 0.5 | fire | #f5f7ff |  |
| `copper_oxide` | oxidation | seedling | fire | +1 | 0.8 | fire |  |  |
| `zinc_oxide` | oxidation | apprentice | fire | +1 | 0.8 | fire |  |  |
| `aluminium_oxide` | oxidation | apprentice | fire | +1 | 0.8 | fire |  |  |
| `lithium_oxygen` | oxidation | apprentice | glow | +1 | 0.5 | fire | #d6213a |  |
| `sodium_oxygen` | oxidation | seedling | glow | +1 | 0.5 | fire | #ffb21e |  |
| `calcium_oxygen` | oxidation | apprentice | glow | +1 | 0.5 | fire | #ff6b2b |  |
| `silicon_oxygen` | oxidation | adept | fire | +1 | 0.8 | fire |  |  |
| `sulfur_burns` | combustion | sprout | fire | +1 | 1 | fire |  |  |
| `phosphorus_burns` | combustion | apprentice | fire | +1 | 1 | fire |  |  |
| `co_burns` | combustion | apprentice | fire | +1 | 1 | fire |  |  |
| `carbon_little_air` | combustion | apprentice | fire | +1 | 1 | fire |  | yes |
| `ethane_burns` | combustion | apprentice | fire | +1 | 1 | fire |  |  |
| `butane_burns` | combustion | sprout | fire | +1 | 1 | fire |  |  |
| `pentane_burns` | combustion | apprentice | fire | +1 | 1 | fire |  |  |
| `hexane_burns` | combustion | apprentice | fire | +1 | 1 | fire |  |  |
| `octane_burns` | combustion | apprentice | fire | +1 | 1 | fire |  |  |
| `ethene_burns` | combustion | apprentice | fire | +1 | 1 | fire |  |  |
| `acetylene_burns` | combustion | apprentice | fire | +1 | 1 | fire |  |  |
| `benzene_burns` | combustion | adept | fire | +1 | 1 | fire |  |  |
| `methanol_burns` | combustion | apprentice | fire | +1 | 1 | fire |  |  |
| `propanol_burns` | combustion | adept | fire | +1 | 1 | fire |  |  |
| `ammonia_burns` | combustion | adept | fire | +1 | 1 | fire |  |  |
| `hydrogen_sulfide_burns` | combustion | adept | fire | +1 | 1 | fire |  |  |
| `carbon_disulfide_burns` | combustion | adept | fire | +1 | 1 | fire |  |  |
| `silane_burns` | combustion | adept | fire | +1 | 1 | fire |  |  |
| `haber` | industrial | apprentice | foam | +1 | 0.5 | none |  | yes |
| `ammonia_splits` | decomposition | adept | frost | -1 | 0.3 | frost |  |  |
| `hcl_synthesis` | synthesis | apprentice | foam | +1 | 0.5 | none |  |  |
| `hbr_synthesis` | synthesis | adept | foam | +1 | 0.5 | none |  |  |
| `hf_synthesis` | synthesis | adept | foam | +1 | 0.5 | none |  |  |
| `contact_so3` | industrial | adept | glow | +1 | 0.5 | fire |  |  |
| `so3_water` | industrial | adept | glow | +1 | 0.5 | fire |  |  |
| `ostwald_1` | industrial | adept | foam | +1 | 0.5 | none |  |  |
| `no_oxidises` | oxidation | adept | foam | +1 | 0.8 | none | #a2401c |  |
| `ostwald_3` | industrial | adept | foam | +1 | 0.5 | none |  |  |
| `lightning_nitrogen` | synthesis | apprentice | frost | -1 | 0.3 | frost |  | yes |
| `steam_reforming` | industrial | adept | frost | -1 | 0.3 | frost |  |  |
| `water_gas_shift` | industrial | adept | foam | +1 | 0.5 | none |  |  |
| `water_gas` | industrial | adept | frost | -1 | 0.3 | frost |  | yes |
| `boudouard` | redox | adept | frost | -1 | 0.3 | frost |  |  |
| `methanol_synthesis` | industrial | adept | glow | +1 | 0.5 | fire |  |  |
| `urea_synthesis` | industrial | adept | glow | +1 | 0.5 | fire |  |  |
| `limestone_roast` | thermal | sprout | frost | -1 | 0.3 | frost |  |  |
| `slaking_lime` | synthesis | sprout | fire | +1 | 0.8 | fire |  |  |
| `lime_dries` | thermal | adept | frost | -1 | 0.3 | frost |  |  |
| `copper_carbonate_heat` | thermal | apprentice | frost | -1 | 0.3 | frost |  |  |
| `magnesium_carbonate_heat` | thermal | apprentice | frost | -1 | 0.3 | frost |  |  |
| `baking_soda_heat` | thermal | seedling | frost | -1 | 0.3 | frost |  |  |
| `potassium_chlorate_heat` | thermal | adept | foam | +1 | 0.8 | none |  |  |
| `mercury_oxide_heat` | thermal | adept | frost | -1 | 0.3 | frost |  |  |
| `silver_oxide_heat` | thermal | adept | frost | -1 | 0.3 | frost |  |  |
| `copper_nitrate_heat` | thermal | adept | frost | -1 | 0.3 | frost | #a2401c |  |
| `ammonium_chloride_heat` | thermal | apprentice | frost | -1 | 0.3 | frost |  |  |
| `lab_ammonia` | gas_prep | adept | frost | -1 | 0.3 | frost |  |  |
| `salt_electrolysis` | electrolysis | apprentice | frost | -1 | 0.3 | frost | #c8d84a |  |
| `brine_electrolysis` | electrolysis | adept | frost | -1 | 0.3 | frost | #c8d84a |  |
| `alumina_electrolysis` | electrolysis | adept | frost | -1 | 0.3 | frost |  |  |
| `copper_chloride_electrolysis` | electrolysis | apprentice | frost | -1 | 0.3 | frost | #c8d84a |  |
| `hcl_electrolysis` | electrolysis | adept | frost | -1 | 0.3 | frost | #c8d84a |  |
| `neutralise_hcl_naoh` | neutralisation | seedling | glow | +1 | 0.5 | water |  |  |
| `antacid` | neutralisation | seedling | glow | +1 | 0.5 | water |  |  |
| `neutralise_hcl_koh` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `neutralise_hcl_lioh` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `neutralise_h2so4_naoh` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `neutralise_h2so4_koh` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `neutralise_hno3_naoh` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `neutralise_hno3_koh` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `neutralise_ammonia_hcl` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `neutralise_caoh2_hcl` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `neutralise_vinegar_naoh` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `neutralise_ammonia_h2so4` | neutralisation | adept | glow | +1 | 0.5 | water |  |  |
| `neutralise_ammonia_hno3` | neutralisation | adept | glow | +1 | 0.5 | water |  |  |
| `neutralise_baoh2_h2so4` | precipitation | adept | cloud | +1 | 0.5 | water | #f4f4f4 |  |
| `co2_naoh` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `cuo_acid` | neutralisation | apprentice | fire | +1 | 0.8 | fire |  |  |
| `mgo_acid` | neutralisation | apprentice | glow | +1 | 0.5 | water |  |  |
| `zinc_acid` | metal_acid | seedling | foam | +1 | 0.5 | none |  |  |
| `magnesium_acid` | metal_acid | seedling | foam | +1 | 0.5 | none |  |  |
| `iron_acid` | metal_acid | apprentice | foam | +1 | 0.5 | none |  |  |
| `aluminium_acid` | metal_acid | apprentice | foam | +1 | 0.5 | none |  |  |
| `calcium_acid` | metal_acid | apprentice | foam | +1 | 0.5 | none |  |  |
| `zinc_sulfuric` | metal_acid | apprentice | foam | +1 | 0.5 | none |  |  |
| `magnesium_sulfuric` | metal_acid | apprentice | foam | +1 | 0.5 | none |  |  |
| `iron_sulfuric` | metal_acid | apprentice | foam | +1 | 0.5 | none |  |  |
| `copper_nitric` | redox | adept | foam | +1 | 0.8 | none |  |  |
| `chalk_acid` | carbonate_acid | sprout | foam | +1 | 0.5 | foam |  |  |
| `washing_soda_acid` | carbonate_acid | seedling | foam | +1 | 0.5 | foam |  |  |
| `baking_soda_hcl` | carbonate_acid | apprentice | frost | -1 | 0.6 | frost |  |  |
| `copper_carbonate_acid` | carbonate_acid | apprentice | foam | +1 | 0.5 | foam |  |  |
| `marble_sulfuric` | carbonate_acid | adept | foam | +0 | 0.4 | foam |  |  |
| `magnesium_carbonate_acid` | carbonate_acid | apprentice | foam | +1 | 0.5 | foam |  |  |
| `iron_copper_sulfate` | displacement | sprout | glow | +1 | 0.5 | water |  |  |
| `zinc_copper_sulfate` | displacement | seedling | glow | +1 | 0.5 | water |  |  |
| `magnesium_copper_sulfate` | displacement | apprentice | glow | +1 | 0.5 | water |  |  |
| `aluminium_copper_sulfate` | displacement | adept | glow | +1 | 0.5 | water |  |  |
| `magnesium_zinc_sulfate` | displacement | apprentice | glow | +1 | 0.5 | water |  |  |
| `zinc_lead_nitrate` | displacement | adept | glow | +1 | 0.5 | water |  |  |
| `copper_silver_nitrate` | displacement | seedling | glow | +1 | 0.5 | water |  |  |
| `zinc_silver_nitrate` | displacement | apprentice | glow | +1 | 0.5 | water |  |  |
| `thermite` | displacement | apprentice | fire | +1 | 1 | fire |  |  |
| `blast_furnace` | redox | apprentice | foam | +1 | 0.5 | foam |  |  |
| `copper_oxide_carbon` | redox | apprentice | foam | +1 | 0.8 | foam |  |  |
| `copper_oxide_hydrogen` | redox | adept | fire | +1 | 0.8 | fire |  |  |
| `zinc_oxide_carbon` | redox | adept | frost | -1 | 0.3 | frost |  |  |
| `magnesium_co2` | redox | adept | fire | +1 | 0.8 | fire | #f5f7ff |  |
| `peroxide_iodide` | redox | adept | fire | +1 | 0.8 | fire |  |  |
| `chlorine_bromide` | halogen | apprentice | glow | +1 | 0.5 | water |  |  |
| `chlorine_iodide` | halogen | apprentice | glow | +1 | 0.5 | water |  |  |
| `bromine_iodide` | halogen | apprentice | glow | +1 | 0.5 | water |  |  |
| `bleach_making` | halogen | adept | glow | +1 | 0.5 | water |  |  |
| `iron_chlorine` | synthesis | apprentice | glow | +1 | 0.5 | fire |  |  |
| `aluminium_iodine` | synthesis | adept | glow | +1 | 0.5 | fire |  |  |
| `zinc_iodine` | synthesis | adept | glow | +1 | 0.5 | fire |  |  |
| `magnesium_nitrogen` | synthesis | adept | fire | +1 | 0.8 | fire | #f5f7ff |  |
| `lithium_nitrogen` | synthesis | adept | glow | +1 | 0.5 | fire | #d6213a |  |
| `copper_sulfur` | synthesis | apprentice | fire | +1 | 0.8 | fire |  |  |
| `zinc_sulfur` | synthesis | apprentice | fire | +1 | 0.8 | fire |  |  |
| `hydrogen_sulfur` | synthesis | adept | foam | +1 | 0.8 | none |  |  |
| `sodium_hydride` | synthesis | adept | glow | +1 | 0.5 | fire | #ffb21e |  |
| `silver_tarnish` | oxidation | apprentice | glow | +1 | 0.5 | fire |  |  |
| `silver_chloride` | precipitation | sprout | cloud | +1 | 0.5 | water | #f4f4f4 |  |
| `silver_bromide` | precipitation | apprentice | cloud | +1 | 0.5 | water | #f1e6b0 |  |
| `silver_iodide` | precipitation | apprentice | cloud | +1 | 0.5 | water | #f2df73 |  |
| `golden_rain` | precipitation | apprentice | cloud | +0 | 0.4 | water | #ffd21f |  |
| `barium_sulfate` | precipitation | apprentice | cloud | +1 | 0.5 | water | #f4f4f4 |  |
| `copper_hydroxide` | precipitation | apprentice | cloud | +0 | 0.4 | water | #3d8fe0 |  |
| `iron2_hydroxide` | precipitation | adept | cloud | +0 | 0.4 | water | #5b7f4f |  |
| `iron3_hydroxide` | precipitation | adept | cloud | +0 | 0.4 | water | #a0522d |  |
| `calcium_carbonate_ppt` | precipitation | apprentice | cloud | -1 | 0.6 | water | #f4f4f4 |  |
| `magnesium_hydroxide_ppt` | precipitation | adept | cloud | +0 | 0.4 | water | #f4f4f4 |  |
| `lead_chloride` | precipitation | adept | cloud | +0 | 0.4 | water | #f4f4f4 |  |
| `zinc_hydroxide` | precipitation | adept | cloud | +0 | 0.4 | water | #f4f4f4 |  |
| `co2_water` | synthesis | seedling | water | +0 | 0.4 | water |  |  |
| `so2_water` | synthesis | adept | water | +0 | 0.4 | water |  |  |
| `ammonia_water` | synthesis | adept | fire | +1 | 0.8 | fire |  |  |
| `sodium_oxide_water` | synthesis | adept | fire | +1 | 0.8 | fire |  |  |
| `p4o10_water` | synthesis | adept | fire | +1 | 0.8 | fire |  |  |
| `ethene_hydrogen` | organic | adept | foam | +1 | 0.5 | none |  |  |
| `ethene_bromine` | organic | adept | glow | +1 | 0.5 | fire |  |  |
| `ethene_water` | organic | adept | fire | +1 | 0.8 | fire |  |  |
| `esterification` | organic | adept | water | +0 | 0.4 | water |  |  |
| `methane_chlorine` | organic | adept | foam | +1 | 0.5 | none |  |  |
| `ethanol_dehydration` | organic | adept | frost | -1 | 0.3 | frost |  |  |
| `cracking` | organic | adept | frost | -1 | 0.3 | frost |  |  |
| `sucrose_hydrolysis` | life | adept | glow | +1 | 0.5 | fire |  |  |
| `ozone_forms` | synthesis | adept | frost | -1 | 0.3 | frost |  | yes |
| `ozone_splits` | decomposition | adept | foam | +1 | 0.5 | none |  |  |
| `nitrogen_dioxide_dimer` | synthesis | adept | foam | +1 | 0.8 | none |  |  |
| `glass_making` | industrial | adept | frost | -1 | 0.3 | frost |  |  |
| `cement_clinker` | industrial | adept | frost | -1 | 0.3 | frost |  |  |
| `nitrogen_triiodide` | decomposition | adept | foam | +1 | 0.5 | foam |  |  |
