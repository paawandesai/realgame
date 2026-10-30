# Science sources

The single register of every science claim the first-person 3D client shows the player (`crates/bevy-client`), and of every number in its rules engine that drives a shown outcome. `npm test` (`test/science-sources.test.ts`) fails if a client string that looks like a science claim has no row here, if a row is malformed or cites an undefined source, if a `_C` temperature constant in `chem.rs` has no row or a different value, or if a `C-` row's text is no longer in a file it names. The fresh-model science review that scores bar B6 follows `docs/loop/science-review.md`.

**What the lint sees.** The lint flags a client string as a claim when it has a temperature, a chemical formula, a science keyword, or the shape of a sentence (three or more words ending in `.`, `!`, `?` or `…`); everything else, including strings built at runtime and the skipped machine-output lines, is covered by the review protocol (see [Lint limits](../docs/loop/science-review.md#lint-limits)).

**Conditions.** Phase-change temperatures are at 101.325 kPa (1 atm). Standard enthalpies (ΔH°) are at 298.15 K and 1 bar, per mole of reaction as written. Temperatures shown to the player are rounded to the nearest whole degree (or kelvin).

**Verdicts.**

- `correct`: the statement shown to the player is true as stated, at the conditions in the register header, within the tolerance rules.
- `simplified`: the statement is true but idealized or incomplete (it omits a mechanism, rounds, or describes a limiting case), and no reasonable reading of it is false. The note says what is simplified and why it is not false.
- `error`: a statement shown to the player that is false, or a parameter that drives a shown outcome and lies outside every cited sourced range. The note gives the correct statement or value with its source.

**Adding a row.** Copy the exact in-game text into the `claim` cell inside single backticks (keep placeholders such as `{who}` and `${...}` literally; write a literal `|` as `\|`). Give it the next unused `C-NNN` id (ids are never reused), the repo-relative file(s) that show it, a verdict derived from the cited source, a note unless the verdict is `correct`, and one or more `[S<n>]` sources defined below. A new `_C` constant in `chem.rs` needs a `## Parameters` row with the same value. `[S0]` may be cited only in `## Parameters`, with a note that starts `gameplay tuning` or `scenario assumption`. Any item that adds or changes a player-facing science string or a `_C` constant updates this file in the same PR.

## Claims

| id | claim | where | verdict | note | sources |
|---|---|---|---|---|---|
| C-001 | `CH4 + 2O2 → CO2 + 2H2O` | crates/bevy-client/src/chem.rs, crates/bevy-client/web/index.html | correct | | [S7] |
| C-002 | `H2O absorbs heat` | crates/bevy-client/src/chem.rs | correct | | [S11] [S10] |
| C-003 | `N2(l) boils at −196 °C` | crates/bevy-client/src/chem.rs, crates/bevy-client/web/index.html | correct | | [S1] |
| C-004 | `CO2 blanket, no O2` | crates/bevy-client/src/chem.rs | simplified | Limiting case. A CO2 blanket dilutes and displaces O2; it does not remove all of it. Flaming stops well before O2 reaches zero (fuel-specific limits of roughly 12.5 to 16 vol% O2; 10 vol% stops nearly all fires), and CO2 extinguishes mainly by absorbing heat, with dilution secondary. Not false as a hotbar label for "keeps oxygen away"; the reviewer may escalate. | [S14] |
| C-006 | `Water soaks up heat and cools things below their ignition temperature.` | crates/bevy-client/src/chem.rs | correct | | [S9] [S11] |
| C-007 | `Liquid nitrogen boils at −196 °C (77 K), pulling heat out of anything it touches.` | crates/bevy-client/src/chem.rs | correct | | [S1] |
| C-009 | `It caught fire: hot enough to reach its ignition temperature, with oxygen around.` | crates/bevy-client/src/chem.rs | correct | | [S6] [S14] |
| C-010 | `Fire out! Water cooled it below its ignition temperature.` | crates/bevy-client/src/chem.rs | correct | | [S9] |
| C-011 | `Fire out! Liquid nitrogen at −196 °C cooled it far below ignition.` | crates/bevy-client/src/chem.rs | correct | | [S1] [S6] |
| C-013 | `It burnt out: the fuel is used up.` | crates/bevy-client/src/chem.rs | correct | | [S7] |
| C-014 | `The ice melted at 0 °C: H2O(s) → H2O(l). Now it is a puddle.` | crates/bevy-client/src/chem.rs | correct | | [S10] |
| C-016 | `Frozen! Liquid nitrogen at −196 °C (77 K) froze the water: H2O(l) → H2O(s).` | crates/bevy-client/src/chem.rs | simplified | The text is true. The outcome it captions freezes a whole puddle in one cast; real freezing is limited by heat transfer (6.0 kJ/mol must leave the water) and takes much longer. Limiting case, not false. | [S1] [S10] |
| C-017 | `Hot water boils at 100 °C and turns into steam: H2O(l) → H2O(g).` | crates/bevy-client/src/chem.rs | correct | | [S2] [S10] |
| C-019 | `Soaked: it must dry out (boil off at 100 °C) before it can burn.` | crates/bevy-client/src/chem.rs | simplified | Water evaporates at any temperature, not only at 100 °C. Under a flame, a wet surface stays near 100 °C until its water is gone, because the heat goes into vaporizing it, and that part is true. | [S10] [S2] |
| C-020 | `Brrr! Liquid nitrogen boils away at −196 °C as it chills the surface.` | crates/bevy-client/src/chem.rs | correct | | [S1] |
| C-021 | `Sizzle! The water must boil away at 100 °C before it can burn.` | crates/bevy-client/src/chem.rs | simplified | As C-019. | [S10] [S2] |
| C-022 | `No oxygen under the CO2 blanket, so the fire cannot start.` | crates/bevy-client/src/chem.rs | simplified | As C-004: the blanket lowers O2 below what flaming ignition needs; "no oxygen" is the limiting case. | [S14] |
| C-023 | `That does not burn.` | crates/bevy-client/src/chem.rs | correct | | [S6] |
| C-024 | `Already burnt: no fuel left.` | crates/bevy-client/src/chem.rs | simplified | Char is a mostly carbonaceous residue whose oxidation is strongly exothermic, so it can still glow or smoulder. It has no fuel left for flaming, which is what the game models. Incomplete, not false for flames. | [S18] |
| C-025 | `Heating up... a little more heat and it will catch fire.` | crates/bevy-client/src/chem.rs | correct | | [S6] |
| C-026 | `Water-splitting station (coming soon): 2H2O → 2H2 + O2 with electricity.` | crates/bevy-client/src/chem.rs | correct | | [S7] [S13] [S3] |
| C-027 | `Oh no! Heat from the fire reached the greenhouse. Press R to try again.` | crates/bevy-client/src/main.rs | correct | | [S6] |
| C-028 | `Heat from a neighbour raised the {who} to its ignition temperature: it caught fire!` | crates/bevy-client/src/main.rs | correct | | [S6] |
| C-030 | `Heat spread from bush to bush. Press R to try again.` | crates/bevy-client/web/hud.js | correct | | [S6] |
| C-031 | `cools below ignition` | crates/bevy-client/web/index.html | correct | | [S9] |
| C-032 | `CO₂ displaces O₂` | crates/bevy-client/web/index.html | correct | | [S14] [S5] [S8] |
| C-033 | `The greenhouse is heating up: ${state.ghTemp} °C. Wood can catch fire at about 300 °C!` | crates/bevy-client/web/hud.js | simplified | Piloted ignition of wood (a flame or spark present) that goes straight to flaming normally happens at surface temperatures of about 300 to 365 °C, and as low as about 250 °C under long, low heat flux (glowing first); autoignition with no pilot needs higher temperatures. "About 300 °C" is the typical piloted value, which is what the greenhouse frame models (P-004). The shown temperature is the game's one-number model of the frame's surface. | [S6] |
| C-034 | `Fire spell: a gas flame. Methane burns: CH4 + 2O2 → CO2 + 2H2O, and lots of heat.` | crates/bevy-client/src/chem.rs | correct | | [S7] [S4] |
| C-035 | `CO2 from an extinguisher is heavier than air: it smothers fire by pushing the oxygen away.` | crates/bevy-client/src/chem.rs | simplified | "Heavier than air" is correct: M(CO2) = 44.01 g/mol vs 28.96 g/mol for sea-level air, so about 1.52 times as dense at the same T and p. A CO2 extinguishing agent discharges carbon dioxide and puts fire out by lowering the oxygen concentration [S19]. "Pushing the oxygen away" is the textbook picture; the EPA review says CO2 puts out flames mainly by a thermophysical (heat-absorbing) mechanism, with dilution of the reacting species secondary [S14]. Incomplete, not false. | [S5] [S8] [S14] [S19] |
| C-036 | `Fire out! The CO2 smothered it: no oxygen, no fire.` | crates/bevy-client/src/chem.rs | simplified | A CO2 extinguishing agent puts fire out by lowering the oxygen concentration [S19]. "No oxygen" is the limiting case: flaming stops well before O2 reaches zero (see C-004), and CO2 also works by absorbing heat [S14]. | [S19] [S14] |
| C-037 | `Melting... it takes a lot of heat to turn ice into water.` | crates/bevy-client/src/chem.rs | correct | | [S10] |
| C-038 | `A CO2 blanket covers it: no oxygen can reach it, so it cannot burn.` | crates/bevy-client/src/chem.rs | simplified | As C-036: CO2 lowers the oxygen concentration [S19]; "no oxygen" is the limiting case (see C-004). | [S19] [S14] |
| C-039 | `You saved the greenhouse! Water, frost and CO2 each stop fire in their own way.` | crates/bevy-client/src/main.rs | correct | | [S9] [S1] [S19] |

## Parameters

| id | constant | value | unit | where | verdict | note | sources |
|---|---|---|---|---|---|---|---|
| P-001 | `AMBIENT_C` | 20 | °C | crates/bevy-client/src/chem.rs | correct | scenario assumption: a mild day in the courtyard. | [S0] |
| P-002 | `LN2_C` | −196 | °C | crates/bevy-client/src/chem.rs | correct | | [S1] |
| P-003 | `FLAME_C` | 800 | °C | crates/bevy-client/src/chem.rs | simplified | One value stands in for a whole burning object. Measured fire-flame temperatures run from about 300 to 600 °C at the flame tips to about 900 °C in the continuous flame region (wildfire peaks around 927 °C; burning rack-storage solids averaged 870 °C). 800 °C lies inside that range. | [S15] |
| P-004 | `ignition_c(Bush)` | 300 | °C | crates/bevy-client/src/chem.rs | simplified | Also used for `Greenhouse` (the `_` arm). Wood piloted ignition that goes straight to flaming normally happens at surface temperatures of 300 to 365 °C, and about 250 °C at the lowest heat flux (glowing first). Measured leaf ignition temperatures span 227 to 453 °C. 300 °C sits at the low end of flaming ignition; one fixed value ignores heating rate and moisture. | [S6] [S16] |
| P-005 | `ignition_c(DryHedge)` | 230 | °C | crates/bevy-client/src/chem.rs | simplified | Inside the measured species-averaged leaf ignition range of 227 to 453 °C (near its low end), and below solid wood (about 250 °C minimum). Dead, dry leaves ignite more easily than the moist leaves measured, so a low value is reasonable; one fixed value ignores heating rate. | [S16] [S6] |
| P-006 | `Obj::new(SteamVent)` | 110 | °C | crates/bevy-client/src/chem.rs | correct | Steam above the 100 °C normal boiling point (373.17 K) is superheated, which a vent fed by hot water can supply. | [S2] |
| P-007 | `Obj::new(IceWall)` | −5 | °C | crates/bevy-client/src/chem.rs | correct | Ice below its 0 °C melting point. | [S10] |
| P-008 | `apply(Water) clamps temp` | 100 | °C | crates/bevy-client/src/chem.rs | simplified | Not shown as text. A wet object is held at 100 °C while its water boils off, because a substance changing state stays at constant temperature; the game reaches that temperature instantly. | [S2] [S10] |
| P-009 | `apply(Frost) sets temp` | −196 | °C | crates/bevy-client/src/chem.rs | simplified | Not shown as text. The target is set to the boiling point of liquid nitrogen instantly; real cooling is limited by heat transfer. Not a B6 error unless a caption states it. | [S1] |
| P-010 | `FIRE_HEAT` | 300 | °C | crates/bevy-client/src/chem.rs | correct | gameplay tuning, not a physical claim: how much one Fire cast heats an object, added to its current temperature (so a dry bush at 20 °C lights in one cast, and one chilled to −196 °C by liquid nitrogen needs two). | [S0] |
| P-011 | `COOLING` | 0.02 | 1/s | crates/bevy-client/src/chem.rs | correct | gameplay tuning, not a physical claim | [S0] |
| P-012 | `SPREAD_HEAT` | 12 | °C/s | crates/bevy-client/src/chem.rs | correct | gameplay tuning, not a physical claim | [S0] |
| P-013 | `SMOTHER_S` | 25 | s | crates/bevy-client/src/chem.rs | correct | gameplay tuning, not a physical claim | [S0] |
| P-014 | `NEIGHBOUR_GAP` | 2.5 | m | crates/bevy-client/src/chem.rs | correct | gameplay tuning, not a physical claim | [S0] |
| P-015 | `electrolysis prop H2:O2 tube volume ratio` | 2 | ratio | crates/bevy-client/src/chem.rs, crates/bevy-client/src/world.rs | correct | Shown visually: the gas columns collected at the top of two equal tubes are 0.6 m (H2, cathode side) and 0.3 m (O2), set by `chem::electrolysis::GAS_H2 = 2 × GAS_O2` and checked by the test `electrolysis_gas_is_two_to_one`. 2H2O(l) → 2H2(g) + O2(g) gives 2 mol H2 per mol O2, and by Avogadro's law equal moles of gas fill equal volumes at the same T and p. | [S13] [S12] |

## Not claims

| text | where | reason |
|---|---|---|
| `Fires put out: ${Math.min(state.extinguished, state.goal)} / ${state.goal} · burning now: ${state.burning}` | crates/bevy-client/web/hud.js | Status counter (fires put out, fires burning). Flagged only by the keyword `burn`; it states no science. |
| `The greenhouse is heating up!` | crates/bevy-client/web/index.html | Placeholder of the warning banner, replaced by C-033 before it is shown. A narrative warning with no number or mechanism. |
| `Not the greenhouse! We are here to save it. Aim at the burning bushes.` | crates/bevy-client/src/chem.rs | Game instruction (Fire aimed at the greenhouse refuses). Flagged by sentence shape and `burn`; it states no science. |
| `Click to look around. Esc shows the controls and point-and-click mode.` | crates/bevy-client/web/index.html, crates/bevy-client/web/hud.js | Game instruction (controls). Flagged by sentence shape; it states no science. |
| `Aim at a glowing target: the crosshair turns yellow. Left click casts.` | crates/bevy-client/web/hud.js | Game instruction (aiming). Flagged by sentence shape; it states no science. |
| `Mouse lock is blocked here: hold a mouse button and drag to look. Click to cast.` | crates/bevy-client/web/hud.js | Game instruction (controls when the browser refuses pointer lock). Flagged by sentence shape; it states no science. |
| `Point and click: click a glowing target to cast at it. Drag to look.` | crates/bevy-client/web/hud.js | Game instruction (point-and-click mode). Flagged by sentence shape; it states no science. |
| `Another bush is on fire! Put it out before it spreads.` | crates/bevy-client/src/chem.rs | Game event: a new fire started so the level can still be won after a fire burnt out by itself. Flagged by sentence shape and `fire`; it states no science. |
| `Click again to lock the mouse.` | crates/bevy-client/web/hud.js | Game instruction (the browser refused a new pointer lock right after Esc). Flagged by sentence shape; it states no science. |
| `CO2` | crates/bevy-client/src/chem.rs, crates/bevy-client/web/index.html | Name of spell 4 on the hotbar (carbon dioxide from an extinguisher). A label naming the substance; it asserts nothing. Its science is C-004, C-032 and C-035. |
| `Save the Greenhouse! Put out 3 fires before they spread. Press 2 for Water.` | crates/bevy-client/src/main.rs | Game instruction (the objective and a key hint). Flagged by sentence shape; it states no science. |
| `Save the Greenhouse! Put out 3 fires before they spread.` | crates/bevy-client/src/main.rs | Game instruction (the objective). Flagged by sentence shape; it states no science. |
| `You saved the greenhouse!` | crates/bevy-client/web/hud.js | Win banner. Flagged by sentence shape; it states no science. |
| `${state.extinguished} fires put out. Press R to play again.` | crates/bevy-client/web/hud.js | Win banner subtitle (a count and a key hint). Flagged by sentence shape; it states no science. |
| `The greenhouse caught fire` | crates/bevy-client/web/hud.js | Lose banner: it reports the game outcome. Flagged by the keyword `caught fire`; the science of why (heat spread) is the subtitle, C-030. |
| `Without locking the mouse you can also click (or tap) a glowing target to cast at it.` | crates/bevy-client/web/index.html | Game instruction (controls). Flagged by sentence shape; it states no science. |
| `Look for a fire. Press 2 for Water, aim at the fire and click.` | crates/bevy-client/web/hud.js | Idle hint after 45 s with no progress (a key and aiming instruction). Flagged by sentence shape and `fire`; it states no science. |

## Sources

[S17] and [S20] were removed in I-012 once no row cited them; their ids are not reused.

- [S0] Design choice: gameplay tuning or scenario assumption, no physical claim.
- [S1] NIST Chemistry WebBook, SRD 69, Nitrogen, phase change data (Tboil = 77.34 K, Jacobsen, Stewart, et al., 1986): https://webbook.nist.gov/cgi/cbook.cgi?ID=C7727379&Mask=4
- [S2] NIST Chemistry WebBook, SRD 69, Water, phase change data (Tboil = 373.17 ± 0.04 K, average of 7 values): https://webbook.nist.gov/cgi/cbook.cgi?ID=C7732185&Mask=4
- [S3] NIST Chemistry WebBook, SRD 69, Water, condensed phase thermochemistry (ΔfH°liquid = −285.830 ± 0.040 kJ/mol, CODATA review value, Cox, Wagman, et al., 1984): https://webbook.nist.gov/cgi/cbook.cgi?ID=C7732185&Mask=2
- [S4] NIST Chemistry WebBook, SRD 69, Methane, gas phase thermochemistry (ΔcH°gas = −890.7 ± 0.4 kJ/mol, Pittam and Pilcher, 1972): https://webbook.nist.gov/cgi/cbook.cgi?ID=C74828&Mask=1
- [S5] NIST Chemistry WebBook, SRD 69, Carbon dioxide (molecular weight 44.0095): https://webbook.nist.gov/cgi/cbook.cgi?ID=C124389
- [S6] Babrauskas, V., "Ignition of Wood: A Review of the State of the Art", Journal of Fire Protection Engineering 12(3), 163–189, 2002. doi:10.1106/104239102028711
- [S7] Flowers, P., Theopold, K., Langley, R., Robinson, W. R., *Chemistry 2e*, OpenStax, 2019, § 4.1 "Writing and Balancing Chemical Equations": https://openstax.org/books/chemistry-2e/pages/4-1-writing-and-balancing-chemical-equations
- [S8] Minzner, R. A. (ed.), *The 1976 Standard Atmosphere Above 86-km Altitude*, NASA SP-398, NASA Goddard Space Flight Center, 1976. Standard sea-level mean molecular weight of air M0 = 28.9644 kg/kmol, stated in the Symbols list ("The standard, sea-level value of M, 28.9644 kg kmol-1") and in the chapter "Lower Boundary Conditions" (the list of boundary values, "M0 = 28.9644 kg kmol-1"); the same value defines the *U.S. Standard Atmosphere, 1976* (NOAA-S/T 76-1562, NASA-TM-X-74335, NTRS 19770009539). Fetched 2026-10-30: https://ntrs.nasa.gov/api/citations/19770003812/downloads/19770003812.pdf
- [S9] Grant, G., Brenton, J., Drysdale, D., "Fire suppression by water sprays", Progress in Energy and Combustion Science 26, 79–130, 2000. doi:10.1016/S0360-1285(99)00012-X
- [S10] Flowers, P., et al., *Chemistry 2e*, OpenStax, 2019, § 10.3 "Phase Transitions": https://openstax.org/books/chemistry-2e/pages/10-3-phase-transitions
- [S11] Flowers, P., et al., *Chemistry 2e*, OpenStax, 2019, § 5.1 "Energy Basics": https://openstax.org/books/chemistry-2e/pages/5-1-energy-basics
- [S12] Flowers, P., et al., *Chemistry 2e*, OpenStax, 2019, § 9.2 "Relating Pressure, Volume, Amount, and Temperature: The Ideal Gas Law": https://openstax.org/books/chemistry-2e/pages/9-2-relating-pressure-volume-amount-and-temperature-the-ideal-gas-law
- [S13] Flowers, P., et al., *Chemistry 2e*, OpenStax, 2019, § 17.7 "Electrolysis": https://openstax.org/books/chemistry-2e/pages/17-7-electrolysis
- [S14] U.S. EPA, Office of Air and Radiation, *Carbon Dioxide as a Fire Suppressant: Examining the Risks*, EPA430-R-00-002, February 2000: https://www.epa.gov/sites/production/files/2015-06/documents/co2report.pdf
- [S15] Babrauskas, V., "Temperatures in flames and fires", Fire Science and Technology Inc., 1997, revised 2006: https://doctorfire.com/temperatures-in-flames-and-fires/
- [S16] Fletcher, T. H., Pickett, B. M., Smith, S. G., Spittle, G. S., Woodhouse, M. M., Haake, E., Weise, D. R., "Effects of moisture on ignition behavior of moist California chaparral and Utah leaves", Combustion Science and Technology, 1183–1203, 2007. Record: https://research.fs.usda.gov/treesearch/30610
- [S18] Ohlemiller, T. J., *Modeling of Smoldering Combustion Propagation*, NBSIR 84-2895, National Bureau of Standards, 1984. doi:10.6028/NBS.IR.84-2895
- [S19] U.S. Federal Aviation Administration, Advisory Circular 150/5210-6E, *Aircraft Fire Extinguishing Agents for Airports*, 27 November 2023: § 1.2 (foams "are produced by incorporation of air into a solution of foam concentrate and water"; "They extinguish fire by physically separating the fuel vapors from the heat and oxygen necessary for combustion"), § 1.4.2 (carbon dioxide "extinguishes fire by reducing the concentration of oxygen, the vapor phase of the fuel, or both"), App. C item 9 (foam: "An aggregation of small bubbles used to form an air-excluding, vapor-suppressing blanket"): https://www.faa.gov/documentLibrary/media/Advisory_Circular/AC-150-5210-6e-arff-ext-agents.pdf
