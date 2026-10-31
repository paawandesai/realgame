# I-027 rust lane, round 1

Commits: c95f8d6 (unit tests 1-11 and 8b against the stub), 8328543 (bodies).
Only `crates/bevy-client/src/progress.rs` changed.

## Tests first: the stub split (as the spec predicts)

`rustc --edition 2021 --test crates/bevy-client/src/progress.rs -o /tmp/progress && /tmp/progress` on the stub:

| test | stub | with bodies |
|---|---|---|
| t01 stages in order | FAILED | ok |
| t02 worked casts | FAILED | ok |
| t03 two-day rule | FAILED | ok |
| t04 predict | FAILED | ok |
| t05 two transitions (via load) | FAILED | ok |
| t06 never fades (50 seeds x 20 000 ops, water reaches Mastered) | FAILED | ok |
| t07 save round trip | FAILED | ok |
| t08 decode rejects whole | FAILED | ok |
| t08b stage kept as saved (inherited by I-030) | FAILED | ok |
| t09 commands and the debug gate (`"needs ?debug"`) | FAILED | ok |
| t10 ids | ok | ok |
| t11 state_json (fresh exact string; parses for walked books) | ok | ok |
| total | 2 passed, 10 failed | 12 passed, 0 failed (2.2 s) |

## Notes on the bodies

- One private `op(i, f)` runs every operation: out-of-range index returns `[]` with nothing
  changed; it reports the op's own stage change, then `settle` (Practised, then Mastered, in
  order); `rev` bumps only if the spell's fields changed. So a wrong worked cast (+0), a wrong
  predict, a day-0 build and an op on a stage it does not apply to leave `rev` alone; an
  instant cast on Mastered (+2 XP) bumps it by exactly 1.
- `unaided_build` carries the midnight/clock-change comment from "Learning-science basis".
- `decode`: length > 512, prefix other than `p1`, field count other than exactly 7, unknown or
  repeated id, any non-digit (so `-1`, `;`, spaces) or a field over its range reject the whole
  save. Stage is kept as saved in both directions (no settle on load).
- `apply_cmd`: `load`/`day` always; `learn|worked|instant|build|predict|clear` give
  `Err("needs ?debug")` without debug; malformed args are `Err` and change nothing; `load` does
  not bump `rev`.
- The file keeps `#![allow(dead_code)]` because I-028..I-030 are the callers of most of the API.
- No `SpellBook::parse`/`load`, `ReactionBook::load`, `crate::`, Bevy or `realgame_sim`.

## Other checks (final branch)

| check | result |
|---|---|
| `cargo test --locked` in crates/bevy-client (native, temp target dir, deleted after) | 64 passed, 0 failed (includes the 12 progress tests and spells.rs tests) |
| `cargo clippy --release --locked --target wasm32-unknown-unknown -- -D warnings` | clean (CI runs the same lint in the debug profile; not re-run here for disk) |
| `cargo fmt --check` | clean |
| `bash build-web.sh` | pass, wasm gzip 5 087 853 B (stub 5 084 359 B, +3.5 KB) |
