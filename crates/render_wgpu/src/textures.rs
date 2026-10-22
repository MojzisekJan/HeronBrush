//! Texture streaming: material images are decoded and mipped on worker threads when a
//! visible draw first needs them, uploaded within a per-frame budget, and evicted least
//! recently used when the textures outgrow a memory budget. Until a texture is resident
//! its draws show the material's average colour (from the material table), so a map
//! with ten thousand textures opens at once and costs only the memory of what is seen.
//!
//! The decision for per-material textures over arrays by size class: a draw is already
//! one (chunk, material) range, so arrays would save only the `set_bind_group` between
//! draws, which measured at about 0.35 µs each on Metal, some 3 ms of a 50k-brush frame's
//! 10 ms of encoding (see `docs/renderer-wgpu.md`, *Textures*), while arrays would force every
//! texture in a page to one size and format, and make eviction a page at a time. The
//! lever that pays is fewer draws: bindless textures with indirect draws, later.

use std::collections::VecDeque;
use std::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

use map_model::material::{Material, PixelInfo};

/// One level of a mip chain: width, height, RGBA8 texels.
pub(crate) type Mip = (u32, u32, Vec<u8>);

/// Asked of a worker: decode and mip one material.
struct Job {
    generation: u64,
    index: usize,
    material: Material,
}

/// What a worker made of a [`Job`].
pub(crate) struct Decoded {
    pub generation: u64,
    pub index: usize,
    /// The albedo's mips (sRGB, averaged in linear light); `None` if it did not decode.
    pub albedo: Option<Vec<Mip>>,
    /// The normal map's mips (linear) and whether its green channel points down.
    pub normal: Option<(Vec<Mip>, bool)>,
    pub pbr: Option<(Vec<Mip>, Vec<Mip>)>,
    pub info: Option<PixelInfo>,
}

impl Decoded {
    /// GPU bytes the texture will take.
    pub fn bytes(&self) -> u64 {
        let size = |mips: &Vec<Mip>| {
            mips.iter()
                .map(|(_, _, data)| data.len() as u64)
                .sum::<u64>()
        };
        self.albedo.as_ref().map_or(0, size)
            + self.normal.as_ref().map_or(0, |(mips, _)| size(mips))
            + self.pbr.as_ref().map_or(0, |(orm, e)| size(orm) + size(e))
    }
}

/// Decodes and mips `material` (on whatever thread calls it).
pub(crate) fn decode(generation: u64, index: usize, material: &Material) -> Decoded {
    let expected = (material.width as usize) * (material.height as usize) * 4;
    let albedo = material
        .pixels()
        .filter(|_| expected > 0)
        .map(|pixels| mip_chain(material.width, material.height, &pixels[..expected], true));
    let normal = material
        .normal_map
        .as_ref()
        .filter(|n| n.width > 0 && n.height > 0)
        .and_then(|n| {
            let pixels = n.pixels()?;
            Some((
                mip_chain(
                    n.width,
                    n.height,
                    &pixels[..(n.width * n.height * 4) as usize],
                    false,
                ),
                n.flip_y,
            ))
        });
    let pbr = material.model_pbr.as_ref().map(|p| {
        let orm = p.preview_orm();
        let white = map_model::entity_model::Skin {
            width: 1,
            height: 1,
            pixels: vec![255; 4],
        };
        let e = p.emissive.as_deref().unwrap_or(&white);
        (
            mip_chain(orm.width, orm.height, &orm.pixels, false),
            mip_chain(e.width, e.height, &e.pixels, true),
        )
    });
    Decoded {
        generation,
        index,
        albedo,
        normal,
        pbr,
        info: material.info(),
    }
}

/// The worker threads and the queues to and from them.
pub(crate) struct Workers {
    jobs: Option<mpsc::Sender<Job>>,
    results: mpsc::Receiver<Decoded>,
    /// Results taken off the channel that did not fit this frame's upload budget.
    pub backlog: VecDeque<Decoded>,
    /// Jobs sent and not yet taken back.
    pub in_flight: usize,
    /// The browser has no threads: there the queue, the way back and when the frame
    /// thread last stopped decoding, for [`Workers::next`] to decode a few each frame.
    #[cfg(target_arch = "wasm32")]
    inline: (
        mpsc::Receiver<Job>,
        mpsc::Sender<Decoded>,
        std::cell::Cell<Option<web_time::Instant>>,
    ),
}

impl Workers {
    /// Starts a few decoding threads: enough to keep up with a camera turning into new
    /// textures, few enough to leave the editor its cores.
    pub fn new() -> Self {
        let (jobs, inbox) = mpsc::channel::<Job>();
        let (done, results) = mpsc::channel::<Decoded>();
        #[cfg(target_arch = "wasm32")]
        {
            Workers {
                jobs: Some(jobs),
                results,
                backlog: VecDeque::new(),
                in_flight: 0,
                inline: (inbox, done, std::cell::Cell::new(None)),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let inbox = Arc::new(Mutex::new(inbox));
            let threads = std::thread::available_parallelism()
                .map_or(2, |n| n.get())
                .saturating_sub(2)
                .clamp(1, 4);
            for n in 0..threads {
                let inbox = inbox.clone();
                let done = done.clone();
                let _ = std::thread::Builder::new()
                    .name(format!("texture decode {n}"))
                    .spawn(move || loop {
                        // The lock is held only to take a job, not while decoding it.
                        let job = match inbox.lock() {
                            Ok(inbox) => inbox.recv(),
                            Err(_) => return,
                        };
                        let Ok(job) = job else { return };
                        if done
                            .send(decode(job.generation, job.index, &job.material))
                            .is_err()
                        {
                            return;
                        }
                    });
            }
            Workers {
                jobs: Some(jobs),
                results,
                backlog: VecDeque::new(),
                in_flight: 0,
            }
        }
    }

    /// Queues `material` for decoding.
    pub fn request(&mut self, generation: u64, index: usize, material: Material) {
        if let Some(jobs) = &self.jobs {
            if jobs
                .send(Job {
                    generation,
                    index,
                    material,
                })
                .is_ok()
            {
                self.in_flight += 1;
            }
        }
    }

    /// The next finished decode, if any.
    pub fn next(&mut self) -> Option<Decoded> {
        if let Some(done) = self.backlog.pop_front() {
            return Some(done);
        }
        // In the browser, decode for about 8 ms once a frame (`next` is called in a loop).
        #[cfg(target_arch = "wasm32")]
        {
            const BUDGET: std::time::Duration = std::time::Duration::from_millis(8);
            let (queue, done, worked) = &self.inline;
            if worked.get().is_none_or(|at| at.elapsed() > BUDGET / 2) {
                let start = web_time::Instant::now();
                while start.elapsed() < BUDGET {
                    let Ok(job) = queue.try_recv() else { break };
                    let _ = done.send(decode(job.generation, job.index, &job.material));
                }
                worked.set(Some(web_time::Instant::now()));
            }
        }
        let done = self.results.try_recv().ok()?;
        self.in_flight = self.in_flight.saturating_sub(1);
        Some(done)
    }

    /// Whether decodes are still to come back.
    pub fn busy(&self) -> bool {
        self.in_flight > 0 || !self.backlog.is_empty()
    }
}

impl Drop for Workers {
    fn drop(&mut self) {
        // Closing the queue ends the threads once they finish what they hold.
        self.jobs.take();
    }
}

/// Box-filtered mips with alpha kept straight; averaged in linear light when the texels
/// are sRGB, plainly otherwise.
pub(crate) fn mip_chain(width: u32, height: u32, pixels: &[u8], srgb: bool) -> Vec<Mip> {
    use map_model::material::{linear_to_srgb, srgb_to_linear};
    let mut out = vec![(width, height, pixels.to_vec())];
    let lut: Vec<f32> = (0..=255u8)
        .map(|c| {
            if srgb {
                srgb_to_linear(c)
            } else {
                f32::from(c) / 255.0
            }
        })
        .collect();
    while let Some((w, h, data)) = out.last() {
        if *w == 1 && *h == 1 {
            break;
        }
        let (w, h) = (*w, *h);
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                let mut sum = [0f32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let sx = (x * 2 + dx).min(w - 1);
                    let sy = (y * 2 + dy).min(h - 1);
                    let i = ((sy * w + sx) * 4) as usize;
                    for c in 0..3 {
                        sum[c] += lut[data[i + c] as usize];
                    }
                    sum[3] += f32::from(data[i + 3]) / 255.0;
                }
                let o = ((y * nw + x) * 4) as usize;
                for c in 0..3 {
                    next[o + c] = if srgb {
                        linear_to_srgb(sum[c] / 4.0)
                    } else {
                        (sum[c] / 4.0 * 255.0 + 0.5) as u8
                    };
                }
                next[o + 3] = (sum[3] / 4.0 * 255.0 + 0.5) as u8;
            }
        }
        out.push((nw, nh, next));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mips_average_in_linear_light() {
        // Black and white texels average to linear grey 0.5, sRGB 188 -- not 128.
        let pixels = [
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
        ];
        let mips = mip_chain(2, 2, &pixels, true);
        assert_eq!(mips.len(), 2);
        assert_eq!(&mips[1].2[..], &[188, 188, 188, 255]);
        let plain = mip_chain(2, 2, &pixels, false);
        assert_eq!(&plain[1].2[..3], &[128, 128, 128]);
    }

    #[test]
    fn a_worker_decodes_what_it_is_asked() {
        let mut workers = Workers::new();
        workers.request(
            7,
            3,
            Material::rgba(
                "red".into(),
                "red.png".into(),
                2,
                2,
                [255, 0, 0, 255].repeat(4),
            ),
        );
        let start = web_time::Instant::now();
        let done = loop {
            if let Some(done) = workers.next() {
                break done;
            }
            assert!(start.elapsed().as_secs() < 10, "no decode came back");
            std::thread::yield_now();
        };
        assert_eq!((done.generation, done.index), (7, 3));
        assert_eq!(done.albedo.unwrap().len(), 2);
        assert!(!workers.busy());
    }
}
