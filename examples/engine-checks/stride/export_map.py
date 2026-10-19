#!/usr/bin/env python3
"""Export any map into a Stride game package through the real MCP.

python3 examples/engine-checks/stride/export_map.py demos/relay_yard/relay_yard.map \
    --package ~/StrideGame/StrideGame.Game [--fgd file.fgd] [--name level]

The level lands in <package>/Assets/Levels/<name>, so its content URL is Levels/<name>
and the export adds its models to the package's root assets.
"""
import argparse
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "assets"))
import prefabs_gen


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("map", type=Path)
    parser.add_argument("--package", type=Path, required=True, help="the folder holding the game's .sdpkg")
    parser.add_argument("--fgd", type=Path)
    parser.add_argument("--name")
    parser.add_argument("--textures", type=Path, help="texture folder; defaults to textures/ beside the map")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/heron-brush")
    args = parser.parse_args()
    source = args.map.resolve()
    name = args.name or source.stem
    package = args.package.expanduser().resolve()
    output = package / "Assets/Levels"
    output.mkdir(parents=True, exist_ok=True)
    fgd = args.fgd.resolve() if args.fgd else next(iter(source.parent.glob("*.fgd")), None)
    options = {"project_dir": str(package.parent)}
    textures = (args.textures or source.parent / "textures").resolve()
    if textures.is_dir():
        options["texture_dir"] = str(textures)
    os.chdir(ROOT)
    prefabs_gen.BIN = str(args.binary.resolve())
    editor = prefabs_gen.Editor()
    try:
        print(editor.call("open_map", path=str(source)))
        if fgd:
            print(editor.call("load_definitions", path=str(fgd)))
        print(editor.call("export", engine="stride", path=str(output), name=name, scale=32,
                          options=options))
    finally:
        editor.close()


if __name__ == "__main__":
    main()
