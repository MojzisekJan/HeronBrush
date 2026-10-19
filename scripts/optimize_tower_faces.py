#!/usr/bin/env python3
"""Audit and hide fully covered faces of a mitered tower copy using editor geometry.
Usage: optimize_tower_faces.py source.map mitered.map output.map
"""
import sys,json,re,hashlib,time
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'assets'))
import prefabs_gen
from asset_skip import FACE,area,subtract,face_boundary,opaque,shared_face_hidden,body_groups
prefabs_gen.BIN=str(ROOT/'target/release/heron-brush')

def describe(ed,path):
    ed.call('open_map',path=str(path))
    listed=json.loads(ed.call('list_brushes'))
    return [json.loads(ed.call('describe_brush',brush=b['id'])) for b in listed]

def stats(brushes):
    visible=[f for b in brushes for f in b['faces'] if f['material'].rsplit('/',1)[-1]!='skip']
    return dict(brushes=len(brushes),faces=sum(len(b['faces']) for b in brushes),rendered_faces=len(visible),
        rendered_triangles=sum(len(f['vertices'])-2 for f in visible),
        rendered_area=float(sum(area([np.array(p) for p in f['vertices']]) for f in visible)))

def hidden_target_faces(brushes):
    prepared=[]
    for b in brushes:
        planes=[(np.array(f['normal']),float(np.array(f['normal'])@np.array(f['vertices'][0]))-1e-6) for f in b['faces']]
        prepared.append((np.array(b['min']),np.array(b['max']),planes,opaque(b),[face_boundary(f) for f in b['faces']]))
    lo=np.array([b['min'] for b in brushes]);hi=np.array([b['max'] for b in brushes]);hidden=set()
    for i,b in enumerate(brushes):
        neighbours=np.flatnonzero(np.all(hi>=lo[i]-1e-4,axis=1)&np.all(lo<=hi[i]+1e-4,axis=1))
        local=[prepared[i]]+[prepared[j] for j in neighbours if j!=i]
        for f in b['faces']:
            if f['material'].rsplit('/',1)[-1] in ('skip','clip','origin'):continue
            key=(b['id'],f.get('source_face',f['face']))
            if shared_face_hidden(0,f,local):hidden.add(key);continue
            normal=np.array(f['normal']);poly=[np.array(p)+normal*1e-4 for p in f['vertices']]
            if area(poly)<1e-10:continue
            flo=np.min(poly,axis=0);fhi=np.max(poly,axis=0);pieces=[poly]
            for blo,bhi,planes,is_opaque,_ in local[1:]:
                if not is_opaque or np.any(fhi<blo-1e-8) or np.any(flo>bhi+1e-8):continue
                pieces=[part for piece in pieces for part in subtract(piece,planes)]
                if not pieces:hidden.add(key);break
                if len(pieces)>256:break
        if i%250==0:print('coverage',i,'/',len(brushes),'hidden',len(hidden),flush=True)
    return hidden

def main():
    source,mitered,output=map(lambda p:Path(p).resolve(),sys.argv[1:])
    assert not output.exists(), 'Output already exists'
    original=source.read_bytes();text=mitered.read_text()
    ed=prefabs_gen.Editor()
    try:
        before=describe(ed,source);after=describe(ed,mitered)
        assert [b['id'] for b in before]==[b['id'] for b in after]
        hidden=set()
        for group in body_groups(text,after):hidden.update(hidden_target_faces(group))
        lines=text.splitlines(keepends=True);bid=None;face=0;written=0
        for i,line in enumerate(lines):
            if line.startswith('// heron_brush id '):bid=str(int(line.split()[-1],16));face=0
            m=FACE.match(line.rstrip('\n'))
            if m:
                if (bid,face) in hidden:lines[i]=m[1]+'skip'+m[3]+'\n';written+=1
                face+=1
        assert written==len(hidden)
        text=''.join(lines)
        # Resolve the original resource declaration against its original map directory.
        match=re.search(r'^// heron material-sources (.*)$',text,re.M)
        if match:
            paths=json.loads(match[1]);paths=[[kind,str((source.parent/path).resolve())] for kind,path in paths]
            text=text[:match.start(1)]+json.dumps(paths)+text[match.end(1):]
        output.write_text(text)
        final=describe(ed,output)
        assert [b['id'] for b in final]==[b['id'] for b in before]
        # A material-only pass must never alter any reconstructed geometry.
        for a,b in zip(after,final):
            assert a['min']==b['min'] and a['max']==b['max']
            assert [f['vertices'] for f in a['faces']]==[f['vertices'] for f in b['faces']]
        assert source.read_bytes()==original,'source changed during run'
        report=dict(source=str(source),source_sha256=hashlib.sha256(original).hexdigest(),output=str(output),
            before=stats(before),after_miters=stats(after),after=stats(final),additional_skip_faces=written)
        output.with_suffix('.report.json').write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps(report,indent=2),flush=True)
    finally:ed.close()
if __name__=='__main__':main()
