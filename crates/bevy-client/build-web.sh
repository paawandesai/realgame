#!/usr/bin/env bash
# Builds the first-person prototype for the browser into web/pkg-out/ (gitignored).
# Needs: rustup target wasm32-unknown-unknown, and wasm-bindgen-cli at the
# exact version of wasm-bindgen in Cargo.lock (0.2.129). wasm-opt is optional
# (it saved under 1% compressed in the spike) and runs only with WASM_OPT=1.
#   ./build-web.sh                      WebGL2 build (default features)
#   WEBGPU=1 ./build-web.sh             WebGPU build (no WebGL2 fallback)
#   LEAN=1 ./build-web.sh               no bloom / LUT tonemapping (smaller)
set -euo pipefail
cd "$(dirname "$0")"
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
FEATURES=()
if [[ "${WEBGPU:-0}" == 1 ]]; then
  FEATURES=(--no-default-features --features "$([[ "${LEAN:-0}" == 1 ]] && echo webgpu || echo bloom,webgpu)")
elif [[ "${LEAN:-0}" == 1 ]]; then
  FEATURES=(--no-default-features --features webgl2)
fi
cargo build --release --target wasm32-unknown-unknown ${FEATURES[@]+"${FEATURES[@]}"}
OUT=web/pkg-out
rm -rf "$OUT" && mkdir -p "$OUT"
cp web/index.html web/style.css web/boot.js web/hud.js "$OUT/"
wasm-bindgen --target web --no-typescript --out-dir "$OUT" --out-name bevy-client \
  "$TARGET_DIR/wasm32-unknown-unknown/release/bevy-client.wasm"
if [[ "${WASM_OPT:-0}" == 1 ]]; then
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
    --enable-mutable-globals --enable-reference-types --enable-multivalue \
    "$OUT/bevy-client_bg.wasm" -o "$OUT/bevy-client_bg.wasm"
fi
ls -l "$OUT"
gzip -9c "$OUT/bevy-client_bg.wasm" | wc -c | xargs echo "wasm gzip bytes:"
if command -v brotli >/dev/null; then brotli -c -q 11 "$OUT/bevy-client_bg.wasm" | wc -c | xargs echo "wasm brotli bytes:"; fi
