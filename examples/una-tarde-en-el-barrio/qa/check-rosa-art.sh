#!/usr/bin/env bash
# Verify the installed sprite has transparency and is derived from preserved source.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v magick >/dev/null || { echo 'ImageMagick required for Rosa provenance check' >&2; exit 1; }
src="$root/sources/characters/rosa-fullbody-anime-v076.png"
out="$root/assets/npcs/rosa.png"
[[ -s "$src" && -s "$out" ]] || { echo 'Rosa source or sprite missing' >&2; exit 1; }
[[ $(magick identify -format '%m' "$out") == PNG ]] || { echo 'Rosa sprite is not a PNG' >&2; exit 1; }
[[ $(magick identify -format '%[opaque]' "$out") == False ]] || { echo 'Rosa sprite lacks transparency' >&2; exit 1; }
height=$(magick identify -format '%h' "$out")
[[ $height -ge 1200 ]] || { echo 'Rosa art source resolution too small' >&2; exit 1; }
coverage=$(magick "$out" -alpha extract -format '%[fx:mean]' info:)
awk -v x="$coverage" 'BEGIN {exit !(x > 0.2 && x < 0.8)}' || { echo "Rosa alpha coverage invalid: $coverage" >&2; exit 1; }
# Regenerate in memory and compare exact pixels to catch asset/source mismatch.
expected=$(mktemp --suffix=.png)
trap 'rm -f "$expected"' EXIT
magick "$src" -channel A -evaluate Subtract 2% +channel -trim +repage "$expected"
expected_hash=$(magick "$expected" -depth 8 RGBA:- | sha256sum | cut -d' ' -f1)
actual_hash=$(magick "$out" -depth 8 RGBA:- | sha256sum | cut -d' ' -f1)
[[ $actual_hash == "$expected_hash" ]] || { echo 'Rosa sprite no longer matches source' >&2; exit 1; }
echo "Rosa art OK: derived transparent ${height}px sprite; alpha coverage $coverage"
