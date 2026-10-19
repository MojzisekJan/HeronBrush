import * as THREE from "three";
import { loadLevel, findSpawn } from "./heron_brushlevel.js";
import { Collider } from "./collision.js";
import { Player } from "./player.js";
import {HeronWater} from "../assets/heron_water.js";

const canvas = document.getElementById("view");
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFShadowMap; // PCFSoftShadowMap is deprecated for WebGL since r182
renderer.outputColorSpace = THREE.SRGBColorSpace;

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x7891b8);

const camera = new THREE.PerspectiveCamera(70, 1, 0.1, 2000);

// The map's own light: a warm sun plus a soft fill. `light_sun` in the level
// gives the direction.
const sun = new THREE.DirectionalLight(0xfff3e0, 2.1);
sun.castShadow = true;
sun.shadow.mapSize.set(2048, 2048);
sun.shadow.camera.left = sun.shadow.camera.bottom = -40;
sun.shadow.camera.right = sun.shadow.camera.top = 40;
sun.shadow.camera.far = 120;
sun.shadow.bias = -0.0006;
scene.add(sun, sun.target);
scene.add(new THREE.HemisphereLight(0xbcd0ea, 0x54503f, 1.1));

const hud = document.getElementById("hud");

function resize() {
  const w = innerWidth;
  const h = innerHeight;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}
addEventListener("resize", resize);
resize();

const parameters = new URLSearchParams(location.search);
const level = await loadLevel(new URL(parameters.get('level') || "../assets/demo.json", import.meta.url).href);
scene.add(level.root);

const collider = new Collider(level.colliderParts);
const spawn = findSpawn(level.entities);
const player = new Player(collider, camera, spawn.feet, spawn.yaw,level.waters);

// Aim the sun down the light_sun direction if the map has one.
const lightSun = level.entities.find((e) => e.classname === "light_sun");
let sunDir = new THREE.Vector3(-0.45, -0.85, 0.4).normalize();
if (lightSun) {
  const [pitch, yaw] = lightSun.rotationYXZ;
  const f = new THREE.Vector3(
    -Math.sin(yaw) * Math.cos(pitch),
    Math.sin(pitch),
    -Math.cos(yaw) * Math.cos(pitch),
  );
  if (f.y > 0) f.negate();
  sunDir.copy(f.normalize());
}

canvas.addEventListener("click", () => canvas.requestPointerLock());
document.addEventListener("pointerlockchange", () => {
  hud.classList.toggle("locked", !!document.pointerLockElement);
});

const timer = new THREE.Timer(); // THREE.Clock is deprecated since r183
function frame(timestamp) {
  timer.update(timestamp);
  const dt = timer.getDelta();
  if(!parameters.has("verify"))step(dt);
  sun.position.copy(camera.position).addScaledVector(sunDir, -40);
  sun.target.position.copy(camera.position);
  const wet=HeronWater.sample(level.waters,camera.position);scene.fog=wet?new THREE.FogExp2(new THREE.Color(...wet.data.color),wet.data.fog_density):null;
  renderer.render(scene, camera);
  requestAnimationFrame(frame);
}
function step(dt) {
  const feet = player.feet();
  let support = null;
  if (player.grounded && !player.wantJump) {
    collider.floorBelow(feet.clone().add(new THREE.Vector3(0,0.05,0)), 0.15);
    support = collider.support;
  }
  const previous = support?.matrixWorld.clone();
  level.advance(Math.min(dt,0.05), feet);
  if (support) {
    player.pos.applyMatrix4(new THREE.Matrix4().multiplyMatrices(support.matrixWorld, previous.invert()));
  }
  collider.setParts(level.colliderParts);
  player.update(dt);
}
if (parameters.has('verify')) {
  window.heronPlaytest = {step, THREE, animations:level.animations, velocity:player.vel,
    get grounded() {return player.grounded;},waters:level.waters,get water(){return player.waterSample;},camera,scene, feet:()=>player.feet(),
    teleport:feet=>player.teleport(feet), face:yaw=>{player.yaw=yaw;},
    keys:{add(key){player.keys[key]=true;if(key==='Space')player.wantJump=true;},
      delete(key){delete player.keys[key];},clear(){player.keys=Object.create(null);player.wantJump=false;}}};
}
frame();
const report = document.createElement('pre');
report.id = 'verification';
report.style.cssText = 'position:fixed;left:12px;top:12px;color:white;background:#18202bbb;padding:10px';
document.body.append(report);
const reportTimer = setInterval(() => {
 report.textContent = JSON.stringify({entities:level.entities.length, meshes:level.entities.filter(e=>e.object.children.length).length,
  triangles:collider.tris.length, player:player.pos.toArray().map(v=>+v.toFixed(3)),
  grounded:player.grounded, textures:level.level.textures.filter(t=>t.image).length,
  targets:level.entities.reduce((n,e)=>n+e.targets.length,0)},null,2);
},1000);
