#!/usr/bin/env python3
"""Verify Heron -> Defold editor -> running game -> Heron on a fresh Linux project.

Requires Defold and Xephyr. The project is disposable; logs remain in --output.
This catches editor build dependencies that a Bob build of all resources can hide.
"""
import argparse
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import tempfile
import time
import urllib.request

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--display', default=':81')
    args = parser.parse_args()
    project = (args.output.resolve() if args.output else
               Path(tempfile.mkdtemp(prefix='heron-defold-editor-')))
    if project.exists() and any(project.iterdir()):
        parser.error('output must be new or empty')
    project.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ['cargo', 'test', '-p', 'editor_core', '--lib', 'native_addon_fixture',
         '--', '--ignored', '--nocapture'], cwd=REPO, check=True,
        env=dict(os.environ, HERON_DEFOLD_ADDON_PROJECT=str(project)))
    (project / 'input').mkdir()
    (project / 'input/game.input_binding').write_text('')
    (project / 'main.collection').write_text((HERE / 'addon/main.collection').read_text())
    (project / 'check.script').write_text('''function on_message(self,id,message,sender)
 if id==hash("heron_level_loaded") then print("HERON_EDITOR_LEVEL_READY "..message.generation) end
 if id==hash("heron_level_failed") then print("HERON_EDITOR_LEVEL_FAILED "..message.reason) end
end
''')
    sdk = Path(os.environ.get('DEFOLD_HOME', Path.home() / '.local/opt/Defold'))
    with (project / 'editor.log').open('w') as elog, (project / 'xephyr.log').open('w') as xlog:
        nested = subprocess.Popen(
            ['Xephyr', args.display, '-screen', '960x720', '-ac', '-br', '-noreset'],
            stdout=xlog, stderr=subprocess.STDOUT)
        editor = None
        target = None
        try:
            time.sleep(1)
            if nested.poll() is not None:
                raise RuntimeError('Xephyr failed; choose an unused --display')
            editor = subprocess.Popen(
                [str(sdk / 'Defold'), str(project / 'game.project')],
                env=dict(os.environ, DISPLAY=args.display), stdout=elog,
                stderr=subprocess.STDOUT, start_new_session=True)

            def request(path, body=None):
                port = (project / '.internal/editor.port').read_text().strip()
                token = (project / '.internal/editor.token').read_text().strip()
                req = urllib.request.Request(
                    'http://127.0.0.1:' + port + path, body,
                    headers={'Authorization': 'Bearer ' + token, 'Content-Type': 'text/plain'},
                    method='POST' if body is not None else 'GET')
                with urllib.request.urlopen(req, timeout=90) as response:
                    return response.read()

            deadline = time.monotonic() + 180
            while True:
                try:
                    request('/openapi.json')
                    break
                except (OSError, ValueError):
                    if editor.poll() is not None or time.monotonic() > deadline:
                        raise RuntimeError('Editor did not start; see editor.log')
                    time.sleep(1)
            for command in ['compile', 'run']:
                result = json.loads(request('/command/' + command, b''))
                (project / f'{command}.json').write_text(json.dumps(result, indent=2))
                assert result.get('success'), result
                if command == 'run':
                    target = result.get('target', {}).get('url')
            time.sleep(3)
            result = subprocess.run(
                ['cargo', 'test', '-p', 'editor_core', '--lib', 'native_editor_refresh',
                 '--', '--ignored', '--nocapture'], cwd=REPO, capture_output=True,
                text=True, timeout=90,
                env=dict(os.environ, HERON_DEFOLD_LIVE_PROJECT=str(project),
                         HERON_DEFOLD_HOT_RELOAD='1'))
            (project / 'refresh.log').write_text(result.stdout + result.stderr)
            print(result.stdout, end='', flush=True)
            assert result.returncode == 0, result.stderr
            generation = re.search(r'runtime confirmed level generation (\d+)', result.stdout)
            assert generation and int(generation[1]) >= 2, 'Initial load or replacement did not complete'
            print('HERON_EDITOR_RUNTIME_ROUNDTRIP_OK', project)
        finally:
            if target and target.startswith('http://127.0.0.1:'):
                try:
                    req = urllib.request.Request(target.rstrip('/') + '/post/@system/exit',
                                                 b'\x08\x00', method='POST')
                    urllib.request.urlopen(req, timeout=2).close()
                except OSError:
                    pass
            if editor and editor.poll() is None:
                os.killpg(editor.pid, signal.SIGTERM)
                editor.wait(timeout=15)
                # Let the JVM release its GL context before taking away the X server.
                time.sleep(1)
            nested.terminate()
            nested.wait(timeout=10)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
