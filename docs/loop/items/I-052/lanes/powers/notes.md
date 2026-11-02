# Spells are powers (rank, traversal powers, stronger practice, caches)

Branch: `claude/project-thread-21govy`, from `preview/play` at af45055. Asked for by the main thread after Paawan's
playtest ("there is no incentive to learn the spells"). Voxel mode only.

## What is in

- **`crates/bevy-client/src/powers.rs` (new).**
  - **Rank.** Points = each spell's stage (Learning 1, Practised 2, Mastered 3) + 1 per cache found. Tiers: Spark 0,
    Kindler 2, Catalyst 5, Synthesist 9, Archon 13 (Archon needs a cache as well as four mastered spells). A rank-up
    is a moment (caption, banner, fanfare) only when the progress book's `rev`, this run's practice or the found
    caches changed; loading the save or restoring caches is silent.
  - **Powers on F** (the selected spell; locked with a "learn it at the Formula Bench" caption while Unknown; each
    power has its own recharge):
    - Fire: **Flare**, a light that follows you (20/30/45 s). `late.rs` counts it as light in the dark cave.
    - Water: **H2 lift**, rise at 5 m/s by 8/10/12 blocks from the ground, drifting at most 1.5 m/s until landing.
    - Frost: **Ice walkway**, freezes the top layer of water ahead (15.5/19.5/23.5 m; 1 or 3 wide). The ice is set
      to 258 K and stays frozen (tested 30 s in the matter sim).
    - CO2: **dash**, 20/22/24 m/s along the view plus 5 m/s up, from the ground only.
  - **Stronger with practice** (`scale`): spell reach/range ×1.0/1.3/1.6 (via `powers::Boost`, read in
    `voxel_cast::plan`), cast cooldown ×1.0/0.75/0.5, power recharge 3.0/2.0/1.2 s, and the bigger powers above.
  - **Caches** (stamped at plugin build, before meshing; each on a levelled pad with a walkable ramp, protected
    from digging and building; an orb with 4 crystals and a rank point):
    - Ember Vault, Village (112, 190): brick hut, a dry-thicket door (leaves) only fire clears.
    - Sky Pillar, Marsh (176, 128): 3×3 pillar, 7 tall.
    - Frost Isle, Frozen Fields (70, 120): 3×3 island in a 13-wide moat, 2 deep (too deep to wade).
    - Ravine Cache, Marsh (214, 128): 5×5 platform in a 9-wide, 4-deep pit with steps out.
  - Tests walk the real `voxel_view::walk` over the stamped world: a sprint jump, a Mastered dash and a Mastered
    lift each fail where they should, and the intended power succeeds at every stage.
- **`web/rank.js` (new):** badge under the Bench button (rank, bar to next, `F: <power>` with lock and recharge),
  the rank-up banner with `fanfare`, power and pickup sounds, found caches saved under `rg.caches.v1`.
- **Golden `valleyPowers`** (`ONLY=valleyPowers`): 8/8 on this sandbox (`?low`, headless).

## The first two minutes (guided first lift)

- **Plaza Tower** (`ledge`, Village (134, 196), between the benches and the workshop): a 3×3 cliffstone tower, 5
  tall, with a Water cache on top and a beam of light over it until it is taken. Protected 3 blocks around.
- Before Water is learned, standing within 5 m shows a tease card: "Something glows up there. Learn Water at the
  Formula Bench (B) to rise up to it."
- Learning Water in play (the book's `rev` changes; not a save load) starts a 150 s guide card that follows the
  player: press 2 (skipped if Water is selected), walk to the tower (with metres), press F, hold W onto the top.
- Water learned (1 point) plus the tower cache (1 point) is the first rank-up: Kindler, with the banner.
- Golden `valleyFirstLift` (`ONLY=valleyFirstLift`): 4/4. The card is `#power-guide` (rank.js), above the E chip.

## Shared files touched

| File | Change |
|---|---|
| `crates/bevy-client/src/main.rs` | `mod powers;` and `powers::plugin` in the Voxel plugin list |
| `crates/bevy-client/src/voxel_cast.rs` | `plan` takes a `boost` (range multiplier); `fly` and `preview` read `Option<Res<powers::Boost>>` |
| `crates/bevy-client/src/late.rs` | the Flare counts as light in `step` |
| `crates/bevy-client/web/boot.js`, `build-web.sh` | load and copy `rank.js` |
| `crates/bevy-client/web/style.css` | appended `rank-*` rules |
| `content/science-sources.md` | C-106..C-110, six Not-claims rows, [S41] |
| `tests/e2e/golden.mjs` | `valleyPowers`; and a missing `}` that closed `valleyPuzzles` (lost in the af45055 merge, so the file did not parse) |

Not touched: `gen.rs`, mesh and shader files.

## Open

- The pads cut hills straight up beyond their ramps, which shows tall dirt walls in the block look; the smooth
  world's terrain should hide or reshape these.
- Cache coordinates were picked on the current generator; `caches_sit_in_their_areas_and_clear_of_the_story`
  fails loudly if a new generator moves a stamp or protected place onto a pad.

## Music and the full playthrough (round 3)

- **`web/music.js`:** each area (Village, Frozen, Marsh, Cave, Kiln) keeps its own synthesised theme with 2 s
  crossfades, now with an answering B phrase every other pass and a bass line. The ending plays a short rising
  sting and then the Kiln theme. M (or the ♪ button under the rank badge) turns the music alone on and off, saved
  as `rg.music.v1`; the Sound setting still silences everything. No AudioContext is made before the first key or
  click (checked in the playthrough: `started` false before, Village theme playing after the first click).
- **Golden `playthrough`** (`ONLY=playthrough`, not in the default run): spawn to the ending card on one page with no
  `quest <n>`; stand-ins are `tp`, `prog learn`, `give torch` and `give dirt` (each tested elsewhere). It logs an
  estimated new-player time per step. **Status:** green end to end on this sandbox (0.5-2 fps headless, about
  40 min wall time): every quest step, the field Benches, the music checks and the estimate check pass.
- **Field Benches** (`bench::FIELD_STATIONS`, stamped by `powers::stamp_field_bench`): small stone shrines with
  runestone corner posts on levelled ground, at the ford approach (146.5, 162.5, Village bank) and the Marsh gate
  (118.5, 70.5, Frozen side). Each teaches anything, like the Plaza Bench. Native test: flat ground, clear of
  every protected box, the bench lights beside it, and you can walk up to it.
- **Workshop fire** (`firebreak.rs`): the fire is a puzzle, never a soft-lock. When the world loads, every
  flammable block within 4 of the workshop except its fuel store is made fireproof (wood posts to brick, the
  plank workbench to stone, grass to dirt), so the fire can never spread to the walls or the plaza. A Water
  cast that lands within 5 m of the fire also lands on the nearest burning block, so every cast wins ground.
  Native tests: in 6000 steps only the store burns; from five landing spots and four burn times it always
  goes out in under 5 casts, each cast puts out some of it, and it stays out. In the playthrough: 3 casts.
- **Estimate** (walking model: straight line × 1.3 at 4 m/s, plus task allowances in the script):

  | Step | Walk | Est. |
  |---|---|---|
  | Intro | 0 m | 25 s |
  | 1 Workshop fire (learn Water) | 31 m | 113 s |
  | 2 Ford (Frost and Fire at the ford Bench) | 91 m | 179 s |
  | 3 Ice gate | 111 m | 52 s |
  | 4 Coal | 42 m | 35 s |
  | 5 Hollow (CO2 at the Marsh Bench on the way) | 154 m | 156 s |
  | 6 Cave (torch at the Marsh Bench, the rig, the crate) | 294 m | 183 s |
  | 7 Kiln (door, Invention Bench, ending) | 286 m | 156 s |
  | **Total** | **1009 m** | **15.0 min** |

  Before the field Benches it was 1721 m and 18.0 min (Bench round trips).
