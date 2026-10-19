"""The GUI runner must reject incomplete evidence instead of silently passing it."""
import json
from pathlib import Path
import tempfile
import unittest
import run_native


class EvidenceTests(unittest.TestCase):
    def test_missing_snapshot_fails_even_when_undo_pair_is_incomplete(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            script = root / "check.script"
            script.write_text("snapshot base\nsnapshot changed\nsnapshot undo\n")
            (root / "base.json").write_text(json.dumps({"map": "before"}))
            with self.assertRaisesRegex(AssertionError, "Missing snapshot"):
                run_native.validate(script, root)

    def test_wrong_undo_and_missing_picture_fail(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            script = root / "check.script"
            script.write_text("snapshot base\nsnapshot changed\nsnapshot undo\nshot edited\n")
            for name, value in [("base", "before"), ("changed", "after"), ("undo", "wrong")]:
                (root / f"{name}.json").write_text(json.dumps({"map": value}))
            with self.assertRaisesRegex(AssertionError, "Missing shot"):
                run_native.validate(script, root)
            (root / "edited.png").write_bytes(b"fixture")
            with self.assertRaisesRegex(AssertionError, "One Undo"):
                run_native.validate(script, root)

    def test_saved_snapshot_must_match_disk_and_be_clean(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            script = root / "check.script"
            script.write_text("snapshot saved\n")
            (root / "saved.json").write_text(json.dumps({"map": "new", "modified": False}))
            disk = root / "working.map"
            disk.write_text("old")
            with self.assertRaisesRegex(AssertionError, "Saved file"):
                run_native.validate(script, root, disk)
            disk.write_text('// heron material-sources [["folder","textures"]]\nnew')
            self.assertEqual(run_native.validate(script, root, disk)["saved"]["map"], "new")


if __name__ == "__main__":
    unittest.main()
