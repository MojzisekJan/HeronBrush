//! Tests for taking the document tree back apart into a map file.

use map_model::io::map_reader::read_map;
use map_model::io::map_writer::write_map;
use map_model::map_format::MapFormat;
use map_model::node::{self, NodeKind};
use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

fn boxed(min: (i32, i32, i32), max: (i32, i32, i32), material: &str) -> String {
    let (x0, y0, z0) = min;
    let (x1, y1, z1) = max;
    format!(
        "{{\n\
         ( {x0} {y0} {z0} ) ( {x0} {y1} {z0} ) ( {x0} {y0} {z1} ) {material} 0 0 0 1 1\n\
         ( {x0} {y0} {z0} ) ( {x0} {y0} {z1} ) ( {x1} {y0} {z0} ) {material} 0 0 0 1 1\n\
         ( {x0} {y0} {z0} ) ( {x1} {y0} {z0} ) ( {x0} {y1} {z0} ) {material} 0 0 0 1 1\n\
         ( {x1} {y1} {z1} ) ( {x1} {y0} {z1} ) ( {x0} {y1} {z1} ) {material} 0 0 0 1 1\n\
         ( {x1} {y1} {z1} ) ( {x0} {y1} {z1} ) ( {x1} {y1} {z0} ) {material} 0 0 0 1 1\n\
         ( {x1} {y1} {z1} ) ( {x1} {y1} {z0} ) ( {x1} {y0} {z1} ) {material} 0 0 0 1 1\n\
         }}\n"
    )
}

/// Reads a map, builds the tree, takes it apart again and reads the result.
fn round_trip(source: &str) -> (map_model::node::Tree, map_model::node::Tree) {
    let map = read_map(source, None).expect("the sample parses");
    let before = node::build_tree(&map, &world_bounds());

    let flattened = node::flatten_tree(&before.world, map.format, map.game.clone());
    let text = write_map(&flattened);
    let reread = read_map(&text, None).expect("what we wrote parses");
    let after = node::build_tree(&reread, &world_bounds());

    (before, after)
}

#[test]
fn a_worldspawn_with_brushes_round_trips() {
    let source = format!(
        "// Format: Standard\n{{\n\"classname\" \"worldspawn\"\n\"wad\" \"x.wad\"\n{}{}}}\n",
        boxed((-64, -64, -16), (64, 64, 16), "floor"),
        boxed((-16, -16, 16), (16, 16, 80), "pillar")
    );
    let (before, after) = round_trip(&source);

    assert_eq!(node::brushes(&after.world).len(), 2);
    assert!(after.warnings.is_empty(), "{:?}", after.warnings);

    // The bounds survive.
    for (a, b) in node::brushes(&before.world)
        .iter()
        .zip(node::brushes(&after.world))
    {
        geometry_math::assert_approx_eq!(a.bounds().min, b.bounds().min, 1e-6);
        geometry_math::assert_approx_eq!(a.bounds().max, b.bounds().max, 1e-6);
    }

    // And so do the game's own properties.
    assert!(after
        .world
        .properties
        .iter()
        .any(|(k, v)| k == "wad" && v == "x.wad"));
}

#[test]
fn a_layer_round_trips_with_its_settings_and_contents() {
    let source = format!(
        "// Format: Standard\n{{\n\"classname\" \"worldspawn\"\n}}\n\
         {{\n\
         \"classname\" \"func_group\"\n\
         \"_tb_type\" \"_tb_layer\"\n\
         \"_tb_name\" \"Detail\"\n\
         \"_tb_id\" \"2\"\n\
         \"_tb_layer_sort_index\" \"3\"\n\
         \"_tb_layer_locked\" \"1\"\n\
         {}}}\n",
        boxed((0, 0, 0), (32, 32, 32), "trim")
    );
    let (_, after) = round_trip(&source);

    assert_eq!(after.world.layers.len(), 2);
    let detail = &after.world.layers[1];
    assert_eq!(detail.name, "Detail");
    assert_eq!(detail.id.as_deref(), Some("2"));
    assert_eq!(detail.sort_index, 3);
    assert!(detail.locked);
    assert!(!detail.hidden);
    assert_eq!(detail.children.len(), 1);
}

#[test]
fn a_group_round_trips_inside_its_layer() {
    let source = "// Format: Standard\n{\n\"classname\" \"worldspawn\"\n}\n\
                  {\n\"classname\" \"func_group\"\n\"_tb_type\" \"_tb_layer\"\n\"_tb_name\" \"Detail\"\n\"_tb_id\" \"2\"\n}\n\
                  {\n\"classname\" \"func_group\"\n\"_tb_type\" \"_tb_group\"\n\"_tb_name\" \"Door\"\n\"_tb_id\" \"7\"\n\"_tb_layer\" \"2\"\n}\n\
                  {\n\"classname\" \"light\"\n\"_tb_group\" \"7\"\n\"light\" \"300\"\n}\n";
    let (_, after) = round_trip(source);

    let detail = &after.world.layers[1];
    let NodeKind::Group(group) = &detail.children[0].kind else {
        panic!("expected a group in the layer, got {:?}", detail.children);
    };
    assert_eq!(group.name, "Door");
    assert_eq!(group.id, "7");

    let NodeKind::Entity(light) = &group.children[0].kind else {
        panic!("expected the light inside the group");
    };
    assert_eq!(light.classname(), Some("light"));
    assert_eq!(light.property("light"), Some("300"));
}

#[test]
fn a_point_entity_in_the_default_layer_names_no_parent() {
    let source = "// Format: Standard\n{\n\"classname\" \"worldspawn\"\n}\n\
                  {\n\"classname\" \"info_player_start\"\n\"origin\" \"0 64 24\"\n}\n";
    let map = read_map(source, None).expect("parses");
    let tree = node::build_tree(&map, &world_bounds());
    let flattened = node::flatten_tree(&tree.world, map.format, None);

    let spawn = flattened
        .entities
        .iter()
        .find(|e| e.classname() == Some("info_player_start"))
        .expect("the spawn was written");
    // Nothing to name: the worldspawn is the default layer.
    assert_eq!(spawn.property("_tb_layer"), None);
    assert_eq!(spawn.property("origin"), Some("0 64 24"));
}

#[test]
fn a_brush_entity_keeps_its_brushes() {
    let source = format!(
        "// Format: Standard\n{{\n\"classname\" \"worldspawn\"\n}}\n\
         {{\n\"classname\" \"func_door\"\n\"speed\" \"100\"\n{}}}\n",
        boxed((0, 0, 0), (32, 32, 96), "door")
    );
    let (_, after) = round_trip(&source);

    let NodeKind::Entity(door) = &after.world.layers[0].children[0].kind else {
        panic!("expected the door entity");
    };
    assert_eq!(door.classname(), Some("func_door"));
    assert_eq!(door.property("speed"), Some("100"));
    assert_eq!(door.children.len(), 1);
}

#[test]
fn the_map_format_is_preserved() {
    let source = format!(
        "// Game: heron_brush-fps\n// Format: Valve\n{{\n\"mapversion\" \"220\"\n\"classname\" \"worldspawn\"\n{}}}\n",
        // A Valve 220 face line, with its UV axes written out.
        "{\n\
         ( 0 0 0 ) ( 0 1 0 ) ( 0 0 1 ) w [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1\n\
         ( 0 0 0 ) ( 0 0 1 ) ( 1 0 0 ) w [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1\n\
         ( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) w [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n\
         ( 32 32 32 ) ( 32 0 32 ) ( 0 32 32 ) w [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1\n\
         ( 32 32 32 ) ( 0 32 32 ) ( 32 32 0 ) w [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1\n\
         ( 32 32 32 ) ( 32 32 0 ) ( 32 0 32 ) w [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1\n\
         }\n"
    );
    let map = read_map(&source, None).expect("parses");
    assert_eq!(map.format, MapFormat::Valve);

    let flattened = node::flatten_tree(&map_tree(&map).world, map.format, map.game.clone());
    let text = write_map(&flattened);

    assert!(text.contains("// Format: Valve"), "{text}");
    assert!(text.contains("// Game: heron_brush-fps"), "{text}");
    // The UV axes are written back out in brackets, not collapsed to Standard offsets.
    assert!(text.contains("[ 0 1 0 0 ]"), "{text}");

    let reread = read_map(&text, None).expect("what we wrote parses");
    assert_eq!(reread.format, MapFormat::Valve);
}

fn map_tree(map: &map_model::io::map_reader::MapFile) -> map_model::node::Tree {
    node::build_tree(map, &world_bounds())
}

#[test]
fn an_edited_brush_is_written_where_it_now_is() {
    let source = format!(
        "// Format: Standard\n{{\n\"classname\" \"worldspawn\"\n{}}}\n",
        boxed((-16, -16, -16), (16, 16, 16), "stone")
    );
    let map = read_map(&source, None).expect("parses");
    let mut tree = node::build_tree(&map, &world_bounds());

    let (id, brush) = node::brushes_with_ids(&tree.world)[0];
    let moved = brush
        .translate(&world_bounds(), Vec3d::new(0.0, 0.0, 128.0))
        .expect("a translated box is still a box");
    assert!(node::replace_brush(&mut tree.world, id, moved));

    let text = write_map(&node::flatten_tree(&tree.world, map.format, None));
    let reread = node::build_tree(
        &read_map(&text, None).expect("what we wrote parses"),
        &world_bounds(),
    );

    let bounds = node::brushes(&reread.world)[0].bounds();
    geometry_math::assert_approx_eq!(bounds.min, Vec3d::new(-16.0, -16.0, 112.0), 1e-6);
    geometry_math::assert_approx_eq!(bounds.max, Vec3d::new(16.0, 16.0, 144.0), 1e-6);
}

#[test]
fn a_layer_with_no_id_is_given_one() {
    use map_model::node::{Layer, World};

    let world = World {
        properties: Vec::new(),
        layers: vec![
            Layer {
                id: None,
                name: "Default Layer".to_string(),
                sort_index: 0,
                hidden: false,
                locked: false,
                omit_from_export: false,
                color: None,
                children: Vec::new(),
            },
            // Made by the editor rather than read from a file, so it has no id yet.
            Layer {
                id: None,
                name: "New Layer".to_string(),
                sort_index: 1,
                hidden: false,
                locked: false,
                omit_from_export: false,
                color: None,
                children: Vec::new(),
            },
        ],
    };

    let flattened = node::flatten_tree(&world, MapFormat::Standard, None);
    let layer = flattened
        .entities
        .iter()
        .find(|e| e.property("_tb_name") == Some("New Layer"))
        .expect("the layer was written");
    assert!(layer.property("_tb_id").is_some(), "it needs an id to be found by");
}
