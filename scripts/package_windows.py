"""Bundle the MSVC release of Heron Brush with its assets, then smoke test it.

The release workflow links the C runtime statically and OpenGL comes from the
graphics driver, so the package is the executable, the assets and the licence.
"""
from pathlib import Path
import json, os, shutil, subprocess

root = Path(__file__).resolve().parents[1]
bundle = root / 'target/windows-package/HeronBrush'
if bundle.exists():
    shutil.rmtree(bundle)
bundle.mkdir(parents=True)
exe = bundle / 'heron-brush.exe'
shutil.copy2(root / 'target/release/heron-brush.exe', exe)
# Editable source assets accompany the compiled-in starter plugins.
assets = bundle / 'assets'
assets.mkdir()
for source in (root / 'assets').iterdir():
    if source.is_file() and source.suffix.lower() in {'.map', '.png', '.jpg', '.jpeg', '.md'}:
        shutil.copy2(source, assets / source.name)
shutil.copytree(root / 'assets/plugins', assets / 'plugins')
shutil.copytree(root / 'assets/textures', assets / 'textures')
# Game configurations shipped with the editor, found as `games` beside the binary.
shutil.copytree(root / 'gamedata', bundle / 'games')
source_maps = {p.name for p in (root / 'assets').glob('*.map')}
assert source_maps and {p.name for p in assets.glob('*.map')} == source_maps
shutil.copy2(root / 'LICENSE', bundle / 'LICENSE')
commit = os.environ.get('GITHUB_SHA') or subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
(bundle / 'BUILD.txt').write_text(f'Heron Brush Windows x64 release (MSVC)\nCommit: {commit}\nDefault features: LAN collaboration; internet iroh feature disabled.\nSource: https://github.com/MojzisekJan/HeronBrush/tree/{commit}\nExtract the entire folder before running heron-brush.exe.\n', encoding='utf-8')
# Check the shipped binary with only the bundle and System32 on PATH.
env = os.environ.copy()
env['PATH'] = str(bundle) + ';' + os.environ.get('SYSTEMROOT', r'C:\Windows') + r'\System32'
version = subprocess.run([str(exe), '--version'], text=True, capture_output=True, timeout=30, env=env)
if version.returncode or not version.stdout.startswith('Heron Brush '):
    raise RuntimeError(f'Packaged --version failed: {version.returncode} {version.stdout} {version.stderr}')
print(version.stdout.strip())
request = '\n'.join(json.dumps(r) for r in [{'jsonrpc': '2.0', 'id': 1, 'method': 'tools/list'}, {'jsonrpc': '2.0', 'id': 2, 'method': 'tools/call', 'params': {'name': 'list_plugins', 'arguments': {}}}]) + '\n'
# Run from the bundle so plugin discovery sees the packaged assets, not the checkout.
result = subprocess.run([str(exe), '--mcp'], input=request, text=True, capture_output=True, timeout=60, env=env, cwd=bundle)
print(result.stderr)
if result.returncode:
    raise RuntimeError(f'Packaged MCP startup failed: {result.returncode}')
responses = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
assert any(r.get('id') == 1 and 'result' in r for r in responses), result.stdout
print('MCP smoke test passed.')
assert any(r.get('id') == 2 and 'selection-inspector' in json.dumps(r) and not r.get('result', {}).get('isError', False) for r in responses), responses
print(f'{len(source_maps)} asset maps packaged; bundled plugin discovery passed.')
