#!/usr/bin/env python3
"""Describe the worker artifacts Trunk just built, preserving cache-version identity."""
import hashlib
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
js = root / 'heron-brush-export-web.js'
wasm = root / 'heron-brush-export-web_bg.wasm'
version = hashlib.sha256(b''.join(path.read_bytes() for path in [js, wasm, root / 'playtest_worker.mjs', root / 'playtest_protocol.js'])).hexdigest()
(root / 'playtest-worker.json').write_text(json.dumps({'format':1, 'version':version, 'js':js.name, 'wasm':wasm.name}, separators=(',', ':')) + '\n')
