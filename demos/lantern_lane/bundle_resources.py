#!/usr/bin/env python3
"""Refresh the Rust resource list after adding or removing demo support files."""
from pathlib import Path
base=Path(__file__).resolve().parent
root=base.parents[1]
files=[]
for directory in ('textures','sounds','definitions','godot'):
 files.extend(p for p in (base/directory).rglob('*') if p.is_file() and '.godot' not in p.parts and p.suffix not in ('.import','.uid'))
files.extend(base/name for name in ('README.md','lantern_lane.fgd','play.html','preview.png'))
text='// Portable demo resources. Regenerate with demos/lantern_lane/bundle_resources.py.\npub const FILES: &[(&str, &[u8])] = &[\n'
for path in sorted(set(files)):
 text+=f'    ("lantern_lane/{path.relative_to(base).as_posix()}", include_bytes!("../../../{path.relative_to(root).as_posix()}")),\n'
text+='];\n'
(root/'crates/editor_core/src/sample_resources.rs').write_text(text)
print(f'Bundled {len(files)} support files')
