#!/usr/bin/env python3
"""Test production collision broadphase with the bundled Three.js, without npm."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--node', default='node')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='heron-three-collision-') as temporary:
        root = Path(temporary)
        package = root / 'node_modules/three'
        vendor = ROOT / 'crates/heron-brush-web/vendor/three'
        for name in ['build/three.module.js', 'build/three.core.js',
                     'examples/jsm/math/Octree.js', 'examples/jsm/math/Capsule.js']:
            target = package / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(vendor / name, target)
        (package / 'package.json').write_text(json.dumps({
            'type': 'module', 'exports': {'.': './build/three.module.js',
                                        './addons/*': './examples/jsm/*'}}))
        shutil.copy2(ROOT / 'crates/scene_export/src/writer/three_collision.js', root / 'collision.mjs')
        shutil.copy2(ROOT / 'scripts/tests/three-collision.test.mjs', root / 'check.mjs')
        subprocess.run([args.node, '--max-old-space-size=384', str(root / 'check.mjs')],
                       check=True, timeout=60)


if __name__ == '__main__':
    main()
