#!/usr/bin/env python3
"""Gathers the textures of Half-Life: Uplink's maps into a WAD3 for mapping.

Usage: extract_wad.py PAK0 OUTPUT_WAD

Uplink ships no texture wad. Its maps name halflife.wad, liquids.wad and hldemo.wad in
worldspawn, but none of those came with the demo: every map carries its textures
inside the bsp instead. This reads maps/*.bsp out of valve/pak0.pak and writes every
texture into one WAD3, first map first, so a texture used by several maps comes from
the first. A bsp's miptex and a WAD3's texture lump are the same bytes, palette
included, so the textures are copied as they are.

The wad is game data and is not committed; build.sh makes it.
"""

import struct
import sys

BSP_VERSION = 30
LUMP_TEXTURES = 2
TYPE_MIPTEX = 0x43  # 'C'


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
        # Four mip levels, then a two-byte colour count, the palette and padding to
        # a multiple of four.
        size = 40 + width * height * 85 // 64 + 2 + 768
        size += -size % 4
        yield name.split(b"\0", 1)[0].decode("latin-1"), bsp[base : base + size]


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    files = read_pak(sys.argv[1])
    textures = {}
    for name in sorted(n for n in files if n.startswith("maps/") and n.endswith(".bsp")):
        for texture, data in miptextures(files[name]):
            textures.setdefault(texture.lower(), (texture, data))

    body = bytearray()
    directory = bytearray()
    for texture, data in textures.values():
        position = 12 + len(body)
        body += data
        directory += struct.pack(
            "<iiibbh16s", position, len(data), len(data), TYPE_MIPTEX, 0, 0,
            texture.encode("latin-1")[:15],
        )
    with open(sys.argv[2], "wb") as wad:
        wad.write(struct.pack("<4sii", b"WAD3", len(textures), 12 + len(body)))
        wad.write(body)
        wad.write(directory)
    print(f"{sys.argv[2]}: {len(textures)} textures")


if __name__ == "__main__":
    main()
