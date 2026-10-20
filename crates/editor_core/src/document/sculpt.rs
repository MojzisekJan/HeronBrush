//! Sculpting terrain made of brushes: dabs that move the upper corners of the selected
//! brushes up or down, and splitting a box into triangular columns to sculpt.
use super::*;
use crate::i18n::translation_key;
use crate::sculpt::{falloff, SculptMode, SculptParameters};

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
    /// The height Flatten levels to: where the stroke began.
    level: Option<f64>,
}

/// A vertex position as a key, so the corner several brushes share moves once.
type Key = [i64; 3];

fn key(p: Vec3d) -> Key {
    [p.x(), p.y(), p.z()].map(|v| (v * 1024.0).round() as i64)
}

/// The corners of `brush` on a face that looks up: the terrain's surface.
fn top_corners(brush: &Brush) -> Vec<Vec3d> {
    let geometry = brush.geometry();
    let planes: Vec<Plane3d> = (0..geometry.face_count())
        .map(|i| geometry.face_plane(map_model::polyhedron::FaceId(i as u32)))
        .filter(|plane| plane.normal.z() > UPWARD)
        .collect();
    brush
        .vertex_positions()
        .into_iter()
        .filter(|&p| planes.iter().any(|plane| (vec::dot(plane.normal, p) - plane.distance).abs() < 1e-4))
        .collect()
}

/// The distance between two points across the ground.
fn across(a: Vec3d, b: Vec3d) -> f64 {
    (a.x() - b.x()).hypot(a.y() - b.y())
}

impl Document {
    /// Starts a stroke: every dab until the next begins is one undo step.
    pub fn begin_sculpt_stroke(&self) -> SculptStroke {
        SculptStroke { mark: self.history.undo_len(), revision: self.revision(), level: None }
    }

    /// Where a sculpt dab under `ray` would land: the nearest brush it hits that a dab
    /// changes, which is a selected one when there is a selection.
    pub fn sculpt_target(&self, ray: &geometry_math::ray::Ray3d) -> Option<Vec3d> {
        let picked = if self.selection.is_empty() { self.pick(ray)? } else { self.pick_selected(ray)? };
        self.brush(picked.node)?;
        Some(geometry_math::abstract_line::AbstractLine::point_at_distance(ray, picked.distance))
    }

    /// The brush's outline round `centre`, laid on the selected surface where there is
    /// one under it: consecutive points of a closed loop.
    pub fn sculpt_ring(&self, centre: Vec3d, radius: f64) -> Vec<Vec3d> {
        const SEGMENTS: usize = 48;
        let down = Vec3d::new(0.0, 0.0, -1.0);
        (0..=SEGMENTS)
            .map(|i| {
                let angle = std::f64::consts::TAU * i as f64 / SEGMENTS as f64;
                let at = centre + Vec3d::new(angle.cos(), angle.sin(), 0.0) * radius;
                let from = Vec3d::new(at.x(), at.y(), centre.z() + radius);
                let ray = geometry_math::ray::Ray3d::new(from, down);
                // A little above the surface, so it is not lost in it.
                self.sculpt_target(&ray).map_or(at, |hit| hit + Vec3d::new(0.0, 0.0, 0.5))
            })
            .collect()
    }

    /// One dab of the brush at `centre`: moves the upper corners of the brushes within
    /// the radius straight up or down (the selected ones, or every editable one when
    /// nothing is selected), by the same amount wherever brushes share a
    /// corner, so the surface stays closed. Lower keeps every column at least
    /// [`MIN_THICKNESS`] thick. A dab that would fold a brush in on itself changes
    /// nothing. Returns whether anything moved.
    pub fn sculpt_dab(&mut self, centre: Vec3d, parameters: &SculptParameters, stroke: &mut SculptStroke) -> bool {
        if self.revision() != stroke.revision || self.in_transaction() {
            return false;
        }
        let radius = parameters.radius;
        let strength = parameters.strength.clamp(0.0, 1.0);
        let level = *stroke.level.get_or_insert(centre.z());
        let reach = |bounds: &Bbox3d| {
            centre.x() + radius >= bounds.min.x() && centre.x() - radius <= bounds.max.x() && centre.y() + radius >= bounds.min.y() && centre.y() - radius <= bounds.max.y()
        };
        let everything = self.selection.is_empty();
        let brushes: Vec<(NodeId, Brush)> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, brush)| (everything || self.selection.contains(id)) && reach(&brush.bounds()) && self.is_editable(*id))
            .map(|(id, brush)| (id, brush.clone()))
            .collect();

        // Each surface corner once, with the lowest it may go; any other corner is fixed.
        let mut floor: HashMap<Key, f64> = HashMap::new();
        let mut fixed: HashSet<Key> = HashSet::new();
        let mut surface: HashMap<Key, Vec3d> = HashMap::new();
        for (_, brush) in &brushes {
            let top = top_corners(brush);
            let top_keys: HashSet<Key> = top.iter().map(|&p| key(p)).collect();
            let bottom = brush.vertex_positions().into_iter().filter(|p| !top_keys.contains(&key(*p))).map(|p| p.z()).fold(f64::INFINITY, f64::min);
            for p in brush.vertex_positions() {
                if !top_keys.contains(&key(p)) {
                    fixed.insert(key(p));
                }
            }
            for p in top {
                let lowest = if bottom.is_finite() { bottom + MIN_THICKNESS } else { f64::NEG_INFINITY };
                let entry = floor.entry(key(p)).or_insert(lowest);
                *entry = entry.max(lowest);
                surface.insert(key(p), p);
            }
        }

        // How far each corner in reach moves.
        let neighbours = radius * 0.5;
        let mut lift: HashMap<Key, f64> = HashMap::new();
        for (&k, &p) in &surface {
            let weight = falloff(across(p, centre), radius);
            if weight <= 0.0 || fixed.contains(&k) {
                continue;
            }
            let target = match parameters.mode {
                SculptMode::Raise => p.z() + radius * 0.1 * strength,
                SculptMode::Lower => p.z() - radius * 0.1 * strength,
                SculptMode::Flatten => p.z() + (level - p.z()) * strength,
                SculptMode::Smooth => {
                    let (sum, total) = surface
                        .values()
                        .map(|q| (q, falloff(across(*q, p), neighbours)))
                        .filter(|(_, w)| *w > 0.0)
                        .fold((0.0, 0.0), |(sum, total), (q, w)| (sum + q.z() * w, total + w));
                    let average = if total > 0.0 { sum / total } else { p.z() };
                    p.z() + (average - p.z()) * strength
                }
            };
            let z = ((p.z() + (target - p.z()) * weight) / HEIGHT_STEP).round() * HEIGHT_STEP;
            let z = z.max(floor[&k]);
            if (z - p.z()).abs() > 1e-6 {
                lift.insert(k, z - p.z());
            }
        }
        if lift.is_empty() {
            return false;
        }

        let mut before = Vec::new();
        let mut after = Vec::new();
        for (id, brush) in brushes {
            if !brush.vertex_positions().iter().any(|p| lift.contains_key(&key(*p))) {
                continue;
            }
            let Ok(moved) = brush.map_vertices(|p| p + Vec3d::new(0.0, 0.0, lift.get(&key(p)).copied().unwrap_or(0.0))) else {
                return false;
            };
            // A corner pushed inside the others is gone, and the surface would tear there.
            if moved.vertex_positions().len() != brush.vertex_positions().len() || !self.world_bounds.contains(&moved.bounds()) {
                return false;
            }
            before.push(brush_node(id, brush));
            after.push(brush_node(id, moved));
        }
        let length = self.history.undo_len();
        self.apply_and_record(Edit::Changed { before, after });
        // A full history drops its oldest step to make room, which moves the mark.
        let evicted = (length + 1).saturating_sub(self.history.undo_len());
        stroke.mark = stroke.mark.saturating_sub(evicted);
        self.history.coalesce_since(stroke.mark);
        stroke.revision = self.revision();
        true
    }

    /// Cuts each selected box into triangular columns `cell` units across, to sculpt.
    /// The columns keep the box's materials and go into a group named Terrain, which is
    /// selected. Returns how many there are.
    pub fn split_into_columns(&mut self, cell: f64) -> Result<usize, &'static str> {
        if !(cell.is_finite() && cell > 0.0) {
            return Err(translation_key("Set a grid size first."));
        }
        let boxes: Vec<(NodeId, Brush)> = self.selected_brushes_with_ids().into_iter().filter(|(id, _)| self.is_editable(*id)).collect();
        if boxes.is_empty() {
            return Err(translation_key("Select a box to split into columns."));
        }
        let mut cells = Vec::new();
        for (id, brush) in &boxes {
            let bounds = brush.bounds();
            let corners = brush.vertex_positions();
            let on_corner = |p: &Vec3d| [p.x(), p.y(), p.z()].iter().zip([[bounds.min.x(), bounds.max.x()], [bounds.min.y(), bounds.max.y()], [bounds.min.z(), bounds.max.z()]]).all(|(v, ends)| ends.iter().any(|e| (v - e).abs() < 1e-6));
            if corners.len() != 8 || !corners.iter().all(on_corner) {
                return Err(translation_key("Only boxes aligned with the axes can be split into columns."));
            }
            let size = bounds.size();
            let columns = ((size.x() / cell).round() as usize).max(1);
            let rows = ((size.y() / cell).round() as usize).max(1);
            cells.push((*id, columns, rows));
        }
        if cells.iter().map(|(_, columns, rows)| columns * rows * 2).sum::<usize>() > MAX_COLUMNS {
            return Err(translation_key("That makes more than 8192 columns. Use a bigger grid size."));
        }

        let keep_behind = |brush: &Brush, normal: Vec3d, at: f64| -> Option<Brush> {
            match brush.clip(&Plane3d::new(at, normal)) {
                map_model::brush::ClipOutcome::Unchanged => Some(brush.clone()),
                map_model::brush::ClipOutcome::Clipped(clipped) => Some(clipped),
                map_model::brush::ClipOutcome::Empty => None,
            }
        };
        let mut replacements = HashMap::new();
        let mut made = 0;
        for ((id, brush), (_, columns, rows)) in boxes.iter().zip(&cells) {
            let bounds = brush.bounds();
            let (w, h) = (bounds.size().x() / *columns as f64, bounds.size().y() / *rows as f64);
            let mut pieces = Vec::with_capacity(columns * rows * 2);
            for row in 0..*rows {
                for column in 0..*columns {
                    let x0 = bounds.min.x() + w * column as f64;
                    let y0 = bounds.min.y() + h * row as f64;
                    let (x1, y1) = (x0 + w, y0 + h);
                    let cell = [
                        (Vec3d::new(-1.0, 0.0, 0.0), -x0),
                        (Vec3d::new(1.0, 0.0, 0.0), x1),
                        (Vec3d::new(0.0, -1.0, 0.0), -y0),
                        (Vec3d::new(0.0, 1.0, 0.0), y1),
                    ]
                    .iter()
                    .try_fold(brush.clone(), |piece, (normal, at)| keep_behind(&piece, *normal, *at));
                    let Some(cell) = cell else { continue };
                    // The diagonal alternates, so the triangles meet in a regular pattern
                    // rather than all leaning one way.
                    let (a, b) = if (row + column) % 2 == 0 { (Vec3d::new(x0, y0, 0.0), Vec3d::new(x1, y1, 0.0)) } else { (Vec3d::new(x1, y0, 0.0), Vec3d::new(x0, y1, 0.0)) };
                    let along = b - a;
                    let normal = vec::normalize(Vec3d::new(along.y(), -along.x(), 0.0));
                    let at = vec::dot(normal, a);
                    pieces.extend(keep_behind(&cell, normal, at));
                    pieces.extend(keep_behind(&cell, -normal, -at));
                }
            }
            if pieces.is_empty() {
                return Err(translation_key("The box could not be split."));
            }
            made += pieces.len();
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

    /// A 128 × 128 box, 32 tall, split into 16-unit columns and selected.
    fn terrain() -> Document {
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(128.0, 128.0, 32.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(&d.world_bounds(), &bounds, map_model::shapes::DEFAULT_MATERIAL).unwrap()]);
        assert_eq!(d.split_into_columns(16.0), Ok(128));
        d
    }

    /// Every distinct surface corner with its height.
    fn heights(d: &Document) -> HashMap<[i64; 2], f64> {
        let mut heights = HashMap::new();
        for (_, brush, _) in node::brushes_by_layer(d.tree_world()) {
            for p in top_corners(brush) {
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
        assert!(columns.iter().all(|(_, brush, _)| brush.vertex_positions().len() == 6));
        let heights = heights(&d);
        assert_eq!(heights.len(), 81);
        assert!(heights.values().all(|z| (*z - 32.0).abs() < 1e-9));
    }

    #[test]
    fn raise_lower_smooth_and_flatten_keep_the_surface_closed() {
        let mut d = terrain();
        let params = SculptParameters { radius: 48.0, strength: 1.0, ..Default::default() };
        let centre = Vec3d::new(64.0, 64.0, 32.0);
        let mut stroke = d.begin_sculpt_stroke();
        for _ in 0..3 {
            assert!(d.sculpt_dab(centre, &params, &mut stroke));
        }
        let raised = heights(&d);
        assert!((raised[&[64, 64]] - (32.0 + 3.0 * 4.8)).abs() <= 3.0 * HEIGHT_STEP, "the centre rises a tenth of the radius per dab");
        assert!(raised.values().all(|z| (z / HEIGHT_STEP).fract() == 0.0), "heights stay on the eighth-unit grid");
        assert_eq!(raised[&[0, 0]], 32.0, "outside the radius nothing moves");
        // The whole stroke is one undo step.
        d.undo();
        assert!(heights(&d).values().all(|z| (*z - 32.0).abs() < 1e-9));
        d.redo();

        let mut stroke = d.begin_sculpt_stroke();
        let smooth = SculptParameters { mode: SculptMode::Smooth, ..params };
        assert!(d.sculpt_dab(centre, &smooth, &mut stroke));
        assert!(heights(&d)[&[64, 64]] < raised[&[64, 64]], "smoothing lowers a peak");

        let mut stroke = d.begin_sculpt_stroke();
        let flatten = SculptParameters { mode: SculptMode::Flatten, radius: 200.0, ..params };
        for _ in 0..4 {
            d.sculpt_dab(Vec3d::new(0.0, 0.0, 32.0), &flatten, &mut stroke);
        }
        assert!((heights(&d)[&[64, 64]] - 32.0).abs() < 1.0, "flattening levels to where the stroke began");

        // Lower stops a column short of its bottom.
        let mut stroke = d.begin_sculpt_stroke();
        let lower = SculptParameters { mode: SculptMode::Lower, radius: 200.0, ..params };
        for _ in 0..20 {
            d.sculpt_dab(centre, &lower, &mut stroke);
        }
        let lowered = heights(&d);
        assert!(lowered.values().all(|z| *z >= MIN_THICKNESS - 1e-9), "{lowered:?}");
        assert!(lowered.values().any(|z| (*z - MIN_THICKNESS).abs() < 1e-6));
    }

    #[test]
    fn a_selection_limits_the_brush_and_an_edit_ends_the_stroke() {
        let mut d = terrain();
        let centre = Vec3d::new(64.0, 64.0, 32.0);
        // Something else selected: the terrain is left alone.
        let far = Bbox3d::new(Vec3d::new(512.0, 512.0, 0.0), Vec3d::new(544.0, 544.0, 32.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(&d.world_bounds(), &far, map_model::shapes::DEFAULT_MATERIAL).unwrap()]);
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
        assert!(!d.sculpt_dab(Vec3d::new(64.0, 64.0, 32.0), &SculptParameters::default(), &mut stroke));
    }

    #[test]
    fn only_boxes_split_and_not_into_too_many_columns() {
        let mut d = terrain();
        assert!(d.split_into_columns(16.0).is_err(), "columns are not boxes");
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(4096.0, 4096.0, 32.0));
        d.add_brushes(vec![map_model::shapes::create_cuboid(&d.world_bounds(), &bounds, map_model::shapes::DEFAULT_MATERIAL).unwrap()]);
        assert!(d.split_into_columns(16.0).is_err());
        assert_eq!(d.split_into_columns(256.0), Ok(512));
    }
}
