#!/usr/bin/env python3
"""Apply editable radial highlights to portal inlays, preserving map geometry.
Run from repository root after building heron-brush. Safe to repeat.
"""
import json
import re
from pathlib import Path
import prefabs_gen


def apply(name, editor):
    path = Path('assets') / f'{name}.map'
    source = path.read_text()
    editor.call('open_map', path=str(path.resolve()))
    selected = set()
    for brush in json.loads(editor.call('list_brushes')):
        info = json.loads(editor.call('describe_brush', brush=brush['id']))
        for face in info['faces']:
            paint = face.get('paint') or {}
            jade = paint.get('color') == [65, 190, 181]
            turquoise = paint.get('gradient', {}).get('from') == [147, 251, 220]
            if (jade or turquoise) and abs(face['normal'][1]) > .999 and face['material'] != 'skip':
                selected.add((brush['id'], face['source_face']))
    lines = source.splitlines(keepends=True)
    bid = None
    index = 0
    for i, line in enumerate(lines):
        if line.startswith('// heron_brush id '):
            bid = str(int(line.split()[-1], 16))
            index = 0
        if line.startswith('('):
            if (bid, index) in selected:
                # Three stops: pale centre, saturated jade, a darker border.
                radial = 'radial 0 3 0 204 255 232 0.5 65 190 181 1 25 126 126'
                line, count = re.subn(r'(?<=heron_brush )gradient(?: [-+\d.eE]+){7}', radial, line)
                if not count:
                    # Override the inherited solid brush colour on this face only.
                    line = line.replace('// heron_brush ', '// heron_brush ' + radial + ' ', 1) if '// heron_brush ' in line else line.rstrip('\n') + ' // heron_brush ' + radial + '\n'
                lines[i] = line
            index += 1
    result = ''.join(lines)
    if result != source:
        assert path.read_text() == source
        path.write_text(result)
    return len(selected)


if __name__ == '__main__':
    prefabs_gen.BIN = 'target/release/heron-brush'
    editor = prefabs_gen.Editor()
    try:
        for name in ('portal_frame', 'portal_marble'):
            print(name, apply(name, editor))
    finally:
        editor.close()
