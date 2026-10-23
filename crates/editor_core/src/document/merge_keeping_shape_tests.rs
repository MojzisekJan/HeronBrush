//! Tests of Merge keeping shape: known shapes with the count they should come to, the
//! rules about what may merge, and seeded random layouts checked point by point.
//!
//! Every test that merges checks the same three things through `merge_and_check`: the
//! total volume stays, every probe point is inside or outside as it was, and every brush
//! left is a valid one. Equal volume with an unchanged union means no two brushes left
//! overlap.
use super::*;
use crate::sculpt::ColumnShape;
use crate::sculpt::SculptMode;
use crate::sculpt::SculptParameters;

/// A box from `min` to `max` wearing `material`.
fn cuboid(d: &Document, min: [f64; 3], max: [f64; 3], material: &str) -> Brush {
    map_model::shapes::create_cuboid(
        &d.world_bounds(),
        &Bbox3d::new(
            Vec3d::new(min[0], min[1], min[2]),
            Vec3d::new(max[0], max[1], max[2]),
        ),
        material,
    )
    .expect("a box")
}

/// The convex brush round `points`.
fn hull(points: &[[f64; 3]], material: &str) -> Brush {
    let points: Vec<Vec3d> = points
        .iter()
        .map(|p| Vec3d::new(p[0], p[1], p[2]))
        .collect();
    let geometry =
        map_model::polyhedron::Polyhedron::from_points(&points, 1e-7).expect("a solid hull");
    Brush::from_polyhedron(geometry, material)
}

/// A document holding `brushes`, all selected.
fn document_with(make: impl FnOnce(&Document) -> Vec<Brush>) -> Document {
    let mut d = Document::new(Default::default());
    let brushes = make(&d);
    d.add_brushes(brushes);
    d.select_all();
    d
}

/// Unit cells of `size` at the given grid positions, each its own box.
fn cells(d: &Document, size: f64, at: &[[i32; 3]]) -> Vec<Brush> {
    at.iter()
        .enumerate()
        .map(|(i, c)| {
            let min = c.map(|v| f64::from(v) * size);
            cuboid(d, min, min.map(|v| v + size), &format!("m{}", i % 3))
        })
        .collect()
}

fn brushes(d: &Document) -> Vec<Brush> {
    node::brushes_by_layer(d.tree_world())
        .into_iter()
        .map(|(_, brush, _)| brush.clone())
        .collect()
}

fn total_volume(d: &Document) -> f64 {
    brushes(d).iter().map(Brush::volume).sum()
}

/// Whether `p` is inside one of `brushes`.
fn solid_at(brushes: &[Brush], p: Vec3d) -> bool {
    brushes.iter().any(|brush| {
        brush.bounds().expand(1e-6).contains_point(p)
            && brush
                .faces()
                .iter()
                .all(|face| vec::dot(face.plane.normal, p) <= face.plane.distance + 1e-6)
    })
}

/// Points through the bounds of `d`'s brushes and a little round them, `n` along each
/// axis, set off the round numbers so none lies on a face.
fn probes(d: &Document, n: usize) -> Vec<Vec3d> {
    let all = brushes(d);
    let mut min = Vec3d::new(f64::MAX, f64::MAX, f64::MAX);
    let mut max = Vec3d::new(f64::MIN, f64::MIN, f64::MIN);
    for brush in &all {
        let b = brush.bounds();
        for i in 0..3 {
            min[i] = min[i].min(b.min[i]);
            max[i] = max[i].max(b.max[i]);
        }
    }
    let step = [0, 1, 2].map(|i| (max[i] - min[i] + 8.0) / n as f64);
    let mut out = Vec::with_capacity(n * n * n);
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                out.push(Vec3d::new(
                    min[0] - 4.0 + step[0] * (i as f64 + 0.37),
                    min[1] - 4.0 + step[1] * (j as f64 + 0.61),
                    min[2] - 4.0 + step[2] * (k as f64 + 0.53),
                ));
            }
        }
    }
    out
}

/// Runs Merge keeping shape on the selection and checks that the shape held. Returns how
/// many brushes are left.
fn merge_and_check(d: &mut Document) -> usize {
    let before_count = brushes(d).len();
    let volume = total_volume(d);
    let points = probes(d, 24);
    let old = brushes(d);
    let inside: Vec<bool> = points.iter().map(|p| solid_at(&old, *p)).collect();

    let removed = d.merge_keeping_shape().expect("a selection to merge");

    let after = brushes(d);
    assert_eq!(after.len(), before_count - removed, "the count it reports");
    assert!(
        (total_volume(d) - volume).abs() < 1e-6 * volume.max(1.0),
        "the volume held: {} against {volume}",
        total_volume(d)
    );
    let now: Vec<bool> = points.iter().map(|p| solid_at(&after, *p)).collect();
    let moved: Vec<&Vec3d> = points
        .iter()
        .zip(inside.iter().zip(&now))
        .filter(|(_, (a, b))| a != b)
        .map(|(p, _)| p)
        .collect();
    assert!(moved.is_empty(), "the outside moved at {moved:?}");
    for brush in &after {
        assert!(brush.volume() > 0.0, "a brush without volume");
        assert!(brush.faces().len() >= 4, "a brush with too few faces");
    }
    after.len()
}

// Known shapes.

#[test]
fn a_row_of_boxes_becomes_one() {
    for n in [2, 3, 7] {
        let at: Vec<[i32; 3]> = (0..n).map(|x| [x, 0, 0]).collect();
        let mut d = document_with(|d| cells(d, 16.0, &at));
        assert_eq!(merge_and_check(&mut d), 1, "a row of {n}");
    }
}

#[test]
fn a_column_of_boxes_becomes_one() {
    let mut d = document_with(|d| cells(d, 16.0, &[[0, 0, 0], [0, 0, 1], [0, 0, 2]]));
    assert_eq!(merge_and_check(&mut d), 1);
}

#[test]
fn a_full_block_of_boxes_becomes_one() {
    let mut at = Vec::new();
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..2 {
                at.push([x, y, z]);
            }
        }
    }
    let mut d = document_with(|d| cells(d, 16.0, &at));
    assert_eq!(merge_and_check(&mut d), 1);
}

#[test]
fn an_l_becomes_two_and_is_never_filled_in() {
    let mut d = document_with(|d| cells(d, 16.0, &[[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 2, 0]]));
    assert_eq!(merge_and_check(&mut d), 2);
}

#[test]
fn a_t_becomes_two() {
    let mut d = document_with(|d| cells(d, 16.0, &[[0, 1, 0], [1, 1, 0], [2, 1, 0], [1, 0, 0]]));
    assert_eq!(merge_and_check(&mut d), 2);
}

#[test]
fn a_plus_becomes_three() {
    let at = [[1, 0, 0], [0, 1, 0], [1, 1, 0], [2, 1, 0], [1, 2, 0]];
    let mut d = document_with(|d| cells(d, 16.0, &at));
    assert_eq!(merge_and_check(&mut d), 3);
}

/// Eight boxes round an empty middle can be four bars; fewer would cover the hole.
#[test]
fn a_ring_round_a_hole_becomes_four() {
    let mut at = Vec::new();
    for x in 0..3 {
        for y in 0..3 {
            if (x, y) != (1, 1) {
                at.push([x, y, 0]);
            }
        }
    }
    let mut d = document_with(|d| cells(d, 16.0, &at));
    assert_eq!(merge_and_check(&mut d), 4);
}

/// Columns rising one step at a time: any two side by side would have to fill the step.
#[test]
fn steps_of_different_heights_stay_apart() {
    let mut d = document_with(|d| {
        (0..4)
            .map(|i| {
                let x = f64::from(i) * 16.0;
                cuboid(
                    d,
                    [x, 0.0, 0.0],
                    [x + 16.0, 16.0, 16.0 * f64::from(i + 1)],
                    "m",
                )
            })
            .collect()
    });
    assert_eq!(merge_and_check(&mut d), 4);
}

#[test]
fn boxes_touching_only_along_an_edge_stay_apart() {
    let mut d = document_with(|d| cells(d, 16.0, &[[0, 0, 0], [1, 1, 0]]));
    assert_eq!(merge_and_check(&mut d), 2);
}

#[test]
fn boxes_touching_only_at_a_corner_stay_apart() {
    let mut d = document_with(|d| cells(d, 16.0, &[[0, 0, 0], [1, 1, 1]]));
    assert_eq!(merge_and_check(&mut d), 2);
}

#[test]
fn boxes_with_a_gap_stay_apart() {
    let mut d = document_with(|d| {
        vec![
            cuboid(d, [0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "m"),
            cuboid(d, [17.0, 0.0, 0.0], [33.0, 16.0, 16.0], "m"),
        ]
    });
    assert_eq!(merge_and_check(&mut d), 2);
}

/// Overlapping boxes together hold less than their two volumes, so merging them would
/// not keep the shape's volume: they stay.
#[test]
fn overlapping_boxes_stay_apart() {
    let mut d = document_with(|d| {
        vec![
            cuboid(d, [0.0, 0.0, 0.0], [32.0, 16.0, 16.0], "m"),
            cuboid(d, [16.0, 0.0, 0.0], [48.0, 16.0, 16.0], "m"),
        ]
    });
    assert_eq!(merge_and_check(&mut d), 2);
}

/// A box beside a smaller one meeting it over part of a face: the hull would add a slope.
#[test]
fn a_box_against_part_of_a_bigger_face_stays_apart() {
    let mut d = document_with(|d| {
        vec![
            cuboid(d, [0.0, 0.0, 0.0], [32.0, 32.0, 32.0], "m"),
            cuboid(d, [32.0, 8.0, 0.0], [48.0, 24.0, 32.0], "m"),
        ]
    });
    assert_eq!(merge_and_check(&mut d), 2);
}

/// Two wedges cut along a box's diagonal are the box again.
#[test]
fn two_wedges_make_their_box() {
    let mut d = document_with(|_| {
        vec![
            hull(
                &[
                    [0.0, 0.0, 0.0],
                    [32.0, 0.0, 0.0],
                    [0.0, 32.0, 0.0],
                    [0.0, 0.0, 16.0],
                    [32.0, 0.0, 16.0],
                    [0.0, 32.0, 16.0],
                ],
                "a",
            ),
            hull(
                &[
                    [32.0, 0.0, 0.0],
                    [32.0, 32.0, 0.0],
                    [0.0, 32.0, 0.0],
                    [32.0, 0.0, 16.0],
                    [32.0, 32.0, 16.0],
                    [0.0, 32.0, 16.0],
                ],
                "b",
            ),
        ]
    });
    assert_eq!(merge_and_check(&mut d), 1);
}

/// A ramp against a box whose top it continues makes one convex prism.
#[test]
fn a_ramp_continuing_a_slope_merges() {
    let mut d = document_with(|_| {
        vec![
            // A wedge rising from 0 to 16 over x 0..32, and one rising from 16 to 32 over
            // x 32..64 on a block 16 tall: one slope from 0 to 32 over x 0..64.
            hull(
                &[
                    [0.0, 0.0, 0.0],
                    [32.0, 0.0, 0.0],
                    [32.0, 0.0, 16.0],
                    [0.0, 16.0, 0.0],
                    [32.0, 16.0, 0.0],
                    [32.0, 16.0, 16.0],
                ],
                "a",
            ),
            hull(
                &[
                    [32.0, 0.0, 0.0],
                    [64.0, 0.0, 0.0],
                    [64.0, 0.0, 32.0],
                    [32.0, 0.0, 16.0],
                    [32.0, 16.0, 0.0],
                    [64.0, 16.0, 0.0],
                    [64.0, 16.0, 32.0],
                    [32.0, 16.0, 16.0],
                ],
                "b",
            ),
        ]
    });
    assert_eq!(merge_and_check(&mut d), 1);
}

/// A ramp up onto a flat top bends over a ridge, which is convex, so it merges; a ramp
/// down into a valley and up again would need the valley filled, so it does not.
#[test]
fn a_ridge_merges_and_a_valley_does_not() {
    let mut d = document_with(|_| {
        vec![
            hull(
                &[
                    [0.0, 0.0, 0.0],
                    [32.0, 0.0, 0.0],
                    [32.0, 0.0, 16.0],
                    [0.0, 16.0, 0.0],
                    [32.0, 16.0, 0.0],
                    [32.0, 16.0, 16.0],
                ],
                "a",
            ),
            // Flat on top at 16, so the ramp ends in a ridge.
            hull(
                &[
                    [32.0, 0.0, 0.0],
                    [64.0, 0.0, 0.0],
                    [64.0, 0.0, 16.0],
                    [32.0, 0.0, 16.0],
                    [32.0, 16.0, 0.0],
                    [64.0, 16.0, 0.0],
                    [64.0, 16.0, 16.0],
                    [32.0, 16.0, 16.0],
                ],
                "b",
            ),
        ]
    });
    assert_eq!(merge_and_check(&mut d), 1);

    let mut valley = document_with(|_| {
        vec![
            // Falling from 16 to 0 over x 0..32, then rising from 0 to 16 over x 32..64.
            hull(
                &[
                    [0.0, 0.0, 0.0],
                    [32.0, 0.0, 0.0],
                    [0.0, 0.0, 16.0],
                    [0.0, 16.0, 0.0],
                    [32.0, 16.0, 0.0],
                    [0.0, 16.0, 16.0],
                ],
                "a",
            ),
            hull(
                &[
                    [32.0, 0.0, 0.0],
                    [64.0, 0.0, 0.0],
                    [64.0, 0.0, 16.0],
                    [32.0, 16.0, 0.0],
                    [64.0, 16.0, 0.0],
                    [64.0, 16.0, 16.0],
                ],
                "b",
            ),
        ]
    });
    assert_eq!(merge_and_check(&mut valley), 2);
}

/// Boxes turned off the axes merge as they would on them.
#[test]
fn turned_boxes_merge() {
    for degrees in [17.0_f64, 30.0, 45.0] {
        let (sin, cos) = degrees.to_radians().sin_cos();
        let mut d = document_with(|d| {
            cells(d, 16.0, &[[0, 0, 0], [1, 0, 0], [2, 0, 0]])
                .into_iter()
                .map(|brush| {
                    brush
                        .map_vertices(|p| {
                            Vec3d::new(p.x() * cos - p.y() * sin, p.x() * sin + p.y() * cos, p.z())
                        })
                        .expect("a turned box")
                })
                .collect()
        });
        assert_eq!(merge_and_check(&mut d), 1, "turned {degrees}°");
    }
}

// What may merge.

#[test]
fn different_materials_merge_and_one_of_them_is_kept() {
    let mut d = document_with(|d| {
        vec![
            cuboid(d, [0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "rock"),
            cuboid(d, [16.0, 0.0, 0.0], [32.0, 16.0, 16.0], "grass"),
        ]
    });
    assert_eq!(merge_and_check(&mut d), 1);
    let merged = &brushes(&d)[0];
    for face in merged.faces() {
        assert!(
            face.material_name == "rock" || face.material_name == "grass",
            "a face wears one of the two materials, not {}",
            face.material_name
        );
    }
}

/// Merge columns refuses where the looks differ; Merge keeping shape does not.
#[test]
fn merge_columns_still_keeps_looks_apart() {
    let mut d = document_with(|d| {
        vec![
            cuboid(d, [0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "rock"),
            cuboid(d, [16.0, 0.0, 0.0], [32.0, 16.0, 16.0], "grass"),
        ]
    });
    assert_eq!(d.merge_columns(), Ok(0));
    assert_eq!(d.merge_keeping_shape(), Ok(1));
}

#[test]
fn only_selected_brushes_merge() {
    let mut d = document_with(|d| cells(d, 16.0, &[[0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0]]));
    let ids: Vec<NodeId> = node::brushes_with_ids(d.tree_world())
        .iter()
        .map(|(id, _)| *id)
        .collect();
    d.clear_selection();
    d.toggle_selected(ids[0]);
    d.toggle_selected(ids[1]);
    assert_eq!(merge_and_check(&mut d), 3, "two merge, two are left alone");
    let untouched: Vec<NodeId> = node::brushes_with_ids(d.tree_world())
        .iter()
        .map(|(id, _)| *id)
        .collect();
    assert!(untouched.contains(&ids[2]) && untouched.contains(&ids[3]));
}

#[test]
fn brushes_in_different_groups_stay_apart_and_each_group_merges() {
    let mut d = document_with(|d| cells(d, 16.0, &[[0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0]]));
    let ids: Vec<NodeId> = node::brushes_with_ids(d.tree_world())
        .iter()
        .map(|(id, _)| *id)
        .collect();
    for (pair, name) in [(&ids[0..2], "left"), (&ids[2..4], "right")] {
        d.clear_selection();
        for id in pair {
            d.toggle_selected(*id);
        }
        assert!(d.group_selection(name).is_some());
    }
    let groups: Vec<NodeId> = node::groups_with_ids(d.tree_world())
        .iter()
        .map(|(id, _)| *id)
        .collect();
    d.select_all();
    assert_eq!(merge_and_check(&mut d), 2, "one brush per group");
    for group in groups {
        assert_eq!(
            node::contents_of(d.tree_world(), group)
                .iter()
                .filter(|id| d.brush(**id).is_some())
                .count(),
            1,
            "each group keeps one brush"
        );
    }
}

#[test]
fn fewer_than_two_brushes_is_refused() {
    let mut d = document_with(|d| cells(d, 16.0, &[[0, 0, 0]]));
    assert!(d.merge_keeping_shape().is_err());
    let mut empty = Document::new(Default::default());
    assert!(empty.merge_keeping_shape().is_err());
}

#[test]
fn a_merge_undoes_in_one_step() {
    let mut d = document_with(|d| cells(d, 16.0, &[[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 2, 0]]));
    let before = brushes(&d);
    merge_and_check(&mut d);
    assert!(d.undo().is_some());
    let mut again = brushes(&d);
    let mut before = before;
    let key = |b: &Brush| {
        let c = b.bounds().center();
        (c.x() as i64, c.y() as i64, c.z() as i64)
    };
    before.sort_by_key(key);
    again.sort_by_key(key);
    assert_eq!(again.len(), before.len());
    for (a, b) in before.iter().zip(&again) {
        assert_eq!(a.bounds(), b.bounds());
        assert!((a.volume() - b.volume()).abs() < 1e-9);
    }
}

#[test]
fn a_second_merge_finds_nothing_more() {
    let mut d = document_with(|d| {
        cells(
            d,
            16.0,
            &[[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 2, 0], [2, 2, 0]],
        )
    });
    merge_and_check(&mut d);
    d.select_all();
    assert_eq!(d.merge_keeping_shape(), Ok(0));
}

// Terrain.

/// A 128 × 128 slab, 32 tall, split into `shape` columns a grid step of 16 across.
fn terrain(shape: ColumnShape) -> Document {
    let mut d = document_with(|d| vec![cuboid(d, [0.0, 0.0, 0.0], [128.0, 128.0, 32.0], "grass")]);
    d.split_into_columns(16.0, shape).expect("columns");
    d
}

#[test]
fn flat_terrain_of_every_shape_becomes_one_brush() {
    for shape in ColumnShape::ALL {
        let mut d = terrain(shape);
        d.select_all();
        assert_eq!(merge_and_check(&mut d), 1, "{shape:?}");
    }
}

/// Raised and lowered here and there, terrain of every shape keeps its surface and ends
/// with far fewer brushes than columns.
#[test]
fn sculpted_terrain_of_every_shape_keeps_its_surface() {
    for shape in ColumnShape::ALL {
        let mut d = terrain(shape);
        let columns = brushes(&d).len();
        d.clear_selection();
        let mut stroke = d.begin_sculpt_stroke();
        for (x, y, mode) in [
            (40.0, 40.0, SculptMode::Raise),
            (96.0, 80.0, SculptMode::Lower),
            (70.0, 20.0, SculptMode::Raise),
        ] {
            let parameters = SculptParameters {
                radius: 30.0,
                strength: 0.8,
                mode,
                ..Default::default()
            };
            d.sculpt_dab(Vec3d::new(x, y, 32.0), &parameters, &mut stroke);
        }
        d.select_all();
        let left = merge_and_check(&mut d);
        assert!(
            left < columns / 2,
            "{shape:?}: {left} of {columns} columns left"
        );
    }
}

// Random layouts.

/// A small deterministic random number source, so a failing seed can be run again.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

/// Random cells of a 6 × 6 × 3 grid filled with boxes: whatever merges, the shape holds
/// and no brush is added.
#[test]
fn random_boxes_keep_their_shape() {
    for seed in 0..24 {
        let mut random = Lcg(seed);
        let mut at = Vec::new();
        for x in 0..6 {
            for y in 0..6 {
                for z in 0..3 {
                    if random.chance(55) {
                        at.push([x, y, z]);
                    }
                }
            }
        }
        if at.len() < 2 {
            continue;
        }
        let mut d = document_with(|d| cells(d, 16.0, &at));
        let left = merge_and_check(&mut d);
        assert!(left <= at.len(), "seed {seed}");
    }
}

/// Random cells, each a box or a box cut along one of its diagonals into two wedges, so
/// merges have to work across slanted faces too.
#[test]
fn random_boxes_and_wedges_keep_their_shape() {
    for seed in 0..24 {
        let mut random = Lcg(seed + 1000);
        let mut make: Vec<([f64; 3], u64)> = Vec::new();
        for x in 0..5 {
            for y in 0..5 {
                if random.chance(70) {
                    let height = 16.0 * (1 + random.next() % 2) as f64;
                    make.push((
                        [f64::from(x) * 16.0, f64::from(y) * 16.0, height],
                        random.next() % 3,
                    ));
                }
            }
        }
        if make.len() < 2 {
            continue;
        }
        let mut d = document_with(|d| {
            let mut out = Vec::new();
            for ([x, y, h], cut) in &make {
                let (x1, y1) = (x + 16.0, y + 16.0);
                let corners = |points: &[[f64; 2]]| -> Vec<[f64; 3]> {
                    points
                        .iter()
                        .flat_map(|p| [[p[0], p[1], 0.0], [p[0], p[1], *h]])
                        .collect()
                };
                match cut {
                    0 => out.push(cuboid(d, [*x, *y, 0.0], [x1, y1, *h], "m")),
                    1 => {
                        out.push(hull(&corners(&[[*x, *y], [x1, *y], [*x, y1]]), "a"));
                        out.push(hull(&corners(&[[x1, *y], [x1, y1], [*x, y1]]), "b"));
                    }
                    _ => {
                        out.push(hull(&corners(&[[*x, *y], [x1, *y], [x1, y1]]), "a"));
                        out.push(hull(&corners(&[[*x, *y], [x1, y1], [*x, y1]]), "b"));
                    }
                }
            }
            out
        });
        let pieces = brushes(&d).len();
        let left = merge_and_check(&mut d);
        assert!(left <= pieces, "seed {seed}");
        // Every cut cell has two wedges that make its box again, so at least those merge.
        let cut = make.iter().filter(|(_, cut)| *cut != 0).count();
        assert!(
            left <= pieces - cut,
            "seed {seed}: {left} left of {pieces}, {cut} cut cells"
        );
    }
}

/// The fewest boxes `cells` of a grid can be cut into, a box being a full rectangle of
/// cells: found by trying every box at the first cell left, with what is left
/// remembered. Cell (x, y) is bit `y * 8 + x`.
fn fewest_boxes(cells: u64, memo: &mut HashMap<u64, usize>) -> usize {
    if cells == 0 {
        return 0;
    }
    if let Some(known) = memo.get(&cells) {
        return *known;
    }
    let first = cells.trailing_zeros();
    let (x, y) = (first % 8, first / 8);
    let has = |x: u32, y: u32| x < 8 && y < 8 && cells & (1 << (y * 8 + x)) != 0;
    let mut best = usize::MAX;
    let mut width = 0;
    while has(x + width, y) {
        width += 1;
    }
    for w in 1..=width {
        let mut box_cells = 0u64;
        let mut h = 0;
        while (0..w).all(|i| has(x + i, y + h)) {
            for i in 0..w {
                box_cells |= 1 << ((y + h) * 8 + x + i);
            }
            h += 1;
            best = best.min(1 + fewest_boxes(cells & !box_cells, memo));
        }
    }
    memo.insert(cells, best);
    best
}

/// On random flat shapes of grid cells, the merge comes to the fewest boxes possible
/// nearly always: in 120 shapes, at most 4 % come to more, none by more than one, and
/// all together no more than 1 % more. A single greedy pass, as merging was at first,
/// fails this: some shapes came to two or more boxes too many.
#[test]
fn random_flat_shapes_come_close_to_the_fewest() {
    let (mut ours, mut fewest, mut worse) = (0, 0, 0);
    let mut shapes = 0;
    for seed in 0..120 {
        let mut random = Lcg(seed + 5000);
        let side = 4 + random.next() % 3;
        let fill = [50, 70, 85][(random.next() % 3) as usize];
        let mut at = Vec::new();
        let mut bits = 0u64;
        for x in 0..side as i32 {
            for y in 0..side as i32 {
                if random.chance(fill) {
                    at.push([x, y, 0]);
                    bits |= 1 << (y * 8 + x);
                }
            }
        }
        if at.len() < 2 {
            continue;
        }
        shapes += 1;
        let best = fewest_boxes(bits, &mut HashMap::new());
        let mut d = document_with(|d| cells(d, 16.0, &at));
        let left = merge_and_check(&mut d);
        assert!(left >= best, "seed {seed}: fewer than possible?");
        assert!(
            left <= best + 1,
            "seed {seed}: {left} where {best} would do"
        );
        ours += left;
        fewest += best;
        worse += usize::from(left > best);
    }
    assert!(
        worse * 25 <= shapes,
        "{worse} of {shapes} shapes came to more than the fewest"
    );
    assert!(
        ours * 100 <= fewest * 101,
        "{ours} brushes where {fewest} would do"
    );
}
