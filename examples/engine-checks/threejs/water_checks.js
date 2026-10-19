// Shared by the visible browser fixture and the optional Playwright runner.
export function runWaterCheck(p) {
 const T=p.THREE,dt=1/60,frames=n=>{for(let i=0;i<n;i++)p.step(dt);};
 const check=(ok,label)=>{if(!ok)throw Error(label);};
 check(p.waters.length===2,'two enabled volumes, disabled one excluded');
 p.teleport(new T.Vector3(0,-3,0));frames(1);check(p.water?.data.priority===10,'overlap priority');
 const start=p.feet().y;p.keys.add('Space');frames(25);check(p.feet().y>start+.25,'rise through water');
 p.keys.clear();p.teleport(new T.Vector3(0,-2,0));p.keys.add('ControlLeft');frames(20);check(p.feet().y < -2.25,'dive');
 p.keys.clear();p.teleport(new T.Vector3(3,-2,0));frames(30);check(p.feet().y < -2.1,'idle sink');
 p.teleport(new T.Vector3(3,-2,0));p.keys.add('KeyD');frames(20);check(Math.abs(p.feet().x-3)>.1,'horizontal swim');
 p.keys.clear();p.teleport(new T.Vector3(8,1,0));frames(30);check(!p.water,'exit restores dry movement');
 const water=p.waters[0],before=water.object.position.clone();water.object.position.set(7,2,3);water.object.rotation.y=.7;water.object.scale.set(1.5,1.5,1.5);
 check(water.sample(water.object.localToWorld(new T.Vector3()))!==null,'translated rotated scaled query');
 water.object.position.copy(before);water.object.rotation.set(0,0,0);water.object.scale.set(1,1,1);
 check(water.materials.length>0,'baseline surface adapter installed');
 return {volumes:p.waters.length,checks:['overlap','rise','dive','sink','swim','exit','transform','surface']};
}
