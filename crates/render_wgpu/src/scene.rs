//! The retained render world: objects in spatial chunks, the object and material tables,
//! and the material textures. Fed by [`editor_core::render_scene::SceneSink`] deltas;
//! GPU writes happen in [`RenderScene::finish`], touching only dirty chunks and object
//! slots.

use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use editor_core::render_scene::{
    flags_of, triangle_flags, ObjectKey, ObjectState, SceneLight, SceneSink,
};
use map_model::brush_face::SurfaceMaterial;
use map_model::material::{Alpha, Material};
use render::Triangle;

use crate::textures::{self, Decoded, Mip, Workers};

/// Compact indexed vertex: 32 bytes.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct Vertex {
    /// World position.
    pub position: [f32; 3],
    /// Face normal, snorm8 xyz + pad.
    pub normal: [i8; 4],
    /// Texture coordinates in texels.
    pub uv: [f32; 2],
    /// sRGB colour and alpha, unorm8.
    pub color: [u8; 4],
    /// Object slot in the low 24 bits, per-vertex flags in the top 8.
    pub object: u32,
}

impl Vertex {
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x3, 1 => Snorm8x4, 2 => Float32x2, 3 => Unorm8x4, 4 => Uint32
    ];
}

/// One entry of the object table, 16 bytes.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, PartialEq)]
pub struct ObjectGpu {
    flags: u32,
    tint: u32,
    pad: [u32; 2],
}

/// One entry of the material table, 64 bytes.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct MaterialGpu {
    size: [f32; 2],
    alpha_mode: u32,
    flags: u32,
    factors: [f32; 4],
    model_params: [f32; 4],
    model_emissive: [f32; 4],
}

/// Which pass a run of triangles belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Phase {
    /// Depth-tested and written, shaded once.
    Opaque,
    /// As opaque, with clear texels discarded.
    Masked,
    /// Alpha-blended after the opaque scene, not in the prepass.
    Blended,
}

/// A material instance: a texture (or none) with per-face PBR factors. Its slot in the
/// material table is what draws pass as their instance index.
type InstanceKey = (Option<usize>, Option<SurfaceMaterial>);

/// Material-table slot 0 is "no texture", 1 the skip hatching; real instances follow.
const SLOT_NONE: u32 = 0;
const SLOT_SKIP: u32 = 1;

/// The side of a chunk's grid cell, in map units.
pub const CHUNK_SIZE: f64 = 1024.0;

type ChunkKey = (i32, i32, i32);

struct Object {
    slot: u32,
    chunk: ChunkKey,
    vertices: Vec<Vertex>,
    /// Where each vertex was before the last upsert moved it (same vertices, same
    /// triangles, other positions: the animation preview or a drag), for motion vectors;
    /// `None` while the object is still.
    previous: Option<Vec<[f32; 3]>>,
    /// The sync ([`RenderScene::sync_epoch`]) that moved it.
    moved: u64,
    /// Local indices grouped per (see-through paint, material slot). The phase is worked
    /// out when the chunk is built, from what is known of the material's alpha then: a
    /// texture whose alpha channel turns out to cut or blend, once decoded, moves its
    /// draws without the triangles being sent again.
    batches: Vec<((bool, u32), Vec<u32>)>,
}

/// One draw: an index range of a chunk, with one material instance.
#[derive(Clone, Debug)]
pub struct Draw {
    /// Its pass.
    pub phase: Phase,
    /// Material-table slot, passed as the instance index.
    pub material: u32,
    /// Index range in the chunk's index buffer.
    pub indices: std::ops::Range<u32>,
    /// The prepass needs this material for normal mapping or imported face rules.
    pub prepass_material: bool,
}

/// A spatial chunk with its own vertex and index buffers.
pub struct Chunk {
    objects: HashSet<ObjectKey>,
    pub(crate) vertex_buffer: Option<wgpu::Buffer>,
    pub(crate) index_buffer: Option<wgpu::Buffer>,
    pub(crate) wire_buffer: Option<wgpu::Buffer>,
    /// The vertices where they were last frame (only while an object in the chunk
    /// moves), read by the prepass for motion vectors in place of the vertex buffer.
    pub(crate) previous_buffer: Option<wgpu::Buffer>,
    pub(crate) draws: Vec<Draw>,
    /// The index range of every opaque draw (they are contiguous): what a depth-only
    /// pass draws in one call when no imported single-sided material needs a
    /// per-material back-face test.
    pub(crate) opaque: std::ops::Range<u32>,
    /// Imported opaque casters need their own material slot for back-face tests.
    pub(crate) single_sided_opaque: bool,
    /// The leading part of `opaque` whose materials have no normal map: the prepass
    /// draws it in one call too, its normals being the geometry's own.
    pub(crate) plain: std::ops::Range<u32>,
    pub(crate) wire_count: u32,
    pub(crate) bounds: ([f32; 3], [f32; 3]),
    /// Changes whenever the chunk's geometry is rebuilt, so a cached shadow cascade can
    /// tell whether what it drew is still what the chunk holds.
    pub(crate) generation: u64,
    dirty: bool,
}

/// GPU textures for one base material: albedo and, when the material has one, its
/// tangent-space normal map (else the shared flat one).
struct MaterialTexture {
    bind_group: wgpu::BindGroup,
    _texture: wgpu::Texture,
    _normal: Option<wgpu::Texture>,
    _orm: Option<wgpu::Texture>,
    _emissive: Option<wgpu::Texture>,
}

/// Material-table flags, as `common.wgsl` reads them.
const MATERIAL_OWN_FACTORS: u32 = 1;
const MATERIAL_SKIP: u32 = 2;
const MATERIAL_NORMAL_MAP: u32 = 4;
const MATERIAL_NORMAL_FLIP_Y: u32 = 8;
/// The texture is not resident: draw the average colour packed into `factors[3]`.
const MATERIAL_PENDING: u32 = 16;
const MATERIAL_MODEL_PBR: u32 = 32;

/// Bytes of texture uploaded per frame at most, so streaming in a view full of new
/// textures never stalls a frame for long (about 4 ms on the M4).
const UPLOAD_BUDGET: u64 = 48 << 20;

/// What a renderer holds of one base material's texture.
enum Residency {
    /// Not asked for yet (or evicted).
    Pending,
    /// With a worker.
    Queued,
    /// On the GPU, taking this many bytes. Boxed so the other states, which most
    /// materials are in, stay small.
    Resident(Box<MaterialTexture>, u64),
    /// Did not decode; drawn with its placeholder.
    Failed,
}

/// One point-entity sprite as the billboard pass reads it, 32 bytes.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct SpriteGpu {
    position: [f32; 3],
    half_width: f32,
    half_height: f32,
    pad: [f32; 3],
}

/// What the last [`RenderScene::finish`] wrote, for profiling.
#[derive(Debug, Default, Clone, Copy)]
pub struct UploadStats {
    /// Chunks rebuilt and re-uploaded.
    pub chunks: usize,
    /// Bytes written to vertex and index buffers.
    pub geometry_bytes: usize,
    /// Object-table slots written.
    pub object_slots: usize,
}

/// What texture streaming did in the last frame, and where it stands.
#[derive(Debug, Default, Clone, Copy)]
pub struct TextureStats {
    /// Textures made resident this frame.
    pub uploaded: usize,
    /// Bytes uploaded this frame.
    pub uploaded_bytes: u64,
    /// Textures evicted this frame.
    pub evicted: usize,
    /// Resident textures.
    pub resident: usize,
    /// The bytes they take.
    pub resident_bytes: u64,
    /// Textures queued with the workers or waiting for upload.
    pub waiting: usize,
}

/// The retained scene.
pub struct RenderScene {
    device: wgpu::Device,
    queue: wgpu::Queue,
    objects: HashMap<ObjectKey, Object>,
    pub(crate) chunks: HashMap<ChunkKey, Chunk>,
    object_table: Vec<ObjectGpu>,
    free_slots: Vec<u32>,
    dirty_slots: Vec<u32>,
    pub(crate) object_buffer: wgpu::Buffer,
    // Material instances and their table.
    instances: HashMap<InstanceKey, u32>,
    instance_base: Vec<Option<usize>>,
    material_table: Vec<MaterialGpu>,
    material_table_dirty: bool,
    pub(crate) material_buffer: wgpu::Buffer,
    /// The per-face PBR factors of each instance slot (beside `instance_base`), so a slot
    /// can be written again when its texture's residency changes.
    instance_surface: Vec<Option<SurfaceMaterial>>,
    // Base materials as loaded.
    materials: Vec<Material>,
    material_sizes: Vec<(f32, f32)>,
    /// What is known of each base material's alpha (a guess until decoded).
    material_alpha: Vec<Alpha>,
    /// Per base material: has a resident normal map, and whether its green channel is
    /// flipped.
    material_normals: Vec<Option<bool>>,
    /// Per base material: the average colour (sRGB + alpha, packed), the placeholder.
    material_average: Vec<u32>,
    textures: Vec<Residency>,
    /// The frame each base material was last drawn in, for eviction.
    last_used: Vec<u64>,
    frame: u64,
    resident_bytes: u64,
    /// Bumped by `set_materials`, so decodes of a replaced collection are dropped.
    texture_epoch: u64,
    workers: Option<Workers>,
    /// GPU memory the material textures may take before the least recently drawn are
    /// dropped.
    pub texture_budget: u64,
    /// Bumped whenever a texture becomes resident or is evicted: what the picture caches
    /// key on.
    pub(crate) textures_generation: u64,
    /// What streaming did in the last frame.
    pub texture_stats: TextureStats,
    white: MaterialTexture,
    flat_normal: wgpu::TextureView,
    /// Point-entity sprites, sorted by material, and one draw range per material.
    pub(crate) sprite_buffer: Option<wgpu::Buffer>,
    pub(crate) sprite_draws: Vec<(usize, std::ops::Range<u32>)>,
    /// The light entities' lights, and a generation bumped when they change.
    pub(crate) lights: Vec<SceneLight>,
    pub(crate) lights_generation: u64,
    /// Hands out chunk generations.
    next_generation: u64,
    /// Bumped whenever an object is added, changed or removed (not when only its motion
    /// stops), so a frame can tell the geometry changed.
    pub(crate) geometry_generation: u64,
    /// Objects that moved in the last sync (they have `previous` positions).
    moving: HashSet<ObjectKey>,
    /// Counts syncs that reached [`RenderScene::finish`].
    pub(crate) sync_epoch: u64,
    /// Bumped when any object's hidden flag changes: hidden objects cast no shadow.
    pub(crate) hidden_generation: u64,
    /// Bumped when the material collection is replaced (masked casters change).
    pub(crate) materials_generation: u64,
    pub(crate) texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Bumped whenever a table buffer is replaced, so frame bind groups are rebuilt.
    pub(crate) buffers_generation: u64,
    /// What the last finish wrote.
    pub stats: UploadStats,
}

fn storage_buffer(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(256),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

impl RenderScene {
    /// An empty scene on `device`.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_wgpu material texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("render_wgpu material sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        // Straight up in tangent space: what a material without a normal map samples.
        let flat_normal = upload_mips(
            device,
            queue,
            wgpu::TextureFormat::Rgba8Unorm,
            textures::mip_chain(1, 1, &[128, 128, 255, 255], false),
            "flat normal",
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let white = make_texture(
            device,
            queue,
            &texture_layout,
            &sampler,
            &flat_normal,
            textures::mip_chain(1, 1, &[255, 255, 255, 255], true),
            None,
            None,
            "white",
        );
        let mut scene = Self {
            device: device.clone(),
            queue: queue.clone(),
            objects: HashMap::new(),
            chunks: HashMap::new(),
            object_table: Vec::new(),
            free_slots: Vec::new(),
            dirty_slots: Vec::new(),
            object_buffer: storage_buffer(device, "render_wgpu objects", 256),
            instances: HashMap::new(),
            instance_base: Vec::new(),
            instance_surface: Vec::new(),
            material_table: Vec::new(),
            material_table_dirty: true,
            material_buffer: storage_buffer(device, "render_wgpu materials", 256),
            materials: Vec::new(),
            material_sizes: Vec::new(),
            material_alpha: Vec::new(),
            material_normals: Vec::new(),
            material_average: Vec::new(),
            textures: Vec::new(),
            last_used: Vec::new(),
            frame: 0,
            resident_bytes: 0,
            texture_epoch: 0,
            workers: None,
            texture_budget: DEFAULT_TEXTURE_BUDGET,
            textures_generation: 0,
            texture_stats: TextureStats::default(),
            white,
            flat_normal,
            sprite_buffer: None,
            sprite_draws: Vec::new(),
            lights: Vec::new(),
            lights_generation: 0,
            next_generation: 0,
            geometry_generation: 0,
            moving: HashSet::new(),
            sync_epoch: 0,
            hidden_generation: 0,
            materials_generation: 0,
            texture_layout,
            sampler,
            buffers_generation: 0,
            stats: UploadStats::default(),
        };
        scene.reset_instances();
        scene
    }

    fn reset_instances(&mut self) {
        self.instances.clear();
        self.instance_base = vec![None, None];
        self.instance_surface = vec![None, None];
        self.material_table = vec![
            MaterialGpu {
                size: [1.0, 1.0],
                alpha_mode: 0,
                flags: 0,
                factors: [0.0; 4],
                model_params: [1., 0., 0.5, 0.],
                model_emissive: [0.; 4],
            },
            MaterialGpu {
                size: [1.0, 1.0],
                alpha_mode: 2,
                flags: MATERIAL_SKIP,
                factors: [0.0; 4],
                model_params: [1., 0., 0.5, 0.],
                model_emissive: [0.; 4],
            },
        ];
        self.material_table_dirty = true;
    }

    /// The material instance slot for a triangle, interned.
    fn instance(&mut self, triangle: &Triangle) -> u32 {
        if triangle.material == Some(render::SKIP_PREVIEW_MATERIAL) {
            return SLOT_SKIP;
        }
        let material = triangle.material.filter(|m| *m < self.material_sizes.len());
        if material.is_none() && triangle.surface_material.is_none() {
            return SLOT_NONE;
        }
        let key = (material, triangle.surface_material);
        if let Some(slot) = self.instances.get(&key) {
            return *slot;
        }
        let slot = self.material_table.len() as u32;
        self.material_table
            .push(self.material_entry(material, triangle.surface_material));
        self.instance_base.push(material);
        self.instance_surface.push(triangle.surface_material);
        self.instances.insert(key, slot);
        self.material_table_dirty = true;
        slot
    }

    /// The material-table entry of an instance, from what is known of its base material
    /// now: its size and alpha, its normal map and whether its texture is resident.
    fn material_entry(
        &self,
        material: Option<usize>,
        surface: Option<SurfaceMaterial>,
    ) -> MaterialGpu {
        let material = material.filter(|m| *m < self.materials.len());
        let resident =
            material.is_none_or(|m| matches!(self.textures.get(m), Some(Residency::Resident(..))));
        let (size, alpha_mode) = match material {
            // A texture that is not there cannot cut or blend anything.
            Some(m) if resident => (
                self.material_sizes[m],
                match self.material_alpha[m] {
                    Alpha::Opaque => 0,
                    Alpha::Masked => 1,
                    Alpha::Blended => 2,
                },
            ),
            Some(m) => (self.material_sizes[m], 0),
            None => ((1.0, 1.0), 0),
        };
        let (mut flags, mut factors) = match surface {
            Some(s) => {
                let [r, m, e] = s.factors();
                (MATERIAL_OWN_FACTORS, [r, m, e, 0.0])
            }
            None => (0, [0.0; 4]),
        };
        if let Some(flip_y) = material
            .and_then(|m| self.material_normals[m])
            .filter(|_| resident)
        {
            flags |= MATERIAL_NORMAL_MAP | if flip_y { MATERIAL_NORMAL_FLIP_Y } else { 0 };
        }
        if let Some(m) = material.filter(|_| !resident) {
            flags |= MATERIAL_PENDING;
            factors[3] = f32::from_bits(self.material_average[m]);
        }
        let mut model_params = [1., 0., 0.5, 0.];
        let mut model_emissive = [0.; 4];
        if let Some(pbr) = material.and_then(|m| self.materials[m].model_pbr.as_ref()) {
            if resident {
                flags |= MATERIAL_MODEL_PBR;
            }
            if surface.is_none() {
                flags |= MATERIAL_OWN_FACTORS;
                factors[0] = pbr.roughness;
                factors[1] = pbr.metallic;
            }
            model_params = [
                pbr.normal_scale,
                pbr.occlusion_strength,
                pbr.alpha_cutoff,
                if pbr.double_sided { 2. } else { 1. },
            ];
            if resident {
                model_emissive[..3].copy_from_slice(&pbr.emissive_factor);
            }
        }
        MaterialGpu {
            size: [size.0, size.1],
            alpha_mode,
            flags,
            factors,
            model_params,
            model_emissive,
        }
    }

    /// Writes again the table entries of every instance of base material `material`.
    fn refresh_instances(&mut self, material: usize) {
        for slot in 0..self.instance_base.len() {
            if self.instance_base[slot] == Some(material) {
                self.material_table[slot] =
                    self.material_entry(Some(material), self.instance_surface[slot]);
                self.material_table_dirty = true;
            }
        }
    }

    pub(crate) fn single_sided(&self, slot: u32) -> bool {
        self.material_table
            .get(slot as usize)
            .is_some_and(|material| material.model_params[3] == 1.0)
    }

    /// Whether the material of an instance slot has a normal map: from the collection's
    /// metadata, not the texture's residency, so a chunk's draw order does not change as
    /// textures stream in and out.
    fn normal_mapped(&self, slot: u32) -> bool {
        self.instance_base
            .get(slot as usize)
            .copied()
            .flatten()
            .is_some_and(|m| {
                self.materials
                    .get(m)
                    .is_some_and(|m| m.normal_map.is_some())
            })
    }

    /// The pass a batch is drawn in, from what is known of its material now.
    fn phase_of(&self, translucent: bool, slot: u32) -> Phase {
        if slot == SLOT_SKIP {
            return Phase::Blended;
        }
        let alpha = self
            .instance_base
            .get(slot as usize)
            .copied()
            .flatten()
            .and_then(|m| self.material_alpha.get(m).copied())
            .unwrap_or(Alpha::Opaque);
        if alpha == Alpha::Blended || translucent {
            Phase::Blended
        } else if alpha == Alpha::Masked {
            Phase::Masked
        } else {
            Phase::Opaque
        }
    }

    /// The bind group with the textures of a base material (an index into the
    /// document's collection), for sprites.
    pub(crate) fn base_texture_bind_group(&self, material: usize) -> Option<&wgpu::BindGroup> {
        match self.textures.get(material) {
            Some(Residency::Resident(texture, _)) => Some(&texture.bind_group),
            _ => None,
        }
    }

    /// The bind group with the texture a material slot samples.
    pub(crate) fn texture_bind_group(&self, slot: u32) -> &wgpu::BindGroup {
        self.instance_base
            .get(slot as usize)
            .copied()
            .flatten()
            .and_then(|m| self.base_texture_bind_group(m))
            .unwrap_or(&self.white.bind_group)
    }

    fn allocate_slot(&mut self) -> u32 {
        if let Some(slot) = self.free_slots.pop() {
            self.dirty_slots.push(slot);
            return slot;
        }
        self.object_table.push(ObjectGpu::default());
        let slot = (self.object_table.len() - 1) as u32;
        // The GPU copy may hold whatever a cleared scene left there.
        self.dirty_slots.push(slot);
        slot
    }

    fn write_state(&mut self, slot: u32, state: ObjectState) {
        let flags = u32::from(state.selected)
            | (u32::from(state.locked) << 1)
            | (u32::from(state.hidden) << 2);
        let entry = &mut self.object_table[slot as usize];
        if (entry.flags ^ flags) & 4 != 0 {
            self.hidden_generation += 1;
        }
        if entry.flags != flags {
            entry.flags = flags;
            self.dirty_slots.push(slot);
        }
    }

    fn mark_chunk(&mut self, key: ChunkKey) -> &mut Chunk {
        let chunk = self.chunks.entry(key).or_insert_with(|| Chunk {
            objects: HashSet::new(),
            vertex_buffer: None,
            index_buffer: None,
            wire_buffer: None,
            previous_buffer: None,
            draws: Vec::new(),
            opaque: 0..0,
            single_sided_opaque: false,
            plain: 0..0,
            wire_count: 0,
            bounds: ([0.0; 3], [0.0; 3]),
            generation: 0,
            dirty: true,
        });
        chunk.dirty = true;
        chunk
    }

    /// Number of objects held.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Whether objects moved in the last sync, so the next frame has motion vectors.
    pub fn moving(&self) -> bool {
        !self.moving.is_empty()
    }

    /// Forgets the motion of every object moved before the current sync (all of them
    /// with `all`): they have stopped, so their motion vectors go back to zero. Their
    /// chunks are rebuilt by the caller.
    fn stop_moving(&mut self, all: bool) {
        let epoch = self.sync_epoch;
        let stopped: Vec<ObjectKey> = self
            .moving
            .iter()
            .filter(|k| all || self.objects.get(k).is_none_or(|o| o.moved != epoch))
            .copied()
            .collect();
        for key in stopped {
            self.moving.remove(&key);
            if let Some(object) = self.objects.get_mut(&key) {
                object.previous = None;
                let chunk = object.chunk;
                self.mark_chunk(chunk);
            }
        }
    }

    /// A frame is drawn without a sync since the last one: whatever moved then has
    /// stopped. Rebuilds the chunks of objects that were moving. Returns whether any were.
    pub(crate) fn settle(&mut self) -> bool {
        if self.moving.is_empty() {
            return false;
        }
        self.stop_moving(true);
        self.rebuild_dirty(&mut UploadStats::default());
        true
    }

    fn rebuild_dirty(&mut self, stats: &mut UploadStats) {
        let dirty: Vec<ChunkKey> = self
            .chunks
            .iter()
            .filter(|(_, c)| c.dirty)
            .map(|(k, _)| *k)
            .collect();
        for key in dirty {
            if self.chunks.get(&key).is_some_and(|c| c.objects.is_empty()) {
                self.chunks.remove(&key);
                continue;
            }
            stats.geometry_bytes += self.rebuild_chunk(key);
            stats.chunks += 1;
        }
    }

    fn rebuild_chunk(&mut self, key: ChunkKey) -> usize {
        let Some(chunk) = self.chunks.get(&key) else {
            return 0;
        };
        let mut vertices: Vec<Vertex> = Vec::new();
        let moving = chunk
            .objects
            .iter()
            .any(|k| self.objects.get(k).is_some_and(|o| o.previous.is_some()));
        let mut previous: Vec<Vertex> = Vec::new();
        // Sort material-independent opaque prepass draws together. Imported face rules
        // need their own material slot even when they have no normal map.
        let mut grouped: std::collections::BTreeMap<(Phase, bool, u32), Vec<u32>> =
            std::collections::BTreeMap::new();
        let phase_of = |(translucent, slot): (bool, u32)| {
            let imported_faces = self
                .material_table
                .get(slot as usize)
                .is_some_and(|material| material.model_params[3] != 0.0);
            (
                self.phase_of(translucent, slot),
                self.normal_mapped(slot) || imported_faces,
                slot,
            )
        };
        let mut wire: Vec<u32> = Vec::new();
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for object_key in &chunk.objects {
            let Some(object) = self.objects.get(object_key) else {
                continue;
            };
            let base = vertices.len() as u32;
            for v in &object.vertices {
                for axis in 0..3 {
                    lo[axis] = lo[axis].min(v.position[axis]);
                    hi[axis] = hi[axis].max(v.position[axis]);
                }
            }
            vertices.extend_from_slice(&object.vertices);
            if moving {
                match &object.previous {
                    Some(positions) => previous.extend(
                        object
                            .vertices
                            .iter()
                            .zip(positions)
                            .map(|(v, p)| Vertex { position: *p, ..*v }),
                    ),
                    None => previous.extend_from_slice(&object.vertices),
                }
            }
            for (batch, indices) in &object.batches {
                let out = grouped.entry(phase_of(*batch)).or_default();
                out.extend(indices.iter().map(|i| i + base));
                for tri in indices.as_chunks::<3>().0 {
                    let [a, b, c] = [tri[0] + base, tri[1] + base, tri[2] + base];
                    wire.extend_from_slice(&[a, b, b, c, c, a]);
                }
            }
        }
        let mut indices: Vec<u32> = Vec::new();
        let mut draws = Vec::new();
        let (mut opaque, mut plain) = (0..0, 0..0);
        for ((phase, prepass_material, material), list) in grouped {
            let start = indices.len() as u32;
            indices.extend_from_slice(&list);
            let end = indices.len() as u32;
            if phase == Phase::Opaque {
                opaque = if opaque.is_empty() {
                    start..end
                } else {
                    opaque.start..end
                };
                if !prepass_material {
                    plain = if plain.is_empty() {
                        start..end
                    } else {
                        plain.start..end
                    };
                }
            }
            draws.push(Draw {
                phase,
                material,
                indices: start..end,
                prepass_material,
            });
        }
        let single_sided_opaque = draws
            .iter()
            .any(|draw| draw.phase == Phase::Opaque && self.single_sided(draw.material));
        let device = self.device.clone();
        let queue = self.queue.clone();
        let chunk = self.chunks.get_mut(&key).expect("present");
        let mut bytes = 0;
        bytes += upload(
            &device,
            &queue,
            &mut chunk.vertex_buffer,
            bytemuck::cast_slice(&vertices),
            wgpu::BufferUsages::VERTEX,
            "chunk vertices",
        );
        bytes += upload(
            &device,
            &queue,
            &mut chunk.index_buffer,
            bytemuck::cast_slice(&indices),
            wgpu::BufferUsages::INDEX,
            "chunk indices",
        );
        bytes += upload(
            &device,
            &queue,
            &mut chunk.wire_buffer,
            bytemuck::cast_slice(&wire),
            wgpu::BufferUsages::INDEX,
            "chunk wire",
        );
        if moving {
            bytes += upload(
                &device,
                &queue,
                &mut chunk.previous_buffer,
                bytemuck::cast_slice(&previous),
                wgpu::BufferUsages::VERTEX,
                "chunk previous vertices",
            );
        } else {
            chunk.previous_buffer = None;
        }
        chunk.draws = draws;
        chunk.opaque = opaque;
        chunk.single_sided_opaque = single_sided_opaque;
        chunk.plain = plain;
        chunk.wire_count = wire.len() as u32;
        chunk.bounds = (lo, hi);
        self.next_generation += 1;
        chunk.generation = self.next_generation;
        chunk.dirty = false;
        bytes
    }
}

/// Writes `data` into `buffer`, growing it (by half again) when it does not fit.
fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut Option<wgpu::Buffer>,
    data: &[u8],
    usage: wgpu::BufferUsages,
    label: &str,
) -> usize {
    if data.is_empty() {
        return 0;
    }
    let needed = (data.len() as u64).next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT);
    if buffer.as_ref().is_none_or(|b| b.size() < needed) {
        *buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (needed + needed / 2).next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT),
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }
    let buffer = buffer.as_ref().expect("just made");
    if (data.len() as u64).is_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT) {
        queue.write_buffer(buffer, 0, data);
    } else {
        let mut padded = data.to_vec();
        padded.resize(needed as usize, 0);
        queue.write_buffer(buffer, 0, &padded);
    }
    data.len()
}

fn upload_mips(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    mips: Vec<(u32, u32, Vec<u8>)>,
    label: &str,
) -> wgpu::Texture {
    let (width, height) = (mips[0].0, mips[0].1);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: mips.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (w, h, data)) in mips.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(*h),
            },
            wgpu::Extent3d {
                width: *w,
                height: *h,
                depth_or_array_layers: 1,
            },
        );
    }
    texture
}

fn make_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    flat_normal: &wgpu::TextureView,
    albedo: Vec<Mip>,
    normal: Option<Vec<Mip>>,
    pbr: Option<(Vec<Mip>, Vec<Mip>)>,
    label: &str,
) -> MaterialTexture {
    let texture = upload_mips(
        device,
        queue,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        albedo,
        label,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    // Normal maps are linear data: their mips are plain averages, not sRGB-weighted.
    let normal =
        normal.map(|mips| upload_mips(device, queue, wgpu::TextureFormat::Rgba8Unorm, mips, label));
    let normal_view = normal
        .as_ref()
        .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));
    let (orm, emissive) = if let Some((orm, e)) = pbr {
        (
            Some(upload_mips(
                device,
                queue,
                wgpu::TextureFormat::Rgba8Unorm,
                orm,
                label,
            )),
            Some(upload_mips(
                device,
                queue,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                e,
                label,
            )),
        )
    } else {
        (None, None)
    };
    let orm_view = orm.as_ref().map(|t| t.create_view(&Default::default()));
    let emissive_view = emissive
        .as_ref()
        .map(|t| t.create_view(&Default::default()));
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(
                    normal_view.as_ref().unwrap_or(flat_normal),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(
                    orm_view.as_ref().unwrap_or(flat_normal),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(
                    emissive_view.as_ref().unwrap_or(flat_normal),
                ),
            },
        ],
    });
    MaterialTexture {
        bind_group,
        _texture: texture,
        _normal: normal,
        _orm: orm,
        _emissive: emissive,
    }
}

fn pack_normal(n: geometry_math::vec::Vec3d) -> [i8; 4] {
    let q = |v: f64| (v.clamp(-1.0, 1.0) * 127.0).round() as i8;
    [q(n.x()), q(n.y()), q(n.z()), 0]
}

fn pack_color(c: [f32; 3], a: f32) -> [u8; 4] {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    [q(c[0]), q(c[1]), q(c[2]), q(a)]
}

impl SceneSink for RenderScene {
    fn set_materials(&mut self, materials: &[Material]) {
        // Nothing is decoded here: textures are streamed in as draws ask for them.
        self.materials = materials.to_vec();
        self.material_sizes = materials
            .iter()
            .map(|m| (m.width.max(1) as f32, m.height.max(1) as f32))
            .collect();
        self.material_alpha = materials.iter().map(Material::alpha).collect();
        self.material_normals = vec![None; materials.len()];
        self.material_average = materials
            .iter()
            .map(|m| u32::from_le_bytes(m.average_color().unwrap_or(PLACEHOLDER)))
            .collect();
        self.textures = materials.iter().map(|_| Residency::Pending).collect();
        self.last_used = vec![0; materials.len()];
        self.resident_bytes = 0;
        self.texture_epoch += 1;
        if let Some(workers) = &mut self.workers {
            workers.backlog.clear();
        }
        self.materials_generation += 1;
        self.textures_generation += 1;
        // Existing vertices and batches reference these slots. Preserve their
        // identities and refresh the entries when a collection is replaced.
        for slot in 2..self.material_table.len() {
            self.material_table[slot] =
                self.material_entry(self.instance_base[slot], self.instance_surface[slot]);
        }
        self.material_table_dirty = true;
        for chunk in self.chunks.values_mut() {
            chunk.dirty = true;
        }
    }

    fn clear(&mut self) {
        self.geometry_generation += 1;
        self.objects.clear();
        self.moving.clear();
        self.chunks.clear();
        self.object_table.clear();
        self.free_slots.clear();
        self.dirty_slots.clear();
        self.reset_instances();
    }

    fn remove(&mut self, key: ObjectKey) {
        if let Some(object) = self.objects.remove(&key) {
            self.geometry_generation += 1;
            self.moving.remove(&key);
            self.free_slots.push(object.slot);
            self.write_state(
                object.slot,
                ObjectState {
                    hidden: true,
                    ..Default::default()
                },
            );
            let chunk = self.mark_chunk(object.chunk);
            chunk.objects.remove(&key);
        }
    }

    fn upsert(&mut self, key: ObjectKey, triangles: &[Triangle], state: ObjectState) {
        self.geometry_generation += 1;
        let slot = match self.objects.get(&key) {
            Some(old) => {
                let (slot, chunk) = (old.slot, old.chunk);
                self.mark_chunk(chunk).objects.remove(&key);
                slot
            }
            None => self.allocate_slot(),
        };
        self.write_state(slot, state);
        let mut vertices: Vec<Vertex> = Vec::with_capacity(triangles.len() + 2);
        let mut lookup: HashMap<[u32; 8], u32> = HashMap::with_capacity(triangles.len() + 2);
        let mut batches: Vec<((bool, u32), Vec<u32>)> = Vec::new();
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        for triangle in triangles {
            let material = self.instance(triangle);
            let translucent = triangle.alpha < 0.999;
            let flags = flags_of(triangle, state.selected)
                & (triangle_flags::FACE_SELECTED | triangle_flags::NO_TINT | triangle_flags::SKIP);
            let batch = match batches
                .iter_mut()
                .position(|(k, _)| *k == (translucent, material))
            {
                Some(i) => i,
                None => {
                    batches.push(((translucent, material), Vec::new()));
                    batches.len() - 1
                }
            };
            for corner in 0..3 {
                let p = triangle.positions[corner];
                for (axis, value) in [p.x(), p.y(), p.z()].into_iter().enumerate() {
                    lo[axis] = lo[axis].min(value);
                    hi[axis] = hi[axis].max(value);
                }
                let color = triangle.corner_colors.map_or(triangle.color, |c| c[corner]);
                let uv = triangle.uv[corner];
                let vertex = Vertex {
                    position: [p.x() as f32, p.y() as f32, p.z() as f32],
                    normal: pack_normal(
                        triangle
                            .corner_normals
                            .map_or(triangle.normal, |n| n[corner]),
                    ),
                    uv: [uv.x(), uv.y()],
                    color: pack_color(color, triangle.alpha),
                    object: slot | (flags << 24),
                };
                let bits: [u32; 8] = bytemuck::cast(vertex);
                let index = *lookup.entry(bits).or_insert_with(|| {
                    vertices.push(vertex);
                    (vertices.len() - 1) as u32
                });
                batches[batch].1.push(index);
            }
        }
        let center = if triangles.is_empty() {
            [0.0; 3]
        } else {
            [
                (lo[0] + hi[0]) / 2.0,
                (lo[1] + hi[1]) / 2.0,
                (lo[2] + hi[2]) / 2.0,
            ]
        };
        let chunk = (
            (center[0] / CHUNK_SIZE).floor() as i32,
            (center[1] / CHUNK_SIZE).floor() as i32,
            (center[2] / CHUNK_SIZE).floor() as i32,
        );
        self.mark_chunk(chunk).objects.insert(key);
        // The same mesh somewhere else (same vertices, same triangles; positions apart):
        // the object moved, and its old positions give the prepass motion vectors.
        let previous = self
            .objects
            .get(&key)
            .filter(|old| old.vertices.len() == vertices.len() && old.batches == batches)
            .and_then(|old| {
                let moved = old
                    .vertices
                    .iter()
                    .zip(&vertices)
                    .any(|(a, b)| a.position != b.position);
                moved.then(|| old.vertices.iter().map(|v| v.position).collect::<Vec<_>>())
            });
        if previous.is_some() {
            self.moving.insert(key);
        } else {
            self.moving.remove(&key);
        }
        let moved = self.sync_epoch;
        self.objects.insert(
            key,
            Object {
                slot,
                chunk,
                vertices,
                batches,
                previous,
                moved,
            },
        );
    }

    fn set_state(&mut self, key: ObjectKey, state: ObjectState) {
        if let Some(slot) = self.objects.get(&key).map(|o| o.slot) {
            self.write_state(slot, state);
        }
    }

    fn set_sprites(&mut self, sprites: &[render::SpriteInstance]) {
        let mut sorted: Vec<&render::SpriteInstance> = sprites.iter().collect();
        sorted.sort_by_key(|s| s.material);
        self.sprite_draws.clear();
        let mut data: Vec<SpriteGpu> = Vec::with_capacity(sorted.len());
        for (index, sprite) in sorted.iter().enumerate() {
            let p = sprite.position;
            data.push(SpriteGpu {
                position: [p.x() as f32, p.y() as f32, p.z() as f32],
                half_width: sprite.half_width,
                half_height: sprite.half_height,
                pad: [0.0; 3],
            });
            let index = index as u32;
            match self.sprite_draws.last_mut() {
                Some((material, range)) if *material == sprite.material => range.end = index + 1,
                _ => self.sprite_draws.push((sprite.material, index..index + 1)),
            }
        }
        let (device, queue) = (self.device.clone(), self.queue.clone());
        upload(
            &device,
            &queue,
            &mut self.sprite_buffer,
            bytemuck::cast_slice(&data),
            wgpu::BufferUsages::VERTEX,
            "sprites",
        );
    }

    fn set_lights(&mut self, lights: &[SceneLight]) {
        if self.lights != lights {
            self.lights = lights.to_vec();
            self.lights_generation += 1;
        }
    }

    fn finish(&mut self) {
        // What moved in an earlier sync but not in this one has stopped.
        self.stop_moving(false);
        self.sync_epoch += 1;
        self.flush();
    }
}

impl RenderScene {
    /// Rebuilds dirty chunks and writes the dirty table slots.
    pub(crate) fn flush(&mut self) {
        let mut stats = UploadStats::default();
        self.rebuild_dirty(&mut stats);
        // The object table: grow (and rewrite whole) or write the dirty slots in runs.
        let needed = (self.object_table.len() * std::mem::size_of::<ObjectGpu>()) as u64;
        if needed > self.object_buffer.size() {
            self.object_buffer = storage_buffer(
                &self.device,
                "render_wgpu objects",
                (needed + needed / 2).next_multiple_of(256),
            );
            self.queue.write_buffer(
                &self.object_buffer,
                0,
                bytemuck::cast_slice(&self.object_table),
            );
            self.buffers_generation += 1;
            stats.object_slots = self.object_table.len();
            self.dirty_slots.clear();
        } else if !self.dirty_slots.is_empty() {
            self.dirty_slots.sort_unstable();
            self.dirty_slots.dedup();
            let mut i = 0;
            while i < self.dirty_slots.len() {
                let start = self.dirty_slots[i];
                let mut end = start;
                while i + 1 < self.dirty_slots.len() && self.dirty_slots[i + 1] == end + 1 {
                    i += 1;
                    end += 1;
                }
                let slice = &self.object_table[start as usize..=end as usize];
                self.queue.write_buffer(
                    &self.object_buffer,
                    u64::from(start) * 16,
                    bytemuck::cast_slice(slice),
                );
                i += 1;
            }
            stats.object_slots = self.dirty_slots.len();
            self.dirty_slots.clear();
        }
        if self.material_table_dirty {
            let needed = (self.material_table.len() * std::mem::size_of::<MaterialGpu>()) as u64;
            if needed > self.material_buffer.size() {
                self.material_buffer = storage_buffer(
                    &self.device,
                    "render_wgpu materials",
                    (needed * 2).next_multiple_of(256),
                );
                self.buffers_generation += 1;
            }
            self.queue.write_buffer(
                &self.material_buffer,
                0,
                bytemuck::cast_slice(&self.material_table),
            );
            self.material_table_dirty = false;
        }
        self.stats = stats;
    }
}

/// The texture memory budget until the host sets one: 1 GiB, which holds a few hundred
/// 1024² textures with their mips and normal maps.
pub const DEFAULT_TEXTURE_BUDGET: u64 = 1 << 30;

/// The placeholder colour of a material whose average is not known yet: a mid grey.
const PLACEHOLDER: [u8; 4] = [128, 128, 128, 255];

impl RenderScene {
    /// The base materials the visible draws of `chunks` and the sprites use.
    pub(crate) fn materials_of<'a>(&self, chunks: impl Iterator<Item = &'a Chunk>) -> Vec<usize> {
        let mut used: Vec<usize> = chunks
            .flat_map(|chunk| chunk.draws.iter())
            .filter_map(|draw| {
                self.instance_base
                    .get(draw.material as usize)
                    .copied()
                    .flatten()
            })
            .chain(self.sprite_draws.iter().map(|(material, _)| *material))
            .collect();
        used.sort_unstable();
        used.dedup();
        used
    }

    /// One frame of streaming: notes that `used` is being drawn, queues what of it is not
    /// resident, uploads finished decodes within the frame's budget, and evicts the least
    /// recently drawn textures while over the memory budget. Returns whether anything
    /// drawn changed.
    pub(crate) fn stream(&mut self, used: &[usize]) -> bool {
        self.frame += 1;
        let mut stats = TextureStats::default();
        for &m in used {
            let Some(state) = self.textures.get(m) else {
                continue;
            };
            self.last_used[m] = self.frame;
            if matches!(state, Residency::Pending) {
                let workers = self.workers.get_or_insert_with(Workers::new);
                workers.request(self.texture_epoch, m, self.materials[m].clone());
                self.textures[m] = Residency::Queued;
            }
        }
        let mut changed = false;
        while stats.uploaded_bytes < UPLOAD_BUDGET {
            let Some(done) = self.workers.as_mut().and_then(Workers::next) else {
                break;
            };
            if done.generation != self.texture_epoch
                || !matches!(self.textures.get(done.index), Some(Residency::Queued))
            {
                continue;
            }
            stats.uploaded_bytes += done.bytes();
            stats.uploaded += 1;
            changed |= self.make_resident(done);
        }
        changed |= self.evict(&mut stats);
        stats.resident = self
            .textures
            .iter()
            .filter(|t| matches!(t, Residency::Resident(..)))
            .count();
        stats.resident_bytes = self.resident_bytes;
        stats.waiting = self
            .textures
            .iter()
            .filter(|t| matches!(t, Residency::Queued))
            .count();
        self.texture_stats = stats;
        if changed {
            self.textures_generation += 1;
            self.flush();
        }
        changed
    }

    /// Whether textures that are being drawn are still on their way.
    pub fn streaming(&self) -> bool {
        self.workers.as_ref().is_some_and(Workers::busy)
    }

    /// Whether any of `used` is not resident yet (and has not failed).
    pub(crate) fn any_missing(&self, used: &[usize]) -> bool {
        used.iter().any(|m| {
            matches!(
                self.textures.get(*m),
                Some(Residency::Pending | Residency::Queued)
            )
        })
    }

    /// Puts a finished decode on the GPU. Returns whether anything drawn changed.
    fn make_resident(&mut self, done: Decoded) -> bool {
        let m = done.index;
        let bytes = done.bytes();
        let Some(albedo) = done.albedo else {
            self.textures[m] = Residency::Failed;
            return false;
        };
        let flip = done.normal.as_ref().map(|(_, flip)| *flip);
        let texture = make_texture(
            &self.device,
            &self.queue,
            &self.texture_layout,
            &self.sampler,
            &self.flat_normal,
            albedo,
            done.normal.map(|(mips, _)| mips),
            done.pbr,
            &self.materials[m].name,
        );
        self.textures[m] = Residency::Resident(Box::new(texture), bytes);
        self.resident_bytes += bytes;
        self.material_normals[m] = flip;
        if let Some(info) = done.info {
            self.material_average[m] = u32::from_le_bytes(info.average);
            let alpha = self.materials[m]
                .model_pbr
                .as_ref()
                .map_or(info.alpha, |pbr| pbr.alpha_mode);
            if self.material_alpha[m] != alpha {
                // It cuts or blends after all: its draws move pass, so the chunks that
                // hold them are built again (from the vertices they keep).
                self.material_alpha[m] = alpha;
                let slots: HashSet<u32> = (0..self.instance_base.len() as u32)
                    .filter(|s| self.instance_base[*s as usize] == Some(m))
                    .collect();
                for chunk in self.chunks.values_mut() {
                    if chunk.draws.iter().any(|d| slots.contains(&d.material)) {
                        chunk.dirty = true;
                    }
                }
                self.materials_generation += 1;
            }
        }
        self.refresh_instances(m);
        true
    }

    /// Drops the least recently drawn textures (never one drawn this frame) until the
    /// resident ones fit the budget.
    fn evict(&mut self, stats: &mut TextureStats) -> bool {
        if self.resident_bytes <= self.texture_budget {
            return false;
        }
        let mut candidates: Vec<(u64, usize)> = self
            .textures
            .iter()
            .enumerate()
            .filter(|(m, t)| {
                matches!(t, Residency::Resident(..)) && self.last_used[*m] < self.frame
            })
            .map(|(m, _)| (self.last_used[m], m))
            .collect();
        candidates.sort_unstable();
        let mut changed = false;
        for (_, m) in candidates {
            if self.resident_bytes <= self.texture_budget {
                break;
            }
            if let Residency::Resident(_, bytes) =
                std::mem::replace(&mut self.textures[m], Residency::Pending)
            {
                self.resident_bytes -= bytes;
            }
            self.material_normals[m] = None;
            self.refresh_instances(m);
            stats.evicted += 1;
            changed = true;
        }
        changed
    }

    /// Makes every texture the scene draws resident now, decoding on all cores and
    /// waiting for it: for offline renders and tests, which want the real picture from
    /// their first frame. Ignores the memory budget.
    pub fn load_textures_now(&mut self) {
        let used = self.materials_of(self.chunks.values());
        let wanted: Vec<usize> = used
            .into_iter()
            .filter(|m| {
                !matches!(
                    self.textures.get(*m),
                    Some(Residency::Resident(..)) | Some(Residency::Failed) | None
                )
            })
            .collect();
        if wanted.is_empty() {
            return;
        }
        let epoch = self.texture_epoch;
        let threads = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(wanted.len())
            .max(1);
        let chunk = wanted.len().div_ceil(threads);
        let materials = &self.materials;
        let decoded: Vec<Decoded> = std::thread::scope(|scope| {
            let jobs: Vec<_> = wanted
                .chunks(chunk)
                .map(|part| {
                    scope.spawn(move || {
                        part.iter()
                            .map(|m| textures::decode(epoch, *m, &materials[*m]))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            jobs.into_iter()
                .flat_map(|job| job.join().unwrap_or_default())
                .collect()
        });
        for done in decoded {
            self.make_resident(done);
        }
        self.textures_generation += 1;
        self.flush();
    }
}
