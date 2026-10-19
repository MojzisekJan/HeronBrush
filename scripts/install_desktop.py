#!/usr/bin/env python3
"""Install the Linux desktop launcher and app icon for the current user."""
import argparse,os,shutil,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1]
assets=root/'crates/heron-brush/assets'
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',type=Path,default=root/'target/release/heron-brush')
p.add_argument('--data-dir',type=Path,default=Path(os.environ.get('XDG_DATA_HOME',Path.home()/'.local/share')))
a=p.parse_args();binary=a.binary.absolute()
if not binary.is_file() or not os.access(binary,os.X_OK):p.error(f'Executable not found: {binary}')
appid='io.github.mojzisekjan.HeronBrush'
icons=a.data_dir/'icons/hicolor/scalable/apps';apps=a.data_dir/'applications'
icons.mkdir(parents=True,exist_ok=True);apps.mkdir(parents=True,exist_ok=True)
shutil.copyfile(assets/'logo.svg',icons/(appid+'.svg'))
# Raster sizes also support desktop shells without SVG icon loading: rendered from the
# SVG with rsvg-convert, else scaled from the 1024 px app icon with Pillow, else the
# scalable icon alone.
def raster(size,target):
    if shutil.which('rsvg-convert'):
        target.parent.mkdir(parents=True,exist_ok=True)
        subprocess.run(['rsvg-convert','-w',str(size),'-h',str(size),'-o',str(target),str(assets/'logo.svg')],check=True)
        return True
    try:
        from PIL import Image
    except ImportError:
        return False
    target.parent.mkdir(parents=True,exist_ok=True)
    Image.open(assets/'app-icons/heron-brush.png').resize((size,size),Image.LANCZOS).save(target)
    return True
for size in (16,24,32,48,64,128,256,512):
    if not raster(size,a.data_dir/f'icons/hicolor/{size}x{size}/apps'/(appid+'.png')):
        print('Neither rsvg-convert nor Pillow found; installed only the scalable SVG icon.')
        break
if shutil.which('gtk-update-icon-cache'):
    subprocess.run(['gtk-update-icon-cache','--force','--ignore-theme-index',str(a.data_dir/'icons/hicolor')],check=True)

# Desktop-entry string escaping, followed by Exec argument escaping.
def value(s):return str(s).replace('\\','\\\\').replace('\n','\\n').replace('\r','\\r')
def arg(s):return '"'+value(str(s).replace('\\','\\\\').replace('"','\\"').replace('`','\\`').replace('$','\\$').replace('%','%%'))+'"'
(apps/(appid+'.desktop')).write_text(f'''[Desktop Entry]
Type=Application
Name=Heron Brush
Comment=3D brush level editor
Exec={arg(binary)} %f
Path={value(binary.parent)}
Icon={appid}
MimeType=text/x-quakemap;
Terminal=false
Categories=Graphics;3DGraphics;Development;
StartupNotify=true
StartupWMClass={appid}
''')
if shutil.which('update-desktop-database'):subprocess.run(['update-desktop-database',str(apps)],check=True)
print(apps/(appid+'.desktop'))
