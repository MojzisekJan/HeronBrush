//! Image-based lighting from the procedural sky. When the sky or the sun changes (and
//! only then), the sky is drawn into a 128² cube map with a box mip chain, prefiltered
//! into a GGX specular cube (one mip per roughness step) and projected onto nine
//! spherical-harmonic coefficients for diffuse. The split-sum DFG table is made once.
//! About 1.5 MB of textures in all.

use std::hash::{Hash, Hasher};

use render::RenderSettings;

const ENV_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// The sky cube's side; the SH pass reads its 16² mip (mip 3).
const ENV_SIZE: u32 = 128;
const ENV_MIPS: u32 = 8;
const SH_SOURCE_MIP: f32 = 3.0;
/// The specular cube: mip `i` holds roughness `i / (SPECULAR_MIPS - 1)`.
const SPECULAR_SIZE: u32 = 128;
pub(crate) const SPECULAR_MIPS: u32 = 6;
const DFG_SIZE: u32 = 64;
const PARAM_STRIDE: u64 = 256;

pub(crate) struct Ibl {
    _env: wgpu::Texture,
    env_faces: Vec<Vec<wgpu::TextureView>>,
    /// Per env mip `i`, a group sampling mip `i` alone (the source of mip `i + 1`).
    env_mip_groups: Vec<wgpu::BindGroup>,
    env_full_group: wgpu::BindGroup,
    /// A group with an unrelated cube, for the capture pass that reads nothing.
    capture_group: wgpu::BindGroup,
    _specular: wgpu::Texture,
    specular_faces: Vec<Vec<wgpu::TextureView>>,
    pub specular_view: wgpu::TextureView,
    _sh: wgpu::Texture,
    sh_target: wgpu::TextureView,
    pub sh_view: wgpu::TextureView,
    _dfg: wgpu::Texture,
    pub dfg_view: wgpu::TextureView,
    _params: wgpu::Buffer,
    capture: wgpu::RenderPipeline,
    downsample: wgpu::RenderPipeline,
    prefilter: wgpu::RenderPipeline,
    sh: wgpu::RenderPipeline,
    key: Option<u64>,
}

fn cube(
    device: &wgpu::Device,
    label: &str,
    size: u32,
    mips: u32,
) -> (wgpu::Texture, Vec<Vec<wgpu::TextureView>>) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        mip_level_count: mips,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: ENV_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let faces = (0..mips)
        .map(|mip| {
            (0..6)
                .map(|face| {
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        label: Some(label),
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_mip_level: mip,
                        mip_level_count: Some(1),
                        base_array_layer: face,
                        array_layer_count: Some(1),
                        ..Default::default()
                    })
                })
                .collect()
        })
        .collect();
    (texture, faces)
}

fn cube_view(texture: &wgpu::Texture, base_mip: u32, mips: Option<u32>) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        base_mip_level: base_mip,
        mip_level_count: mips,
        ..Default::default()
    })
}

/// The settings the sky's light depends on.
fn sky_key(s: &RenderSettings) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for v in s
        .sky_color
        .iter()
        .chain(&s.horizon_color)
        .chain(&s.ground_color)
        .chain(&s.sun_color)
        .chain(&[s.sun_elevation, s.sun_azimuth])
    {
        v.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

impl Ibl {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame_layout: &wgpu::BindGroupLayout,
        module: &wgpu::ShaderModule,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_wgpu ibl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("render_wgpu ibl"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        // One parameter block per (mip, face) of the larger chain, plus the SH pass.
        let entries = (ENV_MIPS * 6 + 1) as u64;
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("render_wgpu ibl params"),
            size: entries * PARAM_STRIDE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (env, env_faces) = cube(device, "render_wgpu sky cube", ENV_SIZE, ENV_MIPS);
        let (specular, specular_faces) = cube(
            device,
            "render_wgpu specular cube",
            SPECULAR_SIZE,
            SPECULAR_MIPS,
        );
        let specular_view = cube_view(&specular, 0, None);
        let group = |view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("render_wgpu ibl"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &params,
                            offset: 0,
                            size: wgpu::BufferSize::new(16),
                        }),
                    },
                ],
            })
        };
        let env_mip_groups = (0..ENV_MIPS)
            .map(|mip| group(&cube_view(&env, mip, Some(1))))
            .collect();
        let env_full_group = group(&cube_view(&env, 0, None));
        let capture_group = group(&specular_view);
        let target = |label: &str, format, width, height| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let sh = target("render_wgpu sky SH", wgpu::TextureFormat::Rgba32Float, 9, 1);
        let sh_view = sh.create_view(&Default::default());
        let sh_target = sh.create_view(&Default::default());
        let dfg = target(
            "render_wgpu DFG",
            wgpu::TextureFormat::Rg16Float,
            DFG_SIZE,
            DFG_SIZE,
        );
        let dfg_view = dfg.create_view(&Default::default());

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_wgpu ibl"),
            bind_group_layouts: &[Some(frame_layout), Some(&layout)],
            immediate_size: 0,
        });
        let make = |label: &str,
                    fs: &str,
                    format: wgpu::TextureFormat,
                    layout: Option<&wgpu::PipelineLayout>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout,
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_ibl"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let capture = make(
            "sky capture",
            "fs_capture",
            ENV_FORMAT,
            Some(&pipeline_layout),
        );
        let downsample = make(
            "sky downsample",
            "fs_downsample",
            ENV_FORMAT,
            Some(&pipeline_layout),
        );
        let prefilter = make(
            "sky prefilter",
            "fs_prefilter",
            ENV_FORMAT,
            Some(&pipeline_layout),
        );
        let sh_pipeline = make(
            "sky SH",
            "fs_sh",
            wgpu::TextureFormat::Rgba32Float,
            Some(&pipeline_layout),
        );
        let empty = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_wgpu dfg"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let dfg_pipeline = make(
            "DFG",
            "fs_dfg",
            wgpu::TextureFormat::Rg16Float,
            Some(&empty),
        );

        // The DFG table depends on nothing: made here, once.
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_wgpu dfg"),
        });
        {
            let mut pass = begin(&mut encoder, "dfg", &dfg_view);
            pass.set_pipeline(&dfg_pipeline);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);

        // The parameter blocks never change either: (face, roughness, source size, mips)
        // for entry mip * 6 + face; the last is the SH pass's source mip.
        let mut blocks = vec![0u8; (entries * PARAM_STRIDE) as usize];
        for mip in 0..ENV_MIPS {
            for face in 0..6u32 {
                let roughness = if mip < SPECULAR_MIPS {
                    mip as f32 / (SPECULAR_MIPS - 1) as f32
                } else {
                    0.0
                };
                let values = [face as f32, roughness, ENV_SIZE as f32, ENV_MIPS as f32];
                let at = ((mip * 6 + face) as u64 * PARAM_STRIDE) as usize;
                blocks[at..at + 16].copy_from_slice(bytemuck::cast_slice(&values));
            }
        }
        let at = (ENV_MIPS as u64 * 6 * PARAM_STRIDE) as usize;
        blocks[at..at + 16].copy_from_slice(bytemuck::cast_slice(&[
            0.0f32,
            0.0,
            ENV_SIZE as f32,
            SH_SOURCE_MIP,
        ]));
        queue.write_buffer(&params, 0, &blocks);

        Self {
            _env: env,
            env_faces,
            env_mip_groups,
            env_full_group,
            capture_group,
            _specular: specular,
            specular_faces,
            specular_view,
            _sh: sh,
            sh_target,
            sh_view,
            _dfg: dfg,
            dfg_view,
            _params: params,
            capture,
            downsample,
            prefilter,
            sh: sh_pipeline,
            key: None,
        }
    }

    /// Redraws the environment when the sky or sun changed. The frame uniform must
    /// already hold this frame's sky. Returns whether it did any work.
    pub fn update(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame_group: &wgpu::BindGroup,
        settings: &RenderSettings,
    ) -> bool {
        let key = sky_key(settings);
        if self.key == Some(key) {
            return false;
        }
        self.key = Some(key);
        let offset = |mip: u32, face: u32| ((mip * 6 + face) as u64 * PARAM_STRIDE) as u32;
        for face in 0..6 {
            let mut pass = begin(encoder, "sky capture", &self.env_faces[0][face as usize]);
            pass.set_pipeline(&self.capture);
            pass.set_bind_group(0, frame_group, &[]);
            pass.set_bind_group(1, &self.capture_group, &[offset(0, face)]);
            pass.draw(0..3, 0..1);
        }
        for mip in 1..ENV_MIPS {
            for face in 0..6 {
                let mut pass = begin(
                    encoder,
                    "sky downsample",
                    &self.env_faces[mip as usize][face as usize],
                );
                pass.set_pipeline(&self.downsample);
                pass.set_bind_group(0, frame_group, &[]);
                pass.set_bind_group(
                    1,
                    &self.env_mip_groups[mip as usize - 1],
                    &[offset(mip, face)],
                );
                pass.draw(0..3, 0..1);
            }
        }
        for mip in 0..SPECULAR_MIPS {
            for face in 0..6 {
                let mut pass = begin(
                    encoder,
                    "sky prefilter",
                    &self.specular_faces[mip as usize][face as usize],
                );
                pass.set_pipeline(&self.prefilter);
                pass.set_bind_group(0, frame_group, &[]);
                pass.set_bind_group(1, &self.env_full_group, &[offset(mip, face)]);
                pass.draw(0..3, 0..1);
            }
        }
        let mut pass = begin(encoder, "sky SH", &self.sh_target);
        pass.set_pipeline(&self.sh);
        pass.set_bind_group(0, frame_group, &[]);
        pass.set_bind_group(1, &self.env_full_group, &[offset(ENV_MIPS, 0)]);
        pass.draw(0..3, 0..1);
        true
    }
}

fn begin<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}
