//! Sculpting terrain made of brushes: dabs that move the surface corners of the selected
//! brushes up and down, or along any other direction, and splitting brushes into
//! triangular or square columns to sculpt.
use super::*;
use crate::i18n::translation_key;
use crate::sculpt::{falloff, ColumnShape, SculptDirection, SculptMode, SculptParameters};

/// The thinnest a column may get: Lower stops this far above its bottom.
const MIN_THICKNESS: f64 = 1.0;

/// The most columns one split makes.
pub const MAX_COLUMNS: usize = 8192;

/// Sculpted heights land on this grid, the finest the map checks find tidy.
const HEIGHT_STEP: f64 = 0.125;

/// A face counts as the top of the terrain when its normal points at least this far up.
const UPWARD: f64 = 0.1;

/// One drag of the sculpt brush, undone in one step.
pub struct SculptStroke {
    mark: usize,
    revision: u64,
    /// The way Raise moves the surface, a unit vector. Lower goes the other way.
    along: Vec3d,
    /// Whether the brush reaches a ball round its centre. Otherwise it reaches a column
    /// standing on the ground, and `along` points straight up.
    ball: bool,
    /// The level Flatten moves to, measured along the stroke: where the stroke began.
    level: Option<f64>,
}

/// A vertex position as a key, so the corner several brushes share moves once.
type Key = [i64; 3];

fn key(p: Vec3d) -> Key {
    [p.x(), p.y(), p.z()].map(|v| (v * 1024.0).round() as i64)
}

/// Whether `p` is inside `brush` or on its surface.
fn contains(brush: &Brush, p: Vec3d) -> bool {
    let geometry = brush.geometry();
    brush.bounds().expand(1e-6).contains_point(p)
        && (0..geometry.face_count()).all(|i| {
            let plane = geometry.face_plane(map_model::polyhedron::FaceId(i as u32));
            vec::dot(plane.normal, p) - plane.distance <= 1e-6
        })
}

/// The brushes near a dab, bucketed by where they are, to ask quickly whether a point is
/// inside one of them.
struct Around<'a> {
    cell: f64,
    cells: HashMap<[i64; 3], Vec<&'a Brush>>,
    /// Brushes too big to bucket, asked about every point.
    big: Vec<&'a Brush>,
}

impl<'a> Around<'a> {
    /// The most buckets one brush goes into before it counts as big.
    const MOST_CELLS: i64 = 64;

    fn new(brushes: impl IntoIterator<Item = &'a Brush>, cell: f64) -> Self {
        let mut around = Self {
            cell,
            cells: HashMap::new(),
            big: Vec::new(),
        };
        for brush in brushes {
            let bounds = brush.bounds();
            let (low, high) = (around.cell_of(bounds.min), around.cell_of(bounds.max));
            if (0..3).map(|i| high[i] - low[i] + 1).product::<i64>() > Self::MOST_CELLS {
                around.big.push(brush);
                continue;
            }
            for x in low[0]..=high[0] {
                for y in low[1]..=high[1] {
                    for z in low[2]..=high[2] {
                        around.cells.entry([x, y, z]).or_default().push(brush);
                    }
                }
            }
        }
        around
    }

    fn cell_of(&self, p: Vec3d) -> [i64; 3] {
        [p.x(), p.y(), p.z()].map(|v| (v / self.cell).floor() as i64)
    }

    /// Whether `p` is inside one of the brushes or on one's surface.
    fn covers(&self, p: Vec3d) -> bool {
        let bucket = self
            .cells
            .get(&self.cell_of(p))
            .map_or(&[][..], Vec::as_slice);
        bucket
            .iter()
            .chain(&self.big)
            .any(|brush| contains(brush, p))
    }
}

/// The faces of `brush` that `keep` accepts and the brushes `around` leave open, each as
/// its plane and its corners. A face is open unless something covers it, as the
/// neighbouring columns cover a column's sides.
fn open_faces(
    brush: &Brush,
    around: &Around,
    keep: impl Fn(&Plane3d) -> bool,
) -> Vec<(Plane3d, Vec<Vec3d>)> {
    let geometry = brush.geometry();
    let corners = brush.vertex_positions();
    (0..geometry.face_count())
        .map(|i| geometry.face_plane(map_model::polyhedron::FaceId(i as u32)))
        .filter(|plane| keep(plane))
        .filter_map(|plane| {
            let on: Vec<Vec3d> = corners
                .iter()
                .copied()
                .filter(|p| (vec::dot(plane.normal, *p) - plane.distance).abs() < 1e-4)
                .collect();
            if on.is_empty() {
                return None;
            }
            (!covered(&plane, &on, around)).then_some((plane, on))
        })
        .collect()
}

/// Whether the face in `plane` with corners `on` is covered: points spread over it are all
/// inside something just in front of it. A point on the seam between two neighbours is on
/// both, which counts: it lies on the face's diagonal as often as not, and so do seams.
fn covered(plane: &Plane3d, on: &[Vec3d], around: &Around) -> bool {
    let middle =
        on.iter().fold(Vec3d::new(0.0, 0.0, 0.0), |sum, p| sum + *p) * (1.0 / on.len() as f64);
    on.iter()
        .all(|p| around.covers(middle + (*p - middle) * 0.61 + plane.normal * 0.01))
}

/// The corners of `brush` lying in `plane`.
fn corners_in(brush: &Brush, plane: &Plane3d) -> Vec<Vec3d> {
    brush
        .vertex_positions()
        .into_iter()
        .filter(|p| (vec::dot(plane.normal, *p) - plane.distance).abs() < 1e-4)
        .collect()
}

/// `brush` with each face's omitted-at-export mark set as `hidden` says, or `None` when
/// that changes nothing.
fn with_hidden_faces(
    brush: &Brush,
    hidden: impl Fn(&map_model::brush_face::BrushFace) -> bool,
) -> Option<Brush> {
    let mut faces = brush.faces().to_vec();
    let mut changed = false;
    for face in &mut faces {
        let hide = hidden(face);
        changed |= hide != face.export_hidden;
        face.export_hidden = hide;
    }
    if changed {
        brush.with_exact_face_attributes(&faces)
    } else {
        None
    }
}

/// Whether `brush` has a face in `plane`.
fn has_plane(brush: &Brush, plane: &Plane3d) -> bool {
    brush
        .faces()
        .iter()
        .any(|face| geometry_math::plane::is_equal(&face.plane, plane, 1e-6))
}

/// The area of the flat polygon with corners `on`, in any order, square to `normal`.
fn area(on: &[Vec3d], normal: Vec3d) -> f64 {
    let middle =
        on.iter().fold(Vec3d::new(0.0, 0.0, 0.0), |sum, p| sum + *p) * (1.0 / on.len() as f64);
    let (u, v) = square_to(normal);
    let mut around: Vec<(f64, f64)> = on
        .iter()
        .map(|p| (vec::dot(*p - middle, u), vec::dot(*p - middle, v)))
        .collect();
    around.sort_by(|a, b| a.1.atan2(a.0).total_cmp(&b.1.atan2(b.0)));
    let twice: f64 = (0..around.len())
        .map(|i| {
            let (a, b) = (around[i], around[(i + 1) % around.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    twice.abs() * 0.5
}

/// The axis the columns cut from `brush` stand along: the one its open faces look along
/// on both sides, as a floor's top and underside do, or a wall's inside and outside.
/// Among equals, the one with more open face on either side, then the brush's thinnest.
fn standing_axis(brush: &Brush, around: &Around) -> usize {
    let mut open = [[0.0; 2]; 3];
    for (plane, on) in open_faces(brush, around, |_| true) {
        let area = area(&on, plane.normal);
        for (axis, sides) in open.iter_mut().enumerate() {
            let n = plane.normal[axis];
            sides[usize::from(n < 0.0)] += area * n.abs();
        }
    }
    let size = brush.bounds().size();
    let score = |axis: usize| {
        let [plus, minus] = open[axis];
        (plus.min(minus), plus.max(minus), -size[axis])
    };
    // Z first, so a tie stands up like terrain.
    [2, 0, 1]
        .into_iter()
        .reduce(|best, axis| {
            if score(axis).partial_cmp(&score(best)) == Some(std::cmp::Ordering::Greater) {
                axis
            } else {
                best
            }
        })
        .unwrap()
}

/// The corners of `brush` on its open faces that look along `along`, and those on its open
/// faces that look back. Along the Z axis, the first are the corners of the terrain's top
/// and the second those of its underside.
fn surface_corners(brush: &Brush, along: Vec3d, around: &Around) -> (Vec<Vec3d>, HashSet<Key>) {
    let mut front = Vec::new();
    let mut back = HashSet::new();
    for (plane, on) in open_faces(brush, around, |plane| {
        vec::dot(plane.normal, along).abs() > UPWARD
    }) {
        if vec::dot(plane.normal, along) > 0.0 {
            front.extend(on);
        } else {
            back.extend(on.into_iter().map(key));
        }
    }
    let mut seen = HashSet::new();
    front.retain(|p| seen.insert(key(*p)));
    (front, back)
}

/// How far `p` is from the brush's centre: straight for a ball, across the ground for a
/// column.
fn reach_distance(p: Vec3d, centre: Vec3d, along: Vec3d, ball: bool) -> f64 {
    let offset = p - centre;
    if ball {
        vec::length(offset)
    } else {
        vec::length(offset - along * vec::dot(offset, along))
    }
}

/// `p` with each coordinate a dab moves rounded to [`HEIGHT_STEP`]. The others stay as
/// they are, so terrain on an odd grid keeps its columns where they stand.
fn snap(p: Vec3d, along: Vec3d) -> Vec3d {
    let round = |v: f64, moves: f64| {
        if moves.abs() > 1e-9 {
            (v / HEIGHT_STEP).round() * HEIGHT_STEP
        } else {
            v
        }
    };
    Vec3d::new(
        round(p.x(), along.x()),
        round(p.y(), along.y()),
        round(p.z(), along.z()),
    )
}

/// Two directions square to `along` and to each other.
fn square_to(along: Vec3d) -> (Vec3d, Vec3d) {
    let reference = if along.z().abs() < 0.9 {
        Vec3d::new(0.0, 0.0, 1.0)
    } else {
        Vec3d::new(1.0, 0.0, 0.0)
    };
    let u = vec::normalize(vec::cross(along, reference));
    (u, vec::cross(along, u))
}

impl Document {
    /// Starts a stroke that moves the terrain straight up and down: every dab until the
    /// next begins is one undo step.
    pub fn begin_sculpt_stroke(&self) -> SculptStroke {
        SculptStroke {
            mark: self.history.undo_len(),
            revision: self.revision(),
            along: Vec3d::new(0.0, 0.0, 1.0),
            ball: false,
            level: None,
        }
    }

    /// Starts a stroke that moves the surface along `along`, reaching a ball round each
    /// dab, so it shapes walls and ceilings as well as floors.
    pub fn begin_sculpt_stroke_along(&self, along: Vec3d) -> SculptStroke {
        SculptStroke {
            along: vec::normalize(along),
            ball: true,
            ..self.begin_sculpt_stroke()
        }
    }

    /// Starts a stroke under `ray` that moves the surface the way `direction` says. `None`
    /// when Normal finds no surface under the ray to take its direction from.
    pub fn begin_sculpt_stroke_under(
        &self,
        ray: &geometry_math::ray::Ray3d,
        direction: SculptDirection,
    ) -> Option<SculptStroke> {
        match direction {
            SculptDirection::Up => Some(self.begin_sculpt_stroke()),
            _ => Some(self.begin_sculpt_stroke_along(self.sculpt_axis(ray, direction)?)),
        }
    }

    /// The way Raise moves the surface under `ray`: up, back towards the camera, or out of
    /// the face under the pointer.
    pub fn sculpt_axis(
        &self,
        ray: &geometry_math::ray::Ray3d,
        direction: SculptDirection,
    ) -> Option<Vec3d> {
        match direction {
            SculptDirection::Up => Some(Vec3d::new(0.0, 0.0, 1.0)),
            SculptDirection::Camera => Some(vec::normalize(-ray.direction)),
            SculptDirection::Normal => self.sculpt_hit(ray).map(|(_, normal)| normal),
        }
    }

    /// Where a sculpt dab under `ray` would land: the nearest brush it hits that a dab
    /// changes, which is a selected one when there is a selection.
    pub fn sculpt_target(&self, ray: &geometry_math::ray::Ray3d) -> Option<Vec3d> {
        self.sculpt_hit(ray).map(|(at, _)| at)
    }

    /// [`Self::sculpt_target`] with the outward normal of the face hit there.
    fn sculpt_hit(&self, ray: &geometry_math::ray::Ray3d) -> Option<(Vec3d, Vec3d)> {
        let picked = if self.selection.is_empty() {
            self.pick(ray)?
        } else {
            self.pick_selected(ray)?
        };
        let brush = self.brush(picked.node)?;
        let normal = picked
            .face
            .and_then(|face| brush.faces().get(face))
            .map_or(Vec3d::new(0.0, 0.0, 1.0), |face| face.normal());
        Some((
            geometry_math::abstract_line::AbstractLine::point_at_distance(ray, picked.distance),
            normal,
        ))
    }

    /// The brush's outline round `centre`, square to `along` and laid on the surface
    /// where there is one under it: consecutive points of a closed loop.
    pub fn sculpt_ring(&self, centre: Vec3d, radius: f64, along: Vec3d) -> Vec<Vec3d> {
        const SEGMENTS: usize = 48;
        let along = vec::normalize(along);
        let (u, v) = square_to(along);
        (0..=SEGMENTS)
            .map(|i| {
                let angle = std::f64::consts::TAU * i as f64 / SEGMENTS as f64;
                let at = centre + (u * angle.cos() + v * angle.sin()) * radius;
                let ray = geometry_math::ray::Ray3d::new(at + along * radius, -along);
                // A little off the surface, so it is not lost in it.
                self.sculpt_target(&ray).map_or(at, |hit| hit + along * 0.5)
            })
            .collect()
    }

    /// One dab of the brush at `centre`: moves the surface corners of the brushes within
    /// the radius along the stroke's direction (the selected brushes, or every editable
    /// one when nothing is selected), by the same amount wherever brushes share a
    /// corner, so the surface stays closed. Lower keeps every column at least
    /// [`MIN_THICKNESS`] thick. A dab that would fold a brush in on itself is cut back
    /// until it fits, and changes nothing if even an eighth of it does not. Returns
    /// whether anything moved.
    pub fn sculpt_dab(
        &mut self,
        centre: Vec3d,
        parameters: &SculptParameters,
        stroke: &mut SculptStroke,
    ) -> bool {
        if self.revision() != stroke.revision || self.in_transaction() {
            return false;
        }
        let (along, ball) = (stroke.along, stroke.ball);
        let radius = parameters.radius;
        let strength = parameters.strength.clamp(0.0, 1.0);
        let level = *stroke.level.get_or_insert(vec::dot(centre, along));
        let reach = |bounds: &Bbox3d| {
            let axes = if ball { 3 } else { 2 };
            (0..axes)
                .all(|i| centre[i] + radius >= bounds.min[i] && centre[i] - radius <= bounds.max[i])
        };
        let everything = self.selection.is_empty();
        // With their layers, so asking which are editable does not walk the tree for each.
        let world = node::brushes_by_layer(&self.tree.world);
        let open = self.open_group_contents();
        let around = Around::new(
            world
                .iter()
                .filter(|(_, brush, _)| reach(&brush.bounds().expand(1.0)))
                .map(|(_, brush, _)| *brush),
            (radius * 0.25).max(8.0),
        );
        let brushes: Vec<(NodeId, Brush)> = world
            .iter()
            .filter(|(id, brush, layer)| {
                (everything || self.selection.contains(id))
                    && reach(&brush.bounds())
                    && self.is_editable_on(*id, *layer, &open)
            })
            .map(|(id, brush, _)| (*id, (*brush).clone()))
            .collect();

        // Each surface corner once, with the lowest it may go along the stroke. A corner
        // on an open face looking back is fixed, wherever else it is; one inside the
        // rock, on no open face, moves with the brushes it is on the surface of.
        let mut floor: HashMap<Key, f64> = HashMap::new();
        let mut fixed: HashSet<Key> = HashSet::new();
        let mut surface: HashMap<Key, Vec3d> = HashMap::new();
        for (_, brush) in &brushes {
            let (top, back) = surface_corners(brush, along, &around);
            let top_keys: HashSet<Key> = top.iter().map(|&p| key(p)).collect();
            let bottom = brush
                .vertex_positions()
                .into_iter()
                .filter(|p| !top_keys.contains(&key(*p)))
                .map(|p| vec::dot(p, along))
                .fold(f64::INFINITY, f64::min);
            fixed.extend(back);
            for p in top {
                let lowest = if bottom.is_finite() {
                    bottom + MIN_THICKNESS
                } else {
                    f64::NEG_INFINITY
                };
                let entry = floor.entry(key(p)).or_insert(lowest);
                *entry = entry.max(lowest);
                surface.insert(key(p), p);
            }
        }
        drop(around);

        // How far along the stroke each corner in reach moves.
        let neighbours = radius * 0.5;
        let mut lift: HashMap<Key, f64> = HashMap::new();
        for (&k, &p) in &surface {
            let weight = falloff(reach_distance(p, centre, along, ball), radius);
            if weight <= 0.0 || fixed.contains(&k) {
                continue;
            }
            let height = vec::dot(p, along);
            let target = match parameters.mode {
                SculptMode::Raise => height + radius * 0.1 * strength,
                SculptMode::Lower => height - radius * 0.1 * strength,
                SculptMode::Flatten => height + (level - height) * strength,
                SculptMode::Smooth => {
                    let (sum, total) = surface
                        .values()
                        .map(|q| (q, falloff(reach_distance(*q, p, along, ball), neighbours)))
                        .filter(|(_, w)| *w > 0.0)
                        .fold((0.0, 0.0), |(sum, total), (q, w)| {
                            (sum + vec::dot(*q, along) * w, total + w)
                        });
                    let average = if total > 0.0 { sum / total } else { height };
                    height + (average - height) * strength
                }
            };
            let amount = ((target - height) * weight).max(floor[&k] - height);
            if amount.abs() > 1e-6 {
                lift.insert(k, amount);
            }
        }

        // Where each corner lands for the dab cut to `scale`.
        let place = |p: Vec3d, amount: f64| {
            let moved = snap(p + along * amount, along);
            let lowest = floor[&key(p)];
            if vec::dot(moved, along) < lowest {
                p + along * (lowest - vec::dot(p, along))
            } else {
                moved
            }
        };
        let changes = |scale: f64| -> Option<Vec<(NodeId, Brush)>> {
            let moves: HashMap<Key, Vec3d> = lift
                .iter()
                .map(|(k, amount)| (*k, place(surface[k], amount * scale)))
                .filter(|(k, to)| vec::length(*to - surface[k]) > 1e-6)
                .collect();
            if moves.is_empty() {
                return None;
            }
            let mut changed = Vec::new();
            for (id, brush) in &brushes {
                if !brush
                    .vertex_positions()
                    .iter()
                    .any(|p| moves.contains_key(&key(*p)))
                {
                    continue;
                }
                let moved = brush
                    .map_vertices(|p| moves.get(&key(p)).copied().unwrap_or(p))
                    .ok()?;
                // A corner pushed inside the others is gone, and the surface would tear there.
                if moved.vertex_positions().len() != brush.vertex_positions().len()
                    || !self.world_bounds.contains(&moved.bounds())
                {
                    return None;
                }
                changed.push((*id, moved));
            }
            Some(changed)
        };
        let Some(mut changed) = [1.0, 0.5, 0.25, 0.125].into_iter().find_map(changes) else {
            return false;
        };

        // A rebuilt face takes its marks from the old face that looked most its way, so a
        // top tilted far enough takes a column side's omitted-at-export mark. A hidden face
        // the dab turned keeps the mark only while something still covers it.
        {
            let ids: HashSet<NodeId> = changed.iter().map(|(id, _)| *id).collect();
            let unchanged = world
                .iter()
                .filter(|(id, brush, _)| !ids.contains(id) && reach(&brush.bounds().expand(1.0)))
                .map(|(_, brush, _)| *brush);
            let around = Around::new(
                unchanged.chain(changed.iter().map(|(_, brush)| brush)),
                (radius * 0.25).max(8.0),
            );
            let old: HashMap<NodeId, &Brush> =
                brushes.iter().map(|(id, brush)| (*id, brush)).collect();
            let shown: Vec<(usize, Brush)> = changed
                .iter()
                .enumerate()
                .filter_map(|(i, (id, brush))| {
                    let turned =
                        |face: &map_model::brush_face::BrushFace| !has_plane(old[id], &face.plane);
                    with_hidden_faces(brush, |face| {
                        face.export_hidden
                            && !(turned(face)
                                && !covered(&face.plane, &corners_in(brush, &face.plane), &around))
                    })
                    .map(|shown| (i, shown))
                })
                .collect();
            for (i, brush) in shown {
                changed[i].1 = brush;
            }
        }
        drop(world);
        let old: HashMap<NodeId, &Brush> = brushes.iter().map(|(id, brush)| (*id, brush)).collect();
        let before = changed
            .iter()
            .map(|(id, _)| brush_node(*id, old[id].clone()))
            .collect();
        let after = changed
            .into_iter()
            .map(|(id, brush)| brush_node(id, brush))
            .collect();

        let length = self.history.undo_len();
        self.apply_and_record(Edit::Changed { before, after });
        // A full history drops its oldest step to make room, which moves the mark.
        let evicted = (length + 1).saturating_sub(self.history.undo_len());
        stroke.mark = stroke.mark.saturating_sub(evicted);
        self.history.coalesce_since(stroke.mark);
        stroke.revision = self.revision();
        true
    }

    /// Cuts each selected brush into columns to sculpt, along the world's grid lines
    /// `cell` units apart, so brushes side by side are cut in the same places and share
    /// their corners. The columns stand along the brush's thinnest side: up out of a
    /// floor or a ceiling, out of a wall. Each grid cell makes two triangles or one square,
    /// or the brush is tiled with hexagons of six triangles each, as `shape` says; cells at
    /// the brush's edges are cut short. The columns keep the
    /// brush's materials and go into a group named Terrain, which is selected. Returns how
    /// many there are.
    pub fn split_into_columns(
        &mut self,
        cell: f64,
        shape: ColumnShape,
    ) -> Result<usize, &'static str> {
        if !(cell.is_finite() && cell > 0.0) {
            return Err(translation_key("Set a grid size first."));
        }
        let brushes: Vec<(NodeId, Brush)> = self
            .selected_brushes_with_ids()
            .into_iter()
            .filter(|(id, _)| self.is_editable(*id))
            .collect();
        if brushes.is_empty() {
            return Err(translation_key("Select brushes first"));
        }
        // For each brush: the two axes it is cut across, and the range of grid cells
        // along each that it covers.
        let world = node::brushes_with_ids(&self.tree.world);
        let near = |bounds: &Bbox3d| {
            brushes
                .iter()
                .any(|(_, brush)| brush.bounds().expand(1.0).intersects(bounds))
        };
        let around = Around::new(
            world
                .iter()
                .filter(|(_, brush)| near(&brush.bounds()))
                .map(|(_, brush)| *brush),
            cell.max(8.0),
        );
        let plans: Vec<([usize; 2], [std::ops::Range<i64>; 2])> = brushes
            .iter()
            .map(|(_, brush)| {
                let bounds = brush.bounds();
                let axes = match standing_axis(brush, &around) {
                    0 => [1, 2],
                    1 => [0, 2],
                    _ => [0, 1],
                };
                let range = |axis: usize| {
                    // A brush a hair over a line, as float corners often are, is not cut there.
                    let first = (bounds.min[axis] / cell + 1e-6).floor() as i64;
                    let last = (bounds.max[axis] / cell - 1e-6).ceil() as i64;
                    first..last.max(first + 1)
                };
                (axes, [range(axes[0]), range(axes[1])])
            })
            .collect();
        drop(around);
        drop(world);
        let count: usize = plans
            .iter()
            .map(|(_, [a, b])| {
                (a.end - a.start) as usize * (b.end - b.start) as usize * shape.per_cell()
            })
            .sum();
        if count > MAX_COLUMNS {
            return Err(translation_key(
                "That makes more than 8192 columns. Use a bigger grid size.",
            ));
        }

        let keep_behind = |brush: &Brush, normal: Vec3d, at: f64| -> Option<Brush> {
            match brush.clip(&Plane3d::new(at, normal)) {
                map_model::brush::ClipOutcome::Unchanged => Some(brush.clone()),
                map_model::brush::ClipOutcome::Clipped(clipped) => Some(clipped),
                map_model::brush::ClipOutcome::Empty => None,
            }
        };
        let unit = |axis: usize| {
            let mut v = [0.0; 3];
            v[axis] = 1.0;
            Vec3d::new(v[0], v[1], v[2])
        };
        let mut replacements = HashMap::new();
        let mut made = 0;
        for ((id, brush), ([a, b], [across, down])) in brushes.iter().zip(&plans) {
            let (ua, ub) = (unit(*a), unit(*b));
            let mut pieces = Vec::new();
            if shape == ColumnShape::Hexagons {
                let bounds = brush.bounds();
                let span = |axis: usize, step: f64, reach: f64| {
                    let first = ((bounds.min[axis] - reach) / step).floor() as i64;
                    let last = ((bounds.max[axis] + reach) / step).ceil() as i64;
                    first..=last
                };
                // Each triangle is cut out of the brush by its three sides.
                let triangle = |corners: [(f64, f64); 3]| -> Option<Brush> {
                    let mut piece = brush.clone();
                    for k in 0..3 {
                        let ((xa, xb), (ya, yb), (za, zb)) =
                            (corners[k], corners[(k + 1) % 3], corners[(k + 2) % 3]);
                        let mut normal = vec::normalize(ua * (yb - xb) - ub * (ya - xa));
                        let mut at = vec::dot(normal, ua * xa + ub * xb);
                        if vec::dot(normal, ua * za + ub * zb) > at {
                            normal = -normal;
                            at = -at;
                        }
                        piece = keep_behind(&piece, normal, at)?;
                    }
                    let size = piece.bounds().size();
                    (size[*a] >= 1e-3 && size[*b] >= 1e-3).then_some(piece)
                };
                for i in span(*a, 1.5 * cell, cell) {
                    let ca = 1.5 * cell * i as f64;
                    let offset = if i.rem_euclid(2) == 1 { cell } else { 0.0 };
                    for j in span(*b, 2.0 * cell, 2.0 * cell) {
                        let cb = 2.0 * cell * j as f64 + offset;
                        if ca + cell <= bounds.min[*a]
                            || ca - cell >= bounds.max[*a]
                            || cb + cell <= bounds.min[*b]
                            || cb - cell >= bounds.max[*b]
                        {
                            continue;
                        }
                        let ring = [
                            (ca + cell, cb),
                            (ca + cell / 2.0, cb + cell),
                            (ca - cell / 2.0, cb + cell),
                            (ca - cell, cb),
                            (ca - cell / 2.0, cb - cell),
                            (ca + cell / 2.0, cb - cell),
                        ];
                        for k in 0..6 {
                            pieces.extend(triangle([(ca, cb), ring[k], ring[(k + 1) % 6]]));
                        }
                    }
                }
            } else {
                // A strip at a time, so each cell is cut out of a strip, not the whole brush.
                for i in across.clone() {
                    let (a0, a1) = (i as f64 * cell, (i + 1) as f64 * cell);
                    let Some(strip) =
                        keep_behind(brush, -ua, -a0).and_then(|piece| keep_behind(&piece, ua, a1))
                    else {
                        continue;
                    };
                    for j in down.clone() {
                        let (b0, b1) = (j as f64 * cell, (j + 1) as f64 * cell);
                        let Some(cell) = keep_behind(&strip, -ub, -b0)
                            .and_then(|piece| keep_behind(&piece, ub, b1))
                        else {
                            continue;
                        };
                        let size = cell.bounds().size();
                        if size[*a] < 1e-3 || size[*b] < 1e-3 {
                            continue;
                        }
                        if shape == ColumnShape::Squares {
                            pieces.push(cell);
                            continue;
                        }
                        // The diagonal alternates with the world's cells, so the triangles meet
                        // in a regular pattern rather than all leaning one way, across
                        // neighbouring brushes too.
                        let ((pa, pb), (qa, qb)) = if (i + j).rem_euclid(2) == 0 {
                            ((a0, b0), (a1, b1))
                        } else {
                            ((a1, b0), (a0, b1))
                        };
                        let (da, db) = (qa - pa, qb - pb);
                        let normal = vec::normalize(ua * db - ub * da);
                        let at = vec::dot(normal, ua * pa + ub * pb);
                        pieces.extend(keep_behind(&cell, normal, at));
                        pieces.extend(keep_behind(&cell, -normal, -at));
                    }
                }
            }
            if pieces.is_empty() {
                return Err(translation_key("The box could not be split."));
            }
            // A face the cuts made is inside the brush, against the next piece, so it is
            // left out of exports.
            let pieces: Vec<Brush> = pieces
                .into_iter()
                .map(|piece| {
                    with_hidden_faces(&piece, |face| {
                        face.export_hidden || !has_plane(brush, &face.plane)
                    })
                    .unwrap_or(piece)
                })
                .collect();
            made += pieces.len();
            // The count above is an estimate for hexagons, which the brush's edges cut into
            // more pieces; nothing has changed yet, so the limit still holds.
            if made > MAX_COLUMNS {
                return Err(translation_key(
                    "That makes more than 8192 columns. Use a bigger grid size.",
                ));
            }
            replacements.insert(*id, pieces);
        }
        self.run_as_one_edit(|document| {
            document.replace_geometry_in_place(replacements);
            document.group_selection("Terrain");
        });
        Ok(made)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        d
    }

    /// The slab split into triangular columns, 16 units across, and selected.
    fn terrain() -> Document {
        let mut d = slab();
        assert_eq!(d.split_into_columns(16.0, ColumnShape::Triangles), Ok(128));
        d
    }

    /// The slab split into square columns, 16 units across, and selected.
    fn square_terrain() -> Document {
        let mut d = slab();
        assert_eq!(d.split_into_columns(16.0, ColumnShape::Squares), Ok(64));
        d
    }

    /// Every distinct surface corner with its height.
    fn heights(d: &Document) -> HashMap<[i64; 2], f64> {
        let mut heights = HashMap::new();
        for (_, brush, _) in node::brushes_by_layer(d.tree_world()) {
            for p in surface_corners(brush, Vec3d::new(0.0, 0.0, 1.0), &Around::new([], 16.0)).0 {
                let k = [p.x().round() as i64, p.y().round() as i64];
                if let Some(old) = heights.insert(k, p.z()) {
                    assert!((old - p.z()).abs() < 1e-6, "the surface tore at {k:?}");
                }
            }
        }
        heights
    }

    #[test]
    fn a_box_splits_into_triangular_columns_with_a_flat_top() {
        let d = terrain();
        assert_eq!(d.selection_count(), 128);
        let columns = node::brushes_by_layer(d.tree_world());
        assert!(columns
            .iter()
            .all(|(_, brush, _)| brush.vertex_positions().len() == 6));
        let heights = heights(&d);
        assert_eq!(heights.len(), 81);
        assert!(heights.values().all(|z| (*z - 32.0).abs() < 1e-9));
    }

    #[test]
    fn a_box_splits_into_square_columns_with_a_flat_top() {
        let d = square_terrain();
        assert_eq!(d.selection_count(), 64);
        let columns = node::brushes_by_layer(d.tree_world());
        assert!(columns
            .iter()
            .all(|(_, brush, _)| brush.vertex_positions().len() == 8));
        assert!(columns.iter().all(|(_, brush, _)| {
            let size = brush.bounds().size();
            (size.x() - 16.0).abs() < 1e-9
                && (size.y() - 16.0).abs() < 1e-9
                && (size.z() - 32.0).abs() < 1e-9
        }));
        let heights = heights(&d);
        assert_eq!(heights.len(), 81);
        assert!(heights.values().all(|z| (*z - 32.0).abs() < 1e-9));
    }

    #[test]
    fn the_cuts_follow_the_world_grid_and_the_last_cell_is_cut_short() {
        let mut d = slab();
        // Lines at 24, 48, ... 120 make 6 cells a side, the last 8 across.
        assert_eq!(d.split_into_columns(24.0, ColumnShape::Squares), Ok(36));
        let heights = heights(&d);
        assert_eq!(heights.len(), 49);
        assert!(heights.contains_key(&[120, 0]) && heights.contains_key(&[128, 128]));
        let mut d = slab();
        assert_eq!(d.split_into_columns(24.0, ColumnShape::Triangles), Ok(72));
    }

    #[test]
    fn a_wall_splits_into_columns_that_stand_out_of_it() {
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(16.0, 128.0, 96.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        assert_eq!(d.split_into_columns(16.0, ColumnShape::Squares), Ok(8 * 6));
        let columns = node::brushes_by_layer(d.tree_world());
        assert!(
            columns
                .iter()
                .all(|(_, brush, _)| (brush.bounds().size().x() - 16.0).abs() < 1e-9),
            "the wall is not cut through its thickness"
        );
        let mut d = Document::new(Default::default());
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        assert_eq!(
            d.split_into_columns(16.0, ColumnShape::Triangles),
            Ok(8 * 6 * 2)
        );
        let columns = node::brushes_by_layer(d.tree_world());
        assert!(columns
            .iter()
            .all(|(_, brush, _)| brush.vertex_positions().len() == 6));
        // The triangles' long sides cross the wall's face, not its thickness.
        assert!(columns
            .iter()
            .all(|(_, brush, _)| brush
                .vertex_positions()
                .iter()
                .all(|p| p.x().abs() < 1e-9 || (p.x() - 16.0).abs() < 1e-9)));
    }

    #[test]
    fn a_lone_brush_stands_along_its_thinnest_side_and_a_cube_stands_up() {
        let world = Document::new(Default::default()).world_bounds();
        let axis = |max: [f64; 3]| {
            let bounds = Bbox3d::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(max[0], max[1], max[2]),
            );
            standing_axis(
                &map_model::shapes::create_cuboid(
                    &world,
                    &bounds,
                    map_model::shapes::DEFAULT_MATERIAL,
                )
                .unwrap(),
                &Around::new([], 16.0),
            )
        };
        assert_eq!(axis([64.0, 64.0, 64.0]), 2);
        assert_eq!(axis([16.0, 128.0, 96.0]), 0);
        assert_eq!(axis([128.0, 16.0, 96.0]), 1);
        assert_eq!(axis([128.0, 128.0, 16.0]), 2);
    }

    #[test]
    fn a_brush_of_any_shape_splits_into_pieces_inside_it() {
        // A wedge: a box with a corner cut off.
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(128.0, 128.0, 32.0));
        let cuboid = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap();
        let normal = vec::normalize(Vec3d::new(1.0, 1.0, 0.0));
        let map_model::brush::ClipOutcome::Clipped(wedge) = cuboid.clip(&Plane3d::new(
            vec::dot(normal, Vec3d::new(100.0, 100.0, 0.0)),
            normal,
        )) else {
            panic!("the cut misses")
        };
        d.add_brushes(vec![wedge]);
        let made = d.split_into_columns(16.0, ColumnShape::Triangles).unwrap();
        let columns = node::brushes_by_layer(d.tree_world());
        assert_eq!(columns.len(), made);
        assert!(made > 100 && made < 128, "{made}");
        for (_, brush, _) in &columns {
            assert!(brush
                .vertex_positions()
                .iter()
                .all(|p| vec::dot(normal, *p)
                    <= vec::dot(normal, Vec3d::new(100.0, 100.0, 0.0)) + 1e-6));
            assert!(bounds.expand(1e-6).contains(&brush.bounds()));
        }
    }

    #[test]
    fn a_hollowed_room_splits_and_bends_without_tearing() {
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(128.0, 128.0, 128.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        assert_eq!(d.hollow_selection(16.0), 1);
        d.select_all();
        let walls = d.selection_count();
        let made = d.split_into_columns(16.0, ColumnShape::Triangles).unwrap();
        assert!(made > walls * 10, "{walls} walls made {made} columns");
        d.clear_selection();
        let count = corners(&d).len();
        // From inside the room, into the corner where the floor meets the wall at x = 16.
        let mut stroke = d.begin_sculpt_stroke_along(Vec3d::new(1.0, 0.0, 1.0));
        let params = SculptParameters {
            radius: 40.0,
            strength: 1.0,
            ..Default::default()
        };
        for _ in 0..3 {
            assert!(d.sculpt_dab(Vec3d::new(16.0, 64.0, 16.0), &params, &mut stroke));
        }
        let after = corners(&d);
        assert_eq!(after.len(), count, "no corner was lost");
        // A corner that two brushes share either moved in both or in neither, so no two
        // corners sit a little apart where there was one.
        for (i, p) in after.iter().enumerate() {
            for q in &after[i + 1..] {
                let apart = vec::length(*p - *q);
                assert!(
                    apart < 1e-6 || apart > 4.0,
                    "the room tore between {p:?} and {q:?}"
                );
            }
        }
        assert!(
            near(&after, Vec3d::new(16.0, 64.0, 16.0))
                .iter()
                .all(|p| vec::length(*p - Vec3d::new(16.0, 64.0, 16.0)) > 1.0),
            "the corner moved"
        );
        let moved = after
            .iter()
            .filter(|p| {
                (p.y() - 64.0).abs() < 1e-6
                    && (p.x() - p.z()).abs() < 1e-6
                    && p.x() > 17.0
                    && p.x() < 40.0
            })
            .count();
        assert!(
            moved > 1,
            "into the room, along the diagonal, in the floor and the wall alike"
        );
        // The outside of the room did not move.
        assert!(after
            .iter()
            .filter(|p| p.x() < 8.0 || p.z() < 8.0)
            .all(|p| p.x().abs() < 1e-9 || p.z().abs() < 1e-9 || p.x() > 8.0 && p.z() > 8.0));
    }

    #[test]
    fn a_bad_cell_size_is_refused() {
        for cell in [0.0, -16.0, f64::NAN, f64::INFINITY] {
            let mut d = slab();
            assert!(
                d.split_into_columns(cell, ColumnShape::Squares).is_err(),
                "{cell}"
            );
            assert_eq!(
                node::brushes_by_layer(d.tree_world()).len(),
                1,
                "a refused split leaves the box alone"
            );
        }
        let mut d = Document::new(Default::default());
        assert!(
            d.split_into_columns(16.0, ColumnShape::Squares).is_err(),
            "nothing selected"
        );
    }

    #[test]
    fn a_split_is_one_undo_step() {
        let mut d = square_terrain();
        d.undo();
        let brushes = node::brushes_by_layer(d.tree_world());
        assert_eq!(brushes.len(), 1);
        assert_eq!(brushes[0].1.vertex_positions().len(), 8);
        d.redo();
        assert_eq!(node::brushes_by_layer(d.tree_world()).len(), 64);
    }

    #[test]
    fn sculpting_square_columns_keeps_the_surface_closed() {
        let mut d = square_terrain();
        let params = SculptParameters {
            radius: 40.0,
            strength: 1.0,
            ..Default::default()
        };
        // Off the grid, so the four corners of a cell move by different amounts and the
        // tops have to bend.
        let centre = Vec3d::new(60.0, 70.0, 32.0);
        let mut stroke = d.begin_sculpt_stroke();
        for _ in 0..3 {
            assert!(d.sculpt_dab(centre, &params, &mut stroke));
        }
        let raised = heights(&d);
        assert_eq!(raised.len(), 81, "no corner was lost or added");
        assert!(raised[&[64, 64]] > 32.0);
        assert_eq!(raised[&[0, 0]], 32.0);
        let columns = node::brushes_by_layer(d.tree_world());
        assert_eq!(columns.len(), 64);
        assert!(
            columns
                .iter()
                .all(|(_, brush, _)| brush.vertex_positions().len() == 8),
            "every column keeps its eight corners"
        );
        assert!(
            columns.iter().any(|(_, brush, _)| brush.faces().len() > 6),
            "a bent top is two faces"
        );

        let mut stroke = d.begin_sculpt_stroke();
        let smooth = SculptParameters {
            mode: SculptMode::Smooth,
            ..params
        };
        assert!(d.sculpt_dab(centre, &smooth, &mut stroke));
        assert_eq!(heights(&d).len(), 81);
    }

    /// Boxes `cell` units on a side filling `from`..`to`, added to `d` and left
    /// unselected, so a dab reaches all of them.
    fn blocks(d: &mut Document, from: [f64; 3], to: [f64; 3], cell: f64) {
        let mut brushes = Vec::new();
        let count = |axis: usize| ((to[axis] - from[axis]) / cell).round() as usize;
        for i in 0..count(0) {
            for j in 0..count(1) {
                for k in 0..count(2) {
                    let min = Vec3d::new(
                        from[0] + cell * i as f64,
                        from[1] + cell * j as f64,
                        from[2] + cell * k as f64,
                    );
                    let bounds = Bbox3d::new(min, min + Vec3d::new(cell, cell, cell));
                    brushes.push(
                        map_model::shapes::create_cuboid(
                            &d.world_bounds(),
                            &bounds,
                            map_model::shapes::DEFAULT_MATERIAL,
                        )
                        .unwrap(),
                    );
                }
            }
        }
        d.add_brushes(brushes);
        d.clear_selection();
    }

    /// The corners of every brush, a shared corner once for each brush it is on.
    fn corners(d: &Document) -> Vec<Vec3d> {
        node::brushes_by_layer(d.tree_world())
            .iter()
            .flat_map(|(_, brush, _)| brush.vertex_positions())
            .collect()
    }

    fn near(points: &[Vec3d], at: Vec3d) -> Vec<Vec3d> {
        points
            .iter()
            .copied()
            .filter(|p| vec::length(*p - at) < 6.0)
            .collect()
    }

    #[test]
    fn a_stroke_along_x_pushes_a_wall_and_leaves_its_back_alone() {
        // A wall 16 thick facing +x, 128 wide and tall, in 16-unit blocks.
        let mut d = Document::new(Default::default());
        blocks(&mut d, [0.0, 0.0, 0.0], [16.0, 128.0, 128.0], 16.0);
        let before = corners(&d);
        let mut stroke = d.begin_sculpt_stroke_along(Vec3d::new(1.0, 0.0, 0.0));
        let params = SculptParameters {
            radius: 40.0,
            strength: 1.0,
            ..Default::default()
        };
        assert!(d.sculpt_dab(Vec3d::new(16.0, 64.0, 64.0), &params, &mut stroke));
        let after = corners(&d);
        assert_eq!(
            after.len(),
            before.len(),
            "every block keeps its eight corners"
        );
        let bad: Vec<_> = after
            .iter()
            .filter(|p| {
                (p.y() - p.y().round()).abs() > 1e-6 || (p.z() - p.z().round()).abs() > 1e-6
            })
            .collect();
        assert!(bad.is_empty(), "corners move only along x {bad:?}");
        assert!(
            after
                .iter()
                .filter(|p| p.x() < 8.0)
                .all(|p| p.x().abs() < 1e-9),
            "the back stays where it was"
        );
        let centre = near(&after, Vec3d::new(20.0, 64.0, 64.0));
        assert!(
            !centre.is_empty() && centre.iter().all(|p| p.x() > 16.0),
            "{centre:?}"
        );
        // The face between two blocks is inside the wall, so the corner between them
        // moves with the face in front and nothing else.
        assert!(
            after
                .iter()
                .filter(|p| p.x() > 1e-9)
                .all(|p| p.x() >= 16.0 - 1e-9),
            "no block's middle moved"
        );
        let mut seen: HashMap<[i64; 2], f64> = HashMap::new();
        for p in after.iter().filter(|p| p.x() >= 16.0) {
            let k = [p.y().round() as i64, p.z().round() as i64];
            if let Some(old) = seen.insert(k, p.x()) {
                assert!((old - p.x()).abs() < 1e-9, "the wall tore at {k:?}");
            }
        }
        assert!(
            (seen[&[0, 0]] - 16.0).abs() < 1e-9,
            "outside the radius nothing moves"
        );
    }

    #[test]
    fn a_stroke_down_pulls_a_ceiling_down() {
        let mut d = Document::new(Default::default());
        blocks(&mut d, [0.0, 0.0, 128.0], [128.0, 128.0, 144.0], 16.0);
        let mut stroke = d.begin_sculpt_stroke_along(Vec3d::new(0.0, 0.0, -1.0));
        let params = SculptParameters {
            radius: 40.0,
            strength: 1.0,
            ..Default::default()
        };
        assert!(d.sculpt_dab(Vec3d::new(64.0, 64.0, 128.0), &params, &mut stroke));
        let after = corners(&d);
        assert!(near(&after, Vec3d::new(64.0, 64.0, 124.0))
            .iter()
            .all(|p| p.z() < 128.0));
        assert!(
            after.iter().all(|p| p.z() <= 128.0 || p.z() == 144.0),
            "the top of the ceiling stays"
        );
    }

    #[test]
    fn a_floor_and_a_wall_bend_together_at_their_corner() {
        // Triangular floor columns 0..128 across, 32 tall, and a wall of blocks standing
        // against the floor's +x side, from the floor's bottom up to 128.
        let mut d = terrain();
        blocks(&mut d, [128.0, 0.0, 0.0], [144.0, 128.0, 128.0], 16.0);
        let count = corners(&d).len();
        // Looking into the corner from inside the room, down and towards +x.
        let mut stroke = d.begin_sculpt_stroke_along(Vec3d::new(-1.0, 0.0, 1.0));
        let params = SculptParameters {
            radius: 48.0,
            strength: 1.0,
            ..Default::default()
        };
        assert!(d.sculpt_dab(Vec3d::new(128.0, 64.0, 32.0), &params, &mut stroke));
        let after = corners(&d);
        assert_eq!(after.len(), count, "no corner was lost");
        // The corner where the floor meets the wall moved, in both, the same way.
        let seam: Vec<Vec3d> = near(&after, Vec3d::new(128.0, 64.0, 32.0));
        assert!(
            seam.len() > 1 && seam.iter().all(|p| vec::length(*p - seam[0]) < 1e-9),
            "{seam:?}"
        );
        assert!(seam[0].x() < 128.0 && seam[0].z() > 32.0, "{seam:?}");
        // The floor's underside and the wall's back stayed put.
        assert!(
            after
                .iter()
                .filter(|p| p.z() < 16.0 && p.x() < 120.0)
                .all(|p| p.z().abs() < 1e-9),
            "the floor's underside stays"
        );
        assert!(
            after
                .iter()
                .filter(|p| p.x() > 136.0)
                .all(|p| (p.x() - 144.0).abs() < 1e-9),
            "the wall's back stays"
        );
    }

    #[test]
    fn a_dab_too_deep_for_a_thin_wall_is_cut_back_not_refused() {
        // A wall only 2 units thick, and a dab that would push its face 6 units in.
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 64.0, 64.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        d.clear_selection();
        let mut stroke = d.begin_sculpt_stroke_along(Vec3d::new(1.0, 0.0, 0.0));
        let params = SculptParameters {
            mode: SculptMode::Lower,
            radius: 60.0,
            strength: 1.0,
            ..Default::default()
        };
        assert!(d.sculpt_dab(Vec3d::new(2.0, 32.0, 32.0), &params, &mut stroke));
        let after = corners(&d);
        assert_eq!(after.len(), 8);
        assert!(
            after.iter().all(|p| p.x() >= 0.0),
            "the face stops short of the back"
        );
        assert!(after
            .iter()
            .filter(|p| p.x() > 0.0)
            .all(|p| p.x() >= MIN_THICKNESS - 1e-9));
    }

    #[test]
    fn the_axis_follows_the_camera_or_the_face_under_the_pointer() {
        let mut d = Document::new(Default::default());
        blocks(&mut d, [0.0, 0.0, 0.0], [16.0, 64.0, 64.0], 16.0);
        // From in front of the wall, looking down at it a little.
        let ray = geometry_math::ray::Ray3d::new(
            Vec3d::new(100.0, 32.0, 50.0),
            vec::normalize(Vec3d::new(-1.0, 0.0, -0.2)),
        );
        let up = d.sculpt_axis(&ray, SculptDirection::Up).unwrap();
        let camera = d.sculpt_axis(&ray, SculptDirection::Camera).unwrap();
        let normal = d.sculpt_axis(&ray, SculptDirection::Normal).unwrap();
        assert_eq!(up, Vec3d::new(0.0, 0.0, 1.0));
        assert!(vec::length(camera - vec::normalize(Vec3d::new(1.0, 0.0, 0.2))) < 1e-9);
        assert!(vec::length(normal - Vec3d::new(1.0, 0.0, 0.0)) < 1e-9);
        // Normal needs a surface; the camera does not.
        let away = geometry_math::ray::Ray3d::new(
            Vec3d::new(100.0, 32.0, 50.0),
            Vec3d::new(1.0, 0.0, 0.0),
        );
        assert!(d.sculpt_axis(&away, SculptDirection::Normal).is_none());
        assert!(d
            .begin_sculpt_stroke_under(&away, SculptDirection::Normal)
            .is_none());
        assert!(d
            .begin_sculpt_stroke_under(&away, SculptDirection::Camera)
            .is_some());
        // The outline lies on the wall's face, a little in front of it.
        let ring = d.sculpt_ring(Vec3d::new(16.0, 32.0, 32.0), 16.0, normal);
        assert!(ring.iter().all(|p| (p.x() - 16.5).abs() < 1e-6), "{ring:?}");
    }

    /// Every face omitted at export that nothing covers: a hole the export would show.
    fn holes(d: &Document) -> Vec<Vec3d> {
        let brushes: Vec<&Brush> = node::brushes_by_layer(d.tree_world())
            .into_iter()
            .map(|(_, brush, _)| brush)
            .collect();
        let around = Around::new(brushes.iter().copied(), 16.0);
        brushes
            .iter()
            .flat_map(|brush| {
                brush
                    .faces()
                    .iter()
                    .filter(|face| {
                        face.export_hidden
                            && !covered(&face.plane, &corners_in(brush, &face.plane), &around)
                    })
                    .map(|face| face.plane.normal)
            })
            .collect()
    }

    #[test]
    fn a_split_leaves_the_faces_inside_out_of_exports() {
        for shape in ColumnShape::ALL {
            let mut d = slab();
            d.split_into_columns(16.0, shape).unwrap();
            let slab = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(128.0, 128.0, 32.0));
            let (mut ends, mut sides) = (0, 0.0);
            let columns = node::brushes_by_layer(d.tree_world());
            for (_, brush, _) in &columns {
                for face in brush.faces() {
                    let corners = corners_in(brush, &face.plane);
                    let on_the_slab = (0..3).any(|i| {
                        corners.iter().all(|p| (p[i] - slab.min[i]).abs() < 1e-6)
                            || corners.iter().all(|p| (p[i] - slab.max[i]).abs() < 1e-6)
                    });
                    assert_eq!(
                        face.export_hidden, !on_the_slab,
                        "{shape:?} {:?} {corners:?}",
                        face.plane.normal
                    );
                    if !face.export_hidden {
                        if face.normal().z().abs() > 1.0 - 1e-9 {
                            ends += 1;
                        } else {
                            sides += area(&corners, face.normal());
                        }
                    }
                }
            }
            // Top and bottom of every column, and sides that cover the slab's edge.
            assert_eq!(ends, columns.len() * 2, "{shape:?}");
            assert!(
                (sides - 4.0 * 128.0 * 32.0).abs() < 1e-6,
                "{shape:?} {sides}"
            );
            assert!(holes(&d).is_empty());
        }
    }

    #[test]
    fn a_steep_sculpt_never_hides_a_face_that_shows() {
        for shape in ColumnShape::ALL {
            let mut d = slab();
            d.split_into_columns(16.0, shape).unwrap();
            d.clear_selection();
            // A narrow spike, far steeper than 45 degrees, raised a little at a time.
            let spike = SculptParameters {
                radius: 24.0,
                strength: 1.0,
                ..Default::default()
            };
            let mut stroke = d.begin_sculpt_stroke();
            for _ in 0..30 {
                d.sculpt_dab(Vec3d::new(64.0, 64.0, 32.0), &spike, &mut stroke);
            }
            assert!(heights(&d)[&[64, 64]] > 64.0, "the spike rose");
            assert!(holes(&d).is_empty(), "{shape:?}: {:?}", holes(&d));
            // One dab that tilts a top past 45 degrees at once: Flatten to a level far above,
            // where the stroke began.
            let mut stroke = d.begin_sculpt_stroke();
            let flatten = SculptParameters {
                mode: SculptMode::Flatten,
                ..spike
            };
            assert!(d.sculpt_dab(Vec3d::new(32.0, 32.0, 200.0), &flatten, &mut stroke));
            assert!(heights(&d)[&[32, 32]] > 100.0, "the corner jumped");
            assert!(holes(&d).is_empty(), "{shape:?}: {:?}", holes(&d));
            let mut stroke = d.begin_sculpt_stroke_along(Vec3d::new(1.0, 0.0, 0.3));
            for _ in 0..5 {
                d.sculpt_dab(Vec3d::new(60.0, 64.0, 60.0), &spike, &mut stroke);
            }
            assert!(holes(&d).is_empty(), "{shape:?}: {:?}", holes(&d));
        }
    }

    #[test]
    fn raise_lower_smooth_and_flatten_keep_the_surface_closed() {
        let mut d = terrain();
        let params = SculptParameters {
            radius: 48.0,
            strength: 1.0,
            ..Default::default()
        };
        let centre = Vec3d::new(64.0, 64.0, 32.0);
        let mut stroke = d.begin_sculpt_stroke();
        for _ in 0..3 {
            assert!(d.sculpt_dab(centre, &params, &mut stroke));
        }
        let raised = heights(&d);
        assert!(
            (raised[&[64, 64]] - (32.0 + 3.0 * 4.8)).abs() <= 3.0 * HEIGHT_STEP,
            "the centre rises a tenth of the radius per dab"
        );
        assert!(
            raised.values().all(|z| (z / HEIGHT_STEP).fract() == 0.0),
            "heights stay on the eighth-unit grid"
        );
        assert_eq!(raised[&[0, 0]], 32.0, "outside the radius nothing moves");
        // The whole stroke is one undo step.
        d.undo();
        assert!(heights(&d).values().all(|z| (*z - 32.0).abs() < 1e-9));
        d.redo();

        let mut stroke = d.begin_sculpt_stroke();
        let smooth = SculptParameters {
            mode: SculptMode::Smooth,
            ..params
        };
        assert!(d.sculpt_dab(centre, &smooth, &mut stroke));
        assert!(
            heights(&d)[&[64, 64]] < raised[&[64, 64]],
            "smoothing lowers a peak"
        );

        let mut stroke = d.begin_sculpt_stroke();
        let flatten = SculptParameters {
            mode: SculptMode::Flatten,
            radius: 200.0,
            ..params
        };
        for _ in 0..4 {
            d.sculpt_dab(Vec3d::new(0.0, 0.0, 32.0), &flatten, &mut stroke);
        }
        assert!(
            (heights(&d)[&[64, 64]] - 32.0).abs() < 1.0,
            "flattening levels to where the stroke began"
        );

        // Lower stops a column short of its bottom.
        let mut stroke = d.begin_sculpt_stroke();
        let lower = SculptParameters {
            mode: SculptMode::Lower,
            radius: 200.0,
            ..params
        };
        for _ in 0..20 {
            d.sculpt_dab(centre, &lower, &mut stroke);
        }
        let lowered = heights(&d);
        assert!(
            lowered.values().all(|z| *z >= MIN_THICKNESS - 1e-9),
            "{lowered:?}"
        );
        assert!(lowered.values().any(|z| (*z - MIN_THICKNESS).abs() < 1e-6));
    }

    #[test]
    fn a_selection_limits_the_brush_and_an_edit_ends_the_stroke() {
        let mut d = terrain();
        let centre = Vec3d::new(64.0, 64.0, 32.0);
        // Something else selected: the terrain is left alone.
        let far = Bbox3d::new(
            Vec3d::new(512.0, 512.0, 0.0),
            Vec3d::new(544.0, 544.0, 32.0),
        );
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &far,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        let mut stroke = d.begin_sculpt_stroke();
        assert!(!d.sculpt_dab(centre, &SculptParameters::default(), &mut stroke));
        // Nothing selected: every brush under the brush is sculpted.
        d.clear_selection();
        let mut stroke = d.begin_sculpt_stroke();
        assert!(d.sculpt_dab(centre, &SculptParameters::default(), &mut stroke));
        assert!(heights(&d)[&[64, 64]] > 32.0);
        let mut d = terrain();
        let mut stroke = d.begin_sculpt_stroke();
        d.translate_selection(Vec3d::new(16.0, 0.0, 0.0));
        assert!(!d.sculpt_dab(
            Vec3d::new(64.0, 64.0, 32.0),
            &SculptParameters::default(),
            &mut stroke
        ));
    }

    #[test]
    fn columns_split_again_and_not_into_too_many() {
        // Squares on the same grid fit the triangles already there.
        let mut d = terrain();
        assert_eq!(d.split_into_columns(16.0, ColumnShape::Squares), Ok(128));
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(4096.0, 4096.0, 32.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        assert!(d.split_into_columns(16.0, ColumnShape::Triangles).is_err());
        assert_eq!(d.split_into_columns(256.0, ColumnShape::Triangles), Ok(512));
        // Squares make half as many columns, so the same grid fits twice the area.
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2048.0, 1024.0, 32.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        assert!(d.split_into_columns(16.0, ColumnShape::Triangles).is_err());
        assert_eq!(d.split_into_columns(16.0, ColumnShape::Squares), Ok(8192));
    }

    #[test]
    fn column_shapes_have_names_and_counts() {
        assert_eq!(ColumnShape::Triangles.per_cell(), 2);
        assert_eq!(ColumnShape::Squares.per_cell(), 1);
        assert_eq!(
            SculptParameters::default().columns,
            ColumnShape::Triangles,
            "the split keeps its old shape unless asked"
        );
        assert_eq!(ColumnShape::Hexagons.per_cell(), 2);
        assert_eq!(
            ColumnShape::ALL.map(ColumnShape::name),
            ["Triangle", "Square", "Hexagon"]
        );
    }

    /// The area of every top face of `d`'s brushes together.
    fn top_area(d: &Document) -> f64 {
        node::brushes_by_layer(d.tree_world())
            .iter()
            .flat_map(|(_, brush, _)| brush.faces().iter().map(move |face| (brush, face)))
            .filter(|(_, face)| face.normal().z() > 1.0 - 1e-9)
            .map(|(brush, face)| area(&corners_in(brush, &face.plane), face.normal()))
            .sum()
    }

    #[test]
    fn a_box_splits_into_hexagons_of_six_flat_triangles() {
        let mut d = slab();
        let made = d.split_into_columns(16.0, ColumnShape::Hexagons).unwrap();
        assert_eq!(d.selection_count(), made);
        let columns = node::brushes_by_layer(d.tree_world());
        assert!(columns
            .iter()
            .all(|(_, brush, _)| brush.vertex_positions().len() == 6));
        assert!(
            (top_area(&d) - 128.0 * 128.0).abs() < 1e-6,
            "the pieces cover the box"
        );
        let heights = heights(&d);
        assert!(heights.values().all(|z| (*z - 32.0).abs() < 1e-9));
        // A whole hexagon inside the box: its centre and six corners, on half grid steps.
        for k in [
            [24, 16],
            [16, 0],
            [32, 0],
            [40, 16],
            [32, 32],
            [16, 32],
            [8, 16],
        ] {
            assert!(heights.contains_key(&k), "a hexagon has a corner at {k:?}");
        }
        assert!(
            !heights.contains_key(&[24, 0]),
            "the hexagons are not squares"
        );
    }

    #[test]
    fn with_nothing_selected_the_brush_sculpts_the_terrain_group() {
        let mut d = terrain();
        d.clear_selection();
        let params = SculptParameters {
            radius: 40.0,
            strength: 1.0,
            ..Default::default()
        };
        // From above, as the 3D view finds it under the pointer.
        let ray = geometry_math::ray::Ray3d::new(
            Vec3d::new(64.0, 64.0, 256.0),
            Vec3d::new(0.0, 0.0, -1.0),
        );
        let at = d
            .sculpt_target(&ray)
            .expect("the terrain is under the pointer");
        let mut stroke = d.begin_sculpt_stroke();
        assert!(d.sculpt_dab(at, &params, &mut stroke));
        assert!(heights(&d)[&[64, 64]] > 32.0);
    }

    #[test]
    fn a_hexagon_split_over_the_limit_is_refused_and_changes_nothing() {
        let mut d = Document::new(Default::default());
        // 64 × 64 cells at two columns a cell is exactly the limit, which the estimate
        // allows, but the hexagons the edges cut make more pieces than that.
        let side = 1024.0;
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(side, side, 32.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        d.select_all();
        let before = node::brushes_by_layer(d.tree_world()).len();
        assert!(d.split_into_columns(16.0, ColumnShape::Hexagons).is_err());
        assert_eq!(node::brushes_by_layer(d.tree_world()).len(), before);
    }

    #[test]
    fn neighbouring_brushes_split_into_hexagons_that_meet() {
        let mut d = Document::new(Default::default());
        for x in [0.0, 128.0] {
            let bounds = Bbox3d::new(Vec3d::new(x, 0.0, 0.0), Vec3d::new(x + 128.0, 128.0, 32.0));
            d.add_brushes(vec![map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &bounds,
                map_model::shapes::DEFAULT_MATERIAL,
            )
            .unwrap()]);
        }
        d.select_all();
        d.split_into_columns(16.0, ColumnShape::Hexagons).unwrap();
        assert!((top_area(&d) - 256.0 * 128.0).abs() < 1e-6);
        // A hexagon across the seam at x = 128 is cut in two, and the halves share the
        // corners on the seam, so a dab there moves both and the surface stays closed.
        let params = SculptParameters {
            radius: 40.0,
            strength: 1.0,
            ..Default::default()
        };
        d.clear_selection();
        let mut stroke = d.begin_sculpt_stroke();
        for _ in 0..3 {
            assert!(d.sculpt_dab(Vec3d::new(128.0, 60.0, 32.0), &params, &mut stroke));
        }
        let heights = heights(&d);
        assert!(heights[&[128, 64]] > 32.0);
        assert_eq!(heights[&[0, 0]], 32.0);
    }
}

/// How long sculpting takes on terrain the size of the column limit:
/// `cargo test --release -p editor_core --lib sculpt::speed -- --ignored --nocapture`.
#[cfg(test)]
mod speed {
    use super::*;

    #[test]
    #[ignore = "a measurement; prints, never fails"]
    fn split_and_dabs() {
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(1024.0, 1024.0, 32.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &bounds,
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap()]);
        let start = std::time::Instant::now();
        let made = d.split_into_columns(16.0, ColumnShape::Triangles).unwrap();
        println!("split into {made} columns: {:?}", start.elapsed());
        d.clear_selection();
        let params = SculptParameters {
            radius: 128.0,
            strength: 0.5,
            ..Default::default()
        };
        for (name, along) in [("up", None), ("camera", Some(Vec3d::new(-1.0, 0.0, 1.0)))] {
            for mode in SculptMode::ALL {
                let params = SculptParameters { mode, ..params };
                let mut stroke = match along {
                    None => d.begin_sculpt_stroke(),
                    Some(along) => d.begin_sculpt_stroke_along(along),
                };
                let start = std::time::Instant::now();
                for i in 0..20 {
                    d.sculpt_dab(
                        Vec3d::new(300.0 + i as f64 * 16.0, 512.0, 32.0),
                        &params,
                        &mut stroke,
                    );
                }
                println!("{name} {mode:?}: {:?} a dab", start.elapsed() / 20);
            }
        }
    }
}
