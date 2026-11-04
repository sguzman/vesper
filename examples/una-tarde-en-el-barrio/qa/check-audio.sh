#!/usr/bin/env bash
# Technical PCM/audio-range checks only; no subjective listening claims.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for cmd in ffprobe ffmpeg awk; do
  command -v "$cmd" >/dev/null || { echo "ERROR: $cmd is required" >&2; exit 1; }
done
for name in golden-hour-ambient shop-hush-ambient shop-bell paper-rustle; do
  file="$root/assets/audio/$name.wav"
  [[ -s $file ]] || { echo "ERROR: missing cue $name" >&2; exit 1; }
  codec=$(ffprobe -v error -select_streams a:0 -show_entries stream=codec_name -of csv=p=0 "$file")
  rate=$(ffprobe -v error -select_streams a:0 -show_entries stream=sample_rate -of csv=p=0 "$file")
  channels=$(ffprobe -v error -select_streams a:0 -show_entries stream=channels -of csv=p=0 "$file")
  duration=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$file")
  [[ $codec == pcm_s16le && $rate =~ ^(22050|44100)$ && $channels == 1 ]] || {
    echo "ERROR: unexpected PCM format in $name" >&2; exit 1;
  }
  awk -v value="$duration" 'BEGIN {exit !(value > 0.1 && value < 60.0)}' || {
    echo "ERROR: invalid length in $name: $duration" >&2; exit 1;
  }
  levels=$(ffmpeg -nostats -hide_banner -i "$file" -af volumedetect -f null - 2>&1)
  mean=$(printf '%s\n' "$levels" | sed -n 's/.*mean_volume: \([-0-9.]*\) dB/\1/p' | tail -n 1)
  peak=$(printf '%s\n' "$levels" | sed -n 's/.*max_volume: \([-0-9.]*\) dB/\1/p' | tail -n 1)
  [[ -n $mean && -n $peak ]] || { echo "ERROR: no audio level for $name" >&2; exit 1; }
  awk -v p="$peak" 'BEGIN {exit !(p <= -1.0 && p >= -90.0)}' || {
    echo "ERROR: clipping risk in $name: $peak dBFS" >&2; exit 1;
  }
  printf '%s: %s Hz mono, %.2f s, mean %s dBFS, max %s dBFS\n' "$name" "$rate" "$duration" "$mean" "$peak"
done
# This one-shot is specifically fade-protected; require the initial 16-bit sample to be zero.
# A two-byte consumer closes stdout early, which sends SIGPIPE to ffmpeg
# under `set -o pipefail` even when audio is valid. Decode a small bounded
# segment to a temporary file and check the codec process's real exit status.
sample=$(mktemp)
trap 'rm -f "$sample"' EXIT
ffmpeg -y -nostdin -v error -i "$root/assets/audio/paper-rustle.wav" \
  -t 0.02 -f s16le -acodec pcm_s16le "$sample"
first=$(od -An -td2 -N2 "$sample" | tr -d '[:space:]')
[[ $first == 0 ]] || { echo "ERROR: paper-rustle abrupt start sample ($first)" >&2; exit 1; }
echo 'Audio technical checks OK (not a listening/perceived-level approval).'
