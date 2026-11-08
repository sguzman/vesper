#!/usr/bin/env bash
# Inspect actual in-game RGBA assets and enforce same-source character consistency.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for tool in identify magick; do command -v "$tool" >/dev/null || { echo "Missing $tool" >&2; exit 1; }; done
for path in assets/npcs/elena.png assets/npcs/elena-portrait-provisional.png; do
  file="$root/$path"
  [[ -s $file ]] || { echo "Missing $path" >&2; exit 1; }
  [[ $(identify -format '%m' "$file") == PNG ]] || { echo "Not PNG: $path" >&2; exit 1; }
  opaque=$(identify -format '%[opaque]' "$file")
  [[ ${opaque,,} == false ]] || { echo "Missing real alpha: $path" >&2; exit 1; }
done
height=$(identify -format '%h' "$root/assets/npcs/elena.png")
[[ $height -ge 1000 ]] || { echo "Unexpectedly low sprite source resolution: $height" >&2; exit 1; }
# A full game asset cannot be an opaque rectangular screenshot.
alpha=$(magick "$root/assets/npcs/elena.png" -alpha extract -format '%[fx:mean]' info:)
awk -v alpha="$alpha" 'BEGIN {exit !(alpha > 0.25 && alpha < 0.85)}' || {
  echo "Unexpected sprite alpha coverage: $alpha" >&2; exit 1;
}
# Regenerate both game assets from one preserved original under the same
# ImageMagick toolchain. Compare decoded pixels rather than PNG compression.
# A portrait that merely contains transparency is not proof of same-source art.
shadow=$(mktemp -d)
trap 'rm -rf -- "$shadow"' EXIT
mkdir -p "$shadow/sources/characters" "$shadow/assets/npcs" "$shadow/tools"
cp -- "$root/sources/characters/elena-fullbody-anime-v074.png" "$shadow/sources/characters/"
cp -- "$root/tools/rebuild-elena.sh" "$shadow/tools/"
bash "$shadow/tools/rebuild-elena.sh"
for asset in elena.png elena-portrait-provisional.png; do
  actual="$root/assets/npcs/$asset"
  expected="$shadow/assets/npcs/$asset"
  actual_rgba="$shadow/actual-$asset.rgba"
  expected_rgba="$shadow/expected-$asset.rgba"
  magick "$actual" -depth 8 "RGBA:$actual_rgba"
  magick "$expected" -depth 8 "RGBA:$expected_rgba"
  [[ $(magick identify -format '%wx%h' "$actual") == $(magick identify -format '%wx%h' "$expected") ]] || {
    echo "Elena source derivation dimensions changed for $asset" >&2
    exit 1
  }
  cmp -s -- "$actual_rgba" "$expected_rgba" || {
    echo "Elena asset $asset no longer matches the preserved same-character source" >&2
    exit 1
  }
done
echo "Elena art OK: transparent full-body and matching source-rebuilt portrait; full sprite ${height}px high, alpha coverage $alpha"
