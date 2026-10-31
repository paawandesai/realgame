# I-028 sim lane, round 1

Branch `loop/I-028-sim` off the contract stub 230de1b.

## Steps

1. Tests 1-7 first (commit dca7530), red on the stub: 10 of 12 new tests failed. The prompt
   parity test passes on the stub because the prompt data is the stub's. Test 4 (fuzz) passed
   on the stub at first because the stub never returns `Ok`; a positive control (test 1's `Ok`
   cases must be `Ok`) was added and it then failed on the stub too.
2. Bodies of `judge`, `judge_single`, `judge_reaction`, `recipe` (atoms in formula order, own
   walk of `Term.formula`, since `Atoms` is a `BTreeMap`), `learnable`, `prompt`, and the
   prompt rule in `SpellBook::parse` (player-text check, max 120 chars, then `check_prompts`,
   then the reference checks).
3. The lane builder ran out of time before committing step 2. The integrator finished it in the
   same worktree: one clippy fix (`type_complexity` on `coefs`, now a `Side<'a>` alias),
   commit, this note, push.

## Results (integrator, on the final lane head)

- `cargo test -p realgame-sim --no-default-features --locked`: 61 passed, 0 failed.
- `cargo test -p realgame-sim --locked` (default features): 217 passed, 0 failed (before the
  alias-only clippy change).
- `cargo clippy -p realgame-sim --all-targets -D warnings`, both feature sets: clean.
- `cargo fmt --all --check`: clean.

## Spec conflict

The spec puts the `Prompt` rule before the reference checks. That breaks the existing I-026
test 4i (`spell_tier_cannot_be_below_its_reaction`): setting fire's `tier_min` to sprout now
reports `Prompt { fire, Sprout }` before `TierBelowReaction`. The test's setup now also gives
fire a `prompt_sprout`, so it still reaches the tier check, and an added assertion pins the new
order. This is the only edit outside test 7 in `spellbook.rs`.

## Seam note for the client lane

Test 6 checks that `def.formula.text` is ASCII and equals `equation.with_states()`, so the
client fills `SpellCard.formula` with `def.formula.text`.
