//! An opt-in frame-time logger: `HERON_BRUSH_PROFILE=1` prints, every two seconds, the
//! rolling average and 95th percentile of where each frame's CPU time went, and the GPU
//! time of the 3D view. Nothing here runs unless the variable is set; the hooks in the
//! frame are `Option`s that stay `None`. See `docs/performance.md`.
//!
//! `HERON_BRUSH_ORBIT=<frames>` (with profiling on) turns the 3D camera round the map for
//! that many frames, prints a summary and closes the window: a repeatable GPU benchmark.
//!
//! Columns:
//!
//! - `frame`: eframe's own CPU time for the whole previous frame, vsync excluded.
//! - `ui`: [`eframe::App::ui`]: input handling, layout and every panel.
//! - `2d`: the three flat views drawing into egui shapes (inside `ui`).
//! - `3d`: the 3D view's `show` (inside `ui`), split into `build` (the core's surfaces
//!   and overlay, CPU) and `gl` (uploads and draw calls, CPU side).
//! - `gpu`: the 3D view's draw on the GPU, from a `GL_TIME_ELAPSED` query.
//! - `paint`: `frame - ui`: egui ending the pass, tessellating and painting.

use std::collections::VecDeque;
use web_time::{Duration, Instant};

use glow::HasContext as _;

/// What one frame cost, in milliseconds.
#[derive(Clone, Copy, Default)]
struct Sample {
    frame: f64,
    ui: f64,
    flat: f64,
    view3d: f64,
    build: f64,
    gl: f64,
    gpu: f64,
    paint: f64,
}

const COLUMNS: [&str; 8] = ["frame", "ui", "2d", "3d", "build", "gl", "gpu", "paint"];

impl Sample {
    fn values(&self) -> [f64; 8] {
        [
            self.frame,
            self.ui,
            self.flat,
            self.view3d,
            self.build,
            self.gl,
            self.gpu,
            self.paint,
        ]
    }
}

/// The frame-time logger, present only while `HERON_BRUSH_PROFILE` is set.
pub struct Profiler {
    window: VecDeque<Sample>,
    all: Vec<Sample>,
    current: Sample,
    ui_started: Option<Instant>,
    previous_ui: f64,
    last_print: Instant,
    /// Frames left to orbit, and how many the orbit has.
    orbit: Option<(u32, u32)>,
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

impl Profiler {
    /// A logger, if `HERON_BRUSH_PROFILE` asks for one.
    pub fn from_env() -> Option<Self> {
        if !editor_core::profile::enabled() {
            return None;
        }
        let orbit = std::env::var("HERON_BRUSH_ORBIT")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .map(|n| (n, n));
        eprintln!(
            "heron-brush: profiling (columns in ms: {})",
            COLUMNS.join(", ")
        );
        Some(Self {
            window: VecDeque::new(),
            all: Vec::new(),
            current: Sample::default(),
            ui_started: None,
            previous_ui: 0.0,
            last_print: Instant::now(),
            orbit,
        })
    }

    /// The start of [`eframe::App::ui`]; `frame_cpu` is eframe's figure for the frame before.
    pub fn begin(&mut self, frame_cpu: Option<f32>) {
        self.current = Sample::default();
        if let Some(seconds) = frame_cpu {
            self.current.frame = f64::from(seconds) * 1000.0;
            self.current.paint = (self.current.frame - self.previous_ui).max(0.0);
        }
        // Whatever the core counted outside a frame does not belong to this one.
        editor_core::profile::take_scene_build();
        self.ui_started = Some(Instant::now());
    }

    /// Adds a flat view's drawing.
    pub fn add_flat(&mut self, started: Instant) {
        self.current.flat += ms(started.elapsed());
    }

    /// Adds the 3D view's `show`, and the GPU time its last finished query measured.
    pub fn add_3d(&mut self, started: Instant, gpu: Option<f64>) {
        let total = ms(started.elapsed());
        let build = ms(editor_core::profile::take_scene_build());
        self.current.view3d += total;
        self.current.build += build;
        self.current.gl += (total - build).max(0.0);
        if let Some(gpu) = gpu {
            self.current.gpu = gpu;
        }
    }

    /// The end of [`eframe::App::ui`]: files the frame and prints now and then.
    pub fn end(&mut self) {
        if let Some(started) = self.ui_started.take() {
            self.current.ui = ms(started.elapsed());
        }
        self.previous_ui = self.current.ui;
        self.window.push_back(self.current);
        if self.window.len() > 240 {
            self.window.pop_front();
        }
        if self.orbit.is_some() {
            self.all.push(self.current);
        }
        if self.last_print.elapsed() >= Duration::from_secs(2) {
            self.last_print = Instant::now();
            eprintln!(
                "{}",
                summary("last 240 frames", self.window.iter().copied())
            );
        }
    }

    /// Where the orbiting camera should be this frame, if it is orbiting: the angle in
    /// radians. `None` when there is no orbit or it has finished.
    pub fn orbit_step(&mut self) -> Option<f64> {
        let (left, total) = self.orbit.as_mut()?;
        if *left == 0 {
            return None;
        }
        *left -= 1;
        Some(std::f64::consts::TAU * f64::from(*total - *left) / f64::from((*total).max(1)))
    }

    /// Whether an orbit was asked for and has just run out: prints the summary once.
    pub fn orbit_finished(&mut self) -> bool {
        if let Some((0, _)) = self.orbit {
            self.orbit = None;
            // The first frames load and upload everything; the orbit is what is measured.
            let skip = self.all.len().min(5);
            println!("{}", summary("orbit", self.all.iter().skip(skip).copied()));
            return true;
        }
        false
    }
}

fn summary(title: &str, samples: impl Iterator<Item = Sample>) -> String {
    let samples: Vec<Sample> = samples.collect();
    let mut line = format!("[profile] {title} ({} frames):", samples.len());
    if samples.is_empty() {
        return line;
    }
    for (index, name) in COLUMNS.iter().enumerate() {
        let mut values: Vec<f64> = samples.iter().map(|s| s.values()[index]).collect();
        values.sort_by(f64::total_cmp);
        let average = values.iter().sum::<f64>() / values.len() as f64;
        let p95 = values[((values.len() as f64 * 0.95).ceil() as usize).clamp(1, values.len()) - 1];
        line.push_str(&format!("  {name} {average:.2}/{p95:.2}"));
    }
    line
}

/// `GL_TIME_ELAPSED` queries round the 3D view's drawing, read a few frames later so
/// asking never stalls the pipeline.
pub struct GpuTimer {
    free: Vec<glow::Query>,
    pending: VecDeque<glow::Query>,
    last: Option<f64>,
}

impl GpuTimer {
    /// A timer, if profiling is on.
    pub fn from_env() -> Option<Self> {
        editor_core::profile::enabled().then(|| Self {
            free: Vec::new(),
            pending: VecDeque::new(),
            last: None,
        })
    }

    /// Starts timing; the GL context must be current.
    pub fn begin(&mut self, gl: &glow::Context) {
        // SAFETY: The caller keeps `gl`'s context current, as the doc says (`GlView::show` calls
        // this inside egui's update). The query is one this context made.
        unsafe {
            let query = match self.free.pop() {
                Some(query) => query,
                None => match gl.create_query() {
                    Ok(query) => query,
                    Err(_) => return,
                },
            };
            gl.begin_query(glow::TIME_ELAPSED, query);
            self.pending.push_back(query);
        }
    }

    /// Stops timing and returns the latest finished measurement, in milliseconds.
    pub fn end(&mut self, gl: &glow::Context) -> Option<f64> {
        // SAFETY: `GlView::show` calls this right after `begin`, with the same context current.
        // The queries are ones `begin` made in it, and the results are read into locals.
        unsafe {
            gl.end_query(glow::TIME_ELAPSED);
            while self.pending.len() > 1 {
                let query = self.pending[0];
                if gl.get_query_parameter_u32(query, glow::QUERY_RESULT_AVAILABLE) == 0 {
                    break;
                }
                let nanoseconds = gl.get_query_parameter_u64(query, glow::QUERY_RESULT);
                self.last = Some(nanoseconds as f64 / 1e6);
                self.free
                    .push(self.pending.pop_front().expect("a pending query"));
            }
        }
        self.last
    }

    /// Frees the queries; the GL context must be current.
    pub fn destroy(self, gl: &glow::Context) {
        // SAFETY: The caller keeps `gl`'s context current, as the doc says. `self` is consumed,
        // so no query is used after it is deleted.
        unsafe {
            for query in self.free.into_iter().chain(self.pending) {
                gl.delete_query(query);
            }
        }
    }
}

// Per-frame timing of the 3D view on either backend, input through a finished GPU frame
// (the view waits for the GPU while profiling), and separately the frames in which the
// document changed, which is what a selection click costs. A summary goes to stderr
// every 120 frames and when the app exits.

/// Whether profiling is on.
pub fn enabled() -> bool {
    editor_core::profile::enabled()
}

#[derive(Default)]
struct ViewSamples {
    frames: Vec<f64>,
    drawn: Vec<f64>,
    changes: Vec<f64>,
    /// Frames that kept the last picture.
    idle: Vec<f64>,
    /// GPU time per wgpu pass group, from timestamp queries.
    passes: Vec<(&'static str, Vec<f64>)>,
}

thread_local! {
    static VIEW_SAMPLES: std::cell::RefCell<ViewSamples> = std::cell::RefCell::new(ViewSamples::default());
}

fn view_summary(label: &str, values: &mut [f64]) -> String {
    if values.is_empty() {
        return format!("{label}: none");
    }
    values.sort_by(f64::total_cmp);
    let at = |q: f64| values[((values.len() - 1) as f64 * q).round() as usize];
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    format!(
        "{label}: n={} mean={mean:.2}ms median={:.2}ms p95={:.2}ms max={:.2}ms",
        values.len(),
        at(0.5),
        at(0.95),
        at(1.0)
    )
}

/// Records one 3D view frame; `changed` when the document changed during it, `drew`
/// when it drew a new picture (rather than showing the kept one).
pub fn view3d_frame(elapsed: Duration, changed: bool, drew: bool) {
    let ms = elapsed.as_secs_f64() * 1000.0;
    VIEW_SAMPLES.with(|samples| {
        let mut samples = samples.borrow_mut();
        samples.frames.push(ms);
        if drew {
            samples.drawn.push(ms);
        } else {
            samples.idle.push(ms);
        }
        if changed {
            samples.changes.push(ms);
        }
        if samples.frames.len() % 120 == 0 {
            let mut drawn = samples.drawn.clone();
            let mut changes = samples.changes.clone();
            eprintln!(
                "[profile] {} | {}",
                view_summary("drawn view3d frames", &mut drawn),
                view_summary("frames with a document change", &mut changes)
            );
        }
    });
}

/// Records the GPU time of each wgpu pass group of a drawn frame.
#[cfg_attr(not(feature = "wgpu"), allow(dead_code))]
pub fn wgpu_passes(times: &[(&'static str, f64)]) {
    VIEW_SAMPLES.with(|samples| {
        let mut samples = samples.borrow_mut();
        for (label, ms) in times {
            match samples.passes.iter_mut().find(|(l, _)| l == label) {
                Some((_, values)) => values.push(*ms),
                None => samples.passes.push((label, vec![*ms])),
            }
        }
    });
}

/// Prints the summary of every 3D view frame recorded.
pub fn report() {
    if !enabled() {
        return;
    }
    VIEW_SAMPLES.with(|samples| {
        let mut samples = samples.borrow_mut();
        let ViewSamples {
            frames,
            drawn,
            changes,
            idle,
            passes,
        } = &mut *samples;
        eprintln!(
            "[profile] final {} | {} | {} | {}",
            view_summary("all view3d frames", frames),
            view_summary("drawn view3d frames", drawn),
            view_summary("idle view3d frames", idle),
            view_summary("frames with a document change", changes)
        );
        for (label, values) in passes.iter_mut() {
            eprintln!("[profile] wgpu gpu {}", view_summary(label, values));
        }
    });
}
