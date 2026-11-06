#!/usr/bin/env bash
# Exercise the real, unchanged Bevy player with the original Una Tarde game.
# This is native CI evidence (Xvfb/software Mesa), not physical Hyprland QA.
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
player="$root/target/debug/vesper-player"
game="$root/examples/una-tarde-en-el-barrio"
out=${VESPER_URBAN_QA_OUTPUT:-/tmp/vesper-urban-native-qa}
mkdir -p "$out"

for tool in xdotool import identify convert compare awk sed grep timeout; do
  command -v "$tool" >/dev/null || { echo "ERROR: missing QA tool $tool" >&2; exit 1; }
done
[[ -x $player && -s $game/game.toml ]] || { echo 'ERROR: missing compiled player or original game' >&2; exit 1; }
work=$(mktemp -d)
log="$out/native-gameplay.log"
pid=''
cleanup() {
  if [[ -n $pid ]]; then
    kill "$pid" 2>/dev/null || :
    wait "$pid" 2>/dev/null || :
  fi
  rm -rf "$work"
}
trap cleanup EXIT

LIBGL_ALWAYS_SOFTWARE=1 VESPER_TRACE_SCENES=1 \
  VESPER_PREFERENCES_DIR="$work/preferences" \
  VESPER_SAVE_DIR="$work/saves" \
  "$player" "$game" > "$log" 2>&1 &
pid=$!

fail() { echo "ERROR: $*" >&2; tail -n 75 "$log" >&2; exit 1; }
alive() { kill -0 "$pid" 2>/dev/null || fail "Bevy process exited unexpectedly"; }
contains() { grep -Fq -- "$1" "$log"; }
count() { grep -Fc -- "$1" "$log" || :; }
wait_log() {
  local phrase=$1 limit=${2:-80} tick
  for ((tick=0;tick<limit;tick++)); do
    contains "$phrase" && return 0
    alive
    sleep .25
  done
  fail "timed out waiting for log: $phrase"
}
wait_increment() {
  local phrase=$1 prior=$2 limit=${3:-80} tick current
  for ((tick=0;tick<limit;tick++)); do
    current=$(count "$phrase")
    ((current > prior)) && return 0
    alive
    sleep .25
  done
  fail "no new event in log: $phrase (prior=$prior)"
}
window=''
for ((try=0;try<180;try++)); do
  window=$(xdotool search --onlyvisible --name 'Una Tarde en el Barrio' 2>/dev/null | head -n 1 || :)
  [[ -n $window ]] && break
  alive
  sleep .25
done
[[ -n $window ]] || fail 'game window never opened'
timeout 5s xdotool windowfocus --sync "$window" || fail 'could not focus game window'

key() {
  timeout 5s xdotool windowfocus --sync "$window" || fail 'lost window focus'
  xdotool key --clearmodifiers "$1"
  sleep .19
}
walk() {
  local loc=$1 direction=$2 seconds=$3 prior
  prior=$(count "Vesper exploration stopped: $loc at x=")
  timeout 5s xdotool windowfocus --sync "$window" || fail 'could not focus world'
  xdotool keydown --clearmodifiers "$direction"
  sleep "$seconds"
  xdotool keyup --clearmodifiers "$direction"
  wait_increment "Vesper exploration stopped: $loc at x=" "$prior" 40
  current_x=$(grep -F "Vesper exploration stopped: $loc at x=" "$log" \
    | tail -n 1 | sed -E 's/.*at x=([0-9]+(\.[0-9]+)?).*/\1/')
  [[ $current_x =~ ^[0-9]+([.][0-9]+)?$ ]] || fail "no numeric position for $loc"
}
current_x=145
walk_to() {
  local loc=$1 target=$2 tolerance=$3 iter dist secs direction
  for ((iter=0;iter<17;iter++)); do
    if awk -v x="$current_x" -v t="$target" -v tol="$tolerance" \
      'BEGIN {d=x-t; if (d<0)d=-d; exit !(d<=tol)}'; then
      echo "Reached $loc x=$current_x (target $target)"
      return 0
    fi
    dist=$(awk -v x="$current_x" -v t="$target" 'BEGIN {d=x-t; if(d<0)d=-d; print d}')
    secs=$(awk -v d="$dist" 'BEGIN {s=d/235.0; if(s<0.15)s=0.15; if(s>1.05)s=1.05; printf "%.2f",s}')
    direction=$(awk -v x="$current_x" -v t="$target" 'BEGIN {print (x<t)?"Right":"Left"}')
    walk "$loc" "$direction" "$secs"
  done
  fail "could not reach $loc target=$target, last x=$current_x"
}
frame() {
  local filename=$1 destination="$out/$1" colors tick
  for ((tick=0;tick<10;tick++)); do
    if import -window "$window" "$destination.tmp.png" >/dev/null 2>&1 \
       && identify "$destination.tmp.png" >/dev/null 2>&1; then
      colors=$(convert "$destination.tmp.png" -crop 620x330+130+115 +repage -format '%k' info: 2>/dev/null || echo 0)
      if [[ $colors =~ ^[0-9]+$ ]] && (( colors > 100 )); then
        mv "$destination.tmp.png" "$destination"
        echo "Captured $filename ($colors colors)"
        return 0
      fi
    fi
    sleep .35
  done
  fail "no rendered detailed frame available: $filename"
}
# A location-change log is emitted before Bevy's next render/present.
# Require the new painted backdrop to actually appear instead of capturing
# the previous area's framebuffer with newly updated HUD labels. Compare an
# unoccluded artwork region, not the player's position or changing UI.
wait_new_area_frame() {
  local previous=$1 next=$2 diff tick
  local old_crop="$work/prior-area.png" new_crop="$work/next-area.png"
  convert "$out/$previous" -crop 550x305+150+75 +repage "$old_crop" \
    || fail "cannot crop previous area"
  for ((tick=0; tick<30; tick++)); do
    frame "$next"
    convert "$out/$next" -crop 550x305+150+75 +repage "$new_crop" \
      || fail "cannot crop current area"
    diff=$(compare -metric AE "$old_crop" "$new_crop" null: 2>&1 || :)
    if [[ $diff =~ ^[0-9]+$ ]] && (( diff > 100000 )); then
      echo "New area artwork presented for $next ($diff changed pixels)"
      return 0
    fi
    alive
    sleep .3
  done
  fail "new-area backdrop never reached the rendered framebuffer: $next (changed pixels: $diff)"
}

# Assert a real dialog surface is visible but does not paint over the
# background character's face. Pixel checks are deliberately restricted to
# stable background areas (no player sprite or dynamic controls).
check_mateo_bubble() {
  local baseline="$out/04-store-interior.png"
  local talking="$out/05-mateo-conversation.png"
  local face_old="$work/mateo-face-before.png" face_new="$work/mateo-face-after.png"
  local bubble_old="$work/mateo-dialogue-before.png" bubble_new="$work/mateo-dialogue-after.png"
  local face_diff bubble_diff
  convert "$baseline" -crop 230x175+650+85 +repage "$face_old"
  convert "$talking" -crop 230x175+650+85 +repage "$face_new"
  face_diff=$(compare -metric AE "$face_old" "$face_new" null: 2>&1 || :)
  [[ $face_diff =~ ^[0-9]+$ ]] || fail "invalid face comparison pixels: $face_diff"
  (( face_diff < 1500 )) || fail "dialogue obscures Mateo's face ($face_diff altered pixels)"
  convert "$baseline" -crop 530x145+460+360 +repage "$bubble_old"
  convert "$talking" -crop 530x145+460+360 +repage "$bubble_new"
  bubble_diff=$(compare -metric AE "$bubble_old" "$bubble_new" null: 2>&1 || :)
  [[ $bubble_diff =~ ^[0-9]+$ ]] || fail "invalid bubble comparison pixels: $bubble_diff"
  (( bubble_diff > 12000 )) || fail "Mateo dialogue surface not visibly painted ($bubble_diff pixels)"
  echo "Mateo rendered bubble visible ($bubble_diff pixels); face protected ($face_diff pixels)"
}

# The runtime can announce choice mode before the next framebuffer is
# presented. Save the *last* visible narration frame before pressing Enter
# so the next captured PNG can be checked against actual rendered changes.
advance_until() {
  local phrase=$1 max=${2:-80} tick
  for ((tick=0;tick<max;tick++)); do
    contains "$phrase" && return 0
    alive
    if [[ $phrase == 'Vesper choices available: 4 in scene board_read' ]]; then
      frame '06-board-before-choice.png'
    fi
    key Return
  done
  fail "did not advance story to event: $phrase"
}
wait_board_choice_paint() {
  local before="$out/06-board-before-choice.png" current="$out/06-noticeboard-choices.png"
  local first="$work/board-before-choice.png" second="$work/board-current.png"
  local diff tick
  [[ -s $before ]] || fail 'no actual narration-frame baseline captured before board choices'
  convert "$before" -crop 1080x350+100+330 +repage "$first" ||
    fail 'cannot crop pre-choice narration image'
  for ((tick=0;tick<30;tick++)); do
    sleep .3
    frame '06-noticeboard-choices.png'
    convert "$current" -crop 1080x350+100+330 +repage "$second" ||
      fail 'cannot crop rendered board choices'
    diff=$(compare -metric AE "$first" "$second" null: 2>&1 || :)
    if [[ $diff =~ ^[0-9]+$ ]] && (( diff > 35000 )); then
      echo "Board choice paint changed $diff pixels from the last narration frame"
      return 0
    fi
    alive
  done
  fail "board choices were logged but not substantially rendered (changed pixels: $diff)"
}

# Log events precede actual presentation. Compare the board's on-screen
# lower panel before and after postponing or revisiting an interaction.
wait_board_surface_change() {
  local before=$1 after=$2 changed tick
  local previous_crop="$work/board-mode-previous.png"
  local current_crop="$work/board-mode-current.png"
  convert "$out/$before" -crop 1080x350+100+330 +repage "$previous_crop" ||
    fail "cannot crop board baseline: $before"
  for ((tick=0;tick<30;tick++)); do
    sleep .3
    frame "$after"
    convert "$out/$after" -crop 1080x350+100+330 +repage "$current_crop" ||
      fail "cannot crop board frame: $after"
    changed=$(compare -metric AE "$previous_crop" "$current_crop" null: 2>&1 || :)
    if [[ $changed =~ ^[0-9]+$ ]] && (( changed > 35000 )); then
      echo "Board transition painted: $before -> $after ($changed pixels)"
      return 0
    fi
    alive
  done
  fail "board transition logged but not painted: $before -> $after ($changed pixels)"
}

wait_log 'Vesper scene dialogue: intro'
advance_until 'Vesper exploration area loaded: street;' 30
frame '01-street-opening.png'

# The game-authored T shortcut works before walking into Elena's radius.
key t
wait_log 'Vesper exploration direct interaction: elena -> elena_chat'
frame '02-elena-conversation.png'
key F5
wait_log 'Vesper save written'
save="$work/saves/una-tarde-en-el-barrio.json"
[[ -s $save ]] || fail 'no real quick-save file created'
grep -Fq '"return_to"' "$save" || fail 'modal save lost return-to-world position'
key F9
wait_log 'Vesper save restored: Conversation'
key q
wait_log 'Vesper exploration conversation cancelled'

# Walk to the actual shop exit and enter it, instead of simulating a teleport.
walk_to street 654 34
frame '03-street-after-walking.png'
key e
wait_log 'Vesper exploration exit: street/store_door -> store'
wait_log 'Vesper exploration area loaded: store;'
current_x=188
wait_new_area_frame '03-street-after-walking.png' '04-store-interior.png'

key t
wait_log 'Vesper exploration direct interaction: mateo -> mateo_chat'
sleep .8
frame '05-mateo-conversation.png'
check_mateo_bubble
key q
wait_log 'Vesper exploration conversation cancelled'

# Ledger privacy regression: real in-world interactions must preserve the
# earlier privacy breach even if the player later leaves the ledger closed.
# Confirm the logical flags in actual atomic F5 save JSON, not a mock VM.
saved_boolean() {
  local name=$1 expected=$2
  awk -v k="$name" -v v="$expected" '
    $0 ~ "\"" k "\"[[:space:]]*:" {
      seen=1
      if ($0 ~ /"Boolean"[[:space:]]*:/) {
        if ($0 ~ (":" "[[:space:]]*" v "[[:space:]]*[,}]?")) ok=1
        seen=0
      }
      next
    }
    seen && /"Boolean"[[:space:]]*:/ {
      if ($0 ~ (":" "[[:space:]]*" v "[[:space:]]*[,}]?")) ok=1
      seen=0
    }
    END { exit !ok }
  ' "$save" || fail "F5 snapshot missing $name = $expected"
}
finish_ledger_visit() {
  local choice=$1 prior_choice prior_return
  prior_choice=$(count 'Vesper choices available: 3 in scene shop_ledger')
  key e
  wait_log 'Vesper exploration interacting: ledger -> shop_ledger'
  for ((turn=0;turn<18;turn++)); do
    (( $(count 'Vesper choices available: 3 in scene shop_ledger') > prior_choice )) && break
    key Return
  done
  (( $(count 'Vesper choices available: 3 in scene shop_ledger') > prior_choice )) ||
    fail 'Ledger never offered three genuine authored choices'
  prior_return=$(count 'Vesper exploration resumed at previous world position')
  key "$choice"
  for ((turn=0;turn<22;turn++)); do
    (( $(count 'Vesper exploration resumed at previous world position') > prior_return )) && return 0
    key Return
  done
  fail "Ledger choice $choice never returned to the shop"
}
walk_to store 555 24
finish_ledger_visit 2  # Read private accounts.
prior=$(count 'Vesper save written')
key F5
wait_increment 'Vesper save written' "$prior" 40
saved_boolean saw_ledger true
saved_boolean respected_privacy false
finish_ledger_visit 1  # Later close it; cannot undo the breach.
prior=$(count 'Vesper save written')
key F5
wait_increment 'Vesper save written' "$prior" 40
saved_boolean saw_ledger true
saved_boolean respected_privacy false
# F9 actually reloads that saved exploration state. Re-save after restoring
# to detect any discrepancy between the machine snapshot and live VM flags.
prior=$(count 'Vesper save restored:')
key F9
wait_increment 'Vesper save restored:' "$prior" 40
prior=$(count 'Vesper save written')
key F5
wait_increment 'Vesper save written' "$prior" 40
saved_boolean saw_ledger true
saved_boolean respected_privacy false
echo 'REAL BEVY PRIVACY PASS: read -> close -> F5/F9 restore -> F5; breach persists.'

# Travel back through the authored reciprocal exit.
walk_to store 88 24
key e
wait_log 'Vesper exploration exit: store/street_door -> street'
wait_log 'Vesper exploration area loaded: street;'
wait_new_area_frame '04-store-interior.png' '06-returned-to-street.png'
current_x=685
walk_to street 930 32
key e
wait_log 'Vesper exploration interacting: noticeboard -> board_read'
advance_until 'Vesper choices available: 4 in scene board_read' 25
wait_board_choice_paint

# One real save at the board; restore it between all three ending branches.
prior=$(count 'Vesper save written')
key F5
# F5 writes asynchronously; assert a new completion after entering board choices.
wait_increment 'Vesper save written' "$prior" 40
[[ -s $save ]] || fail 'board quick-save missing'

# The fourth choice is not an ending: it must return to the *real* street,
# permit another interaction with the board, and leave the original F5 save
# restorable. This path was absent from the v0.7.8 three-ending native smoke.
prior_return=$(count 'Vesper exploration resumed at previous world position')
key 4
for ((turn=0;turn<30;turn++)); do
  (( $(count 'Vesper exploration resumed at previous world position') > prior_return )) && break
  alive
  key Return
done
(( $(count 'Vesper exploration resumed at previous world position') > prior_return )) ||
  fail 'postponing the board choice did not return to the street'
wait_board_surface_change '06-noticeboard-choices.png' '06-board-postponed-street.png'

prior_interaction=$(count 'Vesper exploration interacting: noticeboard -> board_read')
prior_choices=$(count 'Vesper choices available: 4 in scene board_read')
key e
wait_increment 'Vesper exploration interacting: noticeboard -> board_read' "$prior_interaction" 40
for ((turn=0;turn<30;turn++)); do
  (( $(count 'Vesper choices available: 4 in scene board_read') > prior_choices )) && break
  alive
  key Return
done
(( $(count 'Vesper choices available: 4 in scene board_read') > prior_choices )) ||
  fail 'reopened noticeboard did not offer its four choices'
wait_board_surface_change '06-board-postponed-street.png' '06-board-reopened-choices.png'

# Restore the saved *first* visit, not the second visit, so ending checks
# also validate the earlier board-state snapshot survived postponement.
prior_restored=$(count 'Vesper save restored: Conversation { scene: "board_read"')
key F9
wait_increment 'Vesper save restored: Conversation { scene: "board_read"' "$prior_restored" 40
echo 'REAL BEVY POSTPONE PASS: choice 4 -> rendered street -> board revisit -> four choices -> F9 original board save.'

for item in '1:ending_listen' '2:ending_open' '3:ending_quiet'; do
  index=${item%%:*}
  ending=${item#*:}
  key "$index"
  advance_until "Vesper story reached ending scene: $ending" 85
  frame "07-ending-$index.png"
  if [[ $index != 3 ]]; then
    previous=$(count 'Vesper save restored:')
    key F9
    wait_increment 'Vesper save restored:' "$previous" 40
    # Restored pending choices must be ready; fail rather than guessing navigation.
    [[ $(count 'Vesper choices available: 4 in scene board_read') -ge 1 ]] \
      || fail 'board choices did not persist'
  fi
done

if grep -Ei 'Story transition/asset error|Story error:|thread .* panicked|asset failed' "$log"; then
  fail 'runtime reported story, asset, or renderer error'
fi
printf 'REAL BEVY GAME PASS: intro; 2 worlds; 2 direct NPCs; persistent ledger privacy F5; save/restore; world exit/return; noticeboard; 3 endings.\n'