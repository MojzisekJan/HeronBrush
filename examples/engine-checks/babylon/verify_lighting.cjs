// node verify_lighting.cjs <fixture root> [output folder]
// Verifies rendered shadows, probe diffuse/specular effects, all display curves,
// loose PBR maps, layered materials and the self-contained playtest page.
const fs=require('fs'),path=require('path'),http=require('http'),{chromium}=require('playwright');
(async()=>{
 const root=path.resolve(process.argv[2],'heron-feature-babylon'),cache=path.resolve(process.env.HERON_CDN_CACHE || path.join(process.argv[2],'cdn')),out=path.resolve(process.argv[3] || path.join(process.argv[2],'lighting-results'));fs.mkdirSync(out,{recursive:true});fs.mkdirSync(cache,{recursive:true});
 const html=fs.readFileSync(path.join(root,'lighting.html'),'utf8'),scripts=[...html.matchAll(/<script src="([^"]+)"><\/script>/g)].map(m=>m[1]);
 const shell='<!doctype html><body style="margin:0">'+scripts.map(s=>`<script src="${s}"></script>`).join('')+'<canvas id="view" width="960" height="640"></canvas>';
 const server=http.createServer((req,res)=>{const url=new URL(req.url,'http://localhost');if(url.pathname==='/'){res.setHeader('Content-Type','text/html');return res.end(shell);}const file=path.resolve(root,'.'+url.pathname);if(!file.startsWith(root+'/')||!fs.existsSync(file)){res.statusCode=404;return res.end();}res.setHeader('Content-Type',({'.js':'text/javascript','.json':'application/json','.html':'text/html','.hdr':'application/octet-stream','.png':'image/png','.glb':'model/gltf-binary'})[path.extname(file)]||'application/octet-stream');res.end(fs.readFileSync(file));});
 await new Promise(r=>server.listen(0,'127.0.0.1',r));const origin='http://127.0.0.1:'+server.address().port;
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});const errors=[];
 try {
 const page=await browser.newPage({viewport:{width:960,height:640}});page.on('pageerror',e=>{errors.push(e.message);console.log('PAGE ERROR',e.message)});page.on('console',m=>{if(m.type()==='error'){errors.push(m.text());console.log('CONSOLE ERROR',m.text())}});
 await page.route('https://cdn.jsdelivr.net/**',async route=>{const url=route.request().url(),file=path.join(cache,url.replace(/[^a-zA-Z0-9.@-]/g,'_'));let body;if(fs.existsSync(file))body=fs.readFileSync(file);else{const r=await route.fetch();body=await r.body();fs.writeFileSync(file,body);}return route.fulfill({body,headers:{'content-type':url.endsWith('.wasm')?'application/wasm':'text/javascript','access-control-allow-origin':'*'}});});
 await page.goto(origin);const report=await page.evaluate(async()=>{
 const B=window.BABYLON,{loadLevel}=await import('/heron_babylon.js');const engine=new B.Engine(document.getElementById('view'),true,{preserveDrawingBuffer:true});const scene=new B.Scene(engine);scene.useRightHandedSystem=true;
 const source=await (await fetch('/lighting.json')).json();source.environment.display.tony_lut=(await (await fetch('/tony.json')).json()).environment.display.tony_lut;const level=await loadLevel(B,scene,{level:source});const camera=new B.UniversalCamera('check',new B.Vector3(-20,16,-8),scene);camera.setTarget(new B.Vector3(2,0,-9));scene.activeCamera=camera;
 await Promise.race([scene.whenReadyAsync(),new Promise((_,reject)=>setTimeout(()=>reject(Error("Scene readiness timeout")),45000))]);scene.render();window.audit={B,engine,scene,level,camera};
 const material=level.entities.flatMap(r=>r.meshes).map(m=>m.material).find(m=>m?.anisotropy?.isEnabled);
 if(!material)throw new Error('Anisotropic GLB material missing');
 const report={layers:{coat:material.clearCoat.intensity,anisotropy:material.anisotropy.intensity,tangent:level.entities.flatMap(r=>r.meshes).some(m=>m.isVerticesDataPresent('tangent'))},shadows:{count:level.shadows.length,casters:level.shadows[0]?.getShadowMap().renderList.length},background:scene.meshes.some(m=>m.name==='heron-skybox'),post:camera._postProcesses.filter(Boolean).length,plugins:!!material.heronLocalLighting};
 // New cameras and shadow defines can start asynchronous shader compilation.
 // Read only a completed frame, rather than the previous canvas contents.
 const pixels=async()=>{
   scene.render();await scene.whenReadyAsync();
   for(const post of scene.activeCamera._postProcesses)if(post)await new Promise(resolve=>post.getEffect().executeWhenCompiled(resolve));
   await new Promise(requestAnimationFrame);scene.render();
   return new Uint8Array(await engine.readPixels(0,0,960,640));
 };
 const a=await pixels();scene.shadowsEnabled=false;const noShadow=await pixels();let shadowDelta=0;for(let i=0;i<a.length;i++)if(i%4!==3)shadowDelta+=Math.abs(a[i]-noShadow[i]);report.shadowPixelDifference=shadowDelta/(960*640*3);scene.shadowsEnabled=true;const data=level.data.environment.local_lighting;for(const p of data.probes){p.diffuse=false;p.reflections=false;}const b=await pixels();let diff=0;for(let i=0;i<a.length;i++)if(i%4!==3)diff+=Math.abs(a[i]-b[i]);report.probePixelDifference=diff/(960*640*3);
 for(const p of data.probes){p.diffuse=true;p.reflections=false;}const c=await pixels();diff=0;for(let i=0;i<c.length;i++)if(i%4!==3)diff+=Math.abs(c[i]-b[i]);report.diffusePixelDifference=diff/(960*640*3);
 for(const p of data.probes){p.diffuse=false;p.reflections=true;}const d=await pixels();diff=0;for(let i=0;i<d.length;i++)if(i%4!==3)diff+=Math.abs(d[i]-b[i]);report.reflectionPixelDifference=diff/(960*640*3);
 for(const p of data.probes){p.diffuse=true;p.reflections=true;}
 const display=level.data.environment.display;scene.render();const before=await pixels();display.saturation=0;const after=await pixels();let delta=0;for(let i=0;i<before.length;i++)if(i%4!==3)delta+=Math.abs(before[i]-after[i]);report.gradingPixelDifference=delta/(960*640*3);display.saturation=.7;
 const mapperFrames={};for(const mapper of ['aces','agx','neutral','tony']){display.tone_map=mapper;const frame=await pixels();let sum=0;for(let k=0;k<frame.length;k++)if(k%4!==3)sum+=frame[k];mapperFrames[mapper]=sum/(960*640*3);}report.toneMaps=mapperFrames;display.tone_map='agx';
 scene.render();report.png=document.getElementById('view').toDataURL('image/png');
 const loose=await loadLevel(B,scene,'/loose.json',{sky:false});const wood=loose.entities.flatMap(r=>r.meshes).map(m=>m.material).find(m=>m?.name==='wood'&&m.metallicTexture);report.loosePbr={mr:!!wood?.metallicTexture,ao:!!wood?.ambientTexture,emission:!!wood?.emissiveTexture};loose.dispose();
 return report;
 });
 fs.writeFileSync(path.join(out,'lighting.png'),Buffer.from(report.png.split(',')[1],'base64'));delete report.png;
 await page.goto(origin+'/lighting.html?verify=1');await page.waitForFunction(()=>window.heronPlaytest,null,{timeout:120000});await page.evaluate(()=>{document.getElementById('menu').hidden=true;});await page.screenshot({path:path.join(out,'standalone.png')});report.errors=errors;fs.writeFileSync(path.join(out,'results.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
 if(!Object.values(report.toneMaps).every(v=>v>0)||new Set(Object.values(report.toneMaps).map(v=>Math.round(v*100))).size<3||errors.length||!report.layers.tangent||report.shadows.count===0||report.shadowPixelDifference<.01||report.background||report.post===0||!report.plugins||report.diffusePixelDifference<.01||report.reflectionPixelDifference<.01||report.gradingPixelDifference<.01||!Object.values(report.loosePbr).every(Boolean))throw new Error('Babylon feature regression');
 }finally{await browser.close();server.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
