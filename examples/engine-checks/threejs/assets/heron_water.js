// Optional water adapter. Queries are reusable by a game's own player/controller.
export class HeronWater {
  constructor(THREE,json,object) {
    this.THREE=THREE;this.data=typeof json==='string'?JSON.parse(json):json;this.object=object;this.materials=[];
    if(this.data.version!==1||!Array.isArray(this.data.volumes))throw new Error('Unsupported water payload');
  }
  sample(point) {
    this.object.updateWorldMatrix(true,false);
    const p=this.object.worldToLocal(point.clone());let depth=-Infinity;
    for(const volume of this.data.volumes)if(volume.planes.length>=4&&volume.planes.every(v=>v.x*p.x+v.y*p.y+v.z*p.z<=v.d+1e-5))depth=Math.max(depth,volume.surface-p.y);
    if(depth<0)return null;
    const scale=this.object.getWorldScale(new this.THREE.Vector3());
    return {water:this,data:this.data,depth:depth*Math.abs(scale.y)};
  }
  material(material) {
    const d=this.data;if(!d.surface_material)return;
    material.color.setRGB(...d.color);material.opacity=d.opacity;material.transparent=true;material.depthWrite=false;
    material.side=this.THREE.DoubleSide;material.roughness=0.14;material.metalness=0.05;
    const uniforms={hbTime:{value:0},hbHeight:{value:d.wave_height},hbSpeed:{value:d.wave_speed}};
    material.onBeforeCompile=shader=>{Object.assign(shader.uniforms,uniforms);shader.vertexShader='uniform float hbTime; uniform float hbHeight; uniform float hbSpeed;\n'+shader.vertexShader;
      shader.vertexShader=shader.vertexShader.replace('#include <begin_vertex>','#include <begin_vertex>\ntransformed.y += smoothstep(0.5,0.9,normal.y)*hbHeight*sin(position.x*1.7+hbTime*hbSpeed)*cos(position.z*1.3-hbTime*hbSpeed*0.8);');};
    material.customProgramCacheKey=()=> 'heron-water-v1';this.materials.push(uniforms);material.needsUpdate=true;
  }
  animate(time){for(const u of this.materials)u.hbTime.value=time;}
  static sample(waters,point){let best=null;for(const water of waters){const next=water.sample(point);if(next&&(!best||next.data.priority>best.data.priority||(next.data.priority===best.data.priority&&next.depth>best.depth)))best=next;}return best;}
}
