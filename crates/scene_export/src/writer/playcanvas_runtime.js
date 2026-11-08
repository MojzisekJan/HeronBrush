// heron_playcanvas.js: a Heron Brush level in a PlayCanvas 2 app, with ammo.js physics.
//
//   globalThis.Ammo = await Ammo();      // before the app, or call onLibraryLoaded after
//   const app = new pc.Application(canvas);
//   const level = await loadLevel(pc, app, 'arena.json');
//   app.on('update', dt => level.update(dt, [['player', feet], ['player', head]]));
//   app.start();
//
// Pass the pc namespace: the UMD global, or `import * as pc from 'playcanvas'`.
// PlayCanvas is right-handed with Y up and -Z forward, which is the level's frame
// (metres), so nothing is converted. Without Ammo the level loads with no bodies.
//
// Every entity becomes a pc.Entity at its transform under `level.root`, with its GLB
// under it and its record in `entity.heron`. Collision becomes rigidbody and collision
// components on that entity: a static body, a kinematic one for an entity with
// keyframe animation, and a collision component alone (a trigger volume, which fires
// triggerenter) for a sensor. Triangles are a mesh shape and hulls convex shapes.

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
// Ray masks: everything but trigger volumes, and only static and kinematic bodies.
export const SOLID_MASK = 0xffff ^ 16;
export const WORLD_MASK = 2 | 4;

// The rotation of Euler angles applied Y, then X, then Z (three.js's 'YXZ' order),
// which is how the level's `rotation_yxz` and the animation channels turn.
export function rotationYXZ(pc, x, y, z) {
  const q = new pc.Quat().setFromAxisAngle(pc.Vec3.UP, y / RADIANS);
  return q.mul(new pc.Quat().setFromAxisAngle(pc.Vec3.RIGHT, x / RADIANS))
    .mul(new pc.Quat().setFromAxisAngle(pc.Vec3.BACK, z / RADIANS));
}

// The rotation that turns unit vector `from` onto unit vector `to`.
function rotationBetween(pc, from, to) {
  const d = from.dot(to);
  if (d < -0.999999) {
    const axis = new pc.Vec3().cross(Math.abs(from.x) < 0.9 ? pc.Vec3.RIGHT : pc.Vec3.UP, from).normalize();
    return new pc.Quat().setFromAxisAngle(axis, 180);
  }
  const c = new pc.Vec3().cross(from, to);
  return new pc.Quat(c.x, c.y, c.z, 1 + d).normalize();
}

function worldToLocal(pc, entity, point) {
  return entity.getWorldTransform().clone().invert().transformPoint(point, new pc.Vec3());
}

// A property's typed value ({type, value}, arrays of them) as plain JSON.
export function plainValue(property) {
  if (!property || typeof property !== 'object' || !('type' in property)) return property;
  return property.type === 'array' ? property.value.map(plainValue) : property.value;
}

// Plays the validated _heron_animation payload on an entity, which must keep the
// entity's rest transform. A kinematic body on the same entity follows it.
export class HeronAnimation {
  constructor(pc, json, entity) {
    this.pc = pc;
    this.data = typeof json === 'string' ? JSON.parse(json) : json;
    const d = this.data;
    if (d.version !== 1 || !Number.isFinite(d.scale) || d.scale <= 0 ||
        !d.clips?.length || d.clips.some(c => c.channels.length !== 6 || !Number.isFinite(c.duration) || c.duration <= 0)) {
      throw new Error('Invalid animation payload');
    }
    this.clip = d.clips.find(c => c.name === d.selected);
    if (!this.clip) throw new Error('Missing selected clip');
    this.entity = entity;
    this.rest = {position: entity.getLocalPosition().clone(), rotation: entity.getLocalRotation().clone(), scale: entity.getLocalScale().clone()};
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
    const c = this.clip, pc = this.pc;
    if (this.playing) {
      this.time += dt;
      if (c.mode === 'once' && this.time >= c.duration) { this.time = c.duration; this.playing = false; }
      else if (c.mode !== 'once') this.time %= c.duration * (c.mode === 'pingpong' ? 2 : 1);
    }
    const e = this.entity, r = this.rest;
    if (!this.posed) {
      e.setLocalPosition(r.position); e.setLocalRotation(r.rotation); e.setLocalScale(r.scale);
      return;
    }
    let time = c.mode === 'once' ? Math.min(this.time, c.duration) : this.time % (c.duration * (c.mode === 'pingpong' ? 2 : 1));
    if (time > c.duration) time = 2 * c.duration - time;
    // Channels are X, Y, Z, roll, pitch, yaw in map units and degrees; the map's
    // X, Y, Z are the level's Z, X, Y.
    const v = c.channels.map(channel => sample(channel.keys, time));
    const rotation = rotationYXZ(pc, v[4] * RADIANS, v[5] * RADIANS, v[3] * RADIANS);
    const p = this.data.pivot, scale = this.data.scale;
    const pivot = new pc.Vec3(p[1], p[2], p[0]).mulScalar(1 / scale);
    const position = rotation.transformVector(r.position.clone().sub(pivot), new pc.Vec3())
      .add(pivot).add(new pc.Vec3(v[1], v[2], v[0]).mulScalar(1 / scale));
    e.setLocalPosition(position);
    e.setLocalRotation(rotation.clone().mul(r.rotation));
    e.setLocalScale(r.scale);
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
  constructor(pc, json, entity) {
    this.pc = pc;
    this.data = typeof json === 'string' ? JSON.parse(json) : json;
    this.entity = entity; this.inside = new Set(); this.wait = 0; this.fired = false; this.pending = false;
  }
  activate() { this.pending = true; }
  // Whether a world point (a pc.Vec3) is inside the activation volume.
  contains(point) {
    const p = worldToLocal(this.pc, this.entity, point), o = this.data.origin;
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
  constructor(pc, json, entity) {
    this.pc = pc; this.data = typeof json === 'string' ? JSON.parse(json) : json; this.entity = entity; this.time = 0;
    if (this.data.version !== 1 || !Array.isArray(this.data.volumes)) throw new Error('Unsupported water payload');
    this.materials = [];
  }
  // The water at a world point: {water, data, depth} with the depth under the surface
  // in metres, or null outside every volume.
  sample(point) {
    const p = worldToLocal(this.pc, this.entity, point);
    let depth = -Infinity;
    for (const volume of this.data.volumes) {
      if (volume.planes.length >= 4 && volume.planes.every(v => v.x*p.x+v.y*p.y+v.z*p.z <= v.d+1e-5)) {
        depth = Math.max(depth, volume.surface - p.y);
      }
    }
    if (depth < 0) return null;
    return {water: this, data: this.data, depth: depth * Math.abs(this.entity.getScale().y)};
  }
  // Makes a StandardMaterial the water's: see-through, glossy, and moving its vertices
  // up and down in the vertex shader as the other targets' water shaders do.
  material(material, device) {
    const d = this.data, pc = this.pc;
    if (!d.surface_material) return;
    material.diffuse = new pc.Color(...d.color);
    material.opacity = d.opacity;
    material.blendType = pc.BLEND_NORMAL;
    material.cull = pc.CULLFACE_NONE;
    material.depthWrite = false;
    material.useMetalness = true; material.metalness = 0.05;
    material.gloss = 0.14; material.glossInvert = true;
    const chunk = device && pc.ShaderChunks.get(device, pc.SHADERLANGUAGE_GLSL).get('transformVS');
    const line = 'vec4 posW = modelMatrix * vec4(localPos, 1.0);';
    if (chunk?.includes(line)) {
      // The surface is the volume's upward faces, so every vertex of it moves.
      material.getShaderChunks(pc.SHADERLANGUAGE_GLSL).set('transformVS',
        'uniform float hbTime;\nuniform float hbHeight;\nuniform float hbSpeed;\n' + chunk.replace(line, line +
        '\n\tposW.y += hbHeight * sin(posW.x * 1.7 + hbTime * hbSpeed) * cos(posW.z * 1.3 - hbTime * hbSpeed * 0.8);'));
      material.shaderChunksVersion = '2.23';
      material.heronWater = this;
    }
    this.materials.push(material);
    this.animate(this.time);
    material.update();
  }
  animate(time) {
    this.time = time;
    for (const material of this.materials) {
      material.setParameter('hbTime', time);
      material.setParameter('hbHeight', this.data.wave_height);
      material.setParameter('hbSpeed', this.data.wave_speed);
    }
  }
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

// A pc.Mesh of flat triangle data, or of a hull's points, for a collision component
// to read. Collision follows the entity's scale, so the points stay as written.
function shapeMesh(pc, device, points, triangles) {
  const mesh = new pc.Mesh(device);
  mesh.setPositions(new Float32Array(points));
  if (triangles) {
    const indices = new Uint32Array(points.length / 3);
    for (let i = 0; i < indices.length; i++) indices[i] = i;
    mesh.setIndices(indices);
  }
  mesh.update(triangles ? pc.PRIMITIVE_TRIANGLES : pc.PRIMITIVE_POINTS);
  return mesh;
}

// A mesh collision reads `render.meshes`; the engine keeps its Render class to
// itself, and nothing else of it is used.
function meshCollision(meshes, convexHull) {
  return {type: 'mesh', render: {meshes}, convexHull};
}

// Rigidbody and collision components for an entity's inline collision: a mesh shape
// for the triangles and a convex shape for each hull, under a compound when there
// are both. `body` is 'static', 'kinematic', or null for a trigger volume.
function addCollision(pc, app, entity, collision, body) {
  const device = app.graphicsDevice;
  const triangles = collision.triangles?.length ? [shapeMesh(pc, device, collision.triangles, true)] : [];
  const hulls = (collision.hulls ?? []).filter(hull => hull.length >= 12).map(hull => shapeMesh(pc, device, hull, false));
  if (!triangles.length && !hulls.length) return false;
  if (body) entity.addComponent('rigidbody', {type: body, friction: 0.5, restitution: 0});
  if (!triangles.length || !hulls.length) {
    entity.addComponent('collision', meshCollision(triangles.length ? triangles : hulls, !triangles.length));
    if (!triangles.length) exactHulls(entity.collision.shape);
    return true;
  }
  entity.addComponent('collision', {type: 'compound'});
  for (const [name, meshes, hull] of [['triangles', triangles, false], ['hulls', hulls, true]]) {
    const child = new pc.Entity('heron-collision-' + name);
    entity.addChild(child);
    child.addComponent('collision', meshCollision(meshes, hull));
    if (hull) exactHulls(child.collision.shape);
  }
  return true;
}

// PlayCanvas gives a convex hull a 1 cm margin, which Bullet adds outside the points:
// a 2 m box would collide as 2.02 m. The brush's own planes are the surface, so the
// margin goes. `shape` is the compound of hulls a convexHull mesh collision makes.
function exactHulls(shape) {
  if (!shape?.getNumChildShapes) return;
  for (let i = 0; i < shape.getNumChildShapes(); i++) shape.getChildShape(i).setMargin(0);
}

// The asset loader caches resources by URL. Keep one lease per level so
// unloading a copy cannot destroy resources another copy is still rendering.
const sharedAssets = new WeakMap();
async function acquireAsset(app, kind, source, load) {
  let kinds = sharedAssets.get(app);
  if (!kinds) sharedAssets.set(app, kinds = new Map());
  let assets = kinds.get(kind);
  if (!assets) kinds.set(kind, assets = new Map());
  const key = typeof source === 'string' ? new URL(source, location.href).href : source;
  let entry = assets.get(key);
  if (!entry) {
    entry = {references: 0, promise: Promise.resolve().then(load)};
    assets.set(key, entry);
  }
  entry.references++;
  let asset;
  try { asset = await entry.promise; }
  catch (error) {
    if (assets.get(key) === entry) assets.delete(key);
    throw error;
  }
  return {asset, release() {
    if (--entry.references !== 0) return;
    assets.delete(key);
    cleanupAll([() => app.assets.remove(asset), () => asset.unload()]);
  }};
}

// Loads a GLB (bytes, or a URL) as a container asset.
function loadContainer(pc, app, name, source) {
  let url = source, blob = null;
  if (typeof source !== 'string') url = blob = URL.createObjectURL(new Blob([source], {type: 'model/gltf-binary'}));
  const asset = new pc.Asset(name, 'container', {url, filename: name.split('/').pop()});
  const release = () => { if (blob) URL.revokeObjectURL(blob); app.assets.remove(asset); asset.unload(); };
  return new Promise((resolve, reject) => {
    asset.once('error', error => { release(); reject(new Error(`${name}: ${error}`)); });
    asset.ready(() => { if (blob) URL.revokeObjectURL(blob); resolve(asset); });
    try { app.assets.add(asset); app.assets.load(asset); }
    catch (error) { release(); reject(error); }
  });
}

// An equirectangular .hdr (a URL, or its bytes) as a texture asset.
function loadPanorama(pc, app, source) {
  let url = source, blob = null;
  if (typeof source !== 'string') url = blob = URL.createObjectURL(new Blob([source]));
  const asset = new pc.Asset('heron-sky', 'texture', {url, filename: 'sky.hdr'}, {type: 'rgbe', mipmaps: false});
  const release = () => { if (blob) URL.revokeObjectURL(blob); app.assets.remove(asset); asset.unload(); };
  return new Promise((resolve, reject) => {
    asset.once('error', error => { release(); reject(new Error(`sky: ${error}`)); });
    asset.ready(() => { if (blob) URL.revokeObjectURL(blob); resolve(asset); });
    try { app.assets.add(asset); app.assets.load(asset); }
    catch (error) { release(); reject(error); }
  });
}

// The panorama scaled by `factor`, as a new RGBE texture: PlayCanvas has no strength
// for image-based light of its own, and the map's ambient intensity dims only the
// light, not the sky.
function scaledPanorama(pc, texture, factor) {
  const source = texture.getSource();
  if (factor === 1 || !(source instanceof Uint8Array)) return null;
  const bytes = new Uint8Array(source.length);
  for (let i = 0; i < source.length; i += 4) {
    if (!source[i + 3]) continue;
    const unit = Math.pow(2, source[i + 3] - 136) * factor;
    const rgb = [source[i] * unit, source[i + 1] * unit, source[i + 2] * unit];
    const top = Math.max(...rgb);
    if (!(top > 1e-30)) continue;
    const exponent = Math.floor(Math.log2(top)) + 1, scale = 256 / Math.pow(2, exponent);
    for (let c = 0; c < 3; c++) bytes[i + c] = Math.min(255, Math.floor(rgb[c] * scale));
    bytes[i + 3] = exponent + 128;
  }
  return new pc.Texture(texture.device, {
    name: 'heron-sky-light', width: texture.width, height: texture.height, format: texture.format,
    type: pc.TEXTURETYPE_RGBE, projection: texture.projection, mipmaps: false, levels: [bytes],
  });
}

// The level's sky (an equirectangular .hdr), sun, fog and exposure.
async function applyEnvironment(pc, app, env, sky, root, made, owned, releases) {
  if (!env) return made;
  const scene = app.scene;
  if (sky) {
    const {asset, release} = await acquireAsset(app, 'texture', sky, () => loadPanorama(pc, app, sky));
    releases.push(release);
    made.assets.push(asset);
    const skybox = pc.EnvLighting.generateSkyboxCubemap(asset.resource);
    made.textures.push(skybox);
    let dimmed, lighting, atlas;
    try {
      dimmed = scaledPanorama(pc, asset.resource, env.ambient_intensity);
      lighting = pc.EnvLighting.generateLightingSource(dimmed ?? asset.resource);
      atlas = pc.EnvLighting.generateAtlas(lighting);
      made.textures.push(atlas);
    } finally { lighting?.destroy(); dimmed?.destroy(); }
    setSceneValue(scene, 'envAtlas', atlas, owned);
    setSceneValue(scene, 'skybox', env.show_background === false ? null : skybox, owned);
    setSceneValue(scene, 'skyboxMip', 0, owned);
  }
  const sun = new pc.Entity('heron-sun');
  const toSun = new pc.Vec3(...env.to_sun).normalize();
  // A light shines down its entity's -Y.
  sun.setLocalRotation(rotationBetween(pc, pc.Vec3.DOWN, toSun.clone().mulScalar(-1)));
  // The editor's view and three.js light a diffuse surface by sun / pi; PlayCanvas's
  // lights do not divide by pi.
  sun.addComponent('light', {
    type: 'directional',
    color: new pc.Color(...env.sun_color),
    intensity: env.sun_intensity / Math.PI,
    castShadows: env.shadows === true,
    shadowResolution: 2048,
    shadowDistance: 100,
  });
  root.addChild(sun);
  made.entities.push(sun);
  setSceneValue(scene, 'exposure', env.exposure, owned);
  for (const camera of app.systems.camera?.cameras ?? []) setSceneValue(camera, 'toneMapping', pc.TONEMAP_ACES, owned);
  if (env.fog) {
    setSceneValue(scene.fog, 'type', pc.FOG_EXP2, owned);
    setSceneValue(scene.fog, 'density', env.fog.density, owned);
    setSceneValue(scene.fog, 'color', new pc.Color(...env.fog.color), owned);
  }
  return made;
}

// A light component for an entity's `light` block, on a child turned so the light's
// -Y is the entity's -Z.
function makeLight(pc, record, shadowsEnabled) {
  const l = record.entity.light;
  const child = new pc.Entity(record.name + '/light');
  child.setLocalEulerAngles(90, 0, 0);
  const type = l.kind === 'point' ? 'omni' : l.kind;
  const options = {
    type,
    color: new pc.Color(...l.color),
    intensity: l.energy,
    castShadows: shadowsEnabled && l.shadows === true,
  };
  if (type !== 'directional') Object.assign(options, {range: l.range, falloffMode: pc.LIGHTFALLOFF_INVERSESQUARED});
  if (type === 'spot') Object.assign(options, {innerConeAngle: 0, outerConeAngle: l.angle});
  child.addComponent('light', options);
  record.node.addChild(child);
  return child;
}

// Loads a level. `source` is the level JSON's URL, or {level, meshes, sky}: the parsed
// level, its GLBs by the level's mesh paths (bytes, or URLs), and the sky panorama (a
// URL, or the .hdr's bytes).
// Options:
//   parent     an entity to put the level under (place it first: bodies are placed
//              where it is when they are made)
//   factories  {classname or playcanvas_factory property: (record, level) => cleanup?},
//              called once every entity exists and its targets are resolved
//   lights     false to leave out the entities' light components
//   sky        false to leave out the sky, sun, fog and tone mapping
export async function loadLevel(pc, app, source, options = {}) {
  let level = source.level, base = '', meshSources = source.meshes ?? {};
  if (typeof source === 'string') {
    const response = await fetch(source);
    if (!response.ok) throw new Error(`Could not load level (${response.status}): ${source}`);
    level = await response.json();
    base = new URL('.', new URL(source, location.href)).href;
  }
  validateLevel(level);
  const rigidbody = app.systems.rigidbody;
  if (rigidbody && !rigidbody.physicsWorld && typeof Ammo !== 'undefined') rigidbody.onLibraryLoaded();
  const physics = !!rigidbody?.physicsWorld;
  const root = new pc.Entity(level.name);
  (options.parent ?? app.root).addChild(root);
  const containers = new Map(), materials = new Set(), releases = [];
  const container = path => {
    if (!containers.has(path)) {
      const source = meshSources[path] ?? base + path;
      containers.set(path, acquireAsset(app, 'container', source, () => loadContainer(pc, app, path, source))
        .then(({asset, release}) => { releases.push(release); return asset; }));
    }
    return containers.get(path);
  };
  const out = {
    data: level, root, entities: [], byName: new Map(), animations: new Map(), triggers: [], waters: [],
    lights: [], bodies: [], environment: {entities: [], textures: [], assets: []}, sceneValues: [], assets: [], cleanups: [], time: 0,
  };
  let disposed = false;
  out.dispose = () => {
    if (disposed) return;
    disposed = true;
    cleanupAll([
      ...out.cleanups.splice(0).reverse(),
      () => restoreSceneValues(out.sceneValues),
      () => root.destroy(),
      ...[...materials].map(material => () => material.destroy()),
      ...out.environment.textures.map(texture => () => texture.destroy()),
      ...releases,
    ]);
  };
  try {
  for (const entity of level.entities) {
    const t = entity.transform;
    const node = new pc.Entity(entity.name);
    root.addChild(node);
    node.setLocalPosition(...t.translation);
    node.setLocalRotation(rotationYXZ(pc, ...t.rotation_yxz));
    node.setLocalScale(...t.scale);
    const properties = entity.properties ?? {};
    const record = {
      entity, node, name: entity.name, classname: entity.classname, properties,
      values: Object.fromEntries(Object.entries(properties).map(([key, value]) => [key, plainValue(value)])),
      targetname: plainValue(properties.targetname) ?? null, targets: [], meshes: [],
      body: null, light: null, animation: null, trigger: null, water: null,
    };
    node.heron = record;
    out.entities.push(record); out.byName.set(entity.name, record);
    const waterData = properties._heron_water?.value;
    if (waterData) { record.water = new HeronWater(pc, waterData, node); out.waters.push(record.water); }
    const animationData = properties._heron_animation?.value;
    if (animationData) { record.animation = new HeronAnimation(pc, animationData, node); out.animations.set(entity.name, record.animation); }
    const triggerData = properties._heron_trigger?.value;
    if (triggerData) { record.trigger = new HeronAnimationTrigger(pc, triggerData, node); record.trigger.entityName = entity.name; out.triggers.push(record.trigger); }
    if (entity.mesh) {
      // Copies of one GLB share its meshes and materials; water gets materials of its own.
      const asset = await container(entity.mesh);
      if (!out.assets.includes(asset)) out.assets.push(asset);
      const model = asset.resource.instantiateRenderEntity();
      node.addChild(model);
      const renderers = model.findComponents('render');
      if (waterData?.surface_material) {
        for (const render of renderers) render.castShadows = false;
      }
      record.meshes = renderers.flatMap(render => render.meshInstances);
      if (record.water) {
        const copies = new Map();
        for (const instance of record.meshes) {
          if (!copies.has(instance.material)) {
            const copy = instance.material.clone();
            materials.add(copy);
            record.water.material(copy, app.graphicsDevice);
            copies.set(instance.material, copy);
          }
          instance.material = copies.get(instance.material);
        }
      }
    }
    if (entity.light && options.lights !== false) {
      record.light = makeLight(pc, record, level.environment?.shadows !== false);
      out.lights.push(record.light);
    }
    const collision = entity.collision;
    if (physics && collision) {
      const body = collision.sensor ? null : record.animation ? 'kinematic' : 'static';
      if (addCollision(pc, app, node, collision, body)) {
        record.body = node;
        out.bodies.push(node);
      }
    }
  }
  for (const record of out.entities) {
    record.targets = (record.entity.targets ?? []).map(name => out.byName.get(name)).filter(Boolean);
  }
  if (options.sky !== false) {
    const env = level.environment;
    const sky = source.sky ?? (env?.panorama ? base + env.panorama : null);
    await applyEnvironment(pc, app, env, sky, root, out.environment, out.sceneValues, releases);
  }
  for (const record of out.entities) {
    const factory = options.factories?.[record.values.playcanvas_factory] ?? options.factories?.[record.classname];
    const cleanup = factory?.(record, out);
    if (typeof cleanup === 'function') out.cleanups.push(cleanup);
  }
  // Advances the animations and triggers by dt seconds. `actors` are [id, point] pairs
  // in world space (pc.Vec3); give a standing player several points (feet, middle, head).
  out.update = (dt, actors = []) => {
    out.time += dt;
    for (const trigger of out.triggers) trigger.update(dt, actors, out.animations);
    for (const animation of out.animations.values()) animation.update(dt);
    for (const water of out.waters) water.animate(out.time);
  };
  // The water at a world point, the way HeronWater.sample picks among volumes.
  out.water = point => HeronWater.sample(out.waters, point);
  // The first body a ray meets: {entity, point, normal}, or null. Trigger volumes are
  // passed unless `triggers` is true.
  out.raycast = (from, to, triggers = false) => physics
    ? rigidbody.raycastFirst(from, to, triggers ? {} : {filterCollisionMask: SOLID_MASK}) : null;
  } catch (error) {
    try { out.dispose(); }
    catch (cleanupError) { throw new AggregateError([error, cleanupError], 'Level load failed and cleanup reported errors', {cause: error}); }
    throw error;
  }
  return out;
}

