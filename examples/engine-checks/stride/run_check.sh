#!/bin/bash
# usage: stride-check.sh LEVEL OUT.png [TRIGGER TARGET]
WORK=${TMPDIR:-/tmp}/heron-checks; mkdir -p "$WORK"
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib VK_ICD_FILENAMES=/opt/homebrew/etc/vulkan/icd.d/MoltenVK_icd.json MVK_CONFIG_LOG_LEVEL=0
export HERON_LEVEL=$1 HERON_TRIGGER=$3 HERON_TARGET=$4
cd "${STRIDE_GAME_BIN:?set STRIDE_GAME_BIN to the game's Bin/Linux/Debug folder}"
./*.Linux > $WORK/stride-$1.log 2>&1 & P=$!
for i in $(seq 120); do grep -q 'HERON done' $WORK/stride-$1.log && break; kill -0 $P 2>/dev/null || break; sleep 0.25; done
B=$(osascript -e 'tell application "System Events" to tell (first process whose unix id is '$P') to get {position, size} of window 1' 2>/dev/null | tr -d ' ')
IFS=, read x y w h <<< "$B"
osascript -e 'tell application "System Events" to set frontmost of (first process whose unix id is '$P') to true' >/dev/null 2>&1; sleep 0.5
screencapture -x -R$x,$y,$w,$h "$2"
kill $P 2>/dev/null; wait $P 2>/dev/null
grep -E 'HERON|xception' $WORK/stride-$1.log | grep -v 'HERON done'
