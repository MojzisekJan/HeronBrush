//! Free sculpting: brushes shaped in a 3D grid, with ground added and taken away in any
//! direction, overhangs and caves included. Each dab turns the brushes round it into a
//! grid of depths, changes the grid, and builds brushes from it again, so the result is
//! always ordinary brushes that every other tool can work on.
use super::*;
use map_model::voxel::VoxelField;
use map_model::voxel::VoxelTool;
use map_model::voxel::RANGE;

/// What a free stroke remembers between dabs.
pub(super) struct Free {
    /// How far apart the grid's samples are: the grid step.
    step: f64,
    /// The samples the stroke's brushes were last built from, by their place on the
    /// world's grid. A dab starts from these rather than measuring the brushes again, so
    /// dab after dab adds up exactly; measured again, a small change to a narrow pit is
    /// mostly lost and the pit stops deepening.
    samples: HashMap<[i64; 3], f32>,
}

/// A face of the terrain with some of it on the outside.
struct Outside {
    corners: Vec<Vec3d>,
    normal: Vec3d,
    /// Points spread over its outside part.
    samples: Vec<Vec3d>,
    /// The box round its corners.
    bounds: Bbox3d,
}

/// How far `p` is from the box `bounds`: zero inside it.
fn box_distance(p: Vec3d, bounds: &Bbox3d) -> f64 {
    let gap = |a: usize| (bounds.min[a] - p[a]).max(p[a] - bounds.max[a]).max(0.0);
    vec::length(Vec3d::new(gap(0), gap(1), gap(2)))
}

/// The point of the convex polygon with `corners`, in its plane with `normal`, nearest
/// to `p`.
fn nearest_on(p: Vec3d, corners: &[Vec3d], normal: Vec3d) -> Vec3d {
    let on = p - normal * vec::dot(p - corners[0], normal);
    let n = corners.len();
    let side = |i: usize| {
        let (a, b) = (corners[i], corners[(i + 1) % n]);
        vec::dot(vec::cross(b - a, on - a), normal)
    };
    if (0..n).all(|i| side(i) >= -1e-9) || (0..n).all(|i| side(i) <= 1e-9) {
        return on;
    }
    (0..n)
        .map(|i| {
            let (a, b) = (corners[i], corners[(i + 1) % n]);
            let along = b - a;
            let t = (vec::dot(p - a, along) / vec::dot(along, along).max(1e-12)).clamp(0.0, 1.0);
            a + along * t
        })
        .min_by(|a, b| vec::length(*a - p).total_cmp(&vec::length(*b - p)))
        .unwrap_or(on)
}

/// Points spread `spacing` apart over the convex polygon with `corners`, in its plane
/// with `normal`, its corners included, where they lie within `within`.
fn spread_over(corners: &[Vec3d], normal: Vec3d, spacing: f64, within: &Bbox3d) -> Vec<Vec3d> {
    let (u, v) = square_to(normal);
    let across = |p: Vec3d| (vec::dot(p, u), vec::dot(p, v));
    let (lo, hi) = corners.iter().fold(
        (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            let (a, b) = across(*p);
            ((lo.0.min(a), lo.1.min(b)), (hi.0.max(a), hi.1.max(b)))
        },
    );
    let n = corners.len();
    let inside = |p: Vec3d| {
        let side = |i: usize| {
            let (a, b) = (corners[i], corners[(i + 1) % n]);
            vec::dot(vec::cross(b - a, p - a), normal)
        };
        (0..n).all(|i| side(i) >= -1e-9) || (0..n).all(|i| side(i) <= 1e-9)
    };
    let origin = corners[0] - u * across(corners[0]).0 - v * across(corners[0]).1;
    let mut points: Vec<Vec3d> = corners
        .iter()
        .copied()
        .filter(|p| within.contains_point(*p))
        .collect();
    let steps = |from: f64, to: f64| ((to - from) / spacing).ceil().max(0.0) as usize;
    for i in 0..=steps(lo.0, hi.0) {
        for j in 0..=steps(lo.1, hi.1) {
            let a = (lo.0 + i as f64 * spacing).min(hi.0);
            let b = (lo.1 + j as f64 * spacing).min(hi.1);
            let p = origin + u * a + v * b;
            if within.contains_point(p) && inside(p) {
                points.push(p);
            }
        }
    }
    points
}

/// `brush` with each face looking like the face of `sources` in its plane, or else the
/// one that looks most its way: material, colour and mapping alike, so a dab keeps the
/// ground's paint.
fn dressed_like(brush: &Brush, sources: &[&Brush]) -> Brush {
    let mut faces = brush.faces().to_vec();
    for face in &mut faces {
        let normal = face.plane.normal;
        let best = sources
            .iter()
            .flat_map(|source| source.faces())
            .max_by(|a, b| {
                let score = |f: &map_model::brush_face::BrushFace| {
                    let along = vec::dot(f.plane.normal, normal);
                    let coplanar =
                        along > 1.0 - 1e-7 && (f.plane.distance - face.plane.distance).abs() < 1e-6;
                    (coplanar, along)
                };
                let (a, b) = (score(a), score(b));
                a.0.cmp(&b.0).then(a.1.total_cmp(&b.1))
            });
        let Some(best) = best else {
            return brush.clone();
        };
        face.material_name = best.material_name.clone();
        face.color = best.color.clone();
        face.surface_material = best.surface_material;
        face.uv_attributes = best.uv_attributes;
        face.surface_attributes = best.surface_attributes;
        face.uv_coord_system = best.uv_coord_system.updated_for_normal(
            best.plane.normal,
            normal,
            map_model::uv::WrapStyle::Projection,
        );
    }
    brush
        .with_exact_face_attributes(&faces)
        .unwrap_or_else(|| brush.clone())
}

/// How far a held dab's line runs through what the stroke has made.
const HELD_REACH: f64 = 1024.0;

/// Where a Raise or Lower dab aimed at `hit` lands: where the line through it `along`
/// the stroke leaves the ground for Raise, or meets it for Lower, within `reach`. Seen
/// at a slant, the pointer finds the near side of a column, which moving up leaves as
/// it was; the column grows only from its top. Likewise a pit deepens only from its
/// bottom. Seen from the way the surface moves, the hit is where the dab lands.
fn landing(around: &Around, hit: Vec3d, along: Vec3d, raise: bool, reach: f64, step: f64) -> Vec3d {
    // A point beside the hit on the side the line runs through: in the ground for Raise,
    // in the air for Lower.
    let wanted = |p: Vec3d| around.covers(p) == raise;
    if wanted(hit) {
        // A held dab aims where the last one did, which the ground has since grown over
        // or been dug away from.
        // However tall it has grown since: the line runs through what the stroke made.
        return march(&wanted, hit, along, raise, reach.max(HELD_REACH), step).unwrap_or(hit);
    }
    let mut sides = Vec3d::new(0.0, 0.0, 0.0);
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let way = Vec3d::new(f64::from(x), f64::from(y), f64::from(z));
                if (x, y, z) != (0, 0, 0) && wanted(hit + vec::normalize(way) * 0.5) {
                    sides += vec::normalize(way);
                }
            }
        }
    }
    if vec::length(sides) < 1e-9 {
        return hit;
    }
    let start = hit + vec::normalize(sides) * 0.5;
    if !wanted(start) {
        return hit;
    }
    march(&wanted, start, along, raise, reach, step).unwrap_or(hit)
}

/// From `start`, where `wanted` holds, the first point along the stroke (against it for
/// Lower) where it stops holding, within `reach`.
fn march(
    wanted: &dyn Fn(Vec3d) -> bool,
    start: Vec3d,
    along: Vec3d,
    raise: bool,
    reach: f64,
    step: f64,
) -> Option<Vec3d> {
    let way = if raise { along } else { -along };
    let pace = step / 4.0;
    let mut inside = 0.0;
    while inside < reach {
        let next = inside + pace;
        if !wanted(start + way * next) {
            // The edge lies between: halve the gap down to a hundredth of a unit or so.
            let mut outside = next;
            for _ in 0..12 {
                let middle = (inside + outside) / 2.0;
                if wanted(start + way * middle) {
                    inside = middle;
                } else {
                    outside = middle;
                }
            }
            return Some(start + way * outside);
        }
        inside = next;
    }
    None
}

impl Document {
    /// Starts a free stroke on a grid `step` units apart: Raise moves the surface `along`,
    /// adding ground, Lower moves it back, taking ground away, Smooth evens it out, and
    /// Flatten moves it to the plane square to `along` through where the stroke began. Every dab until the next begins is one undo step.
    pub fn begin_free_stroke(&self, step: f64, along: Vec3d) -> SculptStroke {
        SculptStroke {
            free: Some(Free {
                step: step.max(MIN_CELL),
                samples: HashMap::new(),
            }),
            along: vec::normalize(along),
            ..self.begin_sculpt_stroke()
        }
    }

    /// One dab of a free stroke at `centre`. See [`Self::begin_free_stroke`].
    pub(super) fn free_dab(
        &mut self,
        centre: Vec3d,
        parameters: &SculptParameters,
        stroke: &mut SculptStroke,
    ) -> bool {
        let Some(step) = stroke.free.as_ref().map(|free| free.step) else {
            return false;
        };
        let radius = parameters.free_radius(step);
        let strength = parameters.strength.clamp(0.0, 1.0);
        let along = stroke.along;
        let raise = match parameters.mode {
            SculptMode::Raise => Some(true),
            SculptMode::Lower => Some(false),
            _ => None,
        };
        let centre = match raise {
            Some(raise) => {
                let reach = 2.0 * radius;
                let span = Vec3d::new(reach, reach, reach) + Vec3d::new(step, step, step);
                // Round the hit, and on along the line a held dab follows.
                let way = if raise { along } else { -along };
                let far = centre + way * HELD_REACH;
                let near = Bbox3d::new(centre - span, centre + span)
                    .merged(&Bbox3d::new(far - span, far + span));
                let everything = self.selection.is_empty();
                let world = node::brushes_by_layer(&self.tree.world);
                let open = self.open_group_contents();
                let around = Around::new(
                    world
                        .iter()
                        .filter(|(id, brush, layer)| {
                            (everything || self.selection.contains(id))
                                && brush.bounds().intersects(&near)
                                && self.is_editable_on(*id, *layer, &open)
                        })
                        .map(|(_, brush, _)| *brush),
                    step.max(8.0),
                );
                landing(&around, centre, along, raise, reach, step)
            }
            None => centre,
        };
        let level = *stroke.level.get_or_insert(vec::dot(centre, along));

        // The grid round the dab, on the world's grid lines and a step wider than the
        // ball, so its edge is where nothing changes.
        let lo = [0, 1, 2].map(|a| ((centre[a] - radius) / step).floor() - 1.0);
        let hi = [0, 1, 2].map(|a| ((centre[a] + radius) / step).ceil() + 1.0);
        let region = Bbox3d::new(
            Vec3d::new(lo[0], lo[1], lo[2]) * step,
            Vec3d::new(hi[0], hi[1], hi[2]) * step,
        );
        let dims = [0, 1, 2].map(|a| (hi[a] - lo[a]) as usize + 1);

        let everything = self.selection.is_empty();
        let world = node::brushes_by_layer(&self.tree.world);
        let open = self.open_group_contents();
        let candidates: Vec<(NodeId, Brush)> = world
            .iter()
            .filter(|(id, brush, layer)| {
                (everything || self.selection.contains(id))
                    && brush.bounds().intersects(&region)
                    && self.is_editable_on(*id, *layer, &open)
            })
            .map(|(id, brush, _)| (*id, (*brush).clone()))
            .collect();
        drop(world);
        if candidates.is_empty() {
            return false;
        }

        // The terrain's outside, as points half a step apart over the faces where
        // no other piece of it lies just in front. A point's depth is its distance to the
        // nearest of them, so the faces between pieces, where an earlier dab cut the
        // ground up, are not surface, even where they are only partly covered.
        let reach = f64::from(RANGE) * step;
        let spacing = step / 2.0;
        let around = Around::new(candidates.iter().map(|(_, brush)| brush), step.max(8.0));
        let mut faces: Vec<Outside> = Vec::new();
        let near_region = region.expand(reach);
        for (_, brush) in &candidates {
            for (index, face) in brush.faces().iter().enumerate() {
                let Some(corners) = brush.face_polygon(index) else {
                    continue;
                };
                let in_reach = Bbox3d::merge_all(corners.iter().copied())
                    .is_some_and(|bounds| bounds.intersects(&near_region));
                if corners.len() < 3 || !in_reach {
                    continue;
                }
                let normal = face.normal();
                let samples: Vec<Vec3d> = spread_over(&corners, normal, spacing, &near_region)
                    .into_iter()
                    .filter(|at| !around.covers(*at + normal * 0.01))
                    .collect();
                if let (false, Some(bounds)) = (
                    samples.is_empty(),
                    Bbox3d::merge_all(corners.iter().copied()),
                ) {
                    faces.push(Outside {
                        corners,
                        normal,
                        samples,
                        bounds,
                    });
                }
            }
        }
        // The faces by where they reach, in cells two steps wide.
        let size = step * 2.0;
        let bucket = |p: Vec3d| [0, 1, 2].map(|a| (p[a] / size).floor() as i64);
        let mut buckets: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
        for (index, face) in faces.iter().enumerate() {
            let Some(bounds) = Bbox3d::merge_all(face.samples.iter().copied()) else {
                continue;
            };
            let (lo, hi) = (
                bucket(bounds.min - Vec3d::new(reach, reach, reach)),
                bucket(bounds.max + Vec3d::new(reach, reach, reach)),
            );
            for x in lo[0]..=hi[0] {
                for y in lo[1]..=hi[1] {
                    for z in lo[2]..=hi[2] {
                        buckets.entry([x, y, z]).or_default().push(index);
                    }
                }
            }
        }
        // Past the ball the dab changes nothing, so a sample there needs only its side of
        // the surface, unless it is a corner of a cell the dab can change.
        let exact = radius + 2.0 * step;
        let depth = |p: Vec3d| {
            let inside = around.covers(p);
            if vec::length(p - centre) > exact {
                return if inside { RANGE } else { -RANGE };
            }
            // Faces by how far their boxes are, nearest first: a face whose box is
            // further than the best found so far cannot do better.
            let mut near: Vec<(f64, usize)> = buckets
                .get(&bucket(p))
                .into_iter()
                .flatten()
                .map(|&index| (box_distance(p, &faces[index].bounds), index))
                .filter(|(d, _)| *d < reach)
                .collect();
            near.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
            // A face's nearest point is the surface where nothing covers it there; a face
            // covered at that point counts by its nearest outside sample instead.
            let mut distance = reach;
            for (lower, index) in near {
                if lower >= distance {
                    break;
                }
                let face = &faces[index];
                let q = nearest_on(p, &face.corners, face.normal);
                let d = vec::length(p - q);
                if d >= distance {
                    continue;
                }
                distance = if around.covers(q + face.normal * 0.01) {
                    face.samples
                        .iter()
                        .map(|s| vec::length(p - *s))
                        .fold(distance, f64::min)
                } else {
                    d
                };
            }
            ((if inside { distance } else { -distance }) / step) as f32
        };
        let samples = stroke.free.as_ref().map(|free| &free.samples);
        let key = |p: Vec3d| [0, 1, 2].map(|a| (p[a] / step).round() as i64);
        let remembered = |p: Vec3d| {
            samples
                .and_then(|samples| samples.get(&key(p)).copied())
                .unwrap_or_else(|| depth(p))
        };
        let Ok(mut field) = VoxelField::sampled(region.min, step, dims, remembered) else {
            return false;
        };
        drop(around);
        let tool = match parameters.mode {
            SculptMode::Raise => VoxelTool::Add { along },
            SculptMode::Lower => VoxelTool::Remove { along },
            SculptMode::Smooth => VoxelTool::Smooth,
            SculptMode::Flatten => VoxelTool::Flatten {
                at: along * level,
                normal: along,
            },
        };
        let before = field.clone();
        if field.sculpt(centre, radius, strength, tool).is_none() {
            return false;
        }
        // Only the cells the dab changed are built again; the rest keep the brushes as
        // they were, sharp edges and all.
        let changed: HashSet<[usize; 3]> = field.changed_cells(&before).into_iter().collect();
        if changed.is_empty() {
            return false;
        }
        if let Some(free) = stroke.free.as_mut() {
            for &[i, j, k] in &changed {
                for n in 0..8 {
                    let at = [i + (n & 1), j + ((n >> 1) & 1), k + (n >> 2)];
                    let p =
                        region.min + Vec3d::new(at[0] as f64, at[1] as f64, at[2] as f64) * step;
                    // Far from the surface a sample is held at the edge of the range and
                    // says little; measured again, it comes out right.
                    let value = field.value(at);
                    if value.abs() < RANGE - 1.0 {
                        free.samples.insert(key(p), value);
                    } else {
                        free.samples.remove(&key(p));
                    }
                }
            }
        }

        // The ground in the grid as brushes, merged where they can be, and what lies
        // outside the grid of the brushes it reached, kept as it was.
        let material = candidates
            .iter()
            .flat_map(|(_, brush)| brush.faces())
            .map(|face| face.material_name.clone())
            .next()
            .unwrap_or_else(|| self.current_material.clone().into());
        let merge = |brushes: Vec<Brush>, effort| {
            let mut brushes: Vec<Option<Brush>> = brushes.into_iter().map(Some).collect();
            super::super::merge_columns::merge_neighbours(
                &mut brushes,
                super::super::merge_columns::Faces::Alike,
                effort,
            );
            brushes.into_iter().flatten().collect::<Vec<Brush>>()
        };
        let cuboid = |bounds: &Bbox3d| {
            map_model::shapes::create_cuboid(&self.world_bounds, bounds, &material).ok()
        };
        // Cells that are ground all through become a few boxes; only the cells the
        // surface crosses are cut up and merged again.
        let (full, crossed): (HashSet<[usize; 3]>, HashSet<[usize; 3]>) = changed
            .iter()
            .filter(|cell| field.ground_corners(**cell) > 0)
            .partition(|cell| field.ground_corners(**cell) == 8);
        let raw = field.to_brushes_where(&material, |cell| crossed.contains(&cell));
        let mut pieces = merge(raw, super::super::merge_columns::Effort::Quick);
        pieces.extend(field.boxes_of(&full).iter().filter_map(cuboid));
        // Each piece wears the look of the brushes it grew out of, or of all of them
        // when it grew into the air.
        let pieces: Vec<Brush> = pieces
            .iter()
            .map(|piece| {
                let reach = piece.bounds().expand(step);
                let near: Vec<&Brush> = candidates
                    .iter()
                    .map(|(_, brush)| brush)
                    .filter(|brush| brush.bounds().intersects(&reach))
                    .collect();
                if near.is_empty() {
                    let all: Vec<&Brush> = candidates.iter().map(|(_, brush)| brush).collect();
                    dressed_like(piece, &all)
                } else {
                    dressed_like(piece, &near)
                }
            })
            .collect();
        // The changed cells as a few boxes, to cut out of the brushes they reach.
        let cutters: Vec<Brush> = field.boxes_of(&changed).iter().filter_map(cuboid).collect();
        let mut replacements: Vec<(NodeId, Vec<Brush>)> = candidates
            .iter()
            .filter(|(_, brush)| {
                cutters
                    .iter()
                    .any(|cutter| cutter.bounds().intersects(&brush.bounds()))
            })
            .map(|(id, brush)| {
                let left = map_model::csg::subtract_all(brush, &cutters);
                (
                    *id,
                    left.iter()
                        .map(|part| dressed_like(part, &[brush]))
                        .collect(),
                )
            })
            .collect();
        let Some((_, first)) = replacements.first_mut() else {
            return false;
        };
        first.extend(pieces);
        // The pieces and what the cutters left of the brushes round them, merged again
        // where they can be, so dab after dab does not leave the ground in slivers. Only
        // brushes with one parent merge, and they stay in it.
        let parents = self.brush_parents();
        let mut families: HashMap<Option<NodeId>, Vec<usize>> = HashMap::new();
        for (index, (id, _)) in replacements.iter().enumerate() {
            families
                .entry(parents.get(id).copied().flatten())
                .or_default()
                .push(index);
        }
        for members in families.into_values() {
            let pool: Vec<Brush> = members
                .iter()
                .flat_map(|&index| std::mem::take(&mut replacements[index].1))
                .collect();
            replacements[members[0]].1 = merge(pool, super::super::merge_columns::Effort::Quick);
        }

        // A face against another piece is left out of exports; one on the outside shows.
        {
            // With whether the dab may change each, as it may the brushes it reshapes.
            let others: Vec<(NodeId, Brush, bool)> = node::brushes_by_layer(&self.tree.world)
                .iter()
                .filter(|(id, brush, _)| {
                    brush.bounds().intersects(&region.expand(step))
                        && !replacements.iter().any(|(r, _)| r == id)
                })
                .map(|(id, brush, layer)| {
                    let may = (everything || self.selection.contains(id))
                        && self.is_editable_on(*id, *layer, &open);
                    (*id, (*brush).clone(), may)
                })
                .collect();
            let around = Around::new(
                replacements
                    .iter()
                    .flat_map(|(_, brushes)| brushes)
                    .chain(others.iter().map(|(_, brush, _)| brush)),
                step.max(8.0),
            );
            // A brush beside the dab that it uncovered shows those faces again; left
            // hidden, they would be holes in the export.
            let uncovered: Vec<(NodeId, Brush)> = others
                .iter()
                .filter(|(_, _, may)| *may)
                .filter_map(|(id, brush, _)| {
                    with_hidden_faces(brush, |face| {
                        face.export_hidden
                            && covered(&face.plane, &corners_in(brush, &face.plane), &around)
                    })
                    .map(|brush| (*id, brush))
                })
                .collect();
            let marked: Vec<Vec<Brush>> = replacements
                .iter()
                .map(|(_, brushes)| {
                    brushes
                        .iter()
                        .map(|brush| {
                            with_hidden_faces(brush, |face| {
                                covered(&face.plane, &corners_in(brush, &face.plane), &around)
                            })
                            .unwrap_or_else(|| brush.clone())
                        })
                        .collect()
                })
                .collect();
            drop(around);
            for ((_, brushes), marked) in replacements.iter_mut().zip(marked) {
                *brushes = marked;
            }
            replacements.extend(uncovered.into_iter().map(|(id, brush)| (id, vec![brush])));
        }

        let selected = self.selection.clone();
        let replaced: HashSet<NodeId> = replacements.iter().map(|(id, _)| *id).collect();
        let length = self.history.undo_len();
        self.replace_geometry_in_place(replacements.into_iter().collect());
        // The selection stays what it was: nothing, so the next dab reaches every brush
        // again, or the same brushes with the pieces in place of those they replaced.
        if everything {
            self.selection.clear();
        } else {
            let made = std::mem::take(&mut self.selection);
            self.selection = selected
                .difference(&replaced)
                .copied()
                .chain(made)
                .collect();
        }
        let evicted = (length + 1).saturating_sub(self.history.undo_len());
        stroke.mark = stroke.mark.saturating_sub(evicted);
        self.history.coalesce_since(stroke.mark);
        stroke.revision = self.revision();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::slab;
    use super::*;

    fn brushes(d: &Document) -> Vec<Brush> {
        node::brushes_by_layer(d.tree_world())
            .into_iter()
            .map(|(_, brush, _)| brush.clone())
            .collect()
    }

    fn volume(d: &Document) -> f64 {
        brushes(d).iter().map(Brush::volume).sum()
    }

    fn solid_at(d: &Document, p: Vec3d) -> bool {
        brushes(d).iter().any(|brush| contains(brush, p))
    }

    fn dabs(d: &mut Document, mode: SculptMode, at: Vec3d, count: usize) -> usize {
        let params = SculptParameters {
            mode,
            radius: 40.0,
            strength: 1.0,
            ..Default::default()
        };
        let mut stroke = d.begin_free_stroke(16.0, Vec3d::new(0.0, 0.0, 1.0));
        (0..count)
            .filter(|_| d.sculpt_dab(at, &params, &mut stroke))
            .count()
    }

    #[test]
    fn raise_grows_a_hill_of_brushes_out_of_a_box() {
        let mut d = slab();
        d.clear_selection();
        let before = volume(&d);
        // Each dab rebuilds the brushes, and the next starts from them again, so a hill
        // keeps growing dab after dab.
        assert_eq!(
            dabs(&mut d, SculptMode::Raise, Vec3d::new(64.0, 64.0, 32.0), 4),
            4
        );
        assert!(volume(&d) > before);
        assert!(
            solid_at(&d, Vec3d::new(64.0, 64.0, 40.0)),
            "a hill stands on the top"
        );
        assert!(
            solid_at(&d, Vec3d::new(64.0, 64.0, 16.0)),
            "no hole under it"
        );
        assert!(
            solid_at(&d, Vec3d::new(4.0, 4.0, 16.0)),
            "the far corner is kept"
        );
        assert!(!solid_at(&d, Vec3d::new(4.0, 4.0, 40.0)));
        assert!(brushes(&d).iter().all(|brush| brush.volume() > 0.0));
    }

    #[test]
    fn a_dab_near_an_edge_leaves_the_edge_sharp() {
        let mut d = slab();
        d.clear_selection();
        // The ball reaches the middle of the slab's edge but not its corner, though the
        // grid round the dab covers both.
        dabs(&mut d, SculptMode::Raise, Vec3d::new(40.0, 40.0, 32.0), 2);
        let corners: Vec<Vec3d> = brushes(&d)
            .iter()
            .flat_map(Brush::vertex_positions)
            .collect();
        assert!(corners
            .iter()
            .any(|p| vec::length(*p - Vec3d::new(0.0, 0.0, 32.0)) < 1e-6));
        assert!(solid_at(&d, Vec3d::new(0.5, 0.5, 31.5)));
    }

    /// How long a dab takes on a large flat slab:
    /// `cargo test --release -p editor_core --lib sculpt::free::tests::speed -- --ignored --nocapture`.
    #[test]
    #[ignore = "a measurement; prints, never fails"]
    fn speed() {
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(1024.0, 1024.0, 64.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        let params = SculptParameters {
            radius: 128.0,
            strength: 0.5,
            ..Default::default()
        };
        let mut stroke = d.begin_free_stroke(16.0, Vec3d::new(0.0, 0.0, 1.0));
        for n in 0..8 {
            let at = Vec3d::new(400.0 + 24.0 * f64::from(n), 500.0, 64.0);
            let start = std::time::Instant::now();
            d.sculpt_dab(at, &params, &mut stroke);
            println!(
                "dab {n}: {:?}, {} brushes",
                start.elapsed(),
                node::brushes_by_layer(d.tree_world()).len()
            );
        }
    }

    #[test]
    fn lower_digs_a_pit_and_the_bottom_stays() {
        let mut d = slab();
        d.clear_selection();
        dabs(&mut d, SculptMode::Lower, Vec3d::new(64.0, 64.0, 32.0), 6);
        assert!(!solid_at(&d, Vec3d::new(64.0, 64.0, 28.0)));
        assert!(solid_at(&d, Vec3d::new(4.0, 4.0, 16.0)));
    }

    #[test]
    fn dabs_that_change_nothing_do_not_wear_the_ground_away() {
        let mut d = slab();
        d.clear_selection();
        let before = volume(&d);
        // Smoothing a flat top leaves it flat, dab after dab, though each dab rebuilds
        // the brushes from the depths the previous one left.
        dabs(&mut d, SculptMode::Smooth, Vec3d::new(64.0, 64.0, 32.0), 5);
        assert!(
            (volume(&d) - before).abs() / before < 0.01,
            "{} {before}",
            volume(&d)
        );
        assert!(solid_at(&d, Vec3d::new(64.0, 64.0, 31.0)));
        assert!(!solid_at(&d, Vec3d::new(64.0, 64.0, 33.0)));
    }

    #[test]
    fn the_sculpted_ground_keeps_its_paint() {
        let mut d = slab();
        d.select_all();
        d.set_face_color(Some(map_model::brush_face::FaceColor::solid(
            map_model::brush_face::Rgb::new(112, 158, 84),
        )));
        d.clear_selection();
        dabs(&mut d, SculptMode::Raise, Vec3d::new(40.0, 40.0, 32.0), 2);
        dabs(&mut d, SculptMode::Lower, Vec3d::new(96.0, 96.0, 32.0), 2);
        assert!(brushes(&d).len() > 1);
        assert!(brushes(&d)
            .iter()
            .flat_map(|brush| brush.faces())
            .all(|face| face.color.is_some()));
    }

    #[test]
    fn a_stroke_is_one_undo_step() {
        let mut d = slab();
        d.clear_selection();
        let before = brushes(&d).len();
        dabs(&mut d, SculptMode::Raise, Vec3d::new(64.0, 64.0, 32.0), 3);
        assert!(brushes(&d).len() > before);
        assert!(d.undo().is_some());
        assert_eq!(brushes(&d).len(), before);
        assert!(d.selection_count() == 0, "the selection stays empty");
    }

    /// A box from the origin to `size`, nothing selected.
    fn block(size: Vec3d) -> Document {
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), size);
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        d.clear_selection();
        d
    }

    /// The button held over one spot with the Camera direction: `count` dabs, each where
    /// `from` looking `towards` meets the brushes, as the 3D view aims them. Returns how
    /// far along `towards` the last hit was.
    fn hold(
        d: &mut Document,
        mode: SculptMode,
        radius: f64,
        from: Vec3d,
        towards: Vec3d,
        count: usize,
    ) -> f64 {
        let params = SculptParameters {
            mode,
            radius,
            strength: 1.0,
            ..Default::default()
        };
        let ray = geometry_math::ray::Ray3d::new(from, towards);
        let mut stroke = d.begin_free_stroke(16.0, -towards);
        for _ in 0..count {
            let at = d.sculpt_target(&ray).expect("the ray meets the ground");
            d.sculpt_dab(at, &params, &mut stroke);
        }
        vec::dot(
            d.sculpt_target(&ray).expect("the ray meets the ground"),
            towards,
        )
    }

    /// The grid is 16, and the rays miss its lines, as a pointer does.
    fn column(d: &mut Document) {
        hold(
            d,
            SculptMode::Raise,
            48.0,
            Vec3d::new(256.3, 256.7, 1000.0),
            Vec3d::new(0.0, 0.0, -1.0),
            30,
        );
    }

    #[test]
    fn held_raise_grows_a_column_as_wide_as_the_ring() {
        let mut d = block(Vec3d::new(512.0, 512.0, 64.0));
        column(&mut d);
        // A tenth of the radius a dab: some 140 units in 30 dabs, not a spike chasing its
        // own tip.
        assert!(solid_at(&d, Vec3d::new(256.0, 256.0, 180.0)));
        for side in [Vec3d::new(28.0, 0.0, 0.0), Vec3d::new(0.0, -28.0, 0.0)] {
            assert!(
                solid_at(&d, Vec3d::new(256.0, 256.0, 150.0) + side),
                "the column is wide"
            );
        }
        assert!(!solid_at(&d, Vec3d::new(256.0 + 72.0, 256.0, 150.0)));
        assert!(
            brushes(&d).len() < 300,
            "{} brushes for one column",
            brushes(&d).len()
        );
    }

    #[test]
    fn a_branch_grows_out_of_the_column_over_air() {
        let mut d = block(Vec3d::new(512.0, 512.0, 64.0));
        column(&mut d);
        let side = hold(
            &mut d,
            SculptMode::Raise,
            32.0,
            Vec3d::new(1000.0, 256.7, 160.3),
            Vec3d::new(-1.0, 0.0, 0.0),
            30,
        );
        assert!(-side > 256.0 + 100.0, "the branch reaches out to {}", -side);
        let out = 256.0 + 90.0;
        assert!(solid_at(&d, Vec3d::new(out, 256.0, 160.0)));
        assert!(
            !solid_at(&d, Vec3d::new(out, 256.0, 100.0)),
            "air under the branch"
        );
        assert!(
            solid_at(&d, Vec3d::new(out, 256.0, 32.0)),
            "the ground stays"
        );
    }

    #[test]
    fn held_lower_drills_a_tunnel_into_a_wall() {
        let mut d = block(Vec3d::new(512.0, 256.0, 256.0));
        let deep = hold(
            &mut d,
            SculptMode::Lower,
            24.0,
            Vec3d::new(250.3, -1000.0, 123.7),
            Vec3d::new(0.0, 1.0, 0.0),
            40,
        );
        // A narrow pit kept deepening only once each dab built on the last one's samples;
        // measuring the brushes again, it stopped some two cells in.
        assert!(deep > 40.0, "the tunnel reaches {deep}");
        assert!(!solid_at(&d, Vec3d::new(250.0, 30.0, 124.0)));
        assert!(
            solid_at(&d, Vec3d::new(250.0, 30.0, 124.0 + 48.0)),
            "the roof holds"
        );
        assert!(solid_at(&d, Vec3d::new(250.0, 30.0, 124.0 - 48.0)));
    }

    #[test]
    fn a_brush_smaller_than_two_cells_still_grows_on_a_coarse_grid() {
        let mut d = block(Vec3d::new(512.0, 512.0, 64.0));
        let ray = geometry_math::ray::Ray3d::new(
            Vec3d::new(1000.0, 256.7, 40.3),
            Vec3d::new(-1.0, 0.0, 0.0),
        );
        let params = SculptParameters {
            mode: SculptMode::Raise,
            radius: 40.0,
            strength: 0.5,
            ..Default::default()
        };
        let mut stroke = d.begin_free_stroke(32.0, Vec3d::new(1.0, 0.0, 0.0));
        let side = d.sculpt_target(&ray).unwrap();
        for _ in 0..20 {
            d.sculpt_dab(side, &params, &mut stroke);
        }
        // The ball reaches two steps, 64 units, so a dab moves the surface 3.2; a ball of
        // 40 on a grid of 32 stalled some 15 units out.
        let out = d.sculpt_target(&ray).unwrap().x() - side.x();
        assert!(out > 48.0, "the side moved {out}");
    }

    #[test]
    fn sculpted_brushes_come_back_from_a_map_file_with_no_face_wrongly_left_out() {
        let mut d = block(Vec3d::new(512.0, 512.0, 64.0));
        column(&mut d);
        hold(
            &mut d,
            SculptMode::Raise,
            32.0,
            Vec3d::new(1000.0, 256.7, 160.3),
            Vec3d::new(-1.0, 0.0, 0.0),
            30,
        );
        // Corners off the eighth-unit lattice made faces too near alike to build back.
        let map = map_model::io::map_reader::read_map(&d.to_map_text(), None).unwrap();
        let tree = map_model::node::build_tree(&map, &d.world_bounds());
        assert!(tree.warnings.is_empty(), "{}", tree.warnings[0]);
        // A face left out of exports has something just in front of it all over.
        let all = brushes(&d);
        for brush in &all {
            for (index, face) in brush.faces().iter().enumerate() {
                if !face.export_hidden {
                    continue;
                }
                let corners = brush.face_polygon(index).unwrap();
                let middle = corners
                    .iter()
                    .fold(Vec3d::new(0.0, 0.0, 0.0), |a, b| a + *b)
                    / corners.len() as f64;
                for p in corners
                    .iter()
                    .map(|c| middle + (*c - middle) * 0.8)
                    .chain([middle])
                {
                    let ahead = p + face.normal() * 0.05;
                    assert!(all.iter().any(|b| contains(b, ahead)), "{ahead:?} shows");
                }
            }
        }
    }
}
