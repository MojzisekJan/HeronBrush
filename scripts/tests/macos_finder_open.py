#!/usr/bin/env python3
"""Exercise Finder document events against a built macOS bundle (opens a test window).

Build with scripts/install-macos.sh --no-install, then run this script on macOS.
"""
import json, pathlib, subprocess, tempfile, time
root=pathlib.Path(tempfile.mkdtemp(prefix='heron-finder-test-'))
app=str(pathlib.Path(__file__).resolve().parents[2]/'target/macos/Heron Brush.app')
for name, marker in [('první mapa #1.map','finder-cold'),('druhá mapa %.MAP','finder-warm')]:
    (root/name).write_text('{\n"classname" "worldspawn"\n"message" "'+marker+'"\n}\n')
script=root/'input.txt'
script.write_text('wait 3\nsnapshot cold\nwait 8\nsnapshot warm\nquit\n')
for name in ['cold.json','warm.json']:
    (root/name).unlink(missing_ok=True)
subprocess.run(['open','-n','-a',app,'--env',f'HERON_BRUSH_SCRIPT={script}','--env',f'HERON_BRUSH_SHOTS={root}','--stdout',str(root/'stdout.log'),'--stderr',str(root/'stderr.log'),str(root/'první mapa #1.map'),'--args','--new-instance'],check=True)
def wait(name):
    path=root/name
    deadline=time.monotonic()+40
    while not path.exists():
        if time.monotonic()>deadline: raise RuntimeError(f'Timed out: {name}; '+(root/'stderr.log').read_text())
        time.sleep(.2)
    return json.loads(path.read_text())
assert 'finder-cold' in wait('cold.json')['map'], 'Cold launch did not open map'
subprocess.run(['open','-a',app,str(root/'druhá mapa %.MAP')],check=True)
assert 'finder-warm' in wait('warm.json')['map'], 'Running app did not open map'
print('PASS: Finder cold launch and running-app delivery; Unicode, spaces, #, %, uppercase extension')
