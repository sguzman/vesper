#!/usr/bin/env bash
# Real Bevy screenshot verifies optional raised props (counter-top items)
# and unchanged legacy ground-level props in an external authored fixture.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
fixture="$root/examples/exploration-schema"
player="$root/target/debug/vesper-player"
out=/tmp/vesper-actor-elevation-qa
mkdir -p "$out"
for executable in xdotool import convert identify timeout; do
  command -v "$executable" >/dev/null || {
    echo "ERROR: native elevation QA missing $executable" >&2; exit 1;
  }
done
[[ -x $player && -f "$fixture/game.toml" ]] || {
  echo 'ERROR: no player or external authored schema fixture' >&2; exit 1;
}
scratch=$(mktemp -d)
pid=''
cleanup() {
  [[ -z $pid ]] || { kill "$pid" 2>/dev/null || :; wait "$pid" 2>/dev/null || :; }
  rm -rf -- "$scratch"
}
trap cleanup EXIT
game="$scratch/elevated-world"
cp -a "$fixture" "$game"
convert -size 100x100 xc:'#ff00ff' "$game/assets/backgrounds/raised-prop.png"
convert -size 100x100 xc:'#00ffff' "$game/assets/backgrounds/legacy-prop.png"
cat >> "$game/game.toml" <<'EOF'

# Isolated test sprites only: do not alter the original fixture in git.
[exploration.locations.street.props.raised_test]
x = 950.0
image = "assets/backgrounds/raised-prop.png"
scale = 2.0
elevation = 180.0
interaction_radius = 65.0

[exploration.locations.street.props.legacy_test]
x = 1100.0
image = "assets/backgrounds/legacy-prop.png"
interaction_radius = 65.0
EOF
LIBGL_ALWAYS_SOFTWARE=1 \
VESPER_PREFERENCES_DIR="$scratch/preferences" \
VESPER_SAVE_DIR="$scratch/saves" \
"$player" "$game" >"$out/native.log" 2>&1 &
pid=$!
window=''
for ((try=0;try<150;try++)); do
  window=$(xdotool search --onlyvisible --name 'Optional Walkable Location Schema' \
      2>/dev/null | head -n1 || :)
  [[ -z $window ]] || break
  kill -0 "$pid" 2>/dev/null || {
    echo 'ERROR: exploration player exited before screenshot' >&2
    tail -n 50 "$out/native.log" >&2
    exit 1
  }
  sleep .2
done
[[ -n $window ]] || { echo 'ERROR: no native exploration window' >&2; exit 1; }
timeout 5s xdotool windowfocus --sync "$window"
sleep 2.7
frame="$out/elevated-props-native.png"
import -window "$window" "$frame"
[[ $(identify -format '%wx%h' "$frame") == 1280x720 ]] || {
  echo 'ERROR: unexpected native exploration viewport' >&2; exit 1;
}
color() {
  local x=$1 y=$2 expression=$3
  local observed
  observed=$(convert "$frame" -crop "1x1+$x+$y" +repage \
    -format "%[fx:$expression]" info:)
  [[ $observed == 1 ]]
}
magenta='r>0.8 && g<0.2 && b>0.8'
cyan='r<0.2 && g>0.8 && b>0.8'
# Ground line is screen y=590 in the fixture at 1280x720, authored
# horizon .57. Positive 180 logical pixels puts a raised prop at y=410.
color 950 410 "$magenta" || {
  echo 'ERROR: raised prop did not appear 180 pixels above ground' >&2; exit 1;
}
color 985 410 "$magenta" || {
  echo 'ERROR: authored scale 2.0 did not widen raised prop' >&2; exit 1;
}
if color 950 590 "$magenta"; then
  echo 'ERROR: raised prop also remained at ground height' >&2; exit 1;
fi
color 1100 590 "$cyan" || {
  echo 'ERROR: legacy prop no longer occupies the old ground baseline' >&2; exit 1;
}
if color 1100 410 "$cyan"; then
  echo 'ERROR: legacy prop unexpectedly inherited visual elevation' >&2; exit 1;
fi
if color 435 590 "$magenta"; then
  echo 'ERROR: independently authored NPC was shifted or occluded' >&2; exit 1;
fi
if grep -Eq 'panicked at|Story asset error' "$out/native.log"; then
  echo 'ERROR: native elevation run reported a Bevy/asset error' >&2
  exit 1
fi
echo 'Native actor elevation OK: raised magenta prop, scaled width, unchanged cyan ground prop and NPC.'
