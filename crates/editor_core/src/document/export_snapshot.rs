//! Transfer the authored state to an isolated export process without saving the map.
use super::*;
use map_model::fgd::{DefinitionKind, EntityDefinition, EntityDefinitions, PropertyDefinition};

/// A versioned authoring snapshot. Files referenced by it travel separately.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportSnapshot {
    version: u32,
    source: String,
    path: Option<PathBuf>,
    resource_base: PathBuf,
    half_extent: f64,
    definitions_path: Option<PathBuf>,
    definitions: Option<Vec<Definition>>,
    definition_warnings: Vec<String>,
    export_classes: (scene_export::indexmap::IndexMap<String, scene_export::fgd::EntityClass>, Vec<String>),
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Definition {
    name: String,
    kind: DefinitionKind,
    description: String,
    base_classes: Vec<String>,
    bounds: Option<[f64; 6]>,
    color: Option<[u8; 3]>,
    model_expression: Option<String>,
    properties: Vec<PropertyDefinition>,
}

impl Document {
    /// Capture geometry, current definitions and export rules without saving the map.
    /// `resource_base` is the fallback map location for an unsaved document.
    pub fn export_snapshot(&self, resource_base: &Path) -> ExportSnapshot {
        let base = self.path.as_deref().unwrap_or(resource_base);
        let source = format!("{}{}", self.saved_material_paths(base), write_map(&node::flatten_tree(
            &self.tree.world, map_model::map_format::MapFormat::LOSSLESS, self.game.clone())));
        ExportSnapshot {
            version: 1, source, path: self.path.clone(), resource_base: base.to_path_buf(),
            half_extent: self.world_bounds().max.x(),
            definitions_path: self.definitions_path.clone(),
            definitions: self.definitions.as_ref().map(|definitions| definitions.all().iter().map(|d| Definition {
                name: d.name.clone(), kind: d.kind, description: d.description.clone(), base_classes: d.base_classes.clone(),
                bounds: d.bounds.map(|b| [b.min.x(), b.min.y(), b.min.z(), b.max.x(), b.max.y(), b.max.z()]),
                color: d.color, model_expression: d.model.as_ref().map(|m| m.expression().as_string()),
                properties: d.properties.clone(),
            }).collect()),
            definition_warnings: self.definitions.as_ref().map(|d| d.warnings.clone()).unwrap_or_default(),
            export_classes: crate::export::entity_classes(self),
        }
    }

    /// Restore a snapshot using the recipient's file store, retaining unsaved state.
    /// Definitions come from the sender, not a possibly changed FGD on disk.
    pub fn from_export_snapshot(snapshot: ExportSnapshot) -> Result<Self, String> {
        if snapshot.version != 1 || !snapshot.half_extent.is_finite() || snapshot.half_extent <= 0.0 {
            return Err("Invalid export snapshot version or world bounds".into());
        }
        let map = read_map(&snapshot.source, None).map_err(|e| e.to_string())?;
        let mut document = Self::from_map_owned(map, WorldBounds::new(snapshot.half_extent), snapshot.path);
        crate::assets::load_library_materials(&mut document);
        if !document.restore_material_paths(&snapshot.source, &snapshot.resource_base) {
            if let Some(path) = crate::document_setup::project_material_directory(&snapshot.resource_base) {
                document.load_materials(path).map_err(|e| e.to_string())?;
            }
        }
        document.definitions = snapshot.definitions.map(|definitions| {
            definitions.into_iter().map(|d| Ok(EntityDefinition {
                name: d.name, kind: d.kind, description: d.description, base_classes: d.base_classes,
                bounds: d.bounds.map(|b| Bbox3d::new(Vec3d::new(b[0],b[1],b[2]), Vec3d::new(b[3],b[4],b[5]))),
                color: d.color,
                model: d.model_expression.map(|s| map_model::model_definition::parse_model_definition(&s)).transpose()?,
                properties: d.properties,
            })).collect::<Result<Vec<_>, String>>().map(|definitions| EntityDefinitions::new(definitions, snapshot.definition_warnings))
        }).transpose()?;
        document.definitions_path = snapshot.definitions_path;
        document.export_classes_override = Some(snapshot.export_classes);
        document.load_entity_models();
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_round_trip_keeps_unsaved_geometry_and_loaded_definitions() {
        let root = std::env::temp_dir().join(format!("heron-export-snapshot-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let fgd = root.join("project.fgd");
        std::fs::write(&fgd, "@SolidClass = custom_block : \"@heron_brush collision=none\" [ charge(integer) : \"Charge\" : 7 ]\n").unwrap();
        let source = include_str!("../../../../demos/motion_lab/motion_lab.map");
        let mut document = Document::from_map(&read_map(source, None).unwrap(), WorldBounds::new(32768.0), None);
        document.load_definitions(&fgd).unwrap();
        let before = crate::export::entity_classes(&document).0.into_iter().collect::<std::collections::BTreeMap<_,_>>();
        let snapshot = serde_json::to_string(&document.export_snapshot(&root.join("unsaved.map"))).unwrap();
        std::fs::write(&fgd, "broken later FGD").unwrap();
        let restored = Document::from_export_snapshot(serde_json::from_str(&snapshot).unwrap()).unwrap();
        assert!(restored.path.is_none());
        assert_eq!(document.to_map_text(), restored.to_map_text());
        assert_eq!(restored.world_bounds().max.x(), 32768.0);
        assert_eq!(format!("{before:?}"), format!("{:?}", crate::export::entity_classes(&restored).0.into_iter().collect::<std::collections::BTreeMap<_,_>>()));
        let options = crate::export::Options {target: crate::export::Target::ThreeJs, name:"snapshot".into(), ..Default::default()};
        let (_, original) = crate::export::export_files(&document, &root, &options).unwrap();
        let (_, copied) = crate::export::export_files(&restored, &root, &options).unwrap();
        let normalize = |files:Vec<(PathBuf, crate::export::Contents)>| files.into_iter().map(|(p,c)| (p, match c {crate::export::Contents::Text(s)=>s.into_bytes(),crate::export::Contents::Bytes(b)=>b})).collect::<Vec<_>>();
        assert_eq!(normalize(original), normalize(copied));
        std::fs::remove_dir_all(root).unwrap();
    }
}
