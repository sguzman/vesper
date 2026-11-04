#!/usr/bin/env bash
# Reproducible 12 ms PCM fade-in on the original generated one-shot.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v ffmpeg >/dev/null || { echo 'ffmpeg required' >&2; exit 1; }
ffmpeg -hide_banner -loglevel error -y \
  -i "$root/sources/audio/paper-rustle-raw.wav" \
  -af 'afade=t=in:st=0:d=0.012' -c:a pcm_s16le \
  "$root/assets/audio/paper-rustle.wav"
