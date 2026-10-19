//! Point and spot lights from light entities: a light table, clustered forward shading
//! with the clusters filled on the CPU, and cached cube shadow maps for the brightest
//! few shadowed lights.
//!
//! **Clusters.** The view frustum is cut into a 16 x 9 grid of screen tiles and 24
//! depth slices spaced logarithmically (Doom 2016 / Bevy), and each cluster lists the
//! lights whose sphere may reach into it. Each visible light's view-space box gives a
//! conservative range of tiles and slices; two passes over those ranges (count, then
//! fill after a prefix sum) build the lists in vectors kept across frames, so a frame
//! allocates nothing. A few hundred lights cost well under a millisecond, and the
//! fragment shader reads only the lights of its own cluster. No compute is needed, so
//! a future GLES path keeps it.
//!
//! **Shadows.** The `MAX_SHADOWED` most important lights in view that cast shadows each
//! get a cube in one `Depth32Float` cube-map array (allocated for as many as are needed,
//! side a quarter of a sun cascade's). A cube face is drawn again only when its key
//! changes: the light (position, reach) or the generation of a chunk its frustum covers,
//! hidden objects or materials. Spot lights skip the faces their cone cannot reach.
//! Lights keep their slot, and their cube, until another light needs it, so a camera
//! move redraws nothing unless it brings a light into the set that was never drawn.

use std::hash::{Hash, Hasher};

use bytemuck::{Pod, Zeroable};
use editor_core::render_scene::{SceneLight, MAP_UNITS_PER_METRE};

/// Lights the table holds; more are ignored (the nearest first would be the next step).
pub(crate) const MAX_LIGHTS: usize = 1024;
/// At most this many lights in one cluster.
pub(crate) const MAX_PER_CLUSTER: u32 = 64;
/// At most this many lights cast shadows at once.
pub(crate) const MAX_SHADOWED: usize = 8;
/// The cluster grid: screen tiles across, down, and depth slices.
pub(crate) const GRID: [u32; 3] = [16, 9, 24];
/// Where the first depth slice ends; everything nearer is in it.
const CLUSTER_NEAR: f32 = 16.0;
/// The near plane of a light's cube, in map units: the light's own marker (an entity's
/// box is 16 units) stays out of its shadow map instead of enclosing the light.
pub(crate) const SHADOW_NEAR: f32 = 8.0;
/// Each cube face's view-projection sits at this stride in the uniform buffer.
const FACE_STRIDE: u64 = 256;

/// One light as the shader reads it, 64 bytes.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, PartialEq)]
pub(crate) struct LightGpu {
    /// xyz position, w reach in map units.
    position_range: [f32; 4],
    /// rgb colour times energy, w the distance falloff exponent (in metres).
    color_decay: [f32; 4],
    /// xyz the spot direction, w the cosine of its half-angle (-2 for a point light).
    direction_cutoff: [f32; 4],
    /// x the cone's rim falloff exponent, y the cube slot (-1 without a shadow), z map
    /// units per metre, w unused.
    params: [f32; 4],
}

/// What the frame uniform needs to know about this frame's lights.
#[derive(Clone, Copy, Default)]
pub(crate) struct LightFrame {
    /// x lights in view (0: skip the loop), yzw the grid.
    pub lights: [f32; 4],
    /// x, y tile size in pixels; z, w turn view depth into a slice: `log2(z) * z - w`.
    pub cluster: [f32; 4],
    /// x cube side, y 1 / side, z the cube near plane, w unused.
    pub point_shadow: [f32; 4],
}

/// The cube shadow maps.
pub(crate) struct CubeShadows {
    pub size: u32,
    pub slots: usize,
    _texture: wgpu::Texture,
    /// All cubes, for sampling.
    pub array_view: wgpu::TextureView,
    /// One 2D view per (slot, face), to draw into.
    faces: Vec<wgpu::TextureView>,
    uniform: wgpu::Buffer,
    pub uniform_group: wgpu::BindGroup,
    /// What each (slot, face) last drew.
    cache: Vec<Option<u64>>,
}

impl CubeShadows {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, size: u32, slots: usize) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_wgpu light shadows"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 6 * slots as u32 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let array_view = texture.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::CubeArray), ..Default::default() });
        let faces = (0..6 * slots as u32)
            .map(|layer| {
                texture.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2), base_array_layer: layer, array_layer_count: Some(1), ..Default::default() })
            })
            .collect();
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("render_wgpu light shadow faces"),
            size: FACE_STRIDE * 6 * slots as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_wgpu light shadow faces"),
            layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &uniform, offset: 0, size: wgpu::BufferSize::new(64) }) }],
        });
        Self { size, slots, _texture: texture, array_view, faces, uniform, uniform_group, cache: vec![None; 6 * slots] }
    }

    /// The dynamic offset of a face's matrix.
    pub fn offset(slot: usize, face: usize) -> u32 {
        (FACE_STRIDE * (slot * 6 + face) as u64) as u32
    }

    /// The depth view to draw a face into.
    pub fn face(&self, slot: usize, face: usize) -> &wgpu::TextureView {
        &self.faces[slot * 6 + face]
    }

    /// Whether a face must be drawn again for `key`; if so writes its matrix and
    /// remembers the key.
    pub fn claim(&mut self, queue: &wgpu::Queue, slot: usize, face: usize, key: u64, matrix: &[[f32; 4]; 4]) -> bool {
        let entry = &mut self.cache[slot * 6 + face];
        if *entry == Some(key) {
            return false;
        }
        *entry = Some(key);
        queue.write_buffer(&self.uniform, u64::from(Self::offset(slot, face)), bytemuck::cast_slice(matrix));
        true
    }
}

/// The lights, their clusters and their shadows.
pub(crate) struct Lights {
    /// The scene's lights as last uploaded, and the scene generation they are from.
    table: Vec<LightGpu>,
    uploaded: Option<u64>,
    pub light_buffer: wgpu::Buffer,
    pub cluster_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    /// Bumped whenever a buffer is replaced, so the bind group holding them is rebuilt.
    pub generation: u64,
    // Scratch kept across frames.
    boxes: Vec<(u32, [u32; 6])>,
    counts: Vec<u32>,
    clusters: Vec<[u32; 2]>,
    indices: Vec<u32>,
    ranked: Vec<(f32, usize)>,
    /// Which light (by id) holds each shadow slot.
    slot_owner: [Option<u64>; MAX_SHADOWED],
    /// The light index per shadow slot this frame.
    pub shadowed: Vec<(usize, usize)>,
    pub shadows: Option<CubeShadows>,
    /// Stands in for the cube array when no light casts a shadow.
    pub no_shadows: wgpu::TextureView,
    _no_shadows: wgpu::Texture,
}

fn storage(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: size.max(256), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })
}

/// Godot's distance attenuation, in metres: the light's own falloff, windowed to zero at
/// its reach. What the shader evaluates; here to pick shadowed lights by their effect.
fn attenuation(distance: f32, range: f32, decay: f32) -> f32 {
    let nd = (distance / range).powi(4);
    let window = (1.0 - nd).max(0.0).powi(2);
    window * (distance / MAP_UNITS_PER_METRE as f32).max(0.1).powf(-decay)
}

impl Lights {
    pub fn new(device: &wgpu::Device) -> Self {
        let cells = (GRID[0] * GRID[1] * GRID[2]) as usize;
        let no_shadows_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_wgpu no light shadows"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let no_shadows = no_shadows_texture.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::CubeArray), ..Default::default() });
        Self {
            table: Vec::new(),
            uploaded: None,
            light_buffer: storage(device, "render_wgpu lights", 64 * 16),
            cluster_buffer: storage(device, "render_wgpu light clusters", (cells * 8) as u64),
            index_buffer: storage(device, "render_wgpu light indices", 4096),
            generation: 0,
            boxes: Vec::new(),
            counts: vec![0; cells],
            clusters: vec![[0; 2]; cells],
            indices: Vec::new(),
            ranked: Vec::new(),
            slot_owner: [None; MAX_SHADOWED],
            shadowed: Vec::new(),
            shadows: None,
            no_shadows,
            _no_shadows: no_shadows_texture,
        }
    }

    /// The cube array the forward pass samples.
    pub fn shadow_view(&self) -> &wgpu::TextureView {
        self.shadows.as_ref().map_or(&self.no_shadows, |s| &s.array_view)
    }

    /// Brings the light table, the shadow slots and the clusters up to date for one
    /// frame. `view` and `proj` are the camera's (column-major, reverse-Z projection),
    /// `planes` its frustum, `size` the viewport in pixels. Returns the uniform values;
    /// a zero light count when lights are off or none is in view.
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cascade_layout: &wgpu::BindGroupLayout,
        scene: &[SceneLight],
        scene_generation: u64,
        eye: [f32; 3],
        view: &[[f32; 4]; 4],
        proj: &[[f32; 4]; 4],
        planes: &[[f32; 4]; 6],
        size: (u32, u32),
        shadow_size: u32,
        enabled: bool,
    ) -> LightFrame {
        self.shadowed.clear();
        let mut frame = LightFrame::default();
        if !enabled || scene.is_empty() {
            return frame;
        }
        let lights = &scene[..scene.len().min(MAX_LIGHTS)];

        // Which lights cast shadows this frame: the most important in view, keeping the
        // slots of those that stay so their cubes stay cached.
        self.ranked.clear();
        for (index, light) in lights.iter().enumerate() {
            let p = [light.position.x() as f32, light.position.y() as f32, light.position.z() as f32];
            if !sphere_visible(planes, p, light.range) {
                continue;
            }
            if light.shadows {
                let distance = ((p[0] - eye[0]).powi(2) + (p[1] - eye[1]).powi(2) + (p[2] - eye[2]).powi(2)).sqrt();
                // The light's strength at the camera's distance from its sphere (or at its
                // half reach from inside), so a near lamp outranks a far floodlight.
                let probe = (distance - light.range).max(light.range * 0.5);
                let luminance = 0.2126 * light.color[0] + 0.7152 * light.color[1] + 0.0722 * light.color[2];
                self.ranked.push((luminance * attenuation(probe.min(light.range * 0.99), light.range, light.decay), index));
            }
        }
        self.ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
        self.ranked.truncate(MAX_SHADOWED);
        // A light that leaves the set keeps its cube until another light needs the slot,
        // so turning the camera away and back does not redraw it.
        let wanted = |id: u64, ranked: &[(f32, usize)]| ranked.iter().any(|(_, i)| lights[*i].id == id);
        for &(_, index) in &self.ranked {
            let id = lights[index].id;
            let slot = match self.slot_owner.iter().position(|o| *o == Some(id)) {
                Some(slot) => slot,
                None => {
                    let free = self
                        .slot_owner
                        .iter()
                        .position(Option::is_none)
                        .or_else(|| self.slot_owner.iter().position(|o| o.is_some_and(|owner| !wanted(owner, &self.ranked))))
                        .expect("as many slots as ranked lights");
                    self.slot_owner[free] = Some(id);
                    free
                }
            };
            self.shadowed.push((slot, index));
        }
        let slots_needed = self.shadowed.iter().map(|(slot, _)| slot + 1).max().unwrap_or(0);
        if slots_needed > 0 {
            let slots = slots_needed.next_power_of_two().min(MAX_SHADOWED);
            if self.shadows.as_ref().is_none_or(|s| s.size != shadow_size || s.slots < slots) {
                self.shadows = Some(CubeShadows::new(device, cascade_layout, shadow_size, slots));
                self.generation += 1;
            }
        }

        // The light table: rewritten when the scene's lights or the shadow slots changed.
        let mut table_changed = self.uploaded != Some(scene_generation) || self.table.len() != lights.len();
        self.table.resize(lights.len(), LightGpu::default());
        for (index, light) in lights.iter().enumerate() {
            let slot = self.shadowed.iter().find(|(_, i)| *i == index).map_or(-1.0, |(s, _)| *s as f32);
            let (direction, cutoff, exponent) = light.spot.unwrap_or(([0.0, 0.0, -1.0], -2.0, 1.0));
            let entry = LightGpu {
                position_range: [light.position.x() as f32, light.position.y() as f32, light.position.z() as f32, light.range],
                color_decay: [light.color[0], light.color[1], light.color[2], light.decay],
                direction_cutoff: [direction[0], direction[1], direction[2], cutoff],
                params: [exponent, slot, MAP_UNITS_PER_METRE as f32, 0.0],
            };
            if self.table[index] != entry {
                self.table[index] = entry;
                table_changed = true;
            }
        }
        if table_changed {
            let bytes = (self.table.len() * std::mem::size_of::<LightGpu>()) as u64;
            if bytes > self.light_buffer.size() {
                self.light_buffer = storage(device, "render_wgpu lights", bytes.next_power_of_two());
                self.generation += 1;
            }
            queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&self.table));
            self.uploaded = Some(scene_generation);
        }

        // Clusters. Depth slices: slice = log2(z / near) / log2(far / near) * slices.
        let near = CLUSTER_NEAR;
        let far = lights.iter().map(|l| l.range).fold(near * 4.0, f32::max).max(near * 4.0) * 64.0;
        let slice_scale = GRID[2] as f32 / (far / near).log2();
        let slice_bias = near.log2() * slice_scale;
        let slice_of = |z: f32| ((z.max(near).log2() * slice_scale - slice_bias).floor().max(0.0) as u32).min(GRID[2] - 1);
        let cells = (GRID[0] * GRID[1] * GRID[2]) as usize;
        self.counts.iter_mut().for_each(|c| *c = 0);
        self.boxes.clear();
        let camera_near = near_plane(proj);
        for (index, light) in lights.iter().enumerate() {
            let p = [light.position.x() as f32, light.position.y() as f32, light.position.z() as f32];
            let r = light.range;
            if !sphere_visible(planes, p, r) {
                continue;
            }
            let c = transform(view, p);
            // View space looks down -z.
            let (z_near, z_far) = (-c[2] - r, -c[2] + r);
            if z_far <= 0.0 {
                continue;
            }
            let (s0, s1) = (slice_of(z_near), slice_of(z_far));
            let mut tiles = [0, 0, GRID[0] - 1, GRID[1] - 1];
            if z_near > camera_near {
                // The sphere's view-space box, projected: a conservative screen rectangle.
                let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
                for corner in 0..8 {
                    let q = [c[0] + if corner & 1 == 0 { -r } else { r }, c[1] + if corner & 2 == 0 { -r } else { r }, c[2] + if corner & 4 == 0 { -r } else { r }];
                    let clip = transform4(proj, q);
                    let w = clip[3].max(1e-4);
                    for axis in 0..2 {
                        lo[axis] = lo[axis].min(clip[axis] / w);
                        hi[axis] = hi[axis].max(clip[axis] / w);
                    }
                }
                if lo[0] > 1.0 || hi[0] < -1.0 || lo[1] > 1.0 || hi[1] < -1.0 {
                    continue;
                }
                // NDC y up, tiles counted from the top.
                let tile = |v: f32, n: u32| (((v * 0.5 + 0.5) * n as f32).floor().max(0.0) as u32).min(n - 1);
                tiles = [tile(lo[0], GRID[0]), tile(-hi[1], GRID[1]), tile(hi[0], GRID[0]), tile(-lo[1], GRID[1])];
            }
            let b = [tiles[0], tiles[1], s0, tiles[2], tiles[3], s1];
            for z in b[2]..=b[5] {
                for y in b[1]..=b[4] {
                    for x in b[0]..=b[3] {
                        let cell = ((z * GRID[1] + y) * GRID[0] + x) as usize;
                        self.counts[cell] = (self.counts[cell] + 1).min(MAX_PER_CLUSTER);
                    }
                }
            }
            self.boxes.push((index as u32, b));
        }
        if self.boxes.is_empty() {
            return frame;
        }
        let mut total = 0u32;
        for cell in 0..cells {
            self.clusters[cell] = [total, 0];
            total += self.counts[cell];
        }
        self.indices.clear();
        self.indices.resize(total as usize, 0);
        for &(index, b) in &self.boxes {
            for z in b[2]..=b[5] {
                for y in b[1]..=b[4] {
                    for x in b[0]..=b[3] {
                        let cell = ((z * GRID[1] + y) * GRID[0] + x) as usize;
                        let [offset, count] = &mut self.clusters[cell];
                        if *count < self.counts[cell] {
                            self.indices[(*offset + *count) as usize] = index;
                            *count += 1;
                        }
                    }
                }
            }
        }
        queue.write_buffer(&self.cluster_buffer, 0, bytemuck::cast_slice(&self.clusters));
        let bytes = (self.indices.len() * 4) as u64;
        if bytes > self.index_buffer.size() {
            self.index_buffer = storage(device, "render_wgpu light indices", bytes.next_power_of_two());
            self.generation += 1;
        }
        if !self.indices.is_empty() {
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
        }
        frame.lights = [self.boxes.len() as f32, GRID[0] as f32, GRID[1] as f32, GRID[2] as f32];
        frame.cluster = [size.0 as f32 / GRID[0] as f32, size.1 as f32 / GRID[1] as f32, slice_scale, slice_bias];
        let side = self.shadows.as_ref().map_or(1, |s| s.size) as f32;
        frame.point_shadow = [side, 1.0 / side, SHADOW_NEAR, 0.0];
        frame
    }
}

/// The camera's near plane from a reverse-Z projection (depth 1 at the near plane).
fn near_plane(proj: &[[f32; 4]; 4]) -> f32 {
    // z_clip = a z + b, w = -z; at the near plane z_clip / w = 1: -n = b / (1 + a).
    let (a, b) = (proj[2][2], proj[3][2]);
    (b / (1.0 + a)).abs().max(0.01)
}

fn transform(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    let v = transform4(m, p);
    [v[0], v[1], v[2]]
}

fn transform4(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 4] {
    let mut out = [0.0; 4];
    for (row, value) in out.iter_mut().enumerate() {
        *value = m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row];
    }
    out
}

/// Whether a sphere reaches inside the frustum (planes positive inside, unnormalised).
pub(crate) fn sphere_visible(planes: &[[f32; 4]; 6], c: [f32; 3], r: f32) -> bool {
    planes.iter().all(|p| {
        let length = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-12);
        (p[0] * c[0] + p[1] * c[1] + p[2] * c[2] + p[3]) / length >= -r
    })
}

/// For each cube face, the axis it looks along and the world axes its texture's s and t
/// run along: the cube-map convention the hardware samples by (and `ibl.wgsl`'s
/// `cube_direction` draws by), +X, -X, +Y, -Y, +Z, -Z.
const FACES: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
    ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, -1.0, 0.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]),
    ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
    ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
    ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
];

/// The view-projection of one face of a light's cube (column-major): 90 degrees, depth
/// 0 at `SHADOW_NEAR` to 1 at `far` along the face's axis, laid out so that sampling the
/// cube with the direction from the light finds the texel drawn for it.
pub(crate) fn face_matrix(face: usize, light: [f32; 3], far: f32) -> [[f32; 4]; 4] {
    let (forward, s, t) = FACES[face];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (n, f) = (SHADOW_NEAR, far.max(SHADOW_NEAR * 2.0));
    let a = f / (f - n);
    let b = -f * n / (f - n);
    // Rows: x = s, y = -t (framebuffer y runs down the texture), z = a * w + b, w = forward.
    let rows = [
        [s[0], s[1], s[2], -dot(s, light)],
        [-t[0], -t[1], -t[2], dot(t, light)],
        [a * forward[0], a * forward[1], a * forward[2], -a * dot(forward, light) + b],
        [forward[0], forward[1], forward[2], -dot(forward, light)],
    ];
    let mut columns = [[0.0; 4]; 4];
    for (r, row) in rows.iter().enumerate() {
        for (c, value) in row.iter().enumerate() {
            columns[c][r] = *value;
        }
    }
    columns
}

/// Whether a spot light's cone can reach a cube face: the face's frustum spans up to
/// 54.7 degrees from its axis (the corner), so a cone whose axis is further than that
/// plus its half-angle from the face's axis misses it.
pub(crate) fn face_reached(face: usize, spot: Option<([f32; 3], f32, f32)>) -> bool {
    let Some((direction, cutoff, _)) = spot else { return true };
    let forward = FACES[face].0;
    let angle = (direction[0] * forward[0] + direction[1] * forward[1] + direction[2] * forward[2]).clamp(-1.0, 1.0).acos();
    angle <= cutoff.clamp(-1.0, 1.0).acos() + 54.8f32.to_radians()
}

/// The cache key of a cube face: the light and the casters its frustum covers.
pub(crate) fn face_key(light: &SceneLight, face: usize, chunks: impl Iterator<Item = ((i32, i32, i32), u64)>, hidden: u64, materials: u64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    light.id.hash(&mut hasher);
    face.hash(&mut hasher);
    for v in [light.position.x(), light.position.y(), light.position.z()] {
        v.to_bits().hash(&mut hasher);
    }
    light.range.to_bits().hash(&mut hasher);
    hidden.hash(&mut hasher);
    materials.hash(&mut hasher);
    // Order-independent over the chunks (they come from a hash map).
    let mut sum = 0u64;
    let mut count = 0u64;
    for (key, generation) in chunks {
        let mut chunk = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut chunk);
        generation.hash(&mut chunk);
        sum = sum.wrapping_add(chunk.finish());
        count += 1;
    }
    sum.hash(&mut hasher);
    count.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A point in front of each face lands inside it with the depth the shader expects:
    /// x, y within -1..1 and z / w = a + b / (distance along the axis).
    #[test]
    fn face_matrices_put_points_where_the_cube_samples_them() {
        let light = [10.0, -20.0, 30.0];
        let far = 200.0;
        for (face, (forward, s, t)) in FACES.iter().enumerate() {
            let m = face_matrix(face, light, far);
            // 50 units along the axis, a bit along s and t.
            let p: [f32; 3] = std::array::from_fn(|i| light[i] + forward[i] * 50.0 + s[i] * 10.0 + t[i] * 20.0);
            let clip = transform4(&m, p);
            let (x, y, z) = (clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]);
            assert!((x - 0.2).abs() < 1e-4, "face {face}: s {x}");
            assert!((y + 0.4).abs() < 1e-4, "face {face}: framebuffer y {y}");
            let (n, f) = (SHADOW_NEAR, far);
            let expected = f / (f - n) - f * n / ((f - n) * 50.0);
            assert!((z - expected).abs() < 1e-5, "face {face}: depth {z}");
        }
    }

    #[test]
    fn a_downward_spot_reaches_only_the_faces_round_its_cone() {
        let reached = |half_angle: f32| -> Vec<usize> { (0..6).filter(|&f| face_reached(f, Some(([0.0, 0.0, -1.0], half_angle.to_radians().cos(), 1.0)))).collect() };
        assert_eq!(reached(30.0), vec![5]);
        assert_eq!(reached(60.0), vec![0, 1, 2, 3, 5]);
        assert!((0..6).all(|f| face_reached(f, None)));
    }

    #[test]
    fn attenuation_ends_at_the_reach() {
        assert!(attenuation(32.0, 160.0, 1.0) > attenuation(64.0, 160.0, 1.0));
        assert_eq!(attenuation(160.0, 160.0, 1.0), 0.0);
    }
}
