# Alchemist's Trial v2: design

Status: the plan for the overnight v2 build on branch `claude/playable-v2`.
Inputs: Paawan's playtest of v1 ("extremely basic and slow, half of it doesn't work, the spells don't do anything"), a Playwright audit of v1 (fun 2/10, clarity 3/10), three research briefs (learning games, all-ages UX, tech/perf) and three competing design proposals.

## 0. How the three proposals scored

| Proposal | Fun | Clarity for all ages | Science | Buildable tonight | Verdict |
| --- | --- | --- | --- | --- | --- |
| 1. Kid-first | 7 | 9 | 8 | 9 | Best first 2 minutes, best scope control (Canvas2D glow overlay, WebGL as a stretch). Three rooms is slightly thin. |
| 2. Spectacle-first | 9 | 6 | 7 | 5 | Best payoff: projectile as sim entity, pressure impulse, CO2 that sinks and smothers. Too much for one night (WebGL + 4 zones + marsh + pressure). Claimed 6 reactions; `reactions.json` has 4. |
| 3. Learning-first | 7 | 8 | 10 | 7 | Best science framing: one concept per mechanic, limiting-reagent conservation test, crystal drains charge by ΔH, honest feedback, mastery nudges. |

The synthesis below takes Proposal 1's onboarding and scope discipline, Proposal 2's spectacle and the CO2 callback, and Proposal 3's concept table and science safeguards.

## 1. Goals

1. **Every cast does something visible within 100 ms.** No path from input to "text only". Missing reagents fizzle on screen with a ghost icon and an arrow.
2. **The first spell works with zero gathering**, on every tier, in the first 10 seconds.
3. **Fast:** fixed 60 Hz simulation, ~2x v1 walk speed, no slow-motion ring, decisive spells (a fire goes out in 3 casts or fewer).
4. **Readable by a 5-year-old and respected by an adult:** one reaction shown three ways (pictures + voice, words, balanced equation + sourced ΔH).
5. **The science is the spectacle:** shake, particles, flash, pitch and frost are driven by the sourced ΔH.
6. **Goals are physical** (fire out, ice gone, door open), accept more than one valid solution, and never complete on their own.
7. Keep every hard rule: deterministic sim from a seed, sourced science, strict CSP, no network, < 2 MB, all checks green.

## 2. The concept each mechanic teaches

| Mechanic | Real idea | What the player feels |
| --- | --- | --- |
| Balance scale on the spell bar; leftover atoms float out | Conservation of mass | Atom counts on both pans; extras drift away as labelled orbs |
| Coefficients set power; unbalanced casts run on the limiting reagent | Stoichiometry | Balanced = gold rim, full blast. Unbalanced = smaller effect + visible leftovers |
| ΔH drives the spectacle | Exothermic vs endothermic | Warm flash/shake/rising pitch vs blue frost/falling pitch/crystal charge drains |
| Water, ice, steam in the grid | States of matter | Ice blocks, melts into flowing water, boils into rising steam |
| Fire needs fuel, O2 and heat | Fire triangle | Water cools it, CO2 smothers it, removing wood starves it |
| Electrolysis bubbles at 2:1 | Molar/volume ratios | Two blue bubbles for each red one, every time |

## 3. Core loop (30-90 s per cycle)

See an obstacle (with a bobbing goal icon over it) -> cast with what you already carry -> a bolt flies, hits, and a real reaction changes the grid -> obstacle state changes, a floating label names what happened -> new Grimoire card (first time only) -> walk right to the next room, which needs that card plus one new idea.

## 4. First 2 minutes per tier

**Start screen (all tiers).** Three big picture buttons (sprout / sapling / wizard hat, each >= 96 px, with a short label and spoken name). If nothing is chosen in 8 s, the game starts in Sprout. A gear button (always visible, top-right, >= 64 px) opens the menu: change age, restart room, restart trial, sound on/off + volume, voice on/off, reduced motion, game speed 0.5x/0.75x/1x. Tier and settings live in `localStorage` only (wrapped in try/catch); the age choice is never locked.

### Sprout (4-7, zero reading)
- 0-3 s: tap the sprout picture. The camera shows the player beside a burning hedge (room 1). Mote speaks (Web Speech, local voice, rate 0.9, caption shown): "Fire! Tap the water drop!" A ghost hand pulses over the water-drop card.
- 3-10 s: tap the card -> charge ring, whoosh, blue bolt toward the hedge (auto-aim at the nearest goal if the child has not aimed), splash, hiss, steam billows, the fire meter drops. Two more taps and the hedge is out: chime, star sticker, "You put out the fire!", the gate rumbles open.
- To 2 min: room 2 (ice wall). A new "boom" card appears with a ghost hand; dragging it onto the ice (or tap card, tap ice) melts it into water. Every result is spoken; every miss is a funny puff plus a ghost icon. Atoms are drawn as balls ("two blue + one red"), never subscripts.
- Spell cards: water drop (H2O), bubbles (split water), boom (2H2 + O2), and later cloud (CO2). Cards appear only when their room introduces them.

### Seedling (8-10)
- Same rooms. Input is a tile tray: H2, O2, H2O, CO2, coefficient buttons 1-4, "+", "->". Tiles snap into the spell bar; the balance scale under it tips live.
- 0:00 the bar is pre-filled with `H2O`; Cast works immediately (water bolt).
- Room 2: the bar is pre-filled with `H2 + O2 -> H2O`, unbalanced. Casting gives a weak pop with one floating "O left over" orb; the "2" tiles pulse. Fixing it gives a full blast and a "Balanced!" Grimoire card.
- Prompts are 8 words or fewer, voiced by default (toggle in menu), formula shown under each picture.

### Apprentice (11+)
- One line: "Type a formula or a word (try `water`). [Skip]". Typed input uses the existing parser; hotkeys 1-9 cast Grimoire spells; Tab cycles; Up recalls the last spell.
- The suit starts with a charge of 4 H2 + 2 O2, so `2H2 + O2 -> 2H2O` works within 30 s: floating label `2H₂ + O₂ → 2H₂O  ΔH = −483.6 kJ (NIST)`.
- Hovering a spell or Grimoire card shows ΔH, phase and source. Optional per-room challenge with a local score (casts used, kJ spent, atoms wasted), computed from the seed only.

## 5. Spells and casting

### 5.1 One pipeline, nothing can fail silently
Every cast, on every tier, goes through `Game::cast(text) -> CastOutcome`:
1. **Resolve text.** Trim, alias lookup, then the formula parser. Aliases (case-insensitive): `water` -> H2O, `fire`/`boom`/`pop` -> 2H2 + O2 -> 2H2O, `split`/`bubbles` -> 2H2O -> 2H2 + O2, `hydrogen` -> H2, `oxygen` -> O2, `cloud`/`smother`/`co2` -> CO2, `h20` (digit zero) -> H2O. Unknown text -> outcome `Unknown` (shimmer puff + "not in your Grimoire yet" icon), never an error string alone.
2. **Launch a bolt.** A deterministic projectile entity in the sim (Proposal 2): starts at the player's hand, flies ~6 cells/tick toward the aim, stops at the first solid/liquid cell or the aim point. The reaction happens at impact, not at the player.
3. **Resolve at impact** with whatever reagents the suit holds, using the limiting reagent:
   - `Full` - balanced and enough reagents: full-power effect, gold rim.
   - `Partial` - unbalanced or short: reaction runs to the limiting reagent; leftovers are returned to the suit and shown as labelled floating orbs ("+1 O").
   - `Fizzle` - nothing to react: grey puff at impact, plus a ghost icon of the missing reagent and an arrow (world-space) to its nearest source.
4. **Return an FX event** (section 7) for every outcome, including `Fizzle` and `Unknown`.

Atoms are conserved in every branch: atoms consumed = atoms placed in products + atoms returned as leftovers (tested).

### 5.2 Why the suit always has material (all real technology)
- **Condenser collar (H2O).** An atmospheric water generator: cooled coils condense humid air. H2O starts full (10 casts) and refills 1 cast per 1.5 s; a faint mist streams into the gauntlet while it refills. Gathering from the stream is an instant top-up, not a gate.
- **Wrist electrolyser (H2 + O2).** Splits condensed water using the crystal's charge, always producing H2 and O2 in 2:1 (shown as 2 blue + 1 red bubbles filling the gauge). The "split" spell is this device. Away from a crystal it trickles from a solar strip (slow); touching or standing near a crystal fills it fast with an arc FX. Electrolysis drains crystal charge in proportion to +571.7 kJ per 2 mol (Proposal 3).
- **Starter charge.** Gas jars start with 4 H2 + 2 O2 ("left from your last trial"), so the first three boom casts always work.
- **Air is ~21% O2.** Casting `H2` or `2H2` alone still burns with oxygen from the air: a smaller flame (Partial).
- **CO2** comes from burning methane at the marsh vent in room 4 (product capture), and is the second firefighting spell.

### 5.3 Aim vs cast (fixes the v1 input bugs)
- Mouse move / touch drag aims; a crosshair shows the armed spell icon. Clicking or tapping the world **never** casts.
- Cast = Space or Enter (when focus is not in a button or input), the Cast button, a Sprout card tap, or dropping a dragged card. One code path (`requestCast`), so no double casts.
- Sprout and Seedling get soft auto-aim: if no aim was set in the last 3 s, aim at the current room's goal.
- Gather (G key / button) targets the nearest gatherable source within reach automatically; no right-click instructions anywhere.
- The formula ring/tray never slows time and closes after a cast.

## 6. World and level

- **Grid:** 1280 x 180 cells (four rooms of 320 x 180). Viewport stays 320 x 180 cells (full height, horizontal scroll), rendered pixelated and upscaled.
- **Camera:** follows the player with a deadzone (middle third) and lerp; `set_aim` and pointer-to-grid conversion add `view_x`. Exposed as `view_x()` from Rust so render and input agree.
- **Active region:** the sim steps only columns within half a viewport-width of the camera (`[view_x - 160, view_x + 480)`, narrowed from a full width in the W19 perf pass so a 6x-throttled CPU keeps one step per frame), clamped to the world. Off-screen rooms freeze (the Noita approach). Deterministic because the camera is a pure function of the player's state.
- **Player:** 6 x 12 cells, two-frame walk + jump pose. Max speed 2.2 cells/tick reached in <= 7 ticks; jump -4.5 with variable height (release early to cut); coyote time 6 ticks; jump buffer 6 ticks; landing dust (FX only).
- **Rooms** (left to right; each 2-3 min; each ends in a gate that opens visibly with a rumble once its goal state is true):
  1. **Splash Garden - fire.** A burning hedge in front of gate 1. Fire spread is bounded: a fixed fuel budget (the hedge only, separated from the trees by stone), lower spread probability. A water spell is a 40-cell stream with a cooling radius of 6, so 2-3 aimed casts put it out. A "fire left" meter over the hedge. Goal: no Flame cells and no wood above ignition in the room. The goal requires at least one player water/CO2 hit on the fire (tracked as `doused_by_player`); if the hedge ever burns out by itself, Mote says "It burned out by itself" instead of crediting water, and the gate waits for the player's cast at the embers. Feedback only claims causes the player produced.
  2. **Frozen Gate - heat changes phase.** An ice wall blocks the path above a dry pit. Boom (2H2 + O2) melts it; meltwater pours into the pit so you can swim/wade across; steam rises. Second solution: carry fire (a torch at the room entrance can be lit from the room-1 embers or with boom).
  3. **Crystal Lab - electrolysis costs energy.** Cast water at the crystal pool (or "split" near it): an arc runs from the crystal, H2 and O2 bubble up in a visible 2:1 count and collect under a stone overhang. Igniting the trapped gas (boom or torch) blasts the stone door hinge open. The crystal's charge meter visibly drops with each split.
  4. **Marsh Door - fire triangle.** Marsh vents bubble CH4. A spark lights a sustained fireball (CH4 + 2O2 -> CO2 + 2H2O, -802.3 kJ). The brazier by the final door must be lit, but a second burning hedge blocks it: put it out with water, or capture CO2 from the methane flame and pour the "cloud" spell - CO2 is denser than air (1.98 vs 1.20 g/L at 0 °C), sinks, pools and smothers flames. Lighting the brazier opens the final door: Breakthrough screen.
- **Goal UI:** a bobbing icon over each room's target plus a soft edge arrow when it is off screen, and a small 4-dot room-progress strip at the top. The five-item text objective list is removed.
- **End screen:** breakthrough animation, Grimoire summary (cards found / silhouettes), Replay, Change age, Free play (all rooms open, all spells unlocked).
- **Hint ladder** per room, triggered by 8 s idle or 3 fizzles in a row: pulse the target -> ghost the needed card/reagent -> ghost the full spell (Sprout: ghost hand performs it). Never a dead end.

## 7. Feedback and juice (presentation only; never changes the sim)

- **FX event** (returned by `cast`, and drained each tick via `take_fx()` as compact JSON or a flat f32 array): `{kind, x, y, tx, ty, dh_kj, outcome, leftovers, reaction_id}`. Kinds: `charge`, `bolt`, `impact`, `fizzle`, `unknown`, `splash`, `hiss`, `melt`, `arc`, `bubble`, `door`, `discover`, `land`.
- **Timeline:** input -> sound (< 20 ms) + 150 ms charge ring -> bolt with trail -> impact: single soft flash fade (0-80 ms) -> particles at +50 ms -> floating label at +100 ms at the impact point (tier text; Apprentice adds ΔH + source). The bottom-centre toast is removed.
- **Scaled by |ΔH| (kJ, from `reactions.json`):** shake = clamp(|ΔH|/100, 1, 8) px for 120 ms with exponential decay; particles 10-60; hit-stop 3 frames only when |ΔH| > 400; boom pitch falls as |ΔH| grows.
- **Exothermic:** warm glow, sparks/embers, heat shimmer. **Endothermic (electrolysis):** blue frost vignette, falling pitch, arc from crystal. **Gas products:** rising bubbles. **Leftovers:** labelled orbs drifting back to the player.
- **Particles:** fixed pool of 512 in TS, drawn on an overlay canvas, budget of <= 5 feedback events per second.
- **Glow:** emissive cells (Flame, > 400 °C, Crystal) drawn as additive (`lighter`) soft radial sprites prebaked on offscreen canvases. WebGL bloom only if time allows (section 8).
- **WebAudio synth (no audio files):** charge chirp, whoosh (saw sweep + lowpass), splash/hiss (bandpassed noise), pop/boom (noise + 60->30 Hz sine), bubble plinks, chime (triangle arpeggio), fizzle (falling square), door rumble. `AudioContext` created on first gesture; master/SFX/voice volumes; Sprout volume lower by default.
- **Speech:** `speechSynthesis` only after the first gesture; prefer `localService` voices; `cancel()` before each line; every line has a separate speakable string (no symbols, "two hydrogen plus one oxygen") and a caption. Sprout: on by default; Seedling: on, toggle; Apprentice: off.
- **Accessibility:** `prefers-reduced-motion` or the menu toggle -> shake 0, hit-stop off, particles 30%, no camera overshoot. No flash repeats faster than 3 Hz. Okabe-Ito palette plus per-material texture (ripples, flicker, dots). Text >= 18 px (24 px Sprout/Seedling), contrast >= 4.5:1. Touch targets >= 44 px (>= 64 px Sprout). Full keyboard play, visible focus rings. Game speed slider.
- **Layout:** HUD only at screen edges. On touch: move buttons bottom-left, cast/gather/cards bottom-right; on short screens (< 500 px tall) panels collapse to icons.

## 8. Rendering and performance

Measured in v1: about 1.0 ms per sim step natively, 60 fps everywhere; "slow" was pacing, not frame rate. v2 grows the world 4x, so perf work comes before content.

1. **Fixed timestep:** 60 Hz accumulator, at most 4 steps per frame, leftover time dropped; player and camera interpolated by alpha. Game speed setting scales the accumulator rate.
2. **Const property tables:** `Species::props()` becomes a `static` table indexed by `self as usize`, plus `PHASE`, conductivity and heat-capacity arrays. No behaviour change.
3. **Skip work:** skip Air before any call in `move_pass`; skip heat pairs within 0.01 °C. Active-region stepping (section 6). Chunk sleeping (64 x 64, awake flag + dirty rect) only if the bench below fails.
4. **Render only the viewport:** `render(view_x)` writes 320 x 180 pixels; cache the typed-array view (rebuild when `memory.buffer` changes); a `state_version` counter so JSON state is built only when something changed.
5. **Overlay canvas** for particles, bolt, glow sprites, labels, goal icons.
6. **Stretch, behind a flag:** WebGL1 palette shader (species + temperature `u8` textures uploaded with `texSubImage2D`, palette lookup, noise, flicker, bloom at 160 x 90) with automatic fallback to Canvas2D.
7. **Size:** enable `wasm-opt` only if `binaryen` is installed locally (no build-time download); total download stays < 2 MB.
8. **Budgets (tested):** native release bench of 600 ticks of the full 4-room level averages < 2 ms/step; Playwright at 6x CPU throttle keeps frame time < 16 ms average.

## 9. Science accuracy rules

- Every ΔH, ratio and product comes from `content/reactions.json` (NIST/CRC, cited). The existing Hess's-law tests stay and must pass unchanged.
- New physical constants used for gameplay carry a source comment: CO2 density 1.977 g/L and air 1.293 g/L at 0 °C, 1 atm (CRC Handbook); enthalpy of fusion of water 6.01 kJ/mol and vaporisation 40.65 kJ/mol at 100 °C (CRC). Phase changes stay temperature thresholds unless a latent-heat model is added with these values.
- Limiting-reagent resolution conserves atoms exactly (test). Electrolysis drains crystal charge in proportion to +571.7 kJ (test). The 2:1 bubble ratio is exact (test).
- Feedback text only names causes the player produced. No objective completes without a player cause (test).
- Aliases map to real formulas and the label always shows the formula used, so words never replace the science.
- Simplifications are labelled in code comments (wood as carbon fuel, Flame as a zone not a substance).

## 10. Cut list (from v1)

- Ring slow motion and the ring staying open after a cast.
- Left-click / tap on the world re-casting the last spell.
- Global Enter handler double-casting from a focused button.
- Empty jars at start and every hard jar gate on first casts.
- The five chained text objectives and the objectives panel.
- The bottom-centre toast covering the reaction.
- Forest-wide unbounded fire and the fire objective that completes itself; the false "Water took the heat away" message.
- Mouse-specific instructions ("hold right-click").
- The 🔥 picture on the Methane tile; Cl2 tile/jar (hidden; no source in the level).
- The age choice saved forever with no way back.
- Per-`requestAnimationFrame` ticking.

## 11. Tests that change on purpose

v1 gameplay tests "the fire must spread if ignored", "aiming water at it puts it out" and "the trial stays completable" check behaviour deliberately redesigned here. Likewise `unbalanced_cast_fizzles_for_seedling_with_a_fix`, `sprout_gets_auto_balanced`, `apprentice_gets_no_fix_until_three_misses` (unbalanced casts now resolve as `Partial` on the limiting reagent; the one-tap fix and Sprout auto-balance stay as UI help), `electrolysis_needs_charge_then_conserves_atoms` (now: works with low power away from a crystal, still conserves atoms and drains charge by ΔH) and `release_pours_water_into_the_world` (now via the bolt at impact). All are rewritten, not deleted: the fire still spreads within its fuel budget when ignored, water still puts it out (now in <= 3 aimed casts), and the four-room trial must be completable by a scripted run across 5 seeds. Science tests are untouched.
