# Critique: I-028 Formula Bench (spec revision 1)

**Verdict: revise.** It is the right item and the right shape. Five things would stop it shipping as written (M1-M5). The rest are small.

Read: `spec.md` (rev 1); `target.md`, `bars.md`, `areas.md`, `gameplay-standards.md` from origin/main (c4832d2); the specs of I-022, I-024, I-025, I-026, I-027, I-029 and I-030; the code at c4832d2 where a claim needed checking (`main.rs` key reads, `hud.js`, `age.js`). The old critiques of I-022/24/25/26/27 were only skimmed for style.

## 1. Is it the right thing under the core loop?

Yes. `target.md` says you learn a spell by building its formula at the Formula Bench at your age band's level. Nothing in play can move a spell out of Unknown today, and B9 and B10 both need it. Real chemistry is the mechanic: one sim function (`bench::judge`) decides, JS never does. Wrong builds fizzle with a kind reason and cost nothing. The camera, aim and cast code are untouched, so the first-person hard rule holds. The bench is a place the player chooses to go, not a quiz that interrupts play (GS-30).

Parts the spec gets right and should keep:

- one judge in sim, with the fuzz test (4);
- the band is never stored, which matches `age.js` and shared devices;
- a separate command queue, so `control` is not edited;
- Sprout never sees a formula, and Fire is hidden at Sprout;
- Apprentice gets count help only after three misses;
- everything is paused while the bench is open.

Two product limits to state plainly, because the panel will hit them:

- **Nothing in play changes after a build until I-029 lands.** The bench gates nothing (test 21), so alone it yields a badge and a tone. That is fine for sequencing, but B9 and B5 get no credit. The B9 "panel prediction" is informational only; keep it so.
- **Learning by copying is cheap on purpose.** A Sprout child copies a picture, so "Learning" proves no recall. That is correct, because mastery needs unaided builds (I-030, whose trial hides every help). Do not let any text call a Sprout build "mastered" or "knows".

## 2. Smallest version

The spec is at the upper limit for one item: a sim crate module, a Bevy module, a bridge change, three band UIs, a chooser, speech, a tone, a 3D pad, 7 sim tests, 4 node tests and 18 golden checks. I would not split it, because the three bands share one file and one judge. I would cut or shrink these:

1. **Cut the automatic open on walk-in** (M1). It removes the `armed` state, the rising-edge rule, the reset re-arm and half of test 13. Keep the pad as a visible landmark.
2. **Fold `Change age` into the chooser rule.** Keep it, but do not test it separately from test 16.
3. **Keep Fire's balance scale.** It is the one piece that makes conservation concrete for a 9-year-old (GS-32). It is the item's biggest UI piece, so the web lane should build it last, and if time runs out, ship Fire with coefficient tiles and per-element counts after Make. Say that fallback in the spec.
4. **`mountBuilder` is in the contract but not in the smallest version.** Either cut it from this item (I-029 adds it) or test and shape it (M3). Today it is an export nothing can fail.

## 3. Must-fix

**M1. The pad opens a modal four metres from the spawn, in the first minute.**

The start is (0, 0, 18). The pad is at (−3.0, 14.5), radius 1.2, about 4.3 m away and about 41° left of the start view. Any child who steps left will walk onto it. The bench then:

- releases the pointer lock;
- pauses the game;
- covers the screen with a dialog;
- the first time, asks "How old are you?"

That is exactly what B4 ("moves within 10 s, lands a cast within 30 s") and GS-23 ("the first minute teaches move, look and cast ... where the player cannot fail") protect. The spec says B4 is "guarded" because the bench "never opens unless the player walks onto it". That is not a guard when the pad sits on the first route. A persona who wanders into it loses the lock and 10 s. The golden path never sees this because it uses `tp`.

Change:

- Remove the walk-on auto-open. Standing on the pad (`atBench`) only adds a class `near` to `#benchbtn`, which gets a static attention ring. `B` and the button open the bench.
- If the leader wants walk-on opening, move the pad at least 10 m from the spawn, off the first quest's route, and open it only after the first cast (`state.casts >= 1`).
- Then delete `armed`, the rising-edge rule and the `Game.epoch` read, and rewrite test 13 as: the pad is visible and on screen; `tp -3 14.5` adds `near`; the bench does not open.

**M2. How the pad opens the bench is not in the contract.**

`BenchState.armed` lives in Rust. The closed JSON is pinned to `{"v":1,"atBench":<bool>,"open":false}`. JS therefore sees only `atBench` and must do its own edge detection, so Rust's `armed` is dead state. The spec also says `run` "does not read or write `Game`", but re-arming on "a reset epoch change" reads `Game.epoch`. M1 removes all of it. If the pad is kept, add a published `ready` bool, or drop `armed` from Rust and state that JS owns the edge.

**M3. I-029 and I-028 do not fit; settle it in I-028, which comes first.**

I-029 (written before this spec) assumes five things I-028 does not provide:

| I-029 assumes | I-028 gives |
|---|---|
| a Rust `Band` resource and a non-debug `band <id>` command | band lives only in JS; Rust gets it only from `open <band>` and keeps whatever the last `open` said |
| `bench::check(spell, band, answer) -> Correct/Wrong(key)` over an "answer string" | `judge(spells, slot, band, text) -> Verdict` over composed text |
| `mountBuilder(el, { band, spell, seed, guided, compact, onSubmit })` returning a handle `{ pick, undo, submit, reveal, showReason, pieces, destroy }` | `mountBuilder(root, { band, spell, recipe, prompt, onMake })`, no return value |
| pieces carry `data-token` and `data-key` | `.bench-atom[data-el]`, `.bench-tile[data-el]`, `.bench-coef[data-term]` |
| "I-021's `band.js`" as the band source | no `band.js` exists on origin/main; the only band source is `age.js` (analytics-on builds only) |

A worked cast happens in play, not inside the bench, so Rust must know the band when the bench is closed. `BenchState.band` today is set only by `open`.

Change:

- Add a non-debug bench command `band <sprout|seedling|apprentice|adept>`. `bench.js` sends it whenever the band resolves (URL, picker click or chooser), including before the bench is ever opened. Rust stores it in `BenchState.band: Option<Tier>`, and I-029 reads that. Add a test: `__bench.cmd('band seedling')` followed by a worked cast.
- Tell I-029 to call `bench::judge` through the same text path, not a new `check`, and to take its option names from the merged I-028. A one-line note in each spec's "Overlap" table is enough.
- Give `mountBuilder` a stated shape and a test (below), or cut it from the contract.
- The same sim `Band`, `judge` and `recipe` should be what I-030's trial calls. I-030 already matches this spec's names, so only I-029 needs the fix.

**M4. A swallowed keyup can leave a movement key held.**

The modal calls `stopImmediatePropagation()` on every `keydown` and `keyup` in the capture phase. Movement reads `keys.any_pressed([KeyW, ...])` from Bevy's `ButtonInput` (`main.rs:827-836`, sprint and jump likewise). If the bench opens while the player holds W (the usual way to walk onto a pad, and the way to press `B` mid-run), the `keyup` never reaches winit. Whether Bevy clears held keys when the canvas loses focus is unconfirmed here. On close, the player may walk forever until W is pressed again.

Change:

- Swallow `keydown` (and `keypress`) only. Let `keyup` pass through; winit ignores a release of a key it never saw pressed, and `hud.js` has no `keyup` listener.
- Add a golden check: hold `KeyW`, press `KeyB`, release W, close; 30 frames later `state.pos` is unchanged.

**M5. Two modals can stack.**

`B` opens the bench "unless the bench or the age picker is already open". I-029's worked-cast overlay (a panel inside `#stage`, with its own capture `keydown`) and I-030's trial live in other layers. `B` pressed while a worked overlay is up would open the bench on top of it and release the lock under a half-built cast.

Change: define one gate. `B` and the pad are ignored while any `[aria-modal="true"]:not([hidden])` other than `#bench` exists. State in this contract that I-029's overlay must carry `aria-modal="true"`. Add a golden check that mounts such an element and presses `KeyB`.

## 4. Should-fix

- **S1. GS-23 is P0 and the walk says "partial".** The standards say the critic rejects a P0 left unaddressed in a screen the spec touches. Discoverability of the bench is the open half: a 6-year-old finds a flask pad and a button that shows only a flask and a `B`. I-029's guided-cast line ("Learn it at the Formula Bench") will help later. Meet it cheaply now: after the first cast and until the first spell is Learning, give `#benchbtn` one static attention ring (no pulse, no sound, no count: GS-28 stays met). Add one check. If the leader accepts "partial", write that sign-off into the spec.
- **S2. GS-21: the chooser is text.** "How old are you?" is not spoken, and Sprout is the group that needs it. Speak it (with the same speech guard) and speak each button's label on focus. The pictures help, but a pre-reader cannot read "4 to 7" either.
- **S3. Sound default.** `hud.js` defaults `sound: true`; the spec says "if sound is on in the saved settings". With no saved settings it must read as on, or test 17's first half fails for the wrong reason. State: absent key = on. If no speech voices exist (common on school Chromebooks) the speak call is a silent no-op; the recipe picture must work alone, which it does. Add that sentence.
- **S4. GS-24 (controls always available).** `B` is a new control and will not be on the pause card's controls list (`hud.js` and `index.html` are not editable). Let `bench.js` append one `B  Formula Bench` row to the pause card's controls list by DOM, or move the README row into this item.
- **S5. Teachers and shared devices.**
  - The chooser asks on every load, while progress now persists (I-027). Fine for privacy; state it in the teacher-facing notes (GS-29): a class link can fix the band with `?band=seedling`, which already works and should be listed.
  - "Start over on this computer" is a precondition for any classroom link, not a note. The spec repeats I-027's "the planner files it". Put it in the backlog before this item's PR is opened, with this item's row marked "do not ship to teachers before".

## 5. Contract and lanes: can they build from it alone?

Mostly yes. The seam (`BenchView` JSON and four command words) is crisp, and the sim, client and web lanes have disjoint files. Gaps a lane would stall on:

1. **`reasonFor(band, result, names, tries)`: `names` has no shape.** The strings need `${name}`, `${formula}`, `${other}`, `${el}` and `${stateWord}`. State: `names` is `{ [slot]: { name, formula } }` built from the published `spells` list.
2. **The target formula is never published.** `SpellCard` has no `formula`, and `recipe` is absent at Apprentice. The Apprentice done line "`H₂O(l)` is right!" and Seedling's `${formula} is right!` have no source except `simplest`, which is empty when the player typed the canonical form. Add `formula: String` (ASCII, from the def) to `SpellCard`.
3. **No reason row for `not_for_band`.** `Code` has it; the table does not; test 10 says "every code × band". Add a generic row and the case. Also add a test that `pick 1` at Sprout is ignored and `try 1 …` returns `not_for_band`.
4. **`judge` indexes `spells[slot]`**, but slots are 1-4 and slot 0 is `NotForBand`. Write `spells.get(slot-1)` (or a lookup by `slot` field).
5. **`mountBuilder` has no test and no return value.** Add a node test (jsdom-free if possible, or a golden check): mounted into a `div` it builds the band's view without the list, chooser or modal handling, fires `onMake` with the composed text, and returns `{ destroy }`. Give pieces `data-token` now, which is what I-029 needs.
6. **Contract says three `main.rs` additions** (`mod`, `add_systems(Update)`, `add_systems(Startup)`). Test 33 says "the one `add_systems` line". Make test 33 say two lines plus the mod line.
7. **Lane `sim` may not touch `content/**`**, yet it owns the prompt rule and parity test over the shipped 11 prompts. Fine, since the data is the stub's. Say that a wrong string in the stub is the contract builder's bug, fixed by the contract builder only.
8. **The web lane cannot see its UI work end to end until the sim and client lanes merge** (the stub judge returns `Parse`). Say so, and give the web lane a local fake: `window.__bench.cmd` with a test-only `__benchState` setter under `?debug`, so the done and fizzle panels can be built and screenshot first. Otherwise lane three idles.

## 6. Can each acceptance test fail?

Yes for 1, 2, 5-11, 13-23, 27-29, 33. Weak or vacuous ones, with fixes:

- **Tests 3 and 4 reuse the same parser as `judge`.** A parser bug that accepts a wrong formula would pass both. Keep them, but add a hand-written table from outside the parser: 12 formulas that must be `Ok` and 12 that must not (for example `H2O2`, `HO`, `NO2`, `CH4 + O2 -> CO2 + 2H2O`, `2H2 + O2 -> 2H2O` for Fire), each with a one-line chemical reason, in `bench.rs` tests.
- **Test 24** passes if there is no bench text. Add `at least one text element found per view` so an empty DOM fails.
- **Test 25** says "its panel's opaque background colour". Cards, badges and buttons have their own backgrounds. Compute against the nearest opaque ancestor.
- **Test 26** passes vacuously if no bench element animates. Add a positive control: without reduced motion, the done icon has a non-`none` `animation-name` with iteration count 1; with it, `none`.
- **Test 19's key leak** is good, but add a held-key case (M4) and `Escape` typed inside the text box (should close, not clear).
- **Test 21's second half** (an Unknown spell still casts) stays true under I-029, which casts Unknown spells through a guided build. Keep it, and note in I-029 that this check is deliberately kept.
- **Test 17** counts only the first `speak`; also assert the fizzle line is spoken, not only the prompt.
- **Test 31** compares closed-bench CPU. Fine. Add an open-bench frame check (`__benchState` writes: at most one per judged try) so a per-frame rewrite fails.
- **Seam check 4** ("`learn` is called at most once per spell") is not observable through the state. Make it concrete: `xp` stays 10 after five correct builds of one spell.

## 7. Is the science sourced?

Mostly. The register plan covers the "very cold" claims ([S1]), the methane prompts ([S7], [S4]) and the conservation sentence ([S7]). Missing or loose:

1. **State claims in the Apprentice prompts.** "liquid water", "liquid nitrogen" and "carbon dioxide gas" each assert a state that the judge then enforces (`(l)`, `(l)`, `(g)`). These need rows: water liquid [S2]; nitrogen liquid at its boiling point [S1]; CO2 gas [S5]. The spec only registers the two methane prompts and the nitrogen "cold" ones. The lint will flag the formulas anyway, so this is a stale-row risk, not a surprise. Add the rows and say what temperature or pressure the state refers to (the cast is a cold-liquid or room-temperature gas claim, and CO2 from a real extinguisher leaves as gas and snow; I-026 chose `CO2(g)`, do not reopen it here).
2. **`2H2O(l)` is accepted and `OH2(l)` is accepted.** The first conflates a coefficient with a subscript, a classic misconception at 11-15. `simplest` is set (the spec says so for a non-canonical text) but only the `Ok` is asserted in test 1. Assert `simplest == "H2O(l)"` for `2H2O(l)` and have the done panel add: "A 2 in front would mean two molecules. One water molecule is H₂O(l)." That sentence is a claim: [S7]. If that is too much, reject a coefficient other than 1 on a single substance at Apprentice with a kind reason; but pick one.
3. **The scale is atoms, not mass.** The spec says the beam tilts toward "heavier" by total atoms. No text says "heavier", but a scale invites it. Keep the UI wording "more atoms" and add the same to the register not-claims note, so no one writes "mass" later.
4. **Seedling shows the state.** The spec says Sprout and Seedling never see state codes, but the Seedling `ok` line interpolates `${formula}`, which is `H₂O(l)`. Either strip the state for Seedling or accept it and fix the sentence in the spec.

## 8. Overlap with I-022, I-024, I-025 and the phase-2 specs

- **I-022** edits the age picker and its safety, and it may change `age.js` and the `#age` markup. The bench depends on `#age button[data-band]` and `#age` visibility. Add a one-line check to the contract builder: re-read I-022's merged `index.html` and confirm those selectors. Order is already "any".
- **I-024** adds a shared low material and scenery post-pass; the pad is spawned outside `world::build`, so it escapes it. Fine. Two small meshes and two materials are well inside the +1.0 ms budget. If I-024 makes materials shared for `?low`, the pad's two materials are still its own; nothing to do.
- **I-025** changes hint priority and stream visibility; no shared hunk. The bench is `inert` over the HUD, so a hint under it cannot be read or focused.
- **I-026 / I-027:** the dependency order is right. The `build-web.sh` shared line is the one real collision; the planner note (tell I-027's builder to add `web/progress.js`) must be done before either builds. I-027's spec still does not list `build-web.sh`.
- **I-029:** see M3 and M5. Also, I-029 registers its own capture `keydown` in `hud.js`; two capture layers (bench and worked) need a stated order. Rule: the bench's capture runs first and only swallows while `#bench` is open; the worked overlay's runs while its panel is open; they never overlap because of M5.
- **I-030:** compatible. It adds `trial` words and view fields to `bench::run` and hooks to `bench.js`. Its `recipe: null` and empty-`atoms` cases need `mountBuilder` to render without a recipe, which I-028's tests do not cover; add that case to the `mountBuilder` test (M3). I-030's "Seedling Fire hides the scale" relies on `.bench-coef` existing without `.bench-scale`; keep the class names exactly as in the contract.

## 9. GS walk

Each item the spec marks "met" checks out against the text, except:

- **GS-23 (P0): partial.** See S1.
- **GS-24 (P0): listed as unchanged but `B` is a new control.** See S4.
- **GS-21 (P0): met for Sprout prompts, not for the chooser.** See S2.
- **GS-7 (P1):** met. The previous pause state is restored on close; test 15 can fail.
- **GS-13 (P1):** met. Focus is trapped, Tab and arrows work, and Escape closes.
- **GS-11 (P0):** met by the button path (test 17). M1 matters more than this one.
- **GS-28 (P0):** met. The attention ring in S1 must stay static and one-time to keep it so.

## 10. Required changes, in order

1. M1: no walk-on auto-open; `near` class on the button; delete `armed` and the epoch read (M2); rewrite test 13.
2. M3: add the `band <id>` command and Rust-side band; stated `mountBuilder` shape and test; tell I-029 to follow `judge` and drop `check` and `I-021 band.js`.
3. M4: swallow `keydown` only; add the held-key check.
4. M5: one `aria-modal` gate; check it.
5. S1-S5 as above. S1 and S2 are P0-driven.
6. Contract gaps 1-8 in section 5.
7. Test fixes in section 6.
8. Register rows for states, the `2H2O(l)` note, the scale wording and the Seedling state sentence (section 7).

After these, I would approve without another critic pass if the leader accepts the `Fire` scale fallback in section 2. A second pass is only needed if M1 is answered by keeping walk-on opening.

## Planner response

All changes accepted; spec.md is now revision 2. No change is rejected.

**Must-fix**
- M1 accepted: walk-on auto-open removed; the pad only adds `near` to `#benchbtn`; test 13 rewritten as "visible, lights the button, never opens".
- M2 accepted: `armed`, the rising edge and the `Game.epoch` read are deleted; `atBench` is display-only.
- M3 accepted: non-debug `band <tier>` command sent whenever the band resolves, stored as `BenchState.band: Option<Tier>` (shown in the closed JSON); `open` takes no argument; `mountBuilder` has a stated shape returning `{ destroy }` with `data-token`, test 11b (incl. `recipe: null`); I-029 alignment recorded in Overlap and as a planner note.
- M4 accepted: only `keydown`/`keypress` are swallowed, `keyup` passes; held-W golden check added to test 14.
- M5 accepted: one `[aria-modal="true"]` gate for `B` and the button; I-029/I-030 overlays must carry `aria-modal`; test 27 extended with a stand-in overlay.

**Smallest version**
- 1 accepted (same as M1).
- 2 accepted: "Change age" stays, covered by test 16 only.
- 3 accepted: Fire scale built last, with the coefficient-tiles-plus-counts fallback written into section 3 (leader accepts the fallback).
- 4 accepted: `mountBuilder` kept, shaped and tested (M3).

**Should-fix**
- S1 accepted: static one-time `hint` ring after the first cast until the first Learning; test 28a; GS-23 now met for discoverability.
- S2 accepted: chooser question and age-button labels spoken under the speech guard.
- S3 accepted: absent settings key or field reads as sound on; no-voice case is a silent no-op, stated; test 17 checks the absent-key case.
- S4 accepted: `bench.js` appends a `Formula Bench | B` row to the pause card table by DOM; GS-24 row added.
- S5 accepted: `?band=` goes to teacher notes; "Start over on this computer" filed before the PR, row marked "do not ship to teachers before".

**Contract gaps (section 5)**
- 1 accepted: `names = { [slot]: { name, formula } }` stated.
- 2 accepted: `SpellCard.formula` (ASCII, with state) added.
- 3 accepted: `not_for_band` reason row added; test 10 covers all 13 codes; Sprout `pick 1`/`try 1` case in test 28b.
- 4 accepted: def looked up by its `slot` field, never an index.
- 5 accepted: as M3.
- 6 accepted: test 33 says the mod line plus two `add_systems` lines.
- 7 accepted: wrong stub prompt data is the contract builder's bug, stated in the sim lane.
- 8 accepted: debug-only `__bench.fake(view)` lets the web lane build panels against the stub.

**Tests (section 6)**
- Tests 3/4 accepted: hand-written 12 + 12 chemistry table (test 3b), independent of the parser.
- Test 24 accepted: at least one text element per view.
- Test 25 accepted: contrast against the nearest opaque ancestor.
- Test 26 accepted: positive control without reduced motion.
- Test 19 accepted: held key covered in test 14; Escape in the text box closes.
- Test 21 accepted: kept, with a note that I-029 must keep it; now five repeat builds.
- Test 17 accepted: fizzle line must be spoken too.
- Test 31 accepted: open-bench write check added to the seam checks (one write per try).
- Seam check 4 accepted: made concrete as "xp stays 10 after five correct builds".

**Science (section 7)**
- 1 accepted: state-claim rows for liquid water [S2], liquid nitrogen at 77.34 K [S1], CO2 gas [S5]; I-026's `CO2(g)` not reopened.
- 2 accepted, option "keep Ok and explain": `2H2O(l)` is Ok with `simplest == "H2O(l)"` and `coef == 2`; the done panel shows a coefficient line, registered as a claim [S7].
- 3 accepted: the scale says "more atoms", never heavier/mass; not-claims note added.
- 4 accepted: Seedling `${formula}` has state stripped; the ok line also says "You are learning" rather than "You learned" (section 1's "learning is not knowing").

**Overlap (section 8)**
- I-022 selector re-check accepted (Overlap row for the contract builder).
- I-029 capture order accepted (section 1) and the alignment row added.
- I-030 `recipe: null` case accepted (test 11b); class names fixed.

**Product limits (section 1)**: both accepted and written under "Bars moved".

Verdict: approve. Every change is taken, M1 is answered by removing walk-on opening, and the leader accepts the Fire scale fallback, so the critic's condition for no second pass holds.
