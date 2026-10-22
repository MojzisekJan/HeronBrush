//! Whether a patch survives being read, held in the node tree and written back.

use geometry_math::bbox::Bbox3d;
use map_model::io::map_reader::read_map;
use map_model::io::map_writer::write_map;
use map_model::map_format::MapFormat;
use map_model::node::{build_tree, flatten_tree};

const WITH_A_PATCH: &str = r#"
{
"classname" "worldspawn"
{
patchDef2
{
common/caulk
( 3 3 0 0 0 )
(
( ( -64 -64 0 0 0 ) ( -64 0 32 0 0.5 ) ( -64 64 0 0 1 ) )
( ( 0 -64 32 0.5 0 ) ( 0 0 64 0.5 0.5 ) ( 0 64 32 0.5 1 ) )
( ( 64 -64 0 1 0 ) ( 64 0 32 1 0.5 ) ( 64 64 0 1 1 ) )
)
}
}
}
"#;

#[test]
fn a_patch_is_read_from_the_file() {
    let map = read_map(WITH_A_PATCH, None).expect("it reads");
    assert_eq!(map.entities.len(), 1);
    assert_eq!(map.entities[0].patches.len(), 1);
    assert_eq!(map.entities[0].patches[0].control_points.len(), 9);
}

#[test]
fn a_patch_survives_the_node_tree() {
    let map = read_map(WITH_A_PATCH, None).expect("it reads");
    let world_bounds = Bbox3d::from_scalars(-16384.0, 16384.0);
    let tree = build_tree(&map, &world_bounds);

    let flattened = flatten_tree(&tree.world, MapFormat::Valve, None);
    let patches: usize = flattened
        .entities
        .iter()
        .map(|entity| entity.patches.len())
        .sum();
    assert_eq!(patches, 1, "the patch was lost between reading and writing");
}

#[test]
fn a_patch_is_written_back() {
    let map = read_map(WITH_A_PATCH, None).expect("it reads");
    let world_bounds = Bbox3d::from_scalars(-16384.0, 16384.0);
    let tree = build_tree(&map, &world_bounds);
    let flattened = flatten_tree(&tree.world, MapFormat::Valve, None);

    let written = write_map(&flattened);
    assert!(
        written.contains("patchDef2"),
        "the patch is not in the output"
    );
    assert!(written.contains("common/caulk"));
}
