//! Malformed hierarchy and truncated input regressions for map loading.
use geometry_math::bbox::Bbox3d;
use map_model::{
    io::{map_reader::read_map, map_writer::write_map},
    node,
};

fn hierarchy(count: usize, cycle: bool) -> String {
    let mut source = String::from("{\"classname\" \"worldspawn\"}\n");
    for i in 0..count {
        let parent = if i > 0 {
            Some(i - 1)
        } else if cycle {
            Some(count - 1)
        } else {
            None
        };
        let link = parent.map_or_else(String::new, |id| format!("\"_tb_group\" \"{id}\""));
        source.push_str(&format!("{{\"classname\" \"func_group\" \"_tb_type\" \"_tb_group\" \"_tb_id\" \"{i}\" \"_tb_name\" \"Group {i}\" {link}}}\n"));
    }
    source
}

#[test]
fn excessive_nesting_recovers_every_group_and_survives_save_clone_and_drop() {
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            for cycle in [false, true] {
                let mut source = hierarchy(15_000, cycle);
                source.push_str(
                    r#"{ "classname" "light" "_tb_group" "14999" "targetname" "keep-me" }"#,
                );
                let mut map = read_map(&source, None).unwrap();
                let fixture = read_map(include_str!("fixtures/fine_grid.map"), None).unwrap();
                map.entities[15_000].brushes = fixture.entities[0].brushes.clone();
                let brush_count = fixture.entities[0].brushes.len();
                assert!(brush_count > 0);
                let bounds = Bbox3d::from_scalars(-16384.0, 16384.0);
                for owned in [false, true] {
                    let tree = if owned {
                        node::build_tree_owned(map.clone(), &bounds)
                    } else {
                        node::build_tree(&map, &bounds)
                    };
                    assert!(
                        tree.warnings
                            .iter()
                            .any(|w| matches!(w, node::TreeWarning::GroupDepthLimit { .. }))
                    );
                    assert_eq!(node::brushes(&tree.world).len(), brush_count);
                    let clone = tree.world.clone();
                    let flattened = node::flatten_tree(&clone, map.format, None);
                    assert_eq!(flattened.entities.len(), 15_002, "recovery lost groups");
                    assert!(
                        flattened
                            .entities
                            .iter()
                            .any(|e| e.property("targetname") == Some("keep-me"))
                    );
                    let names: std::collections::HashSet<_> = flattened
                        .entities
                        .iter()
                        .filter_map(|e| e.property("_tb_name"))
                        .collect();
                    assert_eq!(names.len(), 15_000);
                    let saved = write_map(&flattened);
                    let reopened = node::build_tree_owned(read_map(&saved, None).unwrap(), &bounds);
                    assert!(
                        reopened.warnings.is_empty(),
                        "recovered hierarchy must be valid"
                    );
                    assert_eq!(
                        node::flatten_tree(&reopened.world, map.format, None)
                            .entities
                            .len(),
                        15_002
                    );
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn truncations_and_token_damage_do_not_panic_during_parse_build_and_save() {
    let source = include_str!("fixtures/fine_grid.map");
    let bounds = Bbox3d::from_scalars(-16384.0, 16384.0);
    let check = |text: &str| {
        if let Ok(map) = read_map(text, None) {
            let format = map.format;
            let tree = node::build_tree_owned(map, &bounds);
            let written = write_map(&node::flatten_tree(&tree.world, format, None));
            read_map(&written, None).expect("recovered input must save readable map text");
        }
    };
    for (offset, _) in source.char_indices().step_by(17) {
        check(&source[..offset]);
        for token in ["{", "\"", "/*", "NaN", "1e999", "\0", "ž"] {
            let mut damaged = source.to_string();
            damaged.insert_str(offset, token);
            check(&damaged);
        }
    }
}

#[test]
fn nesting_limit_only_recovers_the_first_unsafe_attachment() {
    let bounds = Bbox3d::from_scalars(-16384.0, 16384.0);
    for (count, roots, warnings) in [(128, 1, 0), (129, 2, 1)] {
        let map = read_map(&hierarchy(count, false), None).unwrap();
        let tree = node::build_tree_owned(map, &bounds);
        assert_eq!(tree.world.layers[0].children.len(), roots);
        assert_eq!(tree.warnings.len(), warnings);
    }
}
