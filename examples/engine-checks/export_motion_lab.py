#!/usr/bin/env python3
"""Export the same motion_lab map for target parity checks through the real MCP.

Run from any directory: python3 examples/engine-checks/export_motion_lab.py bevy
An optional --output selects a project folder; the default is /tmp/heron-motion-<target>.
"""
import argparse
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "assets"))
import prefabs_gen


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", choices=["raylib", "bevy", "three", "defold", "godot", "flax", "unity", "stride"])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--project-root", type=Path, help="engine project root when exporting into a subfolder")
    parser.add_argument("--name", default="motion_lab")
    parser.add_argument("--content-dir", help="Flax content directory, including the generated mesh subfolder")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/heron-brush")
    args = parser.parse_args()
    project = (args.output or Path("/tmp") / f"heron-motion-{args.target}").resolve()
    project.mkdir(parents=True, exist_ok=True)
    os.chdir(ROOT)
    prefabs_gen.BIN = str(args.binary.resolve())
    editor = prefabs_gen.Editor()
    try:
        print(editor.call("open_map", path=str(ROOT / "demos/motion_lab/motion_lab.map")))
        print(editor.call("load_definitions", path=str(ROOT / "demos/motion_lab/motion_lab.fgd")))
        options = {"project_dir": str((args.project_root or project).resolve()), "material_dir": "__no_external_materials__"}
        if args.target == "defold":
            options.update(model_material="/main/model.material", model_sampler="tex0")
        if args.content_dir:
            options["content_dir"] = args.content_dir
        print(editor.call("export", engine=args.target, path=str(project), name=args.name, scale=32,
                          options=options))
    finally:
        editor.close()


if __name__ == "__main__":
    main()
