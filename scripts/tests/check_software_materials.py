#!/usr/bin/env python3
"""Run production GL and WGPU material regressions on installed Mesa software drivers.

Installs nothing. Both a working EGL context and a CPU Vulkan adapter are required.
"""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--offline', action='store_true', help='Use only cached Cargo dependencies')
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
drivers = sorted(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))
if len(drivers) != 1:
    parser.error('Expected one installed Mesa lavapipe Vulkan ICD; no packages will be installed')
environment = os.environ.copy()
environment.update(LIBGL_ALWAYS_SOFTWARE='1', VK_ICD_FILENAMES=str(drivers[0]),
                   VK_DRIVER_FILES=str(drivers[0]), HERON_REQUIRE_SOFTWARE_GPU='1')
environment.setdefault('CARGO_BUILD_JOBS', '2')
with tempfile.TemporaryDirectory(prefix='heron-software-materials-') as runtime:
    environment['XDG_RUNTIME_DIR'] = runtime
    print('Vulkan software driver:', drivers[0], flush=True)
    subprocess.run([sys.executable, str(root / 'scripts/tests/check_gl_material_shaders.py')],
                   cwd=root, env=environment, check=True)
    command = ['cargo', 'test', '--locked', '-p', 'render_wgpu', '--test', 'headless']
    if args.offline: command.append('--offline')
    command += ['imported_', '--', '--test-threads=1', '--nocapture']
    subprocess.run(command, cwd=root, env=environment, check=True)
print('PASS: software GL and WGPU material regressions', flush=True)
