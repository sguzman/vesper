#!/usr/bin/env python3
"""Render coherent readable copies of four possible playthroughs for editorial QA.
Not used by Vesper at runtime. The authoritative branching novel is story.vn.
"""
from pathlib import Path
import re
import json
import operator

ROOT=Path(__file__).resolve().parents[1]
DEST=ROOT/'reader_editions'
DEST.mkdir(exist_ok=True)
source=(ROOT/'scripts/story.vn').read_text().splitlines()
scenes={}; current=None
for line in source:
 if line.startswith('@scene '):
  current=line[7:];scenes[current]=[]
 elif current is not None:scenes[current].append(line.strip())

policies={
 'open_sky':{'courier':'letter_open','atlas_wakes':'people_route','second_chapter':'trust_courier','third_chapter':'record_names','fourth_chapter':'consent_route','threshold_market':'families_heard','return_to_lens':'fetch_delegates','final_engine':'ending_open'},
 'living_witness':{'courier':'courier_questions','atlas_wakes':'memory_route','second_chapter':'examine_scar','third_chapter':'record_witness','fourth_chapter':'expose_route','threshold_market':'storm_heard','return_to_lens':'write_clause','final_engine':'ending_witness'},
 'solstice_covenant':{'courier':'protective_choice','atlas_wakes':'east_route','second_chapter':'suspect_courier','third_chapter':'record_contract','fourth_chapter':'negotiate_route','threshold_market':'chamber_heard','return_to_lens':'face_orsol','final_engine':'ending_covenant'},
 'unbound_world':{'courier':'letter_open','atlas_wakes':'people_route','second_chapter':'trust_courier','third_chapter':'record_witness','fourth_chapter':'expose_route','threshold_market':'families_heard','return_to_lens':'fetch_delegates','final_engine':'ending_unbound'},
}
ops={'==':operator.eq,'!=':operator.ne,'>':operator.gt,'<':operator.lt,'>=':operator.ge,'<=':operator.le}
def value(s):
 if s.startswith('"'):return json.loads(s)
 if s in ('true','false'):return s=='true'
 return int(s)

def render(name,policy):
 vars={};chunks=['# The Atlas of Unwritten Skies','',f'*Complete story path: {name.replace("_"," ").title()}*','']
 seen=[];where='prologue'
 while True:
  if where in seen:raise RuntimeError('narrative cycle '+where)
  seen.append(where)
  chunks.append(f'## {where.replace("_"," ").title()}\n')
  stack=[];candidate=[];jump=None
  for line in scenes[where]:
   if not line or line.startswith('#'):continue
   if line.startswith('if '):
    match=re.match(r'^if (\w+) (==|!=|>=|<=|>|<) (.*)$',line)
    if not match:raise ValueError(line)
    key,op,arg=match.groups()
    stack.append((all(stack) if stack else True) and ops[op](vars.get(key,0),value(arg)))
    continue
   if line=='end':stack.pop();continue
   if stack and not all(stack):continue
   if line.startswith('set '):
    m=re.match(r'^set (\w+) (\+=|-=|=) (.*)$',line)
    if m:
     key,op,arg=m.groups();v=value(arg)
     vars[key]=v if op=='=' else vars.get(key,0)+(v if op=='+=' else -v)
   elif line.startswith('jump '):jump=line[5:];break
   elif line.startswith('* '):candidate.append(line.rsplit(' -> ',1)[-1])
   elif line.startswith('"'):
    chunks.append(re.sub(r'\[\[(?:fx=[^\]]+|/fx)\]\]','',json.loads(line))+'\n')
   elif line.startswith(('mira: ','cael: ')):
    actor,payload=line.split(': ',1)
    chunks.append('**'+actor.title()+':** '+re.sub(r'\[\[(?:fx=[^\]]+|/fx)\]\]','',json.loads(payload))+'\n')
  if jump:where=jump;continue
  if candidate:
   selected=policy.get(where)
   if selected not in candidate:raise RuntimeError((where,selected,candidate))
   chunks.append(f'*Choice: {selected.replace("_"," ")}*\n')
   where=selected
  else:break
 content='\n'.join(chunks)+'\n'
 (DEST/f'{name}.md').write_text(content,encoding='utf-8')
 return {'path':name,'scene_count':len(seen),'words':len(content.split()),'ending':seen[-1]}

if __name__=='__main__':
 for name,policy in policies.items():print(render(name,policy))
