#!/usr/bin/env python3
"""Install a check against the namespace of one generated export.

python3 install_check.py /tmp/project Assets/FinalMap VerifyExport.cs
"""
import re
import sys
from pathlib import Path
project=Path(sys.argv[1]);export=project/sys.argv[2]
source=Path(__file__).resolve().parent/sys.argv[3]
text=(export/'Editor/HeronBrushLevelImporter.cs').read_text()
namespace=re.search(r'namespace (HeronBrush\.Level_[a-f0-9]+)',text)[1]
destination=project/'Assets/Editor'/source.name;destination.parent.mkdir(parents=True,exist_ok=True)
destination.write_text(f'using {namespace};\n'+source.read_text())
print(destination)
