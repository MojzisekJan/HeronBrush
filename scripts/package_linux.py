#!/usr/bin/env python3
"""Package a verified installed Linux release as a relocatable tar.gz.

This preserves the installed BUILD.json provenance instead of assigning the
current checkout's commit to an older executable. It does not install anything.
Cross-distribution compatibility still requires an appropriate build baseline.
"""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import tarfile
import tempfile

from audit_linux_binary import audit


def verified_files(release):
    release = release.resolve(strict=True)
    metadata = json.loads((release / 'BUILD.json').read_text())
    expected = metadata['sha256']
    actual = set()
    for path in release.rglob('*'):
        if path.is_symlink():
            raise ValueError(f'Release contains a symlink: {path}')
        if path.is_file() and path.relative_to(release).as_posix() != 'BUILD.json':
            actual.add(path.relative_to(release).as_posix())
    if actual != set(expected):
        raise ValueError('Release file set differs from BUILD.json')
    for name, digest in expected.items():
        relative = PurePosixPath(name)
        if relative.is_absolute() or '..' in relative.parts or '\\' in name:
            raise ValueError(f'Invalid manifest path: {name}')
        if hashlib.sha256((release / name).read_bytes()).hexdigest() != digest:
            raise ValueError(f'Release checksum mismatch: {name}')
    return release, metadata


def package(release, output):
    release, metadata = verified_files(release)
    output = output.absolute()
    checksum = output.with_name(output.name + '.sha256')
    if output.exists() or checksum.exists() or output.is_symlink() or checksum.is_symlink():
        raise ValueError('Refusing to overwrite an existing package or checksum')
    dependencies = audit(release / 'heron-brush')
    if dependencies['missing_libraries']:
        raise ValueError('Executable has missing direct libraries')
    # Do not leak a workstation installation path into a distributable report.
    dependencies['binary'] = 'heron-brush'
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='heron-linux-package-', dir=output.parent) as temporary:
        folder = Path(temporary)
        staged = folder / 'HeronBrush'
        shutil.copytree(release, staged)
        _, copied_metadata = verified_files(staged)
        if copied_metadata != metadata:
            raise ValueError('Release changed during packaging')
        (staged / 'DEPENDENCIES.json').write_text(json.dumps(dependencies, indent=2) + '\n')
        (staged / 'RUNNING.txt').write_text(
            'Run ./heron-brush from this folder, or invoke its absolute path.\n'
            'Keep assets/ and games/ beside the executable. No installation is required.\n'
            f"Required glibc symbol version: {dependencies['minimum_glibc_symbol_version']}.\n"
            'Graphics/display drivers and clean-host GUI compatibility require separate checks.\n')
        hashes = {path.relative_to(staged).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
                  for path in sorted(staged.rglob('*')) if path.is_file()}
        (staged / 'PACKAGE.json').write_text(json.dumps({
            'format': 1, 'source_commit': metadata['source_commit'],
            'installed_build': metadata['build'], 'version': metadata['version'],
            'includes_uncommitted_changes': metadata['includes_uncommitted_changes'],
            'sha256': hashes,
        }, indent=2) + '\n')
        archive = folder / 'package.tar.gz'
        with tarfile.open(archive, 'w:gz', compresslevel=6) as target:
            target.add(staged, arcname='HeronBrush')
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        archive.replace(output)
        checksum.write_text(f'{digest}  {output.name}\n')
    return {'archive': str(output), 'sha256': digest, 'source_commit': metadata['source_commit'],
            'minimum_glibc_symbol_version': dependencies['minimum_glibc_symbol_version']}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release', type=Path, required=True, help='Installed release directory with BUILD.json')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(package(args.release, args.output), indent=2))
