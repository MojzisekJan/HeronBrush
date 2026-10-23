#!/usr/bin/env python3
"""Put a fresh feature lab export and VerifyFeatureLab.cs into a Unity project.

python3 prepare_feature_lab.py /tmp/heron-unity-feature

The project must exist (Unity -createProject). The export lands in
Assets/Levels/heron-feature-unity, the editor's reference poses and colours in
feature_lab_reference.json at the project root, and the check in Assets/Editor.
"""
import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("project", type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/heron-brush")
    args = parser.parse_args()
    project = args.project.resolve()
    if not (project / "Assets").is_dir():
        parser.error(f"{project} is not a Unity project; create it with Unity -createProject")
    levels = project / "Assets/Levels"
    levels.mkdir(exist_ok=True)
    # Generated materials and collision assets from an earlier build go too.
    for stale in [levels / "heron-feature-unity", levels / "heron-feature-unity.meta"]:
        if stale.is_dir():
            shutil.rmtree(stale)
        elif stale.exists():
            stale.unlink()
    subprocess.run([sys.executable, str(ROOT / "examples/engine-checks/export_feature_lab.py"), "unity",
                    "--root", str(levels), "--binary", str(args.binary)], check=True)
    env = dict(os.environ, HERON_FEATURE_REFERENCE=str(project / "feature_lab_reference.json"))
    subprocess.run(["cargo", "test", "-p", "editor_core", "--lib", "feature_lab_editor_reference",
                    "--", "--ignored"], cwd=ROOT, env=env, check=True)
    subprocess.run([sys.executable, str(HERE / "install_check.py"), str(project),
                    "Assets/Levels/heron-feature-unity", "VerifyFeatureLab.cs"], check=True)
    print(f"Prepared {project}")


if __name__ == "__main__":
    main()
