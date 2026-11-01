# I-037 round 1: Builder, all lanes (world, core, web, tester's golden section and perf --query)

Wave 13. Branch `loop/I-037`, worktree `.claude/worktrees/w13-I-037`, built on origin/main cdd719e (dff8336 plus
loop-state docs only), merged into the spec branch at the start of the build (dfcc3b7).

## What a player gets

Open the Frozen Gate and a dark arch in its north wall lights up ice-blue. The banner says "Walk to the glowing
arch to visit the Marsh", and 3 s later the finder arrow points at it. Walk in and you are on a plank boardwalk
in a misty marsh, with reeds and water either side and a lantern on a hummock at the far end:

- Twice the boardwalk runs through a low, muddy place where marsh gas bubbles up and burns in low blue flames
  across the planks. Burning flames are too hot to pass.
- Water or Frost only knocks a flame down to 40 % for 1.5 s; Fire on a flame changes nothing. Each says why, at
  the player's band, and points at CO2.
- CO2 sinks into the low place: a pale haze rises with each cast. The shallow dip fills in 2 casts, the deeper
  hollow in 3 (0.34, 0.68, 1.00). Nothing goes out until the pool is full; then every flame in it goes out at
  once and stays out (Fire on it fizzles: no oxygen).
- Walk to the lantern: "You cleared the Marsh!". The walk stops at the win, so held keys do not carry you into
  the lantern; a fresh key press walks again, to the back arch, which returns you to the Frozen Gate at its open
  north arch.

`?level=3` opens the Marsh directly (no debug needed), and CO2 can be cast there on a fresh page. Screenshots:
`evidence/l3-01-arrive.png`, `evidence/l3-02-dip-pooled.png`, `evidence/l3-03-won.png`.

## Order of work

1. Contract stubs: `Kind::GasVent`, `Kind::Hollow`, `Obj.co2` / `co2_step` / `flicker`, `Obj::hollow`,
   `World.hollow_of` (pure helper `chem::hollow_of`, computed in `World::with_goal`), the events `Flicker(Spell)`,
   `Filling`, `Pooled`, `Spilled`, the constants, and `level3::layout` with `caption` returning `None`.
2. Acceptance tests first, exactly as specced: chem tests 1-3 (4 tests), level3 tests 4-5 (4 tests), golden
   `marsh` (tests 6-17 and 20, `ONLY=marsh`), the pinned `I037_KEYS` key order (18), `perf.mjs --query level=3`
   (19). On the stubs: standalone `rustc --test src/chem.rs` 24 passed / 4 failed, native client 82 passed /
   7 failed (all the new behaviour tests). The golden section, run against the main build (d32a4ca pkg-out),
   failed every check it reached and then stopped (no dip vents). Evidence: `evidence/tests-first-red.txt`.
3. The build in the three lanes, committed with the tests (aec6832), then evidence commits.

## Contract notes and decisions

- **`Obj.co2`, not `World.pools`.** No test builds `Obj` with a struct literal (only `Obj::new` does), so the
  fill lives on the Hollow object (`co2`, plus `co2_step` = its per-cast fill and `flicker` on vents). No
  existing test was edited.
- **Events.** `NoChange` is reused for Fire on a burning vent (it already means "nothing changed"). New:
  `Flicker(Spell)`, `Filling`, `Pooled`, `Spilled`. Their engine default captions are the Apprentice lines, so
  `level3::caption` returns `E::Filling.caption()` etc. (as level 2 did). `hit_kind` treats `Pooled` as `Out`.
  `Filling` and `Pooled` are emitted on the Hollow (one event), whether the cast hit a vent or the floor.
- **Extra lines (not in the table, registered as Not claims):** Water/Frost on a vent or floor whose flames are
  already out gives `Soaked`/`Chilled` with "These flames are already out." (otherwise the courtyard's "it must
  dry out before it can burn" line would show). Any spell on the lantern says "That is the lantern. Walk up to
  it!" (the I-049 follow-up for level 2, not repeated here). Both keep captions matching the actual state.
- **Vent positions.** The hollow's vents are drawn at x about ±0.85 (each covering its half of the boardwalk,
  overlapping 0.1 m in the middle) instead of x ±1.5: each vent's collider and picking box are the same box,
  so whatever the reticle shows is what the cast hits, and no part of a vent's hot zone pokes into the reeds.
  Dip vents are x -1, 0, 1, each 1 m wide, the outer ones reaching the rails.
- **Aim rule.** Vent picking box = its collider (the full hot zone, 1.2 m tall, above the 1.0 m jump), so a ray
  into the flame wall always picks a vent. Hollow floors have flat picking boxes (0.1 m) over the boardwalk.
  `aim_and_cast` is not touched.
- **Flames.** Low blue cones (tops at most 0.58 m), driven by `level3::sync` (marker `VentFlame`), shrunk by the
  flicker; they reuse the courtyard's fire particles and fire lights via `sync_visuals`, with one hook there:
  a vent's particles start at 0.3 m (its picking box is the tall hot zone) and vents make no smoke (methane
  burns clean enough). Bubbles: `P::Foam` particles at 1.5/s per vent at `?low` (about 9 alive with 7 vents,
  under the 12 cap), 3/s otherwise.
- **Haze.** One translucent unlit box per hollow (`Haze`), height `0.7 × co2`: at full it just covers the
  flame tops and stays far below head height (eye 1.6 m). Registered as a visual convention (CO2 is colourless).
- **Colliders.** Perimeter walls, reed beds either side (3 m tall, so nothing can be climbed), the clearing
  round the hummock, boardwalk rails (1.2 m, above the jump), the lantern plinth and post (2.2 m), the back arch
  posts, three rocks on the hummock, and the vents. The hummock itself is a walkable mound below ankle height
  (a collider there would stop the player reaching the lantern).
- **Level struct.** `cleared2` added; `next_open()` as specced. `exit_open` changed from `n == 2` to `n != 1`
  so level 3's back arch door glows open too (otherwise a `?level=3` page shows the walk-through back arch as
  shut). Level 1 and 2 behaviour is unchanged.
- **Level-2 north arch.** `level2::NEXT_ARCH (0, -22.6)`, posts and lintel via `Builder::block` (colliders),
  door panel `NextDoor` swapped between `ArchMats::closed/open` in `level_visuals` by `next_open`; the closed
  door box (`level2::next_door()`) is added to the solids in `control` while shut (as level 1's exit). Arrival
  back from the Marsh is `level2::ARRIVE_NORTH (0, -20.4)` facing +z, 2.2 m from the arch (out of reach).
- **Walk stop.** In level 3, on the win the walk stops (velocity zeroed, held keys and scripted moves ignored)
  until R / a level change, or a fresh press of a movement key (so a player can still walk to the back arch, as
  the banner says). A scripted `move` never releases it (golden 15).
- **Finder arrow.** In level 2 after the win, a new system `level3_visuals` (after `level_visuals`) sets the
  existing `fireBearing`/`fireOff` toward the north arch; hud.js shows the arch arrow 3 s after the banner, as
  for level 1. In level 3 the arrow points at the nearest burning vent (the normal fire finder).
- **HUD.** Title "The Marsh", objective, banners, `gas` capicon (style.css), the idle Water nudge and the
  "aim at a fire" hint stay level-1 only (Water is the wrong spell in the Marsh). Pointer-lock block untouched.
- **CO2 on a fresh `?level=3`.** No progress gate exists on main (`select`/`cast` need no unlock); golden 8
  checks it. Nothing in `progress.*`, `bench.rs`, `bench.js` or `aim_and_cast` changed.
- **Stretch S1 (spill vent on a mound) cut.** A ninth vent that burns forever would break `ventsLit === 7 / 0`
  in golden 7 and 14. The `Spilled` rule and its captions exist and are unit tested (chem test 2).

## Pinned golden edits (allowed; list in the PR)

- Key order check (golden ~1072): adds `I037_KEYS = ['nextOpen', 'pools', 'ventsLit']` after `I035_KEYS`.
- `streams: fireInView is a boolean, the last state key` (~1346): `keys.length - 11/12` and `slice(-10)` with
  the three new keys.
- `progress: fresh state ... keys in order` (~2689): the tail includes the three new keys.

No other existing check changed. `objs` keys are unchanged. The `ONLY` switch gains `marsh`.

## Checks

| check | result |
|---|---|
| `cargo fmt --all --check` (root) and `cargo fmt --check` (client) | clean |
| `cargo clippy --all-targets --locked -- -D warnings` (root, sim) | clean |
| `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` (client) | clean |
| `cargo test --locked` (root, sim) | 217 passed |
| native `cargo test --locked` (client, shared target dir) | 89 passed (8 new I-037 tests) |
| `rustc --edition 2021 --test src/chem.rs` | 28 passed |
| `npm test` | 85 passed (science lint: 90 claims, 90 sourced, 22 params) |
| `npm run typecheck` | clean |
| `perf.mjs --self-test` | 50 passed |
| `build-web.sh` | ok, wasm gzip 5131141 B |
| golden `ONLY=marsh` (`?low`) | 16/16 (`evidence/golden-marsh-alone.log`) |
| full golden (`?low`) | FULL_GOLDEN |

## Perf (level 3 vs level 1, same build, `evidence/perf/`)

- 5 alternating pairs, `low@4x`: CPU p50 median level 1 31.0 ms, level 3 29.7 ms; pair diffs (3 − 1)
  -1.6, -2.7, +2.4, 0, -2.2 (median -1.6; level 1's own range 28.6-34.4). All 10 runs valid, workload ok.
- p99 median 53.2 vs 48.2 ms; TTI median 5917 vs 5941 ms.
- Draw calls (GL count run, `low@1x`): 46.6 vs 34.2 per frame. Entities (golden STATS): level 1 633, level 3 522.
- Brotli wasm +0.009 MB against the main build.
- 10 switches 1→2→3→2→1: entities 633 → 633, wasm memory 33.6 → 33.6 MB.

## Science

New register rows C-073..C-096, P-021..P-023, Not claims for the instructions, the `gas` key, the haze and the
flame colour; C-072 updated (level 2 banner now points to the Marsh). New sources, all fetched 2026-11-01:
[S26] USGS FS 172-96 (CO2 heavier than air collects in depressions, kills trees, a danger to people),
[S27] Wikipedia "Wetland methane emissions" (methanogenesis in oxygen-poor, waterlogged soil; ebullition),
[S28] National Grid "Natural gas fires" (water is ineffective on a natural gas fire), [S29] NIST methane
(16.0425), [S30] NIST nitrogen (28.0134), [S31] NSW RFS flammable gases (do not extinguish burning gas without
cutting off the supply; methane is lighter than air). The OSHA extinguisher-classes page and USGS/NOAA/EPA
wetland pages were refused by the egress proxy or did not say it; the archived OSHA page covers only flammable
liquids, so [S28] (gas) is used instead. Verdicts: the Water and Frost lines are `simplified` (a strong spray
or a sustained N2 flood can put out a small gas flame; the game's single cast cannot); pools rising in fixed
steps, never draining and all flames going out at exactly full are `simplified` (C-076). Safety: no line
suggests breathing CO2; the Adept Pooled line states the hazard ([S26] [S14]). The fresh science reviewer
should run `docs/loop/science-review.md` before merge.

## For the breaker

- Try to get past a burning flame line: rails, jumping, hugging a rail, diagonal at the vent seams (the hollow's
  two vents overlap 0.1 m in the middle).
- Aim: through the dip flames at the floor behind them (should pick a vent), at the floor in front (dip),
  between the hollow's two vent rows.
- Walk stop: hold W into the lantern; then release and press W again (should walk); R after the win.
- Arch bounce: arrival in level 2 from the Marsh is 2.2 m from the north arch, facing away.
