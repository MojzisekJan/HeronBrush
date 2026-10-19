"""Small filesystem fixtures for the read-only project-path diagnostic."""
import json
import contextlib
import io
from unittest.mock import patch
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_export_paths import inspect, inspect_map_sources, MAP_HEADER_LIMIT, main


class ExportPathsCheck(unittest.TestCase):
    def profile(self, root, targets):
        path = root / '.heron-brush/exports.json'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({'version': 1, 'targets': targets}))
        return path

    def test_relocated_relative_paths_and_uncreated_output(self):
        with tempfile.TemporaryDirectory() as directory:
            first = Path(directory) / 'first'
            path = self.profile(first, {'godot': {'project_dir': '.', 'directory': 'generated/maps'}})
            (first / 'project.godot').write_text('config_version=5')
            second = Path(directory) / 'moved'
            shutil.copytree(first, second)
            result = inspect(second / path.relative_to(first))
            self.assertEqual(result['errors'], [])
            self.assertEqual(result['targets'][0]['directory'], str(second / 'generated/maps'))
            self.assertEqual(len(result['targets'][0]['warnings']), 1)
            self.assertFalse((second / 'generated').exists(), 'Diagnostic must not create output')

    def test_explicit_missing_project_and_file_blocking_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'blocked').write_text('fixture')
            result = inspect(self.profile(root, {'unity': {'project_dir': 'missing', 'directory': 'blocked/maps'}}))
            self.assertEqual(len(result['errors']), 2)
            self.assertIn('blocked by a file', result['errors'][1])

    def test_empty_paths_and_markerless_projects_are_advisory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = inspect(self.profile(root, {'godot': {'project_dir': '.'}, 'glb': {}}))
            self.assertEqual(result['errors'], [])
            self.assertIn('marker', result['targets'][0]['warnings'][0])
            self.assertIsNone(result['targets'][1]['project_dir'])

    def test_absolute_destinations_are_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = inspect(self.profile(root, {'obj': {'directory': str(root / 'outside')}}))
            self.assertEqual(result['targets'][0]['directory'], str(root / 'outside'))
            self.assertEqual(result['errors'], [])

    def test_duplicate_keys_and_malformed_paths_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = self.profile(root, {})
            path.write_text('{"version":1,"targets":{},"targets":{}}')
            with self.assertRaisesRegex(ValueError, 'duplicate JSON key'):
                inspect(path)
            result = inspect(self.profile(root, {'bevy': {'project_dir': [], 'directory': 'bad\0path'}}))
            self.assertEqual(len(result['errors']), 2)

    def test_dangling_output_link_and_nonstandard_json_numbers_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'dangling').symlink_to(root / 'missing')
            result = inspect(self.profile(root, {'three': {'directory': 'dangling/maps'}}))
            self.assertIn('blocked by a file', result['errors'][0])
            path = self.profile(root, {})
            path.write_text('{"version":1,"targets":{"bevy":{"scale":NaN}}}')
            with self.assertRaisesRegex(ValueError, 'Non-standard JSON number'):
                inspect(path)

    def test_absolute_internal_path_suggests_a_project_relative_path(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'generated').mkdir()
            profile = self.profile(root, {'obj': {'directory': str(root / 'generated')}})
            result = inspect(profile)
            finding = result['targets'][0]['portability'][0]
            self.assertEqual(finding['kind'], 'absolute_path')
            self.assertEqual(finding['relative_suggestion'], 'generated')
            self.assertEqual(result['errors'], [])
            self.assertFalse(result['configured_paths_portable'])
            self.assertEqual(json.loads(profile.read_text())['targets']['obj']['directory'], str(root / 'generated'))

    def test_parent_escape_and_sibling_with_shared_prefix_are_external(self):
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            root = parent / 'game'
            (parent / 'game-extra').mkdir()
            result = inspect(self.profile(root, {'obj': {'directory': '../game-extra'}}))
            findings = result['targets'][0]['portability']
            self.assertEqual([item['kind'] for item in findings], ['outside_project'])
            self.assertNotIn('relative_suggestion', findings[0])
            self.assertEqual(result['errors'], [])

    def test_external_symlink_and_nonexistent_child_are_detected(self):
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            root = parent / 'game'
            root.mkdir()
            external = parent / 'external'
            external.mkdir()
            (root / 'linked').symlink_to(external, target_is_directory=True)
            result = inspect(self.profile(root, {'obj': {'directory': 'linked/new/maps'}}))
            finding = result['targets'][0]['portability'][0]
            self.assertEqual(finding['kind'], 'external_symlink')
            self.assertEqual(finding['resolved'], str(external / 'new/maps'))
            self.assertFalse((external / 'new').exists())

    def test_relative_internal_paths_remain_portable_under_symlinked_project_root(self):
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            root = parent / 'real'
            profile = self.profile(root, {'godot': {'project_dir': '.', 'directory': 'maps'}})
            (root / 'project.godot').write_text('config_version=5')
            alias = parent / 'alias'
            alias.symlink_to(root, target_is_directory=True)
            result = inspect(alias / profile.relative_to(root))
            self.assertTrue(result['configured_paths_portable'])
            self.assertEqual(result['targets'][0]['portability'], [])

    @unittest.skipIf(sys.platform == 'win32', 'Foreign Windows syntax test runs on POSIX')
    def test_foreign_windows_paths_are_reported_without_wrong_host_resolution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            values = [r'C:\Games\Project', r'\\server\share\Project', r'C:relative']
            for value in values:
                with self.subTest(value=value):
                    result = inspect(self.profile(root, {'unity': {'project_dir': value}}))
                    row = result['targets'][0]
                    self.assertEqual(row['project_dir'], value)
                    self.assertEqual(row['portability'][0]['kind'], 'foreign_platform_path')
                    self.assertEqual(row['errors'], [])
                    self.assertFalse(result['configured_paths_portable'])

    def test_cli_can_require_portable_paths_without_changing_default_exit_status(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            profile = self.profile(root, {'obj': {'directory': str(root)}})
            for flags, expected in [([], 0), (['--require-portable-paths'], 1)]:
                output = io.StringIO()
                with patch.object(sys, 'argv', ['check_export_paths', str(profile), '--json', *flags]), contextlib.redirect_stdout(output):
                    self.assertEqual(main(), expected)
                result = json.loads(output.getvalue())
                self.assertFalse(result['configured_paths_portable'])
                self.assertEqual(result['targets'][0]['portability'][0]['relative_suggestion'], '.')
            self.profile(root, {'godot': {'project_dir': '.', 'directory': 'generated'}})
            with patch.object(sys, 'argv', ['check_export_paths', str(profile), '--require-portable-paths']), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(main(), 0, 'Advisory engine-marker and future-output warnings must stay advisory')


class MapMaterialSourcesCheck(unittest.TestCase):
    def map_file(self, root, entries):
        path = root / 'maps/level.map'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('// heron material-sources ' + json.dumps(entries) + '\n{\n}\n')
        return path

    def test_source_paths_are_relative_to_map_not_profile_root(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'textures').mkdir()
            (root / 'pack.pk3').write_bytes(b'not decoded by the diagnostic')
            path = self.map_file(root, [['folder', '../textures'], ['wad', '../pack.pk3']])
            result = inspect_map_sources(path, root)
            self.assertTrue(result['paths_portable'])
            self.assertEqual(result['errors'], [])
            self.assertTrue(all(item['available'] for item in result['sources']))

    def test_missing_and_wrong_type_sources_are_errors(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'file').write_bytes(b'fixture')
            path = self.map_file(root, [['folder', '../file'], ['wad', '../missing.pk3']])
            result = inspect_map_sources(path, root)
            self.assertEqual(len(result['errors']), 2)
            self.assertFalse(result['paths_portable'])

    def test_external_symlink_and_absolute_internal_suggestion(self):
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            root = parent / 'game'
            root.mkdir()
            (root / 'textures').mkdir()
            external = parent / 'outside'
            external.mkdir()
            (root / 'linked').symlink_to(external, target_is_directory=True)
            path = self.map_file(root, [['folder', '../linked'], ['folder', str(root / 'textures')]])
            result = inspect_map_sources(path, root)
            self.assertEqual(result['sources'][0]['portability'][0]['kind'], 'external_symlink')
            self.assertEqual(result['sources'][1]['portability'][0]['relative_suggestion'], '../textures')
            self.assertEqual(result['errors'], [])
            self.assertFalse(result['paths_portable'])

    def test_absent_metadata_is_unknown_while_explicit_empty_list_is_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = self.map_file(root, [])
            self.assertTrue(inspect_map_sources(path, root)['paths_portable'])
            path.write_text('{\n}\n')
            result = inspect_map_sources(path, root)
            self.assertFalse(result['paths_portable'])
            self.assertFalse(result['declaration_found'])
            self.assertEqual(result['errors'], [])

    def test_malformed_or_unknown_metadata_does_not_produce_a_clean_result(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for entries in [{}, [['unknown', 'asset']], [['folder']], [['folder', None]]]:
                with self.subTest(entries=entries):
                    result = inspect_map_sources(self.map_file(root, entries), root)
                    self.assertTrue(result['errors'])
                    self.assertFalse(result['paths_portable'])
            result = inspect_map_sources(root / 'missing.map', root)
            self.assertTrue(result['errors'])

    def test_header_read_is_bounded_and_does_not_claim_late_metadata_was_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = self.map_file(root, [])
            path.write_bytes(b'// padding\n' * (MAP_HEADER_LIMIT // 10) + b'// heron material-sources []\n')
            result = inspect_map_sources(path, root)
            self.assertFalse(result['declaration_found'])
            self.assertFalse(result['paths_portable'])
            path.write_bytes(b'// heron material-sources ' + b' ' * MAP_HEADER_LIMIT + b'[]\n')
            result = inspect_map_sources(path, root)
            self.assertIn('256 KiB', result['errors'][0])

    def test_cli_checks_selected_maps_and_keeps_profile_portability_separate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.heron-brush').mkdir()
            profile = root / '.heron-brush/exports.json'
            profile.write_text('{"version":1,"targets":{}}')
            path = self.map_file(root, [])
            path.write_text('{\n}\n')
            output = io.StringIO()
            with patch.object(sys, 'argv', ['check_export_paths', str(profile), '--json', '--map', str(path), '--require-portable-paths']), contextlib.redirect_stdout(output):
                self.assertEqual(main(), 1)
            result = json.loads(output.getvalue())
            self.assertTrue(result['configured_paths_portable'])
            self.assertFalse(result['map_source_paths_portable'])
            self.assertEqual(result['errors'], [])

    def test_crlf_and_first_declaration_match_editor_selection(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = self.map_file(root, [])
            path.write_bytes(b'// Format: Valve\r\n// heron material-sources []\r\n// heron material-sources broken\r\n')
            self.assertTrue(inspect_map_sources(path, root)['paths_portable'])
            path.write_bytes(b'// heron material-sources []')
            self.assertTrue(inspect_map_sources(path, root)['paths_portable'])
            path.write_bytes(b'// heron material-sources broken\n// heron material-sources []\n')
            self.assertTrue(inspect_map_sources(path, root)['errors'])
