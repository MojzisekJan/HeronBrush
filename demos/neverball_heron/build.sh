#!/bin/sh
# Compiles heron_lift.map with Neverball's mapc and installs the level in Neverball's user
# data folder, from where `neverball -l map-heron/heron_lift.sol` loads it.
#
# NEVERBALL_DIR is where the Debian packages were unpacked; see
# docs/game-profile-tests.md. NEVERBALL_HOME is the home whose .neverball folder gets the
# level (default: $HOME). Pass --remake to build the map again in Heron Brush first
# (HERON_BRUSH names the binary, XDG_CONFIG_HOME should be a scratch directory).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
neverball=${NEVERBALL_DIR:-$HOME/.local/opt/neverball}
root=$neverball/root
data=$root/usr/share/games/neverball
user=${NEVERBALL_HOME:-$HOME}/.neverball

if [ "${1:-}" = "--remake" ]; then
    python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}"
fi

out=$here/build
mkdir -p "$out"
cp "$here/heron_lift.map" "$out/"
LD_LIBRARY_PATH=$root/usr/lib/x86_64-linux-gnu HOME=${NEVERBALL_HOME:-$HOME} \
    "$root/usr/games/mapc" "$out/heron_lift.map" "$data"

mkdir -p "$user/map-heron"
cp "$out/heron_lift.sol" "$user/map-heron/"
echo "Play: LD_LIBRARY_PATH=$root/usr/lib/x86_64-linux-gnu HOME=${NEVERBALL_HOME:-$HOME} \\"
echo "      $root/usr/games/neverball --data $data -l map-heron/heron_lift.sol"
