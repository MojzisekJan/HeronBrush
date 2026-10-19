// node verify_interchange.cjs /tmp/heron-three/package /tmp/heron-interchange-custom/workshop.map
// Actual GLTFLoader/OBJLoader consume the three portable export formats in Chromium.
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
    const result=await page.evaluate(async()=>{
      const THREE=await import('three');
      const {GLTFLoader}=await import('three/addons/loaders/GLTFLoader.js');
      const {OBJLoader}=await import('three/addons/loaders/OBJLoader.js');
      function check(value,message){if(!value)throw new Error(message);}
      const loader=new GLTFLoader(),gltf=await loader.loadAsync('/fixtures/gltf/workshop.gltf'),glb=await loader.loadAsync('/fixtures/glb/workshop.glb');
      const obj=await new OBJLoader().loadAsync('/fixtures/obj/workshop.obj');
      let bounds;
      for(const scene of [gltf.scene,glb.scene,obj]){
        scene.updateMatrixWorld(true);const box=new THREE.Box3().setFromObject(scene);check(!box.isEmpty(),'Empty imported geometry');
        if(bounds)check(box.min.distanceTo(bounds.min)<1e-6&&box.max.distanceTo(bounds.max)<1e-6,'Interchange format bounds disagree');else bounds=box;
        let triangles=0;scene.traverse(object=>{if(object.isMesh)triangles+=(object.geometry.index?.count||object.geometry.attributes.position.count)/3;});check(triangles===12,'Cube triangles lost');
      }
      for(const data of [gltf,glb]){
        const beacon=data.scene.getObjectByName('beacon');check(beacon,'Custom point entity missing');
        const metadata=beacon.userData.heron;check(metadata?.version===1,'Typed extras missing');
        check(metadata.properties.charge.type==='int'&&metadata.properties.charge.value===7,'Authored typed override');
        check(metadata.properties.armed.value===true,'Boolean default');
        check(metadata.properties.sequence.value.map(item=>item.value).join(',')==='2,3,5','Typed array default');
        check(beacon.userData.entity.charge==='7','Original metadata changed');
        const ownGameComponent={charge:metadata.properties.charge.value,activate(){this.charge++;}};ownGameComponent.activate();check(ownGameComponent.charge===8,'Own importer action');
      }
      return {formats:['glTF','GLB','OBJ'],triangles:12,typedExtras:true,authoredOverride:true,bounds:{min:bounds.min.toArray(),max:bounds.max.toArray()}};
    });
    assert.deepEqual(errors,[]);console.log(JSON.stringify(result));console.log('HERON_INTERCHANGE_CUSTOM_GAME_OK');
  }finally{if(browser)await browser.close();await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1;});
