#!/usr/bin/env bash
# Positive/negative tests for shop scenery continuity. All mutations are
# disposable synthetic QA probes; no source/game artwork is overwritten.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
original="$root/assets/backgrounds/store-illustration-study.png"
checker="$root/qa/check-store-background-continuity.sh"
command -v magick >/dev/null || { echo 'ERROR: missing ImageMagick 7 magick' >&2; exit 1; }
[[ -s $original && -s $checker ]] || { echo 'ERROR: missing original shop or checker' >&2; exit 1; }
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT

# Same painting and edits in a non-protected actor region must be allowed.
# Passing this test NEVER certifies that the actor was actually removed.
bash "$checker" "$original" "$original" >"$work/baseline.log"
magick "$original" -fill '#e6b089' -draw 'rectangle 690,50 760,120' \
  "$work/actor-region-probe.png"
bash "$checker" "$original" "$work/actor-region-probe.png" >"$work/actor-probe.log"

reject() {
  local label=$1 path=$2
  if bash "$checker" "$original" "$path" >"$work/rejected.log" 2>&1; then
    echo "ERROR: accepted unsafe $label shop change" >&2; exit 1;
  fi
  grep -Fq 'ERROR:' "$work/rejected.log" || {
    echo "ERROR: rejected $label without diagnostic" >&2; exit 1;
  }
  echo "Shop continuity rejects: $label"
}
magick "$original" -roll +42+0 "$work/shifted.png"
reject 'shifted whole scene' "$work/shifted.png"
magick "$original" -fill '#00ffff' -draw 'rectangle 460,398 520,455' \
  "$work/erased-mug.png"
reject 'erased mug next to ledger' "$work/erased-mug.png"
magick "$original" -resize 960x540 "$work/wrong-canvas.png"
reject 'wrong 960x540 canvas' "$work/wrong-canvas.png"
echo 'Shop continuity QA passed: unchanged source and actor-region probe permitted; scene shifts, mug erasure and resize rejected.'
