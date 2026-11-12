//! Select loop: every brush joined to a brush through overlaps or Join corner joints.

use super::{node, Bbox3d, Brush, Document, HashMap, HashSet, NodeId};

/// The side of the cells brushes are filed under for finding their neighbours.
const CELL: f64 = 64.0;

impl Document {
    /// Every editable brush connected to `start`, `start` included: two brushes are
    /// connected when they share volume, as walls do before Join corner, or when a face
    /// of one lies on a face of the other over some area and one of the two is a joint's
    /// face: skip, which Join corner gives the faces it hides, or slanted, as a mitre is.
    /// Touching along an edge or at a corner does not connect, and neither does a wall
    /// standing on a floor. Followed both ways round a room, and down a T's stem.
    pub fn loop_of(&self, start: NodeId) -> Vec<NodeId> {
        let brushes: Vec<(NodeId, &Brush)> = node::brushes_with_ids(self.tree_world())
            .into_iter()
            .filter(|(id, _)| self.is_editable(*id))
            .collect();
        let Some(first) = brushes.iter().position(|(id, _)| *id == start) else {
            return Vec::new();
        };
        let bounds: Vec<Bbox3d> = brushes.iter().map(|(_, b)| b.bounds()).collect();
        // Cells of `CELL` units, each listing the brushes whose bounds reach into it, so
        // a brush is only compared with the few near it.
        let cells_of = |b: &Bbox3d| {
            let low = [0, 1, 2].map(|a| (b.min[a] / CELL).floor() as i64);
            let high = [0, 1, 2].map(|a| (b.max[a] / CELL).floor() as i64);
            let mut out = Vec::new();
            for x in low[0]..=high[0] {
                for y in low[1]..=high[1] {
                    for z in low[2]..=high[2] {
                        out.push([x, y, z]);
                    }
                }
            }
            out
        };
        let mut grid: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
        for (index, b) in bounds.iter().enumerate() {
            for cell in cells_of(&b.expand(1e-3)) {
                grid.entry(cell).or_default().push(index);
            }
        }
        let mut seen: HashSet<usize> = HashSet::from([first]);
        let mut queue = vec![first];
        let mut found = vec![start];
        while let Some(current) = queue.pop() {
            let near: HashSet<usize> = cells_of(&bounds[current].expand(1e-3))
                .iter()
                .filter_map(|cell| grid.get(cell))
                .flatten()
                .copied()
                .collect();
            let mut near: Vec<usize> = near.into_iter().collect();
            near.sort_unstable();
            for other in near {
                if seen.contains(&other)
                    || !bounds[current].expand(1e-3).intersects(&bounds[other])
                    || !connected(brushes[current].1, brushes[other].1)
                {
                    continue;
                }
                seen.insert(other);
                queue.push(other);
                found.push(brushes[other].0);
            }
        }
        found
    }

    /// Selects [`Document::loop_of`] every selected brush, or of `start` when given.
    /// Returns how many brushes are selected.
    pub fn select_loop(&mut self, start: Option<NodeId>) -> usize {
        let starts: Vec<NodeId> = match start {
            Some(id) => vec![id],
            None => self
                .selection
                .iter()
                .copied()
                .filter(|id| self.brush(*id).is_some())
                .collect(),
        };
        let mut all: Vec<NodeId> = Vec::new();
        for id in starts {
            if all.contains(&id) {
                continue;
            }
            all.extend(self.loop_of(id));
        }
        all.sort();
        all.dedup();
        if all.is_empty() || self.select_nodes(&all).is_err() {
            return 0;
        }
        self.selection.len()
    }
}

/// Whether two brushes whose bounds meet are joined: they share volume, or a joint's
/// face of one lies on a face of the other over some area.
fn connected(a: &Brush, b: &Brush) -> bool {
    if crate::corner_join::shared_volume(a, b) > 1e-6 {
        return true;
    }
    let joint = |face: &map_model::brush_face::BrushFace| {
        let n = face.plane.normal;
        face.material_name.as_str() == "skip"
            || (0..3).filter(|axis| n[*axis].abs() > 1e-6).count() > 1
    };
    for (i, fa) in a.faces().iter().enumerate() {
        for (j, fb) in b.faces().iter().enumerate() {
            if geometry_math::vec::dot(fa.plane.normal, fb.plane.normal) > -1.0 + 1e-6
                || (fa.plane.distance + fb.plane.distance).abs() > 1e-4
                || !(joint(fa) || joint(fb))
            {
                continue;
            }
            let (Some(pa), Some(pb)) = (a.face_polygon(i), b.face_polygon(j)) else {
                continue;
            };
            if crate::corner_join::overlap_area(&pa, &pb, fa.plane.normal) > 1e-3 {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::vec::Vec3d;

    fn owner_map(name: &str) -> Document {
        let path = format!(
            "{}/../heron-brush/tests/data/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        Document::open(path, Default::default()).unwrap()
    }

    fn ids(d: &Document) -> Vec<NodeId> {
        node::brushes_with_ids(d.tree_world())
            .into_iter()
            .map(|(id, _)| id)
            .collect()
    }

    fn boxes(list: &[([f64; 3], [f64; 3])]) -> Document {
        let mut d = Document::new(Default::default());
        for (min, max) in list {
            let brush = map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &Bbox3d {
                    min: Vec3d::new(min[0], min[1], min[2]),
                    max: Vec3d::new(max[0], max[1], max[2]),
                },
                "stone",
            )
            .unwrap();
            d.add_brushes(vec![brush]);
        }
        d
    }

    fn join_all(d: &mut Document) {
        d.select_all();
        let draft = d.preview_corner_join().unwrap();
        d.join_corner(draft.revision).unwrap();
    }

    #[test]
    fn the_owners_rooms_select_whole_before_and_after_joining() {
        for name in ["four_walls.map", "ten_brushes.map"] {
            let mut d = owner_map(name);
            for joined in [false, true] {
                if joined {
                    join_all(&mut d);
                }
                let all = ids(&d);
                for start in &all {
                    let mut found = d.loop_of(*start);
                    found.sort();
                    let mut want = all.clone();
                    want.sort();
                    assert_eq!(found, want, "{name}, joined {joined}");
                }
                d.clear_selection();
                assert_eq!(d.select_loop(Some(all[2])), all.len());
            }
        }
        // As first drawn the north-west wall only touches the west wall, square on:
        // still one loop, the long way round.
        let d = owner_map("ten_brushes_first_drawn.map");
        assert_eq!(d.loop_of(ids(&d)[0]).len(), 10);
    }

    #[test]
    fn the_right_click_menu_offers_select_loop_on_a_brush() {
        let mut d = owner_map("four_walls.map");
        let all = ids(&d);
        let probe = crate::view2d::Probe {
            entity: None,
            material: None,
            brush: Some(all[1]),
            at: Vec3d::zero(),
        };
        let menu = crate::context_menu::build(&d, &probe);
        let action = menu
            .iter()
            .flatten()
            .find_map(|item| match item {
                crate::context_menu::Item::Action(
                    label,
                    action @ crate::context_menu::Action::SelectLoop(_),
                ) => {
                    assert_eq!(label, crate::i18n::tr("Select loop"));
                    Some(action.clone())
                }
                _ => None,
            })
            .expect("Select loop in the menu");
        crate::context_menu::run(&action, &mut d, 16.0);
        assert_eq!(d.selection_count(), 4);
        // Over empty space there is no brush to start from.
        let nothing = crate::view2d::Probe {
            entity: None,
            material: None,
            brush: None,
            at: Vec3d::zero(),
        };
        assert!(!crate::context_menu::build(&d, &nothing)
            .iter()
            .flatten()
            .any(|item| matches!(
                item,
                crate::context_menu::Item::Action(_, crate::context_menu::Action::SelectLoop(_))
            )));
    }

    #[test]
    fn a_tee_takes_its_stem_and_a_wall_ends_where_it_meets_nothing() {
        let mut d = boxes(&[
            ([0.0, 0.0, 0.0], [128.0, 16.0, 64.0]),
            ([56.0, 8.0, 0.0], [72.0, 96.0, 64.0]),
            ([300.0, 0.0, 0.0], [316.0, 128.0, 64.0]),
        ]);
        let all = ids(&d);
        assert_eq!(d.loop_of(all[0]).len(), 2);
        assert_eq!(d.loop_of(all[2]), vec![all[2]], "a brush joined to nothing");
        join_all(&mut d);
        let all = ids(&d);
        let mut found = d.loop_of(all[1]);
        found.sort();
        assert_eq!(found, vec![all[0], all[1]], "the stem's cut end on the bar");
    }

    #[test]
    fn a_room_among_thousands_of_brushes_is_found_by_its_neighbours_only() {
        let mut list = Vec::new();
        for x in 0..60 {
            for y in 0..60 {
                let (x, y) = (x as f64 * 24.0 + 400.0, y as f64 * 24.0);
                list.push(([x, y, 0.0], [x + 16.0, y + 16.0, 64.0]));
            }
        }
        list.extend([
            ([-144.0, -144.0, 0.0], [-128.0, 144.0, 128.0]),
            ([128.0, -144.0, 0.0], [144.0, 144.0, 128.0]),
            ([-144.0, -144.0, 0.0], [144.0, -128.0, 128.0]),
            ([-144.0, 128.0, 0.0], [144.0, 144.0, 128.0]),
        ]);
        let d = boxes(&list);
        let all = ids(&d);
        let started = std::time::Instant::now();
        assert_eq!(d.loop_of(all[all.len() - 1]).len(), 4);
        assert_eq!(d.loop_of(all[0]).len(), 1);
        // Two cells' worth of neighbours each, not every pair of 3604 brushes.
        assert!(started.elapsed().as_secs() < 5, "{:?}", started.elapsed());
    }

    #[test]
    fn touching_at_an_edge_or_standing_on_a_floor_does_not_connect() {
        let d = boxes(&[
            // Two walls meeting only along an upright edge.
            ([0.0, 0.0, 0.0], [128.0, 16.0, 64.0]),
            ([128.0, 16.0, 0.0], [144.0, 128.0, 64.0]),
            // A floor the first wall stands on.
            ([-64.0, -64.0, -16.0], [256.0, 256.0, 0.0]),
            // Touching another wall's corner only.
            ([-16.0, -16.0, 64.0], [0.0, 0.0, 128.0]),
        ]);
        let all = ids(&d);
        assert_eq!(d.loop_of(all[0]), vec![all[0]]);
    }
}
