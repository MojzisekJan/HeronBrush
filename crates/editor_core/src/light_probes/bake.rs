//! CPU ray-traced probe capture, accelerated by a median-split triangle BVH.
use super::*;
use geometry_math::vec::{cross, dot, normalize, Vec3d};
type V = Vec3d;
#[derive(Clone)]
struct Texture {
    width: u32,
    height: u32,
    pixels: Arc<[u8]>,
    emission: [f32; 3],
    cutoff: f32,
}
struct Node {
    lo: V,
    hi: V,
    start: usize,
    end: usize,
    children: Option<(usize, usize)>,
}
pub(super) struct Snapshot {
    triangles: Vec<render::Triangle>,
    textures: Vec<Option<Texture>>,
    materials: Vec<map_model::material::Material>,
    indices: Vec<usize>,
    nodes: Vec<Node>,
    lights: Vec<crate::render_scene::SceneLight>,
    settings: render::RenderSettings,
    volumes: Vec<render::probes::Volume>,
    size: u32,
}
fn v(a: [f32; 3]) -> V {
    V::new(a[0] as f64, a[1] as f64, a[2] as f64)
}
fn array(p: V) -> [f32; 3] {
    [p.x() as f32, p.y() as f32, p.z() as f32]
}
fn multiply(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] * b[i])
}
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    a.map(|v| v * s)
}
fn linear(a: [f32; 3]) -> [f32; 3] {
    a.map(|v| v.clamp(0.0, 1.0).powf(2.2))
}
impl Snapshot {
    pub(super) fn new(
        document: &Document,
        config: &Settings,
        settings: &render::RenderSettings,
    ) -> Self {
        let triangles = document.render_triangles();
        let materials = document.materials().materials().to_vec();
        let textures = vec![None; materials.len()];
        let mut result = Self {
            indices: (0..triangles.len()).collect(),
            triangles,
            textures,
            materials,
            nodes: Vec::new(),
            lights: document.scene_lights(),
            settings: settings.clone(),
            volumes: config.probes.iter().map(Probe::volume).collect(),
            size: config.resolution,
        };
        result.settings.probes = None;
        result
    }
    fn build(&mut self, start: usize, end: usize) -> usize {
        let mut lo = V::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut hi = -lo;
        for &i in &self.indices[start..end] {
            for point in self.triangles[i].positions {
                for axis in 0..3 {
                    lo[axis] = lo[axis].min(point[axis]);
                    hi[axis] = hi[axis].max(point[axis]);
                }
            }
        }
        let index = self.nodes.len();
        self.nodes.push(Node {
            lo,
            hi,
            start,
            end,
            children: None,
        });
        if end - start > 8 {
            let axis = (0..3)
                .max_by(|&a, &b| (hi[a] - lo[a]).total_cmp(&(hi[b] - lo[b])))
                .unwrap();
            let middle = (start + end) / 2;
            let triangles = &self.triangles;
            self.indices[start..end].select_nth_unstable_by(middle - start, |&a, &b| {
                let center = |i: usize| triangles[i].positions.iter().map(|p| p[axis]).sum::<f64>();
                center(a).total_cmp(&center(b))
            });
            let a = self.build(start, middle);
            let b = self.build(middle, end);
            self.nodes[index].children = Some((a, b));
        }
        index
    }
    fn texel(&self, index: usize, bary: [f64; 3]) -> ([f32; 3], f32, [f32; 3]) {
        let t = &self.triangles[index];
        let mut color = t.color;
        if let Some(colors) = t.corner_colors {
            color = std::array::from_fn(|i| (0..3).map(|k| colors[k][i] * bary[k] as f32).sum());
        }
        let mut albedo = linear(color);
        let mut alpha = t.alpha;
        let mut emissive = [0.0; 3];
        if let Some(texture) = t
            .material
            .and_then(|i| self.textures.get(i))
            .and_then(Option::as_ref)
        {
            let uv: [f64; 2] = std::array::from_fn(|axis| {
                (0..3).map(|k| f64::from(t.uv[k][axis]) * bary[k]).sum()
            });
            let x = (uv[0].floor() as i64).rem_euclid(i64::from(texture.width)) as usize;
            let y = (uv[1].floor() as i64).rem_euclid(i64::from(texture.height)) as usize;
            let offset = (y * texture.width as usize + x) * 4;
            if let Some(pixel) = texture.pixels.get(offset..offset + 4) {
                albedo = multiply(
                    albedo,
                    linear([
                        pixel[0] as f32 / 255.0,
                        pixel[1] as f32 / 255.0,
                        pixel[2] as f32 / 255.0,
                    ]),
                );
                alpha *= pixel[3] as f32 / 255.0;
                if alpha < texture.cutoff {
                    alpha = 0.0;
                }
            }
            emissive = texture.emission;
        }
        (albedo, alpha, emissive)
    }
    fn hit(&self, origin: V, direction: V, maximum: f64) -> Option<(usize, f64, [f64; 3])> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut stack = vec![0];
        let mut closest = maximum;
        let mut hit = None;
        while let Some(index) = stack.pop() {
            let node = &self.nodes[index];
            let (mut near, mut far) = (0.0f64, closest);
            for axis in 0..3 {
                if direction[axis].abs() < 1e-12 {
                    if origin[axis] < node.lo[axis] || origin[axis] > node.hi[axis] {
                        far = -1.0;
                        break;
                    }
                } else {
                    let a = (node.lo[axis] - origin[axis]) / direction[axis];
                    let b = (node.hi[axis] - origin[axis]) / direction[axis];
                    near = near.max(a.min(b));
                    far = far.min(a.max(b));
                }
            }
            if near > far {
                continue;
            }
            if let Some((a, b)) = node.children {
                stack.push(a);
                stack.push(b);
                continue;
            }
            for &i in &self.indices[node.start..node.end] {
                let t = &self.triangles[i];
                let e1 = t.positions[1] - t.positions[0];
                let e2 = t.positions[2] - t.positions[0];
                let p = cross(direction, e2);
                let determinant = dot(e1, p);
                if determinant.abs() < 1e-10 {
                    continue;
                }
                let delta = origin - t.positions[0];
                let u = dot(delta, p) / determinant;
                if !(0.0..=1.0).contains(&u) {
                    continue;
                }
                let q = cross(delta, e1);
                let w = dot(direction, q) / determinant;
                if w < 0.0 || u + w > 1.0 {
                    continue;
                }
                let distance = dot(e2, q) / determinant;
                if distance > 0.05 && distance < closest {
                    let bary = [1.0 - u - w, u, w];
                    if self.texel(i, bary).1 < 0.5 {
                        continue;
                    }
                    closest = distance;
                    hit = Some((i, distance, bary));
                }
            }
        }
        hit
    }
    fn environment(&self, d: V) -> [f32; 3] {
        if let Some(environment) = &self.settings.environment {
            return scale(
                environment
                    .image
                    .sample(array(d), environment.rotation_degrees),
                environment.intensity,
            );
        }
        let horizon = linear(self.settings.horizon_color);
        let color = if d.z() > 0.0 {
            linear(self.settings.sky_color)
        } else {
            linear(self.settings.ground_color)
        };
        let t = d.z().abs().sqrt() as f32;
        std::array::from_fn(|i| horizon[i] * (1.0 - t) + color[i] * t)
    }
    fn radiance(&self, origin: V, direction: V, seed: u32) -> [f32; 3] {
        let Some((index, distance, bary)) = self.hit(origin, direction, 1000000.0) else {
            return self.environment(direction);
        };
        let triangle = &self.triangles[index];
        let point = origin + direction * distance;
        let mut geometric = normalize(triangle.normal);
        if dot(geometric, direction) > 0.0 {
            geometric = -geometric;
        }
        let mut n = triangle.corner_normals.map_or(geometric, |normals| {
            normalize(normals[0] * bary[0] + normals[1] * bary[1] + normals[2] * bary[2])
        });
        if dot(n, direction) > 0.0 {
            n = -n;
        }
        let origin = point + geometric * 0.1;
        let (albedo, _, emissive) = self.texel(index, bary);
        let imported = triangle
            .material
            .and_then(|index| self.materials.get(index))
            .and_then(|material| material.model_pbr.as_ref());
        let factors = triangle.surface_material.map_or(
            [
                imported.map_or(self.settings.default_roughness, |p| p.roughness),
                imported.map_or(self.settings.default_metalness, |p| p.metallic),
                0.0,
            ],
            |s| s.factors(),
        );
        let mut incoming = [0.0; 3];
        let mut direct = [0.0; 3];
        let view = -direction;
        let layer = triangle.surface_material.unwrap_or_else(|| {
            imported.map_or_else(Default::default, |p| {
                map_model::brush_face::SurfaceMaterial {
                    clearcoat: (p.clearcoat * 100.0).round() as u8,
                    coat_roughness: (p.coat_roughness * 100.0).round() as u8,
                    anisotropy: (p.anisotropy * 100.0).round() as u8,
                    anisotropy_rotation: p
                        .anisotropy_rotation
                        .to_degrees()
                        .rem_euclid(180.0)
                        .round() as u8,
                    ..Default::default()
                }
            })
        });
        let dp1 = triangle.positions[1] - triangle.positions[0];
        let dp2 = triangle.positions[2] - triangle.positions[0];
        let uv1 = triangle.uv[1] - triangle.uv[0];
        let uv2 = triangle.uv[2] - triangle.uv[0];
        let raw = dp1 * f64::from(uv2.y()) - dp2 * f64::from(uv1.y());
        let mut tangent = raw - n * dot(n, raw);
        if dot(tangent, tangent) < 1e-10 {
            tangent = cross(
                if n.z().abs() > 0.9 {
                    V::new(1.0, 0.0, 0.0)
                } else {
                    V::new(0.0, 0.0, 1.0)
                },
                n,
            );
        }
        tangent = normalize(tangent);
        let bitangent = cross(n, tangent);
        let rotation = f64::from(layer.anisotropy_rotation % 180).to_radians();
        tangent = tangent * rotation.cos() + bitangent * rotation.sin();
        let bitangent = cross(n, tangent);
        let evaluate = |l: V, radiance: [f32; 3]| {
            let nl = dot(n, l).max(0.0) as f32;
            let nv = dot(n, view).max(0.0001) as f32;
            let h = normalize(l + view);
            let nh = dot(n, h).max(0.0) as f32;
            let vh = dot(view, h).clamp(0.0, 1.0) as f32;
            let rough = factors[0].clamp(0.05, 1.0);
            let alpha = rough * rough;
            let alpha2 = alpha * alpha;
            let mut distribution = alpha2
                / (std::f32::consts::PI * (nh * nh * (alpha2 - 1.0) + 1.0).powi(2)).max(0.000001);
            let k = (rough + 1.0).powi(2) / 8.0;
            let mut geometry = nl / (nl * (1.0 - k) + k) * nv / (nv * (1.0 - k) + k);
            if layer.anisotropy > 0 {
                let amount = f32::from(layer.anisotropy.min(100)) / 100.0;
                let at = (alpha * (1.0 + amount * 0.95)).max(0.002);
                let ab = (alpha * (1.0 - amount * 0.95)).max(0.002);
                let th = dot(tangent, h) as f32 / at;
                let bh = dot(bitangent, h) as f32 / ab;
                distribution = 1.0
                    / (std::f32::consts::PI * at * ab * (th * th + bh * bh + nh * nh).powi(2))
                        .max(0.000001);
                let lambda_v = nl
                    * ((at * dot(tangent, view) as f32).powi(2)
                        + (ab * dot(bitangent, view) as f32).powi(2)
                        + nv * nv)
                        .sqrt();
                let lambda_l = nv
                    * ((at * dot(tangent, l) as f32).powi(2)
                        + (ab * dot(bitangent, l) as f32).powi(2)
                        + nl * nl)
                        .sqrt();
                geometry = (4.0 * nl * nv) * 0.5 / (lambda_v + lambda_l).max(0.00001);
            }
            let f0 = std::array::from_fn(|i| 0.04 * (1.0 - factors[1]) + albedo[i] * factors[1]);
            let fresnel: [f32; 3] = f0.map(|f| f + (1.0 - f) * (1.0 - vh).powi(5));
            let coat = f32::from(layer.clearcoat.min(100)) / 100.0;
            let fc = coat * (0.04 + 0.96 * (1.0 - vh).powi(5));
            let coat_alpha = (f32::from(layer.coat_roughness.min(100)) / 100.0)
                .max(0.05)
                .powi(2);
            let coat_nh = dot(geometric, h).max(0.0) as f32;
            let coat_d = coat_alpha.powi(2)
                / (std::f32::consts::PI
                    * (coat_nh * coat_nh * (coat_alpha.powi(2) - 1.0) + 1.0).powi(2))
                .max(0.000001);
            let coat_specular = coat_d * fc * 0.25 / (dot(l, h) as f32).powi(2).max(0.0001);
            let base: [f32; 3] = std::array::from_fn(|i| {
                radiance[i]
                    * nl
                    * ((1.0 - fresnel[i]) * albedo[i] * (1.0 - factors[1]) / std::f32::consts::PI
                        + distribution * geometry * fresnel[i] / (4.0 * nl * nv).max(0.0001))
            });
            std::array::from_fn(|i| {
                base[i] * (1.0 - fc)
                    + radiance[i] * dot(geometric, l).max(0.0) as f32 * coat_specular
            })
        };
        let elevation = f64::from(self.settings.sun_elevation).to_radians();
        let azimuth = f64::from(self.settings.sun_azimuth).to_radians();
        let sun = V::new(
            elevation.cos() * azimuth.cos(),
            elevation.cos() * azimuth.sin(),
            elevation.sin(),
        );
        let cosine = dot(n, sun).max(0.0) as f32;
        if cosine > 0.0 && (!self.settings.shadows || self.hit(origin, sun, 1000000.0).is_none()) {
            direct = add(
                direct,
                evaluate(
                    sun,
                    scale(self.settings.sun_color, self.settings.sun_intensity),
                ),
            );
        }
        for light in &self.lights {
            let delta = light.position - point;
            let distance = geometry_math::vec::length(delta);
            if distance >= light.range as f64 || distance < 0.01 {
                continue;
            }
            let l = delta / distance;
            let cosine = dot(n, l).max(0.0) as f32;
            if cosine == 0.0 || (light.shadows && self.hit(origin, l, distance - 0.1).is_some()) {
                continue;
            }
            let window = (1.0 - (distance as f32 / light.range).powi(4))
                .max(0.0)
                .powi(2);
            let mut attenuation = window * (distance as f32 / 32.0).max(0.1).powf(-light.decay);
            if let Some((axis, cutoff, exponent)) = light.spot {
                let alignment = dot(-l, v(axis)) as f32;
                if alignment <= cutoff {
                    continue;
                }
                attenuation *=
                    1.0 - ((1.0 - alignment) / (1.0 - cutoff).max(0.0001)).powf(exponent);
            }
            direct = add(direct, evaluate(l, scale(light.color, attenuation)));
        }
        // Environment-to-wall-to-surface diffuse transport; only visible sky contributes.
        let helper = if n.z().abs() > 0.9 {
            V::new(1.0, 0.0, 0.0)
        } else {
            V::new(0.0, 0.0, 1.0)
        };
        let t = normalize(cross(helper, n));
        let b = cross(n, t);
        for sample in 0..8 {
            let u = (sample as f64 + 0.5) / 8.0;
            let phi =
                (sample as f64 * 2.39996323 + seed as f64 * 0.61803399) % std::f64::consts::TAU;
            let d = normalize(
                t * (u.sqrt() * phi.cos()) + b * (u.sqrt() * phi.sin()) + n * (1.0 - u).sqrt(),
            );
            if self.hit(origin, d, 1000000.0).is_none() {
                incoming = add(incoming, scale(self.environment(d), 1.0 / 8.0));
            }
        }
        let reflected = normalize(direction - n * (2.0 * dot(direction, n)));
        if self.hit(origin, reflected, 1000000.0).is_none() {
            let f0: [f32; 3] =
                std::array::from_fn(|i| 0.04 * (1.0 - factors[1]) + albedo[i] * factors[1]);
            direct = add(direct, multiply(self.environment(reflected), f0));
        }
        add(
            direct,
            add(
                multiply(albedo, scale(incoming, 1.0 - factors[1])),
                add(scale(albedo, factors[2]), emissive),
            ),
        )
        .map(|value| value.clamp(0.0, 65504.0))
    }
    pub(super) fn bake(
        mut self,
        cancel: &AtomicBool,
        progress: &AtomicUsize,
    ) -> Result<render::probes::Lighting, String> {
        if !self.indices.is_empty() {
            self.build(0, self.indices.len());
        }
        let used: std::collections::HashSet<usize> = self
            .triangles
            .iter()
            .filter_map(|triangle| triangle.material)
            .collect();
        for index in used {
            if cancel.load(Ordering::Relaxed) {
                return Err("Bake cancelled".into());
            }
            if let Some(m) = self.materials.get(index) {
                self.textures[index] = m.pixels().map(|pixels| Texture {
                    width: m.width.max(1),
                    height: m.height.max(1),
                    pixels,
                    emission: m.model_pbr.as_ref().map_or([0.0; 3], |p| p.emissive_factor),
                    cutoff: if m.alpha() == map_model::material::Alpha::Masked {
                        0.5
                    } else {
                        0.0
                    },
                });
            }
        }
        let mut output = Vec::new();
        for volume in &self.volumes {
            let width = self.size * 2;
            let height = self.size;
            let mut pixels = Vec::with_capacity((width * height) as usize);
            let mut sh = [[0.0; 3]; 9];
            for y in 0..height {
                if cancel.load(Ordering::Relaxed) {
                    return Err("Bake cancelled".into());
                }
                let theta = (y as f64 + 0.5) / height as f64 * std::f64::consts::PI;
                let weight = (theta.sin() * std::f64::consts::PI / height as f64
                    * std::f64::consts::TAU
                    / width as f64) as f32;
                for x in 0..width {
                    let phi = ((x as f64 + 0.5) / width as f64 - 0.5) * std::f64::consts::TAU;
                    let d = V::new(
                        theta.sin() * phi.cos(),
                        theta.sin() * phi.sin(),
                        theta.cos(),
                    );
                    let rgb = self.radiance(v(volume.position), d, x + y * width);
                    pixels.push([rgb[0], rgb[1], rgb[2], 1.0]);
                    let [dx, dy, dz] = array(d);
                    let basis = [
                        0.282095,
                        0.488603 * dy,
                        0.488603 * dz,
                        0.488603 * dx,
                        1.092548 * dx * dy,
                        1.092548 * dy * dz,
                        0.315392 * (3.0 * dz * dz - 1.0),
                        1.092548 * dx * dz,
                        0.546274 * (dx * dx - dy * dy),
                    ];
                    for i in 0..9 {
                        for channel in 0..3 {
                            sh[i][channel] += rgb[channel] * basis[i] * weight;
                        }
                    }
                }
            }
            let panorama = render::environment::EnvironmentImage::new(width, height, pixels)?;
            let mut mips = Vec::new();
            for level in 0..=self.size.ilog2() {
                let size = (self.size >> level).max(1);
                let roughness = level as f64 / self.size.ilog2() as f64;
                let mut pixels = Vec::new();
                for face in 0..6 {
                    for y in 0..size {
                        for x in 0..size {
                            if cancel.load(Ordering::Relaxed) {
                                return Err("Bake cancelled".into());
                            }
                            let s = (x as f64 + 0.5) / size as f64 * 2.0 - 1.0;
                            let t = (y as f64 + 0.5) / size as f64 * 2.0 - 1.0;
                            let n = normalize(match face {
                                0 => V::new(1.0, -t, -s),
                                1 => V::new(-1.0, -t, s),
                                2 => V::new(s, 1.0, t),
                                3 => V::new(s, -1.0, -t),
                                4 => V::new(s, -t, 1.0),
                                _ => V::new(-s, -t, -1.0),
                            });
                            let rgb = if level == 0 {
                                panorama.sample(array(n), 0.0)
                            } else {
                                let helper = if n.z().abs() > 0.99 {
                                    V::new(1.0, 0.0, 0.0)
                                } else {
                                    V::new(0.0, 0.0, 1.0)
                                };
                                let t = normalize(cross(helper, n));
                                let b = cross(n, t);
                                let mut sum = [0.0; 3];
                                let mut weights = 0.0;
                                for sample in 0..64u32 {
                                    let u = (sample as f64 + 0.5) / 64.0;
                                    let v = sample.reverse_bits() as f64 / 4294967296.0;
                                    let phi = std::f64::consts::TAU * u;
                                    let alpha = roughness * roughness;
                                    let cosine =
                                        ((1.0 - v) / (1.0 + (alpha * alpha - 1.0) * v)).sqrt();
                                    let sine = (1.0 - cosine * cosine).max(0.0).sqrt();
                                    let h = normalize(
                                        t * (phi.cos() * sine)
                                            + b * (phi.sin() * sine)
                                            + n * cosine,
                                    );
                                    let d = h * (2.0 * dot(n, h)) - n;
                                    let weight = dot(n, d).max(0.0) as f32;
                                    sum = add(sum, scale(panorama.sample(array(d), 0.0), weight));
                                    weights += weight;
                                }
                                scale(sum, 1.0 / weights.max(0.0001))
                            };
                            pixels.push([rgb[0], rgb[1], rgb[2], 1.0]);
                        }
                    }
                }
                mips.push(pixels);
            }
            output.push(render::probes::BakedProbe {
                volume: volume.clone(),
                size: self.size,
                mips,
                sh,
            });
            progress.fetch_add(1, Ordering::Relaxed);
        }
        render::probes::Lighting::new(output)
    }
}
