#!/usr/bin/env python3
"""Inspect export paths and declared map material sources without writing files."""
import argparse
import json
import os
from pathlib import Path, PureWindowsPath, PurePosixPath
import sys

MARKERS = {
    'godot': ['project.godot'], 'defold': ['game.project'],
    'bevy': ['Cargo.toml'], 'three': ['package.json'],
    'unity': ['ProjectSettings/ProjectVersion.txt'],
    'flax': ['*.flaxproj'], 'stride': ['*.sdpkg', '*.sln'],
    'unreal': ['*.uproject'], 'cocos': ['package.json'],
}


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'duplicate JSON key: {key}')
        result[key] = value
    return result


def invalid_constant(value):
    raise ValueError(f'Non-standard JSON number: {value}')


def portability(root, value, field, base=None):
    """Inspect only configured host paths, never engine virtual resource paths."""
    base = root if base is None else base
    findings = []
    windows = PureWindowsPath(value)
    foreign = (os.name != 'nt' and bool(windows.drive or value.startswith('\\')))
    foreign |= os.name == 'nt' and PurePosixPath(value).is_absolute() and not windows.drive
    if foreign:
        return [{'field': field, 'kind': 'foreign_platform_path', 'configured': value,
                 'message': f'{field} uses a path from another platform; its location cannot be checked on this host'}], True
    path = Path(value)
    if path.is_absolute():
        findings.append({'field': field, 'kind': 'absolute_path', 'configured': value,
                         'message': f'{field} is absolute and will keep its old location when the project moves'})
    elif os.name != 'nt' and '\\' in value:
        findings.append({'field': field, 'kind': 'platform_separator', 'configured': value,
                         'message': f'{field} contains backslashes, which have different path semantics on Windows and this host'})
    candidate = path if path.is_absolute() else base / path
    try:
        resolved_root, resolved = root.resolve(), candidate.resolve()
        relative = resolved.relative_to(resolved_root) if resolved.is_relative_to(resolved_root) else None
        if relative is None:
            lexical = Path(os.path.abspath(candidate))
            lexical_root = Path(os.path.abspath(root))
            kind = 'external_symlink' if lexical.is_relative_to(lexical_root) else 'outside_project'
            findings.append({'field': field, 'kind': kind, 'configured': value,
                             'resolved': str(resolved),
                             'message': f'{field} resolves outside the project; copying the project folder alone will not include this location'})
        elif path.is_absolute():
            suggestion = Path(os.path.relpath(resolved, base.resolve())).as_posix()
            findings[0]['relative_suggestion'] = suggestion
            findings[0]['message'] += f'; use {suggestion!r} to keep it relative to its base directory'
    except (OSError, RuntimeError) as error:
        findings.append({'field': field, 'kind': 'unresolved_path', 'configured': value,
                         'message': f'{field} could not be resolved for portability: {error}'})
    return findings, foreign


def inspect(profile):
    profile = Path(profile).absolute()
    if profile.name != 'exports.json' or profile.parent.name != '.heron-brush':
        raise ValueError('Expected PROJECT/.heron-brush/exports.json')
    document = json.loads(profile.read_text(encoding='utf-8'), object_pairs_hook=unique_object, parse_constant=invalid_constant)
    if not isinstance(document, dict) or type(document.get('version')) is not int or document['version'] != 1:
        raise ValueError('Expected an export profile with version 1')
    targets = document.get('targets')
    if not isinstance(targets, dict):
        raise ValueError('Profile targets must be an object')
    root = profile.parent.parent
    result = {'profile': str(profile), 'root': str(root), 'targets': [], 'errors': []}
    for target, options in targets.items():
        row = {'target': target, 'project_dir': None, 'directory': None, 'errors': [], 'warnings': [], 'portability': []}
        result['targets'].append(row)
        if not isinstance(options, dict):
            row['errors'].append('Target settings must be an object')
            continue
        for field in ['project_dir', 'directory']:
            value = options.get(field, '')
            if not isinstance(value, str) or '\0' in value:
                row['errors'].append(f'{field} must be a path string without NUL characters')
                continue
            if not value:
                continue
            findings, foreign = portability(root, value, field)
            row['portability'].extend(findings)
            row['warnings'].extend(item['message'] for item in findings)
            if foreign:
                row[field] = value
                continue
            path = Path(value)
            if not path.is_absolute():
                path = root / path
            # Keep the same lexical root as the exporter, including symlink parents.
            row[field] = str(path)
            if field == 'project_dir':
                if not path.is_dir():
                    row['errors'].append(f'Project directory does not exist or is not a directory: {path}')
                elif target in MARKERS and not any(next(path.glob(marker), None) is not None for marker in MARKERS[target]):
                    row['warnings'].append('No expected engine project marker found; check that this is the intended project root')
            else:
                ancestor = path
                while not (ancestor.exists() or ancestor.is_symlink()) and ancestor != ancestor.parent:
                    ancestor = ancestor.parent
                if not ancestor.is_dir():
                    row['errors'].append(f'Output path is blocked by a file: {ancestor}')
                elif not path.exists():
                    row['warnings'].append('Output directory does not exist yet; the exporter may create it')
        if row['project_dir'] is None and not row['errors']:
            row['warnings'].append('Project root is automatic; this tool does not reproduce engine-specific root discovery')
        if row['directory'] is None and not row['errors']:
            row['warnings'].append('Output directory will be selected when exporting')
    result['errors'] = [f"{row['target']}: {error}" for row in result['targets'] for error in row['errors']]
    result['configured_paths_portable'] = not result['errors'] and not any(row['portability'] for row in result['targets'])
    return result


MATERIAL_PREFIX = b'// heron material-sources '
MAP_HEADER_LIMIT = 256 * 1024


def inspect_map_sources(map_path, root):
    """Read only a bounded map header and stat explicitly declared sources."""
    map_path = Path(map_path).absolute()
    row = {'map': str(map_path), 'declaration_found': False, 'sources': [],
           'warnings': [], 'errors': [], 'paths_portable': False}
    try:
        with map_path.open('rb') as source:
            data = source.read(MAP_HEADER_LIMIT + 1)
        truncated = len(data) > MAP_HEADER_LIMIT
        header = data[:MAP_HEADER_LIMIT]
        lines = header.split(b'\n')
        for line_index, line in enumerate(lines):
            if not line.startswith(MATERIAL_PREFIX):
                continue
            if truncated and line_index == len(lines) - 1 and not header.endswith(b'\n'):
                raise ValueError('Material source declaration exceeds the 256 KiB inspection limit')
            entries = json.loads(line[len(MATERIAL_PREFIX):].decode('utf-8'),
                                 object_pairs_hook=unique_object, parse_constant=invalid_constant)
            row['declaration_found'] = True
            if not isinstance(entries, list) or len(entries) > 2048:
                raise ValueError('Expected a material source list with at most 2048 entries')
            for index, entry in enumerate(entries):
                if not isinstance(entry, list) or len(entry) != 2 or not all(isinstance(value, str) for value in entry):
                    raise ValueError(f'Invalid material source entry {index}')
                kind, value = entry
                if kind not in ('folder', 'wad') or not value or '\0' in value:
                    raise ValueError(f'Invalid material source kind or path at entry {index}')
                findings, foreign = portability(Path(root), value, f'material_sources[{index}]', base=map_path.parent)
                path = Path(value)
                if not path.is_absolute():
                    path = map_path.parent / path
                item = {'kind': kind, 'configured': value, 'resolved': value if foreign else str(path),
                        'available': None, 'portability': findings}
                row['sources'].append(item)
                row['warnings'].extend(finding['message'] for finding in findings)
                if foreign:
                    continue
                item['available'] = path.is_dir() if kind == 'folder' else path.is_file()
                if not item['available']:
                    row['errors'].append(f'Material {kind} is missing or has the wrong file type: {path}')
            break
        if not row['declaration_found']:
            row['warnings'].append('No explicit material source declaration found in the inspected map header; automatic material lookup is not audited')
        row['paths_portable'] = row['declaration_found'] and not row['errors'] and not any(item['portability'] for item in row['sources'])
    except (OSError, ValueError) as error:
        row['errors'].append(str(error))
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('profile', type=Path, help='PROJECT/.heron-brush/exports.json')
    parser.add_argument('--json', action='store_true', help='Print structured diagnostics')
    parser.add_argument('--require-portable-paths', action='store_true',
                        help='Also exit with status 1 for nonportable configured paths')
    parser.add_argument('--map', dest='maps', action='append', type=Path, default=[],
                        help='Also inspect explicit material sources in this map; may be repeated')
    args = parser.parse_args()
    try:
        result = inspect(args.profile)
    except (OSError, ValueError) as error:
        print(f'Export paths: {error}', file=sys.stderr)
        return 2
    if args.maps:
        result['maps'] = [inspect_map_sources(path, Path(result['root'])) for path in args.maps]
        result['errors'].extend(f"{row['map']}: {error}" for row in result['maps'] for error in row['errors'])
        result['map_source_paths_portable'] = all(row['paths_portable'] for row in result['maps'])
    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print(f"Project: {result['root']}")
        for row in result['targets']:
            print(f"{row['target']}: project={row['project_dir'] or '(automatic)'}; output={row['directory'] or '(selected at export)'}")
            for kind in ['errors', 'warnings']:
                for message in row[kind]:
                    print(f'  {kind[:-1]}: {message}')
        for row in result.get('maps', []):
            print(f"Map: {row['map']}")
            for item in row['sources']:
                print(f"  {item['kind']}: {item['resolved']}")
            for kind in ['errors', 'warnings']:
                for message in row[kind]:
                    print(f'  {kind[:-1]}: {message}')
        print('Checks configured paths and selected maps material-source declarations only. Individual textures/models, bindings, permissions and engine imports are not audited.')
    return 1 if result['errors'] or (args.require_portable_paths and (not result['configured_paths_portable'] or not result.get('map_source_paths_portable', True))) else 0


if __name__ == '__main__':
    sys.exit(main())
