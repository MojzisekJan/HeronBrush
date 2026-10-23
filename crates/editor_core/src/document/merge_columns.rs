//! Merging brushes into fewer: two neighbours become one wherever together they make a
//! convex brush, so the outside does not move. Merge columns also keeps every face's
//! look; Merge keeping shape lets the faces' looks go.
use super::*;
use crate::i18n::translation_key;

/// How far apart two numbers may be and still count as the same, for planes and volumes.
const SAME: f64 = 1e-6;

/// What a merge asks of the faces of two brushes that end up in one face.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::document) enum Faces {
    /// They must look alike, so the merge changes nothing that shows.
    Alike,
    /// Any look will do: the merged face wears the first one's.
    Any,
}

impl Document {
    /// Merges the selected brushes, two neighbours at a time, wherever the pair together
    /// is a convex brush: the corners' convex hull holds no more than the two do. The
    /// merged brush has exactly their surface, and a merge happens only where the faces
    /// lying in one plane wear the same material, alignment, paint and export mark, so
    /// nothing looks different afterwards. Flat stretches of terrain, which columns
    /// leave split into many triangles, become a few large brushes.
    ///
    /// The merged brushes stay where the first of each pair was, in its group, and the
    /// whole merge undoes in one step. Returns how many brushes there are fewer.
    pub fn merge_columns(&mut self) -> Result<usize, &'static str> {
        self.merge_selected(Faces::Alike)
    }

    /// Merges the selected brushes as [`Self::merge_columns`] does, but whatever their
    /// faces look like: where two faces become one, it takes the look of one of them. The
    /// outside keeps its exact shape, so four brushes that pair up into two convex ones
    /// become two, and a whole stretch of terrain becomes as few brushes as merging
    /// neighbours finds.
    pub fn merge_keeping_shape(&mut self) -> Result<usize, &'static str> {
        self.merge_selected(Faces::Any)
    }

    fn merge_selected(&mut self, faces: Faces) -> Result<usize, &'static str> {
        let selected: Vec<(NodeId, Brush)> = self
            .selected_brushes_with_ids()
            .into_iter()
            .filter(|(id, _)| self.is_editable(*id))
            .collect();
        if selected.len() < 2 {
            return Err(translation_key("Select brushes first"));
        }
        // Only brushes with one parent merge: the merged brush stays where the first was,
        // so one from a door merged with one from the world would make the world part of
        // the door.
        let parents = self.brush_parents();
        let mut families: HashMap<Option<NodeId>, Vec<(NodeId, Brush)>> = HashMap::new();
        for (id, brush) in selected {
            let parent = parents.get(&id).copied().flatten();
            families.entry(parent).or_default().push((id, brush));
        }
        let mut replacements: HashMap<NodeId, Vec<Brush>> = HashMap::new();
        let mut removed = 0;
        for family in families.into_values() {
            let ids: Vec<NodeId> = family.iter().map(|(id, _)| *id).collect();
            let mut brushes: Vec<Option<Brush>> =
                family.into_iter().map(|(_, brush)| Some(brush)).collect();
            for (i, absorbed) in merge_neighbours(&mut brushes, faces) {
                let Some(brush) = brushes[i].take() else {
                    continue;
                };
                replacements.insert(ids[i], vec![brush]);
                for j in absorbed {
                    replacements.insert(ids[j], Vec::new());
                    removed += 1;
                }
            }
        }
        if replacements.is_empty() {
            return Ok(0);
        }
        self.run_as_one_edit(|document| document.replace_geometry_in_place(replacements));
        Ok(removed)
    }

    /// Each brush's parent: the group or entity it is directly in, or `None` for one
    /// lying in a layer.
    fn brush_parents(&self) -> HashMap<NodeId, Option<NodeId>> {
        fn walk(
            children: &[Node],
            parent: Option<NodeId>,
            out: &mut HashMap<NodeId, Option<NodeId>>,
        ) {
            for node in children {
                match &node.kind {
                    NodeKind::Brush(_) => {
                        out.insert(node.id, parent);
                    }
                    NodeKind::Group(group) => walk(&group.children, Some(node.id), out),
                    NodeKind::Entity(entity) => walk(&entity.children, Some(node.id), out),
                    NodeKind::Layer(layer) => walk(&layer.children, Some(node.id), out),
                    _ => {}
                }
            }
        }
        let mut out = HashMap::new();
        for layer in &self.tree.world.layers {
            walk(&layer.children, None, &mut out);
        }
        out
    }
}

/// Merges neighbours among `brushes` wherever [`merged`] allows, until none can be: a
/// merged brush takes the first one's place and the other's becomes `None`. Returns,
/// for each brush that took others in, which ones.
pub(in crate::document) fn merge_neighbours(
    brushes: &mut [Option<Brush>],
    faces: Faces,
) -> HashMap<usize, Vec<usize>> {
    // Each brush in the cells its bounds cover, to find its neighbours quickly. A
    // merged brush is added to the cells it grew into; one that is gone is skipped. The
    // cells are as big as a middling brush: sized by the smallest, a sliver would make
    // every big brush cover thousands of them.
    let mut sizes: Vec<f64> = brushes
        .iter()
        .flatten()
        .map(|brush| {
            let size = brush.bounds().size();
            size.x().max(size.y()).max(size.z())
        })
        .collect();
    sizes.sort_by(f64::total_cmp);
    let cell = sizes.get(sizes.len() / 2).copied().unwrap_or(1.0).max(1.0);
    let cells_of = |bounds: &Bbox3d| {
        let lo = [0, 1, 2].map(|i| ((bounds.min[i] - SAME) / cell).floor() as i64);
        let hi = [0, 1, 2].map(|i| ((bounds.max[i] + SAME) / cell).floor() as i64);
        let mut cells = Vec::new();
        for x in lo[0]..=hi[0] {
            for y in lo[1]..=hi[1] {
                for z in lo[2]..=hi[2] {
                    cells.push([x, y, z]);
                }
            }
        }
        cells
    };
    let mut grid: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
    for (i, brush) in brushes.iter().enumerate() {
        let Some(brush) = brush else {
            continue;
        };
        for c in cells_of(&brush.bounds()) {
            grid.entry(c).or_default().push(i);
        }
    }

    let mut merged_into: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut queue: std::collections::VecDeque<usize> = (0..brushes.len()).collect();
    while let Some(i) = queue.pop_front() {
        let Some(brush) = brushes[i].clone() else {
            continue;
        };
        let bounds = brush.bounds().expand(SAME * 10.0);
        let mut near: Vec<usize> = cells_of(&bounds)
            .iter()
            .filter_map(|c| grid.get(c))
            .flatten()
            .copied()
            .filter(|&j| j != i)
            .collect();
        near.sort_unstable();
        near.dedup();
        let found = near.into_iter().find_map(|j| {
            let other = brushes[j].as_ref()?;
            if !other.bounds().intersects(&bounds) {
                return None;
            }
            Some((j, merged(&brush, other, faces)?))
        });
        let Some((j, joined)) = found else {
            continue;
        };
        for c in cells_of(&joined.bounds()) {
            grid.entry(c).or_default().push(i);
        }
        brushes[i] = Some(joined);
        brushes[j] = None;
        let absorbed = merged_into.remove(&j).unwrap_or_default();
        merged_into
            .entry(i)
            .or_default()
            .extend(absorbed.into_iter().chain([j]));
        // It may take in more neighbours now that it is bigger.
        queue.push_back(i);
    }
    merged_into
}

/// `a` and `b` as one brush, when they share a face, together are convex, and, when
/// `faces` asks for it, every face of the result lies on faces of theirs that look
/// alike; `None` otherwise.
fn merged(a: &Brush, b: &Brush, faces: Faces) -> Option<Brush> {
    // Touching across a face: anything less leaves a gap or a notch the hull would fill.
    let touching = a.faces().iter().any(|fa| {
        b.faces()
            .iter()
            .any(|fb| geometry_math::plane::is_equal(&fa.plane, &fb.plane.flip(), SAME))
    });
    if !touching {
        return None;
    }
    // A quick test before the hull: the two are convex together only if each one's
    // corners lie behind every plane of the other but the ones they meet across.
    let behind = |corners: &[Vec3d], brush: &Brush, other: &Brush| {
        brush.faces().iter().all(|face| {
            let shared = other
                .faces()
                .iter()
                .any(|f| geometry_math::plane::is_equal(&face.plane, &f.plane.flip(), SAME));
            shared
                || corners
                    .iter()
                    .all(|p| vec::dot(face.plane.normal, *p) <= face.plane.distance + SAME * 10.0)
        })
    };
    if !behind(&b.vertex_positions(), a, b) || !behind(&a.vertex_positions(), b, a) {
        return None;
    }
    // Corners a split or a dab left a hair off a fine grid go back onto it, so the
    // merged brush is not written with noise like 1e-13 in its planes.
    let tidy = |v: f64| {
        let r = (v * 1024.0).round() / 1024.0;
        if (v - r).abs() < SAME {
            r
        } else {
            v
        }
    };
    let points: Vec<Vec3d> = a
        .vertex_positions()
        .into_iter()
        .chain(b.vertex_positions())
        .map(|p| Vec3d::new(tidy(p.x()), tidy(p.y()), tidy(p.z())))
        .collect();
    let hull = map_model::polyhedron::Polyhedron::from_points(&points, f64::ALMOST_ZERO).ok()?;
    let sources: Vec<&map_model::brush_face::BrushFace> =
        a.faces().iter().chain(b.faces()).collect();
    let joined = Brush::from_polyhedron(hull, &sources[0].material_name);
    let (va, vb) = (a.volume(), b.volume());
    if (joined.volume() - (va + vb)).abs() > SAME * (va + vb).max(1.0) {
        return None;
    }
    // Each face of the result wears the look of the faces of the two it stands on. For
    // Merge columns they have to agree, or the merge would change a material, an
    // alignment or what is exported.
    let mut worn_faces = Vec::with_capacity(joined.faces().len());
    for face in joined.faces() {
        let mut on = sources
            .iter()
            .filter(|f| geometry_math::plane::is_equal(&f.plane, &face.plane, SAME));
        let first = on.next()?;
        if faces == Faces::Alike && !on.all(|f| looks_alike(first, f)) {
            return None;
        }
        let mut worn = (*first).clone();
        worn.points = face.points;
        worn.plane = face.plane;
        worn_faces.push(worn);
    }
    joined.with_exact_face_attributes(&worn_faces)
}

/// Whether two faces in one plane look the same: everything but where their plane's
/// points were written.
fn looks_alike(a: &map_model::brush_face::BrushFace, b: &map_model::brush_face::BrushFace) -> bool {
    let mut b = b.clone();
    b.points = a.points;
    b.plane = a.plane;
    b == *a
}

#[cfg(test)]
#[path = "merge_keeping_shape_tests.rs"]
mod keeping_shape_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sculpt::ColumnShape;
    use crate::sculpt::SculptParameters;

    /// A 128 × 128 box, 32 tall, selected.
    fn slab() -> Document {
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(128.0, 128.0, 32.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        d.select_all();
        d
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

    /// Whether `p` is inside one of `d`'s brushes.
    fn solid_at(d: &Document, p: Vec3d) -> bool {
        brushes(d).iter().any(|brush| {
            brush
                .faces()
                .iter()
                .all(|face| vec::dot(face.plane.normal, p) <= face.plane.distance + 1e-6)
        })
    }

    #[test]
    fn flat_columns_merge_back_into_one_brush() {
        for shape in ColumnShape::ALL {
            let mut d = slab();
            let made = d.split_into_columns(16.0, shape).unwrap();
            let removed = d.merge_columns().unwrap();
            assert_eq!(removed, made - 1, "{shape:?}");
            // The box it was, to the last digit.
            let left = brushes(&d);
            assert_eq!(left.len(), 1);
            assert_eq!(left[0].faces().len(), 6, "{shape:?}");
            assert_eq!(left[0].bounds().min, Vec3d::new(0.0, 0.0, 0.0), "{shape:?}");
            assert_eq!(
                left[0].bounds().max,
                Vec3d::new(128.0, 128.0, 32.0),
                "{shape:?}"
            );
            // The faces inside are gone, and the outside keeps its marks: none hidden.
            assert!(left[0].faces().iter().all(|face| !face.export_hidden));
        }
    }

    #[test]
    fn a_sculpted_surface_keeps_its_shape_and_the_flat_rest_merges() {
        let mut d = slab();
        d.split_into_columns(16.0, ColumnShape::Triangles).unwrap();
        let params = SculptParameters {
            radius: 40.0,
            strength: 1.0,
            ..Default::default()
        };
        d.clear_selection();
        let mut stroke = d.begin_sculpt_stroke();
        assert!(d.sculpt_dab(Vec3d::new(64.0, 64.0, 32.0), &params, &mut stroke));
        let before = brushes(&d).len();
        let volume = total_volume(&d);
        // Points over a grid, a little off it so none sits on an edge, sample the shape.
        let probes: Vec<Vec3d> = (0..32)
            .flat_map(|i| (0..32).map(move |j| (i, j)))
            .flat_map(|(i, j)| {
                (0..12).map(move |k| {
                    Vec3d::new(
                        2.1 + 4.0 * i as f64,
                        1.9 + 4.0 * j as f64,
                        0.7 + 4.0 * k as f64,
                    )
                })
            })
            .collect();
        let inside: Vec<bool> = probes.iter().map(|p| solid_at(&d, *p)).collect();

        d.select_all();
        let removed = d.merge_columns().unwrap();
        assert!(
            removed > 0 && removed < before - 1,
            "some merge, the bump keeps its own"
        );
        assert!((total_volume(&d) - volume).abs() < 1e-6);
        let after: Vec<bool> = probes.iter().map(|p| solid_at(&d, *p)).collect();
        assert_eq!(inside, after, "the surface did not move");
    }

    #[test]
    fn faces_that_look_different_keep_their_brushes_apart() {
        let mut d = Document::new(Default::default());
        for (x, material) in [(0.0, "rock"), (64.0, "grass")] {
            let bounds = Bbox3d::new(Vec3d::new(x, 0.0, 0.0), Vec3d::new(x + 64.0, 64.0, 32.0));
            d.add_brushes(vec![map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &bounds,
                material,
            )
            .unwrap()]);
        }
        d.select_all();
        assert_eq!(d.merge_columns(), Ok(0));
        assert_eq!(brushes(&d).len(), 2);
    }

    #[test]
    fn merging_keeping_shape_lets_the_looks_go() {
        let mut d = Document::new(Default::default());
        for (x, material) in [(0.0, "rock"), (64.0, "grass")] {
            let bounds = Bbox3d::new(Vec3d::new(x, 0.0, 0.0), Vec3d::new(x + 64.0, 64.0, 32.0));
            d.add_brushes(vec![map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &bounds,
                material,
            )
            .unwrap()]);
        }
        d.select_all();
        assert_eq!(d.merge_keeping_shape(), Ok(1));
        assert_eq!(brushes(&d).len(), 1);
        assert!((total_volume(&d) - 2.0 * 64.0 * 64.0 * 32.0).abs() < 1e-6);
    }

    /// Four boxes in an L, each with its own material: they pair up into two convex
    /// brushes, and the L is never filled in.
    #[test]
    fn four_brushes_in_an_l_become_two() {
        let mut d = Document::new(Default::default());
        let boxes = [
            ([0.0, 0.0], [32.0, 32.0], "a"),
            ([32.0, 0.0], [64.0, 32.0], "b"),
            ([0.0, 32.0], [32.0, 64.0], "c"),
            ([0.0, 64.0], [32.0, 96.0], "d"),
        ];
        for (min, max, material) in boxes {
            let bounds = Bbox3d::new(
                Vec3d::new(min[0], min[1], 0.0),
                Vec3d::new(max[0], max[1], 32.0),
            );
            d.add_brushes(vec![map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &bounds,
                material,
            )
            .unwrap()]);
        }
        let volume = total_volume(&d);
        let probes: Vec<Vec3d> = (0..16)
            .flat_map(|x| {
                (0..16).map(move |y| Vec3d::new(x as f64 * 6.0 + 1.0, y as f64 * 6.0 + 1.0, 16.0))
            })
            .collect();
        let inside: Vec<bool> = probes.iter().map(|p| solid_at(&d, *p)).collect();
        d.select_all();
        assert_eq!(d.merge_keeping_shape(), Ok(2));
        assert_eq!(brushes(&d).len(), 2);
        assert!((total_volume(&d) - volume).abs() < 1e-6);
        let after: Vec<bool> = probes.iter().map(|p| solid_at(&d, *p)).collect();
        assert_eq!(inside, after, "the outside did not move");
    }

    /// A brush of a door and one of the world side by side stay apart, so the world
    /// does not become part of the door.
    #[test]
    fn brushes_with_different_parents_stay_apart() {
        let mut d = Document::new(Default::default());
        for x in [0.0, 64.0] {
            let bounds = Bbox3d::new(Vec3d::new(x, 0.0, 0.0), Vec3d::new(x + 64.0, 64.0, 32.0));
            d.add_brushes(vec![map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &bounds,
                map_model::shapes::DEFAULT_MATERIAL,
            )
            .unwrap()]);
        }
        let first = node::brushes_with_ids(d.tree_world())[0].0;
        d.select_only(first);
        assert!(d.create_brush_entity("func_door").is_some());
        d.select_all();
        assert_eq!(d.merge_keeping_shape(), Ok(0));
        assert_eq!(brushes(&d).len(), 2);
    }

    #[test]
    fn an_l_shape_is_not_merged_into_its_hull() {
        let mut d = Document::new(Default::default());
        for (min, max) in [([0.0, 0.0], [64.0, 32.0]), ([0.0, 32.0], [32.0, 64.0])] {
            let bounds = Bbox3d::new(
                Vec3d::new(min[0], min[1], 0.0),
                Vec3d::new(max[0], max[1], 32.0),
            );
            d.add_brushes(vec![map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &bounds,
                map_model::shapes::DEFAULT_MATERIAL,
            )
            .unwrap()]);
        }
        d.select_all();
        assert_eq!(d.merge_columns(), Ok(0));
    }

    /// How long merging takes on terrain the size of the column limit:
    /// `cargo test --release -p editor_core --lib merge_columns::tests::speed -- --ignored --nocapture`.
    #[test]
    #[ignore = "a measurement; prints, never fails"]
    fn speed() {
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(1024.0, 1024.0, 32.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        d.select_all();
        let made = d.split_into_columns(16.0, ColumnShape::Triangles).unwrap();
        let start = std::time::Instant::now();
        let removed = d.merge_columns().unwrap();
        println!(
            "merged {made} columns into {}: {:?}",
            made - removed,
            start.elapsed()
        );
    }

    #[test]
    fn a_merge_is_one_undo_step_and_needs_two_brushes() {
        let mut d = slab();
        assert!(d.merge_columns().is_err());
        d.split_into_columns(16.0, ColumnShape::Squares).unwrap();
        let split = brushes(&d).len();
        d.merge_columns().unwrap();
        assert!(d.undo().is_some());
        assert_eq!(brushes(&d).len(), split);
    }
}
