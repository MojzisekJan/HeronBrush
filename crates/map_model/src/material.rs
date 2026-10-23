//! Materials: the images a brush face can be given.
//!
//! A material comes from a *collection*, and a face names it by a string
//! that the collection resolves. `MaterialUtils::getMaterialNameFromPathSuffix` decides
//! what that string is: the file's path relative to the collection's root, with the
//! extension removed and `/` as the separator, so `walls/brick01.png` is `walls/brick01`.
//! That is what a `.map` file stores and what makes a collection swappable.
//!
//! Image-directory collections are loaded here; archive loading is handled by the document resource
//! layer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};

/// One material: a name and the image behind it.
///
/// What a face needs to know about it -- the name, the size UV coordinates are measured
/// against -- comes from the file's header when the folder is read. The pixels are
/// decoded only when something asks for them ([`Material::pixels`]), and are kept only
/// while someone holds them: a folder of ten thousand textures costs its headers, not
/// gigabytes of RGBA. Cloning a material is cheap; clones share the pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// What a face calls it.
    pub name: String,
    /// Where it was read from.
    pub path: PathBuf,
    /// The image's width in pixels.
    pub width: u32,
    /// The image's height in pixels.
    pub height: u32,
    /// The image, four bytes per pixel, rows top to bottom, decoded on demand.
    pixels: Pixels,
    /// The `surfaceparm`s of the shader this came from, `Material::surfaceParms`: how an
    /// idTech 3 game says a surface is a clip brush, a trigger, a hint or the sky. Empty
    /// for a material that is a plain image, which has nothing to say about itself.
    pub surface_parms: Vec<String>,
    /// Tangent-space normals in linear RGB, independent of albedo dimensions.
    pub normal_map: Option<NormalMap>,
    /// Imported glTF material, including original PBR images and factors.
    pub model_pbr: Option<Arc<crate::model_pbr::ModelPbr>>,
    /// The flags a Quake 2 `.wal` or a Heretic II `.m8` gives a face that names none of
    /// its own: the compiler makes a face wearing `sky1` sky because the texture says so,
    /// not the map.
    pub surface_defaults: Option<crate::uv::SurfaceAttributes>,
}

/// A linear tangent-space normal image associated with an albedo texture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalMap {
    /// Source file of the normal image.
    pub path: PathBuf,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Linear RGBA8 values, with alpha ignored by normal mapping; decoded on demand.
    pixels: Pixels,
    /// DirectX maps invert the green channel relative to OpenGL tangent space.
    pub flip_y: bool,
}

impl NormalMap {
    /// A normal map from pixels already in memory.
    pub fn rgba(path: PathBuf, width: u32, height: u32, pixels: Vec<u8>, flip_y: bool) -> Self {
        NormalMap {
            path,
            width,
            height,
            pixels: Pixels::memory(pixels),
            flip_y,
        }
    }

    /// A normal map read from `path` only when its pixels are asked for.
    pub fn lazy(path: PathBuf, width: u32, height: u32, flip_y: bool) -> Self {
        NormalMap {
            pixels: Pixels::file(path.clone(), true),
            path,
            width,
            height,
            flip_y,
        }
    }

    /// The pixels, `width * height * 4` bytes, decoded now if nobody holds them;
    /// `None` when the file no longer decodes to that size.
    pub fn pixels(&self) -> Option<Arc<[u8]>> {
        self.pixels.get((self.width * self.height * 4) as usize)
    }
}

/// What a decode says about an image beyond its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelInfo {
    /// How its alpha channel is drawn.
    pub alpha: Alpha,
    /// The mean colour (averaged in linear light, stored as sRGB) and mean alpha: what a
    /// renderer shows until the texture itself is resident.
    pub average: [u8; 4],
}

impl PixelInfo {
    /// Works the alpha class and average colour out of RGBA pixels.
    pub fn of(name: &str, pixels: &[u8]) -> Self {
        PixelInfo {
            alpha: classify_alpha(name, pixels),
            average: average_color(pixels),
        }
    }
}

/// Where a material's pixels come from, and the last decode while anyone holds it.
#[derive(Clone)]
struct Pixels(Arc<PixelsInner>);

struct PixelsInner {
    source: Source,
    palette: Option<Arc<[u8; 768]>>,
    /// The last decode, kept while any caller holds it, so two users of one image share
    /// a copy, and it goes away once neither needs it.
    resident: Mutex<Option<Weak<[u8]>>>,
    /// Known after the first decode, or from a cache of earlier ones.
    info: OnceLock<PixelInfo>,
    /// The file's header said it has no alpha channel, so it is opaque whatever else.
    opaque: bool,
}

enum Source {
    Memory(Arc<[u8]>),
    File(PathBuf),
    Asset(crate::asset_fs::AssetFile),
}

impl std::fmt::Debug for Pixels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0.source {
            Source::Memory(bytes) => write!(f, "Pixels({} bytes in memory)", bytes.len()),
            Source::File(path) => write!(f, "Pixels({})", path.display()),
            Source::Asset(asset) => write!(f, "Pixels({})", asset.display_path().display()),
        }
    }
}

impl PartialEq for Pixels {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || self.0.palette == other.0.palette
                && match (&self.0.source, &other.0.source) {
                    (Source::Memory(a), Source::Memory(b)) => a == b,
                    (Source::File(a), Source::File(b)) => a == b,
                    (Source::Asset(a), Source::Asset(b)) => a == b,
                    _ => false,
                }
    }
}

impl Eq for Pixels {}

impl Pixels {
    fn memory(pixels: Vec<u8>) -> Self {
        Pixels(Arc::new(PixelsInner {
            source: Source::Memory(pixels.into()),
            palette: None,
            resident: Mutex::new(None),
            info: OnceLock::new(),
            opaque: false,
        }))
    }

    fn file(path: PathBuf, opaque: bool) -> Self {
        Pixels(Arc::new(PixelsInner {
            source: Source::File(path),
            palette: None,
            resident: Mutex::new(None),
            info: OnceLock::new(),
            opaque,
        }))
    }

    fn asset(asset: crate::asset_fs::AssetFile, opaque: bool) -> Self {
        Pixels(Arc::new(PixelsInner {
            source: Source::Asset(asset),
            palette: None,
            resident: Mutex::new(None),
            info: OnceLock::new(),
            opaque,
        }))
    }

    /// A `.wal`, with the palette it is drawn with unless it carries its own.
    fn indexed(source: Source, palette: Option<Arc<[u8; 768]>>) -> Self {
        Pixels(Arc::new(PixelsInner {
            source,
            palette,
            resident: Mutex::new(None),
            info: OnceLock::new(),
            opaque: false,
        }))
    }

    fn resident(&self) -> Option<Arc<[u8]>> {
        match &self.0.source {
            Source::Memory(bytes) => Some(bytes.clone()),
            Source::File(_) | Source::Asset(_) => self.0.resident.lock().ok()?.as_ref()?.upgrade(),
        }
    }

    /// The pixels, `len` bytes, decoding the file if nobody holds them.
    fn get(&self, len: usize) -> Option<Arc<[u8]>> {
        let bytes = match &self.0.source {
            Source::Memory(bytes) => bytes.clone(),
            source @ (Source::File(_) | Source::Asset(_)) => {
                // Held across the decode, so two threads asking at once decode once.
                let mut resident = self.0.resident.lock().ok()?;
                match resident.as_ref().and_then(Weak::upgrade) {
                    Some(bytes) => bytes,
                    None => {
                        let (encoded, path) = match source {
                            Source::File(path) => (file_store::read(path).ok()?, path.clone()),
                            Source::Asset(asset) => {
                                (asset.read().ok()?, PathBuf::from(&asset.name))
                            }
                            Source::Memory(_) => unreachable!(),
                        };
                        let bytes: Arc<[u8]> = if is_wal(&path) {
                            crate::wal::read(&encoded, self.0.palette.as_deref())
                                .ok()?
                                .pixels
                                .into()
                        } else if is_m8(&path) {
                            crate::m8::read(&encoded).ok()?.pixels.into()
                        } else if is_pcx(&path) {
                            crate::pcx::read(&encoded).ok()?.pixels.into()
                        } else {
                            let decoded = image::ImageFormat::from_path(&path)
                                .ok()
                                .and_then(|format| {
                                    image::load_from_memory_with_format(&encoded, format).ok()
                                })
                                .or_else(|| image::load_from_memory(&encoded).ok())?;
                            decoded.to_rgba8().into_raw().into()
                        };
                        *resident = Some(Arc::downgrade(&bytes));
                        keep_recent(&bytes);
                        bytes
                    }
                }
            }
        };
        (bytes.len() >= len).then_some(bytes)
    }
}

/// How many bytes of recent decodes are held on to although nobody asked to keep them:
/// a view that asks for the same image every frame (the UV editor) decodes it once, and
/// memory stays bounded however many images are looked at.
const RECENT_BYTES: usize = 64 << 20;

fn keep_recent(bytes: &Arc<[u8]>) {
    static RECENT: Mutex<std::collections::VecDeque<Arc<[u8]>>> =
        Mutex::new(std::collections::VecDeque::new());
    let Ok(mut recent) = RECENT.lock() else {
        return;
    };
    recent.push_back(bytes.clone());
    let mut total: usize = recent.iter().map(|b| b.len()).sum();
    while total > RECENT_BYTES && recent.len() > 1 {
        total -= recent.pop_front().map_or(0, |b| b.len());
    }
}

/// The mean of RGBA pixels: colour averaged in linear light, alpha plainly.
fn average_color(pixels: &[u8]) -> [u8; 4] {
    let texels = pixels.as_chunks::<4>().0;
    if texels.is_empty() {
        return [0, 0, 0, 0];
    }
    let lut: [f32; 256] = std::array::from_fn(|c| srgb_to_linear(c as u8));
    let mut sum = [0f64; 4];
    // A strided sample of a big image is as good an average as every texel.
    let step = (texels.len() / 65_536).max(1);
    let mut count = 0.0;
    for texel in texels.iter().step_by(step) {
        for c in 0..3 {
            sum[c] += f64::from(lut[texel[c] as usize]);
        }
        sum[3] += f64::from(texel[3]);
        count += 1.0;
    }
    let colour = |c: usize| linear_to_srgb((sum[c] / count) as f32);
    [
        colour(0),
        colour(1),
        colour(2),
        (sum[3] / count).round() as u8,
    ]
}

/// An sRGB-encoded byte in linear light, 0 to 1.
pub fn srgb_to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear light, 0 to 1, as an sRGB-encoded byte.
pub fn linear_to_srgb(c: f32) -> u8 {
    let c = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

/// The image files of a folder by (directory, lower-case stem), sorted, for finding an
/// albedo's companion normal map without searching every file for every material: that
/// search was quadratic, a second for two thousand textures.
type ByStem = HashMap<(PathBuf, String), Vec<PathBuf>>;

fn files_by_stem(paths: &[PathBuf]) -> ByStem {
    let mut by_stem: ByStem = HashMap::new();
    for path in paths {
        let (Some(parent), Some(stem)) = (path.parent(), path.file_stem()) else {
            continue;
        };
        by_stem
            .entry((parent.to_path_buf(), stem.to_string_lossy().to_lowercase()))
            .or_default()
            .push(path.clone());
    }
    for list in by_stem.values_mut() {
        list.sort();
    }
    by_stem
}

fn collection_normal_path(by_stem: &ByStem, albedo: &Path, stem: &str) -> Option<PathBuf> {
    by_stem
        .get(&(albedo.parent()?.to_path_buf(), stem.to_lowercase()))?
        .first()
        .cloned()
}

/// How a material's transparency is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alpha {
    /// Every texel is opaque; draw it the ordinary way.
    Opaque,
    /// Every texel is either fully opaque or fully clear -- a fence, a leaf, a decal.
    /// Drawn in the opaque pass with the clear texels cut away.
    Masked,
    /// Some texel is part-way clear -- glass, water. Drawn in a blended pass on top of
    /// the opaque scene.
    Blended,
}

/// Classifies an RGBA image by its alpha channel. A name beginning `{` is the
/// Quake-lineage mark for a masked texture and counts as masked whatever the pixels say.
fn classify_alpha(name: &str, pixels: &[u8]) -> Alpha {
    let mut any_clear = name.starts_with('{');
    let mut any_partial = false;
    for chunk in pixels.as_chunks::<4>().0 {
        match chunk[3] {
            255 => {}
            0 => any_clear = true,
            _ => {
                any_partial = true;
                break;
            }
        }
    }
    if any_partial {
        Alpha::Blended
    } else if any_clear {
        Alpha::Masked
    } else {
        Alpha::Opaque
    }
}

impl Material {
    /// A material from an RGBA image in memory, its alpha class worked out from the name
    /// and the pixels.
    pub fn rgba(name: String, path: PathBuf, width: u32, height: u32, pixels: Vec<u8>) -> Self {
        let info = PixelInfo::of(&name, &pixels);
        let pixels = Pixels::memory(pixels);
        let _ = pixels.0.info.set(info);
        Material {
            name,
            path,
            width,
            height,
            pixels,
            surface_parms: Vec::new(),
            normal_map: None,
            model_pbr: None,
            surface_defaults: None,
        }
    }

    /// A material whose image is read from `path` only when its pixels are asked for.
    /// `has_alpha` is what the file's header says about an alpha channel: without one
    /// the material is known to be opaque before it is ever decoded.
    pub fn lazy(name: String, path: PathBuf, width: u32, height: u32, has_alpha: bool) -> Self {
        let pixels = Pixels::file(path.clone(), !has_alpha && !name.starts_with('{'));
        Material {
            name,
            path,
            width,
            height,
            pixels,
            surface_parms: Vec::new(),
            normal_map: None,
            model_pbr: None,
            surface_defaults: None,
        }
    }

    /// A lazy material backed by a resolved loose or archived resource.
    pub fn asset(name: String, asset: crate::asset_fs::AssetFile) -> Result<Self, MaterialError> {
        Self::asset_with_palette(name, asset, None)
    }

    fn asset_with_palette(
        name: String,
        asset: crate::asset_fs::AssetFile,
        palette: Option<Arc<[u8; 768]>>,
    ) -> Result<Self, MaterialError> {
        let path = asset.display_path();
        let (width, height, alpha) =
            asset_header(&asset).map_err(|message| MaterialError::Decode {
                path: path.clone(),
                message,
            })?;
        let mut surface_defaults = None;
        let pixels = if is_wal(Path::new(&asset.name)) {
            let header = if asset.is_archived() {
                asset.read().ok()
            } else {
                header_bytes(&asset.origin, 100)
            };
            surface_defaults = header.as_deref().and_then(crate::wal::surface_defaults);
            // Daikatana's carry their own palette.
            let palette = if header.as_deref().is_some_and(crate::wal::is_daikatana) {
                None
            } else {
                Some(palette.ok_or_else(|| MaterialError::Decode {
                    path: path.clone(),
                    message: "WAL requires an external palette (pics/colormap.pcx)".into(),
                })?)
            };
            Pixels::indexed(Source::Asset(asset), palette)
        } else if is_m8(Path::new(&asset.name)) {
            let header = if asset.is_archived() {
                asset.read().ok()
            } else {
                header_bytes(&asset.origin, crate::m8::HEADER_LEN)
            };
            surface_defaults = header.and_then(|bytes| crate::m8::surface_defaults(&bytes));
            Pixels::asset(asset, true)
        } else {
            Pixels::asset(asset, !alpha && !name.starts_with('{'))
        };
        Ok(Material {
            name,
            path,
            width,
            height,
            pixels,
            surface_parms: Vec::new(),
            normal_map: None,
            model_pbr: None,
            surface_defaults,
        })
    }

    /// Whether this texture came from the shared game/mod resolver.
    pub fn is_game_asset(&self) -> bool {
        matches!(self.pixels.0.source, Source::Asset(_))
    }

    /// Returns the size of the image, which is what UV coordinates are measured against.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The pixels, `width * height * 4` bytes, decoded now (on the calling thread) unless
    /// someone holds them already. `None` when the file no longer decodes to that size.
    ///
    /// Decoding also settles [`Material::info`]. Hold the result only as long as it is
    /// needed: that is what keeps unused images out of memory.
    pub fn pixels(&self) -> Option<Arc<[u8]>> {
        let pixels = self.pixels.get((self.width * self.height * 4) as usize)?;
        if self.pixels.0.info.get().is_none() {
            let _ = self.pixels.0.info.set(PixelInfo::of(&self.name, &pixels));
        }
        Some(pixels)
    }

    /// The pixels if they are in memory now (held by someone, or never on disk), without
    /// decoding anything.
    pub fn resident_pixels(&self) -> Option<Arc<[u8]>> {
        self.pixels
            .resident()
            .filter(|p| p.len() >= (self.width * self.height * 4) as usize)
    }

    /// Whether the pixels come from a file and are decoded on demand.
    pub fn is_lazy(&self) -> bool {
        matches!(self.pixels.0.source, Source::File(_))
    }

    /// The alpha class and average colour, once a decode (or [`Material::set_info`])
    /// has said.
    pub fn info(&self) -> Option<PixelInfo> {
        self.pixels.0.info.get().copied()
    }

    /// Settles [`Material::info`] from elsewhere (a cache of earlier decodes); ignored
    /// once it is known.
    pub fn set_info(&self, info: PixelInfo) {
        let _ = self.pixels.0.info.set(info);
    }

    /// The average colour, when known.
    pub fn average_color(&self) -> Option<[u8; 4]> {
        self.info().map(|info| info.average)
    }

    /// What the image's alpha channel means for drawing: solid, a cut-out mask, or
    /// see-through glass. Explicit imported model rules take precedence over pixels.
    /// Otherwise, until the image has been decoded this is the best guess: opaque
    /// when the header has no alpha channel (certain) or when it has one (a guess
    /// [`Material::alpha_known`] owns up to), masked for a `{` name.
    pub fn alpha(&self) -> Alpha {
        if let Some(pbr) = &self.model_pbr {
            return pbr.alpha_mode;
        }
        match self.info() {
            Some(info) => info.alpha,
            None if self.name.starts_with('{') => Alpha::Masked,
            None => Alpha::Opaque,
        }
    }

    /// [`Material::alpha`], decoding the image first if that is still a guess: for the
    /// exports, which must not hide what is behind glass.
    pub fn decoded_alpha(&self) -> Alpha {
        if !self.alpha_known() {
            drop(self.pixels());
        }
        self.alpha()
    }

    /// Whether [`Material::alpha`] is certain rather than a guess.
    pub fn alpha_known(&self) -> bool {
        self.model_pbr.is_some() || self.pixels.0.info.get().is_some() || self.pixels.0.opaque
    }

    /// Returns whether the name marks this material as masked.
    ///
    /// `isMaskedTextureName`: a leading `{` is the Quake-lineage convention for a texture
    /// whose last palette entry is transparent. Nothing here acts on it yet, but the name
    /// is what carries it, so the question belongs with the material.
    pub fn is_masked(&self) -> bool {
        self.name.starts_with('{')
    }
}

/// Why a material collection could not be read.
#[derive(Debug)]
pub enum MaterialError {
    /// The directory could not be read.
    Io(std::io::Error),
    /// A file that looked like an image could not be decoded.
    Decode {
        /// The file that failed.
        path: PathBuf,
        /// What the decoder said.
        message: String,
    },
}

impl std::fmt::Display for MaterialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterialError::Io(cause) => write!(f, "{cause}"),
            MaterialError::Decode { path, message } => {
                write!(f, "{}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for MaterialError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MaterialError::Io(cause) => Some(cause),
            MaterialError::Decode { .. } => None,
        }
    }
}

impl From<std::io::Error> for MaterialError {
    fn from(cause: std::io::Error) -> Self {
        MaterialError::Io(cause)
    }
}

/// The file extensions a collection is read from.
///
/// Enough for what a game ships, and no more: every one of these is a format the decoder
/// already has, so adding one here is not a promise the loader cannot keep.
const EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "tga", "pcx", "wal", "m8"];

/// The materials a map may use.
#[derive(Debug, Clone, Default)]
pub struct MaterialCollection {
    /// Where the collection was read from, which is what the names are relative to.
    root: PathBuf,
    materials: Vec<Material>,
    by_name: HashMap<String, usize>,
    /// Files that looked like images and were not, reported rather than swallowed.
    warnings: Vec<String>,
}

impl MaterialCollection {
    /// Resolves game textures and shader previews through the same sources as models.
    /// Image pixels remain lazy; archive payloads are not expanded into a temporary folder.
    pub fn load_assets(fs: &crate::asset_fs::AssetFileSystem, texture_root: &str) -> Self {
        Self::load_assets_with_palette(fs, texture_root, "pics/colormap.pcx", None)
    }

    /// Mounted palette overrides the optional bundled profile fallback.
    pub fn load_assets_with_palette(
        fs: &crate::asset_fs::AssetFileSystem,
        texture_root: &str,
        palette_name: &str,
        fallback: Option<Arc<[u8; 768]>>,
    ) -> Self {
        let prefix = crate::asset_fs::normalize(texture_root)
            .map(|p| format!("{p}/"))
            .unwrap_or_default();
        let mut collection = Self::default();
        let palette = match fs.resolve(palette_name) {
            Some(asset) => match asset
                .read()
                .and_then(|bytes| crate::wal::palette(&bytes, &asset.name))
            {
                Ok(palette) => Some(Arc::new(palette)),
                Err(error) => {
                    collection
                        .warnings
                        .push(format!("{}: {error}", asset.display_path().display()));
                    None
                }
            },
            None => fallback,
        };
        let mut images = std::collections::BTreeMap::<String, crate::asset_fs::AssetFile>::new();
        for asset in fs.files() {
            if !asset
                .name
                .get(..prefix.len())
                .is_some_and(|name| name.eq_ignore_ascii_case(&prefix))
            {
                continue;
            }
            let ext = Path::new(&asset.name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            let Some(rank) = EXTENSIONS
                .iter()
                .position(|known| ext.eq_ignore_ascii_case(known))
            else {
                continue;
            };
            let stem = Path::new(&asset.name)
                .with_extension("")
                .to_string_lossy()
                .to_string();
            let key = stem.to_lowercase();
            let replace = images.get(&key).is_none_or(|previous| {
                let ext = Path::new(&previous.name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default();
                rank < EXTENSIONS
                    .iter()
                    .position(|known| ext.eq_ignore_ascii_case(known))
                    .unwrap_or(usize::MAX)
            });
            if replace {
                images.insert(key, asset.clone());
            }
        }
        let mut companions = std::collections::HashSet::new();
        for (stem, asset) in &images {
            let name = Path::new(&asset.name[prefix.len()..])
                .with_extension("")
                .to_string_lossy()
                .to_string();
            let mut material =
                match Material::asset_with_palette(name, asset.clone(), palette.clone()) {
                    Ok(material) => material,
                    Err(error) => {
                        collection.warnings.push(error.to_string());
                        continue;
                    }
                };
            let base = ["_albedo", "_basecolor", "_diffuse"]
                .iter()
                .find_map(|suffix| stem.strip_suffix(suffix))
                .unwrap_or(stem);
            for suffix in ["_normal_gl", "_normal", "_n", "_normal_dx"] {
                let Some(normal) = images
                    .get(&format!("{base}{suffix}"))
                    .filter(|n| n.name != asset.name && !is_indexed(Path::new(&n.name)))
                else {
                    continue;
                };
                match asset_header(normal) {
                    Ok((width, height, _)) => {
                        material.normal_map = Some(NormalMap {
                            path: normal.display_path(),
                            width,
                            height,
                            pixels: Pixels::asset(normal.clone(), true),
                            flip_y: suffix == "_normal_dx",
                        });
                        companions.insert(normal.display_path());
                        break;
                    }
                    Err(error) => collection
                        .warnings
                        .push(format!("{}: {error}", normal.display_path().display())),
                }
            }
            collection.insert(material);
        }
        collection
            .materials
            .retain(|m| !companions.contains(&m.path));
        collection.by_name = collection
            .materials
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name.to_lowercase(), i))
            .collect();
        let normals: HashMap<_, _> = collection
            .materials
            .iter()
            .filter_map(|m| m.normal_map.clone().map(|n| (m.path.clone(), n)))
            .collect();
        for asset in fs.files().filter(|asset| {
            asset.name.to_lowercase().starts_with("scripts/")
                && asset.name.to_lowercase().ends_with(".shader")
        }) {
            let text = match asset
                .read()
                .and_then(|bytes| String::from_utf8(bytes).map_err(|e| e.to_string()))
            {
                Ok(text) => text,
                Err(error) => {
                    collection
                        .warnings
                        .push(format!("{}: {error}", asset.display_path().display()));
                    continue;
                }
            };
            for shader in crate::shader::read(&text) {
                // A map names a shader without the texture root, as it does an image:
                // q3map2 puts `textures/` back in front of every face's name. A shader
                // outside the root (a model's, an effect's) cannot go on a face at all.
                let Some(name) = shader
                    .name
                    .get(..prefix.len())
                    .filter(|root| root.eq_ignore_ascii_case(&prefix))
                    .map(|_| shader.name[prefix.len()..].to_string())
                else {
                    continue;
                };
                let mut material = None;
                for named in shader.image_candidates() {
                    let named = named.trim_start_matches('/');
                    let stem = Path::new(named).with_extension("");
                    let candidates = std::iter::once(named.to_string()).chain(
                        EXTENSIONS
                            .iter()
                            .map(|ext| stem.with_extension(ext).to_string_lossy().to_string()),
                    );
                    for path in candidates {
                        if let Some(asset) = fs.resolve(&path) {
                            match Material::asset_with_palette(
                                name.clone(),
                                asset.clone(),
                                palette.clone(),
                            ) {
                                Ok(mut found) => {
                                    found.normal_map = normals.get(&found.path).cloned();
                                    found.surface_parms = shader.surface_parms.clone();
                                    material = Some(found);
                                    break;
                                }
                                Err(error) => collection.warnings.push(error.to_string()),
                            }
                        }
                    }
                    if material.is_some() {
                        break;
                    }
                }
                match material {
                    Some(material) => collection.insert(material),
                    None => collection
                        .warnings
                        .push(format!("{}: no preview image in game assets", shader.name)),
                }
            }
        }
        collection
    }

    /// Reads every image under the given directory, including subdirectories: the
    /// headers only (name, size, whether there is an alpha channel); the pixels are
    /// decoded when asked for ([`Material::pixels`]).
    ///
    /// A file whose header cannot be read is reported as a warning and skipped: one bad
    /// file in a directory of hundreds should not cost the other 999.
    pub fn load_directory(root: impl AsRef<Path>) -> Result<Self, MaterialError> {
        let root = root.as_ref().to_path_buf();
        let mut collection = Self {
            root: root.clone(),
            ..Default::default()
        };

        let mut paths = Vec::new();
        collect_image_files(&root, &mut paths)?;
        // Read in a stable order, so the browser lists them the same way every time.
        paths.sort();

        for decoded in read_headers(&root, &paths) {
            match decoded {
                Ok(material) => {
                    let rank = |path: &Path| {
                        EXTENSIONS
                            .iter()
                            .position(|ext| {
                                path.extension()
                                    .is_some_and(|found| found.eq_ignore_ascii_case(ext))
                            })
                            .unwrap_or(usize::MAX)
                    };
                    if collection
                        .get(&material.name)
                        .is_none_or(|previous| rank(&material.path) < rank(&previous.path))
                    {
                        collection.insert(material);
                    }
                }
                Err(MaterialError::Decode { path, message }) => collection
                    .warnings
                    .push(format!("{}: {message}", path.display())),
                Err(error) => return Err(error),
            }
        }

        let mut companions = std::collections::HashSet::new();
        let by_stem = files_by_stem(&paths);
        for material in &mut collection.materials {
            let stem = material
                .path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            let base = ["_albedo", "_basecolor", "_diffuse"]
                .iter()
                .find_map(|suffix| stem.strip_suffix(suffix))
                .unwrap_or(&stem);
            for suffix in ["_normal_gl", "_normal", "_n", "_normal_dx"] {
                let Some(path) =
                    collection_normal_path(&by_stem, &material.path, &format!("{base}{suffix}"))
                        .filter(|path| path != &material.path && !is_indexed(path))
                else {
                    continue;
                };
                match read_header(&path) {
                    Ok((width, height, _)) => {
                        material.normal_map = Some(NormalMap::lazy(
                            path.clone(),
                            width,
                            height,
                            suffix == "_normal_dx",
                        ));
                        companions.insert(path);
                        break;
                    }
                    Err(error) => collection
                        .warnings
                        .push(format!("{}: {error}", path.display())),
                }
            }
        }

        // Paired data textures belong to their material, not to the albedo picker.
        collection
            .materials
            .retain(|m| !companions.contains(&m.path));
        collection.by_name = collection
            .materials
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name.to_lowercase(), i))
            .collect();
        Ok(collection)
    }

    /// The materials, given up rather than copied: each holds its pixels, and a folder of
    /// thousands of textures is hundreds of megabytes of them.
    pub fn into_materials(self) -> Vec<Material> {
        self.materials
    }

    /// Adds every shader in the collection's `scripts` directory as a material of its
    /// own, `loadShaderMaterials`.
    ///
    /// Shader materials use the first available image: editor preview, shader path, light image,
    /// then stage maps. Shaders with no resolvable image are omitted.
    ///
    /// A shader replaces an image of the same name, since the map is naming the shader.
    /// Returns what could not be read.
    pub fn load_shaders(&mut self, root: impl AsRef<Path>) -> Vec<String> {
        let root = root.as_ref();
        let mut warnings = Vec::new();
        let mut scripts: Vec<PathBuf> = match file_store::read_dir(&root.join("scripts")) {
            Ok(entries) => entries
                .into_iter()
                .filter(|path| {
                    path.extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("shader"))
                })
                .collect(),
            // No `scripts` directory: this is not an idTech 3 game, which is not a fault.
            Err(_) => return warnings,
        };
        // Read in a stable order, so two runs give the same collection.
        scripts.sort();

        for script in scripts {
            let Ok(text) = file_store::read_to_string(&script) else {
                warnings.push(format!("{}: could not be read", script.display()));
                continue;
            };
            for shader in crate::shader::read(&text) {
                match shader_material(root, &shader) {
                    Some(material) => self.insert(material),
                    None => warnings.push(format!(
                        "{}: none of the images it names is here",
                        shader.name
                    )),
                }
            }
        }
        warnings
    }

    /// Adds a material, replacing any of the same name.
    pub fn insert(&mut self, material: Material) {
        let key = material.name.to_lowercase();
        match self.by_name.get(&key) {
            Some(index) => self.materials[*index] = material,
            None => {
                self.by_name.insert(key, self.materials.len());
                self.materials.push(material);
            }
        }
    }

    /// Returns where in the collection a face's material sits.
    ///
    /// The index rather than the material, for the renderer, which groups its geometry by
    /// material and needs something cheap to sort by.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied().or_else(|| {
            let name = name.to_lowercase();
            self.by_name.get(&name).copied().or_else(|| {
                name.strip_prefix("textures/")
                    .and_then(|name| self.by_name.get(name).copied())
            })
        })
    }

    /// Returns the material a face's name refers to.
    pub fn get(&self, name: &str) -> Option<&Material> {
        self.index_of(name).map(|index| &self.materials[index])
    }

    /// Returns every material, in the order they were read.
    pub fn materials(&self) -> &[Material] {
        &self.materials
    }

    /// Returns how many materials there are.
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Returns whether there are none.
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    /// Returns where the collection was read from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns what the loader had to skip.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

/// What earlier decodes said about image files (alpha class, average colour), kept on
/// disk so a folder read again knows them from its headers' stat alone -- the
/// placeholders a renderer shows before a texture is resident, and which pass a surface
/// is drawn in, are right from the first frame.
///
/// One line per file: byte length, modification time in nanoseconds, alpha class
/// (`o`/`m`/`b`), the average colour as eight hex digits, and the path. An entry whose
/// stamp no longer matches the file is ignored. External-palette materials are excluded,
/// because a texture file stamp does not validate its palette.
#[derive(Debug, Default)]
pub struct InfoCache {
    entries: HashMap<PathBuf, (u64, u128, PixelInfo)>,
    changed: bool,
}

fn file_stamp(path: &Path) -> Option<(u64, u128)> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some((metadata.len(), modified))
}

impl InfoCache {
    /// Reads the cache at `file`; empty when there is none or it does not parse.
    pub fn load(file: &Path) -> Self {
        let mut cache = Self::default();
        let Ok(text) = std::fs::read_to_string(file) else {
            return cache;
        };
        for line in text.lines() {
            let mut parts = line.splitn(5, '\t');
            let (Some(len), Some(time), Some(alpha), Some(average), Some(path)) = (
                parts.next(),
                parts.next(),
                parts.next(),
                parts.next(),
                parts.next(),
            ) else {
                continue;
            };
            let alpha = match alpha {
                "o" => Alpha::Opaque,
                "m" => Alpha::Masked,
                "b" => Alpha::Blended,
                _ => continue,
            };
            let (Ok(len), Ok(time), Ok(average)) =
                (len.parse(), time.parse(), u32::from_str_radix(average, 16))
            else {
                continue;
            };
            cache.entries.insert(
                PathBuf::from(path),
                (
                    len,
                    time,
                    PixelInfo {
                        alpha,
                        average: average.to_be_bytes(),
                    },
                ),
            );
        }
        cache
    }

    /// Writes the cache to `file` if anything was added since it was read.
    pub fn save(&mut self, file: &Path) -> std::io::Result<()> {
        if !self.changed {
            return Ok(());
        }
        // Files that have gone (a temporary folder, a renamed texture) are forgotten.
        self.entries.retain(|path, _| path.is_file());
        let mut text = String::with_capacity(self.entries.len() * 80);
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (path, (len, time, info)) in entries {
            let alpha = match info.alpha {
                Alpha::Opaque => 'o',
                Alpha::Masked => 'm',
                Alpha::Blended => 'b',
            };
            text.push_str(&format!(
                "{len}\t{time}\t{alpha}\t{:08x}\t{}\n",
                u32::from_be_bytes(info.average),
                path.display()
            ));
        }
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, text)?;
        self.changed = false;
        Ok(())
    }

    /// Gives every lazy material of `materials` that the cache knows (at its file's
    /// current stamp) its info. Returns how many it knew.
    pub fn apply(&self, materials: &[Material]) -> usize {
        let mut known = 0;
        for material in materials
            .iter()
            .filter(|m| m.is_lazy() && m.info().is_none() && m.pixels.0.palette.is_none())
        {
            let Some((len, time, info)) = self.entries.get(&material.path) else {
                continue;
            };
            if file_stamp(&material.path) == Some((*len, *time)) {
                material.set_info(*info);
                known += 1;
            }
        }
        known
    }

    /// Takes in what is now known about the lazy materials of `materials`.
    pub fn record(&mut self, materials: &[Material]) {
        for material in materials
            .iter()
            .filter(|m| m.is_lazy() && m.pixels.0.palette.is_none())
        {
            let Some(info) = material.info() else {
                continue;
            };
            if self
                .entries
                .get(&material.path)
                .is_some_and(|(_, _, known)| *known == info)
            {
                continue;
            }
            if let Some((len, time)) = file_stamp(&material.path) {
                self.entries
                    .insert(material.path.clone(), (len, time, info));
                self.changed = true;
            }
        }
    }
}

/// Gathers every file under `directory` whose extension names an image format.
fn collect_image_files(directory: &Path, into: &mut Vec<PathBuf>) -> Result<(), MaterialError> {
    for path in file_store::read_dir(directory)? {
        if file_store::is_dir(&path) {
            collect_image_files(&path, into)?;
            continue;
        }

        let is_image = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                EXTENSIONS
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(extension))
            });
        if is_image {
            into.push(path);
        }
    }
    Ok(())
}

fn is_wal(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wal"))
}

fn is_m8(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("m8"))
}

/// A Quake 2 `.wal` or Heretic II `.m8`: indexed pixels that are never a normal map.
fn is_indexed(path: &Path) -> bool {
    is_wal(path) || is_m8(path)
}

fn is_pcx(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pcx"))
}

fn pcx_header(bytes: &[u8]) -> Result<(u32, u32, bool), String> {
    crate::pcx::dimensions(bytes).map(|(width, height)| (width, height, true))
}

/// Reads image dimensions and alpha availability without expanding pixels.
fn read_header(path: &Path) -> Result<(u32, u32, bool), String> {
    use image::ImageDecoder;
    if is_pcx(path) || is_wal(path) || is_m8(path) {
        #[cfg(not(target_arch = "wasm32"))]
        let length = if is_wal(path) {
            crate::wal::HEADER_BYTES
        } else if is_m8(path) {
            crate::m8::HEADER_LEN
        } else {
            128
        };
        let header = |bytes: &[u8]| {
            if is_wal(path) {
                crate::wal::header(bytes).map(|(w, h, _)| (w, h, true))
            } else if is_m8(path) {
                crate::m8::header(bytes).map(|(w, h, _)| (w, h, false))
            } else {
                pcx_header(bytes)
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let bytes = read_prefix(path, length).map_err(|error| error.to_string())?;
            return header(&bytes);
        }
        #[cfg(target_arch = "wasm32")]
        return header(&file_store::read(path).map_err(|error| error.to_string())?);
    }
    // The desktop reads only as far as the header; the web build has the file in memory.
    #[cfg(not(target_arch = "wasm32"))]
    let reader = image::ImageReader::open(path).map_err(|error| error.to_string())?;
    #[cfg(target_arch = "wasm32")]
    let reader = image::ImageReader::new(std::io::Cursor::new(
        file_store::read(path).map_err(|error| error.to_string())?,
    ));
    let decoder = reader
        .with_guessed_format()
        .map_err(|error| error.to_string())?
        .into_decoder()
        .map_err(|error| error.to_string())?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) * 4 > 256 << 20 {
        return Err("the image is empty".into());
    }
    Ok((width, height, decoder.color_type().has_alpha()))
}

fn asset_header(asset: &crate::asset_fs::AssetFile) -> Result<(u32, u32, bool), String> {
    use image::ImageDecoder;
    if !asset.is_archived() {
        return read_header(&asset.origin);
    }
    let bytes = asset.read()?;
    if is_pcx(Path::new(&asset.name)) {
        return pcx_header(&bytes);
    }
    if is_wal(Path::new(&asset.name)) {
        return crate::wal::header(&bytes).map(|(w, h, _)| (w, h, true));
    }
    if is_m8(Path::new(&asset.name)) {
        return crate::m8::header(&bytes).map(|(w, h, _)| (w, h, false));
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes));
    if let Ok(format) = image::ImageFormat::from_path(&asset.name) {
        reader.set_format(format);
    }
    let decoder = reader
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .into_decoder()
        .map_err(|e| e.to_string())?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || (width as u64) * (height as u64) * 4 > 256 << 20 {
        return Err("invalid or oversized texture dimensions".into());
    }
    Ok((width, height, decoder.color_type().has_alpha()))
}

/// [`read_material`] for every path, in order, on a few threads at once: ten thousand
/// headers are ten thousand small reads.
fn read_headers(root: &Path, paths: &[PathBuf]) -> Vec<Result<Material, MaterialError>> {
    let palette = if paths.iter().any(|path| is_wal(path)) {
        directory_palette(root)
    } else {
        Ok(None)
    };
    let read = |path: &Path| read_material_with_palette(root, path, &palette);
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(8)
        .min(paths.len() / 64)
        .max(1);
    if threads == 1 {
        return paths.iter().map(|path| read(path)).collect();
    }
    let chunk = paths.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let workers: Vec<_> = paths
            .chunks(chunk)
            .map(|part| scope.spawn(move || part.iter().map(|path| read(path)).collect::<Vec<_>>()))
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| {
                worker
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    })
}

/// One image file as a material, from its header; the pixels wait until asked for.
fn directory_palette(root: &Path) -> Result<Option<Arc<[u8; 768]>>, String> {
    for base in [Some(root), root.parent()].into_iter().flatten() {
        for suffix in ["pics/colormap.pcx", "colormap.pcx"] {
            let path = base.join(suffix);
            if file_store::is_file(&path) {
                return crate::pcx::palette(&file_store::read(&path).map_err(|e| e.to_string())?)
                    .map(|palette| Some(Arc::new(palette)))
                    .map_err(|e| format!("{}: {e}", path.display()));
            }
        }
    }
    Ok(None)
}

fn read_material(root: &Path, path: &Path) -> Result<Material, MaterialError> {
    let palette = if is_wal(path) {
        directory_palette(root)
    } else {
        Ok(None)
    };
    read_material_with_palette(root, path, &palette)
}

fn read_material_with_palette(
    root: &Path,
    path: &Path,
    palette: &Result<Option<Arc<[u8; 768]>>, String>,
) -> Result<Material, MaterialError> {
    let (width, height, has_alpha) =
        read_header(path).map_err(|message| MaterialError::Decode {
            path: path.to_path_buf(),
            message,
        })?;
    let mut material = Material::lazy(
        material_name(root, path),
        path.to_path_buf(),
        width,
        height,
        has_alpha,
    );
    if is_wal(path) {
        let header = header_bytes(path, crate::wal::HEADER_BYTES);
        // Daikatana's carry their own palette.
        let palette = if header.as_deref().is_some_and(crate::wal::is_daikatana) {
            None
        } else {
            let palette = palette
                .as_ref()
                .map_err(|message| MaterialError::Decode {
                    path: path.into(),
                    message: message.clone(),
                })?
                .clone()
                .ok_or_else(|| MaterialError::Decode {
                    path: path.into(),
                    message: "WAL requires an external palette (pics/colormap.pcx)".into(),
                })?;
            Some(palette)
        };
        material.pixels = Pixels::indexed(Source::File(path.to_path_buf()), palette);
        material.surface_defaults = header.as_deref().and_then(crate::wal::surface_defaults);
    } else if is_m8(path) {
        material.surface_defaults = header_bytes(path, crate::m8::HEADER_LEN)
            .and_then(|bytes| crate::m8::surface_defaults(&bytes));
    }
    Ok(material)
}

/// Up to the first `length` bytes of a loose file: a texture's header, which is up to
/// Daikatana's 892 bytes for a `.wal` (a Quake II one's is 100) and 1040 for an `.m8`.
fn header_bytes(path: &Path, length: usize) -> Option<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    return read_prefix(path, length).ok();
    #[cfg(target_arch = "wasm32")]
    {
        // The web build has the whole file in memory.
        let _ = length;
        file_store::read(path).ok()
    }
}

/// Up to the first `length` bytes of a file; fewer if the file is shorter.
#[cfg(not(target_arch = "wasm32"))]
fn read_prefix(path: &Path, length: usize) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::with_capacity(length);
    std::fs::File::open(path)?
        .take(length as u64)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// A shader as a material: the first of its images that is on disk, under the shader's
/// own name.
fn shader_material(root: &Path, shader: &crate::shader::Shader) -> Option<Material> {
    for candidate in shader.image_candidates() {
        for path in image_paths(root, candidate) {
            if let Ok(mut material) = read_material(root, &path) {
                material.name = shader.name.clone();
                material.surface_parms = shader.surface_parms.clone();
                return Some(material);
            }
        }
    }
    None
}

/// Where an image a shader names might be: at the path as written when that already ends
/// in an extension this reads, and otherwise with each of them tried in turn -- a shader
/// names `textures/x/y` and the file is `textures/x/y.tga` or `.jpg`.
fn image_paths(root: &Path, named: &str) -> Vec<PathBuf> {
    let named = named.trim_start_matches('/');
    let has_extension = Path::new(named)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        });
    let mut paths = Vec::with_capacity(EXTENSIONS.len() + 1);
    if has_extension {
        paths.push(root.join(named));
    }
    // A shader often names a `.tga` that the game ships as a `.jpg`, so the extension it
    // gives is not the last word: whatever it says, each is tried.
    let stem = Path::new(named).with_extension("");
    for extension in EXTENSIONS {
        let path = root.join(stem.with_extension(extension));
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

/// What a face calls the material at the given path.
///
/// `getMaterialNameFromPathSuffix`: relative to the collection's root, without the
/// extension, with `/` between the parts whatever the platform writes.
fn material_name(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    // `generic_string()`: the separator is `/` whatever the platform writes, so a map
    // built on Windows names the same material as one built anywhere else.
    relative
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod shader_tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("heron-shader-materials-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    fn write_image(path: &Path, colour: [u8; 4]) {
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        image::RgbaImage::from_pixel(2, 2, image::Rgba(colour))
            .save(path)
            .expect("the image");
    }

    #[test]
    fn a_shader_becomes_a_material_named_after_itself() {
        let root = scratch("named");
        write_image(&root.join("textures/base/wall.png"), [1, 2, 3, 255]);
        std::fs::create_dir_all(root.join("scripts")).expect("the directory");
        std::fs::write(
            root.join("scripts/base.shader"),
            "textures/base/fancywall\n{\nqer_editorimage textures/base/wall.tga\n}\n",
        )
        .expect("the script");

        let mut collection = MaterialCollection::load_directory(&root).expect("it loads");
        // The image on its own is one material, under its own path.
        assert!(collection.get("textures/base/wall").is_some());
        assert!(collection.get("textures/base/fancywall").is_none());

        let warnings = collection.load_shaders(&root);
        assert!(warnings.is_empty(), "{warnings:?}");
        // The shader names a `.tga` and the file is a `.png`; it is found anyway, which
        // is what a game shipping converted images needs.
        let material = collection
            .get("textures/base/fancywall")
            .expect("the shader is a material now");
        assert_eq!(material.size(), (2, 2));
    }

    #[test]
    fn a_shader_with_no_image_of_its_own_falls_back_to_its_name() {
        let root = scratch("byname");
        write_image(&root.join("textures/base/floor.png"), [4, 5, 6, 255]);
        std::fs::create_dir_all(root.join("scripts")).expect("the directory");
        std::fs::write(
            root.join("scripts/base.shader"),
            "textures/base/floor\n{\nsurfaceparm metalsteps\n}\n",
        )
        .expect("the script");

        let mut collection = MaterialCollection::default();
        let warnings = collection.load_shaders(&root);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(collection.get("textures/base/floor").is_some());
    }

    #[test]
    fn a_shader_naming_nothing_that_exists_is_left_out_with_a_word() {
        let root = scratch("missing");
        std::fs::create_dir_all(root.join("scripts")).expect("the directory");
        std::fs::write(
            root.join("scripts/base.shader"),
            "textures/base/ghost\n{\nqer_editorimage textures/base/ghost.tga\n}\n",
        )
        .expect("the script");

        let mut collection = MaterialCollection::default();
        let warnings = collection.load_shaders(&root);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(collection.is_empty());
    }

    #[test]
    fn a_mounted_game_s_shaders_are_named_as_a_map_names_them() {
        let root = scratch("mounted");
        write_image(&root.join("textures/skies/clouds.png"), [1, 2, 3, 255]);
        write_image(&root.join("textures/common/caulk.png"), [4, 5, 6, 255]);
        write_image(&root.join("models/thing/glow.png"), [7, 8, 9, 255]);
        std::fs::create_dir_all(root.join("scripts")).expect("the directory");
        std::fs::write(
            root.join("scripts/oa.shader"),
            "textures/skies/day\n{\nqer_editorimage textures/skies/clouds.tga\nsurfaceparm sky\n}\n\
             textures/common/caulk\n{\nsurfaceparm nodraw\n}\n\
             models/thing/glow\n{\n{\nmap models/thing/glow.tga\n}\n}\n",
        )
        .expect("the script");
        let assets = crate::asset_fs::AssetFileSystem::mount(std::slice::from_ref(&root));
        let collection = MaterialCollection::load_assets(&assets, "textures");

        // OpenArena's sky is `skies/day` in a map, which q3map2 reads as
        // `textures/skies/day`; a name with the root kept would be looked up as
        // `textures/textures/skies/day` and the sky would be lost.
        let sky = collection.get("skies/day").expect("the sky shader");
        assert_eq!(sky.name, "skies/day");
        assert_eq!(sky.surface_parms, ["sky"]);
        // The shader takes the place of the image of the same name, parms and all.
        let caulk = collection.get("common/caulk").expect("caulk");
        assert_eq!(caulk.surface_parms, ["nodraw"]);
        let names: Vec<_> = collection.materials().iter().map(|m| &m.name).collect();
        assert_eq!(names.len(), 3, "{names:?}");
        assert!(names
            .iter()
            .all(|name| !name.starts_with("textures/") && !name.starts_with("models/")));
    }

    #[test]
    fn a_game_with_no_scripts_directory_is_not_a_fault() {
        let root = scratch("noscripts");
        std::fs::create_dir_all(&root).expect("the directory");
        let mut collection = MaterialCollection::default();
        assert!(collection.load_shaders(&root).is_empty());
        assert!(collection.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a solid-colour PNG, so the tests do not need any files checked in.
    fn write_png(path: &Path, width: u32, height: u32, color: [u8; 4]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("a directory");
        }
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba(color));
        image.save(path).expect("a written png");
    }

    fn temp_dir(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("heron-materials-{name}"));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a directory");
        directory
    }

    #[test]
    fn companion_normals_pair_without_becoming_albedo_choices() {
        let root = temp_dir("normal-pair");
        write_png(&root.join("stone_albedo.png"), 1, 1, [180, 150, 120, 255]);
        write_png(
            &root.join("stone_NORMAL_DX.png"),
            2,
            2,
            [128, 230, 220, 255],
        );
        write_png(&root.join("orphan_normal.png"), 1, 1, [128, 128, 255, 255]);
        let collection = MaterialCollection::load_directory(&root).unwrap();
        assert_eq!(collection.materials().len(), 2);
        let normal = collection
            .get("stone_albedo")
            .unwrap()
            .normal_map
            .as_ref()
            .unwrap();
        assert!(normal.flip_y);
        assert_eq!(&normal.pixels().unwrap()[..4], &[128, 230, 220, 255]);
        assert!(collection.get("orphan_normal").is_some());
        assert!(collection.get("stone_NORMAL_DX").is_none());
        // A broken companion must not remove its valid base texture.
        std::fs::write(root.join("stone_normal_gl.png"), "broken").unwrap();
        let collection = MaterialCollection::load_directory(&root).unwrap();
        assert!(collection.get("stone_albedo").is_some());
        assert!(!collection.warnings().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_material_is_named_by_its_path_below_the_root() {
        let root = Path::new("/game/textures");
        assert_eq!(
            material_name(root, Path::new("/game/textures/brick01.png")),
            "brick01"
        );
        assert_eq!(
            material_name(root, Path::new("/game/textures/walls/brick01.png")),
            "walls/brick01"
        );
        // A file outside the root keeps its whole path rather than being renamed to
        // something that would collide.
        assert_eq!(
            material_name(root, Path::new("/elsewhere/brick01.png")),
            "/elsewhere/brick01"
        );
    }

    #[test]
    fn a_directory_of_images_becomes_a_collection() {
        let root = temp_dir("basic");
        write_png(&root.join("brick.png"), 64, 64, [200, 100, 60, 255]);
        write_png(
            &root.join("walls/plaster.png"),
            32,
            16,
            [220, 220, 210, 255],
        );

        let collection = MaterialCollection::load_directory(&root).expect("a collection");
        assert_eq!(collection.len(), 2);
        assert!(collection.warnings().is_empty());

        let brick = collection.get("brick").expect("the brick");
        assert_eq!(brick.size(), (64, 64));
        // Read from the header alone; decoded when asked for.
        assert!(brick.resident_pixels().is_none() && brick.info().is_none());
        let pixels = brick.pixels().expect("it decodes");
        assert_eq!(pixels.len(), 64 * 64 * 4);
        assert_eq!(&pixels[0..4], &[200, 100, 60, 255]);
        assert_eq!(brick.info().unwrap().average, [200, 100, 60, 255]);
        // Shared by clones while held (and for a while after, among the recent decodes).
        assert!(brick.clone().resident_pixels().is_some());
        drop(pixels);

        let plaster = collection.get("walls/plaster").expect("the plaster");
        assert_eq!(plaster.size(), (32, 16));

        // A name nothing was read under is not a material.
        assert!(collection.get("brick.png").is_none());
        assert!(collection.get("nothing").is_none());
    }

    #[test]
    fn a_file_that_will_not_decode_costs_only_itself() {
        let root = temp_dir("broken");
        write_png(&root.join("good.png"), 8, 8, [255; 4]);
        std::fs::write(root.join("bad.png"), b"this is not a png").expect("a written file");

        let collection = MaterialCollection::load_directory(&root).expect("a collection");
        assert_eq!(collection.len(), 1);
        assert!(collection.get("good").is_some());
        assert_eq!(collection.warnings().len(), 1);
        assert!(collection.warnings()[0].contains("bad.png"));
    }

    #[test]
    fn files_that_are_not_images_are_passed_over() {
        let root = temp_dir("mixed");
        write_png(&root.join("brick.png"), 8, 8, [255; 4]);
        std::fs::write(root.join("readme.txt"), b"nothing to see").expect("a written file");
        std::fs::write(root.join("palette.lmp"), b"\x00\x01\x02").expect("a written file");

        let collection = MaterialCollection::load_directory(&root).expect("a collection");
        assert_eq!(collection.len(), 1);
        // Not an image by its extension, so not a failure to decode either.
        assert!(collection.warnings().is_empty());
    }

    #[test]
    fn a_missing_directory_is_an_error_rather_than_an_empty_collection() {
        let missing = std::env::temp_dir().join("heron-materials-nothing-here");
        let _ = std::fs::remove_dir_all(&missing);
        assert!(MaterialCollection::load_directory(&missing).is_err());
    }

    #[test]
    fn the_same_name_read_twice_keeps_the_later_one() {
        let mut collection = MaterialCollection::default();
        let first = Material::rgba("brick".into(), "a.png".into(), 8, 8, vec![0; 8 * 8 * 4]);
        let second = Material {
            width: 16,
            ..first.clone()
        };

        collection.insert(first);
        collection.insert(second);
        assert_eq!(collection.len(), 1);
        assert_eq!(collection.get("brick").expect("the brick").width, 16);
    }

    #[test]
    fn a_leading_brace_marks_a_masked_material() {
        let masked = Material::rgba("{grate".into(), PathBuf::new(), 1, 1, vec![0; 4]);
        assert!(masked.is_masked());

        let plain = Material {
            name: "grate".into(),
            ..masked
        };
        assert!(!plain.is_masked());
    }

    #[test]
    fn imported_alpha_rules_override_pixel_classification_and_lazy_guesses() {
        for mode in [Alpha::Opaque, Alpha::Masked, Alpha::Blended] {
            let mut material = Material::rgba(
                "imported".into(),
                PathBuf::new(),
                1,
                1,
                vec![255, 255, 255, 128],
            );
            material.model_pbr = Some(Arc::new(crate::model_pbr::ModelPbr {
                alpha_mode: mode,
                ..Default::default()
            }));
            assert_eq!(material.alpha(), mode);
            assert_eq!(material.decoded_alpha(), mode);
            let mut lazy =
                Material::lazy("imported".into(), PathBuf::from("missing.png"), 1, 1, true);
            lazy.model_pbr = material.model_pbr.clone();
            assert!(lazy.alpha_known());
            assert_eq!(lazy.decoded_alpha(), mode);
            assert!(
                lazy.resident_pixels().is_none(),
                "Explicit source rules must not force an image decode"
            );
        }
    }

    #[test]
    fn the_header_says_opaque_and_the_cache_remembers_what_a_decode_said() {
        let root = temp_dir("info-cache");
        image::RgbImage::from_pixel(4, 4, image::Rgb([10, 20, 30]))
            .save(root.join("solid.png"))
            .unwrap();
        write_png(&root.join("glass.png"), 4, 4, [200, 200, 255, 100]);
        let collection = MaterialCollection::load_directory(&root).unwrap();
        let solid = collection.get("solid").unwrap();
        let glass = collection.get("glass").unwrap();
        // No alpha channel: opaque for certain, before any decode.
        assert!(solid.alpha_known() && solid.alpha() == Alpha::Opaque);
        // An alpha channel: unknown until decoded.
        assert!(!glass.alpha_known());
        drop(glass.pixels());
        assert_eq!(glass.alpha(), Alpha::Blended);

        let file = root.join("cache.txt");
        let mut cache = InfoCache::default();
        cache.record(collection.materials());
        cache.save(&file).unwrap();
        let again = MaterialCollection::load_directory(&root).unwrap();
        assert_eq!(InfoCache::load(&file).apply(again.materials()), 1);
        let glass = again.get("glass").unwrap();
        assert!(glass.alpha_known() && glass.resident_pixels().is_none());
        assert_eq!(
            glass.info(),
            Some(PixelInfo {
                alpha: Alpha::Blended,
                average: [200, 200, 255, 100]
            })
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod pcx_material_tests {
    use super::*;

    fn pcx() -> Vec<u8> {
        let mut bytes = vec![0; 128];
        bytes[..4].copy_from_slice(&[10, 5, 1, 8]);
        bytes[8] = 1;
        bytes[65] = 1;
        bytes[66] = 2;
        bytes.extend_from_slice(&[1, 0xc1, 255, 12]);
        let mut palette = [0; 768];
        palette[3..6].copy_from_slice(&[21, 43, 65]);
        palette[765..].copy_from_slice(&[87, 109, 131]);
        bytes.extend_from_slice(&palette);
        bytes
    }

    fn root(suffix: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "heron-pcx-material-{}-{suffix}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("textures/base")).unwrap();
        root
    }

    #[test]
    fn pcx_directory_materials_are_lazy_and_decode_palette_alpha() {
        let root = root("directory");
        std::fs::write(root.join("textures/base/wall.PCX"), pcx()).unwrap();
        std::fs::write(root.join("textures/bad.pcx"), [0; 128]).unwrap();
        let collection = MaterialCollection::load_directory(root.join("textures")).unwrap();
        assert_eq!(collection.warnings().len(), 1);
        let material = collection.get("base/wall").unwrap();
        assert_eq!(material.size(), (2, 1));
        assert!(material.resident_pixels().is_none());
        assert!(!material.alpha_known());
        assert_eq!(
            &*material.pixels().unwrap(),
            &[21, 43, 65, 255, 87, 109, 131, 0]
        );
        assert_eq!(material.alpha(), Alpha::Masked);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pcx_archive_materials_remain_lazy_and_loose_files_override_them() {
        let root = root("archive");
        let payload = pcx();
        let mut pak = b"PACK".to_vec();
        pak.extend_from_slice(&(12u32 + payload.len() as u32).to_le_bytes());
        pak.extend_from_slice(&64u32.to_le_bytes());
        pak.extend_from_slice(&payload);
        let mut entry = [0; 56];
        let name = b"textures/base/wall.pcx";
        entry[..name.len()].copy_from_slice(name);
        pak.extend_from_slice(&entry);
        pak.extend_from_slice(&12u32.to_le_bytes());
        pak.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        std::fs::write(root.join("pak0.pak"), pak).unwrap();
        let assets = crate::asset_fs::AssetFileSystem::mount(std::slice::from_ref(&root));
        let collection = MaterialCollection::load_assets(&assets, "textures");
        assert!(
            collection.warnings().is_empty(),
            "{:?}",
            collection.warnings()
        );
        let material = collection.get("base/wall").unwrap();
        assert!(material.is_game_asset());
        assert!(material.path.to_string_lossy().contains("pak0.pak!"));
        assert!(material.resident_pixels().is_none());
        assert_eq!(
            &*material.pixels().unwrap(),
            &[21, 43, 65, 255, 87, 109, 131, 0]
        );
        let mut loose = pcx();
        let palette = loose.len() - 768;
        loose[palette + 3..palette + 6].copy_from_slice(&[9, 8, 7]);
        std::fs::write(root.join("textures/base/wall.pcx"), loose).unwrap();
        let assets = crate::asset_fs::AssetFileSystem::mount(std::slice::from_ref(&root));
        let collection = MaterialCollection::load_assets(&assets, "textures");
        assert_eq!(
            &collection.get("base/wall").unwrap().pixels().unwrap()[..4],
            &[9, 8, 7, 255]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_pcx_pixels_fail_lazily_and_png_keeps_extension_priority() {
        let root = root("malformed");
        let mut broken = pcx();
        broken[128] = 0xc0;
        std::fs::write(root.join("textures/broken.pcx"), broken).unwrap();
        std::fs::write(root.join("textures/wall.pcx"), pcx()).unwrap();
        image::RgbaImage::from_pixel(1, 1, image::Rgba([1u8, 2, 3, 255]))
            .save(root.join("textures/wall.png"))
            .unwrap();
        let assets = crate::asset_fs::AssetFileSystem::mount(std::slice::from_ref(&root));
        let collection = MaterialCollection::load_assets(&assets, "textures");
        assert!(collection.get("broken").unwrap().pixels().is_none());
        assert_eq!(collection.get("wall").unwrap().size(), (1, 1));
        assert_eq!(
            &*collection.get("wall").unwrap().pixels().unwrap(),
            &[1, 2, 3, 255]
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod wal_material_tests {
    use super::*;
    fn palette(color: [u8; 3]) -> Vec<u8> {
        let mut bytes = vec![0; 128];
        bytes[..4].copy_from_slice(&[10, 5, 1, 8]);
        bytes[65] = 1;
        bytes[66] = 1;
        bytes.extend([1, 12]);
        let mut rgb = [0; 768];
        rgb[3..6].copy_from_slice(&color);
        bytes.extend(rgb);
        bytes
    }
    fn root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("heron-wal-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("textures/base")).unwrap();
        root
    }
    fn pak(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut bytes = b"PACK".to_vec();
        bytes.extend([0; 8]);
        let mut directory = Vec::new();
        for (name, data) in entries {
            let mut entry = [0; 56];
            entry[..name.len()].copy_from_slice(name.as_bytes());
            directory.extend(entry);
            directory.extend((bytes.len() as u32).to_le_bytes());
            directory.extend((data.len() as u32).to_le_bytes());
            bytes.extend(data);
        }
        let offset = bytes.len() as u32;
        bytes[4..8].copy_from_slice(&offset.to_le_bytes());
        bytes[8..12].copy_from_slice(&(directory.len() as u32).to_le_bytes());
        bytes.extend(directory);
        bytes
    }
    #[test]
    fn a_wal_s_header_flags_reach_the_material_from_every_source() {
        let root = root("flags");
        let mut sky = crate::wal::tests::fixture();
        sky[88..92].copy_from_slice(&4i32.to_le_bytes());
        sky[96..100].copy_from_slice(&300i32.to_le_bytes());
        let pcx = palette([12, 34, 56]);
        std::fs::create_dir_all(root.join("pics")).unwrap();
        std::fs::write(root.join("pics/colormap.pcx"), &pcx).unwrap();
        std::fs::write(root.join("textures/base/sky.wal"), &sky).unwrap();
        std::fs::write(
            root.join("textures/base/plain.wal"),
            crate::wal::tests::fixture(),
        )
        .unwrap();
        std::fs::write(
            root.join("pak0.pak"),
            pak(&[
                ("textures/base/packed_sky.wal", sky),
                ("pics/colormap.pcx", pcx),
            ]),
        )
        .unwrap();
        let folder = MaterialCollection::load_directory(root.join("textures")).unwrap();
        let assets = crate::asset_fs::AssetFileSystem::mount(std::slice::from_ref(&root));
        let mounted = MaterialCollection::load_assets(&assets, "textures");
        let expected = Some(crate::uv::SurfaceAttributes {
            content_flags: 0,
            surface_flags: 4,
            value: 300,
            color: None,
        });
        for material in [
            folder.get("base/sky").unwrap(),
            mounted.get("base/sky").unwrap(),
            mounted.get("base/packed_sky").unwrap(),
        ] {
            assert_eq!(material.surface_defaults, expected, "{}", material.name);
        }
        assert_eq!(mounted.get("base/plain").unwrap().surface_defaults, None);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn an_m8_brings_its_own_palette_and_flags_from_every_source() {
        let root = root("m8");
        let mut wood = crate::m8::tests::fixture();
        wood[1028..1032].copy_from_slice(&0x0300_0000i32.to_le_bytes());
        // Heretic II's demo keeps its textures in folders named like `Silverspring`.
        std::fs::create_dir_all(root.join("textures/Silverspring")).expect("mkdir");
        std::fs::write(root.join("textures/Silverspring/beam.m8"), &wood).expect("write");
        std::fs::write(
            root.join("pak0.pak"),
            pak(&[("textures/base/packed.M8", wood)]),
        )
        .expect("write");
        let folder =
            MaterialCollection::load_directory(root.join("textures")).expect("the folder reads");
        assert!(folder.warnings().is_empty(), "{:?}", folder.warnings());
        let assets = crate::asset_fs::AssetFileSystem::mount(std::slice::from_ref(&root));
        let mounted = MaterialCollection::load_assets(&assets, "textures");
        assert!(mounted.warnings().is_empty(), "{:?}", mounted.warnings());
        let expected = Some(crate::uv::SurfaceAttributes {
            content_flags: 0,
            surface_flags: 0x0300_0000,
            value: 0,
            color: None,
        });
        for material in [
            folder
                .get("Silverspring/beam")
                .expect("loose, from the folder"),
            mounted.get("Silverspring/beam").expect("loose, mounted"),
            mounted.get("base/packed").expect("in the pak"),
        ] {
            assert_eq!(material.size(), (8, 4), "{}", material.name);
            assert_eq!(material.surface_defaults, expected, "{}", material.name);
            assert!(material.resident_pixels().is_none());
            assert_eq!(
                &material.pixels().expect("the pixels decode")[..8],
                &[0, 0, 0, 255, 12, 34, 56, 255]
            );
            assert_eq!(material.alpha(), Alpha::Opaque, "{}", material.name);
        }
        std::fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn wal_materials_decode_lazily_from_directories_and_pak_with_shared_palette() {
        let root = root("sources");
        let wal = crate::wal::tests::fixture();
        let pcx = palette([12, 34, 56]);
        std::fs::create_dir_all(root.join("pics")).unwrap();
        std::fs::write(root.join("pics/colormap.pcx"), &pcx).unwrap();
        std::fs::write(root.join("textures/base/loose.WAL"), &wal).unwrap();
        std::fs::write(
            root.join("pak0.pak"),
            pak(&[
                ("textures/base/packed.wal", wal),
                ("pics/colormap.pcx", pcx),
            ]),
        )
        .unwrap();
        let folder = MaterialCollection::load_directory(root.join("textures")).unwrap();
        assert!(folder.warnings().is_empty(), "{:?}", folder.warnings());
        let assets = crate::asset_fs::AssetFileSystem::mount(std::slice::from_ref(&root));
        let mounted = MaterialCollection::load_assets(&assets, "textures");
        assert!(mounted.warnings().is_empty(), "{:?}", mounted.warnings());
        for material in [
            folder.get("base/loose").unwrap(),
            mounted.get("base/loose").unwrap(),
            mounted.get("base/packed").unwrap(),
        ] {
            assert_eq!(material.size(), (8, 8));
            assert!(material.resident_pixels().is_none());
            assert_eq!(
                &material.pixels().unwrap()[..8],
                &[0, 0, 0, 0, 12, 34, 56, 255]
            );
            assert_eq!(material.alpha(), Alpha::Masked);
        }
        image::RgbaImage::from_pixel(1, 1, image::Rgba([4u8, 5, 6, 255]))
            .save(root.join("textures/base/loose.png"))
            .unwrap();
        let preferred = MaterialCollection::load_directory(root.join("textures")).unwrap();
        assert_eq!(
            preferred.get("base/loose").unwrap().size(),
            (1, 1),
            "Converted PNG takes priority over WAL in folders too"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn wal_palette_reload_mod_override_and_profile_fallback_change_pixels() {
        let root = root("palette-reload");
        let mods = root.join("mod");
        std::fs::create_dir_all(mods.join("pics")).unwrap();
        std::fs::write(
            root.join("textures/base/wall.wal"),
            crate::wal::tests::fixture(),
        )
        .unwrap();
        let fallback = Arc::new(crate::pcx::palette(&palette([1, 2, 3])).unwrap());
        let assets = crate::asset_fs::AssetFileSystem::mount(std::slice::from_ref(&root));
        let first = MaterialCollection::load_assets_with_palette(
            &assets,
            "textures",
            "pics/colormap.pcx",
            Some(fallback.clone()),
        );
        let original = first.get("base/wall").unwrap();
        assert_eq!(&original.pixels().unwrap()[4..8], &[1, 2, 3, 255]);
        let mut cache = InfoCache::default();
        cache.record(first.materials());
        std::fs::write(mods.join("pics/colormap.pcx"), palette([9, 8, 7])).unwrap();
        let assets = crate::asset_fs::AssetFileSystem::mount(&[root.clone(), mods]);
        let changed = MaterialCollection::load_assets_with_palette(
            &assets,
            "textures",
            "pics/colormap.pcx",
            Some(fallback),
        );
        assert_eq!(
            cache.apply(changed.materials()),
            0,
            "A WAL file stamp does not validate its external palette"
        );
        let material = changed.get("base/wall").unwrap();
        assert_ne!(original, material);
        assert_eq!(&material.pixels().unwrap()[4..8], &[9, 8, 7, 255]);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn wal_missing_or_invalid_palette_is_reported_and_bad_mips_fail_on_decode() {
        let root = root("invalid");
        std::fs::write(
            root.join("textures/base/wall.wal"),
            crate::wal::tests::fixture(),
        )
        .unwrap();
        let folder = MaterialCollection::load_directory(root.join("textures")).unwrap();
        assert!(folder.get("base/wall").is_none());
        assert!(folder
            .warnings()
            .iter()
            .any(|warning| warning.contains("external palette")));
        std::fs::create_dir_all(root.join("pics")).unwrap();
        std::fs::write(root.join("pics/colormap.pcx"), [0; 128]).unwrap();
        let assets = crate::asset_fs::AssetFileSystem::mount(std::slice::from_ref(&root));
        let mounted = MaterialCollection::load_assets(&assets, "textures");
        assert!(mounted.get("base/wall").is_none());
        assert!(mounted.warnings().len() >= 2);
        std::fs::write(root.join("pics/colormap.pcx"), palette([1, 2, 3])).unwrap();
        let mut broken = crate::wal::tests::fixture();
        broken.pop();
        std::fs::write(root.join("textures/base/wall.wal"), broken).unwrap();
        let folder = MaterialCollection::load_directory(root.join("textures")).unwrap();
        assert!(folder.get("base/wall").unwrap().pixels().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
