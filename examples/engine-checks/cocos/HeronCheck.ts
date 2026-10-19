// Test bootstrap: loads an exported level into whatever scene starts, without editing
// the scene, then drops a ball, walks into a button and reports what happened.
import { BoxCollider, director, Director, Node, Camera, Vec3, Quat, MeshRenderer, RigidBody, SphereCollider, ERigidBodyType, PhysicsSystem, geometry, MeshCollider, utils, primitives } from 'cc';
import { HeronBrushLevel } from '../resources/levels/HeronBrushLevel';

const params = new URLSearchParams(location.search);
director.once(Director.EVENT_AFTER_SCENE_LAUNCH, async () => {
    const scene = director.getScene()!;
    const name = params.get('level') || 'motion_lab';
    for (const child of [...scene.children]) if (child.name !== 'Main Light' && !child.getComponent(Camera) && !child.getComponentInChildren(Camera)) child.active = false;
    const node = new Node('HeronLevel');
    scene.addChild(node);
    const level = node.addComponent(HeronBrushLevel);
    level.level = 'levels/' + name;
    const walker = new Node('walker');
    scene.addChild(walker);
    walker.setWorldPosition(0, -1000, 0);
    level.activators = [walker];
    const only = params.get('colliders');
    if (only) { const add = (level as any).addCollider.bind(level); (level as any).addCollider = (n: Node, e: any) => { if (only.split(',').includes(e.collision.sensor ? 'sensor' : e.collision.kind)) add(n, e); }; }
    if (params.get('notriggers')) (level as any).triggers = { push() {} };
    if (params.get('nomodels')) (level as any).attachModel = async () => {};
    if (params.get('noenv')) level.environment = false;
    level.ready = level.build(level.level);
    try { await level.ready; } catch (e) { console.log('HERON build failed ' + e); }
    const meshes = node.getComponentsInChildren(MeshRenderer).length;
    const colliders = node.getComponentsInChildren(MeshCollider);
    console.log(`HERON entities=${level.entities.size} meshes=${meshes} colliders=${colliders.length} triggers=${colliders.filter(c => c.isTrigger).length} kinematic=${node.getComponentsInChildren(RigidBody).length} animations=${level.animations.size}`);
    if (params.get('only_world')) for (const c of colliders) if (c.node.name !== 'worldspawn') { c.node.getComponent(RigidBody)?.destroy(); c.destroy(); }
    const probe = (label: string) => {
        const ray = new geometry.Ray(1, 5, 11, 0, -1, 0);
        const hit = PhysicsSystem.instance.raycastClosest(ray);
        console.log(`HERON probe ${label} hit=${hit ? PhysicsSystem.instance.raycastClosestResult.collider.node.name + '@' + PhysicsSystem.instance.raycastClosestResult.hitPoint.y.toFixed(3) : 'none'} worldspawn=${level.entities.get('worldspawn')!.worldPosition} scale=${level.entities.get('worldspawn')!.worldScale}`);
    };
    if (params.get('box')) { const b = new Node('testbox'); scene.addChild(b); b.setWorldPosition(1, -0.5, 11); const bc = b.addComponent(BoxCollider); bc.size = new Vec3(4, 1, 4); }
    if (params.get('boxmesh')) { const b = new Node('testmesh'); scene.addChild(b); b.setWorldPosition(1, -0.5, 11); const mc = b.addComponent(MeshCollider); mc.mesh = utils.MeshUtils.createMesh(primitives.box({ width: 4, height: 1, length: 4 })); }
    if (params.get('ourmesh')) {
        const pts = level.records.get('worldspawn')!.collision!.triangles!;
        const b = new Node('ourmesh'); scene.addChild(b);
        const mc = b.addComponent(MeshCollider);
        const m = utils.MeshUtils.createMesh({ positions: pts, indices: [...Array(pts.length / 3).keys()] });
        console.log(`HERON ourmesh verts=${pts.length / 3} sub=${m.renderingSubMeshes.length} geo=${m.renderingSubMeshes[0].geometricInfo.positions.length} idx=${m.renderingSubMeshes[0].geometricInfo.indices?.length} mode=${m.renderingSubMeshes[0].primitiveMode}`);
        mc.mesh = m;
    }
    let frames = 0; const t0 = performance.now();
    director.on(Director.EVENT_AFTER_UPDATE, () => { frames++; });
    setInterval(() => console.log(`HERON fps=${(frames / ((performance.now() - t0) / 1000)).toFixed(1)}`), 2000);
    probe('built');
    setTimeout(() => probe('100ms'), 100);
    setTimeout(() => probe('1s'), 1000);
    for (const [n, a] of level.animations) console.log(`HERON anim ${n} autoplay=${a.data.autoplay} playing=${a.playing}`);
    for (const c of colliders) console.log(`HERON collider ${c.node.name} convex=${c.convex} trigger=${c.isTrigger} verts=${c.mesh ? c.mesh.struct.vertexBundles[0].view.count : -1} rb=${!!c.node.getComponent(RigidBody)}`);
    setTimeout(() => { for (const t of (level as any).triggers) console.log(`HERON trigger ${t.node.name} fired=${t.fired} inside=${t.inside.size}`); }, 1500);
    const camera = scene.getComponentInChildren(Camera)!;
    const view = (params.get('view') || '0,16,24,-35,0').split(',').map(Number);
    camera.node.setWorldPosition(new Vec3(view[0], view[1], view[2]));
    // Yaw about the world's Y, then pitch about the turned X, so `view` is an aim.
    const aim = new Quat(); Quat.multiply(aim, Quat.fromEuler(new Quat(), 0, view[4], 0), Quat.fromEuler(new Quat(), view[3], 0, 0));
    camera.node.setWorldRotation(aim);
    camera.far = 2000;
    const ballAt = (params.get('ball') || '0,3,11').split(',').map(Number);
    const ball = new Node('ball');
    scene.addChild(ball);
    ball.setWorldPosition(ballAt[0], ballAt[1], ballAt[2]);
    const sphere = ball.addComponent(SphereCollider); sphere.radius = 0.25;
    const body = ball.addComponent(RigidBody); body.type = ERigidBodyType.DYNAMIC; body.mass = 1;
    const trigger = params.get('trigger'), target = params.get('target');
    let start: number[] = [];
    setTimeout(() => {
        console.log(`HERON ball y=${ball.worldPosition.y.toFixed(3)}`);
        const ray = new geometry.Ray(ballAt[0] + 1, 5, ballAt[2], 0, -1, 0);
        if (PhysicsSystem.instance.raycastClosest(ray)) {
            const hit = PhysicsSystem.instance.raycastClosestResult;
            console.log(`HERON ray hit=${hit.collider.node.name} y=${hit.hitPoint.y.toFixed(3)}`);
        }
        if (trigger && target) {
            const m = level.entities.get(target)!;
            start = [m.position.x, m.position.y, m.position.z];
            console.log(`HERON before ${target} playing=${level.animations.get(target)!.playing}`);
            walker.setWorldPosition(level.entities.get(trigger)!.worldPosition.clone().add3f(0, 0.05, 0));
        }
    }, 3000);
    setTimeout(() => {
        if (trigger && target) {
            const m = level.entities.get(target)!, a = level.animations.get(target)!;
            const r = m.rotation;
            console.log(`HERON ${target} clip=${a.clip.name} start=${start.map(v => v.toFixed(3))} now=${[m.position.x, m.position.y, m.position.z].map(v => v.toFixed(3))} rot=${[r.x, r.y, r.z, r.w].map(v => v.toFixed(4))}`);
        }
        console.log('HERON done');
    }, 7000);
});
