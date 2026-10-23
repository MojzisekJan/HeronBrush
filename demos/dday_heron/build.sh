#!/bin/sh
# Compiles heron_hedgerow_q2.map (the map exported in Quake 2's dialect) with ericw-tools
# and installs the bsp in D-Day: Normandy's dday/maps.
#
# DDAY_DIR is the folder holding dday/ (the mod's textures are read from it);
# ERICW_TOOLS is where qbsp, vis and light are. See docs/game-profile-tests.md. Pass
# --remake to build the map again in Heron Brush first (HERON_BRUSH names the binary,
# XDG_CONFIG_HOME should be a scratch directory whose preferences pick the D-Day
# Normandy profile and its game path).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
game=${DDAY_DIR:-$HOME/.local/opt/quake2/dday-normandy}
tools=${ERICW_TOOLS:-$HOME/.local/opt/quake/ericw-tools}

if [ "${1:-}" = "--remake" ]; then
    python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}"
fi

out=$here/build
mkdir -p "$out"
cp "$here/heron_hedgerow_q2.map" "$out/heron_hedgerow.map"
# The gamedir has to be absolute: a relative one is taken from the basedir's parent.
# qbsp reads each texture's size and its default flags (sky1 is sky) from the .wal
# files under dday/textures.
"$tools/qbsp" -q2bsp -basedir "$game" -gamedir "$game/dday" "$out/heron_hedgerow.map"
"$tools/vis" "$out/heron_hedgerow.bsp"
"$tools/light" -basedir "$game" -gamedir "$game/dday" -extra4 "$out/heron_hedgerow.bsp"

mkdir -p "$game/dday/maps"
cp "$out/heron_hedgerow.bsp" "$game/dday/maps/"
cp "$here/playtest.cfg" "$game/dday/"
echo "Play:      ~/.local/opt/quake2/run-dday +set deathmatch 1 +map heron_hedgerow"
echo "Play-test: add +exec playtest.cfg"
