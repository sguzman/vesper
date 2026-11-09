#!/usr/bin/env bash
# Native Bevy proof that authored transparent PNG layers really occlude the
# intended actors. Runs disposable external-game copies under Xvfb/Mesa.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
player="$root/target/debug/vesper-player"
fixture="$root/examples/exploration-schema"
out=/tmp/vesper-layer-plane-qa
mkdir -p "$out"
for tool in convert import xdotool identify timeout; do
  command -v "$tool" >/dev/null || { echo "ERROR: missing $tool" >&2; exit 1; }
done
[[ -x $player && -s "$fixture/game.toml" ]] || {
  echo 'ERROR: missing player or authored exploration fixture' >&2
  exit 1
}
scratch=$(mktemp -d)
pid=''
cleanup() {
  if [[ -n $pid ]]; then
    kill "$pid" 2>/dev/null || :
    wait "$pid" 2>/dev/null || :
  fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT

is_magenta() {
  local file=$1 x=$2 y=$3 observed
  observed=$(convert "$file" -crop "1x1+$x+$y" +repage \
    -format '%[fx:(r>0.80 && g<0.20 && b>0.80)]' info:)
  [[ $observed == 1 ]]
}
for plane in background foreground overlay; do
  game="$scratch/$plane"
  cp -a "$fixture" "$game"
  # Two solid, original test-only occlusion squares. Center pixels overlap
  # the real placeholder avatar at x=150 and NPC at x=435 on a 1280x720
  # viewport with the fixture's authored horizon=.57.
  convert -size 1280x720 xc:none -fill '#ff00ff' \
    -draw 'rectangle 130,560 170,620 rectangle 415,560 455,620' \
    "$game/assets/backgrounds/layer-probe.png"
  cat >> "$game/game.toml" <<EOF

[exploration.locations.street.layers.occlusion_probe]
image = "assets/backgrounds/layer-probe.png"
plane = "$plane"
depth = 0.0
parallax = 1.0
EOF
  VESPER_PREFERENCES_DIR="$scratch/preferences-$plane" \
    VESPER_SAVE_DIR="$scratch/saves-$plane" \
    LIBGL_ALWAYS_SOFTWARE=1 \
    "$player" "$game" >"$out/$plane.log" 2>&1 &
  pid=$!
  window=''
  for ((tick=0;tick<100;tick++)); do
    window=$(xdotool search --onlyvisible --name 'Optional Walkable Location Schema' \
      2>/dev/null | head -n1 || :)
    [[ -n $window ]] && break
    kill -0 "$pid" 2>/dev/null || {
      echo "ERROR: player exited during $plane capture" >&2
      tail -n 40 "$out/$plane.log" >&2
      exit 1
    }
    sleep .2
  done
  [[ -n $window ]] || { echo "ERROR: no player window for $plane" >&2; exit 1; }
  timeout 5s xdotool windowfocus --sync "$window"
  sleep 2.4
  frame="$out/$plane.png"
  import -window "$window" "$frame"
  [[ $(identify -format '%wx%h' "$frame") == 1280x720 ]] || {
    echo "ERROR: unexpected native screenshot size for $plane" >&2
    exit 1
  }
  ! grep -Eq 'panicked at|Story asset error' "$out/$plane.log" || {
    echo "ERROR: player logged asset/panic error for $plane" >&2
    exit 1
  }
  avatar=false npc=false
  if is_magenta "$frame" 150 590; then avatar=true; fi
  if is_magenta "$frame" 435 590; then npc=true; fi
  case "$plane" in
    background)
      [[ $avatar == false && $npc == false ]] || {
        echo 'ERROR: legacy background layer incorrectly covers an actor' >&2
        exit 1
      } ;;
    foreground)
      [[ $avatar == false && $npc == true ]] || {
        echo 'ERROR: foreground layer must cover NPC but not avatar' >&2
        exit 1
      } ;;
    overlay)
      [[ $avatar == true && $npc == true ]] || {
        echo 'ERROR: overlay layer must cover both avatar and NPC' >&2
        exit 1
      } ;;
  esac
  echo "Native $plane Z-plane OK: avatar masked=$avatar, NPC masked=$npc"
  kill "$pid" 2>/dev/null || :
  wait "$pid" 2>/dev/null || :
  pid=''
done
echo 'Native foreground layer compositing OK: original game fixture, three planes.'
