#!/usr/bin/env bash
# Builds gfx-lab twice (WebGL2 and WebGPU) into web/pkg-out/{webgl2,webgpu}.
# Needs the wasm32 target and wasm-bindgen-cli 0.2.129 (same as bevy-client).
#   ./build-web.sh            both builds
#   API=webgl2 ./build-web.sh one of them
set -euo pipefail
cd "$(dirname "$0")"
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
OUT=web/pkg-out
mkdir -p "$OUT"
cp web/index.html web/lab.css web/boot.js "$OUT/"
for api in ${API:-webgl2 webgpu}; do
  cargo build --release --locked --target wasm32-unknown-unknown \
    --no-default-features --features "$api" --target-dir "$TARGET_DIR/$api"
  rm -rf "$OUT/$api"
  wasm-bindgen --target web --no-typescript --out-dir "$OUT/$api" --out-name gfx-lab \
    "$TARGET_DIR/$api/wasm32-unknown-unknown/release/gfx-lab.wasm"
  if command -v brotli >/dev/null; then
    brotli -c -q 11 "$OUT/$api/gfx-lab_bg.wasm" | wc -c | xargs echo "$api wasm brotli bytes:"
  else
    gzip -9c "$OUT/$api/gfx-lab_bg.wasm" | wc -c | xargs echo "$api wasm gzip bytes:"
  fi
done
