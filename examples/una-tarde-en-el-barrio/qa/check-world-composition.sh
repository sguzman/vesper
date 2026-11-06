#!/usr/bin/env bash
# Sanity-check interaction positions against the current 1280px shop painting.
# These art-specific bounds must be re-reviewed when the background is replaced.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
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
' "$root/game.toml"
