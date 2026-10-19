#!/usr/bin/env python3
"""Serve the local PBR browser check and existing vendored Three.js on localhost."""
import argparse, http.server, urllib.parse
from pathlib import Path
repo=Path(__file__).resolve().parents[2]
p=argparse.ArgumentParser();p.add_argument('fixtures',type=Path);p.add_argument('--port',type=int,default=0);args=p.parse_args();fixtures=args.fixtures.resolve()
class Handler(http.server.SimpleHTTPRequestHandler):
    def translate_path(self,path):
        path=urllib.parse.unquote(urllib.parse.urlsplit(path).path)
        if path=='/':return str(repo/'examples/editor-checks/pbr-check.html')
        if path.startswith('/fixtures/'):root=fixtures;relative=path[len('/fixtures/'):]
        elif path.startswith('/three/'):root=repo/'crates/heron-brush-web/vendor/three';relative=path[len('/three/'):]
        else:return str(fixtures/'not-found')
        target=(root/relative).resolve()
        return str(target) if target.is_relative_to(root) else str(fixtures/'not-found')
server=http.server.ThreadingHTTPServer(('127.0.0.1',args.port),Handler)
print(f'http://127.0.0.1:{server.server_port}/',flush=True)
server.serve_forever()
