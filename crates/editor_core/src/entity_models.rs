//! Finding and keeping the models point entities are drawn as.
//!
//! Resolves entity models through the shared loose-file, PAK and ZIP/PK3 index, including
//! unextracted game assets.
//!
//! A model that cannot be found is remembered as missing, so a map full of entities whose
//! game is not installed does not go looking for the same file once per entity.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use map_model::asset_fs::AssetFileSystem;
use map_model::entity_model::EntityModel;
use map_model::sprite::Sprite;
use std::sync::Arc;

/// The models loaded so far, and where to look for more.
#[derive(Default)]
pub struct ModelStore {
    /// The game directory everything is looked for under.
    game_path: PathBuf,
    /// What has been loaded, by the path the definition names. `None` is a model that was
    /// looked for and not found.
    loaded: HashMap<String, Option<Rc<EntityModel>>>,
    /// The same for `.spr` sprites an `iconsprite(...)` names.
    sprites: HashMap<String, Option<Rc<Sprite>>>,
    /// Shared loose-file and archive index, mounted once.
    assets: Option<Arc<AssetFileSystem>>,
    /// Whether those have been looked for yet.
    mounted: bool,
    /// The palette the skins are coloured with.
    palette: Option<Vec<u8>>,
}

impl ModelStore {
    /// A store that looks under the given game directory.
    pub fn new(game_path: impl Into<PathBuf>) -> Self {
        Self {
            game_path: game_path.into(),
            ..Self::default()
        }
    }

    /// Where it is looking.
    pub fn game_path(&self) -> &Path {
        &self.game_path
    }

    /// Returns an already loaded source model without performing file IO.
    pub fn cached_model(&self, named: &str) -> Option<Rc<EntityModel>> {
        self.loaded.get(named).cloned().flatten()
    }

    /// The model a definition names, loading it the first time it is asked for.
    ///
    /// The name is the one the definition gives, such as `progs/armor.mdl`.
    pub fn model(&mut self, named: &str) -> Option<Rc<EntityModel>> {
        if let Some(known) = self.loaded.get(named) {
            return known.clone();
        }

        // Unsupported formats remain bounding boxes; supported readers report malformed files.
        let lowercase = named.to_lowercase();
        let is_mdl = lowercase.ends_with(".mdl");
        let is_obj = lowercase.ends_with(".obj");
        let is_glb = lowercase.ends_with(".glb");
        let is_md2 = lowercase.ends_with(".md2");
        let is_md3 = lowercase.ends_with(".md3");
        if !is_mdl && !is_obj && !is_glb && !is_md2 && !is_md3 {
            self.loaded.insert(named.to_string(), None);
            return None;
        }

        self.mount();
        let explicit = crate::files::is_absolute(Path::new(named));
        let bytes = if explicit {
            crate::files::read(Path::new(named)).ok()
        } else {
            self.read(named)
        };
        let model = bytes.and_then(|bytes| {
            let result = if is_glb {
                map_model::glb_model::read(&bytes)
            } else if is_md2 || is_md3 {
                let directory = Path::new(named).parent().unwrap_or(Path::new(""));
                let read = if is_md3 {
                    map_model::md3_model::read
                } else {
                    map_model::md2_model::read
                };
                read(&bytes, &mut |skin| {
                    if explicit {
                        let path = Path::new(skin);
                        let relative = directory.join(path);
                        crate::files::read(&relative).ok().or_else(|| {
                            let basename = path.file_name()?;
                            crate::files::read(&directory.join(basename)).ok()
                        })
                    } else {
                        self.read(skin)
                            .or_else(|| self.read(&directory.join(skin).to_string_lossy()))
                    }
                })
            } else if is_obj {
                // An `.obj` names its material file and that names its texture, both
                // beside it, so the reader is handed the way this store finds a file.
                let directory = std::path::Path::new(named)
                    .parent()
                    .map(|parent| parent.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut sibling = |name: &str| {
                    let relative = if directory.is_empty() {
                        name.to_string()
                    } else {
                        format!("{directory}/{name}")
                    };
                    let read = |path: &str| {
                        if explicit && crate::files::is_absolute(Path::new(path)) {
                            crate::files::read(Path::new(path)).ok()
                        } else {
                            self.read(path)
                        }
                    };
                    read(&relative).or_else(|| read(name))
                };
                map_model::obj_model::read(&bytes, &mut sibling).map_err(|why| why.to_string())
            } else if map_model::studio_model::is_studio_model(&bytes) {
                // A Half-Life model may keep its textures in `<name>t.mdl` beside it.
                let stem = &named[..named.len() - ".mdl".len()];
                let companion = format!("{stem}t.mdl");
                map_model::studio_model::read(&bytes, &mut || {
                    if explicit {
                        crate::files::read(Path::new(&companion)).ok()
                    } else {
                        self.read(&companion)
                    }
                })
            } else {
                let palette = self.palette.clone();
                map_model::entity_model::read(&bytes, palette.as_deref())
                    .map_err(|why| why.to_string())
            };
            match result {
                Ok(model) => Some(Rc::new(model)),
                Err(why) => {
                    crate::complain!("{named}: {why}");
                    None
                }
            }
        });
        self.loaded.insert(named.to_string(), model.clone());
        model
    }

    /// The `.spr` an `iconsprite(...)` names, loaded the first time it is asked for -- its
    /// first frame, as RGBA. A path that is not a `.spr` is left alone.
    pub fn sprite(&mut self, named: &str) -> Option<Rc<Sprite>> {
        if let Some(known) = self.sprites.get(named) {
            return known.clone();
        }
        if !named.to_lowercase().ends_with(".spr") {
            self.sprites.insert(named.to_string(), None);
            return None;
        }

        self.mount();
        let bytes = if Path::new(named).is_absolute() {
            crate::files::read(Path::new(named)).ok()
        } else {
            self.read(named)
        };
        let sprite = bytes.and_then(|bytes| {
            match map_model::sprite::read(&bytes, self.palette.as_deref()) {
                Ok(sprite) => Some(Rc::new(sprite)),
                Err(why) => {
                    crate::complain!("{named}: {why}");
                    None
                }
            }
        });
        self.sprites.insert(named.to_string(), sprite.clone());
        sprite
    }

    /// Mounts the standalone fallback roots and resolves the palette, once.
    ///
    /// A Quake game is either extracted beside the executable or left in its `.pak`s, and
    /// the directory the files are under is the mod's -- `id1` for the game itself -- so
    /// both the game path and its subdirectories are looked in.
    fn mount(&mut self) {
        if self.mounted {
            return;
        }
        self.mounted = true;
        if self.game_path.as_os_str().is_empty() {
            return;
        }

        if self.assets.is_none() {
            let roots: Vec<_> = self
                .directories()
                .into_iter()
                .rev()
                .filter(|p| crate::files::is_dir(p))
                .collect();
            let assets = Arc::new(AssetFileSystem::mount(&roots));
            for warning in assets.warnings() {
                crate::complain!("{warning}");
            }
            self.assets = Some(assets);
        }

        self.palette = self
            .read("gfx/palette.lmp")
            .or_else(|| self.read("palette.lmp"));
    }

    /// The directories a game's own files are looked for in.
    fn directories(&self) -> Vec<PathBuf> {
        let mut out = vec![self.game_path.clone()];
        // The game's own directory and the mods beside it: `id1` first, since that is
        // where the models a stock `.fgd` names live.
        for mod_directory in ["id1", "valve", "baseq2", "baseq3"] {
            out.push(self.game_path.join(mod_directory));
        }
        out
    }

    /// One file's bytes, from a directory or from a pak.
    fn read(&self, named: &str) -> Option<Vec<u8>> {
        let asset = self.assets.as_ref()?.resolve(named)?;
        match asset.read() {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                crate::complain!("{}: {error}", asset.display_path().display());
                None
            }
        }
    }

    /// A model reader using the document's shared asset index.
    pub fn with_assets(game_path: PathBuf, assets: Arc<AssetFileSystem>) -> Self {
        Self {
            game_path,
            assets: Some(assets),
            ..Self::default()
        }
    }

    /// An already mounted shared index, if available.
    pub fn shared_assets(&self) -> Option<&Arc<AssetFileSystem>> {
        self.assets.as_ref()
    }

    /// The shared index, mounting the legacy standalone search roots when needed.
    pub fn assets(&mut self) -> &Arc<AssetFileSystem> {
        self.mount();
        self.assets
            .get_or_insert_with(|| Arc::new(AssetFileSystem::default()))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn a_format_this_does_not_read_is_left_alone() {
        let mut store = ModelStore::new(std::env::temp_dir());
        assert!(store.model("sprites/flame.spr").is_none());
    }

    #[test]
    fn a_model_that_is_not_there_is_only_looked_for_once() {
        let mut store = ModelStore::new(std::env::temp_dir().join("heron-no-such-game"));
        assert!(store.model("progs/armor.mdl").is_none());
        assert!(store.model("progs/armor.mdl").is_none());
        assert_eq!(store.loaded.len(), 1, "it was remembered as missing");
    }

    #[test]
    fn a_model_beside_the_game_is_found_and_read() {
        let game = std::env::temp_dir().join("heron-model-store-loose");
        let _ = std::fs::remove_dir_all(&game);
        std::fs::create_dir_all(game.join("id1/progs")).expect("a game directory");
        std::fs::write(game.join("id1/progs/thing.mdl"), one_triangle_model())
            .expect("a written model");

        let mut store = ModelStore::new(&game);
        let model = store.model("progs/thing.mdl").expect("it is found");
        assert_eq!(model.triangles.len(), 1);

        let _ = std::fs::remove_dir_all(&game);
    }

    #[test]
    fn a_model_inside_a_pak_is_found_too() {
        let game = std::env::temp_dir().join("heron-model-store-pak");
        let _ = std::fs::remove_dir_all(&game);
        std::fs::create_dir_all(game.join("id1")).expect("a game directory");
        std::fs::write(
            game.join("id1/pak0.pak"),
            pak_holding(&one_triangle_model()),
        )
        .expect("a written pak");

        let mut store = ModelStore::new(&game);
        assert!(store.model("progs/thing.mdl").is_some());

        let _ = std::fs::remove_dir_all(&game);
    }

    #[test]
    fn md2_and_its_pcx_skin_are_identical_loose_pak_and_pk3() {
        let root = std::env::temp_dir().join("heron-md2-model-store");
        let _ = std::fs::remove_dir_all(&root);
        let mut expected = None;
        for container in ["loose", "pak", "pk3"] {
            let base = root.join(container);
            std::fs::create_dir_all(&base).unwrap();
            if container == "loose" {
                let dir = base.join("models/check");
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(
                    dir.join("preview.md2"),
                    include_bytes!("../../../examples/editor-checks/fixtures/md2/preview.md2"),
                )
                .unwrap();
                std::fs::write(
                    dir.join("skin.pcx"),
                    include_bytes!("../../../examples/editor-checks/fixtures/md2/skin.pcx"),
                )
                .unwrap();
            } else {
                let data: &[u8] = if container == "pak" {
                    include_bytes!("../../../examples/editor-checks/fixtures/md2/models.pak")
                } else {
                    include_bytes!("../../../examples/editor-checks/fixtures/md2/models.pk3")
                };
                std::fs::write(base.join(format!("pak0.{container}")), data).unwrap();
            }
            let mut store = ModelStore::new(&base);
            let model = store.model("models/check/preview.md2").unwrap();
            assert_eq!(model.triangles.len(), 6);
            assert_eq!(&model.skin.pixels[..4], &[255, 40, 40, 255]);
            if let Some(expected) = &expected {
                assert_eq!(&model, expected);
            } else {
                expected = Some(model.clone());
            }
            assert!(Rc::ptr_eq(
                &model,
                &store.model("models/check/preview.md2").unwrap()
            ));
            assert!(!base.join("models/check/skin.pcx").exists() || container == "loose");
        }
        // Explicit loose model imports resolve the skin beside the chosen model too.
        let path = root
            .join("loose/models/check/preview.md2")
            .to_string_lossy()
            .into_owned();
        let mut store = ModelStore::default();
        assert_eq!(&store.model(&path).unwrap(), expected.as_ref().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn md3_surfaces_and_skins_are_identical_loose_pak_and_pk3() {
        let root = std::env::temp_dir().join(format!(
            "heron-md3-model-store-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut expected = None;
        for container in ["loose", "pak", "pk3"] {
            let base = root.join(container);
            std::fs::create_dir_all(&base).unwrap();
            if container == "loose" {
                let dir = base.join("models/check");
                std::fs::create_dir_all(&dir).unwrap();
                for (name, bytes) in [
                    (
                        "preview.md3",
                        include_bytes!("../../../examples/editor-checks/fixtures/md3/preview.md3")
                            .as_slice(),
                    ),
                    (
                        "skin0.tga",
                        include_bytes!("../../../examples/editor-checks/fixtures/md3/skin0.tga")
                            .as_slice(),
                    ),
                    (
                        "skin1.tga",
                        include_bytes!("../../../examples/editor-checks/fixtures/md3/skin1.tga")
                            .as_slice(),
                    ),
                ] {
                    std::fs::write(dir.join(name), bytes).unwrap();
                }
            } else {
                let data: &[u8] = if container == "pak" {
                    include_bytes!("../../../examples/editor-checks/fixtures/md3/models.pak")
                } else {
                    include_bytes!("../../../examples/editor-checks/fixtures/md3/models.pk3")
                };
                std::fs::write(base.join(format!("pak0.{container}")), data).unwrap();
            }
            let mut store = ModelStore::new(&base);
            let model = store.model("models/check/preview.md3").unwrap();
            assert_eq!(model.triangles.len(), 2);
            assert_eq!(model.extra_skins.len(), 1);
            assert_eq!(&model.skin.pixels[..4], &[240, 20, 30, 255]);
            assert_eq!(&model.extra_skins[0].pixels[..4], &[10, 220, 40, 255]);
            if let Some(expected) = &expected {
                assert_eq!(&model, expected);
            } else {
                expected = Some(model.clone());
            }
            assert!(Rc::ptr_eq(
                &model,
                &store.model("models/check/preview.md3").unwrap()
            ));
            assert!(container == "loose" || !base.join("models/check").exists());
        }
        let path = root
            .join("loose/models/check/preview.md3")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            &ModelStore::default().model(&path).unwrap(),
            expected.as_ref().unwrap()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// The smallest `.mdl` there is: one triangle, one skin, one frame.
    pub fn one_triangle_model() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"IDPO");
        out.extend_from_slice(&6i32.to_le_bytes());
        for value in [1.0f32; 3] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32; 3] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&1.0f32.to_le_bytes());
        for value in [0.0f32; 3] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        for value in [1i32, 2, 2, 3, 1, 1, 0, 0] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&1.0f32.to_le_bytes());

        out.extend_from_slice(&0i32.to_le_bytes());
        out.extend_from_slice(&[0, 1, 2, 3]);
        for (on_seam, s, t) in [(0i32, 0i32, 0i32), (0, 1, 0), (0, 0, 1)] {
            out.extend_from_slice(&on_seam.to_le_bytes());
            out.extend_from_slice(&s.to_le_bytes());
            out.extend_from_slice(&t.to_le_bytes());
        }
        out.extend_from_slice(&1i32.to_le_bytes());
        for corner in [0i32, 1, 2] {
            out.extend_from_slice(&corner.to_le_bytes());
        }
        out.extend_from_slice(&0i32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&[16, 16, 16, 0]);
        out.extend_from_slice(&[b'f'; 16]);
        for vertex in [[0u8, 0, 0, 0], [16, 0, 0, 0], [0, 16, 0, 0]] {
            out.extend_from_slice(&vertex);
        }
        out
    }

    #[test]
    fn an_obj_beside_its_material_and_texture_is_read() {
        let game = std::env::temp_dir().join("heron-model-store-obj");
        let _ = std::fs::remove_dir_all(&game);
        std::fs::create_dir_all(game.join("id1/props")).expect("the directory");
        std::fs::write(
            game.join("id1/props/crate.obj"),
            "mtllib crate.mtl\nv 0 0 0\nv 8 0 0\nv 8 8 0\nvt 0 0\nvt 1 0\nvt 1 1\nf 1/1 2/2 3/3\n",
        )
        .expect("the model");
        std::fs::write(
            game.join("id1/props/crate.mtl"),
            "newmtl c\nmap_Kd crate.png\n",
        )
        .expect("the material");
        let image = image::RgbaImage::from_pixel(4, 2, image::Rgba([10, 20, 30, 255]));
        image
            .save(game.join("id1/props/crate.png"))
            .expect("the texture");

        let mut store = ModelStore::new(&game);
        let model = store.model("props/crate.obj").expect("it is found");
        assert_eq!(model.triangles.len(), 1);
        assert_eq!((model.skin.width, model.skin.height), (4, 2));
        assert_eq!(&model.skin.pixels[..4], &[10, 20, 30, 255]);

        // And a second ask comes from the cache rather than the disk.
        std::fs::remove_file(game.join("id1/props/crate.obj")).expect("it goes");
        assert!(store.model("props/crate.obj").is_some());
    }

    /// A pak holding that model at `progs/thing.mdl`.
    fn pak_holding(contents: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"PACK");
        out.extend_from_slice(&(12i32 + contents.len() as i32).to_le_bytes());
        out.extend_from_slice(&64i32.to_le_bytes());
        out.extend_from_slice(contents);
        let mut name = [0u8; 56];
        name[.."progs/thing.mdl".len()].copy_from_slice(b"progs/thing.mdl");
        out.extend_from_slice(&name);
        out.extend_from_slice(&12i32.to_le_bytes());
        out.extend_from_slice(&(contents.len() as i32).to_le_bytes());
        out
    }
}
