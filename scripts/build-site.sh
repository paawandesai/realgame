#!/usr/bin/env bash
# Builds everything Cloudflare Pages serves into web/dist:
#   1. the first-person 3D client (crates/bevy-client) at the site root, with
#      its rg-analytics switch flipped to "on" (anonymous analytics and the age
#      picker; docs/ANALYTICS.md). The source page and web/pkg-out stay "off".
#   2. the classic 2D game (web/, wasm-pack + tsc + Vite) at /classic/
#   3. the strict security headers (web/public/_headers) for the whole site
#   4. the analytics dashboard (static files from dashboard/) at /dashboard/
# The Pages Functions in functions/ are bundled by wrangler at deploy time.
#
# Needs: the Rust wasm32 target and wasm-bindgen-cli 0.2.129 (see
# crates/bevy-client/README.md), wasm-pack for the classic game, and Node.
# CARGO_TARGET_DIR and PATH are honoured.
set -euo pipefail
cd "$(dirname "$0")/.."

OUT=web/dist
CLIENT=crates/bevy-client/web
fail() {
  echo "build-site: $*" >&2
  exit 1
}

# 1. The 3D client at the root.
bash crates/bevy-client/build-web.sh
rm -rf "$OUT"
mkdir -p "$OUT"
cp -R "$CLIENT/pkg-out/." "$OUT/"
# The analytics modules are loaded on demand by boot.js only when the switch
# is "on", so they are copied here rather than into pkg-out.
cp "$CLIENT/analytics.js" "$CLIENT/age.js" "$OUT/"
OFF='<meta name="rg-analytics" content="off">'
ON='<meta name="rg-analytics" content="on">'
[ "$(grep -cF "$OFF" "$OUT/index.html")" = 1 ] || fail "expected exactly one rg-analytics=off switch in $OUT/index.html"
sed -i.bak "s|$OFF|$ON|" "$OUT/index.html"
rm -f "$OUT/index.html.bak"
[ "$(grep -cF "$ON" "$OUT/index.html")" = 1 ] || fail "rg-analytics switch was not flipped to on"
grep -qF "$OFF" "$OUT/index.html" && fail "an rg-analytics=off switch is left in $OUT/index.html"
grep -qF "$OFF" "$CLIENT/index.html" || fail "the source $CLIENT/index.html must keep rg-analytics=off"

# 2. The classic 2D game at /classic/ (arguments after -- go to `vite build`).
[ -d web/node_modules ] || npm --prefix web ci --no-audit --no-fund
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
npm --prefix web run build -- --base=/classic/ --outDir "$TMP/classic" --emptyOutDir
mv "$TMP/classic" "$OUT/classic"

# 3. Headers at the root, so the strict CSP covers the whole site.
cp web/public/_headers "$OUT/_headers"
rm -f "$OUT/classic/_headers"

# 4. The dashboard.
mkdir -p "$OUT/dashboard"
cp dashboard/index.html dashboard/dashboard.js dashboard/dashboard.css "$OUT/dashboard/"
echo "build-site: $OUT is ready ($(du -sh "$OUT" | cut -f1)): 3D client at /, classic game at /classic/, dashboard at /dashboard/"
