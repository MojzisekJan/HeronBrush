#!/bin/bash
# usage: run.sh PROJECT_DIR OUT.png [TRIGGER TARGET [QUERY]]
WORK=${TMPDIR:-/tmp}/heron-checks; mkdir -p "$WORK"
set -e
PORT=$((8800 + RANDOM % 900))
P=$1; OUT=$2
"/Applications/GDevelop 5.app/Contents/MacOS/GDevelop 5" "$P/game.json" --run-command EXPORT_HTML5_EXTERNAL > "$P/cli.log" 2>&1
grep -E 'STATUS|finished|rror' "$P/cli.log" | grep -v 'storage provider' | head -5
B=$P/build
cp "$(dirname "$0")/probe.js" $B/heron-probe.js
python3 - "$B/index.html" "$3" "$4" <<'PY'
import sys,re
p,trig,targ=sys.argv[1:4]; s=open(p).read()
if 'heron-probe' not in s:
    s=s.replace('</body>','<script>window.__heronTrigger=%r;window.__heronTarget=%r;</script><script src="heron-probe.js"></script></body>'%(trig or '',targ or ''))
s=re.sub(r'(var|const|let)\s+game\s*=\s*new gdjs.RuntimeGame\(([^;]*)\);', lambda m: m.group(0)+' window.__heronGame=game;', s)
open(p,'w').write(s)
PY
grep -c __heronGame $B/index.html
(cd $B && python3 -m http.server $PORT >/dev/null 2>&1 & echo $! > $WORK/gd-http.pid)
sleep 1
rm -rf $WORK/edge-prof
"/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge" --user-data-dir=$WORK/edge-prof --enable-logging=stderr --v=0 \
  --no-first-run --no-default-browser-check --window-position=60,60 --window-size=1280,760 --app=http://localhost:$PORT/index.html$5 2> $WORK/edge.log &
EDGE=$!
for i in $(seq 60); do grep -q 'HERON done' $WORK/edge.log && break; sleep 0.5; done
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $EDGE) to true" >/dev/null 2>&1 || true
sleep 0.8
screencapture -x -R60,60,1280,760 "$OUT"
kill $EDGE 2>/dev/null; sleep 1; pkill -f 'edge-prof' 2>/dev/null || true
kill $(cat $WORK/gd-http.pid) 2>/dev/null || true; pkill -f "http.server $PORT" 2>/dev/null || true
grep -o 'HERON[^"]*\|Uncaught[^"]*\|Error[^"]*' $WORK/edge.log | head -20
