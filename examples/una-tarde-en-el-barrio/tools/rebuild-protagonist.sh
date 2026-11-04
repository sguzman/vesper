#!/usr/bin/env bash
# Deterministically extract five aligned RGBA game sprites from one original sheet.
# The source figures were laid out as five unique, nonoverlapping poses. Equal
# 600x754 canvases keep Bevy's natural-height scaling and pivot constant.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v magick >/dev/null || { echo 'ImageMagick 7 required' >&2; exit 1; }
sheet="$root/sources/characters/protagonist-idle-walk-sheet-v077.png"
[[ -s "$sheet" ]] || { echo 'Missing original protagonist spritesheet' >&2; exit 1; }
[[ $(magick identify -format '%wx%h' "$sheet") == 1983x793 ]] || {
  echo 'Protagonist sheet dimensions changed; review sprite cuts before export' >&2; exit 1;
}
# x-start, cut width, canvas x-offset: head remains centered across all poses.
starts=(0 330 806 1150 1635)
widths=(330 476 344 485 348)
offsets=(125 75 144 100 170)
names=(idle walk-1 walk-2 walk-3 walk-4)
for i in "${!starts[@]}"; do
  magick -size 600x754 canvas:none \
    \( "$sheet" -crop "${widths[i]}x754+${starts[i]}+0" +repage \
       -channel A -evaluate Subtract 2% +channel \) \
    -geometry "+${offsets[i]}+0" -compose over -composite \
    "$root/assets/avatars/${names[i]}.png"
done
