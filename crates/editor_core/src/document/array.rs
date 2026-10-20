//! Arrays: copies of the selection in a line, a grid, a ring or scattered over a disc.
//! The copies stay a preview, made again whenever the parameters change, until they are
//! confirmed or cancelled; the whole array undoes in one step.
use super::*;
use crate::i18n::translation_key;

/// The most copies one array makes, as the error for more says.
pub const MAX_COPIES: usize = 1024;

/// How an array lays out its copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayKind {
    /// `count` items, each one `step` on from the last and turned `turn` degrees more.
    Line,
    /// `columns` × `rows` × `layers` items, `step` apart along X, Y and Z.
    Grid,
    /// `count` items spread over `angle` degrees of a ring about `axis`, `radius` from
    /// the selection's centre.
    Radial,
    /// `count` copies at random places within `radius` of the selection's centre, across
    /// `axis`.
    Scatter,
}

impl ArrayKind {
    pub const ALL: [Self; 4] = [Self::Line, Self::Grid, Self::Radial, Self::Scatter];
}

/// What an array makes. Counts include the original.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrayParameters {
    pub kind: ArrayKind,
    /// Items in a line, a ring or a scatter (with the original), or a grid's columns,
    /// rows and layers.
    pub count: [u32; 3],
    /// The offset between neighbours in a line or a grid.
    pub step: Vec3d,
    /// How far each item in a line turns about `axis` from the one before, in degrees.
    pub turn: f64,
    /// The axis a line turns about, a ring goes round and a scatter lies across.
    pub axis: Axis,
    /// How much of the ring a radial array covers, in degrees; 360 is a full ring.
    pub angle: f64,
    /// A ring's radius, or how far a scatter reaches.
    pub radius: f64,
    /// Whether the copies in a ring turn to face its centre, and the copies in a scatter
    /// turn at random about `axis`.
    pub rotate: bool,
    /// The scatter's random sequence.
    pub seed: u32,
    /// Make each copy of a group a linked duplicate of it.
    pub linked: bool,
}

impl Default for ArrayParameters {
    fn default() -> Self {
        Self {
            kind: ArrayKind::Line,
            count: [4, 3, 1],
            step: Vec3d::new(64.0, 0.0, 0.0),
            turn: 0.0,
            axis: Axis::Z,
            angle: 360.0,
            radius: 256.0,
            rotate: true,
            seed: 1,
            linked: false,
        }
    }
}

/// A unit vector along `axis`.
fn unit(axis: Axis) -> Vec3d {
    match axis {
        Axis::X => Vec3d::new(1.0, 0.0, 0.0),
        Axis::Y => Vec3d::new(0.0, 1.0, 0.0),
        Axis::Z => Vec3d::new(0.0, 0.0, 1.0),
    }
}

/// The two unit vectors across `axis`, in turning order.
fn across(axis: Axis) -> (Vec3d, Vec3d) {
    match axis {
        Axis::X => (unit(Axis::Y), unit(Axis::Z)),
        Axis::Y => (unit(Axis::Z), unit(Axis::X)),
        Axis::Z => (unit(Axis::X), unit(Axis::Y)),
    }
}

/// A turn of `degrees` about `axis` through `pivot`.
fn turn_about(pivot: Vec3d, axis: Axis, degrees: f64) -> Mat4x4d {
    mat_ext::translation_matrix(pivot) * mat_ext::rotation_matrix_about_axis(unit(axis), scalar::to_radians(degrees)) * mat_ext::translation_matrix(-pivot)
}

/// SplitMix64: a scatter is the same for the same seed on every platform.
struct Random(u64);
impl Random {
    /// A number in [0, 1).
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Where each copy goes, as a transform of the selection centred on `centre`. The
/// original is not among them.
pub fn array_transforms(parameters: &ArrayParameters, centre: Vec3d) -> Result<Vec<Mat4x4d>, &'static str> {
    let p = parameters;
    let finite = [p.step.x(), p.step.y(), p.step.z(), p.turn, p.angle, p.radius].iter().all(|v| v.is_finite());
    if !finite {
        return Err(translation_key("Enter numbers for every value."));
    }
    let count = p.count[0] as usize;
    let items = match p.kind {
        ArrayKind::Grid => p.count.iter().map(|&n| n.max(1) as usize).product(),
        _ => count,
    };
    if items < 2 {
        return Err(translation_key("An array needs at least two items."));
    }
    if items - 1 > MAX_COPIES {
        return Err(translation_key("An array makes at most 1024 copies."));
    }
    let transforms = match p.kind {
        ArrayKind::Line => (1..count)
            .map(|i| {
                let i = i as f64;
                mat_ext::translation_matrix(p.step * i) * turn_about(centre, p.axis, p.turn * i)
            })
            .collect(),
        ArrayKind::Grid => {
            let [columns, rows, layers] = p.count.map(|n| n.max(1));
            let mut transforms = Vec::with_capacity(items - 1);
            for z in 0..layers {
                for y in 0..rows {
                    for x in 0..columns {
                        if (x, y, z) != (0, 0, 0) {
                            let offset = Vec3d::new(p.step.x() * f64::from(x), p.step.y() * f64::from(y), p.step.z() * f64::from(z));
                            transforms.push(mat_ext::translation_matrix(offset));
                        }
                    }
                }
            }
            transforms
        }
        ArrayKind::Radial => {
            if p.angle == 0.0 {
                return Err(translation_key("Set an angle for the ring."));
            }
            // A full ring spaces the items evenly round it; an arc puts the last at its end.
            let full = (p.angle.abs() - 360.0).abs() < 1e-9;
            let spacing = if full { p.angle / count as f64 } else { p.angle / (count - 1) as f64 };
            let pivot = centre - across(p.axis).0 * p.radius;
            (1..count)
                .map(|i| {
                    let turn = turn_about(pivot, p.axis, spacing * i as f64);
                    if p.rotate { turn } else { mat_ext::translation_matrix(turn * centre - centre) }
                })
                .collect()
        }
        ArrayKind::Scatter => {
            if p.radius <= 0.0 {
                return Err(translation_key("Set a radius for the scatter."));
            }
            let (u, v) = across(p.axis);
            let mut random = Random(u64::from(p.seed));
            (1..count)
                .map(|_| {
                    // Uniform over the disc; whole units keep integer planes integer.
                    let distance = p.radius * random.next().sqrt();
                    let bearing = random.next() * std::f64::consts::TAU;
                    let offset = (u * bearing.cos() + v * bearing.sin()) * distance;
                    let offset = Vec3d::new(offset.x().round(), offset.y().round(), offset.z().round());
                    let spin = if p.rotate { random.next() * 360.0 } else { 0.0 };
                    mat_ext::translation_matrix(offset) * turn_about(centre, p.axis, spin)
                })
                .collect()
        }
    };
    Ok(transforms)
}

/// An array being previewed: the selection it copies and the edit that made it.
pub(super) struct ArrayPreview {
    originals: HashSet<NodeId>,
    parameters: ArrayParameters,
    /// Whether the originals are one whole group.
    linkable: bool,
    revision: u64,
}

impl Document {
    /// Whether the selection is one whole group, which an array can copy as linked
    /// duplicates.
    pub fn can_link_array(&self) -> bool {
        if let Some(preview) = self.array_preview.as_ref().filter(|_| self.has_array_preview()) {
            return preview.linkable;
        }
        node::groups_with_contents(&self.tree.world)
            .iter()
            .any(|group| !group.contents.is_empty() && group.contents.len() == self.selection.len() && group.contents.iter().all(|id| self.selection.contains(id)))
    }

    /// Copies the selection where `parameters` say, as one undo step, and selects the
    /// original with the copies. The copies can be made again with other parameters
    /// until [`Document::finish_array_preview`] or [`Document::cancel_array_preview`].
    /// Returns how many copies there are.
    pub fn begin_array_preview(&mut self, parameters: ArrayParameters) -> Result<usize, &'static str> {
        if self.in_transaction() {
            return Err(translation_key("Finish the current drag first."));
        }
        if self.selection.is_empty() {
            return Err(translation_key("Select what to copy first."));
        }
        self.array_preview = None;
        let originals = self.selection.clone();
        let linkable = self.can_link_array();
        let copies = self.make_array(&originals, &parameters, linkable)?;
        self.array_preview = Some(ArrayPreview { originals, parameters, linkable, revision: self.revision() });
        Ok(copies)
    }

    /// Whether the selection is still the previewed array, with nothing edited since.
    pub fn has_array_preview(&self) -> bool {
        self.array_preview.as_ref().is_some_and(|preview| preview.revision == self.revision())
    }

    /// The parameters the preview was last made with.
    pub fn array_preview_parameters(&self) -> Option<ArrayParameters> {
        self.array_preview.as_ref().filter(|_| self.has_array_preview()).map(|preview| preview.parameters)
    }

    /// Makes the previewed array again with `parameters`. On an error the previous array
    /// stays. Returns how many copies there are now.
    pub fn update_array_preview(&mut self, parameters: ArrayParameters) -> Result<usize, &'static str> {
        if !self.has_array_preview() || self.in_transaction() {
            return Err(translation_key("The array is no longer being previewed."));
        }
        let preview = self.array_preview.take().expect("checked");
        // Taken back first, so the copies are made from the original selection again.
        self.undo();
        self.selection = preview.originals.clone();
        let made = self.make_array(&preview.originals, &parameters, preview.linkable);
        let parameters = if made.is_ok() {
            parameters
        } else {
            // Puts the last good array back.
            self.make_array(&preview.originals, &preview.parameters, preview.linkable).expect("it was made before");
            preview.parameters
        };
        self.array_preview = Some(ArrayPreview { parameters, revision: self.revision(), ..preview });
        made
    }

    /// Keeps the array as it is: the original and the copies stay selected.
    pub fn finish_array_preview(&mut self) -> bool {
        let finished = self.array_preview.take().is_some();
        if finished {
            self.mark_selection_changed();
        }
        finished
    }

    /// Takes the copies away and selects the original again.
    pub fn cancel_array_preview(&mut self) -> bool {
        if !self.has_array_preview() {
            self.array_preview = None;
            return false;
        }
        let preview = self.array_preview.take().expect("checked");
        self.undo();
        self.selection = preview.originals;
        self.face_selection.clear();
        self.mark_selection_changed();
        true
    }

    /// Copies `originals` to every place [`array_transforms`] gives, as one undo step, and
    /// selects the originals with the copies. Changes nothing on an error.
    fn make_array(&mut self, originals: &HashSet<NodeId>, parameters: &ArrayParameters, linkable: bool) -> Result<usize, &'static str> {
        let centre = self.selection_bounds().ok_or(translation_key("Select what to copy first."))?.center();
        let transforms = array_transforms(parameters, centre)?;
        let linked = parameters.linked && linkable;
        let mark = self.history.undo_len();
        let made = self.run_as_one_edit(|document| {
            let mut selected = originals.clone();
            for transform in &transforms {
                document.selection = originals.clone();
                let copied = if linked { document.create_linked_duplicate(0.0).is_some() } else { document.duplicate_selection_by(Vec3d::zero()) > 0 };
                if !copied {
                    return Err(translation_key("The selection could not be copied."));
                }
                if !document.transform_selection(transform) {
                    return Err(translation_key("A copy would leave the map's bounds or break a brush."));
                }
                selected.extend(document.selection.iter().copied());
            }
            document.selection = selected;
            Ok(transforms.len())
        });
        if made.is_err() && self.history.undo_len() > mark {
            self.undo();
        }
        if made.is_err() {
            self.selection = originals.clone();
        }
        self.face_selection.clear();
        self.mark_selection_changed();
        made
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(document: &mut Document) -> NodeId {
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(32.0, 32.0, 32.0));
        document.add_brushes(vec![map_model::shapes::create_cuboid(&document.world_bounds(), &bounds, map_model::shapes::DEFAULT_MATERIAL).unwrap()]);
        *document.selection_ids().iter().next().unwrap()
    }

    fn centres(document: &Document) -> Vec<[i64; 3]> {
        let mut centres: Vec<_> = node::brushes_by_layer(document.tree_world())
            .into_iter()
            .map(|(_, brush, _)| {
                let c = brush.bounds().center();
                [c.x().round() as i64, c.y().round() as i64, c.z().round() as i64]
            })
            .collect();
        centres.sort();
        centres
    }

    #[test]
    fn line_grid_ring_and_scatter_place_their_copies() {
        let mut d = Document::new(Default::default());
        cube(&mut d);
        let before = d.to_map_text();
        let line = ArrayParameters { count: [3, 1, 1], step: Vec3d::new(64.0, 0.0, 0.0), ..Default::default() };
        assert_eq!(d.begin_array_preview(line).unwrap(), 2);
        assert_eq!(centres(&d), vec![[16, 16, 16], [80, 16, 16], [144, 16, 16]]);
        assert_eq!(d.selection_count(), 3);

        let grid = ArrayParameters { kind: ArrayKind::Grid, count: [2, 3, 1], step: Vec3d::new(48.0, 64.0, 0.0), ..line };
        assert_eq!(d.update_array_preview(grid).unwrap(), 5);
        assert_eq!(centres(&d).len(), 6);
        assert!(centres(&d).contains(&[64, 144, 16]));

        let ring = ArrayParameters { kind: ArrayKind::Radial, count: [4, 1, 1], radius: 100.0, ..line };
        assert_eq!(d.update_array_preview(ring).unwrap(), 3);
        // The ring's centre is 100 units along -X from the cube's centre.
        assert_eq!(centres(&d), vec![[-184, 16, 16], [-84, -84, 16], [-84, 116, 16], [16, 16, 16]]);

        let scatter = ArrayParameters { kind: ArrayKind::Scatter, count: [6, 1, 1], radius: 200.0, seed: 7, ..line };
        assert_eq!(d.update_array_preview(scatter).unwrap(), 5);
        let first = centres(&d);
        assert!(first.iter().all(|c| ((c[0] - 16).pow(2) + (c[1] - 16).pow(2)) as f64 <= 201.0f64.powi(2) && c[2] == 16));
        assert_eq!(d.update_array_preview(ArrayParameters { seed: 8, ..scatter }).unwrap(), 5);
        assert_ne!(centres(&d), first, "another seed scatters differently");
        assert_eq!(d.update_array_preview(scatter).unwrap(), 5);
        assert_eq!(centres(&d), first, "the same seed scatters the same");

        // The preview is one undo step, and cancelling it leaves the map as it was.
        assert!(d.has_array_preview());
        assert!(d.cancel_array_preview());
        assert_eq!(d.to_map_text(), before);
        assert_eq!(d.selection_count(), 1);
    }

    #[test]
    fn a_bad_update_keeps_the_last_array_and_a_finished_one_undoes_at_once() {
        let mut d = Document::new(Default::default());
        cube(&mut d);
        let before = d.to_map_text();
        let line = ArrayParameters { count: [3, 1, 1], ..Default::default() };
        d.begin_array_preview(line).unwrap();
        let three = centres(&d);
        assert!(d.update_array_preview(ArrayParameters { count: [1, 1, 1], ..line }).is_err());
        assert!(d.update_array_preview(ArrayParameters { step: Vec3d::new(1e9, 0.0, 0.0), ..line }).is_err());
        // The last good array is made again, with new ids.
        assert_eq!(centres(&d), three);
        assert_eq!(d.selection_count(), 3);
        let three = d.to_map_text();
        assert_eq!(d.array_preview_parameters(), Some(line));
        assert!(d.finish_array_preview());
        assert!(!d.has_array_preview());
        d.undo();
        assert_eq!(d.to_map_text(), before);
        d.redo();
        assert_eq!(d.to_map_text(), three);
    }

    #[test]
    fn an_edit_ends_the_preview() {
        let mut d = Document::new(Default::default());
        cube(&mut d);
        d.begin_array_preview(ArrayParameters::default()).unwrap();
        d.translate_selection(Vec3d::new(16.0, 0.0, 0.0));
        assert!(!d.has_array_preview());
        assert!(d.update_array_preview(ArrayParameters::default()).is_err());
        assert!(!d.cancel_array_preview());
    }

    #[test]
    fn a_group_can_be_arrayed_as_linked_copies() {
        let mut d = Document::new(Default::default());
        cube(&mut d);
        d.group_selection("pillar");
        assert!(d.can_link_array());
        let ring = ArrayParameters { kind: ArrayKind::Radial, count: [4, 1, 1], radius: 100.0, linked: true, ..Default::default() };
        assert_eq!(d.begin_array_preview(ring).unwrap(), 3);
        d.finish_array_preview();
        let groups = node::groups_with_contents(d.tree_world());
        assert_eq!(groups.len(), 4);
        let links: HashSet<_> = groups
            .iter()
            .filter_map(|g| match &g.node.kind {
                NodeKind::Group(group) => group.linked_group_id.clone(),
                _ => None,
            })
            .collect();
        assert_eq!(links.len(), 1, "every copy is in the original's linked set");
        let placed = centres(&d);
        // Syncing the set keeps every copy where the array put it.
        d.sync_linked_groups();
        assert_eq!(centres(&d), placed);
        assert_eq!(placed.len(), 4);
    }
}
