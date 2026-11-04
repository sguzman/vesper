#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
out="${1:-$root/qa/street-static-art-composite.png}"
magick "$root/assets/backgrounds/street.png" -resize 1280x720! "$out"
for item in 'assets/avatars/idle.png 145 132' 'assets/npcs/elena.png 250 132' 'assets/npcs/rosa.png 830 132'; do
  read -r name x height <<<"$item"
  sprite=$(mktemp --suffix=.png)
  magick "$root/$name" -resize "x$height" "$sprite"
  w=$(identify -format '%w' "$sprite")
  h=$(identify -format '%h' "$sprite")
  left=$((x-w/2)); top=$((618-h))
  magick "$out" "$sprite" -geometry "+$left+$top" -compose over -composite "$out"
  rm -f "$sprite"
done
printf 'Static asset composite, not Vesper runtime screenshot: %s\n' "$out"
