// A small first-person walker against the level's triangle soup: gravity,
// jump, mouse look, and a step-up so kerbs and stairs are walkable. The feel
// and the numbers are lifted from the raylib example's player.c.

import * as THREE from "three";
import {HeronWater} from "../assets/heron_water.js";

const RADIUS = 0.35;
const HALF = 0.9; // half the capsule height
const EYE = 0.68; // eye above the capsule centre
const STEP = 0.5; // tallest step that can be walked up
const GRAVITY = 20;
const JUMP_VEL = 7;
const WALK = 5;
const RUN = 8.5;

export class Player {
  feet() { return this.pos.clone().add(new THREE.Vector3(0, -HALF, 0)); }
  teleport(feet) {
    this.pos.copy(feet).y += HALF;
    this.vel.set(0,0,0); this.grounded = false; this.wantJump = false;
    this.keys = Object.create(null);
  }
  constructor(collider, camera, feet, yaw, waters=[]) {
    this.waters=waters;this.waterSample=null;
    this.collider = collider;
    this.camera = camera;
    this.pos = feet.clone().add(new THREE.Vector3(0, HALF, 0)); // capsule centre
    this.spawn = this.pos.clone();
    this.vel = new THREE.Vector3();
    this.yaw = yaw;
    this.pitch = 0;
    this.grounded = false;
    this.keys = Object.create(null);
    this.wantJump = false;

    addEventListener("keydown", (e) => {
      this.keys[e.code] = true;
      if (e.code === "Space") this.wantJump = true;
    });
    addEventListener("keyup", (e) => (this.keys[e.code] = false));
    addEventListener("blur", () => (this.keys = Object.create(null)));
    addEventListener("mousemove", (e) => {
      if (!document.pointerLockElement) return;
      this.yaw -= e.movementX * 0.0022;
      this.pitch = clamp(this.pitch - e.movementY * 0.0022, -1.5, 1.5);
    });
  }

  update(dt) {
    dt = Math.min(dt, 0.05);
    const s = Math.sin(this.yaw);
    const c = Math.cos(this.yaw);
    const fwd = new THREE.Vector3(-s, 0, -c);
    const right = new THREE.Vector3(c, 0, -s);

    const f = (this.keys.KeyW ? 1 : 0) - (this.keys.KeyS ? 1 : 0);
    const st = (this.keys.KeyD ? 1 : 0) - (this.keys.KeyA ? 1 : 0);
    const wish = fwd.multiplyScalar(f).add(right.multiplyScalar(st));
    if (wish.lengthSq() > 1e-4) wish.normalize();
    this.waterSample=HeronWater.sample(this.waters,this.pos);
    const wet=this.waterSample;
    const top = wet ? wet.data.swim_speed : this.keys.ShiftLeft || this.keys.ShiftRight ? RUN : WALK;

    const grip = wet ? wet.data.drag : this.grounded && !this.wantJump ? 12 : 2.5;
    this.vel.x += (wish.x * top - this.vel.x) * clamp(grip * dt, 0, 1);
    this.vel.z += (wish.z * top - this.vel.z) * clamp(grip * dt, 0, 1);
    if(wet){const rise=!!this.keys.Space,dive=!!this.keys.ControlLeft,d=wet.data;const vertical=(rise?d.rise_speed:0)-(dive?d.rise_speed:0)-(!rise&&!dive?d.sink_speed:0);this.vel.y+=(vertical-this.vel.y)*(1-Math.exp(-d.drag*dt));this.pos.addScaledVector(this.vel,dt);this.collider.resolveCapsule(this.pos,RADIUS,HALF,this.vel);this.grounded=false;this.wantJump=false;this.camera.position.set(this.pos.x,this.pos.y+EYE,this.pos.z);this.camera.rotation.set(this.pitch,this.yaw,0,"YXZ");return;}
    this.vel.y -= GRAVITY * dt;

    if (this.grounded) {
      if (this.vel.y < 0) this.vel.y = -3;
      if (this.wantJump) {
        this.vel.y = JUMP_VEL;
        this.grounded = false;
      }
    }
    this.wantJump = false;

    const wasGrounded = this.grounded;

    // Horizontal move: lift the whole capsule by a step so short kerbs are
    // glided over, then plant the feet back down.
    const lift = wasGrounded ? STEP : 0;
    const p = this.pos.clone();
    p.x += this.vel.x * dt;
    p.z += this.vel.z * dt;
    p.y += lift;
    let g = this.collider.resolveCapsule(p, RADIUS, HALF, this.vel);
    if (wasGrounded) {
      const foot = new THREE.Vector3(p.x, p.y - HALF + 0.05, p.z);
      const drop = this.collider.floorBelow(foot, lift + 0.45);
      if (drop >= 0) {
        p.y += 0.05 - drop;
        g = true;
      } else {
        p.y -= lift;
      }
    }
    this.pos.copy(p);

    if (wasGrounded && g && !this.keys.Space) {
      this.grounded = true;
      if (this.vel.y > 0) this.vel.y = 0;
    } else {
      this.pos.y += this.vel.y * dt;
      this.grounded = this.collider.resolveCapsule(this.pos, RADIUS, HALF, this.vel);
    }

    if (this.pos.y < -40) {
      this.pos.copy(this.spawn);
      this.vel.set(0, 0, 0);
    }

    this.camera.position.set(this.pos.x, this.pos.y + EYE, this.pos.z);
    this.camera.rotation.set(this.pitch, this.yaw, 0, "YXZ");
  }
}

function clamp(v, lo, hi) {
  return v < lo ? lo : v > hi ? hi : v;
}
