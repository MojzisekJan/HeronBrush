#!/usr/bin/env python3
"""Tiny, self-authored assets for mod/archive lookup integration checks."""
import struct,zlib,binascii,zipfile
from pathlib import Path
root=Path(__file__).parent/'fixtures/game-assets'
root.mkdir(parents=True,exist_ok=True)
def png(color):
 def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',binascii.crc32(kind+data)&0xffffffff)
 return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',1,1,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\x00'+bytes(color)))+chunk(b'IEND',b'')
for name,color in [('red',(255,0,0,255)),('green',(0,255,0,255)),('blue',(0,0,255,255))]:(root/f'{name}.png').write_bytes(png(color))
normal=struct.pack('<BBBHHBHHHHBB',0,0,2,0,0,0,0,0,1,1,24,32)+bytes([255,128,128])
files={
 'models/marker.obj':b'mtllib marker.mtl\nv 0 0 0\nv 7 0 0\nv 0 7 0\nvt 0 0\nvt 1 0\nvt 0 1\nusemtl surface\nf 1/1 2/2 3/3\n',
 'models/marker.mtl':b'newmtl surface\nmap_Kd ../textures/check.png\n',
 'textures/check.png':png((0,255,0,255)),
 'textures/check_normal.tga':normal,
 'scripts/surface.shader':b'textures/tagged\n{\n qer_editorimage textures/check.png\n surfaceparm nodraw\n}\n',
 'gfx/palette.lmp':bytes([0,255,0])*256,
}
with zipfile.ZipFile(root/'mod.pk3','w') as archive:
 for name,data in files.items():
  info=zipfile.ZipInfo(name,date_time=(1980,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;archive.writestr(info,data)
print(root)
