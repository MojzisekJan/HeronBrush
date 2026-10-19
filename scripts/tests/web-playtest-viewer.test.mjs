import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const protocol = await import('data:text/javascript;base64,' + Buffer.from(await readFile(new URL('../../crates/heron-brush/src/playtest_protocol.js',import.meta.url))).toString('base64'));
const source = await readFile(process.env.HERON_VIEWER_SOURCE || new URL('../../crates/heron-brush-web/playtest.mjs',import.meta.url),'utf8');
const data = {level:{scale:1,entities:[{name:'world',mesh:'world.glb'}]},meshes:{'world.glb':'x'.repeat(1024*1024)},textures:{}};
const html = '<script type="importmap">{}</script><script id="level-data" type="application/json">'+JSON.stringify(data)+'</script>';
async function viewer(update) {
  const frames=[], events=new Map(), sent=[], status={};
  let channel, packs=0;
  globalThis.location={hash:'#viewer-test',href:'http://localhost/play.html',origin:'http://localhost'};
  globalThis.document={getElementById:()=>status,createElement:()=>({style:{},contentWindow:{heronPlaytestUpdate:update,postMessage(){}},remove(){this.removed=true;}}),body:{append(frame){frames.push(frame);}}};
  globalThis.window={addEventListener:(name,fn)=>events.set(name,fn)};
  globalThis.BroadcastChannel=class {constructor(){channel=this;} postMessage(value){sent.push(value);} close(){}};
  const oldInterval=globalThis.setInterval;
  globalThis.setInterval=()=>0;
  globalThis.viewerProtocol={...protocol,pack(...args){packs++;return protocol.pack(...args);}};
  try {await import('data:text/javascript;base64,'+Buffer.from(source.replace(/^import .*\n/,'const {unpack,pack,apply,unchanged}=globalThis.viewerProtocol;\n')+`\n// instance ${Math.random()}`).toString('base64'));}
  finally {globalThis.setInterval=oldInterval;}
  const send=message=>channel.onmessage({data:message});
  const ready=()=>events.get('message')({origin:location.origin,source:frames.at(-1).contentWindow,data:{type:'heron-ready'}});
  await send({type:'level',html,mapKey:'map',revision:1});ready();
  return {frames,sent,status,send,ready,packs:()=>packs};
}
const patch=(base,revision)=>({type:'patch',mapKey:'map',base,revision,patch:{level:{scale:1},entities:[{name:'world',fields:{value:revision},remove:[]}],entityOrder:['world'],meshes:{},textures:{},removeMeshes:[],removeTextures:[]}});
test('compatible repeated updates skip full HTML packing and retain the iframe',async()=>{
  const updates=[];const v=await viewer(async value=>{updates.push(value);return true;});
  await v.send(patch(1,2));await v.send(patch(2,3));
  assert.equal(v.packs(),0);assert.equal(v.frames.length,1);assert.equal(v.status.hidden,true);
  assert.equal(updates[1].level.entities[0].value,3);assert.equal(updates[1].meshes['world.glb'],data.meshes['world.glb']);
});
test('an incompatible update packs a complete replacement with the latest patch',async()=>{
  const v=await viewer(async()=>false);await v.send(patch(1,2));
  assert.equal(v.packs(),1);assert.equal(v.frames.length,2);
  assert.equal(protocol.unpack(v.frames[1].srcdoc).data.level.entities[0].value,2);
  assert.match(v.frames[1].srcdoc,/http:\/\/localhost\/vendor\/three/);
});
test('an update error keeps the current frame and reports the error without packing',async()=>{
  const v=await viewer(async()=>{throw Error('fixture failure');});await v.send(patch(1,2));
  assert.equal(v.packs(),0);assert.equal(v.frames.length,1);assert.match(v.status.textContent,/fixture failure/);
});
test('out of order patches request resync without rebuilding HTML',async()=>{
  const v=await viewer(async()=>true);await v.send(patch(9,10));
  assert.equal(v.packs(),0);assert.equal(v.frames.length,1);assert.deepEqual(v.sent.at(-1),{type:'ready'});
});

test('a full resync after a patch reaches the active frame even when it matches the original HTML',async()=>{
  const values=[];const v=await viewer(async value=>{values.push(value);return true;});
  await v.send(patch(1,2));await v.send({type:'level',html,mapKey:'map',revision:3});
  assert.equal(values.length,2);assert.equal(values[1].level.entities[0].value,undefined);
  assert.equal(v.frames.length,1);assert.equal(v.packs(),0);
});
test('an older asynchronous fallback cannot replace a newer successful update',async()=>{
  let release, calls=0;
  const v=await viewer(()=>++calls===1 ? new Promise(resolve=>{release=resolve;}) : Promise.resolve(true));
  const first=v.send(patch(1,2));await v.send(patch(2,3));release(false);await first;
  assert.equal(v.frames.length,1);assert.equal(v.packs(),0);assert.equal(v.status.hidden,true);
});

test('an unchanged patch advances revision without rebuilding the active scene',async()=>{
  let updates=0;const v=await viewer(async()=>{updates++;return true;});
  const empty=patch(1,2);empty.patch.entities=[];await v.send(empty);
  assert.equal(updates,0);assert.equal(v.packs(),0);
  await v.send(patch(2,3));assert.equal(updates,1);
});
test('metadata and entity order changes are not mistaken for unchanged patches',()=>{
  const empty=patch(1,2).patch;empty.entities=[];
  assert.equal(protocol.unchanged(data,empty),true);
  assert.equal(protocol.unchanged(data,{...empty,level:{scale:2}}),false);
  assert.equal(protocol.unchanged(data,{...empty,entityOrder:[]}),false);
  assert.equal(protocol.unchanged(data,{...empty,removeMeshes:['world.glb']}),false);
});

if (process.env.HERON_VIEWER_BENCH === '1') test('profile large compatible updates',async()=>{
  data.meshes['world.glb']='x'.repeat(8*1024*1024);
  const v=await viewer(async()=>true);
  // Bootstrap data uses the same template as ordinary updates; subsequent patches
  // carry a large mesh once and retain it for the rest of the sequence.
  const initial=patch(1,2);initial.patch.meshes=data.meshes;await v.send(initial);
  const times=[];
  for(let i=0;i<20;i++) {const start=performance.now();await v.send(patch(i+2,i+3));times.push(performance.now()-start);}
  times.sort((a,b)=>a-b);
  console.log(JSON.stringify({benchmark:'compatible-viewer-update',meshBytes:data.meshes['world.glb'].length,iterations:times.length,medianMs:times[10],p95Ms:times[18],packs:v.packs()}));
});
