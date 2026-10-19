"""A portable package must reject incomplete, changed or linked source releases."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from package_linux import verified_files


class LinuxPackageManifestCheck(unittest.TestCase):
    def release(self, root):
        (root / 'heron-brush').write_bytes(b'owned-fixture-binary')
        (root / 'BUILD.json').write_text(json.dumps({'sha256': {
            'heron-brush': hashlib.sha256(b'owned-fixture-binary').hexdigest()}}))

    def test_changed_or_extra_files_are_not_packaged(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.release(root)
            verified_files(root)
            (root / 'heron-brush').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
                verified_files(root)
            self.release(root)
            (root / 'unlisted-file').write_bytes(b'not in manifest')
            with self.assertRaisesRegex(ValueError, 'file set differs'):
                verified_files(root)

    def test_symlinks_cannot_include_unverified_external_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.release(root)
            (root / 'external').symlink_to('/etc/hosts')
            with self.assertRaisesRegex(ValueError, 'symlink'):
                verified_files(root)


if __name__ == '__main__':
    unittest.main()
