"""Keep reviewed rectangular gunwale corners mitred when regenerating the boat."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent


def join_file(editor, path):
    path = Path(path).resolve()
    if path.parent != ROOT or path.name != 'boat.map':
        return
    binary = ROOT.parent / 'target/release/examples/repair_asset_corners'
    if not binary.exists():
        subprocess.run(['cargo', 'build', '--release', '--offline', '--locked', '-j', '1',
                        '-p', 'map_model', '--example', 'repair_asset_corners'], cwd=ROOT.parent, check=True)
    with tempfile.TemporaryDirectory(prefix='heron-asset-corners-') as directory:
        output = Path(directory) / path.name
        result = subprocess.run([str(binary), str(path), str(output)], check=True,
                                capture_output=True, text=True)
        report = json.loads(result.stdout)
        if report.get('union_preserved') is not True:
            raise RuntimeError('Corner repair did not validate the original solid union')
        if report['changed_brushes']:
            # Keep replacement on the same filesystem as the asset.
            descriptor, pending = tempfile.mkstemp(prefix='.corner-', suffix='.map', dir=ROOT)
            try:
                with os.fdopen(descriptor, 'wb') as stream:
                    stream.write(output.read_bytes())
                os.chmod(pending, path.stat().st_mode & 0o777)
                os.replace(pending, path)
            finally:
                Path(pending).unlink(missing_ok=True)
            editor.call('open_map', path=str(path))
