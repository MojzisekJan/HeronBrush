"""Exercise two built editor processes and a real browser against LAN admission.

Set HERON_NODE and NODE_PATH for Playwright; HERON_CHROMIUM selects Chromium.
"""
import concurrent.futures
import http.server
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[2]


class Editor:
    def __init__(self, scratch):
        env = dict(os.environ, XDG_CONFIG_HOME=str(scratch / 'config'),
                   XDG_DATA_HOME=str(scratch / 'data'), XDG_CACHE_HOME=str(scratch / 'cache'))
        self.process = subprocess.Popen([str(ROOT / 'target/debug/heron-brush'), '--mcp'],
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=subprocess.DEVNULL, text=True, env=env)
        self.serial = 0

    def call(self, name, arguments=None):
        self.serial += 1
        self.process.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': self.serial,
            'method': 'tools/call', 'params': {'name': name, 'arguments': arguments or {}}}) + '\n')
        self.process.stdin.flush()
        response = json.loads(self.process.stdout.readline())
        assert not response.get('error'), response
        result = response['result']
        assert not result.get('isError'), result
        return '\n'.join(item.get('text', '') for item in result.get('content', []))

    def close(self):
        self.process.terminate()
        self.process.wait(timeout=5)


class QuietHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'text/html')
        self.end_headers()
        self.wfile.write(b'<!doctype html><title>Private session check</title>')

    def log_message(self, *args):
        pass


def main():
    with tempfile.TemporaryDirectory(prefix='heron-private-integration-') as folder:
        scratch = Path(folder)
        host, guest = Editor(scratch / 'host'), Editor(scratch / 'guest')
        try:
            host.call('add_shape', {'kind': 'cuboid', 'min': [-32, -32, -32],
                                   'max': [32, 32, 32], 'material': 'security-integration'})
            invitation = re.search(r'[\w.]+:\d+#[a-f0-9]{64}', host.call('host_session', {'port': 0}))[0]
            port, secret = invitation.split(':')[-1].split('#')
            invitation = f'127.0.0.1:{port}#{secret}'
            with concurrent.futures.ThreadPoolExecutor() as pool:
                joined = pool.submit(guest.call, 'join_session', {'target': invitation, 'nickname': 'native'})
                while not joined.done():
                    host.call('session_status')
                    time.sleep(.02)
                joined.result(timeout=5)
            assert 'security-integration' in guest.call('get_map')
            guest.call('add_shape', {'kind': 'cuboid', 'min': [64, 64, 64],
                                     'max': [96, 96, 96], 'material': 'guest-shared'})
            deadline = time.monotonic() + 5
            while 'guest-shared' not in host.call('get_map'):
                assert time.monotonic() < deadline, 'Guest edit did not reach host'
                time.sleep(.02)
            guest.call('leave_session')
            host.call('session_status')
            with concurrent.futures.ThreadPoolExecutor() as pool:
                joined = pool.submit(guest.call, 'join_session', {'target': invitation, 'nickname': 'native-reconnected'})
                while not joined.done():
                    host.call('session_status')
                    time.sleep(.02)
                joined.result(timeout=5)
            assert 'security-integration' in guest.call('get_map')
            print('PASS two editor processes share the map and reconnect with the private invitation', flush=True)
            server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), QuietHandler)
            threading.Thread(target=server.serve_forever, daemon=True).start()
            browser = r'''
const {chromium}=require('playwright');
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.HERON_CHROMIUM,headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage();
  await page.goto(process.env.HERON_CHECK_PAGE);
  await page.evaluate(async ({port,secret})=>{
   const receive=(credential)=>new Promise((resolve,reject)=>{
    const socket=new WebSocket('ws://127.0.0.1:'+port+'/');
    const timer=setTimeout(()=>{socket.close();reject(Error('admission timed out'));},6000);
    socket.onopen=()=>socket.send(JSON.stringify({Authenticate:credential}));
    socket.onmessage=event=>{const message=JSON.parse(event.data);if(credential==='wrong'||message.World){clearTimeout(timer);socket.close();resolve(message);}};
    socket.onclose=()=>{clearTimeout(timer);resolve(null);};
    socket.onerror=()=>{clearTimeout(timer);resolve(null);};
   });
   if(await receive('wrong')!==null)throw Error('Unauthenticated browser received session data');
   const world=await receive(secret);
   if(!world?.World)throw Error('Authenticated browser did not receive the world');
  },{port:process.env.HERON_CHECK_PORT,secret:process.env.HERON_CHECK_SECRET});
  console.log('PASS real Chromium rejects wrong credentials and receives the world with the invitation');
 }finally{await browser.close();}
})().catch(error=>{console.error(error.message);process.exitCode=1;});
'''
            process = subprocess.Popen([os.environ.get('HERON_NODE', 'node'), '-e', browser],
                                       env=dict(os.environ, HERON_CHECK_PORT=port, HERON_CHECK_SECRET=secret,
                                                HERON_CHECK_PAGE=f'http://127.0.0.1:{server.server_port}'))
            deadline = time.monotonic() + 30
            try:
                while process.poll() is None and time.monotonic() < deadline:
                    host.call('session_status')
                    time.sleep(.02)
                assert process.wait(timeout=2) == 0
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
                server.shutdown()
        finally:
            guest.close()
            host.close()


if __name__ == '__main__':
    main()
