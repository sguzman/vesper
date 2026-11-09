#!/usr/bin/env bash
# Synthetic fixture only: exercise the future production art gate without
# committing colored rectangles, placeholders or revised gameplay art.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v magick >/dev/null || { echo 'ERROR: ImageMagick 7 required' >&2; exit 1; }
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
mkdir -p "$work/assets/backgrounds" "$work/sources/backgrounds" \
  "$work/sources/characters" "$work/sources/layers" "$work/qa" "$work/tools"
cp -- "$root/assets/backgrounds/store-illustration-study.png" "$work/assets/backgrounds/"
for path in qa/check-store-layer-art.sh qa/check-store-background-continuity.sh tools/rebuild-store-layer-art.sh; do
  cp -- "$root/$path" "$work/$path"
done
cat > "$work/sources/SHOP_ART_PROVENANCE.md" <<'META'
Synthetic QA-only source for Mateo, cat, counter, background. Not production artwork.
META
# Deliberately crude solid-color test edits are never accepted game artwork.
# They only prove the validator detects changed actor regions.
magick "$work/assets/backgrounds/store-illustration-study.png" \
  -fill '#a87858' -draw 'rectangle 660,200 900,410' \
  -fill '#6d4b32' -draw 'rectangle 220,365 370,435' \
  "$work/sources/backgrounds/store-clean.png"
magick -size 512x640 xc:none -fill '#bc896c' \
  -draw 'ellipse 256,305 185,275 0,360' \
  "$work/sources/characters/mateo-isolated.png"
magick -size 512x260 xc:none -fill '#dec89c' \
  -draw 'ellipse 256,138 181,95 0,360' \
  "$work/sources/characters/shop-cat-isolated.png"
magick -size 1280x720 xc:none -fill '#8a5432' \
  -draw 'rectangle 32,470 1247,540' \
  "$work/sources/layers/shop-counter-front.png"
cat > "$work/game.toml" <<'TOML'
[backgrounds]
store_interior = "assets/backgrounds/store-clean.png"
[exploration.locations.store]
background = "assets/backgrounds/store-clean.png"
[exploration.locations.store.npcs.mateo]
image = "assets/npcs/mateo.png"
[exploration.locations.store.props.cat]
image = "assets/props/shop-cat.png"
[exploration.locations.store.layers.counterfront]
image = "assets/layers/shop-counter-front.png"
plane = "foreground"
TOML
bash "$work/tools/rebuild-store-layer-art.sh" >/dev/null
check() { bash "$work/qa/check-store-layer-art.sh"; }
check
reject() {
  local label=$1
  if check >"$work/rejection.log" 2>&1; then
    echo "ERROR: accepted synthetic invalid art: $label" >&2; exit 1
  fi
  grep -Fq 'ERROR:' "$work/rejection.log" || {
    echo "ERROR: missing rejection diagnostic for $label" >&2; exit 1
  }
  echo "Shop source QA rejects: $label"
}
cp -- "$work/assets/npcs/mateo.png" "$work/mateo-good.png"
magick "$work/assets/npcs/mateo.png" -fill '#ff00ff' \
  -draw 'rectangle 120,120 170,160' "$work/assets/npcs/mateo.png"
reject 'stale/mismatched Mateo export'
cp -- "$work/mateo-good.png" "$work/assets/npcs/mateo.png"
magick "$work/sources/characters/shop-cat-isolated.png" \
  -background '#ff00ff' -alpha remove -alpha off \
  "$work/sources/characters/shop-cat-isolated.png"
bash "$work/tools/rebuild-store-layer-art.sh" >/dev/null
reject 'opaque cutout instead of independent cat RGBA'
magick -size 512x260 xc:none -fill '#dec89c' \
  -draw 'ellipse 256,138 181,95 0,360' \
  "$work/sources/characters/shop-cat-isolated.png"
bash "$work/tools/rebuild-store-layer-art.sh" >/dev/null
cp -- "$work/sources/backgrounds/store-clean.png" "$work/candidate-good.png"
cp -- "$work/assets/backgrounds/store-illustration-study.png" \
  "$work/sources/backgrounds/store-clean.png"
bash "$work/tools/rebuild-store-layer-art.sh" >/dev/null
reject 'unrepainted original used as clean backdrop'
cp -- "$work/candidate-good.png" "$work/sources/backgrounds/store-clean.png"
bash "$work/tools/rebuild-store-layer-art.sh" >/dev/null
sed -i 's/plane = "foreground"/plane = "background"/' "$work/game.toml"
reject 'counter layer beneath NPCs'
sed -i 's/plane = "background"/plane = "foreground"/' "$work/game.toml"
cp -- "$work/sources/layers/shop-counter-front.png" "$work/counter-good.png"
magick "$work/sources/layers/shop-counter-front.png" -fill '#663322' \
  -draw 'rectangle 100,10 1180,240' \
  "$work/sources/layers/shop-counter-front.png"
bash "$work/tools/rebuild-store-layer-art.sh" >/dev/null
reject 'foreground mask covers upper shop and faces'
cp -- "$work/counter-good.png" "$work/sources/layers/shop-counter-front.png"
bash "$work/tools/rebuild-store-layer-art.sh" >/dev/null
mv "$work/sources/characters/mateo-isolated.png" "$work/missing-mateo.png"
reject 'missing editable source art'
echo 'Synthetic shop QA complete: 1 positive and 6 negative checks.'
