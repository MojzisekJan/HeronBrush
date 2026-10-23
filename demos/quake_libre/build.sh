#!/bin/sh
# Compiles libre_lift.map with ericw-tools and installs the bsp as a mod of LibreQuake.
#
# QUAKE_DIR is where LibreQuake, its mapping wads and the tools were unpacked; see
# docs/game-profile-tests.md. Pass --remake to build the map again in Heron Brush first
# (HERON_BRUSH names the binary, XDG_CONFIG_HOME should be a scratch directory).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
quake=${QUAKE_DIR:-$HOME/.local/opt/quake}
tools=$quake/ericw-tools
game=$quake/librequake

# The map names its wads relative to its own folder, which is how qbsp and Heron Brush
# both find them. They are LibreQuake's, so they are copied here and not committed.
mkdir -p "$here/wads"
for wad in lq_medieval.wad lq_tech.wad lq_utility.wad; do
    cp -u "$quake/librequake-dev/texture-wads/$wad" "$here/wads/"
done

if [ "${1:-}" = "--remake" ]; then
    python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}"
fi

out=$here/build
mkdir -p "$out"
cp "$here/libre_lift.map" "$out/"
cp -r "$here/wads" "$out/"
"$tools/qbsp" "$out/libre_lift.map"
"$tools/vis" "$out/libre_lift.bsp"
"$tools/light" -extra4 "$out/libre_lift.bsp"

mkdir -p "$game/heron/maps"
cp "$out/libre_lift.bsp" "$game/heron/maps/"
if [ -f "$out/libre_lift.lit" ]; then
    cp "$out/libre_lift.lit" "$game/heron/maps/"
fi
cp "$here/playtest.cfg" "$game/heron/"
echo "Play:      $quake/ironwail/ironwail -basedir $game -game heron +map libre_lift"
echo "Play-test: add -condebug +exec playtest.cfg; screenshots land in $game/heron/screenshots"
