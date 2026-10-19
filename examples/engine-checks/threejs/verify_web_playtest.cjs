// Offline Chromium integration check using a fresh generated motion_lab HTML.
// Usage: node verify_web_playtest.cjs /tmp/heron-motion-three/motion_lab.html
const {chromium} = require('playwright');
const fs = require('node:fs'), http = require('node:http'), path = require('node:path');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '../../..');
const web = path.join(root, 'crates/heron-brush-web');
let html = fs.readFileSync(process.argv[2], 'utf8').replace("new URLSearchParams(location.search).has('verify')", 'true');
const server = http.createServer((req,res) => {
  const url = new URL(req.url,'http://localhost');
  if(url.pathname === '/controller.html') {
    res.setHeader('Content-Type','text/html');
    res.end('<button id="open">Play</button><script type="module">window.bridge=await import("/bridge.js");document.querySelector("button").onclick=()=>bridge.open_playtest();</script>');return;
  }
  const built=path.resolve(web,'dist','.'+url.pathname);
  if(url.pathname !== '/bridge.js' && built.startsWith(path.join(web,'dist')+path.sep) && fs.existsSync(built) && fs.statSync(built).isFile()) {
    const types={'.html':'text/html','.js':'text/javascript','.mjs':'text/javascript','.wasm':'application/wasm','.png':'image/png','.txt':'text/plain'};
    res.setHeader('Content-Type',types[path.extname(built)]||'application/octet-stream');res.end(fs.readFileSync(built));return;
  }
  if(url.pathname === '/playtest_protocol.js') {res.setHeader('Content-Type','text/javascript');res.end(fs.readFileSync(path.join(root,'crates/heron-brush/src/playtest_protocol.js')));return;}
  const file = url.pathname === '/bridge.js' ? path.join(root,'crates/heron-brush/src/web_files.js') : path.resolve(web,'.'+decodeURIComponent(url.pathname));
  if(!file.startsWith(web+path.sep) && url.pathname !== '/bridge.js') {res.writeHead(403).end();return;}
  if(!fs.existsSync(file) || !fs.statSync(file).isFile()) {res.writeHead(404).end();return;}
  res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')||file.endsWith('.mjs')?'text/javascript':'application/octet-stream');res.end(fs.readFileSync(file));
});
(async () => {
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const browser=await chromium.launch({headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
  try {
    const context=await browser.newContext();
    const remote=[]; await context.route('**/*',route=> {
      if(!route.request().url().startsWith('http://127.0.0.1:') && !route.request().url().startsWith('data:')) {remote.push(route.request().url());return route.abort();}
      return route.continue();
    });
    const editor=await context.newPage();const base=`http://127.0.0.1:${server.address().port}`;
    await editor.goto(base+'/controller.html');await editor.waitForFunction(()=>window.bridge);
    const popup=context.waitForEvent('page');await editor.click('#open');const viewer=await popup;
    await viewer.waitForLoadState();
    await editor.evaluate(html=>bridge.publish_playtest(html,'map-a'),html);
    const current=()=>viewer.frames().find(f=>f.parentFrame());
    await viewer.waitForFunction(()=>Array.from(document.querySelectorAll('iframe')).some(f=>f.style.visibility==='visible'));
    let frame=current();await frame.waitForFunction(()=>window.heronPlaytest);
    await frame.evaluate(()=>{
      heronPlaytest.teleport(new heronPlaytest.THREE.Vector3(1,4,2));heronPlaytest.face(.7);
      const animation=[...heronPlaytest.animations.values()][0];animation.seek(1.5);animation.pause();animation.update(0);
      const trigger=heronPlaytest.triggers[0];trigger.wait=3;trigger.fired=true;trigger.pending=true;trigger.inside.add('player');
    });
    const counters=await frame.evaluate(()=>({...heronPlaytest.metrics}));
    const clocks=await frame.evaluate(()=>heronPlaytestSnapshot());

    const data = JSON.parse(html.match(/<script id="level-data" type="application\/json">(.*?)<\/script>/s)[1]);
    data.level.name = 'edited_lab';
    // Move the static floor down a metre. This must replace the collision tree too.
    const floor = data.level.entities.find(e=>e.name==='worldspawn');
    floor.transform.translation[1] -= 1;
    const edited=html.replace(/(<script id="level-data" type="application\/json">).*?(<\/script>)/s,(_,start,end)=>start+JSON.stringify(data).replaceAll('<','\\u003c')+end);
    await editor.evaluate(html=>bridge.publish_playtest(html,'map-a'),edited);
    await viewer.waitForFunction(()=>document.querySelectorAll('iframe').length===1 && document.querySelector('iframe').contentDocument?.title.startsWith('edited_lab'));
    frame=current(); await frame.waitForFunction(()=>window.heronPlaytest);
    const feet=await frame.evaluate(()=>heronPlaytest.feet().toArray());assert.deepEqual(feet,[1,4,2]);
    const restored=await frame.evaluate(()=>heronPlaytestSnapshot());
    assert.deepEqual(restored.animations,clocks.animations);assert.deepEqual(restored.triggers,clocks.triggers);
    const updated=await frame.evaluate(()=>({...heronPlaytest.metrics}));
    assert.equal(updated.meshDecodes,counters.meshDecodes);assert.equal(updated.textureLoads,counters.textureLoads);
    assert.equal(updated.updates,1,'same iframe updates without re-decoding unchanged assets');
    const floorY = await frame.evaluate(()=>heronPlaytest.world.rayIntersect(new heronPlaytest.THREE.Ray(new heronPlaytest.THREE.Vector3(15,5,10),new heronPlaytest.THREE.Vector3(0,-1,0))).position.y);
    assert.ok(Math.abs(floorY + 1)<1e-6, 'new collision floor must replace the old tree');
    // Change one valid GLB payload: decoder work must be limited to that asset.
    const changed=structuredClone(data), path=Object.keys(changed.meshes)[0];
    const glb=Buffer.from(changed.meshes[path],'base64');
    const jsonEnd=20+glb.readUInt32LE(12), jsonText=glb.subarray(20,jsonEnd).toString();
    const generator=jsonText.match(/"generator":"([^"]+)"/);
    assert.ok(generator,'fixture GLB generator');
    const start=20+jsonText.indexOf(generator[1]);glb[start]=glb[start]===65?66:65;
    changed.meshes[path]=glb.toString('base64');
    const makeHtml=d=>html.replace(/(<script id="level-data" type="application\/json">).*?(<\/script>)/s,(_,a,b)=>a+JSON.stringify(d).replaceAll('<','\\u003c')+b);
    changed.level.name='mesh_changed';
    await editor.evaluate(h=>bridge.publish_playtest(h,'map-a'),makeHtml(changed));
    await frame.waitForFunction(()=>document.title.startsWith('mesh_changed'));
    assert.equal(await frame.evaluate(()=>heronPlaytest.metrics.meshDecodes),counters.meshDecodes+1);
    // Rapid updates including entity deletion/addition must converge to the last export.
    const fewer=structuredClone(changed);fewer.level.entities=fewer.level.entities.filter(e=>!e.properties?._heron_animation);fewer.level.name='fewer';
    await editor.evaluate(h=>bridge.publish_playtest(h,'map-a'),makeHtml(fewer));
    await frame.waitForFunction(()=>document.title.startsWith('fewer'));
    assert.equal(await frame.evaluate(()=>heronPlaytest.animations.size),0);
    const final=structuredClone(changed);final.level.name='rapid_final';
    await editor.evaluate(([a,b])=>{bridge.publish_playtest(a,'map-a');bridge.publish_playtest(b,'map-a');},[makeHtml(fewer),makeHtml(final)]);
    await frame.waitForFunction(()=>document.title.startsWith('rapid_final'));
    const removedPaths=new Set(changed.level.entities.filter(e=>e.properties?._heron_animation).map(e=>e.mesh).filter(Boolean));
    assert.equal(await frame.evaluate(()=>heronPlaytest.metrics.meshDecodes),counters.meshDecodes+1+removedPaths.size,'removed models are released and decoded again on restore');
    await editor.evaluate(h=>bridge.publish_playtest(h,'map-a'),edited);
    await frame.waitForFunction(()=>document.title.startsWith('edited_lab'));
    // Other editor channels must not replace the active map.
    await editor.evaluate(html=>{const other=new BroadcastChannel('heron-playtest-other');other.postMessage({type:'level',html,mapKey:'other'});other.close();},html);
    await viewer.waitForTimeout(100);assert.ok((await frame.title()).startsWith('edited_lab'));
    // A failed import retains the last good runtime.
    const broken=edited.replace(/"meshes":\{.*?\},"textures"/s,'"meshes":{},"textures"');
    await editor.evaluate(html=>bridge.publish_playtest(html,'map-a'),broken);
    await viewer.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Could not update'));
    assert.equal(await viewer.locator('iframe').count(),1);assert.deepEqual(await frame.evaluate(()=>heronPlaytest.feet().toArray()),feet);
    // Restore a good export; another map resets to its own spawn.
    await editor.evaluate(html=>bridge.publish_playtest(html,'map-b'),html);
    await viewer.waitForFunction(()=>document.querySelector('#status').hidden && document.querySelectorAll('iframe').length===1);
    frame=current();assert.notDeepEqual(await frame.evaluate(()=>heronPlaytest.feet().toArray()),feet);
    await viewer.reload();await viewer.waitForFunction(()=>document.querySelector('#status').hidden);
    assert.equal(await viewer.locator('iframe').count(),1);
    await viewer.close();await editor.waitForFunction(()=>!bridge.playtest_active());
    if(process.argv.includes('--editor')) {
      const app=await context.newPage();const errors=[];app.on('pageerror',e=>errors.push(e.message));
      await app.goto(base+'/index.html?renderer=gl');
      await app.waitForFunction(()=>!document.getElementById('loading') || document.querySelector('#loading.failed'),null,{timeout:60000});
      assert.equal(await app.locator('#loading.failed').count(),0, await app.evaluate(()=>document.querySelector('#loading .status')?.textContent || ''));
      await app.waitForTimeout(1000);await app.mouse.click(480,150);
      await app.waitForFunction(()=>document.title.includes('motion_lab'));
      await app.waitForTimeout(700);await app.mouse.click(49,18);await app.waitForTimeout(100);
      await app.screenshot({path:'/tmp/heron-web-editor.png'});
      const opened=context.waitForEvent('page');await app.mouse.click(131,248);
      const actualViewer=await opened;await actualViewer.waitForLoadState();
      await actualViewer.waitForFunction(()=>document.querySelector('iframe')?.style.visibility==='visible',null,{timeout:30000});
      const actualFrame=actualViewer.frames().find(f=>f.parentFrame());
      const exported=await actualFrame.evaluate(()=>JSON.parse(document.getElementById('level-data').textContent));
      assert.equal(exported.level.entities.length,10);
      assert.ok(exported.level.entities.some(e=>e.collision?.triangles?.length));
      await actualViewer.screenshot({path:'/tmp/heron-web-playing.png'});
      await actualViewer.close();
      assert.deepEqual(errors,[]);await app.close();
    }
    assert.deepEqual(remote,[]);
    console.log('PASS: local modules, initial export, player/animation/trigger preservation, cached hot update, one changed GLB, rapid entity removal/restore, isolation, failed import retention, map reset, reload handshake, close' + (process.argv.includes('--editor') ? ', WASM editor motion_lab and File menu export.' : '.'));
  } finally {await browser.close();server.close();}
})().catch(error=>{console.error(error);server.close();process.exitCode=1;});
