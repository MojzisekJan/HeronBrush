"""Mark fully occluded brush faces with skip, preserving all brush geometry.

A face is removed only when its complete polygon, nudged towards its visible
side, is covered by the union of other opaque convex brushes in the asset.
Partial coverage is retained. No dependence on a camera or on ground placement.
"""
import json
import re
from pathlib import Path
import numpy as np

ROOT = Path(__file__).resolve().parent
POINT = r'\([^)]*\)'
FACE = re.compile(r'^(\s*'+POINT+r'\s*'+POINT+r'\s*'+POINT+r'\s+)(\S+)(.*)$')


def area(poly):
    if len(poly) < 3:
        return 0.
    return np.linalg.norm(sum((np.cross(poly[i]-poly[0], poly[i+1]-poly[0]) for i in range(1,len(poly)-1)),np.zeros(3))) / 2


def split(poly, normal, distance, epsilon=0.):
    inside=[];outside=[]
    for a,b in zip(poly,poly[1:]+poly[:1]):
        da=float(a@normal-distance);db=float(b@normal-distance)
        if abs(da)<=epsilon:da=0.
        if abs(db)<=epsilon:db=0.
        if da <= 0: inside.append(a)
        if da >= 0: outside.append(a)
        if (da<0 and db>0) or (da>0 and db<0):
            p=a+(b-a)*(da/(da-db));inside.append(p);outside.append(p)
    return inside,outside


def subtract(poly, planes, epsilon=0.):
    """Convex pieces of polygon outside a convex brush."""
    outside=[]
    for n,d in planes:
        poly,part=split(poly,n,d,epsilon)
        if len(part)>=3 and area(part)>1e-12:outside.append(part)
        if len(poly)<3:return outside
    return outside


def opaque(brush):
    for face in brush['faces']:
        material=face['material'].lower().rsplit('/',1)[-1]
        if material in ('glass','clip','origin') or 'glass' in material:
            return False
        if face.get('paint') and face['paint'].get('alpha',255)<255:
            return False
    return True


# Map plane reconstruction can differ by fractions of a micro-unit. This tolerance
# is only for coplanar contacts, never for the volume-occlusion fallback below.
CONTACT_EPSILON=1e-6


def face_boundary(face):
    vertices=[np.array(v,dtype=float) for v in face['vertices']]
    normal=np.array(face['normal'],dtype=float)
    center=np.mean(vertices,axis=0)
    edges=[]
    for a,b in zip(vertices,vertices[1:]+vertices[:1]):
        n=np.cross(b-a,normal);length=np.linalg.norm(n)
        if length<1e-12:continue
        n/=length;d=float(n@a)
        if n@center>d:n=-n;d=-d
        edges.append((n,d))
    return normal,float(normal@vertices[0]),edges


def shared_face_hidden(index,face,prepared):
    poly=[np.array(v,dtype=float) for v in face['vertices']]
    normal=np.array(face['normal'],dtype=float)
    lo=np.min(poly,axis=0);hi=np.max(poly,axis=0)
    pieces=[poly]
    for other,(blo,bhi,_,is_opaque,boundaries) in enumerate(prepared):
        if other==index or not is_opaque or np.any(hi<blo-CONTACT_EPSILON) or np.any(lo>bhi+CONTACT_EPSILON):continue
        for opposite,distance,edges in boundaries:
            if np.dot(normal,opposite)>-1+1e-10:continue
            if max(abs(p@opposite-distance) for p in poly)>CONTACT_EPSILON:continue
            pieces=[part for piece in pieces for part in subtract(piece,edges,CONTACT_EPSILON)]
            if not pieces:return True
            if len(pieces)>256:return False
    return False


def hidden_faces(brushes):
    # A small inward margin retains uncertain edge seams instead of opening cracks.
    prepared=[]
    for b in brushes:
        planes=[(np.array(f['normal']),float(np.array(f['normal'])@np.array(f['vertices'][0])) - 1e-6) for f in b['faces']]
        prepared.append((np.array(b['min']),np.array(b['max']),planes,opaque(b),[face_boundary(f) for f in b['faces']]))
    hidden=[]
    for i,b in enumerate(brushes):
        for f in b['faces']:
            if f['material'].lower().rsplit('/',1)[-1] in ('skip','clip','origin'):continue
            if shared_face_hidden(i,f,prepared):
                hidden.append((b['id'],f.get('source_face',f['face'])));continue
            normal=np.array(f['normal']);poly=[np.array(v)+normal*1e-4 for v in f['vertices']]
            if area(poly)<1e-10:continue
            lo=np.min(poly,axis=0);hi=np.max(poly,axis=0)
            pieces=[poly]
            for j,(blo,bhi,planes,is_opaque,_) in enumerate(prepared):
                if i==j or not is_opaque or np.any(hi<blo-1e-8) or np.any(lo>bhi+1e-8):continue
                pieces=[part for piece in pieces for part in subtract(piece,planes)]
                if not pieces:
                    hidden.append((b['id'],f.get('source_face',f['face'])));break
                # Conservatively stop if highly fragmented; never guess coverage.
                if len(pieces)>256:break
    return hidden



def body_groups(source, brushes):
    """Group brush IDs by owning map entity; independent movers cannot occlude each other."""
    owners={};depth=0;entity=-1;bid=None
    for line in source.splitlines():
        stripped=line.strip()
        if stripped.startswith('// heron_brush id '):
            bid=str(int(stripped.split()[-1],16))
        elif stripped=='{':
            if depth==0:entity+=1
            depth+=1
        elif stripped=='}':depth-=1
        elif FACE.match(line) and bid is not None:owners[bid]=entity
    grouped={}
    for brush in brushes:
        # Helper volumes have no visible surfaces and must not hide real geometry.
        if all(f['material'].lower().rsplit('/',1)[-1] in ('skip','clip','origin','nodraw','caulk')
               for f in brush['faces']):continue
        owner=owners.get(brush['id'])
        if owner is None:raise RuntimeError(f"Missing entity for brush {brush['id']}")
        grouped.setdefault(owner,[]).append(brush)
    return list(grouped.values())


def optimize_file(editor,path):
    path=Path(path).resolve()
    if path.parent!=ROOT or path.stem=='monkey' or path.suffix!='.map':return 0
    info=json.loads(editor.call('list_brushes'))
    brushes=[json.loads(editor.call('describe_brush',brush=b['id'])) for b in info]
    source=path.read_text()
    hidden={face for body in body_groups(source,brushes) for face in hidden_faces(body)}
    if not hidden:return 0
    lines=source.splitlines(keepends=True);bid=None;face=0;changed=0
    for i,line in enumerate(lines):
        if line.startswith('// heron_brush id '):
            bid=str(int(line.split()[-1],16));face=0
        match=FACE.match(line.rstrip('\n'))
        if match:
            if (bid,face) in hidden:
                lines[i]=match[1]+'skip'+match[3]+'\n';changed+=1
            face+=1
    if changed!=len(hidden):raise RuntimeError(f'{path.name}: face IDs did not match source')
    path.write_text(''.join(lines))
    editor.call('open_map',path=str(path))
    return changed
