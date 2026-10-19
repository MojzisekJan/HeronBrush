#!/usr/bin/env python3
"""Report an owned Linux executable's ELF dependencies and minimum glibc symbols.

This does not prove GPU-driver, desktop-session or cross-distribution compatibility.
Use only with the editor binary you built: ldd may execute an ELF interpreter.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def audit(binary):
    dependencies = subprocess.check_output(['ldd', str(binary)], text=True)
    versions = subprocess.check_output(['readelf', '--version-info', str(binary)], text=True)
    needed = sorted(set(re.findall(r'Name: GLIBC_([0-9.]+)', versions)),
                    key=lambda version: tuple(map(int, version.split('.'))))
    return {
        'binary': str(binary),
        'sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'minimum_glibc_symbol_version': needed[-1] if needed else None,
        'missing_libraries': [line.strip() for line in dependencies.splitlines() if 'not found' in line],
        'ldd': dependencies.splitlines(),
        'limits': 'Direct ELF dependencies only. Dynamic graphics/display libraries and clean-host GUI checks remain required.',
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    args = parser.parse_args()
    report = audit(args.binary.resolve(strict=True))
    print(json.dumps(report, indent=2))
    raise SystemExit(bool(report['missing_libraries']))
