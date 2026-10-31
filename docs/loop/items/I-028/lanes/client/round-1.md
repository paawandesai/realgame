# I-028 client lane, round 1

Branch `loop/I-028-cw` from the contract head 230de1b (stub 762424d on origin/main dc1d336, I-027 merged).
Path touched: `crates/bevy-client/src/bench.rs` only. `main.rs`, `bridge.rs`, `spells.rs`, `progress.rs`,
`Cargo.*`, `web/**` and tests are unchanged beyond the stub; no I-046 file (`cues.rs`, `feel.rs`,
`main.rs` stream code) is touched.

## What landed

- **Station** (`spawn_station`, Startup): inserts `BenchState::default()`, then spawns at
  `STATION = (-3.0, 14.5)` a white rim disc (r 1.3, y 0.015), the lab-blue pad (r `RADIUS` = 1.2,
  y 0.02) and a static flask icon (bulb r 0.25 at y 2.2, neck r 0.08 above it), two materials
  (lab blue with mild emissive, white). All `NotShadowCaster` (the flask also `NotShadowReceiver`).
  Nothing is added to `Colliders` or the chemistry targets.
- **`atBench`**: horizontal distance of `Player.pos` (x, z) to `STATION` `<= RADIUS`. Display only;
  the pad never opens anything.
- **Command loop** (`run`, Update after `control`, before `publish`): drains `WebIn.bench`
  (`bridge::poll` runs inside `control`), `parse` -> `Cmd::{Band, Open, Pick, Try, Close}`, `apply`:
  - `band <tier>` (exact `Tier::from_id`, so `adept` is accepted) open or closed; while open it also
    clears `pick` and `result`;
  - `open` sets `open`, clears `pick`/`result`;
  - `pick <slot>` only while open with a band and `bench::learnable(def, band)`;
  - `try <slot> <text...>` only while open with a band; text = remaining words joined by one space;
    `bench::judge(spells, slot, band, text)`, so an unlearnable or missing slot gives `not_for_band`;
  - `close` (keeps `band`).
  Anything else, a malformed argument, or a command while closed other than `band`/`open` is
  ignored with `warn!`.
- **Learn call** (`record`): bumps `n`; on `ok`, if the spell's stage is `Unknown`,
  `progress.0.learn(slot - 1)`; `learned` = the call returned a transition. Repeat builds earn
  nothing here. Slots are resolved by the def's `slot` field (0, 5, 255 never panic).
- **`BenchView` build** (`view`): closed -> `BenchView::closed(atBench, band)`; open -> `spells`
  (learnable at the band, slot order, with `formula` = `def.formula.text`, `stage` from `Progress`,
  `prompt` from `bench::prompt`), `pick`, `recipe` (Sprout and Seedling only), `result`. Open with
  no band carries no `spells` (the page shows its chooser).
- **Change-only publish**: the JSON is built each frame and `bridge::publish_bench` is called only
  when it differs from `last_json`.
- Spell data only through `crate::spells::book().all()` (no `SpellBook`/`ReactionBook` load in
  `src/`). `#![allow(dead_code)]` from the stub is removed.

## Checks

| check | result |
|---|---|
| `cargo fmt --check` (client) | clean |
| native `cargo check --locked` (client) | ok; only the existing `WebIn` dead-field warning |
| native `cargo test --locked` (client) | 73 passed (9 new in `bench::tests`) |
| wasm `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` | clean |
| `build-web.sh` (wasm release) | ok, `pkg-out` holds `bench.js`; wasm gzip 5 094 646 B |
| `npm test` | 73/73 |

New client unit tests (sim-independent, so they pass on the stub judge): the five words parse and
14 malformed lines do not; `atBench` is horizontal and false at the start pose; the closed view is
exactly `{"v":1,"atBench":false,"open":false}` and gains `"band":"seedling"` after `band seedling`;
closed `pick`/`try`/`close` and open-without-band `pick`/`try` are ignored; `band` while open clears
`pick`/`result`, `close` keeps the band; a correct build learns once, five repeats keep `xp` and
report `learned:false`; out-of-book slots never panic; open cards carry the def's ASCII `formula`.

One hiccup: the science lint (`test/science-sources.test.ts`) cannot parse Rust raw strings, so
the first draft's `r#"..."#` test literals failed `npm test`; they are now escaped strings.

## Depends on the sim lane

On the stub `learnable` is false, `prompt` is None, `recipe` is empty and `judge` returns `parse`,
so an open view lists no spells and every `try` is `parse`. The golden checks that need real
cards and verdicts (13-28b, apart from 12/12b) pass only once the sim lane merges.
