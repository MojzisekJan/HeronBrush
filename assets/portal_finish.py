"""Convex rounded edge profiles and native face gradients for portal prefabs."""
import math,json
from pathlib import Path
import redesign_gen as r
from prefabs_gen import Editor


def inset(poly,distance):
    if sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(poly,poly[1:]+poly[:1]))<0:poly=list(reversed(poly))
    lines=[]
    for a,b in zip(poly,poly[1:]+poly[:1]):
        dx,dz=b[0]-a[0],b[1]-a[1];length=math.hypot(dx,dz)
        nx,nz=-dz/length,dx/length
        lines.append((nx,nz,nx*a[0]+nz*a[1]+distance))
    result=[]
    for (a,b,c),(d,e,f) in zip(lines[-1:]+lines[:-1],lines):
        det=a*e-b*d
        result.append(((c*e-b*f)/det,(a*f-c*d)/det))
    return result


def rounded(poly,depth,colour,radius=.07):
    # Three facets approximate a quarter-round fillet at each rim, in ONE brush.
    points=[]
    for side in (-1,1):
        for angle in (0,math.pi/4,math.pi/2):
            y=side*(depth/2-radius+radius*math.sin(angle))
            profile=inset(poly,radius*(1-math.cos(angle)))
            points.extend((x*32,y*32,z*32) for x,z in profile)
    r.hull(points,colour)


def paint_world_gradient(ed,brush,light,dark,direction,limits=None):
    """Convert a world-space ramp to each generated face's unit, world-aligned UVs.

    Shared limits keep adjoining facets from restarting the entire colour ramp.
    Constant-height faces get a solid colour instead of an arbitrary UV gradient.
    """
    import numpy as np
    info=json.loads(ed.call('describe_brush',brush=brush['id']))
    direction=np.array(direction(info) if callable(direction) else direction,dtype=float)
    points=np.array([p for f in info['faces'] for p in f['vertices']])
    lo,hi=limits if limits is not None else ((points@direction).min(),(points@direction).max())
    def colour(value):
        t=np.clip((value-lo)/max(hi-lo,1e-9),0,1)
        return np.rint(np.array(light)*(1-t)+np.array(dark)*t).astype(int).tolist()
    for face in info['faces']:
        points=np.array(face['vertices']);values=points@direction
        args=dict(brush=brush['id'],face=face['face'])
        if np.ptp(values)<1e-6:
            solid=colour(values.mean())
            if face.get('paint')!={'color':solid,'alpha':255}:
                ed.call('set_face_color',**args,color=solid)
            continue
        axis=max([2,0,1],key=lambda i:abs(face['normal'][i]))
        uv=points[:,[0,1]]*np.array([1,-1]) if axis==2 else points[:,[1 if axis==0 else 0,2]]*np.array([1,-1])
        coeff=np.linalg.lstsq(uv-uv.mean(axis=0),values-values.mean(),rcond=None)[0]
        assert np.max(np.abs((uv-uv.mean(axis=0))@coeff-(values-values.mean())))<1e-5
        angle=math.degrees(math.atan2(coeff[1],coeff[0]))
        gradient={'from':colour(values.min()),'to':colour(values.max()),'angle':float(np.float32(angle))}
        if face.get('paint')!={'gradient':gradient,'alpha':255}:
            ed.call('set_face_color',**args,gradient=gradient)


def finish(name,palettes,solids=None,world_gradients=None):
    """Apply actual editable gradient metadata via MCP, not a baked texture."""
    solids=r.SOLIDS if solids is None else solids
    ed=Editor()
    try:
        path=f'assets/{name}.map';ed.call('open_map',path=path)
        brushes=json.loads(ed.call('list_brushes'))
        assert len(brushes)==len(solids)
        for brush,(_,colour) in zip(brushes,solids):
            if colour not in palettes:continue
            light,dark=palettes[colour]
            if world_gradients and colour in world_gradients:
                direction,limits=world_gradients[colour]
                paint_world_gradient(ed,brush,light,dark,direction,limits)
            else:
                expected={'gradient':{'from':light,'to':dark,'angle':90},'alpha':255}
                faces=json.loads(ed.call('describe_brush',brush=brush['id']))['faces']
                if not all(face.get('paint')==expected for face in faces):
                    ed.call('select',ids=[brush['id']])
                    ed.call('set_face_color',gradient=expected['gradient'])
        ed.call('save',path=path);ed.call('open_map',path=path)
        ed.call('render_preview',path=f'assets/{name}.png',size=1200)
        ed.call('render_preview',path=f'assets/{name}_editor.png',size=800)
    finally:ed.close()
