#!/usr/bin/env python3
"""Exercise the production bridge in a real opaque-origin iframe.

No IndexedDB mocks: the browser rejects access with SecurityError. Recovery
payload is inspected at the native Blob URL boundary; download completion on the
OS is not asserted. Open the printed URL in an installed browser, then use the
visible Retry and Download buttons inside the frame.
"""
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PARENT = '''<!doctype html><meta charset="utf-8"><title>Heron denied storage check</title>
<h1>Unavailable storage recovery</h1><p>This disposable frame has an opaque origin. Browser storage is naturally denied.</p>
<button id="control">Download normal-origin control map</button>
<pre id="result" style="white-space:pre-wrap">Waiting for frame</pre>
<iframe id="check" sandbox="allow-scripts allow-downloads" src="/denied" style="width:100%;height:380px;border:1px solid #999"></iframe>
<script>window.addEventListener('message', event => {
 if(event.source!==document.querySelector('iframe').contentWindow || event.origin!=='null' || event.data?.type!=='heron-denied-check')return;
 document.querySelector('#result').textContent=JSON.stringify(event.data,null,2);
});</script>
<script type="module">
import {download} from '/web_files.js';
document.querySelector('#control').onclick=()=>download('normal-origin-control.map',new TextEncoder().encode('Heron recovery control'));
</script>'''
FRAME = r'''<!doctype html><meta charset="utf-8"><title>Opaque storage frame</title>
<h2>Production storage panel</h2><p id="phase">Starting</p>
<button id="multiple">Create multi-file recovery</button>
<script type="module">
const report = data => parent.postMessage({type:'heron-denied-check',...data},'*');
try {
 let denied=false;
 try {indexedDB.open('heron-denied-check');} catch(error) {denied=error.name==='SecurityError';}
 if(!denied)throw Error('IndexedDB was not naturally denied');
 const bridge=await import('/web_files.js');
 await bridge.load_files();
 const expected=new TextEncoder().encode('// Recovery fixture\n{\n"classname" "worldspawn"\n}\n');
 let multiple=false;
 const input=expected.slice();
 const pending=bridge.store_file('/heron-brush/picked/denied/recovery.map',input); input.fill(0); await pending;
 let downloadRequests=0;
 const create=URL.createObjectURL.bind(URL);
 URL.createObjectURL=blob=>{
  const url=create(blob);
  blob.arrayBuffer().then(buffer=>{
   const actual=new Uint8Array(buffer);
   const equal=actual.length===expected.length && actual.every((b,i)=>b===expected[i]);
   if(multiple){report({phase:'zip-download-requested',naturalSecurityError:true,payloadBytes:actual.length,panel:document.querySelector('aside').textContent});return;}
   downloadRequests++;
   report({phase:equal?'recovery-payload-pass':'FAIL',naturalSecurityError:true,downloadRequests,payloadBytes:actual.length,payloadMatches:equal,panel:document.querySelector('aside').textContent});
  });
  return url;
 };
 document.querySelector('#multiple').onclick=async()=>{
  multiple=true;
  for(let i=0;i<25;i++)await bridge.store_file(`/project/level-${i}/map.map`,new TextEncoder().encode(`level ${i}\n`));
  await bridge.store_file('/project/český.txt',new TextEncoder().encode('žluťoučký\n'));
  report({phase:'multiple-ready',naturalSecurityError:true,panel:document.querySelector('aside').textContent});
 };
 let retries=0;
 const panel=document.querySelector('aside');
 // Observe actual button clicks without replacing production handlers.
 panel.addEventListener('click',event=>{
  if(event.target.textContent==='Retry storage'){
   retries++;
   setTimeout(()=>report({phase:'retry-still-protected',naturalSecurityError:true,retries,panel:panel.textContent,buttons:[...panel.querySelectorAll('button')].map(b=>b.textContent)}),100);
  }
 });
 document.querySelector('#phase').textContent='Storage denied. Use the real production buttons below.';
 report({phase:'ready',naturalSecurityError:true,panel:panel.textContent,buttons:[...panel.querySelectorAll('button')].map(b=>b.textContent)});
}catch(error){document.querySelector('#phase').textContent='FAIL: '+error.stack;report({phase:'FAIL',error:String(error.stack)});}
</script>'''

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/':
            data, kind = PARENT.encode(), 'text/html'
        elif self.path == '/denied':
            data, kind = FRAME.encode(), 'text/html'
        elif self.path in ['/web_files.js', '/playtest_protocol.js']:
            data, kind = (ROOT / 'crates/heron-brush/src' / self.path[1:]).read_bytes(), 'text/javascript'
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header('Content-Type', kind)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Cache-Control', 'no-store')
        self.end_headers()
        self.wfile.write(data)

if __name__ == '__main__':
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    print(f'http://127.0.0.1:{server.server_port}/', flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.server_close()
