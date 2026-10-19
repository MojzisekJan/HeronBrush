//! Fine-grid assets must keep their actual lattice corners after saving and loading.
use map_model::{
    brush::Brush,
    io::{map_reader::read_map, map_writer::write_map},
};
use geometry_math::{bbox::Bbox3d, vec};
#[test]
fn nearly_coplanar_grid_facets_survive_loading_and_saving() {
    let bounds = Bbox3d::from_scalars(-16384., 16384.);
    let map = read_map(include_str!("fixtures/fine_grid.map"), None).unwrap();
    let source = &map.entities[0].brushes[0];
    let brush = Brush::from_faces(&bounds, source.faces.clone()).unwrap();
    assert!(brush.geometry().face_count() > 10);
    for vertex in brush.geometry().vertices() {
        assert!(vec::length(vertex.position - vec::round(vertex.position * 8.) / 8.) < 1e-7);
    }
    let saved = read_map(&write_map(&map), None).unwrap();
    let again = Brush::from_faces(&bounds, saved.entities[0].brushes[0].faces.clone()).unwrap();
    assert_eq!(
        brush.geometry().vertex_count(),
        again.geometry().vertex_count()
    );
    assert_eq!(brush.geometry().face_count(), again.geometry().face_count());
    assert!((brush.volume() - again.volume()).abs() < 1e-7);
}
