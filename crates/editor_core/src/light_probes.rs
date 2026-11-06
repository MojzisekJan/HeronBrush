//! Scene-authored local reflections and baked diffuse transport.
mod bake;
use crate::document::Document;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
};
use web_time::{Duration, Instant};
pub const KEY: &str = "_tb_light_probes";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Probe {
    pub id: u64,
    pub position: [f32; 3],
    pub half_size: [f32; 3],
    pub blend: f32,
    pub intensity: f32,
    pub reflections: bool,
    pub diffuse: bool,
}
impl Default for Probe {
    fn default() -> Self {
        Self {
            id: 1,
            position: [0.0; 3],
            half_size: [256.0; 3],
            blend: 0.2,
            intensity: 1.0,
            reflections: true,
            diffuse: true,
        }
    }
}
impl Probe {
    pub fn volume(&self) -> render::probes::Volume {
        render::probes::Volume {
            position: self.position,
            half_size: self.half_size,
            blend: self.blend,
            intensity: self.intensity,
            reflections: self.reflections,
            diffuse: self.diffuse,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub probes: Vec<Probe>,
    pub resolution: u32,
    pub lighting: Option<crate::render_camera::Lighting>,
    pub sun_elevation: f32,
    pub sun_azimuth: f32,
    pub shadows: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            probes: Vec::new(),
            resolution: 32,
            lighting: None,
            sun_elevation: 45.0,
            sun_azimuth: 135.0,
            shadows: true,
        }
    }
}
impl Settings {
    pub fn normalized(mut self) -> Self {
        self.lighting = self.lighting.map(crate::render_camera::Lighting::validated);
        self.sun_elevation = if self.sun_elevation.is_finite() {
            self.sun_elevation.clamp(0.0, 90.0)
        } else {
            45.0
        };
        self.sun_azimuth = if self.sun_azimuth.is_finite() {
            self.sun_azimuth.rem_euclid(360.0)
        } else {
            135.0
        };
        self.probes.truncate(render::probes::Lighting::MAX);
        if ![16, 32, 64].contains(&self.resolution) {
            self.resolution = 32;
        }
        for probe in &mut self.probes {
            for value in &mut probe.position {
                *value = if value.is_finite() {
                    value.clamp(-1000000.0, 1000000.0)
                } else {
                    0.0
                };
            }
            for value in &mut probe.half_size {
                *value = if value.is_finite() {
                    value.clamp(1.0, 100000.0)
                } else {
                    256.0
                };
            }
            probe.blend = if probe.blend.is_finite() {
                probe.blend.clamp(0.01, 1.0)
            } else {
                0.2
            };
            probe.intensity = if probe.intensity.is_finite() {
                probe.intensity.clamp(0.0, 16.0)
            } else {
                1.0
            };
        }
        self
    }
}
pub fn settings(document: &Document) -> Settings {
    document
        .tree_world()
        .properties
        .iter()
        .find_map(|(key, value)| (key == KEY).then_some(value))
        .and_then(|value| serde_json::from_str::<Settings>(value).ok())
        .unwrap_or_default()
        .normalized()
}
pub fn save(document: &mut Document, settings: &Settings) {
    let text =
        serde_json::to_string(&settings.clone().normalized()).expect("finite probe settings");
    document.set_entity_property_on(&[Document::WORLDSPAWN], KEY, &text);
}
struct Job {
    key: u64,
    receiver: mpsc::Receiver<Result<render::probes::Lighting, String>>,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicUsize>,
}
struct Cache {
    key: Option<u64>,
    ready: Option<u64>,
    revision: Option<(u64, u64, u64)>,
    config: String,
    changed: Instant,
    job: Option<Job>,
    lighting: Option<Arc<render::probes::Lighting>>,
    error: Option<String>,
    force: bool,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            key: None,
            ready: None,
            revision: None,
            config: String::new(),
            changed: Instant::now(),
            job: None,
            lighting: None,
            error: None,
            force: false,
        }
    }
}
impl Drop for Cache {
    fn drop(&mut self) {
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::Relaxed);
        }
    }
}
thread_local! {static CACHE:RefCell<Cache>=RefCell::new(Cache::default());}
fn revision(document: &Document) -> (u64, u64, u64) {
    (
        document.generation(),
        document.content_revision(),
        document.materials_revision(),
    )
}
pub enum Status {
    Disabled,
    Waiting,
    Baking(usize, usize),
    Ready,
    Failed(String),
}
pub fn status(document: &Document) -> Status {
    let config = settings(document);
    if config.probes.is_empty() {
        return Status::Disabled;
    }
    CACHE.with(|cache| {
        let cache = cache.borrow();
        if cache.revision != Some(revision(document)) {
            return Status::Waiting;
        }
        if let Some(job) = &cache.job {
            return Status::Baking(job.progress.load(Ordering::Relaxed), config.probes.len());
        }
        if let Some(error) = &cache.error {
            return Status::Failed(error.clone());
        }
        if cache.ready == cache.key && cache.lighting.is_some() {
            Status::Ready
        } else {
            Status::Waiting
        }
    })
}
pub fn ready(document: &Document) -> bool {
    matches!(status(document), Status::Disabled | Status::Ready)
}
pub fn request_bake() {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.force = true;
        cache.error = None;
        cache.ready = None;
    });
}
/// Reuses an immutable completed bake; expensive capture and filtering run on a worker.
pub fn apply(document: &Document, target: &mut render::RenderSettings) {
    let config = settings(document);
    if config.probes.is_empty() {
        target.probes = None;
        CACHE.with(|cache| {
            if cache.borrow().lighting.is_some() || cache.borrow().job.is_some() {
                *cache.borrow_mut() = Cache::default();
            }
        });
        return;
    }
    let mut bake_settings = crate::render_settings::from_preferences();
    bake_settings.environment = target.environment.clone();
    if let Some(lighting) = &config.lighting {
        lighting.apply(&mut bake_settings);
        bake_settings.sun_elevation = config.sun_elevation;
        bake_settings.sun_azimuth = config.sun_azimuth;
        bake_settings.shadows = config.shadows;
    }
    let encoded = serde_json::to_string(&config).expect("finite settings");
    let revision = revision(document);
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    revision.hash(&mut hash);
    encoded.hash(&mut hash);
    format!(
        "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
        bake_settings.sky_color,
        bake_settings.horizon_color,
        bake_settings.ground_color,
        bake_settings.sun_color,
        bake_settings.sun_intensity,
        bake_settings.sun_elevation,
        bake_settings.sun_azimuth,
        bake_settings.environment,
        bake_settings.default_metalness,
        bake_settings.default_roughness
    )
    .hash(&mut hash);
    let key = hash.finish();
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.key != Some(key) {
            if let Some(job) = cache.job.take() {
                job.cancel.store(true, Ordering::Relaxed);
            }
            if cache.config != encoded || cache.revision.is_some_and(|r| r.0 != revision.0) {
                cache.lighting = None;
            }
            cache.key = Some(key);
            cache.revision = Some(revision);
            cache.config = encoded;
            cache.changed = Instant::now();
            cache.error = None;
        }
        let received = cache.job.as_ref().map(|job| job.receiver.try_recv());
        if matches!(received, Some(Err(mpsc::TryRecvError::Disconnected))) {
            cache.job = None;
            cache.error = Some("Light probe worker stopped".into());
        }
        if let Some(Ok(result)) = received {
            let job = cache.job.take().expect("received");
            if Some(job.key) == cache.key {
                match result {
                    Ok(lighting) => {
                        cache.lighting = Some(Arc::new(lighting));
                        cache.ready = Some(job.key);
                    }
                    Err(error) => cache.error = Some(error),
                }
            }
        }
        if cache.job.is_none()
            && cache.error.is_none()
            && (cache.ready != Some(key) || cache.force)
            && (cache.force || cache.changed.elapsed() >= Duration::from_millis(300))
        {
            cache.force = false;
            let snapshot = bake::Snapshot::new(document, &config, &bake_settings);
            let (sender, receiver) = mpsc::sync_channel(1);
            let cancel = Arc::new(AtomicBool::new(false));
            let progress = Arc::new(AtomicUsize::new(0));
            let worker_cancel = Arc::clone(&cancel);
            let worker_progress = Arc::clone(&progress);
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || {
                let result = snapshot.bake(&worker_cancel, &worker_progress);
                let _ = sender.send(result);
            });
            #[cfg(target_arch = "wasm32")]
            {
                let result = snapshot.bake(&worker_cancel, &worker_progress);
                let _ = sender.send(result);
            }
            cache.job = Some(Job {
                key,
                receiver,
                cancel,
                progress,
            });
        }
        target.probes = cache.lighting.clone();
    });
}

/// Synchronous capture for batch rendering, tests and command-line tools.
/// Interactive viewports use the cancellable worker in `apply`.
pub fn bake_blocking(
    document: &Document,
    config: &Settings,
    settings: &render::RenderSettings,
) -> Result<render::probes::Lighting, String> {
    bake::Snapshot::new(document, &config.clone().normalized(), settings)
        .bake(&AtomicBool::new(false), &AtomicUsize::new(0))
}
#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    fn room(emission: bool) -> Document {
        let mut document = Document::new(Default::default());
        let world = document.world_bounds();
        let cube = |lo: [f64; 3], hi: [f64; 3]| {
            map_model::shapes::create_cuboid(
                &world,
                &Bbox3d::new(
                    Vec3d::new(lo[0], lo[1], lo[2]),
                    Vec3d::new(hi[0], hi[1], hi[2]),
                ),
                "probe-room",
            )
            .unwrap()
        };
        let mut floor = cube([-72.0, -72.0, -8.0], [72.0, 72.0, 0.0]);
        if emission {
            let mut faces = floor.faces().to_vec();
            for face in &mut faces {
                face.set_paint(Some(map_model::brush_face::FaceColor::solid(
                    map_model::brush_face::Rgb::new(255, 0, 0),
                )));
                face.surface_material = Some(map_model::brush_face::SurfaceMaterial {
                    emission: 400,
                    ..Default::default()
                });
            }
            floor = map_model::brush::Brush::from_faces(&world, faces).unwrap();
        }
        document.add_brushes(vec![
            floor,
            cube([-72.0, -72.0, 64.0], [72.0, 72.0, 72.0]),
            cube([-72.0, -72.0, 0.0], [-64.0, 72.0, 64.0]),
            cube([64.0, -72.0, 0.0], [72.0, 72.0, 64.0]),
            cube([-64.0, -72.0, 0.0], [64.0, -64.0, 64.0]),
            cube([-64.0, 64.0, 0.0], [64.0, 72.0, 64.0]),
        ]);
        document.clear_selection();
        document
    }
    #[test]
    fn probes_round_trip_with_the_map_and_participate_in_undo() {
        let mut document = room(false);
        let config = Settings {
            probes: vec![Probe {
                position: [0.0, 0.0, 24.0],
                half_size: [64.0, 64.0, 32.0],
                intensity: 1.25,
                ..Default::default()
            }],
            resolution: 16,
            ..Default::default()
        };
        save(&mut document, &config);
        assert_eq!(settings(&document), config);
        document.undo();
        assert!(settings(&document).probes.is_empty());
        document.redo();
        assert_eq!(settings(&document), config);
        let path =
            std::env::temp_dir().join(format!("heron-light-probes-{}.map", std::process::id()));
        std::fs::write(&path, document.to_map_text()).unwrap();
        let reopened = Document::open(&path, Default::default()).unwrap();
        assert_eq!(settings(&reopened), config);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn a_closed_room_blocks_the_environment_and_emission_produces_colored_diffuse_transport() {
        let config = Settings {
            probes: vec![Probe {
                position: [0.0, 0.0, 24.0],
                half_size: [64.0, 64.0, 32.0],
                ..Default::default()
            }],
            resolution: 16,
            ..Default::default()
        };
        let mut render = crate::render_settings::from_preferences();
        render.sun_intensity = 0.0;
        render.sky_color = [1.0; 3];
        render.horizon_color = [1.0; 3];
        render.ground_color = [1.0; 3];
        let dark = bake_blocking(&room(false), &config, &render).unwrap();
        assert!(
            dark.probes()[0].sh[0]
                .iter()
                .all(|value| value.abs() < 0.00001),
            "closed walls must block the outside sky"
        );
        let red = bake_blocking(&room(true), &config, &render).unwrap();
        let irradiance = red.probes()[0].sh[0];
        assert!(
            irradiance[0] > 1.0 && irradiance[1] < 0.00001 && irradiance[2] < 0.00001,
            "emissive red floor must illuminate the volume: {irradiance:?}"
        );
        assert!(
            red.probes()[0].mips[0].iter().any(|pixel| pixel[0] > 1.0),
            "reflection capture must retain HDR geometry outside the camera view"
        );
        assert!(
            format!("{red:?}").len() < 100,
            "per-frame cache keys must not print probe pixels"
        );
    }
    #[test]
    fn legacy_surface_notes_and_layered_properties_survive_map_serialization() {
        let mut document = room(false);
        document.select_all();
        let surface = map_model::brush_face::SurfaceMaterial {
            roughness: 45,
            metallic: 80,
            clearcoat: 75,
            coat_roughness: 20,
            anisotropy: 90,
            anisotropy_rotation: 135,
            ..Default::default()
        };
        document.set_surface_material(Some(surface));
        let path =
            std::env::temp_dir().join(format!("heron-layered-surfaces-{}.map", std::process::id()));
        std::fs::write(&path, document.to_map_text()).unwrap();
        let reopened = Document::open(&path, Default::default()).unwrap();
        assert!(reopened
            .render_triangles()
            .iter()
            .all(|triangle| triangle.surface_material == Some(surface)));
        std::fs::remove_file(path).unwrap();
        let old: map_model::brush_face::SurfaceMaterial =
            serde_json::from_str(r#"{"roughness":45,"metallic":80,"emission":0}"#).unwrap();
        assert_eq!(old.clearcoat, 0);
        assert_eq!(old.anisotropy, 0);
        assert_eq!(old.coat_roughness, 10);
        let camera = crate::render_camera::Settings {
            samples: 4096,
            ..Default::default()
        }
        .validated();
        assert_eq!(camera.samples, 4096);
    }
    #[test]
    fn worker_bakes_are_published_and_configuration_changes_invalidate_them() {
        let mut document = room(true);
        let mut config = Settings {
            resolution: 16,
            probes: vec![Probe {
                position: [0.0, 0.0, 24.0],
                ..Default::default()
            }],
            ..Default::default()
        };
        save(&mut document, &config);
        request_bake();
        let mut render = crate::render_settings::from_preferences();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            apply(&document, &mut render);
            if ready(&document) {
                break;
            }
            assert!(Instant::now() < deadline, "worker did not finish");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(render.probes.is_some());
        config.probes[0].position[0] = 16.0;
        save(&mut document, &config);
        assert!(!ready(&document));
        apply(&document, &mut render);
        assert!(
            render.probes.is_none(),
            "changed volumes must not keep stale captures"
        );
        config.probes.clear();
        save(&mut document, &config);
        apply(&document, &mut render);
        assert!(ready(&document));
        assert!(render.probes.is_none());
    }
}
