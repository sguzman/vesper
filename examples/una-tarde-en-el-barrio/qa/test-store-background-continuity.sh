#!/usr/bin/env bash
# Synthetic, temp-only positive/negative tests of painting continuity.
# These probe validator behavior, NOT production image readiness.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/.." && pwd)
original="$root/assets/backgrounds/store-illustration-study.png"
checker="$root/qa/check-store-background-continuity.sh"
command -v magick >/dev/null || { echo 'ERROR: ImageMagick 7 missing' >&2; exit 1; }
[[ -s "$original" && -s "$checker" ]] || {
  echo 'ERROR: missing original shop or continuity gate' >&2; exit 1;
}
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
bash "$checker" "$original" "$original" >/dev/null
# A localized test-color change inside Mateo must not move untouched scenery.
magick "$original" -fill '#e6b089' -draw 'rectangle 690,50 760,120' \
  "$work/actor-only-synthetic.png"
bash "$checker" "$original" "$work/actor-only-synthetic.png" >/dev/null
reject() {
  local case_name=$1 path=$2
  if bash "$checker" "$original" "$path" >"$work/rejected.log" 2>&1; then
    echo "ERROR: accepted unsafe $case_name edit" >&2; exit 1
  fi
  grep -Fq 'ERROR:' "$work/rejected.log" || {
    echo "ERROR: $case_name rejected without actionable diagnostic" >&2
    exit 1
  }
  echo "Shop continuity rejects $case_name"
}
magick "$original" -roll +42+0 "$work/shifted.png"
reject 'shifted whole store' "$work/shifted.png"
magick "$original" -fill '#00ffff' -draw 'rectangle 460,398 520,455' "$work/erased-mug.png"
reject 'changed mug landmark' "$work/erased-mug.png"
magick "$original" -resize 960x540 "$work/wrong-canvas.png"
reject 'wrong 1280x720 canvas' "$work/wrong-canvas.png"
echo 'Store source continuity regression OK: unchanged outside actor area; bad shifts, erased objects and sizes rejected.'