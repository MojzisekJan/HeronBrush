// node verify_custom_game.cjs /tmp/heron-three/package /tmp/heron-three-custom-game/export/workshop.json
// Uses the real importer in Chromium, with project-owned puzzle factories.
const {chromium}=require('playwright');
const fs=require('node:fs'),path=require('node:path'),http=require('node:http'),assert=require('node:assert/strict');
(async()=>{
  const repository=path.resolve(__dirname,'../../..'),packageRoot=path.resolve(process.argv[2]),fixtures=path.dirname(path.resolve(process.argv[3]));
  const html='<script type="importmap">'+JSON.stringify({imports:{three:'/three/build/three.module.js','three/addons/':'/three/examples/jsm/'}})+'</script>';
  const server=http.createServer((request,response)=>{
    const pathname=new URL(request.url,'http://localhost').pathname;
    if(pathname==='/'){response.setHeader('Content-Type','text/html');response.end(html);return;}
    const prefix=pathname.startsWith('/three/')?'/three/':pathname.startsWith('/fixtures/')?'/fixtures/':'/';
    const root=prefix==='/three/'?packageRoot:prefix==='/fixtures/'?fixtures:repository;
    const file=path.resolve(root,decodeURIComponent(pathname.slice(prefix.length)));
    if(!file.startsWith(root+path.sep)||!fs.existsSync(file)){response.statusCode=404;response.end();return;}
    response.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.json')?'application/json':'application/octet-stream');response.end(fs.readFileSync(file));
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));let browser;
  try{
    browser=await chromium.launch({headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
    const page=await browser.newPage(),errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    const result=await page.evaluate(async filename=>{
      const THREE=await import('three');
      const {loadLevel}=await import('/examples/engine-checks/threejs/src/heron_brushlevel.js');
      const {Collider}=await import('/examples/engine-checks/threejs/src/collision.js');
      function check(value,message){if(!value)throw new Error(message);}
      let panel,beacon,configured=0,released=0;
      const factories={
        panel:async(record,context)=>{
          panel=record;configured++;
          check(record.properties.armed===true,'Boolean default lost');
          check(JSON.stringify(record.properties.sequence)==='[2,3,5]','Typed array lost');
          check(context.byName.get(record.targets[0])?.classname==='beacon','Forward target missing');
          record.object.add(new THREE.Mesh(new THREE.BoxGeometry(2,2,2),new THREE.MeshBasicMaterial()));
          record.object.userData.activate=()=>context.byName.get(record.targets[0]).object.userData.charge++;
          return ()=>released++;
        },
        beacon:record=>{beacon=record;configured++;check(record.properties.charge===7,'Authored override lost');record.object.userData.charge=record.properties.charge;return ()=>released++;},
      };
      const scene=await loadLevel('/fixtures/'+filename,{factories});
      check(configured===2,'Project factories not invoked');panel.object.userData.activate();check(beacon.object.userData.charge===8,'Own puzzle action failed');
      const collider=new Collider(scene.colliderParts),part=scene.colliderParts.find(part=>part.object===panel.object);
      check(part,'Custom brush collision missing');const buffer=part.tris,revision=part.revision;
      scene.advance(0,new THREE.Vector3(20,20,20));collider.setParts(scene.colliderParts);check(part.revision===revision,'Unchanged pose rebuilt');
      const index=scene.colliderParts.slice(0,scene.colliderParts.indexOf(part)).reduce((sum,part)=>sum+part.tris.length/9,0);
      const triangle=collider.tris[index],oldX=triangle.a.x;
      panel.object.position.x+=3;scene.advance(0,new THREE.Vector3(20,20,20));collider.setParts(scene.colliderParts);
      check(part.tris===buffer&&collider.tris[index]===triangle,'Pose update reallocated collision');
      check(Math.abs(triangle.a.x-oldX-3)<1e-10,'Project-owned moving collider stale');
      const before=triangle.a.clone();scene.root.position.set(4,2,-3);scene.root.rotation.y=0.3;scene.root.scale.setScalar(1.2);
      scene.advance(0,new THREE.Vector3(20,20,20));collider.setParts(scene.colliderParts);
      check(triangle.a.distanceTo(before)>1,'Level parent transform ignored');
      const source=new THREE.Vector3(...part.local.slice(0,3)).applyMatrix4(panel.object.matrixWorld);
      check(source.distanceTo(triangle.a)<1e-10,'World collision disagrees with native object matrix');
      const projectGeometry=panel.object.children[0].geometry;let projectDisposed=false;projectGeometry.addEventListener("dispose",()=>projectDisposed=true);
      const second=await loadLevel('/fixtures/'+filename,{factories});check(second.root!==scene.root&&scene.root.children.length===3,'Reload mutated existing level');
      scene.dispose();scene.dispose();check(released===2&&!projectDisposed,"Cleanup repeated or disposed project assets");check(second.root.children.length===3,"Disposing one copy affected another");second.dispose();check(released===4,"Second copy cleanup missing");projectGeometry.dispose();
      return {dispose:true,factories:configured,typedProperties:true,forwardReference:true,customMovement:true,parentTransform:true,reload:true};
    },path.basename(process.argv[3]));
    assert.deepEqual(errors,[]);console.log(JSON.stringify(result));console.log('HERON_THREE_CUSTOM_GAME_OK');
  }finally{if(browser)await browser.close();await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1;});
