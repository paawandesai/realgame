#!/usr/bin/env sh
# Type-check the web shell. Used by the pre-commit hook.
# tsc needs the wasm-pack output in web/src/pkg (it imports its .d.ts).
# Building it from cold takes about 15 s, which is over the pre-commit budget,
# so if it is missing we say so and leave it to pre-push and CI, which always
# build it first.
set -eu
cd "$(git rev-parse --show-toplevel)/web"
if [ ! -d node_modules ]; then
  npm ci --no-audit --no-fund --silent
fi
if [ ! -f src/pkg/realgame_sim.d.ts ]; then
  echo "pre-commit: web/src/pkg is missing, skipping tsc here (pre-push and CI run it). Build it once with: npm --prefix web run wasm" >&2
  exit 0
fi
exec npx --no-install tsc --noEmit
