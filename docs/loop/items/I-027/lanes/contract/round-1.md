# I-027 contract lane, round 1

Branch `loop/I-027` from origin/main 76b5f13 (e70df08 + the wave-11 state commit).
Commits: 40fd6bf (stub, wiring, boot check, shipped progress.js), 42cb448 (streams
fireInView check, see "Deviation" below).

## What landed

- `crates/bevy-client/src/progress.rs` stub: every public item of Smallest version 1 with
  the Contract signatures; neutral bodies; real `SPELL_IDS`, `spell_id`, `index_of`,
  `state_json`.
- `main.rs`: `mod progress;` between `player` and `spells`; `struct Progress(progress::Book)`
  resource and `init_resource::<Progress>()`; the I-026 break-1 boot check in `setup` right
  after the `STARTUP` log (`let _ = spells::book();` + `assert_eq!(Spell::ALL.map(Spell::spell_id),
  progress::SPELL_IDS, ...)`); `control` gains `mut prog: ResMut<Progress>` and one
  `Some("prog")` arm (warn!, never error!); `publish` gains `prog: Res<Progress>` and appends
  `progress`, `progRev`, `progSave` after `fireInView`. `reset` untouched.
- `build-web.sh:21`: `web/progress.js` added to the `cp` list (only line changed).
- `web/progress.js` stub (constants + no-op `startProgress`), `boot.js` guarded dynamic import
  right after `startHud();`.
- golden key-order lines: `I027_KEYS` after `I025_KEYS`, added to the length sum and order.

`git diff -U0 origin/main -- crates/bevy-client/src/main.rs` hunks: mod list (23), Progress struct
(143-147), init_resource (382), boot check (507-512), control param (761), match arm (845-849),
publish param (1687), format string (1714), argument tail (1768-1771). None of I-046's stream hunks.

## Checks on the stub

| check | result |
|---|---|
| `bash build-web.sh` (wasm32 release, CARGO_TARGET_DIR=/home/claude/bevy-target) | pass, 3m04s, wasm gzip 5 084 359 B |
| `pkg-out/progress.js` byte-identical to `web/progress.js` | yes |
| `cargo fmt --check` | pass |
| `npm test` (incl. `test/spells.test.ts`) | pass |
| golden on the stub (with the web lane's new checks) | 130/139; see below |

Golden on the stub (`QUERY=?low`, box shared with I-046's wasm build during the first minutes):
the game boots (so the boot check held), the key-order check in `clockAndBudget` passed with the
three neutral keys. Failures:

| check | why |
|---|---|
| streams: fireInView is a boolean, the last state key | pinned `fireInView` as the very last key; see Deviation |
| cast: a real click casts exactly once | known wall-time flake on main under box load (recorded, not touched) |
| progress: 17, 18, 19, 23a, 22, 20, 21 | expected on the stub (no bodies yet) |

## Deviation: one existing golden check edited (needs leader/critic sign-off)

The spec's overlap table missed I-025's check `streams: fireInView is a boolean, the last state
key` (`golden.mjs:1325`), which required `keys[keys.length - 1] === 'fireInView'`. Appending
keys after `fireInView`, as the I-027 Contract requires, cannot pass it. The edit keeps the same
strictness for the new tail: `fireInView` must be a boolean, directly after `fxWater`, and followed
by exactly `progress,progRev,progSave` (the check name is unchanged). Nothing else in that check
changed. It is the same kind of edit as the sanctioned key-order edit in `clockAndBudget`.
