//! What the editor writes, held still.
//!
//! Two people sharing a map need the file to be the same file: the same document has to
//! come out byte for byte the same, or every save churns the whole thing and no diff and
//! no merge is worth reading. These tests are what makes that a promise rather than an
//! accident. `fixtures/format.map` is a map the editor wrote, carrying one of everything
//! the format can say, and a change to the writer that alters a single byte of it fails
//! here and has to be looked at on purpose.
//!
//! Changing the format is allowed. Changing it without noticing is not.

use map_model::io::map_reader::read_map;
use map_model::io::map_writer::write_map;
use map_model::map_format::MapFormat;
use map_model::node::{build_tree, flatten_tree};
use geometry_math::bbox::Bbox3d;

const FIXTURE: &str = include_str!("fixtures/format.map");

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

/// Reads a map, puts it through the node tree the editor holds it in, and writes it back.
fn round_trip(source: &str) -> String {
    let map = read_map(source, None).expect("it reads");
    let tree = build_tree(&map, &world_bounds());
    write_map(&flatten_tree(&tree.world, MapFormat::LOSSLESS, map.game.clone()))
}

#[test]
fn the_fixture_is_what_the_writer_writes() {
    let written = round_trip(FIXTURE);
    if written != FIXTURE {
        // The whole file rather than the first difference: what changed is the point, and
        // a fixture worth updating is one a person has read.
        panic!(
            "the writer no longer produces `fixtures/format.map`.\n\
             If the change is meant, update the fixture and say why in the commit.\n\
             \n--- what it writes now ---\n{written}"
        );
    }
}

#[test]
fn writing_is_a_fixed_point() {
    // Not the same claim: a writer could be stable and still disagree with the fixture,
    // and one that agreed once could wander on the second pass.
    let once = round_trip(FIXTURE);
    assert_eq!(round_trip(&once), once);
}

/// Everything the fixture is there to hold still, named so that a fixture rebuilt from a
/// smaller map fails rather than quietly covering less.
#[test]
fn the_fixture_carries_one_of_everything() {
    let claims: [(&str, &str); 15] = [
        ("the version of the editor's own notes", "// heron_brush format 3"),
        ("an unresolved brush-primitive matrix", "primitive_uv 0.0078125 0 0.25 0 0.015625 -0.125"),
        ("a durable id", "// heron_brush id "),
        ("a Valve face, with its own UV axes", "[ 0 1 0 4 ] [ 0 0 -1 8 ]"),
        ("a Standard face, deriving them", "south 12 16 0 1 1"),
        ("the Quake 2 surface flag trio", "1 8 700"),
        ("a colour shared by every face of a brush", "\n// heron_brush color 210 90 40\n"),
        ("a gradient shared by every face", "\n// heron_brush gradient 20 40 90 200 220 255 35\n"),
        ("a Bezier patch", "patchDef2"),
        ("a patch control point, five numbers in one bracket", "( -64 0 32 0 0.5 )"),
        ("a layer", "\"_tb_type\" \"_tb_layer\""),
        ("a layer left out of exports", "\"_tb_layer_omit_from_export\" \"1\""),
        ("an entity on a layer", "\"_tb_layer\" \"7\""),
        ("a protected property", "\"_tb_protected_properties\" \"targetname\""),
        ("a point entity", "\"classname\" \"info_player_start\""),
    ];
    for (what, text) in claims {
        assert!(FIXTURE.contains(text), "the fixture no longer holds {what}");
    }
}

/// A patch control point is `( x y z u v )`, five numbers in one pair of brackets.
///
/// This reader is lenient enough to take the position in brackets of its own, so a map
/// written that way went out and came back unharmed and nothing here said otherwise --
/// while no other tool in the lineage would have read it at all. The shape of the line
/// is the part that matters to everyone else, so it is checked as a shape.
#[test]
fn a_patch_control_point_is_five_numbers_in_one_bracket() {
    let line = FIXTURE
        .lines()
        .find(|line| line.contains("-64 0 32"))
        .expect("the patch's middle row is in the fixture");
    for group in line.split(") (").map(|g| g.trim_matches([' ', '(', ')'])) {
        if group.is_empty() {
            continue;
        }
        assert_eq!(
            group.split_whitespace().count(),
            5,
            "`{group}` is not five numbers, in {line}"
        );
    }
}

/// A map from another editor is normalised once, and never moves again.
///
/// The one-time change is the point of having a dialect of our own: whatever a map was
/// written as, the first save puts it in the one that can hold everything. What matters
/// for two people sharing a repository is the second save and every save after it.
#[test]
fn a_foreign_map_settles_after_one_save() {
    let foreign = "\
{
\"classname\" \"worldspawn\"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) west 4 8 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) south 12 16 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) floor 20 24 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) ceiling 0 0 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) north 0 0 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) east 0 0 0 1 1
}
}
";
    let first = round_trip(foreign);
    assert_eq!(round_trip(&first), first, "the second save moves nothing");
}

/// An id names the block that opens after it, so it sits on the line before.
///
/// Checked as a shape rather than as text: both entities and brushes carry one, and a
/// claim that the file contains the words says nothing about which blocks have them.
#[test]
fn every_entity_and_brush_is_named_by_the_line_above_it() {
    let lines: Vec<&str> = FIXTURE.lines().collect();
    // The id is somewhere in the run of comments above the block, not necessarily on the
    // line immediately above it: a brush whose faces share a colour carries that on a
    // line of its own between the two.
    let named = |at: usize| {
        lines[..at]
            .iter()
            .rev()
            .take_while(|line| line.starts_with("//"))
            .any(|line| line.starts_with("// heron_brush id "))
    };

    let entities: Vec<usize> = (0..lines.len())
        .filter(|at| lines[*at].starts_with("// entity "))
        .collect();
    let brushes: Vec<usize> = (0..lines.len())
        .filter(|at| lines[*at].starts_with("// brush "))
        .collect();
    assert!(entities.len() >= 3 && brushes.len() >= 2, "the fixture shrank");

    for at in brushes {
        assert!(named(at), "the brush at line {} has no id", at + 1);
    }
    // Every entity but the two that already say what they are: the worldspawn, matched
    // as the one entity with that classname, and a layer or group, which carries the
    // `_tb_id` its members point back at.
    for at in entities {
        let head = &lines[at..(at + 6).min(lines.len())];
        let says_what_it_is = head
            .iter()
            .any(|line| line.contains("\"worldspawn\"") || line.contains("\"_tb_id\""));
        assert_eq!(
            named(at),
            !says_what_it_is,
            "the entity at line {} is named when it should not be, or the other way round",
            at + 1
        );
    }
}

/// A brush painted all over says so once, and a brush whose faces disagree says it per
/// face.
///
/// Painting in the editor paints what is selected, so a brush is normally uniform: on
/// the dragon in `assets/`, all ninety-nine of its brushes are, and none is mixed. One
/// comment per brush rather than one per face is what makes recolouring a brush a
/// one-line change in the file.
#[test]
fn a_uniform_colour_is_written_once_and_a_mixed_one_per_face() {
    let brush = |faces: &[&str]| {
        format!(
            "{{\n( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) a 0 0 0 1 1{}\n\
             ( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) a 0 0 0 1 1{}\n\
             ( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) a 0 0 0 1 1{}\n\
             ( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) a 0 0 0 1 1{}\n\
             ( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) a 0 0 0 1 1{}\n\
             ( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) a 0 0 0 1 1{}\n}}\n",
            faces[0], faces[1], faces[2], faces[3], faces[4], faces[5]
        )
    };
    let all = " // heron_brush color 10 20 30";
    let map = |body: String| format!("{{\n\"classname\" \"worldspawn\"\n{body}}}\n");

    let uniform = round_trip(&map(brush(&[all; 6])));
    assert_eq!(
        uniform.matches("heron_brush color").count(),
        1,
        "one comment for the whole brush:\n{uniform}"
    );

    let mixed = round_trip(&map(brush(&[all, all, all, all, all, " // heron_brush color 99 99 99"])));
    assert_eq!(
        mixed.matches("heron_brush color").count(),
        6,
        "one comment per face where they disagree:\n{mixed}"
    );

    // And either way the colours come back as what they were.
    let back = read_map(&mixed, None).expect("it reads");
    let faces = &back.entities[0].brushes[0].faces;
    assert_eq!(faces.iter().filter(|f| f.color.is_some()).count(), 6);
    assert_ne!(faces[0].color, faces[5].color, "the odd face kept its own");
}

/// A file written by a newer editor says so, and the reader says so back.
#[test]
fn a_map_from_a_newer_editor_is_reported_rather_than_quietly_read() {
    let from_the_future = format!(
        "// Format: Valve\n// heron_brush format {}\n{{\n\"classname\" \"worldspawn\"\n}}\n",
        map_model::map_format::NOTES_VERSION + 1
    );
    let map = read_map(&from_the_future, None).expect("it still reads");
    assert_eq!(map.notes_version, Some(map_model::map_format::NOTES_VERSION + 1));
    assert!(
        map.warnings.iter().any(|w| w.message.contains("newer Heron Brush")),
        "{:?}",
        map.warnings
    );

    // A file that says nothing is not from the future, and is not complained about.
    let plain = read_map("{\n\"classname\" \"worldspawn\"\n}\n", None).expect("it reads");
    assert_eq!(plain.notes_version, None);
    assert!(plain.warnings.is_empty(), "{:?}", plain.warnings);
}


#[test]
fn consuming_the_map_preserves_the_complete_frozen_format() {
    let map = read_map(FIXTURE, None).unwrap();
    let game = map.game.clone();
    let borrowed = build_tree(&map, &world_bounds());
    let owned = map_model::node::build_tree_owned(map, &world_bounds());
    assert_eq!(borrowed.warnings, owned.warnings);
    assert_eq!(write_map(&flatten_tree(&owned.world, MapFormat::LOSSLESS, game)), FIXTURE);
}
