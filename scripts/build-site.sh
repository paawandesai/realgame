#!/usr/bin/env bash
# Builds everything Cloudflare Pages serves into web/dist:
#   1. the game (wasm-pack, tsc, Vite), exactly as `npm --prefix web run build`
#   2. the analytics dashboard (static files from dashboard/) at /dashboard/
# The Pages Functions in functions/ are bundled by wrangler at deploy time.
set -euo pipefail
cd "$(dirname "$0")/.."

[ -d web/node_modules ] || npm --prefix web ci --no-audit --no-fund
npm --prefix web run build

rm -rf web/dist/dashboard
mkdir -p web/dist/dashboard
cp dashboard/index.html dashboard/dashboard.js dashboard/dashboard.css web/dist/dashboard/
echo "build-site: web/dist is ready ($(du -sh web/dist | cut -f1)), dashboard at /dashboard/"
