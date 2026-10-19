// Runtime for the validated _heron_animation payload. Supply Three.js and the
// exported entity root. Collision adapters must use that same root's pose.
export class HeronAnimation {
  constructor(THREE, json, object) {
    this.THREE = THREE;
    this.data = typeof json === 'string' ? JSON.parse(json) : json;
    const d = this.data;
    if (d.version !== 1 || !Number.isFinite(d.scale) || d.scale <= 0 ||
        !d.clips?.length || d.clips.some(c => c.channels.length !== 6 || !Number.isFinite(c.duration) || c.duration <= 0)) {
      throw new Error('Invalid animation payload');
    }
    this.clip = d.clips.find(c => c.name === d.selected);
    if (!this.clip) throw new Error('Missing selected clip');
    this.object = object;
    this.rest = {position: object.position.clone(), rotation: object.quaternion.clone(), scale: object.scale.clone()};
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
    const c = this.clip;
    if (this.playing) {
      this.time += dt;
      if (c.mode === 'once' && this.time >= c.duration) { this.time = c.duration; this.playing = false; }
      else if (c.mode !== 'once') this.time %= c.duration * (c.mode === 'pingpong' ? 2 : 1);
    }
    const o = this.object, r = this.rest, T = this.THREE;
    if (!this.posed) { o.position.copy(r.position); o.quaternion.copy(r.rotation); o.scale.copy(r.scale); return; }
    let time = c.mode === 'once' ? Math.min(this.time, c.duration) : this.time % (c.duration * (c.mode === 'pingpong' ? 2 : 1));
    if (time > c.duration) time = 2 * c.duration - time;
    const v = c.channels.map(channel => sample(channel.keys, time));
    const radians = Math.PI / 180;
    const rotation = new T.Quaternion().setFromEuler(new T.Euler(v[4] * radians, v[5] * radians, v[3] * radians, 'YXZ'));
    const p = this.data.pivot, scale = this.data.scale;
    const pivot = new T.Vector3(p[1], p[2], p[0]).divideScalar(scale);
    o.position.copy(r.position).sub(pivot).applyQuaternion(rotation).add(pivot)
      .add(new T.Vector3(v[1], v[2], v[0]).divideScalar(scale));
    o.quaternion.copy(rotation).multiply(r.rotation);
    o.scale.copy(r.scale);
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

export class HeronAnimationTrigger {
  constructor(json, object) {
    this.data = typeof json === 'string' ? JSON.parse(json) : json;
    this.object = object; this.inside = new Set(); this.wait = 0; this.fired = false; this.pending = false;
  }
  activate() { this.pending = true; }
  update(dt, actors, animations) {
    this.wait = Math.max(0, this.wait - dt);
    this.object.updateWorldMatrix(true, false);
    const inside = new Set(), d = this.data;
    for (const [id, point] of actors) {
      const p = this.object.worldToLocal(point.clone()), o = d.origin;
      if (d.volumes.some(planes => planes.length && planes.every(v =>
        v[0]*(p.x+o[0])+v[1]*(p.y+o[1])+v[2]*(p.z+o[2]) <= v[3]+1e-5))) inside.add(id);
    }
    const entered = [...inside].some(id => !this.inside.has(id));
    this.inside = inside;
    const requested = this.pending || entered; this.pending = false;
    if (!requested || this.wait > 0 || (d.once && this.fired)) return;
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
  }
}
