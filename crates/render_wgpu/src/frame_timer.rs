//! Nonblocking whole-frame GPU timestamps. Three slots prevent readback stalls.
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
struct Slot {
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    state: Arc<AtomicU8>,
    serial: u64,
}
pub(crate) struct FrameTimer {
    queries: wgpu::QuerySet,
    _marker: wgpu::Texture,
    marker_view: wgpu::TextureView,
    slots: Vec<Slot>,
    period: f64,
    next: u64,
    last: Option<(u64, f64)>,
}
impl FrameTimer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frame latency"),
            ty: wgpu::QueryType::Timestamp,
            count: 6,
        });
        let slots = (0..3)
            .map(|_| Slot {
                resolve: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("frame timing resolve"),
                    size: 16,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                readback: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("frame timing readback"),
                    size: 16,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                state: Arc::new(AtomicU8::new(0)),
                serial: 0,
            })
            .collect();
        let marker = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frame timer attachment"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let marker_view = marker.create_view(&Default::default());
        Some(Self {
            _marker: marker,
            marker_view,
            queries,
            slots,
            period: f64::from(queue.get_timestamp_period()),
            next: 0,
            last: None,
        })
    }
    fn collect(&mut self, device: &wgpu::Device) {
        let _ = device.poll(wgpu::PollType::Poll);
        for slot in &mut self.slots {
            match slot.state.load(Ordering::Acquire) {
                1 => {
                    if let Ok(data) = slot.readback.slice(..).get_mapped_range() {
                        let stamps: &[u64] = bytemuck::cast_slice(&data);
                        let ms =
                            stamps[1].saturating_sub(stamps[0]).max(1) as f64 * self.period / 1e6;
                        if ms.is_finite()
                            && ms > 0.0
                            && self.last.is_none_or(|(serial, _)| serial < slot.serial)
                        {
                            self.last = Some((slot.serial, ms));
                        }
                    }
                    slot.readback.unmap();
                    slot.state.store(0, Ordering::Release);
                }
                2 => {
                    slot.readback.unmap();
                    slot.state.store(0, Ordering::Release);
                }
                _ => {}
            }
        }
    }
    pub fn take_ms(&mut self, device: &wgpu::Device) -> Option<f64> {
        self.collect(device);
        self.last.take().map(|(_, ms)| ms)
    }
    pub fn reserve(&mut self, device: &wgpu::Device) -> Option<usize> {
        self.collect(device);
        let index = self
            .slots
            .iter()
            .position(|slot| slot.state.load(Ordering::Acquire) == 0)?;
        self.next += 1;
        self.slots[index].serial = self.next;
        self.slots[index].state.store(3, Ordering::Release);
        Some(index)
    }
    pub fn marker(&self, encoder: &mut wgpu::CommandEncoder, index: usize, start: bool) {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frame timing marker"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.marker_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: Some(wgpu::RenderPassTimestampWrites {
                query_set: &self.queries,
                beginning_of_pass_write_index: start.then_some(index as u32 * 2),
                end_of_pass_write_index: (!start).then_some(index as u32 * 2 + 1),
            }),
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    pub fn finish(&self, encoder: &mut wgpu::CommandEncoder, index: usize) {
        self.marker(encoder, index, false);
        let slot = &self.slots[index];
        encoder.resolve_query_set(
            &self.queries,
            index as u32 * 2..index as u32 * 2 + 2,
            &slot.resolve,
            0,
        );
        encoder.copy_buffer_to_buffer(&slot.resolve, 0, &slot.readback, 0, 16);
    }
    pub fn submitted(&self, index: usize) {
        let slot = &self.slots[index];
        let state = Arc::clone(&slot.state);
        slot.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                state.store(if result.is_ok() { 1 } else { 2 }, Ordering::Release)
            });
    }
}
