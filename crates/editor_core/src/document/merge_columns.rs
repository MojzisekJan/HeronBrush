//! Merging brushes into fewer: two neighbours become one wherever together they make a
//! convex brush, so the outside does not move. Merge columns also keeps every face's
//! look; Join brushes lets the faces' looks go.
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
    pub fn join_brushes(&mut self) -> Result<usize, &'static str> {
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
            let mut best = merge_family(&family, faces);
            // Join brushes also tries cutting off the base the brushes stand on: columns
            // reaching down to one floor merge there into one block, and what stands on
            // it merges on its own. Whichever leaves fewer brushes wins.
            if faces == Faces::Any {
                let brushes: Vec<Brush> = family.iter().map(|(_, brush)| brush.clone()).collect();
                if let Some(cut) = cut_off_base(&brushes) {
                    let parts: Vec<(NodeId, Brush)> = cut
                        .into_iter()
                        .map(|(i, brush)| (family[i].0, brush))
                        .collect();
                    let other = merge_family(&parts, faces);
                    if other.left < best.left {
                        best = other;
                    }
                }
            }
            removed += family.len() - best.left;
            replacements.extend(best.replacements);
        }
        if replacements.is_empty() {
            return Ok(0);
        }
        self.run_as_one_edit(|document| document.replace_geometry_in_place(replacements));
        Ok(removed)
    }

    /// Each brush's parent: the group or entity it is directly in, or `None` for one
    /// lying in a layer.
    pub(super) fn brush_parents(&self) -> HashMap<NodeId, Option<NodeId>> {
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

/// What merging one family of brushes comes to: what each original that changed becomes,
/// and how many brushes are left of the family.
struct Outcome {
    replacements: HashMap<NodeId, Vec<Brush>>,
    left: usize,
}

/// Merges `parts`, each named by the original it came from; an original may have been cut
/// into several parts. An original changes when it was cut or a part of it merged, and
/// then becomes the merged brushes that kept the place of one of its parts.
fn merge_family(parts: &[(NodeId, Brush)], faces: Faces) -> Outcome {
    let mut brushes: Vec<Option<Brush>> =
        parts.iter().map(|(_, brush)| Some(brush.clone())).collect();
    let merged = merge_neighbours(&mut brushes, faces, Effort::Thorough);
    let mut count: HashMap<NodeId, usize> = HashMap::new();
    for (id, _) in parts {
        *count.entry(*id).or_default() += 1;
    }
    let mut replacements: HashMap<NodeId, Vec<Brush>> = count
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(id, _)| (id, Vec::new()))
        .collect();
    for (i, absorbed) in &merged {
        for m in std::iter::once(i).chain(absorbed) {
            replacements.entry(parts[*m].0).or_default();
        }
    }
    for (i, brush) in brushes.iter().enumerate() {
        if let (Some(brush), Some(list)) = (brush, replacements.get_mut(&parts[i].0)) {
            list.push(brush.clone());
        }
    }
    Outcome {
        replacements,
        left: brushes.iter().flatten().count(),
    }
}

/// `brushes` with the base they stand on cut off, each part named by the brush it came
/// from; `None` when there is no such base. The floor is the outermost plane along one
/// of the six axis directions that two or more brushes have a face in, the one most
/// brushes stand on. The cut lies as high above it as it can while every brush standing
/// there keeps upright sides below it: the lowest corner of any face that is not a side
/// or the floor. Below the cut each such brush is then a straight prism, and the prisms
/// side by side merge into one block, which columns reaching all the way down never do
/// under a hill whose foot curves in. A brush that ends at the cut is left whole; the
/// faces the cut makes lie against each other and are left out of exports.
fn cut_off_base(brushes: &[Brush]) -> Option<Vec<(usize, Brush)>> {
    let along = |b: &Brush, n: Vec3d| -> (f64, f64) {
        b.vertex_positions()
            .iter()
            .map(|p| vec::dot(n, *p))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), h| {
                (lo.min(h), hi.max(h))
            })
    };
    let floor_plane = |n: Vec3d, floor: f64| Plane3d::new(-floor, -n);
    let stands_on = |b: &Brush, n: Vec3d, floor: f64| {
        let plane = floor_plane(n, floor);
        b.faces()
            .iter()
            .any(|face| geometry_math::plane::is_equal(&face.plane, &plane, SAME))
    };
    let mut best: Option<(usize, Vec3d, f64)> = None;
    for axis in 0..3 {
        for sign in [1.0, -1.0] {
            let mut n = Vec3d::zero();
            n[axis] = sign;
            let floor = brushes
                .iter()
                .map(|b| along(b, n).0)
                .fold(f64::INFINITY, f64::min);
            let on = brushes.iter().filter(|b| stands_on(b, n, floor)).count();
            if on >= 2 && best.is_none_or(|(most, _, _)| on > most) {
                best = Some((on, n, floor));
            }
        }
    }
    let (_, n, floor) = best?;
    let plane = floor_plane(n, floor);
    let level = brushes
        .iter()
        .filter(|b| stands_on(b, n, floor))
        .flat_map(|b| {
            b.geometry_faces()
                .filter(|(face, _)| {
                    vec::dot(face.plane.normal, n).abs() > SAME
                        && !geometry_math::plane::is_equal(&face.plane, &plane, SAME)
                })
                .flat_map(|(_, corners)| corners.into_iter().map(|p| vec::dot(n, p)))
                .collect::<Vec<f64>>()
        })
        .fold(f64::INFINITY, f64::min);
    // A base thinner than a unit is not worth a cut, and makes slivers.
    if !level.is_finite() || level - floor < 1.0 {
        return None;
    }
    let hidden_on = |brush: Brush, plane: &Plane3d| -> Option<Brush> {
        let mut faces = brush.faces().to_vec();
        for face in &mut faces {
            if geometry_math::plane::is_equal(&face.plane, plane, SAME) {
                face.export_hidden = true;
            }
        }
        brush.with_exact_face_attributes(&faces)
    };
    let mut out = Vec::with_capacity(brushes.len() * 2);
    for (i, brush) in brushes.iter().enumerate() {
        if !stands_on(brush, n, floor) || along(brush, n).1 <= level + SAME {
            out.push((i, brush.clone()));
            continue;
        }
        let (below, above) = (Plane3d::new(level, n), Plane3d::new(-level, -n));
        let (
            map_model::brush::ClipOutcome::Clipped(base),
            map_model::brush::ClipOutcome::Clipped(top),
        ) = (brush.clip(&below), brush.clip(&above))
        else {
            return None;
        };
        if base.volume() < 1e-3 || top.volume() < 1e-3 {
            return None;
        }
        out.push((i, hidden_on(base, &below)?));
        out.push((i, hidden_on(top, &above)?));
    }
    Some(out)
}

/// How hard [`merge_neighbours`] looks for the fewest brushes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::document) enum Effort {
    /// One pass: what a sculpt dab needs, many times a second.
    Quick,
    /// Six passes in five orders, keeping the one that leaves the fewest: what the merge
    /// commands use.
    Thorough,
}

/// The orders the thorough passes take the brushes in besides the one they came in, by
/// where their corners' average lies: along x then y, along y then x, and the same
/// backwards along x. Merging is greedy, so the order decides which neighbours pair up
/// first; on random shapes of boxes one order left 6 % more brushes than the fewest
/// possible and the best of these four 1.7 %. The order they came in stays one of the
/// passes: split columns come row by row, which merges flat terrain back into one.
const ORDERS: [fn(Vec3d) -> [f64; 3]; 4] = [
    |c| [c.x(), c.y(), c.z()],
    |c| [c.y(), c.x(), c.z()],
    |c| [-c.x(), c.y(), c.z()],
    |c| [c.y(), -c.x(), c.z()],
];

/// How many brushes make the merge passes worth a thread each.
#[cfg(not(target_arch = "wasm32"))]
const PARALLEL_FROM: usize = 256;

/// Merges neighbours among `brushes` wherever [`merged`] allows, until none can be: a
/// merged brush takes the place of the first of its brushes and the others become
/// `None`. Returns, for each brush that took others in, which ones.
pub(in crate::document) fn merge_neighbours(
    brushes: &mut [Option<Brush>],
    faces: Faces,
    effort: Effort,
) -> HashMap<usize, Vec<usize>> {
    // The brushes there are, and where each was in `brushes`.
    let (originals, parts): (Vec<usize>, Vec<Brush>) = brushes
        .iter_mut()
        .enumerate()
        .filter_map(|(i, brush)| Some((i, brush.take()?)))
        .unzip();
    if parts.len() < 2 {
        for (place, brush) in originals.into_iter().zip(parts) {
            brushes[place] = Some(brush);
        }
        return HashMap::new();
    }
    let touching = touching_pairs(&parts);
    let all: Vec<usize> = (0..parts.len()).collect();
    let pieces = match effort {
        Effort::Quick => greedy(&parts, &touching, &all, faces, Growth::Turns),
        Effort::Thorough => best_of_orders(&parts, &touching, &all, faces),
    };

    // Every brush was taken out above; each piece goes back where its first one was.
    let mut merged_into = HashMap::new();
    for mut piece in pieces {
        piece.members.sort_unstable();
        let first = originals[piece.members[0]];
        brushes[first] = Some(piece.brush);
        if piece.members.len() > 1 {
            let absorbed = piece.members[1..].iter().map(|m| originals[*m]).collect();
            merged_into.insert(first, absorbed);
        }
    }
    merged_into
}

/// A brush made by merging: it and the original brushes it was made of.
#[derive(Clone)]
struct Piece {
    brush: Brush,
    members: Vec<usize>,
}

/// For each of `parts`, the others it meets across a face, found once through cells as
/// big as a middling brush: sized by the smallest, a sliver would make every big brush
/// cover thousands of them.
fn touching_pairs(parts: &[Brush]) -> Vec<Vec<usize>> {
    let mut sizes: Vec<f64> = parts
        .iter()
        .map(|brush| {
            let size = brush.bounds().size();
            size.x().max(size.y()).max(size.z())
        })
        .collect();
    sizes.sort_by(f64::total_cmp);
    let cell = sizes[sizes.len() / 2].max(1.0);
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
    for (i, brush) in parts.iter().enumerate() {
        for c in cells_of(&brush.bounds()) {
            grid.entry(c).or_default().push(i);
        }
    }
    parts
        .iter()
        .enumerate()
        .map(|(i, brush)| {
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
            near.retain(|&j| {
                parts[j].bounds().intersects(&bounds) && share_a_face(brush, &parts[j])
            });
            near
        })
        .collect()
}

/// Whether `a` and `b` lie against each other across a face: one of each in the same
/// plane, facing opposite ways.
fn share_a_face(a: &Brush, b: &Brush) -> bool {
    a.faces().iter().any(|fa| {
        b.faces()
            .iter()
            .any(|fb| geometry_math::plane::is_equal(&fa.plane, &fb.plane.flip(), SAME))
    })
}

/// How a piece that has just taken in a neighbour goes on growing.
#[derive(Clone, Copy)]
enum Growth {
    /// After every other piece has had a turn: pairs form first, then pairs of pairs.
    Turns,
    /// At once, until it can take in no more, before the next piece starts. On random
    /// shapes of boxes this, in the four orders, left 0.5 % more brushes than the fewest
    /// possible, where taking turns left 1.7 %.
    Whole,
}

/// The pieces greedy merging leaves of the `order`ed parts: each piece in turn takes in
/// the first neighbour it can, in that order, and grows again as `growth` says, until
/// none can.
fn greedy(
    parts: &[Brush],
    touching: &[Vec<usize>],
    order: &[usize],
    faces: Faces,
    growth: Growth,
) -> Vec<Piece> {
    // Pieces are named by their first part and kept by that part's number, so plain
    // vectors serve where maps cost a third of a pass.
    const OUTSIDE: usize = usize::MAX;
    let mut rank = vec![OUTSIDE; parts.len()];
    for (place, m) in order.iter().enumerate() {
        rank[*m] = place;
    }
    let mut pieces: Vec<Option<Piece>> = vec![None; parts.len()];
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); parts.len()];
    for m in order {
        pieces[*m] = Some(Piece {
            brush: parts[*m].clone(),
            members: vec![*m],
        });
        neighbours[*m] = touching[*m]
            .iter()
            .copied()
            .filter(|n| rank[*n] != OUTSIDE)
            .collect();
    }
    let mut queue: std::collections::VecDeque<usize> = order.iter().copied().collect();
    while let Some(i) = queue.pop_front() {
        let Some(piece) = &pieces[i] else {
            continue;
        };
        // Neighbour lists may repeat a piece; they are put in order and tidied here.
        let candidates = &mut neighbours[i];
        candidates.sort_unstable_by_key(|n| rank[*n]);
        candidates.dedup();
        let found = candidates.iter().find_map(|j| {
            let other = pieces[*j].as_ref()?;
            Some((*j, merged(&piece.brush, &other.brush, faces)?))
        });
        let Some((j, joined)) = found else {
            continue;
        };
        let taken = pieces[j].take().expect("a neighbour is a piece");
        let piece = pieces[i].as_mut().expect("still a piece");
        piece.brush = joined;
        piece.members.extend(taken.members);
        // The neighbours of the two together, each told that j is now i.
        let theirs = std::mem::take(&mut neighbours[j]);
        for k in &theirs {
            for n in neighbours[*k].iter_mut() {
                if *n == j {
                    *n = i;
                }
            }
        }
        let mine = &mut neighbours[i];
        mine.extend(theirs);
        mine.retain(|n| *n != i && *n != j);
        match growth {
            Growth::Turns => queue.push_back(i),
            Growth::Whole => queue.push_front(i),
        }
    }
    pieces.into_iter().flatten().collect()
}

/// [`greedy`] in the order `members` came in, growing both ways, and in each of
/// [`ORDERS`] growing [`Growth::Whole`], keeping whichever leaves the fewest pieces. Off the web
/// the passes run side by side, one thread each.
fn best_of_orders(
    parts: &[Brush],
    touching: &[Vec<usize>],
    members: &[usize],
    faces: Faces,
) -> Vec<Piece> {
    let centre = |m: usize| {
        let corners = parts[m].vertex_positions();
        corners.iter().fold(Vec3d::zero(), |sum, p| sum + *p) / corners.len().max(1) as f64
    };
    let mut given = members.to_vec();
    given.sort_unstable();
    let sorted = ORDERS.iter().map(|key| {
        let mut order: Vec<(usize, [f64; 3])> =
            members.iter().map(|m| (*m, key(centre(*m)))).collect();
        order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        order.into_iter().map(|(m, _)| m).collect::<Vec<usize>>()
    });
    let passes: Vec<(Vec<usize>, Growth)> =
        [(given.clone(), Growth::Turns), (given, Growth::Whole)]
            .into_iter()
            .chain(sorted.map(|order| (order, Growth::Whole)))
            .collect();
    let run =
        |(order, growth): &(Vec<usize>, Growth)| greedy(parts, touching, order, faces, *growth);
    #[cfg(not(target_arch = "wasm32"))]
    let results: Vec<Vec<Piece>> = if members.len() >= PARALLEL_FROM {
        std::thread::scope(|scope| {
            let handles: Vec<_> = passes
                .iter()
                .map(|pass| scope.spawn(move || run(pass)))
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("a merge pass"))
                .collect()
        })
    } else {
        passes.iter().map(run).collect()
    };
    #[cfg(target_arch = "wasm32")]
    let results: Vec<Vec<Piece>> = passes.iter().map(run).collect();
    // The first of the fewest, so the result does not depend on which thread ends first.
    let fewest = results
        .iter()
        .map(Vec::len)
        .min()
        .expect("there are passes");
    results
        .into_iter()
        .find(|pieces| pieces.len() == fewest)
        .expect("one has the fewest")
}

/// `a` and `b` as one brush, when they share a face, together are convex, and, when
/// `faces` asks for it, every face of the result lies on faces of theirs that look
/// alike; `None` otherwise.
fn merged(a: &Brush, b: &Brush, faces: Faces) -> Option<Brush> {
    // Which faces of each lie in a plane of the other's, facing the other way: where
    // they meet. Worked out once for both tests below.
    let meets = |face: &map_model::brush_face::BrushFace, other: &Brush| {
        other
            .faces()
            .iter()
            .any(|f| geometry_math::plane::is_equal(&face.plane, &f.plane.flip(), SAME))
    };
    let met_a: Vec<bool> = a.faces().iter().map(|face| meets(face, b)).collect();
    // Touching across a face: anything less leaves a gap or a notch the hull would fill.
    if !met_a.contains(&true) {
        return None;
    }
    let met_b: Vec<bool> = b.faces().iter().map(|face| meets(face, a)).collect();
    // A quick test before the hull: the two are convex together only if each one's
    // corners lie behind every plane of the other but the ones they meet across.
    let behind = |corners: &[Vec3d], brush: &Brush, met: &[bool]| {
        brush.faces().iter().zip(met).all(|(face, shared)| {
            *shared
                || corners
                    .iter()
                    .all(|p| vec::dot(face.plane.normal, *p) <= face.plane.distance + SAME * 10.0)
        })
    };
    let (corners_a, corners_b) = (a.vertex_positions(), b.vertex_positions());
    if !behind(&corners_b, a, &met_a) || !behind(&corners_a, b, &met_b) {
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
    let points: Vec<Vec3d> = corners_a
        .into_iter()
        .chain(corners_b)
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
        let on = sources
            .iter()
            .filter(|f| geometry_math::plane::is_equal(&f.plane, &face.plane, SAME));
        let on: Vec<&&map_model::brush_face::BrushFace> = on.collect();
        let first = *on.first()?;
        if faces == Faces::Alike && !on.iter().all(|f| looks_alike(first, f)) {
            return None;
        }
        // A face that shows anywhere shows: it wears the look of a part of it that
        // shows, and is left out of exports only when all of it was.
        let first = on
            .iter()
            .find(|f| !f.export_hidden)
            .copied()
            .unwrap_or(first);
        let mut worn = (*first).clone();
        worn.points = face.points;
        worn.plane = face.plane;
        worn_faces.push(worn);
    }
    let joined = joined.with_exact_face_attributes(&worn_faces)?;
    // Faces too near alike, as two pieces of a gently curved surface make, cannot be
    // built back from their planes when the map is read: such a merge is not made.
    joined.survives_a_file().then_some(joined)
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
#[path = "join_brushes_tests.rs"]
mod join_brushes_tests;

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
    fn joining_brushes_lets_the_looks_go() {
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
        assert_eq!(d.join_brushes(), Ok(1));
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
        assert_eq!(d.join_brushes(), Ok(2));
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
        assert_eq!(d.join_brushes(), Ok(0));
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

    /// How long Join brushes takes on sculpted terrain the size of the column
    /// limit, hills and hollows all over:
    /// `cargo test --release -p editor_core --lib merge_columns::tests::speed_sculpted -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn speed_sculpted() {
        for shape in ColumnShape::ALL {
            let mut d = Document::new(Default::default());
            let side = if shape == ColumnShape::Hexagons {
                640.0
            } else {
                1024.0
            };
            let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(side, side, 32.0));
            d.add_brushes(vec![map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &bounds,
                map_model::shapes::DEFAULT_MATERIAL,
            )
            .unwrap()]);
            d.select_all();
            let made = d.split_into_columns(16.0, shape).unwrap();
            d.clear_selection();
            let mut stroke = d.begin_sculpt_stroke();
            for i in 0..12 {
                let parameters = SculptParameters {
                    radius: 96.0,
                    strength: 0.6,
                    mode: if i % 3 == 0 {
                        crate::sculpt::SculptMode::Lower
                    } else {
                        crate::sculpt::SculptMode::Raise
                    },
                    ..Default::default()
                };
                let at = Vec3d::new(
                    80.0 + (i * 337 % 880) as f64,
                    80.0 + (i * 571 % 880) as f64,
                    32.0,
                );
                d.sculpt_dab(at, &parameters, &mut stroke);
            }
            let parts: Vec<Brush> = brushes(&d);
            let all: Vec<usize> = (0..parts.len()).collect();
            let start = std::time::Instant::now();
            let touching = touching_pairs(&parts);
            println!(
                "{shape:?}: {made} columns, neighbours found in {:?}",
                start.elapsed()
            );
            let start = std::time::Instant::now();
            let quick = greedy(&parts, &touching, &all, Faces::Any, Growth::Turns);
            println!("  one pass: {} in {:?}", quick.len(), start.elapsed());
            let start = std::time::Instant::now();
            let pieces = best_of_orders(&parts, &touching, &all, Faces::Any);
            println!("  all passes: {} in {:?}", pieces.len(), start.elapsed());
        }
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
