# Deterministic source-level VN scenario interpreter for Una Tarde en el Barrio.
# This does NOT replace vesper check-game or a Bevy runtime playthrough.
# Usage: awk -f qa/check-scenarios.awk scripts/story.vn qa/scenarios.tsv
BEGIN { FS="\t"; errors=0; tests=0 }
FNR==NR {
  content=$0
  sub(/\r$/, "", content)
  if (content ~ /^[[:space:]]*$/ || content ~ /^#/) next
  if (content ~ /^@scene[[:space:]]+/) {
    split(content, words, /[[:space:]]+/)
    current=words[2]
    if (current in exists) { print "Duplicate scene: " current > "/dev/stderr"; errors++ }
    exists[current]=1
    next
  }
  if (current == "") { print "Instruction before first scene: " content > "/dev/stderr"; errors++; next }
  source[current, ++lines_in[current]]=content
  if (content ~ /^if[[:space:]]+/) conditions[current SUBSEP lines_in[current]]=1
  if (content ~ /^\*[[:space:]]/) {
    target=content; sub(/^.* -> /, "", target)
    authored_edges[current SUBSEP target]=1
  }
  next
}
NF >= 3 && $1 !~ /^#/ {
  cases[++case_count]=$1
  plans[case_count]=$2
  must[case_count]=$3
  never[case_count]=$4
  next
}
function fail(message) { print "FAIL " case_name ": " message > "/dev/stderr"; failed=1 }
function visit(id, selected,     step, line, key, cmp, level, ok, dest, options, ret, value) {
  if (!(id in exists)) { fail("missing scene " id); return "ERROR" }
  if (++visit_count > 128) { fail("script visit limit exceeded"); return "ERROR" }
  visits[id]++
  visited_any[id]=1
  level=0
  options=0
  ret=""
  returned_here=0
  for (step=1; step<=lines_in[id]; step++) {
    line=source[id,step]
    if (line ~ /^if[[:space:]]+/) {
      split(line, cmp, /[[:space:]]+/)
      key=cmp[2]
      if (!(key in state)) fail("condition references uninitialized flag " key " at " id)
      level++
      parent_ok[level]=(level==1 || frame[level-1])
      branch_ok[level]=(state[key] == cmp[4])
      if (parent_ok[level]) {
        if (branch_ok[level]) condition_yes[id SUBSEP step]=1
        else condition_no[id SUBSEP step]=1
      }
      frame[level]=parent_ok[level] && branch_ok[level]
      already_else[level]=0
      continue
    }
    if (line == "else") {
      if (level==0 || already_else[level]) fail("invalid else in " id)
      else { frame[level]=parent_ok[level] && !branch_ok[level]; already_else[level]=1 }
      continue
    }
    if (line == "end") {
      if (level==0) fail("unmatched conditional end in " id)
      else level--
      continue
    }
    if (level>0 && !frame[level]) continue
    if (line ~ /^set[[:space:]]+/) {
      split(line, cmp, /[[:space:]]+/)
      value=cmp[4]
      state[cmp[2]]=value
      continue
    }
    if (line ~ /^\*[[:space:]]/) {
      options++
      dest=line
      sub(/^.* -> /,"", dest)
      if (dest==selected) ret=dest
      continue
    }
    if (line ~ /^explore[[:space:]]/) { world_visits++; returned_here++; continue }
    if (line ~ /^"/ || line ~ /^[[:alnum:]_]+:[[:space:]]*"/) { transcript=transcript "\n" line; continue }
    if (line ~ /^(music|sound|background|surface|mode|show|hide)[[:space:]]/) continue
    fail("unsupported line in " id ": " line)
  }
  if (level!=0) fail("unclosed conditional in " id)
  if (options) {
    if (selected=="" || ret=="") { fail("no matching choice '" selected "' in " id); return "ERROR" }
    choice_count++
    visited_edges[id SUBSEP ret]=1
    return visit(ret, "")
  }
  if (selected!="") { fail("choice supplied for scene without choices " id); return "ERROR" }
  if (id ~ /^ending_/) { ending=id; return "ENDING" }
  if (returned_here == 0) { fail("scene failed to return to exploration: " id); return "ERROR" }
  return "WORLD"
}
function assertions(raw, should_exist,       tests, count, i, term) {
  if (raw=="") return
  count=split(raw, tests, /;;/)
  for (i=1; i<=count; i++) {
    term=tests[i]
    if (term=="") continue
    if (should_exist && !index(transcript, term)) fail("required story beat missing: " term)
    if (!should_exist && index(transcript, term)) fail("unwanted conditional story beat: " term)
  }
}
END {
  if (case_count==0) { print "No scenario cases found" > "/dev/stderr"; exit 1 }
  for (c=1;c<=case_count;c++) {
    case_name=cases[c]
    delete state; delete visits; delete frame; delete parent_ok; delete branch_ok; delete already_else
    transcript=""; ending=""; world_visits=0; visit_count=0; choice_count=0; failed=0
    if (visit("intro", "") != "WORLD") fail("introduction failed to enter exploration")
    n=split(plans[c], moves, /,/)
    last=""
    for (i=1; i<=n; i++) {
      split(moves[i], fields, />/)
      if (fields[1]=="") { fail("empty scenario action"); break }
      last=visit(fields[1], fields[2])
      if (last=="ERROR") break
      if (last=="ENDING" && i != n) fail("ending reached before last action")
      if (last=="WORLD" && i == n) fail("last action did not reach an ending")
    }
    if (ending=="" || last != "ENDING") fail("no complete ending")
    assertions(must[c], 1)
    assertions(never[c], 0)
    if (!failed) { tests++; print "PASS " case_name " => " ending " (" visit_count " scenes, " choice_count " choices, " world_visits " world returns)" }
    else errors++
  }
  for (id in exists) {
    scenes_total++
    if (!(id in visited_any)) { print "UNTESTED scene: " id > "/dev/stderr"; errors++ }
    else scenes_seen++
  }
  for (edge in authored_edges) {
    edges_total++
    if (!(edge in visited_edges)) {
      split(edge, parts, SUBSEP)
      print "UNTESTED choice: " parts[1] " -> " parts[2] > "/dev/stderr"; errors++
    } else edges_seen++
  }
  for (condition in conditions) {
    conditions_total++
    if (!(condition in condition_yes) || !(condition in condition_no)) {
      split(condition, parts, SUBSEP)
      print "UNTESTED conditional outcome: " parts[1] " line " parts[2]         " (true=" (condition in condition_yes) ", false=" (condition in condition_no) ")" > "/dev/stderr"
      errors++
    } else conditions_covered++
  }
  printf "Scenario coverage: %d/%d cases, %d/%d scenes, %d/%d choices, %d/%d true+false condition pairs, %d errors. SOURCE-LEVEL ONLY.\n", tests,case_count,scenes_seen,scenes_total,edges_seen,edges_total,conditions_covered,conditions_total,errors
  exit errors>0 ? 1 : 0
}
