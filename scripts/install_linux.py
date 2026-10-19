#!/usr/bin/env python3
"""Install a versioned Linux release for the current user, with assets and plugins.

Build first with `cargo build --release -p heron-brush`.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
APP_ID = 'io.github.mojzisekjan.HeronBrush'


def replace_link(link, target):
    if link.exists() and not link.is_symlink():
        raise RuntimeError(f'Refusing to replace a regular file or directory: {link}')
    link.parent.mkdir(parents=True, exist_ok=True)
    temporary = link.with_name(link.name + '.new')
    if temporary.exists() or temporary.is_symlink():
        raise RuntimeError(f'Temporary link already exists: {temporary}')
    temporary.symlink_to(target)
    temporary.replace(link)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prefix', type=Path, default=Path.home()/'.local/opt/heron-brush')
    parser.add_argument('--bin-dir', type=Path, default=Path.home()/'.local/bin')
    parser.add_argument('--data-dir', type=Path, default=Path(os.environ.get('XDG_DATA_HOME', str(Path.home()/'.local/share'))))
    parser.add_argument('--default-map-handler', action='store_true')
    args = parser.parse_args()
    version = tomllib.loads((ROOT/'Cargo.toml').read_text())['workspace']['package']['version']
    binary = ROOT/'target/release/heron-brush'
    actual_version = subprocess.check_output([str(binary), '--version'], text=True).strip()
    if actual_version.split(' (')[0] != f'Heron Brush {version}':
        raise RuntimeError(f'Rebuild the release first: {actual_version}')
    commit = subprocess.check_output(['git','rev-parse','HEAD'], cwd=ROOT, text=True).strip()
    dirty = bool(subprocess.check_output(['git','status','--porcelain'], cwd=ROOT, text=True))
    stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    release_id = f'{version}+{stamp}.{commit[:8]}' + ('.dirty' if dirty else '')
    release = args.prefix/'releases'/release_id
    current = args.prefix/'current'
    command = args.bin_dir/'heron-brush'
    for link in (current, command):
        if link.exists() and not link.is_symlink():
            raise RuntimeError(f'Refusing to replace: {link}')
    release.mkdir(parents=True)
    shutil.copy2(binary, release/'heron-brush')
    assets = release/'assets'
    assets.mkdir()
    for source in (ROOT/'assets').iterdir():
        if source.is_file() and source.suffix.lower() in {'.map','.png','.jpg','.jpeg','.md'}:
            shutil.copy2(source, assets/source.name)
    for directory in ('plugins','textures'):
        shutil.copytree(ROOT/'assets'/directory, assets/directory)
    # Game configurations shipped with the editor, found as `games` beside the binary.
    if (ROOT/'gamedata').is_dir():
        shutil.copytree(ROOT/'gamedata', release/'games')
    shutil.copy2(ROOT/'LICENSE', release/'LICENSE')
    # Run from outside the checkout so the smoke check cannot rely on its CWD.
    requests = [
        {'jsonrpc':'2.0','id':1,'method':'initialize','params':{}},
        {'jsonrpc':'2.0','id':2,'method':'tools/call','params':{'name':'list_plugins','arguments':{}}},
    ]
    check = subprocess.run([str(release/'heron-brush'),'--mcp'],
        input=''.join(json.dumps(r)+'\n' for r in requests), text=True,
        capture_output=True, cwd=release, timeout=60, check=True)
    responses = [json.loads(line) for line in check.stdout.splitlines() if line.startswith('{')]
    plugins = next(r for r in responses if r.get('id')==2)
    if plugins.get('error') or plugins.get('result',{}).get('isError') or 'selection-inspector' not in json.dumps(plugins):
        raise RuntimeError(f'Packaged plugin discovery failed: {plugins}')
    expected = {p.name for p in (ROOT/'assets').glob('*.map')}
    assert expected and expected == {p.name for p in assets.glob('*.map')}
    hashes = {str(p.relative_to(release)):hashlib.sha256(p.read_bytes()).hexdigest()
              for p in sorted(release.rglob('*')) if p.is_file()}
    (release/'BUILD.json').write_text(json.dumps({
        'version':version,'build':release_id,'source_commit':commit,
        'includes_uncommitted_changes':dirty,'profile':'release',
        'features':'default (LAN collaboration)', 'asset_count':len(expected),
        'sha256':hashes}, indent=2)+'\n')
    replace_link(current, release)
    replace_link(command, current/'heron-brush')
    subprocess.run([sys.executable,str(ROOT/'scripts/install_desktop.py'),
        '--binary',str(current/'heron-brush'),'--data-dir',str(args.data_dir)],check=True)
    desktop = args.data_dir/'applications'/(APP_ID+'.desktop')
    if shutil.which('desktop-file-validate'):
        subprocess.run(['desktop-file-validate',str(desktop)],check=True)
    if args.default_map_handler:
        subprocess.run(['xdg-mime','default',APP_ID+'.desktop','text/x-quakemap'],check=True)
    # Migrate only this checkout's asset path, retaining all custom library choices.
    config_home = Path(os.environ.get('XDG_CONFIG_HOME', str(Path.home()/'.config')))
    preferences = config_home/'heron-brush/preferences'
    if preferences.is_file():
        old = preferences.read_text()
        old_line = 'ui/Asset library=' + str(ROOT/'assets')
        lines = old.splitlines(keepends=True)
        updated = ''.join('ui/Asset library=' + str(current/'assets') + '\n'
                          if line.rstrip('\r\n') == old_line else line for line in lines)
        if updated != old:
            shutil.copy2(preferences, preferences.with_name('preferences.before-install-' + stamp))
            preferences.write_text(updated)
    print(f'Installed {release}\nCommand: {command}\nAssets: {len(expected)}')


if __name__ == '__main__':
    main()
