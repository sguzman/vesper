#!/usr/bin/env bash
# Sanity-check interaction positions against the current 1280px shop painting.
# These art-specific bounds must be re-reviewed when the background is replaced.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
manifest=${1:-"$root/game.toml"}
[[ -s $manifest ]] || { echo "ERROR: missing shop manifest: $manifest" >&2; exit 1; }
awk '
  /^\[exploration\.locations\.store\.(props|npcs)\.(ledger|cat|mateo)\]$/ {
    section=$0
    sub(/^.*\./,"",section)
    sub(/\]$/,"",section)
    next
  }
  /^\[/ { section="" }
  section!="" && /^x[[:space:]]*=/ { x[section]=$3+0 }
  section!="" && /^interaction_radius[[:space:]]*=/ { radius[section]=$3+0 }
  END {
    if (!("ledger" in x) || !("mateo" in x) || !("cat" in x))
      { print "ERROR: missing shop interaction coordinates" > "/dev/stderr"; exit 1 }
    if (x["ledger"] < 490 || x["ledger"] > 620)
      { print "ERROR: ledger is no longer aligned to notebook near x=555" > "/dev/stderr"; exit 1 }
    if (x["cat"] > x["ledger"]-150 || x["mateo"] < x["ledger"]+180)
      { print "ERROR: shop actor locations are inconsistent with painted order" > "/dev/stderr"; exit 1 }
    if (x["ledger"]-x["cat"] <= radius["cat"] ||
        x["mateo"]-x["ledger"] <= radius["mateo"])
      { print "ERROR: ledger spot overlaps a different focus zone" > "/dev/stderr"; exit 1 }
    printf "Shop composition coordinates OK: cat x=%g, ledger x=%g, Mateo x=%g.\n",
      x["cat"],x["ledger"],x["mateo"]
  }
' "$manifest"

# Atomic art transition gate: the interim shop painting already contains
# Mateo and the cat. Their world interaction hotspots deliberately remain
# transparent until a repainted background and both actual sprites are
# delivered together. The VN background alias must change at the same time.
manifest_asset() {
  local section=$1 field=$2
  awk -v section="$section" -v field="$field" '
    $0 == "[" section "]" { active=1; next }
    /^\[/ { active=0 }
    active && $1 == field && $2 == "=" {
      if ($3 !~ /^"[^"]+"$/) exit 2
      gsub(/"/,"",$3)
      print $3
      exit
    }
  ' "$manifest"
}
store_background=$(manifest_asset exploration.locations.store background)
vn_background=$(manifest_asset backgrounds store_interior)
mateo_image=$(manifest_asset exploration.locations.store.npcs.mateo image)
cat_image=$(manifest_asset exploration.locations.store.props.cat image)
for value in "$store_background" "$vn_background" "$mateo_image" "$cat_image"; do
  [[ -n $value && $value == assets/* && $value != *'..'* ]] || {
    echo 'ERROR: unsafe/missing shop art manifest reference' >&2; exit 1;
  }
done
[[ $store_background == "$vn_background" ]] || {
  echo 'ERROR: shop world painting and VN store_interior background diverged' >&2
  exit 1
}
interim_background=assets/backgrounds/store-illustration-study.png
invisible=assets/props/invisible-hitspot.png
if [[ $store_background == "$interim_background" &&
      $mateo_image == "$invisible" && $cat_image == "$invisible" ]]; then
  echo 'Shop layer gate OK: interim flattened painting with transparent interaction hotspots.'
elif [[ $store_background != "$interim_background" &&
        $mateo_image != "$invisible" && $cat_image != "$invisible" &&
        $mateo_image != "$cat_image" ]]; then
  # Production source QA is deliberately required rather than trusting image
  # filenames: reproducible exports, transparent silhouette edges and a
  # clean painted backdrop must all be checked after those assets exist.
  for path in "$store_background" "$mateo_image" "$cat_image"; do
    [[ -s "$root/$path" ]] || {
      echo "ERROR: missing complete shop production asset: $path" >&2; exit 1;
    }
  done
  source_check="$root/qa/check-store-layer-art.sh"
  [[ -f $source_check ]] || {
    echo 'ERROR: missing deterministic shop-layer source/alpha validation' >&2
    exit 1
  }
  bash "$source_check"
  # Even fully transparent source-verified sprites must not authorize a new
  # unrelated storefront. Stable scenery must remain pixel-continuous with
  # the original shop painting; both visible figures are checked separately.
  bash "$root/qa/check-store-background-continuity.sh" \
    "$root/$interim_background" "$root/$store_background"
  echo 'Shop layer gate OK: source-verified independent sprites and continuous clean background.'
else
  echo 'ERROR: partial shop art migration: change painted background, VN alias, Mateo and cat sprites atomically' >&2
  exit 1
fi
