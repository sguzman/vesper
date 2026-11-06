#!/usr/bin/env bash
# Deterministic branch/variable consequence smoke tests without a graphics driver.
# This is NOT the engine interpreter, save/restart test, or native gameplay QA.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v awk >/dev/null || { echo 'awk is required' >&2; exit 1; }
awk -f "$root/qa/check-scenarios.awk" "$root/scripts/story.vn" "$root/qa/scenarios.tsv"

# Prove that the assertions actually reject meaningful narrative regressions.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
awk '{if ($0=="set saw_ledger = true") $0="set saw_ledger = false"; print}' \
  "$root/scripts/story.vn" > "$tmp/ledger-broken.vn"
if awk -f "$root/qa/check-scenarios.awk" \
  "$tmp/ledger-broken.vn" "$root/qa/scenarios.tsv" > "$tmp/ledger-negative.log" 2>&1; then
  echo 'ERROR: failed to reject a lost ledger consequence' >&2
  exit 1
fi
grep -Fq 'private-accounts-seen' "$tmp/ledger-negative.log" || {
  echo 'ERROR: ledger regression was not diagnosed by its scenario' >&2; exit 1;
}

awk '{if ($0=="if met_elena == true") $0="if met_elena == false"; print}' \
  "$root/scripts/story.vn" > "$tmp/elena-broken.vn"
if awk -f "$root/qa/check-scenarios.awk" \
  "$tmp/elena-broken.vn" "$root/qa/scenarios.tsv" > "$tmp/elena-negative.log" 2>&1; then
  echo 'ERROR: failed to reject an inverted Elena conditional' >&2
  exit 1
fi
grep -Fq 'quiet-informed' "$tmp/elena-negative.log" || {
  echo 'ERROR: the inverted character condition was not diagnosed' >&2; exit 1;
}

# Reading private accounts after an earlier respectful visit must revoke
# any prior "did not snoop" narrative state. This mutation turns the actual
# scene's reset into the opposite value; the reverse-order case must catch it.
awk '
  /^@scene / { in_read=($2=="shop_ledger_read") }
  in_read && $0=="set respected_privacy = false" {
    $0="set respected_privacy = true"
    mutated++
  }
  { print }
  END { if (mutated != 1) exit 22 }
' "$root/scripts/story.vn" > "$tmp/privacy-revocation-broken.vn" || {
  echo 'ERROR: expected exactly one ledger-read privacy reset to mutate' >&2
  exit 1
}
if awk -f "$root/qa/check-scenarios.awk" \
  "$tmp/privacy-revocation-broken.vn" "$root/qa/scenarios.tsv" > "$tmp/privacy-revocation-negative.log" 2>&1; then
  echo 'ERROR: failed to reject reversal of the privacy-breach consequence' >&2
  exit 1
fi
grep -Fq 'close-then-read-revokes-respect' "$tmp/privacy-revocation-negative.log" || {
  echo 'ERROR: reversed privacy consequence did not fail its scenario' >&2; exit 1
}

echo 'Scenario mutation checks passed: ledger consequence, Elena condition and irreversible privacy breach are detected.'
