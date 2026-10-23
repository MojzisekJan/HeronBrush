#!/bin/sh
# Compiles heron_keep_hexen2.map (heron_keep.map exported in Hexen II's dialect) and
# installs the bsp in the Hexen II demo's data1. The demo engine refuses -game ("You must
# have the full version of Hexen II to play modified games"), so the map goes beside the
# demo's own.
#
# The compilers are ericw-tools' (qbsp -hexen2, vis, light), or with --raven Raven's own,
# as Hammer of Thyrion ships them. HEXEN2_DIR is where the demo and Hammer of Thyrion were
# unpacked, ERICW_TOOLS where ericw-tools is; see docs/game-profile-tests.md. Pass
# --remake to build the map again in Heron Brush first (HERON_BRUSH names the binary,
# XDG_CONFIG_HOME should be a scratch directory).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
hexen2=${HEXEN2_DIR:-$HOME/.local/opt/hexen2}
tools=${ERICW_TOOLS:-$HOME/.local/opt/quake/ericw-tools}
game=$hexen2/hexen2demo_nov1997

# The demo has no texture wad, so one is gathered from its maps. The map names it
# relative to its own folder, which is how qbsp and Heron Brush both find it.
mkdir -p "$here/wads"
if [ ! -f "$here/wads/h2demo.wad" ]; then
    python3 -I "$here/extract_wad.py" "$game/data1/pak0.pak" "$here/wads/h2demo.wad"
fi

raven=
for arg in "$@"; do
    case $arg in
        --remake) python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}" ;;
        --raven) raven=yes ;;
        *) echo "unknown option $arg" >&2; exit 2 ;;
    esac
done

out=$here/build
mkdir -p "$out"
cp "$here/heron_keep_hexen2.map" "$out/heron_keep.map"
cp -r "$here/wads" "$out/"
if [ -n "$raven" ]; then
    # Raven's tools take the name without the extension and work in its folder.
    utils=$hexen2/hot-utils/utils
    (cd "$out" && "$utils/qbsp" heron_keep.map && "$utils/vis" heron_keep \
        && "$utils/light" -extra heron_keep)
else
    "$tools/qbsp" -hexen2 "$out/heron_keep.map"
    "$tools/vis" "$out/heron_keep.bsp"
    "$tools/light" -extra4 "$out/heron_keep.bsp"
fi

mkdir -p "$game/data1/maps"
cp "$out/heron_keep.bsp" "$game/data1/maps/"
cp "$here/playtest.cfg" "$game/data1/"
echo "Play:      $hexen2/run.sh +map heron_keep"
echo "Play-test: add -condebug +exec playtest.cfg"
