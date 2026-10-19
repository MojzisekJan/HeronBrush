// node bench_collision.mjs /path/to/three/package /tmp/heron-motion-three/motion_lab.json
// Compare the former hot loop with cached hulls on real exported geometry.
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';
const packageRoot=path.resolve(process.argv[2]);
const core=pathToFileURL(path.join(packageRoot,'build/three.module.js')).href;
const convex=pathToFileURL(path.join(packageRoot,'examples/jsm/geometries/ConvexGeometry.js')).href;
const THREE=await import(core),{ConvexGeometry}=await import(convex);
const directory=path.dirname(new URL(import.meta.url).pathname);
const source=fs.readFileSync(path.join(directory,'src/collision_mesh.js'),'utf8').replace('"three"',JSON.stringify(core)).replace('"three/addons/geometries/ConvexGeometry.js"',JSON.stringify(convex));
const geometry=await import('data:text/javascript;base64,'+Buffer.from(source).toString('base64'));
const collision=fs.readFileSync(path.join(directory,'src/collision.js'),'utf8').replace(/import \* as THREE[^;]+;/,'').replace('export class','class');
const Collider=new Function('THREE',collision+'; return Collider;')(THREE);
const json=JSON.parse(fs.readFileSync(process.argv[3]));
function legacyTriangles(entity,object){
  const triangles=entity.collision.triangles?.slice() || [];
  for(const hull of entity.collision.hulls || []){
    const points=[];for(let i=0;i<hull.length;i+=3)points.push(new THREE.Vector3(...hull.slice(i,i+3)));
    const mesh=new ConvexGeometry(points);triangles.push(...mesh.attributes.position.array);mesh.dispose();
  }
  for(let i=0;i<triangles.length;i+=3){const p=new THREE.Vector3(...triangles.slice(i,i+3)).applyMatrix4(object.matrixWorld);triangles[i]=p.x;triangles[i+1]=p.y;triangles[i+2]=p.z;}
  return triangles;
}
function legacyRecords(parts){
  const records=[];
  for(const part of parts)for(let i=0;i+8<part.tris.length;i+=9){
    const t=part.tris,a=new THREE.Vector3(t[i],t[i+1],t[i+2]),b=new THREE.Vector3(t[i+3],t[i+4],t[i+5]),c=new THREE.Vector3(t[i+6],t[i+7],t[i+8]);
    records.push({a,b,c,normal:new THREE.Vector3().subVectors(b,a).cross(new THREE.Vector3().subVectors(c,a)).normalize(),min:a.clone().min(b).min(c),max:a.clone().max(b).max(c)});
  }
  return records;
}
function measure(cached){
  const parts=json.entities.filter(e=>e.collision&&!e.collision.sensor).map(entity=>{
    const object=new THREE.Object3D();object.position.set(...entity.transform.translation);object.rotation.set(...entity.transform.rotation_yxz,'YXZ');object.updateMatrixWorld(true);
    const local=geometry.localCollisionTriangles(entity),part={entity,object,local,tris:new Float64Array(local.length),offset:new THREE.Vector3()};geometry.updateCollisionPart(part);
    const expected=legacyTriangles(entity,object);assert.equal(expected.length,part.tris.length);for(let i=0;i<expected.length;i++)assert(Math.abs(expected[i]-part.tris[i])<1e-12);
    return part;
  });
  const moving=parts.filter(p=>p.entity.properties._heron_animation),collider=new Collider(parts);
  let records;
  const tick=()=>{for(const part of moving){part.object.rotation.y+=.01;part.object.updateMatrixWorld(true);if(cached)geometry.updateCollisionPart(part);else part.tris=legacyTriangles(part.entity,part.object);}if(cached){collider.setParts(parts);records=collider.tris;}else records=legacyRecords(parts);};
  for(let i=0;i<200;i++)tick();
  const samples=[];for(let repeat=0;repeat<5;repeat++){const begin=performance.now();for(let i=0;i<2000;i++)tick();samples.push((performance.now()-begin)/2000);}
  return {moving:moving.length,triangles:records.length,millisecondsPerFrame:samples,median:[...samples].sort((a,b)=>a-b)[2]};
}
const before=measure(false),after=measure(true);
console.log(JSON.stringify({before,after,speedup:before.median/after.median},null,2));
