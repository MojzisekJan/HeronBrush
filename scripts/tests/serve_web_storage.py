#!/usr/bin/env python3
"""Serve a disposable browser IndexedDB check on a fresh localhost origin.

Open the printed URL in an installed browser and wait for PASS after an automatic
reload. Quota failure is injected; actual disk exhaustion is not tested.
"""
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
from pathlib import Path
root=Path(__file__).resolve().parents[2]
page='''<!doctype html><title>Heron web storage check</title><pre id="result">Running browser IndexedDB checks</pre><script type="module">
import * as bridge from '/web_files.js';
const result=document.querySelector('#result');
const key='/heron-brush/picked/browser-test/level.map';
const check=(value,message)=>{if(!value)throw Error(message)};
try {
 if(sessionStorage.getItem('reload-check')) {
  const files=await bridge.load_files();
  check(files.some(([path,bytes])=>path===key && bytes[0]===73),'saved bytes did not survive reload');
  await bridge.delete_file(key); sessionStorage.removeItem('reload-check');
  // Use a real origin-private writable stream, without an external file picker.
  const directory=await navigator.storage.getDirectory();
  const physical=await directory.getFileHandle('ordered.map',{create:true});
  const handle={name:'ordered.map',queryPermission:async()=> 'granted',createWritable:()=>physical.createWritable()};
  window.showSaveFilePicker=async()=>handle;
  await bridge.pick_save('ordered.map',['map']);bridge.adopt_handle('/ordered.map');
  const bytes=new Uint8Array([91]);
  const first=bridge.save_user_file('/ordered.map','ordered.map',bytes);bytes[0]=0;
  const second=bridge.save_user_file('/ordered.map','ordered.map',new Uint8Array([92]));
  await Promise.all([first,second]);
  check(new Uint8Array(await (await physical.getFile()).arrayBuffer())[0]===92,'real writable stream lost save order');
  check(!document.querySelector('aside').textContent.includes('Could not update'),'real stream unexpectedly fell back to download');
  await directory.removeEntry('ordered.map');
  result.textContent='PASS: real IndexedDB retry/reload, same-name isolation, and ordered real OPFS writable-stream saves';
 }else{
  await bridge.load_files();
  const original=IDBDatabase.prototype.transaction;
  IDBDatabase.prototype.transaction=function(store,mode,...rest){
   if(store==='files' && mode==='readwrite')throw new DOMException('Simulated full storage','QuotaExceededError');
   return original.call(this,store,mode,...rest);
  };
  await bridge.store_file(key,new Uint8Array([73]));
  check(document.querySelector('aside').textContent.includes('Download level.map'),'missing visible recovery download');
  IDBDatabase.prototype.transaction=original;
  [...document.querySelectorAll('button')].find(b=>b.textContent==='Retry storage').click();
  for(let i=0;i<200 && !document.querySelector('aside').hidden;i++)await new Promise(r=>setTimeout(r,10));
  check(document.querySelector('aside').hidden,'retry did not commit');
  check(await bridge.picked_file_path('/heron-brush/picked','level.map')!==await bridge.picked_file_path('/heron-brush/picked','level.map'),'same-name paths collide');
  sessionStorage.setItem('reload-check','1');location.reload();
 }
}catch(error){result.textContent='FAIL: '+error.stack;}
</script>'''
class Handler(BaseHTTPRequestHandler):
 def do_GET(self):
  if self.path=='/':data=page.encode();kind='text/html'
  elif self.path in ['/web_files.js','/playtest_protocol.js']:
   data=(root/'crates/heron-brush/src'/self.path[1:]).read_bytes();kind='text/javascript'
  else:self.send_error(404);return
  self.send_response(200);self.send_header('Content-Type',kind);self.end_headers();self.wfile.write(data)
server=ThreadingHTTPServer(('127.0.0.1',0),Handler)
print(f'http://127.0.0.1:{server.server_port}/',flush=True)
try:
 server.serve_forever()
except KeyboardInterrupt:
 server.server_close()
