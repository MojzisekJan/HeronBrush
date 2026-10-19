#!/usr/bin/env python3
"""Verify a local package, relocate it and export a bundled map outside the checkout.

Runs only the executable from a trusted package you built. This is a host smoke
check, not proof that another Linux distribution or graphics driver works.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import tempfile


def verify(archive):
    archive = archive.resolve(strict=True)
    checksum = archive.with_name(archive.name + '.sha256').read_text().strip()
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    if checksum != f'{digest}  {archive.name}':
        raise ValueError('Archive checksum mismatch')
    with tempfile.TemporaryDirectory(prefix='heron-relocated-') as temporary:
        root = Path(temporary)
        with tarfile.open(archive, 'r:gz') as source:
            for member in source.getmembers():
                path = PurePosixPath(member.name)
                if path.is_absolute() or '..' in path.parts or not path.parts or path.parts[0] != 'HeronBrush' or not (member.isfile() or member.isdir()):
                    raise ValueError(f'Unsafe package member: {member.name}')
            source.extractall(root, filter='data')
        release = root / 'HeronBrush'
        metadata = json.loads((release / 'PACKAGE.json').read_text())
        actual = {p.relative_to(release).as_posix() for p in release.rglob('*') if p.is_file() and p.relative_to(release).as_posix() != 'PACKAGE.json'}
        if actual != set(metadata['sha256']):
            raise ValueError('Packaged file set differs from PACKAGE.json')
        for name, expected in metadata['sha256'].items():
            if hashlib.sha256((release / name).read_bytes()).hexdigest() != expected:
                raise ValueError(f'Packaged file checksum mismatch: {name}')
        unrelated = root / 'empty-working-directory'
        unrelated.mkdir()
        samples = sorted((release / 'assets').glob('*.map'))
        if not samples:
            raise ValueError('Package has no bundled maps')
        requests = [
            {'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {}},
            {'jsonrpc': '2.0', 'id': 2, 'method': 'tools/call', 'params': {'name': 'list_plugins', 'arguments': {}}},
            {'jsonrpc': '2.0', 'id': 3, 'method': 'tools/call', 'params': {'name': 'open_map', 'arguments': {'path': str(samples[0])}}},
            {'jsonrpc': '2.0', 'id': 4, 'method': 'tools/call', 'params': {'name': 'export', 'arguments': {'engine': 'three', 'path': str(root / 'export'), 'name': 'packaged', 'scale': 32}}},
        ]
        env = {**os.environ, **{key: str(root / key.lower()) for key in ['XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME']}}
        result = subprocess.run([str(release / 'heron-brush'), '--mcp'], cwd=unrelated,
                                env=env, input=''.join(json.dumps(r) + '\n' for r in requests),
                                text=True, capture_output=True, timeout=90, check=True)
        replies = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
        for identifier in [1, 2, 3, 4]:
            reply = next(r for r in replies if r.get('id') == identifier)
            if reply.get('error') or reply.get('result', {}).get('isError'):
                raise ValueError(f'Packaged MCP failed: {reply}')
        if 'selection-inspector' not in json.dumps(next(r for r in replies if r.get('id') == 2)):
            raise ValueError('Bundled plugin discovery failed outside the checkout')
        level = json.loads((root / 'export/packaged.json').read_text())
        if not level['entities'] or not any((root / 'export').rglob('*.glb')):
            raise ValueError('Bundled map export did not produce level geometry')
        return {'result': 'PASS', 'source_commit': metadata['source_commit'],
                'files_verified': len(metadata['sha256']), 'bundled_map': samples[0].name,
                'scope': 'Relocated archive, isolated XDG, unrelated CWD, plugin discovery, map open and Three.js export on this host'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.archive), indent=2))
