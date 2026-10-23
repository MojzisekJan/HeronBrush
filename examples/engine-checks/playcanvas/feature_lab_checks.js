// Feature lab checks that run in the browser on PlayCanvas 2 with ammo.js: the
// PlayCanvas export loaded the way a game loads it (heron_playcanvas.js's loadLevel:
// a container asset for each GLB, rigidbody and collision components from the level
// JSON, the exported animation, trigger and water classes, sky.hdr through
// EnvLighting), and the generated playtest page. verify_feature_lab.cjs serves the
// files and calls these.

// What the map says; the same numbers as check_feature_lab.py.
const TEXTURE_SPANS = {wood: [1, 1], metal: [1, 1], crate: [Math.SQRT2, Math.SQRT2], wall: [2, 0.5], dirt: [0.5, 0.5], hazard: [1, 1]};
const OPAQUE_COLOURS = [[220, 30, 40], [30, 200, 60], [40, 60, 230], [240, 220, 30], [255, 255, 255]];
const TRANSLUCENT = [[30, 200, 220], 128];
const GRADIENTS = {'two-colour': [[255, 0, 0], [0, 0, 255]], 'three-stop': [[255, 255, 0], [0, 255, 0], [0, 0, 0]],
  radial: [[255, 255, 255], [64, 0, 128]], 'black to white': [[0, 0, 0], [255, 255, 255]]};
const SURFACES = [[0.2, 0.9, 0], [0.9, 0, 0], [0.65, 0, 3], [0.05, 1, 0]];
const FOG = {density: 0.02, colour: [180, 190, 200]};

const PASS = detail => ({status: 'PASS', detail});
const FAIL = detail => ({status: 'FAIL', detail});
const near = (a, b, e = 1e-4) => Math.abs(a - b) <= e;

async function sourcePixels(url) {
  const image = new Image(); image.src = url; await image.decode();
  const canvas = document.createElement('canvas');
  canvas.width = image.width; canvas.height = image.height;
  const context = canvas.getContext('2d');
  context.drawImage(image, 0, 0);
  return context.getImageData(0, 0, image.width, image.height).data;
}
// A texture's level 0 as RGBA bytes, read back from the GPU, rows in the image's order.
async function texturePixels(texture) {
  return new Uint8Array((await texture.read(0, 0, texture.width, texture.height)).buffer);
}
function surfaces(record) {
  return record.meshes.filter(instance => instance.mesh.vertexBuffer.numVertices > 0).map(instance => ({instance, mesh: instance.mesh, material: instance.material}));
}
function uvSpan(mesh) {
  const uv = [];
  mesh.getUvs(0, uv);
  let lo = [Infinity, Infinity], hi = [-Infinity, -Infinity];
  for (let i = 0; i < uv.length; i += 2) {
    lo = [Math.min(lo[0], uv[i]), Math.min(lo[1], uv[i + 1])];
    hi = [Math.max(hi[0], uv[i]), Math.max(hi[1], uv[i + 1])];
  }
  return [hi[0] - lo[0], hi[1] - lo[1]];
}
function stopsFound(data, stops, tolerance = 24) {
  return stops.every(stop => {
    for (let i = 0; i < data.length; i += 4) {
      if (Math.hypot(data[i] - stop[0], data[i + 1] - stop[1], data[i + 2] - stop[2]) <= tolerance) return true;
    }
    return false;
  });
}
function triangleCount(mesh) {
  return mesh.primitive[0].count / 3;
}
// The world-space normals of the vertices a mesh's triangles use.
function drawnNormals(pc, instance) {
  const normals = [], indices = [];
  instance.mesh.getNormals(normals); instance.mesh.getIndices(indices);
  const world = instance.node.getWorldTransform();
  return indices.map(i => world.transformVector(new pc.Vec3(normals[i * 3], normals[i * 3 + 1], normals[i * 3 + 2]), new pc.Vec3()).normalize());
}
// The vertex shaders a mesh instance was drawn with.
function vertexShaders(instance) {
  return [...instance._shaderCache.values()].map(entry => entry.shader?.definition?.vshader ?? '');
}
// The middle of the canvas after a render, as an RGB average.
function middle(app, half = 4) {
  app.render();
  const gl = app.graphicsDevice.gl, w = 2 * half + 1, pixels = new Uint8Array(w * w * 4);
  gl.readPixels((gl.drawingBufferWidth >> 1) - half, (gl.drawingBufferHeight >> 1) - half, w, w, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  const sum = [0, 0, 0];
  for (let i = 0; i < pixels.length; i += 4) for (let c = 0; c < 3; c++) sum[c] += pixels[i + c];
  return sum.map(v => v / (w * w));
}

export async function checkPlayCanvas(base, poses, canvas, ammo) {
  globalThis.Ammo = await window.Ammo({locateFile: file => ammo + file});
  // No multisampling, so the check can read the canvas back.
  const app = new pc.Application(canvas, {graphicsDeviceOptions: {preserveDrawingBuffer: true, antialias: false}});
  // The check steps physics itself.
  app.systems.rigidbody.timeScale = 0;
  const camera = new pc.Entity('check');
  camera.addComponent('camera', {fov: 55, nearClip: 0.05, farClip: 2000, toneMapping: pc.TONEMAP_ACES, clearColor: new pc.Color(0, 0, 0)});
  app.root.addChild(camera);
  const view = (from, to, fov = 55) => { camera.setPosition(...from); camera.lookAt(...to); camera.camera.fov = fov; };
  view([-20, 16, -8], [2, 0, -9]);
  const {loadLevel, rotationYXZ} = await import(base + 'heron_playcanvas.js');
  const level = await loadLevel(pc, app, base + 'feature_lab.json');
  app.start();
  app.render();
  const physics = app.systems.rigidbody;
  const results = {};
  const at = name => level.entities.find(record => record.targetname === name);

  // Textures: the GLB's own images, repeating, sRGB, with the face's UV span.
  {
    const problems = [];
    let checked = 0;
    for (const {mesh, material} of surfaces(at('fl_textures'))) {
      const span = TEXTURE_SPANS[material.name];
      if (!span) continue;
      const map = material.diffuseMap;
      if (!map) { problems.push(`${material.name} has no diffuseMap`); continue; }
      if (map.addressU !== pc.ADDRESS_REPEAT || map.addressV !== pc.ADDRESS_REPEAT) problems.push(`${material.name} does not repeat`);
      if (!map.srgb) problems.push(`${material.name} map is not sRGB`);
      const got = uvSpan(mesh);
      if (!near(got[0], span[0], 1e-3) || !near(got[1], span[1], 1e-3)) problems.push(`${material.name} UV span ${got.map(v => v.toFixed(3))}, want ${span.map(v => v.toFixed(3))}`);
      const source = await sourcePixels(base + `textures/${material.name}.png`), loaded = await texturePixels(map);
      if (source.length !== loaded.length || source.some((v, i) => v !== loaded[i])) problems.push(`${material.name} pixels differ from textures/${material.name}.png`);
      checked++;
    }
    results.textures = problems.length || checked !== 6 ? FAIL(problems.join('; ') || `${checked} of 6 faces`)
      : PASS('6 container-asset diffuse maps repeat, are sRGB textures, keep the source pixels (read back from the GPU) and the face UV spans (1, 2 by 1/2, sqrt 2 at 45 degrees, 1/2)');
  }

  // Face colours: five exact colours and the translucent paint over wood, clamped.
  {
    const problems = [], found = [];
    const wood = await sourcePixels(base + 'textures/wood.png');
    let translucent = false;
    for (const {material} of surfaces(at('fl_colours'))) {
      const map = material.diffuseMap;
      if (!map) { problems.push(`${material.name} has no map`); continue; }
      if (map.addressU !== pc.ADDRESS_CLAMP_TO_EDGE || map.addressV !== pc.ADDRESS_CLAMP_TO_EDGE) problems.push(`${material.name} paint does not clamp`);
      const data = await texturePixels(map);
      if (map.width === 1) { found.push([data[0], data[1], data[2]]); continue; }
      const [paint, alpha] = TRANSLUCENT;
      let worst = 0;
      for (let i = 0; i < data.length; i += 4) for (let c = 0; c < 3; c++) {
        worst = Math.max(worst, Math.abs(data[i + c] - (wood[i + c] * (255 - alpha) + paint[c] * alpha) / 255));
      }
      translucent = true;
      if (worst > 4) problems.push(`translucent paint off by ${worst.toFixed(1)}/255`);
    }
    for (const want of OPAQUE_COLOURS) if (!found.some(c => c.every((v, i) => v === want[i]))) problems.push(`colour ${want} missing`);
    if (!translucent) problems.push('no translucent face');
    results.colours = problems.length ? FAIL(problems.join('; ')) : PASS('5 exact colours and wood blended with paint at 128/255 within 4/255, read back from the GPU; every paint map clamps');
  }

  // Gradients: each gradient's stops in a clamped image, drawn with UVs inside it.
  {
    const maps = surfaces(at('fl_gradients')).map(({mesh, material}) => ({mesh, map: material.diffuseMap})).filter(m => m.map && m.map.width > 1);
    const problems = [];
    const images = await Promise.all(maps.map(({map}) => texturePixels(map)));
    for (const [name, stops] of Object.entries(GRADIENTS)) if (!images.some(data => stopsFound(data, stops))) problems.push(`${name} gradient not found`);
    // A gradient is one strip or disc image per set of colours, laid on the face by
    // affine UVs that run past its ends; only a clamping sampler draws the end colours there.
    let outside = 0;
    for (const {mesh, map} of maps) {
      if (map.addressU !== pc.ADDRESS_CLAMP_TO_EDGE || map.addressV !== pc.ADDRESS_CLAMP_TO_EDGE) problems.push(`${map.name} gradient repeats`);
      const uv = []; mesh.getUvs(0, uv);
      if (uv.some(v => v < -1e-4 || v > 1 + 1e-4)) outside++;
    }
    results.gradients = problems.length ? FAIL(problems.join('; ')) : PASS(`4 gradients with all their stops in ${maps.length} images read back from the GPU, every sampler clamped (${outside} faces have UVs past the image's ends)`);
  }

  // Surface materials and emissive strength. A glTF material's roughness is the
  // StandardMaterial's gloss with glossInvert.
  {
    const got = surfaces(at('fl_surface')).map(({material}) => material);
    const problems = [];
    for (const [roughness, metallic, emission] of SURFACES) {
      const m = got.find(m => m.glossInvert && near(m.gloss, roughness, 1e-3) && near(m.metalness, metallic, 1e-3));
      if (!m) { problems.push(`no material with roughness ${roughness} metallic ${metallic}`); continue; }
      const strength = Math.max(m.emissive.r, m.emissive.g, m.emissive.b) * m.emissiveIntensity;
      if (emission && (!near(strength, emission, 1e-3) || !m.emissiveMap)) problems.push(`emission ${strength} (colour ${m.emissive} intensity ${m.emissiveIntensity}), map ${!!m.emissiveMap}`);
    }
    results.surface = problems.length ? FAIL(problems.join('; ')) : PASS('roughness (gloss with glossInvert) and metalness from the GLB; KHR_materials_emissive_strength gives emission 3 with the paint as emissive map');
  }

  // Normal maps: a 512 by 512 linear normal map, and the tangents the shader reads.
  {
    const [{instance, mesh, material}] = surfaces(at('fl_normal'));
    const problems = [];
    const normal = material.normalMap;
    if (!normal || normal.width !== 512 || normal.height !== 512) problems.push('no 512 x 512 normal map');
    if (normal?.srgb) problems.push('normal map is an sRGB texture');
    if (!mesh.vertexBuffer.format.elements.some(e => e.name === pc.SEMANTIC_TANGENT)) problems.push('no TANGENT attribute');
    const shaders = vertexShaders(instance);
    if (!shaders.some(s => s.includes('vertex_tangent'))) problems.push(`the drawn shader does not read tangents (${shaders.length} shaders)`);
    results.normal = problems.length ? FAIL(problems.join('; ')) : PASS(`normalMap 512 x 512, linear, the GLB TANGENT attribute, and the drawn vertex shader reads vertex_tangent`);
  }

  // Collision: ammo.js rays, a dropped sphere, and the trigger volume.
  const stepPhysics = seconds => { for (let t = 0; t < seconds; t += 1 / 60) physics.step(1 / 60); };
  {
    const centre = name => at(name).node.getPosition().clone();
    const hitY = (name, triggers = false) => { const p = centre(name); return level.raycast(new pc.Vec3(p.x, 10, p.z), new pc.Vec3(p.x, -10, p.z), triggers)?.point.y; };
    const side = (name, d) => { const p = centre(name), dir = new pc.Vec3(...d).mulScalar(5); return level.raycast(p.clone().sub(dir), p.clone().add(dir)); };
    const drop = name => {
      const p = centre(name);
      const ball = new pc.Entity('probe');
      ball.setPosition(p.x, 6, p.z);
      ball.addComponent('rigidbody', {type: 'dynamic', mass: 1});
      ball.addComponent('collision', {type: 'sphere', radius: 0.3});
      app.root.addChild(ball);
      stepPhysics(3);
      const bottom = ball.getPosition().y - 0.3;
      ball.destroy();
      return bottom;
    };
    const solid = hitY('fl_solid'), clip = hitY('fl_clip'), skip = hitY('fl_skip'), hidden = hitY('fl_hidden'), trigger = hitY('fl_trigger');
    const r = {};
    const fallen = drop('fl_solid');
    const solidBody = at('fl_solid').node.rigidbody;
    r.solid = near(solid, 2.25, 1e-3) && near(fallen, 2.25, 0.02) && solidBody?.type === 'static' && at('fl_solid').node.collision.convexHull
      ? PASS(`a static rigidbody with a convex hull: a ray stops on fl_solid's top at y=${solid.toFixed(3)}, a dropped sphere settles at ${fallen.toFixed(3)}`)
      : FAIL(`fl_solid top at ${solid}, sphere at ${fallen}, body ${solidBody?.type}`);
    const clipMeshes = at('fl_clip').meshes.length;
    r.clip = clipMeshes === 0 && near(clip, 3.25, 1e-3) ? PASS(`no mesh; the hull stops a ray at y=${clip.toFixed(3)}`) : FAIL(`fl_clip draws ${clipMeshes} meshes, top at ${clip}`);
    const skipSides = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1], [0, 1, 0]].filter(d => side('fl_skip', d)).length;
    // Bullet's triangle meshes are two-sided to rays, so the ray stops on the inside of
    // the bottom face.
    r.skip = near(skip, 0.25, 1e-3) && skipSides === 5 && !at('fl_skip').node.collision.convexHull
      ? PASS('a static triangle mesh: a ray from above passes the skip top and stops on the inside of the bottom face at y=0.25; rays at the four walls and the bottom stop') : FAIL(`skip top hit at ${skip}, ${skipSides} of 5 sides hit`);
    const hiddenSides = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1], [0, -1, 0]].filter(d => side('fl_hidden', d)).length;
    const hiddenDrawn = surfaces(at('fl_hidden')).reduce((n, {mesh}) => n + triangleCount(mesh), 0);
    r.hidden = near(hidden, 2.25, 1e-3) && hiddenSides === 5 && hiddenDrawn === 10
      ? PASS('the mesh draws 10 triangles (five sides); collision keeps the hidden face: rays stop on all sides') : FAIL(`hidden top ${hidden}, ${hiddenSides} sides, ${hiddenDrawn} triangles drawn`);
    // The trigger: a collision component with no rigidbody, which PlayCanvas makes a
    // trigger volume that rays and bodies pass and that reports a body entering.
    const record = at('fl_trigger'), events = [];
    const listener = other => events.push('triggerenter ' + other.name);
    record.node.collision.on('triggerenter', listener);
    const throughTrigger = drop('fl_trigger');
    record.node.collision.off('triggerenter', listener);
    const sensorTop = hitY('fl_trigger', true);
    r.trigger = near(trigger, 0, 1e-3) && !record.node.rigidbody && record.node.trigger && record.meshes.length === 0 && events.length && near(throughTrigger, 0, 0.02)
      ? PASS(`a trigger volume, drawn by nothing: a ray passes to the floor (one that takes triggers stops at y=${sensorTop?.toFixed(2)}); a sphere falls through to y=${throughTrigger.toFixed(3)} and the volume fires ${events[0]}`)
      : FAIL(JSON.stringify({trigger, body: !!record.node.rigidbody, volume: !!record.node.trigger, meshes: record.meshes.length, events, throughTrigger}));
    results.collision = r;
  }

  // Animation: the exported runtime's poses against the editor's.
  const rest = new Map();
  for (const record of level.entities) if (record.animation) rest.set(record.targetname, record.node.getWorldTransform().clone().invert());
  const posedProbe = (name, point) => at(name).node.getWorldTransform().transformPoint(rest.get(name).transformPoint(new pc.Vec3(...point), new pc.Vec3()), new pc.Vec3());
  const compare = (name, clip) => {
    const animation = at(name).animation;
    let worst = 0;
    for (const sample of poses[name]) {
      if (clip) animation.play(clip);
      animation.seek(sample.time); animation.update(0);
      sample.rest.forEach((point, i) => { worst = Math.max(worst, posedProbe(name, point).sub(new pc.Vec3(...sample.posed[i])).length()); });
    }
    return worst;
  };
  {
    const mover = at('fl_mover').animation;
    const worst = compare('fl_mover');
    const body = at('fl_mover').node.rigidbody;
    results.animation = worst < 1e-4 && mover.data.autoplay && body?.type === 'kinematic'
      ? PASS(`HeronAnimation matches ${poses.fl_mover.length} editor poses of fl_mover within ${worst.toExponential(1)} m (linear, hold, bezier); its rigidbody is kinematic and follows the entity`)
      : FAIL(`worst pose error ${worst} m, body ${body?.type}`);
    // The kinematic body moves with the entity: a ray finds the mover's top where the pose put it.
    mover.seek(2); mover.update(0); stepPhysics(1 / 60);
    const p = at('fl_mover').node.getPosition();
    const top = level.raycast(new pc.Vec3(p.x, 10, p.z), new pc.Vec3(p.x, -10, p.z));
    results.animatedBody = top && top.entity === at('fl_mover').node ? PASS(`after the pose at 2 s and a physics step, a ray at the mover's new place (z=${p.z.toFixed(2)}) hits its body`) : FAIL(`ray at ${p} hit ${top?.entity?.name}`);
    mover.stop(); mover.update(0);
  }

  // Door, button and trigger: an actor walks in, the door opens to the editor's pose.
  {
    const door = at('fl_door').animation, doorPoses = compare('fl_door', 'open');
    door.stop(); door.update(0);
    const button = at('fl_button').trigger;
    const step = (actors, seconds) => { for (let t = 0; t < seconds; t += 1 / 60) level.update(1 / 60, actors); };
    const outside = [['player', new pc.Vec3(-5, 1, 0)]], inside = [['player', at('fl_trigger').node.getPosition().clone()]];
    step(outside, 0.5);
    const before = {clip: door.clip.name, posed: door.posed};
    step(inside, 1.2);
    const opened = {clip: door.clip.name, time: door.time, playing: door.playing};
    const final = poses.fl_door.at(-1);
    const error = Math.max(...final.rest.map((p, i) => posedProbe('fl_door', p).sub(new pc.Vec3(...final.posed[i])).length()));
    step(outside, 0.1); door.time = 0.5; step(inside, 0.1);
    const refired = door.time < 0.5;
    button.activate(); step([], 1.2);
    const toggled = door.clip.name;
    const ok = !before.posed && opened.clip === 'open' && !opened.playing && near(opened.time, 1) && error < 1e-4 && !refired && toggled === 'close' && doorPoses < 1e-4;
    results.door = ok ? PASS(`walking into fl_trigger plays open; after 1.2 s the door matches the editor's open pose within ${error.toExponential(1)} m; no refire inside the 1.5 s wait; fl_button toggles to close; open clip matches ${poses.fl_door.length} editor poses`)
      : FAIL(JSON.stringify({before, opened, error, refired, toggled, doorPoses}));
    door.stop(); door.update(0);
  }

  // Sky and fog: sky.hdr through EnvLighting as the skybox and the environment atlas,
  // the sun's glow where the environment says the sun is; FOG_EXP2 from the level.
  {
    const problems = [];
    const env = level.data.environment, scene = app.scene;
    if (!scene.skybox?.cubemap) problems.push('no skybox cubemap');
    if (!scene.envAtlas) problems.push('no environment atlas');
    const fog = [scene.fog.color.r, scene.fog.color.g, scene.fog.color.b].map(v => Math.round(v * 255));
    if (scene.fog.type !== pc.FOG_EXP2 || !near(scene.fog.density, FOG.density, 1e-6)) problems.push(`fog ${scene.fog.type} density ${scene.fog.density}`);
    if (fog.some((v, i) => Math.abs(v - FOG.colour[i]) > 1)) problems.push(`fog colour ${fog}`);
    // Look from the origin, with only the sky drawn: at the sun, away from it, and up.
    level.root.enabled = false;
    const look = direction => { view([0, 0, 0], direction, 30); return middle(app); };
    const s = env.to_sun;
    const atSun = look(s), away = look([-s[0], s[1], -s[2]]), mirrored = look([s[0], s[1], -s[2]]), top = look([0.001, 1, 0.001]);
    level.root.enabled = true;
    // The map's ambient intensity (0.45) dims the image-based light and not the sky:
    // a white matte card lit by the environment alone, untoned at exposure 1, against
    // the same card under an atlas made from the undimmed sky.
    const card = new pc.Entity('ambient-card'), matte = new pc.StandardMaterial();
    matte.diffuse = new pc.Color(1, 1, 1); matte.useMetalness = true; matte.metalness = 0; matte.gloss = 0; matte.update();
    card.addComponent('render', {type: 'plane', material: matte});
    card.setPosition(-40, 5, 40); card.setLocalScale(4, 4, 4);
    app.root.addChild(card);
    const sun = level.root.findByName('heron-sun');
    sun.enabled = false;
    const tone = [camera.camera.toneMapping, camera.camera.gammaCorrection], fogType = scene.fog.type, exposure = scene.exposure;
    scene.exposure = 1;
    camera.camera.toneMapping = pc.TONEMAP_LINEAR; camera.camera.gammaCorrection = pc.GAMMA_NONE; scene.fog.type = pc.FOG_NONE;
    view([-40, 7, 40.001], [-40, 5, 40]);
    const dimmed = middle(app);
    const source = pc.EnvLighting.generateLightingSource(level.environment.assets[0].resource);
    const full = pc.EnvLighting.generateAtlas(source), kept = scene.envAtlas;
    scene.envAtlas = full;
    const undimmed = middle(app);
    scene.envAtlas = kept; full.destroy(); source.destroy(); card.destroy();
    sun.enabled = true; [camera.camera.toneMapping, camera.camera.gammaCorrection] = tone; scene.fog.type = fogType; scene.exposure = exposure;
    const ratio = dimmed.reduce((a, b) => a + b) / undimmed.reduce((a, b) => a + b);
    if (!near(ratio, env.ambient_intensity, 0.02)) problems.push(`ambient ratio ${ratio.toFixed(3)} (${dimmed.map(Math.round)} of ${undimmed.map(Math.round)}), want ${env.ambient_intensity.toFixed(2)}`);
    const bright = c => c[0] + c[1] + c[2];
    if (!(bright(atSun) > bright(away) + 30 && bright(atSun) > bright(mirrored) + 30)) problems.push(`sun ${atSun.map(Math.round)}, opposite ${away.map(Math.round)}, mirrored in Z ${mirrored.map(Math.round)}`);
    if (!(top[2] > top[0])) problems.push(`zenith ${top.map(Math.round)} is not blue`);
    view([-20, 16, -8], [2, 0, -9]);
    results.sky = problems.length ? FAIL(problems.join('; '))
      : PASS(`EnvLighting turns sky.hdr into the skybox and environment atlas, the atlas lighting a matte card at ${ratio.toFixed(3)} (${dimmed.map(Math.round)} against ${undimmed.map(Math.round)}, untoned at exposure 1) of the undimmed sky's (ambient_intensity ${env.ambient_intensity.toFixed(2)}); looking along to_sun gives ${atSun.map(Math.round)}, the opposite way ${away.map(Math.round)}, mirrored in Z ${mirrored.map(Math.round)}, up ${top.map(Math.round)} (blue); FOG_EXP2 density 0.02, colour ${fog}`);
  }

  // Water: the adapter's queries, its material on the GLB surface, only upward faces.
  {
    const record = at('fl_water'), water = record.water;
    let triangles = 0, up = true;
    for (const {instance, material} of surfaces(record)) {
      const normals = drawnNormals(pc, instance);
      triangles += normals.length / 3;
      up &&= normals.every(n => n.y > 0.99);
      if (material.blendType !== pc.BLEND_NORMAL || !near(material.opacity, 0.55, 1e-3) || material.heronWater !== water) up = false;
    }
    view([-20, 16, -8], [2, 0, -9]);
    app.render();
    const waves = surfaces(record).every(({instance}) => vertexShaders(instance).some(s => s.includes('hbHeight')));
    const p = record.node.getPosition().clone();
    const inside = level.water(p.clone()), above = level.water(p.clone().add(new pc.Vec3(0, 1, 0)));
    const sensor = !record.node.rigidbody && !!record.node.trigger;
    const ok = inside && near(inside.depth, 0.75, 1e-3) && inside.data.swim_speed === 2.5 && inside.data.priority === 3 && !above && triangles === 2 && up && waves && sensor;
    results.water = ok ? PASS('HeronWater finds the centre 0.75 m under the surface (swim_speed 2.5, priority 3) and nothing above it; the GLB draws 2 upward triangles, blended at opacity 0.55, with the wave in the drawn vertex shader; the volume is a trigger')
      : FAIL(JSON.stringify({inside: inside && {depth: inside.depth}, above: !!above, triangles, up, waves, sensor}));
  }

  // Lights: light components from the entities' light blocks, aimed and lighting the level.
  {
    const problems = [];
    const point = at('fl_light').light, spot = at('fl_spot').light;
    const colour = l => [l.light.color.r, l.light.color.g, l.light.color.b].map(v => Math.round(v * 255)).join(' ');
    if (point?.light.type !== 'omni' || !near(point.light.intensity, 2.5) || !near(point.light.range, 8) || colour(point) !== '255 200 120') problems.push(`point ${point?.light.type} ${point?.light.intensity} ${point?.light.range} ${point && colour(point)}`);
    if (spot?.light.type !== 'spot' || !near(spot.light.outerConeAngle, 30) || !near(spot.light.range, 10) || !near(spot.light.intensity, 3)) problems.push(`spot ${spot?.light.type} cone ${spot?.light.outerConeAngle} range ${spot?.light.range}`);
    // A PlayCanvas light shines down its entity's -Y; the spot must shine down its
    // level entity's -Z, turned by the entity's rotation_yxz.
    const r = at('fl_spot').entity.transform.rotation_yxz;
    const want = rotationYXZ(pc, ...r).transformVector(new pc.Vec3(0, 0, -1), new pc.Vec3());
    const got = spot.up.clone().mulScalar(-1).normalize(), position = spot.getPosition().clone();
    if (got.clone().sub(want).length() > 1e-4) problems.push(`spot direction ${got}, want ${want}`);
    if (position.clone().sub(new pc.Vec3(...at('fl_spot').entity.transform.translation)).length() > 1e-4) problems.push(`spot at ${position}`);
    // Each light brightens a white card where it shines: the middle of a view of the
    // card's lit side, with the light on and off. The feature lab's spot points 60
    // degrees up (its exported pitch, as every target turns it), at the sky.
    const card = new pc.Entity('card');
    const material = new pc.StandardMaterial();
    material.diffuse = new pc.Color(0.8, 0.8, 0.8); material.useMetalness = true; material.metalness = 0; material.gloss = 0;
    material.cull = pc.CULLFACE_NONE; material.update();
    card.addComponent('render', {type: 'plane', material});
    card.setLocalScale(2, 2, 2);
    app.root.addChild(card);
    const turn = (from, to) => {
      const d = from.dot(to);
      const c = new pc.Vec3().cross(from, to);
      return d < -0.999999 ? new pc.Quat().setFromAxisAngle(pc.Vec3.RIGHT, 180) : new pc.Quat(c.x, c.y, c.z, 1 + d).normalize();
    };
    const lit = (light, from, along) => {
      const centre = from.clone().add(along.clone().mulScalar(2));
      card.setPosition(centre);
      card.setRotation(turn(pc.Vec3.UP, along.clone().mulScalar(-1)));
      const side = new pc.Vec3().cross(along, Math.abs(along.y) > 0.9 ? pc.Vec3.RIGHT : pc.Vec3.UP).normalize();
      const eye = from.clone().add(along).add(side.mulScalar(0.4));
      view([eye.x, eye.y, eye.z], [centre.x, centre.y, centre.z]);
      const fog = app.scene.fog.type; app.scene.fog.type = pc.FOG_NONE;
      const on = middle(app);
      light.light.enabled = false; const off = middle(app);
      light.light.enabled = true; app.scene.fog.type = fog;
      return [on, off].map(c => c.reduce((a, b) => a + b));
    };
    const [pointOn, pointOff] = lit(point, point.getPosition().clone(), new pc.Vec3(0, -1, 0));
    const [spotOn, spotOff] = lit(spot, position, got);
    card.destroy();
    view([-20, 16, -8], [2, 0, -9]);
    if (!(pointOn > pointOff + 15)) problems.push(`the point light does not light a card 2 m below it: ${pointOn} on, ${pointOff} off`);
    if (!(spotOn > spotOff + 15)) problems.push(`the spot does not light where it points: ${spotOn} on, ${spotOff} off`);
    results.lights = problems.length ? FAIL(problems.join('; '))
      : PASS(`omni light (255 200 120, intensity 2.5, range 8) and spot light (outer cone 30 degrees from the axis, range 10) on children of their entities; the spot points down the turned -Z within 1e-4; each brightens a card 2 m from it on its axis (sums ${Math.round(pointOff)} to ${Math.round(pointOn)}, ${Math.round(spotOff)} to ${Math.round(spotOn)})`);
  }

  // Imported models: two copies of one GLB, its PBR maps.
  {
    const a = at('fl_model_a'), b = at('fl_model_b');
    const problems = [];
    const [ma, mb] = [surfaces(a), surfaces(b)];
    if (!ma.length || ma.length !== mb.length || ma.some(({mesh}, i) => mesh !== mb[i].mesh)) problems.push('copies do not share meshes');
    for (const {material} of ma) {
      for (const slot of ['diffuseMap', 'normalMap', 'metalnessMap', 'glossMap', 'aoMap', 'emissiveMap']) if (!material[slot]) problems.push(`${material.name} lacks ${slot}`);
      if (!near(material.bumpiness, 0.35, 1e-3)) problems.push(`normal scale ${material.bumpiness}`);
      if (!near(material.aoIntensity, 0.25, 1e-3)) problems.push(`occlusion strength ${material.aoIntensity}`);
    }
    if (b.node.getLocalScale().x / a.node.getLocalScale().x !== 1.5) problems.push('copy scales');
    results.models = problems.length ? FAIL(problems.join('; ')) : PASS('both copies share one mesh; its material has diffuse, normal (bumpiness 0.35), metalness and gloss, ambient occlusion (aoIntensity 0.25) and emissive maps');
  }

  // Typed properties and target links, as game code reads them from the records.
  {
    const probe = at('fl_probe'), problems = [];
    const want = {charge: ['int', 7], armed: ['bool', true], label: ['string', 'probe'], note: ['string', 'free text, with a comma']};
    for (const [key, [type, value]] of Object.entries(want)) {
      if (probe.properties[key]?.type !== type || probe.values[key] !== value) problems.push(`${key} ${JSON.stringify(probe.properties[key])}`);
    }
    if (JSON.stringify(probe.values.sequence) !== '[2,3,5]' || probe.properties.sequence.type !== 'array') problems.push(`sequence ${JSON.stringify(probe.properties.sequence)}`);
    if (at('fl_light').properties.light_energy?.type !== 'float' || at('fl_light').values.light_energy !== 2.5) problems.push('light_energy');
    if (at('fl_light').properties.light_color?.type !== 'color') problems.push('light_color');
    if (probe.node.heron !== probe) problems.push('entity.heron');
    results.properties = problems.length ? FAIL(problems.join('; ')) : PASS('records keep int charge 7 (over the default 9), bool armed, array [2, 3, 5], two strings and colour light_color, also on entity.heron');
    const links = {fl_wp_a: ['fl_wp_b'], fl_probe: ['fl_wp_a'], fl_trigger: ['fl_door'], fl_button: ['fl_door']};
    const bad = Object.entries(links).filter(([from, to]) => JSON.stringify(at(from).targets.map(r => r.targetname)) !== JSON.stringify(to));
    results.links = bad.length ? FAIL(bad.map(([from]) => `${from} -> ${at(from).targets.map(r => r.targetname)}`).join('; ')) : PASS('4 links resolved to entity records: waypoint to waypoint, probe to waypoint, trigger and button to the door');
  }

  // A picture of the lab from above, lit by sky.hdr and the sun, from where the
  // three.js check takes its; and one closer, along the stations, without fog.
  view([-20, 16, -8], [2, 0, -9]);
  app.render();
  results.png = canvas.toDataURL('image/png');
  view([-14, 4.5, -1], [-4, 0.5, -16]);
  const fogType = app.scene.fog.type; app.scene.fog.type = pc.FOG_NONE;
  app.render();
  results.pngNear = canvas.toDataURL('image/png');
  app.scene.fog.type = fogType;
  view([-20, 16, -8], [2, 0, -9]);
  return results;
}

// The generated page with ?verify: its own player stands in the trigger and on the
// floor, the door opens, and the page draws baked paint clamped.
export async function checkPlaytest(h) {
  const {pc, level} = h;
  const results = {};
  const at = name => level.entities.find(record => record.targetname === name);
  const place = name => at(name).node.getPosition().clone();
  const run = seconds => { for (let t = 0; t < seconds; t += 1 / 60) h.step(1 / 60); };
  // Standing: the capsule rests on the floor, then on fl_solid's top.
  h.teleport(new pc.Vec3(-5, 0.3, 0)); run(1);
  const floor = h.feet().y;
  const solid = place('fl_solid');
  h.teleport(new pc.Vec3(solid.x, 3, solid.z)); run(1.5);
  const onSolid = h.feet().y;
  results.stand = near(floor, 0, 0.06) && near(onSolid, 2.25, 0.06)
    ? PASS(`the capsule stands on the floor (feet y=${floor.toFixed(3)}) and on fl_solid (y=${onSolid.toFixed(3)})`)
    : FAIL(`feet ${floor} on the floor, ${onSolid} on fl_solid`);
  // A wall stops it: walk at fl_hidden's side.
  const hidden = place('fl_hidden');
  h.teleport(new pc.Vec3(hidden.x - 3, 0.02, hidden.z)); h.face(-Math.PI / 2);
  h.keys.add('KeyW'); run(2); h.keys.clear();
  // The 2 m box's near wall is 1 m from its middle; the capsule's radius is 8 units.
  const stopped = h.feet().x, wall = hidden.x - 1, radius = 8 / 32;
  results.wall = Math.abs(stopped - (wall - radius)) < 0.05
    ? PASS(`walking at fl_hidden stops the player's middle at x=${stopped.toFixed(3)}, the wall at x=${wall.toFixed(2)} less the 0.25 m radius`)
    : FAIL(`walking at fl_hidden ends at x=${stopped}, the wall at ${wall}`);
  // The trigger starts 0.25 m above the floor: feet on the floor are under it.
  const door = level.animations.get(at('fl_door').name);
  door.stop(); door.update(0);
  for (const trigger of level.triggers) { trigger.wait = 0; trigger.fired = false; trigger.inside.clear(); }
  const trigger = place('fl_trigger');
  h.teleport(new pc.Vec3(trigger.x, 0.02, trigger.z)); h.face(0);
  run(1.5);
  const feet = h.feet().y;
  // The feet alone are under the volume; the middle is in it.
  const sensor = at('fl_trigger').trigger, middle = h.centre();
  const opened = door.clip.name === 'open' && door.posed && near(door.time, 1) && !sensor.contains(h.feet()) && sensor.contains(middle);
  results.trigger = opened
    ? PASS(`a player standing in fl_trigger (feet at y=${feet.toFixed(2)}, outside the volume, which starts at y=0.25; middle inside) opens the door: the page tests feet, middle and head; clip ${door.clip.name}, time ${door.time.toFixed(2)}`)
    : FAIL(`a standing player (feet y=${feet.toFixed(2)}) does not open the door: clip ${door.clip.name}, posed ${door.posed}, time ${door.time}`);
  // E at the button: look at it from in front and press.
  const button = place('fl_button');
  h.teleport(new pc.Vec3(button.x - 1.2, 0.02, button.z)); run(0.1);
  h.face(-Math.PI / 2, -Math.atan2(h.feet().y + 50 / 32 - button.y, 1.2));
  for (const t of level.triggers) t.wait = 0;
  const pressed = h.use();
  run(1.2);
  results.button = pressed && door.clip.name === 'close'
    ? PASS('E aimed at fl_button activates it and the door toggles to close') : FAIL(`pressed ${pressed?.entityName}, door ${door.clip.name}`);
  // The mover's body is kinematic: standing on it, the player is carried.
  const mover = level.animations.get(at('fl_mover').name);
  mover.stop(); mover.update(0); mover.play(mover.data.selected);
  const moverAt = place('fl_mover');
  const top = h.ray(new pc.Vec3(moverAt.x, 10, moverAt.z), new pc.Vec3(moverAt.x, -10, moverAt.z));
  h.teleport(new pc.Vec3(moverAt.x, top.point.y + 0.05, moverAt.z)); h.face(0);
  // Land and take up the platform's speed, then ride it until 0.9 s: it slides 1.8 m
  // in all and starts to turn; at 1 s a held key lifts it.
  run(0.2);
  const start = h.feet(), from = place('fl_mover');
  run(0.7);
  const carried = h.feet().sub(start), platform = place('fl_mover').sub(from);
  results.carried = Math.abs(carried.z - platform.z) < 0.1 && Math.abs(platform.z) > 0.5
    ? PASS(`standing on fl_mover, the player rides ${carried.z.toFixed(3)} m in 0.7 s while the platform moves ${platform.z.toFixed(3)} m`)
    : FAIL(`player moved ${carried}, platform ${platform}`);
  // Water: swim speed applies inside the volume.
  const pool = place('fl_water');
  h.teleport(new pc.Vec3(pool.x, pool.y - 0.5, pool.z)); run(0.2);
  const swimming = level.water(h.centre());
  results.water = swimming && swimming.data.swim_speed === 2.5 ? PASS(`the player's middle in the pool reads depth ${swimming.depth.toFixed(2)} m and swim_speed 2.5`) : FAIL('no water at the pool');
  // Baked paint clamps as the GLB samplers say.
  const wraps = at('fl_colours').meshes.filter(m => m.material?.diffuseMap).map(m => m.material.diffuseMap.addressU);
  results.clamp = wraps.length === 6 && wraps.every(w => w === pc.ADDRESS_CLAMP_TO_EDGE)
    ? PASS('the page draws all 6 fl_colours paint maps clamped') : FAIL(`fl_colours wraps ${wraps}`);
  // Back to the start, looking at the lab, for the screenshot.
  h.reset(); run(0.2);
  const eye = new pc.Vec3(-14, 6, 2), target = new pc.Vec3(2, 0, -14), d = target.clone().sub(eye);
  h.teleport(eye); h.face(Math.atan2(-d.x, -d.z), -Math.atan2(-d.y, Math.hypot(d.x, d.z)));
  return results;
}
