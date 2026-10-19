#!/usr/bin/env python3
"""Bind the standard Flax checks to one generated level namespace."""
import sys
from pathlib import Path
here=Path(__file__).resolve().parent
project=Path(sys.argv[1]);export=project/sys.argv[2]
text=(export/'HeronBrushLevel.cs').read_text()
namespace=next(line[len('namespace '):].strip() for line in text.splitlines() if line.startswith('namespace Game.HeronLevel_'))
destination=project/'Source/Game';destination.mkdir(parents=True,exist_ok=True)
for name in ['ExportCheckPlugin.cs','CollisionCheck.cs','MotionCheck.cs']:
    text=(here/'Source/Game'/name).read_text()
    text='\n'.join(line for line in text.splitlines() if not line.startswith('using Game.HeronLevel_'))+'\n'
    (destination/name).write_text(f'using {namespace};\n'+text)
for name in ['HeronBrushLevel.cs','HeronBrushAnimation.cs','HeronBrushMotionPlayer.cs','HeronBrushLevelRoot.cs','HeronBrushWater.cs']:
    (destination/name).write_bytes((export/name).read_bytes())
