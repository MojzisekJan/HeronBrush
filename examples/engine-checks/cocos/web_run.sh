#!/bin/bash
# usage: web-run.sh BUILD_DIR OUT.png [SECONDS]  -- serves a web build, opens it in an
# isolated Edge window, waits for "HERON done" in the console, screenshots and closes.
WORK=${TMPDIR:-/tmp}/heron-checks; mkdir -p "$WORK"
B=$1; OUT=$2; WAIT=${3:-30}; QUERY=${4:-}
PORT=$((8800 + RANDOM % 900))
(cd "$B" && exec python3 -m http.server $PORT >/dev/null 2>&1) & HTTP=$!
sleep 1
rm -rf $WORK/edge-prof
"/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge" --user-data-dir=$WORK/edge-prof --enable-logging=stderr --v=0 \
  --no-first-run --no-default-browser-check --window-position=60,60 --window-size=1280,760 --app=http://localhost:$PORT/index.html$QUERY 2> $WORK/edge.log &
EDGE=$!
for i in $(seq $((WAIT*2))); do grep -q 'HERON done' $WORK/edge.log && break; sleep 0.5; done
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $EDGE) to true" >/dev/null 2>&1 || true
sleep 0.8
screencapture -x -R60,60,1280,760 "$OUT"
kill $EDGE 2>/dev/null; sleep 1; pkill -f 'edge-prof' 2>/dev/null; kill $HTTP 2>/dev/null
grep -o 'HERON[^"]*\|Uncaught[^"]*\|Error[^"]*\|rror:[^"]*' $WORK/edge.log | head -30
