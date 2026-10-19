//! File-backed static model entities, independent of game entity definitions.
use super::*;
const PROPERTY: &str = "_heron_model";
fn key(entity: &EntityNode) -> Option<String> {
    Some(format!(
        "#import/{}/{}/{}",
        entity.property(PROPERTY)?,
        entity.property("angles").unwrap_or("0 0 0"),
        entity.property("modelscale").unwrap_or("1")
    ))
}
impl Document {
    /// Inserts a validated model reference as one undoable point entity.
    pub fn import_model(&mut self, path: &Path, origin: Vec3d) -> Result<NodeId, String> {
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        if !matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("obj" | "glb")
        ) {
            return Err("Choose an OBJ or GLB model".into());
        }
        if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 256 * 1024 * 1024 {
            return Err("Model exceeds 256 MiB".into());
        }
        let named = path.to_string_lossy().into_owned();
        let model = self
            .model_store
            .model(&named)
            .ok_or("The model could not be loaded; see the log for details")?;
        if model
            .triangles
            .iter()
            .flat_map(|t| t.positions)
            .any(|p| ![p.x(), p.y(), p.z()].iter().all(|v| v.is_finite()))
        {
            return Err("Model contains non-finite positions".into());
        }
        if ![origin.x(), origin.y(), origin.z()]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err("Model placement must be finite".into());
        }
        let reference = self
            .path
            .as_ref()
            .and_then(|map| map.parent())
            .and_then(|folder| path.strip_prefix(folder).ok())
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let mut entity = EntityNode {
            properties: vec![
                ("classname".into(), "heron_model".into()),
                (PROPERTY.into(), reference),
                ("angles".into(), "0 0 0".into()),
                (
                    "modelscale".into(),
                    if path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| e.eq_ignore_ascii_case("glb"))
                    {
                        "32"
                    } else {
                        "1"
                    }
                    .into(),
                ),
            ],
            children: vec![],
        };
        entity.set_origin(origin);
        let node = Node::new(NodeKind::Entity(entity));
        let id = node.id;
        self.selection = std::iter::once(id).collect();
        self.apply_and_record(Edit::Added {
            layer: 0,
            nodes: vec![node],
        });
        self.load_imported_models();
        Ok(id)
    }
    /// Copies a model and its referenced OBJ material/image files beside the saved map.
    /// Undo removes the entity, retaining the reusable asset files.
    pub fn import_model_copy(&mut self, source: &Path, origin: Vec3d) -> Result<NodeId, String> {
        let Some(folder) = self
            .path
            .as_ref()
            .and_then(|map| map.parent())
            .map(Path::to_owned)
        else {
            return self.import_model(source, origin);
        };
        let copied = copy_model_bundle(source, &folder)?;
        self.import_model(&copied, origin)
    }

    pub(super) fn imported_references_for(
        &self,
        path: &Path,
    ) -> std::io::Result<std::collections::BTreeMap<String, String>> {
        if !node::point_entities_by_layer(&self.tree.world)
            .iter()
            .any(|(_, e, _)| e.property(PROPERTY).is_some())
        {
            return Ok(Default::default());
        }
        let folder = path.parent().unwrap_or(Path::new("."));
        let folder = if folder.is_absolute() {
            folder.to_owned()
        } else {
            std::env::current_dir()?.join(folder)
        };
        let same_folder = self
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .and_then(|p| p.canonicalize().ok())
            == folder.canonicalize().ok();
        let mut refs = std::collections::BTreeMap::new();
        for (_, entity, _) in node::point_entities_by_layer(&self.tree.world) {
            let Some(reference) = entity.property(PROPERTY) else {
                continue;
            };
            if same_folder && !Path::new(reference).is_absolute() {
                continue;
            }
            let source = self
                .imported_model_path(entity)
                .ok_or_else(|| std::io::Error::other("Imported model path is unresolved"))?;
            let copied = copy_model_bundle(&source, &folder).map_err(std::io::Error::other)?;
            refs.insert(
                reference.to_owned(),
                copied
                    .strip_prefix(&folder)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
        Ok(refs)
    }
    pub(super) fn adopt_imported_references(
        &mut self,
        refs: &std::collections::BTreeMap<String, String>,
    ) {
        fn visit(nodes: &mut [Node], refs: &std::collections::BTreeMap<String, String>) {
            for node in nodes {
                match &mut node.kind {
                    NodeKind::Entity(entity) => {
                        if let Some(value) = entity
                            .property(PROPERTY)
                            .and_then(|value| refs.get(value))
                            .cloned()
                        {
                            entity.set_property(PROPERTY, value);
                        }
                        visit(&mut entity.children, refs)
                    }
                    NodeKind::Group(group) => visit(&mut group.children, refs),
                    _ => {}
                }
            }
        }
        for layer in &mut self.tree.world.layers {
            visit(&mut layer.children, refs)
        }
        if !refs.is_empty() {
            self.mark_changed();
        }
    }

    /// Resolves a model reference relative to its source map.
    pub fn imported_model_path(&self, entity: &EntityNode) -> Option<std::path::PathBuf> {
        let path = Path::new(entity.property(PROPERTY)?);
        Some(if crate::files::is_absolute(path) {
            path.to_owned()
        } else {
            self.path.as_ref()?.parent()?.join(path)
        })
    }
    /// Source geometry before entity rotation and scale, for shared export assets.
    pub fn imported_model_source(
        &self,
        entity: &EntityNode,
    ) -> Option<std::rc::Rc<map_model::entity_model::EntityModel>> {
        let named = self
            .imported_model_path(entity)?
            .to_string_lossy()
            .into_owned();
        self.model_store.cached_model(&named)
    }

    /// Loaded model in entity-local map axes, with the entity's scale and rotation baked.
    pub fn imported_model(
        &self,
        entity: &EntityNode,
    ) -> Option<std::rc::Rc<map_model::entity_model::EntityModel>> {
        self.models.get(&key(entity)?).cloned()
    }
    pub(super) fn load_imported_models(&mut self) {
        let wanted: Vec<_> = node::point_entities_by_layer(&self.tree.world)
            .into_iter()
            .filter_map(|(_, entity, _)| {
                Some((
                    key(entity)?,
                    self.imported_model_path(entity)?,
                    entity.properties.clone(),
                ))
            })
            .collect();
        let used: HashSet<_> = wanted.iter().map(|(key, _, _)| key.clone()).collect();
        self.models
            .retain(|key, _| !key.starts_with("#import/") || used.contains(key));
        for (key, path, properties) in wanted {
            let named = path.to_string_lossy().into_owned();
            if let Some(model) = self.models.get(&key).cloned() {
                self.register_model_skins(&named, &model);
                continue;
            }
            let Some(raw) = self.model_store.model(&named) else {
                continue;
            };
            let property = |name: &str| {
                properties
                    .iter()
                    .find(|(k, _)| k == name)
                    .map(|(_, v)| v.as_str())
            };
            let values: Vec<f64> = property("modelscale")
                .unwrap_or("1")
                .split_whitespace()
                .map(|s| s.parse())
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_default();
            let scale = match values.as_slice() {
                [s] => Vec3d::new(*s, *s, *s),
                [x, y, z] => Vec3d::new(*x, *y, *z),
                _ => {
                    crate::complain!("Invalid imported model scale");
                    continue;
                }
            };
            if [scale.x(), scale.y(), scale.z()]
                .iter()
                .any(|v| !v.is_finite() || v.abs() < 1e-9)
            {
                crate::complain!("Imported model scale must be finite and nonzero");
                continue;
            }
            let info = map_model::entity_rotation::RotationInfo {
                kind: map_model::entity_rotation::RotationType::Euler,
                property_key: "angles".into(),
                usage: map_model::entity_rotation::RotationUsage::Allowed,
            };
            let rotation = map_model::entity_rotation::rotation(&properties, &info);
            let mut model = (*raw).clone();
            for triangle in &mut model.triangles {
                triangle.normals = triangle.normals.map(|normals| {
                    normals.map(|n| {
                        geometry_math::vec::normalize(
                            rotation
                                * Vec3d::new(
                                    n.x() / scale.x(),
                                    n.y() / scale.y(),
                                    n.z() / scale.z(),
                                ),
                        )
                    })
                });
                for p in &mut triangle.positions {
                    *p = rotation
                        * Vec3d::new(p.x() * scale.x(), p.y() * scale.y(), p.z() * scale.z())
                }
            }
            if model
                .triangles
                .iter()
                .flat_map(|t| t.positions)
                .any(|p| ![p.x(), p.y(), p.z()].iter().all(|v| v.is_finite()))
            {
                crate::complain!("Imported model transform produces non-finite positions");
                continue;
            }
            if scale.x() * scale.y() * scale.z() < 0.0 {
                for t in &mut model.triangles {
                    t.positions.swap(1, 2);
                    t.uv.swap(1, 2);
                    if let Some(normals) = &mut t.normals {
                        normals.swap(1, 2);
                    }
                }
            }
            if let Some(first) = model.triangles.first() {
                model.bounds = Bbox3d::new(first.positions[0], first.positions[0]);
                for t in &model.triangles {
                    for p in t.positions {
                        model.bounds = model.bounds.merged_with_point(p)
                    }
                }
            }
            self.register_model_skins(&named, &model);
            self.models.insert(key, std::rc::Rc::new(model));
        }
    }
}

fn copy_model_bundle(source: &Path, folder: &Path) -> Result<std::path::PathBuf, String> {
    let source = source.canonicalize().map_err(|e| e.to_string())?;
    let parent = source.parent().ok_or("Model has no parent folder")?;
    if std::fs::metadata(&source).map_err(|e| e.to_string())?.len() > 256 * 1024 * 1024 {
        return Err("Model exceeds 256 MiB".into());
    }
    let bytes = std::fs::read(&source).map_err(|e| e.to_string())?;
    let file = source.file_name().ok_or("Model has no file name")?;
    let mut files =
        std::collections::BTreeMap::from([(std::path::PathBuf::from(file), bytes.clone())]);
    let mut missing = None;
    if source
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("obj"))
    {
        let mut sibling = |name: &str| {
            let path = parent.join(name).canonicalize().ok();
            let Some(path) = path.filter(|p| p.starts_with(parent)) else {
                missing = Some(format!(
                    "Model dependency is missing or outside its folder: {name}"
                ));
                return None;
            };
            match std::fs::read(&path) {
                Ok(data) => {
                    files.insert(path.strip_prefix(parent).unwrap().to_owned(), data.clone());
                    Some(data)
                }
                Err(e) => {
                    missing = Some(e.to_string());
                    None
                }
            }
        };
        map_model::obj_model::read(&bytes, &mut sibling).map_err(|e| e.to_string())?;
        if let Some(error) = missing {
            return Err(error);
        }
    } else {
        map_model::glb_model::read(&bytes)?;
    }
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (name, bytes) in &files {
        name.hash(&mut hasher);
        bytes.hash(&mut hasher)
    }
    let destination = folder
        .join(".heron-brush/models")
        .join(format!("{:016x}", hasher.finish()));
    if destination.exists() {
        for (name, bytes) in &files {
            if std::fs::read(destination.join(name)).ok().as_ref() != Some(bytes) {
                return Err(
                    "Imported asset folder contains different files; it was retained".into(),
                );
            }
        }
    } else {
        let copied = (|| -> Result<(), String> {
            for (name, bytes) in &files {
                let path = destination.join(name);
                std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
                crate::godot_live::atomic_write(&path, bytes).map_err(|e| e.to_string())?;
            }
            Ok(())
        })();
        if let Err(error) = copied {
            let _ = std::fs::remove_dir_all(&destination);
            return Err(error);
        }
    }
    Ok(destination.join(file))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pbr_maps_survive_preview_and_all_glb_export_paths() {
        let root=std::env::var_os("HERON_PBR_OUTPUT").map(PathBuf::from).unwrap_or_else(||std::env::temp_dir().join(format!("heron-pbr-{}",std::process::id())));
        std::fs::create_dir_all(&root).unwrap();
        let bytes=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../examples/editor-checks/fixtures/pbr.glb"));
        let source=map_model::glb_model::read(bytes).unwrap();
        std::fs::write(root.join("source.glb"),bytes).unwrap();
        let mut doc=Document::new(WorldBounds::default());
        doc.import_model(&root.join("source.glb"),Vec3d::zero()).unwrap();
        let triangles=doc.triangles(&Default::default());
        assert_eq!(triangles.len(),2);
        assert_ne!(triangles[0].material,triangles[1].material);
        for t in triangles {let m=&doc.materials().materials()[t.material.unwrap()];assert!(m.model_pbr.is_some());assert!(m.normal_map.is_some());}
        let snapshot = serde_json::to_string(&doc.export_snapshot(&root.join("unsaved.map"))).unwrap();
        let isolated = Document::from_export_snapshot(serde_json::from_str(&snapshot).unwrap()).unwrap();
        for target in crate::export::Target::ALL {
            let options=crate::export::Options {target,name:"pbr".into(),..Default::default()};
            let directory=root.join(format!("{target:?}"));
            let (report,files)=crate::export::export_files(&doc,&directory,&options).unwrap_or_else(|e|panic!("{target:?}: {e}"));
            assert_eq!(report.triangles,2,"{target:?}");
            let (_, replica) = crate::export::export_files(&isolated,&directory,&options).unwrap();
            let bytes = |content:&crate::export::Contents| match content {crate::export::Contents::Text(s)=>s.as_bytes().to_vec(),crate::export::Contents::Bytes(b)=>b.clone()};
            assert_eq!(files.len(),replica.len(),"{target:?}: snapshot file count");
            for ((path,data),(replica_path,replica_data)) in files.iter().zip(&replica) {
                assert_eq!(path,replica_path,"{target:?}: snapshot asset path");
                assert_eq!(bytes(data),bytes(replica_data),"{target:?}: snapshot asset bytes at {}",path.display());
            }
            for (path,data) in files {
                let data=match data {crate::export::Contents::Text(s)=>s.into_bytes(),crate::export::Contents::Bytes(b)=>b};
                if path.extension().is_some_and(|e|e=="glb") {
                    let loaded=map_model::glb_model::read(&data).unwrap();
                    assert_eq!(loaded.pbr,source.pbr,"{target:?}: maps or factors changed");
                }
                if std::env::var_os("HERON_PBR_OUTPUT").is_some() {
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(path,data).unwrap();
                }
            }
        }
        if std::env::var_os("HERON_PBR_OUTPUT").is_some() {
            std::fs::write(root.join("Godot/source.glb"),bytes).unwrap();
            std::fs::write(root.join("Godot/project.godot"),"config_version=5\n[application]\nconfig/name=\"Heron PBR check\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n").unwrap();
        } else {std::fs::remove_dir_all(root).unwrap();}
    }

    #[test]
    fn material_modes_survive_all_generated_glb_paths() {
        let output = std::env::var_os("HERON_MATERIAL_MODES_OUTPUT");
        let root = output.as_ref().map(PathBuf::from).unwrap_or_else(||
            std::env::temp_dir().join(format!("heron-material-modes-{}", std::process::id())));
        std::fs::create_dir_all(&root).unwrap();
        let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../examples/editor-checks/fixtures/material_modes.glb"));
        let source = map_model::glb_model::read(bytes).unwrap();
        std::fs::write(root.join("source.glb"), bytes).unwrap();
        let mut doc = Document::new(WorldBounds::default());
        doc.import_model(&root.join("source.glb"), Vec3d::zero()).unwrap();
        let mut checked = 0;
        for target in crate::export::Target::ALL {
            let options = crate::export::Options {target, name:"modes".into(), ..Default::default()};
            let (report, files) = crate::export::export_files(&doc, &root.join(format!("{target:?}")), &options).unwrap();
            assert_eq!(report.triangles, 6, "{target:?}");
            if target == crate::export::Target::Unity {
                assert!(files.iter().any(|(path, _)| path.file_name().is_some_and(|name| name == "HeronBrushPbr.shader")),
                    "Two-sided Unity materials require their exported shader");
            }
            for (path, data) in files {
                let data = match data { crate::export::Contents::Bytes(bytes) => bytes,
                    crate::export::Contents::Text(text) => text.into_bytes() };
                if output.is_some() {
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(&path, &data).unwrap();
                }
                if !path.extension().is_some_and(|extension| extension == "glb") { continue; }
                let loaded = map_model::glb_model::read(&data).unwrap();
                assert_eq!(loaded.pbr, source.pbr, "{target:?}: source material rules or PBR changed");
                assert_eq!(loaded.skins().collect::<Vec<_>>(), source.skins().collect::<Vec<_>>(),
                    "{target:?}: alpha gradient changed");
                checked += 1;
            }
        }
        assert!(checked > 0, "No GLB assets were checked");
        if output.is_some() {
            std::fs::write(root.join("Godot/source.glb"), bytes).unwrap();
            std::fs::write(root.join("Godot/project.godot"),
                "config_version=5\n[application]\nconfig/name=\"Heron material modes\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n").unwrap();
        } else { std::fs::remove_dir_all(root).unwrap(); }
    }

    #[test]
    fn repeated_models_share_assets_without_changing_world_vertices() {
        let root = std::env::var_os("HERON_INSTANCE_OUTPUT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::temp_dir().join(format!("heron-instance-{}", std::process::id()))
            });
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("model.obj"),
            "v 0 0 0\nv 32 0 0\nv 0 32 16\nvn 1 1 1\nf 1//1 2//1 3//1\n",
        )
        .unwrap();
        let mut doc = Document::new(WorldBounds::default());
        let mut expected = Vec::new();
        let mut expected_normals = Vec::new();
        for (origin, angles, scale) in [
            (Vec3d::new(0., 0., 0.), "0 0 0", "1"),
            (Vec3d::new(256., 64., 32.), "20 35 10", "2 3 4"),
            (Vec3d::new(-128., 256., 64.), "-35 120 45", "0.5 2 1"),
        ] {
            let id = doc.import_model(&root.join("model.obj"), origin).unwrap();
            doc.set_entity_property(id, "angles", angles);
            doc.set_entity_property(id, "modelscale", scale);
            let model = doc
                .imported_model(doc.selected_entity().unwrap().1)
                .unwrap();
            expected_normals.extend(model.triangles.iter().flat_map(|t|t.normals.unwrap()));
            expected.extend(
                model
                    .triangles
                    .iter()
                    .flat_map(|t| t.positions.map(|p| p + origin)),
            );
        }
        let mut map = scene_export::map::parse(&doc.to_map_text(), "instances.map").unwrap();
        let options = crate::export::Options::default();
        let mut settings = options.settings(&root);
        crate::imported_export::prepare(&doc, &mut map, &mut settings).unwrap();
        let built = scene_export::builder::build_level(
            &map,
            &Default::default(),
            &settings,
            &Default::default(),
        );
        assert_eq!(built.meshes.len(), 1);
        let mut actual = Vec::new();
        for entity in built.level.entities.iter().filter(|e| e.mesh.is_some()) {
            let mesh = map_model::glb_model::read(&built.meshes[0].bytes).unwrap();
            let t = &entity.transform;
            let basis = scene_export::scene::scale_basis(
                scene_export::scene::basis_from_euler_yxz(scene_export::math::Vec3::from_array(
                    t.rotation_yxz,
                )),
                scene_export::math::Vec3::from_array(t.scale),
            );
            for triangle in mesh.triangles {
                for p in triangle.positions {
                    // GLB reader converts Y-up to map axes, undo that before applying the exported transform.
                    let p = [p.x(), p.z(), -p.y()];
                    let q: [f64; 3] = std::array::from_fn(|row| {
                        (0..3).map(|col| basis[row * 3 + col] * p[col]).sum::<f64>()
                            + t.translation[row]
                    });
                    actual.push(Vec3d::new(q[2], q[0], q[1]) / settings.scale_factor);
                }
            }
        }
        assert_eq!(actual.len(), expected.len());
        for p in &expected {
            assert!(
                actual
                    .iter()
                    .any(|q| geometry_math::vec::length(*q - *p) < 1e-4),
                "missing world vertex {p:?}; got {actual:?}"
            );
        }
        let mut many = scene_export::map::parse(&doc.to_map_text(), "many.map").unwrap();
        let template = map
            .entities
            .iter()
            .find(|e| e.external_asset.is_some())
            .unwrap()
            .clone();
        many.entities.retain(|e| e.classname() == "worldspawn");
        for i in 0..20 {
            let mut entity = template.clone();
            let offset = scene_export::math::Vec3::new(i as f64 * 128., i as f64 * 64., 0.);
            entity.properties.insert(
                "origin".into(),
                format!("{} {} {}", offset.x, offset.y, offset.z),
            );
            for surface in &mut entity.external_surfaces {
                for v in &mut surface.vertices {
                    v.position += offset;
                }
            }
            many.entities.push(entity);
        }
        let shared = scene_export::builder::build_level(
            &many,
            &Default::default(),
            &settings,
            &Default::default(),
        );
        assert_eq!(shared.meshes.len(), 1);
        assert_eq!(shared.stats.triangles, 20);
        for entity in &mut many.entities {
            entity.external_asset = None;
        }
        let baked = scene_export::builder::build_level(
            &many,
            &Default::default(),
            &settings,
            &Default::default(),
        );
        let shared_bytes = shared.meshes.iter().map(|m| m.bytes.len()).sum::<usize>();
        let baked_bytes = baked.meshes.iter().map(|m| m.bytes.len()).sum::<usize>();
        assert!(shared_bytes * 10 < baked_bytes);
        println!(
            "20 instances: {} -> {} mesh files, {baked_bytes} -> {shared_bytes} mesh bytes",
            baked.meshes.len(),
            shared.meshes.len()
        );
        for target in crate::export::Target::ALL {
            let options = crate::export::Options {
                target,
                name: "instances".into(),
                ..Default::default()
            };
            let directory = root.join(format!("{target:?}"));
            let (report, files) = crate::export::export_files(&doc, &directory, &options)
                .unwrap_or_else(|e| panic!("{target:?}: {e}"));
            assert_eq!(report.triangles, 3, "{target:?}");
            if target == crate::export::Target::Godot {
                assert!(
                    files
                        .iter()
                        .any(|(p, _)| p.extension().is_some_and(|e| e == "png")),
                    "Godot must receive imported skin PNGs"
                );
            }
            let meshes = files
                .iter()
                .filter(|(p, _)| p.extension().is_some_and(|e| e == "obj" || e == "glb"))
                .count();
            if !matches!(target, crate::export::Target::Gltf) {
                assert_eq!(meshes, 1, "{target:?}");
            }
            for (path, contents) in files {
                let bytes = match contents {
                    crate::export::Contents::Text(s) => s.into_bytes(),
                    crate::export::Contents::Bytes(b) => b,
                };
                if matches!(
                    target,
                    crate::export::Target::Glb | crate::export::Target::Gltf
                ) && path.extension().is_some_and(|e| e == "glb" || e == "gltf")
                {
                    let json: serde_json::Value = if target == crate::export::Target::Glb {
                        let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
                        serde_json::from_slice(&bytes[20..20 + len]).unwrap()
                    } else {
                        serde_json::from_slice(&bytes).unwrap()
                    };
                    assert_eq!(json["meshes"].as_array().unwrap().len(), 1);
                    assert_eq!(
                        json["nodes"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|n| !n["mesh"].is_null())
                            .count(),
                        3
                    );
                    if target == crate::export::Target::Glb {
                        let loaded = map_model::glb_model::read(&bytes).unwrap();
                        for p in &expected {
                            assert!(
                                loaded
                                    .triangles
                                    .iter()
                                    .flat_map(|t| t.positions)
                                    .any(|q| geometry_math::vec::length(q - *p / options.scale)
                                        < 1e-5)
                            );
                        }
                    }
                }
                if std::env::var_os("HERON_INSTANCE_OUTPUT").is_some()
                    && target == crate::export::Target::Godot
                {
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(path, bytes).unwrap();
                }
            }
        }
        if std::env::var_os("HERON_INSTANCE_OUTPUT").is_some() {
            let expected = expected
                .into_iter()
                .map(|p| {
                    [
                        p.y() * settings.scale_factor,
                        p.z() * settings.scale_factor,
                        p.x() * settings.scale_factor,
                    ]
                })
                .collect::<Vec<_>>();
            std::fs::write(
                root.join("Godot/expected.json"),
                serde_json::to_vec(&serde_json::json!({"vertices":expected,"normals":expected_normals.iter().map(|n|[n.y(),n.z(),n.x()]).collect::<Vec<_>>()})).unwrap(),
            )
            .unwrap();
            std::fs::write(root.join("Godot/project.godot"),"config_version=5\n[application]\nconfig/name=\"Heron instances check\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n").unwrap();
        } else {
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn multiple_materials_and_smooth_normals_survive_transforms_and_export_roundtrips() {
        let root = std::env::temp_dir().join(format!("heron-multi-import-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("mesh.obj"), "mtllib mesh.mtl\nv 0 0 0\nv 32 0 0\nv 0 32 0\nv 32 32 0\nvn 0 0 1\nvn 1 1 1\nusemtl red\nf 1//1 2//2 3//1\nusemtl green\nf 2//2 4//1 3//1\n").unwrap();
        std::fs::write(
            root.join("mesh.mtl"),
            "newmtl red\nKd 1 0 0\nnewmtl green\nKd 0 1 0\n",
        )
        .unwrap();
        let mut doc = Document::new(WorldBounds::default());
        let id = doc
            .import_model(&root.join("mesh.obj"), Vec3d::zero())
            .unwrap();
        doc.set_entity_property(id, "modelscale", "-2 3 4");
        let model = doc
            .imported_model(doc.selected_entity().unwrap().1)
            .unwrap();
        let expected = geometry_math::vec::normalize(Vec3d::new(-0.5, 1.0 / 3.0, 0.25));
        assert!(
            geometry_math::vec::length(model.triangles[0].normals.unwrap()[2] - expected) < 1e-9
        );
        let rendered = doc.triangles(&Default::default());
        assert_eq!(rendered.len(), 2);
        assert_ne!(rendered[0].material, rendered[1].material);
        for (actual, expected) in rendered[0]
            .corner_normals
            .unwrap()
            .into_iter()
            .zip(model.triangles[0].normals.unwrap())
        {
            assert!(geometry_math::vec::length(actual - expected) < 1e-9);
        }
        let mut map = scene_export::map::parse(&doc.to_map_text(), "test.map").unwrap();
        let mut settings = scene_export::settings::Settings::default();
        crate::imported_export::prepare(&doc, &mut map, &mut settings).unwrap();
        let surfaces = &map
            .entities
            .iter()
            .find(|e| e.get("_heron_model").is_some())
            .unwrap()
            .external_surfaces;
        assert_eq!(surfaces.len(), 2);
        assert_eq!(settings.generated_materials.len(), 2);
        assert!((surfaces[0].vertices[2].normal.x - expected.x()).abs() < 1e-9);
        for target in crate::export::Target::ALL {
            let options = crate::export::Options {
                target,
                name: "multi".into(),
                ..Default::default()
            };
            let (report, files) = crate::export::export_files(&doc, &root.join("export"), &options)
                .unwrap_or_else(|e| panic!("{target:?}: {e}"));
            assert_eq!(report.triangles, 2, "{target:?}");
            let files = files
                .into_iter()
                .map(|(p, b)| {
                    (
                        p,
                        match b {
                            crate::export::Contents::Text(s) => s.into_bytes(),
                            crate::export::Contents::Bytes(b) => b,
                        },
                    )
                })
                .collect::<Vec<_>>();
            if let Some((_, bytes)) = files
                .iter()
                .find(|(p, _)| p.extension().is_some_and(|e| e == "glb"))
            {
                let roundtrip = map_model::glb_model::read(bytes).unwrap();
                assert_eq!(roundtrip.extra_skins.len(), 1, "{target:?}");
                assert!(roundtrip.triangles.iter().all(|t| t.normals.is_some()));
                // Engine meshes use the common export axes; the portable writer
                // uses the editor's own glTF axis conversion.
                let expected = if target == crate::export::Target::Glb {
                    expected
                } else {
                    Vec3d::new(expected.y(), -expected.x(), expected.z())
                };
                assert!(
                    roundtrip
                        .triangles
                        .iter()
                        .flat_map(|t| t.normals.unwrap())
                        .any(|n| geometry_math::vec::length(n - expected) < 1e-5),
                    "{target:?}: exported normals {:?}",
                    roundtrip
                        .triangles
                        .iter()
                        .map(|t| t.normals)
                        .collect::<Vec<_>>()
                );
            }
            if let Some((_, bytes)) = files
                .iter()
                .find(|(p, _)| p.extension().is_some_and(|e| e == "obj"))
            {
                let mut sibling = |name: &str| {
                    files
                        .iter()
                        .find(|(p, _)| p.ends_with(name))
                        .map(|(_, b)| b.clone())
                };
                let roundtrip = map_model::obj_model::read(bytes, &mut sibling).unwrap();
                assert_eq!(roundtrip.extra_skins.len(), 1, "{target:?}");
                assert!(roundtrip.triangles.iter().all(|t| t.normals.is_some()));
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn model_copy_roundtrip_transform_and_undo_preserve_sources_and_all_exports() {
        let root = std::env::temp_dir().join(format!("heron-import-{}", std::process::id()));
        let source = root.join("source");
        let project = root.join("project");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        let obj = b"v 0 0 0\nv 32 0 0\nv 0 32 0\nf 1 2 3\n";
        std::fs::write(source.join("triangle.obj"), obj).unwrap();
        let mut doc = Document::new(WorldBounds::default());
        doc.path = Some(project.join("level.map"));
        let before = doc.to_map_text();
        let id = doc
            .import_model_copy(&source.join("triangle.obj"), Vec3d::new(64., 0., 0.))
            .unwrap();
        let inserted = doc.to_map_text();
        assert!(inserted.contains(".heron-brush/models/"));
        assert!(!inserted.contains(source.to_str().unwrap()));
        assert_eq!(doc.triangles(&Default::default()).len(), 1);
        assert!(doc.undo().is_some());
        assert_eq!(doc.to_map_text(), before);
        assert!(doc.redo().is_some());
        assert_eq!(doc.to_map_text(), inserted);
        doc.select_only(id);
        doc.set_entity_property(id, "angles", "0 90 0");
        doc.set_entity_property(id, "modelscale", "2");
        let entity = doc.selected_entity().unwrap().1;
        let model = doc.imported_model(entity).unwrap();
        assert!((model.bounds.max.y() - 64.).abs() < 1e-6);
        std::fs::write(project.join("level.map"), doc.to_map_text()).unwrap();
        let loaded = Document::open(project.join("level.map"), WorldBounds::default()).unwrap();
        assert_eq!(loaded.triangles(&Default::default()).len(), 1);
        for target in crate::export::Target::ALL {
            let options = crate::export::Options {
                target,
                name: "imported".into(),
                ..Default::default()
            };
            let (report, files) =
                crate::export::export_files(&loaded, &project.join("export"), &options)
                    .unwrap_or_else(|e| panic!("{target:?}: {e}"));
            assert_eq!(report.triangles, 1, "{target:?}");
            assert!(!files.is_empty());
        }
        let moved = root.join("moved");
        std::fs::create_dir_all(&moved).unwrap();
        doc.save_as(moved.join("level.map")).unwrap();
        std::fs::remove_dir_all(&project).unwrap();
        let moved_doc = Document::open(moved.join("level.map"), WorldBounds::default()).unwrap();
        assert_eq!(moved_doc.triangles(&Default::default()).len(), 1);
        assert_eq!(std::fs::read(source.join("triangle.obj")).unwrap(), obj);
        std::fs::remove_dir_all(root).unwrap();
    }
}
