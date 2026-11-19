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

# A separate fresh game instance proves the encounter can actually be completed
# using keyboard input, not only via isolated state-machine unit tests.
xvfb-run -a -s '-screen 0 1280x720x24' bash -euo pipefail -c '
  log=/tmp/vesper-action-qa/room-clear.log
  ./target/debug/vesper-action-demo examples/action-adventure > "$log" 2>&1 &
  pid=$!
  trap "kill $pid 2>/dev/null || true" EXIT

  ready=0
  for attempt in $(seq 1 60); do
    if grep -Fq "Vesper action room ready" "$log"; then
      ready=1
      break
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      cat "$log"
      echo "Action game exited before combat smoke"
      exit 1
    fi
    sleep 1
  done
  test "$ready" -eq 1 || { cat "$log"; exit 1; }
  window=$(xdotool search --name "Signal Depot" | tail -n 1)
  test -n "$window"
  xdotool windowfocus --sync "$window"

  # The drone starts beyond its aggro radius. Deliberately approach it.
  xdotool keydown --clearmodifiers Right
  sleep 0.45
  xdotool keyup Right
  sleep 1.35

  # Attacks are real KeyJ events; retry within the health budget to allow
  # modest software-renderer frame-rate variation.
  for attempt in $(seq 1 12); do
    xdotool key --clearmodifiers j
    sleep 0.46
    if grep -Fq "Vesper action: drone disabled" "$log"; then
      break
    fi
  done
  if ! grep -Fq "Vesper action: drone disabled" "$log"; then
    cat "$log"
    echo "Could not disable drone through real player input"
    exit 1
  fi

  # Follow the clear bottom corridor. Repeated interaction attempts let the
  # player activate the terminal as soon as it is actually in reach.
  xdotool keydown --clearmodifiers Right
  for attempt in $(seq 1 70); do
    xdotool key --clearmodifiers e
    sleep 0.20
    if grep -Fq "Vesper action: power restored" "$log"; then
      break
    fi
  done
  xdotool keyup Right
  if ! grep -Fq "Vesper action: power restored" "$log"; then
    cat "$log"
    echo "Power terminal not reachable through real walking"
    exit 1
  fi

  # Go around the machinery and approach the upper-right exit.
  xdotool keydown --clearmodifiers Right
  sleep 1.10
  xdotool keyup Right
  xdotool keydown --clearmodifiers Up
  for attempt in $(seq 1 40); do
    xdotool key --clearmodifiers e
    sleep 0.20
    if grep -Fq "Vesper action: room cleared" "$log"; then
      break
    fi
  done
  xdotool keyup Up
  if ! grep -Fq "Vesper action: room cleared" "$log"; then
    cat "$log"
    echo "Exit was not reachable through real walking"
    exit 1
  fi
  import -window "$window" /tmp/vesper-action-qa/room-cleared.png
  xdotool key --clearmodifiers r
  sleep 0.3
  if ! grep -Fq "Vesper action: room restarted" "$log"; then
    cat "$log"
    echo "R did not restart the completed room"
    exit 1
  fi
  import -window "$window" /tmp/vesper-action-qa/after-restart.png
'
