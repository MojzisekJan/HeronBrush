// Keep each triangle once. Spatial octrees duplicate triangles across cells and
// can consume much more memory than the level on broad, nearly planar geometry.
// Three.js still performs the exact triangle/ray/capsule intersection tests.
export function collisionTree(THREE, Octree, group) {
  const tree = new Octree();
  let root = null, order;
  tree.build = function() {
    const triangles = this.triangles;
    order = Uint32Array.from({length:triangles.length}, (_,index)=>index);
    let nodes = 0, leaves = 0, maxDepth = 0;
    function build(start,end,depth) {
      const box = new THREE.Box3();
      const low = [Infinity,Infinity,Infinity], high = [-Infinity,-Infinity,-Infinity];
      for(let i=start;i<end;i++) {
        const t=triangles[order[i]];
        box.expandByPoint(t.a);box.expandByPoint(t.b);box.expandByPoint(t.c);
        const center=[(t.a.x+t.b.x+t.c.x)/3,(t.a.y+t.b.y+t.c.y)/3,(t.a.z+t.b.z+t.c.z)/3];
        for(let axis=0;axis<3;axis++){low[axis]=Math.min(low[axis],center[axis]);high[axis]=Math.max(high[axis],center[axis]);}
      }
      // Pad both sides so a transformed top face cannot round out of the tree.
      box.expandByScalar(1e-5);
      const node={box,start,end};nodes++;maxDepth=Math.max(maxDepth,depth);
      const spans=high.map((value,axis)=>value-low[axis]);
      const axis=spans[1]>spans[0] ? (spans[2]>spans[1]?2:1) : (spans[2]>spans[0]?2:0);
      if(end-start<=16 || depth>=32 || !(spans[axis]>0)) {leaves++;return node;}
      const key=['x','y','z'][axis],middle=low[axis]+spans[axis]/2;
      let left=start,right=end-1;
      while(left<=right) {
        const t=triangles[order[left]],center=(t.a[key]+t.b[key]+t.c[key])/3;
        if(center<middle) left++;
        else {const swap=order[left];order[left]=order[right];order[right--]=swap;}
      }
      if(left===start || left===end) left=start+Math.floor((end-start)/2);
      node.left=build(start,left,depth+1);node.right=build(left,end,depth+1);
      return node;
    }
    root=triangles.length ? build(0,triangles.length,0) : null;
    this.box=root?.box || new THREE.Box3();
    this.heronStats={triangles:triangles.length,triangleReferences:order.length,nodes,leaves,maxDepth};
    return this;
  };
  function query(intersects, result) {
    if(!root) return;
    const stack=[root];
    while(stack.length) {
      const node=stack.pop();if(!intersects(node.box)) continue;
      if(node.left) {stack.push(node.right,node.left);continue;}
      for(let i=node.start;i<node.end;i++) result.push(tree.triangles[order[i]]);
    }
  }
  tree.getRayTriangles=(ray,result)=>query(box=>ray.intersectsBox(box),result);
  tree.getCapsuleTriangles=(capsule,result)=>query(box=>capsule.intersectsBox(box),result);
  tree.getSphereTriangles=(sphere,result)=>query(box=>sphere.intersectsBox(box),result);
  tree.getBoxTriangles=(bounds,result)=>query(box=>bounds.intersectsBox(box),result);
  const clear=tree.clear;
  tree.clear=function(){root=null;order=undefined;return clear.call(this);};
  return tree.fromGraphNode(group);
}
