// node verify_feature_lab.cjs <feature root> <poses.json> [output folder]
//
// Runs the feature lab's PlayCanvas export in Chromium (SwiftShader WebGL): the level
// loaded the way a game loads it, through the exported heron_playcanvas.js with ammo.js,
// and the generated playtest page with its capsule player. <feature root> holds
// heron-feature-playcanvas from export_feature_lab.py; <poses.json> comes from
// feature_lab_poses.py. Writes playcanvas-lab.png, playcanvas-stations.png,
// playcanvas-playtest.png and results.json.
//
// PlayCanvas and ammo.js load from jsDelivr at the versions the exported page names.
// Set HERON_CDN_CACHE to a folder to keep them between runs.
const {chromium} = require('playwright');
const fs = require('node:fs'), http = require('node:http'), path = require('node:path');

(async () => {
  const featureRoot = path.resolve(process.argv[2]);
  const poses = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
  const output = path.resolve(process.argv[4] || '/tmp/heron-feature-runtime');
  fs.mkdirSync(output, {recursive: true});
  const roots = {'/playcanvas/': path.join(featureRoot, 'heron-feature-playcanvas'), '/check/': __dirname};
  const playtest = fs.readFileSync(path.join(roots['/playcanvas/'], 'feature_lab.html'), 'utf8');
  // The check page loads the same PlayCanvas and ammo.js as the exported page.
  const scripts = [...playtest.matchAll(/<script src="([^"]+)"><\/script>/g)].map(m => m[1]);
  const page = '<!doctype html><body style="margin:0">' + scripts.map(s => `<script src="${s}"></script>`).join('') +
    '<canvas id="view" width="1280" height="720" style="width:1280px;height:720px"></canvas>';
  const types = {'.js': 'text/javascript', '.json': 'application/json', '.png': 'image/png', '.html': 'text/html', '.glb': 'model/gltf-binary'};
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
  const cache = process.env.HERON_CDN_CACHE;
  if (cache) fs.mkdirSync(cache, {recursive: true});
  const cdn = async route => {
    const url = route.request().url();
    const file = cache && path.join(cache, url.replace(/[^a-zA-Z0-9.@-]/g, '_'));
    const headers = {'content-type': url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript', 'access-control-allow-origin': '*'};
    if (file && fs.existsSync(file)) return route.fulfill({body: fs.readFileSync(file), headers});
    const response = await route.fetch();
    const body = await response.body();
    if (file && response.ok()) fs.writeFileSync(file, body);
    return route.fulfill({status: response.status(), body, headers});
  };
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
    const errors = [];
    const open = async url => {
      const tab = await browser.newPage({viewport: {width: 1280, height: 720}});
      await tab.route('https://cdn.jsdelivr.net/**', cdn);
      tab.on('pageerror', error => errors.push(error.message));
      tab.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
      await tab.goto(url);
      return tab;
    };
    const tab = await open(origin + '/');
    const ammo = scripts.find(s => s.includes('/ammo/')).replace(/[^/]*$/, '');
    report.playcanvas = await tab.evaluate(async ([poses, ammo]) => {
      const check = await import('/check/feature_lab_checks.js');
      return check.checkPlayCanvas('/playcanvas/', poses, document.getElementById('view'), ammo);
    }, [poses, ammo]);
    for (const [key, file] of [['png', 'playcanvas-lab.png'], ['pngNear', 'playcanvas-stations.png']]) {
      fs.writeFileSync(path.join(output, file), Buffer.from(report.playcanvas[key].split(',')[1], 'base64'));
      delete report.playcanvas[key];
    }

    // The generated playtest page: its own capsule player stands in the trigger.
    const play = await open(origin + '/playtest.html?verify=1');
    await play.waitForFunction(() => window.heronPlaytest, null, {timeout: 120000});
    report.playtest = await play.evaluate(async () => {
      const check = await import('/check/feature_lab_checks.js');
      return check.checkPlaytest(window.heronPlaytest);
    });
    await play.evaluate(() => { document.getElementById('menu').hidden = true; document.getElementById('hint').hidden = true; });
    await play.waitForTimeout(1000);
    await play.screenshot({path: path.join(output, 'playcanvas-playtest.png')});
    report.errors = errors;
    if (errors.length) { failed = true; console.log('page errors:', errors); }
    show('playcanvas', report.playcanvas);
    show('playtest', report.playtest);

    report.browser = browser.version();
    console.log(`Chromium ${report.browser}`);
    fs.writeFileSync(path.join(output, 'results.json'), JSON.stringify(report, null, 1));
    console.log(failed ? 'FEATURE_LAB PlayCanvas FAIL' : 'FEATURE_LAB PlayCanvas PASS', output);
    process.exitCode = failed ? 1 : 0;
  } finally {
    if (browser) await browser.close();
    server.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
