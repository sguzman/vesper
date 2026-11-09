#!/usr/bin/env bash
# Invoked only when the entire shop art transition is proposed.
# Proves deterministic source derivation and checks for blatant transparency,
# provenance, placement and scenery errors. Does NOT certify artistic quality.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for tool in magick awk cmp grep sed; do
  command -v "$tool" >/dev/null || { echo "ERROR: missing shop QA tool $tool" >&2; exit 1; }
done
fail() { echo "ERROR: $*" >&2; exit 1; }
require() { [[ -s "$root/$1" ]] || fail "missing required shop art $1"; }
format() { magick identify -format '%m %wx%h %[opaque]' "$root/$1" 2>/dev/null | tr '[:upper:]' '[:lower:]' || fail "undecodable art: $1"; }
mean_alpha() { magick "$root/$1" -alpha extract -format '%[fx:mean]' info:; }
range() {
  awk -v value="$1" -v low="$2" -v high="$3" 'BEGIN { exit !(value > low && value < high) }' ||
    fail "$4 out of bounds: $1 (expected >$2 and <$3)"
}

# Paths deliberately fixed so production can be reproduced without a hidden
# GUI project or crop guessed from the flattened painting.
sources=(
  sources/backgrounds/store-clean.png
  sources/characters/mateo-isolated.png
  sources/characters/shop-cat-isolated.png
  sources/layers/shop-counter-front.png
)
outputs=(
  assets/backgrounds/store-clean.png
  assets/npcs/mateo.png
  assets/props/shop-cat.png
  assets/layers/shop-counter-front.png
)
for p in "${sources[@]}" "${outputs[@]}"; do require "$p"; done
require sources/SHOP_ART_PROVENANCE.md
require tools/rebuild-store-layer-art.sh
require qa/check-store-background-continuity.sh
require assets/backgrounds/store-illustration-study.png
require game.toml
for subject in Mateo cat counter background; do
  grep -iq "$subject" "$root/sources/SHOP_ART_PROVENANCE.md" ||
    fail "provenance lacks art subject: $subject"
done

# Assert that the world and VN backgrounds are equal and the mask is in the
# explicit foreground plane; other art-transition checks validate hitspots.
manifest_value() {
  local section=$1 field=$2
  awk -v section="$section" -v field="$field" '
    $0=="[" section "]" {active=1; next}
    /^\[/ {active=0}
    active && $1==field && $2=="=" {gsub(/"/,"",$3); print $3; exit}
  ' "$root/game.toml"
}
[[ $(manifest_value exploration.locations.store background) == "${outputs[0]}" ]] || fail 'wrong clean world background reference'
[[ $(manifest_value backgrounds store_interior) == "${outputs[0]}" ]] || fail 'wrong VN shop background alias'
[[ $(manifest_value exploration.locations.store.npcs.mateo image) == "${outputs[1]}" ]] || fail 'wrong Mateo source-derived sprite reference'
[[ $(manifest_value exploration.locations.store.props.cat image) == "${outputs[2]}" ]] || fail 'wrong cat source-derived sprite reference'
[[ $(manifest_value exploration.locations.store.layers.counterfront image) == "${outputs[3]}" ]] || fail 'missing counterfront mask reference'
[[ $(manifest_value exploration.locations.store.layers.counterfront plane) == foreground ]] || fail 'counterfront must occupy foreground layer plane'

[[ $(format "${sources[0]}") == 'png 1280x720 true' ]] || fail 'clean source must be opaque RGB PNG 1280x720'
[[ $(format "${outputs[0]}") == 'png 1280x720 true' ]] || fail 'clean backdrop must be opaque RGB PNG 1280x720'
for i in 1 2 3; do
  [[ $(format "${outputs[i]}") == png\ *\ false ]] || fail "missing full alpha silhouette in ${outputs[i]}"
  [[ $(format "${sources[i]}") == png\ *\ false ]] || fail "missing original alpha in ${sources[i]}"
done
[[ $(format "${outputs[3]}") == 'png 1280x720 false' ]] || fail 'counterfront must align to 1280x720 original scene'

# Detect empty/solid screenshot substitutes and obvious silhouette edge
# contamination. The eye must still judge painted contact, style and faces.
for i in 1 2 3; do
  f=${outputs[i]}
  alpha=$(mean_alpha "$f")
  if (( i == 3 )); then
    range "$alpha" 0.02 0.40 'counterfront alpha coverage'
    upper=$(magick "$root/$f" -crop 1280x300+0+0 +repage -alpha extract -format '%[fx:mean]' info:)
    range "$upper" -0.000001 0.005 'counterfront covering upper scene'
  else
    range "$alpha" 0.06 0.80 "${f} alpha coverage"
    width=$(magick identify -format '%w' "$root/$f")
    height=$(magick identify -format '%h' "$root/$f")
    (( width >= 220 && height >= 170 )) || fail "insufficient independent sprite resolution for $f"
    edge=$(magick "$root/$f" -alpha extract -crop "${width}x4+0+0" +repage -format '%[fx:mean]' info:)
    range "$edge" -0.000001 0.025 "${f} top silhouette margin"
    edge=$(magick "$root/$f" -alpha extract -crop "4x${height}+0+0" +repage -format '%[fx:mean]' info:)
    range "$edge" -0.000001 0.025 "${f} side silhouette margin"
  fi
done

# Rebuild in a disposable shadow package and compare DECODED pixels rather
# than PNG byte compression, rejecting stale or substituted game exports.
shadow=$(mktemp -d)
trap 'rm -rf -- "$shadow"' EXIT
for p in "${sources[@]}"; do
  mkdir -p "$shadow/$(dirname "$p")"
  cp -- "$root/$p" "$shadow/$p"
done
mkdir -p "$shadow/tools"
cp -- "$root/tools/rebuild-store-layer-art.sh" "$shadow/tools/"
bash "$shadow/tools/rebuild-store-layer-art.sh" >/dev/null
for i in 0 1 2 3; do
  real="$root/${outputs[i]}"
  expected="$shadow/${outputs[i]}"
  [[ $(magick identify -format '%wx%h' "$real") == $(magick identify -format '%wx%h' "$expected") ]] || fail "wrong dimensions in ${outputs[i]}"
  if (( i == 0 )); then channel=RGB; else channel=RGBA; fi
  cmp -s <(magick "$real" -depth 8 "$channel:-") \
    <(magick "$expected" -depth 8 "$channel:-") || fail "runtime pixels do not match preserved source: ${outputs[i]}"
done
bash "$root/qa/check-store-background-continuity.sh" \
  "$root/assets/backgrounds/store-illustration-study.png" "$root/${outputs[0]}" >/dev/null

# An unchanged original painting cannot be labeled the clean background.
# Verify substantial edits at BOTH former actor locations. This is NOT proof
# that repainted shelves are plausible or that both characters are absent.
for spec in 'mateo:240x210+660+200' 'cat:150x70+220+365'; do
  label=${spec%%:*}
  crop=${spec#*:}
  diff=$(magick compare -metric MAE \
    "$root/assets/backgrounds/store-illustration-study.png[$crop]" \
    "$root/${outputs[0]}[$crop]" null: 2>&1) || :
  value=$(printf '%s' "$diff" | sed -nE 's/.*\(([0-9.eE+-]+)\).*/\1/p')
  [[ -n $value ]] || fail "cannot compare removed $label painting"
  range "$value" 0.02 1.001 "$label repaint deviation"
done
echo 'Shop layer SOURCE QA OK: deterministic clean backdrop, Mateo/cat RGBA and foreground mask, without asserting artistic approval.'
