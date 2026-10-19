//! An opt-in frame profiler's hooks inside the core: the frontends time their own
//! sections, and the core adds the parts only it can see (building the 3D scene's
//! surfaces). Off unless `HERON_BRUSH_PROFILE` (or `HERON_PROFILE`) is set; off, each hook
//! costs one relaxed atomic load. See `docs/performance.md`.

use std::cell::Cell;
use std::sync::atomic::{AtomicU8, Ordering};
use web_time::{Duration, Instant};

static STATE: AtomicU8 = AtomicU8::new(0);

/// Whether profiling was asked for, read from the environment once.
#[inline]
pub fn enabled() -> bool {
    match STATE.load(Ordering::Relaxed) {
        0 => {
            let on = ["HERON_BRUSH_PROFILE", "HERON_PROFILE"]
                .iter()
                .any(|key| std::env::var_os(key).is_some_and(|v| !v.is_empty() && v != "0"));
            STATE.store(if on { 2 } else { 1 }, Ordering::Relaxed);
            on
        }
        state => state == 2,
    }
}

/// A clock started only while profiling.
#[inline]
pub fn start() -> Option<Instant> {
    enabled().then(Instant::now)
}

thread_local! {
    static SCENE_BUILD: Cell<Duration> = const { Cell::new(Duration::ZERO) };
}

/// Adds the time since `started` to this frame's 3D scene building.
#[inline]
pub fn add_scene_build(started: Option<Instant>) {
    if let Some(started) = started {
        SCENE_BUILD.with(|total| total.set(total.get() + started.elapsed()));
    }
}

/// This frame's 3D scene building so far, and starts the count again.
pub fn take_scene_build() -> Duration {
    SCENE_BUILD.with(|total| total.replace(Duration::ZERO))
}
