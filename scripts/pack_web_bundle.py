#!/usr/bin/env python3
"""Stage the web build's archives using only Python's standard library.

One startup archive holds what every visit needs. The Chinese, Japanese and Korean fonts
(crates/heron-brush-web/fonts) go into a second archive the editor fetches only for those
interface languages; the startup one keeps their small picker faces, so the language list
reads. Both are content-addressed and named in bundle/index.json.
"""
import json
import hashlib
from pathlib import Path
import struct
import sys


def archive(entries_and_data, destination: Path, stem: str):
    """Writes `<stem>-<sha256>.bin` and returns its index entry."""
    entries = []
    payload = bytearray()
    for name, data in entries_and_data:
        entries.append([name, len(payload), len(data)])
        payload.extend(data)
    manifest = json.dumps(entries, ensure_ascii=False, separators=(',', ':')).encode('utf8')
    blob = b'HBW1' + struct.pack('<I', len(manifest)) + manifest + payload
    digest = hashlib.sha256(blob).hexdigest()
    filename = f'{stem}-{digest}.bin'
    (destination / filename).write_bytes(blob)
    return {'file': filename, 'sha256': digest, 'bytes': len(blob)}, entries, len(payload)


def pack(root: Path, destination: Path):
    startup = []
    cjk = []
    # fonts/ holds the Chinese, Japanese and Korean faces the browser cannot lend.
    sources = [('assets', root / 'assets'), ('games', root / 'gamedata'), ('fonts', root / 'crates/heron-brush-web/fonts')]
    for prefix, source in sources:
        for path in sorted(source.rglob('*')):
            relative = path.relative_to(source)
            if not path.is_file() or any(part.startswith('.') or part == '__pycache__' for part in relative.parts):
                continue
            if prefix == 'assets' and (path.suffix in {'.py', '.md'} or (len(relative.parts) == 1 and path.suffix == '.png')):
                continue
            item = (f'{prefix}/{relative.as_posix()}', path.read_bytes())
            # The full CJK faces wait for a language that needs them; their licence stays
            # with the picker faces.
            (cjk if prefix == 'fonts' and len(relative.parts) == 1 and path.suffix == '.otf' else startup).append(item)
    destination.mkdir(parents=True, exist_ok=True)
    index, entries, payload = archive(startup, destination, 'startup')
    index['format'] = 1
    index['cjk'], cjk_entries, cjk_payload = archive(cjk, destination, 'cjk')
    (destination / 'index.json').write_text(json.dumps(index, separators=(',', ':')) + '\n', encoding='utf8')
    (destination / 'files.txt').write_text(''.join(f'{path}\n' for path, _, _ in entries + cjk_entries), encoding='utf8')
    print(f'Web startup: {len(entries)} files, {payload} payload bytes; CJK fonts: {len(cjk_entries)} files, {cjk_payload} bytes, fetched on demand')


if __name__ == '__main__':
    pack(Path(__file__).resolve().parent.parent, Path(sys.argv[1]))
