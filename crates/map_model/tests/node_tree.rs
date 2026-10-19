//! Tests for building the document tree from a map file.

use map_model::brush::Brush;
use map_model::io::map_reader::read_map;
use map_model::node::{build_tree, brushes, NodeKind, Tree, TreeWarning, DEFAULT_LAYER_NAME};
use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

/// A cube brush at the given position, written in the Standard format.
fn cube(min: i32, max: i32) -> String {
    format!(
        "{{\n\
         ( {min} {min} {min} ) ( {min} {p} {min} ) ( {min} {min} {p} ) stone 0 0 0 1 1\n\
         ( {min} {min} {min} ) ( {min} {min} {p} ) ( {p} {min} {min} ) stone 0 0 0 1 1\n\
         ( {min} {min} {min} ) ( {p} {min} {min} ) ( {min} {p} {min} ) stone 0 0 0 1 1\n\
         ( {max} {max} {max} ) ( {max} {q} {max} ) ( {q} {max} {max} ) stone 0 0 0 1 1\n\
         ( {max} {max} {max} ) ( {q} {max} {max} ) ( {max} {max} {r} ) stone 0 0 0 1 1\n\
         ( {max} {max} {max} ) ( {max} {max} {r} ) ( {max} {q} {max} ) stone 0 0 0 1 1\n\
         }}\n",
        p = min + 1,
        q = max + 1,
        r = max + 1,
    )
}

fn tree(source: &str) -> Tree {
    let map = read_map(source, None).expect("the sample parses");
    build_tree(&map, &world_bounds())
}

#[test]
fn a_bare_worldspawn_gives_a_default_layer() {
    let source = format!("{{\n\"classname\" \"worldspawn\"\n{}}}\n", cube(-64, 64));
    let tree = tree(&source);

    assert_eq!(tree.world.layers.len(), 1);
    let layer = &tree.world.layers[0];
    assert_eq!(layer.name, DEFAULT_LAYER_NAME);
    assert_eq!(layer.id, None);
    assert_eq!(layer.children.len(), 1);
    assert!(matches!(layer.children[0].kind, NodeKind::Brush(_)));
    assert!(tree.warnings.is_empty(), "{:?}", tree.warnings);
}

#[test]
fn the_structural_properties_are_not_kept_on_the_world() {
    let source = "{\n\"classname\" \"worldspawn\"\n\
                  \"_tb_type\" \"_tb_layer\"\n\
                  \"_tb_textures\" \"textures\"\n\
                  \"wad\" \"x.wad\"\n}\n";
    let tree = tree(source);

    // What the tree itself stands for is not kept, since it is written back out from the
    // tree's shape and two copies could disagree.
    assert!(!tree
        .world
        .properties
        .iter()
        .any(|(k, _)| k == "_tb_type"));
    // Everything else is, whoever owns it: the game's properties and the editor's
    // settings alike. Dropping one would lose it on the next save.
    assert!(tree
        .world
        .properties
        .iter()
        .any(|(k, v)| k == "wad" && v == "x.wad"));
    assert!(tree
        .world
        .properties
        .iter()
        .any(|(k, v)| k == "_tb_textures" && v == "textures"));
}

#[test]
fn a_point_entity_goes_into_the_default_layer() {
    let source = "{\n\"classname\" \"worldspawn\"\n}\n\
                  {\n\"classname\" \"info_player_start\"\n\"origin\" \"0 64 24\"\n}\n";
    let tree = tree(source);

    let layer = &tree.world.layers[0];
    assert_eq!(layer.children.len(), 1);
    let NodeKind::Entity(entity) = &layer.children[0].kind else {
        panic!("expected an entity");
    };
    assert_eq!(entity.classname(), Some("info_player_start"));
    assert_eq!(entity.property("origin"), Some("0 64 24"));
    assert!(!entity.is_brush_entity());
}

#[test]
fn layers_are_reassembled_from_their_func_group_entities() {
    let source = format!(
        "{{\n\"classname\" \"worldspawn\"\n}}\n\
         {{\n\
         \"classname\" \"func_group\"\n\
         \"_tb_type\" \"_tb_layer\"\n\
         \"_tb_name\" \"Detail\"\n\
         \"_tb_id\" \"2\"\n\
         \"_tb_layer_sort_index\" \"1\"\n\
         \"_tb_layer_locked\" \"1\"\n\
         {}}}\n",
        cube(0, 32)
    );
    let tree = tree(&source);

    assert_eq!(tree.world.layers.len(), 2);
    let detail = &tree.world.layers[1];
    assert_eq!(detail.name, "Detail");
    assert_eq!(detail.id.as_deref(), Some("2"));
    assert_eq!(detail.sort_index, 1);
    assert!(detail.locked);
    assert!(!detail.hidden);
    assert!(tree.warnings.is_empty(), "{:?}", tree.warnings);
}

#[test]
fn a_layers_own_brushes_are_its_contents() {
    let source = format!(
        "{{\n\"classname\" \"worldspawn\"\n}}\n\
         {{\n\
         \"classname\" \"func_group\"\n\
         \"_tb_type\" \"_tb_layer\"\n\
         \"_tb_name\" \"Detail\"\n\
         \"_tb_id\" \"2\"\n\
         {}}}\n",
        cube(0, 32)
    );
    let tree = tree(&source);

    // The brush written inside the layer's own entity belongs to the layer.
    let detail = &tree.world.layers[1];
    assert_eq!(detail.children.len(), 1);
    assert!(matches!(detail.children[0].kind, NodeKind::Brush(_)));
    // And it is reachable when walking the whole tree.
    assert_eq!(brushes(&tree.world).len(), 1);
}

#[test]
fn a_groups_own_brushes_are_its_contents() {
    let source = format!(
        "{{\n\"classname\" \"worldspawn\"\n}}\n\
         {{\n\
         \"classname\" \"func_group\"\n\
         \"_tb_type\" \"_tb_group\"\n\
         \"_tb_name\" \"Door\"\n\
         \"_tb_id\" \"7\"\n\
         {}}}\n",
        cube(0, 32)
    );
    let tree = tree(&source);

    let NodeKind::Group(group) = &tree.world.layers[0].children[0].kind else {
        panic!("expected a group in the default layer");
    };
    assert_eq!(group.children.len(), 1);
    assert!(matches!(group.children[0].kind, NodeKind::Brush(_)));
}

#[test]
fn an_entity_is_placed_in_the_layer_it_names() {
    let source = "{\n\"classname\" \"worldspawn\"\n}\n\
                  {\n\"classname\" \"func_group\"\n\"_tb_type\" \"_tb_layer\"\n\"_tb_name\" \"Detail\"\n\"_tb_id\" \"2\"\n}\n\
                  {\n\"classname\" \"light\"\n\"_tb_layer\" \"2\"\n}\n";
    let tree = tree(source);

    assert!(tree.world.layers[0].children.is_empty());
    assert_eq!(tree.world.layers[1].children.len(), 1);
}

#[test]
fn a_group_nests_inside_its_layer_and_holds_its_own_children() {
    let source = "{\n\"classname\" \"worldspawn\"\n}\n\
                  {\n\"classname\" \"func_group\"\n\"_tb_type\" \"_tb_layer\"\n\"_tb_name\" \"Detail\"\n\"_tb_id\" \"2\"\n}\n\
                  {\n\"classname\" \"func_group\"\n\"_tb_type\" \"_tb_group\"\n\"_tb_name\" \"Door\"\n\"_tb_id\" \"7\"\n\"_tb_layer\" \"2\"\n}\n\
                  {\n\"classname\" \"light\"\n\"_tb_group\" \"7\"\n}\n";
    let tree = tree(source);

    let detail = &tree.world.layers[1];
    assert_eq!(detail.children.len(), 1);
    let NodeKind::Group(group) = &detail.children[0].kind else {
        panic!("expected a group in the layer");
    };
    assert_eq!(group.name, "Door");
    assert_eq!(group.id, "7");
    assert_eq!(group.children.len(), 1);
}

#[test]
fn a_group_inside_a_group_nests() {
    let source = "{\n\"classname\" \"worldspawn\"\n}\n\
                  {\n\"classname\" \"func_group\"\n\"_tb_type\" \"_tb_group\"\n\"_tb_name\" \"Outer\"\n\"_tb_id\" \"1\"\n}\n\
                  {\n\"classname\" \"func_group\"\n\"_tb_type\" \"_tb_group\"\n\"_tb_name\" \"Inner\"\n\"_tb_id\" \"2\"\n\"_tb_group\" \"1\"\n}\n\
                  {\n\"classname\" \"light\"\n\"_tb_group\" \"2\"\n}\n";
    let tree = tree(source);

    let NodeKind::Group(outer) = &tree.world.layers[0].children[0].kind else {
        panic!("expected the outer group in the default layer");
    };
    assert_eq!(outer.name, "Outer");

    let NodeKind::Group(inner) = &outer.children[0].kind else {
        panic!("expected the inner group inside the outer one");
    };
    assert_eq!(inner.name, "Inner");
    assert_eq!(inner.children.len(), 1);
}

#[test]
fn a_node_naming_a_group_that_is_not_there_lands_in_the_default_layer() {
    let source = "{\n\"classname\" \"worldspawn\"\n}\n\
                  {\n\"classname\" \"light\"\n\"_tb_group\" \"missing\"\n}\n";
    let tree = tree(source);

    assert_eq!(tree.world.layers[0].children.len(), 1);
    assert_eq!(
        tree.warnings,
        vec![TreeWarning::MissingParent {
            id: "missing".to_string(),
            line: 4,
        }]
    );
}

#[test]
fn a_file_without_a_worldspawn_still_opens() {
    let source = "{\n\"classname\" \"light\"\n}\n";
    let tree = tree(source);

    assert_eq!(tree.warnings, vec![TreeWarning::NoWorldspawn]);
    assert_eq!(tree.world.layers.len(), 1);
    assert_eq!(tree.world.layers[0].children.len(), 1);
}

#[test]
fn a_brush_that_encloses_nothing_is_dropped_with_a_warning() {
    // Two faces facing away from each other, plus two more: no volume.
    let source = "{\n\"classname\" \"worldspawn\"\n{\n\
                  ( 0 0 0 ) ( 0 1 0 ) ( 1 0 0 ) a 0 0 0 1 1\n\
                  ( 0 0 10 ) ( 1 0 10 ) ( 0 1 10 ) a 0 0 0 1 1\n\
                  ( 0 0 0 ) ( 1 0 0 ) ( 0 0 1 ) a 0 0 0 1 1\n\
                  ( 0 0 0 ) ( 0 0 1 ) ( 0 1 0 ) a 0 0 0 1 1\n\
                  }\n}\n";
    let tree = tree(source);

    assert!(tree.world.layers[0].children.is_empty());
    assert_eq!(tree.warnings.len(), 1);
    assert!(
        matches!(tree.warnings[0], TreeWarning::InvalidBrush { .. }),
        "{:?}",
        tree.warnings[0]
    );
}

/* ========== the brush itself ========== */

#[test]
fn a_brush_matches_its_geometry_faces_back_to_its_own_faces() {
    let source = format!("{{\n\"classname\" \"worldspawn\"\n{}}}\n", cube(-64, 64));
    let tree = tree(&source);
    let brushes = brushes(&tree.world);
    assert_eq!(brushes.len(), 1);

    let brush = brushes[0];
    assert_eq!(brush.faces().len(), 6);
    assert_eq!(brush.geometry().face_count(), 6);

    // Every surface knows which face it came from, and every one has the same material.
    let surfaces: Vec<_> = brush.geometry_faces().collect();
    assert_eq!(surfaces.len(), 6);
    for (face, polygon) in &surfaces {
        assert_eq!(face.material_name, "stone");
        assert_eq!(polygon.len(), 4);
        // Every vertex of the polygon lies on the face it belongs to.
        for point in polygon {
            assert!(face.plane.point_distance(*point).abs() < 1e-6);
        }
    }
}

#[test]
fn a_brush_reports_its_bounds_vertices_and_edges() {
    let source = format!("{{\n\"classname\" \"worldspawn\"\n{}}}\n", cube(-64, 64));
    let tree = tree(&source);
    let brush = brushes(&tree.world)[0];

    geometry_math::assert_approx_eq!(brush.bounds().min, Vec3d::new(-64.0, -64.0, -64.0), 1e-6);
    geometry_math::assert_approx_eq!(brush.bounds().max, Vec3d::new(64.0, 64.0, 64.0), 1e-6);

    assert_eq!(brush.vertex_positions().len(), 8);
    assert_eq!(brush.edge_positions().len(), 12);
    // Every edge of a cube this size has the same length.
    for (start, end) in brush.edge_positions() {
        assert!((geometry_math::vec::length(end - start) - 128.0).abs() < 1e-6);
    }

    assert!(brush.contains_point(Vec3d::zero()));
    assert!(!brush.contains_point(Vec3d::new(100.0, 0.0, 0.0)));
}

#[test]
fn a_brush_needs_at_least_four_faces() {
    use map_model::brush::BrushError;
    use map_model::brush_face::BrushFace;
    use map_model::uv::{UvAttributes, UvCoordSystem};

    let face = BrushFace::from_points(
        [
            Vec3d::zero(),
            Vec3d::new(0.0, 1.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
        ],
        "stone",
        UvCoordSystem::Paraxial,
        UvAttributes::default(),
    )
    .expect("the points define a plane");

    let error = Brush::from_faces(&world_bounds(), vec![face.clone(), face.clone(), face])
        .expect_err("three faces cannot enclose a volume");
    assert_eq!(error, BrushError::NotEnoughFaces { given: 3 });
}

/* ========== snapping vertices to the grid ========== */

/// Builds a brush from six axis-aligned planes at the given bounds.
fn box_brush(min: Vec3d, max: Vec3d) -> Brush {
    use map_model::brush_face::BrushFace;
    use map_model::uv::{UvAttributes, UvCoordSystem};
    use geometry_math::plane::Plane3d;

    let named = [
        (Plane3d::from_anchor_and_normal(max, Vec3d::new(1.0, 0.0, 0.0)), "east"),
        (Plane3d::from_anchor_and_normal(min, Vec3d::new(-1.0, 0.0, 0.0)), "west"),
        (Plane3d::from_anchor_and_normal(max, Vec3d::new(0.0, 1.0, 0.0)), "north"),
        (Plane3d::from_anchor_and_normal(min, Vec3d::new(0.0, -1.0, 0.0)), "south"),
        (Plane3d::from_anchor_and_normal(max, Vec3d::new(0.0, 0.0, 1.0)), "top"),
        (Plane3d::from_anchor_and_normal(min, Vec3d::new(0.0, 0.0, -1.0)), "bottom"),
    ];

    let faces = named
        .iter()
        .map(|(plane, name)| {
            let mut face = BrushFace::from_points(
                [Vec3d::zero(), Vec3d::new(0.0, 1.0, 0.0), Vec3d::new(1.0, 0.0, 0.0)],
                *name,
                UvCoordSystem::Paraxial,
                UvAttributes::default(),
            )
            .expect("the placeholder points define a plane");
            // Replace the placeholder plane with the one we want; the points are
            // rederived from it, as they would be for a brush the editor built.
            face.plane = *plane;
            face.points = face.derive_points();
            face
        })
        .collect();

    Brush::from_faces(&world_bounds(), faces).expect("six planes make a box")
}

#[test]
fn snapping_puts_the_corners_on_round_numbers() {
    // Deliberately off the grid.
    let brush = box_brush(
        Vec3d::new(-63.7, -63.2, -15.9),
        Vec3d::new(64.4, 63.6, 16.1),
    );
    assert!(!brush.vertices_are_on_grid(16.0));

    let snapped = brush.snap_vertices(16.0).expect("the brush survives snapping");
    assert!(snapped.vertices_are_on_grid(16.0));

    for position in snapped.vertex_positions() {
        for component in position.v {
            assert!(
                (component / 16.0).fract().abs() < 1e-9,
                "{component} is not a multiple of 16"
            );
        }
    }
}

#[test]
fn snapping_removes_the_drift_that_clipping_leaves_behind() {
    // Building by clipping leaves corners a rounding error away from where the planes say.
    let brush = box_brush(Vec3d::new(-512.0, -512.0, -32.0), Vec3d::new(512.0, 512.0, 32.0));
    let exact = brush
        .vertex_positions()
        .iter()
        .all(|p| p.v.iter().all(|c| c.fract() == 0.0));

    let snapped = brush.snap_vertices(1.0).expect("the brush survives snapping");
    assert!(
        snapped
            .vertex_positions()
            .iter()
            .all(|p| p.v.iter().all(|c| c.fract() == 0.0)),
        "snapping should leave every coordinate exact, whether or not it already was ({exact})"
    );

    geometry_math::assert_approx_eq!(snapped.bounds().min, Vec3d::new(-512.0, -512.0, -32.0), 1e-12);
    geometry_math::assert_approx_eq!(snapped.bounds().max, Vec3d::new(512.0, 512.0, 32.0), 1e-12);
}

#[test]
fn snapping_keeps_the_materials_on_the_right_faces() {
    let brush = box_brush(
        Vec3d::new(-63.7, -63.2, -15.9),
        Vec3d::new(64.4, 63.6, 16.1),
    );
    let snapped = brush.snap_vertices(16.0).expect("the brush survives snapping");

    // Every face still carries the material of the face pointing the same way.
    for (face, _) in snapped.geometry_faces() {
        let expected = match (
            face.normal().x().round() as i32,
            face.normal().y().round() as i32,
            face.normal().z().round() as i32,
        ) {
            (1, 0, 0) => "east",
            (-1, 0, 0) => "west",
            (0, 1, 0) => "north",
            (0, -1, 0) => "south",
            (0, 0, 1) => "top",
            (0, 0, -1) => "bottom",
            other => panic!("unexpected face normal {other:?}"),
        };
        assert_eq!(face.material_name, expected);
    }
}

#[test]
fn snapping_to_a_grid_coarser_than_the_brush_is_refused() {
    use map_model::brush::SnapError;

    // A brush 8 units across cannot survive being snapped to a 256 unit grid: every
    // corner lands on the same point.
    let brush = box_brush(Vec3d::new(-4.0, -4.0, -4.0), Vec3d::new(4.0, 4.0, 4.0));
    let error = brush
        .snap_vertices(256.0)
        .expect_err("the brush cannot survive this");
    assert!(matches!(error, SnapError::Collapsed(_)), "{error}");
}

#[test]
fn a_non_positive_grid_size_is_refused() {
    use map_model::brush::SnapError;

    let brush = box_brush(Vec3d::new(-8.0, -8.0, -8.0), Vec3d::new(8.0, 8.0, 8.0));
    // `Brush` has no `PartialEq` — two brushes can describe the same solid while
    // numbering their vertices differently — so the error is matched rather than compared.
    assert!(matches!(
        brush.snap_vertices(0.0),
        Err(SnapError::InvalidGridSize { given }) if given == 0.0
    ));
    assert!(brush.snap_vertices(-1.0).is_err());
}

#[test]
fn an_entity_reads_and_writes_its_protected_property_list() {
    let mut entity = map_model::node::EntityNode {
        properties: vec![
            ("classname".to_string(), "light".to_string()),
            ("_tb_protected_properties".to_string(), "targetname;angle".to_string()),
        ],
        children: Vec::new(),
    };
    assert_eq!(entity.protected_properties(), vec!["targetname", "angle"]);

    // Setting a new list rewrites the key; an empty list drops it.
    entity.set_protected_properties(&["light".to_string()]);
    assert_eq!(entity.property("_tb_protected_properties"), Some("light"));
    entity.set_protected_properties(&[]);
    assert_eq!(entity.property("_tb_protected_properties"), None);
    assert_eq!(entity.protected_properties(), Vec::<String>::new());
}

#[test]
fn snapping_a_brush_that_is_already_on_the_grid_changes_nothing() {
    let brush = box_brush(Vec3d::new(-64.0, -64.0, -16.0), Vec3d::new(64.0, 64.0, 16.0));
    let snapped = brush.snap_vertices(16.0).expect("nothing to do");

    assert_eq!(snapped.vertex_positions().len(), brush.vertex_positions().len());
    geometry_math::assert_approx_eq!(snapped.bounds().min, brush.bounds().min, 1e-9);
    geometry_math::assert_approx_eq!(snapped.bounds().max, brush.bounds().max, 1e-9);
}

/// One walk replacing many nodes leaves the tree exactly as replacing them one at a time
/// does, including where a brush and the entity around it are both in the list.
#[test]
fn replacing_many_nodes_at_once_matches_replacing_them_in_turn() {
    use map_model::node::{brushes_with_ids, entities_with_ids, replace_node, replace_nodes, Node};
    let source = format!(
        "{{\n\"classname\" \"worldspawn\"\n{}{}}}\n{{\n\"classname\" \"func_door\"\n{}{}}}\n",
        cube(-64, 64),
        cube(100, 120),
        cube(200, 220),
        cube(300, 320)
    );
    let original = tree(&source).world;
    let moved = |brush: &Brush, by: f64| brush.translate(&world_bounds(), Vec3d::new(by, 0.0, 0.0)).unwrap();
    let brushes: Vec<_> = brushes_with_ids(&original).into_iter().map(|(id, b)| (id, b.clone())).collect();
    let (door_id, door) = entities_with_ids(&original).into_iter().find(|(_, e)| e.classname() == Some("func_door")).map(|(id, e)| (id, e.clone())).unwrap();
    let mut changed_door = door.clone();
    changed_door.properties.push(("speed".into(), "50".into()));
    let door_child = changed_door.children[0].id;
    let brush_of = |id| brushes.iter().find(|(b, _)| *b == id).map(|(_, b)| b.clone()).unwrap();
    for replacements in [
        vec![Node::with_id(brushes[0].0, NodeKind::Brush(moved(&brushes[0].1, 8.0))), Node::with_id(brushes[1].0, NodeKind::Brush(moved(&brushes[1].1, 16.0)))],
        // The door's brush before the door: the door's own copy wins.
        vec![Node::with_id(door_child, NodeKind::Brush(moved(&brush_of(door_child), 32.0))), Node::with_id(door_id, NodeKind::Entity(changed_door.clone()))],
        // And after it: the brush wins.
        vec![Node::with_id(door_id, NodeKind::Entity(changed_door.clone())), Node::with_id(door_child, NodeKind::Brush(moved(&brush_of(door_child), 32.0))), Node::with_id(brushes[0].0, NodeKind::Brush(moved(&brushes[0].1, 4.0)))],
    ] {
        let mut one_by_one = original.clone();
        for node in &replacements {
            replace_node(&mut one_by_one, node.clone());
        }
        let mut at_once = original.clone();
        replace_nodes(&mut at_once, &replacements);
        assert_eq!(format!("{at_once:?}"), format!("{one_by_one:?}"));
    }
}

/// The one-walk group queries agree with asking group by group: content counts with
/// `contents_of`, and the outermost-group map with `outermost_group_of` and
/// `contents_of`, with no group open and with the nested one open.
#[test]
fn one_walk_group_queries_match_the_per_node_ones() {
    use map_model::node::{all_node_ids, contents_of, groups_with_content_counts, groups_with_ids, outermost_group_of, outermost_groups};
    let group = |name: &str, id: u32, parent: Option<u32>| {
        let parent = parent.map(|p| format!("\"_tb_group\" \"{p}\"\n")).unwrap_or_default();
        format!("\"classname\" \"func_group\"\n\"_tb_type\" \"_tb_group\"\n\"_tb_name\" \"{name}\"\n\"_tb_id\" \"{id}\"\n{parent}")
    };
    let source = [
        format!("{{\n\"classname\" \"worldspawn\"\n{}}}\n", cube(-64, 64)),
        format!("{{\n{}{}}}\n", group("Outer", 1, None), cube(100, 120)),
        format!("{{\n{}{}{}}}\n", group("Inner", 2, Some(1)), cube(200, 220), cube(230, 240)),
        "{\n\"classname\" \"light\"\n\"origin\" \"0 0 0\"\n\"_tb_group\" \"2\"\n}\n".to_string(),
        format!("{{\n\"classname\" \"func_door\"\n\"_tb_group\" \"2\"\n{}}}\n", cube(300, 320)),
        format!("{{\n{}}}\n", group("Empty", 3, None)),
    ]
    .concat();
    let world = tree(&source).world;
    let counted = groups_with_content_counts(&world);
    assert_eq!(counted.len(), 3);
    for (id, _, count) in &counted {
        assert_eq!(*count, contents_of(&world, *id).len());
    }
    let entries = map_model::node::groups_with_contents(&world);
    assert_eq!(entries.iter().map(|e| e.id).collect::<Vec<_>>(), counted.iter().map(|(id, _, _)| *id).collect::<Vec<_>>());
    for entry in &entries {
        assert_eq!(entry.contents, contents_of(&world, entry.id));
        assert_eq!(Some(entry.layer), map_model::node::layer_of(&world, entry.id));
        assert_eq!(entry.node.id, entry.id);
    }
    let inner = groups_with_ids(&world).into_iter().find(|(_, g)| g.name == "Inner").unwrap().0;
    for open in [None, Some(inner)] {
        let (outermost, contents) = outermost_groups(&world, open);
        for id in all_node_ids(&world) {
            let expected = outermost_group_of(&world, id, open);
            assert_eq!(outermost.get(&id).copied(), expected, "outermost group of {id:?}");
            if let Some(group) = expected {
                let mut want = contents_of(&world, group);
                let mut got = contents[&group].clone();
                want.sort();
                got.sort();
                assert_eq!(got, want);
            }
        }
    }
}

#[test]
fn duplicate_parent_ids_keep_the_first_group_until_it_is_placed() {
    let tree = tree(r#"
{ "classname" "worldspawn" }
{ "classname" "func_group" "_tb_type" "_tb_group" "_tb_id" "7" "_tb_name" "First" }
{ "classname" "func_group" "_tb_type" "_tb_group" "_tb_id" "7" "_tb_name" "Second" "_tb_group" "7" }
{ "classname" "light" "_tb_group" "7" }
"#);
    assert!(tree.warnings.is_empty());
    assert_eq!(tree.world.layers[0].children.len(), 1);
    let NodeKind::Group(first) = &tree.world.layers[0].children[0].kind else {
        panic!("expected first group");
    };
    assert_eq!(first.name, "First");
    assert_eq!(first.children.len(), 2);
    assert!(matches!(first.children[0].kind, NodeKind::Entity(_)));
    let NodeKind::Group(second) = &first.children[1].kind else {
        panic!("expected second group inside first");
    };
    assert_eq!(second.name, "Second");
}

#[test]
fn a_removed_group_parent_still_reports_a_missing_parent() {
    let tree = tree(r#"
{ "classname" "worldspawn" }
{ "classname" "func_group" "_tb_type" "_tb_group" "_tb_id" "1" "_tb_group" "2" }
{ "classname" "func_group" "_tb_type" "_tb_group" "_tb_id" "2" }
"#);
    assert_eq!(tree.world.layers[0].children.len(), 2);
    assert!(matches!(tree.warnings.as_slice(), [TreeWarning::MissingParent { id, .. }] if id == "2"));
}

#[test]
fn consuming_map_moves_face_storage_and_preserves_invalid_brush_warnings() {
    let source = format!("{{ \"classname\" \"worldspawn\" {} }}", cube(-64, 64));
    let mut map = read_map(&source, None).unwrap();
    let original_faces = map.entities[0].brushes[0].faces.as_ptr();
    let invalid = map_model::io::map_reader::Brush {
        faces: Vec::new(), line: 99, id: None,
    };
    map.entities[0].brushes.push(invalid);
    let expected_warnings = build_tree(&map, &world_bounds()).warnings;
    let owned = map_model::node::build_tree_owned(map, &world_bounds());
    assert_eq!(owned.warnings, expected_warnings);
    let NodeKind::Brush(brush) = &owned.world.layers[0].children[0].kind else { panic!("expected brush"); };
    assert_eq!(brush.faces().as_ptr(), original_faces, "face allocation must move without cloning");
}

#[test]
fn consuming_tree_keeps_worldspawn_selection_and_warning_order() {
    use map_model::node::{build_tree_owned, flatten_tree};
    use map_model::io::map_writer::write_map;
    use map_model::map_format::MapFormat;
    let source = format!(r#"
{{ "classname" "light" "_tb_group" "missing" }}
{{ "classname" "worldspawn" {} }}
{{ "classname" "worldspawn" {} }}
{{ "classname" "func_group" "_tb_type" "_tb_group" "_tb_id" "7" }}
"#, cube(-64, 64), cube(100, 164));
    let mut map = read_map(&source, None).unwrap();
    for (index, entity) in map.entities.iter_mut().enumerate() {
        entity.id = Some(index as u64 + 1);
        for brush in &mut entity.brushes { brush.id = Some(index as u64 + 100); }
        entity.brushes.push(map_model::io::map_reader::Brush { line: index + 40, ..Default::default() });
    }
    let borrowed = build_tree(&map, &world_bounds());
    let owned = build_tree_owned(map, &world_bounds());
    assert_eq!(borrowed.warnings, owned.warnings);
    assert_eq!(write_map(&flatten_tree(&borrowed.world, MapFormat::LOSSLESS, None)),
        write_map(&flatten_tree(&owned.world, MapFormat::LOSSLESS, None)));
}
