#!/usr/bin/env python3
"""Prepare a disposable Defold or Flax project with a fresh motion_lab export."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", choices=["defold", "flax", "unity"])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/heron-brush")
    args = parser.parse_args()
    project = (args.output or Path("/tmp") / f"heron-{args.target}-motion").resolve()
    if project.is_relative_to(HERE / args.target):
        parser.error("Choose a disposable folder outside the original sample.")
    if args.target == "unity":
        destination = project / "Assets/MotionLab"
        destination.mkdir(parents=True, exist_ok=True)
        subprocess.run([sys.executable, str(HERE / "export_motion_lab.py"), "unity",
                        "--output", str(destination), "--project-root", str(project),
                        "--binary", str(args.binary.resolve())], check=True)
        editor = project / "Assets/Editor"
        editor.mkdir(exist_ok=True)
        subprocess.run([sys.executable,str(HERE / "unity/install_check.py"),str(project),"Assets/MotionLab","VerifyMotion.cs"],check=True)
        fixtures = project / "Assets/MotionFixtures"
        fixtures.mkdir(exist_ok=True)
        source = Path("/tmp/heron-portable-animation-check")
        shutil.copy2(source / "data.json", fixtures / "data.json")
        poses = json.loads((source / "expected.json").read_text())
        for pose in poses:
            pose["axes"] = [value for axis in pose["axes"] for value in axis]
        (fixtures / "expected.json").write_text(json.dumps({"poses": poses}))
        print(f"Prepared {project}")
        return
    shutil.copytree(HERE / args.target, project, dirs_exist_ok=True,
                    ignore=shutil.ignore_patterns("Cache", "Logs", "Binaries", "build", ".internal",
                                                  "textures", "Content", "arena", "*.log", "*.png"))
    destination = project / ("main" if args.target == "defold" else "Content/arena")
    command = [sys.executable, str(HERE / "export_motion_lab.py"), args.target,
               "--output", str(destination), "--project-root", str(project),
               "--name", "arena", "--binary", str(args.binary.resolve())]
    if args.target == "flax":
        command += ["--content-dir", "arena/arena"]
    subprocess.run(command, check=True)
    if args.target == "defold":
        subprocess.run([sys.executable, str(project / "install_game.py")], check=True)
        settings = project / "game.project"
        text = settings.read_text()
        if "use_fixed_timestep" not in text:
            text = text.replace("[physics]", "[physics]\nuse_fixed_timestep = 1")
        settings.write_text(text)
    else:
        shutil.copy2(project / "scene-template.json", project / "Content/ExportCheck.scene")
        shutil.copy2(HERE / "flax/Content/GameSettings.json", project / "Content/GameSettings.json")
        subprocess.run([sys.executable,str(HERE / "flax/install_checks.py"),str(project),"Content/arena"],check=True)
    print(f"Prepared {project}")


if __name__ == "__main__":
    main()
