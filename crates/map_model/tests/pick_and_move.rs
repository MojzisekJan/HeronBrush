//! Tests for picking brushes with a ray and moving them.

use geometry_math::bbox::Bbox3d;
use geometry_math::ray::Ray3d;
use geometry_math::vec::Vec3d;
use map_model::brush::Brush;
use map_model::io::map_reader::read_map;
use map_model::node::{self, NodeKind, Tree};
use map_model::pick::{pick, pick_nearest};

fn world_bounds() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

/// A box brush spanning the given bounds, written in the Standard format.
///
/// Each face is given by three points on its plane, wound clockwise as seen from outside
/// — the usual `.map` convention.
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

/// A cube centred on the origin, spanning `min` to `max` on every axis.
fn cube(min: i32, max: i32, material: &str) -> String {
    boxed((min, min, min), (max, max, max), material)
}

fn tree_with(brushes: &str) -> Tree {
    let source = format!("{{\n\"classname\" \"worldspawn\"\n{brushes}}}\n");
    let map = read_map(&source, None).expect("the sample parses");
    node::build_tree(&map, &world_bounds())
}

#[test]
fn a_ray_through_a_brush_hits_it() {
    let tree = tree_with(&cube(-16, 16, "stone"));

    // Straight down the Z axis from above.
    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, 100.0), Vec3d::new(0.0, 0.0, -1.0));
    let hit = pick_nearest(&tree.world, &ray).expect("the ray goes through the cube");

    // It enters through the top face, 84 units down.
    assert!((hit.distance - 84.0).abs() < 1e-6, "{}", hit.distance);
    geometry_math::assert_approx_eq!(hit.point, Vec3d::new(0.0, 0.0, 16.0), 1e-6);
}

#[test]
fn a_ray_that_misses_hits_nothing() {
    let tree = tree_with(&cube(-16, 16, "stone"));

    let ray = Ray3d::new(Vec3d::new(100.0, 100.0, 100.0), Vec3d::new(0.0, 0.0, -1.0));
    assert_eq!(pick_nearest(&tree.world, &ray), None);
}

#[test]
fn a_ray_pointing_away_hits_nothing() {
    let tree = tree_with(&cube(-16, 16, "stone"));

    // Above the cube, pointing further up.
    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, 100.0), Vec3d::new(0.0, 0.0, 1.0));
    assert_eq!(pick_nearest(&tree.world, &ray), None);
}

#[test]
fn hits_come_back_nearest_first() {
    // Two boxes stacked directly above one another, with a gap between them.
    let tree = tree_with(&format!(
        "{}{}",
        cube(-16, 16, "lower"),
        boxed((-16, -16, 48), (16, 16, 80), "upper")
    ));

    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    let hits = pick(&tree.world, &ray);
    assert_eq!(hits.len(), 2);

    // The upper cube is nearer to a ray coming from above.
    assert!(hits[0].distance < hits[1].distance);
    geometry_math::assert_approx_eq!(hits[0].point, Vec3d::new(0.0, 0.0, 80.0), 1e-6);
    geometry_math::assert_approx_eq!(hits[1].point, Vec3d::new(0.0, 0.0, 16.0), 1e-6);
}

#[test]
fn each_hit_names_the_node_it_came_from() {
    let tree = tree_with(&format!(
        "{}{}",
        cube(-16, 16, "lower"),
        boxed((-16, -16, 48), (16, 16, 80), "upper")
    ));
    let ids: Vec<_> = node::brushes_with_ids(&tree.world)
        .into_iter()
        .map(|(id, _)| id)
        .collect();

    let ray = Ray3d::new(Vec3d::new(0.0, 0.0, 200.0), Vec3d::new(0.0, 0.0, -1.0));
    let hits = pick(&tree.world, &ray);

    // Every hit names one of the brushes in the tree, and the two hits are different ones.
    assert!(ids.contains(&hits[0].node));
    assert!(ids.contains(&hits[1].node));
    assert_ne!(hits[0].node, hits[1].node);
}

#[test]
fn a_ray_from_the_side_enters_through_the_side_face() {
    let tree = tree_with(&cube(-16, 16, "stone"));

    let ray = Ray3d::new(Vec3d::new(-100.0, 0.0, 0.0), Vec3d::new(1.0, 0.0, 0.0));
    let hit = pick_nearest(&tree.world, &ray).expect("the ray goes through the cube");
    geometry_math::assert_approx_eq!(hit.point, Vec3d::new(-16.0, 0.0, 0.0), 1e-6);
}

/* ========== moving a brush ========== */

fn only_brush(tree: &Tree) -> &Brush {
    node::brushes(&tree.world)[0]
}

#[test]
fn translating_a_brush_moves_its_bounds() {
    let tree = tree_with(&cube(-16, 16, "stone"));
    let brush = only_brush(&tree);

    let moved = brush
        .translate(&world_bounds(), Vec3d::new(32.0, 0.0, 64.0))
        .expect("a translated box is still a box");

    geometry_math::assert_approx_eq!(moved.bounds().min, Vec3d::new(16.0, -16.0, 48.0), 1e-6);
    geometry_math::assert_approx_eq!(moved.bounds().max, Vec3d::new(48.0, 16.0, 80.0), 1e-6);
    assert_eq!(moved.geometry().face_count(), 6);
    moved.geometry().check_invariant().expect("a valid mesh");
}

#[test]
fn translating_a_brush_and_back_returns_it_to_where_it_was() {
    let tree = tree_with(&cube(-16, 16, "stone"));
    let brush = only_brush(&tree);
    let delta = Vec3d::new(128.0, -64.0, 32.0);

    let round_tripped = brush
        .translate(&world_bounds(), delta)
        .and_then(|moved| moved.translate(&world_bounds(), -delta))
        .expect("moving a box twice keeps it a box");

    geometry_math::assert_approx_eq!(round_tripped.bounds().min, brush.bounds().min, 1e-9);
    geometry_math::assert_approx_eq!(round_tripped.bounds().max, brush.bounds().max, 1e-9);
}

#[test]
fn translating_a_brush_keeps_its_materials() {
    let tree = tree_with(&cube(-16, 16, "stone"));
    let moved = only_brush(&tree)
        .translate(&world_bounds(), Vec3d::new(10.0, 0.0, 0.0))
        .expect("a translated box is still a box");

    for (face, _) in moved.geometry_faces() {
        assert_eq!(face.material_name, "stone");
    }
}

#[test]
fn a_moved_brush_can_be_picked_where_it_now_is() {
    let mut tree = tree_with(&cube(-16, 16, "stone"));
    let (id, brush) = node::brushes_with_ids(&tree.world)[0];

    let moved = brush
        .translate(&world_bounds(), Vec3d::new(0.0, 0.0, 100.0))
        .expect("a translated box is still a box");
    assert!(node::replace_brush(&mut tree.world, id, moved));

    // Nothing where it was.
    let old_spot = Ray3d::new(Vec3d::new(0.0, 0.0, -100.0), Vec3d::new(0.0, 0.0, 1.0));
    let hit = pick_nearest(&tree.world, &old_spot).expect("the ray still reaches it");
    geometry_math::assert_approx_eq!(hit.point, Vec3d::new(0.0, 0.0, 84.0), 1e-6);

    // And it answers to the same node id.
    assert_eq!(hit.node, id);
}

#[test]
fn replacing_a_brush_that_is_not_there_reports_it() {
    use map_model::node::NodeId;

    let mut tree = tree_with(&cube(-16, 16, "stone"));
    let brush = only_brush(&tree).clone();
    assert!(!node::replace_brush(
        &mut tree.world,
        NodeId(999_999),
        brush
    ));
}

#[test]
fn node_ids_are_distinct_across_the_tree() {
    let source = format!(
        "{{\n\"classname\" \"worldspawn\"\n{}{}}}\n\
         {{\n\"classname\" \"light\"\n}}\n",
        cube(-16, 16, "a"),
        boxed((-16, -16, 48), (16, 16, 80), "b")
    );
    let map = read_map(&source, None).expect("the sample parses");
    let tree = node::build_tree(&map, &world_bounds());

    let mut ids: Vec<_> = tree.world.layers[0]
        .children
        .iter()
        .map(|node| node.id)
        .collect();
    let count = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), count, "node ids must be distinct");

    // Two brushes and one entity.
    let brushes = tree.world.layers[0]
        .children
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Brush(_)))
        .count();
    assert_eq!(brushes, 2);
}
