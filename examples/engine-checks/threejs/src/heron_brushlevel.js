// Reads `arena.heron_brush.json` from `heron_brush export --engine three`: loads a `.glb` per
// solid entity, hangs the level's textures on it, places it at its transform,
// and hands back the scene plus the triangle lists for the collider.

import * as THREE from "three";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";

import {localCollisionTriangles,updateCollisionPart} from "./collision_mesh.js";
import {HeronWater} from "../assets/heron_water.js";
import { HeronAnimation, HeronAnimationTrigger } from "../assets/heron_animation.js";

// Games may pass a predicate to exclude their dynamic bodies from the static collider.
function isBlocking(entity) { return !!entity.collision && !entity.collision.sensor; }

// Flatten a `{type, value}` property map to plain values.
function flattenProps(props) {
  const out = {};
  const plain=value=>Array.isArray(value) ? value.map(plain) : value && typeof value === "object" && "value" in value ? plain(value.value) : value;
  for (const [key, p] of Object.entries(props || {})) {
    out[key] = plain(p);
  }
  return out;
}

export async function loadLevel(url, {blocks = isBlocking, factories = {}} = {}) {
  const base = url.slice(0, url.lastIndexOf("/") + 1);
  const level = await (await fetch(url)).json();
  if(![1,2].includes(level.format))throw new Error(`Unsupported Heron Brush level format ${level.format}`);

  const texLoader = new THREE.TextureLoader();
  const textures = new Map();
  for (const t of level.textures) {
    if (!t.image) continue;
    const tex = await texLoader.loadAsync(base + t.image);
    tex.wrapS = tex.wrapT = THREE.RepeatWrapping;
    tex.colorSpace = THREE.SRGBColorSpace;
    tex.anisotropy = 4;
    tex.flipY = false;
    if (t.clamp) tex.wrapS = tex.wrapT = THREE.ClampToEdgeWrapping;
    textures.set(t.name, tex);
  }

  const gltf = new GLTFLoader();
  const root = new THREE.Group();
  root.name = level.name;
  const colliderParts = [];
  const entities = [];
  const animations = new Map(), triggers = [], waters=[];let waterTime=0;
  const ownedGeometry=new Set(), ownedMaterials=new Set(), ownedTextures=new Set(), cleanup=[];
  let disposed=false;

  const jobs = level.entities.map(async (entity, index) => {
    const t = entity.transform;
    const offset = new THREE.Vector3(...t.translation);
    const object = new THREE.Group();
    object.position.copy(offset);
    object.rotation.set(...t.rotation_yxz, "YXZ");
    object.scale.set(...t.scale);
    root.add(object);
    const properties = flattenProps(entity.properties);
    const water=properties._heron_water?new HeronWater(THREE,properties._heron_water,object):null;if(water)waters.push(water);
    if (blocks(entity)) {
      const local=localCollisionTriangles(entity);
      const part = {local,tris:new Float64Array(local.length), offset: new THREE.Vector3(), object, entity};
      colliderParts.push(part);
    }

    const record = {
      name: entity.name,
      classname: entity.classname,
      solid: entity.solid,
      position: offset,
      rotationYXZ: t.rotation_yxz,
      targets: entity.targets || [],
      tags: entity.tags || [],
      properties,
      object,
    };
    entities[index] = record;

    if (!entity.mesh || !entity.surfaces || entity.surfaces.length === 0) return;

    const data = await gltf.loadAsync(base + entity.mesh);
    const model = data.scene;
    model.traverse(object=>{if(object.isMesh){ownedGeometry.add(object.geometry);for(const material of Array.isArray(object.material)?object.material:[object.material]){ownedMaterials.add(material);for(const value of Object.values(material))if(value?.isTexture)ownedTextures.add(value);}}});

    // Each `surface` line puts a texture on one glTF primitive.
    const meshes = [];
    model.traverse((o) => o.isMesh && meshes.push(o));
    for (const s of entity.surfaces) {
      const m = meshes[s.primitive];
      if (!m) continue;
      const tex = textures.get(s.texture);
      const surface = level.textures.find(t => t.name === s.texture)?.surface;
      m.material = new THREE.MeshStandardMaterial({
        map: tex || null,
        color: tex ? 0xffffff : 0x8891a0,
        roughness: surface?.[0] ?? 0.85,
        metalness: surface?.[1] ?? 0,
        emissive: surface ? 0xffffff : 0,
        emissiveMap: surface ? tex : null,
        emissiveIntensity: surface?.[2] ?? 0,
      });
      if(water)water.material(m.material);
      ownedMaterials.add(m.material);
      m.castShadow = !(water?.data.surface_material);
      m.receiveShadow = true;
    }

    object.add(model);
  });

  await Promise.all(jobs);
  // All roots exist before project factories run, so forward references work.
  const byName=new Map(entities.map(entity=>[entity.name,entity]));
  for(const record of entities) {
    const factory=factories[record.properties.three_factory || record.classname];
    if(factory){const dispose=await factory(record,{root,entities,byName,colliderParts,level});if(typeof dispose==="function")cleanup.push(dispose);}
  }
  for(const record of entities) {
    if(record.properties._heron_animation)animations.set(record.name,new HeronAnimation(THREE,record.properties._heron_animation,record.object));
    if(record.properties._heron_trigger)triggers.push(new HeronAnimationTrigger(record.properties._heron_trigger,record.object));
  }
  root.updateMatrixWorld(true);
  for(const part of colliderParts)updateCollisionPart(part);
  return { level, root, entities, colliderParts, animations, triggers, waters,
    dispose() {
      if(disposed)return;disposed=true;
      for(const release of cleanup)release();
      root.removeFromParent();
      for(const geometry of ownedGeometry)geometry.dispose();
      for(const material of ownedMaterials)material.dispose();
      for(const texture of new Set([...textures.values(),...ownedTextures]))texture.dispose();
      root.clear();
    },
    // `height` reaches from the feet to the top of the player, so a trigger volume
    // that starts above the floor still catches a player standing in it.
    advance(dt, feet, height = 0) {
      waterTime+=dt;for(const water of waters)water.animate(waterTime);
      if(disposed)throw new Error("Cannot advance a disposed level");
      const body = [0, 0.5, 1].map(f => ['player', feet.clone().setY(feet.y + height * f)]);
      for (const trigger of triggers) trigger.update(dt, body, animations);
      for (const animation of animations.values()) animation.update(dt);
      root.updateMatrixWorld(true);
      for (const part of colliderParts) {
        updateCollisionPart(part);
      }
    }
  };
}

// Feet position and Y rotation of the first info_player_start, or a sane
// default.
export function findSpawn(entities) {
  for (const e of entities) {
    if (e.classname === "info_player_start") {
      return { feet: e.position.clone(), yaw: e.rotationYXZ[1] };
    }
  }
  return { feet: new THREE.Vector3(0, 3, 0), yaw: 0 };
}
