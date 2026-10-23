#!/bin/sh
# Compiles heron_court.map with q3map2 (bsp, vis, light), builds the bots' navigation
# file with mbspc and packs both, with the arena file, into heron_court.pk3 in
# OpenArena's baseoa under the home path.
#
# OA_DIR is the OpenArena 0.8.8 folder (the one holding baseoa/pak0.pk3); Q3MAP2 and
# MBSPC name the compilers, from NetRadiant-custom. OA_HOME is the game's home path,
# ~/.openarena unless set: start the game with +set fs_homepath "$OA_HOME" when it is
# not the default. See docs/game-profile-tests.md. Pass --remake to build the map again
# in Heron Brush first (HERON_BRUSH names the binary, XDG_CONFIG_HOME should be a
# scratch directory whose preferences pick the Quake 3 profile, the OpenArena folder
# and the mod baseoa), and --primitives to compile the brush-primitive export instead.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
opt=$HOME/.local/opt/openarena
game=${OA_DIR:-$opt/openarena-0.8.8}
tools=$opt/netradiant-custom/squashfs-root/usr/bin
q3map2=${Q3MAP2:-$tools/q3map2}
mbspc=${MBSPC:-$tools/mbspc}
home=${OA_HOME:-$HOME/.openarena}

source=heron_court.map
for arg in "$@"; do
    case $arg in
        --remake) python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}" ;;
        --primitives) source=heron_court_brushdef.map ;;
        *) echo "unknown option $arg" >&2; exit 2 ;;
    esac
done

out=$here/build
rm -rf "$out"
mkdir -p "$out/maps" "$out/scripts"
cp "$here/$source" "$out/maps/heron_court.map"
cp "$here/heron_court.arena" "$out/scripts/"

# q3map2 has no OpenArena game of its own: it is Quake 3 with baseoa for baseq3. The
# home base points at a folder that does not exist, so q3map2 reads only the install.
fs="-game quake3 -fs_basepath $game -fs_basegame baseoa -fs_homebase .heron-none"
map=$out/maps/heron_court
# shellcheck disable=SC2086
"$q3map2" $fs -meta -patchmeta "$map.map"
# shellcheck disable=SC2086
"$q3map2" $fs -vis -saveprt "$map.bsp"
# shellcheck disable=SC2086
"$q3map2" $fs -light -fast -patchshadows -samples 2 -bounce 2 "$map.bsp"
# The bots find their way with an .aas file beside the bsp. mbspc keeps paths in short
# buffers and garbles a long one, so it is given the file name only.
(cd "$out/maps" && "$mbspc" -forcesidesvisible -optimize -bsp2aas heron_court.bsp)

(cd "$out" && rm -f heron_court.pk3 \
    && zip -q heron_court.pk3 maps/heron_court.bsp maps/heron_court.aas \
        scripts/heron_court.arena)
mkdir -p "$home/baseoa"
cp "$out/heron_court.pk3" "$home/baseoa/"
cp "$here/playtest.cfg" "$home/baseoa/"
echo "Installed $home/baseoa/heron_court.pk3"
echo "Play:      run-openarena +set fs_homepath $home +devmap heron_court"
echo "Play-test: add +exec playtest.cfg; screenshots land in $home/baseoa/screenshots"
