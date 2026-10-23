#!/bin/sh
# Compiles heron_wharf_h2.map (the map exported in Quake 2's dialect) with q2tools-220
# and installs the bsp where the engine reads the demo's maps.
#
# H2_BASE is the demo's Base folder (its loose textures/*.m8 are read for each face's
# size, flags and colour); Q2TOOL is q2tools-220's q2tool; H2_MAPS is the engine's maps
# folder. See docs/game-profile-tests.md. Pass --remake to build the map again in Heron
# Brush first (HERON_BRUSH names the binary, XDG_CONFIG_HOME should be a scratch
# directory whose preferences pick the Heretic 2 profile and the demo's folder).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
base=${H2_BASE:-$HOME/.local/opt/heretic2/h2demo/Base}
tool=${Q2TOOL:-$HOME/.local/opt/heretic2/q2tools/q2tool}
maps=${H2_MAPS:-$HOME/.local/opt/heretic2/0lvin/baseq2/maps}

if [ "${1:-}" = "--remake" ]; then
    python3 "$here/make_map.py" "${HERON_BRUSH:-heron-brush}"
fi

out=$here/build
mkdir -p "$out"
cp "$here/heron_wharf_h2.map" "$out/heron_wharf.map"
# -h2tex reads textures/<name>.m8 under the mod folder instead of .wal files: a face
# with no flags of its own takes the .m8's, which is what makes general/sky sky and
# gives each face its footstep material. -rad lights it from the light entities;
# -scale 2 brings its lightmaps up to about the level of the demo's own maps.
"$tool" -bsp -vis -rad -h2tex -extra -scale 2 -moddir "$base" "$out/heron_wharf.map"

mkdir -p "$maps"
cp "$out/heron_wharf.bsp" "$maps/"
cp "$here/playtest.cfg" "$here/lamps.cfg" "$maps/../"
echo "Play:      ~/.local/opt/heretic2/run-0lvin +map heron_wharf"
echo "Play-test: add +exec playtest.cfg or lamps.cfg"
