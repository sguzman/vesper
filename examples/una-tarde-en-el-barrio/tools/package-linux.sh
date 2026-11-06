#!/usr/bin/env bash
# Package the real, already-built Vesper player plus the external original game.
# Development preview only: not a release or owner-accepted visual QA artifact.
set -euo pipefail

game_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
repo_root=$(cd -- "$game_root/../.." && pwd)
player=${VESPER_PLAYER_BIN:-$repo_root/target/debug/vesper-player}
output=${1:-$repo_root/dist}
name=Una-Tarde-en-el-Barrio-Linux-x86_64

[[ -f "$player" && -x "$player" ]] || {
  printf 'Missing executable Vesper player: %s\n' "$player" >&2
  exit 1
}
for path in game.toml assets scripts styles; do
  [[ -e "$game_root/$path" ]] || {
    printf 'Missing game package input: %s\n' "$game_root/$path" >&2
    exit 1
  }
done

mkdir -p -- "$output"
output=$(cd -- "$output" && pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/vesper-urban-package.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
bundle="$work/$name"
mkdir -p -- "$bundle/game"
install -m 0755 -- "$player" "$bundle/vesper-player"
cp -- "$game_root/game.toml" "$bundle/game/game.toml"
cp -R -- "$game_root/assets" "$game_root/scripts" "$game_root/styles" "$bundle/game/"
cp -- "$game_root/README.md" "$bundle/PROJECT-README.md"
cp -- "$game_root/ASSET_PROVENANCE.md" "$bundle/ASSET-PROVENANCE.md"

cat > "$bundle/play.sh" <<'LAUNCHER'
#!/usr/bin/env bash
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
exec "$here/vesper-player" "$here/game" "$@"
LAUNCHER
chmod 0755 "$bundle/play.sh"

cat > "$bundle/START-HERE.txt" <<'README'
Una Tarde en el Barrio — original Linux development preview

This is an actual playable original game with its native Vesper executable,
story, sound and graphics. It is not yet a final release or an owner-approved
visual build. No compiler or source checkout is needed to start this archive.
Linux x86_64 and ordinary system graphics/audio libraries are required.

Unpack the .tar.gz, then run ./play.sh from the extracted directory.
The launcher resolves the packaged game path regardless of working directory.
Controls: A/D or arrow keys to walk, E to interact, T for direct conversation,
Enter/number keys for story choices, F5/F9 to quick-save/load, P to pause.
See PROJECT-README.md for the authored game and ASSET-PROVENANCE.md for art.
README

(
  cd -- "$bundle"
  find game -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum
  sha256sum vesper-player play.sh
) > "$bundle/SHA256SUMS"

archive="$output/$name.tar.gz"
# Stable ownership, ordering, timestamps and gzip headers for reproducibility.
tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
    -C "$work" -cf - "$name" | gzip -n > "$archive"
[[ -s "$archive" ]] || { printf 'Empty archive: %s\n' "$archive" >&2; exit 1; }
printf 'Packaged playable Linux development preview: %s\n' "$archive"
