#!/usr/bin/env python3
"""Add the local brick texture source to a coloured map fixture."""
import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[2]
source = (Path(sys.argv[2]) if len(sys.argv)>2 else root / "examples/editor-checks/cube.map").read_text()
header = "// heron material-sources " + json.dumps([
    ["folder", str(root / "assets/textures/heron_demo")]
])
Path(sys.argv[1]).write_text(source.replace("// heron material-sources []", header))
