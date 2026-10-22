//! GPU time per pass group from timestamp queries, for profiling. Only exists when the
//! device has `TIMESTAMP_QUERY` (the host asks for it when the adapter offers it) and the
//! host turned profiling on; otherwise passes carry no timestamp writes at all.

/// Room for this many timestamped passes per frame.
const CAPACITY: u32 = 96;

pub(crate) struct GpuTimer {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    labels: Vec<&'static str>,
    period_ns: f64,
    /// The last frame's times, summed per label, in milliseconds.
    pub last: Vec<(&'static str, f64)>,
}

impl GpuTimer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("render_wgpu timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: CAPACITY * 2,
        });
        let size = u64::from(CAPACITY * 2 * wgpu::QUERY_SIZE);
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("render_wgpu timestamps"),
            size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("render_wgpu timestamps"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Some(Self {
            set,
            resolve,
            readback,
            labels: Vec::new(),
            period_ns: f64::from(queue.get_timestamp_period()),
            last: Vec::new(),
        })
    }

    /// Starts a frame.
    pub fn begin(&mut self) {
        self.labels.clear();
    }

    /// Timestamp writes for one more pass of group `label`, if there is room.
    pub fn writes(&mut self, label: &'static str) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let index = self.labels.len() as u32;
        if index >= CAPACITY {
            return None;
        }
        self.labels.push(label);
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.set,
            beginning_of_pass_write_index: Some(index * 2),
            end_of_pass_write_index: Some(index * 2 + 1),
        })
    }

    /// Resolves this frame's queries into the readback buffer.
    pub fn finish(&self, encoder: &mut wgpu::CommandEncoder) {
        let count = self.labels.len() as u32 * 2;
        if count == 0 {
            return;
        }
        encoder.resolve_query_set(&self.set, 0..count, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(
            &self.resolve,
            0,
            &self.readback,
            0,
            u64::from(count * wgpu::QUERY_SIZE),
        );
    }

    /// Reads the frame back (blocking: profiling waits for the GPU anyway).
    pub fn read(&mut self, device: &wgpu::Device) {
        let count = self.labels.len() * 2;
        self.last.clear();
        if count == 0 {
            return;
        }
        let slice = self.readback.slice(..(count * 8) as u64);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        if let Ok(data) = slice.get_mapped_range() {
            let stamps: &[u64] = bytemuck::cast_slice(&data);
            for (i, label) in self.labels.iter().enumerate() {
                let ms =
                    stamps[i * 2 + 1].saturating_sub(stamps[i * 2]) as f64 * self.period_ns / 1e6;
                match self.last.iter_mut().find(|(l, _)| l == label) {
                    Some((_, total)) => *total += ms,
                    None => self.last.push((label, ms)),
                }
            }
        }
        self.readback.unmap();
    }
}
