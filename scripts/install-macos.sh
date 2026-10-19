#!/bin/sh
# Builds Heron Brush as a macOS app bundle and installs it.
#
#   scripts/install-macos.sh            # /Applications (or ~/Applications if not writable)
#   scripts/install-macos.sh --gl       # the 3D view on OpenGL instead of wgpu
#   scripts/install-macos.sh --no-install   # only build target/macos/Heron Brush.app
#
# The bundled maps, textures and plugins go beside the executable, where the editor
# looks for them on a fresh installation (`preferences::bundled_asset_directory`).
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
features=""
install=1
for arg in "$@"; do
    case "$arg" in
        --gl) features="--no-default-features" ;;
        --no-install) install=0 ;;
        *) echo "unknown option: $arg" >&2; exit 2 ;;
    esac
done

cd "$root"
# shellcheck disable=SC2086
cargo build --release --locked -p heron-brush $features

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
app="$root/target/macos/Heron Brush.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/heron-brush "$app/Contents/MacOS/heron-brush"
cp crates/heron-brush/assets/app-icons/heron-brush.icns "$app/Contents/Resources/heron-brush.icns"
# Maps and their textures and plugins; the editor reads them from here.
rsync -a --exclude '.DS_Store' assets/ "$app/Contents/MacOS/assets/"
# Game configurations shipped with the editor, found as `games` beside the binary.
rsync -a --exclude ".DS_Store" gamedata/ "$app/Contents/MacOS/games/"
cp LICENSE "$app/Contents/Resources/LICENSE"

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Heron Brush</string>
    <key>CFBundleDisplayName</key><string>Heron Brush</string>
    <key>CFBundleIdentifier</key><string>io.github.mojzisekjan.HeronBrush</string>
    <key>CFBundleExecutable</key><string>heron-brush</string>
    <key>CFBundleIconFile</key><string>heron-brush</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.graphics-design</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict>
</plist>
EOF

# An ad-hoc signature, so the bundle is consistent after the copy above.
codesign --force --deep --sign - "$app" >/dev/null 2>&1 || true

if [ "$install" = 0 ]; then
    echo "$app"
    exit 0
fi

destination=/Applications
[ -w "$destination" ] || { destination="$HOME/Applications"; mkdir -p "$destination"; }
rm -rf "$destination/Heron Brush.app"
cp -R "$app" "$destination/"
# Make Launchpad, Spotlight and the Dock see the new copy at once.
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$destination/Heron Brush.app" || true
echo "$destination/Heron Brush.app"
