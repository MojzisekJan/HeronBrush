"""Curated physical surfaces for the bundled brush library.

Palettes are scoped to each asset, not global colour guesses: a gold animal beak
is not brass. Match a whole brush's mean paint so gradients keep one surface.
Only roughness/metallic change; geometry, UVs, alpha and authored emission survive.
Run from the repository root: python3 assets/asset_materials.py
"""
import argparse
from collections import Counter
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parent
# Stored percentages. Coatings, glass and stone are dielectrics even on a metal substrate.
SURFACES={
    'stone':(88,0), 'marble':(32,0), 'jade':(25,0), 'ceramic':(24,0),
    'wood':(74,0), 'varnish':(40,0), 'paper':(90,0), 'cloth':(92,0),
    'organic':(82,0), 'rubber':(88,0), 'paint':(32,0),
    'steel':(36,100), 'iron':(58,100), 'chrome':(16,100), 'brass':(30,100),
    'glass':(10,0), 'water':(12,0), 'crystal':(12,0),
}
PROFILES={}
def profile(names, default, **palettes):
    for name in names.split():
        PROFILES[name]=(default,[(colour,kind) for kind,colours in palettes.items() for colour in colours])

profile('tree_acacia tree_birch tree_elm tree_maple tree_oak tree_palm tree_pine shrub bush_flowering bamboo_clump cactus','organic')
profile('fallen_log stump','wood')
profile('donkey dragon monkey heron beaver steve','organic')
profile('bridge pillar rock_boulder factory_chimney','stone')
profile('column_classical','marble',stone=[(186,171,143)])
profile('column_lotus','jade',brass=[(201,158,73)])
profile('column_temple','stone',jade=[(52,118,121)])
profile('column_atlantis','jade',brass=[(209,169,89)],crystal=[(69,197,198)])
profile('crystal_cluster','crystal',stone=[(71,78,96)])
profile('fountain','stone',water=[(57,151,194)])
profile('bench crate','wood',iron=[(70,85,91)],brass=[(214,172,88)])
profile('chest','wood',iron=[(62,72,77)],brass=[(198,151,68)])
profile('bookshelf','paper',wood=[(151,95,52),(101,62,37),(123,78,45)],
        brass=[(207,164,81)],paper=[(39,112,111),(162,48,42),(90,102,130),(176,133,61),(107,79,109),(226,213,180)])
profile('lamp','iron',brass=[(214,172,88)],glass=[(249,209,124)])
profile('gate_open','stone',wood=[(146,94,53)],iron=[(65,80,85)],brass=[(190,148,73)],stone=[(152,165,163),(180,188,175)])
profile('gate_paifang','varnish',varnish=[(189,57,35)],ceramic=[(49,118,103)],stone=[(68,73,70),(116,121,115)],brass=[(185,145,75)])
profile('lantern_red','cloth',cloth=[(194,49,42)],brass=[(205,162,76)],wood=[(81,62,44)])
profile('house well','stone',wood=[(139,89,48)],ceramic=[(165,65,46)],glass=[(95,165,184)],iron=[(57,68,73)],stone=[(225,208,167),(172,179,172)])
profile('windmill','wood',stone=[(131,137,126),(224,209,174)],wood=[(102,65,42),(151,100,54)],
        ceramic=[(44,82,89)],cloth=[(247,230,186)],glass=[(80,147,157)],brass=[(196,146,69)])
profile('ishtar_gate','ceramic',stone=[(208,193,149)],ceramic=[(35,85,159),(25,61,112),(219,175,79),(55,155,171)])
profile('portal_frame','stone',stone=[(163,169,151),(183,185,159),(75,95,88)],brass=[(153,123,73)],crystal=[(46,208,190)])
profile('portal_gate','stone',stone=[(53,72,84),(131,126,113),(106,105,98)],brass=[(177,127,70)],crystal=[(250,179,65)])
profile('portal_ring','steel',iron=[(46,62,76)],steel=[(100,127,143),(155,162,161)],crystal=[(100,177,238)])
profile('portal_marble','marble',marble=[(225,220,201),(192,189,170)],brass=[(171,124,65)],jade=[(111,146,139)],crystal=[(65,190,181)])
profile('barrel_blue barrel_red barrel_yellow barrel_green','paint',steel=[(120,135,140)],iron=[(72,85,91)],
        paint=[(52,113,162),(180,60,46),(211,166,61)])
profile('shipping_container','paint',paint=[(67,113,139),(77,126,151),(48,79,95)],steel=[(174,183,174)])
profile('pipe_straight pipe_elbow pipe_tee pipe_reducer pipe_valve pipe_broken','paint',paint=[(71,125,134),(187,64,45)],steel=[(143,166,170)],iron=[(57,72,79)])
profile('industrial_piston','paint',paint=[(197,145,62),(176,67,44)],chrome=[(176,192,195)],iron=[(54,73,79)])
profile('gear_machine','paint',paint=[(62,112,112)],iron=[(48,62,67)],steel=[(165,186,188)],
        brass=[(204,155,74)],glass=[(182,55,41),(100,187,124),(228,164,49)])
profile('radar','paint',paint=[(71,100,104),(191,209,194)],iron=[(49,61,64)],brass=[(210,157,66)],stone=[(139,150,147)])
profile('fan','paint',paint=[(48,106,113),(225,209,164)],steel=[(58,67,75)],brass=[(219,124,53)])
profile('hydrant','paint',paint=[(191,48,38)],iron=[(73,85,93)],brass=[(200,165,108)])
profile('utility_pole','wood',wood=[(160,117,72)],steel=[(84,104,108),(139,163,164)],ceramic=[(187,220,209)],stone=[(149,151,143)])
profile('utility_pole_metal','steel',steel=[(153,172,181),(119,145,158)],ceramic=[(112,170,174)],stone=[(151,152,143)])
profile('crane','paint',paint=[(223,166,43)],iron=[(57,68,73)],glass=[(95,165,184)],stone=[(172,179,172)])
profile('car motorcycle','paint',paint=[(51,119,154),(180,61,44),(202,44,36)],
        rubber=[(36,42,46)],steel=[(60,73,79),(155,169,171)],glass=[(103,177,194),(242,223,169)])
profile('car_retro_compact car_retro_sedan','paint',paint=[(103,157,128),(228,218,180),(155,43,32),(154,70,53)],
        rubber=[(36,42,43)],chrome=[(186,195,187)],glass=[(91,151,165),(252,230,160)])
profile('airliner fighter_jet locomotive_diesel steamship','paint',
        paint=[(222,230,224),(46,115,174),(176,62,46),(228,211,172),(88,118,112)],
        rubber=[(42,48,54)],steel=[(133,150,154)],glass=[(39,78,97)],wood=[(147,101,60)])
# Train wheels are steel, unlike aircraft tyres. The ship's dark parts are metal fittings.
for name in ('locomotive_diesel','steamship'):
    default,palette=PROFILES[name]
    PROFILES[name]=(default,[(c,'iron' if k=='rubber' else k) for c,k in palette])
profile('fire_engine','paint',paint=[(194,47,39),(237,224,190)],rubber=[(43,49,53)],
        steel=[(166,184,187)],glass=[(69,130,155),(47,135,232)])
profile('submarine','paint',paint=[(204,155,51)],iron=[(49,67,77)],steel=[(157,173,173)],brass=[(181,133,67)],glass=[(65,155,181)])
profile('airship','cloth',cloth=[(221,204,163),(229,214,177),(39,112,111)],
        wood=[(151,95,52)],varnish=[(226,213,180)],glass=[(79,148,168)],iron=[(49,62,66)])
profile('plane','paint',paint=[(214,179,74),(162,48,42)],glass=[(79,148,168)],iron=[(49,62,66)],brass=[(207,164,81)])
profile('tram','paint',iron=[(49,62,66),(74,81,77)],glass=[(79,148,168)],paint=[(162,48,42),(226,213,180),(39,112,111)])
profile('sailboat','wood',wood=[(135,87,47),(181,141,87)],cloth=[(239,226,185),(151,128,87),(221,208,162)],
        paint=[(47,115,129),(170,67,47)],iron=[(67,65,56)])
profile('boat','varnish',varnish=[(156,74,58),(161,65,49),(232,228,214)],wood=[(190,156,108),(140,102,66),(112,73,44)],
        glass=[(150,196,214)],iron=[(70,85,91)],cloth=[(225,222,201)])
profile('truck','paint',paint=[(196,60,52),(151,43,37)],rubber=[(56,56,60)],iron=[(72,74,82),(70,85,91)],
        steel=[(140,146,152),(171,182,181),(163,172,176)],brass=[(214,172,88)],wood=[(166,112,65)],glass=[(150,196,214),(133,195,207),(255,244,180)])
profile('locomotive','paint',paint=[(134,162,194),(150,176,206),(155,65,52)],iron=[(92,92,100),(96,96,104),(70,85,91)],
        brass=[(224,186,96),(214,172,88)],glass=[(150,196,214),(133,195,207),(249,217,143)])

profile('chair_dining table_dining','varnish',varnish=[(174,119,70),(111,72,47)],paint=[(60,124,127)])
profile('refrigerator vending_snacks','paint',paint=[(60,124,127),(229,221,192)],steel=[(155,172,173)],rubber=[(44,56,62)],paper=[(197,75,51),(223,173,73)],glass=[(85,153,179)])
profile('street_lamp','iron',iron=[(50,65,71)],brass=[(183,141,76)],glass=[(250,218,151)])
profile('pallet_wood','wood')
profile('tree_cypress','organic',organic=[(52,108,71),(65,126,78),(36,85,64)],wood=[(108,78,48)])

profile('door_wooden door_industrial door_double','varnish',varnish=[(174,119,70),(111,72,47)],paint=[(60,124,127),(229,221,192),(204,137,62)],steel=[(155,172,173)],iron=[(44,56,62)],brass=[(196,151,75)])

profile('wall_brick_normal','stone')
profile('sofa armchair bed_single desk_office locker_metal toilet sink_pedestal traffic_cone bollard trash_bin',
        'paint', varnish=[(151,101,62)], cloth=[(63,119,126),(201,153,86),(230,223,202)],
        ceramic=[(228,237,231)], rubber=[(43,54,59)], steel=[(155,175,182)],
        paint=[(75,116,133),(224,100,42)])
# Cream paint on street furniture and ceramic cistern lid are hard surfaces.
profile('bollard', 'paint', paint=[(75,116,133),(230,223,202)], rubber=[(43,54,59)], steel=[(155,175,182)])
profile('toilet', 'ceramic', ceramic=[(228,237,231),(230,223,202)], steel=[(155,175,182)])
profile('bed_single', 'varnish', varnish=[(151,101,62)], cloth=[(63,119,126),(230,223,202),(228,237,231)])
profile('desk_office locker_metal', 'paint', varnish=[(151,101,62)], paint=[(75,116,133),(230,223,202)], rubber=[(43,54,59)], steel=[(155,175,182)])
profile('traffic_cone', 'paint', paint=[(224,100,42),(228,237,231)], rubber=[(43,54,59)])

# Anchors for the default surface participate in the same comparison as exceptions.
for name, kind, colours in [
    ('bench','wood',[(166,112,65),(112,73,44)]),
    ('crate','wood',[(166,112,65),(112,73,44)]),
    ('chest','wood',[(139,77,39),(65,39,27)]),
    ('lamp','iron',[(70,85,91)]),
    ('column_classical','marble',[(220,210,186),(232,224,202)]),
    ('column_lotus','jade',[(56,124,118),(72,143,133)]),
    ('column_temple','stone',[(157,116,77),(91,76,62),(204,164,105)]),
    ('column_atlantis','jade',[(47,114,116),(157,189,184),(136,167,166)]),
    ('fountain','stone',[(163,175,174)]),
    ('crystal_cluster','crystal',[(64,119,221),(54,192,201),(153,90,214)]),
    ('tram','brass',[(207,164,81)]),
]:
    PROFILES[name][1].extend((colour,kind) for colour in colours)

PAINT=re.compile(r'// heron_brush (color|gradient) ([\d. -]+)')
PBR=re.compile(r'\bpbr\s+\d+\s+\d+(?:\s+\d+)?')
FACE=re.compile(r'^\s*\([^)]*\)\s*\([^)]*\)\s*\([^)]*\)\s+(\S+)')
HELPERS={'origin','skip','clip','nodraw','caulk'}

def colours(line):
    match=PAINT.search(line)
    if not match:return []
    numbers=list(map(float,match[2].split()))
    return [numbers[:3]] if match[1]=='color' else [numbers[:3],numbers[3:6]]

def brushes(lines):
    header=[]; faces=[]; painted=[]
    for i,line in enumerate(lines):
        if FACE.match(line):
            faces.append(i); painted.extend(colours(line) or header)
        elif line.strip()=='}':
            if faces:yield faces,painted
            header=[];faces=[];painted=[]
        elif line.startswith('// heron_brush color ') or line.startswith('// heron_brush gradient '):
            header=colours(line)

def apply_file(path, write=True):
    path=Path(path).resolve()
    if path.parent!=ROOT or path.suffix!='.map' or path.stem not in PROFILES:return Counter()
    original=path.read_text(); lines=original.splitlines(keepends=True)
    default,palette=PROFILES[path.stem]; counts=Counter()
    all_points=[list(map(float,point.split())) for line in lines if FACE.match(line)
                for point in re.findall(r'\(([^)]*)\)',line)[:3]]
    height=max((p[2] for p in all_points),default=1)-min((p[2] for p in all_points),default=0)
    for faces,paint in brushes(lines):
        kind=default
        if paint and palette:
            mean=[sum(c[i] for c in paint)/len(paint) for i in range(3)]
            nearest=min(palette,key=lambda item:sum((a-b)**2 for a,b in zip(mean,item[0])))
            kind=nearest[1]
        # These generators reuse dark grey for tyres and metal intakes/chassis.
        # Their wheel silhouettes and height separate the parts without fragile brush IDs.
        if kind=='rubber' and path.stem in ('fire_engine','airliner','fighter_jet'):
            points=[list(map(float,p.split())) for i in faces for p in re.findall(r'\(([^)]*)\)',lines[i])[:3]]
            spans=[max(p[k] for p in points)-min(p[k] for p in points) for k in range(3)]
            ceiling={'fire_engine':.36,'airliner':.13,'fighter_jet':.2}[path.stem]*height
            if (max(p[2] for p in points)>ceiling or spans[0]<height*.08
                    or not .75 < spans[0]/max(spans[2],1e-6) < 1.25):
                kind='iron'
        roughness,metallic=SURFACES[kind]
        visible=False
        for i in faces:
            if FACE.match(lines[i])[1].lower().rsplit('/',1)[-1] in HELPERS:continue
            visible=True
            match=PBR.search(lines[i])
            emission=match[0].split()[3] if match and len(match[0].split())>3 else '0'
            value=f'pbr {roughness} {metallic} {emission}'
            if match:lines[i]=lines[i][:match.start()]+value+lines[i][match.end():]
            else:lines[i]=lines[i].rstrip('\n')+(' ' if '// heron_brush ' in lines[i] else ' // heron_brush ')+value+'\n'
        if visible:counts[kind]+=1
    result=''.join(lines)
    if write and result!=original:path.write_text(result)
    return counts

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true',help='Print assignments without writing files')
    args=parser.parse_args()
    paths=sorted(ROOT.glob('*.map'))
    unknown=[p.stem for p in paths if p.stem not in PROFILES]
    if unknown:raise SystemExit(f'No surface profile: {unknown}')
    for path in paths:print(path.stem,dict(apply_file(path,write=not args.check)))
