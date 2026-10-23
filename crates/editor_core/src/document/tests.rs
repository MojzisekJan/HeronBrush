//! Document behavior and integration regressions.

use super::*;
use crate::world::WorldBounds;

/// Read-only profiling of a user-supplied map; no machine-specific path is stored.
#[test]
#[ignore = "set HERON_PROFILE_MAP to a map path to measure interaction costs"]
fn profile_map_interaction() {
    use std::{hint::black_box, time::Instant};
    let path = std::env::var("HERON_PROFILE_MAP").expect("HERON_PROFILE_MAP");
    let start = Instant::now();
    let mut document = Document::open(path, WorldBounds::default()).unwrap();
    eprintln!(
        "open {:?}, brushes {}",
        start.elapsed(),
        document.brush_count()
    );
    let bounds = document.bounds().unwrap();
    let ids: Vec<_> = node::brushes_with_ids(document.tree_world())
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let ray = Ray3d::new(
        bounds.center() + Vec3d::new(0.0, 0.0, bounds.max.z() - bounds.min.z()),
        Vec3d::new(0.0, 0.0, -1.0),
    );
    let start = Instant::now();
    for _ in 0..20 {
        black_box(document.pick(&ray));
    }
    eprintln!("pick {:?}", start.elapsed() / 20);
    let start = Instant::now();
    for id in ids.iter().take(20) {
        document.select_only(*id);
    }
    eprintln!("select {:?}", start.elapsed() / 20);
    let filters = crate::view_filters::ViewFilters::default();
    let start = Instant::now();
    for _ in 0..5 {
        *document.surface_cache.borrow_mut() = Default::default();
        black_box(document.preview_mesh(&filters));
    }
    eprintln!("cold preview mesh {:?}", start.elapsed() / 5);
    let start = Instant::now();
    for id in ids.iter().take(20) {
        document.select_only(*id);
        black_box(document.preview_mesh(&filters));
    }
    eprintln!("select + cached preview {:?}", start.elapsed() / 20);
    let start = Instant::now();
    for id in ids.iter().take(20) {
        document.select_only(*id);
        black_box(document.edges());
    }
    eprintln!("select + edges {:?}", start.elapsed() / 20);
    let start = Instant::now();
    for _ in 0..20 {
        black_box(document.selection_bounds());
        black_box(document.selected_brushes());
        black_box(document.paintable_face_count());
    }
    eprintln!("selection queries {:?}", start.elapsed() / 20);
    let start = Instant::now();
    black_box(document.group_bounds());
    eprintln!("group bounds: {:?}", start.elapsed());
    let start = Instant::now();
    black_box(document.selectable_brushes());
    eprintln!("selectable brushes: {:?}", start.elapsed());
}

#[test]
fn content_revision_ignores_selection_but_tracks_edits_and_undo() {
    let mut document = two_boxes();
    let original = document.content_revision();
    document.select_all();
    document.clear_selection();
    let id = node::brushes_with_ids(document.tree_world())[0].0;
    document.select_only(id);
    document.select_face_only(FaceRef { node: id, face: 0 });
    assert_eq!(document.content_revision(), original);
    document.select_only(id);
    document.translate_selection(Vec3d::new(0.0, 0.0, 16.0));
    assert_ne!(document.content_revision(), original);
    let edited = document.content_revision();
    document.undo();
    assert_ne!(document.content_revision(), edited);
}

#[test]
fn preview_cache_tracks_selection_edits_undo_and_filters() {
    let mut document = two_boxes();
    let filters = crate::view_filters::ViewFilters::default();
    let initial = document.preview_mesh(&filters);
    let id = node::brushes_with_ids(&document.tree.world)[0].0;
    document.select_only(id);
    let selected = document.preview_mesh(&filters);
    // A selection change patches flags; nothing is triangulated again.
    assert_eq!(document.surface_cache.borrow().rebuilt, 0);
    assert_eq!(selected.triangles.iter().filter(|t| t.selected).count(), 12);
    assert_eq!(
        selected
            .triangles
            .iter()
            .map(|t| t.positions)
            .collect::<Vec<_>>(),
        initial
            .triangles
            .iter()
            .map(|t| t.positions)
            .collect::<Vec<_>>()
    );
    document.select_face_only(FaceRef { node: id, face: 0 });
    assert_eq!(
        document
            .preview_mesh(&filters)
            .triangles
            .iter()
            .filter(|t| t.selected)
            .count(),
        2
    );
    document.clear_selection();
    assert!(document
        .preview_mesh(&filters)
        .triangles
        .iter()
        .all(|t| !t.selected));
    document.select_only(id);
    assert!(document.translate_selection(Vec3d::new(0.0, 0.0, 100.0)));
    let moved = document.preview_mesh(&filters);
    assert_ne!(moved.triangles[0].positions, initial.triangles[0].positions);
    document.undo();
    let undone = document.preview_mesh(&filters);
    assert_eq!(
        undone
            .triangles
            .iter()
            .map(|t| t.positions)
            .collect::<Vec<_>>(),
        initial
            .triangles
            .iter()
            .map(|t| t.positions)
            .collect::<Vec<_>>()
    );
    let hidden = crate::view_filters::ViewFilters {
        show_brushes: false,
        ..filters
    };
    assert!(document.preview_mesh(&hidden).triangles.is_empty());
    assert_eq!(
        document.preview_mesh(&filters).triangles.len(),
        initial.triangles.len()
    );
    document.select_only(id);
    document.isolate_selection();
    assert_eq!(document.preview_mesh(&filters).triangles.len(), 12);
    document.show_all();
    assert_eq!(
        document.preview_mesh(&filters).triangles.len(),
        initial.triangles.len()
    );
}

/// A box whose first face carries its own UV axes and whose second carries surface
/// flags -- the two things a dialect can have nowhere to put.
fn box_with_axes_and_flags() -> Document {
    let source = "\
{
\"classname\" \"worldspawn\"
{
( -16 -16 -16 ) ( -16 16 -16 ) ( -16 -16 16 ) stone [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -16 -16 -16 ) ( -16 -16 16 ) ( 16 -16 -16 ) stone 0 0 0 1 1 1 8 0
( -16 -16 -16 ) ( 16 -16 -16 ) ( -16 16 -16 ) stone 0 0 0 1 1
( 16 16 16 ) ( 16 -16 16 ) ( -16 16 16 ) stone 0 0 0 1 1
( 16 16 16 ) ( -16 16 16 ) ( 16 16 -16 ) stone 0 0 0 1 1
( 16 16 16 ) ( 16 16 -16 ) ( 16 -16 16 ) stone 0 0 0 1 1
}
}
";
    let map = read_map(source, None).expect("the sample parses");
    Document::from_map(&map, WorldBounds::default(), None)
}

/// What dragging a big selection costs, printed rather than asserted.
///
/// The monkey is 962 brushes and dragging it was the thing that showed the cost: a
/// step took 136ms, of which 116 was rebuilding every brush's geometry from its
/// planes. A translation does not need that, and does not do it now: 59ms a step in
/// a debug build and 8.8ms in a release one. Kept because the next thing that makes
/// it slow again will show up here first.
#[test]
#[ignore = "a measurement, not a check"]
fn what_one_drag_step_of_the_monkey_costs() {
    use std::time::Instant;
    let source = std::fs::read_to_string("../../assets/monkey.map").expect("the monkey");
    let map = read_map(&source, None).expect("it reads");
    let mut document = Document::from_map(&map, WorldBounds::default(), None);
    document.select_all();
    let count = document.selection_count();

    let shift = geometry_math::mat_ext::translation_matrix(Vec3d::new(8.0, 0.0, 0.0));
    let started = Instant::now();
    for _ in 0..10 {
        document.transform_selection(&shift);
    }
    eprintln!(
        "{count} selected: {:?} per drag step",
        started.elapsed() / 10
    );

    let filters = crate::view_filters::ViewFilters::default();
    let started = Instant::now();
    for _ in 0..10 {
        let _ = document.triangles(&filters);
    }
    eprintln!("redraw geometry: {:?} per frame", started.elapsed() / 10);

    let turn = geometry_math::mat_ext::rotation_matrix_about_axis(Vec3d::new(0.0, 0.0, 1.0), 0.05);
    let started = Instant::now();
    for _ in 0..10 {
        document.transform_selection(&turn);
    }
    eprintln!("one rotation step: {:?}", started.elapsed() / 10);

    let scale = geometry_math::mat_ext::scaling_matrix(Vec3d::new(1.01, 1.01, 1.01));
    let started = Instant::now();
    for _ in 0..10 {
        document.transform_selection(&scale);
    }
    eprintln!("one scale step: {:?}", started.elapsed() / 10);

    // Where the drag step's time goes.
    let brushes: Vec<map_model::brush::Brush> =
        map_model::node::brushes_with_ids(document.tree_world())
            .into_iter()
            .map(|(_, brush)| brush.clone())
            .collect();

    let started = Instant::now();
    for _ in 0..10 {
        let _ = map_model::node::brushes_with_ids(document.tree_world());
    }
    eprintln!("  walking the tree: {:?}", started.elapsed() / 10);

    let started = Instant::now();
    for _ in 0..10 {
        for brush in &brushes {
            let _ = brush.clone();
        }
    }
    eprintln!("  cloning them once: {:?}", started.elapsed() / 10);

    let started = Instant::now();
    for _ in 0..10 {
        for brush in &brushes {
            let _ = brush.transform_with_alignment(
                &document.world_bounds,
                &shift,
                document.alignment_lock(),
                |name| document.material_size(name),
            );
        }
    }
    eprintln!("  rebuilding the geometry: {:?}", started.elapsed() / 10);
}

#[test]
#[ignore = "a measurement, not a check"]
fn which_moves_of_the_monkey_are_refused() {
    let source = std::fs::read_to_string("../../assets/monkey.map").expect("the monkey");
    let map = read_map(&source, None).expect("it reads");
    let mut document = Document::from_map(&map, WorldBounds::default(), None);
    document.select_all();

    for step in [1.0, 16.0, 256.0, 1024.0, 4096.0, 12000.0] {
        let ok = document.translate_selection(Vec3d::new(step, 0.0, 0.0));
        eprintln!("step of {step}: {}", if ok { "moved" } else { "REFUSED" });
        if ok {
            document.translate_selection(Vec3d::new(-step, 0.0, 0.0));
        }
    }
}

#[test]
fn a_dialect_says_what_this_map_would_lose_in_it() {
    use map_model::map_format::MapFormat;

    // A plain map costs nothing in any dialect: what a dialect cannot say only
    // matters once something in the map is saying it.
    let plain = one_box();
    for format in [MapFormat::Standard, MapFormat::Quake2, MapFormat::Valve] {
        assert!(
            plain.what_a_dialect_would_drop(format).is_empty(),
            "{format:?} on a plain map"
        );
    }

    let document = box_with_axes_and_flags();
    assert!(document
        .what_a_dialect_would_drop(MapFormat::Quake2Valve)
        .is_empty());

    // Standard has room for neither.
    let lost = document.what_a_dialect_would_drop(MapFormat::Standard);
    assert_eq!(lost.len(), 2, "{lost:?}");
    assert!(lost.iter().any(|line| line.contains("UV axes")), "{lost:?}");
    assert!(lost.iter().any(|line| line.contains("flags")), "{lost:?}");

    // Quake 2 keeps the flags and loses the axes.
    let lost = document.what_a_dialect_would_drop(MapFormat::Quake2);
    assert_eq!(lost.len(), 1, "{lost:?}");
    assert!(lost[0].contains("UV axes"), "{lost:?}");
}

/// A document holding one axis-aligned box brush.
fn one_box() -> Document {
    let source = "\
{
\"classname\" \"worldspawn\"
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
    let map = read_map(source, None).expect("the sample parses");
    Document::from_map(&map, WorldBounds::default(), None)
}

#[test]
fn a_face_the_face_tool_has_hold_of_is_painted_on_its_own() {
    // The face tool leaves the brush selected -- its handles hang off it -- so
    // painting by the selection alone would colour all six sides.
    let mut document = one_box();
    let brushes = map_model::node::brushes_with_ids(&document.tree.world);
    let (id, _) = *brushes.first().expect("the brush");
    document.select_only(id);

    // The handle at the centre of the top face, which is what the tool picks.
    let picked = document.faces_at(&[Vec3d::new(0.0, 0.0, 16.0)]);
    assert_eq!(picked.len(), 1, "one handle names one face");

    let painted = document.set_face_color_on(
        &picked,
        Some(map_model::brush_face::FaceColor::solid(
            map_model::brush_face::Rgb::new(200, 30, 30),
        )),
    );
    assert_eq!(painted, 1);

    let coloured = map_model::node::brushes(&document.tree.world)
        .into_iter()
        .flat_map(|brush| brush.faces().to_vec())
        .filter(|face| face.color.is_some())
        .count();
    assert_eq!(coloured, 1, "the other five keep their material");
}

#[test]
fn a_handle_centre_on_a_brush_that_is_not_selected_names_nothing() {
    let document = one_box();
    assert!(document.faces_at(&[Vec3d::new(0.0, 0.0, 16.0)]).is_empty());
}

#[test]
fn painting_one_picked_face_leaves_the_other_five_alone() {
    let mut document = one_box();
    let brushes = map_model::node::brushes_with_ids(&document.tree.world);
    let (id, _) = *brushes.first().expect("the brush");
    document.select_face_only(FaceRef { node: id, face: 0 });

    let painted = document.set_face_color(Some(map_model::brush_face::FaceColor::solid(
        map_model::brush_face::Rgb::new(200, 30, 30),
    )));
    assert_eq!(painted, 1, "one face was picked");

    let coloured = map_model::node::brushes(&document.tree.world)
        .into_iter()
        .flat_map(|brush| brush.faces().to_vec())
        .filter(|face| face.color.is_some())
        .count();
    assert_eq!(coloured, 1, "only the picked face wears the colour");
}

#[test]
fn a_box_gives_two_triangles_per_face() {
    let triangles = one_box().triangles(&crate::view_filters::ViewFilters::default());
    assert_eq!(triangles.len(), 12, "six quads, two triangles each");
}

#[test]
fn every_triangle_is_wound_to_match_its_face_normal() {
    for triangle in one_box().triangles(&crate::view_filters::ViewFilters::default()) {
        let [a, b, c] = triangle.positions;
        // Faces are wound clockwise seen from outside, so the outward normal is the
        // reversed cross product — the same one `geometry_math::plane::from_points` takes.
        let wound = geometry_math::vec::normalize(geometry_math::vec::cross(c - a, b - a));
        assert!(
            geometry_math::vec::dot(wound, triangle.normal) > 0.99,
            "winding and normal disagree: {wound:?} vs {:?}",
            triangle.normal
        );
    }
}

#[test]
fn every_triangle_faces_away_from_the_inside() {
    // The origin is inside this box, so every face must point away from it.
    for triangle in one_box().triangles(&crate::view_filters::ViewFilters::default()) {
        let to_origin = Vec3d::zero() - triangle.positions[0];
        assert!(geometry_math::vec::dot(to_origin, triangle.normal) < 0.0);
    }
}

#[test]
fn painting_a_selected_brush_colours_every_face_of_it() {
    let mut document = one_box();
    document.select_all();

    let red = map_model::brush_face::Rgb::new(212, 52, 52);
    let painted = document.set_face_color(Some(map_model::brush_face::FaceColor::solid(red)));
    assert_eq!(painted, 6, "a selected brush is painted all over");

    let filters = crate::view_filters::ViewFilters::default();
    for triangle in document.triangles(&filters) {
        assert_eq!(triangle.color, red.to_f32());
        assert_eq!(
            triangle.material, None,
            "a painted face is drawn in its colour, not through its material"
        );
        assert_eq!(
            triangle.corner_colors, None,
            "a solid colour needs no corners"
        );
    }

    document.undo();
    assert!(
        document
            .triangles(&filters)
            .iter()
            .all(|t| t.color != red.to_f32()),
        "one undo takes the paint off"
    );
}

#[test]
fn a_gradient_gives_every_triangle_a_colour_per_corner() {
    let mut document = one_box();
    document.select_all();
    document.set_face_color(Some(map_model::brush_face::FaceColor::gradient(
        map_model::brush_face::Rgb::new(0, 0, 0),
        map_model::brush_face::Rgb::new(255, 255, 255),
        0.0,
    )));

    let filters = crate::view_filters::ViewFilters::default();
    let triangles = document.triangles(&filters);
    assert!(!triangles.is_empty());
    // Every triangle carries three corner colours, and somewhere on the box a corner
    // is at each end of the blend.
    let mut darkest = f32::INFINITY;
    let mut lightest = f32::NEG_INFINITY;
    for triangle in &triangles {
        let corners = triangle
            .corner_colors
            .expect("a gradient has corner colours");
        for corner in corners {
            darkest = darkest.min(corner[0]);
            lightest = lightest.max(corner[0]);
        }
    }
    assert!(darkest < 0.01, "a corner is the starting colour: {darkest}");
    assert!(lightest > 0.99, "a corner is the ending colour: {lightest}");
}

#[test]
fn painting_a_face_the_colour_it_already_wears_changes_nothing() {
    let mut document = one_box();
    document.select_all();
    let colour = Some(map_model::brush_face::FaceColor::solid(
        map_model::brush_face::Rgb::new(1, 2, 3),
    ));
    assert_eq!(document.set_face_color(colour), 6);
    assert_eq!(document.set_face_color(colour), 0);
}

#[test]
fn rounding_an_edge_replaces_the_brush_in_one_undoable_step() {
    let mut document = one_box();
    document.select_all();

    // The top edge running along Y at x = 16, taken from the brush itself so the
    // positions are exactly the ones the edge tool would hand over.
    let brush = document.selected_brushes()[0].clone();
    let edge = brush
        .edge_positions()
        .into_iter()
        .find(|(from, to)| from.x() > 15.0 && to.x() > 15.0 && from.z() > 15.0 && to.z() > 15.0)
        .expect("the box has a top edge at +x");

    let faces_before = brush.geometry().face_count();
    let rounded = document
        .fillet_edges(
            &[edge],
            &map_model::fillet::Fillet {
                radius: 4.0,
                faces: 3,
                corners: map_model::fillet::Corners::Mitred,
            },
        )
        .expect("the edge rounds");
    assert_eq!(rounded, 1);
    assert_eq!(
        document.selected_brushes()[0].geometry().face_count(),
        faces_before + 3
    );

    document.undo();
    assert_eq!(
        document.selected_brushes()[0].geometry().face_count(),
        faces_before,
        "one undo puts the sharp edge back"
    );
}

#[test]
fn a_radius_the_brush_cannot_give_up_rounds_nothing() {
    let mut document = one_box();
    document.select_all();
    let edge = document.selected_brushes()[0].edge_positions()[0];

    let outcome = document.fillet_edges(
        &[edge],
        &map_model::fillet::Fillet {
            radius: 512.0,
            faces: 3,
            corners: map_model::fillet::Corners::Mitred,
        },
    );
    assert!(outcome.is_err());
    assert_eq!(
        document.selected_brushes()[0].geometry().face_count(),
        6,
        "the brush is untouched"
    );
}

#[test]
fn the_revision_changes_when_the_selection_does() {
    let mut document = one_box();
    let before = document.revision();

    document.select_all();
    assert_ne!(document.revision(), before);
    assert_eq!(document.selection_count(), 1);
    assert!(document
        .triangles(&crate::view_filters::ViewFilters::default())
        .iter()
        .all(|t| t.selected));

    document.clear_selection();
    assert!(document
        .triangles(&crate::view_filters::ViewFilters::default())
        .iter()
        .all(|t| !t.selected));
}

#[test]
fn translating_the_selection_moves_the_triangles() {
    let mut document = one_box();
    document.select_all();
    assert!(document.translate_selection(Vec3d::new(0.0, 0.0, 100.0)));

    let bounds = document.bounds().expect("the box is still there");
    geometry_math::assert_approx_eq!(bounds.min, Vec3d::new(-16.0, -16.0, 84.0), 1e-6);
}

#[test]
fn translating_nothing_does_nothing() {
    let mut document = one_box();
    // Nothing selected.
    assert!(!document.translate_selection(Vec3d::new(0.0, 0.0, 10.0)));
    document.select_all();
    // A zero move.
    assert!(!document.translate_selection(Vec3d::zero()));
}

/* ========== materials ========== */

/// Writes a directory holding one solid-colour image, so the tests need no fixtures.
fn material_dir(name: &str) -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!("heron-document-materials-{name}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a directory");
    image::RgbaImage::from_pixel(32, 16, image::Rgba([180, 90, 60, 255]))
        .save(directory.join("brick.png"))
        .expect("a written png");
    directory
}

#[test]
fn a_face_without_its_material_still_draws() {
    // The normal state of a map opened without its materials: the triangles are there,
    // they just have nothing to wear.
    let document = one_box();
    let triangles = document.triangles(&crate::view_filters::ViewFilters::default());
    assert!(!triangles.is_empty());
    assert!(triangles.iter().all(|triangle| triangle.material.is_none()));
}

#[test]
fn loading_materials_and_applying_one_puts_it_on_the_triangles() {
    let directory = material_dir("apply");
    let mut document = one_box();
    document
        .load_materials(&directory)
        .expect("the materials load");

    // The first material in a fresh collection becomes the current one, since the
    // placeholder the box was drawn with is not in it.
    assert_eq!(document.current_material(), "brick");
    assert_eq!(document.materials().len(), 4);

    document.select_all();
    let changed = document.apply_current_material();
    assert_eq!(changed, 6, "a box has six faces");

    let triangles = document.triangles(&crate::view_filters::ViewFilters::default());
    assert!(triangles
        .iter()
        .all(|triangle| triangle.material == Some(0)));
}

#[test]
fn uv_coordinates_come_out_in_texels() {
    let directory = material_dir("uv");
    let mut document = one_box();
    document
        .load_materials(&directory)
        .expect("the materials load");
    document.select_all();
    document.apply_current_material();

    // The box is 64 units across and the default mapping is one texel per unit, so a
    // face spans 64 texels -- not the 0-to-1 range a normalized coordinate would give.
    let triangles = document.triangles(&crate::view_filters::ViewFilters::default());
    let spread = triangles
        .iter()
        .flat_map(|triangle| triangle.uv)
        .fold(0.0f32, |widest, uv| widest.max(uv.x().abs()));
    assert!(
        spread > 1.0,
        "the coordinates are normalized, not texels: {spread}"
    );
}

#[test]
fn replacing_the_collection_is_visible_to_the_renderer() {
    let directory = material_dir("revision");
    let mut document = one_box();
    let before = document.materials_revision();
    document
        .load_materials(&directory)
        .expect("the materials load");
    assert_ne!(document.materials_revision(), before);
}

#[test]
fn a_source_that_is_taken_away_takes_its_materials_with_it() {
    let bricks = material_dir("sources-bricks");
    let tiles = std::env::temp_dir().join("heron-document-materials-sources-tiles");
    let _ = std::fs::remove_dir_all(&tiles);
    std::fs::create_dir_all(&tiles).expect("a directory");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([40, 90, 160, 255]))
        .save(tiles.join("tile.png"))
        .expect("a written png");

    let mut document = one_box();
    document
        .load_materials(&bricks)
        .expect("the materials load");
    document.load_materials(&tiles).expect("the materials load");

    // Both folders are there, and both folders' materials with them.
    assert_eq!(document.material_sources().len(), 2);
    assert_eq!(document.materials().len(), 5);
    assert_eq!(
        document.material_sources()[1].name(),
        "heron-document-materials-sources-tiles"
    );

    // The same folder again is the same folder, not a second one; it goes to the end,
    // which is where the last word about a shared name belongs.
    document
        .load_materials(&bricks)
        .expect("the materials load");
    assert_eq!(document.material_sources().len(), 2);
    assert_eq!(
        document.material_sources()[1].path(),
        bricks.canonicalize().unwrap()
    );

    assert!(document.remove_material_source(0));
    assert_eq!(document.material_sources().len(), 1);
    assert!(document.materials().get("tile").is_none());
    assert!(document.materials().get("brick").is_some());

    // There is no third source to take away.
    assert!(!document.remove_material_source(5));
}

#[test]
fn applying_a_material_with_nothing_selected_changes_nothing() {
    let directory = material_dir("empty");
    let mut document = one_box();
    document
        .load_materials(&directory)
        .expect("the materials load");
    assert_eq!(document.apply_current_material(), 0);
}

/* ========== copying a face's attributes ========== */

/// The reference to a face of the one box, by which way it faces.
fn face_facing(document: &Document, direction: Vec3d) -> FaceRef {
    let (id, brush) = node::brushes_with_ids(&document.tree.world)
        .into_iter()
        .next()
        .expect("a brush");
    let geometry = brush.geometry();
    let face = (0..geometry.face_count())
        .find(|face| {
            vec::dot(
                geometry
                    .face_plane(map_model::polyhedron::FaceId(*face as u32))
                    .normal,
                direction,
            ) > 0.9
        })
        .expect("a face facing that way");
    FaceRef { node: id, face }
}

#[test]
fn copying_the_material_alone_leaves_the_mapping_where_it_was() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    let side = face_facing(&document, Vec3d::new(1.0, 0.0, 0.0));

    // Give the source a material and a mapping of its own.
    document.select_face_only(top);
    document.edit_faces(|face| {
        face.material_name = "brick".into();
        face.uv_attributes.offset = Vec2f::new(12.0, 34.0);
    });

    let before = document.face_at(side).expect("a face").uv_attributes;
    assert!(document.copy_face_attributes(top, side, FaceAttributeTransfer::MaterialOnly, false));

    let after = document.face_at(side).expect("a face");
    assert_eq!(after.material_name, "brick");
    assert_eq!(after.uv_attributes, before);
}

#[test]
fn copying_the_mapping_carries_the_offsets_too() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    let side = face_facing(&document, Vec3d::new(1.0, 0.0, 0.0));

    document.select_face_only(top);
    document.edit_faces(|face| {
        face.material_name = "brick".into();
        face.uv_attributes.offset = Vec2f::new(12.0, 34.0);
        face.uv_attributes.rotation = 45.0;
    });

    assert!(document.copy_face_attributes(top, side, FaceAttributeTransfer::Projection, false));

    let after = document.face_at(side).expect("a face");
    assert_eq!(after.material_name, "brick");
    assert_eq!(after.uv_attributes.offset, Vec2f::new(12.0, 34.0));
    geometry_math::assert_approx_eq!(after.uv_attributes.rotation, 45.0, 1e-6);
}

#[test]
fn a_double_click_puts_the_material_on_the_whole_brush() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    let side = face_facing(&document, Vec3d::new(1.0, 0.0, 0.0));

    document.select_face_only(top);
    document.edit_faces(|face| face.material_name = "brick".into());

    assert!(document.copy_face_attributes(top, side, FaceAttributeTransfer::MaterialOnly, true));

    let brush = document.brush(side.node).expect("a brush");
    assert!(brush
        .faces()
        .iter()
        .all(|face| face.material_name == "brick"));
}

#[test]
fn a_copy_needs_exactly_one_face_selected() {
    let mut document = one_box();
    // None selected.
    assert_eq!(document.single_selected_face(), None);

    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    let side = face_facing(&document, Vec3d::new(1.0, 0.0, 0.0));
    document.select_face_only(top);
    assert_eq!(document.single_selected_face(), Some(top));

    // Two selected, so there is no saying which one to copy from.
    document.toggle_face_selected(side);
    assert_eq!(document.single_selected_face(), None);
}

#[test]
fn copying_is_one_step_of_undo() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    let side = face_facing(&document, Vec3d::new(1.0, 0.0, 0.0));

    document.select_face_only(top);
    document.edit_faces(|face| face.material_name = "brick".into());
    let before = document
        .face_at(side)
        .expect("a face")
        .material_name
        .clone();

    document.copy_face_attributes(top, side, FaceAttributeTransfer::Projection, false);
    assert_eq!(
        document.face_at(side).expect("a face").material_name,
        "brick"
    );

    document.undo();
    assert_eq!(
        document.face_at(side).expect("a face").material_name,
        before
    );
}

#[test]
fn a_swept_transfer_touches_every_face_in_one_step() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    let side = face_facing(&document, Vec3d::new(1.0, 0.0, 0.0));
    let front = face_facing(&document, Vec3d::new(0.0, 1.0, 0.0));

    document.select_face_only(top);
    document.edit_faces(|face| face.material_name = "brick".into());

    let changed = document.copy_face_attributes_across(
        top,
        &[side, front],
        FaceAttributeTransfer::MaterialOnly,
    );
    assert_eq!(changed, 2);
    assert_eq!(
        document.face_at(side).expect("a face").material_name,
        "brick"
    );
    assert_eq!(
        document.face_at(front).expect("a face").material_name,
        "brick"
    );

    // One undo takes both back.
    document.undo();
    assert_ne!(
        document.face_at(side).expect("a face").material_name,
        "brick"
    );
    assert_ne!(
        document.face_at(front).expect("a face").material_name,
        "brick"
    );
}

#[test]
fn a_swept_transfer_that_changes_nothing_records_nothing() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    document.select_face_only(top);
    document.edit_faces(|face| face.material_name = "brick".into());

    // The source face already wears what it would be given, so nothing changes.
    assert_eq!(
        document.copy_face_attributes_across(top, &[top], FaceAttributeTransfer::MaterialOnly),
        0
    );
    // The only step in the history is the setup's `edit_faces`; the transfer added none.
    assert!(document.undo().is_some());
    assert!(document.undo().is_none());
}

/* ========== the paint tool ========== */

#[test]
fn material_assignment_reveals_textures_over_paint_and_preserves_uvs() {
    let mut document = one_box();
    document.select_all();
    document.set_face_color(Some(map_model::brush_face::FaceColor::solid(
        map_model::brush_face::Rgb::new(120, 160, 200),
    )));
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    let side = face_facing(&document, Vec3d::new(1.0, 0.0, 0.0));
    let original_uv = document.face_at(top).unwrap().uv_attributes;
    // Face selection takes precedence even if its brush is selected too.
    document.select_face_only(top);
    document.selection.insert(top.node);
    let before = document.to_map_text();
    document.set_current_material("brick");
    assert_eq!(document.apply_current_material(), 1);
    assert_eq!(document.face_at(top).unwrap().material_name, "brick");
    assert!(document.face_at(top).unwrap().color.is_none());
    assert!(document.face_at(side).unwrap().color.is_some());
    assert_eq!(document.face_at(top).unwrap().uv_attributes, original_uv);
    let after = document.to_map_text();
    assert_eq!(
        document.apply_current_material(),
        0,
        "repeated clicks must not add undo steps"
    );
    document.undo().unwrap();
    assert_eq!(document.to_map_text(), before);
    document.redo().unwrap();
    assert_eq!(document.to_map_text(), after);
}

#[test]
fn paint_tool_reveals_the_same_material_under_a_colour_override() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    document.select_face_only(top);
    document.set_face_color(Some(map_model::brush_face::FaceColor::solid(
        map_model::brush_face::Rgb::new(120, 160, 200),
    )));
    let before = document.to_map_text();
    let material = document.face_at(top).unwrap().material_name.to_string();
    let uv = document.face_at(top).unwrap().uv_attributes;
    assert!(document.paint_face(top, &material));
    assert!(document.face_at(top).unwrap().color.is_none());
    assert_eq!(document.face_at(top).unwrap().uv_attributes, uv);
    assert!(!document.paint_face(top, &material));
    document.undo().unwrap();
    assert_eq!(document.to_map_text(), before);
}

#[test]
fn painting_a_face_sets_only_that_face() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    let side = face_facing(&document, Vec3d::new(1.0, 0.0, 0.0));
    let untouched = document
        .face_at(side)
        .expect("a face")
        .material_name
        .clone();

    assert!(document.paint_face(top, "brick"));
    assert_eq!(
        document.face_at(top).expect("a face").material_name,
        "brick"
    );
    assert_eq!(
        document.face_at(side).expect("a face").material_name,
        untouched
    );
}

#[test]
fn painting_a_face_the_colour_it_already_is_records_nothing() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    assert!(document.paint_face(top, "brick"));
    // A second paint with the same material changes nothing, so there is no undo step
    // to leave behind.
    assert!(!document.paint_face(top, "brick"));
    assert!(document.undo().is_some());
    assert_ne!(
        document.face_at(top).expect("a face").material_name,
        "brick"
    );
    // And that was the only step: a second undo has nothing to do.
    assert!(document.undo().is_none());
}

#[test]
fn painting_a_brush_covers_every_face_in_one_step() {
    let mut document = one_box();
    let node = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0)).node;

    assert!(document.paint_brush(node, "brick"));
    assert!(document
        .brush(node)
        .expect("a brush")
        .faces()
        .iter()
        .all(|face| face.material_name == "brick"));

    document.undo();
    assert!(document
        .brush(node)
        .expect("a brush")
        .faces()
        .iter()
        .all(|face| face.material_name != "brick"));
}

/* ========== replacing a material ========== */

#[test]
fn replacing_a_material_swaps_every_face_that_wears_it() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    let side = face_facing(&document, Vec3d::new(1.0, 0.0, 0.0));
    document.paint_face(top, "brick");
    document.paint_face(side, "brick");

    assert_eq!(document.replaceable_face_count("brick"), 2);
    assert_eq!(document.replace_material("brick", "grass"), 2);

    let brush = document.brush(top.node).expect("a brush");
    assert_eq!(
        brush
            .faces()
            .iter()
            .filter(|f| f.material_name == "brick")
            .count(),
        0
    );
    assert_eq!(
        brush
            .faces()
            .iter()
            .filter(|f| f.material_name == "grass")
            .count(),
        2
    );
}

#[test]
fn replacing_a_material_is_one_undo_step() {
    let mut document = one_box();
    document.paint_brush(
        face_facing(&document, Vec3d::new(0.0, 0.0, 1.0)).node,
        "brick",
    );

    assert_eq!(document.replace_material("brick", "grass"), 6);
    document.undo();

    let brush = document
        .brush(face_facing(&document, Vec3d::new(0.0, 0.0, 1.0)).node)
        .expect("a brush");
    assert!(brush.faces().iter().all(|f| f.material_name == "brick"));
}

#[test]
fn replacing_a_material_with_itself_changes_nothing() {
    let mut document = one_box();
    document.paint_face(face_facing(&document, Vec3d::new(0.0, 0.0, 1.0)), "brick");
    assert_eq!(document.replace_material("brick", "brick"), 0);
}

#[test]
fn replacing_a_material_inside_a_face_selection_leaves_the_rest() {
    let mut document = one_box();
    let node = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0)).node;
    document.paint_brush(node, "brick");

    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    document.select_face_only(top);
    // With a face selected the replace is scoped to it, so only one of the six changes.
    assert_eq!(document.replaceable_face_count("brick"), 1);
    assert_eq!(document.replace_material("brick", "grass"), 1);

    let brush = document.brush(node).expect("a brush");
    assert_eq!(
        brush
            .faces()
            .iter()
            .filter(|f| f.material_name == "grass")
            .count(),
        1
    );
    assert_eq!(
        brush
            .faces()
            .iter()
            .filter(|f| f.material_name == "brick")
            .count(),
        5
    );
}

/* ========== entities in the 3D view ========== */

#[test]
fn a_selected_linked_group_shows_where_its_copies_are() {
    let mut document = one_box();
    document.select_all();
    document.group_selection("crate");
    document.create_linked_duplicate(64.0).expect("a copy");

    let groups: Vec<NodeId> = node::groups_with_ids(&document.tree.world)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert_eq!(groups.len(), 2);

    // With one of the set picked, a line runs to the other.
    document.select_only(groups[0]);
    let links = document.linked_group_links();
    assert_eq!(links.len(), 1);
    assert!(links[0].0 != links[0].1, "the two copies are apart");

    // From the other one it runs the other way.
    document.select_only(groups[1]);
    let back = document.linked_group_links();
    assert_eq!(back.len(), 1);
    assert_eq!((back[0].0, back[0].1), (links[0].1, links[0].0));

    // There is no single selected group and no open group to use as a fallback.
    document.select_only(groups[0]);
    document.toggle_selected(groups[1]);
    assert!(document.linked_group_links().is_empty());
}

#[test]
fn an_unlinked_group_has_no_copies_to_point_at() {
    let mut document = one_box();
    document.select_all();
    let group = document.group_selection("crate").expect("a group");
    document.select_only(group);
    assert!(document.linked_group_links().is_empty());
}

#[test]
fn a_link_is_drawn_for_every_key_that_aims_at_a_name() {
    let mut document = Document::new(WorldBounds::default());
    let button = document.create_point_entity("func_button", Vec3d::zero());
    let door = document.create_point_entity("func_door", Vec3d::new(128.0, 0.0, 0.0));
    let doomed = document.create_point_entity("func_wall", Vec3d::new(0.0, 128.0, 0.0));
    document.set_entity_property(door, "targetname", "d1");
    document.set_entity_property(doomed, "targetname", "w1");
    document.set_entity_property(button, "target", "d1");
    // `killtarget` links too, which the old reader only knew as a validator rule.
    document.set_entity_property(button, "killtarget", "w1");

    let links = document.entity_links();
    assert_eq!(links.len(), 2, "{links:?}");
    assert!(links.iter().all(|link| link.from == Vec3d::zero()));
    assert!(links.iter().any(|link| link.to.x() == 128.0));
    assert!(links.iter().any(|link| link.to.y() == 128.0));
}

#[test]
fn a_numbered_key_links_the_way_the_plain_one_does() {
    let mut document = Document::new(WorldBounds::default());
    let button = document.create_point_entity("func_button", Vec3d::zero());
    let door = document.create_point_entity("func_door", Vec3d::new(128.0, 0.0, 0.0));
    document.set_entity_property(door, "targetname", "d1");
    document.set_entity_property(button, "target2", "d1");
    assert_eq!(document.entity_links().len(), 1);
}

#[test]
fn the_selections_chain_reaches_further_than_what_touches_it() {
    // A targets B, B targets C. With A selected, one link touches the selection and
    // both are in its chain: `CollectTransitiveSelectedLinksVisitor`.
    let mut document = Document::new(WorldBounds::default());
    let a = document.create_point_entity("func_button", Vec3d::zero());
    let b = document.create_point_entity("func_door", Vec3d::new(128.0, 0.0, 0.0));
    let c = document.create_point_entity("func_door", Vec3d::new(256.0, 0.0, 0.0));
    document.set_entity_property(b, "targetname", "b");
    document.set_entity_property(c, "targetname", "c");
    document.set_entity_property(a, "target", "b");
    document.set_entity_property(b, "target", "c");
    document.select_only(a);

    let links = document.entity_links();
    assert_eq!(links.len(), 2);
    assert_eq!(
        links.iter().filter(|link| link.touches_selection).count(),
        1
    );
    assert_eq!(
        links.iter().filter(|link| link.reaches_selection).count(),
        2
    );

    use crate::view_filters::EntityLinkMode;
    let showing = |mode: EntityLinkMode| links.iter().filter(|link| mode.shows(link)).count();
    assert_eq!(showing(EntityLinkMode::All), 2);
    assert_eq!(showing(EntityLinkMode::Transitive), 2);
    assert_eq!(showing(EntityLinkMode::Connected), 1);
    assert_eq!(showing(EntityLinkMode::None), 0);
}

#[test]
fn nothing_selected_means_no_chain_to_follow() {
    let mut document = Document::new(WorldBounds::default());
    let a = document.create_point_entity("func_button", Vec3d::zero());
    let b = document.create_point_entity("func_door", Vec3d::new(128.0, 0.0, 0.0));
    document.set_entity_property(b, "targetname", "b");
    document.set_entity_property(a, "target", "b");
    document.clear_selection();

    let links = document.entity_links();
    assert_eq!(links.len(), 1);
    assert!(!links[0].touches_selection);
    assert!(!links[0].reaches_selection);
}

#[test]
fn a_point_entity_is_drawn_as_a_box() {
    let mut document = one_box();
    let before = document
        .triangles(&crate::view_filters::ViewFilters {
            show_point_entity_models: false,
            ..Default::default()
        })
        .len();

    document.create_point_entity("info_player_start", Vec3d::new(64.0, 0.0, 0.0));

    // Twelve triangles more: the six faces of the box the class is given.
    assert_eq!(
        document
            .triangles(&crate::view_filters::ViewFilters {
                show_point_entity_models: false,
                ..Default::default()
            })
            .len(),
        before + 12
    );
}

#[test]
fn an_entity_is_drawn_in_its_own_colour_and_a_brush_is_not() {
    let mut document = one_box();
    document.create_point_entity("info_player_start", Vec3d::zero());

    let triangles = document.triangles(&crate::view_filters::ViewFilters {
        show_point_entity_models: false,
        ..Default::default()
    });
    let entity_colour = triangles
        .last()
        .expect("the entity's triangles come last")
        .color;
    let brush_colour = triangles[0].color;

    assert_ne!(entity_colour, brush_colour);
    // A brush with no material takes `Preferences::FaceColor` as its albedo. White
    // was tried and made every untextured face clip to the top of the tone map.
    assert_eq!(
        brush_colour,
        crate::preferences::pref(|p| p.face_color).rgb_f32()
    );
}

#[test]
fn a_brush_wearing_a_material_is_white_so_the_material_comes_through() {
    // The face colour is the albedo of a surface with nothing on it; a surface with a
    // material takes white, or the material would be tinted by it.
    assert_eq!(surface_color(Some(0)), [1.0, 1.0, 1.0]);
    assert_eq!(
        surface_color(None),
        crate::preferences::pref(|p| p.face_color).rgb_f32()
    );
}

#[test]
fn every_face_of_an_entity_box_faces_outwards() {
    let mut document = one_box();
    document.clear_selection();
    document.create_point_entity("info_player_start", Vec3d::new(256.0, 0.0, 0.0));

    // The entity's own triangles are the last twelve.
    let triangles = document.triangles(&crate::view_filters::ViewFilters {
        show_point_entity_models: false,
        ..Default::default()
    });
    let entity = &triangles[triangles.len() - 12..];
    let centre = Vec3d::new(256.0, 0.0, 0.0);

    for triangle in entity {
        let middle = (triangle.positions[0] + triangle.positions[1] + triangle.positions[2]) / 3.0;
        // The normal points away from the middle of the box, which is what makes the
        // renderer's back-face culling keep the right half.
        assert!(
            vec::dot(triangle.normal, middle - centre) > 0.0,
            "a face points inwards"
        );
    }
}

/* ========== layers ========== */

#[test]
fn a_new_document_has_one_layer_that_cannot_be_removed() {
    let mut document = one_box();
    let layers = document.layers();
    assert_eq!(layers.len(), 1);
    assert!(layers[0].is_default);
    assert_eq!(layers[0].objects, 1);

    // The default layer is the worldspawn itself.
    assert!(!document.remove_layer(0));
    assert_eq!(document.layers().len(), 1);
}

#[test]
fn the_window_title_shows_the_name_and_whether_there_are_unsaved_changes() {
    let mut document = one_box();
    assert_eq!(document.window_title(), "untitled — Heron Brush");

    // Any edit brings up the unsaved-changes mark.
    document.select_all();
    document.translate_selection(Vec3d::new(16.0, 0.0, 0.0));
    assert_eq!(document.window_title(), "• untitled — Heron Brush");

    // Saving clears it and takes the file's name.
    let path = std::env::temp_dir().join(format!("heron-title-{}.map", std::process::id()));
    document.save_as(&path).expect("it saves");
    assert_eq!(
        document.window_title(),
        format!(
            "{} — Heron Brush",
            path.file_name().unwrap().to_string_lossy()
        )
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_new_brush_lands_on_the_current_layer() {
    let mut document = one_box();
    let second = document.add_layer("Detail");
    assert_eq!(document.current_layer(), second);

    let world = document.world_bounds();
    let brush = map_model::shapes::create_cuboid(&world, &Bbox3d::from_scalars(-8.0, 8.0), "stone")
        .expect("a cube");
    document.add_brushes(vec![brush]);

    let layers = document.layers();
    assert_eq!(layers[0].objects, 1, "the first box stayed put");
    assert_eq!(layers[1].objects, 1, "the new one went to the new layer");
}

#[test]
fn a_hidden_layer_is_not_drawn_and_cannot_be_picked() {
    let mut document = one_box();
    assert!(!document
        .triangles(&crate::view_filters::ViewFilters::default())
        .is_empty());

    document.set_layer_hidden(0, true);
    assert!(document
        .triangles(&crate::view_filters::ViewFilters::default())
        .is_empty());
    assert!(document.edges().is_empty());

    // And a ray straight at it reaches nothing.
    let ray = Ray3d {
        origin: Vec3d::new(0.0, 0.0, 512.0),
        direction: Vec3d::new(0.0, 0.0, -1.0),
    };
    assert!(document.pick(&ray).is_none());

    document.set_layer_hidden(0, false);
    assert!(!document
        .triangles(&crate::view_filters::ViewFilters::default())
        .is_empty());
    assert!(document.pick(&ray).is_some());
}

#[test]
fn a_locked_layer_is_drawn_but_cannot_be_picked() {
    let mut document = one_box();
    document.set_layer_locked(0, true);

    // Still there to look at.
    assert!(!document
        .triangles(&crate::view_filters::ViewFilters::default())
        .is_empty());

    // But not to click on.
    let ray = Ray3d {
        origin: Vec3d::new(0.0, 0.0, 512.0),
        direction: Vec3d::new(0.0, 0.0, -1.0),
    };
    assert!(document.pick(&ray).is_none());
}

#[test]
fn hiding_a_layer_drops_what_was_selected_on_it() {
    // A selection nobody can see is a selection that surprises whoever edits next.
    let mut document = one_box();
    document.select_all();
    assert!(document.has_selection());

    document.set_layer_hidden(0, true);
    assert!(!document.has_selection());
}

#[test]
fn select_all_leaves_a_locked_layer_alone() {
    let mut document = one_box();
    document.set_layer_locked(0, true);
    document.select_all();
    assert_eq!(document.selection_count(), 0);

    document.set_layer_locked(0, false);
    document.select_all();
    assert_eq!(document.selection_count(), 1);
}

#[test]
fn marquee_preview_grows_shrinks_and_restores_without_history_edits() {
    let mut document = Document::new(WorldBounds::default());
    let a = add_box(&mut document, Vec3d::zero(), Vec3d::new(32.0, 32.0, 32.0));
    let b = add_box(
        &mut document,
        Vec3d::new(64.0, 0.0, 0.0),
        Vec3d::new(96.0, 32.0, 32.0),
    );
    document.select_only(a);
    let initial = document.selection_snapshot();
    document.preview_marquee(&initial, &[b], true);
    assert_eq!(document.selection_ids(), &HashSet::from([a, b]));
    let revision = document.revision();
    document.preview_marquee(&initial, &[b], true);
    assert_eq!(
        document.revision(),
        revision,
        "unchanged preview should not invalidate the scene"
    );
    document.preview_marquee(&initial, &[], true);
    assert_eq!(
        document.selection_ids(),
        &HashSet::from([a]),
        "shrinking removes only newly caught objects"
    );
    document.preview_marquee(&initial, &[b], false);
    assert_eq!(document.selection_ids(), &HashSet::from([b]));
    document.restore_selection(&initial);
    assert_eq!(document.selection_ids(), &HashSet::from([a]));
    document.undo().unwrap();
    assert_eq!(
        document.brush_count(),
        1,
        "preview must not add an Undo operation"
    );
}

#[test]
fn marquee_preview_selects_a_group_once_and_cancel_restores_faces() {
    let mut document = Document::new(WorldBounds::default());
    let a = add_box(&mut document, Vec3d::zero(), Vec3d::new(32.0, 32.0, 32.0));
    let b = add_box(
        &mut document,
        Vec3d::new(64.0, 0.0, 0.0),
        Vec3d::new(96.0, 32.0, 32.0),
    );
    document.select_all();
    document.group_selection("Pair");
    document.clear_selection();
    let initial = document.selection_snapshot();
    document.preview_marquee(&initial, &[a, b], false);
    assert_eq!(document.selection_ids(), &HashSet::from([a, b]));
    document.preview_marquee(&initial, &[a], false);
    assert_eq!(document.selection_ids(), &HashSet::from([a, b]));
    document.preview_marquee(&initial, &[], false);
    assert!(!document.has_selection());
    let face = FaceRef { node: a, face: 0 };
    document.select_face_only(face);
    let initial = document.selection_snapshot();
    document.preview_marquee(&initial, &[b], false);
    document.restore_selection(&initial);
    assert!(document.is_face_selected(face));
    assert!(!document.has_selection());
}

/// Adds an axis-aligned box brush spanning `min..max` and returns its id, leaving the
/// selection as it was.
fn add_box(document: &mut Document, min: Vec3d, max: Vec3d) -> NodeId {
    add_box_of(document, min, max, "stone")
}

/// [`add_box`] with a chosen material on every face.
fn add_box_of(document: &mut Document, min: Vec3d, max: Vec3d, material: &str) -> NodeId {
    let kept: Vec<NodeId> = document.selection.iter().copied().collect();
    let world = document.world_bounds();
    let brush =
        map_model::shapes::create_cuboid(&world, &Bbox3d::new(min, max), material).expect("a box");
    document.add_brushes(vec![brush]);
    let id = node::brushes_with_ids(&document.tree.world)
        .into_iter()
        .map(|(id, _)| id)
        .next_back()
        .expect("the box that was just added");
    document.selection = kept.into_iter().collect();
    id
}

#[test]
fn select_touching_takes_what_the_lasso_overlaps_and_eats_the_lasso() {
    let mut document = Document::new(WorldBounds::default());
    let lasso = add_box(
        &mut document,
        Vec3d::new(-40.0, -40.0, -40.0),
        Vec3d::new(40.0, 40.0, 40.0),
    );
    let overlapping = add_box(
        &mut document,
        Vec3d::new(20.0, 20.0, 20.0),
        Vec3d::new(80.0, 80.0, 80.0),
    );
    let far = add_box(
        &mut document,
        Vec3d::new(200.0, 200.0, 200.0),
        Vec3d::new(240.0, 240.0, 240.0),
    );

    document.selection = std::iter::once(lasso).collect();
    assert!(document.select_touching());

    assert_eq!(document.brush_count(), 2, "the lasso brush is consumed");
    assert!(document.is_selected(overlapping));
    assert!(!document.is_selected(far));
}

#[test]
fn select_inside_takes_only_what_is_wholly_enclosed() {
    let mut document = Document::new(WorldBounds::default());
    let lasso = add_box(
        &mut document,
        Vec3d::new(-100.0, -100.0, -100.0),
        Vec3d::new(100.0, 100.0, 100.0),
    );
    let within = add_box(
        &mut document,
        Vec3d::new(-20.0, -20.0, -20.0),
        Vec3d::new(20.0, 20.0, 20.0),
    );
    let straddling = add_box(
        &mut document,
        Vec3d::new(50.0, 50.0, 50.0),
        Vec3d::new(150.0, 150.0, 150.0),
    );

    document.selection = std::iter::once(lasso).collect();
    assert!(document.select_inside());

    assert!(document.is_selected(within));
    assert!(!document.is_selected(straddling));
}

#[test]
fn select_tall_reaches_along_the_view_axis() {
    let mut document = Document::new(WorldBounds::default());
    // A thin lasso slab high up, footprint 0..64 in X and Y.
    let lasso = add_box(
        &mut document,
        Vec3d::new(0.0, 0.0, 400.0),
        Vec3d::new(64.0, 64.0, 432.0),
    );
    // Under the footprint, far below -- only "tall" selection reaches it.
    let below = add_box(
        &mut document,
        Vec3d::new(8.0, 8.0, -200.0),
        Vec3d::new(56.0, 56.0, -160.0),
    );
    // Same height as `below` but outside the footprint in X.
    let aside = add_box(
        &mut document,
        Vec3d::new(200.0, 8.0, -200.0),
        Vec3d::new(248.0, 56.0, -160.0),
    );

    document.selection = std::iter::once(lasso).collect();
    assert!(document.select_tall(2), "stretched along Z");

    assert!(document.is_selected(below));
    assert!(!document.is_selected(aside));
}

#[test]
fn corner_extrude_is_one_undo_step_and_can_roll_back_to_start() {
    let mut document = Document::new(WorldBounds::default());
    add_box(&mut document, Vec3d::zero(), Vec3d::new(32.0, 32.0, 32.0));
    document.select_all();
    let ray = Ray3d::new(Vec3d::new(64.0, 64.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    let (brush, corner) = document.extrude_corner_2d(&ray).expect("a corner");
    let moves: Vec<_> = corner
        .faces
        .iter()
        .map(|(index, _)| {
            (
                brush.face_polygon(*index).unwrap(),
                Vec3d::new(16.0, 8.0, 0.0),
            )
        })
        .collect();
    document.begin_transaction();
    assert!(document.extrude_face_planes(&moves));
    geometry_math::assert_approx_eq!(
        document.selection_bounds().unwrap().max,
        Vec3d::new(48.0, 40.0, 32.0),
        1e-6
    );
    document.rollback_transaction();
    geometry_math::assert_approx_eq!(
        document.selection_bounds().unwrap().max,
        brush.bounds().max,
        1e-6
    );
    assert!(document.extrude_face_planes(&moves));
    assert!(document.commit_transaction());
    assert!(document.undo().is_some());
    geometry_math::assert_approx_eq!(
        document.selection_bounds().unwrap().max,
        brush.bounds().max,
        1e-6
    );
    assert!(document.redo().is_some());
    geometry_math::assert_approx_eq!(
        document.selection_bounds().unwrap().max,
        Vec3d::new(48.0, 40.0, 32.0),
        1e-6
    );
}

#[test]
fn the_extrude_preview_outlines_the_hit_face_across_the_selection() {
    let mut document = Document::new(WorldBounds::default());
    let world = document.world_bounds();
    for offset in [0.0, 64.0] {
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(
                Vec3d::new(offset, -32.0, -32.0),
                Vec3d::new(offset + 32.0, 32.0, 32.0),
            ),
            "stone",
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
    }
    document.select_all();

    // A ray straight down onto the top of the first cube.
    let ray = Ray3d::new(Vec3d::new(16.0, 0.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    let outlines = document.extrude_preview_faces(&ray);

    // The top face of each selected cube -- both are coplanar at z = 32.
    assert_eq!(outlines.len(), 2);
    for outline in &outlines {
        assert!(outline.iter().all(|p| (p.z() - 32.0).abs() < 1e-6));
    }

    // Nothing hit, nothing outlined.
    let miss = Ray3d::new(Vec3d::new(500.0, 500.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    assert!(document.extrude_preview_faces(&miss).is_empty());
}

#[test]
fn the_flat_view_extrude_preview_outlines_the_face_behind_the_silhouette_edge() {
    let mut document = Document::new(WorldBounds::default());
    let world = document.world_bounds();
    for offset in [0.0, 64.0] {
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(
                Vec3d::new(offset, -32.0, -32.0),
                Vec3d::new(offset + 32.0, 32.0, 32.0),
            ),
            "stone",
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
    }
    document.select_all();

    // A Front view looks along Y; a ray grazing the top edge of the first cube picks
    // its top face's silhouette.
    let ray = Ray3d::new(Vec3d::new(16.0, -200.0, 32.0), Vec3d::new(0.0, 1.0, 0.0));
    let outlines = document.extrude_preview_faces_2d(&ray);

    // The top face of each selected cube, both coplanar at z = 32.
    assert_eq!(outlines.len(), 2);
    for outline in &outlines {
        assert!(outline.iter().all(|p| (p.z() - 32.0).abs() < 1e-6));
    }

    // With the selection cleared there is nothing to extrude.
    document.clear_selection();
    let ray = Ray3d::new(Vec3d::new(16.0, -200.0, 32.0), Vec3d::new(0.0, 1.0, 0.0));
    assert!(document.extrude_preview_faces_2d(&ray).is_empty());
}

/// A group of patches used to report itself empty, so it had no outline, a click on
/// it selected nothing, and a linked duplicate of it was refused.
#[test]
fn an_open_group_leaves_the_rest_of_the_map_alone() {
    // Two boxes, with one inside an open group. Objects outside the open group cannot be
    // selected.
    let mut document = two_boxes();
    let ids: Vec<NodeId> = node::brushes_with_ids(&document.tree.world)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert_eq!(ids.len(), 2);
    document.select_only(ids[0]);
    document.group_selection("a crate");
    assert!(document.open_selected_group());

    let inside = ids[0];
    let outside = ids[1];
    assert!(document
        .selectable_brushes()
        .iter()
        .any(|(id, _)| *id == inside));
    assert!(
        !document
            .selectable_brushes()
            .iter()
            .any(|(id, _)| *id == outside),
        "the brush outside the open group is out of reach"
    );

    document.select_all();
    assert_eq!(
        document.selection_count(),
        1,
        "Select All stays in the group"
    );
    assert!(document.is_selected(inside));

    // Closing the group brings the rest of the map back.
    document.close_group();
    document.clear_selection();
    document.select_all();
    assert_eq!(document.selection_count(), 2);
}

#[test]
fn a_group_of_patches_holds_them() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    document.select_face_only(top);
    assert_eq!(document.convert_selection_to_patches(3), 1);
    document.select_all();
    let group = document.group_selection("a patch").expect("a group");

    let contents = node::contents_of(&document.tree.world, group);
    assert_eq!(contents.len(), 1, "the patch is what the group holds");
    // And the group answers to a click, which goes through the same contents.
    document.clear_selection();
    document.select_only(contents[0]);
    assert_eq!(document.selection_count(), 1);
}

#[test]
fn converting_a_face_to_a_patch_replaces_the_brush() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    document.select_face_only(top);

    let made = document.convert_selection_to_patches(3);
    assert_eq!(made, 1, "a quad face makes one patch");
    assert_eq!(document.brush_count(), 0, "the brush is gone");
    assert_eq!(
        node::patches_by_layer(&document.tree.world).len(),
        1,
        "and a patch is in its place"
    );

    document.undo();
    assert_eq!(document.brush_count(), 1);
    assert_eq!(node::patches_by_layer(&document.tree.world).len(), 0);
}

#[test]
fn converting_with_no_face_selection_does_nothing() {
    let mut document = one_box();
    assert_eq!(document.convert_selection_to_patches(3), 0);
    assert_eq!(document.brush_count(), 1);
}

#[test]
fn hiding_the_selection_drops_it_from_the_drawn_geometry_and_from_select_all() {
    let mut document = two_boxes();
    let first = node::brushes_with_ids(&document.tree.world)[0].0;
    document.selection = std::iter::once(first).collect();

    assert_eq!(document.hide_selection(), 1);
    assert!(
        !document.has_selection(),
        "a hidden object cannot stay selected"
    );

    document.select_all();
    assert!(
        !document.is_selected(first),
        "select-all skips the hidden one"
    );
    assert_eq!(document.selection_count(), 1, "only the visible box");

    assert_eq!(document.show_all(), 1);
    document.select_all();
    assert_eq!(document.selection_count(), 2);
}

#[test]
fn isolating_hides_everything_but_the_selection() {
    let mut document = two_boxes();
    let first = node::brushes_with_ids(&document.tree.world)[0].0;
    document.selection = std::iter::once(first).collect();

    assert_eq!(document.isolate_selection(), 1, "the other box is hidden");
    assert!(document.is_selected(first), "the selection stays");

    document.select_all();
    assert_eq!(document.selection_count(), 1);

    document.show_all();
    document.select_all();
    assert_eq!(document.selection_count(), 2);
}

#[test]
fn a_hidden_brush_cannot_be_picked() {
    let mut document = one_box();
    let id = node::brushes_with_ids(&document.tree.world)[0].0;
    document.selection = std::iter::once(id).collect();
    document.hide_selection();

    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    assert!(
        document.pick(&ray).is_none(),
        "the ray passes through the hidden box"
    );
}

#[test]
fn select_by_brush_needs_a_selection_of_only_brushes() {
    let mut document = one_entity();
    let entity = node::entities_with_ids(&document.tree.world)
        .into_iter()
        .map(|(id, _)| id)
        .next()
        .expect("an entity");
    document.selection = std::iter::once(entity).collect();
    assert!(!document.select_touching());
    assert!(!document.select_inside());
}

#[test]
fn select_touching_is_one_undo_step() {
    let mut document = Document::new(WorldBounds::default());
    let lasso = add_box(
        &mut document,
        Vec3d::new(-40.0, -40.0, -40.0),
        Vec3d::new(40.0, 40.0, 40.0),
    );
    add_box(
        &mut document,
        Vec3d::new(20.0, 20.0, 20.0),
        Vec3d::new(80.0, 80.0, 80.0),
    );

    document.selection = std::iter::once(lasso).collect();
    document.select_touching();
    assert_eq!(document.brush_count(), 1);

    document.undo();
    assert_eq!(document.brush_count(), 2, "one press brings the lasso back");
}

#[test]
fn select_siblings_takes_the_rest_of_a_group() {
    let mut document = Document::new(WorldBounds::default());
    let a = add_box(
        &mut document,
        Vec3d::new(-16.0, -16.0, -16.0),
        Vec3d::new(16.0, 16.0, 16.0),
    );
    let b = add_box(
        &mut document,
        Vec3d::new(48.0, -16.0, -16.0),
        Vec3d::new(80.0, 16.0, 16.0),
    );
    let loose = add_box(
        &mut document,
        Vec3d::new(200.0, -16.0, -16.0),
        Vec3d::new(232.0, 16.0, 16.0),
    );

    document.selection = [a, b].into_iter().collect();
    document.group_selection("Doorway");
    document.selection = std::iter::once(a).collect();
    document.select_siblings();

    assert!(document.is_selected(a));
    assert!(document.is_selected(b));
    assert!(!document.is_selected(loose));
}

#[test]
fn adding_a_face_to_a_brush_selection_keeps_what_was_picked() {
    // `convertToFaceSelection`: with brushes selected, adding a face turns them into
    // all their faces rather than throwing them away.
    let mut document = two_boxes();
    let ids: Vec<NodeId> = node::brushes_with_ids(&document.tree.world)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    document.select_only(ids[0]);

    let face = FaceRef {
        node: ids[1],
        face: 0,
    };
    document.toggle_face_selected(face);

    assert_eq!(document.selection_count(), 0, "the brushes are faces now");
    assert_eq!(
        document.selected_faces().len(),
        7,
        "six faces of the selected box, and the one just added"
    );
    assert!(document.is_face_selected(face));

    // Clicking it again takes only that face away.
    document.toggle_face_selected(face);
    assert_eq!(document.selected_faces().len(), 6);
    assert!(!document.is_face_selected(face));
}

#[test]
fn select_faces_with_material_takes_only_the_faces_wearing_it() {
    let mut document = one_box();
    let top = face_facing(&document, Vec3d::new(0.0, 0.0, 1.0));
    document.select_face_only(top);
    document.edit_faces(|face| face.material_name = "brick".into());
    document.clear_selection();

    assert_eq!(document.select_faces_with_material("brick"), 1);
    assert_eq!(document.selected_faces().len(), 1);
    assert_eq!(
        document.selection_count(),
        0,
        "the brush itself is not taken"
    );

    // And the other five faces are the other material.
    assert_eq!(document.select_faces_with_material("stone"), 5);
    // A material nothing wears takes nothing, and clears what was there.
    assert_eq!(document.select_faces_with_material("nothing"), 0);
    assert!(document.selected_faces().is_empty());
}

#[test]
fn select_brushes_with_material_takes_every_brush_wearing_it() {
    let mut document = Document::new(WorldBounds::default());
    let stone_a = add_box_of(
        &mut document,
        Vec3d::new(-16.0, -16.0, -16.0),
        Vec3d::new(16.0, 16.0, 16.0),
        "stone",
    );
    let brick = add_box_of(
        &mut document,
        Vec3d::new(48.0, -16.0, -16.0),
        Vec3d::new(80.0, 16.0, 16.0),
        "brick",
    );
    let stone_b = add_box_of(
        &mut document,
        Vec3d::new(112.0, -16.0, -16.0),
        Vec3d::new(144.0, 16.0, 16.0),
        "STONE",
    );

    document.select_brushes_with_material("stone");

    assert!(document.is_selected(stone_a));
    assert!(document.is_selected(stone_b), "the match ignores case");
    assert!(!document.is_selected(brick));
}

#[test]
fn invert_selection_swaps_what_is_and_is_not_selected() {
    let mut document = Document::new(WorldBounds::default());
    let a = add_box(
        &mut document,
        Vec3d::new(-16.0, -16.0, -16.0),
        Vec3d::new(16.0, 16.0, 16.0),
    );
    let b = add_box(
        &mut document,
        Vec3d::new(48.0, -16.0, -16.0),
        Vec3d::new(80.0, 16.0, 16.0),
    );
    let c = add_box(
        &mut document,
        Vec3d::new(112.0, -16.0, -16.0),
        Vec3d::new(144.0, 16.0, 16.0),
    );

    document.selection = std::iter::once(a).collect();
    document.invert_selection();

    assert!(!document.is_selected(a));
    assert!(document.is_selected(b));
    assert!(document.is_selected(c));
}

#[test]
fn the_selection_can_be_moved_between_layers() {
    let mut document = one_box();
    let second = document.add_layer("Detail");

    document.set_current_layer(0);
    document.select_all();
    document.set_current_layer(second);
    assert_eq!(document.move_selection_to_layer(second), 1);

    let layers = document.layers();
    assert_eq!(layers[0].objects, 0);
    assert_eq!(layers[1].objects, 1);
}

#[test]
fn removing_a_layer_keeps_what_was_on_it() {
    let mut document = one_box();
    let second = document.add_layer("Detail");
    document.select_all();
    document.move_selection_to_layer(second);
    assert_eq!(document.layers()[1].objects, 1);

    assert!(document.remove_layer(second));
    let layers = document.layers();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].objects, 1, "the brush moved to the default layer");
}

#[test]
fn selecting_a_layer_takes_everything_on_it_and_nothing_else() {
    let mut document = one_box();
    let second = document.add_layer("Detail");
    let world = document.world_bounds();
    let brush = map_model::shapes::create_cuboid(&world, &Bbox3d::from_scalars(-8.0, 8.0), "stone")
        .expect("a cube");
    document.add_brushes(vec![brush]);

    assert_eq!(document.select_layer(0), 1);
    assert_eq!(document.select_layer(second), 1);

    // A hidden layer selects nothing, since nothing on it can be edited.
    document.set_layer_hidden(second, true);
    assert_eq!(document.select_layer(second), 0);
}

#[test]
fn a_layer_can_be_renamed() {
    let mut document = one_box();
    assert!(document.rename_layer(0, "Structure"));
    assert_eq!(document.layers()[0].name, "Structure");
    assert!(!document.rename_layer(9, "Nowhere"));
}

/* ========== groups ========== */

/// A document with two boxes, both selected.
fn two_boxes() -> Document {
    let mut document = Document::new(WorldBounds::default());
    let world = document.world_bounds();
    for offset in [0.0, 128.0] {
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(
                Vec3d::new(offset - 32.0, -32.0, -32.0),
                Vec3d::new(offset + 32.0, 32.0, 32.0),
            ),
            "stone",
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
    }
    document.select_all();
    document
}

/// A document holding one point entity, selected.
fn one_entity() -> Document {
    let mut document = Document::new(WorldBounds::default());
    document.create_point_entity("monster_army", Vec3d::new(64.0, 0.0, 16.0));
    document
}

/// The entity the document holds.
fn the_entity(document: &Document) -> &EntityNode {
    node::entities_with_ids(&document.tree.world)
        .into_iter()
        .map(|(_, entity)| entity)
        .next()
        .expect("an entity")
}

/// A hollow room with a point entity inside it, and one wall optionally missing.
fn sealed_room(missing: Option<usize>) -> Document {
    let mut document = Document::new(WorldBounds::default());
    let world = document.world_bounds();
    let outer = 80.0;
    let inner = 64.0;
    let walls = [
        (
            Vec3d::new(-outer, -outer, -outer),
            Vec3d::new(-inner, outer, outer),
        ),
        (
            Vec3d::new(inner, -outer, -outer),
            Vec3d::new(outer, outer, outer),
        ),
        (
            Vec3d::new(-outer, -outer, -outer),
            Vec3d::new(outer, -inner, outer),
        ),
        (
            Vec3d::new(-outer, inner, -outer),
            Vec3d::new(outer, outer, outer),
        ),
        (
            Vec3d::new(-outer, -outer, -outer),
            Vec3d::new(outer, outer, -inner),
        ),
        (
            Vec3d::new(-outer, -outer, inner),
            Vec3d::new(outer, outer, outer),
        ),
    ];
    let brushes: Vec<map_model::brush::Brush> = walls
        .into_iter()
        .enumerate()
        .filter(|(index, _)| Some(*index) != missing)
        .filter_map(|(_, (min, max))| {
            map_model::shapes::create_cuboid(&world, &Bbox3d::new(min, max), "stone").ok()
        })
        .collect();
    document.add_brushes(brushes);
    document.create_point_entity("info_player_start", Vec3d::zero());
    document
}

#[test]
fn a_sealed_room_leaves_no_trace_to_draw() {
    let mut document = sealed_room(None);
    assert_eq!(document.check_for_leaks(8.0), map_model::leak::Leak::Sealed);
    assert!(document.leak_trace().is_none());
}

#[test]
fn a_leaking_room_leaves_a_trace_the_views_can_draw() {
    let mut document = sealed_room(Some(0));
    assert!(matches!(
        document.check_for_leaks(8.0),
        map_model::leak::Leak::Leaked(_)
    ));
    let trace = document.leak_trace().expect("a trace to draw");
    assert!(trace.points.len() >= 2);
}

/* ========== repeating ========== */

#[test]
fn repeat_does_the_last_run_of_edits_to_whatever_is_selected_now() {
    let mut document = one_box();
    let world = document.world_bounds();
    let second = map_model::shapes::create_cuboid(
        &world,
        &Bbox3d::new(Vec3d::new(128.0, 0.0, 0.0), Vec3d::new(192.0, 64.0, 64.0)),
        "stone",
    )
    .expect("a second box");
    document.add_brushes(vec![second]);

    // Nothing has been done, so there is nothing to do again.
    assert!(!document.can_repeat());

    // Move the first box twice: one run, two edits.
    document.select_all();
    let brushes = map_model::node::brushes_with_ids(&document.tree.world);
    let (first_id, _) = *brushes.first().expect("a brush");
    let (second_id, _) = *brushes.get(1).expect("a second brush");
    document.select_only(first_id);
    assert!(document.translate_selection(Vec3d::new(0.0, 0.0, 16.0)));
    assert!(document.translate_selection(Vec3d::new(0.0, 0.0, 16.0)));
    assert!(document.can_repeat());

    // Select the other box and have the same happen to it.
    document.select_only(second_id);
    let before = document.selection_bounds().expect("a selection").center();
    assert!(document.repeat());
    let after = document.selection_bounds().expect("a selection").center();
    assert!(vec::length(after - before - Vec3d::new(0.0, 0.0, 32.0)) < 0.001);

    // Repeating does not itself become something to repeat, so it stays the same run.
    assert!(document.repeat());
    let again = document.selection_bounds().expect("a selection").center();
    assert!(vec::length(again - after - Vec3d::new(0.0, 0.0, 32.0)) < 0.001);
}

#[test]
fn an_edit_on_a_different_selection_starts_a_new_run() {
    let mut document = one_box();
    let world = document.world_bounds();
    let second = map_model::shapes::create_cuboid(
        &world,
        &Bbox3d::new(Vec3d::new(128.0, 0.0, 0.0), Vec3d::new(192.0, 64.0, 64.0)),
        "stone",
    )
    .expect("a second box");
    document.add_brushes(vec![second]);

    let brushes = map_model::node::brushes_with_ids(&document.tree.world);
    let (first_id, _) = *brushes.first().expect("a brush");
    let (second_id, _) = *brushes.get(1).expect("a second brush");

    document.select_only(first_id);
    document.translate_selection(Vec3d::new(0.0, 0.0, 16.0));
    document.translate_selection(Vec3d::new(0.0, 0.0, 16.0));

    // A move on the other box is a new run, so repeating does that one move rather
    // than all three.
    document.select_only(second_id);
    document.translate_selection(Vec3d::new(32.0, 0.0, 0.0));

    let before = document.selection_bounds().expect("a selection").center();
    assert!(document.repeat());
    let after = document.selection_bounds().expect("a selection").center();
    assert!(vec::length(after - before - Vec3d::new(32.0, 0.0, 0.0)) < 0.001);
}

#[test]
fn duplicate_preserves_nested_groups_layer_and_fresh_ids_through_undo_and_save() {
    let mut document = one_box();
    let layer = document.add_layer("Props");
    document.select_all();
    document.move_selection_to_layer(layer);
    document.set_current_layer(layer);
    document.group_selection("Inner").unwrap();
    let inner = document.tree.world.layers[layer].children.remove(0);
    let outer = Node::new(NodeKind::Group(Group {
        id: document.fresh_id(),
        name: "Outer".into(),
        linked_group_id: None,
        transformation: None,
        children: vec![inner],
    }));
    let original_id = outer.id;
    document.tree.world.layers[layer].children.push(outer);
    document.select_all();
    let original_selection = document.selection_ids().clone();
    let center = document.selection_bounds().unwrap().center();
    assert_eq!(
        document.duplicate_selection_by(Vec3d::new(128.0, 0.0, 0.0)),
        1
    );
    assert!(document.selection_ids().is_disjoint(&original_selection));
    assert!(
        vec::length(
            document.selection_bounds().unwrap().center() - center - Vec3d::new(128.0, 0.0, 0.0)
        ) < 0.001
    );
    assert_eq!(document.tree.world.layers[layer].children.len(), 2);
    let groups = node::groups_with_ids(&document.tree.world);
    assert_eq!(groups.len(), 4);
    let ids: HashSet<_> = groups.iter().map(|(_, group)| &group.id).collect();
    assert_eq!(ids.len(), 4, "serialized group IDs must be unique too");
    assert_eq!(groups.iter().filter(|(_, g)| g.name == "Inner").count(), 2);
    assert!(node::find_node(&document.tree.world, original_id).is_some());
    let text = document.to_map_text();
    let map = read_map(&text, None).unwrap();
    let reloaded = node::build_tree(&map, &document.world_bounds);
    assert_eq!(node::groups_with_ids(&reloaded.world).len(), 4);
    document.undo().unwrap();
    assert_eq!(document.brush_count(), 1);
    assert_eq!(node::groups_with_ids(&document.tree.world).len(), 2);
    assert!(
        !document.has_selection(),
        "Undo must not leave deleted child IDs selected"
    );
    document.redo().unwrap();
    assert_eq!(document.brush_count(), 2);
    assert_eq!(node::groups_with_ids(&document.tree.world).len(), 4);
}

#[test]
fn duplicate_inside_open_group_stays_inside_and_invalid_move_is_atomic() {
    let mut document = one_box();
    document.select_all();
    let group = document.group_selection("Editable").unwrap();
    assert!(document.open_selected_group());
    let original = document.selection_ids().clone();
    assert_eq!(
        document.duplicate_selection_by(Vec3d::new(128.0, 0.0, 0.0)),
        1
    );
    assert_eq!(node::contents_of(&document.tree.world, group).len(), 2);
    assert_eq!(node::groups_with_ids(&document.tree.world).len(), 1);
    assert!(document.selection_ids().is_disjoint(&original));
    document.undo().unwrap();
    assert_eq!(node::contents_of(&document.tree.world, group).len(), 1);
    assert!(!document.has_selection());
    document.redo().unwrap();
    assert_eq!(node::contents_of(&document.tree.world, group).len(), 2);
    document.close_group();
    document.select_all();
    let before = document.to_map_text();
    let selected = document.selection_ids().clone();
    assert_eq!(
        document.duplicate_selection_by(Vec3d::new(1e9, 0.0, 0.0)),
        0
    );
    assert_eq!(document.to_map_text(), before);
    assert_eq!(document.selection_ids(), &selected);
}

#[test]
fn a_duplicate_and_a_move_repeat_together() {
    // The row-of-copies workflow: duplicate, move the copy, then repeat.
    let mut document = one_box();
    document.select_all();
    let before = document.brush_count();
    document.duplicate_selection_by(Vec3d::new(0.0, 0.0, 0.0));
    document.translate_selection(Vec3d::new(128.0, 0.0, 0.0));

    assert!(document.repeat());
    assert_eq!(document.brush_count(), before + 2);
    // The last copy is two moves along from where the original sits.
    let last = document.selection_bounds().expect("a selection").center();
    assert!((last.x() - 256.0).abs() < 0.001, "{last:?}");
}

#[test]
fn undoing_forgets_what_would_be_repeated() {
    let mut document = one_box();
    document.select_all();
    document.translate_selection(Vec3d::new(0.0, 0.0, 16.0));
    assert!(document.can_repeat());
    document.undo();
    assert!(!document.can_repeat());
}

#[test]
fn a_point_file_can_be_walked_from_one_end_to_the_other() {
    let path = std::env::temp_dir().join("heron-document-point-file.pts");
    std::fs::write(&path, "0 0 0\n0 0 64\n128 0 64\n").expect("a written point file");

    let mut document = Document::new(WorldBounds::default());
    assert_eq!(document.load_point_file(&path).expect("it reads"), 3);
    assert!(document.point_file_loaded());
    assert!(document.can_reload_point_file());

    // The camera stands above the point it is sent to and looks the way the trace
    // goes on from there.
    let (at, looking) = document.step_along_trace(true).expect("a second point");
    assert_eq!(at, Vec3d::new(0.0, 0.0, 80.0));
    assert_eq!(looking, Vec3d::new(1.0, 0.0, 0.0));

    // At the end there is nowhere further to go, and the way back is the way in.
    assert!(document.step_along_trace(true).is_some());
    assert!(document.step_along_trace(true).is_none());
    assert!(document.step_along_trace(false).is_some());

    document.unload_point_file();
    assert!(!document.point_file_loaded());
    assert!(document.step_along_trace(true).is_none());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn an_export_leaves_out_the_layers_that_are_marked_to_be_left_out() {
    let mut document = one_box();
    let notes = document.add_layer("Notes");
    document.select_all();
    assert_eq!(document.move_selection_to_layer(notes), 1);

    // In the map itself the brush is written whatever layer it is on.
    assert!(document.to_map_text().contains("stone"));

    document.set_layer_omitted(notes, true);
    let exported = document.export_map_text(map_model::map_format::MapFormat::LOSSLESS);
    assert!(
        !exported.contains("stone"),
        "the omitted layer's brush is written"
    );
    // The layer itself is still there, so the export opens as the same map.
    assert!(exported.contains("Notes"));
}

/* ========== entity models ========== */

#[test]
fn the_class_defaults_can_be_put_back_or_filled_in() {
    let fgd = std::env::temp_dir().join("heron-document-defaults.fgd");
    std::fs::write(
        &fgd,
        "@PointClass size(-8 -8 -8, 8 8 8) = thing : \"A thing\" [\n\
         \thealth(integer) : \"Health\" : 25\n\
         \tspeed(integer) : \"Speed\" : 100\n\
         ]\n",
    )
    .expect("a written fgd");

    let mut document = Document::new(WorldBounds::default());
    document
        .load_definitions(&fgd)
        .expect("the definitions load");
    let id = document.create_point_entity("thing", Vec3d::zero());
    // Placing one writes the class's defaults, so this starts from an entity that has
    // been edited since and had a property taken off it.
    document.set_entity_property(id, "health", "3");
    document.remove_entity_property_on(&[id], "speed");
    document.select_only(id);

    // Only what is there, put back to what the class says.
    assert_eq!(
        document.set_default_properties(DefaultProperties::Existing),
        1
    );
    let entity = document.selected_entity().expect("the entity").1;
    assert_eq!(entity.property("health"), Some("25"));
    assert_eq!(entity.property("speed"), None);

    // Then the ones that are not there at all.
    assert_eq!(
        document.set_default_properties(DefaultProperties::Missing),
        1
    );
    let entity = document.selected_entity().expect("the entity").1;
    assert_eq!(entity.property("speed"), Some("100"));

    let _ = std::fs::remove_file(&fgd);
}

#[test]
fn selected_brushes_become_a_brush_entity_and_undo_takes_it_back() {
    let mut document = one_box();
    document.select_all();

    let id = document
        .create_brush_entity("func_door")
        .expect("an entity is made");
    let entity = match &map_model::node::find_node(&document.tree.world, id)
        .expect("it is there")
        .kind
    {
        NodeKind::Entity(entity) => entity.clone(),
        _ => panic!("that is not an entity"),
    };
    assert_eq!(entity.classname(), Some("func_door"));
    assert_eq!(entity.children.len(), 1, "the brush went inside it");
    // The brush is still selected, since it is what is being worked on.
    assert_eq!(document.selection_count(), 1);
    // It is written as a brush entity rather than as part of the world.
    assert!(document.to_map_text().contains("func_door"));

    // Choosing another class for it re-classes it rather than nesting a second one.
    assert!(document.create_brush_entity("func_wall").is_some());
    assert!(document.to_map_text().contains("func_wall"));
    assert!(!document.to_map_text().contains("func_door"));

    document.undo();
    document.undo();
    assert!(!document.to_map_text().contains("func_door"));
    assert_eq!(document.brush_count(), 1);
}

/* ========== insetting and bevelling ========== */

/// The one box with the face pointing the given way picked.
fn box_with_face_picked(direction: Vec3d) -> Document {
    let mut document = one_box();
    let brushes = map_model::node::brushes_with_ids(&document.tree.world);
    let (id, brush) = *brushes.first().expect("a brush");
    let index = brush
        .faces()
        .iter()
        .position(|face| geometry_math::vec::dot(face.normal(), direction) > 0.99)
        .expect("a face that way");
    document.select_face_only(FaceRef {
        node: id,
        face: index,
    });
    document
}

#[test]
fn bevelling_a_face_draws_it_in_and_leaves_the_rest_where_it_was() {
    // The box is 32 units across, from -16 to 16.
    let mut document = box_with_face_picked(Vec3d::new(0.0, 0.0, 1.0));
    assert_eq!(
        document.bevel_faces(&document.selected_faces(), 4.0, 0.0),
        1
    );

    let brushes = map_model::node::brushes_with_ids(&document.tree.world);
    let (_, brush) = *brushes.first().expect("the brush");
    let top: Vec<Vec3d> = brush
        .vertex_positions()
        .into_iter()
        .filter(|corner| corner.z() > 15.0)
        .collect();
    assert_eq!(top.len(), 4);
    assert!(top
        .iter()
        .all(|corner| (corner.x().abs() - 12.0).abs() < 0.001));
    // The bottom is untouched, so the brush still stands where it did.
    assert!((brush.bounds().min.z() + 16.0).abs() < 0.001);
}

#[test]
fn an_inset_that_would_eat_the_face_changes_nothing() {
    let mut document = box_with_face_picked(Vec3d::new(0.0, 0.0, 1.0));
    assert_eq!(
        document.bevel_faces(&document.selected_faces(), 32.0, 0.0),
        0
    );
    let brushes = map_model::node::brushes_with_ids(&document.tree.world);
    assert_eq!(brushes[0].1.vertex_positions().len(), 8);
}

#[test]
fn a_panel_is_a_new_brush_standing_on_the_face() {
    let mut document = box_with_face_picked(Vec3d::new(0.0, 0.0, 1.0));
    let before = document.brush_count();
    assert_eq!(
        document.panel_on_faces(&document.selected_faces(), 4.0, 8.0),
        1
    );
    assert_eq!(document.brush_count(), before + 1);

    // It sits on the face and is as deep as it was asked to be.
    let raised = map_model::node::brushes(&document.tree.world)
        .into_iter()
        .find(|brush| brush.bounds().min.z() > 15.0)
        .expect("the panel");
    assert!((raised.bounds().max.z() - 24.0).abs() < 0.001);

    document.undo();
    assert_eq!(document.brush_count(), before);
}

#[test]
fn a_recess_takes_the_shape_out_of_the_brush() {
    let mut document = box_with_face_picked(Vec3d::new(0.0, 0.0, 1.0));
    assert_eq!(
        document.panel_on_faces(&document.selected_faces(), 4.0, -8.0),
        1
    );

    // The middle of the recess is empty, and the wall beside it is not.
    let pieces = map_model::node::brushes(&document.tree.world);
    assert!(!pieces
        .iter()
        .any(|brush| brush.contains_point(Vec3d::new(0.0, 0.0, 12.0))));
    assert!(pieces
        .iter()
        .any(|brush| brush.contains_point(Vec3d::new(15.0, 0.0, 12.0))));

    document.undo();
    assert_eq!(document.brush_count(), 1);
}

/* ========== tags ========== */

/// A one-box document in the Quake 2 format, so its faces carry the surface-flag trio.
fn quake2_box() -> Document {
    let source = "\
// Format: Quake2
{
\"classname\" \"worldspawn\"
{
( -16 -16 -16 ) ( -16 16 -16 ) ( -16 -16 16 ) stone 0 0 0 1 1 0 0 0
( -16 -16 -16 ) ( -16 -16 16 ) ( 16 -16 -16 ) stone 0 0 0 1 1 0 0 0
( -16 -16 -16 ) ( 16 -16 -16 ) ( -16 16 -16 ) stone 0 0 0 1 1 0 0 0
( 16 16 16 ) ( 16 -16 16 ) ( -16 16 16 ) stone 0 0 0 1 1 0 0 0
( 16 16 16 ) ( -16 16 16 ) ( 16 16 -16 ) stone 0 0 0 1 1 0 0 0
( 16 16 16 ) ( 16 16 -16 ) ( 16 -16 16 ) stone 0 0 0 1 1 0 0 0
}
}
";
    let map = read_map(source, None).expect("the sample parses");
    Document::from_map(&map, WorldBounds::default(), None)
}

#[test]
fn a_flag_tag_sets_and_clears_its_bit_on_the_selected_faces() {
    use crate::games::{Tag, TagRule};

    let mut document = quake2_box();
    assert_eq!(document.format, MapFormat::Quake2);

    let detail = Tag {
        name: "Detail".to_string(),
        rule: TagRule::ContentFlag,
        pattern: (1 << 27).to_string(),
        transparent: false,
    };
    document.set_tags(Vec::new(), vec![detail.clone()]);
    assert!(document.can_enable_tag(&detail));
    assert!(document.can_disable_tag(&detail));

    let node = map_model::node::brushes_with_ids(&document.tree.world)[0].0;
    document.select_face_only(FaceRef { node, face: 0 });

    assert_eq!(document.enable_tag(&detail), TagOutcome::Changed(1));
    assert_eq!(
        document.brush(node).unwrap().faces()[0]
            .surface_attributes
            .unwrap()
            .content_flags,
        1 << 27
    );
    // Idempotent.
    assert_eq!(document.enable_tag(&detail), TagOutcome::Changed(0));

    assert_eq!(document.disable_tag(&detail), TagOutcome::Changed(1));
    assert_eq!(
        document.brush(node).unwrap().faces()[0]
            .surface_attributes
            .unwrap()
            .content_flags,
        0
    );

    document.undo();
    assert_eq!(
        document.brush(node).unwrap().faces()[0]
            .surface_attributes
            .unwrap()
            .content_flags,
        1 << 27
    );
}

#[test]
fn a_flag_tag_needs_a_format_that_carries_flags() {
    use crate::games::{Tag, TagRule};

    let mut document = one_box(); // Standard format
    let detail = Tag {
        name: "Detail".to_string(),
        rule: TagRule::ContentFlag,
        pattern: (1 << 27).to_string(),
        transparent: false,
    };
    document.set_tags(Vec::new(), vec![detail.clone()]);
    assert!(!document.can_enable_tag(&detail));

    let node = map_model::node::brushes_with_ids(&document.tree.world)[0].0;
    document.select_face_only(FaceRef { node, face: 0 });
    assert_eq!(document.enable_tag(&detail), TagOutcome::Unsupported);
}

#[test]
fn a_surfaceparm_tag_goes_by_what_the_shader_said() {
    use crate::games::{Tag, TagRule};

    let mut document = one_box();
    // A material out of a shader that says the surface is a clip brush, and a plain
    // image, which has nothing to say about itself.
    let mut from_shader = map_model::material::Material::rgba(
        "textures/common/clip".to_string(),
        std::path::PathBuf::new(),
        1,
        1,
        vec![0, 0, 0, 255],
    );
    from_shader.surface_parms = vec!["playerclip".to_string(), "nodraw".to_string()];
    document.materials.insert(from_shader);
    document
        .materials
        .insert(map_model::material::Material::rgba(
            "brick".to_string(),
            std::path::PathBuf::new(),
            1,
            1,
            vec![0, 0, 0, 255],
        ));

    let tag = Tag {
        name: "Clip".to_string(),
        rule: TagRule::SurfaceParm,
        pattern: "playerclip".to_string(),
        transparent: true,
    };
    document.set_tags(Vec::new(), vec![tag.clone()]);
    document.set_tag_hidden("Clip", true);

    let node = map_model::node::brushes_with_ids(&document.tree.world)[0].0;
    let brush = document.brush(node).expect("the box").clone();
    let mut shader_face = brush.faces()[0].clone();
    shader_face.material_name = "textures/common/clip".into();
    let mut plain_face = brush.faces()[1].clone();
    plain_face.material_name = "brick".into();

    assert!(document.face_hidden_by_tag(&shader_face));
    assert!(!document.face_hidden_by_tag(&plain_face));
}

#[test]
fn turning_the_selection_into_a_material_tag_puts_that_material_on_it() {
    use crate::games::{Tag, TagRule};

    let mut document = one_box();
    for name in ["clip", "brick"] {
        document
            .materials
            .insert(map_model::material::Material::rgba(
                name.to_string(),
                std::path::PathBuf::new(),
                1,
                1,
                vec![0, 0, 0, 255],
            ));
    }
    let clip = Tag {
        name: "Clip".to_string(),
        rule: TagRule::Material,
        pattern: "clip".to_string(),
        transparent: true,
    };
    let missing = Tag {
        name: "Missing".to_string(),
        rule: TagRule::Material,
        pattern: "missing_test_material".to_string(),
        transparent: true,
    };
    document.set_tags(Vec::new(), vec![clip.clone(), missing.clone()]);

    assert!(document.can_enable_tag(&clip));
    assert!(
        !document.can_enable_tag(&missing),
        "the custom material is not loaded"
    );

    // With nothing selected there is nothing to act on.
    assert_eq!(document.enable_tag(&clip), TagOutcome::NoSelection);

    let node = map_model::node::brushes_with_ids(&document.tree.world)[0].0;
    document.select_face_only(FaceRef { node, face: 0 });
    assert_eq!(document.enable_tag(&clip), TagOutcome::Changed(1));
    assert_eq!(
        document.brush(node).unwrap().faces()[0].material_name,
        "clip"
    );

    // One undo takes the material back off.
    document.undo();
    assert_ne!(
        document.brush(node).unwrap().faces()[0].material_name,
        "clip"
    );

    // Re-select and try a tag with no matching material.
    document.select_face_only(FaceRef { node, face: 0 });
    assert_eq!(document.enable_tag(&missing), TagOutcome::NoMaterial);

    // And the face already wearing the material is reported as no change.
    document.enable_tag(&clip);
    assert_eq!(document.enable_tag(&clip), TagOutcome::Changed(0));
}

#[test]
fn hiding_a_tag_takes_its_faces_and_brushes_out_of_what_is_drawn() {
    use crate::games::{Tag, TagRule};

    let mut document = one_box();
    document.set_tags(
        vec![Tag {
            name: "Trigger".to_string(),
            rule: TagRule::Classname,
            pattern: "trigger_*".to_string(),
            transparent: true,
        }],
        vec![Tag {
            name: "Clip".to_string(),
            rule: TagRule::Material,
            pattern: "clip".to_string(),
            transparent: true,
        }],
    );

    let filters = crate::view_filters::ViewFilters::default();
    let all = document.triangles(&filters).len();
    assert_eq!(all, 12, "a box is twelve triangles");

    // One face wears the clip material; hiding that tag takes its two triangles.
    document.select_all();
    let face = FaceRef {
        node: map_model::node::brushes_with_ids(&document.tree.world)[0].0,
        face: 0,
    };
    document.select_face_only(face);
    document.set_current_material("clip");
    assert_eq!(document.apply_current_material(), 1);

    document.set_tag_hidden("Clip", true);
    assert_eq!(document.triangles(&filters).len(), all - 2);

    // And showing it again brings them back.
    document.set_tag_hidden("Clip", false);
    assert_eq!(document.triangles(&filters).len(), all);

    // A brush tag matches the entity a brush is in; in the world it is in none, so
    // hiding the trigger tag changes nothing here.
    document.set_tag_hidden("Trigger", true);
    assert_eq!(document.triangles(&filters).len(), all);
}

#[test]
fn transparent_tags_draw_triggers_and_clip_faces_see_through() {
    use crate::games::{Tag, TagRule};

    let mut document = one_box();
    let tag = |name: &str, rule, pattern: &str| Tag {
        name: name.to_string(),
        rule,
        pattern: pattern.to_string(),
        transparent: true,
    };
    document.set_tags(
        vec![tag("Trigger", TagRule::Classname, "trigger_*")],
        vec![tag("Clip", TagRule::Material, "clip")],
    );
    let filters = crate::view_filters::ViewFilters::default();
    let alphas = |document: &Document| {
        let mut alphas: Vec<f32> = document
            .triangles(&filters)
            .iter()
            .map(|triangle| triangle.alpha)
            .collect();
        alphas.dedup();
        alphas
    };
    assert_eq!(alphas(&document), [1.0], "a plain brush is solid");

    // The box becomes a trigger's brush.
    document.select_all();
    let trigger = document.create_brush_entity("trigger_once").unwrap();
    assert_eq!(alphas(&document), [surfaces::SEE_THROUGH_ALPHA]);
    // A door is solid, though only the entity changed, not its brush.
    assert!(document.set_entity_property(trigger, "classname", "func_door"));
    assert_eq!(alphas(&document), [1.0]);
    assert!(document.set_entity_property(trigger, "classname", "trigger_multiple"));
    assert_eq!(alphas(&document), [surfaces::SEE_THROUGH_ALPHA]);

    // A clip brush in the world is see-through by its faces' material.
    let mut document = Document::new(Default::default());
    document.set_tags(Vec::new(), vec![tag("Clip", TagRule::Material, "clip")]);
    let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(32.0, 32.0, 32.0));
    document.add_brushes(vec![map_model::shapes::create_cuboid(
        &document.world_bounds(),
        &bounds,
        "clip",
    )
    .unwrap()]);
    assert_eq!(alphas(&document), [surfaces::SEE_THROUGH_ALPHA]);
}

#[test]
fn a_brush_tag_hides_the_brushes_of_the_entity_it_matches() {
    use crate::games::{Tag, TagRule};

    let mut document = one_box();
    document.set_tags(
        vec![Tag {
            name: "Trigger".to_string(),
            rule: TagRule::Classname,
            pattern: "trigger_*".to_string(),
            transparent: true,
        }],
        Vec::new(),
    );

    // The box becomes a trigger's brush.
    document.select_all();
    assert!(document.create_brush_entity("trigger_once").is_some());

    let filters = crate::view_filters::ViewFilters::default();
    assert_eq!(document.triangles(&filters).len(), 12);
    // The brushes are still selected, so their edges are the selected ones.
    assert!(!document.selected_edges().is_empty());

    document.set_tag_hidden("Trigger", true);
    assert!(
        document.triangles(&filters).is_empty(),
        "the trigger is still drawn"
    );
    // The flat views draw gathered edges, so those go too.
    assert!(document.selected_edges().is_empty());
    assert!(document.edges().is_empty());
}

#[test]
fn a_class_with_a_model_is_drawn_as_it_rather_than_as_a_box() {
    // A game directory holding one model, and a definition naming it.
    let game = std::env::temp_dir().join("heron-document-entity-model");
    let _ = std::fs::remove_dir_all(&game);
    std::fs::create_dir_all(game.join("id1/progs")).expect("a game directory");
    std::fs::write(
        game.join("id1/progs/thing.mdl"),
        crate::entity_models::tests::one_triangle_model(),
    )
    .expect("a written model");
    crate::preferences::edit(|preferences| {
        preferences.game = "Quake".into();
        preferences.game_path = game.display().to_string()
    });

    let fgd = game.join("test.fgd");
    std::fs::write(
        &fgd,
        "@PointClass size(-8 -8 -8, 8 8 8) model(\"progs/thing.mdl\") = thing : \"A thing\" []\n",
    )
    .expect("a written fgd");

    let mut document = Document::new(WorldBounds::default());
    document
        .load_definitions(&fgd)
        .expect("the definitions load");
    document.create_point_entity("thing", Vec3d::new(64.0, 0.0, 0.0));

    let filters = crate::view_filters::ViewFilters::default();
    let triangles = document.triangles(&filters);
    // The model is one triangle; a box would be twelve.
    assert_eq!(triangles.len(), 1, "the box was drawn instead of the model");
    assert!(
        triangles[0].material.is_some(),
        "the skin is there to draw it with"
    );
    // It stands where the entity does.
    assert!(triangles[0].positions.iter().all(|point| point.x() >= 64.0));

    // Disabling models uses the entity bounding box instead.
    let boxes = document.triangles(&crate::view_filters::ViewFilters {
        show_point_entity_models: false,
        ..filters
    });
    assert_eq!(boxes.len(), 12);

    crate::preferences::edit(|preferences| preferences.game_path = String::new());
    let _ = std::fs::remove_dir_all(&game);
}

#[test]
fn a_class_with_an_iconsprite_is_drawn_as_a_sprite_not_a_box() {
    let game = std::env::temp_dir().join("heron-document-sprite");
    let _ = std::fs::remove_dir_all(&game);
    std::fs::create_dir_all(game.join("id1/sprites")).expect("a game directory");
    // A Half-Life sprite carries its own palette.
    let mut spr = Vec::new();
    spr.extend_from_slice(b"IDSP");
    spr.extend_from_slice(&2i32.to_le_bytes()); // version
    spr.extend_from_slice(&2i32.to_le_bytes()); // type
    spr.extend_from_slice(&3i32.to_le_bytes()); // texFormat
    spr.extend_from_slice(&1.0f32.to_le_bytes()); // radius
    spr.extend_from_slice(&4i32.to_le_bytes()); // max width
    spr.extend_from_slice(&2i32.to_le_bytes()); // max height
    spr.extend_from_slice(&1i32.to_le_bytes()); // frames
    spr.extend_from_slice(&0.0f32.to_le_bytes()); // beam
    spr.extend_from_slice(&0i32.to_le_bytes()); // sync
    spr.extend_from_slice(&256i16.to_le_bytes()); // palette size
    spr.extend_from_slice(&vec![7u8; 768]); // palette
    spr.extend_from_slice(&0i32.to_le_bytes()); // lone frame
    spr.extend_from_slice(&0i32.to_le_bytes()); // origin x
    spr.extend_from_slice(&0i32.to_le_bytes()); // origin y
    spr.extend_from_slice(&4i32.to_le_bytes()); // width
    spr.extend_from_slice(&2i32.to_le_bytes()); // height
    spr.extend_from_slice(&[1u8; 8]); // 8 pixels
    std::fs::write(game.join("id1/sprites/bulb.spr"), &spr).expect("a written sprite");

    crate::preferences::edit(|p| {
        p.game = "Quake".into();
        p.game_path = game.display().to_string();
    });
    let fgd = game.join("test.fgd");
    std::fs::write(
        &fgd,
        "@PointClass iconsprite(\"sprites/bulb.spr\") = light : \"L\" []\n",
    )
    .expect("a written fgd");

    let mut document = Document::new(WorldBounds::default());
    document.load_definitions(&fgd).expect("definitions load");
    document.create_point_entity("light", Vec3d::new(64.0, 0.0, 0.0));

    let filters = crate::view_filters::ViewFilters::default();
    // No box triangles for it -- the sprite is a separate billboard pass.
    assert!(
        document.triangles(&filters).is_empty(),
        "the box was still drawn"
    );

    let sprites = document.sprites();
    assert_eq!(sprites.len(), 1);
    assert_eq!(sprites[0].position, Vec3d::new(64.0, 0.0, 0.0));
    assert!(
        sprites[0].half_width > sprites[0].half_height,
        "4x2 sprite is wider than tall"
    );

    // With the model filter off it is a box again.
    let boxes = document.triangles(&crate::view_filters::ViewFilters {
        show_point_entity_models: false,
        ..filters
    });
    assert_eq!(boxes.len(), 12);

    crate::preferences::edit(|p| p.game_path = String::new());
    let _ = std::fs::remove_dir_all(&game);
}

#[test]
fn a_class_with_a_builtin_icon_keeps_its_see_through_box_and_outline() {
    let mut document = Document::new(WorldBounds::default());
    document.create_point_entity("info_player_start", Vec3d::new(0.0, 0.0, 32.0));
    let filters = crate::view_filters::ViewFilters::default();
    assert_eq!(document.sprites().len(), 1, "the icon is still drawn");

    let triangles = document.triangles(&filters);
    assert_eq!(triangles.len(), 12, "a box of twelve triangles");
    assert!(
        triangles.iter().all(|t| t.alpha < 1.0),
        "the box is see-through"
    );
    let colour = document.entity_boxes()[0]
        .color
        .map(|channel| f32::from(channel) / 255.0);
    assert!(
        triangles.iter().all(|t| t.color == colour),
        "in the class's colour"
    );

    let outlines = document.entity_box_outlines(&filters);
    assert_eq!(outlines.len(), 1);
    assert_eq!(outlines[0].bounds, document.entity_boxes()[0].bounds);
    let hidden = crate::view_filters::ViewFilters {
        show_point_entities: false,
        ..filters
    };
    assert!(document.entity_box_outlines(&hidden).is_empty());

    // A light is drawn as its symbol, with no outline.
    document.create_point_entity("light", Vec3d::new(64.0, 0.0, 0.0));
    assert_eq!(document.entity_box_outlines(&filters).len(), 1);
}

#[test]
fn a_portal_file_is_read_and_can_be_put_away() {
    let path = std::env::temp_dir().join("heron-document-portals.prt");
    std::fs::write(
        &path,
        "PRT1\n6\n1\n4 1 4 (0 0 0 ) (0 64 0 ) (0 64 64 ) (0 0 64 )\n",
    )
    .expect("a written portal file");

    let mut document = Document::new(WorldBounds::default());
    assert_eq!(document.load_portal_file(&path).expect("it reads"), 1);
    assert!(document.portal_file_loaded());
    assert!(document.can_reload_portal_file());
    assert_eq!(document.portals().expect("portals").polygons[0].len(), 4);

    document.unload_portal_file();
    assert!(!document.portal_file_loaded());
    assert!(document.portals().is_none());

    // A file that is not a portal file is an error rather than an empty drawing.
    std::fs::write(&path, "not a portal file\n").expect("a written file");
    assert!(document.load_portal_file(&path).is_err());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn editing_the_level_forgets_the_trace() {
    // The trace describes geometry that has just changed, so leaving it drawn would
    // point at a hole that may now be filled.
    let mut document = sealed_room(Some(0));
    document.check_for_leaks(8.0);
    assert!(document.leak_trace().is_some());

    document.select_all();
    document.translate_selection(Vec3d::new(8.0, 0.0, 0.0));
    assert!(document.leak_trace().is_none());
}

#[test]
fn the_check_uses_every_brush_and_every_entity() {
    // A brush entity's brushes seal as well as the world's do, and an entity left
    // outside the level is a leak of its own.
    let mut document = sealed_room(None);
    document.create_point_entity("light", Vec3d::new(512.0, 0.0, 0.0));
    assert!(matches!(
        document.check_for_leaks(8.0),
        map_model::leak::Leak::Leaked(_)
    ));
}

#[test]
fn a_point_entity_moves_with_the_selection() {
    let mut document = one_entity();
    assert!(document.translate_selection(Vec3d::new(32.0, 0.0, 0.0)));
    assert_eq!(
        the_entity(&document).origin(),
        Some(Vec3d::new(96.0, 0.0, 16.0))
    );
}

#[test]
fn moving_a_point_entity_can_be_undone() {
    let mut document = one_entity();
    document.translate_selection(Vec3d::new(32.0, 0.0, 0.0));
    document.undo();
    assert_eq!(
        the_entity(&document).origin(),
        Some(Vec3d::new(64.0, 0.0, 16.0))
    );
}

#[test]
fn a_point_entity_turns_about_the_selections_centre() {
    let mut document = one_entity();
    // A quarter turn about the origin takes (64, 0) to (0, 64) -- but the centre it
    // turns about is the selection's own, which is the entity, so it stays put and
    // only its facing changes.
    assert!(document.rotate_selection(Vec3d::new(0.0, 0.0, 1.0), 90.0));
    assert_eq!(
        the_entity(&document).origin(),
        Some(Vec3d::new(64.0, 0.0, 16.0))
    );
    assert_eq!(the_entity(&document).property("angle"), Some("90"));
}

#[test]
fn a_point_entity_turned_about_a_point_away_from_it_moves_as_well() {
    let mut document = one_entity();
    assert!(document.rotate_selection_about(Vec3d::zero(), Vec3d::new(0.0, 0.0, 1.0), 90.0));
    let origin = the_entity(&document).origin().expect("an origin");
    assert!(
        vec::length(origin - Vec3d::new(0.0, 64.0, 16.0)) < 0.001,
        "{origin:?}"
    );
}

#[test]
fn a_point_entity_is_mirrored_with_the_selection() {
    let mut document = Document::new(WorldBounds::default());
    let world = document.world_bounds();
    let brush = map_model::shapes::create_cuboid(
        &world,
        &Bbox3d::new(
            Vec3d::new(-32.0, -32.0, -32.0),
            Vec3d::new(32.0, 32.0, 32.0),
        ),
        "stone",
    )
    .expect("a cube");
    document.add_brushes(vec![brush]);
    document.create_point_entity("monster_army", Vec3d::new(64.0, 0.0, 0.0));
    document.select_all();

    // The selection spans the brush, which reaches x = 32, and the entity's box,
    // which with no definition to size it reaches eight past its origin to x = 72.
    // The middle of that is x = 20, and the entity mirrors to 2 * 20 - 64.
    assert!(document.flip_selection(geometry_math::util::Axis::X));
    let origin = the_entity(&document).origin().expect("an origin");
    assert!((origin.x() + 24.0).abs() < 0.001, "{origin:?}");
}

#[test]
fn all_library_assets_stay_on_grid_after_insertion() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let grid_one = include_str!("../../../../assets/grid1_assets.txt");
    for name in grid_one.lines().filter(|name| !name.is_empty()) {
        assert!(
            directory.join(format!("{name}.map")).is_file(),
            "missing grid 1 asset {name}"
        );
    }
    let mut checked = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "map") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let map = map_model::io::map_reader::read_map(&text, None).unwrap();
        let expected: usize = map.entities.iter().map(|e| e.brushes.len()).sum();
        let mut document = Document::new(WorldBounds::default());
        document.paste_asset_at(&text, Vec3d::new(128.0, 96.0, 64.0), "Asset");
        let brushes = node::brushes_with_ids(document.tree_world());
        assert_eq!(brushes.len(), expected, "{} lost brushes", path.display());
        for (_, brush) in brushes {
            let grid = if grid_one
                .lines()
                .any(|name| Some(name) == path.file_stem().and_then(|s| s.to_str()))
            {
                1.0
            } else {
                0.125
            };
            assert!(
                brush.geometry().vertices().iter().all(|v| (0..3)
                    .all(|i| (v.position[i] / grid - (v.position[i] / grid).round()).abs() < 1e-5)),
                "{} moved off grid {grid}",
                path.display()
            );
        }
        checked += 1;
    }
    assert!(checked >= 54);
}

#[test]
fn asset_paste_with_a_half_grid_center_preserves_the_vertex_grid() {
    let mut source = Document::new(WorldBounds::default());
    source.add_brushes(vec![map_model::shapes::create_cuboid(
        &source.world_bounds(),
        &Bbox3d::new(Vec3d::zero(), Vec3d::new(4.125, 8.125, 16.125)),
        "stone",
    )
    .unwrap()]);
    let mut target = Document::new(WorldBounds::default());
    assert_eq!(
        target.paste_asset_at(
            &source.to_map_text(),
            Vec3d::new(32.0, 48.0, 64.0),
            "Detail"
        ),
        1
    );
    for (_, brush) in node::brushes_with_ids(target.tree_world()) {
        assert!(brush.vertices_are_on_grid(0.125));
        assert_eq!(brush.bounds().size(), Vec3d::new(4.125, 8.125, 16.125));
    }
}

#[test]
fn asset_paste_groups_all_parts_in_one_undo_step() {
    let source = two_boxes().to_map_text();
    let mut document = Document::new(WorldBounds::default());
    assert_eq!(
        document.paste_asset_at(&source, Vec3d::new(128.0, 0.0, 0.0), "Gate"),
        2
    );
    let groups = document.groups();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].1, "Gate");
    assert_eq!(groups[0].2, 2);
    assert_eq!(document.selection_ids().len(), 2);
    assert!(!document.selection_ids().contains(&groups[0].0));
    assert!((document.selection_bounds().unwrap().center().x() - 128.0).abs() < 1e-6);
    assert_eq!(document.paste_asset_at(&source, Vec3d::zero(), "Gate"), 2);
    assert_eq!(document.groups().len(), 2);
    assert!(document.undo().is_some());
    assert_eq!(document.groups().len(), 1);
    assert!(document.undo().is_some());
    assert!(document.groups().is_empty());
    assert!(document
        .tree
        .world
        .layers
        .iter()
        .all(|layer| layer.children.is_empty()));
    assert!(document.undo().is_none());
    assert!(document.redo().is_some());
    assert_eq!(document.groups().len(), 1);
    assert_eq!(document.groups()[0].2, 2);
}

#[test]
fn grouping_the_selection_makes_a_group_of_it() {
    let mut document = two_boxes();
    assert!(document.groups().is_empty());

    let group = document.group_selection("Doorway").expect("a group");
    let groups = document.groups();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].0, group);
    assert_eq!(groups[0].1, "Doorway");
    assert_eq!(groups[0].2, 2, "both boxes are in it");

    // The brushes are the same brushes, still drawn and still selected.
    assert_eq!(document.brush_count(), 2);
    assert!(!document
        .triangles(&crate::view_filters::ViewFilters::default())
        .is_empty());
}

#[test]
fn grouping_and_ungrouping_can_be_undone() {
    let mut document = two_boxes();
    let brushes: Vec<NodeId> = node::brushes_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .collect();

    document.group_selection("Doorway");
    assert_eq!(document.groups().len(), 1);
    assert_eq!(document.selection_count(), 2, "the contents stay selected");

    document.undo();
    assert!(document.groups().is_empty(), "the group is gone");
    assert_eq!(
        document.brush_count(),
        2,
        "the boxes came back to the layer"
    );

    document.redo();
    assert_eq!(document.groups().len(), 1);

    // Now ungroup and undo that.
    document.select_only(brushes[0]); // takes the whole group
    document.ungroup_selection();
    assert!(document.groups().is_empty());

    document.undo();
    assert_eq!(document.groups().len(), 1, "the group is back");
    assert_eq!(document.groups()[0].2, 2, "with both boxes in it");
}

#[test]
fn objects_move_in_and_out_of_the_open_group() {
    let mut document = Document::new(WorldBounds::default());
    // A group of one brush, and a loose brush beside it.
    for x in [0.0, 128.0] {
        let brush = map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &Bbox3d::new(
                Vec3d::new(x - 16.0, -16.0, -16.0),
                Vec3d::new(x + 16.0, 16.0, 16.0),
            ),
            "stone",
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
    }
    let brushes: Vec<NodeId> = node::brushes_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .collect();
    document.select_only(brushes[0]);
    let group = document.group_selection("G").expect("a group");
    document.select_only(brushes[0]); // opens the group when asked
    document.open_selected_group();
    assert_eq!(document.open_group(), Some(group));

    // Move the loose brush in.
    document.select_only(brushes[1]);
    assert_eq!(document.add_selection_to_open_group(), 1);
    assert_eq!(
        document.groups()[0].2,
        2,
        "both brushes are in the group now"
    );

    document.undo();
    assert_eq!(document.groups()[0].2, 1, "the move undid");
    document.redo();
    assert_eq!(document.groups()[0].2, 2);

    // Move both back out.
    document.select_all();
    assert_eq!(document.remove_selection_from_open_group(), 2);
    assert!(document.groups().is_empty() || document.groups()[0].2 == 0);
}

#[test]
fn extracting_from_a_linked_set_splits_it_in_two() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(256.0);
    // Two linked groups of two brushes each.
    assert_eq!(document.groups().len(), 2);
    assert!(document.groups().iter().all(|(_, _, count)| *count == 2));

    // Open one group and select just its first brush.
    let group = node::groups_with_ids(&document.tree.world)[0].0;
    document.select_only(node::contents_of(&document.tree.world, group)[0]);
    document.open_selected_group();
    let first_brush = {
        let NodeKind::Group(g) =
            &node::find_node(&document.tree.world, document.open_group().unwrap())
                .unwrap()
                .kind
        else {
            unreachable!()
        };
        g.children[0].id
    };
    document.select_only(first_brush);

    let new_groups = document.extract_linked_groups().expect("it extracts");
    assert_eq!(new_groups.len(), 2, "a new group per member");
    assert_eq!(document.groups().len(), 4, "two kept, two extracted");
    assert!(
        document.groups().iter().all(|(_, _, count)| *count == 1),
        "every group holds one brush now"
    );

    // The two new groups share a link id distinct from the originals'.
    let links: Vec<Option<String>> = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(_, g)| g.linked_group_id.clone())
        .collect();
    assert!(links.iter().all(|l| l.is_some()));

    document.undo();
    assert_eq!(document.groups().len(), 2, "back to the one linked set");
    assert!(document.groups().iter().all(|(_, _, count)| *count == 2));
}

#[test]
fn merging_groups_folds_them_into_one_and_undoes() {
    let mut document = Document::new(WorldBounds::default());
    for x in [0.0, 128.0, 256.0] {
        let brush = map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &Bbox3d::new(
                Vec3d::new(x - 16.0, -16.0, -16.0),
                Vec3d::new(x + 16.0, 16.0, 16.0),
            ),
            "stone",
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
        document.group_selection(format!("G{x}"));
        document.clear_selection();
    }
    assert_eq!(document.groups().len(), 3);

    // Select every member of every group, then merge.
    document.select_all();
    assert_eq!(
        document.merge_selected_groups(),
        2,
        "two folded into the third"
    );
    assert_eq!(document.groups().len(), 1);
    assert_eq!(
        document.groups()[0].2,
        3,
        "all three brushes in the one group"
    );

    document.undo();
    assert_eq!(document.groups().len(), 3, "the three groups are back");

    // A single group selected is not enough to merge.
    document.clear_selection();
    let first_group_member = node::contents_of(
        &document.tree.world,
        node::groups_with_ids(&document.tree.world)[0].0,
    )[0];
    document.select_only(first_group_member);
    assert_eq!(document.merge_selected_groups(), 0);
    assert_eq!(document.groups().len(), 3);
}

#[test]
fn clicking_one_member_of_a_group_takes_the_whole_group() {
    let mut document = two_boxes();
    document.group_selection("Doorway");

    let first = node::brushes_with_ids(&document.tree.world)[0].0;
    document.select_only(first);
    assert_eq!(document.selection_count(), 2, "the other box came too");
}

#[test]
fn an_open_group_lets_one_member_be_selected_on_its_own() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    assert!(document.open_selected_group());
    assert!(document.open_group().is_some());

    let first = node::brushes_with_ids(&document.tree.world)[0].0;
    document.select_only(first);
    assert_eq!(document.selection_count(), 1);

    // And closing it puts the group back together.
    assert!(document.close_group());
    document.select_only(first);
    assert_eq!(document.selection_count(), 2);
}

#[test]
fn ungrouping_takes_a_group_apart_and_keeps_what_was_in_it() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.select_all();

    assert_eq!(document.ungroup_selection(), 1);
    assert!(document.groups().is_empty());
    assert_eq!(document.brush_count(), 2);
}

#[test]
fn a_group_can_be_renamed_and_the_rename_undone() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.select_all();

    assert_eq!(document.rename_selected_groups("Arch"), 1);
    assert_eq!(document.groups()[0].1, "Arch");

    document.undo();
    assert_eq!(document.groups()[0].1, "Doorway");
}

#[test]
fn renaming_leaves_a_half_selected_group_alone() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    // Only one of the two members in the selection set.
    let first = node::brushes_with_ids(&document.tree.world)[0].0;
    document.selection = std::iter::once(first).collect();

    assert_eq!(document.rename_selected_groups("Arch"), 0);
    assert_eq!(document.groups()[0].1, "Doorway");
}

#[test]
fn renaming_one_linked_group_renames_the_set() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(128.0);
    // The copy is what is selected after a linked duplicate.
    assert_eq!(document.rename_selected_groups("Arch"), 2);
    assert!(document.groups().iter().all(|(_, name, _)| name == "Arch"));
}

/// The `linked_group_id` of every group, in listing order.
fn link_ids(document: &Document) -> Vec<Option<String>> {
    node::groups_with_ids(&document.tree.world)
        .into_iter()
        .map(|(_, group)| group.linked_group_id.clone())
        .collect()
}

#[test]
fn separating_one_of_three_linked_groups_frees_only_it() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(128.0);
    document.create_linked_duplicate(128.0);
    assert_eq!(
        link_ids(&document).iter().filter(|l| l.is_some()).count(),
        3
    );

    // The last copy is what is selected.
    assert_eq!(document.separate_selected_linked_groups(), 1);
    let linked: Vec<_> = link_ids(&document).into_iter().flatten().collect();
    assert_eq!(linked.len(), 2, "two groups stay linked to each other");
}

#[test]
fn separating_from_a_pair_frees_both() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(128.0);
    assert_eq!(
        link_ids(&document).iter().filter(|l| l.is_some()).count(),
        2
    );

    assert_eq!(
        document.separate_selected_linked_groups(),
        2,
        "a set of one is no set"
    );
    assert!(link_ids(&document).iter().all(|l| l.is_none()));
}

#[test]
fn select_linked_groups_takes_the_whole_set() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(128.0);
    document.create_linked_duplicate(128.0);

    // Only the last copy's contents are selected right now.
    let before = document.selection_count();
    document.select_linked_groups();
    assert!(document.selection_count() > before);
    // Every brush in every linked group is now selected: 3 groups x 2 boxes.
    assert_eq!(document.selection_count(), 6);
}

#[test]
fn a_linked_duplicate_shares_an_id_and_sits_a_step_over() {
    let mut document = two_boxes();
    document.group_selection("Doorway");

    let copy_id = document
        .create_linked_duplicate(64.0)
        .expect("a whole group is selected");

    // Two groups, both wearing the same link id, each with a transformation.
    let groups: Vec<(NodeId, Option<String>, bool)> = node::groups_with_ids(&document.tree.world)
        .into_iter()
        .map(|(id, g)| (id, g.linked_group_id.clone(), g.transformation.is_some()))
        .collect();
    assert_eq!(groups.len(), 2);
    assert!(groups[0].1.is_some() && groups[0].1 == groups[1].1);
    assert!(groups.iter().all(|(_, _, has_transform)| *has_transform));

    // The copy is already selected; its brushes sit a grid step along x from the
    // source's.
    let copy_center = document
        .selection_bounds()
        .expect("the copy is selected")
        .center();
    let source_id = groups.iter().find(|(id, _, _)| *id != copy_id).unwrap().0;
    let source_first = node::contents_of(&document.tree.world, source_id)[0];
    document.select_only(source_first);
    let src_center = document.selection_bounds().expect("the source").center();
    assert!(
        (copy_center.x() - src_center.x() - 64.0).abs() < 0.001,
        "{copy_center:?} {src_center:?}"
    );
}

#[test]
fn a_linked_duplicate_can_be_undone() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    assert_eq!(node::groups_with_ids(&document.tree.world).len(), 1);

    document.create_linked_duplicate(64.0);
    assert_eq!(node::groups_with_ids(&document.tree.world).len(), 2);

    document.undo();
    let groups = node::groups_with_ids(&document.tree.world);
    assert_eq!(groups.len(), 1, "undo takes the copy back out");
    assert!(
        groups[0].1.linked_group_id.is_none(),
        "and the source is no longer linked"
    );
}

#[test]
fn a_linked_duplicate_needs_a_whole_group_selected() {
    let mut document = two_boxes();
    // No group at all.
    assert!(document.create_linked_duplicate(64.0).is_none());
}

#[test]
fn an_edit_to_one_linked_group_reaches_the_others_on_its_own() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(128.0);

    let all: Vec<NodeId> = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .collect();
    // The copy is selected; the source is the other one.
    let source_id = *all
        .iter()
        .find(|id| !document.is_selected(node::contents_of(&document.tree.world, **id)[0]))
        .unwrap();
    let copy_id = *all.iter().find(|id| **id != source_id).unwrap();

    // Where the copy's brush sits before the edit.
    document.select_only(node::contents_of(&document.tree.world, copy_id)[0]);
    let copy_z_before = document.selection_bounds().unwrap().center().z();

    // Move one of the source's brushes up -- with no call to sync.
    let source_brush = node::contents_of(&document.tree.world, source_id)[0];
    document.select_only(source_brush);
    document.translate_selection(Vec3d::new(0.0, 0.0, 32.0));

    // The copy followed the same 32 units.
    let copy_id = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .find(|id| *id != source_id)
        .unwrap();
    document.select_only(node::contents_of(&document.tree.world, copy_id)[0]);
    let copy_z_after = document.selection_bounds().unwrap().center().z();
    assert!(
        (copy_z_after - copy_z_before - 32.0).abs() < 0.001,
        "copy moved from {copy_z_before} to {copy_z_after}"
    );
}

#[test]
fn syncing_with_only_ordinary_groups_does_nothing() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    assert_eq!(document.sync_linked_groups(), 0);
}

#[test]
fn the_automatic_linked_sync_is_part_of_the_same_undo_step() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(128.0);

    let source_id = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .find(|id| !document.is_selected(node::contents_of(&document.tree.world, *id)[0]))
        .unwrap();
    let brush = node::contents_of(&document.tree.world, source_id)[0];
    document.select_only(brush);

    let brushes_before = node::brushes_with_ids(&document.tree.world).len();
    document.translate_selection(Vec3d::new(0.0, 0.0, 32.0));
    assert_eq!(
        node::brushes_with_ids(&document.tree.world).len(),
        brushes_before,
        "the rebuild replaces the copy's brushes, it does not add any"
    );

    let copy_id = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .find(|id| *id != source_id)
        .unwrap();
    document.select_only(node::contents_of(&document.tree.world, copy_id)[0]);
    let copy_z_moved = document.selection_bounds().unwrap().center().z();

    // One undo takes back both the move and the rebuild it set off.
    document.undo();
    assert_eq!(node::groups_with_ids(&document.tree.world).len(), 2);
    let copy_id = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .find(|id| *id != source_id)
        .unwrap();
    document.select_only(node::contents_of(&document.tree.world, copy_id)[0]);
    let copy_z_undone = document.selection_bounds().unwrap().center().z();
    assert!(
        (copy_z_moved - copy_z_undone - 32.0).abs() < 0.001,
        "undo put the copy back: was {copy_z_moved}, now {copy_z_undone}"
    );
}

/// Drops a light facing along +x into a group and returns the group id it went in.
fn put_a_light_in(document: &mut Document, group_id: NodeId) {
    let light = Node::new(NodeKind::Entity(map_model::node::EntityNode {
        properties: vec![
            ("classname".to_string(), "light".to_string()),
            ("origin".to_string(), "0 0 0".to_string()),
            ("mangle".to_string(), "0 0 0".to_string()),
        ],
        children: Vec::new(),
    }));
    let mut group = node::find_node(&document.tree.world, group_id)
        .unwrap()
        .clone();
    if let NodeKind::Group(g) = &mut group.kind {
        g.children.push(light);
    }
    assert!(node::replace_node(&mut document.tree.world, group));
}

/// The first component of a linked group's only light's `mangle`.
fn light_yaw_in(document: &Document, group_id: NodeId) -> f64 {
    let mangle = node::contents_of(&document.tree.world, group_id)
        .iter()
        .filter_map(|id| node::find_node(&document.tree.world, *id))
        .find_map(|n| match &n.kind {
            NodeKind::Entity(e) => e.property("mangle").map(str::to_string),
            _ => None,
        })
        .expect("the group has the light");
    mangle
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .expect("a number")
}

#[test]
fn a_linked_set_at_a_turn_turns_its_entities_facings_too() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(64.0);

    let all: Vec<NodeId> = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .collect();
    let source_id = *all
        .iter()
        .find(|id| !document.is_selected(node::contents_of(&document.tree.world, **id)[0]))
        .unwrap();
    let copy_id = *all.iter().find(|id| **id != source_id).unwrap();

    put_a_light_in(&mut document, source_id);

    // Set the copy a quarter turn about z from the source.
    let mut copy_group = node::find_node(&document.tree.world, copy_id)
        .unwrap()
        .clone();
    if let NodeKind::Group(g) = &mut copy_group.kind {
        g.transformation = Some("0 -1 0 0 1 0 0 0 0 0 1 0 0 0 0 1".to_string());
    }
    assert!(node::replace_node(&mut document.tree.world, copy_group));

    // Edit the source -- with no call to sync.
    let source_brush = node::contents_of(&document.tree.world, source_id)[0];
    document.select_only(source_brush);
    document.translate_selection(Vec3d::new(0.0, 0.0, 16.0));

    let copy_id = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .find(|id| *id != source_id)
        .unwrap();
    let yaw = light_yaw_in(&document, copy_id).rem_euclid(360.0);
    assert!(
        (yaw - 90.0).abs() < 1.0 || (yaw - 270.0).abs() < 1.0,
        "the copy's light turned a quarter circle, to yaw {yaw}"
    );
}

#[test]
fn a_linked_set_related_only_by_a_shift_leaves_facings_alone() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(64.0);

    let all: Vec<NodeId> = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .collect();
    let source_id = *all
        .iter()
        .find(|id| !document.is_selected(node::contents_of(&document.tree.world, **id)[0]))
        .unwrap();
    put_a_light_in(&mut document, source_id);

    let source_brush = node::contents_of(&document.tree.world, source_id)[0];
    document.select_only(source_brush);
    document.translate_selection(Vec3d::new(0.0, 0.0, 16.0));

    let copy_id = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .find(|id| *id != source_id)
        .unwrap();
    assert_eq!(
        light_yaw_in(&document, copy_id),
        0.0,
        "a shift is not a turn, so the light still faces where it did"
    );
}

/// Mutates the one light in the group `group_id` through `edit`, then puts it back.
fn edit_the_light_in(
    document: &mut Document,
    group_id: NodeId,
    edit: impl FnOnce(&mut map_model::node::EntityNode),
) {
    let mut group = node::find_node(&document.tree.world, group_id)
        .unwrap()
        .clone();
    let NodeKind::Group(g) = &mut group.kind else {
        panic!("not a group")
    };
    let light = g
        .children
        .iter_mut()
        .find_map(|child| match &mut child.kind {
            NodeKind::Entity(entity) => Some(entity),
            _ => None,
        })
        .expect("the group has a light");
    edit(light);
    assert!(node::replace_node(&mut document.tree.world, group));
}

/// The one light's value for `key` in the group `group_id`.
fn light_property_in(document: &Document, group_id: NodeId, key: &str) -> Option<String> {
    node::contents_of(&document.tree.world, group_id)
        .iter()
        .filter_map(|id| node::find_node(&document.tree.world, *id))
        .find_map(|n| match &n.kind {
            NodeKind::Entity(e) => e.property(key).map(str::to_string),
            _ => None,
        })
}

#[test]
fn a_protected_property_stays_the_copys_own_through_a_linked_sync() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    let group_id = node::groups_with_ids(&document.tree.world)[0].0;
    put_a_light_in(&mut document, group_id);
    document.select_only(node::contents_of(&document.tree.world, group_id)[0]);
    document.create_linked_duplicate(128.0);

    let all: Vec<NodeId> = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .collect();
    let source_id = *all
        .iter()
        .find(|id| !document.is_selected(node::contents_of(&document.tree.world, **id)[0]))
        .unwrap();
    let copy_id = *all.iter().find(|id| **id != source_id).unwrap();

    // The copy's light guards its own targetname; the source's light has a plain one.
    edit_the_light_in(&mut document, copy_id, |light| {
        light.set_property("targetname", "copy_light");
        light.set_property("light", "100");
        light.set_protected_properties(&["targetname".to_string()]);
    });
    edit_the_light_in(&mut document, source_id, |light| {
        light.set_property("targetname", "source_light");
        light.set_property("light", "500");
    });

    // An edit to the source rebuilds the copy.
    let source_brush = node::contents_of(&document.tree.world, source_id)[0];
    document.select_only(source_brush);
    document.translate_selection(Vec3d::new(0.0, 0.0, 32.0));

    let copy_id = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .find(|id| *id != source_id)
        .unwrap();

    // The protected property kept the copy's own value; the plain one took the source's.
    assert_eq!(
        light_property_in(&document, copy_id, "targetname").as_deref(),
        Some("copy_light"),
        "a protected property is the copy's own"
    );
    assert_eq!(
        light_property_in(&document, copy_id, "light").as_deref(),
        Some("500"),
        "a plain property follows the source"
    );
    assert_eq!(
        light_property_in(&document, copy_id, "_tb_protected_properties").as_deref(),
        Some("targetname"),
        "the copy still knows which of its properties is protected"
    );
}

#[test]
fn the_protected_property_toggle_is_undoable_and_knows_the_linked_group() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    let group_id = node::groups_with_ids(&document.tree.world)[0].0;
    put_a_light_in(&mut document, group_id);
    let light_id = node::contents_of(&document.tree.world, group_id)
        .into_iter()
        .find(|id| {
            matches!(
                node::find_node(&document.tree.world, *id).map(|n| &n.kind),
                Some(NodeKind::Entity(_))
            )
        })
        .unwrap();

    // An ordinary group is not a linked one.
    assert!(!document.node_is_in_linked_group(light_id));

    document.select_only(node::contents_of(&document.tree.world, group_id)[0]);
    document.create_linked_duplicate(128.0);
    assert!(document.node_is_in_linked_group(light_id));

    assert!(!document.is_entity_property_protected(light_id, "mangle"));
    document.set_entity_property_protected_on(&[light_id], "mangle", true);
    assert!(document.is_entity_property_protected(light_id, "mangle"));

    document.undo();
    assert!(!document.is_entity_property_protected(light_id, "mangle"));
}

#[test]
fn make_structural_pulls_brushes_out_of_a_brush_entity() {
    let mut document = Document::new(WorldBounds::default());
    for x in [0.0, 128.0] {
        let brush = map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &Bbox3d::new(
                Vec3d::new(x - 16.0, -16.0, -16.0),
                Vec3d::new(x + 16.0, 16.0, 16.0),
            ),
            "stone",
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
    }
    document.select_all();
    document.create_brush_entity("func_wall");
    assert_eq!(node::entities_with_ids(&document.tree.world).len(), 1);

    let brushes: Vec<NodeId> = node::brushes_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .collect();

    // Pull one brush out: the entity keeps the other.
    document.select_only(brushes[0]);
    assert_eq!(document.make_structural(), 1);
    let entity = &node::entities_with_ids(&document.tree.world)[0].1;
    assert_eq!(entity.children.len(), 1, "one brush left in the entity");

    document.undo();
    assert_eq!(
        node::entities_with_ids(&document.tree.world)[0]
            .1
            .children
            .len(),
        2,
        "the move undid"
    );

    // Pull both out: the emptied entity goes too.
    document.select_all();
    assert_eq!(document.make_structural(), 2);
    assert!(
        node::entities_with_ids(&document.tree.world).is_empty(),
        "the entity is gone"
    );
    assert_eq!(document.brush_count(), 2, "the brushes are still there");
}

#[test]
fn clearing_protected_properties_takes_the_siblings_values_and_drops_the_flag() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    let group_id = node::groups_with_ids(&document.tree.world)[0].0;
    put_a_light_in(&mut document, group_id);
    document.select_only(node::contents_of(&document.tree.world, group_id)[0]);
    document.create_linked_duplicate(128.0);

    let all: Vec<NodeId> = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .collect();
    let source_id = *all
        .iter()
        .find(|id| !document.is_selected(node::contents_of(&document.tree.world, **id)[0]))
        .unwrap();
    let copy_id = *all.iter().find(|id| **id != source_id).unwrap();

    // The source light has mangle "10 0 0"; the copy protects its own "90 0 0".
    edit_the_light_in(&mut document, source_id, |light| {
        light.set_property("mangle", "10 0 0");
    });
    edit_the_light_in(&mut document, copy_id, |light| {
        light.set_property("mangle", "90 0 0");
        light.set_protected_properties(&["mangle".to_string()]);
    });

    // Select the copy's light and clear its protection.
    let copy_light = node::contents_of(&document.tree.world, copy_id)
        .into_iter()
        .find(|id| {
            matches!(
                node::find_node(&document.tree.world, *id).map(|n| &n.kind),
                Some(NodeKind::Entity(_))
            )
        })
        .unwrap();
    document.select_only(copy_light);
    assert_eq!(document.clear_protected_properties(), 1);

    match &node::find_node(&document.tree.world, copy_light)
        .unwrap()
        .kind
    {
        NodeKind::Entity(e) => {
            assert_eq!(
                e.property("mangle"),
                Some("10 0 0"),
                "took the sibling's value"
            );
            assert!(e.protected_properties().is_empty(), "the flag is gone");
        }
        _ => unreachable!(),
    }
}

#[test]
fn a_property_edit_reaches_every_selected_entity_in_one_undo_step() {
    let mut document = Document::new(WorldBounds::default());
    let a = document.create_point_entity("light", Vec3d::new(0.0, 0.0, 0.0));
    let b = document.create_point_entity("light", Vec3d::new(64.0, 0.0, 0.0));
    let c = document.create_point_entity("light", Vec3d::new(128.0, 0.0, 0.0));
    document.set_entity_property_on(&[a], "spawnflags", "1");
    document.set_entity_property_on(&[b], "spawnflags", "3");

    let ids = [a, b, c];
    document.set_entity_property_on(&ids, "light", "300");
    let value_of = |doc: &Document, id| match &node::find_node(&doc.tree.world, id).unwrap().kind {
        NodeKind::Entity(e) => e.property("light").map(str::to_string),
        _ => None,
    };
    assert_eq!(value_of(&document, a).as_deref(), Some("300"));
    assert_eq!(value_of(&document, c).as_deref(), Some("300"));

    // One undo takes it back off all three.
    document.undo();
    assert_eq!(value_of(&document, a), None);
    assert_eq!(value_of(&document, b), None);

    let flags_of = |doc: &Document, id| match &node::find_node(&doc.tree.world, id).unwrap().kind {
        NodeKind::Entity(e) => e
            .property("spawnflags")
            .and_then(|v| v.trim().parse::<i64>().ok()),
        _ => None,
    };

    // Seeding: `c` has no spawnflags, so it starts from the given default bits.
    document.set_entity_flag_seeded_on(&[c], "spawnflags", 1024, 2, true);
    assert_eq!(
        flags_of(&document, c),
        Some(1024 | 2),
        "the default bit plus the ticked one"
    );
    document.undo();

    // A plain flag edit keeps each entity's other bits: a has bit 1, b has 1|2, c none.
    document.set_entity_flag_on(&ids, "spawnflags", 4, true);
    assert_eq!(flags_of(&document, a), Some(1 | 4));
    assert_eq!(flags_of(&document, b), Some(1 | 2 | 4));
    assert_eq!(flags_of(&document, c), Some(4));
}

#[test]
fn a_group_only_half_selected_is_left_alone() {
    // Ungrouping acts on groups the selection covers; half of one is not a group the
    // user has asked about.
    let mut document = two_boxes();
    document.group_selection("Doorway");

    let first = node::brushes_with_ids(&document.tree.world)[0].0;
    document.open_selected_group();
    document.select_only(first);

    assert_eq!(document.ungroup_selection(), 0);
    assert_eq!(document.groups().len(), 1);
}

#[test]
fn a_group_can_be_saved_and_read_back() {
    let mut document = two_boxes();
    document.group_selection("Doorway");

    let path = std::env::temp_dir().join("heron-group-roundtrip.map");
    let _ = std::fs::remove_file(&path);
    document.save_as(&path).expect("it saves");

    let reopened = Document::open(&path, WorldBounds::default()).expect("it opens");
    let groups = reopened.groups();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].1, "Doorway");
    assert_eq!(reopened.brush_count(), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_linked_duplicate_survives_a_save() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(64.0);

    let path =
        std::env::temp_dir().join(format!("heron-linked-roundtrip-{}.map", std::process::id()));
    document.save_as(&path).expect("it saves");
    let reopened = Document::open(&path, WorldBounds::default()).expect("it opens");
    let _ = std::fs::remove_file(&path);

    let groups = node::groups_with_ids(&reopened.tree.world);
    assert_eq!(groups.len(), 2);
    // Both still linked to the same id, both still carrying a transformation.
    let link_ids: Vec<Option<&str>> = groups
        .iter()
        .map(|(_, g)| g.linked_group_id.as_deref())
        .collect();
    assert!(link_ids[0].is_some() && link_ids[0] == link_ids[1]);
    assert!(groups.iter().all(|(_, g)| g.transformation.is_some()));
}

/* ========== what the views draw ========== */

#[test]
fn a_link_runs_from_the_one_targeting_to_the_one_targeted() {
    let mut document = one_box();
    let trigger = document.create_point_entity("trigger_once", Vec3d::zero());
    document.set_entity_property(trigger, "target", "the_door");
    let door = document.create_point_entity("func_door", Vec3d::new(128.0, 0.0, 0.0));
    document.set_entity_property(door, "targetname", "the_door");

    let links = document.entity_links();
    assert_eq!(links.len(), 1);
    geometry_math::assert_approx_eq!(links[0].from, Vec3d::zero(), 1e-6);
    geometry_math::assert_approx_eq!(links[0].to, Vec3d::new(128.0, 0.0, 0.0), 1e-6);
}

#[test]
fn one_name_targeted_by_several_gives_several_links() {
    let mut document = one_box();
    let door = document.create_point_entity("func_door", Vec3d::zero());
    document.set_entity_property(door, "targetname", "the_door");
    for x in [64.0, 128.0] {
        let trigger = document.create_point_entity("trigger_once", Vec3d::new(x, 0.0, 0.0));
        document.set_entity_property(trigger, "target", "the_door");
    }

    assert_eq!(document.entity_links().len(), 2);
}

#[test]
fn a_link_knows_whether_the_selection_is_part_of_it() {
    let mut document = one_box();
    let trigger = document.create_point_entity("trigger_once", Vec3d::zero());
    document.set_entity_property(trigger, "target", "the_door");
    let door = document.create_point_entity("func_door", Vec3d::new(128.0, 0.0, 0.0));
    document.set_entity_property(door, "targetname", "the_door");

    document.clear_selection();
    assert!(!document.entity_links()[0].touches_selection);

    document.select_only(trigger);
    assert!(document.entity_links()[0].touches_selection);
}

#[test]
fn a_target_nothing_answers_to_makes_no_link() {
    let mut document = one_box();
    let trigger = document.create_point_entity("trigger_once", Vec3d::zero());
    document.set_entity_property(trigger, "target", "nothing_at_all");
    assert!(document.entity_links().is_empty());
}

#[test]
fn turning_off_brushes_stops_them_being_drawn() {
    let document = one_box();
    let mut filters = crate::view_filters::ViewFilters::default();
    assert!(!document.triangles(&filters).is_empty());

    filters.show_brushes = false;
    assert!(document.triangles(&filters).is_empty());
}

#[test]
fn turning_off_entities_stops_them_being_drawn_but_leaves_the_brushes() {
    let mut document = one_box();
    document.create_point_entity("info_player_start", Vec3d::new(128.0, 0.0, 0.0));

    let mut filters = crate::view_filters::ViewFilters {
        show_point_entity_models: false,
        ..Default::default()
    };
    let with_entity = document.triangles(&filters).len();
    filters.show_point_entities = false;
    let without = document.triangles(&filters).len();

    assert_eq!(
        with_entity - without,
        12,
        "the entity's box is twelve triangles"
    );
    assert!(without > 0, "the brush is still drawn");
}

#[test]
fn a_group_has_an_outline_around_everything_in_it() {
    let mut document = two_boxes();
    assert!(document.group_bounds().is_empty());

    document.group_selection("Doorway");
    let bounds = document.group_bounds();
    assert_eq!(bounds.len(), 1);
    // Around both boxes, which span from -32 to 160.
    geometry_math::assert_approx_eq!(bounds[0].bounds.min.x(), -32.0, 1e-6);
    geometry_math::assert_approx_eq!(bounds[0].bounds.max.x(), 160.0, 1e-6);
    assert!(bounds[0].selected, "everything in it is selected");
}

/* ========== patches ========== */

/// A map holding one flat 3x3 patch and nothing else.
// The file is named after the test using it: the tests run in parallel, and one
// fixed name would have them writing over each other's map.
fn document_with_a_patch(name: &str) -> Document {
    let source = r#"
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
    let path = std::env::temp_dir().join(format!("heron-patch-document-{name}.map"));
    std::fs::write(&path, source).expect("a written map");
    Document::open(&path, WorldBounds::default()).expect("it opens")
}

#[test]
fn a_patch_is_drawn() {
    let document = document_with_a_patch("drawn");
    let filters = crate::view_filters::ViewFilters::default();
    let triangles = document.triangles(&filters);
    assert!(!triangles.is_empty(), "the patch produced no triangles");
}

#[test]
fn a_patch_s_uv_coordinates_are_scaled_to_the_material() {
    // The map counts a patch's UV coordinates in whole copies of the material and a
    // face's in texels, and the shader divides by the material's size for the face's
    // sake. A patch's have to be turned into texels to survive that: this patch runs
    // 0..1 both ways, so on a 128x64 material it should reach 128 across and 64 down.
    let mut document = document_with_a_patch("uv-scale");
    // Only the size matters here; the pixels are never read.
    document
        .materials
        .insert(map_model::material::Material::rgba(
            "common/caulk".to_string(),
            std::path::PathBuf::new(),
            128,
            64,
            vec![0, 0, 0, 255],
        ));

    let filters = crate::view_filters::ViewFilters::default();
    let (mut widest, mut tallest) = (0.0f32, 0.0f32);
    for triangle in document.triangles(&filters) {
        for uv in triangle.uv {
            widest = widest.max(uv.x());
            tallest = tallest.max(uv.y());
        }
    }
    assert!((widest - 128.0).abs() < 0.001, "across: {widest}");
    assert!((tallest - 64.0).abs() < 0.001, "down: {tallest}");
}

#[test]
fn a_patch_can_be_picked_by_its_surface() {
    let document = document_with_a_patch("pick");
    // Straight down through the middle, where the surface bulges up to just under the
    // middle control point.
    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, 256.0), Vec3d::new(0.0, 0.0, -1.0));
    let hit = document.pick(&ray).expect("the patch was hit");
    assert!(matches!(
        node::find_node(&document.tree.world, hit.node).map(|node| &node.kind),
        Some(NodeKind::Patch(_))
    ));
}

#[test]
fn a_control_point_moves_and_takes_the_surface_with_it() {
    let mut document = document_with_a_patch("control");
    document.select_all();

    let before = document.selected_patches()[0].clone();
    let corner = before.control_points[0].position;
    let highest_before = highest_point(&document);

    assert!(document.move_control_points(&[corner], Vec3d::new(0.0, 0.0, 32.0)));

    let after = document.selected_patches()[0].clone();
    assert_eq!(
        after.control_points[0].position,
        corner + Vec3d::new(0.0, 0.0, 32.0)
    );
    // Every other control point stayed where it was.
    for (moved, still) in after
        .control_points
        .iter()
        .zip(&before.control_points)
        .skip(1)
    {
        assert_eq!(moved.position, still.position);
    }
    // And the surface followed, rather than the point moving on its own.
    assert!(highest_point(&document) > highest_before - 1.0);
}

#[test]
fn moving_a_control_point_can_be_undone() {
    let mut document = document_with_a_patch("undo");
    document.select_all();
    let before = document.selected_patches()[0].clone();
    let corner = before.control_points[0].position;

    document.move_control_points(&[corner], Vec3d::new(0.0, 0.0, 32.0));
    document.undo();

    assert_eq!(
        document.selected_patches()[0].control_points,
        before.control_points
    );
}

#[test]
fn a_control_point_no_patch_has_moves_nothing() {
    let mut document = document_with_a_patch("nothing");
    document.select_all();
    assert!(!document.move_control_points(
        &[Vec3d::new(1024.0, 1024.0, 1024.0)],
        Vec3d::new(0.0, 0.0, 32.0)
    ));
}

#[test]
fn resizing_a_patch_grows_its_grid_in_one_undo_step() {
    let mut document = document_with_a_patch("resize");
    document.select_all();
    assert_eq!(document.selected_patches()[0].columns, 3);

    assert_eq!(
        document.resize_selected_patches(map_model::patch_edit::add_columns),
        1
    );
    assert_eq!(document.selected_patches()[0].columns, 5);
    assert_eq!(document.selected_patches()[0].rows, 3);

    document.undo();
    assert_eq!(document.selected_patches()[0].columns, 3);
}

#[test]
fn resizing_with_no_patch_selected_does_nothing() {
    let mut document = document_with_a_patch("resize-none");
    document.clear_selection();
    assert_eq!(
        document.resize_selected_patches(map_model::patch_edit::add_rows),
        0
    );
}

/// The highest point of everything the document draws.
fn highest_point(document: &Document) -> f64 {
    document
        .triangles(&crate::view_filters::ViewFilters::default())
        .iter()
        .flat_map(|triangle| triangle.positions)
        .fold(f64::MIN, |highest, corner| highest.max(corner.z()))
}

#[test]
fn turning_off_patches_stops_them_being_drawn() {
    let document = document_with_a_patch("filtered");
    let filters = crate::view_filters::ViewFilters {
        show_patches: false,
        ..Default::default()
    };
    assert!(document.triangles(&filters).is_empty());
}

#[test]
fn a_patch_reaches_where_its_control_points_are() {
    let document = document_with_a_patch("reach");
    let filters = crate::view_filters::ViewFilters::default();
    let triangles = document.triangles(&filters);

    let highest = triangles
        .iter()
        .flat_map(|triangle| triangle.positions)
        .fold(f64::MIN, |highest, corner| highest.max(corner.z()));
    // The middle control point is at 64 and a quadratic curve does not reach it, but
    // the corners at 32 are on the surface.
    assert!(highest >= 32.0, "the patch did not rise: {highest}");
    assert!(highest < 64.0);
}

/// A document with one box and one point entity in it.
fn box_and_entity() -> Document {
    let source = "\
{
\"classname\" \"worldspawn\"
{
( -16 -16 -16 ) ( -16 16 -16 ) ( -16 -16 16 ) stone 0 0 0 1 1
( -16 -16 -16 ) ( -16 -16 16 ) ( 16 -16 -16 ) stone 0 0 0 1 1
( -16 -16 -16 ) ( 16 -16 -16 ) ( -16 16 -16 ) stone 0 0 0 1 1
( 16 16 16 ) ( 16 -16 16 ) ( -16 16 16 ) stone 0 0 0 1 1
( 16 16 16 ) ( -16 16 16 ) ( 16 16 -16 ) stone 0 0 0 1 1
( 16 16 16 ) ( 16 16 -16 ) ( 16 -16 16 ) stone 0 0 0 1 1
}
}
{
\"classname\" \"light\"
\"origin\" \"0 0 128\"
}
";
    let map = read_map(source, None).expect("the sample parses");
    Document::from_map(&map, WorldBounds::default(), None)
}

#[test]
fn creating_a_point_entity_places_it_and_selects_it() {
    let mut document = one_box();
    let id = document.create_point_entity("light", Vec3d::new(0.0, 0.0, 64.0));

    assert!(document.is_selected(id));
    let (selected, entity) = document.selected_entity().expect("it is selected");
    assert_eq!(selected, id);
    assert_eq!(entity.classname(), Some("light"));
    assert_eq!(entity.origin(), Some(Vec3d::new(0.0, 0.0, 64.0)));
}

#[test]
fn a_loose_brush_moves_into_an_existing_entity_and_undoes() {
    let mut document = two_boxes();
    let brushes: Vec<NodeId> = node::brushes_by_layer(&document.tree.world)
        .into_iter()
        .map(|(id, _, _)| id)
        .collect();

    document.select_only(brushes[0]);
    let entity = document
        .create_brush_entity("func_wall")
        .expect("an entity");

    document.select_only(brushes[1]);
    let moved = document.move_selection_to_entity(entity);
    assert_eq!(moved, 1);

    let (_, entity_node) = node::entities_with_ids(&document.tree.world)
        .into_iter()
        .find(|(id, _)| *id == entity)
        .expect("the entity is still there");
    assert_eq!(entity_node.children.len(), 2, "both brushes are in it now");

    document.undo();
    let (_, entity_node) = node::entities_with_ids(&document.tree.world)
        .into_iter()
        .find(|(id, _)| *id == entity)
        .expect("the entity is still there");
    assert_eq!(
        entity_node.children.len(),
        1,
        "undo put the brush back on the layer"
    );
}

#[test]
fn the_layers_a_selection_sits_on_and_isolating_them() {
    let mut document = two_boxes();
    let brushes: Vec<NodeId> = node::brushes_by_layer(&document.tree.world)
        .into_iter()
        .map(|(id, _, _)| id)
        .collect();
    let notes = document.add_layer("notes");
    document.select_only(brushes[1]);
    document.move_selection_to_layer(notes);

    document.select_only(brushes[0]);
    assert_eq!(document.layers_of_selection(), vec![0]);

    document.isolate_layers(&[0]);
    assert!(document.layers()[notes].hidden, "the other layer went dark");
    assert!(!document.layers()[0].hidden);

    assert_eq!(document.select_layers(&[0]), 1);
}

#[test]
fn select_all_of_a_class_takes_every_entity_that_wears_it() {
    let mut document = one_box();
    let first = document.create_point_entity("light", Vec3d::new(0.0, 0.0, 64.0));
    let second = document.create_point_entity("light", Vec3d::new(64.0, 0.0, 64.0));
    let other = document.create_point_entity("info_player_start", Vec3d::zero());

    let count = document.select_entities_with_classname("LIGHT");

    assert_eq!(count, 2, "both lights, matched without regard to case");
    assert!(document.is_selected(first));
    assert!(document.is_selected(second));
    assert!(!document.is_selected(other));
}

#[test]
fn creating_an_entity_writes_the_defaults_its_class_defines() {
    let mut document = one_box();
    let definitions = map_model::fgd::read_fgd(
        r#"@PointClass = light : "A light" [ light(integer) : "Brightness" : 300 ]"#,
    )
    .expect("it parses");
    document.definitions = Some(definitions);

    let id = document.create_point_entity("light", Vec3d::zero());
    let (_, entity) = document.selected_entity().expect("it is selected");
    assert_eq!(entity.property("light"), Some("300"));
    let _ = id;
}

#[test]
fn setting_a_property_adds_it_then_changes_it() {
    let mut document = one_box();
    let id = document.create_point_entity("light", Vec3d::zero());

    assert!(document.set_entity_property(id, "targetname", "lamp"));
    assert_eq!(
        document.selected_entity().unwrap().1.property("targetname"),
        Some("lamp")
    );

    assert!(document.set_entity_property(id, "targetname", "torch"));
    let entity = document.selected_entity().unwrap().1;
    assert_eq!(entity.property("targetname"), Some("torch"));
    // Changing does not add a second copy.
    assert_eq!(
        entity
            .properties
            .iter()
            .filter(|(k, _)| k == "targetname")
            .count(),
        1
    );
}

#[test]
fn removing_a_property_takes_it_out() {
    let mut document = one_box();
    let id = document.create_point_entity("light", Vec3d::zero());
    document.set_entity_property(id, "targetname", "lamp");

    assert!(document.remove_entity_property_on(&[id], "targetname"));
    assert_eq!(
        document.selected_entity().unwrap().1.property("targetname"),
        None
    );
}

#[test]
fn renaming_a_property_keeps_its_place_in_the_list() {
    let mut document = one_box();
    let id = document.create_point_entity("light", Vec3d::zero());
    document.set_entity_property(id, "a", "1");
    document.set_entity_property(id, "b", "2");
    document.set_entity_property(id, "c", "3");

    let before: Vec<String> = document
        .selected_entity()
        .unwrap()
        .1
        .properties
        .iter()
        .map(|(k, _)| k.clone())
        .collect();

    assert!(document.rename_entity_property_on(&[id], "b", "renamed"));
    let after: Vec<String> = document
        .selected_entity()
        .unwrap()
        .1
        .properties
        .iter()
        .map(|(k, _)| k.clone())
        .collect();

    // Same positions, one different name, so a saved file does not shuffle.
    assert_eq!(after.len(), before.len());
    let position = before.iter().position(|k| k == "b").expect("it was there");
    assert_eq!(after[position], "renamed");
    assert_eq!(
        document.selected_entity().unwrap().1.property("renamed"),
        Some("2")
    );
}

#[test]
fn an_entity_edit_can_be_undone() {
    let mut document = one_box();
    let id = document.create_point_entity("light", Vec3d::zero());
    document.set_entity_property(id, "light", "500");

    assert!(document.undo().is_some());
    assert_eq!(
        document.selected_entity().unwrap().1.property("light"),
        None
    );
}

#[test]
fn the_property_editor_shows_one_entity_at_a_time() {
    let mut document = one_box();
    let first = document.create_point_entity("light", Vec3d::zero());
    let second = document.create_point_entity("light", Vec3d::new(64.0, 0.0, 0.0));

    // Two selected is ambiguous, so nothing is offered for editing.
    document.selection = [first, second].into_iter().collect();
    assert!(document.selected_entity().is_none());

    // And a brush is not an entity.
    let (brush_id, _) = map_model::node::brushes_with_ids(&document.tree.world)[0];
    document.select_only(brush_id);
    assert!(document.selected_entity().is_none());
}

#[test]
fn editing_a_node_that_is_not_an_entity_is_refused() {
    let mut document = one_box();
    let (brush_id, _) = map_model::node::brushes_with_ids(&document.tree.world)[0];
    assert!(!document.set_entity_property(brush_id, "a", "1"));
}

#[test]
fn a_created_entity_survives_a_save_and_reload() {
    let mut document = one_box();
    let id = document.create_point_entity("info_player_start", Vec3d::new(0.0, 64.0, 24.0));
    document.set_entity_property(id, "angle", "90");

    let map = map_model::node::flatten_tree(&document.tree.world, document.format, None);
    let text = map_model::io::map_writer::write_map(&map);
    let reread = read_map(&text, None).expect("what we wrote parses");
    let reloaded = Document::from_map(&reread, WorldBounds::default(), None);

    let entities = map_model::node::point_entities(&reloaded.tree.world);
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].1.classname(), Some("info_player_start"));
    assert_eq!(entities[0].1.property("angle"), Some("90"));
    assert_eq!(entities[0].1.origin(), Some(Vec3d::new(0.0, 64.0, 24.0)));
}

#[test]
fn a_point_entity_is_drawn_as_a_box_at_its_origin() {
    let document = box_and_entity();
    let boxes = document.entity_boxes();

    assert_eq!(boxes.len(), 1);
    // The bundled light definition supplies its bounds and display colour.
    geometry_math::assert_approx_eq!(boxes[0].bounds.center(), Vec3d::new(0.0, 0.0, 128.0), 1e-9);
    assert_eq!(boxes[0].bounds.size().x(), 16.0);
    assert_eq!(boxes[0].color, [255, 255, 40]);
    assert!(!boxes[0].selected);
}

#[test]
fn a_brush_entity_is_not_drawn_as_a_box() {
    // Only point entities get a box; a brush entity is its brushes.
    let document = one_box();
    assert!(document.entity_boxes().is_empty());
}

#[test]
fn a_point_entity_can_be_picked() {
    use geometry_math::ray::Ray3d;

    let document = box_and_entity();
    // Straight down onto the light, which is above the box.
    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, 400.0), Vec3d::new(0.0, 0.0, -1.0));
    let hit = document.pick(&ray).expect("the ray goes through the light");

    // It hit the entity, which has no faces, rather than the box behind it.
    assert_eq!(hit.face, None);
    assert!(document
        .entity_boxes()
        .iter()
        .any(|entity| entity.node == hit.node));
}

#[test]
fn the_nearer_of_a_brush_and_an_entity_is_picked() {
    use geometry_math::ray::Ray3d;

    let document = box_and_entity();
    // From below, so the box comes first.
    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, -400.0), Vec3d::new(0.0, 0.0, 1.0));
    let hit = document.pick(&ray).expect("the ray goes through the box");
    assert!(hit.face.is_some(), "it should have hit the brush first");
}

#[test]
fn a_selected_entity_is_marked_as_such() {
    let mut document = box_and_entity();
    let node = document.entity_boxes()[0].node;

    document.select_only(node);
    assert!(document.entity_boxes()[0].selected);
}

#[test]
fn a_selection_holds_either_brushes_or_faces_but_not_both() {
    let mut document = one_box();
    let (id, _) = map_model::node::brushes_with_ids(&document.tree.world)[0];

    document.select_only(id);
    assert!(document.has_selection());
    assert!(!document.has_face_selection());

    // Picking a face clears the brush selection, and the other way round.
    document.select_face_only(FaceRef { node: id, face: 0 });
    assert!(!document.has_selection());
    assert!(document.has_face_selection());
    assert_eq!(document.face_selection_count(), 1);

    document.select_only(id);
    assert!(!document.has_face_selection());
}

#[test]
fn toggling_a_face_adds_and_removes_it() {
    let mut document = one_box();
    let (id, _) = map_model::node::brushes_with_ids(&document.tree.world)[0];
    let face = FaceRef { node: id, face: 2 };

    document.toggle_face_selected(face);
    assert!(document.is_face_selected(face));
    document.toggle_face_selected(face);
    assert!(!document.is_face_selected(face));
}

#[test]
fn editing_a_selected_face_leaves_the_others_alone() {
    let mut document = one_box();
    let (id, _) = map_model::node::brushes_with_ids(&document.tree.world)[0];
    document.select_face_only(FaceRef { node: id, face: 0 });

    document.edit_faces(|face| face.move_uv(geometry_math::vec::Vec2f::new(16.0, 0.0)));

    let brush = map_model::node::brushes(&document.tree.world)[0];
    let offsets: Vec<f32> = brush
        .geometry_faces()
        .map(|(face, _)| face.uv_attributes.offset.x())
        .collect();
    assert_eq!(offsets.iter().filter(|o| **o == 16.0).count(), 1);
    assert_eq!(offsets.iter().filter(|o| **o == 0.0).count(), 5);
}

#[test]
fn editing_with_a_brush_selected_touches_every_face() {
    let mut document = one_box();
    let (id, _) = map_model::node::brushes_with_ids(&document.tree.world)[0];
    document.select_only(id);

    document.edit_faces(|face| face.move_uv(geometry_math::vec::Vec2f::new(8.0, 0.0)));

    let brush = map_model::node::brushes(&document.tree.world)[0];
    assert!(brush
        .geometry_faces()
        .all(|(face, _)| face.uv_attributes.offset.x() == 8.0));
}

#[test]
fn editing_a_face_can_be_undone() {
    let mut document = one_box();
    let (id, _) = map_model::node::brushes_with_ids(&document.tree.world)[0];
    document.select_face_only(FaceRef { node: id, face: 0 });

    document.edit_faces(|face| face.rotate_uv(45.0));
    assert!(document.undo().is_some());

    let brush = map_model::node::brushes(&document.tree.world)[0];
    assert!(brush
        .geometry_faces()
        .all(|(face, _)| face.uv_attributes.rotation == 0.0));
}

#[test]
fn a_pick_names_the_face_the_ray_entered_through() {
    use geometry_math::ray::Ray3d;

    let document = one_box();
    // From above, so it enters through the top face.
    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    let hit = document.pick(&ray).expect("the ray goes through the box");

    let brush = map_model::node::brushes(&document.tree.world)[0];
    let (face, _) = brush
        .geometry_faces()
        .nth(hit.face.expect("a brush hit names a face"))
        .expect("the hit names a real face");
    geometry_math::assert_approx_eq!(face.normal(), Vec3d::new(0.0, 0.0, 1.0), 1e-9);
}

#[test]
fn picking_finds_the_box_and_selects_it() {
    use geometry_math::ray::Ray3d;

    let mut document = one_box();
    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    let hit = document.pick(&ray).expect("the ray goes through the box");

    assert!(!document.is_selected(hit.node));
    document.select_only(hit.node);
    assert!(document.is_selected(hit.node));
    assert_eq!(document.selection_count(), 1);
}

#[test]
fn two_documents_stay_in_step_through_the_wire() {
    use crate::collab::wire::{WireEdit, WireWorld};
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut a = one_box();
    let bounds = a.world_bounds();

    // B joins: it adopts A's whole world, node ids and all, rather than parsing the
    // map itself and minting its own.
    let snapshot = WireWorld::from_world(&a.tree.world).to_json();
    let world = WireWorld::from_json(&snapshot)
        .unwrap()
        .into_world(&bounds)
        .unwrap();
    let mut b = Document::from_world(world, WorldBounds::default(), a.format, None);
    assert_eq!(
        map_model::io::map_writer::write_map(&node::flatten_tree(&a.tree.world, a.format, None)),
        map_model::io::map_writer::write_map(&node::flatten_tree(&b.tree.world, b.format, None)),
        "B starts identical to A"
    );

    // Every edit A records, captured as it would be sent.
    let sent: Rc<RefCell<Vec<Edit>>> = Rc::new(RefCell::new(Vec::new()));
    a.set_on_edit(Rc::new({
        let sent = sent.clone();
        move |edit: &Edit| sent.borrow_mut().push(edit.clone())
    }));

    let ship = |sent: &Rc<RefCell<Vec<Edit>>>, b: &mut Document| {
        for edit in sent.borrow_mut().drain(..) {
            let json = WireEdit::from_edit(&edit).to_json();
            let remote = WireEdit::from_json(&json)
                .unwrap()
                .into_edit(&bounds)
                .unwrap();
            b.apply_remote(&remote);
        }
    };

    let flatten = |doc: &Document| {
        map_model::io::map_writer::write_map(&node::flatten_tree(&doc.tree.world, doc.format, None))
    };

    // A adds a brush.
    let brush =
        map_model::shapes::create_cuboid(&bounds, &Bbox3d::from_scalars(-8.0, 8.0), "stone")
            .expect("a cube");
    a.add_brushes(vec![brush]);
    ship(&sent, &mut b);
    assert_eq!(flatten(&a), flatten(&b), "after the add");

    // A moves everything.
    a.select_all();
    assert!(a.translate_selection(Vec3d::new(64.0, 0.0, 0.0)));
    ship(&sent, &mut b);
    assert_eq!(flatten(&a), flatten(&b), "after the move");
}

#[test]
fn the_peers_selection_draws_in_its_own_edge_set() {
    use std::collections::HashSet;

    let mut document = one_box();
    let box_id = node::brushes_with_ids(&document.tree.world)[0].0;

    assert!(document.peer_edges().is_empty());
    let plain_before = document.edges().len();
    assert!(plain_before > 0);

    document.set_peer_selection(1, HashSet::from([box_id]));
    assert!(
        !document.peer_edges().is_empty(),
        "the box moved to the peer set"
    );
    assert!(document.edges().is_empty(), "and left the plain set");
    assert!(
        document.selected_edges().is_empty(),
        "the local selection is untouched"
    );

    // The local selection wins where the two overlap.
    document.select_only(box_id);
    assert!(document.peer_edges().is_empty());
    assert!(!document.selected_edges().is_empty());

    document.set_peer_selection(1, HashSet::new());
    document.clear_selection();
    assert_eq!(
        document.edges().len(),
        plain_before,
        "back to how it started"
    );
}

#[test]
fn a_peer_selection_message_round_trips_through_a_session() {
    use crate::collab::{Session, SessionEvent};
    use std::collections::HashSet;
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let host_thread = std::thread::spawn(move || Session::host(port).unwrap());
    std::thread::sleep(Duration::from_millis(50));
    let guest = Session::join(&format!("127.0.0.1:{port}")).unwrap();
    let host = host_thread.join().unwrap();

    let ids: HashSet<NodeId> = HashSet::from([NodeId(11), NodeId(22)]);
    host.send_selection(&ids);

    let bounds = Bbox3d::from_scalars(-16384.0, 16384.0);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let events = guest.poll(&bounds);
        if let Some(SessionEvent::PeerSelection { peer, ids: got }) = events.into_iter().next() {
            assert_eq!(
                peer,
                crate::collab::HOST_PEER,
                "a guest hears from the host"
            );
            assert_eq!(got, ids);
            break;
        }
        assert!(Instant::now() < deadline, "the selection never arrived");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn edits_made_while_apart_replay_onto_the_rejoined_world() {
    use crate::collab::{Session, SessionEvent};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    // Two documents in sync, then the link drops. The guest keeps working: its
    // offline edit is what must survive the rejoin.
    let mut host = one_box();
    let mut guest = Document::new(WorldBounds::default());
    guest.adopt_world(host.tree.world.clone());

    let world = guest.world_bounds();
    let brush = map_model::shapes::create_cuboid(
        &world,
        &geometry_math::bbox::Bbox3d::new(Vec3d::fill(128.0), Vec3d::fill(192.0)),
        "offline",
    )
    .expect("the offline brush builds");
    // The edit hook is the same buffer main.rs keeps while the link is down.
    let held: std::rc::Rc<std::cell::RefCell<Vec<Edit>>> = Default::default();
    guest.set_on_edit(std::rc::Rc::new({
        let sink = std::rc::Rc::clone(&held);
        move |edit: &Edit| sink.borrow_mut().push(edit.clone())
    }));
    guest.add_brushes(vec![brush]);
    guest.clear_on_edit();
    let offline: Vec<Edit> = held.borrow().clone();
    assert_eq!(offline.len(), 1, "the offline edit was held");

    // Meanwhile the host also worked: a second brush of its own.
    let brush = map_model::shapes::create_cuboid(
        &world,
        &geometry_math::bbox::Bbox3d::new(Vec3d::fill(-192.0), Vec3d::fill(-128.0)),
        "host_side",
    )
    .expect("the host brush builds");
    host.add_brushes(vec![brush]);

    // The rejoin: a fresh link, the host re-sends its world, the guest adopts it
    // and replays what it held -- the exact sequence main.rs runs.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let host_thread = std::thread::spawn(move || Session::host(port).unwrap());
    std::thread::sleep(Duration::from_millis(50));
    let guest_session = Session::join(&format!("127.0.0.1:{port}")).unwrap();
    let host_session = host_thread.join().unwrap();
    host_session.send_world(&host.tree.world);

    let bounds = world;
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut adopted = false;
    while Instant::now() < deadline && !adopted {
        for event in guest_session.poll(&bounds) {
            if let SessionEvent::AdoptWorld(incoming) = event {
                guest.adopt_world(incoming);
                for edit in &offline {
                    guest.apply_remote(edit);
                    guest_session.send_edit(edit);
                }
                adopted = true;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(adopted, "the world never arrived");

    // The host applies the replayed edit; both sides converge with both brushes.
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut applied = false;
    while Instant::now() < deadline && !applied {
        for event in host_session.poll(&bounds) {
            if let SessionEvent::ApplyEdit(edit) = event {
                host.apply_remote(&edit);
                applied = true;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(applied, "the replayed edit never arrived");

    let flatten = |doc: &Document| {
        map_model::io::map_writer::write_map(&node::flatten_tree(&doc.tree.world, doc.format, None))
    };
    assert_eq!(
        flatten(&host),
        flatten(&guest),
        "both sides hold both brushes"
    );
    assert!(
        flatten(&host).contains("offline"),
        "the outage edit survived"
    );
    assert!(
        flatten(&host).contains("host_side"),
        "the host's work survived"
    );
}

#[test]
fn a_guest_joins_a_real_session_and_follows_the_host() {
    use crate::collab::{Session, SessionEvent};
    use std::net::TcpListener;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let host_thread = std::thread::spawn(move || Session::host(port).unwrap());
    std::thread::sleep(Duration::from_millis(50));
    let guest_session = Session::join(&format!("127.0.0.1:{port}")).unwrap();
    let host_session = host_thread.join().unwrap();

    let mut host = one_box();
    let mut guest = Document::new(WorldBounds::default());
    let bounds = host.world_bounds();

    let flatten = |doc: &Document| {
        map_model::io::map_writer::write_map(&node::flatten_tree(&doc.tree.world, doc.format, None))
    };

    // The host sends its world, the guest polls and adopts it -- the join handover.
    host_session.send_world(&host.tree.world);
    host.set_on_edit(Rc::new({
        let session = host_session;
        move |edit: &Edit| session.send_edit(edit)
    }));

    let poll_until = |guest: &mut Document, want_applied: bool| {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            let events = guest_session.poll(&bounds);
            let mut did_something = false;
            for event in events {
                match event {
                    SessionEvent::AdoptWorld(world) => {
                        guest.adopt_world(world);
                        did_something = true;
                    }
                    SessionEvent::ApplyEdit(edit) => {
                        guest.apply_remote(&edit);
                        did_something = true;
                    }
                    other => panic!("unexpected event: {:?}", std::mem::discriminant(&other)),
                }
            }
            if did_something {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!want_applied, "the guest never received anything");
    };

    poll_until(&mut guest, true);
    assert_eq!(
        flatten(&host),
        flatten(&guest),
        "the guest adopted the host's world"
    );

    // The host makes an edit; the guest picks it up off the socket.
    host.select_all();
    assert!(host.translate_selection(Vec3d::new(32.0, 0.0, 0.0)));
    poll_until(&mut guest, true);
    assert_eq!(flatten(&host), flatten(&guest), "after the host's move");
}

#[test]
fn surface_material_survives_history_save_and_geometry_edits() {
    use map_model::brush_face::{FaceColor, Rgb, SurfaceMaterial};
    for paint in [
        None,
        Some(FaceColor::solid(Rgb::new(200, 100, 30))),
        Some(FaceColor::gradient(
            Rgb::new(200, 100, 30),
            Rgb::new(20, 40, 80),
            45.0,
        )),
    ] {
        let mut doc = one_box();
        doc.select_all();
        doc.set_face_color(paint);
        let id = *doc.selection_ids().iter().next().unwrap();
        let face = FaceRef { node: id, face: 0 };
        let surface = SurfaceMaterial {
            roughness: 20,
            metallic: 80,
            emission: 250,
        };
        assert_eq!(doc.set_surface_material_on(&[face], Some(surface)), 1);
        assert_eq!(doc.face_at(face).unwrap().surface_material, Some(surface));
        let saved = doc.to_map_text();
        let map = read_map(&saved, None).unwrap();
        let faces = &map.entities[0].brushes[0].faces;
        assert_eq!(
            faces
                .iter()
                .filter(|f| f.surface_material == Some(surface))
                .count(),
            1
        );
        assert!(faces.iter().all(|f| f.paint() == paint));
        doc.undo();
        assert_eq!(doc.face_at(face).unwrap().surface_material, None);
        doc.redo();
        assert_eq!(doc.to_map_text(), saved);
        doc.rotate_selection(Vec3d::new(0., 0., 1.), 30.);
        let triangles = doc.triangles(&crate::view_filters::ViewFilters::default());
        assert_eq!(
            triangles
                .iter()
                .filter(|t| t.surface_material == Some(surface))
                .count(),
            2
        );
        assert_eq!(doc.set_surface_material(None), 1);
        assert!(doc
            .triangles(&crate::view_filters::ViewFilters::default())
            .iter()
            .all(|t| t.surface_material.is_none()));
    }
}

#[test]
fn point_light_symbol_tracks_color_selection_and_stays_out_of_render_output() {
    let mut document = Document::new(WorldBounds::default());
    document.create_point_entity("light", Vec3d::new(24.0, 16.0, 32.0));
    let entity = document.entity_boxes()[0].node;
    document.set_entity_property(entity, "light_color", "40 120 255");
    document.select_only(entity);
    let original = document.to_map_text();
    let bounds = document.entity_boxes()[0].bounds;
    let filters = crate::view_filters::ViewFilters::default();
    let symbol = document.triangles(&filters);
    assert!(symbol.len() > 12);
    assert!(symbol.iter().all(|t| t.selected));
    for triangle in &symbol {
        assert!((vec::length(triangle.normal) - 1.0).abs() < 1e-9);
        for p in triangle.positions {
            for axis in 0..3 {
                assert!(p[axis] >= bounds.min[axis] && p[axis] <= bounds.max[axis]);
            }
        }
    }
    assert!(symbol.iter().any(|t| t.color[2] > t.color[0] + 0.5));
    assert!(
        document.render_triangles().is_empty(),
        "editor symbols must not enter image renders"
    );
    assert!(document
        .triangles(&crate::view_filters::ViewFilters {
            show_point_entities: false,
            ..filters
        })
        .is_empty());
    assert_eq!(document.to_map_text(), original);
    document.set_entity_property(entity, "light_color", "NaN 1 2");
    assert!(document
        .triangles(&filters)
        .iter()
        .all(|t| t.color.iter().all(|v| v.is_finite())));
}

#[test]
fn library_asset_placement_aligns_base_and_footprint_to_active_grid() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let grid = crate::grid::Grid::new(2);
    let mut checked = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "map") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let mut document = Document::new(WorldBounds::default());
        document.paste_asset_on_grid(&text, Vec3d::new(-128.0, 96.0, 64.0), "Asset", &grid);
        let bounds = Bbox3d::merge_all(
            node::brushes_with_ids(document.tree_world())
                .into_iter()
                .flat_map(|(_, brush)| [brush.bounds().min, brush.bounds().max]),
        )
        .unwrap();
        for axis in 0..3 {
            assert!(
                (bounds.min[axis] / 4.0 - (bounds.min[axis] / 4.0).round()).abs() < 1e-6,
                "{}: {:?}",
                path.display(),
                bounds.min
            );
        }
        assert!((bounds.min.z() - 64.0).abs() < 1e-6, "{}", path.display());
        for (_, brush) in node::brushes_with_ids(document.tree_world()) {
            assert!(brush.vertices_are_on_grid(0.125), "{}", path.display());
        }
        assert_eq!(document.groups().len(), 1);
        assert!(document.undo().is_some());
        assert_eq!(document.brush_count(), 0);
        checked += 1;
    }
    assert!(checked >= 102);
}

#[test]
fn modular_pipe_ports_remain_on_grid_four_after_asset_placement() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (name, ports) in [
        ("pipe_straight", vec![[-32., 0., 12.], [32., 0., 12.]]),
        ("pipe_elbow", vec![[-36., 0., 12.], [0., 36., 12.]]),
        (
            "pipe_tee",
            vec![[-32., 0., 12.], [32., 0., 12.], [0., 36., 12.]],
        ),
        ("pipe_valve", vec![[-32., 0., 12.], [32., 0., 12.]]),
        ("pipe_reducer", vec![[-24., 0., 12.], [24., 0., 12.]]),
        ("pipe_broken", vec![[-32., 0., 12.], [32., 0., 12.]]),
    ] {
        let text = std::fs::read_to_string(directory.join(format!("{name}.map"))).unwrap();
        let mut original = Document::new(WorldBounds::default());
        original.paste_at(&text, None);
        let before = original.selection_bounds().unwrap();
        let mut document = Document::new(WorldBounds::default());
        document.paste_asset_on_grid(
            &text,
            Vec3d::new(128., 96., 64.),
            name,
            &crate::grid::Grid::new(2),
        );
        let after = document.selection_bounds().unwrap();
        let delta = after.min - before.min;
        for port in ports {
            let p = Vec3d::new(port[0], port[1], port[2]) + delta;
            for axis in 0..3 {
                assert!(
                    (p[axis] / 4. - (p[axis] / 4.).round()).abs() < 1e-6,
                    "{name}: {p:?}"
                );
                assert!((after.size()[axis] / 8. - (after.size()[axis] / 8.).round()).abs() < 1e-6);
            }
        }
    }
}

#[test]
fn fan_frame_contacts_are_skip_and_preview_does_not_use_the_frame_paint() {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fan.map"),
    )
    .unwrap();
    let mut document = Document::new(WorldBounds::default());
    document.paste_at(&text, None);
    let brushes = node::brushes_with_ids(document.tree_world());
    let mut contacts = 0;
    for (_, brush) in brushes.iter().take(8) {
        brush.for_each_geometry_face(|_, face, polygon| {
            let normal = face.normal();
            // The shared diagonal cuts have both X and Z components. Outer
            // bevels also have these normals, so count their skip designation.
            if face.material_name == "skip" {
                assert!(normal.x().abs() > 0.5 && normal.z().abs() > 0.5);
                assert_eq!(polygon.len(), 4);
                contacts += 1;
            }
        });
    }
    assert_eq!(contacts, 16, "both sides of all eight frame joins");
    let triangles = document.triangles(&crate::view_filters::ViewFilters::default());
    assert!(
        triangles
            .iter()
            .filter(|t| t.material == Some(render::SKIP_PREVIEW_MATERIAL)
                && t.alpha < 1.0
                && t.corner_colors.is_none())
            .count()
            >= 32
    );
    assert!(triangles
        .iter()
        .any(|t| t.color == [48. / 255., 106. / 255., 113. / 255.]));
}

#[test]
fn orthographic_move_can_hit_a_selected_brush_behind_another() {
    use geometry_math::ray::Ray3d;
    let mut document = one_box();
    let rear = node::brushes_with_ids(&document.tree.world)[0].0;
    let front = map_model::shapes::create_cuboid(
        &document.world_bounds(),
        &Bbox3d::new(Vec3d::new(-32.0, -32.0, 64.0), Vec3d::new(32.0, 32.0, 96.0)),
        "stone",
    )
    .unwrap();
    document.add_brushes(vec![front]);
    document.select_only(rear);
    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    let front = document.pick(&ray).unwrap().node;
    assert_ne!(front, rear);
    assert_eq!(document.pick_selected(&ray).unwrap().node, rear);
    let before = document.brush(front).unwrap().bounds();
    assert!(document.translate_selection(Vec3d::new(8.0, 0.0, 0.0)));
    assert_eq!(document.brush(front).unwrap().bounds(), before);
    assert_eq!(document.selection_count(), 1);

    // The foreground brush can still be picked outside the selected geometry.
    let edge = Ray3d::new(Vec3d::new(-24.0, 0.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    assert!(document.pick_selected(&edge).is_none());
    assert_eq!(document.pick(&edge).unwrap().node, front);
}

#[test]
fn selected_group_picking_does_not_fill_the_gap_between_members() {
    use geometry_math::ray::Ray3d;
    let mut document = one_box();
    let second = map_model::shapes::create_cuboid(
        &document.world_bounds(),
        &Bbox3d::new(Vec3d::new(64.0, -16.0, -16.0), Vec3d::new(96.0, 16.0, 16.0)),
        "stone",
    )
    .unwrap();
    document.add_brushes(vec![second]);
    document.select_all();
    document.group_selection("Pair");
    let ray = |x| Ray3d::new(Vec3d::new(x, 0.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    assert!(document.pick_selected(&ray(0.0)).is_some());
    assert!(document.pick_selected(&ray(80.0)).is_some());
    assert!(document.pick_selected(&ray(40.0)).is_none());
}

#[test]
fn special_materials_survive_loading_and_removing_project_textures() {
    let mut document = one_box();
    for name in ["skip", "clip", "origin"] {
        assert!(document.materials().get(name).is_some());
    }
    let directory = material_dir("special-fallback");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([12, 34, 56, 255]))
        .save(directory.join("clip.png"))
        .unwrap();
    document.load_materials(&directory).unwrap();
    assert_eq!(document.current_material(), "brick");
    for name in ["skip", "clip", "origin"] {
        assert!(document.materials().get(name).is_some());
    }
    assert_eq!(document.materials().get("clip").unwrap().width, 2);
    document.remove_material_source(0);
    assert!(document
        .materials()
        .get("clip")
        .unwrap()
        .path
        .starts_with("builtin/special"));
    for name in ["skip", "clip", "origin"] {
        assert!(document.materials().get(name).is_some());
    }
    document.set_current_material("skip");
    document.select_all();
    document.apply_current_material();
    let issues = crate::validation::validate(&document);
    assert!(!issues.iter().any(|i| i.kind == "missing material"));
}

#[test]
fn keyframe_animation_persists_and_undoes_as_one_edit() {
    let mut document = one_box();
    document.select_all();
    let before = document.to_map_text();
    document.run_as_one_edit(|d| {
        let id = d.create_brush_entity("func_animate").unwrap();
        d.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 2 128 0 0 0 0 90");
        d.set_entity_property(id, "animation_pivot", "0 0 0");
        d.set_entity_property(id, "animation_mode", "pingpong");
    });
    let text = document.to_map_text();
    assert!(text.contains("animation_keys"));
    let parsed = map_model::io::map_reader::read_map(&text, None).unwrap();
    let tree = map_model::node::build_tree(
        &parsed,
        &geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0),
    );
    let motions = crate::motion_preview::motions(&tree.world);
    assert_eq!(motions.len(), 1);
    document.undo();
    assert_eq!(document.to_map_text(), before);
    document.redo();
    assert_eq!(document.to_map_text(), text);
}

#[test]
fn live_gradient_edits_group_without_swallowing_other_edits() {
    let mut d = one_box();
    d.select_all();
    let before = d.to_map_text();
    let mut session = super::LivePaint::default();
    for angle in [0.0, 15.0, 30.0] {
        d.set_face_color_live(
            Some(map_model::brush_face::FaceColor::Gradient {
                from: map_model::brush_face::Rgb::new(255, 0, 0),
                to: map_model::brush_face::Rgb::new(0, 0, 255),
                angle,
                alpha: 255,
            }),
            &[],
            &mut session,
        );
    }
    let gradient = d.to_map_text();
    d.undo();
    assert_eq!(d.to_map_text(), before);
    d.redo();
    assert_eq!(d.to_map_text(), gradient);
    d.set_face_color(Some(map_model::brush_face::FaceColor::solid(
        map_model::brush_face::Rgb::new(0, 255, 0),
    )));
    let solid = d.to_map_text();
    d.set_face_color_live(
        Some(map_model::brush_face::FaceColor::solid(
            map_model::brush_face::Rgb::new(255, 255, 0),
        )),
        &[],
        &mut session,
    );
    d.undo();
    assert_eq!(d.to_map_text(), solid);
    d.undo();
    assert_eq!(d.to_map_text(), gradient);
}

#[test]
fn timeline_last_frame_does_not_wrap_and_preview_does_not_edit() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map");
    let mut d = Document::open(path, Default::default()).unwrap();
    d.select_all();
    let id = d.create_brush_entity("func_animate").unwrap();
    d.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 2 128 0 0 0 0 0");
    d.set_entity_property(id, "animation_mode", "loop");
    d.set_entity_property(id, "start_on", "1");
    let before = d.to_map_text();
    let mesh = d.preview_mesh(&Default::default());
    assert!(!mesh.motions.is_empty());
    let frame = mesh.at_frame(2.0);
    let loop_frame = mesh.at(2.0);
    for (range, _) in &mesh.motions {
        for index in range.clone() {
            assert!(
                (frame[index].positions[0].x() - mesh.triangles[index].positions[0].x() - 128.0)
                    .abs()
                    < 1e-8
            );
            for (a, b) in loop_frame[index]
                .positions
                .into_iter()
                .zip(mesh.triangles[index].positions)
            {
                assert!(geometry_math::vec::length(a - b) < 1e-8);
            }
        }
    }
    assert_eq!(d.to_map_text(), before);
}

#[test]
fn animation_of_brush_group_preserves_group_and_moves_every_member() {
    let mut d = two_boxes();
    let group = d.group_selection("Moving platform").unwrap();
    let member = node::contents_of(d.tree_world(), group)
        .into_iter()
        .find(|id| d.brush(*id).is_some())
        .unwrap();
    d.select_only(member);
    let before = d.to_map_text();
    let members = d.selection_ids().clone();
    let entity = d.create_brush_entity("func_animate").unwrap();
    let created = d.to_map_text();
    let NodeKind::Group(g) = &node::find_node(d.tree_world(), group).unwrap().kind else {
        panic!()
    };
    assert_eq!(g.children.len(), 1);
    assert_eq!(g.children[0].id, entity);
    assert!(members
        .iter()
        .all(|id| node::contents_of(d.tree_world(), entity).contains(id)));
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.redo().unwrap();
    assert_eq!(d.to_map_text(), created);
    d.set_entity_property(entity, "animation_keys", "0 0 0 0 0 0 0; 2 128 32 64 0 0 0");
    d.select_only(member);
    assert_eq!(
        crate::animation_pose::Pose::selected(&d, 1.0)
            .unwrap()
            .entity,
        entity
    );
    let mesh = d.preview_mesh(&Default::default());
    let moved = mesh.at_frame(2.0);
    assert!(!mesh.motions.is_empty());
    for (range, _) in &mesh.motions {
        for i in range.clone() {
            for (a, b) in moved[i]
                .positions
                .into_iter()
                .zip(mesh.triangles[i].positions)
            {
                assert!(geometry_math::vec::length(a - b - Vec3d::new(128.0, 32.0, 64.0)) < 1e-8);
            }
        }
    }
    let text = d.to_map_text();
    let parsed = map_model::io::map_reader::read_map(&text, None).unwrap();
    let reloaded = Document::from_map(&parsed, WorldBounds::default(), None);
    assert_eq!(reloaded.brush_count(), 2);
    assert_eq!(node::groups_with_ids(reloaded.tree_world()).len(), 1);
    assert_eq!(
        node::entities_with_ids(reloaded.tree_world())
            .iter()
            .filter(|(_, e)| e.property("animation_keys").is_some())
            .count(),
        1
    );
}

/// The edges walked again for the selection alone (a drag) are the ones a full walk
/// gives, through moves, a rollback, a commit, undo and redo; the unselected edges'
/// generation stands still through all of that and moves when an unselected brush does.
#[test]
fn the_selection_only_edge_refill_matches_a_full_walk() {
    let mut document = two_boxes();
    let ids: Vec<NodeId> = node::brushes_with_ids(document.tree_world())
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    document.select_only(ids[0]);
    let full = |document: &mut Document| {
        document.mark_changed();
        (document.edges().clone(), document.selected_edges().clone())
    };
    let fast = |document: &Document| (document.edges().clone(), document.selected_edges().clone());
    let start = full(&mut document);
    let generation = document.static_edges_generation();

    document.translate_selection(Vec3d::new(8.0, 0.0, 0.0));
    let after_move = fast(&document);
    assert_ne!(after_move.1, start.1, "the selected edges moved");
    assert_eq!(after_move.0, start.0, "the unselected ones did not");
    assert_eq!(document.static_edges_generation(), generation);
    assert_eq!(after_move, full(&mut document));

    document.begin_transaction();
    for step in 1..=3 {
        document.rollback_transaction();
        document.rotate_selection_about(
            Vec3d::zero(),
            Vec3d::new(0.0, 0.0, 1.0),
            10.0 * f64::from(step),
        );
        let dragged = fast(&document);
        assert_eq!(dragged, full(&mut document));
    }
    document.commit_transaction();
    assert_eq!(document.static_edges_generation(), generation);
    let committed = fast(&document);
    document.undo();
    assert_eq!(fast(&document), after_move);
    document.redo();
    assert_eq!(fast(&document), committed);
    assert_eq!(document.static_edges_generation(), generation);

    // Moving the other brush changes what a flat view's cached picture shows.
    document.select_only(ids[1]);
    let _ = document.edges();
    let generation = document.static_edges_generation();
    document.translate_selection(Vec3d::new(0.0, 8.0, 0.0));
    document.select_only(ids[0]);
    assert_ne!(document.static_edges_generation(), generation);
    let shown = fast(&document);
    assert_eq!(shown, full(&mut document));
}

/// Adding a folder keeps an unchanged folder's materials without reading it again, but a
/// folder whose files changed is read again: the new pixels are what the collection holds.
#[test]
fn an_unchanged_folder_is_kept_and_a_changed_one_read_again_when_another_is_added() {
    let write = |dir: &std::path::Path, name: &str, colour: u8, size: u32| {
        std::fs::create_dir_all(dir).unwrap();
        let pixels: Vec<u8> = (0..size * size).flat_map(|_| [colour, 0, 0, 255]).collect();
        std::fs::write(
            dir.join(name),
            crate::assets::encode_png(size, size, &pixels).unwrap(),
        )
        .unwrap();
    };
    let root = std::env::temp_dir().join(format!("heron-folder-reuse-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (a, b, c) = (root.join("a"), root.join("b"), root.join("c"));
    write(&a, "wall.png", 10, 8);
    write(&b, "floor.png", 20, 8);
    write(&c, "roof.png", 30, 8);
    let mut document = Document::new(WorldBounds::default());
    document.load_materials(&a).unwrap();
    document.load_materials(&b).unwrap();
    assert_eq!(
        document.materials().get("wall").unwrap().pixels().unwrap()[0],
        10
    );
    assert_eq!(
        document.materials().get("floor").unwrap().pixels().unwrap()[0],
        20
    );

    // `a` changes on disk (a new size, so its stamp moves even within the same second).
    write(&a, "wall.png", 99, 16);
    document.load_materials(&c).unwrap();
    let wall = document.materials().get("wall").unwrap();
    assert_eq!(
        (wall.width, wall.pixels().unwrap()[0]),
        (16, 99),
        "the changed folder was read again"
    );
    assert_eq!(
        document.materials().get("floor").unwrap().pixels().unwrap()[0],
        20
    );
    assert_eq!(
        document.materials().get("roof").unwrap().pixels().unwrap()[0],
        30
    );

    // Taking a source away leaves the others as they were.
    assert!(document.remove_material_source(1));
    assert!(document.materials().get("floor").is_none());
    assert_eq!(
        document.materials().get("wall").unwrap().pixels().unwrap()[0],
        99
    );
    assert_eq!(
        document.materials().get("roof").unwrap().pixels().unwrap()[0],
        30
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_linked_sync_rebuilds_only_the_copies_of_what_changed() {
    let mut document = two_boxes();
    document.group_selection("Doorway");
    document.create_linked_duplicate(128.0);
    let groups: Vec<NodeId> = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .collect();
    let source_id = *groups
        .iter()
        .find(|id| !document.is_selected(node::contents_of(&document.tree.world, **id)[0]))
        .unwrap();
    let copy_id = *groups.iter().find(|id| **id != source_id).unwrap();
    let corners = |document: &Document, group: NodeId| -> Vec<(NodeId, Vec<Vec3d>)> {
        node::contents_of(&document.tree.world, group)
            .into_iter()
            .map(|id| (id, document.brush(id).unwrap().vertex_positions()))
            .collect()
    };
    let before = corners(&document, copy_id);

    // Move the source's brushes (a click selects the whole closed group): their copies
    // follow piecewise, keeping their ids, rather than being rebuilt as new nodes.
    document.select_only(node::contents_of(&document.tree.world, source_id)[0]);
    assert!(document.translate_selection(Vec3d::new(0.0, 0.0, 32.0)));
    let after = corners(&document, copy_id);
    assert_eq!(
        after.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        before.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        "the copy's children keep their ids"
    );
    assert!(
        after.iter().zip(&before).all(|(a, b)| a.1 != b.1),
        "the copies followed"
    );

    // The piecewise result is what a full rebuild (the menu action) makes.
    let mut piecewise: Vec<Vec<Vec3d>> = after.into_iter().map(|(_, c)| c).collect();
    document.select_only(node::contents_of(&document.tree.world, source_id)[0]);
    document.sync_linked_groups();
    let copy_id = node::groups_with_ids(&document.tree.world)
        .iter()
        .map(|(id, _)| *id)
        .find(|id| *id != source_id)
        .unwrap();
    let mut full: Vec<Vec<Vec3d>> = corners(&document, copy_id)
        .into_iter()
        .map(|(_, c)| c)
        .collect();
    let key = |c: &Vec<Vec3d>| format!("{c:?}");
    piecewise.sort_by_key(key);
    full.sort_by_key(key);
    assert_eq!(piecewise, full);
}

#[test]
fn clipping_is_one_undo_step_and_keeps_brush_entity_parent() {
    let mut d = Document::new(WorldBounds::default());
    let brush = map_model::shapes::create_cuboid(
        &d.world_bounds(),
        &Bbox3d::new(Vec3d::fill(-64.0), Vec3d::fill(64.0)),
        "stone",
    )
    .unwrap();
    d.add_brushes(vec![brush]);
    d.select_all();
    let entity = d.create_brush_entity("project_door").unwrap();
    let brush_id = node::brushes_with_ids(d.tree_world())[0].0;
    d.select_only(brush_id);
    let before = d.to_map_text();
    let mut clip = crate::clip_tool::ClipTool::new();
    clip.add_point(
        Vec3d::new(0.0, -64.0, 0.0),
        vec![Vec3d::new(0.0, 0.0, -1.0)],
    );
    clip.add_point(Vec3d::new(0.0, 64.0, 0.0), vec![Vec3d::new(0.0, 0.0, -1.0)]);
    clip.toggle_side();
    assert_eq!(d.clip_selection(&clip), 1);
    assert_eq!(d.brush_count(), 2);
    for (id, _) in node::brushes_with_ids(d.tree_world()) {
        assert_eq!(d.entity_around(id).unwrap().id, entity);
    }
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.redo().unwrap();
    assert_eq!(d.brush_count(), 2);
}

fn game_asset_fixture(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/editor-checks/fixtures/game-assets")
        .join(file)
}

#[test]
fn game_and_mod_materials_models_dependencies_shaders_and_reload_share_the_same_winners() {
    let root =
        std::env::temp_dir().join(format!("heron-shared-game-assets-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for folder in [
        "id1/textures",
        "id1/progs",
        "first",
        "second/textures",
        "second/gfx",
        "explicit",
    ] {
        std::fs::create_dir_all(root.join(folder)).unwrap();
    }
    std::fs::copy(
        game_asset_fixture("red.png"),
        root.join("id1/textures/check.png"),
    )
    .unwrap();
    std::fs::copy(game_asset_fixture("mod.pk3"), root.join("first/pak0.pk3")).unwrap();
    std::fs::write(
        root.join("id1/progs/thing.mdl"),
        crate::entity_models::tests::one_triangle_model(),
    )
    .unwrap();
    std::fs::write(root.join("second/gfx/palette.lmp"), [0, 0, 255].repeat(256)).unwrap();
    std::fs::copy(
        game_asset_fixture("blue.png"),
        root.join("second/textures/check.png"),
    )
    .unwrap();
    std::fs::copy(
        game_asset_fixture("red.png"),
        root.join("explicit/check.png"),
    )
    .unwrap();
    crate::preferences::edit(|p| {
        p.game = "Quake".into();
        p.game_path = root.to_string_lossy().into_owned();
        p.game_mods = "first;second".into();
    });
    let mut doc = Document::new(WorldBounds::default());
    let fgd = root.join("objects.fgd");
    std::fs::write(&fgd,"@PointClass model({ \"path\": \"models/marker.obj\" }) = prop_marker []\n@PointClass model(\"progs/thing.mdl\") = mdl_marker []\n").unwrap();
    doc.load_definitions(&fgd).unwrap();
    let color = |doc: &Document| {
        doc.materials()
            .get("textures/CHECK")
            .unwrap()
            .pixels()
            .unwrap()[..4]
            .to_vec()
    };
    assert_eq!(color(&doc), [0, 0, 255, 255]);
    assert_eq!(
        doc.models["mdl_marker"].skins().next().unwrap().pixels[..4],
        [0, 0, 255, 255],
        "MDL palette comes from the winning mod"
    );
    let model = doc.models.get("prop_marker").unwrap();
    assert_eq!(model.skins().next().unwrap().pixels[..4], [0, 0, 255, 255]);
    assert!(model
        .triangles
        .iter()
        .any(|t| t.positions.iter().any(|p| p.x() == 7.0)));
    assert_eq!(
        doc.materials()
            .get("check")
            .unwrap()
            .normal_map
            .as_ref()
            .unwrap()
            .pixels()
            .unwrap()[..4],
        [128, 128, 255, 255]
    );
    assert!(doc
        .materials()
        .get("textures/tagged")
        .unwrap()
        .surface_parms
        .contains(&"nodraw".to_string()));
    let old_fs = doc.model_store.shared_assets().unwrap().clone();
    doc.load_entity_models();
    assert!(
        std::sync::Arc::ptr_eq(&old_fs, doc.model_store.shared_assets().unwrap()),
        "ordinary edits reuse the mounted archives"
    );
    crate::preferences::edit(|p| p.game_mods = "second;first".into());
    doc.load_entity_models();
    assert_eq!(color(&doc), [0, 255, 0, 255]);
    assert_eq!(
        doc.models["mdl_marker"].skins().next().unwrap().pixels[..4],
        [0, 255, 0, 255],
        "palette and model caches change with mod order"
    );
    assert_eq!(
        doc.models["prop_marker"].skins().next().unwrap().pixels[..4],
        [0, 255, 0, 255]
    );
    assert!(doc
        .materials()
        .get("check")
        .unwrap()
        .path
        .to_string_lossy()
        .contains("pak0.pk3!/textures/check.png"));
    doc.load_asset_materials(&root.join("explicit")).unwrap();
    assert_eq!(
        color(&doc),
        [0, 255, 0, 255],
        "game assets override generic library defaults"
    );
    doc.load_materials(root.join("explicit")).unwrap();
    assert_eq!(
        color(&doc),
        [255, 0, 0, 255],
        "explicit project materials override game defaults"
    );
    doc.remove_material_source(0);
    assert_eq!(color(&doc), [0, 255, 0, 255]);
    std::fs::create_dir_all(root.join("first/textures")).unwrap();
    std::fs::copy(
        game_asset_fixture("red.png"),
        root.join("first/textures/check.png"),
    )
    .unwrap();
    doc.reload_game_assets();
    assert_eq!(
        color(&doc),
        [255, 0, 0, 255],
        "Reload sees newly added loose overrides"
    );
    assert_eq!(
        doc.models["prop_marker"].skins().next().unwrap().pixels[..4],
        [255, 0, 0, 255]
    );
    assert!(
        doc.game_asset_warnings().is_empty(),
        "{:?}",
        doc.game_asset_warnings()
    );
    crate::preferences::set_current(Default::default());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn shared_game_texture_pixels_survive_portable_engine_and_obj_exports_without_unpacking() {
    let root =
        std::env::temp_dir().join(format!("heron-shared-assets-export-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("mod")).unwrap();
    std::fs::copy(game_asset_fixture("mod.pk3"), root.join("mod/pak0.pk3")).unwrap();
    crate::preferences::edit(|p| {
        p.game = "heron_brush".into();
        p.game_path = root.to_string_lossy().into_owned();
        p.game_mods = "mod".into();
    });
    let mut doc = Document::new(WorldBounds::default());
    let brush = map_model::shapes::create_cuboid(
        &doc.world_bounds(),
        &Bbox3d::from_scalars(-16.0, 16.0),
        "check",
    )
    .unwrap();
    doc.add_brushes(vec![brush]);
    for target in [
        crate::export::Target::Glb,
        crate::export::Target::Godot,
        crate::export::Target::ThreeJs,
    ] {
        let out = root.join(format!("{:?}", target));
        std::fs::create_dir_all(&out).unwrap();
        let options = crate::export::Options {
            target,
            name: "assets".into(),
            ..Default::default()
        };
        let report = crate::export::export(&doc, &out, &options).unwrap();
        assert!(!report.files.is_empty());
        let mut found = false;
        for file in report.files {
            if file.extension().is_some_and(|ext| ext == "png") {
                let pixels = image::open(&file).unwrap().to_rgba8();
                if pixels.get_pixel(0, 0).0 == [0, 255, 0, 255] {
                    found = true;
                }
            }
        }
        if target == crate::export::Target::Glb {
            let model = map_model::glb_model::read(&std::fs::read(out.join("assets.glb")).unwrap())
                .unwrap();
            assert_eq!(model.skins().next().unwrap().pixels[..4], [0, 255, 0, 255]);
        } else {
            assert!(found, "{target:?} packages the winning green texture");
        }
    }
    let (_, files) = crate::obj_export::files(
        &doc,
        &root.join("mesh.obj"),
        crate::obj_export::ObjOptions {
            selection_only: false,
            y_up: true,
        },
    );
    let mtl = files
        .iter()
        .find(|(p, _)| p.extension().is_some_and(|e| e == "mtl"))
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&mtl.1).contains("!/"),
        "MTL must not contain an archive pseudo-path"
    );
    assert!(files
        .iter()
        .any(|(p, bytes)| p.extension().is_some_and(|e| e == "png")
            && image::load_from_memory(bytes)
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)
                .0
                == [0, 255, 0, 255]));
    assert!(
        !root.join("mod/textures").exists(),
        "archives were never unpacked into the game"
    );
    crate::preferences::set_current(Default::default());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn md2_entity_preview_uses_archive_skin_class_scale_and_entity_placement() {
    let root = std::env::temp_dir().join("heron-md2-document");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("baseq2")).unwrap();
    std::fs::write(
        root.join("baseq2/pak0.pk3"),
        include_bytes!("../../../../examples/editor-checks/fixtures/md2/models.pk3"),
    )
    .unwrap();
    let before = crate::preferences::pref(Clone::clone);
    crate::preferences::edit(|p| {
        p.game = "Quake 2".into();
        p.game_path = root.to_string_lossy().into_owned();
        p.game_mods.clear();
    });
    let fgd = root.join("preview.fgd");
    std::fs::write(&fgd,"@PointClass size(-8 -8 -8, 8 8 8) model({\"path\":\":models/check/preview.md2\",\"scale\":2}) = prop_md2 : \"MD2\" []\n").unwrap();
    let mut doc = Document::new(WorldBounds::default());
    doc.load_definitions(&fgd).unwrap();
    doc.create_point_entity("prop_md2", Vec3d::new(128., 0., 0.));
    let triangles = doc.triangles(&crate::view_filters::ViewFilters::default());
    assert_eq!(triangles.len(), 6);
    let points: Vec<_> = triangles.iter().flat_map(|t| t.positions).collect();
    assert!(points.contains(&Vec3d::new(64., -48., 0.)));
    assert!(points.contains(&Vec3d::new(128., 0., 128.)));
    for t in &triangles {
        let material = &doc.materials.materials()[t.material.unwrap()];
        assert_eq!((material.width, material.height), (4, 2));
        assert_eq!(&material.pixels().unwrap()[..4], &[255, 40, 40, 255]);
    }
    assert_eq!(
        doc.triangles(&crate::view_filters::ViewFilters {
            show_point_entity_models: false,
            ..Default::default()
        })
        .len(),
        12
    );
    crate::preferences::edit(|p| *p = before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn md3_entity_preview_uses_quake_three_archive_surfaces_scale_and_normals() {
    let root = std::env::temp_dir().join(format!(
        "heron-md3-document-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("baseq3")).unwrap();
    std::fs::write(
        root.join("baseq3/pak0.pk3"),
        include_bytes!("../../../../examples/editor-checks/fixtures/md3/models.pk3"),
    )
    .unwrap();
    let before = crate::preferences::pref(Clone::clone);
    crate::preferences::edit(|p| {
        p.game = "Quake 3".into();
        p.game_path = root.to_string_lossy().into_owned();
        p.game_mods.clear();
    });
    assert_eq!(crate::games::current().unwrap().name, "Quake 3");
    let fgd = root.join("preview.fgd");
    std::fs::write(&fgd, r#"@PointClass size(-8 -8 -8, 8 8 8) model({"path":":models/check/preview.md3","scale":2}) = prop_md3 : "MD3" []
"#).unwrap();
    let mut doc = Document::new(WorldBounds::default());
    doc.load_definitions(&fgd).unwrap();
    doc.create_point_entity("prop_md3", Vec3d::new(128., 0., 0.));
    let triangles = doc.triangles(&crate::view_filters::ViewFilters::default());
    assert_eq!(triangles.len(), 2);
    for (i, triangle) in triangles.iter().enumerate() {
        assert_eq!(
            triangle.positions,
            [
                Vec3d::new(126., 0., (i * 2) as f64),
                Vec3d::new(130., 0., (i * 2) as f64),
                Vec3d::new(128., 2., (i * 2) as f64)
            ]
        );
        let expected_normal = if i == 0 {
            Vec3d::new(1., 0., 0.)
        } else {
            Vec3d::new(0., 1., 0.)
        };
        for n in triangle.corner_normals.unwrap() {
            assert!(geometry_math::vec::length(n - expected_normal) < 1e-12);
        }
        let material = &doc.materials.materials()[triangle.material.unwrap()];
        assert_eq!((material.width, material.height), (2, 2));
        let expected = if i == 0 {
            [240, 20, 30, 255]
        } else {
            [10, 220, 40, 255]
        };
        assert_eq!(&material.pixels().unwrap()[..4], &expected);
    }
    assert_eq!(
        doc.triangles(&crate::view_filters::ViewFilters {
            show_point_entity_models: false,
            ..Default::default()
        })
        .len(),
        12
    );
    assert!(!root.join("baseq3/models").exists());
    crate::preferences::edit(|p| *p = before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn water_volume_contract_and_runtime_adapters_export_to_all_five_targets() {
    use crate::export::{self, Target};
    let dir = std::env::temp_dir().join("heron-water-check");
    std::fs::create_dir_all(&dir).unwrap();
    let mut doc = Document::new(WorldBounds::default());
    let mut add = |class: &str, min: Vec3d, max: Vec3d, props: &[(&str, &str)]| {
        doc.clear_selection();
        let brush = map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &geometry_math::bbox::Bbox3d::new(min, max),
            "plain",
        )
        .unwrap();
        doc.add_brushes(vec![brush]);
        doc.set_face_color(Some(map_model::brush_face::FaceColor::Solid {
            color: map_model::brush_face::Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            alpha: 255,
        }));
        if class != "worldspawn" {
            let id = doc.create_brush_entity(class).unwrap();
            for (key, value) in props {
                doc.set_entity_property(id, key, value);
            }
        }
    };
    add(
        "worldspawn",
        Vec3d::new(-320., -320., -160.),
        Vec3d::new(320., 320., -128.),
        &[],
    );
    add(
        "water_volume",
        Vec3d::new(-128., -128., -128.),
        Vec3d::new(128., 128., 0.),
        &[("targetname", "pool"), ("water_color", "0.05 0.38 0.48")],
    );
    add(
        "custom_pool",
        Vec3d::new(-64., -64., -96.),
        Vec3d::new(64., 64., -32.),
        &[
            ("water_enabled", "1"),
            ("water_priority", "10"),
            ("water_surface_material", "0"),
            ("targetname", "overlap"),
        ],
    );
    add(
        "water_volume",
        Vec3d::new(256., 256., -128.),
        Vec3d::new(320., 320., 0.),
        &[("water_enabled", "0"), ("targetname", "disabled")],
    );
    doc.create_point_entity("info_player_start", Vec3d::new(0., 0., -96.));
    std::fs::write(dir.join("water.map"), doc.to_map_text()).unwrap();
    for (target, sub) in [
        (Target::ThreeJs, "three"),
        (Target::Bevy, "bevy"),
        (Target::Unity, "unity/Assets/Water"),
        (Target::Flax, "flax/Content/arena"),
        (Target::Defold, "defold/main"),
    ] {
        let path = dir.join(sub);
        std::fs::create_dir_all(&path).unwrap();
        let project = if target == Target::Unity {
            dir.join("unity")
        } else if target == Target::Defold {
            dir.join("defold")
        } else if target == Target::Flax {
            dir.join("flax")
        } else {
            path.clone()
        };
        let report = export::export(
            &doc,
            &path,
            &export::Options {
                target,
                name: "water".into(),
                scale: 32.,
                project_dir: project.to_string_lossy().into_owned(),
                asset_dir: "/main/water".into(),
                content_dir: "arena/water".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!report.warnings.iter().any(|w| w.starts_with("Water ")));
        let json: serde_json::Value = serde_json::from_slice(
            &std::fs::read(path.join(if target == Target::Flax {
                "water.level.json"
            } else {
                "water.json"
            }))
            .unwrap(),
        )
        .unwrap();
        let entities = json["entities"].as_array().unwrap();
        let pool = entities
            .iter()
            .find(|e| e["name"] == "water_volume")
            .unwrap();
        assert_eq!(pool["collision"]["sensor"], true);
        assert_eq!(pool["collision"]["kind"], "convex");
        let water: serde_json::Value = serde_json::from_str(
            pool["properties"]["_heron_water"]["value"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(water["version"], 1);
        assert_eq!(water["surface_indices"].as_array().unwrap().len(), 6);
        assert_eq!(water["swim_speed"], 3.);
        assert_eq!(water["volumes"][0]["planes"].as_array().unwrap().len(), 6);
        let overlap = entities
            .iter()
            .find(|e| e["name"] == "custom_pool")
            .unwrap();
        let custom: serde_json::Value = serde_json::from_str(
            overlap["properties"]["_heron_water"]["value"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(custom["priority"], 10);
        assert_eq!(custom["surface_material"], false);
        assert!(entities
            .iter()
            .find(|e| e["name"] == "water_volume_1")
            .unwrap()["properties"]
            .get("_heron_water")
            .is_none());
        if target == Target::Bevy {
            std::fs::write(
                dir.join("water-data.json"),
                serde_json::to_vec(&water).unwrap(),
            )
            .unwrap();
        }
        if target == Target::ThreeJs {
            // Godot has no level JSON; its volumes are nodes in the scene.
            let path = dir.join("godot");
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join("project.godot"), "").unwrap();
            let report = export::export(
                &doc,
                &path,
                &export::Options {
                    target: Target::Godot,
                    name: "water".into(),
                    scale: 32.,
                    project_dir: path.to_string_lossy().into_owned(),
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(
                !report.warnings.iter().any(|w| w.starts_with("Water ")),
                "{:?}",
                report.warnings
            );
            let scene = std::fs::read_to_string(path.join("water.tscn")).unwrap();
            assert_eq!(
                scene
                    .matches("[node name=\"heron_water\" type=\"Node\"")
                    .count(),
                2,
                "{scene}"
            );
            assert!(scene.contains("[node name=\"water_volume\" type=\"Area3D\""));
            assert!(scene.contains("[node name=\"custom_pool\" type=\"Area3D\""));
            assert_eq!(
                scene.matches("type=\"ShaderMaterial\"").count(),
                1,
                "the custom pool keeps its own material"
            );
            assert!(path.join("water/heron_water_view.gd").exists());
            // Cocos, GDevelop and Stride read the same payload with runtimes of their own.
            for (target, sub) in [
                (Target::Babylon, "babylon"),
                (Target::PlayCanvas, "playcanvas"),
                (Target::Cocos, "cocos"),
                (Target::GDevelop, "gdevelop"),
                (Target::Stride, "stride"),
                (Target::Raylib, "raylib"),
            ] {
                let path = dir.join(sub);
                std::fs::create_dir_all(&path).unwrap();
                let report = export::export(
                    &doc,
                    &path,
                    &export::Options {
                        target,
                        name: "water".into(),
                        scale: 32.,
                        project_dir: path.to_string_lossy().into_owned(),
                        ..Default::default()
                    },
                )
                .unwrap();
                assert!(
                    !report.warnings.iter().any(|w| w.starts_with("Water ")),
                    "{target:?} {:?}",
                    report.warnings
                );
                match target {
                    Target::Babylon => {
                        assert!(std::fs::read_to_string(path.join("water.json"))
                            .unwrap()
                            .contains("_heron_water"));
                        let runtime =
                            std::fs::read_to_string(path.join("heron_babylon.js")).unwrap();
                        assert!(
                            runtime.contains("export class HeronWater")
                                && runtime.contains("CUSTOM_VERTEX_UPDATE_POSITION")
                        );
                    }
                    Target::PlayCanvas => {
                        assert!(std::fs::read_to_string(path.join("water.json"))
                            .unwrap()
                            .contains("_heron_water"));
                        let runtime =
                            std::fs::read_to_string(path.join("heron_playcanvas.js")).unwrap();
                        assert!(
                            runtime.contains("export class HeronWater")
                                && runtime.contains("transformVS")
                        );
                    }
                    Target::Cocos => {
                        assert!(std::fs::read_to_string(path.join("water.json"))
                            .unwrap()
                            .contains("_heron_water"));
                        let runtime =
                            std::fs::read_to_string(path.join("HeronBrushLevel.ts")).unwrap();
                        assert!(
                            runtime.contains("export class HeronWater")
                                && runtime.contains("waterSurface")
                        );
                    }
                    Target::GDevelop => {
                        let project: serde_json::Value = serde_json::from_str(
                            &std::fs::read_to_string(path.join("game.json")).unwrap(),
                        )
                        .unwrap();
                        let layout = project["layouts"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|l| l["name"] == "water")
                            .unwrap();
                        let code = layout["events"][0]["inlineCode"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|l| l.as_str().unwrap())
                            .collect::<Vec<_>>()
                            .join("\n");
                        assert_eq!(
                            code.matches("\"water\":\"{").count(),
                            2,
                            "both enabled volumes reach the runtime"
                        );
                        assert!(code.contains("waterAt"));
                        let pool = layout["objects"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|o| o["name"] == "water_volume")
                            .unwrap();
                        assert_eq!(pool["content"]["isCastingShadow"], false);
                    }
                    Target::Raylib => {
                        let header = std::fs::read_to_string(path.join("water_water.h")).unwrap();
                        assert!(
                            header.contains("#define HB_WATER_COUNT 2\n")
                                && header.contains("HbWaterFind"),
                            "{header}"
                        );
                        assert_eq!(header.matches("\"custom_pool\"").count(), 1);
                        let preview =
                            std::fs::read_to_string(path.join("water_preview.c")).unwrap();
                        assert!(preview.contains("#define HB_WATER\n#include \"water_water.h\""));
                        assert!(std::fs::read_to_string(path.join("water.level"))
                            .unwrap()
                            .contains("sensor"));
                        // The plain-C half, as a C loader without raylib includes it.
                        std::fs::write(path.join("check.c"),"#include <assert.h>\n#include \"water_water.h\"\nint main(void){double d,v[3],z[3]={0},in[3]={3,-2,3},both[3]={0,-2,0},dry[3]={0,0.5,0};\n\
                            int w=HbWaterFind(in,&d);assert(w>=0&&fabs(d-2)<1e-9&&hb_waters[w].priority==0);\n\
                            w=HbWaterFind(both,&d);assert(w>=0&&hb_waters[w].priority==10);assert(HbWaterFind(dry,&d)<0&&d<0);\n\
                            HbWaterVelocity(w,z,z,0,0,1,v);assert(v[1]<-0.79&&v[1]>-0.81);return 0;}\n").unwrap();
                        let built = std::process::Command::new("cc")
                            .current_dir(&path)
                            .args([
                                "-std=c99", "-Wall", "-Wextra", "-Werror", "check.c", "-lm", "-o",
                                "check",
                            ])
                            .output()
                            .unwrap();
                        assert!(
                            built.status.success(),
                            "{}",
                            String::from_utf8_lossy(&built.stderr)
                        );
                        assert!(std::process::Command::new(path.join("check"))
                            .status()
                            .unwrap()
                            .success());
                    }
                    _ => {
                        let level =
                            std::fs::read_to_string(path.join("water/HeronBrushStrideLevel.cs"))
                                .unwrap();
                        assert!(
                            level.contains("_heron_water") && level.contains("level.Waters.Add")
                        );
                        let blended = std::fs::read_dir(path.join("water/Materials"))
                            .unwrap()
                            .filter_map(|f| f.ok())
                            .filter(|f| f.file_name().to_string_lossy().contains("_blend_"))
                            .collect::<Vec<_>>();
                        assert_eq!(
                            blended.len(),
                            1,
                            "the pool's see-through surface is a material of its own"
                        );
                        assert!(std::fs::read_to_string(blended[0].path())
                            .unwrap()
                            .contains("MaterialTransparencyBlendFeature"));
                    }
                }
            }
        }
        if target != Target::Unity {
            let mesh = pool["mesh"].as_str().unwrap();
            let file = if target == Target::Flax {
                path.join("water")
                    .join(std::path::Path::new(mesh).file_name().unwrap())
            } else {
                path.join(mesh)
            };
            let bytes = std::fs::read(file).unwrap();
            let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
            let glb: serde_json::Value = serde_json::from_slice(&bytes[20..20 + len]).unwrap();
            for material in glb["materials"].as_array().unwrap() {
                assert_eq!(material["alphaMode"], "BLEND");
                assert_eq!(material["doubleSided"], true);
            }
        }
    }
}

#[test]
fn quake_two_wal_uses_profile_palette_mod_override_and_exports_decoded_pixels() {
    let root = std::env::temp_dir().join(format!("heron-wal-document-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("baseq2/textures/base")).unwrap();
    std::fs::create_dir_all(root.join("mod/pics")).unwrap();
    let mut wal = vec![0; 100];
    wal[32..36].copy_from_slice(&8u32.to_le_bytes());
    wal[36..40].copy_from_slice(&8u32.to_le_bytes());
    for level in 0..4 {
        let offset = wal.len() as u32;
        wal[40 + level * 4..44 + level * 4].copy_from_slice(&offset.to_le_bytes());
        wal.extend(vec![1; (8 >> level) * (8 >> level)]);
    }
    std::fs::write(root.join("baseq2/textures/base/wall.wal"), wal).unwrap();
    let before = crate::preferences::pref(Clone::clone);
    crate::preferences::edit(|p| {
        p.game = "Quake 2".into();
        p.game_path = root.to_string_lossy().into_owned();
        p.game_mods.clear();
    });
    let game = crate::games::current().expect("bundled Quake II profile");
    assert_eq!(game.materials_palette.as_deref(), Some("pics/colormap.pcx"));
    let bundled =
        map_model::pcx::palette(&file_store::read(&game.directory.join("colormap.pcx")).unwrap())
            .unwrap();
    let mut doc = Document::new(WorldBounds::default());
    let brush = map_model::shapes::create_cuboid(
        &doc.world_bounds(),
        &Bbox3d::from_scalars(-16., 16.),
        "base/wall",
    )
    .unwrap();
    doc.add_brushes(vec![brush]);
    assert_eq!(
        &doc.materials()
            .get("base/wall")
            .expect("WAL fallback material")
            .pixels()
            .unwrap()[..3],
        &bundled[3..6]
    );
    let mut palette = vec![0; 128];
    palette[..4].copy_from_slice(&[10, 5, 1, 8]);
    palette[65] = 1;
    palette[66] = 1;
    palette.extend([1, 12]);
    let mut rgb = [0; 768];
    rgb[3..6].copy_from_slice(&[17, 39, 61]);
    palette.extend(rgb);
    std::fs::write(root.join("mod/pics/colormap.pcx"), palette).unwrap();
    crate::preferences::edit(|p| p.game_mods = "mod".into());
    doc.reload_game_assets();
    assert!(
        doc.game_asset_warnings().is_empty(),
        "{:?}",
        doc.game_asset_warnings()
    );
    assert_eq!(
        &doc.materials().get("base/wall").unwrap().pixels().unwrap()[..4],
        &[17, 39, 61, 255]
    );
    for target in [
        crate::export::Target::Glb,
        crate::export::Target::Godot,
        crate::export::Target::ThreeJs,
    ] {
        let out = root.join(format!("{target:?}"));
        std::fs::create_dir_all(&out).unwrap();
        let report = crate::export::export(
            &doc,
            &out,
            &crate::export::Options {
                target,
                name: "wal".into(),
                ..Default::default()
            },
        )
        .unwrap();
        if target == crate::export::Target::Glb {
            let model =
                map_model::glb_model::read(&std::fs::read(out.join("wal.glb")).unwrap()).unwrap();
            assert_eq!(
                &model.skins().next().unwrap().pixels[..4],
                &[17, 39, 61, 255]
            );
        } else {
            assert!(
                report
                    .files
                    .iter()
                    .filter(|path| path.extension().is_some_and(|e| e == "png"))
                    .any(
                        |path| image::open(path).unwrap().to_rgba8().get_pixel(0, 0).0
                            == [17, 39, 61, 255]
                    ),
                "{target:?} exports the resolved WAL colors"
            );
        }
    }
    crate::preferences::edit(|p| *p = before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pending_primitive_uv_survives_save_and_reopen_until_material_discovery() {
    let root = std::env::temp_dir().join(format!(
        "heron-pending-uv-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let before = crate::preferences::pref(Clone::clone);
    crate::preferences::edit(|p| {
        p.game = "heron_brush".into();
        p.game_path.clear();
        p.game_mods.clear();
    });
    let mut map = map_model::io::map_reader::read_map(
        include_str!("../../../map_model/tests/fixtures/format.map"),
        None,
    )
    .unwrap();
    map.entities.truncate(1);
    map.entities[0].brushes.truncate(1);
    map.entities[0].patches.clear();
    map.entities[0]
        .properties
        .retain(|(key, _)| key == "classname");
    for face in &mut map.entities[0].brushes[0].faces {
        face.material_name = "primitive_uv_check".into();
        face.primitive_uv = Some(Box::new([
            [0.0078125, 0.001, 0.25],
            [-0.002, 0.015625, -0.125],
        ]));
    }
    let doc = Document::from_map(&map, WorldBounds::default(), None);
    assert!(doc.materials().get("primitive_uv_check").is_none());
    let saved = doc.to_map_text();
    let reopened_map = map_model::io::map_reader::read_map(&saved, None).unwrap();
    let pending = reopened_map.entities[0].brushes[0].faces.clone();
    assert!(pending.iter().all(|f| f.primitive_uv.is_some()));
    let mut reopened = Document::from_map(&reopened_map, WorldBounds::default(), None);
    assert_eq!(reopened.to_map_text(), saved);
    reopened.select_all();
    reopened.edit_faces(|f| f.set_paint(None));
    let painted = map_model::io::map_reader::read_map(&reopened.to_map_text(), None).unwrap();
    assert!(painted.entities[0].brushes[0]
        .faces
        .iter()
        .all(|f| f.primitive_uv.is_some()));
    reopened.undo();
    reopened.edit_faces(|f| f.move_uv(geometry_math::vec::Vec2f::new(7., -3.)));
    let edited = map_model::io::map_reader::read_map(&reopened.to_map_text(), None).unwrap();
    assert!(edited.entities[0].brushes[0]
        .faces
        .iter()
        .all(|f| f.primitive_uv.is_none()));
    reopened.undo();
    reopened.edit_faces(|f| f.reset_uv_to_paraxial());
    let reset = map_model::io::map_reader::read_map(&reopened.to_map_text(), None).unwrap();
    assert!(reset.entities[0].brushes[0]
        .faces
        .iter()
        .all(|f| f.primitive_uv.is_none()));
    reopened.undo();
    assert_eq!(reopened.to_map_text(), saved);
    image::RgbaImage::from_pixel(128, 64, image::Rgba([50, 100, 150, 255]))
        .save(root.join("primitive_uv_check.png"))
        .unwrap();
    reopened.load_materials(&root).unwrap();
    let converted = map_model::io::map_reader::read_map(&reopened.to_map_text(), None).unwrap();
    for face in &converted.entities[0].brushes[0].faces {
        assert!(face.primitive_uv.is_none());
        assert!(matches!(
            face.uv_coord_system,
            map_model::uv::UvCoordSystem::Parallel { .. }
        ));
        let mut expected = pending
            .iter()
            .find(|p| p.points == face.points)
            .unwrap()
            .clone();
        expected.resolve_primitive_uv(geometry_math::vec::Vec2f::new(128., 64.));
        for point in face.points {
            geometry_math::assert_approx_eq!(
                face.uv_coords(point),
                expected.uv_coords(point),
                1e-5
            );
        }
    }
    crate::preferences::edit(|p| *p = before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn quake3_native_export_uses_texture_dimensions_omits_layers_and_preserves_existing_files_on_failure(
) {
    let root = std::env::temp_dir().join(format!(
        "heron-q3-export-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let mut document = one_box();
    let path = root.join("native.map");
    std::fs::write(&path, "existing map").unwrap();
    assert!(document
        .quake3_map_export_problems()
        .iter()
        .any(|s| s.contains("stone")));
    assert!(document.export_quake3_map_to(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "existing map");
    // An omitted layer needs no material dimensions and contributes no brushes.
    let layer = document.add_layer("Omitted");
    document.select_all();
    document.move_selection_to_layer(layer);
    document.set_layer_omitted(layer, true);
    assert!(document.quake3_map_export_problems().is_empty());
    assert!(!document.export_quake3_map_text().unwrap().contains("stone"));
    document.set_layer_omitted(layer, false);
    image::RgbaImage::from_pixel(128, 64, image::Rgba([80, 120, 160, 255]))
        .save(root.join("stone.png"))
        .unwrap();
    document.load_materials(&root).unwrap();
    document.edit_faces(|face| {
        face.rotate_uv(37.);
        face.scale_uv(geometry_math::vec::Vec2f::new(-0.5, 2.));
        face.move_uv(geometry_math::vec::Vec2f::new(7., -3.));
    });
    let source = document.to_map_text();
    document.export_quake3_map_to(&path).unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("\nbrushDef\n"));
    assert!(!written.contains("primitive_uv"));
    assert_eq!(
        document.to_map_text(),
        source,
        "export does not mutate the editable map"
    );
    let expected = map_model::io::map_reader::read_map(&source, None).unwrap();
    let mut actual = map_model::io::map_reader::read_map(&written, None).unwrap();
    for (a, b) in expected.entities.iter().zip(&mut actual.entities) {
        for (a, b) in a.brushes.iter().zip(&mut b.brushes) {
            assert_eq!(a.faces.len(), b.faces.len());
            for (a, b) in a.faces.iter().zip(&mut b.faces) {
                b.resolve_primitive_uv(geometry_math::vec::Vec2f::new(128., 64.));
                assert_eq!(a.points, b.points);
                for point in a.points {
                    geometry_math::assert_approx_eq!(a.uv_coords(point), b.uv_coords(point), 1e-4);
                }
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn doom3_native_export_preserves_world_geometry_and_rejects_unhandled_rotation_before_writing() {
    let root = std::env::temp_dir().join(format!(
        "heron-doom3-export-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("native.map");
    std::fs::write(&path, "existing map").unwrap();
    let mut doc = one_box();
    assert!(doc.export_doom3_map_to(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "existing map");
    image::RgbaImage::from_pixel(128, 64, image::Rgba([80, 120, 160, 255]))
        .save(root.join("stone.png"))
        .unwrap();
    doc.load_materials(&root).unwrap();
    doc.select_all();
    let id = doc.create_brush_entity("func_static").unwrap();
    doc.set_entity_property(id, "origin", "128 64 32");
    let before = doc.to_map_text();
    doc.export_doom3_map_to(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("Version 2\n"));
    assert!(text.contains("brushDef3"));
    assert_eq!(doc.to_map_text(), before);
    let expected = map_model::io::map_reader::read_map(&before, None).unwrap();
    let mut actual = map_model::io::map_reader::read_map(&text, None).unwrap();
    for (a, b) in expected.entities.iter().zip(&mut actual.entities) {
        for (a, b) in a.brushes.iter().zip(&mut b.brushes) {
            for (a, b) in a.faces.iter().zip(&mut b.faces) {
                geometry_math::assert_approx_eq!(a.plane.normal, b.plane.normal, 1e-9);
                assert!((a.plane.distance - b.plane.distance).abs() < 1e-8);
                b.resolve_primitive_uv(geometry_math::vec::Vec2f::new(128., 64.));
                for point in a.points {
                    geometry_math::assert_approx_eq!(a.uv_coords(point), b.uv_coords(point), 1e-4);
                }
            }
        }
    }
    doc.set_entity_property(id, "rotation", "0 -1 0 1 0 0 0 0 1");
    assert!(doc
        .export_doom3_map_to(&path)
        .unwrap_err()
        .to_string()
        .contains("rotation"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    std::fs::remove_dir_all(root).unwrap();
}
