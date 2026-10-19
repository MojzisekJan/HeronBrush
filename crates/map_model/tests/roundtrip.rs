//! What survives being read, held in the node tree and written back.
//!
//! Every claim here was written as a test rather than assumed. A patch used to be read
//! and written by code that never met in the middle, so opening a map and saving it
//! destroyed it; these check the rest of the same path.

use map_model::io::map_reader::read_map;
use map_model::io::map_writer::write_map;
use map_model::map_format::MapFormat;
use map_model::node::{build_tree, flatten_tree};
use geometry_math::bbox::Bbox3d;

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

/// Reads a map, puts it through the node tree, and writes it back.
fn round_trip(source: &str, format: MapFormat) -> String {
    let map = read_map(source, None).expect("it reads");
    let tree = build_tree(&map, &world_bounds());
    let flattened = flatten_tree(&tree.world, format, None);
    write_map(&flattened)
}

/// Reads a map twice, so the second reading can be compared with the first.
fn read_twice(source: &str, format: MapFormat) -> (map_model::io::map_reader::MapFile, map_model::io::map_reader::MapFile) {
    let first = read_map(source, None).expect("it reads");
    let written = round_trip(source, format);
    let second = read_map(&written, None)
        .unwrap_or_else(|error| panic!("what was written does not read back: {error}\n{written}"));
    (first, second)
}

const A_BOX: &str = r#"
{
"classname" "worldspawn"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone 0 0 0 1 1
}
}
"#;

#[test]
fn a_brush_survives() {
    let (first, second) = read_twice(A_BOX, MapFormat::Standard);
    assert_eq!(first.entities[0].brushes.len(), 1);
    assert_eq!(second.entities[0].brushes.len(), 1);
    assert_eq!(
        second.entities[0].brushes[0].faces.len(),
        first.entities[0].brushes[0].faces.len()
    );
}

#[test]
fn a_material_name_survives() {
    let (_, second) = read_twice(A_BOX, MapFormat::Standard);
    for face in &second.entities[0].brushes[0].faces {
        assert_eq!(face.material_name, "stone");
    }
}

#[test]
fn an_entity_keeps_its_properties_in_order() {
    // The format keeps duplicates and order, which is what a game reading top to bottom
    // depends on.
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "light"
"origin" "0 0 64"
"light" "300"
"_color" "1 0.9 0.8"
}
"#;
    let (first, second) = read_twice(source, MapFormat::Standard);
    let before: Vec<_> = first.entities[1].properties.clone();
    let after: Vec<_> = second.entities[1].properties.clone();
    assert_eq!(before, after);
}

#[test]
fn a_brush_entity_keeps_its_brushes() {
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_door"
"targetname" "the_door"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone 0 0 0 1 1
}
}
"#;

    let (_, second) = read_twice(source, MapFormat::Standard);
    let door = second
        .entities
        .iter()
        .find(|entity| entity.classname() == Some("func_door"))
        .expect("the door survived");
    assert_eq!(door.brushes.len(), 1);
    assert_eq!(door.property("targetname"), Some("the_door"));
}

#[test]
fn layers_survive_with_their_names_and_flags() {
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_group"
"_tb_type" "_tb_layer"
"_tb_name" "Detail"
"_tb_id" "2"
"_tb_layer_sort_index" "1"
"_tb_layer_locked" "1"
"_tb_layer_hidden" "1"
"_tb_layer_omit_from_export" "1"
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);
    let layer = second
        .entities
        .iter()
        .find(|entity| entity.property("_tb_type") == Some("_tb_layer"))
        .expect("the layer survived");

    assert_eq!(layer.property("_tb_name"), Some("Detail"));
    assert_eq!(layer.property("_tb_layer_locked"), Some("1"));
    assert_eq!(layer.property("_tb_layer_hidden"), Some("1"));
    assert_eq!(layer.property("_tb_layer_omit_from_export"), Some("1"));
}

#[test]
fn a_group_survives_with_its_name() {
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_group"
"_tb_type" "_tb_group"
"_tb_name" "Doorway"
"_tb_id" "3"
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);
    let group = second
        .entities
        .iter()
        .find(|entity| entity.property("_tb_type") == Some("_tb_group"))
        .expect("the group survived");
    assert_eq!(group.property("_tb_name"), Some("Doorway"));
}

#[test]
fn a_valve_face_keeps_its_axes() {
    let source = r#"
{
"classname" "worldspawn"
"mapversion" "220"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone [ 0 1 0 8 ] [ 0 0 -1 16 ] 30 2 3
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}
}
"#;
    let (first, second) = read_twice(source, MapFormat::Valve);

    let before = &first.entities[0].brushes[0].faces;
    let after = &second.entities[0].brushes[0].faces;
    assert_eq!(before.len(), after.len());

    // The offsets, rotation and scale of the first face come back as they went in.
    let find = |faces: &[map_model::brush_face::BrushFace], normal_axis: usize| {
        faces
            .iter()
            .find(|face| face.plane.normal[normal_axis].abs() > 0.9)
            .cloned()
            .expect("a face facing that way")
    };
    let was = find(before, 0);
    let is = find(after, 0);
    geometry_math::assert_approx_eq!(is.uv_attributes.offset, was.uv_attributes.offset, 1e-4);
    geometry_math::assert_approx_eq!(is.uv_attributes.scale, was.uv_attributes.scale, 1e-4);
    geometry_math::assert_approx_eq!(is.uv_attributes.rotation, was.uv_attributes.rotation, 1e-3);
}

#[test]
fn surface_flags_survive() {
    // The Quake 2 and 3 trio after the scale: contents, flags, value.
    let source = r#"
{
"classname" "worldspawn"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone 0 0 0 1 1 1 2 3
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone 0 0 0 1 1 1 2 3
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone 0 0 0 1 1 1 2 3
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone 0 0 0 1 1 1 2 3
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone 0 0 0 1 1 1 2 3
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone 0 0 0 1 1 1 2 3
}
}
"#;
    let (first, second) = read_twice(source, MapFormat::Standard);

    let was = first.entities[0].brushes[0].faces[0].surface_attributes;
    let is = second.entities[0].brushes[0].faces[0].surface_attributes;
    assert!(was.is_some(), "the reader did not pick up the flags");
    assert_eq!(was, is, "the flags were lost on the way out");
}

#[test]
fn the_worldspawns_own_properties_survive() {
    let source = r#"
{
"classname" "worldspawn"
"message" "A test map"
"wad" "textures.wad"
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);
    let world = &second.entities[0];
    assert_eq!(world.property("message"), Some("A test map"));
    assert_eq!(world.property("wad"), Some("textures.wad"));
}

#[test]
fn an_entity_with_no_brushes_and_no_origin_survives() {
    // A class with neither is still an entity the game will read.
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "worldspawn_settings"
"gravity" "800"
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);
    assert!(second
        .entities
        .iter()
        .any(|entity| entity.property("gravity") == Some("800")));
}

#[test]
fn a_group_inside_a_layer_stays_inside_it() {
    // The structural case most likely to be got wrong: `_tb_layer` on a group says which
    // layer it belongs to, and losing that would move everything to the default layer.
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_group"
"_tb_type" "_tb_layer"
"_tb_name" "Detail"
"_tb_id" "2"
"_tb_layer_sort_index" "1"
}
{
"classname" "func_group"
"_tb_type" "_tb_group"
"_tb_name" "Doorway"
"_tb_id" "3"
"_tb_layer" "2"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone 0 0 0 1 1
}
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);

    let group = second
        .entities
        .iter()
        .find(|entity| entity.property("_tb_type") == Some("_tb_group"))
        .expect("the group survived");
    assert_eq!(group.brushes.len(), 1, "the group kept its brush");

    // And it is still on the layer it was on, rather than having fallen back to the
    // default one.
    let layer_id = second
        .entities
        .iter()
        .find(|entity| entity.property("_tb_name") == Some("Detail"))
        .and_then(|layer| layer.property("_tb_id"))
        .expect("the layer survived");
    assert_eq!(group.property("_tb_layer"), Some(layer_id));
}

#[test]
fn a_group_inside_a_group_stays_inside_it() {
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_group"
"_tb_type" "_tb_group"
"_tb_name" "Outer"
"_tb_id" "2"
}
{
"classname" "func_group"
"_tb_type" "_tb_group"
"_tb_name" "Inner"
"_tb_id" "3"
"_tb_group" "2"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone 0 0 0 1 1
}
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);

    let outer_id = second
        .entities
        .iter()
        .find(|entity| entity.property("_tb_name") == Some("Outer"))
        .and_then(|group| group.property("_tb_id"))
        .expect("the outer group survived");
    let inner = second
        .entities
        .iter()
        .find(|entity| entity.property("_tb_name") == Some("Inner"))
        .expect("the inner group survived");

    assert_eq!(inner.property("_tb_group"), Some(outer_id));
    assert_eq!(inner.brushes.len(), 1);
}

#[test]
fn an_entity_inside_a_group_stays_inside_it() {
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_group"
"_tb_type" "_tb_group"
"_tb_name" "Doorway"
"_tb_id" "2"
}
{
"classname" "light"
"origin" "0 0 64"
"_tb_group" "2"
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);

    let group_id = second
        .entities
        .iter()
        .find(|entity| entity.property("_tb_name") == Some("Doorway"))
        .and_then(|group| group.property("_tb_id"))
        .expect("the group survived");
    let light = second
        .entities
        .iter()
        .find(|entity| entity.classname() == Some("light"))
        .expect("the light survived");

    assert_eq!(light.property("_tb_group"), Some(group_id));
    assert_eq!(light.property("origin"), Some("0 0 64"));
}

#[test]
fn a_linked_groups_transformation_survives() {
    // What tells one copy of a linked group how it sits relative to the others. Losing it
    // would silently move every linked copy back onto the original.
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_group"
"_tb_type" "_tb_group"
"_tb_name" "group"
"_tb_id" "1"
"_tb_linked_group_id" "{38b3b39d-a165-4999-985d-d40563ce51c1}"
"_tb_transformation" "1 0 0 128 0 1 0 0 0 0 1 0 0 0 0 1"
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);
    let group = second
        .entities
        .iter()
        .find(|entity| entity.property("_tb_type") == Some("_tb_group"))
        .expect("the group survived");

    assert_eq!(
        group.property("_tb_transformation"),
        Some("1 0 0 128 0 1 0 0 0 0 1 0 0 0 0 1")
    );
    assert_eq!(
        group.property("_tb_linked_group_id"),
        Some("{38b3b39d-a165-4999-985d-d40563ce51c1}")
    );
}

#[test]
fn the_worldspawns_editor_properties_survive() {
    // The material collections, the definition file and the soft bounds are written on
    // the worldspawn. They are the editor's own, but they belong to the map.
    let source = r#"
{
"classname" "worldspawn"
"_tb_textures" "textures/base;textures/trim"
"_tb_def" "external:/home/someone/game.fgd"
"_tb_soft_map_bounds" "-4096 -4096 -4096 4096 4096 4096"
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);
    let world = &second.entities[0];

    assert_eq!(
        world.property("_tb_textures"),
        Some("textures/base;textures/trim")
    );
    assert_eq!(world.property("_tb_def"), Some("external:/home/someone/game.fgd"));
    assert_eq!(
        world.property("_tb_soft_map_bounds"),
        Some("-4096 -4096 -4096 4096 4096 4096")
    );
}

#[test]
fn a_layer_keeps_its_colour() {
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_group"
"_tb_type" "_tb_layer"
"_tb_name" "Detail"
"_tb_id" "2"
"_tb_layer_sort_index" "0"
"_tb_layer_color" "0.5 0.25 0.75 1"
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);
    let layer = second
        .entities
        .iter()
        .find(|entity| entity.property("_tb_name") == Some("Detail"))
        .expect("the layer survived");

    assert_eq!(layer.property("_tb_layer_color"), Some("0.5 0.25 0.75 1"));
}

#[test]
fn an_entitys_protected_properties_survive() {
    // Inside a linked group, this says which of an entity's properties are its own rather
    // than copied from the other members. Losing it would let the next link update
    // overwrite them.
    let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_group"
"_tb_type" "_tb_group"
"_tb_name" "group"
"_tb_id" "1"
}
{
"classname" "monster_army"
"origin" "0 0 0"
"angle" "90"
"_tb_group" "1"
"_tb_protected_properties" "angle"
}
"#;
    let (_, second) = read_twice(source, MapFormat::Standard);
    let monster = second
        .entities
        .iter()
        .find(|entity| entity.classname() == Some("monster_army"))
        .expect("the entity survived");

    assert_eq!(monster.property("_tb_protected_properties"), Some("angle"));
}

/// A cube with a colour on its first face and a gradient on its second.
const COLOURED: &str = "\
{
\"classname\" \"worldspawn\"
{
( -16 -16 -16 ) ( -16 16 -16 ) ( -16 -16 16 ) stone 0 0 0 1 1 // heron_brush color 220 40 40
( -16 -16 -16 ) ( -16 -16 16 ) ( 16 -16 -16 ) stone 0 0 0 1 1 // heron_brush gradient 0 0 0 255 255 255 45
( -16 -16 -16 ) ( 16 -16 -16 ) ( -16 16 -16 ) stone 0 0 0 1 1
( 16 16 16 ) ( 16 -16 16 ) ( -16 16 16 ) stone 0 0 0 1 1
( 16 16 16 ) ( -16 16 16 ) ( 16 16 -16 ) stone 0 0 0 1 1
( 16 16 16 ) ( 16 16 -16 ) ( 16 -16 16 ) stone 0 0 0 1 1
}
}
";

#[test]
fn a_face_colour_survives_being_read_and_written() {
    let map = read_map(COLOURED, None).expect("it reads");
    let faces = &map.entities[0].brushes[0].faces;
    assert_eq!(
        faces[0].paint(),
        Some(map_model::brush_face::FaceColor::solid(map_model::brush_face::Rgb::new(
            220, 40, 40
        )))
    );
    assert_eq!(
        faces[1].paint(),
        Some(map_model::brush_face::FaceColor::gradient(
            map_model::brush_face::Rgb::new(0, 0, 0),
            map_model::brush_face::Rgb::new(255, 255, 255),
            45.0,
        ))
    );
    assert_eq!(faces[2].color, None, "a face with no comment has no colour");

    let written = round_trip(COLOURED, MapFormat::Standard);
    assert!(written.contains("// heron_brush color 220 40 40"), "{written}");
    assert!(
        written.contains("// heron_brush gradient 0 0 0 255 255 255 45"),
        "{written}"
    );

    // And reading what was written gives the same colours again.
    let again = read_map(&written, None).expect("it reads back");
    assert_eq!(
        again.entities[0].brushes[0].faces[0].color,
        faces[0].color,
        "the colour is the same after a round trip"
    );
}

#[test]
fn a_see_through_face_colour_survives_a_round_trip() {
    use map_model::brush_face::{FaceColor, Rgb};

    let source = COLOURED
        .replace(
            "// heron_brush color 220 40 40",
            "// heron_brush color 220 40 40 128",
        )
        .replace(
            "// heron_brush gradient 0 0 0 255 255 255 45",
            "// heron_brush gradient 0 0 0 255 255 255 45 64",
        );

    let map = read_map(&source, None).expect("it reads");
    let faces = &map.entities[0].brushes[0].faces;
    assert_eq!(faces[0].paint(), Some(FaceColor::solid(Rgb::new(220, 40, 40)).with_alpha(128)));
    assert_eq!(
        faces[1].paint(),
        Some(FaceColor::gradient(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255), 45.0).with_alpha(64)),
    );

    let written = round_trip(&source, MapFormat::Standard);
    assert!(written.contains("// heron_brush color 220 40 40 128"), "{written}");
    assert!(written.contains("// heron_brush gradient 0 0 0 255 255 255 45 64"), "{written}");

    // An opaque colour still writes with no trailing number.
    let opaque = round_trip(COLOURED, MapFormat::Standard);
    assert!(opaque.contains("// heron_brush color 220 40 40\n") || opaque.contains("// heron_brush color 220 40 40 "));
    assert!(!opaque.contains("// heron_brush color 220 40 40 255"));
}

#[test]
fn a_comment_that_is_not_a_colour_is_left_alone() {
    let source = COLOURED.replace("// heron_brush color 220 40 40", "// somebody's note");
    let map = read_map(&source, None).expect("it reads");
    assert_eq!(map.entities[0].brushes[0].faces[0].color, None);
}

/// One brush whose six faces are split between the two UV coordinate systems: three
/// written the Standard way, three with explicit Valve axes.
const MIXED_UV: &str = r#"
{
"classname" "worldspawn"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) west 4 8 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) south 12 16 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) floor 20 24 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) ceiling [ 1 0 0 3 ] [ 0 -1 0 5 ] 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) north [ 0 1 0 7 ] [ 0 0 -1 9 ] 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) east [ 0 1 0 11 ] [ 0 0 -1 13 ] 0 1 1
}
}
"#;

/// The Valve dialect is a container for both UV systems at once, which is what lets a
/// map be edited without picking a flavour first.
///
/// The reader decides per face, by whether the line has `[ ... ]` axes, and the writer
/// asks the face rather than the format. So a face that derives its axes from its
/// dominant plane and one that carries its own can sit in the same brush and both come
/// back as what they were. Nothing about this is true of the Standard dialect, which has
/// nowhere to put the axes.
#[test]
fn valve_carries_both_uv_systems_and_neither_face_changes() {
    use map_model::uv::UvCoordSystem;

    let once = round_trip(MIXED_UV, MapFormat::Valve);
    let map = read_map(&once, None).expect("what was written reads back");
    let brush = &map.entities[0].brushes[0];

    let paraxial: Vec<&str> = brush
        .faces
        .iter()
        .filter(|face| matches!(face.uv_coord_system, UvCoordSystem::Paraxial))
        .map(|face| face.material_name.as_str())
        .collect();
    assert_eq!(paraxial, ["west", "south", "floor"], "in {once}");

    let parallel: Vec<&str> = brush
        .faces
        .iter()
        .filter(|face| matches!(face.uv_coord_system, UvCoordSystem::Parallel { .. }))
        .map(|face| face.material_name.as_str())
        .collect();
    assert_eq!(parallel, ["ceiling", "north", "east"], "in {once}");

    // And it is stable: a second trip changes nothing, so a map can be opened and saved
    // any number of times without drifting towards one system or the other.
    assert_eq!(round_trip(&once, MapFormat::Valve), once);
}

/// The Standard dialect has no axes on a face line, so a face that carries its own
/// loses them. This is why it cannot be what the editor works in.
#[test]
fn standard_drops_the_axes_a_valve_face_carries() {
    use map_model::uv::UvCoordSystem;

    let once = round_trip(MIXED_UV, MapFormat::Standard);
    let map = read_map(&once, None).expect("what was written reads back");
    let brush = &map.entities[0].brushes[0];

    assert!(
        brush
            .faces
            .iter()
            .all(|face| matches!(face.uv_coord_system, UvCoordSystem::Paraxial)),
        "every face came back paraxial, in {once}"
    );
}

/// A Quake 2 map's surface flags ride along in the lossless dialect too.
///
/// The trio is written whenever a face has one and read wherever three numbers turn up,
/// neither of them gated on the dialect, so Valve 220 holds the flags a Quake 2 map
/// carries as well as the axes a Valve one does. That is the whole of what makes it the
/// dialect the editor can work in: there is nothing left that only another one can say.
#[test]
fn the_lossless_dialect_carries_a_quake_2_maps_flags() {
    let source = r#"
{
"classname" "worldspawn"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) west 0 0 0 1 1 1 8 700
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) south 0 0 0 1 1 2 16 800
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) floor 0 0 0 1 1 0 0 0
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) ceiling 0 0 0 1 1 0 0 0
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) north 0 0 0 1 1 0 0 0
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) east 0 0 0 1 1 0 0 0
}
}
"#;

    let written = round_trip(source, MapFormat::LOSSLESS);
    let map = read_map(&written, None).expect("what was written reads back");
    let faces = &map.entities[0].brushes[0].faces;

    let flags: Vec<(i32, i32, i32)> = faces
        .iter()
        .filter_map(|face| face.surface_attributes.as_ref())
        .map(|s| (s.content_flags, s.surface_flags, s.value))
        .collect();
    assert_eq!(
        flags,
        [(1, 8, 700), (2, 16, 800), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0)],
        "in {written}"
    );
}


#[test]
fn multicolor_and_radial_gradient_comments_round_trip() {
    for kind in ["linear", "radial"] {
        let comment = format!("heron_brush {kind} 37.5 3 0 255 0 0 0.35 0 255 0 1 0 0 255 123");
        let source = COLOURED.replace("heron_brush gradient 0 0 0 255 255 255 45", &comment);
        let (before, after) = read_twice(&source, MapFormat::Standard);
        assert!(matches!(before.entities[0].brushes[0].faces[1].paint(), Some(map_model::brush_face::FaceColor::MultiGradient { .. })));
        assert_eq!(before.entities[0].brushes[0].faces[1].color, after.entities[0].brushes[0].faces[1].color);
        assert!(round_trip(&source, MapFormat::Standard).contains(&format!("heron_brush {kind} ")));
    }
}

#[test]
fn malformed_multicolor_gradient_comments_are_ignored() {
    for bad in ["heron_brush linear 0 999", "heron_brush radial NaN 2 0 1 2 3 1 4 5 6", "heron_brush linear 0 2 -1 1 2 3 1 4 5 6", "heron_brush linear 0 2 0 1 2 3 1 4 5 6 999"] {
        let source = COLOURED.replace("heron_brush gradient 0 0 0 255 255 255 45", bad);
        assert_eq!(read_map(&source,None).unwrap().entities[0].brushes[0].faces[1].color,None);
    }
}
