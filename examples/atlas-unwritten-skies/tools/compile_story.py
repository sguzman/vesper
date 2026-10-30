#!/usr/bin/env python3
"""Deterministic authoring compiler for the Vesper native .vn dialogue format.

Source format has readable prose paragraphs, `Mira:` and `Cael:` lines,
`!` for canonical Vesper commands, and `? label -> target` for choices.
No runtime Python; its output is standard external Vesper authoring data.
"""
from pathlib import Path
import json
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / 'scripts' / 'novel.source'
DEST = ROOT / 'scripts' / 'story.vn'

def paginate(text: str, limit: int = 44) -> list[str]:
    """Author-dialogue pages are a comfortable 2–5 lines, not a wall of prose.

    Prefer sentence boundaries, then comma/semicolon, finally word boundaries.
    Long source paragraphs retain their order and wording verbatim.
    """
    words = text.split()
    if len(words) <= limit:
        return [text]
    pieces = re.split(r'(?<=[.!?])\s+(?=[A-Z0-9“"I])', text)
    result=[]
    current=[]
    def flush():
        nonlocal current
        if current:
            result.append(' '.join(current))
            current=[]
    for piece in pieces:
        tokens = piece.split()
        if len(tokens)>limit:
            for token in tokens:
                if len(current)>=limit: flush()
                current.append(token)
        else:
            if current and len(current)+len(tokens)>limit:flush()
            current.extend(tokens)
    flush()
    assert ' '.join(result)== ' '.join(words)
    return result


def build() -> dict:
    output = ['# THE ATLAS OF UNWRITTEN SKIES', '# Compiled offline from scripts/novel.source', '# Do not edit this generated file.']
    scenes = []
    dialogue = 0
    choices = 0
    for lineno, raw in enumerate(SRC.read_text(encoding='utf-8').splitlines(), 1):
        line = raw.strip()
        if not line:
            output.append('')
            continue
        if line.startswith('#'):
            continue
        if line.startswith('@scene '):
            scenes.append(line[7:])
            output.append(line)
        elif line.startswith('!'):
            output.append(line[1:])
        elif line.startswith('? '):
            choice, sep, target = line[2:].rpartition(' -> ')
            if not sep:
                raise ValueError(f'line {lineno}: choice missing destination')
            output.append('* ' + json.dumps(choice, ensure_ascii=False) + ' -> ' + target)
            choices += 1
        else:
            speaker = None
            if line.startswith('Mira: '):
                speaker = 'mira'
                line = line[6:]
            elif line.startswith('Cael: '):
                speaker = 'cael'
                line = line[6:]
            # Avoid double escaping [[fx=...]] tags; json.dumps only escapes quotes.
            for chunk in paginate(line):
                output.append((speaker + ': ' if speaker else '') + json.dumps(chunk, ensure_ascii=False))
                dialogue += 1
    if len(scenes) != len(set(scenes)):
        raise ValueError('duplicate @scene identifiers')
    DEST.write_text('\n'.join(output)+'\n', encoding='utf-8')
    stats = {'scenes':len(scenes), 'dialogue_passages':dialogue, 'choice_options':choices,
             'source_words':len(SRC.read_text(encoding='utf-8').split())}
    return stats

if __name__=='__main__':
    print(json.dumps(build(), indent=2))
