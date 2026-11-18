#!/usr/bin/env bash
# Ensure player-facing names are deliberately authored, not raw engine IDs.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
manifest="$root/game.toml"
expected=(
  '[exploration.ui.target_names]'
  'elena = "Elena"'
  'rosa = "Rosa"'
  'mateo = "Mateo"'
  'noticeboard = "the noticeboard"'
  'old_door = "the old door"'
  'cat = "the cat"'
  'ledger = "the account book"'
  'display = "the display"'
  'store = "the shop"'
  'street = "the street"'
)
for line in "${expected[@]}"; do
  grep -Fqx -- "$line" "$manifest" || {
    echo "ERROR: missing authored world target label: $line" >&2
    exit 1
  }
done
[[ $(grep -Fxc '[exploration.ui.target_names]' "$manifest") -eq 1 ]] || {
  echo 'ERROR: duplicate world target labels section' >&2
  exit 1
}
echo 'Original game authored English world labels OK: 10 identities.'
