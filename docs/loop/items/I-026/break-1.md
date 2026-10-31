# I-026 break-1 (wave 10, fresh breaker)

## Blocking
None.

## Should-fix
- crates/bevy-client/src/spells.rs (`book()` get_or_init, about line 83): validation panics lazily on first use, not at boot. Content is test-covered, so low risk; call `book()` once at startup so a bad spells.json fails at load rather than mid-play.
- crates/sim/src/spellbook.rs (`check_reference`, `RefKind::Register` arm): only the id shape `C-nnn` is checked; a register id absent from content/science-sources.md is accepted by sim. The Node lint (test/spells.test.ts) covers it, so this only matters for non-npm consumers.

## Nit
- spells.rs `#[allow(dead_code)]` on `reactions`, `all`, `spell_id` is fine for phase 1; remove as I-027/I-029 consume them.

## Checked
- Lanes: all 16 changed files (excluding docs/loop/items) lie in the contract, sim, client or lint lanes; main.rs diff is the single `mod spells;` line; no Cargo change outside the contract commit.
- Spell text in content/spells.json is byte-identical to the deleted chem.rs literals.
- Native tests: realgame-sim --no-default-features 50 passed; client 47 passed (CARGO_TARGET_DIR=/home/claude/bevy-target).
- Release web build ok; golden under the chromium lock: 124 PASS, 0 FAIL.
- Playtest (Playwright, ?low&debug, analytics switch forced on, age.js/analytics.js copied in) at 16:9 1280x720, 4:3 1024x768, 21:9 1680x720, as a 7-year-old (sprout) and a 13-year-old (apprentice): the age picker shows before any pointer lock and a band click dismisses it and starts the game; objective caption and spell state are identical for both ages (zero player-visible change, as designed); the idle hint appears after about 9 s with hints on (default). Hints-off and edge-of-wide-screen aiming were NOT re-exercised: this item touches no hud.js, camera or aim code, and golden's aim checks pass.
- Note: web/pkg-out lacks age.js/analytics.js (only the built site has them), so a picker test needs the dist build or a copy.
