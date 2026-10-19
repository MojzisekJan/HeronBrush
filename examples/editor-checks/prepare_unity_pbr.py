#!/usr/bin/env python3
"""Prepare a new offline Unity project from the PBR fixture's Unity export."""
import argparse
import json
from pathlib import Path
import shutil

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('export', type=Path)
parser.add_argument('project', type=Path)
parser.add_argument('--verifier',type=Path,default=Path(__file__).with_name('verify_unity_pbr.cs'))
args = parser.parse_args()
if not (args.export / 'Editor/HeronBrushLevelImporter.cs').is_file():
    parser.error('Generate fixture exports with HERON_PBR_OUTPUT and the pbr_maps_survive core test first.')
args.project.mkdir(parents=True, exist_ok=False)
shutil.copytree(args.export, args.project / 'Assets/Level')
editor = args.project / 'Assets/Editor'
editor.mkdir()
shutil.copy2(args.verifier, editor / args.verifier.name)
packages = args.project / 'Packages'
packages.mkdir()
(packages / 'manifest.json').write_text(json.dumps({'dependencies': {
    'com.unity.modules.' + name: '1.0.0'
    for name in ['physics', 'imgui', 'jsonserialize', 'imageconversion']
}}, indent=2))
settings = args.project / 'ProjectSettings'
settings.mkdir()
(settings / 'ProjectVersion.txt').write_text('m_EditorVersion: 6000.3.24f1\n')
print(args.project)
