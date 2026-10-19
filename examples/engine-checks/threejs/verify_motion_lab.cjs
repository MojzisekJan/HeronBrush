// Usage: node verify_motion_lab.cjs /tmp/heron-motion-three/motion_lab.html
// Requires playwright and its Chromium. Tests the generated HTML's actual player
// and Octree against fresh exported collision, without a second physics model.
const {chromium} = require('playwright');
const fs = require('node:fs');
const http = require('node:http');
const assert = require('node:assert/strict');
const path = require('node:path');

(async () => {
  const html = fs.readFileSync(process.argv[2]);
  const importer = process.argv.includes('--importer');
  const repository = path.resolve(__dirname,'../../..');
  const fixtures = path.dirname(path.resolve(process.argv[2]));
  const server = http.createServer((req, res) => {
    if(importer) {
      const url = new URL(req.url,'http://localhost');
      const folder=url.pathname.startsWith('/fixtures/') ? fixtures : repository;
      const relative=url.pathname.startsWith('/fixtures/') ? url.pathname.slice(10) : url.pathname.slice(1);
      const file=path.resolve(folder,decodeURIComponent(relative));
      if(!file.startsWith(folder+path.sep) || !fs.existsSync(file)) {res.statusCode=404;res.end();return;}
      const types={'.html':'text/html','.js':'text/javascript','.json':'application/json','.png':'image/png'};
      res.setHeader('Content-Type',types[path.extname(file)]||'application/octet-stream');res.end(fs.readFileSync(file));return;
    }
    res.setHeader('Content-Type', 'text/html'); res.end(html);
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser;
  try {
    browser = await chromium.launch({headless:true, args:['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader']});
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const route=importer ? '/examples/engine-checks/threejs/index.html?verify=1&level=/fixtures/motion_lab.json' : '/?verify=1';
    await page.goto(`http://127.0.0.1:${server.address().port}${route}`);
    await page.waitForFunction(() => window.heronPlaytest || document.querySelector('#status')?.textContent.startsWith('Could not'), null, {timeout:60000});
    assert.ok(await page.evaluate(() => !!window.heronPlaytest), errors.join('\n'));
    const results = await page.evaluate(only => {
      const p = window.heronPlaytest, T = p.THREE, dt = 1/60;
      const frames = n => { for (let i=0; i<n; i++) p.step(dt); };
      const stop = () => {for (const a of p.animations.values()) {a.stop(); a.update(0);} p.teleport(new T.Vector3(16,1,12)); frames(8);};
      const results = [];
      for (const [name, seconds, offset] of [['func_animate',17,[0,2,0]], ['func_animate_1',15,[0,2,0]], ['func_animate_2',120,[1,2,0]]]) {
        if (only && name !== only) continue;
        stop();
        const a = p.animations.get(name);
        if (!a) throw new Error('Missing animation '+name);
        p.teleport(a.object.position.clone().add(new T.Vector3(...offset))); frames(60);
        a.object.updateWorldMatrix(true,false);
        const anchor = a.object.worldToLocal(p.feet());
        a.play(a.data.selected);
        let drift=0, vertical=0, airborne=0, turned=false, cut=false; const missed=[];
        const lo=a.object.position.clone(), hi=lo.clone();
        for (let i=0; i<seconds*60; i++) {
          p.step(dt); a.object.updateWorldMatrix(true,false);
          const local=a.object.worldToLocal(p.feet());
          drift=Math.max(drift,Math.hypot(local.x-anchor.x,local.z-anchor.z));
          vertical=Math.max(vertical,Math.abs(local.y-anchor.y));
          if (!p.grounded) {
            airborne++;
            if(missed.length<8) missed.push({frame:i,feet:p.feet().toArray(),platform:a.object.position.toArray(),velocity:p.velocity.toArray(),diagnostic:p.diagnostic});
          }
          lo.min(a.object.position); hi.max(a.object.position);
          turned ||= Math.abs(a.object.quaternion.y)>0.4;
          if(name==='func_animate' && a.object.position.x < -4.04 && a.object.position.z > -5.96) cut=true;
        }
        if(name==='func_animate_2' && (!turned || drift>=0.01)) throw new Error('Rotor stayed stationary or idle correction drifted');
        if(cut) throw new Error('Shuttle cut across the L path');
        results.push({name,drift,vertical,airborne,travel:hi.sub(lo).toArray(),missed});
        if (!only) {
          const before = a.object.worldToLocal(p.feet());
          if(name==='func_animate_2') {
            const direction=a.object.position.clone().sub(p.feet());
            p.face(Math.atan2(-direction.x,-direction.z));
            p.keys.add('KeyW'); frames(12); p.keys.delete('KeyW');
            const walked=a.object.worldToLocal(p.feet()).distanceTo(before);
            if(walked<0.4) throw new Error(`Cannot walk on rotor: ${walked}`);
          }
          const beforeY=p.feet().y;
          p.keys.add('Space'); frames(12); p.keys.delete('Space');
          if(p.grounded || p.feet().y<beforeY+0.4) throw new Error(`Jump failed to detach from ${name}`);
        }
      }
      if(!only) {
        stop();
        const door=p.animations.get('func_animate_3'), lift=p.animations.get('func_animate_1');
        p.teleport(new T.Vector3(8,0.02,3)); frames(35);
        if(!door.playing) throw new Error('Door pad did not activate animation');
        frames(100);
        const yaw=new T.Euler().setFromQuaternion(door.object.quaternion,'YXZ').y;
        if(Math.abs(Math.abs(yaw)-Math.PI/2)>0.02) throw new Error('Door failed to open fully');
        p.teleport(new T.Vector3(8,0.1,4.5)); p.face(Math.PI); p.keys.add('KeyW'); frames(35); p.keys.clear();
        const openZ=p.feet().z;
        if(openZ<6.5) throw new Error(`Open door blocked player at ${openZ}`);
        door.stop(); frames(5);
        p.teleport(new T.Vector3(8,0.1,4.5)); p.keys.add('KeyW'); frames(35); p.keys.clear();
        const closedZ=p.feet().z;
        if(closedZ>=5.8) throw new Error(`Closed door failed to block player at ${closedZ}`);
        lift.stop(); frames(5);
        p.teleport(new T.Vector3(6,0.02,-3)); frames(10);
        if(!lift.playing) throw new Error('Lift pad did not activate animation');
        results.push({interactions:'rotor walk, jumps, door and lift pads, open/closed door',openZ,closedZ});
      }
      return results;
    }, importer ? null : process.argv[3]);
    console.log(JSON.stringify(results, null, 2));
    assert.deepEqual(errors, []);
    for (const result of results) {
      if(!result.name) continue;
      assert.ok(result.drift<0.04, `${result.name}: horizontal drift ${result.drift}`);
      assert.ok(result.vertical<0.06, `${result.name}: vertical drift ${result.vertical}`);
      assert.equal(result.airborne,0, `${result.name}: lost floor contact`);
    }
    const shuttle=results.find(r=>r.name==='func_animate'), lift=results.find(r=>r.name==='func_animate_1');
    if(shuttle) assert.ok(shuttle.travel[0]>5.9 && shuttle.travel[2]>5.9);
    if(lift) assert.ok(lift.travel[1]>5.9);
    console.log('HERON_MOTION_LAB_THREE_RIDES_OK');
  } finally {
    if (browser) await browser.close();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); process.exitCode=1; });
