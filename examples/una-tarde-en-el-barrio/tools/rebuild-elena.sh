#!/usr/bin/env bash
# Export transparent production sprites from the authored high-resolution RGBA original.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v magick >/dev/null || { echo 'ImageMagick required' >&2; exit 1; }
source="$root/sources/characters/elena-fullbody-anime-v074.png"
[[ -s $source ]] || { echo "Missing $source" >&2; exit 1; }
# Subtract only near-zero alpha haze, then trim transparent source margins.
magick "$source" -channel A -evaluate Subtract 2% +channel \
  -trim +repage "$root/assets/npcs/elena.png"
# Calm expression portrait from exactly the same full-body design and palette.
# Crop the head and upper torso; pad instead of stretching painted artwork.
magick "$root/assets/npcs/elena.png" -gravity North -crop 100%x610+0+0 +repage \
  -resize x500 -gravity Center -background none -extent 520x500 \
  "$root/assets/npcs/elena-portrait-provisional.png"
