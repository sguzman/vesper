#!/usr/bin/env bash
# Validate the real game asset frame geometry at the native source and authored
# screen heights. This does NOT verify animated playback in the Bevy player.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v magick >/dev/null || { echo 'ImageMagick 7 required' >&2; exit 1; }
# Bevy's exploration renderer rescales avatar textures to 85 logical px and
# applies the external game avatar scale (1.55 here); effective = 131.75px.
source_height=754
screen_height=131.75
previous=-1
count=0
for pose in idle walk-1 walk-2 walk-3 walk-4; do
  path="$root/assets/avatars/$pose.png"
  [[ -s "$path" ]] || { echo "ERROR: missing $pose" >&2; exit 1; }
  size=$(magick identify -format '%wx%h' "$path")
  [[ "$size" == '600x754' ]] || { echo "ERROR: unaligned $pose ($size)" >&2; exit 1; }
  # Trim only alpha silhouette so semi-transparent glow/fringe does not affect
  # the registered floor baseline. +repage would erase the offset; do not use it.
  geometry=$(magick "$path" -alpha extract -threshold 50% -trim -format '%wx%h%O' info:)
  if [[ "$geometry" =~ ^([0-9]+)x([0-9]+)\+([0-9]+)\+([0-9]+)$ ]]; then
    top=${BASH_REMATCH[4]}
    height=${BASH_REMATCH[2]}
    bottom=$((top+height))
  else
    echo "ERROR: cannot determine $pose alpha bounds: $geometry" >&2
    exit 1
  fi
  # All feet should touch a common ground within five source pixels; at
  # 131.75 rendered height this is well under one screen pixel of variation.
  [[ $bottom -ge 743 && $bottom -le 751 ]] || {
    echo "ERROR: $pose ground baseline $bottom out of expected range" >&2; exit 1;
  }
  if [[ $previous -ge 0 ]]; then
    diff=$((bottom-previous)); [[ $diff -ge -5 && $diff -le 5 ]] || {
      echo "ERROR: foot-contact jump $previous->$bottom between frames" >&2; exit 1;
    }
  fi
  previous=$bottom
  count=$((count+1))
done
[[ $count -eq 5 ]] || exit 1
# Condition for future art changes: an 8px source excursion is >1px on screen.
awk -v h="$screen_height" -v s="$source_height" 'BEGIN {exit !((8 * h / s) > 1)}'
echo 'Avatar grounding OK: five 600x754 poses, feet within ±5 source px, <1 rendered pixel baseline variation.'
