# I-052 integration, round 1 (wave 15)

Branch `loop/I-052` (worktree `.claude/worktrees/w14-I-052`), from the contract at 3315e28.

## Merges

| Order | Branch | Head | Result |
|---|---|---|---|
| 1 | `origin/loop/I-052-A` | af5f0a2 | clean, no conflicts (85a99f4) |
| 2 | `origin/loop/I-052-B` | 4f42971 | clean, no conflicts (c7a7877) |
| 3 | `origin/loop/I-052-C` | a2e5e86 (has `lanes/C/round-1.md`) | clean, no conflicts (b0e259a) |
| 4 | `origin/main` | already contained | "Already up to date" |

No textual conflicts. The lanes did collide in meaning in three places, fixed in the commits below.

## What the integration changed (and why)

1. **One pack (56220d5).** Lane A's view gathers into `crate::Pack` (the contract's resource in `main.rs`) and places
   from it. Lane C's `story.rs` kept a second `Pack` inside `Story`, and on `PlayerEvent::Dug` it added the drop again.
   The `pack` state key showed Lane C's copy, so the HUD and E4 never saw what was dug. Now:
   - `crate::Pack` is the run's only pack, and it starts with the Pick (that was Lane C's rule, now in `Pack::default`).
   - `story`'s systems swap `crate::Pack` into `story.pack` for the length of each call (`with_pack`), so
     `run_command`, `holding`, `state_json` and Lane C's tests work as written.
   - `on_event` no longer adds a drop on `Dug` or takes on `Placed`, because the view already did both. It still tells
     the quest what the pack holds (`holding`), so digging coal still finishes step 4.
2. **Matter runs in the world (56220d5).** The contract left the wiring for `main.rs`/`voxel_view`, and neither
   lane's notes owned it. `blockchem::drive` (added to Lane B's plugin) runs each frame after `projectiles`:
   - each `SpellLanded` (already delayed by the flight time in `projectiles`) becomes `on_impact` → `world.emit`. A
     landing with a zero normal (hit nothing) is skipped.
   - `Matter::tick` runs at most one 0.1 s step on `VoxWorld` and fills `activeChunks`, `matterMs` and `gasCO2Max`.
   - Matter events go out as `crate::MatterEvent` before `story::on_event`. The chunks it dirties are remeshed on the
     next frame.

   `Matter::queue` is not used: `projectiles` already waits the flight time, so queueing would delay twice.
   Dig, place and `setcell` call `matter.world.touch`. `probe` now reads `tempC` from `matter.world.temp_c` and `co2`
   from `gas_at` (it was `null`).
3. **The intro against the voxel checks (9beb6be, 98dd21e; `web/story.js`).** Lane C's intro makes the stage inert
   and swallows input. On the integrated page Lane A's E3 (W never walked: the key was aimed at the intro's button,
   not the canvas) and E5 (the 5 key was swallowed, so the click cast a spell) failed. Both lanes' intents are kept:
   - the key that skips the intro (after 1 s) is sent again to the game's canvas once the intro closes. A click that
     skips is still swallowed.
   - a frame gap counts for at most 500 ms toward a card's time and the 1 s guard. At load the page stalls about
     1.7-2 s meshing the spawn ring while the intro is already unhidden, so before this change a key pressed before
     the intro was ever drawn skipped it. A slow machine still moves forward by up to 500 ms a frame.

   No golden test was edited.

## Checks (integrated head)

- `cargo fmt --check`, root and client: clean.
- `cargo clippy --all-targets -- -D warnings`, root (native): clean.
- `cargo clippy --target wasm32-unknown-unknown -- -D warnings`, root and client: clean.
- `cargo test --locked` (root): all pass.
  - sim: 226 passed, 1 ignored (C4).
  - matter: 9 + 8 passed, 7 + 1 ignored.
  - voxel: 14 passed, 2 ignored.
- `npm test`: 97 pass.
- `build-web.sh`: ok, wasm gzip 5 220 144 bytes.
- Golden `ONLY=voxelEngine,blockMatter,valleyStory`, `QUERY=?low`: **7/8**.
  - PASS: E1, E2, E3, E4 (dirt +1, cell air, key 6 bound, placed back), E5 (runestone caption), E17, no console
    errors. `blockMatter` has no checks yet.
  - FAIL: `story: the intro shows three cards and any key skips it after 1 s`. See below.
- Full golden (`QUERY=?low`, every section, about 75 min): **251/254**, exit 1. See "Full golden" below.

### Full golden

The run took about 75 min with no console errors. Three checks failed:

1. `clock: cast cadence keeps the 0.3 s cooldown in wall time` (casts=8, need 9, expectedGame=3.750, frames=16).
2. `cast: a real click casts exactly once` (casts 1 -> 2 -> 2 within 20 frames on, but `lit[1]` was still burning
   at the end of the 20-frame window).
3. `story: the intro shows three cards and any key skips it after 1 s` (E15, below).

Checks 1 and 2 run on the classic courtyard (`classic=1`). Nothing in this integration runs there:
- `story`, `blockchem` and `voxel_view` are added only in `Mode::Voxel`;
- the `main.rs` edits are the `Pack` default and an `Option` matter field read only by voxel debug commands.

Lane A's full run of its own branch passed every classic section, these two included. The page ran at 1.6 fps
(`STATS fps=1.6 p50_ms=638`) during this run, with only this golden's Chromium on the 4 cores. Both checks count
frames or wall time across a few frames, so they are most likely timing on this sandbox, not regressions. **This was
not re-verified:** golden has no `ONLY=` entry for the clock or cast sections, and a second full run did not fit
this round. The coordinator should re-run them on the Mac or in the next smoke step.

### E15 on this machine

Instrumented run: the intro is unhidden at 742 ms page time, then the spawn-ring stall ends at 2845 ms. The harness's
"early" key (meant to land inside the first second) arrives at 5322 ms, about 2.5-2.8 s of drawn frames later. That
time goes to `load`'s `frames(3)`, `until(mode)`, the `poll` for the intro and two evaluates, with frames of about
400 ms each in headless software GL. So the key correctly skips, and `early.shown` is false.

Lane C measured 3/3 on a Mac with real Chrome, and it also failed here before any integration change (same symptom on
the first run). The test is not wrong for its target hardware. On this sandbox, the harness cannot press a key within
1 s of the intro showing. It was not edited. Lane C could make the "early" press wait on the intro's own clock (for
example a `data-shown-ms` attribute), if the coordinator agrees that is not a loosening.

## Ignored tests that now pass or fail on the merged world (run with `--ignored`, not un-ignored here)

These belong to the lanes. They were run for information only, with no `#[ignore]` removed.

- Generated-world matter checks: `frost_freezes_top_layer_first`, `ford_bridges_in_three_casts_per_row`,
  `co2_pools_in_hollow`, `fire_spreads_through_wood_and_water_puts_it_out` and `generated_m5_water_and_frost_only_flicker_vents` **pass**
  on Lane A's real generator. Lane B can un-ignore them.
- `fire_melts_ice_gate_in_three_casts` **fails**. Lane A's gate stamp is a solid ice block 4 × 12 × 8 (x 58-61,
  y 36-47, z 86-93), not the 4 × 3 × 1 opening the spec and Lane B's fixture assume, so 3 casts cannot melt it.
  Lane A should reshape the stamp (Tuesday, V3).
- `hollow_gap_spills_until_closed` **fails** ("with the gap open at least one vent still burns"). The generated gap
  does not drain fast enough. Lane B's notes give the outlet the stamp needs (an open sunken pool east of x 196).
- `lift_sign` (M9) still fails: it is the known spec error in Lane B's notes (CH4 cannot lift 1.5 kg).
- V3, V4: still `todo!()` (Lane A, Tuesday).

## Open items for round 2

- Lane A:
  - reshape the ice gate and the hollow outlet as above;
  - draw flames for `BURNING` cells (the workshop shows no fire yet);
  - V3/V4, E6-E8;
  - the courtyard objective box is still built and then despawned.
- Lane B:
  - un-ignore the five generated-world tests that now pass;
  - `blockMatter` E9-E14 can start now (spells reach the sim, and `probe` shows `co2` and `tempC`);
  - a burning count for the `burning` key.
- Lane C:
  - matter events in `story::on_event` (they arrive now);
  - `worked_ok` from the bench;
  - E15's timing on slow machines (above);
  - the HUD hotbar still shows only the 4 spells while keys 5-9 are tools.
- Lane C's `build-web.sh` line (adding `story.js`, `pack.js`, `music.js`, `sfx.js`) is kept; Lane A owns that file.

## Evidence

`evidence/round-1/`:
- `01-overview.png`: the valley from the south ridge, with the Kiln and chimney, the terraces and the river canyon;
- `02-after-digging.png`: a 3 × 3 pit dug with the Pick, which put 11 dirt in the pack;
- `03-frozen-fields.png`: `?biome=frozen`.
