# Alchemist's Trial v3: design

Status: the plan for the overnight v3 build, which branches from `claude/playable-v2` once v2 is merged.
Inputs:
- v2 playtests (fun/clarity): 6yo 6/6, 9yo 6.5/6, 13yo 5/8, chemistry teacher 6/8, QA 7/7.
- Four research briefs: depth, progression, classroom, replay.
- Three competing proposals.
- Read-only inspection of the v2 code:
  - `game.rs` is 4.2k lines, `rooms.rs` 3.5k lines and `main.ts` 2.4k lines.
  - `ROOM_W = 320` and `HEIGHT = 180`.
  - Rooms are `ROOMS`-indexed const arrays (`GATE_X`, `gates_open`).
  - `Grid` has `cells`, `temp` and an `aux: i16` scratch field.
  - Fx go out through `take_fx`.
  - There is no Playwright suite yet.

## 0. How the three proposals scored

| Proposal | Fun | Clarity | Science | Serves 10k launch | Buildable tonight | Verdict |
|---|---|---|---|---|---|---|
| 1. More to Play (Foundry rooms 5-8 + Trials + runes) | 8 | 7 | 9 | 5 | 4 | Best verbs and the strongest signature moment (Mg ignores the CO2 spell). Its cost is too high for one night. Widening the world from 1280 to 2560 columns means rewriting every `ROOMS`-indexed array in `rooms.rs`/`game.rs`, and the rune socket UI plus the circuit model is a project on its own. |
| 2. Lab + Living Grimoire | 8 | 7 | 8 | 6 | 6 | Best for Sprout (a toy that reacts to anything) and for collection pull ("Found 9 of 20"). Two content holes: the Fizz ΔH has no source and the limewater phase is unconfirmed. Trials gate the elements, which is fine. |
| 3. Classroom-ready (chapters, Predict, Demo, Teacher page) | 7 | 9 | 9 | 10 | 7 | The only proposal that actually reaches 10k kids. Predict-then-cast is a cheap, strong fun lever for every tier, especially in a room full of kids. It adds little new play for a solo 13-year-old. |

**Synthesis.**
- From Proposal 1: the four new verbs and the Mg rule-break.
- From Proposal 2: the Lab and the silhouette Grimoire with stars.
- From Proposal 3: Predict-then-cast, Demo mode, deep links and the teacher page.

**Core structural decision: new content goes into single-screen 320x180 Trial levels, not into a wider story world.**
- A Trial is a separate `Game` instance built from a level spec of the same width as one v2 room (`ROOM_W = 320`). The story's const arrays therefore never change.
- The v2 story keeps its exact determinism hashes.
- The Lab is the same level type, blank.

**Deferred to v3.1:**
- Runes/circuits. They are the MMO hook, but too expensive tonight; they appear as a teaser silhouette card.
- Share/solution codes.
- Histograms.
- Procedural rooms.
- Thermite, sodium and brine.

## 1. Goals

1. **Fun 8+ on every tier** by adding new verbs and things to find, not more juice:
   - Sprout: toys and surprise.
   - Seedling: collection and a clear twist.
   - Apprentice: par, second solutions and a rule that breaks.
   - Teacher: a ready-to-run lesson.
2. **More verbs than "aim a bolt at the target".** v3 adds:
   - **Catalyse**: a spell whose catalyst comes back afterwards.
   - **Pour**: acid or base into an indicator pool.
   - **Drop**: salts into a flame.
   - **Smother**: sand, the physical fix that works when chemistry fails.
   - **Predict**: guess the outcome before casting.
   - **Paint**: in the Lab.
3. **Something is still left to find after the story ends:**
   - 6 Trials with 3 stars each.
   - A Grimoire of about 20 cards shown as silhouettes.
   - A Lab.
   - A Today's Trial everyone shares.
4. **A teacher can run it cold in one 15-minute period.** That needs:
   - one link,
   - no login,
   - a projector mode,
   - a printable plan,
   - Predict cards that drive class discussion.
5. Keep every hard rule:
   - deterministic sim,
   - sourced science with `reviewed_by`,
   - strict CSP,
   - no network calls,
   - no PII,
   - under 2 MB (target under 200 KB gzipped),
   - 60 Hz on a 4 GB Chromebook,
   - all v2 tests green, with v2 story grid hashes unchanged.

## 2. The concept each new mechanic teaches

| Mechanic | Real idea | What the player feels | Tiers |
|---|---|---|---|
| Catalyst spell on an H2O2 pool makes foam and O2, and the catalyst icon flies back to the suit | Catalysts speed up a reaction and are not used up | A huge foam surge you can walk on, and a brazier that roars when the O2 reaches it | All |
| Red-cabbage indicator moat with vinegar / washing-soda / CO2 spells | Acids, bases, neutralisation, the pH scale | The whole pool cross-fades colour. Overshoot is visible and can be undone | All (Sprout: match the swatch) |
| Magnesium fire that ignores the CO2 spell and flares with water; sand smothers it | Not every fire obeys the fire triangle (Class D fires) | "My spell didn't work?!", then the reveal, then the satisfaction of fixing it with sand | All (Sprout gets a spoken hint) |
| Salts dropped into a flame recolour it | Each element emits its own colours of light (atomic emission) | Coloured fire bursts, then a fireworks finale | All (Apprentice sees nm) |
| Limewater turns milky with CO2 | The classic chemical test for CO2 | Detective moment: "prove the hidden gas is CO2" | Seedling+ |
| Predict card before the key cast | Hypothesis, then test, then explanation | A bet, a surprise and a "why" | All |
| Star 3 = a second, different solution | There is more than one valid chemical route | "Can I do it another way?" | All |

## 3. Core loops

**Story (v2, unchanged except for Predict cards):**
1. 4 rooms, about 8-12 minutes.
2. The end screen now offers **Trials**, **Lab** and **Today's Trial**.

**Trial loop (1-3 min each):**
1. A single screen with one physical goal and one twist.
2. Predict card (optional).
3. Cast.
4. Solve, and earn stars.
5. The solution "signature" is recorded.
6. The Trials menu shows "Solved 1 way · try another" for star 3.

**Lab loop (open-ended):**
1. Paint any matter you have discovered.
2. Cast spells.
3. Watch the chain reactions.
4. The first firing of any reaction reveals its Grimoire card.
5. The "?" glow points at an undiscovered pair nearby.

**Collection:**
- Grimoire silhouettes have sourced hints.
- Each card has three stars:
  - Star 1: found it.
  - Star 2: used it to solve a goal.
  - Star 3 depends on the tier:
    - Sprout: a surprise use.
    - Seedling: cast it balanced on the first try.
    - Apprentice: at or under par.
- The spine shows "Found N of M".
- **Realms** count star-3 cards and reuse the v2 breakthrough FX:
  - Condensation at 4.
  - Foundation at 10.
  - Core at 16.
- Realms never decay.

## 4. New content

### 4.1 Species (appended to the enum; existing ids do not change)

| Species | Notes |
|---|---|
| `Mg` | A solid strip. It ignites from a flame or sustained high temperature. Burning Mg is the existing Flame species tagged `emitter = MgWhite`, plus the per-cell burn progress in `aux`. |
| `MgO` | White powder ash, inert. |
| `Sand` | Powder (falls and piles). Inert. Any Flame cell with Sand on 2 or more of its 4 neighbours is smothered: no oxidiser reaches it. |
| `H2O2` | Liquid. It decomposes very slowly on its own. A catalyst speeds it up. |
| `Foam` | A light solid that you can stand on. It decays back to H2O after about 20 s; the timer lives in `aux`. |
| `Indicator` | Red-cabbage water. A liquid with pH ×100 stored in `aux`. It does not boil or freeze in v3: a documented simplification, since no Trial heats it. |
| `Limewater` | Ca(OH)2(aq). A liquid. |
| `Chalk` | CaCO3. A fine white precipitate that drifts down. |

**Per-cell tag.**
- `Grid` gains one new `tag: Vec<u8>`, used only by Flame cells. It holds the emitter:
  - None, Na, Li, K, Cu, Sr or MgWhite.
- 180 × 320 bytes per Trial is negligible.
- The story grid never sets it, so story hashes stay the same as long as the hash ignores `tag` when it is zero. The hash function must keep that property.

### 4.2 Reactions (each goes into `content/reactions.json` with a source and `reviewed_by: null` until the chemistry teacher signs)

| id | Equation | ΔH (kJ, as written) | Trigger | Source / reviewer flag |
|---|---|---|---|---|
| `mg_combustion` | 2Mg + O2 -> 2MgO | -1203.4 | Flame or heat next to Mg with O2/air | NIST ΔHf MgO(s) -601.7 |
| `mg_co2` | 2Mg + CO2 -> 2MgO + C | -809.9 | Burning Mg next to CO2 (the rule-breaker) | NIST ΔHf (CO2 -393.5); RSC "burning Mg in CO2" demo |
| `mg_steam` | Mg + H2O -> MgO + H2 | -359.9 | Burning Mg next to Water or Steam: a flare | NIST ΔHf H2O(g) -241.8 |
| `h2o2_decomp` | 2H2O2 -> 2H2O + O2 | -196.1 | Catalyst spell (needs: `catalyst`, not consumed) | NIST ΔHf H2O2(l) -187.8, H2O(l) -285.8 |
| `neutralisation` | H+ + OH- -> H2O | -55.8 per mol | Acid/base spell into Indicator. The card says the weak acid (vinegar) releases slightly less | CRC; **reviewer: confirm -55.8 vs -57.3** |
| `co2_dissolves` | CO2 + H2O ⇌ H2CO3 | n/a (shown as an equilibrium) | CO2 next to Indicator lowers pH toward a floor of ~5.6 | Standard carbonate chemistry; **reviewer** |
| `limewater_test` | Ca(OH)2 + CO2 -> CaCO3 + H2O | -114.7 (solid-phase ΔHf) | CO2 next to Limewater gives Chalk | NIST/CRC; **reviewer: confirm phase** |
| `flame_test` (5 light cards) | Not a reaction: atomic emission | n/a | A salt bolt hits a Flame and sets `tag` | NIST ASD: Na 589.0/589.6 nm, Li 670.8, K 766.5/769.9, Cu blue-green (CuCl bands), Sr red (SrOH/SrCl molecular bands, flagged) |

**Spells.**

| Spell | Typed (Apprentice) | Seedling tile | Sprout picture |
|---|---|---|---|
| Catalyst | `2H2O2 -> 2H2O + O2` | "yeast + H2O2" | yeast cup |
| Acid | `CH3COOH` | "vinegar" | lemon |
| Base | `Na2CO3` | "washing soda" | soap bar |
| Sand | `SiO2` | sand | bucket |
| Salts | `NaCl`, `LiCl`, `KCl`, `CuCl2`, `SrCl2` | salts | coloured gems |

- The sand spell is physical, not a reaction: the suit carries a hopper.
- Each salt card shows its element glyph, so colour is never the only cue.
- All new casts go through the existing `Game::cast` pipeline (bolt, impact, ΔH-scaled FX). That keeps the v2 rule that every cast does something visible within 100 ms.

**The pH model, accurate for strong acid/base mixing.**
- Each Indicator cell stores pH ×100.
- When two cells mix, convert each to net excess `e = 10^-pH − 10^(pH−14)`, average them, then convert back.
- Each spell adds a fixed excess.
- Palette ramp:

| pH | Colour |
|---|---|
| ≤3 | red |
| ~5 | pink |
| ~7 | purple |
| ~8 | blue |
| ~10 | green |
| ≥12 | yellow |

- Source: ACS ChemMatters, red cabbage.
- The HUD pH strip always shows a number for Seedling+ and a word for Sprout ("sour / middle / soapy").

**Safety cards** carry the line "trained chemists only" and never give quantities or an ignition method. They go on:
- Mg
- H2O2 (the high-concentration warning)
- the flame salts (Cu and Sr compounds are toxic)

Thermite, Na + water, brine/Cl2 and lead compounds are out of v3.

### 4.3 Trials (single-screen 320x180 levels; each has at least 2 scripted solutions in CI)

| # | Trial | Goal and twist | Solution A | Solution B | Tiers |
|---|---|---|---|---|---|
| T1 | **Foam Bridge** | Cross a pit; a dying brazier sits beside a stagnant H2O2 pool | Catalyse the pool, and foam fills the pit to walk across | Send the O2 to the brazier, which lifts the counterweight platform | All |
| T2 | **Colour Moat** | The door opens when the moat reads purple (pH 6-8). The moat starts basic and green | Vinegar spells (overshoot to red, fix with washing soda) | CO2 cloud spell, which dissolves and lowers the pH | All |
| T3 | **White Fire** (signature) | Burning Mg strip next to a wooden bridge. The CO2 spell does nothing (`mg_co2` fires and Mg keeps burning), and water flares it | Sand smothers the Mg | Wet the wooden bridge to protect it and let the strip burn out to MgO | All (Sprout: Mote says "This fire is different! Try sand") |
| T4 | **Flame Lock** | A rune shows 3 coloured flames to match in the brazier | Na, Cu, Li for yellow, green, red | Na, Cu, Sr: two elements both read "red" (Apprentice sees 670.8 nm vs band) | All |
| T5 | **Milky Proof** | A sealed jar of unknown gas; prove it is CO2 by turning the limewater cup milky | Vent the jar gas onto the limewater | Burn the wood (carbon combustion) to make CO2, then let it sink onto the limewater | Seedling+ |
| T6 | **Foundry Finale** | Mg fire in front of a furnace with a foam pit and a colour rune, in one screen | Sand + foam + salts | Wet the bridge + O2 to the brazier + a different salt pair | All (tier rules simplify it for Sprout) |
| T7 (stretch) | **Lean & Rich** | Light a CH4 lantern without exploding the ceiling pocket (5-15% flammability limits, NFPA 497) | Dilute with CO2 | Burn it off at the vent while it is lean | Apprentice |

- Sprout sees T1-T4 and T6, with unlimited reagents.
- Seedling and Apprentice see all of them, with limited reagents.

### 4.4 Lab

- A blank 320x180 level with a stone floor and walls, using the same `step()` function.
- The player stands in it and casts as usual. A palette paints any **discovered** species, plus Stone, Water, Wood and Ice, which are always available.
- Controls:
  - pause
  - step one tick
  - speed 0.25x, 1x or 2x
  - the insight lens
- Painted matter is counted as "spawned" in the atom ledger, so nothing is created silently.
- Sprout gets:
  - 8 big picture jars
  - a big brush
  - 3 one-tap **preset tableaux** ("Volcano shelf", "Foam party", "Rainbow fire"), so it is never a blank page
- Seedling: named species and 3 brush sizes.
- Apprentice: formulas, ΔH on hover and the atom ledger.
- The "?" glow sits over a pair of adjacent species that has an undiscovered reaction. It can be switched off.

### 4.5 Today's Trial

- Uses a hash of the local date (`YYYY-MM-DD`) to pick a Trial id from the tier's list, plus a noise seed.
- Everyone in a class gets the same Trial that day, with no server.
- The UI shows "Days played: N" and never shows a streak. Missed days cost nothing.

## 5. Scoring and stars (Rust, deterministic)

`game/score.rs` returns `TrialResult` with these fields:
- `solved`
- `casts`
- `hints_used`
- `balanced_all`
- `atoms_wasted`
- `kj_net`
- `sig`

**The signature.** `sig` is a 32-bit hash of the sorted set of reaction ids that changed goal-relevant cells, plus the goal mechanism id. Two solutions are "different" when their `sig` differs. The scripted solutions in CI must produce distinct sigs.

| | Star 1 | Star 2 | Star 3 |
|---|---|---|---|
| Sprout | Solved | No hint used | A second, different `sig` |
| Seedling | Solved | Every cast balanced | A second, different `sig` |
| Apprentice | Solved | Casts at or under par (par = the best scripted solution) | A second, different `sig` |

- Apprentice also sees atoms wasted and net kJ against par.
- Histograms are deferred to v3.1. The headless solver data needed for them comes for free from the CI scripted solutions.
- The v2 `scoreCast` story score in `main.ts` stays as it is.

## 6. Grimoire v3 and progression

- Every card shows from the start as a silhouette with a sourced hint ("Something white that even CO2 can't stop").
- A card is revealed the first time its reaction fires **anywhere**: story, Trial or Lab. The trigger is a new `discover` Fx emitted by the grid/chem code, carrying the `reaction_id`.
- Each card has 3 stars and a "Seer" eye mark for a correct Predict. The mark is cosmetic.
- Spine counter: "Found N of M". M is about 20 cards:
  - 4 v2 reactions
  - 3 phase changes
  - 7 new reactions
  - 5 light cards
  - 1 locked "Rune" teaser marked "coming soon"
- **Chain card**: `co2` fires and then `limewater_test` within 300 ticks. This is a bonus "oh!" moment.
- **Realms** (Condensation, Foundation, Core) come from star-3 counts and reuse the v2 breakthrough animation.
- **Cosmetic surprises at star milestones:** bolt-trail colours from the flame-test palette and Mote hats. They are never bought, never timed and never gate content.
- **Save:**
  - One `localStorage` key, `alch.v3`, with every access in try/catch.
  - Schema `{v:3, cards:{id:{found,stars,seer}}, trials:{id:{stars,sigs[]}}, lab:{unlocked[]}, days:[]}`, capped at about 4 KB.
  - If storage throws, the end screen shows a 12-character progress code: base32 of the bitfields plus a CRC8. It can be typed back in.

## 7. Predict-then-cast

- `content/predict.json` has entries keyed by `story:1-4` and `trial:T1-T6`. Each entry has:
  - `prompt`
  - `options[2-4]`, each with `picture`, `text` and `tier_text`
  - `answer`
  - `why`
  - `source`
- The card appears when the player enters the room or Trial and **pauses the sim** through the existing speed control.
- Sprout gets pictures and voice, Seedling short words, and Apprentice adds numeric options on 2 cards (for example "How many kJ for 2 H2?").
- After the key cast, a reveal line explains the result: "You guessed X. Here's why it was Y."
- The cards can be skipped, and are never shown twice in a session.
- At least 10 cards, written to make good class discussion. Examples:
  - "What will water do to the fire?"
  - "1:1 or 2:1 bubbles?"
  - "Will the CO2 cloud put out the white fire?"
  - "Can we get the ice back?"

## 8. Classroom layer

**Deep links**, parsed in `web/src/route.ts`:

| Parameter | Meaning |
|---|---|
| `?tier=sprout\|seedling\|apprentice` | Skip the age picker |
| `?mode=story\|trials\|lab\|daily` | Start in that mode |
| `?room=1-4` | Story room |
| `?trial=T1..T7` | Trial |
| `?c=K7QX` | Class code (random, no PII) |
| `?demo=1` | Demo mode |

Unknown values are ignored safely. `?debug` keeps its old behaviour.

**Classroom defaults when `?c` is present:**
- SFX low, speech off, captions on.
- A lens **button**, as an alternative to holding Tab.
- Storage-blocked play works.

**Demo mode** (`?demo=1`):
- Projector scale: 1.5x HUD and 32 px captions.
- A cursor halo.
- The Predict card goes full-screen with large A/B/C/D options. The teacher taps the class vote; there is no device polling.
- A bottom bar with freeze, 0.25x and "show equation at any tier".
- A room/Trial picker.
- Clicker keys: PageUp/PageDown/B.
- The 5-second replay is deferred.

**"Show your teacher" card** at the end of a story run or a Trial set. It shows:
- a large tick
- the tier
- rooms or Trials cleared
- predictions correct
- a 6-character completion code (chapter + class code + score + checksum)

**`web/teach.html`** is a static page with the same CSP and no third-party requests. It contains:
- A per-band class-link generator, with a copy button and a QR code. The QR encoder is inlined, about 8 KB; if it is cut, show a short URL instead.
- The 15-minute plan per band:

| Minutes | Activity |
|---|---|
| 0-2 | Projector demo and a vote on a Predict card |
| 2-12 | Students play (story, or Trials for fast finishers) |
| 12-15 | Exit prompts |

- An NGSS table: K-2 2-PS1-1/-4; 3-5 5-PS1-1/-2/-4; 6-8 MS-PS1-2/-4/-5/-6; HS HS-PS1-2/-4/-7. **Verify these against nextgenscience.org before launch.**
- A "who checked the science" line.
- An "if something goes wrong" box.
- A print stylesheet with exit tickets.

**`privacy.html` and `accessibility.html`** are one screen each:
- "We collect nothing about your students."
- An IT note.
- An accessibility statement.

**Avoid "game" in the domain or path.** Getting the domain re-categorised with school filters is an ops task outside this build.

**Analytics stay separate.** `route.ts` only exposes `classCode()` and `ageBand()` getters. This build makes no network calls.

## 9. Per-tier experience (why each reaches 8+)

**Sprout (6yo, 6 → 8+).**
- Lab presets and big picture jars give a Toca-style toy.
- Spectacle: foam surge, white fire, rainbow flames.
- Picture Predict cards ("guessing wrong is funny").
- Stickers and Mote hats.
- Trials have unlimited reagents and a spoken twist.

**Seedling (9yo, 6.5 → 8+).**
- "Found 9 of 20" collection pull.
- 3-star Trials.
- pH overshoot as "undo".
- Today's Trial to compare with classmates.
- Limewater detective.

**Apprentice (13yo, 5 → 8+).**
- The Mg rule-break: they *learned* that CO2 smothers fire, and now it doesn't.
- A second solution for star 3.
- Par for casts, atoms and kJ.
- Numeric Predicts.
- The Lab for chain reactions.

**Teacher (6 → 8+).**
- Lesson-length links.
- Projector Predict votes.
- Sourced cards with a named reviewer.
- A printable plan.

**QA (7).** Replay via stars and dailies; regressions guarded by golden paths.

## 10. Accuracy, accessibility, performance

- Every new value in `reactions.json` has a source. The chemistry-teacher playtester signs `reviewed_by` before classroom launch, and the reviewer flags above must be resolved.
- Simplifications are listed on the cards:
  - the indicator does not boil;
  - the sand smother rule is "oxidiser blocked";
  - foam decay is a fixed timer.
- No hint depends on colour alone:
  - pH has a number or word;
  - flame salts have element glyphs;
  - the Trial goal icon has a texture.
- The Mg white flash is capped at 3 Hz and replaced by a static white bloom under reduced motion.
- Trials and the Lab are 320 columns wide, smaller than one v2 frame's active region, so they are cheaper per frame than the story.
- The bundle should grow by 40 KB gzipped or less. The gate is under 200 KB gzipped, against a hard limit of 2 MB.

## 11. Forbidden (applies to v3 and later)

- No streak loss, or "streak" wording.
- No loot boxes and no random paid rewards.
- No energy or wait timers.
- No in-game currency for buying anything, and no shop.
- No push nags.
- No leaderboards with names.
- No fake scarcity ("only today!"). Old dailies stay playable.
- No learning gated behind cosmetics. Card science text is readable as soon as the card is revealed.
- No student PII, free-text fields or persistent cross-session identifiers.

## 12. Build contract (Hour 0, done by the lead before the tracks fork, about 45 minutes)

1. **Species enum additions**, appended in the §4.1 order, plus the `Grid.tag` field (zero means none; the hash ignores zero tags).
2. **Reaction ids** (§4.2) and **Fx kinds:**
   - `discover{reaction_id}`
   - `foam`
   - `mg_flare`
   - `ph_shift{ph}`
   - `salt{element}`
   - `smother`
   - `catalyst_return`
3. **The chem hook:**
   - `crates/sim/src/chem.rs` gets `pub fn cast_extra(game_ctx, spell) -> Option<CastOutcome>` and `pub fn grid_rules(grid, x, y)`, both stubs returning None or doing nothing.
   - One call to each is added to `Game::cast` and to the grid step.
   - After the contract, Track A owns `chem.rs` and never edits `game.rs`.
4. **The level hook:**
   - `crates/sim/src/game/levels.rs` gets `LevelSpec`, `Game::new_level(tier, LevelSpec, seed)` and `TrialResult`, as stubs.
   - The `WebGame` exports in `web.rs`, stubbed:
     - `new_trial(tier, id, seed)`
     - `new_lab(tier, seed)`
     - `trial_list(tier) -> json`
     - `trial_result() -> json`
     - `paint(x, y, species, r)`
     - `erase(x, y, r)`
     - `set_paused(bool)`
     - `step_once()`
     - `set_speed(f32)`
     - `is_story() -> bool`
5. **Web:**
   - `web/src/bus.ts`: a typed event bus with these events:
     - `screen:change`
     - `room:enter{room}`
     - `trial:enter{id}`
     - `trial:solved{result}`
     - `cast:result{outcome}`
     - `predict:open|answer|reveal`
     - `discover{id}`
   - `main.ts` emits these from its existing code paths. That is about 10 one-line `bus.emit` calls, plus `initRoute()`, `mountScreens()` and `installTestHooks()` calls.
   - Stub files:
     - `route.ts`
     - `screens.ts`
     - `progress.ts` (save API: `markFound`, `addStar`, `markSeer`, `recordSig`, `load`, `exportCode`, `importCode`)
     - `testhooks.ts`
   - `testhooks.ts` exposes `window.__alch` only under `?test=1` or `?debug`, read-only:
     - `{screen, tier, room, goals, lastCast, trialResult, cards}`
     - plus `cast(text)` and `aim(x,y)` helpers
6. **Content stubs:** `content/predict.json` with its schema, and the save schema written into this document.

**File ownership after Hour 0:**
- Track A: `species.rs`, `palette.rs`, `reactions.rs`, `chem.rs`, `grid.rs`, `spell.rs`, `formula.rs`, `content/reactions.json`.
- Track B: `game.rs`, `game/rooms.rs` (read only in practice), `game/levels.rs`, `game/trials.rs`, `game/lab.rs`, `game/score.rs`, `web.rs`, `crates/sim/tests/trials_*.rs`.
- Track C: `main.ts` screen wiring, `screens.ts`, `trials.ts`, `lab.ts`, `progress.ts`, `grimoire.ts`, `book.ts`, `cards.ts`, `tray.ts`, `icons.ts`, `fx.ts`, `audio.ts`, `v3.css`.
- Track D: `route.ts`, `predict.ts`, `demo.ts`, `demo.css`, `content/predict.json`, `web/teach.html`, `teach.ts`, `teach.css`, `privacy.html`, `accessibility.html`, `vite.config.ts` (multi-page).
- Track E: `web/e2e/**`, `web/playwright.config.ts`, `package.json` and the lockfile (devDependency only), `testhooks.ts`, CI scripts, `scripts/size-gate.*`.

### 12a. The contract as built (Hour 0 commit)

Everything below exists on `claude/v3` and compiles; every stub is a no-op, so the v2 story plays and hashes exactly as before. `crates/sim/tests/story_hash.rs` pins the v2 story grid hashes (FNV-1a, values taken on main `52941cb` before any v3 change); **every merge must keep it green**.

**Sim (Rust)**

| Item | Where | Notes |
|---|---|---|
| Species `Mg`=14, `MgO`=15, `Sand`=16, `H2O2`=17, `Foam`=18, `Indicator`=19, `Limewater`=20, `Chalk`=21 | `species.rs` | Mg, MgO, Sand, Foam, Chalk are `Phase::Solid`; H2O2, Indicator, Limewater are `Liquid`. `is_powder()` (Sand, MgO, Chalk) and `is_v3()` helpers. Real molar masses; heat values are first-cut tuning. Placeholder looks in `palette.rs`. `web/src/gl.ts` lens solid list updated (gpu_ids test). |
| `Emitter` (None=0, Na, Li, K, Cu, Sr, MgWhite) | `species.rs` | `from_u8`, `element()` glyph. |
| `Grid.tag: Vec<u8>` | `grid.rs` | Moves with the cell in `swap`, copied by `copy_columns`, cleared by `set`/`put`. Direct writes to `grid.cells[i]` do **not** clear it. |
| `Grid.events`, `Grid::event`, `Grid::take_events` | `grid.rs`, `events.rs` | `ChemEvent {kind, x, y, reaction_id, value}`, capped at 128. `Game::tick` drains them into `Fx` after the grid step. |
| `Grid::content_hash()` | `grid.rs` | FNV-1a of species, temp bits, aux, and tag only where non-zero. |
| Fx kinds `discover`, `foam`, `mg_flare`, `ph_shift`, `salt`, `smother`, `catalyst_return` | `events.rs::fx_kind` | Payload per kind documented there (`ph_shift`: power = pH; `salt`: reaction_id = `flame_test_<el>`, power = emitter id; `discover`: reaction_id = card id). `Fx` struct unchanged; `Fx::at(kind, x, y, power)` helper. |
| Reaction / card ids | `reactions.rs::ids` | 7 reactions + 5 `flame_test_*` light cards. Not yet in `content/reactions.json` (Track A adds each with a source and `reviewed_by: null`). |
| `chem::cast_extra(&mut CastCtx, text) -> Option<CastOutcome>` | `chem.rs` | Called first in `Game::cast`; `Some` claims the cast. `CastCtx` lends grid, jars, book, tier, hand, aim, ticks, discovered; push `Fx` to `ctx.fx`; set `ctx.launch = Some(ExtraShot)` to fly a normal bolt. `CastOutcome::new(id, input, outcome, spell, label)` builds a blank outcome. |
| `chem::impact_extra(&mut ImpactCtx, &ExtraShot, at)` | `chem.rs` | Runs when an `ExtraShot` bolt lands (after the normal `impact` Fx; the game adds `discover` if `first`). Raise Fx with `grid.event`. |
| `chem::grid_rules(grid, x, y)` | `chem.rs` | Called from the chemistry pass for every v3-species cell not reacted this frame. Flame-cell rules (smother, tags) need Track A to route Flame cells here too in `grid.rs`. |
| `chem::V3_SPELLS` | `chem.rs` | (word, formula, spell) table: catalyst, acid, base, sand, salt x5. The tray casts the formula column. |
| `LevelSpec`, `LevelKind`, `TRIALS` (T1-T7 with tiers), `trial_list(tier)`, `TrialResult`, `Level`, `Game::new_level`, `Game::is_story`, `Game::trial_result`, `Game::paint`, `Game::erase` | `game/levels.rs` | Stub level = blank 320-wide stone room (columns past 320 are stone). `Game.level: Option<Level>`; story room logic and free play skip when it is set. `TrialResult.stars` = stars this run earned alone (0-2); star 3 comes from `progress.recordSig`. |
| `score::solution_sig(ids, mechanism) -> u32` | `game/score.rs` | FNV-1a 32 of sorted, deduped reaction ids + mechanism. Also empty `game/trials.rs`, `game/lab.rs`. |

**WebGame exports (`web.rs`)**: `new_trial(tier, id, seed) -> bool`, `new_lab(tier, seed)`, `new_story(tier, seed)` (extra: back to the story), `trial_list(tier) -> json`, `trial_result() -> json | "null"`, `paint(x, y, species_u8, r)`, `erase(x, y, r)`, `set_paused(bool)`, `is_paused()`, `step_once()`, `set_speed(f32)` (0..4, applied inside `tick`), `is_story()`, `level_id()`. They swap the game **inside** the one `WebGame`, so the page keeps one object and its render buffers. `restart`/`restart_room` rebuild the level in a Trial or the Lab; `goals()` is `[]` there until Track B adds Trial goals.

**Web**

| File | Contents |
|---|---|
| `bus.ts` | Typed `on/once/off/emit/lastOf`. Events: `screen:change{screen,prev}`, `room:enter{room}` (1-based), `trial:enter{id}`, `trial:solved{result}`, `cast:result{outcome}`, `predict:open{key}`, `predict:answer{key,choice,correct}`, `predict:reveal{key,correct}`, `discover{id}` (may repeat; dedupe). A throwing listener is logged and skipped. |
| `route.ts` | `initRoute()`, `parseRoute(search)`, `route()`, `classCode()`, `ageBand()`; parses tier, mode, room, trial, c, demo, debug, test. |
| `screens.ts` | `mountScreens({tier, game})`, `screenHost()`, `showScreen(s)`, `currentScreen()`. Screens: start, story, end, trials, trial, lab, grimoire. |
| `progress.ts` | Save API on key `alch.v3` (schema in the file header, as in §6; `stars` is a bitmask 1/2/4): `load`, `storageOk`, `markFound`, `addStar(kind, id, star)`, `markSeer`, `recordSig -> distinct count`, `markDay`, `exportCode`/`importCode` (stubs), `sanitize`. |
| `testhooks.ts` | `installTestHooks(src)`: `window.__alch` under `?test=1` or `?debug`, frozen, getters return copies: `screen, tier, room, goals, lastCast, trialResult, cards`, plus `cast(text)` and `aim(x, y)`. |
| `types.ts` | New Fx kinds and spell names, `SPECIES` id map, `LevelSpec`, `TrialResult`. |
| `main.ts` | Calls `initRoute()`, `mountScreens()`, `showScreen()` (start, story, end), `installTestHooks()`, and emits `cast:result`, `discover`, `room:enter`. No `trial:*` or `predict:*` emits yet: those code paths do not exist until Tracks C and D add them. |

**Content**: `content/predict.json` holds the card schema and one sourced example (`story:3`, the 2:1 split).

## 13. Schedule and merge order

| Time | Work |
|---|---|
| H0 | Contract commit |
| H0-H4 | Tracks A and B on the sim in parallel. C and D build against the stubs and a mock Fx/event stream. E writes the harness and the v2 story golden paths, which must pass on the contract commit. |
| H4 checkpoint | Merge A, then B (`cargo test` green, story hashes unchanged), then rebuild WASM. |
| H4-H7 | C and D integrate against the real exports. B finishes Trial solutions. |
| H7 | Merge C, then D. E runs the full golden-path suite, the 6x-throttle perf check, the storage-blocked run and the size gate. |
| H8 | Fix, then tag v3.0. |

**Cut order if the night runs short:**
1. T7
2. QR code (fall back to a short URL)
3. Chain card
4. Realms
5. Cosmetic trails
6. T5 with Limewater/Chalk
7. Lab "?" glow
8. Demo slow-mo

**The irreducible core:**
- T1-T4 and T6 with stars
- Mg, foam, pH and salts
- the silhouette Grimoire
- a Lab with presets
- Predict cards on story rooms
- deep links with `?demo` and `?c`
- the teacher page

## 14. Measurement

- Re-run the same 5 testers twice: after the H4 merge (sim-only builds via debug links) and after v3.0.
- Pass bar: fun ≥ 8 for every tester and clarity no lower than v2.
- If the 13-year-old is below 8, the next levers are histograms and solution codes (v3.1).
- Launch-side metrics come from the separate analytics work:
  - class codes seen,
  - Trials started and solved per age band.
