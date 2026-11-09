#!/usr/bin/env bash
# Source-painting continuity audit for proposed clean shop art.
# This protects scene landmarks; it does NOT prove characters were removed,
# alpha sprites exist, or the owner's visual acceptance was obtained.
set -euo pipefail
if [[ $# != 2 ]]; then
  echo 'Usage: check-store-background-continuity.sh ORIGINAL.png CANDIDATE.png' >&2
  exit 2
fi
baseline=$1 candidate=$2
for tool in magick awk; do
  command -v "$tool" >/dev/null || { echo "ERROR: missing $tool" >&2; exit 1; }
done
for input in "$baseline" "$candidate"; do
  [[ -s $input ]] || { echo "ERROR: missing shop art: $input" >&2; exit 1; }
  dimensions=$(magick identify -format '%wx%h' "$input") || {
    echo "ERROR: shop image does not decode: $input" >&2; exit 1;
  }
  [[ $dimensions == 1280x720 ]] || {
    echo "ERROR: shop image must be the original 1280x720 canvas, found $dimensions: $input" >&2
    exit 1
  }
done
# Control crops are outside the original baked-in shopkeeper/cat silhouettes.
# Normalize each region's mean absolute color error. At most 4% permits
# subtle color cleanup, but excludes shifted registers and generic redraws.
regions=(
  'chalkboard:185x260+176+6'
  'cash-register:90x135+86+303'
  'mug:70x72+454+392'
  'ledger:55x15+540+450'
  'candy-jars:174x160+1060+305'
  'counter-front:850x49+16+510'
  'floor:1150x102+40+605'
)
for item in "${regions[@]}"; do
  label=${item%%:*}
  crop=${item#*:}
  result=$(magick compare -metric MAE "${baseline}[${crop}]" "${candidate}[${crop}]" null: 2>&1) || :
  normalized=$(printf '%s\n' "$result" | sed -nE 's/.*\(([0-9.eE+-]+)\).*/\1/p')
  [[ -n $normalized ]] || {
    echo "ERROR: cannot compare shop landmark $label: $result" >&2; exit 1;
  }
  if ! awk -v value="$normalized" 'BEGIN { exit !(value >= 0 && value <= 0.04) }'; then
    echo "ERROR: shop landmark $label changed too much (normalized pixel error $normalized > 0.04)" >&2
    exit 1
  fi
  printf 'Shop original landmark preserved: %-16s MAE=%s\n' "$label" "$normalized"
done
echo 'Shop continuity reference OK: stable original scenery preserved across seven sampled regions.'