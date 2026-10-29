#!/usr/bin/env bash
# Build command for the GitHub-connected Cloudflare Pages setup (docs/DEPLOY.md,
# option B). Cloudflare's build image has Node but not the Rust toolchain or
# wasm-pack, so install them first, pinned to what CI uses.
set -euo pipefail
cd "$(dirname "$0")/.."

WASM_PACK_VERSION="0.15.0"

if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
rustup target add wasm32-unknown-unknown
if ! wasm-pack --version 2>/dev/null | grep -q "$WASM_PACK_VERSION"; then
  cargo install wasm-pack --locked --version "$WASM_PACK_VERSION"
fi

bash scripts/build-site.sh
