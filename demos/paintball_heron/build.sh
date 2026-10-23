#!/bin/sh
# Compiles heron_bases.map with ericw-tools for Quake 2 and installs the bsp in
# Paintball 2's pball/maps.
#
# PB2_DIR is the Paintball 2 folder (the one holding the paintball2 binary and pball/);
# ERICW_TOOLS is where qbsp, vis and light are. See docs/game-profile-tests.md. Pass
# --remake to build the map again in Heron Brush first (HERON_BRUSH names the binary,
# XDG_CONFIG_HOME should be a scratch directory whose preferences pick the Paintball 2
# profile and its game path).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
game=${PB2_DIR:-$HOME/.local/opt/paintball2/paintball2}
tools=${ERICW_TOOLS:-$HOME/.local/opt/quake/ericw-tools}

if [ "${1:-}" = "--remake" ]; then
    python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}"
fi

out=$here/build
mkdir -p "$out"
cp "$here/heron_bases.map" "$out/"
# The gamedir has to be absolute: a relative one is taken from the basedir's parent.
# qbsp reads each texture's size and its default surface flags (sky1.wal is sky and
# emits light) from pball/textures.
"$tools/qbsp" -q2bsp -basedir "$game" -gamedir "$game/pball" "$out/heron_bases.map"
"$tools/vis" "$out/heron_bases.bsp"
"$tools/light" -basedir "$game" -gamedir "$game/pball" -extra4 "$out/heron_bases.bsp"

mkdir -p "$game/pball/maps"
cp "$out/heron_bases.bsp" "$game/pball/maps/"
cp "$here/playtest.cfg" "$game/pball/configs/"
echo "Play:      (cd $game && ./paintball2 +map heron_bases)"
echo "Play-test: add +exec playtest.cfg; screenshots land in $game/pball/scrnshot"
