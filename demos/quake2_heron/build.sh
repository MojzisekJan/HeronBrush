#!/bin/sh
# Compiles heron_outpost_q2.map (the map exported in Quake 2's dialect) with ericw-tools
# and installs the bsp in the game's baseq2/maps.
#
# Q2_DIR is the folder holding baseq2/ (the demo's pak0.pak is read for the textures);
# ERICW_TOOLS is where qbsp, vis and light are. See docs/game-profile-tests.md. Pass
# --remake to build the map again in Heron Brush first (HERON_BRUSH names the binary,
# XDG_CONFIG_HOME should be a scratch directory whose preferences pick the Quake 2
# profile and its game path).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
game=${Q2_DIR:-$HOME/.local/opt/quake2/yquake2}
tools=${ERICW_TOOLS:-$HOME/.local/opt/quake/ericw-tools}

if [ "${1:-}" = "--remake" ]; then
    python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}"
fi

out=$here/build
mkdir -p "$out"
cp "$here/heron_outpost_q2.map" "$out/heron_outpost.map"
# The gamedir has to be absolute: a relative one is taken from the basedir's parent.
# qbsp reads each texture's size and its default flags (sky1 is sky, clip is clip, the
# light panels emit) from the .wal files in pak0.pak.
"$tools/qbsp" -q2bsp -basedir "$game" -gamedir "$game/baseq2" "$out/heron_outpost.map"
"$tools/vis" "$out/heron_outpost.bsp"
"$tools/light" -basedir "$game" -gamedir "$game/baseq2" -extra4 "$out/heron_outpost.bsp"

mkdir -p "$game/baseq2/maps"
cp "$out/heron_outpost.bsp" "$game/baseq2/maps/"
cp "$here/playtest.cfg" "$here/teleporter.cfg" "$here/lamps.cfg" "$game/baseq2/"
echo "Play:      ~/.local/opt/quake2/run-yquake2 +map heron_outpost"
echo "Play-test: add +exec playtest.cfg, teleporter.cfg or lamps.cfg"
