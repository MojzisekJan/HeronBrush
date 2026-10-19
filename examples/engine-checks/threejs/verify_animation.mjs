// Node runtime check against the poses computed by editor_core's animation
// regression. Arguments: local three.core.js, portable-animation fixture folder.
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
import path from 'node:path';

const THREE = await import(pathToFileURL(path.resolve(process.argv[2])));
const fixtures = process.argv[3] || '/tmp/heron-portable-animation-check';
const runtime = new URL('../../../crates/scene_export/src/writer/three_animation.js', import.meta.url);
const source = await readFile(runtime, 'utf8');
const {HeronAnimation, HeronAnimationTrigger} = await import('data:text/javascript;base64,' + Buffer.from(source).toString('base64'));
const data = JSON.parse(await readFile(path.join(fixtures, 'data.json'), 'utf8'));
const expected = JSON.parse(await readFile(path.join(fixtures, 'expected.json'), 'utf8'));
const object = new THREE.Object3D();
object.position.set(3, 4, 2);
const animation = new HeronAnimation(THREE, data, object);
assert.equal(animation.playing, false);
function near(actual, expected, label) {
  assert.ok(actual.distanceTo(new THREE.Vector3(...expected)) < 1e-6, `${label}: ${actual.toArray()} versus ${expected}`);
}
for (const pose of expected) {
  assert.ok(animation.play(pose.mode));
  animation.seek(pose.time);
  animation.update(0);
  near(object.position, pose.position, `${pose.mode} at ${pose.time}`);
  for (let axis = 0; axis < 3; axis++) {
    near(new THREE.Vector3().setComponent(axis, 1).applyQuaternion(object.quaternion), pose.axes[axis], `axis ${axis}`);
  }
}
animation.stop(); animation.update(0);
near(object.position, [3,4,2], 'restored rest pose');
animation.play('once'); animation.update(100);
assert.equal(animation.playing, false);
assert.equal(animation.time, animation.clip.duration);
animation.play('loop'); animation.pause(); animation.update(1);
assert.equal(animation.time, 0);
animation.resume(); animation.update(0.25);
assert.equal(animation.time, 0.25);
assert.equal(animation.play('missing'), false);
const root = new THREE.Object3D();
root.position.set(10, 0, 0); root.rotation.y = Math.PI / 2; root.scale.set(2, 1, 3);
const trigger = new HeronAnimationTrigger({origin:[10,0,0], volumes:[[[1,0,0,11],[-1,0,0,-9],[0,1,0,1],[0,-1,0,1],[0,0,1,1],[0,0,-1,1]]], targets:['platform'], action:'play', clip:'once', once:true, wait:0}, root);
const animations = new Map([['platform', animation]]);
animation.stop();
trigger.update(0, new Map([['feet', new THREE.Vector3(10,0,0)]]), animations);
assert.equal(animation.playing, true, 'entry into transformed exported volume must activate');
animation.stop();
trigger.update(1, new Map(), animations);
trigger.update(1, new Map([['feet', new THREE.Vector3(10,0,0)]]), animations);
assert.equal(animation.playing, false, 'once trigger must not fire twice');
console.log(`HERON_THREE_ANIMATION_OK: ${expected.length} editor poses, playback controls, transformed trigger and once behavior`);
