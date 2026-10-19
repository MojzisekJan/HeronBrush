// Triangulate authored hulls once; retain world buffers while only poses change.
import * as THREE from "three";
import {ConvexGeometry} from "three/addons/geometries/ConvexGeometry.js";

export function localCollisionTriangles(entity) {
  const collision=entity.collision;
  const triangles=collision.triangles ? collision.triangles.slice() : [];
  for(const hull of collision.hulls || []) {
    const points=[];
    for(let i=0;i<hull.length;i+=3)points.push(new THREE.Vector3(hull[i],hull[i+1],hull[i+2]));
    const geometry=new ConvexGeometry(points);
    for(const value of geometry.attributes.position.array)triangles.push(value);
    geometry.dispose();
  }
  return Float64Array.from(triangles);
}

export function transformCollisionTriangles(source,matrix,destination) {
  const e=matrix.elements;
  for(let i=0;i<source.length;i+=3) {
    const x=source[i],y=source[i+1],z=source[i+2];
    const w=1/(e[3]*x+e[7]*y+e[11]*z+e[15]);
    destination[i]=(e[0]*x+e[4]*y+e[8]*z+e[12])*w;
    destination[i+1]=(e[1]*x+e[5]*y+e[9]*z+e[13])*w;
    destination[i+2]=(e[2]*x+e[6]*y+e[10]*z+e[14])*w;
  }
}

export function updateCollisionPart(part) {
  const matrix=part.object.matrixWorld;
  const elements=matrix.elements;
  if(part.pose && elements.every((value,i)=>value===part.pose[i]))return false;
  transformCollisionTriangles(part.local,matrix,part.tris);
  if(!part.pose)part.pose=new Float64Array(16);
  part.pose.set(elements);part.revision=(part.revision || 0)+1;
  return true;
}
