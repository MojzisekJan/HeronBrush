// node verify_web_lifecycle.cjs <feature root> [output directory]
// Uses the exported runtimes with real engine assets in Chromium/WebGL.
const {chromium} = require('playwright');
const fs = require('node:fs'), http = require('node:http'), path = require('node:path');
const root = path.resolve(process.argv[2]);
const output = path.resolve(process.argv[3] || '/tmp/heron-web-lifecycle-results');
fs.mkdirSync(output, {recursive:true});
const runtimes = path.resolve(__dirname, '../../crates/scene_export/src/writer');
const types = {'.js':'text/javascript','.json':'application/json','.glb':'model/gltf-binary','.hdr':'application/octet-stream','.png':'image/png'};
const server = http.createServer((req,res) => {
  const parts = new URL(req.url, 'http://localhost').pathname.split('/').filter(Boolean);
  const engine = parts.shift();
  if (!['babylon','playcanvas'].includes(engine)) {res.writeHead(404); return res.end();}
  const folder = path.join(root, 'heron-feature-'+engine);
  if (!parts.length) {
    const html = fs.readFileSync(path.join(folder,'feature_lab.html'),'utf8');
    const scripts = [...html.matchAll(/<script src="([^"]+)"><\/script>/g)].map(m=>`<script src="${m[1]}"></script>`).join('');
    res.setHeader('content-type','text/html');return res.end(scripts+'<canvas id="view" width="320" height="240"></canvas>');
  }
  let file = path.resolve(folder,...parts);
  // Test the repository runtime directly too, before a Rust rebuild embeds it.
  if (process.env.HERON_SOURCE_RUNTIME === '1' && parts.join('/')===`heron_${engine}.js`)
    file=path.join(runtimes,engine+'_runtime.js');
  else if (!file.startsWith(folder+path.sep)) {res.writeHead(404);return res.end();}
  if (!fs.existsSync(file)) {res.writeHead(404); return res.end('missing fixture');}
  res.setHeader('content-type',types[path.extname(file)] || 'application/octet-stream');res.end(fs.readFileSync(file));
});
(async()=>{
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const browser=await chromium.launch({executablePath:process.env.HERON_CHROMIUM,headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
  const results={};
  try {
    for (const engine of ['babylon','playcanvas'].filter(name => !process.env.HERON_WEB_ENGINE || name === process.env.HERON_WEB_ENGINE)) {
      const page=await browser.newPage();
      if(process.env.HERON_WEB_VERBOSE){
        page.on('console', message=>console.log(engine+': '+message.text()));
        page.on('requestfailed', request=>console.log(engine+': '+request.url()+' '+request.failure()?.errorText));
      }
      const cache=path.join(output,'cdn');fs.mkdirSync(cache,{recursive:true});
      await page.route('https://cdn.jsdelivr.net/**',async route=>{
        const url=route.request().url(),file=path.join(cache,url.replace(/[^a-zA-Z0-9.@-]/g,'_'));
        const headers={'content-type':url.endsWith('.wasm')?'application/wasm':'text/javascript','access-control-allow-origin':'*'};
        if(fs.existsSync(file))return route.fulfill({body:fs.readFileSync(file),headers});
        const response=await route.fetch(), body=await response.body();
        if(response.ok())fs.writeFileSync(file,body);
        return route.fulfill({status:response.status(),body,headers});
      });
      await page.goto(`http://127.0.0.1:${server.address().port}/${engine}/`);
      results[engine]=await page.evaluate(async engine=>{
        const {loadLevel}=await import(`./heron_${engine}.js`);
        const data=await (await fetch('./feature_lab.json')).json();
        const B=window.BABYLON,pc=window.pc,canvas=document.querySelector('canvas');
        let scene,app,nativeEngine;
        if(B){
          nativeEngine=new B.Engine(canvas,true);scene=new B.Scene(nativeEngine);scene.useRightHandedSystem=true;
          scene.enablePhysics(new B.Vector3(0,-9.81,0),new B.HavokPlugin(true,await HavokPhysics()));
        }else{
          globalThis.Ammo=await Ammo({locateFile:file=>`https://cdn.jsdelivr.net/gh/playcanvas/engine@v${pc.version}/examples/assets/wasm/ammo/${file}`});
          app=new pc.Application(canvas);scene=app.scene;
          const camera=new pc.Entity('project camera');camera.addComponent('camera');app.root.addChild(camera);
        }
        const load=(source,options={})=>B?loadLevel(B,scene,source,options):loadLevel(pc,app,source,options);
        const counts=()=>B?JSON.stringify([scene.meshes.length,scene.transformNodes.length,scene.materials.length,scene.multiMaterials.length,scene.textures.length,scene.geometries.length,scene.skeletons.length,scene.animationGroups.length,scene.lights.length]):JSON.stringify([app.root.children.length,app.assets.list().length]);
        const checks=[];
        const check=(value,name)=>{if(!value)throw Error(`${engine}: ${name}`);checks.push(name);console.debug('PASS '+name);};
        const reject=async(fn,name)=>{let failed=false;try{await fn();}catch{failed=true;}check(failed,name);};
        // Warm native default resources once, then require stable counts.
        let warm=await load('./feature_lab.json');warm.dispose();
        const baseline=counts();
        for(let i=0;i<4;i++){
          const level=await load('./feature_lab.json');
          check(level.entities.length===data.entities.length,'complete load '+i);
          level.dispose();level.dispose();check(counts()===baseline,`no resource growth ${i}: ${counts()} vs ${baseline}; ${B?scene.textures.map(t=>t.name).join(","):""}`);
        }
        const source=structuredClone(data);delete source.environment;
        const meshes=Object.fromEntries(source.entities.filter(e=>e.mesh).map(e=>[e.mesh,new URL(e.mesh,location.href).href]));
        const wrapped=level=>({level,meshes});
        const invalid=structuredClone(source);invalid.entities[1].transform.scale=[1,NaN,1];
        await reject(()=>load(wrapped(invalid)),'reject invalid transform');check(counts()===baseline,'invalid transform leaves no actors');
        const duplicate=structuredClone(source);duplicate.entities.push(structuredClone(duplicate.entities[0]));
        await reject(()=>load(wrapped(duplicate)),'reject duplicate name');check(counts()===baseline,'duplicate leaves no actors');
        const missing=structuredClone(source);missing.entities[1].mesh='missing.glb';
        await reject(()=>load(wrapped(missing)),'missing GLB rejects');check(counts()===baseline,'missing GLB rolls back assets');
        let calls=0,cleaned=0;
        const factories=Object.fromEntries([...new Set(source.entities.map(e=>e.classname))].map(name=>[name,()=>{if(++calls===2)throw Error('factory failed');return()=>cleaned++;}]));
        await reject(()=>load(wrapped(source),{factories}),'factory error rejects');check(cleaned===1&&counts()===baseline,'factory failure cleans previous callbacks and resources');
        const badCleanup=await load(wrapped(source));badCleanup.cleanups.push(()=>{throw Error('cleanup failed');});badCleanup.cleanups.push(()=>cleaned++);
        await reject(()=>badCleanup.dispose(),'cleanup reports callback error');badCleanup.dispose();check(cleaned===2&&counts()===baseline,'cleanup error still releases all engine resources');
        const first=await load(wrapped(source)), second=await load(wrapped(source));
        const visuals=second.entities.flatMap(record=>record.meshes)
          .filter(mesh=>B?mesh.getTotalVertices()>0:!!mesh.mesh?.vertexBuffer);
        first.dispose();
        check(visuals.length>0 && visuals.every(mesh=>B?mesh.getTotalVertices()>0:!!mesh.mesh?.vertexBuffer), 'unloading one copy preserves other copy geometry');
        check(B || second.assets.every(asset=>asset.resource), 'other copy retains loaded container assets');
        second.dispose();check(counts()===baseline,'two copies release their resources');
        const env=exposure=>({format:2,name:'environment',entities:[],environment:{...data.environment,panorama:null,display:null,exposure}});
        await reject(()=>load({level:env(0.4),sky:new URL('missing.hdr',location.href).href}), 'missing sky rejects');
        check(counts()===baseline,'missing sky rolls back all resources');
        const exposure=()=>B?scene.imageProcessingConfiguration.exposure:scene.exposure;
        const initial=exposure();
        let a=await load({level:env(0.4)}),b=await load({level:env(1.7)});
        a.dispose();check(Math.abs(exposure()-1.7)<1e-5,'removing older map keeps newer environment');
        b.dispose();check(Math.abs(exposure()-initial)<1e-5,'out-of-order disposal restores project environment');
        a=await load({level:env(0.4)});b=await load({level:env(1.7)});
        b.dispose();check(Math.abs(exposure()-0.4)<1e-5,'removing newer map restores earlier environment');a.dispose();
        a=await load({level:env(0.4)});
        if(B)scene.imageProcessingConfiguration.exposure=2.3;else scene.exposure=2.3;
        a.dispose();check(Math.abs(exposure()-2.3)<1e-5,'project environment edits survive disposal');
        if(B && data.environment.display){
          const graded=env(0.4);graded.environment.display=data.environment.display;
          a=await load({level:graded});b=await load({level:graded});
          a.dispose();check(exposure()===1,'newer display settings survive disposal of older map');
          b.dispose();check(Math.abs(exposure()-2.3)<1e-5,'display disposal restores project exposure');
        }
        check(counts()===baseline,'environment leaves no native objects');
        if(app)app.destroy();else{scene.dispose();nativeEngine.dispose();}
        return checks;
      },engine);
      console.log(engine+': '+results[engine].length+' lifecycle checks passed');
      await page.close();
    }
    fs.writeFileSync(path.join(output,'lifecycle.json'),JSON.stringify(results,null,2)+'\n');
  }finally{await browser.close();server.close();}
})().catch(error=>{console.error(error);server.close();process.exitCode=1;});
