#!/bin/sh
# Compiles heron_marsh_dk.map (the map exported in the Quake 2 face lines Daikatana's
# compilers read) with Ion Storm's qbsp3, qvis3 and arghrad from the Daikatana 1.3
# project's Linux dktools, and installs the bsp where the viewer and the game look.
#
# DK_DIR is the folder of run-dktool, run-yq2remaster and game/data (the 1.3 build
# with the demo's paks, whose pak1.pak the tools read the textures from). See
# docs/game-profile-tests.md. Pass --remake to build the map again in Heron Brush first
# (HERON_BRUSH names the binary, XDG_CONFIG_HOME should be a scratch directory whose
# preferences pick the Daikatana profile and its game path).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
dk=${DK_DIR:-$HOME/.local/opt/daikatana}

if [ "${1:-}" = "--remake" ]; then
    python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}"
fi

out=$here/build
mkdir -p "$out"
cp "$here/heron_marsh_dk.map" "$out/heron_marsh.map"
cd "$out"
# The tools glue "pak1.pak" straight onto -gamedir, so it needs its trailing slash;
# without it every texture is "Couldn't load".
"$dk/run-dktool" qbsp3 -gamedir "$dk/game/data/" heron_marsh.map
"$dk/run-dktool" qvis3 -level 3 heron_marsh
"$dk/run-dktool" arghrad -chop 128 -bounce 1 -gamedir "$dk/game/data/" heron_marsh

# The Daikatana 1.3 engine reads data/maps; the viewer reads its own game folder.
for maps in "$dk/game/data/maps" "$dk/yq2remaster/data/daikatana/maps"; do
    mkdir -p "$maps"
    cp heron_marsh.bsp "$maps/"
done
cp "$here/playtest.cfg" "$dk/yq2remaster/data/daikatana/"
echo "View:      $dk/run-yq2remaster +map heron_marsh +exec playtest.cfg"
