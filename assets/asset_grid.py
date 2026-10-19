"""Keep generated library maps on the editor's 0.125-unit lattice."""
import subprocess
from pathlib import Path
ROOT = Path(__file__).resolve().parent

def snap_file(editor, path):
    path = Path(path).resolve()
    if path.parent != ROOT or path.suffix != '.map':
        return
    binary = ROOT.parent / 'target/debug/examples/snap_asset_grid'
    if not binary.exists():
        subprocess.run(['cargo', 'build', '-p', 'map_model', '--offline', '--example', 'snap_asset_grid'], cwd=ROOT.parent, check=True)
    result = subprocess.run([str(binary), str(path), '--write'], cwd=ROOT.parent, capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(f'{path.name}: grid validation failed\n{result.stdout}\n{result.stderr}')
    editor.call('open_map', path=str(path))
