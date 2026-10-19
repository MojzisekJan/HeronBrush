#!/usr/bin/env python3
"""Export any map into a Cocos Creator project through the real MCP.

python3 examples/engine-checks/gdevelop/export_map.py demos/relay_yard/relay_yard.map \
    --project ~/CocosGames/Yard [--fgd file.fgd] [--name level] [--textures dir]

Writes the level under <project>/assets/resources/levels, where its loader component
reads it as levels/<name>.
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
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--fgd", type=Path)
    parser.add_argument("--name")
    parser.add_argument("--textures", type=Path, help="texture folder; defaults to textures/ beside the map")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/heron-brush")
    args = parser.parse_args()
    source = args.map.resolve()
    name = args.name or source.stem
    project = args.project.expanduser().resolve()
    (project / "assets/resources/levels").mkdir(parents=True, exist_ok=True)
    fgd = args.fgd.resolve() if args.fgd else next(iter(source.parent.glob("*.fgd")), None)
    options = {"project_dir": str(project)}
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
        print(editor.call("export", engine="cocos", path=str(project / "assets/resources/levels"), name=name, scale=32, options=options))
    finally:
        editor.close()


if __name__ == "__main__":
    main()
