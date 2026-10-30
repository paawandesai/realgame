#!/usr/bin/env bash
# Build command for the GitHub-connected Cloudflare Pages setup (docs/DEPLOY.md,
# option B). Cloudflare's build image has Node but not the Rust toolchain or
# wasm-pack (classic game) or wasm-bindgen-cli (3D client), so install them
# first, pinned to what CI uses and to crates/bevy-client/Cargo.lock.
set -euo pipefail
cd "$(dirname "$0")/.."

WASM_PACK_VERSION="0.15.0"
# Must equal the wasm-bindgen version in crates/bevy-client/Cargo.lock.
WASM_BINDGEN_VERSION="0.2.129"

if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
rustup target add wasm32-unknown-unknown
if ! wasm-pack --version 2>/dev/null | grep -q "$WASM_PACK_VERSION"; then
  cargo install wasm-pack --locked --version "$WASM_PACK_VERSION"
fi
if ! wasm-bindgen --version 2>/dev/null | grep -q "$WASM_BINDGEN_VERSION"; then
  cargo install wasm-bindgen-cli --locked --version "$WASM_BINDGEN_VERSION"
fi

bash scripts/build-site.sh
