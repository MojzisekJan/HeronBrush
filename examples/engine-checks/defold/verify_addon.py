#!/usr/bin/env python3
"""Verify the Defold addon with Bob and a native Linux engine in Xephyr.

Uses a new disposable project. Checks collider replacement, added/deleted objects,
coalesced reloads, notification timing and a preserved object outside the level.
DEFOLD_HOME selects an installed editor. Requires DISPLAY and Xephyr.
"""
import argparse
import os
from pathlib import Path
import queue
import re
import shutil
import subprocess
import tempfile
import threading
import time
import urllib.request
import zipfile

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]


def reload_resources(port, paths):
    # Resource.Reload has repeated string field 1. These fixture paths fit one-byte lengths.
    encoded = [path.encode() for path in paths]
    assert all(len(path) < 128 for path in encoded)
    body = b''.join(b'\x0a' + bytes([len(path)]) + path for path in encoded)
    request = urllib.request.Request(
        f'http://127.0.0.1:{port}/post/@resource/reload', body, method='POST')
    with urllib.request.urlopen(request, timeout=10) as response:
        assert response.status == 200


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--display', default=':78', help='unused nested X display')
    args = parser.parse_args()
    project = (args.output.resolve() if args.output else
               Path(tempfile.mkdtemp(prefix='heron-defold-addon-')))
    if project.exists() and any(project.iterdir()):
        parser.error('output must be new or empty')
    project.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ['cargo', 'test', '-p', 'editor_core', '--lib', 'native_addon_fixture',
         '--', '--ignored', '--nocapture'], cwd=REPO, check=True,
        env=dict(os.environ, HERON_DEFOLD_ADDON_PROJECT=str(project)))
    for name in ['main.collection', 'check.script', 'marker.go', 'marker.script']:
        shutil.copy2(HERE / 'addon' / name, project / name)
    (project / 'input').mkdir(exist_ok=True)
    (project / 'input/game.input_binding').write_text('')
    sdk = Path(os.environ.get('DEFOLD_HOME', Path.home() / '.local/opt/Defold'))
    jar = next((sdk / 'packages').glob('defold-*.jar'))
    java = next((sdk / 'packages').glob('jdk-*/bin/java'))
    engine_path = project / 'dmengine'
    with zipfile.ZipFile(jar) as archive:
        engine_path.write_bytes(archive.read('libexec/x86_64-linux/dmengine'))
    engine_path.chmod(0o755)

    def build():
        with (project / 'bob.log').open('w') as log:
            subprocess.run(
                [str(java), '--enable-native-access=ALL-UNNAMED', '-cp', str(jar),
                 'com.dynamo.bob.Bob', '--root', str(project), '--max-cpu-threads',
                 '2', 'build'], stdout=log, stderr=subprocess.STDOUT, check=True)

    receipt_log = (project / 'receipt.log').open('w')
    receipt_process = subprocess.Popen(
        ['cargo', 'test', '-p', 'editor_core', '--lib', 'native_runtime_receipt',
         '--', '--ignored', '--nocapture'], cwd=REPO,
        env=dict(os.environ, HERON_DEFOLD_ADDON_PROJECT=str(project)),
        stdout=receipt_log, stderr=subprocess.STDOUT)
    try:
        deadline=time.monotonic()+90
        while 'HERON_RECEIPT_READY' not in (project/'receipt.log').read_text():
            if receipt_process.poll() is not None or time.monotonic()>deadline:
                raise RuntimeError((project/'receipt.log').read_text())
            time.sleep(0.1)
        build()
    except BaseException:
        receipt_process.terminate(); receipt_process.wait(timeout=10); receipt_log.close()
        raise
    collection = project / 'heron_brush_live/preview.collection'
    original_collection = collection.read_text()
    revision = project / 'heron_brush_live/revision.script'

    def bump_revision(prefix):
        revision.write_text(revision.read_text().replace(
            'local revision = ', f'local revision = "{prefix}-" .. '))

    with (project / 'xephyr.log').open('w') as xlog:
        nested_x = subprocess.Popen(
            ['Xephyr', args.display, '-screen', '640x480', '-ac', '-br', '-noreset'],
            stdout=xlog, stderr=subprocess.STDOUT)
        engine = None
        try:
            time.sleep(1)
            if nested_x.poll() is not None:
                raise RuntimeError('Xephyr failed; choose an unused --display')
            engine = subprocess.Popen(
                [str(engine_path), './game.projectc'], cwd=project / 'build/default',
                env=dict(os.environ, DISPLAY=args.display, DM_SERVICE_PORT='dynamic'),
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
            output = queue.Queue()

            def collect():
                for line in engine.stdout:
                    output.put(line)

            threading.Thread(target=collect, daemon=True).start()
            port = None
            deadline = time.monotonic() + 90
            lines = []
            while time.monotonic() < deadline:
                try:
                    line = output.get(timeout=1)
                except queue.Empty:
                    if engine.poll() is not None:
                        break
                    continue
                lines.append(line)
                print(line, end='', flush=True)
                match = re.search(r'Engine service started on port (\d+)', line)
                if match:
                    port = int(match[1])
                if 'HERON_ADDON_READY_FOR_HOT_RELOAD' in line:
                    assert port
                    collider = project / 'heron_brush_live/preview/worldspawn.collisionobject'
                    index = 0

                    def scale_height(match):
                        nonlocal index
                        multiplier = 2 if index % 3 == 1 else 1
                        index += 1
                        return 'data: ' + str(float(match[1]) * multiplier)

                    collider.write_text(re.sub(
                        r'data: ([-0-9.]+)', scale_height, collider.read_text()))
                    collection.write_text(original_collection +
                                          '\ninstances { id: "added" prototype: "/marker.go" }\n')
                    bump_revision('add')
                    build()
                    reload_resources(port, [
                        '/heron_brush_live/preview/worldspawn.collisionobjectc',
                        '/heron_brush_live/revision.scriptc'])
                if 'HERON_ADDON_READY_FOR_REMOVAL' in line:
                    receipt_process.wait(timeout=10)
                    assert receipt_process.returncode==0, (project/'receipt.log').read_text()
                    assert 'HERON_NATIVE_RECEIPT_OK' in (project/'receipt.log').read_text()
                    print('HERON_ENGINE_TO_EDITOR_RECEIPT_OK',flush=True)
                    revision.write_text(re.sub(r'local ack_url = "[^"\n]*"', 'local ack_url = ""',revision.read_text()))
                    collection.write_text(original_collection)
                    bump_revision('remove')
                    build()
                    reload_resources(port, ['/heron_brush_live/revision.scriptc'])
                if 'HERON_ADDON_NATIVE_OK' in line:
                    break
            (project / 'engine.log').write_text(''.join(lines))
            assert any('HERON_ADDON_NATIVE_OK' in line for line in lines), 'native check did not pass'
            assert not any('ERROR:SCRIPT:' in line or 'HERON_ADDON_FAIL ' in line
                           for line in lines), 'runtime error'
            print('Defold addon verification passed:', project)
        finally:
            if engine and engine.poll() is None:
                engine.terminate()
                engine.wait(timeout=10)
            if receipt_process.poll() is None:
                receipt_process.terminate(); receipt_process.wait(timeout=10)
            receipt_log.close()
            nested_x.terminate()
            nested_x.wait(timeout=10)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
