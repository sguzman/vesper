#!/usr/bin/env bash
# Rebuild the active Rosa NPC sprite from its original transparent character illustration.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v magick >/dev/null || { echo 'ImageMagick required' >&2; exit 1; }
source="$root/sources/characters/rosa-fullbody-anime-v076.png"
[[ -s "$source" ]] || { echo "Missing Rosa art source: $source" >&2; exit 1; }
# Remove low-opacity generation haze. No background-guessing or rescaling the
# artwork into an opaque 1280x720 stage; Bevy sizes it at the authored NPC height.
magick "$source" -channel A -evaluate Subtract 2% +channel \
  -trim +repage "$root/assets/npcs/rosa.png"
