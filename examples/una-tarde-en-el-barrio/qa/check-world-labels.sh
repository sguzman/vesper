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
  'noticeboard = "el tablón de anuncios"'
  'old_door = "la puerta antigua"'
  'cat = "el gato"'
  'ledger = "el cuaderno"'
  'display = "el mostrador"'
  'store = "la tienda"'
  'street = "la calle"'
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
echo 'Original game authored Spanish world labels OK: 10 identities.'
