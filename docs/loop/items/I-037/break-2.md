# I-037 break 2: level 3, the Marsh

Breaker, wave 13, round 2. I did not write this code. I tested origin/loop/I-037 at cbb9243 in a fresh detached
worktree (`/tmp/claude-0/brk13b-I-037`, removed afterwards). The wasm came from `build-web.sh` (ok, gzip 5132238 B).
Every headless Chromium `?low` run held the shared lock.

## Verdict

**BLOCKING: none.** B1 is fixed. Coming back from the Marsh, by walking, by `?level=3` or by the debug `level 2`,
now always lands in a solved level 2: stream `ice 1, frozen`, gate `wall false, ice 0`, outcome latched `won`. No
walk to the lantern gives an unsolved win. S1, S2, S3 and S5 are fixed as well. Nothing regressed in the Marsh,
the Frozen Gate or the Formula Bench.

Two new should-fix items follow, both about what the screen says on the way back (N1, N2). The game state they
describe is correct; only the words are wrong.

## Runs

| run | result |
|---|---|
| `rustc --edition 2021 --test src/chem.rs` | 28 passed |
| native `cargo test --locked` (client, shared target dir) | 90 passed |
| `npm test` (science lint included) | 85 passed (90 claims, 90 sourced, 22 params) |
| golden `ONLY=marsh` | **18/18**, both new round-2 checks included |
| golden `ONLY=frozenGate` | runs 1 and 2 15/16, each failing only `Leidenfrost beads ... then 0 within 2 s` (beads gone after 2195 ms and 2158 ms; a wall-clock check, load average about 4). Run 3: **16/16** (gone after 1267 ms) |
| golden `formulaBench` (a local, uncommitted copy of golden.mjs whose only change is an `ONLY=formulaBench` branch; no check edited) | **55/55** |
| breaker probes (real W/S/R key presses plus the state bridge, below) | no console errors, no panics |

## Break-1 findings, re-checked

| finding | status | evidence |
|---|---|---|
| B1: the walk back wins level 2 unsolved | **fixed** | `?level=3` (no debug), then the back arch reached by real W. Holding W 2 s more walks to z -17.75 next to the lantern. The outcome is `won`, but the gate reads `wall false, ice 0` and the stream `ice 1, frozen true`, so the win describes what is on screen. The full walking path (L1 won, L2 solved, north arch, Marsh, back arch) gives the same. |
| S1: dark arch after a direct `?level=3` load | **fixed** | `?level=3` then back arch: `nextOpen true`. After R in that level 2, `nextOpen` stays `true`, but the gate stands again (`wall true`) and the stream is liquid, so the open arch is reachable only by solving. No free path. |
| S2: Sprout "All the fires went out!" | **fixed** | `level3::caption` and its native test say "These fires went out!"; C-089 is updated. |
| S3: bubbles stop at smothered vents | **fixed** | `level3::sync` no longer filters on `burning` (code read). Two simplified Not-claims rows record the rising-methane and "cut the supply first" [S31] caveats. |
| S5: no "Press R" on the level-3 banner | **fixed** | hud.js subtitle and C-096 end "Press R to play again." |

## Arrival paths checked (probe log, abridged)

| path | result |
|---|---|
| L1 won, then `level 2` | L2 fresh, `playing` |
| L2 solved, north arch to L3 | L3 fresh: 7 vents, pools `[0,0]`, `playing`, `nextOpen false` |
| L3 back arch to L2 (walking path) | L2 solved and latched, at `ARRIVE_NORTH (0,-20.4)`, `nextOpen true` |
| right after that arrival, hold S 1.5 s | back into the Marsh, fresh (the arch is 2.2 m behind). Expected |
| `?level=3` (no debug), then back arch by real W | as the walking path, solved and latched |
| that level 2, then walk south over the melted gate and frozen stream to the back arch | reaches level 1 fresh (3 fires). `exitOpen` is false on that page because level 1 was never won. Correct: the player has to save the greenhouse |
| L2 back arch to L1 (walking path) | L1 fresh with 3 fires, `exitOpen true` (`cleared1`), the same as before this item |
| solved L2, real R | L2 fresh and unsolved at START; `nextOpen true` behind a standing gate |
| `?level=2` direct | north arch shut (`nextOpen false` at z -21.8). Back arch to L1: L1 fresh, exit shut |
| 3, 2, 3 by arches, then real R in L3 | L3 fresh at START, 7 vents, pools `[0,0]` |
| Frost on the melted gate, Fire on the frozen stream in the latched level | nothing changes (spells are ignored after the latch). The same as after a normal win |
| debug `level 2` from L3 | the same as the back arch (solved). Debug only |

## Should fix

- **N1. The arrival caption "Back at the Frozen Gate. The stream is frozen and the gate is open." never shows.**
  - `levels()` sets it with `game.say`, but the outcome is already `Won`. On the next frame, the caption block at the
    end of the event system (`Outcome::Won if level.n == 2`, main.rs around line 1893) replaces any caption that does
    not start with "You opened". It writes "You opened the Frozen Gate! Freeze to cross, melt to open: water changed
    state both ways."
  - Probe: the caption reads "You opened the Frozen Gate! ..." at +0, +2 s and +4 s after every 3 to 2 arrival.
  - The lane notes say the arrival caption replaces the stale reset caption, and the Not-claims row for it describes
    a line no player sees. Neither is true.
  - Fix: let the level-2 win caption keep a caption that starts with "Back at" (or carry a `revisit` flag), and pin
    it with a golden check on `s.caption`. The round-2 golden check only looks at the banner.
- **N2. After a direct `?level=3` load, the way back says "You opened the Frozen Gate!" to a player who never played
  level 2.**
  - Both the banner and the caption say "You opened the Frozen Gate!", with the subtitle "Freeze to cross, melt to
    open: water changed state both ways." main.rs calls `?level=N` "the player's way", and I-034's level select will
    use it.
  - The gate really is open and the stream really is frozen, so this is not B1's wrong-state lie. It does claim
    something the child did not do.
  - On the walking path it is only a repeat of a true banner, but it still pops a full win banner over a player who
    simply walked back.
  - Fix (one option): on a 3 to 2 arrival, hide the win banner and keep the N1 caption until the player acts. Or show
    a neutral banner, for example "The Frozen Gate is open." The objective "Gate opened!" can stay.
- **N3. The analytics quest event fires on the way back (code read, not run live).**
  - `analytics.js questEvents` counts a completion when `outcome` goes from `playing` to `won` between polls.
  - The game state has no `quest` field, so every level counts as quest 0.
  - `?level=3`, then the back arch, goes from L3 `playing` to L2 `won` and sends one quest-0 completion with nothing
    won, on the built site where `rg-analytics` is on.
  - On the walking path it is already sent from the level-1 win, so nothing extra goes out there.
  - Fix: also require `prev.level === state.level`, or flag the latched arrival. analytics.js is not on the I-029 list.
- **N4 (minor). Three seconds after the 3 to 2 arrival, the HUD exit arrow points at the Marsh arch the player just
  came out of.** hud.js `wonAt` restarts on the level change, and the arrow shows `findExit` once the arch is off
  screen behind them. It is harmless, since the arch really is open, but it nudges a child straight back. If N2
  hides the banner, the arrow can wait for the same condition.
- **S4 from break 1 (rail overlap, debug `tp` only)**: unchanged and still not reachable by a real player. The 3 to 2
  arrival point `(0,-20.4)` is open ground, and I saw no new arrival point land in a prop.

## Known flakes seen

Only the frozenGate Leidenfrost 2 s wall-clock check failed (twice, then passed). Round 2 changed nothing on that path: `level2::solved` runs only on a 3 to 2 arrival, and the bubble change is in `level3::sync`, which level 2 never calls. I did not change any check, and the only golden copy I used was the local `ONLY=formulaBench` dispatcher, which was not committed. The probe script was not committed either.
