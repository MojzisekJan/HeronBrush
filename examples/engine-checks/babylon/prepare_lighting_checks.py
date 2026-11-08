"""Generate material and lighting fixtures through the real editor MCP."""
from pathlib import Path
import sys, tempfile, json, shutil, argparse
repo = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(repo / 'examples/engine-checks'))
from export_feature_lab import Editor, MAP, TEXTURES
parser = argparse.ArgumentParser(description='Export material and lighting fixtures through the real editor MCP.')
parser.add_argument('root', type=Path)
parser.add_argument('--binary', type=Path, default=repo / 'target/debug/heron-brush')
args = parser.parse_args()
root = args.root.resolve()
root.mkdir(parents=True, exist_ok=True)
w, h = (16, 8)
pixels = bytearray()
for y in range(h):
    for x in range(w):
        pixels += bytes([220, 25, 10, 130] if x < w // 2 else [10, 40, 220, 130])
(root / 'colored.hdr').write_bytes(f'#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {h} +X {w}\n'.encode() + pixels)
with tempfile.TemporaryDirectory(prefix='heron-babylon-fixes-') as scratch:
    e = Editor(args.binary.resolve(), Path(scratch))
    try:
        e.call('open_map', path=str(MAP))
        dest = root / 'heron-feature-babylon'
        if dest.exists():
            shutil.rmtree(dest)
        dest.mkdir()
        shutil.copytree(TEXTURES, dest / 'textures', dirs_exist_ok=True)
        e.call('export', engine='babylon', path=str(dest), name='feature_lab', scale=32, options={'project_dir': str(dest), 'texture_dir': 'textures', 'entities': {'fl_probe': {'defaults': {'charge': 9, 'armed': True, 'sequence': [2, 3, 5], 'label': 'probe'}}}})
        for suffix in ['roughness', 'metallic', 'ao', 'emission']:
            shutil.copy2(dest / 'textures/wood.png', dest / f'textures/wood_{suffix}.png')
        e.call('export', engine='babylon', path=str(dest), name='loose', scale=32, options={'project_dir': str(dest), 'texture_dir': 'textures'})
        e.call('select', what='all')
        e.call('set_surface_material', roughness=0.3, metallic=0.6, clearcoat=0.8, coat_roughness=0.2, anisotropy=0.7, anisotropy_rotation=45)
        e.call('set_entity_property', entity='worldspawn', key='_tb_environment', value=json.dumps({'path': str(root / 'colored.hdr'), 'rotation_degrees': 70, 'intensity': 2, 'show_background': False}))
        e.call('set_entity_property', entity='worldspawn', key='_tb_light_probes', value=json.dumps({'probes': [{'position': [0, 0, 96], 'half_size': [2048, 2048, 1024], 'blend': 0.2, 'intensity': 1, 'reflections': True, 'diffuse': True}], 'resolution': 16}))
        camera_text = e.call('add_point_entity', classname='_tb_render_camera', origin=[0, 0, 96], properties={'_tb_render_settings': json.dumps({'imaging': {'exposure_ev': 1, 'temperature': 8000, 'tint': 0.2, 'contrast': 1.3, 'saturation': 0.7, 'tone_map': 'Agx'}})})
        e.call('export', engine='babylon', path=str(dest), name='lighting', scale=32, options={'project_dir': str(dest), 'texture_dir': 'textures'})
        camera_id = int(camera_text.rsplit('id ', 1)[1].rstrip(')'))
        e.call('set_entity_property', entity=str(camera_id), key='_tb_render_settings', value=json.dumps({'imaging': {'tone_map': 'TonyMcMapface'}}))
        e.call('export', engine='babylon', path=str(dest), name='tony', scale=32, options={'project_dir': str(dest), 'texture_dir': 'textures'})
        print('Prepared feature_lab, loose, lighting and Tony LUT exports')
    finally:
        e.close()
