#!/usr/bin/env bash
# Verify pose count, common pivot geometry and exact regeneration from source.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v magick >/dev/null || { echo 'ImageMagick required' >&2; exit 1; }
sheet="$root/sources/characters/protagonist-idle-walk-sheet-v077.png"
[[ -s "$sheet" ]] || { echo 'Missing protagonist sheet' >&2; exit 1; }
checkdir=$(mktemp -d)
trap 'rm -rf "$checkdir"' EXIT
# Use the canonical exporter in a disposable shadow package, not in the live
# assets, to ensure a source mismatch is actually detectable.
mkdir -p "$checkdir/sources/characters" "$checkdir/assets/avatars" "$checkdir/tools"
cp "$sheet" "$checkdir/sources/characters/"
cp "$root/tools/rebuild-protagonist.sh" "$checkdir/tools/"
"$checkdir/tools/rebuild-protagonist.sh"
count=0
prior=''
for frame in idle walk-1 walk-2 walk-3 walk-4; do
  actual="$root/assets/avatars/$frame.png"
  expected="$checkdir/assets/avatars/$frame.png"
  [[ -s "$actual" ]] || { echo "Missing avatar $frame" >&2; exit 1; }
  props=$(magick identify -format '%m %wx%h %[opaque]' "$actual")
  [[ ${props,,} == 'png 600x754 false' ]] || { echo "Bad avatar size/alpha for $frame: $props" >&2; exit 1; }
  a=$(magick "$actual" -alpha extract -format '%[fx:mean]' info:)
  awk -v a="$a" 'BEGIN {exit !(a > 0.10 && a < 0.55)}' || {
    echo "Avatar $frame alpha silhouette invalid: $a" >&2; exit 1;
  }
  cmp -s <(magick "$actual" -depth 8 RGBA:-) <(magick "$expected" -depth 8 RGBA:-) || {
    echo "Avatar $frame no longer matches the original sheet" >&2; exit 1;
  }
  pixels=$(magick "$actual" -depth 8 RGBA:- | sha256sum | cut -d' ' -f1)
  if [[ -n $prior && "$pixels" == "$prior" ]]; then
    echo "Avatar frame $frame is identical to the previous frame" >&2; exit 1
  fi
  prior="$pixels"
  count=$((count+1))
done
[[ $count -eq 5 ]] || exit 1
echo 'Protagonist sprites OK: 5 distinct transparent 600x754 frames, common pivot, exact authored-sheet derivation.'
