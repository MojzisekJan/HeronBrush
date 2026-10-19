// A world-space triangle soup and the little bit of maths a first-person
// walker needs against it: push a capsule out, and measure the drop to the
// floor. Ported from the raylib example's collision.c.
//
// heron_brush already did the expensive part: the `collision.triangles` in the level
// JSON are the reduced mesh (buried faces gone, neighbours joined), so the
// arena is a few hundred triangles and brute force per frame is nothing.

import * as THREE from "three";

const _ab = new THREE.Vector3();
const _ac = new THREE.Vector3();
const _ap = new THREE.Vector3();
const _bp = new THREE.Vector3();
const _cp = new THREE.Vector3();
const _n = new THREE.Vector3();
const _d = new THREE.Vector3();
const _closest = new THREE.Vector3();
const _seg = new THREE.Vector3();

// Closest point on triangle abc to p (Ericson, Real-Time Collision Detection).
function closestOnTriangle(p, a, b, c, out) {
  _ab.subVectors(b, a);
  _ac.subVectors(c, a);
  _ap.subVectors(p, a);
  const d1 = _ab.dot(_ap);
  const d2 = _ac.dot(_ap);
  if (d1 <= 0 && d2 <= 0) return out.copy(a);

  _bp.subVectors(p, b);
  const d3 = _ab.dot(_bp);
  const d4 = _ac.dot(_bp);
  if (d3 >= 0 && d4 <= d3) return out.copy(b);

  const vc = d1 * d4 - d3 * d2;
  if (vc <= 0 && d1 >= 0 && d3 <= 0) {
    const v = d1 / (d1 - d3);
    return out.copy(a).addScaledVector(_ab, v);
  }

  _cp.subVectors(p, c);
  const d5 = _ab.dot(_cp);
  const d6 = _ac.dot(_cp);
  if (d6 >= 0 && d5 <= d6) return out.copy(c);

  const vb = d5 * d2 - d1 * d6;
  if (vb <= 0 && d2 >= 0 && d6 <= 0) {
    const w = d2 / (d2 - d6);
    return out.copy(a).addScaledVector(_ac, w);
  }

  const va = d3 * d6 - d5 * d4;
  if (va <= 0 && d4 - d3 >= 0 && d5 - d6 >= 0) {
    const w = (d4 - d3) / (d4 - d3 + (d5 - d6));
    return out.copy(b).addScaledVector(_cp.subVectors(c, b), w);
  }

  const denom = 1 / (va + vb + vc);
  const v = vb * denom;
  const w = vc * denom;
  return out.copy(a).addScaledVector(_ab, v).addScaledVector(_ac, w);
}

// Closest point on segment ab to p.
function closestOnSegment(p, a, b, out) {
  _seg.subVectors(b, a);
  let t = _seg.dot(_d.subVectors(p, a)) / (_seg.dot(_seg) + 1e-9);
  t = Math.min(1, Math.max(0, t));
  return out.copy(a).addScaledVector(_seg, t);
}

export class Collider {
  /**
   * @param {Array<{tris:number[], offset:THREE.Vector3}>} parts - one entry per
   *   solid entity: its flat triangle list (entity-local metres) and world offset.
   */
  constructor(parts) {
    this.setParts(parts);
  }
  setParts(parts) {
    let rebuild=!this._parts || parts.length!==this._parts.length;
    if(!rebuild)for(let i=0;i<parts.length;i++)if(parts[i]!==this._parts[i] || parts[i].tris.length!==this._ranges[i].length){rebuild=true;break;}
    if(rebuild){
      this._parts=parts.slice();this._ranges=[];this.tris=[];
      for(const part of parts){
        const start=this.tris.length;
        for(let i=0;i+8<part.tris.length;i+=9)this.tris.push({a:new THREE.Vector3(),b:new THREE.Vector3(),c:new THREE.Vector3(),normal:new THREE.Vector3(),min:new THREE.Vector3(),max:new THREE.Vector3(),object:part.object});
        this._ranges.push({start,length:part.tris.length});
      }
    }
    for (let partIndex=0;partIndex<parts.length;partIndex++) {
      const part=parts[partIndex],range=this._ranges[partIndex];
      const t = part.tris;
      const o = part.offset;
      if(part.revision!==undefined && range.revision===part.revision && range.buffer===t && range.object===part.object && range.x===o.x && range.y===o.y && range.z===o.z)continue;
      for (let i = 0; i + 8 < t.length; i += 9) {
        const triangle=this.tris[range.start+i/9];
        const {a,b,c,normal,min,max}=triangle;
        a.set(t[i] + o.x, t[i + 1] + o.y, t[i + 2] + o.z);
        b.set(t[i + 3] + o.x, t[i + 4] + o.y, t[i + 5] + o.z);
        c.set(t[i + 6] + o.x, t[i + 7] + o.y, t[i + 8] + o.z);
        normal.subVectors(b,a).cross(_ac.subVectors(c,a)).normalize();
        min.set(
          Math.min(a.x, b.x, c.x),
          Math.min(a.y, b.y, c.y),
          Math.min(a.z, b.z, c.z),
        );
        max.set(
          Math.max(a.x, b.x, c.x),
          Math.max(a.y, b.y, c.y),
          Math.max(a.z, b.z, c.z),
        );
        triangle.object=part.object;
      }
      range.revision=part.revision;range.buffer=t;range.object=part.object;range.x=o.x;range.y=o.y;range.z=o.z;
    }
  }

  // Push a vertical capsule (radius r, half-height h, centre `centre`) out of
  // anything it overlaps. Removes the into-surface part of `vel`. Returns true
  // when it ends up resting on something near-level.
  resolveCapsule(centre, r, h, vel) {
    let grounded = false;
    const half = Math.max(0, h - r);

    for (let iter = 0; iter < 5; iter++) {
      const s0y = centre.y - half;
      const s1y = centre.y + half;
      const loX = centre.x - r, loY = s0y - r, loZ = centre.z - r;
      const hiX = centre.x + r, hiY = s1y + r, hiZ = centre.z + r;
      let moved = false;

      for (const t of this.tris) {
        if (
          t.max.x < loX || t.min.x > hiX ||
          t.max.y < loY || t.min.y > hiY ||
          t.max.z < loZ || t.min.z > hiZ
        )
          continue;

        const seedY = Math.min(s1y, Math.max(s0y, centre.y));
        _closest.set(centre.x, seedY, centre.z);
        let pt = closestOnTriangle(_closest, t.a, t.b, t.c, new THREE.Vector3());
        let ps = closestOnSegment(pt, _n.set(centre.x, s0y, centre.z), _d.set(centre.x, s1y, centre.z), new THREE.Vector3());
        pt = closestOnTriangle(ps, t.a, t.b, t.c, pt);
        ps = closestOnSegment(pt, _n.set(centre.x, s0y, centre.z), _d.set(centre.x, s1y, centre.z), ps);

        const delta = ps.sub(pt);
        const dist = delta.length();
        let nx, ny, nz;
        if (dist > 1e-4) {
          nx = delta.x / dist; ny = delta.y / dist; nz = delta.z / dist;
        } else {
          nx = t.normal.x; ny = t.normal.y; nz = t.normal.z;
        }
        if (dist >= r) continue;

        const push = r - dist + 0.001;
        centre.x += nx * push;
        centre.y += ny * push;
        centre.z += nz * push;

        const vn = vel.x * nx + vel.y * ny + vel.z * nz;
        if (vn < 0) {
          vel.x -= vn * nx;
          vel.y -= vn * ny;
          vel.z -= vn * nz;
        }
        if (ny > 0.5) grounded = true;
        moved = true;
      }
      if (!moved) break;
    }
    return grounded;
  }

  // Distance straight down from `from` to the nearest near-horizontal triangle
  // within `maxDist`, or -1. Used for ground snapping and step-ups.
  floorBelow(from, maxDist) {
    let best = -1;
    this.support = null;
    for (const t of this.tris) {
      if (Math.abs(t.normal.y) <= 0.5) continue;
      if (from.x < t.min.x || from.x > t.max.x || from.z < t.min.z || from.z > t.max.z)
        continue;

      _ab.subVectors(t.b, t.a);
      _ac.subVectors(t.c, t.a);
      // Ray (0,-1,0) vs triangle, Moller-Trumbore.
      _n.set(-_ac.z, 0, _ac.x); // cross((0,-1,0), e2)
      const det = _ab.dot(_n);
      if (Math.abs(det) < 1e-6) continue;
      const inv = 1 / det;
      _ap.subVectors(from, t.a);
      const u = _ap.dot(_n) * inv;
      if (u < -1e-4 || u > 1.0001) continue;
      _d.crossVectors(_ap, _ab);
      const v = -_d.y * inv;
      if (v < -1e-4 || u + v > 1.0001) continue;
      const dist = _ac.dot(_d) * inv;
      if (dist >= -0.02 && dist <= maxDist && (best < 0 || dist < best)) {
        best = dist;
        this.support = t.object;
      }
    }
    return best;
  }

  // An arbitrary ray vs the soup, for shooting or line-of-sight. Distance
  // along `dir` (unit) or -1.
  rayHit(origin, dir, maxDist) {
    let best = -1;
    for (const t of this.tris) {
      _ab.subVectors(t.b, t.a);
      _ac.subVectors(t.c, t.a);
      _n.crossVectors(_d.copy(dir), _ac);
      const det = _ab.dot(_n);
      if (Math.abs(det) < 1e-8) continue;
      const inv = 1 / det;
      _ap.subVectors(origin, t.a);
      const u = _ap.dot(_n) * inv;
      if (u < 0 || u > 1) continue;
      _cp.crossVectors(_ap, _ab);
      const v = dir.dot(_cp) * inv;
      if (v < 0 || u + v > 1) continue;
      const dist = _ac.dot(_cp) * inv;
      if (dist >= 0 && dist <= maxDist && (best < 0 || dist < best)) best = dist;
    }
    return best;
  }
}
