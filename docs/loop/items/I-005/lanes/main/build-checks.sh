#!/usr/bin/env bash
# Acceptance test 2 checks on web/dist.
cd "$1"
D=web/dist
grep -E "build-site:|wasm gzip|wasm brotli" "$2"
echo "canvas: $(grep -c '<canvas id="game"' $D/index.html)"
echo "switch on: $(grep -c 'name="rg-analytics" content="on"' $D/index.html)"
ls -l $D/boot.js $D/hud.js $D/analytics.js $D/age.js $D/bevy-client.js $D/bevy-client_bg.wasm $D/classic/index.html $D/dashboard/index.html $D/_headers
echo "connect-src self in _headers: $(grep -c "connect-src 'self'" $D/_headers)"
echo "source switch off: $(grep -c 'name="rg-analytics" content="off"' crates/bevy-client/web/index.html)"
echo "pkg-out switch off: $(grep -c 'name="rg-analytics" content="off"' crates/bevy-client/web/pkg-out/index.html)"
echo "classic refs:"; grep -oE '(src|href)="[^"]+"' $D/classic/index.html
echo "urls in root files (expect none outside comments):"; grep -nE 'https?://' $D/index.html $D/boot.js $D/analytics.js $D/age.js || echo "none"
echo "classic/_headers present: $(test -e $D/classic/_headers && echo yes || echo no)"
ls $D $D/classic
