# I-046 rust lane, round 2

## Should-fix: the drawn hand end was thinner than `stream_width(0)` on short streams

`stream_transforms` scaled the shared lead frustum (near/far ratio fixed at
`lead_min`) to `thick·k` at its far end, so for any stream shorter than `lead`
(0.8 m) the hand end was `thick·k·lead_min`, down to `thick·lead_min²` (about
4 mm). That is not only targets within 0.8 m: every cast's first frames, while
the head is still within 0.8 m of the hand, drew it.

Fix (`cues.rs`, `main.rs`): new pure `cues::stream_parts(len, style)` returns
both drawn parts as `(start, length, width)`. From `lead` on it is the old
split (frustum, then the full-width rod). A shorter stream has no rod, so the
otherwise idle rod cylinder becomes a `thick·lead_min` core along the lead: the
drawn hand end is exactly `stream_width(0)` and the head end is still
`thick·k = stream_width(len)`. No new entities, no mesh mutation, the shared
frustum and cylinder are unchanged. `stream_transforms` now just places the two
parts `stream_parts` returns.

New unit test U6 `drawn_stream_matches_width_at_both_ends` reads the parts as
the shared meshes draw them (frustum near end = `width·lead_min`, cylinder =
`width`) and checks hand width == `stream_width(0)`, head width ==
`stream_width(len)` and coverage == `len` for every spell 0..=255 and every
length 0..2 m in 1 cm steps. Failing first: with the old second part
(`(lead_len, rod_len, thick)`) U6 fails (`hand` = `thick·k·lead_min`); with
the fix it passes. U1-U5 unchanged and passing.

## Checks

- `rustc --edition 2021 --test cues.rs`: 13 passed (was 12; U6 added).
- `cargo test --locked` (native): 58 passed (was 57). The native debug dir was
  deleted afterwards (disk).
- `cargo fmt --check`: clean. `cargo clippy --release --locked --target
  wasm32-unknown-unknown -- -D warnings`: clean. (Native `--all-targets`
  clippy fails on `bridge.rs` dead fields and `clock.rs` duplicate_mod, files
  this item does not touch; same as before this item.)
- `build-web.sh` (CARGO_TARGET_DIR=/home/claude/bevy-target-I025): ok, wasm
  gzip 5 086 327 B.
