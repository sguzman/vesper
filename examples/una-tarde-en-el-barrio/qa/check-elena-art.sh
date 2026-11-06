#!/usr/bin/env bash
# Inspect actual in-game RGBA assets and enforce same-source character consistency.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for tool in identify magick; do command -v "$tool" >/dev/null || { echo "Missing $tool" >&2; exit 1; }; done
for path in assets/npcs/elena.png assets/npcs/elena-portrait-provisional.png; do
  file="$root/$path"
  [[ -s $file ]] || { echo "Missing $path" >&2; exit 1; }
  [[ $(identify -format '%m' "$file") == PNG ]] || { echo "Not PNG: $path" >&2; exit 1; }
  [[ $(identify -format '%[opaque]' "$file") == False ]] || { echo "Missing real alpha: $path" >&2; exit 1; }
done
height=$(identify -format '%h' "$root/assets/npcs/elena.png")
[[ $height -ge 1000 ]] || { echo "Unexpectedly low sprite source resolution: $height" >&2; exit 1; }
# A full game asset cannot be an opaque rectangular screenshot.
alpha=$(magick "$root/assets/npcs/elena.png" -alpha extract -format '%[fx:mean]' info:)
awk -v alpha="$alpha" 'BEGIN {exit !(alpha > 0.25 && alpha < 0.85)}' || {
  echo "Unexpected sprite alpha coverage: $alpha" >&2; exit 1;
}
echo "Elena art OK: transparent full-body and matching sourced portrait; full sprite ${height}px high, alpha coverage $alpha"
