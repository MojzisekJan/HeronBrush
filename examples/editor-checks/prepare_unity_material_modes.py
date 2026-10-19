#!/usr/bin/env python3
"""Prepare an offline Unity alpha-mode check from the six-mode fixture exports."""
import argparse
import json
from pathlib import Path
import struct
import subprocess
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('exports', type=Path)
parser.add_argument('project', type=Path)
args = parser.parse_args()
# Independent GLB data supplies expected per-material rules, rather than parsing
# the generated Unity importer whose behavior the native check is testing.
asset = args.exports / 'Bevy/modes/heron_model.glb'
blob = asset.read_bytes()
size = struct.unpack_from('<I', blob, 12)[0]
scene = json.loads(blob[20:20 + size])
materials = [{'name': item['name'], 'mode': item.get('alphaMode', 'OPAQUE'),
              'cutoff': item.get('alphaCutoff', 0.5), 'doubleSided':item.get('doubleSided',False)} for item in scene['materials']]
if len(materials) != 6 or any(sum(item['mode'] == mode for item in materials) != 2
                            for mode in ['OPAQUE', 'MASK', 'BLEND']):
    parser.error('Expected the six-mode fixture GLB')
here = Path(__file__).parent
subprocess.run([sys.executable, str(here / 'prepare_unity_pbr.py'),
                str(args.exports / 'Unity'), str(args.project),
                '--verifier', str(here / 'verify_unity_material_modes.cs')], check=True)
(args.project / 'Assets/expected_material_modes.json').write_text(json.dumps({'materials': materials}))
