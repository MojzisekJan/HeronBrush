#!/usr/bin/env python3
"""Self-authored MD2/PCX pyramid, with two frames to check first-frame selection."""
from pathlib import Path
import struct,zipfile
root=Path(__file__).parent/'fixtures/md2'
root.mkdir(parents=True,exist_ok=True)
header=bytearray(128);header[:4]=bytes([10,5,1,8]);struct.pack_into('<4H',header,4,0,0,3,1);header[65]=1;struct.pack_into('<H',header,66,4)
palette=bytearray(768)
for i,color in enumerate([(255,40,40),(40,255,40),(40,40,255),(255,220,40)],1):palette[i*3:i*3+3]=bytes(color)
pcx=bytes(header)+bytes([1,2,3,4,4,3,2,1,12])+palette
name=b'models/check/skin.pcx'.ljust(64,b'\0')
uv=struct.pack('<10h',0,0,3,0,3,1,0,1,1,0)
faces=[(0,1,4),(1,2,4),(2,3,4),(3,0,4),(0,3,2),(0,2,1)]
tri=b''.join(struct.pack('<6H',*f,*f) for f in faces)
vertices=[(0,0,0,5),(32,0,0,5),(32,16,0,5),(0,16,0,5),(16,8,64,5)]
frames=b''.join(struct.pack('<6f16s',2,3,1,-32+dx,-24,0,b'preview')+b''.join(bytes(v) for v in vertices) for dx in (0,100))
commands=b''
for face in faces:
 commands+=struct.pack('<i',-3)
 for vertex in face:
  s,t=struct.unpack_from('<2h',uv,vertex*4);commands+=struct.pack('<ffi',(s+0.5)/4,(t+0.5)/2,vertex)
commands+=bytes(4)
skin_at=68;st_at=skin_at+len(name);tri_at=st_at+len(uv);frame_at=tri_at+len(tri);command_at=frame_at+len(frames);end=command_at+len(commands)
md2=struct.pack('<17i',844121161,8,4,2,60,1,5,5,len(faces),len(commands)//4,2,skin_at,st_at,tri_at,frame_at,command_at,end)+name+uv+tri+frames+commands
(root/'preview.md2').write_bytes(md2);(root/'skin.pcx').write_bytes(pcx)
files={'models/check/preview.md2':md2,'models/check/skin.pcx':pcx}
with zipfile.ZipFile(root/'models.pk3','w') as z:
 for path,data in files.items():
  info=zipfile.ZipInfo(path,date_time=(1980,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,data)
payload=b'';directory=b''
for path,data in files.items():
 directory+=struct.pack('<56sii',path.encode(),12+len(payload),len(data));payload+=data
(root/'models.pak').write_bytes(b'PACK'+struct.pack('<ii',12+len(payload),len(directory))+payload+directory)
print(root)
