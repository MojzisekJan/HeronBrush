#!/usr/bin/env python3
"""Gathers the textures of the Hexen II demo's maps into a WAD2 for mapping.

Usage: extract_wad.py PAK0 OUTPUT_WAD

The demo ships no texture wad: Raven's own wads (gfx/castle.wad and the rest, which
worldspawn names in the original maps) stayed on Raven's machines. What the demo does
have is the textures themselves, inside each map's bsp. This reads maps/*.bsp out of
pak0.pak and writes every texture that has pixels into one WAD2, first map first, so a
texture used by several maps comes from the first. The pixels are 8-bit and use the
game's palette, gfx/palette.lmp in the same pak, which is how qbsp and Heron Brush read
them.

The wad is game data and is not committed; build.sh makes it.
"""

import struct
import sys

BSP_VERSION = 29
LUMP_TEXTURES = 2
TYPE_MIPTEX = 0x44  # 'D'


def read_pak(path):
    """Returns {name: bytes} for every file in an id pak."""
    with open(path, "rb") as pak:
        data = pak.read()
    magic, directory, length = struct.unpack_from("<4sii", data, 0)
    if magic != b"PACK":
        sys.exit(f"{path}: not a pak file")
    files = {}
    for offset in range(directory, directory + length, 64):
        name, position, size = struct.unpack_from("<56sii", data, offset)
        files[name.split(b"\0", 1)[0].decode("latin-1")] = data[position : position + size]
    return files


def miptextures(bsp):
    """Yields (name, miptex bytes) for each texture a bsp carries pixels for."""
    version = struct.unpack_from("<i", bsp, 0)[0]
    if version != BSP_VERSION:
        return
    offset, length = struct.unpack_from("<ii", bsp, 4 + LUMP_TEXTURES * 8)
    if length == 0:
        return
    count = struct.unpack_from("<i", bsp, offset)[0]
    starts = struct.unpack_from(f"<{count}i", bsp, offset + 4)
    for start in starts:
        if start < 0:
            continue
        base = offset + start
        name, width, height, *mips = struct.unpack_from("<16sII4I", bsp, base)
        if mips[0] == 0 or width == 0 or height == 0:
            continue
        size = 40 + width * height * 85 // 64
        yield name.split(b"\0", 1)[0].decode("latin-1").lower(), bsp[base : base + size]


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    files = read_pak(sys.argv[1])
    textures = {}
    for name in sorted(n for n in files if n.startswith("maps/") and n.endswith(".bsp")):
        for texture, data in miptextures(files[name]):
            textures.setdefault(texture, data)

    body = bytearray()
    directory = bytearray()
    for name, data in textures.items():
        position = 12 + len(body)
        body += data
        directory += struct.pack(
            "<iiibbh16s", position, len(data), len(data), TYPE_MIPTEX, 0, 0,
            name.encode("latin-1")[:15],
        )
    with open(sys.argv[2], "wb") as wad:
        wad.write(struct.pack("<4sii", b"WAD2", len(textures), 12 + len(body)))
        wad.write(body)
        wad.write(directory)
    print(f"{sys.argv[2]}: {len(textures)} textures")


if __name__ == "__main__":
    main()
