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
