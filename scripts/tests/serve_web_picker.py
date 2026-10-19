#!/usr/bin/env python3
"""Production picker bridge in an opaque origin, with actual FSA SecurityErrors.

Use only generated fixture files. Native cancellation is covered by the Node
suite; this page verifies unavailable APIs, in-page fallback and real downloads.
"""
from http.server import ThreadingHTTPServer
import os
from serve_web_storage_denied import Handler as StorageHandler

INDEX = '''<!doctype html><meta charset="utf-8"><title>Heron picker check</title>
<h1>Picker fallback check</h1><pre id="result" style="white-space:pre-wrap">Waiting</pre>
<iframe sandbox="allow-scripts allow-downloads" src="/picker" style="width:100%;height:500px"></iframe>
<script>addEventListener('message',event=>{
 if(event.source===document.querySelector('iframe').contentWindow && event.origin==='null' && event.data?.type==='heron-picker-check')
  document.querySelector('#result').textContent=JSON.stringify(event.data,null,2);
});</script>'''
FRAME = r'''<!doctype html><meta charset="utf-8"><title>Restricted picker frame</title>
<h2>Production picker actions</h2>
<button id="probe-open">Probe native open</button><button id="probe-save">Probe native save</button>
<button id="open">Open map through bridge</button><button id="save">Save map through bridge</button>
<script type="module">
import * as bridge from '/web_files.js';
const report=data=>parent.postMessage({type:'heron-picker-check',...data},'*');
const payload=new TextEncoder().encode('// Heron picker fixture\n{\n"classname" "worldspawn"\n}\n');
for(const kind of ['open','save'])document.querySelector('#probe-'+kind).onclick=async()=>{
 try{
  if(kind==='open')await window.showOpenFilePicker();else await window.showSaveFilePicker();
  report({phase:'native-succeeded',kind});
 }catch(error){report({phase:'native-error',kind,error:error.name,message:error.message});}
};
document.querySelector('#open').onclick=async()=>{
 report({phase:'waiting-open'});
 try{
  const result=await bridge.pick_open(['map']);
  report(result?{phase:'bridge-open-result',name:result[0],bytes:result[1].length,text:new TextDecoder().decode(result[1])}:{phase:'bridge-open-null'});
 }catch(error){report({phase:'FAIL',error:String(error.stack)});}
};
document.querySelector('#save').onclick=async()=>{
 report({phase:'waiting-save'});
 try{
  const name=await bridge.pick_save('heron-picker-check.map',['map']);
  if(!name){report({phase:'bridge-save-null'});return;}
  const path='/heron/picker-check/'+name;bridge.adopt_handle(path);
  await bridge.save_user_file(path,name,payload);
  report({phase:'bridge-save-result',name,bytes:payload.length});
 }catch(error){report({phase:'FAIL',error:String(error.stack)});}
};
report({phase:'ready',openAPI:typeof window.showOpenFilePicker,saveAPI:typeof window.showSaveFilePicker});
</script>'''

class Handler(StorageHandler):
    def do_GET(self):
        page = INDEX if self.path == '/' else FRAME if self.path == '/picker' else None
        if page is None:
            return super().do_GET()
        self.send_response(200)
        self.send_header('Content-Type', 'text/html; charset=utf-8')
        self.send_header('Cache-Control', 'no-store')
        self.end_headers()
        self.wfile.write(page.encode())

if __name__ == '__main__':
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    print(f'http://127.0.0.1:{server.server_port}/', flush=True)
    print(f'PID {os.getpid()}', flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.server_close()
