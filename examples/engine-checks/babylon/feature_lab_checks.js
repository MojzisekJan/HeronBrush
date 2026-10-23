// Feature lab checks that run in the browser on Babylon.js 9 with Havok: the Babylon.js
// export loaded the way a game loads it (heron_babylon.js's loadLevel: the glTF loader
// for each GLB, Havok bodies from the level JSON, the exported animation, trigger and
// water classes, sky.hdr through HDRCubeTexture), and the generated playtest page.
// verify_feature_lab.cjs serves the files and calls these.

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
// A texture's level 0 as RGBA bytes, rows in the image's own order.
async function texturePixels(texture) {
  return new Uint8Array((await texture.readPixels()).buffer);
}
function surfaces(record) {
  return record.meshes.filter(mesh => mesh.getTotalVertices() > 0).map(mesh => ({mesh, material: mesh.material}));
}
function uvSpan(mesh) {
  const uv = mesh.getVerticesData('uv');
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
// The world-space normals of the vertices a mesh's triangles use.
function drawnNormals(B, mesh) {
  const normals = mesh.getVerticesData('normal'), indices = mesh.getIndices();
  const world = mesh.computeWorldMatrix(true);
  return [...indices].map(i => B.Vector3.TransformNormal(B.Vector3.FromArray(normals, i * 3), world).normalize());
}
// The canvas as RGBA bytes, after a render.
async function capture(canvas) {
  const image = new Image(); image.src = canvas.toDataURL('image/png'); await image.decode();
  const copy = document.createElement('canvas');
  copy.width = canvas.width; copy.height = canvas.height;
  const context = copy.getContext('2d');
  context.drawImage(image, 0, 0);
  return context.getImageData(0, 0, copy.width, copy.height);
}
function average(image, cx, cy, half = 4) {
  const sum = [0, 0, 0];
  let count = 0;
  for (let y = cy - half; y <= cy + half; y++) for (let x = cx - half; x <= cx + half; x++) {
    const i = (y * image.width + x) * 4;
    for (let c = 0; c < 3; c++) sum[c] += image.data[i + c];
    count++;
  }
  return sum.map(v => v / count);
}

export async function checkBabylon(base, poses, canvas) {
  const B = window.BABYLON;
  const engine = new B.Engine(canvas, true, {preserveDrawingBuffer: true, stencil: true});
  const scene = new B.Scene(engine);
  scene.useRightHandedSystem = true;
  scene.enablePhysics(new B.Vector3(0, -9.81, 0), new B.HavokPlugin(true, await HavokPhysics()));
  // The check steps physics itself.
  scene.physicsEnabled = false;
  const physics = scene.getPhysicsEngine(), plugin = physics.getPhysicsPlugin();
  const {loadLevel} = await import(base + 'heron_babylon.js');
  const level = await loadLevel(B, scene, base + 'feature_lab.json');
  const results = {};
  const at = name => level.entities.find(record => record.targetname === name);
  const camera = new B.UniversalCamera('check', new B.Vector3(-20, 16, -8), scene);
  camera.minZ = 0.05; camera.maxZ = 2000; camera.fov = 55 * Math.PI / 180;
  camera.fovMode = B.Camera.FOVMODE_VERTICAL_FIXED;
  camera.setTarget(new B.Vector3(2, 0, -9));
  scene.activeCamera = camera;
  await scene.whenReadyAsync();
  scene.render();

  // Textures: the GLB's own images, repeating, sRGB, with the face's UV span.
  {
    const problems = [];
    let checked = 0;
    for (const {mesh, material} of surfaces(at('fl_textures'))) {
      const span = TEXTURE_SPANS[material.name];
      if (!span) continue;
      const map = material.albedoTexture;
      if (!map) { problems.push(`${material.name} has no albedoTexture`); continue; }
      if (map.wrapU !== B.Texture.WRAP_ADDRESSMODE || map.wrapV !== B.Texture.WRAP_ADDRESSMODE) problems.push(`${material.name} does not repeat`);
      // Babylon decodes sRGB in the GPU's texture format rather than in the shader.
      if (!map.getInternalTexture()._useSRGBBuffer && !map.gammaSpace) problems.push(`${material.name} map is not sRGB`);
      const got = uvSpan(mesh);
      if (!near(got[0], span[0], 1e-3) || !near(got[1], span[1], 1e-3)) problems.push(`${material.name} UV span ${got.map(v => v.toFixed(3))}, want ${span.map(v => v.toFixed(3))}`);
      const source = await sourcePixels(base + `textures/${material.name}.png`), loaded = await texturePixels(map);
      if (source.length !== loaded.length || source.some((v, i) => v !== loaded[i])) problems.push(`${material.name} pixels differ from textures/${material.name}.png`);
      checked++;
    }
    results.textures = problems.length || checked !== 6 ? FAIL(problems.join('; ') || `${checked} of 6 faces`)
      : PASS('6 glTF-loader albedo maps repeat, are sRGB textures, keep the source pixels (read back from the GPU) and the face UV spans (1, 2 by 1/2, sqrt 2 at 45 degrees, 1/2)');
  }

  // Face colours: five exact colours and the translucent paint over wood, clamped.
  {
    const problems = [], found = [];
    const wood = await sourcePixels(base + 'textures/wood.png');
    let translucent = false;
    for (const {material} of surfaces(at('fl_colours'))) {
      const map = material.albedoTexture;
      if (!map) { problems.push(`${material.name} has no map`); continue; }
      if (map.wrapU !== B.Texture.CLAMP_ADDRESSMODE || map.wrapV !== B.Texture.CLAMP_ADDRESSMODE) problems.push(`${material.name} paint does not clamp`);
      const data = await texturePixels(map);
      if (map.getSize().width === 1) { found.push([data[0], data[1], data[2]]); continue; }
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
    const maps = surfaces(at('fl_gradients')).map(({mesh, material}) => ({mesh, map: material.albedoTexture})).filter(m => m.map && m.map.getSize().width > 1);
    const problems = [];
    const images = await Promise.all(maps.map(({map}) => texturePixels(map)));
    for (const [name, stops] of Object.entries(GRADIENTS)) if (!images.some(data => stopsFound(data, stops))) problems.push(`${name} gradient not found`);
    // A gradient is one strip or disc image per set of colours, laid on the face by
    // affine UVs that run past its ends; only a clamping sampler draws the end colours there.
    let outside = 0;
    for (const {mesh, map} of maps) {
      if (map.wrapU !== B.Texture.CLAMP_ADDRESSMODE || map.wrapV !== B.Texture.CLAMP_ADDRESSMODE) problems.push(`${mesh.name} gradient repeats`);
      if (mesh.getVerticesData('uv').some(v => v < -1e-4 || v > 1 + 1e-4)) outside++;
    }
    results.gradients = problems.length ? FAIL(problems.join('; ')) : PASS(`4 gradients with all their stops in ${maps.length} images read back from the GPU, every sampler clamped (${outside} faces have UVs past the image's ends)`);
  }

  // Surface materials and emissive strength.
  {
    const got = surfaces(at('fl_surface')).map(({material}) => material);
    const problems = [];
    for (const [roughness, metallic, emission] of SURFACES) {
      const m = got.find(m => near(m.roughness, roughness, 1e-3) && near(m.metallic, metallic, 1e-3));
      if (!m) { problems.push(`no material with roughness ${roughness} metallic ${metallic}`); continue; }
      const strength = Math.max(m.emissiveColor.r, m.emissiveColor.g, m.emissiveColor.b) * m.emissiveIntensity;
      if (emission && (!near(strength, emission, 1e-3) || !m.emissiveTexture)) problems.push(`emission ${strength} (colour ${m.emissiveColor} intensity ${m.emissiveIntensity}), map ${!!m.emissiveTexture}`);
    }
    results.surface = problems.length ? FAIL(problems.join('; ')) : PASS('roughness and metallic from the GLB; KHR_materials_emissive_strength gives emission 3 with the paint as emissive texture');
  }

  // Normal maps: a 512 by 512 linear bump texture, and the tangents the shader reads.
  {
    const [{mesh, material}] = surfaces(at('fl_normal'));
    const problems = [];
    const bump = material.bumpTexture;
    if (!bump || bump.getSize().width !== 512 || bump.getSize().height !== 512) problems.push('no 512 x 512 normal map');
    // Babylon never gamma-decodes a bump texture in the shader; the GPU format must
    // not do it either.
    if (bump?.getInternalTexture()._useSRGBBuffer) problems.push('normal map is an sRGB texture');
    if (!mesh.isVerticesDataPresent('tangent')) problems.push('no TANGENT attribute');
    const effect = mesh.subMeshes[0].effect;
    if (!effect?.defines.includes('#define TANGENT')) problems.push('the drawn shader does not read tangents');
    results.normal = problems.length ? FAIL(problems.join('; ')) : PASS('bumpTexture 512 x 512, linear, the GLB TANGENT attribute, and the rendered effect is compiled with TANGENT');
  }

  // Collision: Havok rays, a dropped sphere, and the trigger as a sensor.
  const ray = (from, to, shouldHitTriggers = false) => {
    const result = new B.PhysicsRaycastResult();
    physics.raycastToRef(from, to, result, {shouldHitTriggers});
    return result.hasHit ? result : null;
  };
  const stepPhysics = seconds => { for (let t = 0; t < seconds; t += 1 / 60) scene._advancePhysicsEngineStep(1000 / 60); };
  {
    const centre = name => at(name).node.computeWorldMatrix(true) && at(name).node.getAbsolutePosition();
    const hitY = (name, triggers = false) => { const p = centre(name); return ray(new B.Vector3(p.x, 10, p.z), new B.Vector3(p.x, -10, p.z), triggers)?.hitPointWorld.y; };
    const side = (name, d) => { const p = centre(name), dir = new B.Vector3(...d); return ray(p.subtract(dir.scale(5)), p.add(dir.scale(5))); };
    const drop = name => {
      const p = centre(name);
      const ball = B.MeshBuilder.CreateSphere('probe', {diameter: 0.6}, scene);
      ball.position.set(p.x, 6, p.z);
      const aggregate = new B.PhysicsAggregate(ball, B.PhysicsShapeType.SPHERE, {mass: 1, radius: 0.3}, scene);
      stepPhysics(3);
      const bottom = ball.position.y - 0.3;
      aggregate.dispose(); ball.dispose();
      return bottom;
    };
    const solid = hitY('fl_solid'), clip = hitY('fl_clip'), skip = hitY('fl_skip'), hidden = hitY('fl_hidden'), trigger = hitY('fl_trigger');
    const r = {};
    const fallen = drop('fl_solid');
    r.solid = near(solid, 2.25, 1e-3) && near(fallen, 2.25, 0.02) && at('fl_solid').body.getMotionType() === B.PhysicsMotionType.STATIC
      ? PASS(`a static Havok body: a ray stops on fl_solid's hull top at y=${solid.toFixed(3)}, a dropped sphere settles at ${fallen.toFixed(3)}`)
      : FAIL(`fl_solid top at ${solid}, sphere at ${fallen}`);
    const clipMeshes = at('fl_clip').meshes.length;
    r.clip = clipMeshes === 0 && near(clip, 3.25, 1e-3) ? PASS(`no mesh; the hull stops a ray at y=${clip.toFixed(3)}`) : FAIL(`fl_clip draws ${clipMeshes} meshes, top at ${clip}`);
    const skipSides = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1], [0, 1, 0]].filter(d => side('fl_skip', d)).length;
    // Havok's triangles are two-sided, so the ray stops on the inside of the bottom face.
    r.skip = near(skip, 0.25, 1e-3) && skipSides === 5 ? PASS('a ray from above passes the skip top and stops on the inside of the bottom face at y=0.25; rays at the four walls and the bottom stop') : FAIL(`skip top hit at ${skip}, ${skipSides} of 5 sides hit`);
    const hiddenSides = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1], [0, -1, 0]].filter(d => side('fl_hidden', d)).length;
    const hiddenDrawn = surfaces(at('fl_hidden')).reduce((n, {mesh}) => n + mesh.getIndices().length / 3, 0);
    r.hidden = near(hidden, 2.25, 1e-3) && hiddenSides === 5 && hiddenDrawn === 10
      ? PASS('the mesh draws 10 triangles (five sides); collision keeps the hidden face: rays stop on all sides') : FAIL(`hidden top ${hidden}, ${hiddenSides} sides, ${hiddenDrawn} triangles drawn`);
    // The trigger: a Havok trigger shape that rays and bodies pass, which reports a body.
    const record = at('fl_trigger'), events = [];
    const observer = plugin.onTriggerCollisionObservable.add(event => {
      if (event.collider === record.body || event.collidedAgainst === record.body) events.push(event.type);
    });
    const throughTrigger = drop('fl_trigger');
    plugin.onTriggerCollisionObservable.remove(observer);
    const sensorTop = hitY('fl_trigger', true);
    r.trigger = near(trigger, 0, 1e-3) && record.body.shape.isTrigger && record.meshes.length === 0 && events.includes('TRIGGER_ENTERED') && near(throughTrigger, 0, 0.02)
      ? PASS(`a Havok trigger shape, drawn by nothing: a ray passes to the floor (with shouldHitTriggers it stops at y=${sensorTop?.toFixed(2)}); a sphere falls through to y=${throughTrigger.toFixed(3)} and Havok reports ${events[0]}`)
      : FAIL(JSON.stringify({trigger, isTrigger: record.body.shape.isTrigger, meshes: record.meshes.length, events, throughTrigger}));
    results.collision = r;
  }

  // Animation: the exported runtime's poses against the editor's.
  const rest = new Map();
  for (const record of level.entities) if (record.animation) rest.set(record.targetname, record.node.computeWorldMatrix(true).clone().invert());
  const posedProbe = (name, point) => B.Vector3.TransformCoordinates(B.Vector3.TransformCoordinates(B.Vector3.FromArray(point), rest.get(name)), at(name).node.computeWorldMatrix(true));
  const compare = (name, clip) => {
    const animation = at(name).animation;
    let worst = 0;
    for (const sample of poses[name]) {
      if (clip) animation.play(clip);
      animation.seek(sample.time); animation.update(0);
      sample.rest.forEach((point, i) => { worst = Math.max(worst, posedProbe(name, point).subtract(B.Vector3.FromArray(sample.posed[i])).length()); });
    }
    return worst;
  };
  {
    const mover = at('fl_mover').animation;
    const worst = compare('fl_mover');
    const body = at('fl_mover').body;
    results.animation = worst < 1e-4 && mover.data.autoplay && body.getMotionType() === B.PhysicsMotionType.ANIMATED && !body.disablePreStep
      ? PASS(`HeronAnimation matches ${poses.fl_mover.length} editor poses of fl_mover within ${worst.toExponential(1)} m (linear, hold, bezier); its Havok body is ANIMATED and follows the node`)
      : FAIL(`worst pose error ${worst} m, motion ${body.getMotionType()}`);
    // The animated body moves with the node: a ray finds the mover's top where the pose put it.
    mover.seek(2); mover.update(0); stepPhysics(1 / 60);
    const p = at('fl_mover').node.computeWorldMatrix(true) && at('fl_mover').node.getAbsolutePosition();
    const top = ray(new B.Vector3(p.x, 10, p.z), new B.Vector3(p.x, -10, p.z));
    results.animatedBody = top && top.body === body ? PASS(`after the pose at 2 s and a physics step, a ray at the mover's new place (z=${p.z.toFixed(2)}) hits its body`) : FAIL(`ray at ${p} hit ${top?.body?.transformNode?.name}`);
    mover.stop(); mover.update(0);
  }

  // Door, button and trigger: an actor walks in, the door opens to the editor's pose.
  {
    const door = at('fl_door').animation, doorPoses = compare('fl_door', 'open');
    door.stop(); door.update(0);
    const button = at('fl_button').trigger;
    const step = (actors, seconds) => { for (let t = 0; t < seconds; t += 1 / 60) level.update(1 / 60, actors); };
    const outside = [['player', new B.Vector3(-5, 1, 0)]], inside = [['player', at('fl_trigger').node.getAbsolutePosition()]];
    step(outside, 0.5);
    const before = {clip: door.clip.name, posed: door.posed};
    step(inside, 1.2);
    const opened = {clip: door.clip.name, time: door.time, playing: door.playing};
    const final = poses.fl_door.at(-1);
    const error = Math.max(...final.rest.map((p, i) => posedProbe('fl_door', p).subtract(B.Vector3.FromArray(final.posed[i])).length()));
    step(outside, 0.1); door.time = 0.5; step(inside, 0.1);
    const refired = door.time < 0.5;
    button.activate(); step([], 1.2);
    const toggled = door.clip.name;
    const ok = !before.posed && opened.clip === 'open' && !opened.playing && near(opened.time, 1) && error < 1e-4 && !refired && toggled === 'close' && doorPoses < 1e-4;
    results.door = ok ? PASS(`walking into fl_trigger plays open; after 1.2 s the door matches the editor's open pose within ${error.toExponential(1)} m; no refire inside the 1.5 s wait; fl_button toggles to close; open clip matches ${poses.fl_door.length} editor poses`)
      : FAIL(JSON.stringify({before, opened, error, refired, toggled, doorPoses}));
    door.stop(); door.update(0);
  }

  // Sky and fog: sky.hdr through HDRCubeTexture as the environment and skybox, the
  // sun's glow where the environment says the sun is; FOGMODE_EXP2 from the level.
  {
    const problems = [];
    const env = level.data.environment, texture = scene.environmentTexture;
    if (!texture || texture.getClassName() !== 'HDRCubeTexture' || !texture.isReady()) problems.push(`environment texture ${texture?.getClassName()} ready ${texture?.isReady()}`);
    const skybox = scene.getMeshByName('heron-skybox');
    const shown = skybox?.material.reflectionTexture;
    if (shown?.getClassName() !== 'HDRCubeTexture' || shown.url !== texture.url) problems.push('the skybox does not show sky.hdr');
    const fog = scene.fogColor.asArray().map(v => Math.round(v * 255));
    if (scene.fogMode !== B.Scene.FOGMODE_EXP2 || !near(scene.fogDensity, FOG.density, 1e-6)) problems.push(`fog mode ${scene.fogMode} density ${scene.fogDensity}`);
    if (fog.some((v, i) => Math.abs(v - FOG.colour[i]) > 1)) problems.push(`fog colour ${fog}`);
    // Look from the origin, with only the sky drawn: at the sun, away from it, and up.
    const hidden = scene.meshes.filter(mesh => mesh !== skybox && mesh.isEnabled());
    for (const mesh of hidden) mesh.setEnabled(false);
    const eye = new B.UniversalCamera('sky', B.Vector3.Zero(), scene);
    eye.fov = 0.5; eye.minZ = 0.1; eye.maxZ = 5000;
    scene.activeCamera = eye;
    const look = async direction => {
      eye.setTarget(B.Vector3.FromArray(direction));
      scene.render();
      return average(await capture(canvas), canvas.width >> 1, canvas.height >> 1);
    };
    const s = env.to_sun;
    const atSun = await look(s), away = await look([-s[0], s[1], -s[2]]), mirrored = await look([s[0], s[1], -s[2]]), top = await look([0.001, 1, 0]);
    const bright = c => c[0] + c[1] + c[2];
    if (!(bright(atSun) > bright(away) + 30 && bright(atSun) > bright(mirrored) + 30)) problems.push(`sun ${atSun.map(Math.round)}, opposite ${away.map(Math.round)}, mirrored in Z ${mirrored.map(Math.round)}`);
    if (!(top[2] > top[0])) problems.push(`zenith ${top.map(Math.round)} is not blue`);
    for (const mesh of hidden) mesh.setEnabled(true);
    scene.activeCamera = camera;
    results.sky = problems.length ? FAIL(problems.join('; '))
      : PASS(`HDRCubeTexture reads sky.hdr as the environment and skybox; looking along to_sun gives ${atSun.map(Math.round)}, the opposite way ${away.map(Math.round)}, mirrored in Z ${mirrored.map(Math.round)}, up ${top.map(Math.round)} (blue); FOGMODE_EXP2 density 0.02, colour ${fog}`);
  }

  // Water: the adapter's queries, its material on the GLB surface, only upward faces.
  {
    const record = at('fl_water'), water = record.water;
    let triangles = 0, up = true;
    for (const {mesh, material} of surfaces(record)) {
      const normals = drawnNormals(B, mesh);
      triangles += normals.length / 3;
      up &&= normals.every(n => n.y > 0.99);
      if (material.transparencyMode !== B.Material.MATERIAL_ALPHABLEND || !near(material.alpha, 0.55, 1e-3) || !material.heronWater) up = false;
    }
    scene.render();
    const defines = surfaces(record).map(({mesh}) => mesh.subMeshes[0].effect?.defines ?? 'no effect');
    const waves = defines.every(d => d.includes('#define HERON_WATER'));
    const p = record.node.getAbsolutePosition();
    const inside = level.water(p.clone()), above = level.water(p.add(new B.Vector3(0, 1, 0)));
    const sensor = record.body?.shape.isTrigger;
    const ok = inside && near(inside.depth, 0.75, 1e-3) && inside.data.swim_speed === 2.5 && inside.data.priority === 3 && !above && triangles === 2 && up && waves && sensor;
    results.water = ok ? PASS('HeronWater finds the centre 0.75 m under the surface (swim_speed 2.5, priority 3) and nothing above it; the GLB draws 2 upward triangles, alpha-blended at 0.55, with the wave plugin compiled in; the volume is a Havok trigger')
      : FAIL(JSON.stringify({inside: inside && {depth: inside.depth}, above: !!above, triangles, up, waves, sensor, defines: defines.map(d => d.slice(0, 200))}));
  }

  // Lights: native lights from the entities' light blocks, aimed and lighting the level.
  {
    const problems = [];
    const point = at('fl_light').light, spot = at('fl_spot').light;
    const colour = l => l.diffuse.asArray().map(v => Math.round(v * 255)).join(' ');
    if (point?.getClassName() !== 'PointLight' || !near(point.intensity, 2.5) || !near(point.range, 8) || colour(point) !== '255 200 120') problems.push(`point ${point?.getClassName()} ${point?.intensity} ${point?.range} ${point && colour(point)}`);
    if (spot?.getClassName() !== 'SpotLight' || !near(spot.angle, 60 * Math.PI / 180) || !near(spot.range, 10) || !near(spot.intensity, 3)) problems.push(`spot ${spot?.getClassName()} angle ${spot?.angle} range ${spot?.range}`);
    // The spot shines down its entity's -Z, turned by the entity's rotation_yxz.
    spot.computeTransformedInformation();
    const r = at('fl_spot').entity.transform.rotation_yxz;
    const want = new B.Vector3(0, 0, -1).applyRotationQuaternion(B.Quaternion.RotationAxis(B.Axis.Y, r[1]).multiply(B.Quaternion.RotationAxis(B.Axis.X, r[0])).multiply(B.Quaternion.RotationAxis(B.Axis.Z, r[2])));
    const got = spot.transformedDirection.normalizeToNew(), position = spot.getAbsolutePosition();
    if (got.subtract(want).length() > 1e-4) problems.push(`spot direction ${got}, want ${want}`);
    if (position.subtract(B.Vector3.FromArray(at('fl_spot').entity.transform.translation)).length() > 1e-4) problems.push(`spot at ${position}`);
    // Each light brightens a white card where it shines: the middle of a view of the
    // card's lit side, with the light on and off. The feature lab's spot points 60
    // degrees up (its exported pitch, as every target turns it), at the sky.
    const card = B.MeshBuilder.CreatePlane('card', {size: 2, sideOrientation: B.Mesh.DOUBLESIDE}, scene);
    card.material = new B.PBRMaterial('card', scene);
    card.material.albedoColor = new B.Color3(0.8, 0.8, 0.8); card.material.metallic = 0; card.material.roughness = 1;
    const lit = async (light, from, along) => {
      card.position = from.add(along.scale(2));
      card.lookAt(from);
      const side = B.Vector3.Cross(along, Math.abs(along.y) > 0.9 ? B.Axis.X : B.Axis.Y).normalize();
      const eye = new B.UniversalCamera('lit', from.add(along.scale(1)).add(side.scale(0.4)), scene);
      eye.setTarget(card.position); eye.minZ = 0.05; scene.activeCamera = eye;
      await scene.whenReadyAsync();
      const fogMode = scene.fogMode; scene.fogMode = B.Scene.FOGMODE_NONE;
      scene.render(); const on = average(await capture(canvas), canvas.width >> 1, canvas.height >> 1);
      light.setEnabled(false); scene.render(); const off = average(await capture(canvas), canvas.width >> 1, canvas.height >> 1);
      light.setEnabled(true); scene.fogMode = fogMode; scene.activeCamera = camera; eye.dispose();
      return [on, off].map(c => c.reduce((a, b) => a + b));
    };
    const [pointOn, pointOff] = await lit(point, point.getAbsolutePosition(), new B.Vector3(0, -1, 0));
    const [spotOn, spotOff] = await lit(spot, position, got);
    card.material.dispose(); card.dispose();
    if (!(pointOn > pointOff + 15)) problems.push(`the point light does not light a card 2 m below it: ${pointOn} on, ${pointOff} off`);
    if (!(spotOn > spotOff + 15)) problems.push(`the spot does not light where it points: ${spotOn} on, ${spotOff} off`);
    results.lights = problems.length ? FAIL(problems.join('; '))
      : PASS(`PointLight (255 200 120, intensity 2.5, range 8) and SpotLight (60 degree cone, range 10) under their entities; the spot points down the turned -Z within 1e-4; each brightens a card 2 m from it on its axis (sums ${Math.round(pointOff)} to ${Math.round(pointOn)}, ${Math.round(spotOff)} to ${Math.round(spotOn)})`);
  }

  // Imported models: two copies of one GLB, its PBR maps.
  {
    const a = at('fl_model_a'), b = at('fl_model_b');
    const problems = [];
    const [ma, mb] = [surfaces(a), surfaces(b)];
    if (!ma.length || ma.length !== mb.length || ma.some(({mesh}, i) => mesh.geometry !== mb[i].mesh.geometry)) problems.push('copies do not share geometry');
    for (const {material} of ma) {
      for (const slot of ['albedoTexture', 'bumpTexture', 'metallicTexture', 'ambientTexture', 'emissiveTexture']) if (!material[slot]) problems.push(`${material.name} lacks ${slot}`);
      if (!near(material.bumpTexture?.level, 0.35, 1e-3)) problems.push(`normal scale ${material.bumpTexture?.level}`);
      if (!near(material.ambientTextureStrength, 0.25, 1e-3)) problems.push(`occlusion strength ${material.ambientTextureStrength}`);
    }
    if (b.node.scaling.x / a.node.scaling.x !== 1.5) problems.push('copy scales');
    results.models = problems.length ? FAIL(problems.join('; ')) : PASS('both copies share one geometry; its material has albedo, bump (level 0.35), metallic-roughness, ambient occlusion (0.25) and emissive textures');
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
    if (probe.node.metadata.heron !== probe) problems.push('node metadata');
    results.properties = problems.length ? FAIL(problems.join('; ')) : PASS('records keep int charge 7 (over the default 9), bool armed, array [2, 3, 5], two strings and colour light_color, also on node.metadata.heron');
    const links = {fl_wp_a: ['fl_wp_b'], fl_probe: ['fl_wp_a'], fl_trigger: ['fl_door'], fl_button: ['fl_door']};
    const bad = Object.entries(links).filter(([from, to]) => JSON.stringify(at(from).targets.map(r => r.targetname)) !== JSON.stringify(to));
    results.links = bad.length ? FAIL(bad.map(([from]) => `${from} -> ${at(from).targets.map(r => r.targetname)}`).join('; ')) : PASS('4 links resolved to entity records: waypoint to waypoint, probe to waypoint, trigger and button to the door');
  }

  // A picture of the lab from above, lit by sky.hdr and the sun, from where the
  // three.js check takes its; and one closer, along the stations, without fog.
  scene.activeCamera = camera;
  scene.render();
  results.png = canvas.toDataURL('image/png');
  const closer = new B.UniversalCamera('near', new B.Vector3(-14, 4.5, -1), scene);
  closer.minZ = 0.05; closer.fov = 55 * Math.PI / 180; closer.fovMode = B.Camera.FOVMODE_VERTICAL_FIXED;
  closer.setTarget(new B.Vector3(-4, 0.5, -16));
  scene.activeCamera = closer;
  const fogMode = scene.fogMode; scene.fogMode = B.Scene.FOGMODE_NONE;
  scene.render();
  results.pngNear = canvas.toDataURL('image/png');
  scene.fogMode = fogMode; scene.activeCamera = camera;
  return results;
}

// The generated page with ?verify: its own player stands in the trigger and on the
// floor, the door opens, and the page draws baked paint clamped.
export async function checkPlaytest(h) {
  const {B, level} = h;
  const results = {};
  const at = name => level.entities.find(record => record.targetname === name);
  const place = name => at(name).node.computeWorldMatrix(true) && at(name).node.getAbsolutePosition().clone();
  // Each step stands for a rendered frame, as the page steps once a frame: the
  // character controller reads a moving floor's velocity between frame numbers.
  const run = seconds => { for (let t = 0; t < seconds; t += 1 / 60) { h.scene._frameId++; h.step(1 / 60); } };
  // Standing: the controller rests on the floor, then on fl_solid's top.
  h.teleport(new B.Vector3(-5, 0.3, 0)); run(1);
  const floor = h.feet().y;
  const solid = place('fl_solid');
  h.teleport(new B.Vector3(solid.x, 3, solid.z)); run(1.5);
  const onSolid = h.feet().y;
  results.stand = near(floor, 0, 0.06) && near(onSolid, 2.25, 0.06)
    ? PASS(`the character controller stands on the floor (feet y=${floor.toFixed(3)}) and on fl_solid (y=${onSolid.toFixed(3)})`)
    : FAIL(`feet ${floor} on the floor, ${onSolid} on fl_solid`);
  // A wall stops it: walk at fl_hidden's side.
  const hidden = place('fl_hidden');
  h.teleport(new B.Vector3(hidden.x - 3, 0.02, hidden.z)); h.face(-Math.PI / 2);
  h.keys.add('KeyW'); run(2); h.keys.clear();
  const stopped = h.feet().x;
  results.wall = stopped < hidden.x - 1.25 && stopped > hidden.x - 1.75
    ? PASS(`walking at fl_hidden stops the player at x=${stopped.toFixed(2)}, the wall at ${(hidden.x - 1.25).toFixed(2)} less the radius`)
    : FAIL(`walking at fl_hidden ends at x=${stopped}`);
  // The trigger starts 0.25 m above the floor: feet on the floor are under it.
  const door = level.animations.get(at('fl_door').name);
  door.stop(); door.update(0);
  for (const trigger of level.triggers) { trigger.wait = 0; trigger.fired = false; trigger.inside.clear(); }
  const trigger = place('fl_trigger');
  h.teleport(new B.Vector3(trigger.x, 0.02, trigger.z)); h.face(0);
  run(1.5);
  const feet = h.feet().y;
  const opened = door.clip.name === 'open' && door.posed && near(door.time, 1);
  results.trigger = opened
    ? PASS(`a player standing in fl_trigger (feet at y=${feet.toFixed(2)}, the volume from y=0.25) opens the door: the page tests feet, middle and head; clip ${door.clip.name}, time ${door.time.toFixed(2)}`)
    : FAIL(`a standing player (feet y=${feet.toFixed(2)}) does not open the door: clip ${door.clip.name}, posed ${door.posed}, time ${door.time}`);
  // E at the button: look at it from in front and press.
  const button = place('fl_button');
  h.teleport(new B.Vector3(button.x - 1.2, 0.02, button.z)); run(0.1);
  h.face(-Math.PI / 2, -Math.atan2(h.feet().y + 50 / 32 - button.y, 1.2));
  for (const t of level.triggers) t.wait = 0;
  const pressed = h.use();
  run(1.2);
  results.button = pressed && door.clip.name === 'close'
    ? PASS('E aimed at fl_button activates it and the door toggles to close') : FAIL(`pressed ${pressed?.entityName}, door ${door.clip.name}`);
  // The door's body is animated: standing on the mover is carried.
  const mover = level.animations.get(at('fl_mover').name);
  mover.stop(); mover.update(0); mover.play(mover.data.selected);
  const moverAt = place('fl_mover');
  const top = new B.PhysicsRaycastResult();
  h.scene.getPhysicsEngine().raycastToRef(new B.Vector3(moverAt.x, 10, moverAt.z), new B.Vector3(moverAt.x, -10, moverAt.z), top);
  h.teleport(new B.Vector3(moverAt.x, top.hitPointWorld.y + 0.05, moverAt.z)); h.face(0);
  // Land and take up the platform's speed, then ride it until 0.9 s: it slides 1.8 m
  // in all and starts to turn; at 1 s a held key lifts it.
  run(0.2);
  const start = h.feet(), from = place('fl_mover');
  run(0.7);
  const carried = h.feet().subtract(start), platform = place('fl_mover').subtract(from);
  results.carried = Math.abs(carried.z - platform.z) < 0.1 && Math.abs(platform.z) > 0.5
    ? PASS(`standing on fl_mover, the player rides ${carried.z.toFixed(3)} m in 0.7 s while the platform moves ${platform.z.toFixed(3)} m`)
    : FAIL(`player moved ${carried}, platform ${platform}`);
  // Water: swim speed applies inside the volume.
  const pool = place('fl_water');
  h.teleport(new B.Vector3(pool.x, pool.y - 0.5, pool.z)); run(0.2);
  const swimming = level.water(h.controller.getPosition());
  results.water = swimming && swimming.data.swim_speed === 2.5 ? PASS(`the player's middle in the pool reads depth ${swimming.depth.toFixed(2)} m and swim_speed 2.5`) : FAIL('no water at the pool');
  // Baked paint clamps as the GLB samplers say.
  const wraps = at('fl_colours').meshes.filter(m => m.material?.albedoTexture).map(m => m.material.albedoTexture.wrapU);
  results.clamp = wraps.length === 6 && wraps.every(w => w === B.Texture.CLAMP_ADDRESSMODE)
    ? PASS('the page draws all 6 fl_colours paint maps clamped') : FAIL(`fl_colours wraps ${wraps}`);
  // Back to the start, looking at the lab, for the screenshot.
  h.reset(); run(0.2);
  const eye = new B.Vector3(-14, 6, 2), target = new B.Vector3(2, 0, -14), d = target.subtract(eye);
  h.teleport(eye); h.face(Math.atan2(-d.x, -d.z), -Math.atan2(-d.y, Math.hypot(d.x, d.z)));
  return results;
}
