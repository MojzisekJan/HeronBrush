#!/usr/bin/env python3
"""Generate original two-surface/two-frame MD3 test assets. No game data needed."""
import struct
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent / 'fixtures/md3'

def integer(*values):
    return struct.pack('<' + 'i' * len(values), *values)

def text(value, size):
    return value.encode().ljust(size, b'\0')

def surface(index):
    triangles = integer(0, 1, 2)
    shader = text(f'models/check/skin{index}', 64) + integer(0)
    uv = struct.pack('<6f', 0, 0, 1, 0, 0, 1)
    vertices = b''
    for frame in range(2):
        for x, y in [(-64, 0), (64, 0), (0, 64)]:
            vertices += struct.pack('<3hH', x + frame * 6400, y, index * 64, (index * 64 << 8) | 64)
    offsets = [108, 120, 188, 212, 260]
    return b'IDP3' + text(f'surface{index}', 64) + integer(0, 2, 1, 3, 1, *offsets) + triangles + shader + uv + vertices

def tga(color):
    return bytes([0, 0, 2]) + bytes(9) + struct.pack('<HHBB', 2, 2, 32, 0x28) + bytes([color[2], color[1], color[0], 255]) * 4

def main():
    ROOT.mkdir(parents=True, exist_ok=True)
    frames = b''.join(struct.pack('<10f16s', -1, 0, 0, 101, 1, 1, 0, 0, 0, 200, text(f'frame{i}', 16)) for i in range(2))
    tags = (text('tag_test', 64) + struct.pack('<12f', 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1)) * 2
    surfaces = surface(0) + surface(1)
    at = 108 + len(frames) + len(tags)
    model = b'IDP3' + integer(15) + text('original editor fixture', 64) + integer(0, 2, 1, 2, 0, 108, 220, at, at + len(surfaces)) + frames + tags + surfaces
    files = {'models/check/preview.md3': model, 'models/check/skin0.tga': tga((240, 20, 30)), 'models/check/skin1.tga': tga((10, 220, 40))}
    for name, data in files.items():
        (ROOT / Path(name).name).write_bytes(data)
    with zipfile.ZipFile(ROOT / 'models.pk3', 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        for name, data in files.items():
            entry = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            entry.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(entry, data)
    body = bytearray(); directory = bytearray()
    for name, data in files.items():
        directory += text(name, 56) + integer(12 + len(body), len(data))
        body += data
    (ROOT / 'models.pak').write_bytes(b'PACK' + integer(12 + len(body), len(directory)) + body + directory)

if __name__ == '__main__':
    main()
