//! Document operations for materials.

use super::{
    brush_node, node, Brush, Document, Edit, FaceRef, HashMap, MaterialCollection, MaterialSource,
    NodeId, Path, PathBuf, Vec2f,
};

/// Editor helper textures are embedded so maps never depend on a project copy.
pub(super) fn add_special_materials(collection: &mut MaterialCollection) {
    use map_model::material::Material;
    for (name, bytes) in [
        (
            "skip",
            include_bytes!("../../../../assets/textures/special/skip.png").as_slice(),
        ),
        (
            "clip",
            include_bytes!("../../../../assets/textures/special/clip.png").as_slice(),
        ),
        (
            "origin",
            include_bytes!("../../../../assets/textures/special/origin.png").as_slice(),
        ),
    ] {
        if collection.get(name).is_some() {
            continue;
        }
        let image = image::load_from_memory(bytes)
            .expect("embedded helper texture")
            .to_rgba8();
        collection.insert(Material::rgba(
            name.into(),
            PathBuf::from(format!("builtin/special/{name}.png")),
            image.width(),
            image.height(),
            image.into_raw(),
        ));
    }
}

/// Where [`Document::save_material_info`] keeps what it learnt; none in this crate's own
/// tests, which must not write to the user's configuration.
fn material_info_file() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    Some(crate::preferences::config_dir().join("material-info.txt"))
}

pub(super) fn initial_materials() -> MaterialCollection {
    let mut collection = MaterialCollection::default();
    add_special_materials(&mut collection);
    collection
}

impl Document {
    /// Remounts game and mod resources and invalidates model/material caches together.
    pub fn reload_game_assets(&mut self) {
        self.game_asset_key = None;
        self.load_entity_models();
    }

    /// Current mount/texture diagnostics, without rescanning the game directory.
    pub fn game_asset_warnings(&self) -> &[String] {
        &self.game_asset_warnings
    }

    pub(super) fn sync_game_assets(&mut self) -> bool {
        let key = crate::preferences::pref(|p| {
            (p.game.clone(), p.game_path.clone(), p.game_mods.clone())
        });
        if self.game_asset_key.as_ref() == Some(&key) {
            return false;
        }
        let path = PathBuf::from(&key.1);
        // Without a game path the profile's own assets folder is still mounted.
        let game = crate::games::current();
        let roots = game
            .as_ref()
            .map(|game| game.asset_roots(&path, &key.2))
            .unwrap_or_else(|| {
                if path.as_os_str().is_empty() {
                    Vec::new()
                } else {
                    let mut roots = vec![path.clone()];
                    roots.extend(
                        key.2
                            .split(';')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(|s| path.join(s)),
                    );
                    roots
                }
            });
        let assets = std::sync::Arc::new(map_model::asset_fs::AssetFileSystem::mount(&roots));
        let palette_name = game
            .as_ref()
            .and_then(|g| g.materials_palette.as_deref())
            .unwrap_or("pics/colormap.pcx");
        let fallback = game.as_ref().and_then(|game| {
            let name = game.materials_palette.as_deref()?;
            [
                game.directory.join(name),
                game.directory.join(Path::new(name).file_name()?),
            ]
            .into_iter()
            .find_map(|path| {
                file_store::read(&path)
                    .ok()
                    .and_then(|bytes| map_model::wal::palette(&bytes, name).ok())
                    .map(std::sync::Arc::new)
            })
        });
        let collection = MaterialCollection::load_assets_with_palette(
            &assets,
            game.as_ref()
                .and_then(|g| g.materials_root.as_deref())
                .unwrap_or("textures"),
            palette_name,
            fallback,
        );
        self.game_asset_warnings = assets
            .warnings()
            .iter()
            .chain(collection.warnings())
            .cloned()
            .collect();
        for warning in &self.game_asset_warnings {
            crate::complain!("game assets: {warning}");
        }
        self.game_materials = collection.into_materials();
        // Quetoo keeps each texture's normal, specular and glow maps beside it; the
        // profile's excludes say which images those are.
        if let Some(game) = &game {
            self.game_materials
                .retain(|material| !game.excludes_material(&material.name));
        }
        self.model_store = crate::entity_models::ModelStore::with_assets(path, assets);
        self.models.clear();
        self.sprites.clear();
        self.game_asset_key = Some(key);
        true
    }

    /// Adds the textures in a Quake `.wad` to the material collection.
    ///
    /// Loads textures from a `.wad` named by worldspawn's `wad` property. WAD3 contains its
    /// palette; WAD2 requires `palette.lmp` beside the archive or under the game path. WAD2
    /// textures are skipped when no palette is available.
    ///
    /// Returns what had to be skipped.
    pub fn load_wad(&mut self, path: &Path) -> Result<Vec<String>, String> {
        // Read once here, so a file that is not a `.wad` is an error before it is added
        // to what the map draws on.
        let warnings = self.read_wad(path)?;
        self.material_sources.retain(|source| source.path() != path);
        self.material_sources
            .push(MaterialSource::Package(path.to_path_buf()));
        self.rebuild_materials();
        Ok(warnings)
    }

    /// [`Self::load_wad`], and names the package in worldspawn's `wad` property, which is
    /// where the compiler looks for the textures the map uses (`qbsp` reads no others).
    ///
    /// A map that has been saved names it relative to its own folder, which is how both
    /// the compilers and [`Self::wads_named_by_the_map`] resolve it; an unsaved one by its
    /// absolute path. A package the property already names is not named twice. The
    /// property change is one undo step.
    pub fn add_wad_to_map(&mut self, path: &Path) -> Result<Vec<String>, String> {
        let warnings = self.load_wad(path)?;
        let written = match &self.path {
            Some(map) => relative_material_path(path, map),
            None => absolute_material_path(path)
                .to_string_lossy()
                .replace('\\', "/"),
        };
        let named = self
            .tree
            .world
            .properties
            .iter()
            .find(|(key, _)| key == "wad")
            .map(|(_, value)| value.clone())
            .unwrap_or_default();
        let mut packages: Vec<&str> = named
            .split(';')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect();
        let already = self
            .wads_named_by_the_map()
            .iter()
            .any(|found| absolute_material_path(found) == absolute_material_path(path));
        if !already {
            packages.push(&written);
            let value = packages.join(";");
            self.set_entity_property_on(&[Self::WORLDSPAWN], "wad", &value);
        }
        Ok(warnings)
    }

    /// Loads each package worldspawn's `wad` names that is not loaded yet: after the map
    /// opens, and after the property is edited. Returns each package tried, with what it
    /// skipped or why it failed.
    pub fn load_named_wads(&mut self) -> Vec<(PathBuf, Result<Vec<String>, String>)> {
        let mut tried = Vec::new();
        for wad in self.wads_named_by_the_map() {
            // The same file may be spelled differently by the map's own note on its
            // material sources, which is read first.
            let same = |path: &Path| {
                path == wad || absolute_material_path(path) == absolute_material_path(&wad)
            };
            let loaded = self
                .material_sources
                .iter()
                .any(|source| matches!(source, MaterialSource::Package(path) if same(path)));
            if !loaded {
                let result = self.load_wad(&wad);
                tried.push((wad, result));
            }
        }
        tried
    }

    /// Reads one `.wad` into the collection, without touching the list of sources.
    ///
    /// What both the loader above and the rebuild below do; the difference between them
    /// is the bookkeeping, not the reading.
    fn read_wad(&mut self, path: &Path) -> Result<Vec<String>, String> {
        // A WAD2's palette may be in the game's packages, which are mounted lazily.
        self.sync_game_assets();
        let wad = scene_export::wad::Wad::open(path).map_err(|why| why.to_string())?;

        let palette = [
            path.with_file_name("palette.lmp"),
            PathBuf::from(crate::preferences::pref(|p| p.game_path.clone())).join("palette.lmp"),
        ]
        .into_iter()
        .find(|candidate| crate::files::is_file(candidate))
        .and_then(|candidate| scene_export::wad::read_palette(&candidate).ok())
        .or_else(|| {
            let assets = self.model_store.shared_assets()?;
            let bytes = assets
                .read("gfx/palette.lmp")
                .or_else(|| assets.read("palette.lmp"))?;
            bytes.as_slice().try_into().ok()
        });

        let mut warnings = Vec::new();
        let mut added = 0;
        for entry in wad.entries() {
            let Some(pixels) = wad.pixels(entry, palette.as_ref()) else {
                warnings.push(format!("{}: no palette to colour it with", entry.name));
                continue;
            };
            self.materials.insert(map_model::material::Material::rgba(
                entry.name.clone(),
                path.to_path_buf(),
                entry.width,
                entry.height,
                pixels,
            ));
            added += 1;
        }

        if added == 0 && warnings.is_empty() {
            warnings.push(format!("{} holds no textures", path.display()));
        }
        Ok(warnings)
    }

    /// The `.wad` files worldspawn names, as paths to try.
    ///
    /// `Model::WorldNode`'s `wad` property, which is a `;` separated list. A name is
    /// looked for as it is written, under the game path, and by its own file name in both
    /// -- which is what makes a map written on someone else's machine open here.
    pub fn wads_named_by_the_map(&self) -> Vec<PathBuf> {
        let Some(named) = self
            .tree
            .world
            .properties
            .iter()
            .find(|(key, _)| key == "wad")
            .map(|(_, value)| value.clone())
        else {
            return Vec::new();
        };
        let game_path = crate::preferences::pref(|p| p.game_path.clone());

        let mut paths = Vec::new();
        for name in named.split(';').map(str::trim).filter(|n| !n.is_empty()) {
            let written = PathBuf::from(name);
            let file_name = written.file_name().map(PathBuf::from);
            if !written.is_absolute() {
                if let Some(asset) = self.model_store.shared_assets().and_then(|fs| {
                    fs.resolve(name).or_else(|| {
                        file_name
                            .as_ref()
                            .and_then(|file| fs.resolve(&file.to_string_lossy()))
                    })
                }) {
                    if !asset.is_archived() {
                        paths.push(asset.origin.clone());
                        continue;
                    }
                }
            }
            // The compilers resolve a relative name against the map's own folder, so
            // that is the first place to look; the working directory is a last resort.
            let map_directory = self.path.as_ref().and_then(|map| map.parent());
            let candidates = [
                map_directory
                    .filter(|_| !written.is_absolute())
                    .map(|directory| directory.join(&written)),
                Some(written.clone()),
                (!game_path.is_empty()).then(|| PathBuf::from(&game_path).join(&written)),
                file_name
                    .as_ref()
                    .map(|name| PathBuf::from(&game_path).join(name))
                    .filter(|_| !game_path.is_empty()),
                self.path
                    .as_ref()
                    .and_then(|map| map.parent())
                    .zip(file_name.as_ref())
                    .map(|(directory, name)| directory.join(name)),
            ];
            if let Some(found) = candidates
                .into_iter()
                .flatten()
                .find(|candidate| crate::files::is_file(candidate))
            {
                paths.push(found);
            }
        }
        paths
    }

    /// Reads a collection of materials from a directory of images.
    ///
    /// Adds material sources without replacing existing ones. Adding the same folder again moves it
    /// to the end rather than loading it twice. Returns the entries skipped by the loader.
    pub fn load_materials(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<String>, map_model::material::MaterialError> {
        self.load_material_folder(path.as_ref(), false)
    }

    /// Asset textures are defaults; explicit project material sources keep precedence.
    pub fn load_asset_materials(
        &mut self,
        path: &Path,
    ) -> Result<Vec<String>, map_model::material::MaterialError> {
        if self
            .material_sources
            .iter()
            .any(|source| source.path() == path)
        {
            return Ok(Vec::new());
        }
        let current = self.current_material.clone();
        let result = self.load_material_folder(path, true);
        self.current_material = current;
        result
    }

    fn load_material_folder(
        &mut self,
        path: &Path,
        defaults: bool,
    ) -> Result<Vec<String>, map_model::material::MaterialError> {
        let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path = resolved.as_path();
        if defaults {
            self.material_defaults.insert(resolved.clone());
        } else {
            self.material_defaults.remove(path);
        }
        // Loaded once here so a folder that cannot be read is an error before anything
        // is changed, rather than an empty collection afterwards.
        let mut collection = MaterialCollection::load_directory(path)?;
        let mut warnings = collection.warnings().to_vec();
        // An idTech 3 game keeps some of its surfaces in `scripts/*.shader` rather than
        // as images; a directory with no `scripts` in it is left alone.
        warnings.extend(collection.load_shaders(path));

        self.material_sources.retain(|source| source.path() != path);
        let source = MaterialSource::Folder(path.to_path_buf());
        if defaults {
            self.material_sources.insert(0, source);
        } else {
            self.material_sources.push(source);
        }
        // The folder was read just above; reading it again for the rebuild doubled the
        // time a big texture folder took to load.
        self.rebuild_materials_with(Some((path.to_path_buf(), collection)));
        Ok(warnings)
    }

    /// What the materials have been loaded from, in the order they were loaded.
    pub fn material_sources(&self) -> &[MaterialSource] {
        &self.material_sources
    }

    /// Takes one source away and loads what is left again.
    ///
    /// `MaterialCollectionEditor`'s remove: a collection cannot have a folder unpicked
    /// out of it, so what is left is loaded afresh.
    pub fn remove_material_source(&mut self, at: usize) -> bool {
        if at >= self.material_sources.len() {
            return false;
        }
        self.material_sources.remove(at);
        self.rebuild_materials();
        true
    }

    /// Loads every source again, in order, into an empty collection.
    ///
    /// Later sources win where two of them hold a material of the same name, which is
    /// what loading them in order means.
    pub(super) fn rebuild_materials(&mut self) {
        self.rebuild_materials_with(None);
    }

    /// [`Document::rebuild_materials`], with one folder's collection already read (shaders
    /// and all), which is used rather than read again.
    fn rebuild_materials_with(&mut self, mut loaded: Option<(PathBuf, MaterialCollection)>) {
        // What was loaded before, by name, for the folders that have not changed to take
        // their materials back from: adding a second folder used to decode the first
        // one's every image again.
        // What this session's decodes learnt about the outgoing collection is kept for
        // the next time those files are read.
        self.save_material_info();
        let mut previous: HashMap<String, map_model::material::Material> =
            std::mem::take(&mut self.materials)
                .into_materials()
                .into_iter()
                .map(|material| (material.name.clone(), material))
                .collect();
        let mut game_inserted = false;
        let mut folders = std::mem::take(&mut self.material_folders);
        for source in self.material_sources.clone() {
            if !game_inserted && !self.material_defaults.contains(source.path()) {
                for material in &self.game_materials {
                    self.materials.insert(material.clone());
                }
                game_inserted = true;
            }
            match source {
                MaterialSource::Folder(path) => {
                    let stamp = folder_stamp(&path);
                    let materials: Vec<map_model::material::Material> =
                        match loaded.take_if(|(at, _)| *at == path) {
                            Some((_, collection)) => collection.into_materials(),
                            None => match folders
                                .get(&path)
                                .filter(|(was, _)| *was == stamp)
                                .and_then(|(_, names)| take_named(&mut previous, names, &path))
                            {
                                Some(kept) => kept,
                                None => MaterialCollection::load_directory(&path)
                                    .ok()
                                    .map(|mut collection| {
                                        collection.load_shaders(&path);
                                        collection.into_materials()
                                    })
                                    .unwrap_or_default(),
                            },
                        };
                    folders.insert(
                        path.clone(),
                        (
                            stamp,
                            materials
                                .iter()
                                .map(|material| material.name.clone())
                                .collect(),
                        ),
                    );
                    for material in materials {
                        self.materials.insert(material);
                    }
                }
                MaterialSource::Package(path) => {
                    let _ = self.read_wad(&path);
                }
            }
        }

        if !game_inserted {
            for material in &self.game_materials {
                self.materials.insert(material.clone());
            }
        }
        drop(previous);
        folders.retain(|path, _| {
            self.material_sources
                .iter()
                .any(|source| matches!(source, MaterialSource::Folder(at) if at == path))
        });
        self.material_folders = folders;

        // The collection was emptied, so the models' skins have to go back into it.
        self.load_entity_models();

        self.materials_revision = self.materials_revision.wrapping_add(1);
        // A map drawn before the materials were loaded keeps the placeholder; once there
        // is a collection, the first material in it is a better default.
        if self.materials.get(&self.current_material).is_none() {
            if let Some(first) = self.materials.materials().first() {
                self.current_material = first.name.clone();
            } else {
                self.current_material = map_model::shapes::DEFAULT_MATERIAL.to_string();
            }
        }
        add_special_materials(&mut self.materials);
        // Alpha classes and average colours from earlier sessions' decodes, so the
        // renderer's placeholders and passes are right before anything is decoded.
        if let Some(file) = material_info_file() {
            map_model::material::InfoCache::load(&file).apply(self.materials.materials());
        }
        self.resolve_brush_primitive_uv();
        self.mark_changed();
    }

    /// Writes what decodes have found out about the lazily read materials (alpha class,
    /// average colour) to the cache in the configuration directory. Called when the
    /// collection is replaced, and by a frontend when it closes.
    pub fn save_material_info(&self) {
        let Some(file) = material_info_file() else {
            return;
        };
        if !self
            .materials
            .materials()
            .iter()
            .any(|m| m.is_lazy() && m.info().is_some())
        {
            return;
        }
        let mut cache = map_model::material::InfoCache::load(&file);
        cache.record(self.materials.materials());
        let _ = cache.save(&file);
    }

    /// Turns any `brushDef` / `brushDef3` face's pending texture matrix into real UV axes,
    /// now that the material sizes are known. A no-op for a map that has none, which is
    /// almost all of them; a face whose material is still missing keeps its matrix for the
    /// next time a collection loads.
    fn resolve_brush_primitive_uv(&mut self) {
        let size_of = |name: &str| {
            self.materials
                .get(name)
                .map(|material| Vec2f::new(material.width as f32, material.height as f32))
        };
        let resolved: Vec<(NodeId, map_model::brush::Brush)> =
            node::brushes_with_ids(&self.tree.world)
                .into_iter()
                .filter_map(|(id, brush)| brush.resolved_primitive_uv(size_of).map(|b| (id, b)))
                .collect();
        for (id, brush) in resolved {
            node::replace_brush(&mut self.tree.world, id, brush);
        }
    }

    /// Returns the materials a face may wear.
    pub fn materials(&self) -> &MaterialCollection {
        &self.materials
    }

    /// Returns a counter that changes whenever the collection is replaced.
    pub fn materials_revision(&self) -> u64 {
        self.materials_revision
    }

    /// Returns which material a newly drawn brush gets.
    ///
    /// `Map::currentMaterialName`.
    pub fn current_material(&self) -> &str {
        &self.current_material
    }

    /// Sets which material a newly drawn brush gets.
    pub fn set_current_material(&mut self, name: impl Into<String>) {
        self.current_material = name.into();
    }

    /// Puts the current material on every selected face, or on every face of every
    /// selected brush.
    ///
    /// Replaces any solid/gradient paint so the assigned texture is visible; UVs stay intact.
    /// Returns the number of changed faces. An explicit face selection takes precedence over a
    /// brush selection.
    pub fn apply_current_material(&mut self) -> usize {
        let mut faces = self.selected_edit_faces();
        if faces.is_empty() {
            for (id, brush) in node::brushes_with_ids(&self.tree.world) {
                if self.selection.contains(&id) {
                    faces.extend(
                        (0..brush.geometry().face_count()).map(|face| FaceRef { node: id, face }),
                    );
                }
            }
        }
        self.apply_current_material_on(&faces)
    }

    /// Assign the current material to face-tool handles without expanding to their brush.
    pub fn apply_current_material_on(&mut self, faces: &[FaceRef]) -> usize {
        let name = self.current_material.clone();
        self.edit_faces_on(faces, |face| {
            if face.material_name == name && face.color.is_none() {
                return false;
            }
            face.material_name = name.clone().into();
            face.set_paint(None);
            true
        })
    }

    /// Puts a material on one face, wherever the pointer is, without selecting it first.
    ///
    /// Assigns the material name without changing UV mapping. The operation is one undo step and
    /// returns whether any face changed.
    pub fn begin_material_stroke(&self) -> super::MaterialStroke {
        super::MaterialStroke {
            material: self.current_material.clone(),
            visited: Default::default(),
            mark: self.history.undo_len(),
            revision: self.revision(),
        }
    }

    /// Paint only newly crossed faces, coalescing this gesture without older edits.
    pub fn paint_material_stroke(
        &mut self,
        faces: &[FaceRef],
        stroke: &mut super::MaterialStroke,
    ) -> usize {
        if self.revision() != stroke.revision {
            return 0;
        }
        let targets: Vec<_> = faces
            .iter()
            .copied()
            .filter(|face| stroke.visited.insert(*face))
            .collect();
        let before_len = self.history.undo_len();
        let changed = self.edit_faces_on(&targets, |face| {
            if face.material_name == stroke.material && face.color.is_none() {
                return false;
            }
            face.material_name = stroke.material.clone().into();
            face.set_paint(None);
            true
        });
        if changed > 0 {
            let evicted = (before_len + 1).saturating_sub(self.history.undo_len());
            stroke.mark = stroke.mark.saturating_sub(evicted);
            self.history.coalesce_since(stroke.mark);
            stroke.revision = self.revision();
        }
        changed
    }

    pub fn paint_face(&mut self, target: FaceRef, material: &str) -> bool {
        self.paint_faces(target.node, &[target.face], material)
    }

    /// Puts a material on every face of a brush. The paint tool's double click.
    pub fn paint_brush(&mut self, node: NodeId, material: &str) -> bool {
        let Some(brush) = self.brush(node) else {
            return false;
        };
        let faces: Vec<usize> = (0..brush.geometry().face_count()).collect();
        self.paint_faces(node, &faces, material)
    }

    /// Shared body of [`Self::paint_face`] and [`Self::paint_brush`]: sets the material
    /// on the given geometry faces of one brush and records it as a single edit.
    fn paint_faces(&mut self, node: NodeId, geometry_faces: &[usize], material: &str) -> bool {
        let Some(brush) = self.brush(node) else {
            return false;
        };
        let brush = brush.clone();

        let mut faces = brush.faces().to_vec();
        let mut changed = false;
        for &geometry_face in geometry_faces {
            let Some(source) = brush.source_face_index(geometry_face) else {
                continue;
            };
            if faces[source].material_name != material || faces[source].color.is_some() {
                faces[source].material_name = material.into();
                faces[source].set_paint(None);
                changed = true;
            }
        }
        if !changed {
            return false;
        }

        let Ok(edited) = Brush::from_faces(&self.world_bounds, faces) else {
            return false;
        };
        self.apply_and_record(Edit::Changed {
            before: vec![brush_node(node, brush)],
            after: vec![brush_node(node, edited)],
        });
        true
    }

    /// Every material name worn by a face somewhere in the map, sorted, without repeats.
    ///
    /// `ReplaceMaterialDialog` fills its "replace" list from the whole collection and its
    /// "with" list from the same; this is for showing which of them are actually in use,
    /// so replacing one that is not there is not offered.
    pub fn materials_in_use(&self) -> Vec<String> {
        let mut names: Vec<String> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .flat_map(|(_, brush)| {
                brush
                    .faces()
                    .iter()
                    .map(|face| face.material_name.to_string())
            })
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Whether a face is in reach of [`Self::replace_material`]: any face when nothing is
    /// selected, otherwise a face of a selected brush or a face that is selected itself.
    fn face_in_replace_scope(&self, scoped: bool, face: FaceRef) -> bool {
        !scoped || self.selection.contains(&face.node) || self.is_face_selected(face)
    }

    /// How many faces [`Self::replace_material`] would change, for the dialog's preview.
    pub fn replaceable_face_count(&self, from: &str) -> usize {
        let scoped = !self.selection.is_empty() || !self.face_selection.is_empty();
        let mut count = 0;
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            for index in 0..brush.geometry().face_count() {
                let face = FaceRef {
                    node: id,
                    face: index,
                };
                if !self.face_in_replace_scope(scoped, face) {
                    continue;
                }
                if let Some(source) = brush.source_face_index(index) {
                    if brush.faces()[source].material_name == from {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// Swaps one material for another across the map, or across the selection when there
    /// is one.
    ///
    /// `ReplaceMaterialDialog` / `Map::replaceMaterial`: every face wearing `from` is
    /// given `to`, the mapping left where it is. One undo step for the lot. Returns how
    /// many faces changed. When brushes or faces are selected only those are touched, so
    /// the dialog can be pointed at one room; with nothing selected it is the whole map,
    /// which is the common case of retiring a material.
    pub fn replace_material(&mut self, from: &str, to: &str) -> usize {
        if from == to {
            return 0;
        }
        let scoped = !self.selection.is_empty() || !self.face_selection.is_empty();

        let mut before = Vec::new();
        let mut after = Vec::new();
        let mut changed = 0;

        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            let mut faces = brush.faces().to_vec();
            let mut here = 0;
            for index in 0..brush.geometry().face_count() {
                if !self.face_in_replace_scope(
                    scoped,
                    FaceRef {
                        node: id,
                        face: index,
                    },
                ) {
                    continue;
                }
                let Some(source) = brush.source_face_index(index) else {
                    continue;
                };
                if faces[source].material_name == from {
                    faces[source].material_name = to.into();
                    here += 1;
                }
            }
            if here == 0 {
                continue;
            }

            let Ok(edited) = Brush::from_faces(&self.world_bounds, faces) else {
                continue;
            };
            changed += here;
            before.push(brush_node(id, brush.clone()));
            after.push(brush_node(id, edited));
        }

        if !before.is_empty() {
            self.apply_and_record(Edit::Changed { before, after });
        }
        changed
    }

    /// Fits one copy of the material onto every face of every selected brush.
    ///
    /// Each face is fitted to the polygon it actually covers, which the geometry knows,
    /// so this is written as its own loop rather than going through
    /// `Document::edit_selected_faces`.
    pub fn fit_materials_to_selection(&mut self, material_size: geometry_math::vec::Vec2f) {
        let mut before = Vec::new();
        let mut after = Vec::new();

        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            if !self.selection.contains(&id) {
                continue;
            }

            let faces = brush
                .geometry_faces()
                .map(|(face, polygon)| {
                    let mut face = face.clone();
                    face.fit_uv_to_face(&polygon, material_size);
                    face
                })
                .collect();
            let Ok(fitted) = Brush::from_faces(&self.world_bounds, faces) else {
                continue;
            };
            before.push(brush_node(id, brush.clone()));
            after.push(brush_node(id, fitted));
        }

        if !before.is_empty() {
            self.apply_and_record(Edit::Changed { before, after });
        }
    }
}

const MATERIAL_PATHS: &str = "// heron material-sources ";

fn absolute_material_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    let mut clean = PathBuf::new();
    for part in absolute.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                clean.pop();
            }
            part => clean.push(part.as_os_str()),
        }
    }
    clean
}

/// `path` as seen from the folder holding `map`, with `/` separators, or the whole
/// path when the two share no root at all.
pub(crate) fn relative_material_path(path: &Path, map: &Path) -> String {
    // Both sides resolved through symlinks, as material sources are when they load: on
    // macOS `/var` and `/tmp` are links into `/private`, and comparing a resolved source
    // with an unresolved map folder left no shared prefix and an absolute path.
    let resolve = |p: &Path| {
        p.canonicalize()
            .unwrap_or_else(|_| absolute_material_path(p))
    };
    let path = resolve(path);
    let base = resolve(map.parent().unwrap_or(Path::new(".")));
    let a: Vec<_> = path.components().collect();
    let b: Vec<_> = base.components().collect();
    let shared = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if shared == 0 {
        return path.to_string_lossy().replace('\\', "/");
    }
    let mut relative = PathBuf::new();
    for _ in shared..b.len() {
        relative.push("..");
    }
    for part in &a[shared..] {
        relative.push(part.as_os_str());
    }
    if relative.as_os_str().is_empty() {
        relative.push(".");
    }
    relative.to_string_lossy().replace('\\', "/")
}

impl Document {
    /// Persist editor resource references as a comment, without changing engine entities.
    pub(super) fn saved_material_paths(&self, map: &Path) -> String {
        let paths: Vec<_> = self
            .material_sources
            .iter()
            .filter(|source| !self.material_defaults.contains(source.path()))
            .map(|source| {
                (
                    match source {
                        MaterialSource::Folder(_) => "folder",
                        MaterialSource::Package(_) => "wad",
                    },
                    relative_material_path(source.path(), map),
                )
            })
            .collect();
        format!(
            "{MATERIAL_PATHS}{}\n",
            serde_json::to_string(&paths).expect("string resource paths serialize")
        )
    }

    pub fn project_material_folder(&self) -> Option<PathBuf> {
        self.material_sources
            .iter()
            .rev()
            .find_map(|source| match source {
                MaterialSource::Folder(path) if !self.material_defaults.contains(path) => {
                    Some(path.clone())
                }
                _ => None,
            })
    }

    pub(super) fn restore_material_paths(&mut self, source: &str, map: &Path) -> bool {
        let Some(line) = source
            .lines()
            .find_map(|line| line.strip_prefix(MATERIAL_PATHS))
        else {
            return false;
        };
        let paths = match serde_json::from_str::<Vec<(String, String)>>(line) {
            Ok(paths) => paths,
            Err(error) => {
                crate::complain!("material paths: {error}");
                return false;
            }
        };
        for (kind, name) in paths {
            let path = absolute_material_path(&map.parent().unwrap_or(Path::new(".")).join(name));
            let result = match kind.as_str() {
                "folder" => self
                    .load_materials(&path)
                    .map_err(|error| error.to_string()),
                "wad" => self.load_wad(&path),
                _ => {
                    crate::complain!("unknown material source: {kind}");
                    continue;
                }
            };
            match result {
                Ok(warnings) => {
                    for warning in warnings {
                        crate::complain!("materials: {warning}");
                    }
                }
                Err(error) => {
                    crate::complain!("{}: {error}", path.display());
                    // Keep missing references so a save does not erase the user's settings.
                    self.material_sources.push(if kind == "folder" {
                        MaterialSource::Folder(path)
                    } else {
                        MaterialSource::Package(path)
                    });
                }
            }
        }
        true
    }
}

/// A hash of the names, sizes and times of every file under `folder`: if it is what it
/// was, the folder holds what it held.
fn folder_stamp(folder: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, u64, Option<std::time::SystemTime>)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.metadata() {
                Ok(meta) if meta.is_dir() => walk(&path, out),
                Ok(meta) => out.push((path, meta.len(), meta.modified().ok())),
                Err(_) => {}
            }
        }
    }
    let mut files = Vec::new();
    walk(folder, &mut files);
    files.sort();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    files.hash(&mut h);
    h.finish()
}

/// Takes the materials named `names` out of `previous`, provided every one of them is
/// there and came from `folder` (a later folder may have replaced one); otherwise takes
/// nothing and returns `None`, and the folder is read again.
fn take_named(
    previous: &mut HashMap<String, map_model::material::Material>,
    names: &[String],
    folder: &Path,
) -> Option<Vec<map_model::material::Material>> {
    let all_there = names.iter().all(|name| {
        previous
            .get(name)
            .is_some_and(|material| material.path.starts_with(folder))
    });
    if !all_there {
        return None;
    }
    Some(
        names
            .iter()
            .filter_map(|name| previous.remove(name))
            .collect(),
    )
}
