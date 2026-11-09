#!/usr/bin/env bash
# Deterministic lossless export ONLY. Image authors must provide the four
# coherent independent originals before this command can run.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v magick >/dev/null || { echo 'ERROR: ImageMagick 7 required' >&2; exit 1; }

require_source() {
  [[ -s "$root/$1" ]] || { echo "ERROR: missing source art: $1" >&2; exit 1; }
}
require_source sources/backgrounds/store-clean.png
require_source sources/characters/mateo-isolated.png
require_source sources/characters/shop-cat-isolated.png
require_source sources/layers/shop-counter-front.png
mkdir -p "$root/assets/backgrounds" "$root/assets/npcs" \
  "$root/assets/props" "$root/assets/layers"
# No trimming, background removal, guessed silhouette extraction or scaling:
# the artist owns full transparency and counter perspective in the source.
magick "$root/sources/backgrounds/store-clean.png" -depth 8 \
  "PNG24:$root/assets/backgrounds/store-clean.png"
magick "$root/sources/characters/mateo-isolated.png" -depth 8 \
  "PNG32:$root/assets/npcs/mateo.png"
magick "$root/sources/characters/shop-cat-isolated.png" -depth 8 \
  "PNG32:$root/assets/props/shop-cat.png"
magick "$root/sources/layers/shop-counter-front.png" -depth 8 \
  "PNG32:$root/assets/layers/shop-counter-front.png"
echo 'Store layer art exported from four independent source images (visual approval still required).'
