#!/usr/bin/env bash
# Fail-closed source-painting continuity audit for a proposed CLEAN shop background.
# This checks fixed, unoccluded landmarks, not whether Mateo or cat were removed.
# Distinct character/cat source, alpha, edge, occlusion and in-game QA is still required.
set -euo pipefail
if [[ $# -ne 2 ]]; then
  echo 'Usage: check-store-background-continuity.sh ORIGINAL.png CANDIDATE.png' >&2
  exit 2
fi
baseline=$1 candidate=$2
for tool in magick awk sed; do
  command -v "$tool" >/dev/null || { echo "ERROR: missing $tool" >&2; exit 1; }
done
for input in "$baseline" "$candidate"; do
  [[ -s $input ]] || { echo "ERROR: missing shop art: $input" >&2; exit 1; }
  dimensions=$(magick identify -format '%wx%h' "$input") || {
    echo "ERROR: undecodable shop image: $input" >&2; exit 1;
  }
  [[ $dimensions == 1280x720 ]] || {
    echo "ERROR: shop image must remain 1280x720, found $dimensions: $input" >&2; exit 1;
  }
done

# Coordinates: x,y,w,h for stable, character-free regions in the actual
# 1280x720 painting. A regenerated or rearranged store is not an inpainting
# edit, even when it looks attractive. <=4% mean normalized pixel error per
# region permits slight color corrections without moving authored landmarks.
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
    echo "ERROR: shop landmark $label changed (normalized MAE $normalized exceeds 0.04)" >&2
    exit 1
  fi
  printf 'Shop source landmark preserved: %-16s MAE=%s\n' "$label" "$normalized"
done
echo 'Shop continuity reference OK: seven unoccluded source-painting regions preserved.'
