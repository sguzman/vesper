#!/usr/bin/env bash
# Authoring-language regression: require explicit English UI labels.
# The Vesper parser independently checks field types and length limits.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
theme="$root/styles/urban.toml"
expected=(
  '[menu_labels]'
  'main = ["New Story", "Continue", "Quit", "Settings"]'
  'pause = ["Resume", "Main Menu", "Quit", "Settings"]'
  'ending = ["Play Again", "Main Menu", "Quit"]'
  'main_title = "Main Menu"'
  'pause_title = "Paused"'
  'ending_title = "The End"'
  'main_hint = "↑↓: choose · Enter: confirm · M: close · Esc: quit"'
  'pause_hint = "↑↓: choose · Enter: confirm · P: resume · Esc: quit"'
)
for line in "${expected[@]}"; do
  grep -Fqx -- "$line" "$theme" || {
    echo "ERROR: missing exact authored English menu line: $line" >&2
    exit 1
  }
done
[[ $(grep -Fxc '[menu_labels]' "$theme") -eq 1 ]] || {
  echo 'ERROR: duplicate menu label table' >&2; exit 1;
}
echo 'English menu labels OK: main/pause/ending actions, titles and instructions.'
