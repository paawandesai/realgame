# Mac real-GPU perf run (B2, Mac half)

This is the recipe for the real-GPU half of bar B2: **WebGL2 p5 fps >= 55 over 60 s of the golden-path
workload** on Paawan's Mac (MacBook Pro M5 Pro, Chrome 155). The cloud has a software GPU only, so this
number can come only from the Mac. The orchestrator pastes the commands below into the Remote Control
session on the Mac, one fenced block at a time. The only placeholder is `<sha>`.

The script is `tests/e2e/perf.mjs` in `--mac` mode. It starts the installed Google Chrome
(`channel: 'chrome'`, a visible window, the real GPU), loads the default tier (no `?low`, no throttle) at
1512x945, waits for the first interactive frame, warms up for 5 s, then drives the same fixed golden-path
workload as the cloud run (move, look, aim at targets, select and cast spells 1 to 4) for 60 s. fps comes
from the script's own `requestAnimationFrame` hook: `fps_p5 = 1000 / (95th percentile frame interval)`.

## 1. Preconditions

Check these by hand before the run. Do not change anything else on the machine.

- The charger is connected.
- Low Power Mode is off (System Settings > Battery).
- No other heavy apps are running (close builds, video calls and other browser windows with 3D content).
- The Chrome window the script opens stays in front and is not minimized or covered. A background or
  hidden window throttles `requestAnimationFrame`, and the result is then worthless.
- The display runs at its native refresh rate. ProMotion caps fps at 120, so `fps_p5` can never exceed
  the refresh rate. 55 is well under it.

## 2. Sync

`<sha>` is the commit under test: the merge commit of the item being measured on `main`, or the PR head
commit (`gh pr view <number> --json headRefOid`) if it has not merged yet. The orchestrator fills it in.

```sh
git -C ~/Documents/realgame fetch origin
```

```sh
git -C ~/Documents/realgame checkout --detach <sha>
```

## 3. One-time tools

Each check prints what it finds. Run the install line under it only if the check fails.

```sh
rustup target list --installed | grep wasm32-unknown-unknown
```

```sh
rustup target add wasm32-unknown-unknown
```

The wasm-bindgen CLI must be exactly 0.2.129 (it must match `wasm-bindgen` in `crates/bevy-client/Cargo.lock`):

```sh
wasm-bindgen --version
```

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

Node must be 18 or later:

```sh
node -v
```

Playwright 1.63.0, the same version as the cloud. `--no-save` leaves `package.json` and the lockfile
alone. No browser download is needed, because the run uses the installed Chrome.

```sh
cd ~/Documents/realgame && npm i --no-save playwright@1.63.0
```

## 4. Build

A WebGL2 build (the default). Never set `WEBGPU=1` for this run. The first build takes 5 to 10 minutes.

```sh
cd ~/Documents/realgame && CARGO_TARGET_DIR=~/bevy-target bash crates/bevy-client/build-web.sh
```

## 5. Run

Takes about 90 s: load, 5 s of warm-up, 60 s of sampling. A Chrome window opens; leave it in front and
do not touch the mouse or keyboard until the command returns.

```sh
cd ~/Documents/realgame && node tests/e2e/perf.mjs --mac crates/bevy-client/web/pkg-out | tee ~/perf-mac-<sha>.json
```

The exit code is 0 for a pass and 1 for a fail. Progress lines starting with `[perf]` go to stderr.

## 6. Read the result

Copy these fields back into the project thread:

```sh
jq '{based_on, pass, error, B2_mac: .verdict.B2.mac, fps_p50: .runs.mac.fps_p50, fps_p5: .runs.mac.fps_p5, stats_fps_p5_log: .runs.mac.stats_fps_p5_log, renderer: .runs.mac.renderer, webgl2: .runs.mac.webgl2, errors: .runs.mac.console_errors.count, panics: .runs.mac.panics, workload_ok: .runs.mac.workload_ok}' ~/perf-mac-<sha>.json
```

Pass rule: `fps_p5 >= 55`, `webgl2 == true`, 0 console errors, 0 panics and `workload_ok == true`. The
JSON's `pass` field already applies all of these. `renderer` should name the Apple GPU (for example
`ANGLE (Apple, ANGLE Metal Renderer: Apple M5 Pro, ...)`). A renderer naming SwiftShader means Chrome fell
back to software rendering, and the run does not count. `stats_fps_p5_log` is the game's own `STATS`
figure, a cross-check only.

## 7. Recording

The Mac session does not edit any repository file. The orchestrator appends the result to
`docs/loop/scoreboard.jsonl` (and updates `docs/loop/checks.md` "Last run") as one JSON line:

```
{"date":"YYYY-MM-DDTHH:MMZ","bar":"B2.mac","sha":"<sha>","fps_p50":0,"fps_p5":0,"renderer":"...","webgl2":true,"errors":0,"pass":false,"source":"perf.mjs --mac"}
```

## 8. Troubleshooting

- **fps stuck at 60 or 30:** Low Power Mode is on, the Mac is on battery, or the Chrome window was in the
  background or covered. Fix the precondition in step 1 and run step 5 again.
- **`channel 'chrome' not found` or "Chromium distribution 'chrome' is not found":** Google Chrome is
  not installed in `/Applications`. Install Chrome normally, then run step 5 again. Do not try a
  different browser: the bar is measured in Chrome.
- **wasm-bindgen version mismatch** (the build stops with "rust Wasm file schema version" or "it looks
  like the Rust project used to create this Wasm file was linked against version ..."): reinstall the
  CLI with the `cargo install wasm-bindgen-cli --version 0.2.129 --locked` line in step 3, then build again.
- **Black canvas, `error: "not a WebGL2 build"`, or "Unable to find a GPU":** the output was built with
  `WEBGPU=1`. Build again with the exact command in step 4 (no `WEBGPU`), then run step 5 again.
- **`no interactive frame within 120 s`:** the build in `crates/bevy-client/web/pkg-out` is missing or
  stale. Run step 4 again.
