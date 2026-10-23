#!/bin/sh
# Compiles heron_yard.map with quemap and installs the bsp where Quetoo looks for a
# player's own maps.
#
# QUETOO is the Quetoo installation (the folder holding bin/quemap and share/quetoo).
# The bsp goes to Quetoo's user folder, $XDG_DATA_HOME/WickedOldGames/Quetoo/default/maps
# (~/.local/share/... without XDG_DATA_HOME), which is also where quemap writes its log.
# See docs/game-profile-tests.md. Pass --remake to build the map again in Heron Brush
# first (HERON_BRUSH names the binary, XDG_CONFIG_HOME should be a scratch directory
# whose preferences pick the Quetoo profile and its game path).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
quetoo=${QUETOO:-$HOME/.local/opt/quetoo/quetoo}
user=${XDG_DATA_HOME:-$HOME/.local/share}/WickedOldGames/Quetoo/default

if [ "${1:-}" = "--remake" ]; then
    python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}"
fi

# quemap reads the map from its search path and writes the bsp beside it, under the
# write path it is given.
out=$here/build
mkdir -p "$out/maps"
for map in heron_yard heron_yard_playtest; do
    cp "$here/$map.map" "$out/maps/"
    # quemap searches the user folder too, and hashes the bsp it finds first for the
    # manifest: an installed copy from an earlier build would leave the new manifest
    # naming the old bsp, and the client then refuses the map.
    rm -f "$user/maps/$map.bsp" "$user/maps/$map.mf"
    "$quetoo/bin/quemap" -w "$out" -bsp "maps/$map.map"
done

# The client wants the map's manifest too: without heron_yard.mf it tries to download
# it from the server and gives up.
mkdir -p "$user/maps"
for map in heron_yard heron_yard_playtest; do
    cp "$out/maps/$map.bsp" "$out/maps/$map.mf" "$user/maps/"
done
cp "$here/playtest.cfg" "$user/"
echo "Play:      $quetoo/bin/quetoo +map heron_yard"
echo "Play-test: $here/playtest.py DISPLAY with the game on that display running"
echo "           +exec playtest.cfg +map heron_yard_playtest; screenshots land in $user/screenshots"
