//! Cascaded shadow maps for the sun: four cascades in one `Depth32Float` array, fitted
//! as stable bounding spheres of the view-frustum slices and snapped to whole texels so
//! a moving camera does not make shadow edges crawl; sampled with hardware PCF through a
//! comparison sampler.
//!
//! Each cascade is cached: it is drawn again only when its matrix (the sun, the split, or
//! a camera move of at least a texel) or the casters it covers (chunk rebuilds, hidden
//! objects, materials) change. A selection click, a hover over a handle or a settings
//! change that does not touch the sun costs no shadow pass at all.

use std::hash::{Hash, Hasher};

use geometry_math::vec::Vec3d;
use render::camera3d::Camera3d;

/// Number of cascades.
pub(crate) const CASCADES: usize = 4;
/// How far the split scheme leans to logarithmic (1) from uniform (0).
const SPLIT_LAMBDA: f64 = 0.9;
/// The uniform stride per cascade; each cascade's matrix sits at a 256-byte offset.
const CASCADE_STRIDE: u64 = 256;

/// One frame's cascade setup.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Fit {
    /// World to shadow space (column-major): xy -1..1, z 0..1.
    pub matrices: [[[f32; 4]; 4]; CASCADES],
    /// The far end of each cascade, along the view axis.
    pub splits: [f32; CASCADES],
    /// The world size of a texel.
    pub texel: [f32; CASCADES],
    /// The depth bias, in shadow depth units.
    pub bias: [f32; CASCADES],
}

/// The cascades' depth range snaps outwards to multiples of this many map units.
const DEPTH_STEP: f64 = 256.0;

/// Where the cascades go for `camera` looking at a `width` by `height` view, for a sun
/// shining along `light` (unit, the way light travels), over a scene with the given
/// bounds, at `size` texels per side. `bias_scale` is the user's bias relative to its
/// default.
pub(crate) fn fit(camera: &Camera3d, projection: &geometry_math::mat::Mat4x4d, light: [f64; 3], bounds: ([f32; 3], [f32; 3]), size: u32, bias_scale: f64) -> Fit {
    let eye = camera.position;
    let forward = camera.direction();
    // The frustum's slope, read off the projection so it matches what is drawn.
    let tan_x = 1.0 / projection[0][0];
    let tan_y = 1.0 / projection[1][1];

    let lo = Vec3d::new(f64::from(bounds.0[0]), f64::from(bounds.0[1]), f64::from(bounds.0[2]));
    let hi = Vec3d::new(f64::from(bounds.1[0]), f64::from(bounds.1[1]), f64::from(bounds.1[2]));
    let corners = [
        Vec3d::new(lo.x(), lo.y(), lo.z()),
        Vec3d::new(hi.x(), lo.y(), lo.z()),
        Vec3d::new(lo.x(), hi.y(), lo.z()),
        Vec3d::new(hi.x(), hi.y(), lo.z()),
        Vec3d::new(lo.x(), lo.y(), hi.z()),
        Vec3d::new(hi.x(), lo.y(), hi.z()),
        Vec3d::new(lo.x(), hi.y(), hi.z()),
        Vec3d::new(hi.x(), hi.y(), hi.z()),
    ];
    // Shadows reach as far as the farthest corner of the scene, quantised to quarter
    // octaves so the splits (and with them the cached cascades) stay put while the
    // camera moves about inside the map.
    let farthest = corners.iter().map(|c| geometry_math::vec::length(*c - eye)).fold(0.0, f64::max);
    let reach = farthest.clamp(64.0, camera.far.max(64.0));
    let reach = 2f64.powf((reach.log2() * 4.0).ceil() / 4.0);
    // And they start where the scene does: a camera standing back from the map would
    // otherwise spend its finest cascades on empty air. Quantised down the same way.
    let gap = Vec3d::new(
        (lo.x() - eye.x()).max(eye.x() - hi.x()).max(0.0),
        (lo.y() - eye.y()).max(eye.y() - hi.y()).max(0.0),
        (lo.z() - eye.z()).max(eye.z() - hi.z()).max(0.0),
    );
    let nearest = geometry_math::vec::length(gap);
    let near = camera.near.max(0.1).max(if nearest > 1.0 { 2f64.powf((nearest.log2() * 4.0).floor() / 4.0) } else { 0.0 });
    let near = near.min(reach / 4.0);

    // The light's frame: x and y across the map, z along the light.
    let l = Vec3d::new(light[0], light[1], light[2]);
    let reference = if l.z().abs() > 0.99 { Vec3d::new(1.0, 0.0, 0.0) } else { Vec3d::new(0.0, 0.0, 1.0) };
    let lx = geometry_math::vec::normalize(geometry_math::vec::cross(l, reference));
    let ly = geometry_math::vec::cross(lx, l);
    let dot = geometry_math::vec::dot;
    // Casters anywhere in the scene, in front of or behind the slice, are inside the
    // depth range; it depends on the scene only, so camera moves keep it. Widened to
    // whole steps of `DEPTH_STEP`, so an animated object that moves the scene's bounds
    // does not change every cascade's depth mapping (and its bias) every frame.
    let z_min = ((corners.iter().map(|c| dot(*c, l)).fold(f64::MAX, f64::min) - 16.0) / DEPTH_STEP).floor() * DEPTH_STEP;
    let z_max = ((corners.iter().map(|c| dot(*c, l)).fold(f64::MIN, f64::max) + 16.0) / DEPTH_STEP).ceil() * DEPTH_STEP;
    let z_range = (z_max - z_min).max(1.0);

    let mut out = Fit::default();
    let mut previous = near;
    for i in 0..CASCADES {
        let t = (i + 1) as f64 / CASCADES as f64;
        let logarithmic = near * (reach / near).powf(t);
        let uniform = near + (reach - near) * t;
        let far = SPLIT_LAMBDA * logarithmic + (1.0 - SPLIT_LAMBDA) * uniform;
        // The slice's bounding sphere: centred on the view axis, its radius depending
        // only on the split and the field of view, so turning the camera never changes
        // the cascade's size (no shimmer from a resizing map).
        let middle = (previous + far) / 2.0;
        let corner = |d: f64| ((d - middle).powi(2) + (d * tan_x).powi(2) + (d * tan_y).powi(2)).sqrt();
        let radius = corner(previous).max(corner(far));
        // Rounded up to a sixteenth, so float noise in the sum cannot change it.
        let radius = (radius * 16.0).ceil() / 16.0;
        let center = eye + forward * middle;
        // Snapped to whole texels in the light's frame, so the map slides by texels.
        let texel = 2.0 * radius / f64::from(size);
        let cx = (dot(center, lx) / texel).floor() * texel;
        let cy = (dot(center, ly) / texel).floor() * texel;
        let r = radius;
        let m = [
            [lx.x() / r, ly.x() / r, l.x() / z_range, 0.0],
            [lx.y() / r, ly.y() / r, l.y() / z_range, 0.0],
            [lx.z() / r, ly.z() / r, l.z() / z_range, 0.0],
            [-cx / r, -cy / r, -z_min / z_range, 1.0],
        ];
        out.matrices[i] = m.map(|column| column.map(|v| v as f32));
        out.splits[i] = far as f32;
        out.texel[i] = texel as f32;
        out.bias[i] = (texel * bias_scale * 0.5 / z_range) as f32;
        previous = far;
    }
    out
}

/// The shadow-map array and the per-cascade cache.
pub(crate) struct Shadows {
    pub size: u32,
    _texture: wgpu::Texture,
    /// All cascades, for sampling.
    pub array_view: wgpu::TextureView,
    layers: Vec<wgpu::TextureView>,
    uniform: wgpu::Buffer,
    pub uniform_group: wgpu::BindGroup,
    /// What each cascade last drew.
    cache: [Option<u64>; CASCADES],
}

impl Shadows {
    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, size: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_wgpu shadow cascades"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: CASCADES as u32 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let array_view = texture.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let layers = (0..CASCADES as u32)
            .map(|layer| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("render_wgpu cascades"),
            size: CASCADE_STRIDE * CASCADES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_wgpu cascades"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &uniform, offset: 0, size: wgpu::BufferSize::new(64) }),
            }],
        });
        Self { size, _texture: texture, array_view, layers, uniform, uniform_group, cache: [None; CASCADES] }
    }

    /// The bind group layout of the per-cascade matrix (a dynamic offset per cascade).
    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_wgpu cascade"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: wgpu::BufferSize::new(64) },
                count: None,
            }],
        })
    }

    /// The dynamic offset of cascade `i`'s matrix.
    pub fn offset(i: usize) -> u32 {
        (CASCADE_STRIDE * i as u64) as u32
    }

    /// The depth view of cascade `i`, to draw into.
    pub fn layer(&self, i: usize) -> &wgpu::TextureView {
        &self.layers[i]
    }

    /// Whether cascade `i` must be drawn again for `key` (its matrix and casters), and if
    /// so writes its matrix and remembers the key.
    pub fn claim(&mut self, queue: &wgpu::Queue, i: usize, key: u64, matrix: &[[f32; 4]; 4]) -> bool {
        if self.cache[i] == Some(key) {
            return false;
        }
        self.cache[i] = Some(key);
        queue.write_buffer(&self.uniform, CASCADE_STRIDE * i as u64, bytemuck::cast_slice(matrix));
        true
    }

}

/// The cache key of a cascade: its matrix and the casters it covers.
pub(crate) fn cascade_key(matrix: &[[f32; 4]; 4], chunks: impl Iterator<Item = ((i32, i32, i32), u64)>, hidden: u64, materials: u64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for column in matrix {
        for v in column {
            v.to_bits().hash(&mut hasher);
        }
    }
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

    fn setup(camera: &Camera3d) -> Fit {
        let projection = camera.projection_matrix(800, 600);
        fit(camera, &projection, [-0.3, -0.4, -0.866], ([-1024.0, -1024.0, -256.0], [1024.0, 1024.0, 512.0]), 2048, 1.0)
    }

    #[test]
    fn splits_grow_and_cover_the_scene() {
        let camera = Camera3d::default();
        let f = setup(&camera);
        for i in 1..CASCADES {
            assert!(f.splits[i] > f.splits[i - 1]);
            assert!(f.texel[i] > f.texel[i - 1]);
        }
        assert!(f.splits[CASCADES - 1] >= 1024.0, "{:?}", f.splits);
    }

    #[test]
    fn turning_the_camera_keeps_the_cascade_size_and_a_small_move_keeps_the_matrix() {
        let mut camera = Camera3d::default();
        let before = setup(&camera);
        camera.turn(0.7, 0.2);
        let turned = setup(&camera);
        assert_eq!(before.texel, turned.texel, "the cascade size must not depend on the view direction");
        // A move far below a texel of every cascade leaves the snapped matrices alone
        // (unless it happens to cross a texel boundary, which a 1e-4 move does not).
        let mut moved = camera;
        moved.position += Vec3d::new(1e-4, 0.0, 0.0);
        let after = setup(&moved);
        let same = (0..CASCADES).filter(|&i| after.matrices[i] == turned.matrices[i]).count();
        assert!(same >= CASCADES - 1, "{same}");
    }

    #[test]
    fn an_object_moving_the_scene_bounds_a_little_keeps_the_cascades() {
        // An animated lift rising a few units above the rest of the map, frame by frame.
        let camera = Camera3d::default();
        let projection = camera.projection_matrix(800, 600);
        let at = |top: f32| fit(&camera, &projection, [-0.3, -0.4, -0.866], ([-1024.0, -1024.0, -256.0], [1024.0, 1024.0, top]), 2048, 1.0);
        let first = at(512.0);
        for step in 1..8 {
            let later = at(512.0 + step as f32 * 3.0);
            assert_eq!(first.matrices, later.matrices, "step {step}");
            assert_eq!(first.bias, later.bias, "step {step}");
        }
    }

    #[test]
    fn points_in_a_slice_land_inside_its_map() {
        let camera = Camera3d::default();
        let f = setup(&camera);
        let projection = camera.projection_matrix(800, 600);
        let (tx, ty) = (1.0 / projection[0][0], 1.0 / projection[1][1]);
        for i in 0..CASCADES {
            let d = f64::from(f.splits[i]) * 0.99;
            let p = camera.position + camera.direction() * d + camera.right() * (d * tx * 0.9) + camera.up() * (d * ty * 0.9);
            let m = f.matrices[i];
            let x = m[0][0] as f64 * p.x() + m[1][0] as f64 * p.y() + m[2][0] as f64 * p.z() + m[3][0] as f64;
            let y = m[0][1] as f64 * p.x() + m[1][1] as f64 * p.y() + m[2][1] as f64 * p.z() + m[3][1] as f64;
            assert!(x.abs() <= 1.0 && y.abs() <= 1.0, "cascade {i}: {x} {y}");
        }
    }
}
