# I-049 round 1: Builder (one lane: level 2, worked builder, Marsh revisit, one harness fix)

Wave 14. Branch `loop/I-049`, worktree `.claude/worktrees/w14-I-049`, on main 4b079e0. The fixes and their checks
were written in c03de7a / 96c4b62 (see `spec.md`); this round proves each check fails on the main build and
passes on this one, runs every gate, and adds before/after screenshots of the builder at text size 2.

## What a player gets

- **Frozen Gate:** reaching the lantern stops the walk (held W no longer carries you into it). Sprout's melt
  line says "The ice melted! Make it cold again." until the stream has really been a bridge. Any spell on the
  lantern says "That is the lantern. Walk up to it!" instead of "it must dry out before it can burn". Snow
  mounds now block the walk like the rocks.
- **Worked builder at text size 2:** Make sits in its own footer at the bottom of the panel at every scroll; a
  New spell opens at the top with its name, close button and the Copy line in view (the Copy line moved up
  under the name).
- **Back from the Marsh:** no "You opened the Frozen Gate!" banner for a win you never had; the caption stays
  "Back at the Frozen Gate."; the exit arrow does not point back at the arch you just left.

## Tests first: red on the main build

The new golden checks were run against main's wasm and web files (the w13-I-037 `pkg-out` built at 74895b9,
whose crate tree equals 4b079e0; every web file byte-compared equal to 4b079e0), served with
`BASE_URL`, `ONLY=frozenGate,workedCasts,marsh`, `?low`: **88/96**, log `evidence/golden-red-main-build.log`.

| new check | main build |
|---|---|
| frozen gate: reaching the lantern stops the walk (camera stays >= 1.0 m from the lantern) | FAIL, camera 0.75 m (at the plinth) |
| frozen gate: Sprout's melt line says "ice" before any bridge and "bridge" only after one | FAIL, "The bridge melted!" on a part-frozen stream |
| frozen gate: Water on the lantern says "That is the lantern. Walk up to it!", not that it can burn | FAIL, "Soaked: it must dry out (boil off at 100 °C) before it can burn." |
| worked: I-049 at text size 2 a New spell opens with its name, close button and Copy line in view | FAIL, all 5 cases (head scrolled away) |
| worked: I-049 at text size 2 Make stays in view and on top with the panel scrolled to the top, middle or bottom | FAIL, all 5 cases (Make out of view at the top scroll) |
| marsh: after a direct ?level=3 load, the walk back to level 2 shows no "You opened the Frozen Gate!" (banner or caption) | FAIL, banner shown at arrival and +4 s |
| marsh: after that arrival the exit arrow does not point back at the Marsh arch (4 s of game time, arch behind) | FAIL, arrow shown with `exit` |
| frozen gate: aiming at the lantern through the standing gate targets the gate, and the cast lands on it | PASS (regression guard, no product change; see spec Notes 1c) |

The eighth red failure, `worked: W18 level-1 chore tripwire`, is an artifact of the section order I chose for
that run (frozenGate before workedCasts leaves the page on level 2, so N1's Water-only courtyard run never
wins: `entities=464`, the level-2 count). In the full golden and in `ONLY=workedCasts,...` workedCasts runs
first; it passes in the green runs below. The native I-049 tests call `caption_at` and `prop_boxes`, which do not
exist on main, so on main they do not compile (no separate red run was made for them).

## Harness change in this round

- `tests/e2e/golden.mjs`, the new I-049 worked check only: with `SHOTS` set it saves two screenshots per case
  (`-open` as the panel opened, `-top` scrolled to the top) and then puts the scroll back where it opened, so
  the checks still measure the open state. Without `SHOTS` it does nothing. No check's condition changed.
- `cargo fmt` on `level2.rs` and `main.rs` (the previous commits were not formatted; `cargo fmt --check` failed).

## Evidence

- `evidence/before/` (main build) and `evidence/after/` (this build): `worked-i049-<band>-slot<n>-<w>x<h>-{open,top}.png`,
  text size 2, 5 cases. Before: the Seedling Fire panel opens with the head scrolled off the top, and at the
  top scroll Make is out of view. After: name, close button and the Copy line show at the top and Make sits in
  the footer.
- `evidence/golden-red-main-build.log`, `evidence/golden-sections-green.log`, `evidence/golden-full.log`
  (console lines removed).

## Checks

| check | result |
|---|---|
| `cargo fmt --all --check` (root), `cargo fmt --check` (client) | clean (after formatting the client) |
| `cargo clippy --all-targets --locked -- -D warnings` (root) | clean |
| `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` (client) | clean |
| `cargo test --locked` (root) | 217 passed |
| native `cargo test --locked` (client, shared target dir `/home/claude/bevy-target-I025`) | 101 passed |
| `rustc --edition 2021 --test src/chem.rs` / `src/worked.rs` | 28 / 7 passed |
| `npm test` | 97/97 (science lint: 91 client claims, 91 sourced, 22 params) |
| `npm run typecheck` | clean |
| `build-web.sh` | ok, wasm gzip 5135477 B |
| golden `ONLY=workedCasts,frozenGate,marsh` (`?low`) | **96/96**; 1a camera 1.17 m, Leidenfrost 0.750 s game time (1860 ms wall) |
| full golden (`?low`) | FULL_RESULT |

## For the critic and breakers

- At text size 2 a New spell opens at the top, so on 1024x600 (and for Apprentice on 1280x720) the answer box
  is below the fold: it keeps focus, so typing works, but the player has to scroll to see it. Before the fix
  the box was in view and the name and Copy line were not. The spec chose the head; a smaller panel layout at
  text size 2 would be a separate item.
- 1c is a regression guard only (it passes on main).
