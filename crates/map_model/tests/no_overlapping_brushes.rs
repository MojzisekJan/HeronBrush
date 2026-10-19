//! The prefabs that ship do not have brushes sharing space.
//!
//! Two brushes inside each other is not an error a map can report -- geometry is allowed
//! to overlap, and sometimes should -- but in a piece meant to be dropped into someone's
//! level it is a mess: the faces between them are drawn and lit and exported and never
//! seen, and moving one of the pair leaves a hole in the other.
//!
//! `assets/monkey.map` is here because the first way it was built had 4734 such pairs.
//! It is one brush per face of a mesh, and what the construction promises is that two
//! faces sharing an **edge** give two brushes that share a face and do not overlap. Two
//! that share only a **corner** can still meet in a sliver near it, and a few hundred
//! do. The number is here so a change that makes it worse is noticed.

use geometry_math::bbox::Bbox3d;

/// Reads a `.map` and returns every pair of brushes whose insides meet.
///
/// A pair is only reported when they truly share volume: touching faces, which is how
/// brushwork is normally built, is not overlapping.
fn overlapping_pairs(source: &str) -> usize {
    let map = map_model::io::map_reader::read_map(source, None).expect("the map reads");
    let world = Bbox3d::from_scalars(-16384.0, 16384.0);
    let brushes: Vec<map_model::brush::Brush> = map
        .entities
        .iter()
        .flat_map(|entity| entity.brushes.iter())
        .filter_map(|brush| map_model::brush::Brush::from_faces(&world, brush.faces.clone()).ok())
        .collect();

    let mut found = 0;
    for (at, a) in brushes.iter().enumerate() {
        for b in &brushes[at + 1..] {
            // The bounding boxes first: most pairs of a model's brushes are nowhere near
            // each other, and this is otherwise every brush against every other.
            if !a.bounds().intersects(&b.bounds()) {
                continue;
            }
            if shares_volume(a, b) {
                found += 1;
            }
        }
    }
    found
}

/// Whether two convex solids have any volume in common.
///
/// A separating plane is enough to say they do not, and for two convex solids one of
/// their own face planes is always such a plane if any is. A face lying flat against a
/// face of the other counts as separated, which is what makes ordinary brushwork pass.
fn shares_volume(a: &map_model::brush::Brush, b: &map_model::brush::Brush) -> bool {
    !separated_by_a_face_of(a, b) && !separated_by_a_face_of(b, a)
}

fn separated_by_a_face_of(a: &map_model::brush::Brush, b: &map_model::brush::Brush) -> bool {
    let corners: Vec<_> = b.geometry().vertices().iter().map(|v| v.position).collect();
    // No corner of `b` strictly inside this face's half-space. Sitting on the plane
    // counts as separated, which is what makes two boxes that share a face pass -- that
    // is how brushwork is built, not an overlap.
    a.faces().iter().any(|face| {
        corners
            .iter()
            .all(|corner| face.plane.point_distance(*corner) > -1e-6)
    })
}

#[test]
fn the_monkey_is_no_worse_than_it_was() {
    let source = std::fs::read_to_string("../../assets/monkey.map").expect("the monkey ships");
    let found = overlapping_pairs(&source);
    // 3009 as this counts them. It was 2149 until the asset was snapped to an eighth of a
    // unit (28e4b51, 2026-09-12): rounding the corners grew more slivers where faces meet
    // at a corner. Before the snap every pair was one sharing a single corner and none
    // shared an edge; that was not checked again after it. `scene_export`'s own checker
    // counts fewer -- the two differ over how nearly two faces have to lie together to
    // count as touching rather than crossing. The first construction had 4734.
    assert!(found <= 3050, "{found} pairs share space, which is more than it had");
}

/// The check can see an overlap when there is one.
///
/// Without this, a separating-plane test with its sign the wrong way round answers "no
/// overlap" to everything and the check above passes for the wrong reason.
#[test]
fn two_boxes_inside_each_other_are_found() {
    let box_at = |lo: f64, hi: f64| {
        format!(
            "{{\n\
             ( {lo} {lo} {lo} ) ( {lo} {hi} {lo} ) ( {lo} {lo} {hi} ) a 0 0 0 1 1\n\
             ( {lo} {lo} {lo} ) ( {lo} {lo} {hi} ) ( {hi} {lo} {lo} ) a 0 0 0 1 1\n\
             ( {lo} {lo} {lo} ) ( {hi} {lo} {lo} ) ( {lo} {hi} {lo} ) a 0 0 0 1 1\n\
             ( {hi} {hi} {hi} ) ( {hi} {lo} {hi} ) ( {lo} {hi} {hi} ) a 0 0 0 1 1\n\
             ( {hi} {hi} {hi} ) ( {lo} {hi} {hi} ) ( {hi} {hi} {lo} ) a 0 0 0 1 1\n\
             ( {hi} {hi} {hi} ) ( {hi} {hi} {lo} ) ( {hi} {lo} {hi} ) a 0 0 0 1 1\n\
             }}\n"
        )
    };
    let map = |a: String, b: String| format!("{{\n\"classname\" \"worldspawn\"\n{a}{b}}}\n");

    // Half inside each other.
    assert_eq!(overlapping_pairs(&map(box_at(0.0, 16.0), box_at(8.0, 24.0))), 1);
    // Sharing a face, which is how brushwork is built.
    assert_eq!(overlapping_pairs(&map(box_at(0.0, 16.0), box_at(16.0, 32.0))), 0);
    // Nowhere near each other.
    assert_eq!(overlapping_pairs(&map(box_at(0.0, 16.0), box_at(64.0, 80.0))), 0);
}
