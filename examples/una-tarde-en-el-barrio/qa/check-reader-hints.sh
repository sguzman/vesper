#!/usr/bin/env bash
# Current authored game expects five Spanish states supported by PR #31.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
awk '
/^\[reader_hints\]$/ {found=1; section=1; next}
/^\[/ {section=0}
section && /^[a-z_]+[[:space:]]*=/ {
  name=$1
  if (name != "narration" && name != "choices" && name != "ending" && name != "conversation" && name != "conversation_choices") {print "ERROR unexpected reader hint "name > "/dev/stderr"; failures++}
  if (seen[name]++) {print "ERROR duplicate "name > "/dev/stderr"; failures++}
  value=substr($0,index($0,"=")+1)
  gsub(/^[[:space:]\"]+|[[:space:]\"]+$/, "", value)
  if (length(value) == 0 || length(value) > 260) {print "ERROR invalid length "name > "/dev/stderr"; failures++}
}
END {
  split("narration choices ending conversation conversation_choices", keys, " ")
  for(i in keys) if(!seen[keys[i]]) {print "ERROR missing "keys[i] > "/dev/stderr"; failures++}
  if(!found) {print "ERROR missing [reader_hints]" > "/dev/stderr"; failures++}
  if(!failures) print "Spanish reader hint fields OK: five distinct author-owned labels."
  exit failures ? 1 : 0
}' "$root/styles/urban.toml"
