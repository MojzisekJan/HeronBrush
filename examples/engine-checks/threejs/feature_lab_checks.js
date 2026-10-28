// Feature lab checks that run in the browser on three.js 0.186: the three.js export
// loaded the way a game loads it (level JSON, GLTFLoader for each GLB, the exported
// animation and water adapters, HDRLoader for the sky), and the portable GLB and glTF
// through GLTFLoader alone. verify_feature_lab.cjs serves the files and calls these.
import * as THREE from 'three';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
import {HDRLoader} from 'three/addons/loaders/HDRLoader.js';
import {Octree} from 'three/addons/math/Octree.js';
import {Capsule} from 'three/addons/math/Capsule.js';
import {ConvexGeometry} from 'three/addons/geometries/ConvexGeometry.js';

// What the map says; the same numbers as check_feature_lab.py.
const TEXTURE_SPANS = {wood: [1, 1], metal: [1, 1], crate: [Math.SQRT2, Math.SQRT2], wall: [2, 0.5], dirt: [0.5, 0.5], hazard: [1, 1]};
const OPAQUE_COLOURS = [[220, 30, 40], [30, 200, 60], [40, 60, 230], [240, 220, 30], [255, 255, 255]];
const TRANSLUCENT = [[30, 200, 220], 128];
const GRADIENTS = {'two-colour': [[255, 0, 0], [0, 0, 255]], 'three-stop': [[255, 255, 0], [0, 255, 0], [0, 0, 0]],
  radial: [[255, 255, 255], [64, 0, 128]], 'black to white': [[0, 0, 0], [255, 255, 255]]};
const SURFACES = [[0.2, 0.9, 0], [0.9, 0, 0], [0.65, 0, 3], [0.05, 1, 0]];
const FOG = {density: 0.02, colour: [180, 190, 200]};

const PASS = (detail) => ({status: 'PASS', detail});
const FAIL = (detail) => ({status: 'FAIL', detail});
const near = (a, b, e = 1e-4) => Math.abs(a - b) <= e;

function pixels(image) {
  const canvas = document.createElement('canvas');
  canvas.width = image.width; canvas.height = image.height;
  const context = canvas.getContext('2d');
  context.drawImage(image, 0, 0);
  return context.getImageData(0, 0, image.width, image.height).data;
}
async function picture(url) {
  const image = new Image(); image.src = url; await image.decode();
  return image;
}
function materials(object) {
  const out = [];
  object.traverse(o => {
    if (o.isMesh) for (const m of [o.material].flat()) out.push({mesh: o, material: m});
  });
  return out;
}
function uvSpan(geometry) {
  const uv = geometry.attributes.uv;
  let lo = [Infinity, Infinity], hi = [-Infinity, -Infinity];
  for (let i = 0; i < uv.count; i++) {
    lo = [Math.min(lo[0], uv.getX(i)), Math.min(lo[1], uv.getY(i))];
    hi = [Math.max(hi[0], uv.getX(i)), Math.max(hi[1], uv.getY(i))];
  }
  return [hi[0] - lo[0], hi[1] - lo[1]];
}
// Every stop's colour is in the image, within the distance check_feature_lab.py allows.
function stopsFound(data, stops, tolerance = 24) {
  return stops.every(stop => {
    for (let i = 0; i < data.length; i += 4) {
      if (Math.hypot(data[i] - stop[0], data[i + 1] - stop[1], data[i + 2] - stop[2]) <= tolerance) return true;
    }
    return false;
  });
}
// The normals of the vertices a mesh's triangles use.
function drawnNormals(geometry) {
  const n = geometry.attributes.normal, index = geometry.index;
  const at = index ? [...index.array] : [...Array(n.count).keys()];
  return at.map(i => [n.getX(i), n.getY(i), n.getZ(i)]);
}

// The level the way a game assembles it: one root per entity at its transform, the
// entity's GLB under it, collision built from the level JSON.
export async function loadThree(base) {
  const level = await (await fetch(base + 'feature_lab.json')).json();
  const {createHeronLight} = await import(base + 'heron_lights.js');
  const loader = new GLTFLoader(), cache = new Map();
  const scene = new THREE.Scene(), entities = new Map();
  for (const entity of level.entities) {
    const root = new THREE.Group();
    const t = entity.transform;
    root.position.fromArray(t.translation); root.rotation.set(...t.rotation_yxz, 'YXZ'); root.scale.fromArray(t.scale);
    scene.add(root);
    const light = createHeronLight(THREE,entity); if (light) root.add(light);
    if (entity.mesh) {
      if (!cache.has(entity.mesh)) cache.set(entity.mesh, loader.loadAsync(base + entity.mesh));
      const gltf = await cache.get(entity.mesh);
      root.add(gltf.scene.clone(true));
    }
    entities.set(entity.properties?.targetname?.value ?? entity.name, {entity, root});
  }
  scene.updateMatrixWorld(true);
  return {level, scene, entities, cache};
}

function collisionGroups(level, entities) {
  const solid = new THREE.Group();
  for (const entity of level.entities) {
    const collision = entity.collision;
    if (!collision || collision.sensor) continue;
    const group = new THREE.Group();
    const t = entity.transform;
    group.position.fromArray(t.translation); group.rotation.set(...t.rotation_yxz, 'YXZ'); group.scale.fromArray(t.scale);
    if (collision.triangles?.length) {
      // Collision triangles wind clockwise (Godot's order); three.js wants counter-clockwise.
      const ccw = collision.triangles.slice();
      for (let i = 0; i < ccw.length; i += 9) for (let a = 0; a < 3; a++) [ccw[i + 3 + a], ccw[i + 6 + a]] = [ccw[i + 6 + a], ccw[i + 3 + a]];
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(ccw, 3));
      group.add(new THREE.Mesh(geometry));
    }
    for (const hull of collision.hulls || []) {
      const points = [];
      for (let i = 0; i < hull.length; i += 3) points.push(new THREE.Vector3().fromArray(hull, i));
      group.add(new THREE.Mesh(new ConvexGeometry(points)));
    }
    solid.add(group);
  }
  solid.updateMatrixWorld(true);
  return new Octree().fromGraphNode(solid);
}

export async function checkThree(base, poses) {
  const {level, scene, entities} = await loadThree(base);
  const {HeronAnimation, HeronAnimationTrigger} = await import(base + 'heron_animation.js');
  const {HeronWater} = await import(base + 'heron_water.js');
  const results = {};
  const at = name => entities.get(name);

  {
    const point = at('fl_light').root.children.find(o=>o.isPointLight);
    const spotRoot = at('fl_spot').root;
    const spot = spotRoot.children.find(o=>o.isSpotLight);
    const expected = new THREE.Vector3(0,0,-1).applyQuaternion(spotRoot.quaternion).normalize();
    const direction = spot?.target.getWorldPosition(new THREE.Vector3()).sub(spot.getWorldPosition(new THREE.Vector3())).normalize();
    const valid = point && spot && near(point.intensity,2.5) && near(point.distance,8) && near(spot.distance,10) && near(spot.angle,Math.PI/6) && direction.distanceTo(expected) < 1e-6;
    results.lights = valid ? PASS('native PointLight and SpotLight retain energy, range, half angle and transformed -Z direction') : FAIL('native common-light settings or orientation differ');
  }

  // Textures: the GLB's own images, repeating, with the face's UV span.
  {
    const problems = [];
    let checked = 0;
    for (const {mesh, material} of materials(at('fl_textures').root)) {
      const span = TEXTURE_SPANS[material.name];
      if (!span) continue;
      const map = material.map;
      if (!map) { problems.push(`${material.name} has no map`); continue; }
      if (map.wrapS !== THREE.RepeatWrapping || map.wrapT !== THREE.RepeatWrapping) problems.push(`${material.name} does not repeat`);
      if (map.colorSpace !== THREE.SRGBColorSpace) problems.push(`${material.name} map is not sRGB`);
      const got = uvSpan(mesh.geometry);
      if (!near(got[0], span[0], 1e-3) || !near(got[1], span[1], 1e-3)) problems.push(`${material.name} UV span ${got.map(v => v.toFixed(3))}, want ${span.map(v => v.toFixed(3))}`);
      const source = pixels(await picture(base + `textures/${material.name}.png`)), loaded = pixels(map.image);
      if (source.length !== loaded.length || source.some((v, i) => v !== loaded[i])) problems.push(`${material.name} pixels differ from textures/${material.name}.png`);
      checked++;
    }
    results.textures = problems.length || checked !== 6 ? FAIL(problems.join('; ') || `${checked} of 6 faces`) : PASS('6 GLTFLoader maps repeat, keep the source pixels and the face UV spans (1, 2 by 1/2, sqrt 2 at 45 degrees, 1/2)');
  }

  // Face colours: five exact colours and the translucent paint over wood, clamped.
  {
    const problems = [], found = [];
    const wood = pixels(await picture(base + 'textures/wood.png'));
    let translucent = false;
    for (const {material} of materials(at('fl_colours').root)) {
      const map = material.map;
      if (!map) { problems.push(`${material.name} has no map`); continue; }
      if (map.wrapS !== THREE.ClampToEdgeWrapping || map.wrapT !== THREE.ClampToEdgeWrapping) problems.push(`${material.name} paint does not clamp`);
      const data = pixels(map.image);
      if (map.image.width === 1) { found.push([data[0], data[1], data[2]]); continue; }
      const [paint, alpha] = TRANSLUCENT;
      let worst = 0;
      for (let i = 0; i < data.length; i += 4) for (let c = 0; c < 3; c++) {
        const want = (wood[i + c] * (255 - alpha) + paint[c] * alpha) / 255;
        worst = Math.max(worst, Math.abs(data[i + c] - want));
      }
      translucent = true;
      if (worst > 4) problems.push(`translucent paint off by ${worst.toFixed(1)}/255`);
    }
    for (const want of OPAQUE_COLOURS) if (!found.some(c => c.every((v, i) => v === want[i]))) problems.push(`colour ${want} missing`);
    if (!translucent) problems.push('no translucent face');
    results.colours = problems.length ? FAIL(problems.join('; ')) : PASS('5 exact colours and wood blended with paint at 128/255 within 4/255; every paint map clamps');
  }

  // Gradients: each gradient's stops are in one clamped image.
  {
    const images = materials(at('fl_gradients').root).map(({material}) => material.map).filter(m => m && m.image.width > 1);
    const problems = [];
    for (const [name, stops] of Object.entries(GRADIENTS)) {
      if (!images.some(map => stopsFound(pixels(map.image), stops))) problems.push(`${name} gradient not found`);
    }
    if (images.some(map => map.wrapS !== THREE.ClampToEdgeWrapping)) problems.push('a gradient map repeats');
    results.gradients = problems.length ? FAIL(problems.join('; ')) : PASS(`4 gradients with all their stops in ${images.length} clamped maps`);
  }

  // Surface materials and emissive strength.
  {
    const got = materials(at('fl_surface').root).map(({material}) => material);
    const problems = [];
    for (const [roughness, metalness, emission] of SURFACES) {
      const m = got.find(m => near(m.roughness, roughness, 1e-3) && near(m.metalness, metalness, 1e-3));
      if (!m) { problems.push(`no material with roughness ${roughness} metalness ${metalness}`); continue; }
      if (emission && (!near(m.emissiveIntensity, emission, 1e-3) || !m.emissiveMap || m.emissive.getHex() !== 0xffffff)) problems.push(`emission ${m.emissiveIntensity}, map ${!!m.emissiveMap}`);
    }
    results.surface = problems.length ? FAIL(problems.join('; ')) : PASS('roughness and metalness from the GLB; KHR_materials_emissive_strength gives emissiveIntensity 3 with the paint as emissive map');
  }

  // Normal maps: a 512 by 512 normal map, and the vertex tangents the shader reads.
  {
    const [{mesh, material}] = materials(at('fl_normal').root);
    const problems = [];
    if (!material.normalMap || material.normalMap.image.width !== 512 || material.normalMap.image.height !== 512) problems.push('no 512 x 512 normal map');
    if (material.normalMap && material.normalMap.colorSpace !== THREE.NoColorSpace) problems.push('normal map is colour-managed');
    if (mesh.geometry.attributes.tangent?.itemSize !== 4) problems.push('no TANGENT attribute');
    results.normal = problems.length ? FAIL(problems.join('; ')) : PASS('normalMap 512 x 512, linear, with the GLB TANGENT attribute (checked again in the render below)');
  }

  // Collision: an Octree from the JSON's triangles and inline hulls, sensors left out.
  {
    const world = collisionGroups(level, entities);
    const down = name => {
      const p = at(name).root.position;
      return world.rayIntersect(new THREE.Ray(new THREE.Vector3(p.x, 10, p.z), new THREE.Vector3(0, -1, 0)));
    };
    const side = (name, direction) => {
      const p = at(name).root.position, d = new THREE.Vector3(...direction);
      return world.rayIntersect(new THREE.Ray(p.clone().addScaledVector(d, -5), d));
    };
    const hitY = name => down(name)?.position.y;
    const fall = name => {
      // A capsule dropped onto the station settles on what blocks it.
      const p = at(name).root.position;
      const capsule = new Capsule(new THREE.Vector3(p.x, 6, p.z), new THREE.Vector3(p.x, 7, p.z), 0.3);
      for (let i = 0; i < 400; i++) {
        capsule.translate(new THREE.Vector3(0, -0.02, 0));
        const hit = world.capsuleIntersect(capsule);
        if (hit) capsule.translate(hit.normal.multiplyScalar(hit.depth));
      }
      return capsule.start.y - 0.3;
    };
    const solid = hitY('fl_solid'), clip = hitY('fl_clip'), skip = hitY('fl_skip'), hidden = hitY('fl_hidden'), trigger = hitY('fl_trigger');
    const r = {};
    r.solid = near(solid, 2.25, 1e-3) && near(fall('fl_solid'), 2.25, 0.02) ? PASS(`ray and capsule stop on fl_solid's hull top at y=${solid.toFixed(3)}`) : FAIL(`fl_solid top at ${solid}, capsule at ${fall('fl_solid')}`);
    const clipMeshes = materials(at('fl_clip').root).length;
    r.clip = clipMeshes === 0 && near(clip, 3.25, 1e-3) ? PASS(`no mesh; the hull stops a ray at y=${clip.toFixed(3)}`) : FAIL(`fl_clip draws ${clipMeshes} meshes, top at ${clip}`);
    const skipSides = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1], [0, 1, 0]].filter(d => side('fl_skip', d)?.distance < 5).length;
    r.skip = near(skip, 0, 1e-3) && skipSides === 5 ? PASS('a ray from above passes the skip top to the floor at y=0; rays at the four walls and the bottom stop') : FAIL(`skip top hit at ${skip}, ${skipSides} of 5 sides hit`);
    const hiddenSides = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1], [0, -1, 0]].filter(d => side('fl_hidden', d)?.distance < 5).length;
    r.hidden = near(hidden, 2.25, 1e-3) && hiddenSides === 5 ? PASS('collision keeps the hidden face: rays stop on all sides') : FAIL(`hidden top ${hidden}, ${hiddenSides} sides`);
    r.trigger = near(trigger, 0, 1e-3) ? PASS('the sensor is left out of the Octree: a ray passes to the floor') : FAIL(`trigger blocks at ${trigger}`);
    results.collision = r;
  }

  // Animation: the exported runtime's poses against the editor's.
  const posedProbe = (root, rest, point) => new THREE.Vector3(...point).applyMatrix4(rest).applyMatrix4(root.matrixWorld);
  const compare = (name, animation, clip) => {
    const {root} = at(name);
    let worst = 0;
    for (const sample of poses[name]) {
      if (clip) animation.play(clip);
      animation.seek(sample.time); animation.update(0);
      root.updateMatrixWorld(true);
      sample.rest.forEach((point, i) => {
        const got = posedProbe(root, animation.restInverse, point);
        worst = Math.max(worst, got.distanceTo(new THREE.Vector3(...sample.posed[i])));
      });
    }
    return worst;
  };
  const animations = new Map();
  for (const [name, {entity, root}] of entities) {
    const data = entity.properties?._heron_animation?.value;
    if (!data) continue;
    root.updateMatrixWorld(true);
    const animation = new HeronAnimation(THREE, data, root);
    animation.restInverse = root.matrixWorld.clone().invert();
    animations.set(entity.name, animation);
    animation.targetname = name;
  }
  {
    const mover = [...animations.values()].find(a => a.targetname === 'fl_mover');
    const worst = compare('fl_mover', mover);
    results.animation = worst < 1e-4 && mover.data.autoplay ? PASS(`heron_animation.js matches ${poses.fl_mover.length} editor poses of fl_mover within ${worst.toExponential(1)} m (linear, hold, bezier)`) : FAIL(`worst pose error ${worst} m`);
  }

  // The trigger is a sensor that opens the door: an actor walks in, HeronAnimationTrigger fires.
  {
    const door = [...animations.values()].find(a => a.targetname === 'fl_door');
    const doorPoses = compare('fl_door', door, 'open');
    door.stop(); door.update(0);
    const triggers = [];
    for (const [name, {entity, root}] of entities) {
      const data = entity.properties?._heron_trigger?.value;
      if (data) { const trigger = new HeronAnimationTrigger(data, root); trigger.targetname = name; triggers.push(trigger); }
    }
    const trigger = triggers.find(t => t.targetname === 'fl_trigger'), button = triggers.find(t => t.targetname === 'fl_button');
    const step = (actor, seconds) => {
      for (let t = 0; t < seconds; t += 1 / 60) {
        for (const tr of triggers) tr.update(1 / 60, actor ? [['player', actor]] : [], animations);
        for (const a of animations.values()) a.update(1 / 60);
      }
    };
    const outside = new THREE.Vector3(-5, 1, 0), inside = at('fl_trigger').root.position.clone();
    step(outside, 0.5);
    const before = {clip: door.clip.name, posed: door.posed};
    step(inside, 1.2);
    const opened = {clip: door.clip.name, time: door.time, playing: door.playing};
    const doorRoot = at('fl_door').root;
    doorRoot.updateMatrixWorld(true);
    const final = poses.fl_door.at(-1);
    const error = Math.max(...final.rest.map((p, i) => posedProbe(doorRoot, door.restInverse, p).distanceTo(new THREE.Vector3(...final.posed[i]))));
    // Out and straight back in inside the 1.5 s wait: no second firing.
    step(outside, 0.1); door.time = 0.5; step(inside, 0.1);
    const refired = door.time < 0.5;
    button.activate(); step(null, 1.2);
    const toggled = door.clip.name;
    const ok = !before.posed && opened.clip === 'open' && !opened.playing && near(opened.time, 1) && error < 1e-4 && !refired && toggled === 'close' && doorPoses < 1e-4;
    results.door = ok ? PASS(`walking into fl_trigger plays open; after 1.2 s the door matches the editor's open pose within ${error.toExponential(1)} m; no refire inside the 1.5 s wait; fl_button toggles to close; open clip matches ${poses.fl_door.length} editor poses`)
      : FAIL(JSON.stringify({before, opened, error, refired, toggled, doorPoses}));
  }

  // Sky and fog: sky.hdr through HDRLoader, FogExp2 from the environment.
  {
    const sky = await new HDRLoader().loadAsync(base + 'feature_lab/sky.hdr');
    sky.mapping = THREE.EquirectangularReflectionMapping;
    const {width, height, data} = sky.image;
    const value = (x, y, c) => THREE.DataUtils.fromHalfFloat(data[(y * width + x) * 4 + c]);
    const top = [0, 1, 2].map(c => value(width >> 1, 1, c)), horizon = [0, 1, 2].map(c => value(width >> 1, (height >> 1) - 1, c));
    const env = level.environment;
    const fog = new THREE.FogExp2(new THREE.Color().setRGB(...env.fog.color, THREE.LinearSRGBColorSpace), env.fog.density);
    // The map's colours are the editor's linear light values (writer/sky.rs), so the
    // fog's linear colour is 180 190 200 over 255.
    const fogSrgb = fog.color.toArray().map(v => Math.round(v * 255));
    const problems = [];
    if (width !== 2 * height || data.length !== width * height * 4) problems.push(`sky is ${width} x ${height}`);
    if (!(top[2] > top[0]) || !(horizon[0] > horizon[2])) problems.push(`sky top ${top} horizon ${horizon}`);
    if (!near(fog.density, FOG.density, 1e-6)) problems.push(`fog density ${fog.density}`);
    if (fogSrgb.some((v, i) => Math.abs(v - FOG.colour[i]) > 1)) problems.push(`fog colour ${fogSrgb}`);
    scene.background = sky; scene.environment = sky; scene.fog = fog;
    results.sky = problems.length ? FAIL(problems.join('; ')) : PASS(`HDRLoader reads sky.hdr ${width} x ${height}: blue at the top, warm at the horizon; FogExp2 density 0.02, colour ${fogSrgb}`);
  }

  // Water: the adapter's queries, its material on the GLB surface, only upward faces.
  {
    const {entity, root} = at('fl_water');
    const water = new HeronWater(THREE, entity.properties._heron_water.value, root);
    let triangles = 0, up = true;
    for (const {mesh, material} of materials(root)) {
      water.material(material);
      const normals = drawnNormals(mesh.geometry);
      triangles += normals.length / 3;
      up &&= normals.every(n => n[1] > 0.99);
      if (!material.transparent || !near(material.opacity, 0.55, 1e-3)) up = false;
    }
    const p = root.position;
    const inside = HeronWater.sample([water], new THREE.Vector3(p.x, p.y, p.z));
    const above = HeronWater.sample([water], new THREE.Vector3(p.x, p.y + 1, p.z));
    const ok = inside && near(inside.depth, 0.75, 1e-3) && inside.data.swim_speed === 2.5 && inside.data.priority === 3 && !above && triangles === 2 && up;
    results.water = ok ? PASS('HeronWater finds the centre 0.75 m under the surface (swim_speed 2.5, priority 3) and nothing above it; the GLB draws 2 upward triangles with the blended water material')
      : FAIL(JSON.stringify({inside: inside && {depth: inside.depth}, above: !!above, triangles, up}));
  }

  // Imported models: the model's PBR maps, and two copies of one GLB.
  {
    const a = at('fl_model_a'), b = at('fl_model_b');
    const problems = [];
    for (const {material} of materials(a.root)) {
      for (const slot of ['map', 'normalMap', 'roughnessMap', 'metalnessMap', 'aoMap', 'emissiveMap']) if (!material[slot]) problems.push(`${material.name} lacks ${slot}`);
      if (!near(material.normalScale.x, 0.35, 1e-3)) problems.push(`normal scale ${material.normalScale.x}`);
      if (!near(material.aoMapIntensity, 0.25, 1e-3)) problems.push(`occlusion strength ${material.aoMapIntensity}`);
    }
    if (a.entity.mesh !== b.entity.mesh) problems.push('copies name different GLBs');
    if (b.root.scale.x / a.root.scale.x !== 1.5) problems.push('copy scales');
    results.models = problems.length ? FAIL(problems.join('; ')) : PASS('both copies load one GLB; its material has base colour, normal (scale 0.35), metal-roughness, occlusion (0.25) and emissive maps');
  }
  return {results, scene};
}

// Renders a scene from above the lab into a PNG data URL, after checking that the
// normal-mapped material's linked program reads the tangent attribute.
export function render(scene, camera, size = [1280, 720]) {
  const canvas = document.createElement('canvas');
  const renderer = new THREE.WebGLRenderer({canvas, antialias: true, preserveDrawingBuffer: true});
  renderer.setSize(...size, false);
  renderer.toneMapping = THREE.AgXToneMapping;
  renderer.setClearColor(0x91a8bb);
  scene.add(new THREE.HemisphereLight(0xffffff, 0x444444, 1.5));
  const sun = new THREE.DirectionalLight(0xffffff, 2.5); sun.position.set(5, 10, 3); scene.add(sun);
  renderer.render(scene, camera);
  let tangents = null;
  scene.traverse(o => {
    if (o.isMesh && o.material.normalMap && o.geometry.attributes.tangent && tangents === null) {
      const program = renderer.properties.get(o.material).currentProgram;
      tangents = program ? 'tangent' in program.getAttributes() : null;
    }
  });
  return {png: canvas.toDataURL('image/png'), tangents};
}

// The portable GLB or glTF through GLTFLoader alone, as any glTF tool would read it.
export async function checkPortable(url) {
  const gltf = await new GLTFLoader().loadAsync(url);
  const scene = gltf.scene;
  scene.updateMatrixWorld(true);
  const byName = name => {
    let found = null;
    scene.traverse(o => { if (!found && o.userData?.entity?.targetname === name) found = o; });
    return found;
  };
  const results = {};
  const mats = name => materials(byName(name));
  const problems = [];
  // Textures: maps repeat and keep the source's pixels.
  for (const {material} of mats('fl_textures')) {
    if (!material.map || material.map.wrapS !== THREE.RepeatWrapping) problems.push(`texture ${material.name} map or wrap`);
  }
  results.textures = problems.length ? FAIL(problems.join('; ')) : PASS('every textured face has a repeating map');
  // Colours: solid paint is a named colour factor.
  const colours = mats('fl_colours').map(({material}) => material);
  const named = colours.filter(m => /^Paint #[0-9A-F]{6}$/.test(m.name));
  const exact = OPAQUE_COLOURS.every(want => colours.some(m => !m.map && m.color.clone().convertLinearToSRGB().toArray().every((v, i) => Math.abs(v * 255 - want[i]) < 0.6)));
  const translucent = colours.find(m => m.map && m.map.image.width > 1);
  results.colours = exact && named.length === 5 && translucent?.map.wrapS === THREE.ClampToEdgeWrapping
    ? PASS(`5 exact colour factors named ${named.map(m => m.name).join(', ')}; the translucent face is a clamped baked map`)
    : FAIL(JSON.stringify({exact, names: colours.map(m => m.name), clamp: translucent?.map.wrapS}));
  const gradientImages = mats('fl_gradients').map(({material}) => material.map).filter(Boolean);
  const gradientsOk = Object.values(GRADIENTS).every(stops => gradientImages.some(map => stopsFound(pixels(map.image), stops)));
  results.gradients = gradientsOk ? PASS('4 gradients with all their stops in the baked maps') : FAIL('a gradient lacks a stop');
  const surface = mats('fl_surface').map(({material}) => material);
  const surfaceOk = SURFACES.every(([r, m, e]) => surface.some(x => near(x.roughness, r, 1e-3) && near(x.metalness, m, 1e-3) && (!e || near(x.emissiveIntensity, e, 1e-3))));
  results.surface = surfaceOk ? PASS('roughness, metalness and emissive strength 3') : FAIL(JSON.stringify(surface.map(x => [x.name, x.roughness, x.metalness, x.emissiveIntensity])));
  const [{mesh, material}] = mats('fl_normal');
  results.normal = material.normalMap?.image.width === 512 && mesh.geometry.attributes.tangent ? PASS('512 x 512 normal map with tangents') : FAIL('normal map or tangents missing');
  // Trigger and water: the trigger draws nothing and keeps its volume; the water draws its top.
  const trigger = byName('fl_trigger'), water = byName('fl_water');
  const triggerMeshes = materials(trigger).length, planes = trigger.userData.heron?.volume?.brushes?.[0]?.planes;
  results.trigger = triggerMeshes === 0 && planes?.length === 6 && trigger.userData.entity.target === 'fl_door'
    ? PASS('the trigger node has no mesh; extras keep target fl_door and its 6 planes') : FAIL(JSON.stringify({triggerMeshes, planes}));
  let waterTriangles = 0, waterUp = true;
  for (const {mesh} of materials(water)) {
    const normals = drawnNormals(mesh.geometry);
    waterTriangles += normals.length / 3;
    waterUp &&= normals.every(n => n[1] > 0.99);
  }
  results.water = waterTriangles === 2 && waterUp && water.userData.heron?.volume ? PASS('the water draws 2 upward triangles; its volume is in extras') : FAIL(JSON.stringify({waterTriangles, waterUp}));
  // Properties: GLTFLoader puts extras in userData.
  const probe = byName('fl_probe').userData.heron.properties;
  results.properties = probe.charge.type === 'int' && probe.charge.value === 7 && probe.armed.value === true && probe.sequence.value.map(v => v.value).join() === '2,3,5'
    ? PASS('userData.heron keeps charge int 7, armed true, sequence [2, 3, 5]') : FAIL(JSON.stringify(probe));
  const model = mats('fl_model_a').map(({material}) => material);
  const pbr = model.every(m => m.normalMap && m.roughnessMap && m.metalnessMap && m.aoMap && m.emissiveMap);
  const a = byName('fl_model_a'), b = byName('fl_model_b');
  const shared = materials(a)[0].mesh.geometry === materials(b)[0].mesh.geometry;
  results.models = pbr && shared ? PASS('both model copies share one geometry; all PBR maps present') : FAIL(JSON.stringify({pbr, shared}));
  return {results, scene};
}
