#!/usr/bin/env bash
# Fail-closed regression for shop art migrations. Never edits the live game.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
gate="$root/qa/check-world-composition.sh"
manifest="$root/game.toml"
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
bash "$gate" "$manifest" > "$work/baseline.log" || {
  echo 'ERROR: current shop manifest failed baseline composition QA' >&2
  cat "$work/baseline.log" >&2
  exit 1
}
mutate() {
  local section=$1 field=$2 replacement=$3
  awk -v section="$section" -v field="$field" -v replacement="$replacement" '
    $0 == "[" section "]" { active=1; print; next }
    /^\[/ { active=0 }
    active && $1 == field && $2 == "=" {
      print field " = \"" replacement "\""
      found=1
      next
    }
    { print }
    END { if (!found) exit 2 }
  ' "$manifest" > "$work/mutated.toml" || {
    echo "ERROR: could not mutate fixture $section.$field" >&2
    exit 1
  }
}
reject() {
  local label=$1
  if bash "$gate" "$work/mutated.toml" >"$work/output.log" 2>&1; then
    echo "ERROR: unsafe shop art mutation unexpectedly accepted: $label" >&2
    exit 1
  fi
  grep -Fq 'ERROR:' "$work/output.log" || {
    echo "ERROR: rejection for $label lacked actionable diagnostic" >&2
    cat "$work/output.log" >&2
    exit 1
  }
  echo "Rejects unsafe shop migration: $label"
}
mutate exploration.locations.store.npcs.mateo image assets/npcs/elena.png
reject 'Mateo sprite before background/cat'
mutate exploration.locations.store.props.cat image assets/npcs/rosa.png
reject 'cat sprite before background/Mateo'
mutate exploration.locations.store background assets/backgrounds/street.png
reject 'background changed without sprites or VN alias'
mutate backgrounds store_interior assets/backgrounds/street.png
reject 'VN background changed without world background'
echo 'Store art transition regression OK: interim passes and four partial migrations fail.'
