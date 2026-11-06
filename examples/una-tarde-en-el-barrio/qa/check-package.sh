#!/usr/bin/env bash
# Static, dependency-light authoring checks; never claims to run Vesper or Bevy.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

[[ -s game.toml && -s scripts/story.vn && -s styles/urban.toml ]] || {
  echo 'ERROR: missing main game files' >&2
  exit 1
}

# Local manifest assets must exist, remain beneath the package, and decode.
count=0
while IFS= read -r path; do
  [[ $path == assets/* && $path != *'..'* && -s $path ]] || {
    echo "ERROR: absent/unsafe manifest asset: $path" >&2
    exit 1
  }
  case "$path" in
    *.png)
      if command -v identify >/dev/null; then
        identify -ping "$path" >/dev/null || {
          echo "ERROR: undecodable PNG: $path" >&2; exit 1;
        }
      else
        [[ $(od -An -tx1 -N8 "$path" | tr -d ' \n') == 89504e470d0a1a0a ]] || {
          echo "ERROR: invalid PNG header: $path" >&2; exit 1;
        }
      fi
      ;;
    *.wav)
      if command -v ffprobe >/dev/null; then
        ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1 "$path" \
          | grep -q '^duration=' || { echo "ERROR: invalid WAV: $path" >&2; exit 1; }
      else
        [[ $(head -c 4 "$path") == RIFF ]] || {
          echo "ERROR: invalid WAV header: $path" >&2; exit 1;
        }
      fi
      ;;
    *) echo "ERROR: unrecognized manifest resource type: $path" >&2; exit 1 ;;
  esac
  count=$((count+1))
done < <(grep -Eo '"assets/[^"[:cntrl:]]+"' game.toml | tr -d '"' | LC_ALL=C sort -u)

# Verify the simple scene/choice graph and that each boolean condition is
# defined by the intro or by a prior scene (no interpretive model of .vn).
awk '
function error(message) {print "ERROR: " message > "/dev/stderr"; errors++}
function finish_scene() {
  if (scene == "") return
  if (scene ~ /^ending_/) {
    if (!terminal[scene]) error("ending has no FIN text: " scene)
    if (!visual[scene]) error("ending does not enter visual_novel: " scene)
  } else if (!has_choice[scene] && !has_return[scene]) {
    error("scene has no choices or authored exploration return: " scene)
  }
}
/^@scene[[:space:]]+/ {
  finish_scene()
  if (depth != 0) error("unterminated condition in scene " scene)
  scene = $2
  if (scene in known) error("duplicate scene " scene)
  known[scene] = 1
  depth = 0
  scene_count++
  if (scene ~ /^ending_/) ending_count++
  next
}
/^explore[[:space:]]/ {has_return[scene]=1; next}
/^mode[[:space:]]+visual_novel/ {visual[scene]=1; next}
/^"FIN[[:space:]]/ {terminal[scene]=1; next}
/^set[[:space:]]+/ {defined[$2]=1; next}
/^if[[:space:]]+/ {
  if (!($2 in defined)) error("undefined condition " $2 " in " scene)
  depth++
  next
}
/^end[[:space:]]*$/ {
  depth--
  if (depth < 0) {error("unmatched end in " scene); depth=0}
  next
}
/^\*[[:space:]]/ {
  n=split($0, s, / -> /)
  if (n != 2) error("invalid choice: " $0)
  else {
    dest=s[2]; sub(/[[:space:]]*$/, "", dest)
    edges[scene SUBSEP dest]=1
    destinations[dest]=1
    choice_count++
    has_choice[scene]=1
  }
  next
}
END {
  finish_scene()
  if (depth != 0) error("unterminated final condition")
  for (target in destinations) if (!(target in known)) error("missing choice target " target)
  split("intro elena_chat rosa_chat mateo_chat shop_cat shop_display shop_ledger threshold_memory board_read", entries, " ")
  for (i in entries) {
    entry=entries[i]
    if (!(entry in known)) error("missing entry scene " entry)
    if (!(entry in visited)) {visited[entry]=1; queue[++tail]=entry}
  }
  head=1
  while (head<=tail) {
    from=queue[head++]
    for (key in edges) {
      split(key,p,SUBSEP)
      if (p[1]==from && !(p[2] in visited)) {
        visited[p[2]]=1
        queue[++tail]=p[2]
      }
    }
  }
  for (item in known) if (!(item in visited)) error("unreachable scene " item)
  if (ending_count != 3) error("expected 3 authored endings, got " ending_count)
  if (scene_count != 31) error("expected 31 authored scenes, got " scene_count)
  if (choice_count != 22) error("expected 22 authored choice edges, got " choice_count)
  if (!errors) printf "Story graph OK: %d scenes, %d choices, %d endings, all scene IDs reachable.\n", scene_count, choice_count, ending_count
  exit errors ? 1 : 0
}
' scripts/story.vn

# Confirm every authored interaction scene is real and every world exit targets
# an existing location and named spawn, without asserting native reachability.
awk '
function error(message) {print "ERROR: " message > "/dev/stderr"; errors++}
/^\[exploration\.locations\.[^.]+\]$/ {
  x=$0; sub(/^\[exploration\.locations\./,"",x); sub(/\]$/,"",x)
  locations[x]=1; location_count++
}
/^\[exploration\.locations\.[^.]+\.spawns\.[^.]+\]$/ {
  x=$0; sub(/^\[exploration\.locations\./,"",x); sub(/\]$/,"",x)
  spawns[x]=1
}
/^\[exploration\.locations\.[^.]+\.exits\.[^.]+\]$/ {
  x=$0; sub(/^\[exploration\.locations\./,"",x); sub(/\]$/,"",x)
  exit_id=x; exits[exit_id]=1; exit_count++
}
/^to_location[[:space:]]*=/ {if (exit_id != "") {gsub(/"/, "", $3); exit_to[exit_id]=$3}}
/^to_spawn[[:space:]]*=/ {if (exit_id != "") {gsub(/"/, "", $3); spawn_to[exit_id]=$3}}
/^\[/ {if ($0 !~ /^\[exploration\.locations\.[^.]+\.exits\.[^.]+\]$/) exit_id=""}
END {
  if (location_count != 2) error("expected 2 explorable locations")
  if (exit_count != 2) error("expected 2 world exits")
  for (e in exits) {
    if (!(exit_to[e] in locations)) error("exit " e " has unknown destination " exit_to[e])
    if (!((exit_to[e] ".spawns." spawn_to[e]) in spawns)) error("exit " e " has missing destination spawn " spawn_to[e])
  }
  if (!errors) printf "World links OK: %d locations, %d valid destinations/spawns.\n",location_count,exit_count
  exit errors ? 1 : 0
}
' game.toml
while IFS= read -r scene; do
  grep -Fqx "@scene $scene" scripts/story.vn || { echo "ERROR: missing authored interaction $scene" >&2; exit 1;}
done < <(sed -n 's/^interact_scene[[:space:]]*=[[:space:]]*"\([^"]*\)"$/\1/p' game.toml | sort -u)
echo "Manifest asset references OK: $count distinct decoded/checked resources; all interactions have scenes."
echo 'STATIC QA PASSED. This shell checker does not run the Rust Vesper parser or Bevy; see QA_STATUS.md for independent checks.'
"$root/qa/check-reader-hints.sh"
bash "$root/qa/check-menu-labels.sh"
"$root/qa/check-audio.sh"

"$root/qa/check-elena-art.sh"
"$root/qa/check-rosa-art.sh"
"$root/qa/check-protagonist-art.sh"
"$root/qa/check-avatar-grounding.sh"

"$root/qa/check-scenarios.sh"
