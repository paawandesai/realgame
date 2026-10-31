# I-026 lint lane, round 1

Commit `80721f5`.

## Red first

- **Test 19 / 16** (`test/spells.test.ts`) against the base tree (`git archive origin/main` of `content`, `crates/bevy-client/{src,web}` plus the new test file): `not ok 1 - test/spells.test.ts`, `ENOENT ... content/spells.json` (pass 0, fail 1).
- **Test 17** (new cases in `test/science-sources.test.ts`) before the scan existed: `not ok 16 - fixture: an unregistered cast_caption in content/spells.json is reported as unsourced`, `not ok 17 - fixture: spells.json notes, ids and formulas are not player text`, `not ok 18 - the real content/spells.json is scanned and fully sourced`.

## Step 18 (builder log, the expected red)

With the strings moved out of `chem.rs` and before the `where` edits, the lint reported exactly the eight specced rows stale:

```
C-001 is stale: crates/bevy-client/src/chem.rs no longer contains "CH4 + 2O2 → CO2 + 2H2O"
C-002 is stale: crates/bevy-client/src/chem.rs no longer contains "H2O absorbs heat"
C-003 is stale: crates/bevy-client/src/chem.rs no longer contains "N2(l) boils at -196 °C"
C-004 is stale: crates/bevy-client/src/chem.rs no longer contains "CO2 blanket, no O2"
C-006 is stale: crates/bevy-client/src/chem.rs no longer contains "Water soaks up heat and cools things below their ignition temperature."
C-007 is stale: crates/bevy-client/src/chem.rs no longer contains "Liquid nitrogen boils at -196 °C (77 K), pulling heat out of anything it touches."
C-034 is stale: crates/bevy-client/src/chem.rs no longer contains "Fire spell: a gas flame. Methane burns: CH4 + 2O2 → CO2 + 2H2O, and lots of heat."
C-035 is stale: crates/bevy-client/src/chem.rs no longer contains "CO2 from an extinguisher is heavier than air: it smothers fire by pushing the oxygen away."
```

It also flagged one new string, a panic message in `spells.rs` ending `{:?}` (read as a sentence ending in `?`): `spell {}: slot {} must have effect {:?}`. Fixed by rewording the message to end in `{}` (no register row added).

## Implementation

- `scanClient` also reads `content/spells.json` when it exists under the scan root; `spellsJsonTexts` returns only the string values of `name`, `label`, `hotbar_label`, `cast_caption` in each `spells[]` entry, run through the same `normalize`/`isClaim`. Invalid JSON or a missing `spells[]` fails loudly (`UnsupportedSyntax`), like other unsupported shapes. Relative paths are now computed against the scan root passed in (`relTo(root, abs)`), so fixture roots report `content/spells.json`.
- Register (`content/science-sources.md`), `where` cells only: C-001, C-002, C-003, C-004, C-006, C-007, C-034, C-035 replace `crates/bevy-client/src/chem.rs` with `content/spells.json` (index.html kept where listed); C-031 and C-032 add `content/spells.json`; the `CO2` not-claim row becomes `content/spells.json, crates/bevy-client/web/index.html`. [S1] now names ΔvapH = 5.6 kJ/mol at 77 K (Giauque and Clayton, 1933) alongside Tboil. No claim text, verdict, note or other lint rule changed; no new C-/P-/S- id.

## Mutation checks on test/spells.test.ts (each reverted)

| mutation | failing test |
|---|---|
| `effect: "foam"` → `"co2"` in spells.json | effect ids in chem.rs equal spells.json effects |
| chem.rs arm `Spell::Frost => "ice"` | effect ids in chem.rs equal spells.json effects |
| `// ReactionBook::load()` appended to main.rs | only spells.rs loads the spell and reaction books |
| Water `hotbar_label` one character off | the static hotbar in index.html matches spells.json |
| CO2 `cast_caption` "pushing oxygen away." | every register reference names a sourced row whose claim is that spell's text |

## Results

| check | before (origin/main) | after |
|---|---|---|
| `npm test` | 53 pass | 63 pass, 0 fail (+3 lint cases, +7 spells.test.ts) |
| science-sources summary | 33 client claims, 33 sourced, 0 error verdicts | 33 client claims, 33 sourced, 18 params, 0 error verdicts, 0 stale-file warnings |
| `npx tsc -p tsconfig.json` | clean | clean |
