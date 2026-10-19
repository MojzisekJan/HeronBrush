//! The asset library without its panel: the folders, the prefabs in them, their
//! thumbnails and previews, and inserting one into a map.
//!
//! A library folder holds `.map` files containing small scenes and matching thumbnail images.
//! Inserting an asset adds its contents at the pointer position as one undoable operation.
//!
//! The folders are remembered in the preferences (`asset_library`, one path per line) so
//! the library is there the next time the editor opens.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use geometry_math::vec::Vec3d;

use crate::document::Document;

#[path = "assets_preview.rs"]
mod preview_depth;

/// How large a prefab thumbnail is drawn, in pixels.
pub const THUMBNAIL: i32 = 84;

/// The image extensions a thumbnail is looked for under.
const THUMBNAIL_EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

/// One prefab in the library.
pub struct Prefab {
    /// The name shown, the `.map` file's stem.
    pub name: String,
    /// The `.map` file itself.
    pub path: PathBuf,
}

/// An image in straight-alpha RGBA, row by row from the top.
#[derive(Debug, Clone, PartialEq)]
pub struct Rgba {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Four bytes per pixel.
    pub pixels: Vec<u8>,
}

impl Rgba {
    /// Encodes the image as a PNG file.
    pub fn save_png(&self, path: &Path) -> Result<(), String> {
        image::save_buffer(path, &self.pixels, self.width, self.height, image::ColorType::Rgba8)
            .map_err(|error| error.to_string())
    }
}

/// Encodes straight-alpha RGBA pixels as a PNG file's bytes.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    image::ImageEncoder::write_image(
        image::codecs::png::PngEncoder::new(&mut bytes),
        rgba,
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|error| error.to_string())?;
    Ok(bytes)
}

/// The library folders, from the preference, dropping ones that no longer exist.
pub fn folders() -> Vec<PathBuf> {
    crate::preferences::pref(|p| p.asset_library.clone())
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|path| crate::files::is_dir(path))
        .collect()
}

/// Writes the folder list back.
pub fn set_folders(folders: &[PathBuf]) {
    let joined = folders
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    crate::preferences::edit(|p| p.asset_library = joined);
}

/// Every `.map` in a folder, as a prefab, sorted by name.
pub fn scan(folder: &Path) -> Vec<Prefab> {
    let mut prefabs: Vec<Prefab> = crate::files::read_dir(folder)
        .into_iter()
        .flatten()
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("map"))
        })
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some(Prefab { name, path })
        })
        .collect();
    prefabs.sort_by(|a, b| a.name.cmp(&b.name));
    prefabs
}

/// When the file last changed, or the epoch if that cannot be told.
pub fn modified(path: &Path) -> SystemTime {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// The image files a prefab's thumbnail may be read from, in the order they are tried.
pub fn thumbnail_candidates(prefab: &Path) -> Vec<PathBuf> {
    THUMBNAIL_EXTENSIONS.iter().map(|extension| prefab.with_extension(extension)).collect()
}

/// A prefab's thumbnail: its image scaled to fit [`THUMBNAIL`], or a drawn preview of its
/// brushes when it has none. Safe to call off the UI thread.
pub fn thumbnail(prefab: &Path) -> Option<Rgba> {
    for candidate in thumbnail_candidates(prefab) {
        if let Ok(image) = crate::files::read(&candidate).map_err(image::ImageError::IoError).and_then(|bytes| image::load_from_memory(&bytes)) {
            let image = image.resize(THUMBNAIL as u32, THUMBNAIL as u32, image::imageops::FilterType::Triangle).to_rgba8();
            return Some(Rgba { width: image.width(), height: image.height(), pixels: image.into_raw() });
        }
    }
    let text = crate::files::read_to_string(prefab).ok()?;
    preview(&text, THUMBNAIL)
}

/// [`thumbnail`], kept on disk under `cache` keyed by the file's path, size and time
/// last changed (and those of any image beside it), so a library or a list of recent
/// maps opens with the thumbnails it showed last time instead of drawing every preview
/// again -- a preview of a big map takes a large fraction of a second. A changed file
/// gets a new key; stale entries are simply never read again.
pub fn thumbnail_cached(prefab: &Path, cache: &Path) -> Option<Rgba> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    THUMBNAIL.hash(&mut h);
    for file in std::iter::once(prefab.to_path_buf()).chain(thumbnail_candidates(prefab)) {
        file.hash(&mut h);
        if let Ok(meta) = fs::metadata(&file) {
            meta.len().hash(&mut h);
            meta.modified().ok().hash(&mut h);
        }
    }
    let entry = cache.join(format!("{:016x}.png", h.finish()));
    if let Ok(image) = image::open(&entry) {
        let image = image.to_rgba8();
        return Some(Rgba { width: image.width(), height: image.height(), pixels: image.into_raw() });
    }
    let made = thumbnail(prefab)?;
    if fs::create_dir_all(cache).is_ok() {
        if let Ok(bytes) = encode_png(made.width, made.height, &made.pixels) {
            // Written aside and renamed, so a reader never sees half a file.
            let partial = entry.with_extension(format!("{}.part", std::process::id()));
            if fs::write(&partial, bytes).is_ok() {
                let _ = fs::rename(&partial, &entry);
            }
        }
    }
    Some(made)
}

/// Where [`thumbnail_cached`] keeps the editor's thumbnails.
pub fn thumbnail_cache_dir() -> PathBuf {
    crate::preferences::config_dir().join("thumbnails")
}

/// Draws a prefab's brushes as a shaded isometric preview.
///
/// No GL: the `.map` is read into brushes, their faces are projected along a fixed
/// diagonal, with per-pixel depth and transparency. A face painted with a colour
/// (`// heron_brush color` / `gradient`) keeps its solid colour or interpolated gradient;
/// an unpainted face is a neutral grey.
pub fn preview(map_text: &str, size: i32) -> Option<Rgba> {
    preview_depth::render(map_text, size)
}

/// A scene preview with map-relative material sources resolved.
pub fn preview_file(path: &Path, size: i32) -> Option<Rgba> {
    preview_depth::render_file(path, size)
}

/// Reads a prefab `.map` and places its base at `at` on grid 4, one undo step.
///
/// Shared by the panel's Insert button and a drop onto a view. Returns whether anything
/// landed.
pub fn insert_prefab(document: &mut Document, path: &Path, at: Vec3d) -> bool {
    insert_prefab_on_grid(document, path, at, &crate::grid::Grid::new(2))
}

/// Inserts on the active grid, using the base and footprint as placement anchors.
pub fn insert_prefab_on_grid(document: &mut Document, path: &Path, at: Vec3d, grid: &crate::grid::Grid) -> bool {
    let text = match crate::files::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            crate::complain!("{}: {error}", path.display());
            return false;
        }
    };
    let name = path
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let added = document.paste_asset_on_grid(&text, at, &name, grid);
    if added == 0 {
        crate::complain!("{}: not a map", path.display());
        return false;
    }
    if let Some(folder) = path.parent() {
        load_folder_materials(document, folder);
    }
    crate::say!("inserted {name} ({added} objects)");
    true
}

fn load_folder_materials(document: &mut Document, folder: &Path) {
    let textures = folder.join("textures");
    if crate::files::is_dir(&textures) {
        match document.load_asset_materials(&textures) {
            Ok(warnings) => for warning in warnings { crate::complain!("asset materials: {warning}"); },
            Err(error) => crate::complain!("{}: {error}",textures.display()),
        }
    }
}

/// Restore bundled textures when opening a map that uses library assets.
pub fn load_library_materials(document: &mut Document) {
    for folder in folders() { load_folder_materials(document, &folder); }
}
