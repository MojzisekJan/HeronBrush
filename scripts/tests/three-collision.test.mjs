import {test} from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
import {Octree} from 'three/addons/math/Octree.js';
import {Capsule} from 'three/addons/math/Capsule.js';
import {collisionTree} from './collision.mjs';
function boxes(count=30) {
  const group=new THREE.Group(), side=Math.ceil(Math.sqrt(count));
  for(let i=0;i<count;i++) {
    const mesh=new THREE.Mesh(new THREE.BoxGeometry(4,1+(i%8)*.5,4));
    mesh.position.set((i%side)*5,0,Math.floor(i/side)*5);
    if(count<100) {mesh.rotation.set(i*.03,i*.17,-i*.04);mesh.scale.set(1+(i%3)*.3,.5+(i%5)*.2,i%4 ? 1 : -1);}
    group.add(mesh);
  }
  return group;
}
function nearest(tree,ray) {
  let distance=Infinity;const point=new THREE.Vector3();
  for(const t of tree.triangles) if(ray.intersectTriangle(t.a,t.b,t.c,true,point)) distance=Math.min(distance,point.distanceTo(ray.origin));
  return distance;
}
let seed=123456;
function random(){seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296;}
test('nearest ray hits match all triangles for transformed and mirrored meshes',()=>{
  const tree=collisionTree(THREE,Octree,boxes());
  for(let i=0;i<300;i++) {
    const ray=new THREE.Ray(new THREE.Vector3(random()*35-5,12,random()*35-5),new THREE.Vector3((random()-.5)*.3,-1,(random()-.5)*.3).normalize());
    const expected=nearest(tree,ray),actual=tree.rayIntersect(ray);
    assert.equal(!!actual,Number.isFinite(expected));
    if(actual) assert.ok(Math.abs(actual.distance-expected)<1e-9);
  }
});
test('capsule, sphere and box broadphases include every exact triangle contact',()=>{
  const tree=collisionTree(THREE,Octree,boxes());
  for(let i=0;i<120;i++) {
    const p=new THREE.Vector3(random()*30-2,random()*5-2,random()*30-2);
    const capsule=new Capsule(p,p.clone().add(new THREE.Vector3(.2,1,.1)),.6);
    const sphere=new THREE.Sphere(p,.8),box=new THREE.Box3(p.clone().addScalar(-.7),p.clone().addScalar(.7));
    for(const [shape,query,intersect] of [[capsule,'getCapsuleTriangles','triangleCapsuleIntersect'],[sphere,'getSphereTriangles','triangleSphereIntersect'],[box,'getBoxTriangles','triangleBoxIntersect']]) {
      const candidates=[];tree[query](shape,candidates);const found=new Set(candidates);
      assert.equal(found.size,candidates.length,'candidate duplication');
      for(const triangle of tree.triangles) if(tree[intersect](shape,triangle)) assert.ok(found.has(triangle),query+' omitted a contact');
    }
  }
});
test('padded top faces and rebuilt moving geometry retain ray and capsule contacts',()=>{
  const group=boxes(1);group.children[0].position.set(.000003,1.1111111,-.000009);
  group.children[0].rotation.set(0,.3,0);group.children[0].scale.set(1,1,1);
  for(let i=0;i<3;i++) {
    group.position.y+=.1;const tree=collisionTree(THREE,Octree,group);
    const ray=new THREE.Ray(new THREE.Vector3(0,10,0),new THREE.Vector3(0,-1,0));
    const hit=tree.rayIntersect(ray);assert.ok(hit);
    const capsule=new Capsule(hit.position.clone().add(new THREE.Vector3(0,.2,0)),hit.position.clone().add(new THREE.Vector3(0,1.2,0)),.3);
    assert.ok(tree.capsuleIntersect(capsule));
  }
});
test('empty trees and clear have no contacts',()=>{
  const tree=collisionTree(THREE,Octree,new THREE.Group());
  assert.equal(tree.heronStats.triangleReferences,0);
  assert.equal(tree.rayIntersect(new THREE.Ray()),false);
  const filled=collisionTree(THREE,Octree,boxes(1));filled.clear();
  assert.equal(filled.rayIntersect(new THREE.Ray(new THREE.Vector3(0,10,0),new THREE.Vector3(0,-1,0))),false);
});
test('coincident triangles do not produce unbounded subdivision',()=>{
  const group=new THREE.Group();const positions=[];
  for(let i=0;i<5000;i++) positions.push(0,0,0,1,0,0,0,0,1);
  const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.Float32BufferAttribute(positions,3));group.add(new THREE.Mesh(geometry));
  const tree=collisionTree(THREE,Octree,group);
  assert.equal(tree.heronStats.nodes,1);assert.equal(tree.heronStats.triangleReferences,5000);
});
test('120k triangles fit the bounded heap with exactly one reference each',()=>{
  const group=boxes(10000),start=performance.now(),tree=collisionTree(THREE,Octree,group);
  assert.equal(tree.heronStats.triangles,120000);assert.equal(tree.heronStats.triangleReferences,120000);
  assert.ok(tree.heronStats.nodes<2*120000);assert.ok(tree.heronStats.maxDepth<=32);
  assert.ok(tree.rayIntersect(new THREE.Ray(new THREE.Vector3(1,20,1),new THREE.Vector3(0,-1,0))));
  console.log(JSON.stringify({benchmark:'collision-10000-brushes',buildMs:performance.now()-start,...tree.heronStats,rss:process.memoryUsage().rss,heap:process.memoryUsage().heapUsed}));
});
test('a falling capsule settles on the floor and cannot cross a wall',()=>{
  const group=new THREE.Group();
  const floor=new THREE.Mesh(new THREE.BoxGeometry(40,2,40));floor.position.y=-1;group.add(floor);
  const wall=new THREE.Mesh(new THREE.BoxGeometry(2,10,40));wall.position.y=5;group.add(wall);
  const tree=collisionTree(THREE,Octree,group),radius=.35;
  const capsule=new Capsule(new THREE.Vector3(-3,3,0),new THREE.Vector3(-3,4,0),radius);
  let velocity=0,grounded=false;
  for(let i=0;i<240;i++) {
    velocity-=20/120;
    capsule.translate(new THREE.Vector3(i>120?.035:0,velocity/120,0));
    const hit=tree.capsuleIntersect(capsule);
    if(hit) {
      capsule.translate(hit.normal.clone().multiplyScalar(hit.depth));
      if(hit.normal.y>.5){grounded=true;velocity=0;}
    }
    assert.ok(capsule.start.x<=-1-radius+1e-5,'capsule crossed wall');
    assert.ok(capsule.start.y-radius>=-1e-5,'capsule crossed floor');
  }
  assert.equal(grounded,true);
  assert.ok(Math.abs(capsule.start.y-radius)<1e-4);
});
