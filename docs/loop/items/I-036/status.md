# I-036 status (paused 2026-11-01, wave-14-mac)

Paused by the change of plan to one open valley (the Crystal Lab becomes a region of the valley later). The branch stands as WIP; nothing here is merged.

## Done (on loop/I-036)

- `spec.md` (based_on 5662cd7).
- Tests first (commit 03ad41e): 10 Rust acceptance tests (chem lab rules, `electrolysis::column`, level 4 captions and walk, `level3::solved`) and the golden section `crystalLab` in the `ONLY` map, with the pinned key-order and tail edits for `I036_KEYS = ['balloon', 'crateUp', 'tubes']`. Against the stubs: `cargo test i036` 0/10.
- `chem.rs`: `Kind::Splitter/Balloon/Crate`; `Obj.gas/lifted/run`; events `Split/Lifted/Popped` with the Apprentice captions; `H2_PER_SPLIT` 0.34, `SPLIT_S` 2.5 s; `apply_lab` (Water splits, fills the balloon 0.34 a cast, lifts the crate at full; Fire pops the hydrogen; wrong spells change nothing); `step` runs the splitter's display timer down; `electrolysis::column` (H2 column always exactly 2 × O2).
- `level4.rs`: pure `layout` (perimeter, partition with doorway, ledge, lantern plinth, splitter bench and crate colliders), `caption` per band (the spec table), `build` (stone lab hall, crystals, splitter with battery, 2 tubes and hose, iron crate, balloon and string, lantern, back arch), `sync` (crate ride onto the ledge, its picking box following it, balloon size, tube columns, bubbles), `balloon/crate_up/tubes` for publish.
- `level3.rs`: north arch to the lab (`NEXT_ARCH`, door panel tagged `level2::NextDoor` so `level_visuals` lights it), `ARRIVE_NORTH`, `solved()`.
- `world.rs`: `solid_obj` for Crate (solid until lifted) and Balloon (never).
- `main.rs`: `cleared3`; level 3 `next_open`; the Marsh's north door is solid while shut; level 3 → 4 and 4 → 3 triggers, level-4 lantern; `?level=4` and `level 4`; `reset` line for level 4; back from the lab the Marsh is solved at its north arch; level-4 captions; pop fx; level-4 win caption; walk stop on the level-4 win; finder toward the Marsh's north arch after its win; `level4_visuals` system; publish appends `balloon`, `crateUp`, `tubes`.
- `cargo test --locked` (bevy-client, native): 107/107 pass, including the 10 I-036 tests.

## Left

- `hud.js`: title "The Crystal Lab", objective and win banner for `level === 4`; the level-3 banner's small line when `nextOpen` ("Walk to the glowing arch to visit the Crystal Lab"); finder `exitAhead` for level 3.
- `content/science-sources.md`: rows for every new string (C-097.., per the spec's Science rows), P-024 `H2_PER_SPLIT`, P-025 `SPLIT_S`, sources [S32] NIST Hydrogen (MW 2.01588), [S33] Wikipedia "Splint (laboratory equipment)" (squeaky pop), [S34] Wikipedia "Gas balloon", [S35] Wikipedia "Hydrogen" (flammable, 2H2 + O2 → 2H2O, 4-74 % in air), [S36] Wikipedia "Electrolysis of water" (H2 twice O2; pure water conducts poorly, an electrolyte is added). The C-096 row's text changes when the level-3 banner line changes. `npm test` will fail on the science lint until these rows exist.
- Not yet run: `cargo clippy -D warnings` (one dead-code warning: `level4::BALLOON` is used only by tests), `npm test`, the wasm build of this commit, golden `ONLY=crystalLab,marsh,frozenGate`. Golden against the stub build (03ad41e) stopped after its first check (the Chrome window was closed mid-run); a headless stub rerun was in flight when the plan changed.
- Risk to check: the marsh golden "level 3 has no more entities than level 1" (the new north arch adds 5 entities to level 3).
- Evidence screenshots in `evidence/`.
- When the lab becomes a valley region: `level4::layout/build/sync/caption` and the chem rules move over as they are; the arch plumbing in `main.rs` (level 3 → 4) is the part that changes.
