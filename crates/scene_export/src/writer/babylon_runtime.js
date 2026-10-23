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
function applyEnvironment(B, scene, env, skyUrl, parent) {
  const made = [];
  if (!env) return made;
  if (skyUrl) {
    const texture = new B.HDRCubeTexture(skyUrl, scene, 256, false, true, false, true);
    texture.name = 'heron-sky';
    scene.environmentTexture = texture;
    const skybox = scene.createDefaultSkybox(texture, true, 1000, 0, false);
    skybox.name = 'heron-skybox';
    skybox.applyFog = false;
    skybox.infiniteDistance = true;
    made.push(texture, skybox, skybox.material);
  }
  scene.environmentIntensity = env.ambient_intensity;
  const sun = new B.DirectionalLight('heron-sun', new B.Vector3(...env.to_sun).normalize().negate(), scene);
  sun.diffuse = new B.Color3(...env.sun_color);
  sun.specular = sun.diffuse.clone();
  // The editor's view and three.js light a diffuse surface by sun / pi; Babylon's
  // lights do not divide by pi.
  sun.intensity = env.sun_intensity / Math.PI;
  sun.parent = parent;
  made.push(sun);
  const processing = scene.imageProcessingConfiguration;
  processing.toneMappingEnabled = true;
  processing.toneMappingType = B.ImageProcessingConfiguration.TONEMAPPING_ACES;
  processing.exposure = env.exposure;
  if (env.fog) {
    scene.fogMode = B.Scene.FOGMODE_EXP2;
    scene.fogDensity = env.fog.density;
    scene.fogColor = new B.Color3(...env.fog.color);
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
    level = await (await fetch(source)).json();
    base = new URL('.', new URL(source, location.href)).href;
  }
  if (level?.format !== 2 || !Array.isArray(level.entities)) throw new Error('Not a Heron Brush level (format 2)');
  scene.useRightHandedSystem = true;
  const physics = scene.getPhysicsEngine?.() ?? null;
  const root = new B.TransformNode(level.name, scene);
  if (options.parent) root.parent = options.parent;
  root.computeWorldMatrix(true);
  const containers = new Map();
  const container = path => {
    if (!containers.has(path)) {
      const mesh = meshSources[path] ?? base + path;
      containers.set(path, B.LoadAssetContainerAsync(mesh, scene, {pluginExtension: '.glb'}));
    }
    return containers.get(path);
  };
  const out = {
    data: level, root, entities: [], byName: new Map(), animations: new Map(), triggers: [], waters: [],
    lights: [], bodies: [], environment: [], cleanups: [], time: 0,
  };
  for (const entity of level.entities) {
    const t = entity.transform;
    if (![t.translation, t.rotation_yxz, t.scale].every(v => Array.isArray(v) && v.length === 3 && v.every(Number.isFinite))) {
      throw new Error('Invalid entity transform: ' + entity.name);
    }
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
      const models = (await container(entity.mesh)).instantiateModelsToScene(
        name => `${entity.name}/${name}`, !!record.water, {doNotInstantiate: true});
      for (const model of models.rootNodes) model.parent = node;
      record.meshes = node.getChildMeshes(false);
      if (record.water) {
        const materials = new Set(record.meshes.map(mesh => mesh.material).filter(Boolean));
        for (const material of materials) record.water.material(material);
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
    out.environment = applyEnvironment(B, scene, env, sky, root);
  }
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
  out.dispose = () => {
    for (const cleanup of out.cleanups.splice(0)) cleanup();
    for (const body of out.bodies) { body.shape?.dispose(); body.dispose(); }
    for (const thing of out.environment) thing.dispose();
    for (const light of out.lights) light.dispose();
    root.dispose(false, false);
  };
  return out;
}
