# I-058: evidence for every playable reaction

Branch `loop/bench-audit`, from `loop/bench-play`. Paawan asked whether we can guarantee that the reactions the game shows are accurate in result. This item makes every playable reaction traceable to a real-world source, checks what the player sees against that source, and keeps reactions with no traced source out of play. I-058 was free (the last item was I-057, and no remote branch has an I-058 folder).

## 1. The answer, briefly

- All 486 reactions (182 core, 304 rule) now carry an `evidence` field. Each one names an open source: OpenStax, Wikipedia or an exam board's practical guide. Rule reactions also name the table entries the rule was applied to.
- 436 of the 486 (90 %) were **checked**: the cited page was fetched on 2026-11-05 and states the claim in the `claim` field. The other 50 cite a page that gives only the general rule or an analogue, or a reference that was not fetched. These are `checked: false`, and the `note` field says what is missing.
- No reaction ended up `unverified`, so nothing is hidden today. The gate is in place and tested (section 4). Any future entry with no evidence, or with `unverified` evidence, is hidden on load.
- Three things the player sees were wrong against their sources and are fixed (section 6). Section 7 lists 13 more that are defensible but disagree with the source in a detail. A teacher should rule on those.

What we can claim: every reaction the game offers is a real, documented reaction, with its products as the cited source gives them. Section 8 has the full statement of what we cannot claim.

## 2. Method

### Evidence kinds

`crates/chem/src/evidence.rs`, `Evidence` on `Entry` (`lib.rs`):

| field | meaning |
|---|---|
| `kind` | `textbook`: a standard reaction in a common open reference. `data`: the outcome follows from cited data tables (Table 4.1 solubility rules, standard reduction potentials, CRC enthalpies of formation) and a stated rule. `unverified`: no source traced. |
| `source` | The reference in words: work, edition, section or table, or the article title. |
| `url` | The page. |
| `checked` | `true` only when the page was fetched during this audit and states `claim`. For `textbook`, the page must state **this** reaction (same reactants and products, in words, molecular or ionic form). A page that gives only the general rule or an analogue (for example HCl + NaOH cited for HCl + KOH) is `false`. For `data`, the rule's statement and the table entries must have been read on the fetched pages. |
| `claim` | What the source says, quoted or closely paraphrased. |
| `entries` | For `data`: the table entries used (Table 4.1 lines, E° values, ΔHf terms, precipitate colour and its source). |
| `note` | Caveats: what the page does not cover, or where the game simplifies. |

Every `checked: true` entry has an `https://` URL and a claim (test `every_playable_reaction_has_evidence`).

### Sources and reachability

`curl -s` was blocked for all four suggested sites. The proxy answered 403 to CONNECT for openstax.org, chem.libretexts.org, edu.rsc.org and en.wikipedia.org, and for NIST, PubChem, CLEAPSS and BBC Bitesize. The session's WebFetch tool goes through a separate fetcher, and it reached:

| site | WebFetch | used |
|---|---|---|
| openstax.org | yes | Chemistry 2e sections 4.1 and 4.2 (with Table 4.1); Biology 2e section 8.1 |
| en.wikipedia.org | yes | 94 articles as the main source, plus 'Flame test', 'Iodine', 'Sulfur' and 'Potassium bromide' read for claims (section 9) |
| chem.libretexts.org | yes (one test page) | not cited: the OpenStax originals were reachable |
| ocr-live-prd95.cambridgeassessment.org.uk | yes | OCR GCSE Chemistry PAG 2 (electrolysis of copper(II) chloride) |
| edu.rsc.org | **no** (client error) | not cited as checked |

The fetcher returns a model's reading of the page, not the raw HTML. Each claim was asked for verbatim, and a teacher may want to re-read a sample (section 7).

### Core (182, hand-written)

Each reaction was matched to the page that best states it. About 100 pages were fetched, asking for the equation, the conditions and what is seen (colours, flames, gases, heat). The evidence was written into `core.json` by a one-off script that keyed on reaction ids. The ledger was not committed: `core.json` is the source of truth. Precipitations and displacements whose outcome follows from a table are `data`: Table 4.1 for precipitations, the electrode potential data page for displacements.

### Rule tier (304, generated)

`tools/chemdb/build_rules.py` now writes `evidence` for every rule reaction (`evidence_of`). Each is `data`, built from three things:

- The rule's statement on a fetched page:
  - acid + base: OpenStax 4.2, Wikipedia 'Basic oxide' and 'Ammonia';
  - metal + acid: Wikipedia 'Single displacement reaction' and the electrode potential data page;
  - carbonate + acid: Wikipedia 'Carbonate' and 'Sodium bicarbonate';
  - fuel burns: Wikipedia 'Combustion' and 'Alkane';
  - precipitate: OpenStax Table 4.1;
  - displacement: the electrode potential data page and 'Single displacement reaction'.
- The table entries for this reaction: each salt's Table 4.1 line (quoted), E° for both metals as read on Wikipedia, the flash point that sets a fuel's condition, the precipitate's colour and its source key, and every ΔHf term behind the heat class.
- `checked`:
  - false for Al with a bromide (3 displacements, 1 metal + acid): the fetched page says chloride corrodes aluminium, but that bromide breaks the oxide skin comes from Greenwood and Earnshaw, which was not fetched;
  - false for the two alcohol fuels (butanol, pentanol): the fetched pages state complete combustion for hydrocarbons, ethanol and methanol only;
  - true for the other 298.

Table 4.1 was compared with the rule tables (`solubility`) and agrees for every precipitate and dissolved partner the tier uses. Where the tier is stricter than Table 4.1 (it treats CaSO4, Ag2SO4, PbCl2 and PbBr2 as slightly soluble and never uses them as precipitates), it does not contradict the table. The E° values the tier uses (CRC, in `rules.json`) match the Wikipedia data page to 0.01 V. The data page gives Fe at −0.44 V; the tier has −0.45 V.

The enthalpies of formation behind every heat direction come from the CRC tables in the `chemicals` package, as `substances.json` records. They were not re-fetched. Wikipedia gives a ΔH for 13 core reactions (Ostwald steps, contact process, water gas, water-gas shift, steam reforming, slaking, NO2 dimer, NI3, P4O10 + water, neutralisation, water splitting, sulfur burning, ammonium chloride). In all 13 the sign agrees with the game's heat direction.

## 3. Counts

| tier | kind | checked | unchecked | total |
|---|---|---|---|---|
| core | textbook | 126 | 42 | 168 |
| core | data | 12 | 2 | 14 |
| core | unverified | 0 | 0 | 0 |
| rule | data | 298 | 6 | 304 |
| rule | textbook or unverified | 0 | 0 | 0 |
| **all** | | **436** | **50** | **486** |

Core: 138 checked, 44 unchecked. Rule: 298 checked, 6 unchecked.

Playable: 486 of 486. Hidden: 0.

## 4. The gate

- `Entry::playable()`: true only for `textbook` or `data` evidence. An entry with no evidence field, or with `kind: unverified`, is not playable.
- `Codex::from_parts` (so `Codex::load` and `load_core`) puts entries that are not playable into `Codex::hidden` and leaves them out of `entries`, the match keys and `known_species`. Every entry still has to parse and balance, hidden or not.
- Everything the game reads comes from `entries`:
  - the Bench page's codex (`codex_json`), judging (`judge_json`), effects (`effects_json`) and show records (`show_json`, `Shows::build` over `entries`);
  - the in-world Formula Bench (`ReactionCodex for Codex`);
  - the client's discoveries (`bevy-client/src/discover.rs` `index_of`; `spells::book().codex`).

  A hidden reaction is never listed, never found by running its reactants (the page says nothing in our data happens and may consult the lab records), never judged known (a typed equation falls to the mystery predictor, which is labelled "predicted by the model, not confirmed"), has no effect, so it cannot be saved, cast or equipped, and is dropped from a loaded save.
- `Codex::load_audit()` keeps every entry, hidden ones included, for data checks and the teacher table. Nothing that plays uses it.

### Guide, quests and bottles

Nothing is hidden, so no reference in `guide.json`, `quests.json` or `bottles.json` points at a hidden reaction, and none needed fixing. If a later edit marks a reaction `unverified`, the existing tests catch any reference to it: the guide tests (every reaction id is a codex id) and `every_quest_is_solvable_from_findable_bottles` run on `Codex::load()`, which no longer holds it. The edit then has to restore evidence for that reaction or change the reference, and say which in its spec.

## 5. Tests

- `crates/chem/src/evidence.rs`:
  - `every_playable_reaction_has_evidence`: every playable entry has textbook or data evidence with a source. Data evidence names entries. Checked evidence has an https URL and a claim. Every entry in both data files has an evidence field.
  - `no_unverified_reaction_can_be_run_or_saved`: for the shipped codex, and for a codex in which core `silver_chloride` and rule `rule_precipitate_agno3_kcl` are marked unverified and core `methane_burns` loses its evidence field, each hidden entry:
    - is not in the codex;
    - is never returned by `run` on its reactants or by `judge` on its equation, at every band with every condition;
    - has no spell effect.

    Every effect is a playable reaction's.
  - `the_teacher_table_is_current`: `content/chem/evidence.csv` equals the generator's output and has one row per reaction.
- `crates/bevy-client/src/discover.rs` `an_unverified_reaction_is_never_found_saved_or_equipped`: the world's codex holds only playable entries, and a save that names a hidden id (or an unknown one) loads nothing for it: not found, not saved, not on a key.
- Unchanged and passing: the rule checks in `rules.rs`, the show record tests in `show.rs`, the effects, guide and quest tests in `discover.rs`, and `tests/one_engine.rs`.

## 6. Data changes

| reaction(s) | what the player saw | now | reason |
|---|---|---|---|
| `ammonia_burns` | flame "pale yellow-orange" | "pale yellowish-green" | Wikipedia 'Ammonia': mixed with oxygen it "burns with a pale yellowish-green flame". Changed in `show.json` (emitter `nh3`, new source key `FLAME-NH3`). |
| `silver_iodide` and the 4 rule precipitates of AgI (`rule_precipitate_agno3_{nai,lii,nh4i,zni2}`) | solid "pale yellow" | "yellow" (hex `#f0d43c`) | Wikipedia 'Silver iodide': "a bright yellow salt"; "a yellowish solid quickly precipitates". Core's own `say` already called it yellow. In `build_show_data.py` → `substances.json`. The rule tier's `say` lines now read "a yellow solid". |
| `lithium_nitrogen` | solid "red-brown" | "reddish-pink" (hex `#c4716e`) | Wikipedia 'Lithium nitride': "a reddish-pink solid"; "red-purple or reddish-pink crystals or powder". In `build_show_data.py` → `substances.json`. |

No equation, product, state, band or heat direction changed. Bands were left as they are (Paawan is choosing new ones), and the CSV records each reaction's current band.

## 7. For teacher review

### Where the game simplifies or differs from its source in a detail

1. `alumina_electrolysis`: the game writes 2Al2O3 → 4Al + 3O2 and shows oxygen gas. The Hall–Héroult page says the carbon anodes burn, so the real cell gives CO2 (2Al2O3 + 3C → 4Al + 3CO2). This is the GCSE simplification.
2. `copper_carbonate_heat`, `copper_carbonate_acid` and the four rule `carbonate_acid_cuco3_*`: "CuCO3" stands for basic copper carbonate, Cu2(OH)2CO3, which also gives water. True CuCO3 is rarely met.
3. `iron_rusts`: anhydrous Fe2O3; rust is hydrous Fe2O3·nH2O.
4. `ammonia_water` (NH4OH) and `so2_water` (H2SO3) are written as their textbook forms. The sources describe equilibria, and sulfurous acid "cannot be isolated".
5. `silver_bromide` and the AgBr rule precipitates: shown "cream" (the usual school word). Wikipedia says "pale-yellow".
6. `copper_chlorine`: anhydrous CuCl2 shown "yellow-brown"; Wikipedia says "dark brown".
7. `iron_chlorine`: FeCl3 shown "dark brown-black" solid and smoke; Wikipedia says the crystals look dark green by reflected light and purple-red by transmitted light.
8. `iron2_hydroxide` and the Fe(OH)2 rule precipitates: shown "dirty green". Wikipedia says it is white when pure and greenish with traces of oxygen. Green is what a school lab sees.
9. `thermite`: the game shows white smoke. Wikipedia says thermite makes almost no gas and does not mention smoke.
10. `sodium_bromine`: the game shows no flame. Wikipedia 'Halogen' says heated sodium reacts with halogens with "bright-orange flames", but only as a general statement.
11. `hbr_synthesis` (heat only; the page names a platinum catalyst) and `methane_chlorine` (light; the page names heat). The condition differs from the industrial method the source describes.
12. `candle_burns`: C25H52 stands for paraffin wax (C20–C40; the page names C31H64 as typical).
13. `lead_chloride`: PbCl2 dissolves to about 10 g/L, so it comes out only from fairly concentrated solutions.

### Not checked (the claim was not read on a fetched page)

- **Core, 44 reactions.** Their `note` says why: candle_burns, lithium_water, potassium_water, potassium_chlorine, magnesium_chlorine, sodium_bromine, aluminium_oxide, butane/pentane/hexane/octane_burns, acetylene_burns, benzene_burns, propanol_burns, so3_water, silver_oxide_heat, lab_ammonia, alumina_electrolysis, hcl_electrolysis, eight neutralisations (KOH, LiOH, H2SO4 with NaOH and KOH, HNO3 with NaOH and KOH, Ca(OH)2, vinegar), neutralise_baoh2_h2so4, iron_acid, calcium_acid, zinc_sulfuric, magnesium_sulfuric, washing_soda_acid, copper_carbonate_acid, marble_sulfuric, aluminium_copper_sulfate, bromine_iodide, silver_tarnish, so2_water, ammonia_water, ethene_hydrogen, esterification, cracking, ozone_forms. Most are a general rule applied to a close relative. A teacher can confirm these quickly.
- **Rule, 6 reactions:** rule_metal_acid_al_hbr, rule_displacement_al_{cubr2,febr2,znbr2} (bromide breaking aluminium's oxide skin), rule_fuel_burns_c4h9oh_o2 and rule_fuel_burns_c5h11oh_o2 (alcohol combustion).

### Not covered by the sources: what the player sees beyond the reaction

The fetched pages confirm the reaction and products, and often a colour or flame. They do not confirm every visual detail:

- several flame colours (propane blue, sulfur blue, carbon disulfide blue, zinc bluish-green, the luminous flames of the heavier hydrocarbons);
- speeds;
- the "feels" lines;
- the colours in `substances.json` that cite `COLOUR-GE` (Greenwood and Earnshaw; not fetched).

The flash points that decide spark versus heat for the rule-tier fuels (`FLASH`, PubChem and NIOSH) were not fetched. Heat directions rest on the CRC enthalpies of formation in `substances.json`, which are machine-readable copies, and on the 13 cross-checks in section 2. The heat changes are real at standard state, but many (precipitations, about ±25–150 kJ per equation) are too small to feel at bench dilution.

### Re-reading a sample

Because the fetcher returns a reading of each page rather than the page itself, a teacher should re-open a sample of the `checked: true` URLs. Suggested sample: one per family, for example `iron_copper_sulfate`, `golden_rain`, `limestone_roast`, `ostwald_1`, `rule_precipitate_bacl2_k2so4`, `rule_acid_base_hcl_cuo`.

## 8. What we can and cannot claim

Every reaction the game offers is a documented real reaction, traceable to a named open source. 436 of 486 were checked against the fetched page; the other 50 rest on a general rule or a reference cited from knowledge, and say so. Products follow the source. The three visible details that disagreed with their sources are fixed.

We cannot claim that every visual detail (each colour shade, flame colour, speed or "feel") has been confirmed by a source, or that the game's simplified equations match real practice everywhere. Section 7 lists the known simplifications. The fetcher's readings have also not yet been re-read by a person. Until a teacher has reviewed `content/chem/evidence.csv`, the honest wording is "sourced and checked by an automated audit", not "teacher-verified".

## 9. Files

- New:
  - `crates/chem/src/evidence.rs` (types, CSV, tests);
  - `crates/chem/examples/evidence_csv.rs`;
  - `scripts/evidence-csv.sh`;
  - `content/chem/evidence.csv`;
  - this spec.
- Changed:
  - `crates/chem/src/lib.rs` (`Entry::evidence`, `Entry::playable`, `Codex::hidden`, `Codex::load_audit`);
  - `crates/chem/src/rules.rs` (the test entry gains `evidence: None`);
  - `crates/bevy-client/src/discover.rs` (test);
  - `tools/chemdb/build_rules.py` (`evidence_of`);
  - `tools/chemdb/build_show_data.py` (AgI and Li3N colours);
  - `content/chem/{core,rules,show,substances}.json`.

Regenerate, from a venv with `chemicals`:

```
python -I tools/chemdb/build_rules.py content/chem
python -I tools/chemdb/build_show_data.py content/chem
scripts/evidence-csv.sh        # content/chem/evidence.csv
```

`evidence.csv` columns:

| column | contents |
|---|---|
| `id`, `name`, `tier` (core or rule), `rule`, `band` | which reaction it is |
| `equation` | the equation with the show record's states |
| `needs` | the conditions it needs |
| `player_sees` | flame, gases, solids, coatings, colour changes, smoke, note, heat, feel |
| `dh_kj_per_mol` | ΔH |
| `evidence`, `checked`, `playable` | the evidence kind and status |
| `source`, `url` | the reference |
| `claim_or_entries` | the claim and the table entries |
| `note` | caveats |

Pages fetched (2026-11-05):

- OpenStax:
  - Chemistry 2e sections 4.1 and 4.2;
  - Biology 2e section 8.1.
- OCR Chemistry PAG 2, Suggested Activity 2.
- Wikipedia (94 articles as the main source; each URL is in `evidence.csv`):
  - the elements and groups: Alkali metal, Alkaline earth metal, Aluminium, Bromine, Halogen, Hydrogen, Iodine, Magnesium, Sulfur;
  - reaction types and processes: Alkane, Combustion, Contact process, Cracking, Downs cell, Electrolysis of water, Flame test, Haber process, Hall–Héroult process, Ostwald process, Reactivity series, Single displacement reaction, Standard electrode potential (data page), Steam reforming, Water gas, Water-gas shift reaction, and others;
  - the compound articles named in each `source`.
