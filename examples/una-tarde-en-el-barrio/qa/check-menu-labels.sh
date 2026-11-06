#!/usr/bin/env bash
# Authoring-language regression: require explicit Spanish UI labels.
# The Vesper parser independently checks field types and length limits.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
theme="$root/styles/urban.toml"
expected=(
  '[menu_labels]'
  'main = ["Nueva historia", "Continuar partida", "Salir", "Ajustes"]'
  'pause = ["Reanudar", "Menú principal", "Salir", "Ajustes"]'
  'ending = ["Volver a jugar", "Menú principal", "Salir"]'
  'main_title = "Menú principal"'
  'pause_title = "Pausa"'
  'ending_title = "Fin"'
  'main_hint = "↑↓: elegir · Enter: confirmar · M: cerrar · Esc: salir"'
  'pause_hint = "↑↓: elegir · Enter: confirmar · P: reanudar · Esc: salir"'
)
for line in "${expected[@]}"; do
  grep -Fqx -- "$line" "$theme" || {
    echo "ERROR: missing exact authored Spanish menu line: $line" >&2
    exit 1
  }
done
[[ $(grep -Fxc '[menu_labels]' "$theme") -eq 1 ]] || {
  echo 'ERROR: duplicate menu label table' >&2; exit 1;
}
echo 'Spanish menu labels OK: main/pause/ending actions, titles and instructions.'
