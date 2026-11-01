# I-037 round 2: Builder (fixes for break 1)

Wave 13. Branch `loop/I-037`, worktree `.claude/worktrees/w13-I-037`. `origin/main` had not moved since round 1
(fetched at the start), so there was nothing to merge.

## B1. Coming back from the Marsh no longer wins level 2 unsolved

The fix is the breaker's option (a), the latched variant. When the player arrives in level 2 from level 3 (`levels()`,
`n == 2 && from == 3`), the code now does this:

- `level2::solved(&mut chem.world)` freezes the stream (`ice 1`) and melts the gate (`wall false`, `ice 0`, now a
  puddle). It does this by applying the game's own Frost and Fire rules to the fresh world, the same hits a player
  makes. Every visual, collider, crust and "melted" picking box therefore follows from the state with no extra code.
- It then calls `World::reach()`, so the lantern win is latched.
  - Walking up to the lantern cannot win the level a second time; `apply` ignores spells once the outcome is latched.
  - The state matches what the arrival shows: the gate is open and the stream frozen. The level-2 banner ("You opened
    the Frozen Gate! ... Walk to the glowing arch to visit the Marsh. Press R to play again.") is now true on arrival,
    and the objective reads "Gate opened!".
- `level.cleared2 = true` (S1).
- The arrival caption is "Back at the Frozen Gate. The stream is frozen and the gate is open." It replaces the reset
  caption "Cross the stream, melt the gate, reach the lantern.", which would be stale on a solved level. It has a
  Not-claims row.
- R in level 2 after this works as R after a normal win: it reloads level 2 fresh and unsolved, with `cleared2` still
  set, and reaching the lantern again wins again.

Why not the alternatives:

- (b) "skip `reach()` until the player has crossed back" needs a per-visit flag and leaves a standing gate behind a
  player who just came through it.
- (c) "arrive at START" breaks the story and makes the walk back to the Marsh mean solving level 2 again.

New tests:

- **Native.** `level2::tests::i037_back_from_the_marsh_level_2_is_already_solved`:
  - after `solved()`, the stream is frozen solid, the gate is melted, and the outcome is `Won`;
  - from beside `ARRIVE_NORTH` a walk south crosses the melted gate and the frozen stream (z > 8.5).
- **Golden, marsh section.** `marsh: back from the Marsh, level 2 stays solved (stream frozen, gate melted) and the
  lantern gives no unsolved win`. It runs right after check 17:
  - setup: `look 180 0`, `move 0 1 1.5`, then 2 s of game time;
  - pass condition: the stream is frozen with `ice >= 1`, the gate has `wall false`, the outcome is never `won` while
    the gate stands, the player walked forward, and the banner is hidden or reads "You opened the Frozen Gate!".
- **Golden, marsh section (S1).** `marsh: from a direct ?level=3 load, the back arch reaches level 2 with its forward
  arch open and the gate melted`:
  - setup: a debug page with `&level=3`, then `tp 0 21.8`;
  - pass condition: `nextOpen === true`, the gate is melted and the stream is frozen.

**Red before the fix.** Both new golden checks ran against the round-1 build: a saved copy of the old `pkg-out`,
served by the new `golden.mjs` from a scratch tree. Result `ONLY=marsh` **16/18**
(`evidence/round2-golden-marsh-red.log`):

- B1: `outcome=won {"gateWall":true,"gateIce":1,"streamIce":0,"streamFrozen":false} pos=0,0,-17.75`. This is exactly
  the breaker's repro.
- S1: `nextOpen=false outcome=playing {"gateWall":true,...}`.

## Should-fix

- **S1.** Fixed by the same branch (`cleared2 = true` on 3 to 2). The golden check is above.
- **S2.** The Sprout Pooled line is now `These fires went out!`, in `level3::caption` and in the native caption-table
  test, which follows the spec table's row. The C-089 note says that "These" names the flames the CO₂ covered.
  - Spec deviation: the spec's table row still reads "All the fires went out!". The breaker's finding overrides it.
- **S3.** Gas bubbles now rise from every vent, lit or smothered (`level3::sync` no longer filters on `burning`).
  - The bubble rate per vent is unchanged. With 7 vents that is about 8 alive at `?low`, the same count as a fresh
    level, under the 12 cap.
  - Two new Not-claims rows for the fresh science reviewer:
    - the bubbles at smothered vents (simplified: real methane rises out of a CO₂ pool and disperses [S31]);
    - smothering a gas fire with the gas still flowing, as a simplified mechanic. [S31] says not to extinguish burning
      gas without first cutting off the supply. A marsh has no valve, and no caption tells the player to put out a
      real gas fire.
- **S4** (rail overlap, debug `tp` only). Not changed: no real player can get there, and no new arrival point lands in
  a prop (`ARRIVE_NORTH` is on open ground).
- **S5.** The level-3 win banner subtitle now ends "Press R to play again." (C-096 text updated). Golden 15 checks only
  `.big`, so no check changed.

## Checks

| check | result |
|---|---|
| `cargo fmt --all --check` (root) and `cargo fmt --check` (client) | clean |
| `cargo clippy --all-targets --locked -- -D warnings` (root, sim) | clean |
| `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` (client) | clean |
| `cargo test --locked` (root, sim) | 217 passed |
| native `cargo test --locked` (client, shared target dir) | 90 passed (1 new) |
| `rustc --edition 2021 --test src/chem.rs` | 28 passed |
| `npm test` (after build-web) | 85 passed (science lint: 90 claims, 90 sourced, 22 params) |
| `build-web.sh` | ok, wasm gzip 5130969 B |
| golden `ONLY=marsh` (`?low`), round-1 build (red) | 16/18: the two new checks fail as expected |
| golden `ONLY=marsh` (`?low`) | **18/18** (`evidence/round2-golden-marsh.log`) |
| golden `ONLY=frozenGate` (`?low`) | run 1 15/16: `Leidenfrost beads ... then 0 within 2 s` (beads gone after 2219 ms), the known wall-time flake. Run 2 (load 3.6): **16/16**, beads gone after 1926 ms (`evidence/round2-golden-frozenGate-run*.log`) |
| full golden (`?low`) | **223/225** (`evidence/round2-golden-full.log`). Marsh 17/17 in the full run, both new checks included. Failures, both known wall-time flakes: `cast: a real click casts exactly once` (casts 1 -> 2 -> 2) and `frozen gate: Leidenfrost beads ... then 0 within 2 s` (beads gone after 2152 ms; it passes alone, see the row above) |

## Files

`crates/bevy-client/src/level2.rs` (`solved`, test), `main.rs` (`levels()` 3 to 2 branch), `level3.rs` (Sprout
Pooled line, bubbles), `web/hud.js` (level-3 banner text only; the pointer-lock block is untouched),
`content/science-sources.md` (C-089, C-096, three Not-claims rows), `tests/e2e/golden.mjs` (two added marsh checks;
no existing check edited). Nothing in `aim_and_cast`, `bench.rs`, `bench.js` or `progress.*`.
