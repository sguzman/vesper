#!/usr/bin/env bash
set -euo pipefail
if (( $# != 2 )); then
  echo 'Usage: packaging/linux.sh <compiled-player> <output.tar.gz>' >&2
  exit 2
fi
root="$(cd -- "$(dirname -- "$0")/.." && pwd)"
binary="$1"
output="$2"
if [[ ! -x "$binary" ]]; then
  echo "Missing compiled executable: $binary" >&2
  exit 1
fi
mkdir -p -- "$(dirname -- "$output")"
output="$(realpath -m -- "$output")"
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
mkdir -p -- "$tmp/vesper"
cp -- "$binary" "$tmp/vesper/vesper-player"
cp -a -- "$root/game" "$tmp/vesper/game"
if [[ -d "$root/examples" ]]; then
  cp -a -- "$root/examples" "$tmp/vesper/examples"
fi
cp -- "$root/LICENSE" "$tmp/vesper/LICENSE"
cp -- "$root/README.md" "$tmp/vesper/README.md"
cat > "$tmp/vesper/vesper" <<'LAUNCHER'
#!/usr/bin/env sh
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$here/vesper-player" "$here/game" "$@"
LAUNCHER
chmod 755 "$tmp/vesper/vesper"
if [[ -d "$tmp/vesper/examples/little-lantern" ]]; then
  cat > "$tmp/vesper/lantern" <<'LANTERN'
#!/usr/bin/env sh
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$here/vesper-player" "$here/examples/little-lantern" "$@"
LANTERN
  chmod 755 "$tmp/vesper/lantern"
fi
if [[ -d "$tmp/vesper/examples/clockwork-moon" ]]; then
  cat > "$tmp/vesper/moon" <<'MOON'
#!/usr/bin/env sh
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$here/vesper-player" "$here/examples/clockwork-moon" "$@"
MOON
  chmod 755 "$tmp/vesper/moon"
fi
tar --owner=0 --group=0 --numeric-owner -czf "$output" -C "$tmp" vesper
echo "Created Linux bundle: $output"
