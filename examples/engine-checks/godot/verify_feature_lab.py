#!/usr/bin/env python3
"""Run the feature lab's Godot export in Godot, feature by feature.

    cargo build -p heron-brush
    python3 examples/engine-checks/godot/verify_feature_lab.py [--root DIR] [--shots]

Exports demos/feature_lab/feature_lab.map for Godot into <root>/heron-feature-godot
(export_feature_lab.py, /tmp by default), makes that folder a Godot project, imports
it with `--headless --editor --import` and runs verify_feature_lab.gd there. With
--shots it then renders pictures of each station into <root>/heron-feature-godot-shots,
which needs a display and a GPU. GODOT picks the executable (default `godot`).
--no-export reuses an export that is already there. Logs go beside the export.
Exits with status 1 when a check fails.
"""
import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROJECT = """config_version=5

[application]

config/name="Heron Brush feature lab"
"""


def godot(executable, project, log, *args, timeout=300):
    with log.open("w") as out:
        result = subprocess.run([executable, "--path", str(project), *args], stdout=out,
                                stderr=subprocess.STDOUT, timeout=timeout)
    return result.returncode, log.read_text()


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=Path("/tmp"))
    parser.add_argument("--no-export", action="store_true")
    parser.add_argument("--shots", action="store_true")
    args = parser.parse_args()
    executable = os.environ.get("GODOT", "godot")
    project = args.root.resolve() / "heron-feature-godot"
    if not args.no_export:
        subprocess.run([sys.executable, str(HERE.parent / "export_feature_lab.py"), "godot",
                        "--root", str(args.root)], check=True)
    (project / "project.godot").write_text(PROJECT)
    shutil.copy(HERE / "verify_feature_lab.gd", project / "verify_feature_lab.gd")
    code, output = godot(executable, project, project / "import.log", "--headless", "--editor", "--import")
    if code:
        sys.exit(f"import failed, see {project / 'import.log'}")
    code, output = godot(executable, project, project / "verify.log", "--headless",
                         "--script", "res://verify_feature_lab.gd")
    for line in output.splitlines():
        if line.startswith(("PASS ", "FAIL ", "FEATURE LAB")) or "ERROR" in line or "WARNING" in line:
            print(line)
    if args.shots:
        # Outside the project, so a later import does not pick the pictures up.
        shots = project.parent / "heron-feature-godot-shots"
        godot(executable, project, project / "shots.log", "--script", "res://verify_feature_lab.gd",
              "--resolution", "1280x720", "--", f"--shots={shots}")
        print(f"pictures in {shots}")
    if code or "FEATURE LAB: 0 failures" not in output:
        sys.exit(f"checks failed, see {project / 'verify.log'}")


if __name__ == "__main__":
    main()
