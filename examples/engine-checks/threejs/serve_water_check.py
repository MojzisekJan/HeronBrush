#!/usr/bin/env python3
"""Offline browser fixture over a fresh water export, including the independent importer."""
import argparse
from pathlib import Path
from http.server import BaseHTTPRequestHandler,HTTPServer
ROOT=Path(__file__).resolve().parents[3]
p=argparse.ArgumentParser();p.add_argument('--port',type=int,default=38629);p.add_argument('--export',type=Path,default=Path('/tmp/heron-water-check/three'));args=p.parse_args()
def local(data):
 return data.replace('https://cdn.jsdelivr.net/npm/three@0.186.0/build/','/vendor/build/').replace('https://cdn.jsdelivr.net/npm/three@0.186.0/examples/jsm/','/vendor/examples/jsm/')
TEST='''<script type="module">
import {runWaterCheck} from '/examples/engine-checks/threejs/water_checks.js';
const timer=setInterval(()=>{if(!window.heronPlaytest)return;clearInterval(timer);const result=document.createElement('pre');result.id='water-result';result.style='position:fixed;top:0;left:0;background:#123;color:white;padding:12px;z-index:9999';document.body.append(result);try{const p=window.heronPlaytest,report=runWaterCheck(p);document.getElementById('menu')?.setAttribute('hidden','');const above=new URLSearchParams(location.search).get('view')==='surface';p.teleport(new p.THREE.Vector3(...(above?[6,2,6]:[3,-3,0])));p.face(Math.PI/4);p.step(0);p.camera.rotation.x=above?-0.6:-0.1;result.textContent='WATER_CHECK Three.js PASS '+JSON.stringify(report);window.waterCheckResult=report;setTimeout(()=>{if(!above&&!p.scene.fog)result.textContent='WATER_CHECK FAIL underwater fog';},100);}catch(error){result.textContent='WATER_CHECK FAIL '+error;}},50);
</script>'''
class Handler(BaseHTTPRequestHandler):
 def do_GET(self):
  url=self.path.split('?')[0]
  if url in ['/','/importer']:
   source=ROOT/'examples/engine-checks/threejs/index.html' if url=='/importer' else args.export/'water.html'
   data=local(source.read_text()).replace('src="./src/main.js"','src="/examples/engine-checks/threejs/src/main.js"').replace('</body>',TEST+'</body>').encode();mime='text/html'
  else:
   base,relative=(ROOT/'crates/heron-brush-web/vendor/three',url[8:]) if url.startswith('/vendor/') else (args.export,url[10:]) if url.startswith('/fixtures/') else (ROOT,url[1:])
   file=(base/relative).resolve()
   if not file.is_relative_to(base.resolve()) or not file.is_file():self.send_error(404);return
   data=file.read_bytes();mime={'.js':'text/javascript','.json':'application/json','.png':'image/png','.html':'text/html'}.get(file.suffix,'application/octet-stream')
  self.send_response(200);self.send_header('Content-Type',mime);self.end_headers();self.wfile.write(data)
print(f'http://127.0.0.1:{args.port}/?verify=1&view=surface',flush=True)
print(f'http://127.0.0.1:{args.port}/importer?verify=1&level=/fixtures/water.json&view=surface',flush=True)
HTTPServer(('127.0.0.1',args.port),Handler).serve_forever()
