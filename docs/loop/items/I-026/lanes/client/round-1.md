# I-026 client lane, round 1

Commits `a468e3e` (chem.rs, spells.rs, `mod spells;`), plus the one-line panic-message rewording in `80721f5` (see the lint note) and `size.md` in the evidence commit.

## Red first (test 10)

`rustc --edition 2021 --test src/chem.rs` with `effect_ids_round_trip` added and nothing else changed:

```
error[E0599]: no variant, associated function, or constant named `effect_id` found for enum `Spell`
error[E0599]: no variant, associated function, or constant named `from_effect_id` found for enum `Spell`
```

and `grep -c "Fire spell: a gas flame" src/chem.rs` was 1.

## Implementation

- `chem.rs`: `name`, `formula`, `cast_caption` deleted; `effect_id` (`fire`, `water`, `frost`, `foam`) and `from_effect_id` added. No rule, constant or caption changed.
- `spells.rs` (Bevy-free): `ClientSpells { pub reactions, spells }`, `book()` with a `OnceLock` whose `get_or_init` closure is the only caller of `ReactionBook::load()` / `SpellBook::load()`; init panics (naming the spell) unless there are exactly `Spell::ALL.len()` spells and slot `i + 1` has the effect of `Spell::ALL[i]`. Inherent `impl Spell { def, spell_id, name, formula (= label), cast_caption }`. `spell_id` is the only method returning the permanent id.
- `main.rs`: exactly one added line, `mod spells;` between `mod player;` and `mod world;` (`git diff origin/main -- crates/bevy-client/src/main.rs` is +1 line). `main.rs:1203` and `1690-1691` compile unchanged.

## Results

| check | result |
|---|---|
| test 10: `rustc --edition 2021 --test src/chem.rs` | 20 passed (incl. `effect_ids_round_trip`) |
| `grep -c "Fire spell: a gas flame" src/chem.rs` | 0 |
| `grep -cE "fn (id\|from_id)\b" src/chem.rs` | 0 |
| `cargo test --locked` (client, native) | 47 passed, 0 failed (incl. `spells::tests::slots_map_to_the_chemistry_rules_in_order`) |
| `cargo fmt --check` | clean |
| `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` | clean |
| `cargo check --locked` (native) | ok; the only warning is the pre-existing native-only `dead_code` on `bridge.rs` `WebIn` fields, same as origin/main |
| `./build-web.sh` | ok (3 min 33 s warm) |
| `cargo deny --manifest-path crates/bevy-client/Cargo.toml --locked check advisories bans licenses sources` | advisories ok, bans ok, licenses ok, sources ok |
| test 12: `test -s web/pkg-out/bevy-client.js`; `grep -cE "WebGame\|reaction_book\|heat_lut"` | file present; **0** |
| test 13 size (hard gate 60 KB) | brotli q11 3,536,740 → 3,563,434 bytes, **+26,694** (pass; see `evidence/size.md`) |
| `npm run build:site` (test 9) | ok: 3D client at `/`, classic game at `/classic/` built with `wasm-pack build ../crates/sim` (default features) |

## Golden (test 14, builder runs; the tester repeats 3 of 3 on the shared build)

`git diff origin/main -- tests/e2e/golden.mjs` is empty. Each run under the Chromium lock (`flock /tmp/claude-0/chromium.lock`), `BASE_URL= QUERY=?low node tests/e2e/golden.mjs`, on the branch build:

| run | result | failures |
|---|---|---|
| 1 | 123/124 | `cast: a real click casts exactly once` (`casts 1 -> 2 -> 2 (20 frames on), lit[1] burning=true`): one of the two I-019 wall-time checks the task lists as flaking under box load on main too; recorded, not changed |
| 2 | **124/124** | none; `no console errors` passes |

## Runtime parity (test 15, builder pre-check with a throwaway scratch script, not committed)

Base (origin/main 4c7c9a1 build) vs branch build, `?low&debug`, for N = 1..4: `reset`, `select N`, read `spellName`/`formula`, `look 0 60`, `cast`, wait for `casts` +1, read `caption`:

```
SAME slot 1..4 spellName / formula / caption (12 of 12 strings equal)
SAME first state keys (49); spell 1, spellName "Water", formula "H₂O absorbs heat"
casts per slot base=1,1,1,1 branch=1,1,1,1; console errors base=0 branch=0
PARITY OK
```

The negative control (swap two `cast_caption`s, rebuild, confirm the script reports it) needs a third full build and is left to the tester's run of test 15. The perf-pairs speed record in test 13 is likewise the tester's step and was not run by the builder (it needs the Chromium lock for about an hour and the box was shared with I-044's suites).
