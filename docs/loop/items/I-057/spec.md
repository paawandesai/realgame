# I-057: a chemistry quest for every world use, bottles you can find, Guide links (B3)

Phase B3 of the plan "Formula: free-play Bench to spells" (artifact ARwyByPYQG5Lp6e32JGc1a). Branch `loop/bench-b3`, from `loop/bench-play` (B1 I-054 and B2 I-056 merged). Item number: I-057 was free (`docs/loop/items/` ran to I-056).

**Graphics consistency:** no shader, material, mesh or post-processing change; none of `voxel_view.rs`, `sky.rs`, `mood.rs`, `look.rs`, `terrain-tex.js`, `*.wesl`, `bench-scene.js` is touched. Quest props reuse existing blocks (glass, runestone, ash) and the existing beacon. Section 7 lists what the graphics thread may style later, as data values.

## 1. What changes for the player

- Five chemistry quests run in the block world, one per world use: melt ice (Fire, live since I-053), **smother** the marsh gas fire (Foam: a CO2-only gas), **cool** the charcoal mound's hot vent (Frost: a reaction that takes in heat by itself), build a **wall** in the river at the ford (Wall: a precipitate), and find the **hidden mark** (Reveal: a colour change or metal coating). The pack (Tab) lists each with its objective and, until it is done, a hint that names the kind of reaction it needs, never the answer.
- Each finishes only when a **discovered** spell whose effect matches lands near the world event that proves it. A shipped spell, or good aim with the wrong reaction, never finishes one.
- **Bottle stashes**: the reactants those reactions use (chalk, vinegar, baking soda, salt, copper sulfate, silver nitrate, iron) are picked up by walking up to a stash in the valley. No digging, no debug `give`. A stash fills again after 60 s. The hidden mark is a stash too: it shows only after a Reveal spell lands near it.
- Every bottle item in `content/chem/bottles.json` has a plain-words `find` line and a `from` (stash, clamp, rig, pick, or nowhere).
- The Guide's lessons point at the quest that uses them ("Try it in the world: ..."), at every band, Sprout included.

## 2. The quest table

`content/chem/quests.json`. A quest is done when a discovered spell whose effect meets `needs` lands, and the world reports `event` within `radius_m` of that landing and within `window_s` seconds, inside the quest's place (`at`, `near_m`) when it has one.

| quest (`id`) | place (existing location, props) | objective (Seedling+) | needs | event (who reports it) | radius, window, place | solving reactions (`solved_by`) | bottles and where they are found |
|---|---|---|---|---|---|---|---|
| `melt_with_reaction` | any ice, e.g. the ice gate (Frozen Fields, `stamp::ICE_GATE`) | Chemistry quest: melt some ice with a reaction from the Bench that gives off heat. | heat +1, runs fire | `melted`: `MatterEvent::Melted` | 3 m, 2.5 s | carbon_burns, hydrogen_burns, methane_burns, magnesium_burns | carbon_burns: charcoal (the clamp: Fire in the mound's vent, then E), O2 from the air |
| `smother_with_co2` | the gas fire in the marsh hollow (`gen::HOLLOW_VENTS`, `stamp::HOLLOW`) | Chemistry quest: put out the gas fire in the marsh hollow with a reaction that makes carbon dioxide. | runs foam (CO2/N2 only) | `smothered`: `MatterEvent::Smothered` or `Extinguished { by: NoOxygen }` | 6 m, 20 s, within 10 m of (191.5, 28, 100.5) | limestone_roast, chalk_acid, rule_carbonate_acid_caco3_ch3cooh, washing_soda_acid | limestone_roast: chalk (stash `marsh_chalk`, the edge of the marsh hollow). caco3 + vinegar: chalk and vinegar (stash `kitchen`) |
| `cool_the_vent` | the vent on the charcoal mound, Frozen Fields (`late::CLAMP`, the clamp's vent) | Chemistry quest: light the charcoal mound, then cool its hot vent with a reaction that takes in heat. | heat -1, runs frost | `cooled`: a Frost landing (shipped or discovered) in the vent while it is hot (`Late::vent_hot`: charring, or charred with the charcoal still inside), reported by `discover::quest` | 4 m, 2.5 s, within 4 m of the vent | baking_soda_vinegar, baking_soda_hcl | baking_soda_vinegar: baking soda and vinegar (stash `kitchen`, the shelf by the house next to the well) |
| `wall_in_the_river` | the river at the ford (`stamp::FORD`, P2a) | Chemistry quest: build a wall in the river at the ford with a reaction that makes a solid. | runs wall | `walled_water`: a Wall that took the place of water, at its cells' middle (`world_use::run`) | 6 m, 2.5 s, within 10 m of (128, 29, 154) | silver_chloride, limewater_test, calcium_carbonate_ppt, copper_hydroxide | silver_chloride: silver nitrate (stash `bench_shelf`, beside the Formula Bench) and salt (stash `kitchen`) |
| `reveal_the_hidden_mark` | the hidden mark west of the village (stash `hidden_mark`, a runestone at (100, 35, 200)), or any unfound cache | Chemistry quest: find the hidden mark near the village with a reaction that changes colour. | runs reveal | `revealed`: a Reveal that showed a hidden cache or the hidden mark, at the landing (`world_use::run`) | 3 m, 2.5 s | iron_copper_sulfate, zinc_copper_sulfate, copper_silver_nitrate, magnesium_copper_sulfate | iron_copper_sulfate: iron nails and copper sulfate (stash `workshop`, by the workshop) |

`cloud_in_the_well` stays specified only (`live: false`): since I-056 no reaction has kind `cloud` (a precipitate walls).

Sprout wording, hints and done lines are in the file (`line_sprout`, `hint`, `hint_sprout`, `done`). Every live quest has at least one **Sprout** reaction whose bottles can all be found (tested).

## 3. Bottle stashes and sources

`content/chem/bottles.json`: `items[].from`, `stashes`, `refill_s` (60 s). New pack items are `valley::Item::Bottle(i)`, `BOTTLE_ITEMS` in `crates/sim/src/valley/mod.rs:33` (key and label; the species and find line stay in the data).

| item | species | from | find (shown in the pack and the missing-bottle caption) |
|---|---|---|---|
| charcoal | C | clamp | Heat wood with very little air: cast Fire into the vent on the charcoal mound (Frozen Fields). When the smoke clears, press E there. |
| coal_ore | C | nowhere | Coal works just like charcoal, if you already have some. |
| snow | H2O | pick | Snow covers the Frozen Fields. Point the Pick (key 5) at snowy ground and hold the mouse to scoop some up. |
| iron | Fe | stash | A box of iron nails sits by the workshop in the village. Walk up to it to take some. |
| iron_ore | Fe2O3 | pick | A few rusty brown blocks hide in the rock under the charcoal mound. The Pick (key 5) can pick one up. |
| sand | SiO2 | pick | Sand lines the river bed at the ford. Point the Pick (key 5) at it and hold the mouse to scoop some up. |
| o2_flask, h2_balloon | O2, H2 | rig | Run the water-splitting rig in the cave (press E there). |
| chalk | CaCO3 | stash | White chalk lumps lie at the edge of the marsh hollow, near the gas fire. Walk up to them to take some. |
| vinegar | CH3COOH | stash | Vinegar, baking soda and salt sit on a shelf by the house next to the well. Walk up to it to take some. |
| baking_soda | NaHCO3 | stash | Baking soda sits on the shelf by the house next to the well, with the vinegar. Walk up to it to take some. |
| salt | NaCl | stash | Salt sits on the shelf by the house next to the well, with the vinegar. Walk up to it to take some. |
| copper_sulfate | CuSO4 | stash | A jar of blue copper sulfate sits by the workshop in the village, next to the iron nails. |
| silver_nitrate | AgNO3 | stash | A small dark bottle of silver nitrate stands beside the Formula Bench. It stains skin: only grown-ups use the real one. |

| stash | feet cell | marker block | gives per visit |
|---|---|---|---|
| `kitchen` (the shelf by the house next to the well) | (108, 35, 209) | glass | 4 vinegar, 4 baking soda, 4 salt |
| `workshop` (by the workshop) | (136, 35, 190) | glass | 4 copper sulfate, 4 iron |
| `bench_shelf` (beside the Formula Bench) | (122, 35, 203) | glass | 4 silver nitrate |
| `marsh_chalk` (the edge of the marsh hollow) | (186, 33, 109) | glass | 6 chalk |
| `hidden_mark` (hidden; the hidden mark west of the village) | (100, 35, 200) | runestone, set when a Reveal shows it | 3 copper sulfate, 3 iron, 3 silver nitrate, 3 chalk |

Every stash cell is open ground outside protected stamps in the generated valley (tested). Free play (the Bench page) still has unlimited bottles.

## 4. Code (file:line)

- `crates/chem/src/discover.rs`: the quest events `EVENT_*` and `EVENTS` (:39-55); `Source` (:629), `Stash` (:648), `Bottles::stashes` and `refill_s` (:673); `Bottles::findable` (:732) and `findable_plan` (:745); `ChemQuest` gains `window_s`, `place`, `at`, `near_m`, `line`, `line_sprout`, `hint`, `hint_sprout`, `done`, `solved_by` (:778) and `in_place` (:823).
- `crates/sim/src/valley/mod.rs`: `Item::Bottle(u8)` (:28) and `BOTTLE_ITEMS` (:33); `key`, `label`, `from_key`, `stack_max`.
- `crates/bevy-client/src/discover.rs`: `Disc::landed` (:437) records a landing for every live quest whose needs it meets; `report` (:448) queues a world event from another system; `event` (:455) finishes a quest; `melted` (:481) is the melt event. The quest list in `__discState` carries `line`, `hint` (Sprout wording at Sprout), `place` and `done` (:526). The quest system (:633) turns `SpellLanded` (the vent cooling, :658) and `MatterEvent`s (melted, smothered) into events and runs the queued ones. The quest lines moved to the data (C-113 and C-114 now point at quests.json).
- `crates/bevy-client/src/world_use.rs`: `open` (:79) also lets a wall stand in the ford's water (`in_ford`, :88); `build_wall` (:99) returns a `Wall` with what each cell held (`was`), and `crumble` puts it back (air, or the river's water); a cast that stops on the water's surface walls the water itself; `middle` (:155); `reveal_targets` (:163) adds the hidden stashes to the caches and puts the beacon on the nearest; `run` reports `walled_water` (:320) and `revealed` (:342), and shows the hidden mark.
- `crates/bevy-client/src/stash.rs` (new): `Stashes` (visible, ready, near, take, hidden_near, show), `place_marker` (:166), the plugin (:175): markers set once the block world exists, pickup by walking within 2 m (`REACH`), refill after `refill_s`.
- `crates/bevy-client/src/late.rs`: `Late::vent_hot` (:220) and `in_vent` (:594), which `fire_at` now uses.
- `crates/bevy-client/src/main.rs`: `mod stash` (:47), `stash::plugin` (:914).
- `crates/bevy-client/web/pack.js:189`: the hint under each open quest (`pack-disc-hint`, `style.css`).
- `crates/bevy-client/web/bench-free.js:743`: a lesson's `world` line under its idea (`.lesson .world`, `bench-free.css`), read aloud with it.
- `content/chem/guide.json`: `world: { quest, say }` on 18 lessons (section 5). `content/science-sources.md`: C-122 to C-170 and Not-claims rows for every new kid-facing string; C-113 to C-115 now name quests.json.

## 5. Guide links

| band | lesson | quest | line |
|---|---|---|---|
| sprout | sp_fire | melt | Try it in the world: a hot reaction can melt the ice gate in the Frozen Fields. |
| sprout | sp_fizz | cool | Try it in the world: this fizz gets cold. It can cool the hot vent on the charcoal mound. |
| sprout | sp_hot_rock | smother | Try it in the world: cooking a rock makes a gas that stops fires, like the marsh gas fire. |
| sprout | sp_cloudy | wall | Try it in the world: the cloudy bits are a solid. They can build a wall in the river. |
| sprout | sp_copper | reveal | Try it in the world: a colour change can show the hidden mark near the village. |
| seedling | se_mass | melt | Try it in the world: a reaction that gives off heat can melt the ice gate in the Frozen Fields. |
| seedling | se_particles | cool | Try it in the world: baking soda and vinegar take in heat as they fizz. That can cool the hot vent on the charcoal mound in the Frozen Fields. |
| seedling | se_test_gas | wall | Try it in the world: the milky solid that forms can build a wall in the river at the ford. |
| seedling | se_swap | reveal | Try it in the world: the colour change of a metal swap can show the hidden mark near the village. |
| apprentice | ap_balance | melt | Try it in the world: any of these burnings gives off enough heat to melt the ice gate in the Frozen Fields. |
| apprentice | ap_energy | cool | Try it in the world: the vent on the charcoal mound in the Frozen Fields needs a reaction that takes in heat by itself. |
| apprentice | ap_evidence | wall | Try it in the world: a precipitate builds a short-lived wall. Build one in the river at the ford. |
| apprentice | ap_reactivity | reveal | Try it in the world: a displacement that coats a metal and changes colour can reveal the hidden mark near the village. |
| apprentice | ap_thermal | smother | Try it in the world: breaking down a carbonate gives carbon dioxide, which can smother the gas fire in the marsh hollow. |
| adept | ad_stoich | melt | Try it in the world: a strongly exothermic reaction melts the ice gate in the Frozen Fields. |
| adept | ad_redox | smother | Try it in the world: a reduction whose only gas is CO2 can smother the gas fire in the marsh hollow. |
| adept | ad_hydroxides | wall | Try it in the world: a hydroxide precipitate builds a wall. Dam the river at the ford with one. |
| adept | ad_halogens | reveal | Try it in the world: the colour change of a halogen displacement can reveal the hidden mark near the village. |

Seedling has no smother link and Adept no cooling link: no lesson at those bands teaches a reaction that solves them. Sprout lines have no formulas and short sentences (the Sprout read-aloud test covers them).

## 6. Acceptance tests

- chem `discover.rs`: `every_quest_is_solvable_from_findable_bottles` (one live quest per world use; texts present, hints start "Hint: " and name no reaction; every `solved_by` reaction meets the needs; one example and one Sprout reaction have all their bottles findable; mismatches such as zinc + acid for smother or limestone roast for cooling do not meet them); `bottles_name_codex_substances_and_supply_carbon_burning` (now also: stash items are stash bottles, stash cells real, refill set); `guide_lessons_point_at_quests_their_reactions_solve`.
- sim `valley`: `bottle_items_round_trip_their_keys_and_stack`.
- bevy-client `discover.rs`: `each_quest_completes_only_with_a_matching_discovered_spell` (for all five: the event alone, a non-matching discovery, another quest's event, too far, too late and out of place never finish it; the matching spell does, once, with its done line); `reported_events_wait_for_the_quest_system`; `the_save_round_trips_quest_state` (`save.quests` into a new run's `quests` command and back; lines and hints per band); `a_findable_co2_reaction_smothers_a_marsh_vent_in_time` (the real valley and matter: one limestone-roast cast at a burning vent puts it out inside the window, rim gap open); the melt test is unchanged.
- bevy-client `world_use.rs`: `a_wall_in_the_river_at_the_ford_plugs_its_water` (on the surface or the bed, the wall plugs water inside the quest's place, only in the ford, and gives the water back); `a_reveal_shows_the_hidden_mark_too`; the B2 wall tests updated (dry walls count no water; a plug gives its water back).
- bevy-client `stash.rs`: `every_stash_stands_on_open_ground_in_the_valley`, `walking_up_to_a_stash_fills_the_pack_and_it_refills`, `the_hidden_mark_shows_only_after_a_reveal`. `late.rs`: `the_clamp_vent_is_hot_while_it_chars_and_until_emptied`.
- `test/bench-guide.test.ts`: `lessons point at the live chemistry quest that uses them (I-057)` (every live quest linked, all five at Sprout); the Sprout and young-label tests include the world lines. `test/bench-spells.test.ts`: five live quests by world use with their texts; every bottle has `from` and a find line without digging; stash items are bottles. `test/science-sources.test.ts`: every new string registered.
- e2e `tests/e2e/chem-quests.mjs` (release build, headless): a mastered iron + copper sulfate spell loads from `formula.spells.v1`; without bottles it names copper sulfate and the workshop; the workshop stash fills the pack; one cast near the hidden mark finishes the reveal quest and uses one of each bottle; the quest is written to localStorage; the revealed mark gives bottles; a reload keeps it done.

## 7. For the graphics thread (data values only)

- Stash markers: `bottles.json` `stashes[].block` ("glass" for the four shelves, "runestone" for the hidden mark) at `stashes[].at`. A bottle, jar or crate prop can replace the block later, keyed by stash `id`; `world_use`/`stash` only need the cell.
- Bottle items: `Item::Bottle(i)`, keys `chalk`, `vinegar`, `baking_soda`, `salt`, `copper_sulfate`, `silver_nitrate` (pack icons; they show as text labels now).
- The hidden mark's reveal: the existing beacon (`Uses::reveal_at`) stands on the mark while the reveal lasts (`effects.json` `world.reveal.seconds` 20).
- A wall in the ford: `Wall { cells, was, mat, until }` (ash, `world.wall`); `was` holds the water it plugged.
- Quest state for any HUD styling: `__discState.quests[]` (`id`, `line`, `hint`, `place`, `done`); CSS hooks `pack-disc-quest`, `pack-disc-quest.done`, `pack-disc-hint`, and the Bench's `.lesson .world`.
- The clamp's vent is hot while `late.clamp` is `charring` or `ready` (`Late::vent_hot`); a cooled-vent effect could key off the `cooled` event.

## 8. Notes and limits

- The ford is a protected puzzle stamp; a wall may stand only in its water (like the Frost path), never in its bed or banks, and crumbles back to water after 15 s.
- The smother event counts any vent going out by low oxygen within 20 s and 6 m of a matching discovered landing, so a shipped CO2 cast just after a matching one can help; the discovered spell must still have landed there.
- Bottles are run state like the rest of the pack (not saved); stashes refill, so a quest can be retried.
