// heron_babylon.js: a Heron Brush level in a Babylon.js 9 scene, with Havok physics.
//
//   const havok = await HavokPhysics();
//   scene.enablePhysics(new BABYLON.Vector3(0, -9.81, 0), new BABYLON.HavokPlugin(true, havok));
//   const level = await loadLevel(BABYLON, scene, 'arena.json');
//   scene.onBeforeRenderObservable.add(() =>
//     level.update(engine.getDeltaTime() / 1000, [['player', feet], ['player', head]]));
//
// Pass the BABYLON namespace: the UMD global, or `import * as BABYLON from
// '@babylonjs/core'` with '@babylonjs/loaders/glTF' imported. loadLevel switches the
// scene to Babylon's right-handed mode, which is the level's frame (Y up, -Z forward,
// metres), so nothing is converted. Without a physics engine on the scene the level
// loads with no bodies.
//
// Every entity becomes a TransformNode at its transform under `level.root`, with its
// GLB under it and its record in `node.metadata.heron`. Collision becomes a Havok body
// on that node: static, animated for an entity with keyframe animation, a trigger
// shape for a sensor. Bodies are placed when the level loads; move `options.parent`
// before calling loadLevel rather than the root after it.

// Each scene setting has an ownership stack so maps can be unloaded in either order.
const sceneValues = new WeakMap();
function setSceneValue(target, key, value, owned) {
  let fields = sceneValues.get(target);
  if (!fields) sceneValues.set(target, fields = new Map());
  let stack = fields.get(key);
  if (!stack) fields.set(key, stack = []);
  const entry = {target, key, before: target[key], value};
  target[key] = value;
  stack.push(entry); owned.push(entry);
}
function restoreSceneValues(owned) {
  for (const entry of owned.splice(0).reverse()) {
    const {target, key, before, value} = entry;
    const fields = sceneValues.get(target), stack = fields.get(key), index = stack.indexOf(entry);
    if (index + 1 < stack.length) stack[index + 1].before = before;
    else if (target[key] === value) target[key] = before;
    stack.splice(index, 1);
    if (!stack.length) fields.delete(key);
    if (!fields.size) sceneValues.delete(target);
  }
}
function cleanupAll(tasks) {
  const errors = [];
  for (const task of tasks) try { task(); } catch (error) { errors.push(error); }
  if (errors.length) throw new AggregateError(errors, 'Could not completely dispose the Heron Brush level');
}
function validateLevel(level) {
  if (level?.format !== 2 || !Array.isArray(level.entities)) throw new Error('Not a Heron Brush level (format 2)');
  const names = new Set();
  for (const entity of level.entities) {
    if (!entity || typeof entity.name !== 'string' || !entity.name || names.has(entity.name))
      throw new Error('Missing or duplicate entity name: ' + entity?.name);
    names.add(entity.name);
    const t = entity.transform;
    if (!t || ![t.translation, t.rotation_yxz, t.scale].every(v => Array.isArray(v) && v.length === 3 && v.every(Number.isFinite)))
      throw new Error('Invalid entity transform: ' + entity.name);
  }
}

const RADIANS = Math.PI / 180;

// The rotation of Euler angles applied Y, then X, then Z (three.js's 'YXZ' order),
// which is how the level's `rotation_yxz` and the animation channels turn.
function rotationYXZ(B, x, y, z) {
  return B.Quaternion.RotationAxis(B.Axis.Y, y)
    .multiply(B.Quaternion.RotationAxis(B.Axis.X, x))
    .multiply(B.Quaternion.RotationAxis(B.Axis.Z, z));
}

function worldToLocal(B, node, point) {
  node.computeWorldMatrix(true);
  return B.Vector3.TransformCoordinates(point, node.getWorldMatrix().clone().invert());
}

// A property's typed value ({type, value}, arrays of them) as plain JSON.
export function plainValue(property) {
  if (!property || typeof property !== 'object' || !('type' in property)) return property;
  return property.type === 'array' ? property.value.map(plainValue) : property.value;
}

// Plays the validated _heron_animation payload on a TransformNode, which must keep
// the entity's rest transform. A body on the same node follows it.
export class HeronAnimation {
  constructor(B, json, node) {
    this.B = B;
    this.data = typeof json === 'string' ? JSON.parse(json) : json;
    const d = this.data;
    if (d.version !== 1 || !Number.isFinite(d.scale) || d.scale <= 0 ||
        !d.clips?.length || d.clips.some(c => c.channels.length !== 6 || !Number.isFinite(c.duration) || c.duration <= 0)) {
      throw new Error('Invalid animation payload');
    }
    this.clip = d.clips.find(c => c.name === d.selected);
    if (!this.clip) throw new Error('Missing selected clip');
    this.node = node;
    node.rotationQuaternion ??= B.Quaternion.FromEulerVector(node.rotation);
    this.rest = {position: node.position.clone(), rotation: node.rotationQuaternion.clone(), scale: node.scaling.clone()};
    this.time = 0;
    this.playing = d.autoplay;
    this.posed = this.playing;
  }
  play(name) {
    const clip = this.data.clips.find(c => c.name === name);
    if (!clip) return false;
    this.clip = clip; this.time = 0; this.playing = this.posed = true;
    return true;
  }
  // A button press: toggles between `open` and `close` clips when there are both,
  // reversing from where the current one is, or restarts the selected clip.
  activate() {
    if (['open', 'close'].every(n => this.data.clips.some(c => c.name === n))) {
      const phase = Math.min(this.time / this.clip.duration, 1);
      const reverse = this.playing && this.posed;
      const next = this.clip.name === 'open' && this.posed ? 'close' : 'open';
      this.play(next);
      if (reverse) this.time = (1 - phase) * this.clip.duration;
    } else this.play(this.data.selected);
  }
  pause() { this.playing = false; }
  resume() { this.playing = this.posed = true; }
  stop() { this.playing = this.posed = false; this.time = 0; }
  seek(seconds) { if (Number.isFinite(seconds)) { this.time = Math.max(seconds, 0); this.posed = true; } }
  update(dt) {
    if (!Number.isFinite(dt) || dt < 0) throw new Error('Invalid animation timestep');
    const c = this.clip, B = this.B;
    if (this.playing) {
      this.time += dt;
      if (c.mode === 'once' && this.time >= c.duration) { this.time = c.duration; this.playing = false; }
      else if (c.mode !== 'once') this.time %= c.duration * (c.mode === 'pingpong' ? 2 : 1);
    }
    const o = this.node, r = this.rest;
    if (!this.posed) {
      o.position.copyFrom(r.position); o.rotationQuaternion.copyFrom(r.rotation); o.scaling.copyFrom(r.scale);
      return;
    }
    let time = c.mode === 'once' ? Math.min(this.time, c.duration) : this.time % (c.duration * (c.mode === 'pingpong' ? 2 : 1));
    if (time > c.duration) time = 2 * c.duration - time;
    // Channels are X, Y, Z, roll, pitch, yaw in map units and degrees; the map's
    // X, Y, Z are the level's Z, X, Y.
    const v = c.channels.map(channel => sample(channel.keys, time));
    const rotation = rotationYXZ(B, v[4] * RADIANS, v[5] * RADIANS, v[3] * RADIANS);
    const p = this.data.pivot, scale = this.data.scale;
    const pivot = new B.Vector3(p[1], p[2], p[0]).scaleInPlace(1 / scale);
    o.position.copyFrom(r.position.subtract(pivot).applyRotationQuaternion(rotation).addInPlace(pivot)
      .addInPlace(new B.Vector3(v[1], v[2], v[0]).scaleInPlace(1 / scale)));
    o.rotationQuaternion.copyFrom(rotation.multiply(r.rotation));
    o.scaling.copyFrom(r.scale);
  }
}

function sample(keys, time) {
  if (!keys.length) return 0;
  if (time <= keys[0].time) return keys[0].value;
  let index = 0;
  while (index + 1 < keys.length && keys[index + 1].time <= time) index++;
  const a = keys[index], b = keys[index + 1];
  if (!b || a.interpolation === 'hold') return a.value;
  const duration = b.time - a.time, f = (time - a.time) / duration, u = 1 - f;
  if (a.interpolation === 'bezier') return u*u*u*a.value + 3*u*u*f*(a.value+a.outgoing*duration/3) +
    3*u*f*f*(b.value-b.incoming*duration/3) + f*f*f*b.value;
  return a.value + (b.value - a.value) * f;
}

// The _heron_trigger payload of a button or trigger: fires its targets' animations
// when an actor enters its convex volumes, or when activate() is called.
export class HeronAnimationTrigger {
  constructor(B, json, node) {
    this.B = B;
    this.data = typeof json === 'string' ? JSON.parse(json) : json;
    this.node = node; this.inside = new Set(); this.wait = 0; this.fired = false; this.pending = false;
  }
  activate() { this.pending = true; }
  // Whether a world point is inside the activation volume.
  contains(point) {
    const p = worldToLocal(this.B, this.node, point), o = this.data.origin;
    return this.data.volumes.some(planes => planes.length && planes.every(v =>
      v[0]*(p.x+o[0])+v[1]*(p.y+o[1])+v[2]*(p.z+o[2]) <= v[3]+1e-5));
  }
  // actors: [id, point] pairs, as a Map or an array, points in world space. One id may
  // give several points (a player's feet, middle and head); it is inside when any is.
  // Returns whether it fired.
  update(dt, actors, animations) {
    this.wait = Math.max(0, this.wait - dt);
    const inside = new Set(), d = this.data;
    for (const [id, point] of actors) if (this.contains(point)) inside.add(id);
    const entered = [...inside].some(id => !this.inside.has(id));
    this.inside = inside;
    const requested = this.pending || entered; this.pending = false;
    if (!requested || this.wait > 0 || (d.once && this.fired)) return false;
    this.wait = d.wait; this.fired = true;
    for (const name of d.targets) {
      const animation = animations.get(name);
      if (!animation) continue;
      switch (d.action) {
        case 'play': animation.play(d.clip || animation.data.selected); break;
        case 'open': case 'close': animation.play(d.action); break;
        case 'pause': animation.pause(); break;
        case 'resume': animation.resume(); break;
        case 'stop': animation.stop(); break;
        case '': case 'toggle': animation.activate(); break;
      }
    }
    return true;
  }
}

// The _heron_water payload: depth queries for a game's player, and the surface's
// see-through, waving material.
export class HeronWater {
  constructor(B, json, node) {
    this.B = B; this.data = typeof json === 'string' ? JSON.parse(json) : json; this.node = node; this.time = 0;
    if (this.data.version !== 1 || !Array.isArray(this.data.volumes)) throw new Error('Unsupported water payload');
  }
  // The water at a world point: {water, data, depth} with the depth under the surface
  // in metres, or null outside every volume.
  sample(point) {
    const p = worldToLocal(this.B, this.node, point);
    let depth = -Infinity;
    for (const volume of this.data.volumes) {
      if (volume.planes.length >= 4 && volume.planes.every(v => v.x*p.x+v.y*p.y+v.z*p.z <= v.d+1e-5)) {
        depth = Math.max(depth, volume.surface - p.y);
      }
    }
    if (depth < 0) return null;
    return {water: this, data: this.data, depth: depth * Math.abs(this.node.absoluteScaling.y)};
  }
  material(material) {
    const d = this.data, B = this.B;
    if (!d.surface_material) return;
    if (material.albedoColor) material.albedoColor = new B.Color3(...d.color);
    material.alpha = d.opacity;
    material.transparencyMode = B.Material.MATERIAL_ALPHABLEND;
    material.backFaceCulling = false;
    material.disableDepthWrite = true;
    if ('roughness' in material) { material.roughness = 0.14; material.metallic = 0.05; }
    const plugin = waterPlugin(B);
    if (plugin) material.heronWater = new plugin(material, this);
  }
  animate(time) { this.time = time; }
  // The deepest of the highest-priority waters at a point, as the other runtimes pick.
  static sample(waters, point) {
    let best = null;
    for (const water of waters) {
      const next = water.sample(point);
      if (next && (!best || next.data.priority > best.data.priority ||
        (next.data.priority === best.data.priority && next.depth > best.depth))) best = next;
    }
    return best;
  }
}

// The surface's waves as a material plugin: upward faces rise and fall in the vertex
// shader, as the other targets' water shaders move them.
let WaterPlugin = null;
function waterPlugin(B) {
  if (WaterPlugin || !B.MaterialPluginBase) return WaterPlugin;
  WaterPlugin = class extends B.MaterialPluginBase {
    constructor(material, water) {
      super(material, 'HeronWater', 200, {HERON_WATER: false});
      this.water = water;
      this._enable(true);
    }
    getClassName() { return 'HeronWaterPlugin'; }
    prepareDefines(defines) { defines.HERON_WATER = true; }
    getUniforms() {
      return {ubo: [{name: 'hbTime', size: 1, type: 'float'}, {name: 'hbHeight', size: 1, type: 'float'}, {name: 'hbSpeed', size: 1, type: 'float'}],
        vertex: 'uniform float hbTime;\nuniform float hbHeight;\nuniform float hbSpeed;\n'};
    }
    bindForSubMesh(ubo) {
      const d = this.water.data;
      ubo.updateFloat('hbTime', this.water.time); ubo.updateFloat('hbHeight', d.wave_height); ubo.updateFloat('hbSpeed', d.wave_speed);
    }
    getCustomCode(shaderType, shaderLanguage) {
      if (shaderType !== 'vertex' || shaderLanguage) return null;
      return {CUSTOM_VERTEX_UPDATE_POSITION: '#ifdef HERON_WATER\n#ifdef NORMAL\n' +
        'positionUpdated.y += smoothstep(0.5, 0.9, normalUpdated.y) * hbHeight * sin(positionUpdated.x * 1.7 + hbTime * hbSpeed) * cos(positionUpdated.z * 1.3 - hbTime * hbSpeed * 0.8);\n' +
        '#endif\n#endif\n'};
    }
  };
  return WaterPlugin;
}

// A mesh holding flat triangle or point data, scaled by `scale`, for Havok to read.
// Havok bodies take a node's position and rotation, not its scale, so the scale is
// baked into the shape.
function shapeMesh(B, scene, points, scale, triangles) {
  const positions = new Float32Array(points.length);
  for (let i = 0; i < points.length; i += 3) {
    positions[i] = points[i] * scale.x; positions[i + 1] = points[i + 1] * scale.y; positions[i + 2] = points[i + 2] * scale.z;
  }
  const mesh = new B.Mesh('heron-collision', scene);
  const data = new B.VertexData();
  data.positions = positions;
  if (triangles) {
    // Collision triangles wind clockwise (Godot's order); Havok wants them
    // counter-clockwise in a right-handed scene. A hull is a cloud of points.
    const indices = [];
    for (let i = 0; i < points.length / 3; i += 3) indices.push(i, i + 2, i + 1);
    data.indices = indices;
  }
  data.applyToMesh(mesh);
  return mesh;
}

function bodyShape(B, scene, collision, scale) {
  const shapes = [], helpers = [];
  if (collision.triangles?.length) {
    const mesh = shapeMesh(B, scene, collision.triangles, scale, true);
    helpers.push(mesh); shapes.push(new B.PhysicsShapeMesh(mesh, scene));
  }
  for (const hull of collision.hulls ?? []) {
    if (hull.length < 12) continue;
    const mesh = shapeMesh(B, scene, hull, scale, false);
    helpers.push(mesh); shapes.push(new B.PhysicsShapeConvexHull(mesh, scene));
  }
  for (const mesh of helpers) mesh.dispose();
  if (shapes.length < 2) return shapes[0] ?? null;
  const container = new B.PhysicsShapeContainer(scene);
  for (const shape of shapes) container.addChild(shape);
  return container;
}

// The level's sky (an equirectangular .hdr: its URL, or a data: URL), sun and fog.
async function applyEnvironment(B, scene, env, skyUrl, parent, made, owned) {
  if (!env) return made;
  if (skyUrl) {
    let texture;
    await new Promise((resolve, reject) => {
      texture = new B.HDRCubeTexture(skyUrl, scene, 256, false, true, false, true, resolve,
        (message, error) => reject(error ?? new Error(message ?? 'Could not load sky panorama')));
      made.push(texture);
      if (texture.isReady()) resolve();
    });
    texture.name = 'heron-sky';
    setSceneValue(scene, 'environmentTexture', texture, owned);
    const skybox = env.show_background === false ? null : scene.createDefaultSkybox(texture, true, 1000, 0, false);
    if (skybox) {
      skybox.name = 'heron-skybox';
      skybox.applyFog = false;
      skybox.infiniteDistance = true;
      made.push(skybox, skybox.material);
      const reflection = skybox.material.reflectionTexture;
      if (reflection && reflection !== texture) made.push(reflection);
    }
  }
  setSceneValue(scene, 'environmentIntensity', env.ambient_intensity, owned);
  const sun = new B.DirectionalLight('heron-sun', new B.Vector3(...env.to_sun).normalize().negate(), scene);
  sun.diffuse = new B.Color3(...env.sun_color);
  sun.specular = sun.diffuse.clone();
  // The editor's view and three.js light a diffuse surface by sun / pi; Babylon's
  // lights do not divide by pi.
  sun.intensity = env.sun_intensity / Math.PI;
  sun.parent = parent;
  made.push(sun);
  const processing = scene.imageProcessingConfiguration;
  setSceneValue(processing, 'toneMappingEnabled', true, owned);
  setSceneValue(processing, 'toneMappingType', B.ImageProcessingConfiguration.TONEMAPPING_ACES, owned);
  setSceneValue(processing, 'exposure', env.exposure, owned);
  if (env.fog) {
    setSceneValue(scene, 'fogMode', B.Scene.FOGMODE_EXP2, owned);
    setSceneValue(scene, 'fogDensity', env.fog.density, owned);
    setSceneValue(scene, 'fogColor', new B.Color3(...env.fog.color), owned);
  }
  return made;
}

// A native light for an entity's `light` block, shining down the node's local -Z.
function makeLight(B, scene, record) {
  const l = record.entity.light, name = record.name;
  const forward = new B.Vector3(0, 0, -1);
  let light;
  if (l.kind === 'spot') light = new B.SpotLight(name, B.Vector3.Zero(), forward, 2 * l.angle * RADIANS, 1, scene);
  else if (l.kind === 'directional') light = new B.DirectionalLight(name, forward, scene);
  else light = new B.PointLight(name, B.Vector3.Zero(), scene);
  light.diffuse = new B.Color3(...l.color);
  light.specular = light.diffuse.clone();
  light.intensity = l.energy;
  if (l.kind !== 'directional') { light.range = l.range; light.falloffType = B.Light.FALLOFF_GLTF; }
  light.parent = record.node;
  return light;
}

// Loads a level. `source` is the level JSON's URL, or {level, meshes, sky}: the parsed
// level, its GLBs by the level's mesh paths (bytes, or URLs), and the sky panorama's URL.
// Options:
//   parent     a node to put the level under (place it first, see above)
//   factories  {classname or babylon_factory property: (record, level) => cleanup?},
//              called once every entity exists and its targets are resolved
//   lights     false to leave out the entities' native lights
//   sky        false to leave out the sky, sun, fog and tone mapping
export async function loadLevel(B, scene, source, options = {}) {
  let level = source.level, base = '', meshSources = source.meshes ?? {};
  if (typeof source === 'string') {
    const response = await fetch(source);
    if (!response.ok) throw new Error(`Could not load level (${response.status}): ${source}`);
    level = await response.json();
    base = new URL('.', new URL(source, location.href)).href;
  }
  validateLevel(level);
  if (!scene.useRightHandedSystem && (scene.meshes.length || scene.transformNodes.length || scene.cameras.length || scene.lights.length))
    throw new Error('Heron Brush requires a right-handed scene; set useRightHandedSystem before creating scene objects.');
  scene.useRightHandedSystem = true;
  const physics = scene.getPhysicsEngine?.() ?? null;
  const root = new B.TransformNode(level.name, scene);
  if (options.parent) root.parent = options.parent;
  root.computeWorldMatrix(true);
  const containers = new Map(), loadedContainers = [], instances = [], materials = new Set(), textures = new Set();
  const container = path => {
    if (!containers.has(path)) {
      const mesh = meshSources[path] ?? base + path;
      containers.set(path, B.LoadAssetContainerAsync(mesh, scene, {pluginExtension: '.glb'}).then(asset => { loadedContainers.push(asset); return asset; }));
    }
    return containers.get(path);
  };
  const out = {
    data: level, root, entities: [], byName: new Map(), animations: new Map(), triggers: [], waters: [],
    lights: [], shadows: [], bodies: [], environment: [], sceneValues: [], cleanups: [], time: 0,
  };
  let disposed = false;
  out.dispose = () => {
    if (disposed) return;
    disposed = true;
    cleanupAll([
      ...out.cleanups.splice(0).reverse(),
      () => restoreSceneValues(out.sceneValues),
      ...out.bodies.flatMap(body => { const shape = body.shape; return [() => body.dispose(), () => shape?.dispose()]; }),
      ...out.shadows.map(shadow => () => shadow.dispose()),
      ...instances.map(models => () => models.dispose()),
      () => root.dispose(false, false),
      ...[...materials].map(material => () => material.dispose(false, false)),
      ...[...textures].map(texture => () => texture.dispose()),
      ...out.environment.map(thing => () => thing.dispose()),
      ...loadedContainers.map(asset => () => asset.dispose()),
    ]);
  };
  try {
  for (const entity of level.entities) {
    const t = entity.transform;
    const node = new B.TransformNode(entity.name, scene);
    node.parent = root;
    node.position = B.Vector3.FromArray(t.translation);
    node.rotationQuaternion = rotationYXZ(B, ...t.rotation_yxz);
    node.scaling = B.Vector3.FromArray(t.scale);
    const properties = entity.properties ?? {};
    const record = {
      entity, node, name: entity.name, classname: entity.classname, properties,
      values: Object.fromEntries(Object.entries(properties).map(([key, value]) => [key, plainValue(value)])),
      targetname: plainValue(properties.targetname) ?? null, targets: [], meshes: [],
      body: null, light: null, animation: null, trigger: null, water: null,
    };
    node.metadata = {heron: record};
    out.entities.push(record); out.byName.set(entity.name, record);
    const waterData = properties._heron_water?.value;
    if (waterData) { record.water = new HeronWater(B, waterData, node); out.waters.push(record.water); }
    const animationData = properties._heron_animation?.value;
    if (animationData) { record.animation = new HeronAnimation(B, animationData, node); out.animations.set(entity.name, record.animation); }
    const triggerData = properties._heron_trigger?.value;
    if (triggerData) { record.trigger = new HeronAnimationTrigger(B, triggerData, node); record.trigger.entityName = entity.name; out.triggers.push(record.trigger); }
    if (entity.mesh) {
      // Copies of one GLB share its geometry; water gets materials of its own.
      const template = await container(entity.mesh);
      const models = template.instantiateModelsToScene(
        name => `${entity.name}/${name}`, !!record.water, {doNotInstantiate: true});
      instances.push(models);
      for (const model of models.rootNodes) model.parent = node;
      record.meshes = node.getChildMeshes(false);
      if (record.water) {
        const waterMaterials = new Set(record.meshes.flatMap(mesh => mesh.material?.subMaterials ?? [mesh.material]).filter(Boolean));
        for (const mesh of record.meshes) if (mesh.material) materials.add(mesh.material);
        const sharedTextures = new Set([...template.textures, ...template.materials.flatMap(material => material.getActiveTextures())]);
        for (const material of waterMaterials) {
          materials.add(material);
          for (const texture of material.getActiveTextures()) if (!sharedTextures.has(texture)) textures.add(texture);
          record.water.material(material);
        }
      }
    }
    if (entity.light && options.lights !== false) {
      record.light = makeLight(B, scene, record);
      out.lights.push(record.light);
    }
    const collision = entity.collision;
    if (physics && collision) {
      node.computeWorldMatrix(true);
      const shape = bodyShape(B, scene, collision, node.absoluteScaling);
      if (shape) {
        shape.isTrigger = !!collision.sensor;
        const motion = record.animation ? B.PhysicsMotionType.ANIMATED : B.PhysicsMotionType.STATIC;
        const body = new B.PhysicsBody(node, motion, false, scene);
        body.shape = shape;
        // An animated body follows its node every step; Havok's character controller
        // reads its motion between frames and carries a player standing on it.
        if (record.animation) body.disablePreStep = false;
        record.body = body;
        out.bodies.push(body);
      }
    }
  }
  for (const record of out.entities) {
    record.targets = (record.entity.targets ?? []).map(name => out.byName.get(name)).filter(Boolean);
  }
  if (options.sky !== false) {
    const env = level.environment;
    const sky = source.sky ?? (env?.panorama ? base + env.panorama : null);
    await applyEnvironment(B, scene, env, sky, root, out.environment, out.sceneValues);
    installLocalLighting(B, scene, out, env?.local_lighting);
    installDisplay(B, scene, env?.display, out.cleanups, out.sceneValues);
  }
  out.shadows = makeShadows(B, scene, out, options.sky !== false && level.environment?.shadows !== false);
  for (const record of out.entities) {
    const factory = options.factories?.[record.values.babylon_factory] ?? options.factories?.[record.classname];
    const cleanup = factory?.(record, out);
    if (typeof cleanup === 'function') out.cleanups.push(cleanup);
  }
  // Advances the animations and triggers by dt seconds. `actors` are [id, point] pairs
  // in world space; give a standing player several points (feet, middle, head).
  out.update = (dt, actors = []) => {
    out.time += dt;
    for (const trigger of out.triggers) trigger.update(dt, actors, out.animations);
    for (const animation of out.animations.values()) animation.update(dt);
    for (const water of out.waters) water.animate(out.time);
  };
  // The water at a world point, the way HeronWater.sample picks among volumes.
  out.water = point => HeronWater.sample(out.waters, point);
  } catch (error) {
    try { out.dispose(); }
    catch (cleanupError) { throw new AggregateError([error, cleanupError], 'Level load failed and cleanup reported errors', {cause: error}); }
    throw error;
  }
  return out;
}

// Shadow maps belong to the loaded level. Only visible geometry casts, including
// moving doors/platforms; clip and trigger helpers are never shadow casters.
function makeShadows(B, scene, level, sunShadows) {
  if(level.data.environment?.shadows === false) return [];
  const meshes = level.entities.flatMap(record => record.meshes).filter(mesh => mesh.getTotalVertices() > 0);
  for (const mesh of meshes) {
    mesh.receiveShadows = true;
    if (mesh.material && 'maxSimultaneousLights' in mesh.material) mesh.material.maxSimultaneousLights = 8;
  }
  const sources = level.entities.filter(r => r.light && r.entity.light.shadows).map(r => r.light);
  if (sunShadows) sources.push(...level.environment.filter(thing => thing instanceof B.DirectionalLight));
  return sources.map(light => {
    const shadow = new B.ShadowGenerator(light instanceof B.DirectionalLight ? 2048 : 512, light);
    shadow.bias = light instanceof B.DirectionalLight ? 0.003 : 0.001; shadow.normalBias = 0.04;
    if (light instanceof B.PointLight) shadow.usePoissonSampling = true;
    else shadow.usePercentageCloserFiltering = true;
    if (light instanceof B.DirectionalLight) light.autoCalcShadowZBounds = true;
    shadow.transparencyShadow = true;
    for (const mesh of meshes) shadow.addShadowCaster(mesh, false);
    return shadow;
  });
}


function decodeBase64(text) {
  return typeof Uint8Array.fromBase64 === 'function' ? Uint8Array.fromBase64(text) : Uint8Array.from(atob(text), c => c.charCodeAt(0));
}

// A scene-linear postprocess uses the editor's white balance, contrast, saturation
// and all four display transforms. It also grades the sky, water and custom meshes.
function installDisplay(B, scene, display, cleanups, owned) {
  if (!display) return;
  const processing = scene.imageProcessingConfiguration;
  setSceneValue(processing, 'applyByPostProcess', true, owned);
  setSceneValue(processing, 'skipFinalColorClamp', true, owned);
  setSceneValue(processing, 'toneMappingEnabled', false, owned);
  setSceneValue(processing, 'exposure', 1, owned);
  let lut = null;
  if (display.tony_lut) {
    const bytes = decodeBase64(display.tony_lut), source = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const pixels = new Uint8Array(384 * 288 * 4);
    for (let z=0; z<48; z++) for (let y=0; y<48; y++) for (let x=0; x<48; x++) {
      const packed=source.getUint32(((z*48+y)*48+x)*4,true), exponent=(packed>>>27)-24;
      const rgb=[packed&511,(packed>>>9)&511,(packed>>>18)&511].map(v=>v*Math.pow(2,exponent));
      const peak=Math.max(...rgb), e=peak>1e-32 ? Math.floor(Math.log2(peak))+1 : -128;
      const k=Math.pow(2,8-e), offset=(((Math.floor(z/8)*48+y)*384)+(z%8)*48+x)*4;
      for(let c=0;c<3;c++) pixels[offset+c]=Math.min(255,Math.floor(rgb[c]*k));pixels[offset+3]=e+128;
    }
    lut = new B.RawTexture(pixels,384,288,B.Engine.TEXTUREFORMAT_RGBA,scene,false,false,B.Texture.NEAREST_SAMPLINGMODE);
    lut.wrapU=lut.wrapV=B.Texture.CLAMP_ADDRESSMODE;
  }
  const shader = 'heronDisplay';
  B.Effect.ShadersStore[shader+'PixelShader'] = `
precision highp float;
varying vec2 vUV;
uniform sampler2D textureSampler;
uniform sampler2D hbTony;
uniform vec3 hbBalance;
uniform vec4 hbDisplay;
uniform float hbMapper;
vec3 hbDecode(vec4 c) {return c.rgb*255.0*exp2(c.a*255.0-136.0);}
vec3 hbLut(vec3 p) {p=clamp(p,0.0,1.0)*47.0;vec3 lo=floor(p),f=fract(p),sum=vec3(0.0);
 for(int z=0;z<2;z++) for(int y=0;y<2;y++) for(int x=0;x<2;x++) {vec3 q=min(lo+vec3(float(x),float(y),float(z)),47.0);vec3 w=mix(1.0-f,f,vec3(float(x),float(y),float(z)));vec2 uv=vec2(mod(q.z,8.0)*48.0+q.x,floor(q.z/8.0)*48.0+q.y);sum+=hbDecode(texture2D(hbTony,(uv+0.5)/vec2(384.0,288.0)))*w.x*w.y*w.z;}return sum;}
vec3 hbAgx(vec3 c) {
 mat3 inset=mat3(0.8424790623,0.0423282423,0.0423756549,0.0784336,0.8784686365,0.0784336,0.0792237451,0.0791661275,0.8791429738);
 mat3 outset=mat3(1.1968790051,-0.0528968518,-0.0529716355,-0.0980208811,1.1519031299,-0.0980434501,-0.0990297441,-0.0989611768,1.1510736726);
 vec3 x=clamp((log2(max(inset*max(c,vec3(1e-10)),vec3(1e-10)))+12.47393)/16.5,0.0,1.0);vec3 x2=x*x,x4=x2*x2;
 x=15.5*x4*x2-40.14*x4*x+31.96*x4-6.868*x2*x+0.4298*x2+0.1191*x-0.00232;
 x=pow(max(x,0.0),vec3(1.2));float l=dot(x,vec3(0.2126,0.7152,0.0722));x=l+1.1*(x-l);return pow(max(outset*x,0.0),vec3(2.2));}
vec3 hbNeutral(vec3 c) {float x=min(c.r,min(c.g,c.b));c-=x<0.08?x-6.25*x*x:0.04;float peak=max(c.r,max(c.g,c.b));if(peak<0.76)return c;float np=1.0-0.24*0.24/(peak+0.24-0.76);c*=np/peak;float g=1.0-1.0/(0.15*(peak-np)+1.0);return mix(c,vec3(np),g);}
vec3 hbSrgb(vec3 c) {c=max(c,0.0);return mix(1.055*pow(c,vec3(1.0/2.4))-0.055,c*12.92,step(c,vec3(0.0031308)));}
void main() {
 vec4 color=texture2D(textureSampler,vUV);vec3 c=max(color.rgb,0.0)*hbDisplay.x*hbBalance;
 if(abs(hbDisplay.y-1.0)>0.0001)c=0.18*pow(max(c/0.18,0.0),vec3(hbDisplay.y));float grey=dot(c,vec3(0.2126,0.7152,0.0722));c=max(mix(vec3(grey),c,hbDisplay.z),0.0);
 if(hbMapper<0.5)c=clamp((c*(2.51*c+0.03))/(c*(2.43*c+0.59)+0.14),0.0,1.0);
 else if(hbMapper<1.5)c=hbAgx(c*1.8);else if(hbMapper<2.5)c=hbNeutral(c);else c=hbLut(c/(c+1.0));
 gl_FragColor=vec4(hbSrgb(c),color.a);
}`;
  const posts=[], attached=new WeakSet();
  const attach=camera=>{
    if(attached.has(camera)) return;
    attached.add(camera);
    const type=scene.getEngine().getCaps().textureHalfFloatRender ? B.Engine.TEXTURETYPE_HALF_FLOAT : B.Engine.TEXTURETYPE_UNSIGNED_BYTE;
    const post=new B.PostProcess('heron-display',shader,['hbBalance','hbDisplay','hbMapper'],['hbTony'],1,camera,B.Texture.BILINEAR_SAMPLINGMODE,scene.getEngine(),false,undefined,type);
    post.onApply=effect=>{
      effect.setFloat3('hbBalance',...display.white_balance);
      effect.setFloat4('hbDisplay',display.exposure,display.contrast,display.saturation,0);
      effect.setFloat('hbMapper',Math.max(0,['aces','agx','neutral','tony'].indexOf(display.tone_map)));
      if(lut) effect.setTexture('hbTony',lut);
    };
    posts.push([camera,post]);
  };
  for(const camera of scene.cameras) attach(camera);
  // Babylon announces new cameras inside their base constructor. Attach after
  // construction, before the scene prepares the active camera's render target.
  const observer=scene.onBeforeRenderObservable.add(()=>{for(const camera of scene.cameras)attach(camera);});
  cleanups.push(()=>{scene.onBeforeRenderObservable.remove(observer);for(const [camera,post] of posts)post.dispose(camera);lut?.dispose();});
}


// A single RGBE atlas keeps eight filtered probes within WebGL's sampler budget.
// SH irradiance and box-projected reflections are blended per fragment, not per mesh.
function installLocalLighting(B, scene, level, data) {
  if (!data?.probes?.length) return;
  if (data.version !== 1 || data.probes.length > 8 || data.width !== 384 || data.height !== data.probes.length*127) throw new Error('Invalid local lighting capture');
  const pixels=decodeBase64(data.atlas);
  if(pixels.length !== data.width*data.height*4) throw new Error('Invalid local lighting pixels');
  const atlas=new B.RawTexture(pixels,data.width,data.height,B.Engine.TEXTUREFORMAT_RGBA,scene,false,false,B.Texture.NEAREST_SAMPLINGMODE);
  atlas.name='heron-local-lighting';atlas.wrapU=atlas.wrapV=B.Texture.CLAMP_ADDRESSMODE;
  level.environment.push(atlas);
  const uniforms=[{name:'hbWorldToLevel',size:16,type:'mat4'},{name:'hbNormalToLevel',size:16,type:'mat4'}];
  let declarations='uniform mat4 hbWorldToLevel;\nuniform mat4 hbNormalToLevel;\nuniform sampler2D hbProbeAtlas;\n';
  let contributions='';
  data.probes.forEach((p,i)=>{
    for(const key of ['Pos','Half','Flags']) {uniforms.push({name:`hb${key}${i}`,size:4,type:'vec4'});declarations+=`uniform vec4 hb${key}${i};\n`;}
    for(let k=0;k<9;k++) {uniforms.push({name:`hbSH${i}_${k}`,size:3,type:'vec3'});declarations+=`uniform vec3 hbSH${i}_${k};\n`;}
    contributions+=`{
 float w=hbWeight(hbP,hbPos${i}.xyz,hbHalf${i}.xyz,hbHalf${i}.w);
 if(w>0.0) {
 vec3 n=hbN;vec3 e=hbSH${i}_0*0.282095+(2.0/3.0)*0.488603*(hbSH${i}_1*n.y+hbSH${i}_2*n.z+hbSH${i}_3*n.x);
 e+=0.25*(1.092548*(hbSH${i}_4*n.x*n.y+hbSH${i}_5*n.y*n.z+hbSH${i}_7*n.x*n.z)+0.315392*hbSH${i}_6*(3.0*n.z*n.z-1.0)+0.546274*hbSH${i}_8*(n.x*n.x-n.y*n.y));
 hbD+=max(e,0.0)*w*hbFlags${i}.y*hbPos${i}.w;hbDW+=w*hbFlags${i}.y;
 hbS+=hbSpecular(hbProject(hbP,hbR,hbPos${i}.xyz,hbHalf${i}.xyz),roughness,hbFlags${i}.z,${i}.0)*w*hbFlags${i}.x*hbPos${i}.w;hbSW+=w*hbFlags${i}.x;
 #ifdef CLEARCOAT
 hbC+=hbSpecular(hbProject(hbP,hbCR,hbPos${i}.xyz,hbHalf${i}.xyz),vClearCoatParams.y,hbFlags${i}.z,${i}.0)*w*hbFlags${i}.x*hbPos${i}.w;
 #endif
 }
}\n`;
  });
  const functions=`
vec3 hbRgbe(vec4 c){return c.rgb*255.0*exp2(c.a*255.0-136.0);}
float hbWeight(vec3 p,vec3 center,vec3 halfSize,float blend){vec3 q=abs(p-center)/halfSize;return 1.0-smoothstep(1.0-blend,1.0,max(q.x,max(q.y,q.z)));}
vec3 hbProject(vec3 p,vec3 d,vec3 center,vec3 halfSize){vec3 safe=mix(vec3(0.000001),d,step(vec3(0.000001),abs(d)));vec3 t=max((center-halfSize-p)/safe,(center+halfSize-p)/safe);return p+d*max(min(t.x,min(t.y,t.z)),0.0)-center;}
vec3 hbCube(vec3 d,float lod,float probe){vec3 a=abs(d);vec2 uv;float face;
 if(a.x>=a.y&&a.x>=a.z){face=d.x>=0.0?0.0:1.0;uv=vec2(d.x>=0.0?-d.z:d.z,-d.y)/max(a.x,1e-8);}
 else if(a.y>=a.z){face=d.y>=0.0?2.0:3.0;uv=vec2(d.x,d.y>=0.0?d.z:-d.z)/max(a.y,1e-8);}
 else{face=d.z>=0.0?4.0:5.0;uv=vec2(d.z>=0.0?d.x:-d.x,-d.y)/max(a.z,1e-8);}
 float size=64.0/exp2(lod);vec2 p=clamp((uv*0.5+0.5)*size-0.5,0.0,size-1.0),lo=floor(p),f=fract(p),hi=min(lo+1.0,size-1.0);
 vec2 offset=vec2(face*size,probe*127.0+128.0-2.0*size);vec2 dims=vec2(384.0,${data.height}.0);
 vec3 aa=hbRgbe(texture2D(hbProbeAtlas,(offset+lo+0.5)/dims));vec3 ab=hbRgbe(texture2D(hbProbeAtlas,(offset+vec2(hi.x,lo.y)+0.5)/dims));
 vec3 ba=hbRgbe(texture2D(hbProbeAtlas,(offset+vec2(lo.x,hi.y)+0.5)/dims));vec3 bb=hbRgbe(texture2D(hbProbeAtlas,(offset+hi+0.5)/dims));return mix(mix(aa,ab,f.x),mix(ba,bb,f.x),f.y);}
vec3 hbSpecular(vec3 d,float roughness,float maxLod,float probe){float lod=clamp(roughness,0.0,1.0)*maxLod;return mix(hbCube(d,floor(lod),probe),hbCube(d,min(floor(lod)+1.0,6.0),probe),fract(lod));}
`;
  const code=`
#if defined(REFLECTION) && !defined(UNLIT)
 vec3 hbP=(hbWorldToLevel*vec4(vPositionW,1.0)).xyz.zxy*${Number(data.scale).toFixed(8)};
 vec3 hbN=normalize((hbNormalToLevel*vec4(normalW,0.0)).xyz.zxy);
 vec3 hbR=normalize((hbWorldToLevel*vec4(reflect(-viewDirectionW,normalW),0.0)).xyz.zxy);
 #ifdef ANISOTROPIC
 hbR=normalize((hbWorldToLevel*vec4(reflect(-viewDirectionW,anisotropicOut.anisotropicNormal),0.0)).xyz.zxy);
 #endif
 vec3 hbD=vec3(0.0),hbS=vec3(0.0),hbC=vec3(0.0);float hbDW=0.0,hbSW=0.0;
 #ifdef CLEARCOAT
 vec3 hbCR=normalize((hbWorldToLevel*vec4(reflect(-viewDirectionW,clearcoatOut.clearCoatNormalW),0.0)).xyz.zxy);
 #endif
 ${contributions}
 vec3 hbDiffuseFactor=surfaceAlbedo*vLightingIntensity.z*aoOut.ambientOcclusionColor;
 #if defined(METALLICWORKFLOW) && !defined(LEGACY_SPECULAR_ENERGY_CONSERVATION)
 vec3 hbEnergy=vec3(baseSpecularEnvironmentReflectance);
 #if defined(ENVIRONMENTBRDF) && defined(MS_BRDF_ENERGY_CONSERVATION)
 hbEnergy*=baseSpecularEnergyConservationFactor;
 #endif
 hbDiffuseFactor*=clamp(1.0-hbEnergy,0.0,1.0);
 #endif
 #ifdef CLEARCOAT
 hbDiffuseFactor*=clearcoatOut.conservationFactor;
 #ifdef CLEARCOAT_TINT
 hbDiffuseFactor*=clearcoatOut.absorption;
 #endif
 #endif
 finalIrradiance=finalIrradiance*max(1.0-hbDW,0.0)+hbD/max(hbDW,1.0)*hbDiffuseFactor;
 vec3 hbSpecFactor=colorSpecularEnvironmentReflectance*vLightingIntensity.z;
 #if defined(ENVIRONMENTBRDF) && defined(MS_BRDF_ENERGY_CONSERVATION)
 hbSpecFactor*=coloredEnergyConservationFactor;
 #endif
 finalRadianceScaled=finalRadianceScaled*max(1.0-hbSW,0.0)+hbS/max(hbSW,1.0)*hbSpecFactor;
 #ifdef CLEARCOAT
 float hbF=mix(vClearCoatRefractionParams.x,1.0,pow(1.0-clamp(dot(clearcoatOut.clearCoatNormalW,viewDirectionW),0.0,1.0),5.0))*vClearCoatParams.x;
 clearcoatOut.finalClearCoatRadianceScaled=clearcoatOut.finalClearCoatRadianceScaled*max(1.0-hbSW,0.0)+hbC/max(hbSW,1.0)*hbF*vLightingIntensity.z;
 #endif
#endif
`;
  class LocalLighting extends B.MaterialPluginBase {
    constructor(material){super(material,'HeronLocalLighting',190,{HB_LOCAL_PROBES:data.probes.length,SKIPFINALCOLORCLAMP:true});this._enable(true);}
    getClassName(){return 'HeronLocalLighting';}
    prepareDefines(defines){defines.HB_LOCAL_PROBES=data.probes.length;defines.SKIPFINALCOLORCLAMP=true;}
    getSamplers(samplers){samplers.push('hbProbeAtlas');}
    getUniforms(){return {ubo:uniforms,fragment:declarations};}
    isReadyForSubMesh(){return atlas.isReady();}
    bindForSubMesh(ubo){
      level.root.computeWorldMatrix(true);
      ubo.updateMatrix('hbWorldToLevel',level.root.getWorldMatrix().clone().invert());
      ubo.updateMatrix('hbNormalToLevel',B.Matrix.Transpose(level.root.getWorldMatrix()));
      data.probes.forEach((p,i)=>{
        ubo.updateFloat4('hbPos'+i,...p.position,p.intensity);ubo.updateFloat4('hbHalf'+i,...p.half_size,p.blend);ubo.updateFloat4('hbFlags'+i,Number(p.reflections),Number(p.diffuse),p.max_lod,0);
        p.sh.forEach((c,k)=>ubo.updateFloat3(`hbSH${i}_${k}`,...c));
      });
      ubo.setTexture('hbProbeAtlas',atlas);
    }
    getCustomCode(type,language){if(type!=='fragment'||language)return null;return {CUSTOM_FRAGMENT_DEFINITIONS:functions,CUSTOM_FRAGMENT_BEFORE_FINALCOLORCOMPOSITION:code};}
  }
  const materials=new Set(level.entities.flatMap(r=>r.meshes).map(m=>m.material).filter(m=>m instanceof B.PBRMaterial));
  for(const material of materials) material.heronLocalLighting=new LocalLighting(material);
}
