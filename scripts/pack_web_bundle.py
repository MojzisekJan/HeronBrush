#!/usr/bin/env python3
"""Stage one deterministic startup archive using only Python's standard library."""
import json
import hashlib
from pathlib import Path
import struct
import sys


def pack(root: Path, destination: Path):
    entries = []
    payload = bytearray()
    # fonts/ holds the Chinese, Japanese and Korean faces the browser cannot lend.
    sources = [('assets', root / 'assets'), ('games', root / 'gamedata'), ('fonts', root / 'crates/heron-brush-web/fonts')]
    for prefix, source in sources:
        for path in sorted(source.rglob('*')):
            relative = path.relative_to(source)
            if not path.is_file() or any(part.startswith('.') or part == '__pycache__' for part in relative.parts):
                continue
            if prefix == 'assets' and (path.suffix in {'.py', '.md'} or (len(relative.parts) == 1 and path.suffix == '.png')):
                continue
            data = path.read_bytes()
            entries.append([f'{prefix}/{relative.as_posix()}', len(payload), len(data)])
            payload.extend(data)
    manifest = json.dumps(entries, ensure_ascii=False, separators=(',', ':')).encode('utf8')
    destination.mkdir(parents=True, exist_ok=True)
    archive = b'HBW1' + struct.pack('<I', len(manifest)) + manifest + payload
    digest = hashlib.sha256(archive).hexdigest()
    filename = f'startup-{digest}.bin'
    (destination / filename).write_bytes(archive)
    (destination / 'index.json').write_text(json.dumps({
        'format': 1, 'file': filename, 'sha256': digest, 'bytes': len(archive),
    }, separators=(',', ':')) + '\n', encoding='utf8')
    (destination / 'files.txt').write_text(''.join(f'{path}\n' for path, _, _ in entries), encoding='utf8')
    print(f'Web startup: {len(entries)} files, {len(payload)} payload bytes, one manifest + one cacheable archive')


if __name__ == '__main__':
    pack(Path(__file__).resolve().parent.parent, Path(sys.argv[1]))
