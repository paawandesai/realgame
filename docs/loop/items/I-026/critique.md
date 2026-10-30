# I-026 critique (spec critic, fresh context)

Read: `I-026/spec.md` (36 KB), `target.md`, `bars.md`, `areas.md`, `gameplay-standards.md` from origin/main, the origin/main code for `crates/sim` (lib.rs, Cargo.toml, reactions.json, reactions.rs, spell.rs), `chem.rs` `LN2_C`, and the register rows. Compared with I-022, I-024, I-025, I-027, I-028 and I-029. **There is no `I-030/` folder**, so nothing could be checked against it (see Overlap).

## Verdict: revise

It is the right thing to do. It is not a wrong-thing, because the new core loop (learn at the Bench, master, instant cast, craft) needs a stable "spell = data + real reaction" answer, and the exact-parity, golden-unchanged framing is the correct safety stance. But there is one defect that would corrupt saves, one traceability claim that overstates B10, and one acceptance gate that cannot discriminate. The item is also larger than it needs to be. All of these are fixable in the spec without changing its intent.

## Right thing for kids 4-18 and teachers?

- Player-visible value is zero by design. That is acceptable for a foundation item, but only because it blocks I-027 to I-030 and B10. The spec should say so in one line so nobody scores it as "fun".
- Core loop fit: spells keyed by a permanent id plus a `tier_min` is what I-027 and I-028 need. Crafting-not-shooting is not harmed: nothing in the cast path changes.
- Kid and teacher risk it creates: the client now calls `.expect()` in a `OnceLock` under `panic = "abort"`. If the data were ever wrong, a child sees a blank page. The risk is tolerable only if the check runs in CI before ship (see change 4).
- Gameplay standards: it adds no screen, control, camera or effect, so GS-1 to GS-35 are untouched. The spec has no "GS walk"; add one line saying "none touched" (I-027 does this).
- Content decision hidden in a validation rule: `Fire.tier_min = seedling` is derived from methane's `tier_min`, so Sprout (ages 4-7) can never learn Fire. Is that the intent? The first-quest fires are extinguished with Water, Frost and CO2, so probably yes. But the spec should state it as a decision and tell I-028 and I-029 to check that no Sprout-reachable quest requires casting Fire.

## Smallest version

It is not the smallest. Three build lanes, a contract commit, a sim feature split, a new `Tier`, a role-validation engine with 13 negative cases, a lint extension and a size budget, all for no visible change. Cut or defer:

1. Defer the `smothers` role (see Science). That removes a rule and cases 4e/4f.
2. Make test 13's perf part report-only (see Tests). It is the most expensive gate and the noisiest.
3. Test 16 (1,000 parses under 5 ms) guards nothing real: parse runs once per boot. Replace it with a source check that `book()` is the only caller.
4. Keep the `Tier` type (I-028 needs it), but drop `Tier::ages()` from this item unless I-028's spec uses it. It currently has no consumer.

Do not cut the feature split or the lint scan: both are load-bearing for B3 and B6.

## Can each test fail? Specific problems

- **Test 13 perf (`p50 <= base + 1.0 ms`, `TTI <= base + 0.3 s`) cannot discriminate.** I-024 and I-015 document base-vs-base spread of 30.4 to 48.2 ms on the same build. A 1 ms gate over three pairs is a coin flip, so it either fails randomly or never fails. Use I-024's noise-aware paired rule (base-vs-base run first, gate on paired difference), or demote it to a recorded number and keep the brotli delta as the hard gate. Also add a structural check: no `SpellBook::parse` call outside the `OnceLock` initialiser.
- **Test 4f is a positive control that shows the `smothers` rule is vacuous.** Any gas that is not a reactant passes, so `N2(g)` "smothers" through `water_synthesis`. The rule cannot fail for a wrong science claim, which weakens test 3 and B10.
- **The effect-id seam has no test.** The seam breaker is told to check that `effect` ids equal `Spell::ALL.map(id)`, but the only automated check is a runtime panic in `spells.rs`. The client CI job runs `fmt`, the plain-`rustc` chem test and wasm clippy, not `cargo test`, so nothing in CI catches a mismatch before golden. Add to `test/spells.test.ts` (test 19): parse the `id()` match arms out of `chem.rs` and assert that they equal the `effect` values in slot order.
- **Test 18 is a process step, not a test.** Keep it as a builder log line, not an acceptance test.
- **Test 12 greps `web/pkg-out/bevy-client.js`**, a path nobody else in the repo uses. Confirm the path against `build-web.sh` before building, or the grep passes on a missing file (`grep -c` on a missing file prints an error and a count of 0 in some shells). Add `test -s` on the file.
- Tests 1-3, 5-7, 10, 14, 15, 17 and 19 do fail on a no-op or on c4832d2, and the spec says how. Good.

## Science sourced?

- **Frost and nitrogen row: sound.** 77.34 K gives -195.8 C; the bounds in test 5 are right; [S1] is NIST. The builder must record the exact NIST page value. Add: the new row needs an `[S1]` citation in `science-sources.md` for the enthalpy figure, and the science reviewer must check the 5.57 value against the page, not against this spec.
- **CO2 to `carbon_combustion` is false provenance.** That row's `source` is "standard enthalpy of formation of CO2(g)". It sources a heat of formation, not smothering. Test 3 would assert "traces to a sourced entry" and pass, but the entry does not support the caption's claim ("heavier than air ... smothers fire"). C-004 is already marked `simplified`; it is the only honest register line. Either (a) drop the `reaction` ref for `smothers` and require a register id (`science_ref: "C-004"`) that the lint checks exists, or (b) add a sourced `fire_triangle`/inert-gas entry to `reactions.json` and reference that. Do not claim B10 foundation for CO2 until one of these is done. B10 progress should say "3 of 4 spells traced" if nothing changes.
- **Water to `water_boiling`** is acceptable (latent heat of vaporisation is a real mechanism for cooling), but the caption says "cools things below their ignition temperature". Note the simplification already in C-006 and C-019; no change needed beyond a note in the science reviewer's brief.
- Lint: reading `name`, `label`, `hotbar_label`, `cast_caption` only is correct. Test 17's negative cases (same sentence under `notes`, `id`, `formula`) are well chosen.

## Lanes buildable from the contract alone?

Mostly yes: files, owners, forbidden paths, seam and merge order are explicit. Gaps:

1. `spells::book() -> &'static ClientSpells` is in the contract but `ClientSpells` is never defined (fields, or whether it is a newtype over `SpellBook`). Define it.
2. The stub says `parse` "deserializes only", which requires the custom `Deserialize for Formula` to compile. That is listed under the sim lane's later work ("`Formula` deserialization"). Move the impl into the stub, or the stub does not build.
3. The client lane's native `cargo check` and the contract's `cargo check` both need the lock edits; the contract is the only lock writer, which is right, but say what happens if the client lane finds it needs another dependency (stop and ask the contract owner).
4. CI: only the sim lane edits `ci.yml`. Add the client-side step for change 4 (or state that the Node test replaces it).

## Overlap and consistency with other specs

- **Naming trap with I-027 (must fix).** I-026 adds `chem::Spell::id()` returning the **effect** id (`"foam"` for CO2). I-027 saves progress under spell ids (`fire, water, frost, co2`) and says "expected: a method `Spell::id(self)`" and that its `spell_id` will be swapped to "I-026's id" when I-026 lands. A builder following I-027 literally would call `Spell::id()` and write `foam` into players' saves, with `co2` gone, permanently. Rename in I-026: `Spell::effect_id()` and `Spell::from_effect_id()` in `chem.rs`, and make `spell_id()` (= `def().id`) the only method that returns the permanent id. Update test 10. Tell I-027 and I-028 the exact name.
- **I-027:** its fallback table `["fire","water","frost","co2"]` matches I-026's ids. Good, once the rename is done.
- **I-028:** its assumptions on `SpellDef`, `Tier` and `tier_min` match. Its `prompt_*` extension follows I-026's "Extending" rule and the lint key list. Fine. I-028's Sprout list (Water, Frost, CO2) relies on `Fire.tier_min = seedling`, which this item sets; see the content-decision note above.
- **I-029** reads `spell.def().tier_min` and says "Rendering the hotbar from data" is I-026's non-goal; consistent.
- **I-022, I-024, I-025:** the overlap table is accurate and the claim of no shared hunk holds. The `Cargo.toml` section split with I-024 and "I-024 merges first, then re-measure the size delta" is the right call. Add a rule: if I-024's profile change makes the 60 KB brotli budget unreachable, the budget is re-set by the tester from the base of record, not by the builder.
- **Backlog row mismatch:** the row says "After I-022, I-025", but the spec says neither is a code dependency. Update the row to "none (merge-order only)". I-028's row says "After I-026" but its spec says I-026 and I-027; already noted in I-028.
- **I-030:** no `I-030/` folder exists, so nothing could be compared. If I-030 adds a per-spell prediction question, it needs its own optional `SpellDef` field under I-026's "Extending" rule; no conflict is visible from here.

## Specific changes requested

1. Rename the `chem.rs` methods to `effect_id` and `from_effect_id`; add `Spell::spell_id()` from `def().id`; update the contract, test 10 and the seam list; notify I-027 and I-028 in their "Assumed from I-026" sections.
2. Fix CO2 traceability: either a `science_ref` register id checked by the lint, or a sourced non-combustion entry. Until then, restate the B10 claim as 3 of 4 traced.
3. Drop the `smothers` role from the generic rule set (or make it require a register id); remove cases 4e/4f if the role goes.
4. Add the effect-id vs `chem.rs` check to test 19, so the mismatch is caught in `npm test`, not only by a runtime panic.
5. Replace test 13's perf gate with I-024's paired, noise-aware method or make it report-only; keep the brotli delta gate. Replace test 16 with a "single caller" source check.
6. Move `impl Deserialize for Formula` into the contract stub and define `ClientSpells`.
7. State the Fire-is-Seedling content decision explicitly, and add a one-line GS walk ("no GS item touched") and a one-line statement of the item's purpose (zero visible change, unblocks I-027 to I-030, B10).
8. Confirm the `pkg-out` path in test 12 and add `test -s`.
9. Fix the backlog row's dependency cell.
10. Make test 18 a builder log step, not a numbered acceptance test.

## Planner response

Spec revised to revision 2. Each change:

1. **Accept.** `chem.rs` gets `effect_id`/`from_effect_id`; `spells.rs` adds `Spell::spell_id()` (= `def().id`) as the only permanent-id method; test 10 also asserts no plain `fn id`/`fn from_id` in `chem.rs`; "Notes for dependent specs" tells I-027 and I-028 the exact name (their own "Assumed from I-026" lines are refreshed by their contract builders, as both specs already require).
2. **Accept, option (a).** CO2's ref is now `{kind: register, id: C-035, role: cites}` (C-035 is CO2's own cast caption, sourced [S19] [S14]); sim checks the id pattern, test 19 checks the row exists, is correct/simplified, is sourced and matches the spell's text. B10 now reads "3 of 4 engine-traced, CO2 cited".
3. **Accept.** `smothers` dropped; cases 4e/4f replaced by register-id and role/kind mismatch cases; "Extending" now says a role must be able to fail for a wrong claim.
4. **Accept.** Test 19 parses `effect_id()` match arms from `chem.rs` and compares them to `effect` in slot order; it runs in CI's `npm test`.
5. **Accept.** Perf is recorded only (via I-024's `perf-pairs.mjs` when merged); brotli stays the hard gate. Test 16 is now a single-caller source check, in test 19.
6. **Accept.** The full `impl Deserialize for Formula` is in the stub; `ClientSpells` is defined (fields, `def`, `all`, `book()`).
7. **Accept.** Purpose line, GS walk ("none touched") and a "Content decision: Fire is Seedling and up" section, with a check handed to I-028/I-029.
8. **Accept.** Path confirmed from `build-web.sh:19-22` (`OUT=web/pkg-out`, `--out-name bevy-client`); `test -s` added.
9. **Accept.** Spec states the row should be "none (merge-order only)"; the row lives in origin/main's `backlog.md` (not in this working tree's copy), so the planner fixes it there at the next backlog write.
10. **Accept.** Test 18 is now a builder log step, not an acceptance test.

Also accepted from the body: `Tier::ages()` dropped (no consumer in I-027 to I-030; checked); [S1] must name the enthalpy and the reviewer checks 5.57 against the NIST page; Water's simplification noted in the reviewer brief; brotli budget re-set only by the tester if I-024 makes it unreachable; lanes stop and ask the contract owner for any new dependency; no client CI step is added because test 19 runs in the existing `npm test` step. I-030 now exists and was checked: it uses I-026's `BadText` rule, the `spells.json` scan pattern and `nitrogen_boiling`'s value, none of which changed.
