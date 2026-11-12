//! Scenarios for `map_model::merge`, the structural three-way `.map` merge.

use map_model::io::map_reader::{read_map, Entity, MapFile};
use map_model::map_format::MapFormat;
use map_model::merge::{merge, merge_text, EntityKey};

fn m(s: &str) -> MapFile {
    read_map(s, None).unwrap_or_else(|e| panic!("fixture does not read: {e}\n{s}"))
}

const BRUSH_A: &str = "\
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone 0 0 0 1 1
}
";

const BRUSH_B: &str = "\
{
( 0 0 0 ) ( 0 1 0 ) ( 0 0 1 ) metal 0 0 0 1 1
( 0 0 0 ) ( 0 0 1 ) ( 1 0 0 ) metal 0 0 0 1 1
( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) metal 0 0 0 1 1
( 32 32 32 ) ( 32 33 32 ) ( 33 32 32 ) metal 0 0 0 1 1
( 32 32 32 ) ( 33 32 32 ) ( 32 32 33 ) metal 0 0 0 1 1
( 32 32 32 ) ( 32 32 33 ) ( 32 33 32 ) metal 0 0 0 1 1
}
";

fn world(props: &str, brushes: &str) -> String {
    format!("{{\n\"classname\" \"worldspawn\"\n{props}{brushes}}}\n")
}

fn doc(entities: &[&str]) -> String {
    entities.iter().copied().collect()
}

fn merged_world(map: &MapFile) -> &Entity {
    &map.entities[0]
}

fn prop<'e>(e: &'e Entity, key: &str) -> Option<&'e str> {
    e.property(key)
}

#[test]
fn disjoint_additions_merge_cleanly() {
    let base = m(&doc(&[&world("", BRUSH_A)]));
    // ours adds a point entity
    let ours = m(&doc(&[
        &world("", BRUSH_A),
        "{\n\"classname\" \"light\"\n\"origin\" \"0 0 64\"\n}\n",
    ]));
    // theirs adds a second brush to worldspawn
    let theirs = m(&doc(&[&world("", &format!("{BRUSH_A}{BRUSH_B}"))]));

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(out.map.entities.len(), 2);
    assert_eq!(merged_world(&out.map).brushes.len(), 2);
    assert_eq!(out.map.entities[1].classname(), Some("light"));
}

#[test]
fn both_sides_add_the_same_entity_once() {
    let base = m(&doc(&[&world("", BRUSH_A)]));
    let light = "{\n\"classname\" \"light\"\n\"origin\" \"0 0 64\"\n}\n";
    let ours = m(&doc(&[&world("", BRUSH_A), light]));
    let theirs = m(&doc(&[&world("", BRUSH_A), light]));

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(out.map.entities.len(), 2);
}

#[test]
fn a_one_sided_property_edit_is_kept() {
    let base = m(&doc(&[&world("\"message\" \"old\"\n", BRUSH_A)]));
    let ours = m(&doc(&[&world("\"message\" \"new\"\n", BRUSH_A)]));
    let theirs = base.clone();

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(prop(merged_world(&out.map), "message"), Some("new"));
}

#[test]
fn a_property_added_on_one_side_is_kept() {
    let base = m(&doc(&[&world("", BRUSH_A)]));
    let ours = base.clone();
    let theirs = m(&doc(&[&world("\"_sun_color\" \"1 1 1\"\n", BRUSH_A)]));

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(prop(merged_world(&out.map), "_sun_color"), Some("1 1 1"));
}

#[test]
fn a_property_removed_on_one_side_is_removed() {
    let base = m(&doc(&[&world("\"message\" \"hi\"\n", BRUSH_A)]));
    let ours = m(&doc(&[&world("", BRUSH_A)]));
    let theirs = base.clone();

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(prop(merged_world(&out.map), "message"), None);
}

#[test]
fn a_two_sided_property_edit_conflicts_and_keeps_ours() {
    let base = m(&doc(&[&world("\"message\" \"base\"\n", BRUSH_A)]));
    let ours = m(&doc(&[&world("\"message\" \"ours\"\n", BRUSH_A)]));
    let theirs = m(&doc(&[&world("\"message\" \"theirs\"\n", BRUSH_A)]));

    let out = merge(&base, &ours, &theirs);
    assert_eq!(out.conflicts.len(), 1);
    assert!(out.conflicts[0].detail.contains("message"));
    assert_eq!(prop(merged_world(&out.map), "message"), Some("ours"));
}

#[test]
fn an_entity_removed_on_one_side_and_untouched_on_the_other_is_removed() {
    let light = "{\n\"classname\" \"light\"\n\"_tb_id\" \"7\"\n\"origin\" \"0 0 64\"\n}\n";
    let base = m(&doc(&[&world("", BRUSH_A), light]));
    let ours = m(&doc(&[&world("", BRUSH_A)]));
    let theirs = base.clone();

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(out.map.entities.len(), 1);
}

#[test]
fn an_entity_removed_on_one_side_and_edited_on_the_other_conflicts() {
    let base = m(&doc(&[
        &world("", BRUSH_A),
        "{\n\"classname\" \"light\"\n\"_tb_id\" \"7\"\n\"light\" \"200\"\n}\n",
    ]));
    let ours = m(&doc(&[&world("", BRUSH_A)]));
    let theirs = m(&doc(&[
        &world("", BRUSH_A),
        "{\n\"classname\" \"light\"\n\"_tb_id\" \"7\"\n\"light\" \"300\"\n}\n",
    ]));

    let out = merge(&base, &ours, &theirs);
    assert_eq!(out.conflicts.len(), 1);
    assert!(out.conflicts[0].detail.contains("removed on our side"));
    // ours won the delete
    assert_eq!(out.map.entities.len(), 1);
}

#[test]
fn a_brush_removed_on_one_side_is_removed() {
    let base = m(&doc(&[&world("", &format!("{BRUSH_A}{BRUSH_B}"))]));
    let ours = m(&doc(&[&world("", BRUSH_A)]));
    let theirs = base.clone();

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(merged_world(&out.map).brushes.len(), 1);
}

#[test]
fn a_brush_added_by_each_side_yields_both() {
    let base = m(&doc(&[&world("", BRUSH_A)]));
    let ours = m(&doc(&[&world("", &format!("{BRUSH_A}{BRUSH_B}"))]));
    let theirs = m(&doc(&[&world(
        "",
        &format!("{BRUSH_A}{}", BRUSH_B.replace("metal", "wood")),
    )]));

    let out = merge(&base, &ours, &theirs);
    assert_eq!(merged_world(&out.map).brushes.len(), 3);
}

#[test]
fn tb_id_matches_across_reordering() {
    let a = "{\n\"classname\" \"light\"\n\"_tb_id\" \"1\"\n\"light\" \"100\"\n}\n";
    let b = "{\n\"classname\" \"light\"\n\"_tb_id\" \"2\"\n\"light\" \"200\"\n}\n";
    let a2 = "{\n\"classname\" \"light\"\n\"_tb_id\" \"1\"\n\"light\" \"150\"\n}\n";

    let base = m(&doc(&[&world("", BRUSH_A), a, b]));
    // ours reorders and edits light 1
    let ours = m(&doc(&[&world("", BRUSH_A), b, a2]));
    let theirs = base.clone();

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    let light1 = out
        .map
        .entities
        .iter()
        .find(|e| e.property("_tb_id") == Some("1"))
        .unwrap();
    assert_eq!(light1.property("light"), Some("150"));
}

#[test]
fn the_same_brush_edited_on_both_sides_conflicts_once() {
    let base = m(&doc(&[&world("", &format!("{BRUSH_A}{BRUSH_B}"))]));
    // each side retextures a different face of BRUSH_B
    let ours = m(&doc(&[&world(
        "",
        &format!("{BRUSH_A}{}", BRUSH_B.replacen("metal", "wood", 1)),
    )]));
    let theirs = m(&doc(&[&world(
        "",
        &format!("{BRUSH_A}{}", BRUSH_B.replacen("metal", "stone", 1)),
    )]));

    let out = merge(&base, &ours, &theirs);
    assert_eq!(out.conflicts.len(), 1, "{:?}", out.conflicts);
    assert!(out.conflicts[0].detail.contains("edited on both sides"));
    // one brush, not two overlapping ones
    assert_eq!(merged_world(&out.map).brushes.len(), 2);
    let b = &merged_world(&out.map).brushes[1];
    assert!(b.faces.iter().any(|f| f.material_name == "wood"));
    assert!(!b.faces.iter().any(|f| f.material_name == "stone"));
}

#[test]
fn the_same_brush_edit_on_both_sides_merges_cleanly() {
    let base = m(&doc(&[&world("", BRUSH_B)]));
    let edited = world("", &BRUSH_B.replacen("metal", "wood", 1));
    let ours = m(&doc(&[&edited]));
    let theirs = m(&doc(&[&edited]));

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(merged_world(&out.map).brushes.len(), 1);
}

#[test]
fn a_brush_edited_here_and_deleted_there_conflicts_and_keeps_the_edit() {
    let base = m(&doc(&[&world("", &format!("{BRUSH_A}{BRUSH_B}"))]));
    let ours = m(&doc(&[&world(
        "",
        &format!("{BRUSH_A}{}", BRUSH_B.replacen("metal", "wood", 1)),
    )]));
    let theirs = m(&doc(&[&world("", BRUSH_A)]));

    let out = merge(&base, &ours, &theirs);
    assert_eq!(out.conflicts.len(), 1, "{:?}", out.conflicts);
    assert_eq!(merged_world(&out.map).brushes.len(), 2);
}

#[test]
fn the_format_and_game_are_three_wayed() {
    let base = MapFile {
        format: MapFormat::Standard,
        game: None,
        entities: vec![],
        notes_version: None,
        warnings: vec![],
    };
    let ours = MapFile {
        notes_version: None,
        format: MapFormat::Valve,
        game: Some("Quake".into()),
        entities: vec![],
        warnings: vec![],
    };
    let theirs = base.clone();

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty());
    assert_eq!(out.map.format, MapFormat::Valve);
    assert_eq!(out.map.game.as_deref(), Some("Quake"));
}

#[test]
fn merge_text_round_trips_through_the_reader_and_writer() {
    let base = doc(&[&world("\"message\" \"base\"\n", BRUSH_A)]);
    let ours = doc(&[&world("\"message\" \"new\"\n", BRUSH_A)]);
    let theirs = doc(&[&world(
        "\"message\" \"base\"\n",
        &format!("{BRUSH_A}{BRUSH_B}"),
    )]);

    let (merged, conflicts) = merge_text(&base, &ours, &theirs).unwrap();
    assert!(conflicts.is_empty(), "{conflicts:?}");

    let reread = read_map(&merged, None).unwrap();
    assert_eq!(reread.entities[0].property("message"), Some("new"));
    assert_eq!(reread.entities[0].brushes.len(), 2);
}

#[test]
fn merge_text_reports_conflicts() {
    let base = doc(&[&world("\"message\" \"base\"\n", BRUSH_A)]);
    let ours = doc(&[&world("\"message\" \"a\"\n", BRUSH_A)]);
    let theirs = doc(&[&world("\"message\" \"b\"\n", BRUSH_A)]);

    let (_, conflicts) = merge_text(&base, &ours, &theirs).unwrap();
    assert_eq!(conflicts.len(), 1);
}

#[test]
fn entity_key_display() {
    assert_eq!(EntityKey::Worldspawn.to_string(), "worldspawn");
    assert_eq!(EntityKey::TbId("9".into()).to_string(), "entity _tb_id 9");
    assert_eq!(
        EntityKey::Positional {
            classname: "light".into(),
            nth: 2
        }
        .to_string(),
        "light #2"
    );
    assert_eq!(
        EntityKey::Named {
            classname: "func_door".into(),
            targetname: "door1".into(),
            nth: 0
        }
        .to_string(),
        "func_door \"door1\""
    );
    assert_eq!(
        EntityKey::Spot {
            classname: "light".into(),
            origin: "0 0 64".into(),
            nth: 0
        }
        .to_string(),
        "light at 0 0 64"
    );
}

#[test]
fn a_targetnamed_entity_matches_across_a_move() {
    let button = |origin: &str, wait: &str| {
        format!("{{\n\"classname\" \"func_button\"\n\"targetname\" \"b1\"\n\"origin\" \"{origin}\"\n\"wait\" \"{wait}\"\n}}\n")
    };
    let base = m(&doc(&[&world("", BRUSH_A), &button("0 0 0", "3")]));
    // ours moves the button; theirs changes its wait
    let ours = m(&doc(&[&world("", BRUSH_A), &button("64 0 0", "3")]));
    let theirs = m(&doc(&[&world("", BRUSH_A), &button("0 0 0", "5")]));

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    let button = out
        .map
        .entities
        .iter()
        .find(|e| e.property("targetname") == Some("b1"))
        .unwrap();
    assert_eq!(button.property("origin"), Some("64 0 0"));
    assert_eq!(button.property("wait"), Some("5"));
    // exactly one button, not two
    assert_eq!(
        out.map
            .entities
            .iter()
            .filter(|e| e.classname() == Some("func_button"))
            .count(),
        1
    );
}

#[test]
fn an_unmoved_light_matches_by_its_spot_despite_reordering() {
    let light = |o: &str, brightness: &str| {
        format!("{{\n\"classname\" \"light\"\n\"origin\" \"{o}\"\n\"light\" \"{brightness}\"\n}}\n")
    };
    let base = m(&doc(&[
        &world("", BRUSH_A),
        &light("0 0 64", "200"),
        &light("128 0 64", "300"),
    ]));
    // ours reorders the two lights and brightens the first
    let ours = m(&doc(&[
        &world("", BRUSH_A),
        &light("128 0 64", "300"),
        &light("0 0 64", "250"),
    ]));
    let theirs = base.clone();

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    let first = out
        .map
        .entities
        .iter()
        .find(|e| e.property("origin") == Some("0 0 64"))
        .unwrap();
    assert_eq!(first.property("light"), Some("250"));
    assert_eq!(
        out.map
            .entities
            .iter()
            .filter(|e| e.classname() == Some("light"))
            .count(),
        2
    );
}

// --- brush identity ------------------------------------------------------------------

/// The same two brushes, each carrying the id this editor writes.
fn with_ids(a: &str, b: &str) -> String {
    world(
        "",
        &format!("// heron_brush id a1\n{a}// heron_brush id b2\n{b}"),
    )
}

/// Two people editing different brushes do not conflict, even when one file has grown a
/// line above them.
///
/// This is the everyday case for two people on one map, and it used to fail. A brush
/// carried the line it was read from, and that line was part of comparing one brush to
/// another, so a brush that had merely moved down the file compared unequal to itself.
/// The merge then read both sides as having replaced the brush and dropped one of the
/// two edits. The ids make the match exact; taking the line out of the comparison is
/// what fixes it even for a map with no ids in it.
#[test]
fn edits_to_different_brushes_both_land_across_a_line_shift() {
    let base = with_ids(BRUSH_A, BRUSH_B);
    // Ours retextures the first brush and gains a comment above everything.
    let ours = format!("// somebody left a note\n{}", base.replace("stone", "sand"));
    let theirs = base.replace("metal", "brass");

    let (merged, conflicts) = merge_text(&base, &ours, &theirs).expect("it merges");
    assert!(conflicts.is_empty(), "{conflicts:?}");
    assert!(merged.contains("sand"), "our edit landed:\n{merged}");
    assert!(merged.contains("brass"), "their edit landed:\n{merged}");
}

/// An edit on one side and nothing on the other is an edit, not a replacement.
#[test]
fn a_brush_edited_on_one_side_only_keeps_that_edit() {
    let base = with_ids(BRUSH_A, BRUSH_B);
    let theirs = base.replace("metal", "brass");

    let (merged, conflicts) = merge_text(&base, &base, &theirs).expect("it merges");
    assert!(conflicts.is_empty(), "{conflicts:?}");
    assert!(merged.contains("brass"), "{merged}");
    assert!(
        !merged.contains("metal"),
        "the old brush is not kept beside it:\n{merged}"
    );
}

/// Two people editing the same brush is still a disagreement, and is still reported.
#[test]
fn the_same_brush_edited_both_ways_conflicts() {
    let base = with_ids(BRUSH_A, BRUSH_B);
    let ours = base.replace("stone", "sand");
    let theirs = base.replace("stone", "slate");

    let (merged, conflicts) = merge_text(&base, &ours, &theirs).expect("it merges");
    assert_eq!(conflicts.len(), 1, "{conflicts:?}");
    assert!(merged.contains("sand"), "ours is what is kept:\n{merged}");
}

/// A brush's id survives a save, so the file names the same brush next time.
#[test]
fn an_id_read_from_a_file_is_written_back() {
    use geometry_math::bbox::Bbox3d;
    use map_model::io::map_writer::write_map;
    use map_model::node::{build_tree, flatten_tree};

    let source = with_ids(BRUSH_A, BRUSH_B);
    let map = m(&source);
    assert_eq!(map.entities[0].brushes[0].id, Some(0xa1));
    assert_eq!(map.entities[0].brushes[1].id, Some(0xb2));

    let tree = build_tree(&map, &Bbox3d::from_scalars(-16384.0, 16384.0));
    let written = write_map(&flatten_tree(&tree.world, MapFormat::LOSSLESS, None));
    assert!(written.contains("// heron_brush id a1"), "{written}");
    assert!(written.contains("// heron_brush id b2"), "{written}");
}

/// A light added above another does not make the one below it a different light.
///
/// An entity with no id is matched by classname and its position among the entities of
/// that classname, so an addition earlier in the file shifts everything after it and a
/// merge sees an entity that has changed rather than one that has moved. The ids the
/// editor writes say which entity is which outright.
#[test]
fn an_entity_added_above_another_does_not_disturb_it() {
    let base = format!(
        "{}// heron_brush id 11\n{{\n\"classname\" \"light\"\n\"origin\" \"0 0 64\"\n\"light\" \"300\"\n}}\n",
        world("", "")
    );
    // Ours puts another light in front of it.
    let ours = format!(
        "{}// heron_brush id 22\n{{\n\"classname\" \"light\"\n\"origin\" \"128 0 64\"\n}}\n// heron_brush id 11\n{{\n\"classname\" \"light\"\n\"origin\" \"0 0 64\"\n\"light\" \"300\"\n}}\n",
        world("", "")
    );
    // Theirs brightens the light that was already there.
    let theirs = base.replace("\"300\"", "\"500\"");

    let (merged, conflicts) = merge_text(&base, &ours, &theirs).expect("it merges");
    assert!(conflicts.is_empty(), "{conflicts:?}");
    let map = m(&merged);
    let lights: Vec<&Entity> = map
        .entities
        .iter()
        .filter(|e| e.classname() == Some("light"))
        .collect();
    assert_eq!(lights.len(), 2, "both lights, and neither twice:\n{merged}");
    let brightened = lights
        .iter()
        .find(|e| e.id == Some(0x11))
        .expect("the light that was there");
    assert_eq!(
        brightened.property("light"),
        Some("500"),
        "their edit landed"
    );
}

// --- audit 2026-11-10 -------------------------------------------------------------

/// A 3x3 patch carrying id 42, its middle control point raised to `z`.
fn patch_42(z: i32) -> String {
    format!(
        "// heron_brush id 2a\n{{\npatchDef2\n{{\ncommon/caulk\n( 3 3 0 0 0 )\n(\n\
         ( ( -64 -64 0 0 0 ) ( -64 0 32 0 0.5 ) ( -64 64 0 0 1 ) )\n\
         ( ( 0 -64 32 0.5 0 ) ( 0 0 {z} 0.5 0.5 ) ( 0 64 32 0.5 1 ) )\n\
         ( ( 64 -64 0 1 0 ) ( 64 0 32 1 0.5 ) ( 64 64 0 1 1 ) )\n)\n}}\n}}\n"
    )
}

#[test]
fn patch_edits_on_both_branches_report_a_conflict_without_duplicate_ids() {
    let base = m(&world("", &patch_42(64)));
    let ours = m(&world("", &patch_42(96)));
    let theirs = m(&world("", &patch_42(128)));
    assert_eq!(base.entities[0].patches[0].id, Some(42));

    let out = merge(&base, &ours, &theirs);
    let patches = &merged_world(&out.map).patches;
    assert_eq!(patches.len(), 1, "{patches:?}");
    assert_eq!(patches[0], ours.entities[0].patches[0]);
    assert_eq!(out.conflicts.len(), 1, "{:?}", out.conflicts);

    // One side's edit alone lands without a conflict.
    let out = merge(&base, &base, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(merged_world(&out.map).patches.len(), 1);
    assert_eq!(
        merged_world(&out.map).patches[0],
        theirs.entities[0].patches[0]
    );
}

#[test]
fn leading_comments_do_not_turn_a_clean_entity_deletion_into_a_conflict() {
    let light = "{\n\"classname\" \"light\"\n\"origin\" \"0 0 64\"\n}\n";
    let base = m(&doc(&[&world("", BRUSH_A), light]));
    let ours = m(&doc(&[
        "// a note\n// and another\n",
        &world("", BRUSH_A),
        light,
    ]));
    let theirs = m(&doc(&[&world("", BRUSH_A)]));
    assert_ne!(base.entities[1].line, ours.entities[1].line);

    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(out.map.entities.len(), 1);
}

/// A world brush 42 and two empty brush entities with ids of their own.
fn reparent_fixture(world_brush: &str, first: &str, second: &str) -> MapFile {
    m(&doc(&[
        &world("", world_brush),
        &format!("// heron_brush id 11\n{{\n\"classname\" \"func_wall\"\n{first}}}\n"),
        &format!("// heron_brush id 22\n{{\n\"classname\" \"func_wall\"\n{second}}}\n"),
    ]))
}

fn brushes_with_id(map: &MapFile, id: u64) -> Vec<(usize, &map_model::io::map_reader::Brush)> {
    map.entities
        .iter()
        .enumerate()
        .flat_map(|(at, e)| e.brushes.iter().map(move |b| (at, b)))
        .filter(|(_, b)| b.id == Some(id))
        .collect()
}

#[test]
fn concurrent_reparenting_reports_conflict_and_keeps_unique_brush_id() {
    let brush = format!("// heron_brush id 2a\n{BRUSH_A}");
    let base = reparent_fixture(&brush, "", "");
    let ours = reparent_fixture("", &brush, "");
    let theirs = reparent_fixture("", "", &brush);

    let out = merge(&base, &ours, &theirs);
    assert_eq!(out.conflicts.len(), 1, "{:?}", out.conflicts);
    let found = brushes_with_id(&out.map, 42);
    assert_eq!(found.len(), 1, "{:?}", out.map.entities);
    assert_eq!(found[0].0, 1, "ours' destination wins");

    // Only one side moving it is a clean move.
    let out = merge(&base, &ours, &base);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    let found = brushes_with_id(&out.map, 42);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, 1);

    // A move on one side and an edit in place on the other: one brush, moved and edited.
    let edited = brush.replace("stone", "rock");
    let theirs = reparent_fixture(&edited, "", "");
    let out = merge(&base, &ours, &theirs);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    let found = brushes_with_id(&out.map, 42);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, 1);
    assert_eq!(found[0].1, &theirs.entities[0].brushes[0]);

    // A move on one side and a removal on the other keeps the move and says so.
    let theirs = reparent_fixture("", "", "");
    let out = merge(&base, &ours, &theirs);
    assert_eq!(out.conflicts.len(), 1, "{:?}", out.conflicts);
    assert_eq!(brushes_with_id(&out.map, 42).len(), 1);
}
