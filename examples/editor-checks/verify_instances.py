#!/usr/bin/env python3
"""Generate exported instances offline and verify them in the installed Godot."""
import argparse, os, shutil, subprocess, tempfile
from pathlib import Path
repo = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument('--root', type=Path, help='New or empty output directory')
args = parser.parse_args()
root = args.root.resolve() if args.root else Path(tempfile.mkdtemp(prefix='heron-instance-'))
if args.root and root.exists() and any(root.iterdir()):
    parser.error('--root must be new or empty so stale assets cannot hide export errors')
root.mkdir(parents=True, exist_ok=True)
godot = shutil.which('godot') or shutil.which('godot4')
if not godot:
    parser.error('Godot is not installed; no installation will be attempted')
def run(command, log, env):
    with (root/log).open('w') as output:
        result = subprocess.run(command, cwd=repo, env=env, stdout=output, stderr=subprocess.STDOUT, timeout=240)
    if result.returncode:
        raise RuntimeError(f'Command failed: {command[0]}; see {root/log}')
run(['cargo','test','-p','editor_core','--lib','--offline','repeated_models_share_assets','--','--nocapture'], 'fixture.log', {**os.environ, 'HERON_INSTANCE_OUTPUT':str(root)})
project = root/'Godot'
shutil.copyfile(repo/'examples/editor-checks/verify_instances.gd', project/'verify_instances.gd')
env = {**os.environ,'XDG_CONFIG_HOME':str(root/'config'),'XDG_DATA_HOME':str(root/'data')}
run([godot,'--headless','--editor','--path',str(project),'--import','--quit'], 'import.log', env)
run([godot,'--headless','--path',str(project),'--script','verify_instances.gd'], 'godot.log', env)
assert 'PASS:' in (root/'godot.log').read_text()
print((root/'godot.log').read_text().strip())
print('Logs and exported fixture:',root)
