//! Small fixed-capacity GPU cache for immutable local lighting bakes.
use bytemuck::{Pod, Zeroable};
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ProbeGpu {
    position: [f32; 4],
    half_size: [f32; 4],
    params: [f32; 4],
    sh: [[f32; 4]; 9],
}
pub(crate) struct Probes {
    id: Option<u64>,
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub buffer: wgpu::Buffer,
}
impl Probes {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("local reflection probes"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 48,
            },
            mip_level_count: 7,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::CubeArray),
            ..Default::default()
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("local probe volumes and SH"),
            size: (std::mem::size_of::<ProbeGpu>() * 8) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            id: None,
            texture,
            view,
            buffer,
        }
    }
    pub fn needs_update(&self, lighting: Option<&render::probes::Lighting>) -> bool {
        self.id != lighting.map(render::probes::Lighting::id)
    }
    pub fn prepare(&mut self, queue: &wgpu::Queue, lighting: Option<&render::probes::Lighting>) {
        if !self.needs_update(lighting) {
            return;
        }
        self.id = lighting.map(render::probes::Lighting::id);
        let Some(lighting) = lighting else {
            return;
        };
        let mut data = [ProbeGpu::zeroed(); 8];
        for (index, probe) in lighting.probes().iter().enumerate() {
            let volume = &probe.volume;
            data[index] = ProbeGpu {
                position: [
                    volume.position[0],
                    volume.position[1],
                    volume.position[2],
                    volume.intensity,
                ],
                half_size: [
                    volume.half_size[0],
                    volume.half_size[1],
                    volume.half_size[2],
                    volume.blend,
                ],
                params: [
                    f32::from(u8::from(volume.reflections)),
                    f32::from(u8::from(volume.diffuse)),
                    probe.size.ilog2() as f32,
                    0.0,
                ],
                sh: std::array::from_fn(|i| [probe.sh[i][0], probe.sh[i][1], probe.sh[i][2], 0.0]),
            };
            for level in 0..7 {
                let side = 64u32 >> level;
                let source_level = (level as usize).min(probe.mips.len() - 1);
                let source_side = (probe.size >> source_level).max(1);
                let source = &probe.mips[source_level];
                let mut bytes = Vec::with_capacity((side * side * 6 * 8) as usize);
                for face in 0..6 {
                    for y in 0..side {
                        for x in 0..side {
                            let sx = ((x as f32 + 0.5) / side as f32 * source_side as f32 - 0.5)
                                .clamp(0.0, source_side as f32 - 1.0);
                            let sy = ((y as f32 + 0.5) / side as f32 * source_side as f32 - 0.5)
                                .clamp(0.0, source_side as f32 - 1.0);
                            let (ix, iy) = (sx.floor() as u32, sy.floor() as u32);
                            let (fx, fy) = (sx.fract(), sy.fract());
                            let at = |dx: u32, dy: u32| {
                                source[(face * source_side * source_side
                                    + (iy + dy).min(source_side - 1) * source_side
                                    + (ix + dx).min(source_side - 1))
                                    as usize]
                            };
                            let (a, b, c, d) = (at(0, 0), at(1, 0), at(0, 1), at(1, 1));
                            for ((a, b), (c, d)) in a.into_iter().zip(b).zip(c.into_iter().zip(d)) {
                                let value = (a * (1.0 - fx) + b * fx) * (1.0 - fy)
                                    + (c * (1.0 - fx) + d * fx) * fy;
                                bytes.extend_from_slice(&half::f16::from_f32(value).to_le_bytes());
                            }
                        }
                    }
                }
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.texture,
                        mip_level: level,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: index as u32 * 6,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &bytes,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(side * 8),
                        rows_per_image: Some(side),
                    },
                    wgpu::Extent3d {
                        width: side,
                        height: side,
                        depth_or_array_layers: 6,
                    },
                );
            }
        }
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&data));
    }
}
