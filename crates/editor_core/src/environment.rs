//! Scene-authored HDR environments and a bounded decode cache.
use crate::document::Document;
use render::environment::{Environment, EnvironmentImage};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use web_time::{Duration, Instant};

pub const KEY: &str = "_tb_environment";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub path: PathBuf,
    pub rotation_degrees: f32,
    pub intensity: f32,
    pub show_background: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            rotation_degrees: 0.0,
            intensity: 1.0,
            show_background: true,
        }
    }
}
impl Settings {
    pub fn normalized(mut self) -> Self {
        self.rotation_degrees = if self.rotation_degrees.is_finite() {
            self.rotation_degrees.rem_euclid(360.0)
        } else {
            0.0
        };
        self.intensity = if self.intensity.is_finite() {
            self.intensity.clamp(0.0, 64.0)
        } else {
            1.0
        };
        self
    }
}
pub fn settings(document: &Document) -> Settings {
    document
        .tree_world()
        .properties
        .iter()
        .find_map(|(key, value)| (key == KEY).then_some(value))
        .and_then(|text| serde_json::from_str::<Settings>(text).ok())
        .unwrap_or_default()
        .normalized()
}
pub fn save(document: &mut Document, settings: &Settings) {
    let text =
        serde_json::to_string(&settings.clone().normalized()).expect("finite environment settings");
    document.set_entity_property_on(&[Document::WORLDSPAWN], KEY, &text);
}
pub fn resolve_path(document: &Document, path: &Path) -> PathBuf {
    if crate::files::is_absolute(path) {
        return path.to_owned();
    }
    document
        .path
        .as_deref()
        .and_then(Path::parent)
        .map_or_else(|| path.to_owned(), |base| base.join(path))
}
struct Cached {
    path: PathBuf,
    stamp: Option<(u64, SystemTime)>,
    checked: Instant,
    image: Result<Arc<EnvironmentImage>, String>,
}
thread_local! { static CACHE: RefCell<Option<Cached>> = const { RefCell::new(None) }; }
/// Retains one panorama; repeated frames reuse its pixels and poll file changes at 2 Hz.
pub fn load(path: &Path) -> Result<Arc<EnvironmentImage>, String> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(hit) = cache
            .as_ref()
            .filter(|hit| hit.path == path && hit.checked.elapsed() < Duration::from_millis(500))
        {
            return hit.image.clone();
        }
        let stamp = std::fs::metadata(path)
            .ok()
            .and_then(|m| Some((m.len(), m.modified().ok()?)));
        if let Some(hit) = cache
            .as_mut()
            .filter(|hit| hit.path == path && hit.stamp == stamp)
        {
            hit.checked = Instant::now();
            return hit.image.clone();
        }
        let image = decode(path).map(Arc::new);
        *cache = Some(Cached {
            path: path.to_owned(),
            stamp,
            checked: Instant::now(),
            image: image.clone(),
        });
        image
    })
}
/// Discards the cached file after an explicit user import or replacement.
pub fn invalidate() {
    CACHE.with(|cache| *cache.borrow_mut() = None);
}
fn decode(path: &Path) -> Result<EnvironmentImage, String> {
    if std::fs::metadata(path).is_ok_and(|m| m.len() > 128 * 1024 * 1024) {
        return Err("Environment file exceeds 128 MiB".into());
    }
    let bytes = crate::files::read_shared(path).map_err(|e| e.to_string())?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("Environment file exceeds 128 MiB".into());
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes.as_slice()))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Hdr | image::ImageFormat::OpenExr)
    ) {
        return Err("Choose a Radiance HDR or OpenEXR panorama".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let pixels = reader.decode().map_err(|e| e.to_string())?.into_rgba32f();
    let (width, height) = pixels.dimensions();
    EnvironmentImage::new(width, height, pixels.into_raw().as_chunks::<4>().0.to_vec())
}
pub fn apply(document: &Document, target: &mut render::RenderSettings) {
    let settings = settings(document);
    target.environment = if settings.path.as_os_str().is_empty() {
        None
    } else {
        load(&resolve_path(document, &settings.path))
            .ok()
            .map(|image| Environment {
                image,
                rotation_degrees: settings.rotation_degrees,
                intensity: settings.intensity,
                show_background: settings.show_background,
            })
    };
    crate::light_probes::apply(document,target);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scene_settings_are_normalized_and_undoable() {
        let mut document = Document::new(crate::world::WorldBounds::default());
        assert_eq!(settings(&document), Settings::default());
        document.path = Some(PathBuf::from("/maps/room.map"));
        let values = Settings {
            path: "lighting/studio.exr".into(),
            rotation_degrees: -90.0,
            intensity: 100.0,
            show_background: false,
        };
        save(&mut document, &values);
        let actual = settings(&document);
        assert_eq!(actual.rotation_degrees, 270.0);
        assert_eq!(actual.intensity, 64.0);
        assert!(!actual.show_background);
        assert_eq!(
            resolve_path(&document, &actual.path),
            PathBuf::from("/maps/lighting/studio.exr")
        );
        document.undo();
        assert_eq!(settings(&document), Settings::default());
        document.redo();
        assert_eq!(settings(&document), actual);
        let corrupt = Settings {
            rotation_degrees: f32::NAN,
            intensity: f32::INFINITY,
            ..Default::default()
        }
        .normalized();
        assert_eq!(corrupt, Settings::default());
        document.set_entity_property_on(&[Document::WORLDSPAWN], KEY, "invalid JSON");
        assert_eq!(settings(&document), Settings::default());
    }
    #[test]
    fn quoted_environment_settings_survive_map_save_and_reopen() {
        let mut document = Document::new(crate::world::WorldBounds::default());
        let expected = Settings {
            path: PathBuf::from("lighting/studio with spaces.exr"),
            rotation_degrees: 23.5,
            intensity: 2.25,
            show_background: false,
        };
        save(&mut document, &expected);
        let path = std::env::temp_dir().join(format!(
            "heron-environment-roundtrip-{}.map",
            std::process::id()
        ));
        std::fs::write(&path, document.to_map_text()).unwrap();
        let reopened = Document::open(&path, crate::world::WorldBounds::default()).unwrap();
        assert_eq!(settings(&reopened), expected);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn decodes_hdr_and_exr_without_losing_linear_highlights() {
        let directory =
            std::env::temp_dir().join(format!("heron-environment-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        for extension in ["hdr", "exr"] {
            let path = directory.join(format!("panorama.{extension}"));
            let image = image::ImageBuffer::<image::Rgb<f32>, Vec<f32>>::from_pixel(
                8,
                4,
                image::Rgb([4.0, 2.0, 0.5]),
            );
            image.save(&path).unwrap();
            invalidate();
            let decoded = load(&path).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (8, 4));
            assert!((decoded.pixels()[0][0] - 4.0).abs() < 0.01);
            assert!((decoded.pixels()[0][2] - 0.5).abs() < 0.01);
            assert!(Arc::ptr_eq(&decoded, &load(&path).unwrap()));
            image::ImageBuffer::<image::Rgb<f32>, Vec<f32>>::from_pixel(
                8,
                4,
                image::Rgb([8.0, 1.0, 0.25]),
            )
            .save(&path)
            .unwrap();
            invalidate();
            assert!((load(&path).unwrap().pixels()[0][0] - 8.0).abs() < 0.01);
        }
        let invalid = directory.join("invalid.hdr");
        std::fs::write(&invalid, b"not an image").unwrap();
        assert!(load(&invalid).is_err());
        std::fs::remove_dir_all(directory).unwrap();
        invalidate();
    }
}
