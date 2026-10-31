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

def paginate(text: str, limit: int = 22, char_limit: int = 120) -> list[str]:
    """Fit the fixed manuscript writing rectangle without orphaned words.

    Preserve every source word, prioritize natural sentence boundaries and
    rebalance short tails so the player never gets a one-word page.
    """
    words = text.split()
    if len(words) <= limit and len(text) <= char_limit:
        return [text]
    pieces = re.split(r'(?<=[.!?])\s+(?=[A-Z0-9“"I])', text)
    result = []
    current = []

    def flush():
        nonlocal current
        if current:
            result.append(' '.join(current))
            current = []

    for piece in pieces:
        tokens = piece.split()
        if len(tokens) > limit or len(piece) > char_limit:
            for token in tokens:
                length = len(' '.join(current)) + 1 + len(token) if current else len(token)
                if current and (len(current) >= limit or length > char_limit):
                    flush()
                current.append(token)
        else:
            length = len(' '.join(current)) + 1 + len(piece) if current else len(piece)
            if current and (len(current) + len(tokens) > limit or length > char_limit):
                flush()
            current.extend(tokens)
    flush()

    for index in range(1, len(result)):
        if len(result[index].split()) >= 6:
            continue
        joined = (result[index - 1] + ' ' + result[index]).split()
        options = []
        for cut in range(6, len(joined) - 5):
            left, right = ' '.join(joined[:cut]), ' '.join(joined[cut:])
            if len(left) <= char_limit and len(right) <= char_limit and cut <= limit and len(joined)-cut <= limit:
                score = abs(len(left) - len(right))
                if left.endswith(('.', '!', '?', '.”', '!”', '?”')):
                    score -= 12
                options.append((score, cut))
        if options:
            _, cut = min(options)
            result[index - 1] = ' '.join(joined[:cut])
            result[index] = ' '.join(joined[cut:])
    assert ' '.join(result) == ' '.join(words), 'Pagination lost authored prose'
    assert all(len(chunk) <= char_limit for chunk in result), 'Oversized dialogue page'
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
            # Authoring choices may be quoted for readability. Decode the
            # author's surrounding quotes instead of showing them in the UI.
            choice = choice.strip()
            if choice.startswith('"') and choice.endswith('"'):
                choice = json.loads(choice)
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
