#!/usr/bin/env python3
"""Serve an existing web release locally with gzip, ETags and realistic cache rules.

Diagnostic server only. No public hosting, TLS or production security configuration.
Open the printed URL in an installed browser. Warm reload should validate mutable
files and reuse content-hashed assets. Startup times include this host's browser/GPU.
"""
import argparse
import gzip
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import mimetypes
from pathlib import Path
import re
from urllib.parse import unquote, urlsplit


def gzip_allowed(header):
    for item in header.lower().split(','):
        parts = item.strip().split(';')
        if parts[0] != 'gzip':
            continue
        try:
            return all(float(part.strip()[2:]) > 0 for part in parts[1:] if part.strip().startswith('q='))
        except ValueError:
            return False
    return False


def serve(root, port, metrics=False):
    # Compress before measurements so the first browser run does not pay build work.
    files = {}
    for path in root.rglob('*'):
        if not path.is_file():
            continue
        name = '/' + path.relative_to(root).as_posix()
        raw = path.read_bytes()
        if metrics and name == '/index.html':
            probe = b"""<script>
(() => {
 let seen = false;
 const observer = new MutationObserver(() => {
  if (document.getElementById('loading')) seen = true;
  else if (seen) {
   const readyMs = Number(performance.now().toFixed(1));
   document.documentElement.dataset.heronReadyMs = String(readyMs);
   const resources = performance.getEntriesByType('resource').filter(e => new URL(e.name).origin === location.origin).map(e => ({
    path:new URL(e.name).pathname, transferBytes:e.transferSize,
    encodedBytes:e.encodedBodySize, decodedBytes:e.decodedBodySize,
    durationMs:Number(e.duration.toFixed(1))
   }));
   const navigation = performance.getEntriesByType('navigation')[0];
   document.documentElement.dataset.heronStartup = JSON.stringify({
    readyMs, transferBytes:resources.reduce((sum,e)=>sum+e.transferBytes, navigation?.transferSize || 0),
    heapUsedBytes:performance.memory?.usedJSHeapSize, resources
   });
   observer.disconnect();
  }
 });
 observer.observe(document, {childList:true, subtree:true});
})();
</script>"""
            raw = raw.replace(b'<head>', b'<head>' + probe, 1)
        packed = gzip.compress(raw, compresslevel=6, mtime=0)
        representations = {False: raw}
        if len(packed) < len(raw):
            representations[True] = packed
        files[name] = (representations, {
            encoded: '"' + hashlib.sha256(data).hexdigest() + '"'
            for encoded, data in representations.items()
        })
    if '/index.html' not in files:
        raise SystemExit('No index.html. Supply a completed Trunk release directory.')

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            name = unquote(urlsplit(self.path).path)
            if name.endswith('/'):
                name += 'index.html'
            record = files.get(name)
            if record is None:
                self.send_error(404)
                return
            representations, tags = record
            encoded = gzip_allowed(self.headers.get('Accept-Encoding', '')) and True in representations
            data, tag = representations[encoded], tags[encoded]
            cached = self.headers.get('If-None-Match') == tag
            self.send_response(304 if cached else 200)
            self.send_header('ETag', tag)
            self.send_header('Vary', 'Accept-Encoding')
            immutable = bool(re.search(r'-[0-9a-f]{8,}(?:_bg)?\.(?:wasm|js|mjs|bin)$', name))
            self.send_header('Cache-Control', 'public, max-age=31536000, immutable' if immutable else 'no-cache')
            self.send_header('Content-Type', 'application/wasm' if name.endswith('.wasm') else mimetypes.guess_type(name)[0] or 'application/octet-stream')
            if encoded:
                self.send_header('Content-Encoding', 'gzip')
            if not cached:
                self.send_header('Content-Length', str(len(data)))
            self.end_headers()
            if not cached:
                self.wfile.write(data)

    server = ThreadingHTTPServer(('127.0.0.1', port), Handler)
    print(f'http://127.0.0.1:{server.server_port}/', flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.server_close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--port', type=int, default=0)
    parser.add_argument('--metrics', action='store_true', help='Record loader-removal time in the HTML root dataset')
    args = parser.parse_args()
    serve(args.directory.resolve(strict=True), args.port, args.metrics)
