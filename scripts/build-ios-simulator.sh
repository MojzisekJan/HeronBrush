#!/bin/sh
# Build an ad-hoc signed native iPad app for the Apple Silicon simulator already installed.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
export CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
export IPHONEOS_DEPLOYMENT_TARGET=16.0
xcrun --sdk iphonesimulator --show-sdk-path >/dev/null
# Stop the build before it can exhaust the disk; no simulator runtimes are downloaded.
python3 - <<'PY'
import os, shutil, signal, subprocess, time
if shutil.disk_usage('.').free < 1024 ** 3:
    raise SystemExit('At least 1 GiB of free disk space is required before building.')
p = subprocess.Popen(['cargo', 'build', '--locked', '--offline', '-p', 'heron-brush',
                      '--target', 'aarch64-apple-ios-sim'], start_new_session=True)
try:
    while p.poll() is None:
        if shutil.disk_usage('.').free < 512 * 1024 ** 2:
            raise RuntimeError('Stopped: less than 512 MiB of free disk space.')
        time.sleep(2)
except BaseException:
    if p.poll() is None:
        os.killpg(p.pid, signal.SIGTERM)
        p.wait()
    raise
raise SystemExit(p.returncode)
PY
app="$root/target/ios-simulator/Heron Brush.app"
mkdir -p "$app"
cp target/aarch64-apple-ios-sim/debug/heron-brush "$app/heron-brush"
rsync -a --delete --exclude '.DS_Store' assets/ "$app/assets/"
rsync -a --delete --exclude '.DS_Store' gamedata/ "$app/games/"
python3 - "$app" <<'PY'
import pathlib, plistlib, sys, tomllib
version = tomllib.loads(pathlib.Path('Cargo.toml').read_text())['workspace']['package']['version']
info = dict(CFBundleName='Heron Brush', CFBundleDisplayName='Heron Brush',
    CFBundleIdentifier='io.github.mojzisekjan.HeronBrush', CFBundleExecutable='heron-brush',
    CFBundlePackageType='APPL', CFBundleShortVersionString=version, CFBundleVersion='1',
    CFBundleSupportedPlatforms=['iPhoneSimulator'], MinimumOSVersion='16.0',
    LSRequiresIPhoneOS=True, UIDeviceFamily=[2], UILaunchScreen={},
    UIFileSharingEnabled=True, LSSupportsOpeningDocumentsInPlace=True,
    UISupportedInterfaceOrientations=['UIInterfaceOrientationPortrait',
        'UIInterfaceOrientationPortraitUpsideDown', 'UIInterfaceOrientationLandscapeLeft',
        'UIInterfaceOrientationLandscapeRight'])
with (pathlib.Path(sys.argv[1]) / 'Info.plist').open('wb') as f:
    plistlib.dump(info, f)
PY
codesign --force --sign - "$app"
echo "$app"
