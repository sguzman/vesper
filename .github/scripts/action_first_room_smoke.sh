#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
mkdir -p /tmp/vesper-action-qa
xvfb-run -a -s '-screen 0 1600x1200x24' bash -euo pipefail -c '
  ./target/debug/vesper-action-demo examples/action-adventure \
    > /tmp/vesper-action-qa/boot.log 2>&1 &
  pid=$!
  trap "kill $pid 2>/dev/null || true" EXIT
  ready=0
  for attempt in $(seq 1 60); do
    if grep -Fq "Vesper action room ready" /tmp/vesper-action-qa/boot.log; then
      ready=1
      break
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      cat /tmp/vesper-action-qa/boot.log
      echo "Action player exited before presenting the first room"
      exit 1
    fi
    sleep 1
  done
  if [ "$ready" -ne 1 ]; then
    cat /tmp/vesper-action-qa/boot.log
    echo "Timed out waiting for first action room"
    exit 1
  fi
  window=$(xdotool search --name "Signal Depot" | tail -n 1)
  if [ -z "$window" ]; then
    echo "No Signal Depot window appeared"
    exit 1
  fi

  xdotool windowsize "$window" 960 540
  xdotool windowmove "$window" 80 80
  sleep 3
  import -window "$window" /tmp/vesper-action-qa/landscape-960x540.png
  identify -format "%wx%h\n" /tmp/vesper-action-qa/landscape-960x540.png \
    | grep -F "960x540"

  xdotool windowsize "$window" 640 900
  sleep 3
  import -window "$window" /tmp/vesper-action-qa/portrait-640x900.png
  identify -format "%wx%h\n" /tmp/vesper-action-qa/portrait-640x900.png \
    | grep -F "640x900"

  # Real keyboard input into the live Bevy player, not a pure parser fixture.
  # Xvfb has no EWMH window manager; set X11 input focus directly.
  xdotool windowfocus --sync "$window"
  xdotool key --clearmodifiers Right
  xdotool key --clearmodifiers j
  xdotool key --clearmodifiers space
  sleep 1
  import -window "$window" /tmp/vesper-action-qa/after-input.png

  if ! kill -0 "$pid" 2>/dev/null; then
    cat /tmp/vesper-action-qa/boot.log
    echo "Action player died during input smoke"
    exit 1
  fi
'
