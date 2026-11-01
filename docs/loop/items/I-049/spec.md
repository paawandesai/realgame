# I-049: polish bundle (wave 14)

Small fixes from I-035 break 1, I-029 break 2 and I-037 break 2. Light review.
Each fix has a check that fails before it.

## The list

| # | fix | from | check (fails before the fix) |
|---|---|---|---|
| 1a | Reaching the Frozen Gate's lantern stops the walk (the same stop-on-win as the Marsh, I-037): held W or a scripted `move` no longer carries the camera into the lantern. A fresh key press walks again. Not on the walk back from the Marsh, where level 2 loads already won. | I-035 b1 #3 | golden `frozen gate: reaching the lantern stops the walk (camera stays >= 1.2 m from the lantern)` |
| 1b | Sprout's stream melt line says "The ice melted! Make it cold again." until the stream has been frozen solid once since the last reset; "The bridge melted! ..." only after that (`level2::caption_at`, `Game.bridge_seen`). | I-035 b1 #4 | native `i049_sprout_melt_line_says_bridge_only_after_a_bridge`; golden `frozen gate: Sprout's melt line says "ice" before any bridge and "bridge" only after one` |
| 1c | Aiming at the lantern through the standing gate targets the gate. | I-035 b1 #5 | golden `frozen gate: aiming at the lantern through the standing gate targets the gate, and the cast lands on it` (see Notes) |
| 1d | Any spell on level 2's lantern says "That is the lantern. Walk up to it!" (the Marsh's words), never the courtyard's Soaked line "... before it can burn". `level2::caption` is unchanged (its test pins `None` for the lantern); `caption_at` adds the line. | I-035 b1 #6 | native `i049_spells_on_the_lantern_never_talk_about_burning`; golden `frozen gate: Water on the lantern says "That is the lantern. Walk up to it!", not that it can burn` |
| 1e | Snow mounds and boulders get colliders inside each ball's footprint (`level2::prop_boxes`, in `layout`). The boulders sit on the bank boxes, so theirs change nothing a player can reach; the mounds now stop the walk. | I-035 b1 #7 | native `i049_snow_mounds_stop_the_walk_outside_the_snow`, `i049_prop_colliders_leave_every_path_open` |
| 2a | Worked-cast builder: the Make row moves out of the builder into `.worked-foot`, the panel's last child, sticky to its bottom edge at every scroll position (the old sticky row could not rise above the builder's top). | I-029 b2 #1 | golden `worked: I-049 at text size 2 Make stays in view and on top with the panel scrolled to the top, middle or bottom` |
| 2b | A New spell's panel opens scrolled to the top, with the Copy line moved up under the head (above the "New spell" line), and focus no longer scrolls the head away. Other panels scroll only as far as the first piece (`block: 'nearest'`). | I-029 b2 #2 | golden `worked: I-049 at text size 2 a New spell opens with its name, close button and Copy line in view` |
| 3a | Back from the Marsh (`revisit`, new state key): no "You opened the Frozen Gate!" banner. The banner hides; the caption stays "Back at the Frozen Gate. The stream is frozen and the gate is open." | I-037 b2 N2 | golden `marsh: after a direct ?level=3 load, the walk back to level 2 shows no "You opened the Frozen Gate!" (banner or caption)` |
| 3b | On that arrival the exit arrow does not point back at the Marsh arch. | I-037 b2 N4 | golden `marsh: after that arrival the exit arrow does not point back at the Marsh arch (4 s of game time, arch behind)` |
| 4 | Harness fix: the Leidenfrost bead check measures game time (`state.gameTime` from the first state showing the beads), with the same 2 s bar, polled every 50 ms. | I-037 b2 runs | `frozen gate: Leidenfrost beads ... (>= 4, then 0 within 2 s)`, name unchanged |

## Harness changes and new state (listed as the rules ask)

- **Harness fix (4):** the Leidenfrost check's 2 s bar is now game time, not wall time. It was failing at 2.1-2.2 s of wall time on a loaded host. The bar is the same; the check now measures what its name says.
- **New state key:** `revisit` (bool), true while level 2 was entered from the Marsh. It is cleared by R and by every level change.
- No existing check was edited. The existing marsh check `back from the Marsh ... (!banner.shown || big === 'You opened the Frozen Gate!')` still holds: the banner is now hidden.

## Science register

- C-097 `The ice melted! Make it cold again.` (level2.rs, correct, [S10]).
- The not-claims row for `That is the lantern. Walk up to it!` now lists level2.rs too.

## Files touched

- `crates/bevy-client/src/level2.rs`: `BOULDERS`, `MOUNDS`, `prop_boxes`, `LANTERN_LINE`, `caption_at`, and native tests.
- `crates/bevy-client/src/main.rs`: `Game.revisit` and `Game.bridge_seen`, stop-on-win in level 2, `caption_at` at the impact, and the `revisit` state key.
- `crates/bevy-client/web/hud.js`: banner hidden and exit arrow off on `revisit`. This is in `frame()`; the pointer-lock block (I-047) is not touched.
- `crates/bevy-client/web/worked.js` and `style.css`: the `.worked-foot` actions row, Copy line order, and the open scroll.
- `tests/e2e/golden.mjs`: the new checks above, and the harness fix (4).
- `content/science-sources.md`.

## Notes

- **1c:** `pick` already ray-casts targets nearest-first, and the gate box is in front of the lantern box on every line from the south. Break 1's probe waited on `aimObj === lantern` with `until`, which returns on timeout without throwing, and then cast anyway. So "reports aimObj = lantern" was probably the timeout and not a real aim. The new golden check pins the behaviour from the start and from the north landing: the aim is the gate and the cast lands on the gate. See the run result in the commit or PR for whether it failed before the fix.
