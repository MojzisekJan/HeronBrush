#!/bin/sh
# Compiles heron_lab.map and heron_lab2.map with SDHLT and installs the bsps where
# Xash3D loads them, in the Uplink folder's valve/maps.
#
# HL_DIR is the folder holding valve/ (Uplink's pak0.pak is read for the textures);
# SDHLT is the unpacked SDHLT release (tools/Linux holds the compilers, tools/sdhlt.wad
# the tool textures). See docs/game-profile-tests.md. Pass --remake to build the maps
# again in Heron Brush first (HERON_BRUSH names the binary, XDG_CONFIG_HOME should be a
# scratch directory whose preferences pick the Half-Life profile and its game path).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
game=${HL_DIR:-$HOME/.local/opt/halflife/uplink}
sdhlt=${SDHLT:-$HOME/.local/opt/halflife/sdhlt}
tools=$sdhlt/tools/Linux

# The wads are game data and are not committed: uplink.wad is gathered from Uplink's
# maps, sdhlt.wad comes with the compilers.
mkdir -p "$here/wads"
python3 "$here/extract_wad.py" "$game/valve/pak0.pak" "$here/wads/uplink.wad"
cp "$sdhlt/tools/sdhlt.wad" "$here/wads/"

if [ "${1:-}" = "--remake" ]; then
    python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}"
fi

out=$here/build
mkdir -p "$out/wads"
cp "$here/wads/"*.wad "$out/wads/"
cp "$here/lights.rad" "$out/"
for name in heron_lab heron_lab2; do
    cp "$here/$name.map" "$out/"
    # hlcsg opens the wads the map names relative to the folder it runs in. Uplink
    # has none of them, so -wadinclude puts every texture the map uses into the bsp.
    (
        cd "$out"
        "$tools/sdHLCSG" -wadinclude uplink.wad "$name.map"
        "$tools/sdHLBSP" "$name.map"
        "$tools/sdHLVIS" "$name.map"
        "$tools/sdHLRAD" "$name.map"
    )
done

mkdir -p "$game/valve/maps"
cp "$out/heron_lab.bsp" "$out/heron_lab2.bsp" "$game/valve/maps/"
cp "$here/playtest.cfg" "$game/valve/"
echo "Play:      ~/.local/opt/halflife/run-xash -windowed +map heron_lab"
echo "Play-test: see playtest.cfg"
