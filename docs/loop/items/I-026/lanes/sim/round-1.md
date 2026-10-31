# I-026 sim lane, round 1

Commits `4cf41cc` (tests, red) and `48d9794` (implementation).

## Red first (against the contract stub)

`cargo test -p realgame-sim --no-default-features --locked` on the stub plus the new tests (after adding `PhaseChange.substance`, which test 5 needs to compile; before that the test target did not compile: `no field substance on type &PhaseChange`):

```
test result: FAILED. 35 passed; 15 failed
FAILED: reactions::nitrogen_boiling_is_sourced_from_nist, spellbook::{a_phase_change_is_not_a_reaction,
absorbs_heat_needs_the_same_substance, an_unsourced_reaction_is_rejected, cites_and_register_only_go_together,
every_spell_traces_to_a_sourced_entry, ids_and_slots_are_unique_and_contiguous, ids_must_be_lowercase_words,
runs_needs_the_exact_reaction, register_id_must_match_the_pattern, spell_tier_cannot_be_below_its_reaction,
schema_errors_are_json_errors, text_that_could_break_state_json_is_rejected, unknown_reaction_id_is_rejected,
unbalanced_runs_formula_is_rejected}
```

Test 1 (`shipped_book_loads_in_slot_order`), test 2 (`player_text_matches_the_base_exactly`), test 6 (tier) and test 7 (formula) were green on the stub, as the spec expects for test 2 once the data is right. Test 2 one-character check: changing `cools below ignition` to `cools below ignitiom` in `content/spells.json` fails it (`hotbar_label of water: left "cools below ignitiom", right "cools below ignition"`); reverted.

## Implementation

- `SpellBook::parse` validation in the contract's order: JSON/schema (`version` must be 1, unknown fields denied, formula parse errors name the input), id pattern for `id` and `effect`, duplicate ids, duplicate slots, slots exactly `1..=n` (n <= 9), player text (1-40/80/80/200 chars, no `"`, `\`, control chars), then each reference:
  - `reaction`: found only in `book.reactions` (else `UnknownRef`), source > 10 chars trimmed (else `Unsourced`), role must be `runs`, formula has an arrow, is balanced and its whole-number `to_string()` equals the reaction's; reaction `tier_min` must parse (`Json`) and the spell's must be >= it (`TierBelowReaction`).
  - `phase_change`: found only in `book.phase_changes`, sourced, role `absorbs_heat`, single term whose formula equals `substance`, with a state, latent heat > 0.
  - `register`: id matches `^C-[0-9]{3}$` (else `UnknownRef`), role must be `cites`.
- `content/reactions.json`: new phase change `nitrogen_boiling`.

## The nitrogen row (for the science reviewer)

Read from the NIST page itself (https://webbook.nist.gov/cgi/cbook.cgi?ID=C7727379&Mask=4, fetched 2026-10-31):

| quantity | page value | reference on the page | row value |
|---|---|---|---|
| Tboil | 77.34 K | Jacobsen, Stewart, et al., 1986 (TRC) | `temperature_c` −195.8 (77.34 − 273.15 = −195.81, rounded to 0.1) |
| ΔvapH | **5.6 kJ/mol at 77 K** | Giauque and Clayton, 1933 (AC) | `latent_heat_kj_per_mol` 5.6 |
| ΔvapH (other row) | 6.1 kJ/mol at 78 K | Edejer and Thodos, 1967, "based on data from 63 to 126 K" | not used: a fitted value over a range, not a measurement at Tboil |

The spec says "about 5.57"; the page gives the Giauque and Clayton value to two significant figures (5.6), so the row records the page value 5.6, not 5.57. It is inside test 5's [5.5, 5.65] band. `source` names NIST, Nitrogen, the URL and both references. `content/science-sources.md` [S1] is extended to name the ΔvapH value (lint lane). No other reaction row changed.

## Checks

| check | result |
|---|---|
| `cargo test -p realgame-sim --no-default-features --locked` | 50 passed, 0 failed |
| `cargo test --locked` (root, default features) | 206 passed (185 before + 21 new), 0 failed; no existing test edited |
| `cargo tree -p realgame-sim --no-default-features -e normal` | serde, serde_core, serde_derive (proc-macro), serde_json, itoa, memchr, zmij; **no wasm-bindgen** |
| `cargo clippy -p realgame-sim --no-default-features --target wasm32-unknown-unknown --locked -- -D warnings` | clean |
| `cargo clippy -p realgame-sim --target wasm32-unknown-unknown --locked -- -D warnings` (CI `:145`) | clean |
| `cargo clippy --all-targets --locked -- -D warnings` (root) | clean |
| `cargo fmt --all --check` | clean |
| CI | new step `cargo test -p realgame-sim --no-default-features --locked` after `Test` in `test-rust` |
