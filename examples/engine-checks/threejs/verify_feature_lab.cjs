// node verify_feature_lab.cjs <feature root> <poses.json> [output folder]
//
// Runs the feature lab's three.js, GLB and glTF exports in Chromium (three.js 0.186,
// SwiftShader WebGL). <feature root> holds heron-feature-three, -glb and -gltf from
// export_feature_lab.py; <poses.json> comes from feature_lab_poses.py. Writes
// three-lab.png, three-playtest.png, glb.png and results.json to the output folder.
const {chromium} = require('playwright');
const fs = require('node:fs'), http = require('node:http'), path = require('node:path');
const CDN = 'https://cdn.jsdelivr.net/npm/three@0.186.0/';

(async () => {
  const repository = path.resolve(__dirname, '../../..');
  const featureRoot = path.resolve(process.argv[2]);
  const poses = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
  const output = path.resolve(process.argv[4] || '/tmp/heron-feature-runtime');
  fs.mkdirSync(output, {recursive: true});
  const vendor = path.join(repository, 'crates/heron-brush-web/vendor/three');
  const roots = {
    '/vendor/': vendor,
    '/three/': path.join(featureRoot, 'heron-feature-three'),
    '/glb/': path.join(featureRoot, 'heron-feature-glb'),
    '/gltf/': path.join(featureRoot, 'heron-feature-gltf'),
    '/check/': __dirname,
  };
  // The web build vendors only what the playtest needs; HDRLoader comes from the CDN.
  const imports = {three: '/vendor/build/three.module.js', 'three/addons/': '/vendor/examples/jsm/'};
  if (!fs.existsSync(path.join(vendor, 'examples/jsm/loaders/HDRLoader.js'))) {
    imports['three/addons/loaders/HDRLoader.js'] = CDN + 'examples/jsm/loaders/HDRLoader.js';
  }
  const page = '<!doctype html><script type="importmap">' + JSON.stringify({imports}) + '</script><body style="margin:0">';
  const playtest = fs.readFileSync(path.join(roots['/three/'], 'feature_lab.html'), 'utf8')
    .replaceAll(CDN + 'build/', '/vendor/build/').replaceAll(CDN + 'examples/jsm/', '/vendor/examples/jsm/');
  const types = {'.js': 'text/javascript', '.json': 'application/json', '.png': 'image/png', '.html': 'text/html'};
  const server = http.createServer((request, response) => {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    if (pathname === '/') { response.setHeader('Content-Type', 'text/html'); return response.end(page); }
    if (pathname === '/playtest.html') { response.setHeader('Content-Type', 'text/html'); return response.end(playtest); }
    const prefix = Object.keys(roots).find(p => pathname.startsWith(p));
    const file = prefix && path.resolve(roots[prefix], pathname.slice(prefix.length));
    if (!file || !file.startsWith(roots[prefix] + path.sep) || !fs.existsSync(file)) { response.statusCode = 404; return response.end(); }
    response.setHeader('Content-Type', types[path.extname(file)] || 'application/octet-stream');
    response.end(fs.readFileSync(file));
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  let browser, failed = false;
  const report = {};
  const show = (group, results) => {
    for (const [name, result] of Object.entries(results)) {
      if (result.status === undefined) { show(`${group} ${name}`, result); continue; }
      console.log(`${group} ${name}: ${result.status}. ${result.detail}`);
      if (result.status !== 'PASS') failed = true;
    }
  };
  try {
    browser = await chromium.launch({headless: true, args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader']});
    const tab = await browser.newPage({viewport: {width: 1280, height: 720}}), errors = [];
    tab.on('pageerror', error => errors.push(error.message));
    tab.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
    await tab.goto(origin + '/');
    const three = await tab.evaluate(async poses => {
      const THREE = await import('three');
      const check = await import('/check/feature_lab_checks.js');
      const {results, scene} = await check.checkThree('/three/', poses);
      const camera = new THREE.PerspectiveCamera(55, 16 / 9, 0.1, 200);
      camera.position.set(-20, 16, -8); camera.lookAt(2, 0, -9);
      const {png, tangents} = check.render(scene, camera);
      results.normal.detail += tangents ? '; the linked program reads the tangent attribute' : '';
      if (!tangents) results.normal = {status: 'FAIL', detail: 'the rendered program does not read tangents'};
      const glb = await check.checkPortable('/glb/feature_lab.glb');
      const gltf = await check.checkPortable('/gltf/feature_lab.gltf');
      // The portable files keep the map's own axes: X, Z up, -Y. The same view there.
      const portable = new THREE.PerspectiveCamera(55, 16 / 9, 0.1, 200);
      portable.position.set(-8, 16, 20); portable.lookAt(-9, 0, -2);
      const view = check.render(glb.scene, portable);
      return {three: results, png, glb: glb.results, gltf: gltf.results, glbPng: view.png};
    }, poses);
    fs.writeFileSync(path.join(output, 'three-lab.png'), Buffer.from(three.png.split(',')[1], 'base64'));
    fs.writeFileSync(path.join(output, 'glb.png'), Buffer.from(three.glbPng.split(',')[1], 'base64'));
    report.three = three.three; report.glb = three.glb; report.gltf = three.gltf;

    // The example loader in src/: the door through advance(dt, feet, height), and its
    // wrapping of baked and tinted paint.
    report.loader = await tab.evaluate(async () => {
      const THREE = await import('three');
      const {loadLevel} = await import('/check/src/heron_brushlevel.js');
      const scene = await loadLevel('/three/feature_lab.json');
      const named = name => scene.entities.find(e => e.properties.targetname === name);
      const [x, , z] = named('fl_trigger').position.toArray();
      const door = scene.animations.get(named('fl_door').name);
      for (let i = 0; i < 90; i++) scene.advance(1 / 60, new THREE.Vector3(x, 0.02, z), 1.8);
      // Wrapping of each map; with `wide`, only maps wider than one pixel (fl_surface's
      // one-pixel emissive paint clamps, its 64-pixel tinted textures repeat).
      const wraps = (name, wide) => { const w = []; named(name).object.traverse(o => { if (o.isMesh) for (const m of [o.material].flat()) if (!wide || m.map?.image.width > 1) w.push(m.map?.wrapS); }); return w; };
      const baked = wraps('fl_colours'), tinted = wraps('fl_surface', true);
      return {
        trigger: door.clip.name === 'open' && door.posed && door.time === 1
          ? {status: 'PASS', detail: 'advance(dt, feet, 1.8) with the feet on the floor opens the door'}
          : {status: 'FAIL', detail: `door ${door.clip.name} posed ${door.posed} at ${door.time}`},
        clamp: baked.every(w => w === 1001) && tinted.every(w => w === 1000)
          ? {status: 'PASS', detail: 'baked fl_colours paint clamps; fl_surface textures tinted for their surface repeat'}
          : {status: 'FAIL', detail: `fl_colours ${baked}, fl_surface ${tinted}`},
      };
    });

    // The generated playtest page: its own player capsule walks into the trigger.
    const play = await browser.newPage({viewport: {width: 1280, height: 720}});
    play.on('pageerror', error => errors.push(error.message));
    await play.goto(origin + '/playtest.html?verify=1');
    await play.waitForFunction(() => window.heronPlaytest, null, {timeout: 60000});
    report.playtest = await play.evaluate(() => {
      const h = window.heronPlaytest, THREE = h.capsule.start.constructor;
      const door = h.animations.get('func_animate_1');
      const trigger = h.triggers.find(t => t.entityName === 'trigger_multiple');
      const centre = trigger.data.origin;
      const run = seconds => { for (let t = 0; t < seconds; t += 1 / 60) h.step(1 / 60); };
      h.teleport(new THREE(centre[0], 0.02, centre[2]));
      run(1.5);
      const feet = h.capsule.start.y - h.capsule.radius;
      const opened = door.clip.name === 'open' && door.posed;
      const result = {trigger: opened
        ? {status: 'PASS', detail: `a player standing in fl_trigger (feet at y=${feet.toFixed(2)}) opens the door: clip ${door.clip.name}, time ${door.time.toFixed(2)}`}
        : {status: 'FAIL', detail: `a player standing in fl_trigger (feet at y=${feet.toFixed(2)}, trigger from y=0.25) does not open the door; clip ${door.clip.name}, posed ${door.posed}`}};
      // Baked paint is drawn clamped, as the GLBs' samplers say.
      const data = JSON.parse(document.getElementById('level-data').textContent);
      const painted = new Set(data.level.entities.find(e => e.properties?.targetname?.value === 'fl_colours').surfaces.map(s => s.texture));
      const wraps = [];
      h.scene.traverse(o => { if (o.isMesh) for (const m of [o.material].flat()) if (painted.has(m.name)) wraps.push(m.map?.wrapS); });
      result.clamp = wraps.length === 6 && wraps.every(w => w === 1001)
        ? {status: 'PASS', detail: 'the page draws all 6 fl_colours paint maps clamped'}
        : {status: 'FAIL', detail: `fl_colours paint maps wrap ${wraps.join(', ')} (1001 is clamp)`};
      // Then look at the lab from the start, for the screenshot.
      h.reset(); run(0.2);
      h.camera.position.set(-20, 16, -8); h.camera.lookAt(2, 0, -9);
      return result;
    });
    await play.evaluate(() => document.getElementById('menu').hidden = true);
    await play.waitForTimeout(500);
    await play.screenshot({path: path.join(output, 'three-playtest.png')});
    report.errors = errors;
    if (errors.length) { failed = true; console.log('page errors:', errors); }
    show('three.js', report.three); show('loader', report.loader); show('playtest', report.playtest);
    show('GLB', report.glb); show('glTF', report.gltf);
    fs.writeFileSync(path.join(output, 'results.json'), JSON.stringify(report, null, 1));
    console.log(failed ? 'FEATURE_LAB three.js FAIL' : 'FEATURE_LAB three.js PASS', output);
    process.exitCode = failed ? 1 : 0;
  } finally {
    if (browser) await browser.close();
    server.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
