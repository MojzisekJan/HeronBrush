//! Checking a map for the things that go wrong in it.
//!
//! Document validators report issues and their locations for the issue browser. Corrections use the
//! regular editing commands.
//!
//! The validators fall into three groups. Some catch what the `.map` format cannot
//! express -- a quotation mark in a property, a key too long for the game to read. Some
//! catch what the format allows but the game will not like -- an entity with no class, a
//! link pointing at nothing. And some catch what will merely make the level worse -- a
//! brush whose corners are off the grid, a face whose material was never loaded.

use std::collections::HashSet;

use map_model::node::{Node, NodeId, NodeKind};

use crate::document::Document;

/// How much a problem matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Worth knowing about; the map still works.
    Hint,
    /// The map will load, but not do what was meant.
    Warning,
    /// The map will not load, or the format cannot hold this.
    Error,
}

impl Severity {
    /// A one-word label.
    pub fn name(self) -> &'static str {
        match self {
            Severity::Hint => "hint",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// One thing wrong with the map.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Issue {
    /// How much it matters.
    pub severity: Severity,
    /// What kind of problem it is, which is what a filter would group by.
    pub kind: &'static str,
    /// What is wrong, in a sentence.
    pub description: String,
    /// Which node it is about, so selecting it takes the user there.
    pub node: Option<NodeId>,
}

/// How long a property key or value may be before the format's readers start truncating.
///
/// `LongPropertyKeyValidator` takes this as a parameter from the game config; 1023 is what
/// the Quake-lineage games use, and this editor has no game config to read it from.
const MAX_PROPERTY_LENGTH: usize = 1023;

/// Checks the whole map.
///
/// The issues come back worst first -- errors, then warnings, then hints -- and within a
/// severity in the order the validators ran, so what stops the map loading is at the top
/// of the browser and the order is stable between runs.
pub fn validate(document: &Document) -> Vec<Issue> {
    let mut issues = Vec::new();

    check_entities(document, &mut issues);
    check_entity_bounds(document, &mut issues);
    check_worldspawn_paths(document, &mut issues);
    check_links(document, &mut issues);
    check_groups(document, &mut issues);
    check_brushes(document, &mut issues);

    // A stable sort, so validator order is kept within each severity.
    issues.sort_by_key(|issue| std::cmp::Reverse(issue.severity));
    issues
}

/// `WorldBoundsValidator` for point entities: it checks every node's bounds, not just
/// every brush's, so an entity dropped far outside the world -- a bad paste, a hand edit
/// -- is caught too. A brush entity's position is its brushes', which `check_brushes`
/// already covers.
fn check_entity_bounds(document: &Document, issues: &mut Vec<Issue>) {
    let world_bounds = document.world_bounds();
    for (id, entity) in map_model::node::entities_with_ids(document.tree_world()) {
        let Some(origin) = entity.origin() else { continue };
        if !world_bounds.contains_point(origin) {
            let name = entity.classname().unwrap_or("<no classname>");
            issues.push(Issue {
                severity: Severity::Error,
                kind: "outside the world",
                description: format!("{name} sits outside the world bounds"),
                node: Some(id),
            });
        }
    }
}

/// `WorldNodePathSeparatorValidator`: worldspawn's `wad` and `_tb_def` hold lists of file
/// paths, and a `\` in one is a Windows separator that no other platform reads.
fn check_worldspawn_paths(document: &Document, issues: &mut Vec<Issue>) {
    for (key, value) in &document.tree_world().properties {
        if matches!(
            key.as_str(),
            "wad" | map_model::entity_properties::keys::TB_ENTITY_DEFINITIONS
        ) && value.contains('\\')
        {
            issues.push(Issue {
                severity: Severity::Warning,
                kind: "backslash in a path",
                description: format!(
                    "worldspawn's \"{key}\" has backslashes in its paths, which only Windows reads"
                ),
                node: None,
            });
        }
    }
}

/// The entity validators: classname, properties and what the class says the entity is.
fn check_entities(document: &Document, issues: &mut Vec<Issue>) {
    for (id, entity) in map_model::node::entities_with_ids(document.tree_world()) {
        let name = entity.classname().unwrap_or("<no classname>").to_string();

        // `MissingClassnameValidator`.
        if entity.classname().is_none() {
            issues.push(Issue {
                severity: Severity::Error,
                kind: "missing classname",
                description: "Entity has no classname property".to_string(),
                node: Some(id),
            });
        }

        for (key, value) in &entity.properties {
            // `EmptyPropertyKeyValidator` and `EmptyPropertyValueValidator`.
            if key.is_empty() {
                issues.push(Issue {
                    severity: Severity::Error,
                    kind: "empty property key",
                    description: format!("{name} has a property with an empty name"),
                    node: Some(id),
                });
            }
            if value.is_empty() {
                issues.push(Issue {
                    severity: Severity::Warning,
                    kind: "empty property value",
                    description: format!("{name} has an empty value for \"{key}\""),
                    node: Some(id),
                });
            }

            // `PropertyKeyWithDoubleQuotationMarksValidator` and its value twin: the
            // format quotes both, and the compilers and editors that read the map do not
            // all take an escaped quote. The editor's own JSON (keyframe curves) is
            // written escaped and read back only by Heron Brush and its exports.
            if key.contains('"') || (value.contains('"') && key != "animation_curves") {
                issues.push(Issue {
                    severity: Severity::Error,
                    kind: "quotation mark in property",
                    description: format!(
                        "{name}'s property \"{key}\" contains a quotation mark, which the map format cannot hold"
                    ),
                    node: Some(id),
                });
            }

            // `LongPropertyKeyValidator` and `LongPropertyValueValidator`.
            if key.len() >= MAX_PROPERTY_LENGTH || value.len() >= MAX_PROPERTY_LENGTH {
                issues.push(Issue {
                    severity: Severity::Warning,
                    kind: "long property",
                    description: format!(
                        "{name}'s property \"{key}\" is longer than {MAX_PROPERTY_LENGTH} characters and may be truncated"
                    ),
                    node: Some(id),
                });
            }
        }

        let Some(classname) = entity.classname() else {
            continue;
        };
        if entity.property("_heron_model").is_some() {
            if document.imported_model(entity).is_none(){issues.push(Issue{severity:Severity::Error,kind:"imported model unavailable",description:"The imported model cannot be loaded or its transforms are invalid".into(),node:Some(id)});}
            if classname=="heron_model"{continue}
        }
        let definition = document
            .definitions()
            .and_then(|definitions| definitions.find(classname));

        // `MissingDefinitionValidator`.
        let Some(definition) = definition else {
            if document.definitions().is_some() {
                issues.push(Issue {
                    severity: Severity::Warning,
                    kind: "unknown class",
                    description: format!("No definition for class \"{classname}\""),
                    node: Some(id),
                });
            }
            continue;
        };

        // `EmptyBrushEntityValidator` and `PointEntityWithBrushesValidator`: a class says
        // whether it is made of brushes, and an entity that disagrees will not behave.
        let has_brushes = !entity.children.is_empty();
        match definition.kind {
            map_model::fgd::DefinitionKind::Solid if !has_brushes => issues.push(Issue {
                severity: Severity::Warning,
                kind: "empty brush entity",
                description: format!("{classname} is a brush entity but has no brushes"),
                node: Some(id),
            }),
            map_model::fgd::DefinitionKind::Point if has_brushes => issues.push(Issue {
                severity: Severity::Warning,
                kind: "point entity with brushes",
                description: format!("{classname} is a point entity but has brushes"),
                node: Some(id),
            }),
            _ => {}
        }
    }
}

/// `LinkSourceValidator` and `LinkTargetValidator`: a link with only one end.
///
/// Which keys make a link is [`Document::link_keys`], `EntityLinkManager`'s two sets.
fn check_links(document: &Document, issues: &mut Vec<Issue>) {
    let entities = map_model::node::entities_with_ids(document.tree_world());

    // The same two sets the views draw links from, so what is reported and what is drawn
    // cannot disagree.
    let keys = document.link_keys();
    let is_target_key = |key: &str| keys.is_target(key);
    let is_source_key = |key: &str| keys.is_source(key);

    // Every name that exists, and every name something aims at.
    let mut names: HashSet<&str> = HashSet::new();
    let mut aimed_at: HashSet<&str> = HashSet::new();
    for (_, entity) in &entities {
        for (key, value) in &entity.properties {
            if value.is_empty() {
                continue;
            }
            if is_source_key(key) {
                names.insert(value);
            }
            if is_target_key(key) {
                aimed_at.insert(value);
            }
        }
    }

    for (id, entity) in &entities {
        let name = entity.classname().unwrap_or("<no classname>");
        for (key, value) in &entity.properties {
            if value.is_empty() {
                continue;
            }
            if is_target_key(key) && !names.contains(value.as_str()) {
                issues.push(Issue {
                    severity: Severity::Warning,
                    kind: "missing link target",
                    description: format!(
                        "{name} targets \"{value}\" via \"{key}\", which nothing is called"
                    ),
                    node: Some(*id),
                });
            }
            if is_source_key(key) && !aimed_at.contains(value.as_str()) {
                issues.push(Issue {
                    severity: Severity::Hint,
                    kind: "missing link source",
                    description: format!(
                        "{name} is called \"{value}\", which nothing targets"
                    ),
                    node: Some(*id),
                });
            }
        }
    }
}

/// `EmptyGroupValidator`: a group with nothing in it.
fn check_groups(document: &Document, issues: &mut Vec<Issue>) {
    fn walk(nodes: &[Node], issues: &mut Vec<Issue>) {
        for node in nodes {
            if let NodeKind::Group(group) = &node.kind {
                if group.children.is_empty() {
                    issues.push(Issue {
                        severity: Severity::Hint,
                        kind: "empty group",
                        description: format!("Group \"{}\" is empty", group.name),
                        node: Some(node.id),
                    });
                }
                walk(&group.children, issues);
            }
        }
    }

    for layer in &document.tree_world().layers {
        walk(&layer.children, issues);
    }
}

/// The brush validators: where the corners are, what the faces wear, and whether the
/// brush is in the world at all.
fn check_brushes(document: &Document, issues: &mut Vec<Issue>) {
    let world_bounds = document.world_bounds();
    let soft_bounds = document.soft_map_bounds();
    let materials = document.materials();
    let has_project_materials = materials.materials().iter().any(|m| !m.path.starts_with("builtin/special"));

    for (id, brush) in map_model::node::brushes_with_ids(document.tree_world()) {
        // `WorldBoundsValidator`.
        let bounds = brush.bounds();
        if !world_bounds.contains(&bounds) {
            issues.push(Issue {
                severity: Severity::Error,
                kind: "outside the world",
                description: "Brush reaches outside the world bounds".to_string(),
                node: Some(id),
            });
        } else if soft_bounds.is_some_and(|soft| !soft.contains(&bounds)) {
            // `SoftMapBoundsValidator`: still inside the hard limits, but past the area
            // worldspawn says the map is meant to keep to.
            issues.push(Issue {
                severity: Severity::Warning,
                kind: "outside the soft bounds",
                description: "Brush reaches outside the soft map bounds".to_string(),
                node: Some(id),
            });
        }

        // Validate against the finest grid the editor supports, including fractional assets.
        if !brush.vertices_are_on_grid(2.0f64.powi(crate::grid::MIN_SIZE)) {
            issues.push(Issue {
                severity: Severity::Hint,
                kind: "off-grid vertices",
                description: "Brush has corners outside the 0.125-unit grid"
                    .to_string(),
                node: Some(id),
            });
        }

        // `MixedBrushContentsValidator`: a brush's content flags say what fills the
        // volume it bounds, so its faces should all carry the same ones. Faces without
        // Quake 2 / 3 attributes at all are left out of the comparison.
        let mut contents = brush
            .faces()
            .iter()
            .filter_map(|face| face.surface_attributes.map(|attrs| attrs.content_flags));
        if let Some(first) = contents.next() {
            if contents.any(|flags| flags != first) {
                issues.push(Issue {
                    severity: Severity::Warning,
                    kind: "mixed brush contents",
                    description: "Brush has faces with different content flags".to_string(),
                    node: Some(id),
                });
            }
        }

        let mut missing_materials = HashSet::new();
        for face in brush.faces() {
            // `InvalidUvScaleValidator` / `UvAttributes::valid`: the mapping is undefined
            // if a scale is zero (or all but), and cannot be written at all if an offset,
            // a scale or the rotation is infinite or NaN.
            let uv = face.uv_attributes;
            let finite = uv.offset.x().is_finite()
                && uv.offset.y().is_finite()
                && uv.scale.x().is_finite()
                && uv.scale.y().is_finite()
                && uv.rotation.is_finite();
            if !finite || uv.scale.x().abs() < 1e-6 || uv.scale.y().abs() < 1e-6 {
                issues.push(Issue {
                    severity: Severity::Warning,
                    kind: "invalid UV scale",
                    description: format!(
                        "Face wearing \"{}\" has an invalid UV scale",
                        face.material_name
                    ),
                    node: Some(id),
                });
            }

            // Reports missing materials rather than allowing their placeholder rendering to hide
            // absent game resources. A painted face shows and exports its colour, so its
            // material name is only a label there (the asset library's prefabs are made
            // that way) and its absence hides nothing.
            if has_project_materials
                && face.color.is_none()
                && face.material_name.as_str() != map_model::shapes::DEFAULT_MATERIAL
                && materials.get(&face.material_name).is_none()
                && missing_materials.insert(face.material_name.clone())
            {
                issues.push(Issue {
                    severity: Severity::Warning,
                    kind: "missing material",
                    description: format!(
                        "No material called \"{}\" in the loaded collection",
                        face.material_name
                    ),
                    node: Some(id),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;

    fn document_with_a_cube() -> Document {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::from_scalars(-32.0, 32.0),
            "stone",
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
        document.clear_selection();
        document
    }

    fn kinds(issues: &[Issue]) -> Vec<&str> {
        issues.iter().map(|issue| issue.kind).collect()
    }

    #[test]
    fn a_plain_box_has_nothing_wrong_with_it() {
        let document = document_with_a_cube();
        assert_eq!(validate(&document), Vec::new());
    }

    /// A cube whose faces carry the given content flags, one per face, as a `.map` file.
    fn cube_with_contents(flags: [i32; 6]) -> Document {
        let planes = [
            "( -16 -16 -16 ) ( -16 16 -16 ) ( -16 -16 16 )",
            "( 16 16 16 ) ( 16 -16 16 ) ( -16 16 16 )",
            "( -16 -16 -16 ) ( -16 -16 16 ) ( 16 -16 -16 )",
            "( 16 16 16 ) ( -16 16 16 ) ( 16 16 -16 )",
            "( -16 -16 -16 ) ( 16 -16 -16 ) ( -16 16 -16 )",
            "( 16 16 16 ) ( 16 16 -16 ) ( 16 -16 16 )",
        ];
        let mut source = String::from("{\n\"classname\" \"worldspawn\"\n{\n");
        for (plane, content) in planes.iter().zip(flags) {
            source.push_str(&format!("{plane} stone 0 0 0 1 1 {content} 0 0\n"));
        }
        source.push_str("}\n}\n");

        // A name of its own per call, so tests running in parallel do not fight over it.
        let path = std::env::temp_dir().join(format!(
            "heron-mixed-contents-{}-{:?}.map",
            std::process::id(),
            flags
        ));
        std::fs::write(&path, source).expect("a written map");
        let document = Document::open(&path, WorldBounds::default()).expect("it opens");
        let _ = std::fs::remove_file(&path);
        document
    }

    #[test]
    fn issues_come_back_worst_first() {
        let mut document = document_with_a_cube();
        // A hint (unused targetname), a warning (empty value), an error (quotation mark).
        let id = document.create_point_entity("info_player_start", Vec3d::zero());
        document.set_entity_property_on(&[id], "targetname", "lonely");
        document.set_entity_property_on(&[id], "note", r#"a "quote""#);
        document.set_entity_property_on(&[id], "blank", "");

        let severities: Vec<Severity> =
            validate(&document).iter().map(|i| i.severity).collect();
        assert!(
            severities.windows(2).all(|w| w[0] >= w[1]),
            "not sorted worst-first: {severities:?}"
        );
    }

    #[test]
    fn a_point_entity_outside_the_world_is_an_error() {
        let mut document = document_with_a_cube();
        let inside = document.create_point_entity("info_player_start", Vec3d::zero());
        let outside = document.create_point_entity(
            "info_player_start",
            Vec3d::new(100_000.0, 0.0, 0.0),
        );

        let issues = validate(&document);
        let out: Vec<NodeId> = issues
            .iter()
            .filter(|i| i.kind == "outside the world")
            .filter_map(|i| i.node)
            .collect();
        assert!(out.contains(&outside), "the far entity is caught");
        assert!(!out.contains(&inside), "the one at the origin is not");
    }

    #[test]
    fn a_zero_uv_scale_is_reported() {
        // The first face's x scale is zero, which makes its mapping undefined.
        let source = "\
{
\"classname\" \"worldspawn\"
{
( -16 -16 -16 ) ( -16 16 -16 ) ( -16 -16 16 ) stone 0 0 0 0 1
( -16 -16 -16 ) ( -16 -16 16 ) ( 16 -16 -16 ) stone 0 0 0 1 1
( -16 -16 -16 ) ( 16 -16 -16 ) ( -16 16 -16 ) stone 0 0 0 1 1
( 16 16 16 ) ( 16 -16 16 ) ( -16 16 16 ) stone 0 0 0 1 1
( 16 16 16 ) ( -16 16 16 ) ( 16 16 -16 ) stone 0 0 0 1 1
( 16 16 16 ) ( 16 16 -16 ) ( 16 -16 16 ) stone 0 0 0 1 1
}
}
";
        let path =
            std::env::temp_dir().join(format!("heron-zero-uv-{}.map", std::process::id()));
        std::fs::write(&path, source).expect("a written map");
        let document = Document::open(&path, WorldBounds::default()).expect("it opens");
        let _ = std::fs::remove_file(&path);

        let issues = validate(&document);
        assert_eq!(
            issues.iter().filter(|i| i.kind == "invalid UV scale").count(),
            1,
            "just the one face"
        );
    }

    #[test]
    fn a_brush_whose_faces_disagree_on_content_flags_is_a_warning() {
        let document = cube_with_contents([1, 1, 1, 1, 1, 8]);
        let issues = validate(&document);
        assert!(kinds(&issues).contains(&"mixed brush contents"));
        assert_eq!(
            issues
                .iter()
                .find(|i| i.kind == "mixed brush contents")
                .unwrap()
                .severity,
            Severity::Warning
        );
    }

    #[test]
    fn a_brush_whose_faces_agree_on_content_flags_is_fine() {
        let document = cube_with_contents([1, 1, 1, 1, 1, 1]);
        assert!(!kinds(&validate(&document)).contains(&"mixed brush contents"));
    }

    #[test]
    fn a_brush_past_the_soft_map_bounds_is_a_warning() {
        // A worldspawn that keeps the map to a small box, and a cube well outside it but
        // still inside the hard world bounds.
        let source = "\
{
\"classname\" \"worldspawn\"
\"_tb_soft_map_bounds\" \"-64 -64 -64 64 64 64\"
{
( 500 500 500 ) ( 500 501 500 ) ( 500 500 501 ) stone 0 0 0 1 1
( 600 600 600 ) ( 600 600 601 ) ( 600 601 600 ) stone 0 0 0 1 1
( 500 500 500 ) ( 500 500 501 ) ( 501 500 500 ) stone 0 0 0 1 1
( 500 600 500 ) ( 501 600 500 ) ( 500 600 501 ) stone 0 0 0 1 1
( 500 500 500 ) ( 501 500 500 ) ( 500 501 500 ) stone 0 0 0 1 1
( 500 500 600 ) ( 500 501 600 ) ( 501 500 600 ) stone 0 0 0 1 1
}
}
";
        let path = std::env::temp_dir().join(format!("heron-soft-bounds-{}.map", std::process::id()));
        std::fs::write(&path, source).expect("a written map");
        let document = Document::open(&path, WorldBounds::default()).expect("it opens");

        let issues = validate(&document);
        assert!(kinds(&issues).contains(&"outside the soft bounds"));
        // Not the hard-bounds error -- 600 units is well inside a 16384 world.
        assert!(!kinds(&issues).contains(&"outside the world"));
        assert_eq!(
            issues
                .iter()
                .find(|i| i.kind == "outside the soft bounds")
                .unwrap()
                .severity,
            Severity::Warning
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn with_no_soft_bounds_set_nothing_is_reported_for_them() {
        let document = document_with_a_cube();
        assert!(document.soft_map_bounds().is_none());
        assert!(!kinds(&validate(&document)).contains(&"outside the soft bounds"));
    }

    #[test]
    fn a_backslash_in_worldspawns_paths_is_a_warning() {
        let source = "\
{
\"classname\" \"worldspawn\"
\"wad\" \"C:\\\\quake\\\\id1\\\\textures.wad\"
\"_tb_def\" \"defs/quake.fgd\"
{
( -16 -16 -16 ) ( -16 16 -16 ) ( -16 -16 16 ) stone 0 0 0 1 1
( -16 -16 -16 ) ( -16 -16 16 ) ( 16 -16 -16 ) stone 0 0 0 1 1
( -16 -16 -16 ) ( 16 -16 -16 ) ( -16 16 -16 ) stone 0 0 0 1 1
( 16 16 16 ) ( 16 -16 16 ) ( -16 16 16 ) stone 0 0 0 1 1
( 16 16 16 ) ( -16 16 16 ) ( 16 16 -16 ) stone 0 0 0 1 1
( 16 16 16 ) ( 16 16 -16 ) ( 16 -16 16 ) stone 0 0 0 1 1
}
}
";
        let path =
            std::env::temp_dir().join(format!("heron-backslash-{}.map", std::process::id()));
        std::fs::write(&path, source).expect("a written map");
        let document = Document::open(&path, WorldBounds::default()).expect("it opens");

        let issues = validate(&document);
        let path_issues: Vec<&Issue> =
            issues.iter().filter(|i| i.kind == "backslash in a path").collect();
        assert_eq!(path_issues.len(), 1, "only the wad, not the forward-slashed def");
        assert!(path_issues[0].description.contains("wad"));
        assert_eq!(path_issues[0].severity, Severity::Warning);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_entity_without_a_class_is_an_error() {
        let mut document = document_with_a_cube();
        let id = document.create_point_entity("info_player_start", Vec3d::zero());
        document.remove_entity_property_on(&[id], "classname");

        let issues = validate(&document);
        assert!(kinds(&issues).contains(&"missing classname"));
        assert_eq!(issues[0].severity, Severity::Error);
    }

    #[test]
    fn a_quotation_mark_in_a_property_cannot_be_written_back() {
        let mut document = document_with_a_cube();
        let id = document.create_point_entity("info_player_start", Vec3d::zero());
        document.set_entity_property(id, "message", r#"he said "hello""#);

        assert!(kinds(&validate(&document)).contains(&"quotation mark in property"));
    }

    #[test]
    fn keyframe_curves_json_is_not_a_quotation_mark_error() {
        let mut document = document_with_a_cube();
        let id = document.create_point_entity("info_player_start", Vec3d::zero());
        document.set_entity_property(id, "animation_curves", r#"{"version":1,"tracks":[]}"#);

        assert!(!kinds(&validate(&document)).contains(&"quotation mark in property"));
    }

    #[test]
    fn an_empty_property_value_is_worth_knowing_about() {
        let mut document = document_with_a_cube();
        let id = document.create_point_entity("info_player_start", Vec3d::zero());
        document.set_entity_property(id, "targetname", "");

        assert!(kinds(&validate(&document)).contains(&"empty property value"));
    }

    #[test]
    fn a_link_pointing_at_nothing_is_reported() {
        let mut document = document_with_a_cube();
        let id = document.create_point_entity("trigger_once", Vec3d::zero());
        document.set_entity_property(id, "target", "the_door");

        let issues = validate(&document);
        assert!(kinds(&issues).contains(&"missing link target"));
        assert!(issues
            .iter()
            .any(|issue| issue.description.contains("the_door")));
    }

    #[test]
    fn a_link_nothing_points_at_is_reported_more_gently() {
        let mut document = document_with_a_cube();
        let id = document.create_point_entity("func_door", Vec3d::zero());
        document.set_entity_property(id, "targetname", "the_door");

        let issues = validate(&document);
        let hint = issues
            .iter()
            .find(|issue| issue.kind == "missing link source")
            .expect("the unused name is reported");
        // Nothing targeting a name is often deliberate, so it is a hint rather than a
        // warning.
        assert_eq!(hint.severity, Severity::Hint);
    }

    #[test]
    fn killtarget_and_numbered_targets_are_links_too() {
        let mut document = document_with_a_cube();
        let trigger = document.create_point_entity("trigger_multiple", Vec3d::zero());
        document.set_entity_property(trigger, "killtarget", "the_monster");
        document.set_entity_property(trigger, "target2", "the_door");
        // A real name for one of them, so only the other is reported.
        let door = document.create_point_entity("func_door", Vec3d::new(64.0, 0.0, 0.0));
        document.set_entity_property(door, "targetname", "the_door");

        let issues = validate(&document);
        let targets: Vec<String> = issues
            .iter()
            .filter(|i| i.kind == "missing link target")
            .map(|i| i.description.clone())
            .collect();
        assert_eq!(targets.len(), 1, "the door is fine, the monster is not: {targets:?}");
        assert!(targets[0].contains("the_monster") && targets[0].contains("killtarget"));
    }

    #[test]
    fn a_link_with_both_ends_is_not_reported() {
        let mut document = document_with_a_cube();
        let trigger = document.create_point_entity("trigger_once", Vec3d::zero());
        document.set_entity_property(trigger, "target", "the_door");
        let door = document.create_point_entity("func_door", Vec3d::new(64.0, 0.0, 0.0));
        document.set_entity_property(door, "targetname", "the_door");

        let issues = validate(&document);
        assert!(!kinds(&issues).contains(&"missing link target"));
        assert!(!kinds(&issues).contains(&"missing link source"));
    }

    #[test]
    fn eighth_unit_corners_are_valid() {
        let mut document = Document::new(WorldBounds::default());
        let brush = map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &Bbox3d::new(Vec3d::new(-0.125, 0.375, 0.5), Vec3d::new(32.0, 32.0, 32.0)),
            "stone",
        ).unwrap();
        document.add_brushes(vec![brush]);
        assert!(!kinds(&validate(&document)).contains(&"off-grid vertices"));
    }

    #[test]
    fn corners_off_the_grid_are_a_hint() {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(Vec3d::new(0.06, 0.0, 0.0), Vec3d::new(32.0, 32.0, 32.0)),
            "stone",
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);

        let issues = validate(&document);
        assert!(kinds(&issues).contains(&"off-grid vertices"));
    }

    #[test]
    fn a_material_the_collection_does_not_have_is_reported_once_per_name() {
        let directory = std::env::temp_dir().join("heron-validation-materials");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a directory");
        image::RgbaImage::from_pixel(8, 8, image::Rgba([255; 4]))
            .save(directory.join("brick.png"))
            .expect("a written png");

        let mut document = document_with_a_cube();
        document.load_materials(&directory).expect("the materials load");

        // Every face of the cube wears "stone", which is not in the collection, and one
        // report is enough.
        let issues = validate(&document);
        let missing = issues
            .iter()
            .filter(|issue| issue.kind == "missing material")
            .count();
        assert_eq!(missing, 1);
    }

    #[test]
    fn an_unknown_class_is_reported_against_the_bundled_definitions() {
        let mut document = document_with_a_cube();
        document.create_point_entity("some_made_up_class", Vec3d::zero());

        // Built-in definitions are available even without a project FGD.
        assert!(kinds(&validate(&document)).contains(&"unknown class"));
    }
}


/// What to tell the user about a leak check.
pub fn describe_leak(outcome: &map_model::leak::Leak) -> String {
    match outcome {
        map_model::leak::Leak::Sealed => "Sealed: nothing outside can be reached".to_string(),
        map_model::leak::Leak::Leaked(trace) => format!(
            "Leaks. The line drawn in the views is the way out, {} turns long.",
            trace.points.len().saturating_sub(1)
        ),
        map_model::leak::Leak::NothingToSeal => "There is no geometry to seal".to_string(),
        map_model::leak::Leak::NoStartingPoint => {
            "There is no point entity to check from".to_string()
        }
        map_model::leak::Leak::StartsInsideSolid => {
            "Every point entity is inside solid geometry".to_string()
        }
        map_model::leak::Leak::TooLarge { cells, budget } => format!(
            "Too large to check at this cell size: it would need {cells} cells and \
             {budget} is the most it will do. Raise the cell size in the preferences."
        ),
    }
}

#[cfg(test)]
mod bundled_assets {
    /// The asset library's prefabs open with nothing to warn about: a painted face's
    /// material name is a label, not a missing texture.
    #[test]
    fn every_bundled_asset_checks_clean() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let mut found = Vec::new();
        for path in std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "map")) {
            let document = crate::document::Document::open(&path, Default::default()).unwrap();
            for issue in super::validate(&document) {
                if issue.severity != super::Severity::Hint {
                    found.push(format!("{}: {}", path.file_name().unwrap().to_string_lossy(), issue.description));
                }
            }
        }
        found.sort();
        found.dedup();
        assert!(found.is_empty(), "{found:#?}");
    }
}
