#!/usr/bin/env python3
"""Exercise Flax live replacement and failure recovery in a disposable project."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess

HERE = Path(__file__).resolve().parent
REPOSITORY = HERE.parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True,
                        help='new disposable directory, must not exist')
    args = parser.parse_args()
    root = args.root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    shutil.copy2(HERE / 'flax.flaxproj', root / 'flax.flaxproj')
    shutil.copytree(HERE / 'Source', root / 'Source',
                    ignore=shutil.ignore_patterns('*Check*.cs', 'MotionCheck.cs'))
    (root / 'Content').mkdir()
    shutil.copy2(HERE / 'Content/GameSettings.json', root / 'Content/GameSettings.json')
    shutil.copy2(HERE / 'scene-template.json', root / 'Content/ExportCheck.scene')
    shutil.copy2(HERE / 'LiveCheckPlugin.cs', root / 'Source/Game/LiveCheckPlugin.cs')
    env = dict(os.environ, HERON_FLAX_LIVE_PROJECT=str(root),
               XDG_CONFIG_HOME=str(root / 'test-config'))
    subprocess.run(['cargo', 'test', '-p', 'editor_core', '--lib',
                    'native_flax_live_fixtures', '--', '--ignored', '--nocapture'],
                   cwd=REPOSITORY, env=env, check=True)
    editor = os.environ.get('FLAX_EDITOR', str(Path.home() /
        'Downloads/flax/Binaries/Editor/Linux/Development/FlaxEditor'))
    log = root / 'live-check.log'
    with log.open('w') as output:
        result = subprocess.run([editor, '-headless', '-null', '-project', str(root),
                                 '-std', '-heronlivecheck'], env=env, stdout=output,
                                stderr=subprocess.STDOUT, timeout=240)
    text = log.read_text(errors='replace')
    for line in text.splitlines():
        if 'HERON_LIVE_CHECK' in line:
            print(line)
    if result.returncode or 'HERON_LIVE_CHECK PASS' not in text:
        raise SystemExit(f'Flax live check failed, see {log}')
    print(f'Log: {log}')


if __name__ == '__main__':
    main()
