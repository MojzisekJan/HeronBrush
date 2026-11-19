#!/usr/bin/env python3
"""Compare two prebuilt material_load_probe binaries using disposable PNG fixtures."""
import argparse
import json
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--runs', type=int, default=7, choices=range(1, 32))
    args = parser.parse_args()
    pixel = (Path(__file__).resolve().parents[2] / 'assets/textures/special/clip.png').read_bytes()
    result = {'platform': sys.platform, 'order': ['before', 'after', 'after', 'before'],
              'runs_per_process': args.runs, 'cases': []}
    with tempfile.TemporaryDirectory(prefix='heron-material-bench-') as scratch:
        for paired in [False, True]:
            name = 'paired' if paired else 'plain'
            folder = Path(scratch) / name
            folder.mkdir()
            for index in range(512):
                (folder / f'stone_{index:04}_albedo.png').write_bytes(pixel)
                if paired:
                    (folder / f'stone_{index:04}_normal.png').write_bytes(pixel)
            samples = {'before': [], 'after': []}
            for variant in result['order']:
                completed = subprocess.run(
                    ['nice', '-n', '10', str(getattr(args, variant).resolve()), str(folder), str(args.runs)],
                    check=True, capture_output=True, text=True, timeout=60)
                data = json.loads(completed.stdout)
                assert data['materials'] == 512
                assert data['normals'] == (512 if paired else 0)
                samples[variant].extend(data['samples_ms'])
            medians = {key: statistics.median(values) for key, values in samples.items()}
            result['cases'].append({'fixture': name, 'samples_ms': samples, 'medians_ms': medians})
            print(f'{name}: {medians["before"]:.3f} -> {medians["after"]:.3f} ms', flush=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
