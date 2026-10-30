#!/usr/bin/env python3
"""Offline structural/asset smoke. It complements, never substitutes, Vesper check-game."""
from pathlib import Path
import collections
import re
import tomllib
import wave
from PIL import Image

ROOT=Path(__file__).resolve().parents[1]
m=tomllib.loads((ROOT/'game.toml').read_text())
t=tomllib.loads((ROOT/m['theme']).read_text())
text=(ROOT/m['story']).read_text()
lines=text.splitlines()
scenes={}
current=None
stack=[]
values=set()
all_edges=collections.defaultdict(set)
refs=collections.defaultdict(set)
allowed={'background','music','sound','panel','hide','show','ambient','set','if','end','jump'}
for n,s in enumerate(lines,1):
 s=s.strip()
 if not s or s.startswith('#'):continue
 if s.startswith('@scene '):
  assert not stack,(n,stack)
  current=s[len('@scene '):]
  assert current not in scenes,(n,current)
  scenes[current]=[]
  continue
 assert current,(n,s)
 scenes[current].append(s)
 if s.startswith('if '): stack.append(n)
 if s=='end':assert stack,(n,s);stack.pop()
 if s.startswith('jump '): all_edges[current].add(s[5:]);
 if s.startswith('* '):
  dest=s.rsplit(' -> ',1)[-1]
  all_edges[current].add(dest)
 if s.startswith(('background ','music ','sound ','panel ','hide ')):
  first,val=s.split(' ',1)
  refs[first].add(val)
 if s.startswith('show '):
  parts=s.split()
  assert len(parts)==5 and parts[3]=='at',(n,s)
  refs['characters'].add((parts[1],parts[2]))
 if s.startswith('ambient '):
  parts=s.split(); assert len(parts)==3 and parts[1] in ('show','hide'),(n,s)
  refs['ambient_actors'].add(parts[-1])
 if s.startswith(('"','mira: ','cael: ')):
  assert s.endswith('"'),(n,s)
 if s.startswith('if '):
  assert re.match(r'^if [a-z][a-z0-9_]* (==|!=|>=|<=|>|<) (\d+|"[^"]*"|true|false)$',s),(n,s)
assert not stack,stack
for s,targets in all_edges.items():
 for target in targets:assert target in scenes,(s,target)
for name in refs['background']:assert name in m['backgrounds'],name
for name in refs['music']:assert name in m['music'],name
for name in refs['sound']:assert name in m['sounds'],name
for name in refs['panel']:assert name in m['panel_skins'] or name=='default',name
for char,exp in refs['characters']:assert exp in m['characters'][char]['expressions'],(char,exp)
for name in refs['ambient_actors']:assert name in m['ambient_actors'],name
all_paths=[]
def explore(scene,path):
 assert len(path)<55,(scene,path)
 if scene in path:raise AssertionError(f'cycle {scene} through {path}')
 if not all_edges[scene]:
  assert scene.startswith('epilogue_'),(scene,path)
  all_paths.append(path+[scene]);return
 for target in all_edges[scene]:explore(target,path+[scene])
explore(m['entry'],[])
assert len({p[-1] for p in all_paths})==4
assert set(scenes)=={x for path in all_paths for x in path},'unreachable scenes'
for key,rel in list(m['backgrounds'].items())+list(m['music'].items())+list(m['sounds'].items()):
 assert (ROOT/rel).is_file(),(key,rel)
for cat in ('music','sounds'):
 for rel in m[cat].values():
  with wave.open(str(ROOT/rel)) as fp:
   assert (fp.getframerate(),fp.getnchannels(),fp.getsampwidth())==(16000,1,2)
for obj in m['characters'].values():
 for rel in obj['expressions'].values():
  im=Image.open(ROOT/rel)
  assert im.mode=='RGBA' and im.getchannel('A').getextrema()==(0,255),(rel,im.mode)
for key,rel in m['backgrounds'].items():
 im=Image.open(ROOT/rel)
 assert im.size==(1280,720),(key,im.size)
for obj in m['panel_skins'].values():
 im=Image.open(ROOT/obj['image']);assert im.mode=='RGBA'
 assert all(0<=v<=160 for v in obj['insets'])
print(f"Local validation PASS: {len(scenes)} scenes, {len(all_paths)} distinct graph walks, 4 reachable endings; {len(m['backgrounds'])} backgrounds, {len(m['music'])} music tracks, {len(m['sounds'])} effects, {sum(map(lambda x:len(x['expressions']),m['characters'].values()))} character expression treatments, all referenced and readable")
