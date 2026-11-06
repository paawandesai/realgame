#!/usr/bin/env bash
# Regenerates content/chem/evidence.csv, the per-reaction evidence table a
# teacher can review (docs/loop/items/I-058/spec.md). The chem test
# `the_teacher_table_is_current` fails until this has been run after a
# change to core.json, rules.json or the show data.
set -euo pipefail
cd "$(dirname "$0")/.."
out=content/chem/evidence.csv
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
cargo run -q --locked -p realgame-chem --example evidence_csv > "$tmp"
mv "$tmp" "$out"
trap - EXIT
echo "wrote $out ($(($(wc -l < "$out") - 1)) reactions)"
