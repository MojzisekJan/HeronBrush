#!/usr/bin/env python3
"""Regenerate platform icons from logo.svg. Requires rsvg-convert (librsvg) and Pillow."""
from pathlib import Path
import subprocess
from PIL import Image
root=Path(__file__).resolve().parents[1]
assets=root/'crates/heron-brush/assets'
out=assets/'app-icons';out.mkdir(exist_ok=True)
subprocess.run(['rsvg-convert','-w','1024','-h','1024','-o',str(out/'heron-brush.png'),str(assets/'logo.svg')],check=True)
im=Image.open(out/'heron-brush.png')
im.save(out/'heron-brush.ico',sizes=[(n,n) for n in (16,24,32,48,64,128,256)])
im.save(out/'heron-brush.icns')
print('Generated PNG, ICO and ICNS from logo.svg')
