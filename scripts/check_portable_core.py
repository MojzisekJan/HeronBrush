#!/usr/bin/env python3
"""Check the Android library build and reject desktop UI dependencies in the core."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
CORE = ('geometry_math', 'text_utils', 'expression_lang', 'map_model', 'scene_export', 'render')
# Desktop windowing and UI toolkits (egui/eframe/winit, native dialogs, and the
# GObject-based toolkit libraries), none of which belong in the portable crates.
DESKTOP = {'eframe', 'egui', 'egui-winit', 'egui_glow', 'egui-wgpu', 'egui_extras',
           'winit', 'glutin', 'glutin-winit', 'rfd', 'accesskit_winit', 'x11', 'x11-dl',
           'gtk4', 'gtk4-sys', 'gdk4', 'gdk4-sys', 'glib', 'glib-sys', 'gio', 'gio-sys',
           'cairo-rs', 'pangocairo'}


def check_graph(metadata):
    packages = {p['id']: p for p in metadata['packages']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    roots = [p['id'] for p in metadata['packages']
             if p['id'] in metadata['workspace_members'] and p['name'] in CORE]
    pending, seen = list(roots), set()
    while pending:
        package = pending.pop()
        if package in seen:
            continue
        seen.add(package)
        if packages[package]['name'] in DESKTOP:
            raise RuntimeError(f"Desktop dependency in portable core: {packages[package]['name']}")
        for dependency in nodes[package]['deps']:
            if any(kind['kind'] != 'dev' for kind in dependency['dep_kinds']):
                pending.append(dependency['pkg'])
    if len(roots) != len(CORE):
        raise RuntimeError('Some portable workspace crates were not found')
    return len(seen)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', default='aarch64-linux-android')
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    common = ['--locked', '--features', 'map_model/serde']
    if args.offline:
        common.append('--offline')
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--format-version', '1', '--filter-platform', args.target, *common], cwd=ROOT))
    count = check_graph(metadata)
    print(f'Portable dependency boundary: {count} packages, no desktop UI dependencies.', flush=True)
    command = ['cargo', 'check', '--lib', '--target', args.target, *common]
    for crate in CORE:
        command.extend(['-p', crate])
    subprocess.run(command, cwd=ROOT, check=True)
    print(f'{args.target}: library type-check passed; this does not link an APK or test a GPU.')


if __name__ == '__main__':
    try:
        main()
    except (subprocess.CalledProcessError, RuntimeError) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
