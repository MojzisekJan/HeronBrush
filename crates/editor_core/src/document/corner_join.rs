use super::*;
pub struct CornerDraft {
    pub revision: u64,
    pub ids: [NodeId; 2],
    pub join: crate::corner_join::Join,
    /// With more than two brushes selected, how many joins the command makes; `join`
    /// is the first of them.
    pub corners: usize,
    /// Every selected brush as it is, and every piece as the whole command leaves it,
    /// selected brushes that overlap nothing included: what a preview of the command
    /// draws, from the same computation the command applies.
    pub before_all: Vec<map_model::brush::Brush>,
    pub after_all: Vec<map_model::brush::Brush>,
    /// The two axes a flat preview of the whole selection looks along.
    pub axes: [usize; 2],
    /// Corners the command leaves alone and why: perpendicular brushes that only touch
    /// or stop a little short of each other, so a room's missed corner can be found.
    pub left: Vec<String>,
}

/// Why a Join corner over the selection is refused, for a preview to show: the message,
/// the selected brushes, the overlap it is about if any, and the axes to draw them on.
pub struct CornerRefusal {
    pub message: String,
    pub brushes: Vec<map_model::brush::Brush>,
    pub place: Option<Bbox3d>,
    pub axes: [usize; 2],
}

/// Every join a selection of more than two brushes makes, in the order they are made.
struct Joins {
    /// Each pair and its join, made on the brushes as the joins before it left them.
    pairs: Vec<([NodeId; 2], crate::corner_join::Join)>,
    /// Each brush as all its joins leave it.
    finals: Vec<(NodeId, map_model::brush::Brush)>,
    /// The pieces the joins added, each with the brushes it goes beside.
    additions: Vec<([NodeId; 2], map_model::brush::Brush)>,
    /// How many corners: pairs of selected brushes that overlap.
    corners: usize,
    /// Selected brushes that overlap no other.
    alone: usize,
    /// What [`CornerDraft::left`] says.
    left: Vec<String>,
}
impl Document {
    /// The first overlap among `pieces` (each with the corner, an index into `blocks`,
    /// that made or last cut it), or between one of them and an unselected brush near
    /// the selection that did not overlap the selected `originals` before: why, and
    /// what to mark. Two corners whose roundings meet are named with the brush between
    /// them.
    fn overlap_in(
        &self,
        pieces: &[(map_model::brush::Brush, Option<usize>)],
        selected: &[NodeId],
        originals: &[map_model::brush::Brush],
        blocks: &[Bbox3d],
    ) -> Option<(String, Bbox3d)> {
        const LIMIT: f64 = 1e-4;
        let at = |bounds: &Bbox3d| {
            let c = bounds.center();
            format!("({:.0} {:.0} {:.0})", c.x(), c.y(), c.z())
        };
        let bounds: Vec<Bbox3d> = pieces.iter().map(|(brush, _)| brush.bounds()).collect();
        for i in 0..pieces.len() {
            for j in i + 1..pieces.len() {
                if !bounds[i].intersects(&bounds[j])
                    || crate::corner_join::shared_volume(&pieces[i].0, &pieces[j].0) <= LIMIT
                {
                    continue;
                }
                let both = bounds[i].merged(&bounds[j]);
                let (Some(first), Some(mut second)) = (pieces[i].1, pieces[j].1) else {
                    return Some((
                        format!("At {}: the joined pieces would overlap.", at(&both)),
                        both,
                    ));
                };
                // Both last made by one corner: the other corner on a member of it that
                // is too short, when there is one, is what it ran into.
                if first == second {
                    let near = |b: &Bbox3d, block: &Bbox3d| b.expand(1e-3).intersects(block);
                    if let Some(other) = originals
                        .iter()
                        .map(|brush| brush.bounds())
                        .filter(|b| near(b, &blocks[first]) && near(b, &both))
                        .flat_map(|b| {
                            (0..blocks.len()).filter(move |k| *k != first && near(&b, &blocks[*k]))
                        })
                        .next()
                    {
                        second = other;
                    }
                }
                if first == second {
                    return Some((
                        format!(
                            "At {}: the pieces of this corner would overlap; try other rounding options.",
                            at(&blocks[first])
                        ),
                        blocks[first],
                    ));
                }
                // The member both corners are on: the original that holds both blocks.
                let member = originals
                    .iter()
                    .map(|brush| brush.bounds())
                    .filter(|b| {
                        b.expand(1e-3).intersects(&blocks[first])
                            && b.expand(1e-3).intersects(&blocks[second])
                    })
                    .min_by(|a, b| a.volume().total_cmp(&b.volume()))
                    .unwrap_or(both);
                return Some((
                    format!(
                        "The roundings of the corners at {} and {} would overlap along the brush at {}: it is too short between them for these radii.",
                        at(&blocks[first.min(second)]),
                        at(&blocks[first.max(second)]),
                        at(&member)
                    ),
                    member,
                ));
            }
        }
        let reach = bounds.iter().copied().reduce(|all, b| all.merged(&b))?;
        for (id, other) in node::brushes_with_ids(self.tree_world()) {
            if selected.contains(&id) || !other.bounds().intersects(&reach) {
                continue;
            }
            if originals
                .iter()
                .any(|o| crate::corner_join::shared_volume(o, other) > LIMIT)
            {
                continue;
            }
            for (index, (piece, _)) in pieces.iter().enumerate() {
                if bounds[index].intersects(&other.bounds())
                    && crate::corner_join::shared_volume(piece, other) > LIMIT
                {
                    let place = other.bounds();
                    return Some((
                        format!(
                            "At {}: the join would overlap the unselected brush at {}, such as the rounding of a corner already made at the other end.",
                            at(&bounds[index].merged(&place)),
                            at(&place)
                        ),
                        place,
                    ));
                }
            }
        }
        None
    }
}

/// An overlap refusal with what [`fit_note`] found: when snapping is the cause, the
/// member is long enough after all and the refusal says so.
fn compose(why: &str, fit: &str) -> String {
    if let Some(rest) = fit.strip_prefix(SNAPPED) {
        let why = why.replace(
            ": it is too short between them for these radii.",
            " once their new corners are snapped to the grid.",
        );
        format!("{why}{SNAPPED}{rest}")
    } else {
        format!("{why}{fit}")
    }
}

/// The start of [`fit_note`]'s answer when snapping is the cause.
const SNAPPED: &str = " Snapping";

/// What to add to an overlap refusal about rounding: that snapping causes it, when it
/// fits unsnapped; otherwise the largest radii that fit, found by shrinking every radius
/// in the same proportion until `fits` holds, or that none does. Empty without
/// rounding, where the overlap is not about radii.
fn fit_note(
    rounding: crate::corner_join::Rounding,
    fits: impl Fn(crate::corner_join::Rounding) -> bool,
) -> String {
    // Vertices snapped to the grid one by one can tip a thin wedge into its neighbour:
    // when the same rounding fits unsnapped, that is the cause, not the radius.
    if rounding.snap.is_some()
        && fits(crate::corner_join::Rounding {
            snap: None,
            ..rounding
        })
    {
        return format!(
            "{SNAPPED} the corner to the grid causes it; unsnapped it fits: use a finer grid, fewer segments, or no snapping."
        );
    }
    if rounding.outer.is_none() && rounding.inner.is_none() {
        return String::new();
    }
    let scaled = |t: f64| crate::corner_join::Rounding {
        outer: rounding.outer.map(|(r, n)| (r * t, n)),
        inner: rounding.inner.map(|(r, n)| (r * t, n)),
        ..rounding
    };
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..12 {
        let t = (lo + hi) / 2.0;
        if fits(scaled(t)) {
            lo = t;
        } else {
            hi = t;
        }
    }
    let floor = |r: f64| (r * lo * 100.0).floor() / 100.0;
    let radius = |r: Option<(f64, usize)>| r.map(|(r, _)| floor(r)).filter(|r| *r > 0.0);
    // Wedges and concentric rounding take their size from the inner radius alone.
    let inner_only = rounding.wedges || rounding.concentric;
    match (
        if inner_only {
            None
        } else {
            radius(rounding.outer)
        },
        radius(rounding.inner),
    ) {
        (Some(outer), Some(inner)) => {
            format!(" The largest radii that fit are outer {outer} and inner {inner}.")
        }
        (Some(r), None) | (None, Some(r)) => format!(" The largest radius that fits is {r}."),
        (None, None) => " No rounding fits here; join it without rounding.".to_string(),
    }
}

/// What a preview says about faces whose textures were left as they were.
fn texture_note(seams: crate::corner_join::TextureSeams) -> Option<String> {
    (seams.kept > 0).then(|| {
        format!(
            "{} joints between faces keep their own texture: the faces differ in material or scale, or a face with fixed (paraxial) axes cannot turn the corner.",
            seams.kept
        )
    })
}

/// The corners a Join corner over these brushes leaves alone: pairs of brushes that,
/// seen along the depth axis the preview leaves out, run along different axes, stand
/// side by side in depth and share no volume, and would make an L if one of them ran
/// on along its length to the other's end, which is no further than twice the thicker
/// one's thickness away: two walls of a room where one is drawn a little short or only
/// up to the other's face. Brushes that only touch are not joined: which member should
/// grow is the mapper's choice.
fn missed_corners(
    brushes: &[map_model::brush::Brush],
    at: &dyn Fn(&Bbox3d) -> String,
) -> Vec<String> {
    let mut left = Vec::new();
    let [u, v] = crate::corner_join::view_axes(brushes);
    let depth = 3 - u - v;
    let long = |b: &Bbox3d| {
        let size = b.size();
        if size[u] >= size[v] {
            u
        } else {
            v
        }
    };
    let thickness = |b: &Bbox3d| {
        let size = b.size();
        size[u].min(size[v])
    };
    // The rule reads bounds, which only say where a brush is for square ones.
    let square = |brush: &map_model::brush::Brush| {
        brush.faces().iter().all(|face| {
            let n = face.plane.normal;
            (0..3).filter(|axis| n[*axis].abs() > 1e-9).count() == 1
        })
    };
    for i in 0..brushes.len() {
        for j in i + 1..brushes.len() {
            if !square(&brushes[i]) || !square(&brushes[j]) {
                continue;
            }
            let (a, b) = (brushes[i].bounds(), brushes[j].bounds());
            let (la, lb) = (long(&a), long(&b));
            if la == lb {
                continue;
            }
            if a.max[depth].min(b.max[depth]) - a.min[depth].max(b.min[depth]) <= 1e-6 {
                continue;
            }
            if a.intersects(&b)
                && crate::corner_join::shared_volume(&brushes[i], &brushes[j]) > 1e-6
            {
                continue;
            }
            // The gap on each axis, and the box between the two.
            let mut gap = 0.0f64;
            let mut between = a;
            for axis in 0..3 {
                let (low, high) = (a.min[axis].max(b.min[axis]), a.max[axis].min(b.max[axis]));
                between.min[axis] = low.min(high);
                between.max[axis] = low.max(high);
                gap = gap.max(low - high);
            }
            if gap > 2.0 * thickness(&a).max(thickness(&b)) + 1e-6 {
                continue;
            }
            // `first` run on along `run` meets `second` at `second`'s end: an L, not a
            // wall that would hit another one's side.
            let ends = |first: &Bbox3d, run: usize, second: &Bbox3d| {
                let across = u + v - run;
                let overlap = first.max[across].min(second.max[across])
                    - first.min[across].max(second.min[across]);
                overlap > 1e-6
                    && [second.min[across], second.max[across]].iter().any(|end| {
                        first.min[across] - 1e-6 <= *end && *end <= first.max[across] + 1e-6
                    })
            };
            if !ends(&a, la, &b) && !ends(&b, lb, &a) {
                continue;
            }
            left.push(if gap <= 1e-6 {
                format!(
                    "At {}: the brushes only touch, so this corner is not joined. Make one overlap the other to join it.",
                    at(&between)
                )
            } else {
                format!(
                    "At {}: the brushes stop {} short of each other, so this corner is not joined.",
                    at(&between),
                    (gap * 1000.0).round() / 1000.0
                )
            });
        }
    }
    left
}

impl Document {
    pub fn preview_corner_join(&self) -> Result<CornerDraft, String> {
        self.preview_corner_join_with_rounding(None)
    }
    pub fn preview_corner_join_with_rounding(
        &self,
        rounding: Option<(f64, usize)>,
    ) -> Result<CornerDraft, String> {
        self.preview_corner_join_options(crate::corner_join::Rounding {
            outer: rounding,
            inner: None,
            concentric: false,
            wedges: false,
            preserve_thickness: false,
            snap: None,
            textures: false,
        })
    }
    pub fn preview_corner_join_options(
        &self,
        rounding: crate::corner_join::Rounding,
    ) -> Result<CornerDraft, String> {
        if self.selection.len() > 2 {
            let joins = self.joins_of_selection(rounding, &mut None)?;
            let corners = joins.corners;
            let before_all = self
                .selected_brushes()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();
            let finals: HashMap<NodeId, &map_model::brush::Brush> = joins
                .finals
                .iter()
                .map(|(id, brush)| (*id, brush))
                .collect();
            let mut ids: Vec<NodeId> = self.selection.iter().copied().collect();
            ids.sort();
            let mut after_all: Vec<map_model::brush::Brush> = ids
                .iter()
                .filter_map(|id| {
                    finals
                        .get(id)
                        .map(|brush| (*brush).clone())
                        .or_else(|| self.brush(*id).cloned())
                })
                .collect();
            after_all.extend(joins.additions.iter().map(|(_, brush)| brush.clone()));
            let axes = crate::corner_join::view_axes(&before_all);
            let left = joins.left;
            let (ids, join) = joins.pairs.into_iter().next().expect("at least one pair");
            return Ok(CornerDraft {
                revision: self.revision(),
                ids,
                join,
                corners,
                before_all,
                after_all,
                axes,
                left,
            });
        }
        self.pair_draft(rounding, &mut None, true)
    }

    /// Whether Join corner with `rounding` would go through, without working out what
    /// would fit when it would not: what the window's −/+ steps ask, eight at a time.
    pub fn corner_join_fits(&self, rounding: crate::corner_join::Rounding) -> bool {
        if self.selection.len() > 2 {
            self.raw_joins(rounding, &mut None, &mut false).is_ok()
        } else {
            self.pair_draft(rounding, &mut None, false).is_ok()
        }
    }

    /// The draft for two selected brushes. `place` is set when the refusal is about a
    /// place; `explain` works out the largest rounding that fits when the result would
    /// overlap.
    fn pair_draft(
        &self,
        rounding: crate::corner_join::Rounding,
        place: &mut Option<Bbox3d>,
        explain: bool,
    ) -> Result<CornerDraft, String> {
        if self.selection.len() != 2 {
            return Err("Select exactly two brushes.".into());
        }
        let mut ids = self.selection.iter().copied().collect::<Vec<_>>();
        ids.sort();
        if ids.iter().any(|id| !self.is_editable(*id)) {
            return Err("Both brushes must be editable.".into());
        }
        let a = self
            .brush(ids[0])
            .ok_or("Select brushes, not groups or entities.")?;
        let b = self
            .brush(ids[1])
            .ok_or("Select brushes, not groups or entities.")?;
        if rounding.inner.is_some() && self.corner_parent([ids[0], ids[1]]).is_none() {
            return Err(
                "For an inner fillet, both brushes must have the same immediate parent.".into(),
            );
        }
        let mut join = crate::corner_join::preview_with_rounding(a, b, rounding)?;
        // Nothing the join makes may overlap another piece of it, or a brush next to it
        // that did not overlap before, such as the wedges of a corner already rounded at
        // a member's other end.
        let pieces: Vec<(map_model::brush::Brush, Option<usize>)> = join
            .after
            .iter()
            .chain(&join.added)
            .map(|brush| (brush.clone(), Some(0)))
            .collect();
        let (aa, bb) = (a.bounds(), b.bounds());
        let mut block = aa;
        for axis in 0..3 {
            block.min[axis] = aa.min[axis].max(bb.min[axis]);
            block.max[axis] = aa.max[axis].min(bb.max[axis]).max(block.min[axis]);
        }
        if let Some((why, at)) = self.overlap_in(&pieces, &ids, &[a.clone(), b.clone()], &[block]) {
            *place = Some(at);
            let fit = if explain {
                fit_note(rounding, |smaller| {
                    crate::corner_join::preview_with_rounding(a, b, smaller).is_ok_and(|join| {
                        let pieces: Vec<_> = join
                            .after
                            .iter()
                            .chain(&join.added)
                            .map(|brush| (brush.clone(), Some(0)))
                            .collect();
                        self.overlap_in(&pieces, &ids, &[a.clone(), b.clone()], &[block])
                            .is_none()
                    })
                })
            } else {
                String::new()
            };
            return Err(compose(&why, &fit));
        }
        let mut left = Vec::new();
        if rounding.textures {
            let mut pieces: Vec<map_model::brush::Brush> =
                join.after.iter().chain(&join.added).cloned().collect();
            let seams = crate::corner_join::continue_textures(&mut pieces);
            left.extend(texture_note(seams));
            join.after = [pieces[0].clone(), pieces[1].clone()];
            join.added = pieces.split_off(2);
        }
        Ok(CornerDraft {
            revision: self.revision(),
            ids: [ids[0], ids[1]],
            before_all: join.before.to_vec(),
            after_all: join.after.iter().chain(&join.added).cloned().collect(),
            axes: join.axes,
            join,
            corners: 1,
            left,
        })
    }

    /// Why Join corner over a selection of more than two brushes is refused, with what a
    /// preview needs to show where; `None` when it is not refused or fewer are selected.
    pub fn corner_join_refusal(
        &self,
        rounding: crate::corner_join::Rounding,
    ) -> Option<CornerRefusal> {
        let mut place = None;
        let message = match self.selection.len() {
            0..=1 => return None,
            // Two brushes are refused in the window's status line, unless the refusal
            // is about a place, such as an overlap, which the preview then marks.
            2 => {
                let message = self.pair_draft(rounding, &mut place, true).err()?;
                place?;
                message
            }
            _ => self.joins_of_selection(rounding, &mut place).err()?,
        };
        let brushes: Vec<map_model::brush::Brush> =
            self.selected_brushes().into_iter().cloned().collect();
        let axes = crate::corner_join::view_axes(&brushes);
        Some(CornerRefusal {
            message,
            brushes,
            place,
            axes,
        })
    }

    /// Joins every pair of selected brushes that overlap, for a selection of more than
    /// two. Two brushes overlap where they share volume; each such pair is joined as the
    /// two-brush command joins it, one after the other on the brushes as the pairs
    /// before left them, which is the same whatever the order because each join changes
    /// only its own overlap. The whole command is refused, naming where, when two
    /// overlaps meet (three members in one corner) or a pair cannot be joined: half a
    /// frame is not what anyone asked for.
    /// `place` is set to the overlap that a refusal is about, when it is about one.
    fn joins_of_selection(
        &self,
        rounding: crate::corner_join::Rounding,
        place: &mut Option<Bbox3d>,
    ) -> Result<Joins, String> {
        let mut overlap = false;
        self.raw_joins(rounding, place, &mut overlap)
            .map_err(|why| {
                if !overlap {
                    return why;
                }
                let fit = fit_note(rounding, |smaller| {
                    self.raw_joins(
                        crate::corner_join::Rounding {
                            textures: false,
                            ..smaller
                        },
                        &mut None,
                        &mut false,
                    )
                    .is_ok()
                });
                compose(&why, &fit)
            })
    }

    /// [`Document::joins_of_selection`] without working out what would fit; `overlap`
    /// is set when the refusal is that the result would overlap.
    fn raw_joins(
        &self,
        rounding: crate::corner_join::Rounding,
        place: &mut Option<Bbox3d>,
        overlap: &mut bool,
    ) -> Result<Joins, String> {
        let mut ids: Vec<NodeId> = self.selection.iter().copied().collect();
        ids.sort();
        if ids.iter().any(|id| !self.is_editable(*id)) {
            return Err("All the brushes must be editable.".into());
        }
        let mut brushes: Vec<map_model::brush::Brush> = Vec::with_capacity(ids.len());
        for id in &ids {
            brushes.push(
                self.brush(*id)
                    .ok_or("Select brushes, not groups or entities.")?
                    .clone(),
            );
        }
        let at = |bounds: &Bbox3d| {
            let c = bounds.center();
            format!("({:.0} {:.0} {:.0})", c.x(), c.y(), c.z())
        };
        // The pairs that share volume, and where.
        let mut overlaps: Vec<(usize, usize, Bbox3d)> = Vec::new();
        for i in 0..ids.len() {
            for j in i + 1..ids.len() {
                let (a, b) = (brushes[i].bounds(), brushes[j].bounds());
                if !a.intersects(&b)
                    || crate::corner_join::shared_volume(&brushes[i], &brushes[j]) <= 1e-6
                {
                    continue;
                }
                // The bounds of what they share: for walls at an angle much less than
                // where their bounds overlap.
                let Some(shared) = crate::corner_join::shared_part(&brushes[i], &brushes[j]) else {
                    continue;
                };
                overlaps.push((i, j, shared.bounds()));
            }
        }
        if overlaps.is_empty() {
            return Err("None of the selected brushes overlap another.".into());
        }
        let mut left = missed_corners(&brushes, &at);
        for (n, (_, _, first)) in overlaps.iter().enumerate() {
            for (_, _, second) in &overlaps[n + 1..] {
                let shared = (0..3).all(|axis| {
                    first.max[axis].min(second.max[axis]) - first.min[axis].max(second.min[axis])
                        > 1e-6
                });
                if shared {
                    *place = Some(first.merged(second));
                    return Err(format!(
                        "The overlaps at {} and {} meet: more than two brushes share one corner. Join them two at a time.",
                        at(first),
                        at(second)
                    ));
                }
            }
        }
        // Every piece there is so far: the brushes, and what crossings cut off them, each
        // with the brush it belongs to, whether it is that brush itself, and what an
        // added piece goes beside.
        struct Piece {
            owner: usize,
            brush: map_model::brush::Brush,
            original: bool,
            beside: [NodeId; 2],
            /// Split off a taller member above or below a shorter one.
            layer: bool,
            /// The overlap that made or last cut it.
            corner: Option<usize>,
        }
        let mut pieces: Vec<Piece> = brushes
            .iter()
            .cloned()
            .enumerate()
            .map(|(owner, brush)| Piece {
                owner,
                brush,
                original: true,
                beside: [ids[owner]; 2],
                layer: false,
                corner: None,
            })
            .collect();
        let mut pairs = Vec::with_capacity(overlaps.len());
        for (n, (i, j, block)) in overlaps.iter().enumerate() {
            // The pieces of each brush that hold this overlap: a brush cut by an earlier
            // crossing meets the next one with one of its pieces only; a brush split by
            // height meets it with a piece in each layer, joined one after the other.
            let mut first = true;
            loop {
                let holding: Vec<(usize, usize)> = (0..pieces.len())
                    .filter(|p| pieces[*p].owner == *i)
                    .flat_map(|p| {
                        (0..pieces.len())
                            .filter(|q| pieces[*q].owner == *j)
                            .map(move |q| (p, q))
                    })
                    .filter(|(p, q)| {
                        crate::corner_join::shared_volume(&pieces[*p].brush, &pieces[*q].brush)
                            > 1e-6
                    })
                    .collect();
                let layered = holding
                    .iter()
                    .any(|(p, q)| pieces[*p].layer || pieces[*q].layer);
                let (p, q) = match holding[..] {
                    [] if !first => break,
                    [one] => one,
                    [one, ..] if layered && pairs.len() < 64 * overlaps.len() => one,
                    _ => {
                        *place = Some(*block);
                        return Err(format!(
                            "At {}: the brushes overlap in more than one place.",
                            at(block)
                        ));
                    }
                };
                first = false;
                let join = crate::corner_join::preview_with_rounding(
                    &pieces[p].brush,
                    &pieces[q].brush,
                    rounding,
                )
                .map_err(|why| {
                    *place = Some(*block);
                    format!("At {}: {why}", at(block))
                })?;
                pieces[p].brush = join.after[0].clone();
                pieces[q].brush = join.after[1].clone();
                pieces[p].corner = Some(n);
                pieces[q].corner = Some(n);
                let (owner, beside) = match join.split {
                    Some(0) => (*i, [ids[*i]; 2]),
                    Some(_) => (*j, [ids[*j]; 2]),
                    None => (*i, [ids[*i], ids[*j]]),
                };
                for (k, brush) in join.added.iter().enumerate() {
                    let (owner, beside, layer) = match join.owner_of(k) {
                        Some(0) => (*i, [ids[*i]; 2], true),
                        Some(_) => (*j, [ids[*j]; 2], true),
                        None => (owner, beside, false),
                    };
                    pieces.push(Piece {
                        owner,
                        brush: brush.clone(),
                        original: false,
                        beside,
                        layer,
                        corner: Some(n),
                    });
                }
                pairs.push(([ids[*i], ids[*j]], join));
            }
        }
        // Nothing may overlap: two corners' roundings running into each other along a
        // short member, or a rounding into a brush next to the selection.
        let made: Vec<(map_model::brush::Brush, Option<usize>)> = pieces
            .iter()
            .map(|piece| (piece.brush.clone(), piece.corner))
            .collect();
        let blocks: Vec<Bbox3d> = overlaps.iter().map(|(_, _, block)| *block).collect();
        if let Some((why, at)) = self.overlap_in(&made, &ids, &brushes, &blocks) {
            *place = Some(at);
            *overlap = true;
            return Err(why);
        }
        if rounding.textures {
            // Over every piece at once, so a room's walls carry one texture round.
            let mut all: Vec<map_model::brush::Brush> =
                pieces.iter().map(|piece| piece.brush.clone()).collect();
            let seams = crate::corner_join::continue_textures(&mut all);
            left.extend(texture_note(seams));
            for (piece, brush) in pieces.iter_mut().zip(all) {
                piece.brush = brush;
            }
        }
        let joined: HashSet<usize> = overlaps.iter().flat_map(|(i, j, _)| [*i, *j]).collect();
        let (originals, added): (Vec<Piece>, Vec<Piece>) =
            pieces.into_iter().partition(|piece| piece.original);
        Ok(Joins {
            pairs,
            finals: originals
                .into_iter()
                .filter(|piece| joined.contains(&piece.owner))
                .map(|piece| (ids[piece.owner], piece.brush))
                .collect(),
            additions: added
                .into_iter()
                .map(|piece| (piece.beside, piece.brush))
                .collect(),
            corners: overlaps.len(),
            alone: ids.len() - joined.len(),
            left,
        })
    }

    /// [`Document::join_corner_options`] for more than two brushes: every join of
    /// [`Document::joins_of_selection`] as one undo step. Pieces a join adds go beside
    /// the brush they were cut from, or under the parent the pair shares.
    fn join_corners(&mut self, rounding: crate::corner_join::Rounding) -> Result<(), String> {
        let joins = self.joins_of_selection(rounding, &mut None)?;
        for (_, join) in &joins.pairs {
            if let Some(note) = join.note {
                crate::say!("{note}");
            }
        }
        let additions = joins.additions;
        let finals: HashMap<NodeId, map_model::brush::Brush> = joins.finals.into_iter().collect();
        // Where each addition goes: a group or entity, changed whole with its new
        // children, or a layer.
        let mut into_parents: Vec<(NodeId, Vec<Node>)> = Vec::new();
        let mut into_layers: Vec<(usize, Vec<Node>)> = Vec::new();
        let mut added_ids = Vec::new();
        for (beside, brush) in additions {
            let (layer, parent) = self
                .corner_parent(beside)
                .ok_or("Both brushes must have the same immediate parent.")?;
            let node = Node::new(NodeKind::Brush(brush));
            added_ids.push(node.id);
            match parent {
                Some(parent) => match into_parents.iter_mut().find(|(p, _)| *p == parent) {
                    Some((_, nodes)) => nodes.push(node),
                    None => into_parents.push((parent, vec![node])),
                },
                None => match into_layers.iter_mut().find(|(l, _)| *l == layer) {
                    Some((_, nodes)) => nodes.push(node),
                    None => into_layers.push((layer, vec![node])),
                },
            }
        }
        let mut parts = Vec::new();
        let mut handled = HashSet::new();
        for (parent, nodes) in into_parents {
            let original = node::find_node(&self.tree.world, parent)
                .ok_or("Parent no longer exists.")?
                .clone();
            let mut updated = original.clone();
            let children = match &mut updated.kind {
                NodeKind::Group(group) => &mut group.children,
                NodeKind::Entity(entity) => &mut entity.children,
                _ => return Err("Invalid corner parent.".into()),
            };
            for child in children.iter_mut() {
                if let Some(brush) = finals.get(&child.id) {
                    *child = brush_node(child.id, brush.clone());
                    handled.insert(child.id);
                }
            }
            children.extend(nodes);
            parts.push(Edit::Changed {
                before: vec![original],
                after: vec![updated],
            });
        }
        let mut before = Vec::new();
        let mut after = Vec::new();
        for (id, brush) in &finals {
            if handled.contains(id) {
                continue;
            }
            before.push(brush_node(
                *id,
                self.brush(*id).ok_or("A brush no longer exists.")?.clone(),
            ));
            after.push(brush_node(*id, brush.clone()));
        }
        if !after.is_empty() {
            parts.push(Edit::Changed { before, after });
        }
        for (layer, nodes) in into_layers {
            parts.push(Edit::Added { layer, nodes });
        }
        self.apply_and_record(Edit::Batch(parts));
        self.selection.extend(added_ids);
        if joins.alone > 0 {
            crate::say!(
                "{} {}",
                crate::i18n::counted(joins.alone, "brush"),
                "overlap no other selected brush and were left as they were"
            );
        }
        for note in &joins.left {
            crate::say!("{note}");
        }
        Ok(())
    }
    pub fn join_corner(&mut self, expected_revision: u64) -> Result<(), String> {
        self.join_corner_with_rounding(expected_revision, None)
    }
    pub fn join_corner_with_rounding(
        &mut self,
        expected_revision: u64,
        rounding: Option<(f64, usize)>,
    ) -> Result<(), String> {
        self.join_corner_options(
            expected_revision,
            crate::corner_join::Rounding {
                outer: rounding,
                inner: None,
                concentric: false,
                wedges: false,
                preserve_thickness: false,
                snap: None,
                textures: false,
            },
        )
    }
    pub fn join_corner_options(
        &mut self,
        expected_revision: u64,
        rounding: crate::corner_join::Rounding,
    ) -> Result<(), String> {
        if self.in_transaction() {
            return Err("Finish the current drag first.".into());
        }
        if expected_revision != self.revision() {
            return Err("The map or selection changed. Open a new corner preview.".into());
        }
        if self.selection.len() > 2 {
            return self.join_corners(rounding);
        }
        let draft = self.preview_corner_join_options(rounding)?;
        // Members of different heights leave parts of their own: placed beside their
        // member, as the several-brush command places what it adds.
        if draft.join.owners.iter().any(Option::is_some) {
            return self.join_corners(rounding);
        }
        if let Some(note) = draft.join.note {
            crate::say!("{note}");
        }
        // A crossing's second piece goes beside the brush it was cut from; inner
        // rounding's pieces under the parent both brushes share.
        let beside = match draft.join.split {
            Some(index) => [draft.ids[index]; 2],
            None => draft.ids,
        };
        let before: Vec<Node> = draft
            .ids
            .iter()
            .zip(draft.join.before)
            .map(|(id, b)| brush_node(*id, b))
            .collect();
        let after: Vec<Node> = draft
            .ids
            .iter()
            .zip(draft.join.after)
            .map(|(id, b)| brush_node(*id, b))
            .collect();
        let additions: Vec<_> = draft
            .join
            .added
            .into_iter()
            .map(|brush| Node::new(NodeKind::Brush(brush)))
            .collect();
        let added_ids: Vec<_> = additions.iter().map(|node| node.id).collect();
        let edit = if additions.is_empty() {
            Edit::Changed { before, after }
        } else {
            let (layer, parent) = self
                .corner_parent(beside)
                .ok_or("Both brushes must have the same immediate parent.")?;
            if let Some(parent) = parent {
                let original = node::find_node(&self.tree.world, parent)
                    .ok_or("Parent no longer exists.")?
                    .clone();
                let mut updated = original.clone();
                let children = match &mut updated.kind {
                    NodeKind::Group(group) => &mut group.children,
                    NodeKind::Entity(entity) => &mut entity.children,
                    NodeKind::Layer(layer) => &mut layer.children,
                    _ => return Err("Invalid corner parent.".into()),
                };
                // The other brush of a crossing may sit elsewhere; it changes in place.
                let mut elsewhere = Vec::new();
                let mut elsewhere_before = Vec::new();
                for (replacement, was) in after.into_iter().zip(before) {
                    match children.iter_mut().find(|node| node.id == replacement.id) {
                        Some(child) => *child = replacement,
                        None => {
                            elsewhere.push(replacement);
                            elsewhere_before.push(was);
                        }
                    }
                }
                children.extend(additions);
                let mut parts = vec![Edit::Changed {
                    before: vec![original],
                    after: vec![updated],
                }];
                if !elsewhere.is_empty() {
                    parts.push(Edit::Changed {
                        before: elsewhere_before,
                        after: elsewhere,
                    });
                }
                if parts.len() == 1 {
                    parts.pop().expect("one part")
                } else {
                    Edit::Batch(parts)
                }
            } else {
                Edit::Batch(vec![
                    Edit::Changed { before, after },
                    Edit::Added {
                        layer,
                        nodes: additions,
                    },
                ])
            }
        };
        self.apply_and_record(edit);
        self.selection.extend(added_ids);
        Ok(())
    }

    pub(super) fn corner_parent(&self, ids: [NodeId; 2]) -> Option<(usize, Option<NodeId>)> {
        fn search(
            children: &[Node],
            parent: Option<NodeId>,
            ids: [NodeId; 2],
        ) -> Option<Option<NodeId>> {
            if ids
                .iter()
                .all(|id| children.iter().any(|node| node.id == *id))
            {
                return Some(parent);
            }
            for node in children {
                let children = match &node.kind {
                    NodeKind::Group(group) => &group.children,
                    NodeKind::Entity(entity) => &entity.children,
                    NodeKind::Layer(layer) => &layer.children,
                    _ => continue,
                };
                if let Some(found) = search(children, Some(node.id), ids) {
                    return Some(found);
                }
            }
            None
        }
        self.tree
            .world
            .layers
            .iter()
            .enumerate()
            .find_map(|(layer, contents)| {
                search(&contents.children, None, ids).map(|parent| (layer, parent))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corner_join_preserves_ids_parent_selection_and_undo() {
        let mut d = Document::new(Default::default());
        let a = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &Bbox3d {
                min: Vec3d::zero(),
                max: Vec3d::new(128.0, 16.0, 8.0),
            },
            "stone",
        )
        .unwrap();
        let b = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &Bbox3d {
                min: Vec3d::zero(),
                max: Vec3d::new(24.0, 96.0, 8.0),
            },
            "wood",
        )
        .unwrap();
        d.add_brushes(vec![a, b]);
        d.select_all();
        let parent = d.create_brush_entity("func_detail").unwrap();
        let before = d.to_map_text();
        let selected = d.selection_ids().clone();
        let revision = d.revision();
        let draft = d.preview_corner_join().unwrap();
        assert_eq!(before, d.to_map_text());
        assert_eq!(revision, d.revision());
        d.join_corner_with_rounding(draft.revision, Some((8.0, 8)))
            .unwrap();
        assert_eq!(d.selection_ids(), &selected);
        assert!(selected
            .iter()
            .all(|id| node::contents_of(d.tree_world(), parent).contains(id)));
        let after = d.to_map_text();
        assert_ne!(before, after);
        assert!(d.join_corner(revision).is_err());
        assert_eq!(after, d.to_map_text());
        d.undo();
        assert_eq!(before, d.to_map_text());
        d.redo();
        assert_eq!(after, d.to_map_text());
    }
}

#[cfg(test)]
mod inner_tests {
    use super::*;
    fn document(wu: f64, wv: f64) -> Document {
        let mut d = Document::new(Default::default());
        let bounds = d.world_bounds();
        let brushes = [Vec3d::new(128.0, wv, 8.0), Vec3d::new(wu, 96.0, 8.0)].map(|max| {
            map_model::shapes::create_cuboid(
                &bounds,
                &Bbox3d {
                    min: Vec3d::zero(),
                    max,
                },
                "stone",
            )
            .unwrap()
        });
        d.add_brushes(brushes.into());
        d
    }
    #[test]
    fn inner_and_wedge_additions_keep_parent_ids_and_one_undo_in_layers_and_entities() {
        for nested in [false, true] {
            for (wedges, preserve_thickness) in [(false, false), (true, false), (true, true)] {
                let mut d = if wedges {
                    document(24.0, 8.0)
                } else {
                    document(16.0, 16.0)
                };
                let parent = nested.then(|| d.create_brush_entity("func_detail").unwrap());
                let before = d.to_map_text();
                let original = d.selection_ids().clone();
                let revision = d.revision();
                let options = crate::corner_join::Rounding {
                    outer: Some((8.0, 8)),
                    inner: Some((48.0, 8)),
                    concentric: !wedges,
                    wedges,
                    preserve_thickness,
                    snap: None,
                    textures: false,
                };
                let preview = d.preview_corner_join_options(options).unwrap();
                assert_eq!(preview.join.added.len(), 8);
                assert_eq!(before, d.to_map_text());
                assert_eq!(d.revision(), revision);
                d.join_corner_options(revision, options).unwrap();
                assert_eq!(node::brushes(d.tree_world()).len(), 10);
                assert!(original.is_subset(d.selection_ids()));
                if let Some(parent) = parent {
                    let NodeKind::Entity(entity) =
                        &node::find_node(d.tree_world(), parent).unwrap().kind
                    else {
                        panic!("expected entity");
                    };
                    assert_eq!(entity.children.len(), 10);
                }
                let after = d.to_map_text();
                let reopened = Document::from_map_owned(
                    read_map(&after, None).unwrap(),
                    Default::default(),
                    None,
                );
                assert_eq!(node::brushes(reopened.tree_world()).len(), 10);
                let volume = |document: &Document| {
                    node::brushes(document.tree_world())
                        .iter()
                        .map(|brush| brush.volume())
                        .sum::<f64>()
                };
                assert!((volume(&d) - volume(&reopened)).abs() < 1e-5);
                for (before, after) in node::brushes(d.tree_world())
                    .iter()
                    .zip(node::brushes(reopened.tree_world()))
                {
                    for face in before.faces() {
                        let saved = after
                            .faces()
                            .iter()
                            .find(|f| {
                                vec::length(f.normal() - face.normal()) < 1e-5
                                    && (f.plane.distance - face.plane.distance).abs() < 1e-4
                            })
                            .unwrap();
                        for point in face.points {
                            let delta = face.uv_coords(point) - saved.uv_coords(point);
                            assert!(
                                delta.x().abs() < 0.001 && delta.y().abs() < 0.001,
                                "saved UV changed"
                            );
                        }
                    }
                }
                assert!(d.join_corner_options(revision, options).is_err());
                assert_eq!(d.to_map_text(), after);
                d.undo();
                assert_eq!(d.to_map_text(), before);
                d.redo();
                assert_eq!(d.to_map_text(), after);
            }
        }
    }
    #[test]
    fn inner_fillets_reject_brushes_in_different_parents_without_edits() {
        let mut d = document(16.0, 16.0);
        let ids: Vec<_> = d.selection_ids().iter().copied().collect();
        d.selection = HashSet::from([ids[0]]);
        d.create_brush_entity("func_detail").unwrap();
        d.selection = HashSet::from([ids[1]]);
        d.create_brush_entity("func_detail").unwrap();
        d.selection = ids.into_iter().collect();
        let before = d.to_map_text();
        let revision = d.revision();
        let options = crate::corner_join::Rounding {
            inner: Some((8.0, 8)),
            ..Default::default()
        };
        assert!(d.preview_corner_join_options(options).is_err());
        assert!(d.join_corner_options(revision, options).is_err());
        assert_eq!(before, d.to_map_text());
        assert_eq!(revision, d.revision());
    }
}

#[cfg(test)]
mod crossing_tests {
    use super::*;
    #[test]
    fn a_crossing_is_one_undo_step_and_the_piece_stays_in_the_cut_brushs_entity() {
        let mut d = Document::new(Default::default());
        let cube = |min: Vec3d, max: Vec3d| {
            map_model::shapes::create_cuboid(&d.world_bounds(), &Bbox3d { min, max }, "stone")
                .unwrap()
        };
        let long = cube(Vec3d::new(-128.0, -8.0, 0.0), Vec3d::new(128.0, 8.0, 16.0));
        let short = cube(Vec3d::new(-8.0, -64.0, 0.0), Vec3d::new(8.0, 64.0, 16.0));
        d.add_brushes(vec![long]);
        d.add_brushes(vec![short]);
        let ids: Vec<NodeId> = node::brushes_with_ids(d.tree_world())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        // The short wall is a func_wall of its own.
        d.select_only(ids[1]);
        let wall = d.create_brush_entity("func_wall").unwrap();
        d.select_only(ids[0]);
        d.toggle_selected(ids[1]);
        let before = d.to_map_text();
        let draft = d.preview_corner_join().unwrap();
        d.join_corner(draft.revision).unwrap();
        assert_eq!(node::brushes(d.tree_world()).len(), 3);
        let NodeKind::Entity(entity) = &node::find_node(d.tree_world(), wall).unwrap().kind else {
            panic!("the func_wall")
        };
        assert_eq!(entity.children.len(), 2, "both pieces of the cut wall");
        d.undo();
        assert_eq!(d.to_map_text(), before);
    }
}

#[cfg(test)]
mod skip_export_tests {
    use super::*;
    #[test]
    fn an_exported_join_leaves_out_the_mitre_faces() {
        let mut d = Document::new(Default::default());
        for max in [Vec3d::new(128.0, 16.0, 8.0), Vec3d::new(16.0, 96.0, 8.0)] {
            let brush = map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &Bbox3d {
                    min: Vec3d::zero(),
                    max,
                },
                "stone",
            )
            .unwrap();
            d.add_brushes(vec![brush]);
        }
        d.select_all();
        let draft = d.preview_corner_join().unwrap();
        d.join_corner(draft.revision).unwrap();
        let shown: usize = node::brushes(d.tree_world())
            .iter()
            .flat_map(|b| {
                (0..b.faces().len())
                    .filter(|i| b.faces()[*i].material_name.as_str() != "skip")
                    .map(|i| b.face_polygon(i).map_or(0, |p| p.len() - 2))
                    .collect::<Vec<_>>()
            })
            .sum();
        let hidden = node::brushes(d.tree_world())
            .iter()
            .flat_map(|b| b.faces().iter())
            .filter(|f| f.material_name.as_str() == "skip")
            .count();
        assert_eq!(hidden, 2);
        let options = crate::export::Options {
            target: crate::export::Target::Godot,
            name: "corner".into(),
            cull_hidden: false,
            merge_faces: false,
            ..Default::default()
        };
        let directory = std::env::temp_dir().join(format!("heron-skip-{}", std::process::id()));
        let (report, _) = crate::export::export_files(&d, &directory, &options).unwrap();
        assert_eq!(
            report.triangles, shown,
            "the two skip faces are not exported"
        );
    }
}

#[cfg(test)]
mod many_tests {
    use super::*;

    /// A document of axis-aligned boxes, all selected, and their ids in order.
    fn boxes(list: &[([f64; 3], [f64; 3])]) -> (Document, Vec<NodeId>) {
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
        let ids = node::brushes_with_ids(d.tree_world())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        d.select_all();
        (d, ids)
    }

    /// The owner's gate: two posts and a beam over them.
    const GATE: [([f64; 3], [f64; 3]); 3] = [
        ([-128.0, -16.0, 0.0], [-112.0, 0.0, 144.0]),
        ([80.0, -16.0, 0.0], [96.0, 0.0, 144.0]),
        ([-128.0, -16.0, 128.0], [96.0, 0.0, 144.0]),
    ];

    fn join_all(d: &mut Document) {
        let draft = d.preview_corner_join().unwrap();
        d.join_corner(draft.revision).unwrap();
    }

    fn join_pair(d: &mut Document, a: NodeId, b: NodeId) {
        d.select_only(a);
        d.toggle_selected(b);
        join_all(d);
    }

    #[test]
    fn a_gate_joins_both_corners_at_once_as_one_after_the_other_would() {
        let (mut all, ids) = boxes(&GATE);
        let before = all.to_map_text();
        assert_eq!(all.preview_corner_join().unwrap().corners, 2);
        join_all(&mut all);
        for order in [[0, 1], [1, 0]] {
            let (mut by_hand, _) = boxes(&GATE);
            // The same ids: each document mints its own, so compare geometry instead.
            let hand_ids: Vec<NodeId> = node::brushes_with_ids(by_hand.tree_world())
                .into_iter()
                .map(|(id, _)| id)
                .collect();
            for post in order {
                join_pair(&mut by_hand, hand_ids[post], hand_ids[2]);
            }
            let faces = |d: &Document| -> Vec<Vec<(String, Vec<[i64; 3]>)>> {
                node::brushes(d.tree_world())
                    .iter()
                    .map(|b| {
                        let mut faces: Vec<(String, Vec<[i64; 3]>)> = (0..b.faces().len())
                            .map(|i| {
                                let mut corners: Vec<[i64; 3]> = b
                                    .face_polygon(i)
                                    .unwrap()
                                    .iter()
                                    .map(|p| [0, 1, 2].map(|a| (p[a] * 1000.0).round() as i64))
                                    .collect();
                                corners.sort();
                                (b.faces()[i].material_name.as_str().to_string(), corners)
                            })
                            .collect();
                        faces.sort();
                        faces
                    })
                    .collect()
            };
            assert_eq!(faces(&all), faces(&by_hand), "order {order:?}");
        }
        // Four mitre faces, all skip.
        let hidden = node::brushes(all.tree_world())
            .iter()
            .flat_map(|b| b.faces().iter())
            .filter(|f| f.material_name.as_str() == "skip")
            .count();
        assert_eq!(hidden, 4);
        all.undo();
        assert_eq!(all.to_map_text(), before, "one undo step");
        let _ = ids;
    }

    #[test]
    fn a_frame_with_a_middle_post_makes_two_corners_and_a_tee() {
        let (mut d, _) = boxes(&[
            ([0.0, 0.0, 0.0], [16.0, 16.0, 144.0]),
            ([240.0, 0.0, 0.0], [256.0, 16.0, 144.0]),
            ([0.0, 0.0, 128.0], [256.0, 16.0, 144.0]),
            ([120.0, 0.0, 0.0], [136.0, 16.0, 136.0]),
        ]);
        assert_eq!(d.preview_corner_join().unwrap().corners, 3);
        join_all(&mut d);
        let brushes = node::brushes(d.tree_world());
        assert_eq!(brushes.len(), 4);
        // The middle post now ends flush under the beam.
        assert!(brushes
            .iter()
            .any(|b| (b.bounds().min.x() - 120.0).abs() < 1e-6
                && (b.bounds().max.z() - 128.0).abs() < 1e-6));
        for (i, a) in brushes.iter().enumerate() {
            for b in &brushes[i + 1..] {
                assert!(crate::corner_join::shared_volume(a, b) < 1e-6);
            }
        }
    }

    #[test]
    fn a_window_frame_joins_four_corners_and_a_grid_of_crossings_cuts_through() {
        let (mut window, _) = boxes(&[
            ([0.0, 0.0, 0.0], [16.0, 16.0, 128.0]),
            ([112.0, 0.0, 0.0], [128.0, 16.0, 128.0]),
            ([0.0, 0.0, 0.0], [128.0, 16.0, 16.0]),
            ([0.0, 0.0, 112.0], [128.0, 16.0, 128.0]),
        ]);
        assert_eq!(window.preview_corner_join().unwrap().corners, 4);
        join_all(&mut window);
        assert_eq!(node::brushes(window.tree_world()).len(), 4);

        // Two rails across two longer stiles: each rail is cut where it crosses.
        let (mut grid, _) = boxes(&[
            ([0.0, 32.0, 0.0], [16.0, 48.0, 256.0]),
            ([0.0, 160.0, 0.0], [16.0, 176.0, 256.0]),
            ([0.0, 0.0, 64.0], [16.0, 208.0, 80.0]),
            ([0.0, 0.0, 160.0], [16.0, 208.0, 176.0]),
        ]);
        assert_eq!(grid.preview_corner_join().unwrap().corners, 4);
        let before = grid.to_map_text();
        join_all(&mut grid);
        let brushes = node::brushes(grid.tree_world());
        assert_eq!(brushes.len(), 2 + 2 * 3, "each rail in three pieces");
        for (i, a) in brushes.iter().enumerate() {
            for b in &brushes[i + 1..] {
                assert!(crate::corner_join::shared_volume(a, b) < 1e-6);
            }
        }
        grid.undo();
        assert_eq!(grid.to_map_text(), before);
    }

    #[test]
    fn three_members_in_one_corner_are_refused_and_nothing_changes() {
        let (mut d, _) = boxes(&[
            ([0.0, 0.0, 0.0], [128.0, 16.0, 16.0]),
            ([0.0, 0.0, 0.0], [16.0, 128.0, 16.0]),
            ([0.0, 0.0, 0.0], [16.0, 16.0, 128.0]),
        ]);
        let before = d.to_map_text();
        let error = d.preview_corner_join().err().unwrap();
        assert!(error.contains("meet"), "{error}");
        assert!(d.join_corner(d.revision()).is_err());
        assert_eq!(d.to_map_text(), before);
        // A pair the two-brush command refuses refuses the whole command.
        let (d, _) = boxes(&[
            ([0.0, 0.0, 0.0], [128.0, 16.0, 16.0]),
            ([0.0, 0.0, 0.0], [128.0, 16.0, 16.0]),
            ([500.0, 0.0, 0.0], [516.0, 16.0, 16.0]),
        ]);
        let before = d.to_map_text();
        assert!(d.preview_corner_join().is_err());
        assert_eq!(d.to_map_text(), before);
    }
}

#[cfg(test)]
mod skip_rule_tests {
    use super::*;

    fn joined(list: &[([f64; 3], [f64; 3])], rounding: crate::corner_join::Rounding) -> Document {
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
        d.select_all();
        let draft = d.preview_corner_join_options(rounding).unwrap();
        d.join_corner_options(draft.revision, rounding).unwrap();
        d
    }

    fn check(d: &Document) {
        let brushes: Vec<map_model::brush::Brush> =
            node::brushes(d.tree_world()).into_iter().cloned().collect();
        crate::corner_join::assert_skip_only_where_hidden(&brushes, &[]);
    }

    /// The owner's gate with both inside corners filleted, radius 8 in 8 segments, as
    /// their saved map was: the arc faces of the second corner came out skip.
    #[test]
    fn a_gate_filleted_at_both_corners_keeps_its_arcs_visible() {
        let gate = [
            ([-128.0, -16.0, 0.0], [-112.0, 0.0, 144.0]),
            ([80.0, -16.0, 0.0], [96.0, 0.0, 144.0]),
            ([-128.0, -16.0, 128.0], [96.0, 0.0, 144.0]),
        ];
        let fillet = crate::corner_join::Rounding {
            inner: Some((8.0, 8)),
            ..Default::default()
        };
        let d = joined(&gate, fillet);
        assert_eq!(node::brushes(d.tree_world()).len(), 3 + 16);
        check(&d);
        // Every wedge shows its arc face.
        for brush in node::brushes(d.tree_world()).into_iter().skip(3) {
            let shown = brush
                .faces()
                .iter()
                .filter(|f| f.plane.normal.y().abs() < 0.5 && f.material_name.as_str() != "skip")
                .count();
            assert_eq!(shown, 1, "the arc face of each wedge");
        }
        check(&joined(&gate, crate::corner_join::Rounding::default()));
        check(&joined(
            &gate,
            crate::corner_join::Rounding {
                outer: Some((8.0, 4)),
                ..Default::default()
            },
        ));
    }

    #[test]
    fn several_joins_put_skip_only_on_hidden_faces() {
        let plain = crate::corner_join::Rounding::default();
        check(&joined(
            &[
                ([0.0, 0.0, 0.0], [16.0, 16.0, 144.0]),
                ([240.0, 0.0, 0.0], [256.0, 16.0, 144.0]),
                ([0.0, 0.0, 128.0], [256.0, 16.0, 144.0]),
                ([120.0, 0.0, 0.0], [136.0, 16.0, 136.0]),
            ],
            plain,
        ));
        check(&joined(
            &[
                ([0.0, 0.0, 0.0], [16.0, 16.0, 128.0]),
                ([112.0, 0.0, 0.0], [128.0, 16.0, 128.0]),
                ([0.0, 0.0, 0.0], [128.0, 16.0, 16.0]),
                ([0.0, 0.0, 112.0], [128.0, 16.0, 128.0]),
            ],
            plain,
        ));
        check(&joined(
            &[
                ([0.0, 32.0, 0.0], [16.0, 48.0, 256.0]),
                ([0.0, 160.0, 0.0], [16.0, 176.0, 256.0]),
                ([0.0, 0.0, 64.0], [16.0, 208.0, 80.0]),
                ([0.0, 0.0, 160.0], [16.0, 208.0, 176.0]),
            ],
            plain,
        ));
    }
}

#[cfg(test)]
mod room_tests {
    use super::*;
    use crate::corner_join::Rounding;

    type Box3 = ([f64; 3], [f64; 3]);

    /// A room seen from above: walls `t` thick (west, east, south, north) along the
    /// inside of the outline from `min` to `max`, each running the full outer length so
    /// every corner is a block where two walls overlap, as the owner drew theirs.
    fn room(min: [f64; 2], max: [f64; 2], t: [f64; 4], height: f64) -> Vec<Box3> {
        vec![
            ([min[0], min[1], 0.0], [min[0] + t[0], max[1], height]),
            ([max[0] - t[1], min[1], 0.0], [max[0], max[1], height]),
            ([min[0], min[1], 0.0], [max[0], min[1] + t[2], height]),
            ([min[0], max[1] - t[3], 0.0], [max[0], max[1], height]),
        ]
    }

    /// The owner's four_walls.map.
    fn owners_room() -> Vec<Box3> {
        room([-144.0, -144.0], [144.0, 144.0], [16.0; 4], 128.0)
    }

    fn document(walls: &[Box3]) -> Document {
        let mut d = Document::new(Default::default());
        for (min, max) in walls {
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
        d.select_all();
        d
    }

    /// Joins the whole selection as the window's Apply does, checks it is one undo
    /// step, and returns the joined document and the preview it came from.
    fn join(walls: &[Box3], rounding: Rounding) -> (Document, CornerDraft) {
        let mut d = document(walls);
        let before = d.to_map_text();
        let draft = d.preview_corner_join_options(rounding).unwrap();
        d.join_corner_options(draft.revision, rounding).unwrap();
        let after = d.to_map_text();
        d.undo();
        assert_eq!(d.to_map_text(), before, "one undo step");
        d.redo();
        assert_eq!(d.to_map_text(), after);
        (d, draft)
    }

    fn brushes(d: &Document) -> Vec<map_model::brush::Brush> {
        node::brushes(d.tree_world()).into_iter().cloned().collect()
    }

    /// No two pieces share volume, and skip is on hidden faces only.
    fn sound(d: &Document) {
        let all = brushes(d);
        for (i, a) in all.iter().enumerate() {
            assert!(a.volume() > 1e-6, "brush {i} is a solid");
            for b in &all[i + 1..] {
                assert!(crate::corner_join::shared_volume(a, b) < 1e-6);
            }
        }
        crate::corner_join::assert_skip_only_where_hidden(&all, &[]);
    }

    /// A plainly mitred room: four walls filling exactly the outline's ring, each
    /// mitred at both ends, flush inside and out, the eight mitre faces skip.
    fn mitred_room(d: &Document, min: [f64; 2], max: [f64; 2], t: [f64; 4], height: f64) {
        sound(d);
        let all = brushes(d);
        assert_eq!(all.len(), 4);
        let ring = ((max[0] - min[0]) * (max[1] - min[1])
            - (max[0] - min[0] - t[0] - t[1]) * (max[1] - min[1] - t[2] - t[3]))
            * height;
        let total: f64 = all.iter().map(|b| b.volume()).sum();
        assert!((total - ring).abs() < 1e-3, "{total} {ring}");
        for b in &all {
            let bounds = b.bounds();
            assert!(bounds.min.x() >= min[0] - 1e-6 && bounds.max.x() <= max[0] + 1e-6);
            assert!(bounds.min.y() >= min[1] - 1e-6 && bounds.max.y() <= max[1] + 1e-6);
            assert_eq!(b.faces().len(), 6, "a wall stays a six-sided brush");
            let mitres: Vec<_> = b
                .faces()
                .iter()
                .filter(|f| f.plane.normal.x().abs() > 1e-6 && f.plane.normal.y().abs() > 1e-6)
                .collect();
            assert_eq!(mitres.len(), 2, "mitred at both ends");
            assert!(mitres.iter().all(|f| f.material_name.as_str() == "skip"));
            assert_eq!(
                b.faces()
                    .iter()
                    .filter(|f| f.material_name.as_str() == "skip")
                    .count(),
                2
            );
        }
    }

    fn faces(d: &Document) -> Vec<Vec<(String, Vec<[i64; 3]>)>> {
        let mut out: Vec<_> = brushes(d)
            .iter()
            .map(|b| {
                let mut faces: Vec<(String, Vec<[i64; 3]>)> = (0..b.faces().len())
                    .map(|i| {
                        let mut corners: Vec<[i64; 3]> = b
                            .face_polygon(i)
                            .unwrap()
                            .iter()
                            .map(|p| [0, 1, 2].map(|a| (p[a] * 1000.0).round() as i64))
                            .collect();
                        corners.sort();
                        (b.faces()[i].material_name.as_str().to_string(), corners)
                    })
                    .collect();
                faces.sort();
                faces
            })
            .collect();
        out.sort();
        out
    }

    fn roundings() -> [Rounding; 4] {
        [
            Rounding::default(),
            Rounding {
                inner: Some((8.0, 4)),
                ..Default::default()
            },
            Rounding {
                outer: Some((6.0, 3)),
                ..Default::default()
            },
            Rounding {
                outer: Some((6.0, 3)),
                inner: Some((8.0, 4)),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn the_owners_room_gets_four_mitred_corners_seen_from_above() {
        let (d, draft) = join(&owners_room(), Rounding::default());
        assert_eq!(draft.corners, 4, "four walls, four corners");
        assert_eq!(draft.axes, [0, 1], "seen from above");
        assert_eq!(draft.before_all.len(), 4);
        assert_eq!(draft.after_all.len(), 4);
        assert!(draft.left.is_empty());
        mitred_room(&d, [-144.0, -144.0], [144.0, 144.0], [16.0; 4], 128.0);
    }

    #[test]
    fn a_room_joins_the_same_whatever_order_its_walls_were_drawn_in() {
        let walls = owners_room();
        for rounding in roundings() {
            let (first, draft) = join(&walls, rounding);
            assert_eq!(draft.corners, 4);
            sound(&first);
            let reference = faces(&first);
            for order in [[3, 2, 1, 0], [1, 3, 0, 2], [2, 0, 3, 1], [0, 3, 1, 2]] {
                let shuffled: Vec<Box3> = order.iter().map(|i| walls[*i]).collect();
                let (d, draft) = join(&shuffled, rounding);
                assert_eq!(draft.corners, 4);
                assert_eq!(faces(&d), reference, "order {order:?}");
            }
        }
    }

    #[test]
    fn a_rounded_room_closes_its_loop() {
        for rounding in roundings().into_iter().skip(1) {
            let (d, draft) = join(&owners_room(), rounding);
            assert_eq!(draft.corners, 4);
            sound(&d);
            // Inner fillets add wedges in all four corners, the same in each.
            let added = brushes(&d).len() - 4;
            assert_eq!(added % 4, 0, "{added}");
        }
    }

    #[test]
    fn rectangular_rooms_and_walls_of_different_thickness_join() {
        for (min, max, t) in [
            ([0.0, 0.0], [512.0, 256.0], [16.0; 4]),
            ([-64.0, -32.0], [320.0, 96.0], [8.0, 16.0, 24.0, 32.0]),
            ([0.0, 0.0], [256.0, 256.0], [32.0, 8.0, 16.0, 16.0]),
        ] {
            let walls = room(min, max, t, 96.0);
            let (d, draft) = join(&walls, Rounding::default());
            assert_eq!(draft.corners, 4);
            assert_eq!(draft.axes, [0, 1]);
            mitred_room(&d, min, max, t, 96.0);
            for rounding in roundings().into_iter().skip(1) {
                let (d, draft) = join(&walls, rounding);
                assert_eq!(draft.corners, 4);
                sound(&d);
            }
        }
    }

    #[test]
    fn walls_that_stop_short_are_left_and_named() {
        let mut walls = owners_room();
        // The north wall stops 24 short of the east wall.
        walls[3].1[0] = 104.0;
        let (d, draft) = join(&walls, Rounding::default());
        assert_eq!(draft.corners, 3);
        assert_eq!(draft.left.len(), 1, "{:?}", draft.left);
        assert!(
            draft.left[0].contains("(116 136 64)") && draft.left[0].contains("24"),
            "{}",
            draft.left[0]
        );
        sound(&d);
        assert_eq!(brushes(&d).len(), 4);
        // Both short: the north wall at both ends. Two corners joined, two named.
        let mut walls = owners_room();
        walls[3].0[0] = -112.0;
        walls[3].1[0] = 112.0;
        let (d, draft) = join(&walls, Rounding::default());
        assert_eq!(draft.corners, 2);
        assert_eq!(draft.left.len(), 2, "{:?}", draft.left);
        sound(&d);
    }

    /// The owner's ten_brushes.map as they corrected it: a room with an annex on its
    /// north side, one loop of ten walls, every corner an overlap.
    fn owners_ten_walls() -> Vec<Box3> {
        vec![
            ([-144.0, -144.0, 0.0], [-128.0, 144.0, 128.0]),
            ([128.0, -144.0, 0.0], [144.0, 144.0, 128.0]),
            ([-144.0, -144.0, 0.0], [144.0, -128.0, 128.0]),
            ([32.0, 128.0, 0.0], [144.0, 144.0, 128.0]),
            ([-144.0, 128.0, 0.0], [-80.0, 144.0, 128.0]),
            ([-96.0, 128.0, 0.0], [-80.0, 256.0, 128.0]),
            ([-96.0, 240.0, 0.0], [128.0, 256.0, 128.0]),
            ([112.0, 176.0, 0.0], [128.0, 256.0, 128.0]),
            ([32.0, 176.0, 0.0], [128.0, 192.0, 128.0]),
            ([32.0, 128.0, 0.0], [48.0, 192.0, 128.0]),
        ]
    }

    /// The area the boxes cover seen from above, overlaps counted once.
    fn plan_area(boxes: &[Box3]) -> f64 {
        let mut xs: Vec<f64> = boxes.iter().flat_map(|b| [b.0[0], b.1[0]]).collect();
        let mut ys: Vec<f64> = boxes.iter().flat_map(|b| [b.0[1], b.1[1]]).collect();
        xs.sort_by(f64::total_cmp);
        ys.sort_by(f64::total_cmp);
        let mut area = 0.0;
        for x in xs.windows(2) {
            for y in ys.windows(2) {
                let (cx, cy) = ((x[0] + x[1]) / 2.0, (y[0] + y[1]) / 2.0);
                if boxes
                    .iter()
                    .any(|b| b.0[0] < cx && cx < b.1[0] && b.0[1] < cy && cy < b.1[1])
                {
                    area += (x[1] - x[0]) * (y[1] - y[0]);
                }
            }
        }
        area
    }

    #[test]
    fn the_owners_ten_walls_make_ten_mitred_corners() {
        let walls = owners_ten_walls();
        let (d, draft) = join(&walls, Rounding::default());
        assert_eq!(draft.corners, 10);
        assert_eq!(draft.axes, [0, 1]);
        assert_eq!(draft.after_all.len(), 10);
        assert!(draft.left.is_empty(), "{:?}", draft.left);
        sound(&d);
        let all = brushes(&d);
        assert_eq!(all.len(), 10);
        let total: f64 = all.iter().map(|b| b.volume()).sum();
        assert!((total - plan_area(&walls) * 128.0).abs() < 1e-3);
        for b in &all {
            assert_eq!(b.faces().len(), 6);
            let mitres: Vec<_> = b
                .faces()
                .iter()
                .filter(|f| f.plane.normal.x().abs() > 1e-6 && f.plane.normal.y().abs() > 1e-6)
                .collect();
            assert_eq!(mitres.len(), 2, "mitred at both ends");
            assert!(mitres.iter().all(|f| f.material_name.as_str() == "skip"));
        }
        for rounding in roundings().into_iter().skip(1) {
            let (d, draft) = join(&walls, rounding);
            assert_eq!(draft.corners, 10);
            sound(&d);
        }
        // Drawn in another order, the same result.
        let reference = faces(&d);
        let mut reversed = walls.clone();
        reversed.reverse();
        assert_eq!(faces(&join(&reversed, Rounding::default()).0), reference);
    }

    #[test]
    fn the_ten_walls_as_first_drawn_join_nine_and_name_the_touching_corner() {
        // Before the owner's correction the north-west wall started on the west
        // wall's inner face: the two only touch.
        let mut walls = owners_ten_walls();
        walls[4].0[0] = -128.0;
        let (d, draft) = join(&walls, Rounding::default());
        assert_eq!(draft.corners, 9);
        assert_eq!(draft.left.len(), 1, "{:?}", draft.left);
        assert!(
            draft.left[0].contains("(-128 136 64)") && draft.left[0].contains("only touch"),
            "{}",
            draft.left[0]
        );
        assert_eq!(brushes(&d).len(), 10);
        // Sound, the touching end face of the north-west wall left as it was: hidden
        // against the west wall, but not a face the join made.
        let all = brushes(&d);
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert!(crate::corner_join::shared_volume(a, b) < 1e-6);
            }
        }
        let end: Vec<_> = all
            .iter()
            .flat_map(|b| b.faces())
            .filter(|f| {
                f.plane.normal.x() < -0.999
                    && (f.plane.point_distance(Vec3d::new(-128.0, 136.0, 64.0))).abs() < 1e-6
            })
            .map(|f| f.plane)
            .collect();
        assert_eq!(end.len(), 1);
        assert_ne!(
            all.iter()
                .flat_map(|b| b.faces())
                .find(|f| f.plane.normal.x() < -0.999
                    && f.plane
                        .point_distance(Vec3d::new(-128.0, 136.0, 64.0))
                        .abs()
                        < 1e-6)
                .unwrap()
                .material_name
                .as_str(),
            "skip",
            "the touching end is not the join's"
        );
        crate::corner_join::assert_skip_only_where_hidden(&all, &end);
    }

    #[test]
    fn a_wall_with_a_doorway_joins_both_its_pieces() {
        let mut walls = owners_room();
        // The south wall in two pieces with a 64 wide doorway between.
        let south = walls.remove(2);
        walls.push((south.0, [-32.0, south.1[1], south.1[2]]));
        walls.push(([32.0, south.0[1], south.0[2]], south.1));
        let (d, draft) = join(&walls, Rounding::default());
        assert_eq!(draft.corners, 4);
        assert!(draft.left.is_empty(), "a doorway is not a missed corner");
        sound(&d);
        assert_eq!(brushes(&d).len(), 5);
        let total: f64 = brushes(&d).iter().map(|b| b.volume()).sum();
        let ring = (288.0 * 288.0 - 256.0 * 256.0) * 128.0 - 64.0 * 16.0 * 128.0;
        assert!((total - ring).abs() < 1e-3, "{total} {ring}");
    }
}

#[cfg(test)]
mod texture_tests {
    use super::*;
    use crate::corner_join::Rounding;
    use geometry_math::vec::Vec2f;
    use map_model::uv::{parallel_axes, UvCoordSystem};

    type Box3 = ([f64; 3], [f64; 3]);

    /// Brushes with a Valve 220 mapping each: brick at scale (0.5, 0.5), turned 30
    /// degrees, every brush with its own offset so nothing lines up by chance.
    fn textured(list: &[Box3], material: impl Fn(usize) -> &'static str) -> Document {
        let mut d = Document::new(Default::default());
        for (n, (min, max)) in list.iter().enumerate() {
            let brush = map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &Bbox3d {
                    min: Vec3d::new(min[0], min[1], min[2]),
                    max: Vec3d::new(max[0], max[1], max[2]),
                },
                material(n),
            )
            .unwrap();
            let mut faces = brush.faces().to_vec();
            for face in &mut faces {
                let (u_axis, v_axis) = parallel_axes(face.normal());
                face.uv_coord_system = UvCoordSystem::Parallel { u_axis, v_axis };
                face.uv_attributes.rotation = 0.0;
                face.rotate_uv(30.0);
                face.uv_attributes.scale = Vec2f::new(0.5, 0.5);
                face.uv_attributes.offset = Vec2f::new(13.0 * n as f32 + 5.0, -7.0 * n as f32);
            }
            let brush = map_model::brush::Brush::from_faces(&d.world_bounds(), faces).unwrap();
            d.add_brushes(vec![brush]);
        }
        d.select_all();
        d
    }

    fn joined(list: &[Box3], rounding: Rounding) -> (Document, CornerDraft) {
        let mut d = textured(list, |_| "brick");
        let draft = d.preview_corner_join_options(rounding).unwrap();
        d.join_corner_options(draft.revision, rounding).unwrap();
        (d, draft)
    }

    fn on() -> Rounding {
        Rounding {
            textures: true,
            ..Default::default()
        }
    }

    /// The texture coordinate at `p` from each visible face facing `normal` that `p`
    /// lies on.
    fn uv_at(d: &Document, normal: [f64; 3], p: Vec3d) -> Vec<Vec2f> {
        let normal = Vec3d::new(normal[0], normal[1], normal[2]);
        let mut out = Vec::new();
        for b in node::brushes(d.tree_world()) {
            for (i, face) in b.faces().iter().enumerate() {
                if face.material_name.as_str() == "skip"
                    || vec::length(face.normal() - normal) > 1e-6
                    || face.plane.point_distance(p).abs() > 1e-6
                {
                    continue;
                }
                let polygon = b.face_polygon(i).unwrap();
                if crate::corner_join::polygon_within(&[p], &polygon, face.normal()) {
                    out.push(face.uv_coords(p));
                }
            }
        }
        out
    }

    /// Both faces give the same texture coordinate at both ends of the edge.
    fn seamless(d: &Document, from: [f64; 3], to: [f64; 3], n1: [f64; 3], n2: [f64; 3]) -> bool {
        [from, to].iter().all(|p| {
            let p = Vec3d::new(p[0], p[1], p[2]);
            let (a, b) = (uv_at(d, n1, p), uv_at(d, n2, p));
            assert!(!a.is_empty() && !b.is_empty(), "faces at {p:?}");
            a.iter()
                .chain(&b)
                .all(|uv| (uv.x() - a[0].x()).abs() < 1e-2 && (uv.y() - a[0].y()).abs() < 1e-2)
        })
    }

    const L: [Box3; 2] = [
        ([0.0, 0.0, 0.0], [128.0, 16.0, 64.0]),
        ([0.0, 0.0, 0.0], [16.0, 128.0, 64.0]),
    ];

    #[test]
    fn an_l_carries_its_texture_round_both_sides_and_across_the_top() {
        let (d, draft) = joined(&L, on());
        assert!(draft.left.is_empty(), "{:?}", draft.left);
        // Outside, round the corner; inside, into it; on top and below, across the mitre.
        assert!(seamless(
            &d,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 64.0],
            [0.0, -1.0, 0.0],
            [-1.0, 0.0, 0.0]
        ));
        assert!(seamless(
            &d,
            [16.0, 16.0, 0.0],
            [16.0, 16.0, 64.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0]
        ));
        assert!(seamless(
            &d,
            [0.0, 0.0, 64.0],
            [16.0, 16.0, 64.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0]
        ));
        assert!(seamless(
            &d,
            [0.0, 0.0, 0.0],
            [16.0, 16.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0]
        ));
        // The scale is the user's, and the first wall keeps its mapping.
        for b in node::brushes(d.tree_world()) {
            for face in b.faces() {
                assert_eq!(face.uv_attributes.scale, Vec2f::new(0.5, 0.5));
            }
        }
        let first = textured(&L, |_| "brick");
        let before = node::brushes(first.tree_world())[0]
            .faces()
            .iter()
            .find(|f| f.normal().y() < -0.5)
            .unwrap()
            .clone();
        let after = node::brushes(d.tree_world())[0]
            .faces()
            .iter()
            .find(|f| f.normal().y() < -0.5)
            .unwrap()
            .clone();
        assert_eq!(before.uv_attributes, after.uv_attributes);
        assert_eq!(before.uv_coord_system, after.uv_coord_system);
        // Off, as before: the corner jumps.
        let (old, _) = joined(&L, Rounding::default());
        assert!(!seamless(
            &old,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 64.0],
            [0.0, -1.0, 0.0],
            [-1.0, 0.0, 0.0]
        ));
    }

    #[test]
    fn a_tee_and_a_crossing_carry_the_texture_into_their_inside_corners() {
        let tee = [
            ([0.0, 0.0, 0.0], [128.0, 16.0, 64.0]),
            ([56.0, 8.0, 0.0], [72.0, 96.0, 64.0]),
        ];
        let (d, _) = joined(&tee, on());
        assert!(seamless(
            &d,
            [56.0, 16.0, 0.0],
            [56.0, 16.0, 64.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0]
        ));
        assert!(seamless(
            &d,
            [72.0, 16.0, 0.0],
            [72.0, 16.0, 64.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0]
        ));
        assert!(seamless(
            &d,
            [56.0, 16.0, 64.0],
            [72.0, 16.0, 64.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0]
        ));
        let crossing = [
            ([0.0, 56.0, 0.0], [128.0, 72.0, 64.0]),
            ([56.0, 0.0, 0.0], [72.0, 160.0, 64.0]),
        ];
        let (d, _) = joined(&crossing, on());
        for (x, y, nx, ny) in [
            (56.0, 56.0, -1.0, -1.0),
            (72.0, 56.0, 1.0, -1.0),
            (56.0, 72.0, -1.0, 1.0),
            (72.0, 72.0, 1.0, 1.0),
        ] {
            assert!(
                seamless(
                    &d,
                    [x, y, 0.0],
                    [x, y, 64.0],
                    [0.0, ny, 0.0],
                    [nx, 0.0, 0.0]
                ),
                "({x} {y})"
            );
        }
    }

    #[test]
    fn a_room_carries_its_texture_round_with_one_seam_where_the_loop_closes() {
        let room: Vec<Box3> = vec![
            ([-144.0, -144.0, 0.0], [-128.0, 144.0, 128.0]),
            ([128.0, -144.0, 0.0], [144.0, 144.0, 128.0]),
            ([-144.0, -144.0, 0.0], [144.0, -128.0, 128.0]),
            ([-144.0, 128.0, 0.0], [144.0, 144.0, 128.0]),
        ];
        let (d, draft) = joined(&room, on());
        assert_eq!(draft.corners, 4);
        let outside = [
            (-144.0, -144.0, -1.0, -1.0),
            (144.0, -144.0, 1.0, -1.0),
            (-144.0, 144.0, -1.0, 1.0),
            (144.0, 144.0, 1.0, 1.0),
        ];
        let inside = [
            (-128.0, -128.0, 1.0, 1.0),
            (128.0, -128.0, -1.0, 1.0),
            (-128.0, 128.0, 1.0, -1.0),
            (128.0, 128.0, -1.0, -1.0),
        ];
        for corners in [outside, inside] {
            let joined = corners
                .iter()
                .filter(|(x, y, nx, ny)| {
                    seamless(
                        &d,
                        [*x, *y, 0.0],
                        [*x, *y, 128.0],
                        [0.0, *ny, 0.0],
                        [*nx, 0.0, 0.0],
                    )
                })
                .count();
            assert_eq!(joined, 3, "one seam where the loop closes");
        }
        // The top runs on across every mitre.
        for (x, y, _, _) in outside {
            let (ix, iy) = (x - 16.0 * x.signum(), y - 16.0 * y.signum());
            assert!(seamless(
                &d,
                [x, y, 128.0],
                [ix, iy, 128.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0]
            ));
        }
    }

    #[test]
    fn a_rounded_corner_wraps_the_texture_across_its_wedges() {
        for rounding in [
            Rounding {
                inner: Some((8.0, 4)),
                textures: true,
                ..Default::default()
            },
            Rounding {
                outer: Some((12.0, 4)),
                textures: true,
                ..Default::default()
            },
        ] {
            let (d, _) = joined(&L, rounding);
            // Every vertical edge where two visible faces meet carries one texture
            // coordinate: between pieces, and on one piece between an arc facet and
            // what it turns from (a plain box's own edges are not a wrap).
            let brushes: Vec<_> = node::brushes(d.tree_world()).into_iter().cloned().collect();
            let mut checked = 0;
            for (i, a) in brushes.iter().enumerate() {
                for (j, b) in brushes.iter().enumerate().skip(i) {
                    for (fa, face_a) in a.faces().iter().enumerate() {
                        for (fb, face_b) in b.faces().iter().enumerate() {
                            let slanted =
                                |n: Vec3d| (0..3).filter(|k| n[*k].abs() > 1e-6).count() > 1;
                            if (i == j
                                && (fb <= fa
                                    || !(slanted(face_a.normal()) || slanted(face_b.normal()))))
                                || face_a.material_name.as_str() == "skip"
                                || face_b.material_name.as_str() == "skip"
                                || vec::dot(face_a.normal(), face_b.normal()) < -0.5
                            {
                                continue;
                            }
                            let (pa, pb) =
                                (a.face_polygon(fa).unwrap(), b.face_polygon(fb).unwrap());
                            // The corners of either outline lying on the other face.
                            let on = |p: &Vec3d,
                                      face: &map_model::brush_face::BrushFace,
                                      polygon: &[Vec3d]| {
                                face.plane.point_distance(*p).abs() < 1e-4
                                    && crate::corner_join::polygon_within(
                                        &[*p],
                                        polygon,
                                        face.normal(),
                                    )
                            };
                            let mut common: Vec<Vec3d> = pa
                                .iter()
                                .filter(|p| on(p, face_b, &pb))
                                .chain(pb.iter().filter(|p| on(p, face_a, &pa)))
                                .copied()
                                .collect();
                            common.dedup_by(|p, q| vec::length(*p - *q) < 1e-6);
                            common.sort_by(|p, q| p.z().total_cmp(&q.z()));
                            if common.len() > 2 {
                                common = vec![common[0], common[common.len() - 1]];
                            }
                            if common.len() != 2 || (common[0].z() - common[1].z()).abs() < 1.0 {
                                continue;
                            }
                            // In the open there, not behind a wedge.
                            let beside = (common[0] + common[1]) * 0.5
                                + (face_a.normal() + face_b.normal()) * 0.01;
                            if brushes.iter().any(|b| b.contains_point(beside)) {
                                continue;
                            }
                            for p in &common {
                                let (u, v) = (face_a.uv_coords(*p), face_b.uv_coords(*p));
                                assert!(
                                    (u.x() - v.x()).abs() < 1e-2 && (u.y() - v.y()).abs() < 1e-2,
                                    "pieces {i} and {j} at {p:?}"
                                );
                            }
                            checked += 1;
                        }
                    }
                }
            }
            assert!(checked >= 5, "{checked}");
        }
    }

    #[test]
    fn paraxial_faces_line_up_their_offsets_round_an_upright_corner() {
        // The Standard format's mapping, and the owner's maps, which mix the two.
        for mixed in [false, true] {
            let mut d = textured(&L, |_| "brick");
            let ids: Vec<NodeId> = node::brushes_with_ids(d.tree_world())
                .into_iter()
                .map(|(id, _)| id)
                .collect();
            for (n, id) in ids.iter().enumerate() {
                if mixed && n == 0 {
                    continue;
                }
                let brush = d.brush(*id).unwrap().clone();
                let mut faces = brush.faces().to_vec();
                for face in &mut faces {
                    face.uv_coord_system = UvCoordSystem::Paraxial;
                    face.uv_attributes.rotation = 0.0;
                }
                let brush = map_model::brush::Brush::from_faces(&d.world_bounds(), faces).unwrap();
                d.select_only(*id);
                d.delete_selection();
                d.add_brushes(vec![brush]);
            }
            d.select_all();
            let draft = d.preview_corner_join_options(on()).unwrap();
            d.join_corner_options(draft.revision, on()).unwrap();
            assert!(seamless(
                &d,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 64.0],
                [0.0, -1.0, 0.0],
                [-1.0, 0.0, 0.0]
            ));
            assert!(seamless(
                &d,
                [16.0, 16.0, 0.0],
                [16.0, 16.0, 64.0],
                [0.0, 1.0, 0.0],
                [1.0, 0.0, 0.0]
            ));
            assert!(seamless(
                &d,
                [0.0, 0.0, 64.0],
                [16.0, 16.0, 64.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0]
            ));
            assert!(draft.left.is_empty(), "{:?}", draft.left);
        }
    }

    #[test]
    fn different_materials_keep_their_own_texture_and_the_window_says_so() {
        let mut d = textured(&L, |n| if n == 0 { "brick" } else { "wood" });
        let draft = d.preview_corner_join_options(on()).unwrap();
        assert_eq!(draft.left.len(), 1, "{:?}", draft.left);
        assert!(draft.left[0].contains("differ in material or scale"));
        d.join_corner_options(draft.revision, on()).unwrap();
        let old = {
            let mut d = textured(&L, |n| if n == 0 { "brick" } else { "wood" });
            let draft = d.preview_corner_join_options(Rounding::default()).unwrap();
            d.join_corner_options(draft.revision, Rounding::default())
                .unwrap();
            d.to_map_text()
        };
        let without_ids = |text: &str| {
            text.lines()
                .filter(|line| !line.starts_with("// heron_brush id"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(
            without_ids(&d.to_map_text()),
            without_ids(&old),
            "nothing to carry: as with the option off"
        );
    }
}

#[cfg(test)]
mod height_tests {
    use super::*;
    use crate::corner_join::Rounding;

    type Box3 = ([f64; 3], [f64; 3]);

    fn document(list: &[Box3]) -> Document {
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
        d.select_all();
        d
    }

    fn brushes(d: &Document) -> Vec<map_model::brush::Brush> {
        node::brushes(d.tree_world()).into_iter().cloned().collect()
    }

    /// Joins everything as the window's Apply does; checks one undo step, no overlap
    /// left, skip only on hidden faces, and, unrounded, the same volume as before less
    /// the overlaps.
    fn joined(list: &[Box3], rounding: Rounding) -> (Document, CornerDraft) {
        let mut d = document(list);
        let before = d.to_map_text();
        let originals = brushes(&d);
        let draft = d.preview_corner_join_options(rounding).unwrap();
        d.join_corner_options(draft.revision, rounding).unwrap();
        let all = brushes(&d);
        for (i, a) in all.iter().enumerate() {
            assert!(a.volume() > 1e-6);
            for b in &all[i + 1..] {
                assert!(crate::corner_join::shared_volume(a, b) < 1e-6);
            }
        }
        crate::corner_join::assert_skip_only_where_hidden(&all, &[]);
        if rounding.inner.is_none() && rounding.outer.is_none() {
            let mut union: f64 = originals.iter().map(|b| b.volume()).sum();
            for (i, a) in originals.iter().enumerate() {
                for b in &originals[i + 1..] {
                    union -= crate::corner_join::shared_volume(a, b);
                }
            }
            let total: f64 = all.iter().map(|b| b.volume()).sum();
            assert!((total - union).abs() < 1e-3, "{total} {union}");
        }
        let after = d.to_map_text();
        d.undo();
        assert_eq!(d.to_map_text(), before, "one undo step");
        d.redo();
        assert_eq!(d.to_map_text(), after);
        (d, draft)
    }

    /// The pieces whose bounds span `z` in depth, by their lowest and highest z.
    fn spans(d: &Document) -> Vec<(i64, i64)> {
        let mut out: Vec<(i64, i64)> = brushes(d)
            .iter()
            .map(|b| {
                (
                    b.bounds().min.z().round() as i64,
                    b.bounds().max.z().round() as i64,
                )
            })
            .collect();
        out.sort();
        out
    }

    fn mitred(b: &map_model::brush::Brush) -> bool {
        b.faces()
            .iter()
            .any(|f| f.plane.normal.x().abs() > 1e-6 && f.plane.normal.y().abs() > 1e-6)
    }

    const LOW: [f64; 2] = [0.0, 96.0];

    fn l(a: [f64; 2], b: [f64; 2]) -> [Box3; 2] {
        [
            ([0.0, 0.0, a[0]], [128.0, 16.0, a[1]]),
            ([0.0, 0.0, b[0]], [16.0, 128.0, b[1]]),
        ]
    }

    #[test]
    fn equal_heights_join_as_before() {
        let (d, draft) = joined(&l([0.0, 128.0], [0.0, 128.0]), Rounding::default());
        assert_eq!(draft.corners, 1);
        assert_eq!(spans(&d), vec![(0, 128), (0, 128)]);
        assert!(brushes(&d).iter().all(mitred));
    }

    #[test]
    fn a_lower_wall_is_mitred_over_its_height_and_the_higher_keeps_its_top() {
        for (a, b) in [([0.0, 128.0], LOW), (LOW, [0.0, 128.0])] {
            let (d, draft) = joined(&l(a, b), Rounding::default());
            assert_eq!(draft.corners, 1);
            assert_eq!(spans(&d), vec![(0, 96), (0, 96), (96, 128)]);
            for brush in brushes(&d) {
                let top = brush.bounds().min.z() > 95.0;
                // The two lower parts mitred, the top of the higher wall untouched with
                // its full end.
                assert_eq!(mitred(&brush), !top, "{:?}", brush.bounds());
            }
            // The cut between the higher wall's parts is hidden both ways; the lower
            // wall's top stays visible, as most of it is open.
            let skip_flat = brushes(&d)
                .iter()
                .flat_map(|b| b.faces().iter())
                .filter(|f| f.plane.normal.z().abs() > 0.999 && f.material_name.as_str() == "skip")
                .count();
            assert_eq!(skip_flat, 2);
        }
    }

    #[test]
    fn offset_walls_join_over_the_height_they_share() {
        let (d, draft) = joined(&l([0.0, 128.0], [32.0, 160.0]), Rounding::default());
        assert_eq!(draft.corners, 1);
        assert_eq!(spans(&d), vec![(0, 32), (32, 128), (32, 128), (128, 160)]);
    }

    #[test]
    fn tees_and_crossings_of_different_heights() {
        // A higher stem against a lower bar: the stem's lower part cut flush, its top
        // left running over the bar.
        let (d, _) = joined(
            &[
                ([0.0, 0.0, 0.0], [128.0, 16.0, 96.0]),
                ([56.0, 8.0, 0.0], [72.0, 96.0, 128.0]),
            ],
            Rounding::default(),
        );
        assert_eq!(spans(&d), vec![(0, 96), (0, 96), (96, 128)]);
        // A lower stem against a higher bar: cut flush, the bar whole, as before.
        let (d, _) = joined(
            &[
                ([0.0, 0.0, 0.0], [128.0, 16.0, 128.0]),
                ([56.0, 8.0, 0.0], [72.0, 96.0, 96.0]),
            ],
            Rounding::default(),
        );
        assert_eq!(spans(&d), vec![(0, 96), (0, 128)]);
        // Crossings: the higher member passes through whole, the lower is cut in two,
        // whichever is longer.
        for (lower, higher) in [
            (
                ([0.0, 56.0, 0.0], [128.0, 72.0, 96.0]),
                ([56.0, 0.0, 0.0], [72.0, 160.0, 128.0]),
            ),
            (
                ([0.0, 56.0, 0.0], [256.0, 72.0, 96.0]),
                ([56.0, 0.0, 0.0], [72.0, 160.0, 128.0]),
            ),
        ] {
            let (d, _) = joined(&[lower, higher], Rounding::default());
            assert_eq!(spans(&d), vec![(0, 96), (0, 96), (0, 128)]);
        }
    }

    #[test]
    fn a_room_with_one_lower_wall_closes_its_loop() {
        let room = [
            ([-144.0, -144.0, 0.0], [-128.0, 144.0, 128.0]),
            ([128.0, -144.0, 0.0], [144.0, 144.0, 128.0]),
            ([-144.0, -144.0, 0.0], [144.0, -128.0, 128.0]),
            ([-144.0, 128.0, 0.0], [144.0, 144.0, 96.0]),
        ];
        for rounding in [
            Rounding::default(),
            Rounding {
                inner: Some((8.0, 4)),
                ..Default::default()
            },
        ] {
            let (d, draft) = joined(&room, rounding);
            assert_eq!(draft.corners, 4);
            if rounding.inner.is_none() {
                // West and east split at 96; every lower part mitred at both ends.
                assert_eq!(brushes(&d).len(), 6);
            }
        }
    }

    #[test]
    fn the_upper_part_stays_in_the_walls_group() {
        let mut d = document(&l([0.0, 128.0], LOW));
        let group = d.group_selection("walls").unwrap();
        d.select_all();
        let draft = d.preview_corner_join().unwrap();
        d.join_corner(draft.revision).unwrap();
        let Some(Node {
            kind: NodeKind::Group(walls),
            ..
        }) = node::find_node(d.tree_world(), group)
        else {
            panic!("the group is gone");
        };
        let in_group = walls
            .children
            .iter()
            .filter(|child| matches!(child.kind, NodeKind::Brush(_)))
            .count();
        assert_eq!(in_group, 3);
    }
}

#[cfg(test)]
mod overlap_tests {
    use super::*;
    use crate::corner_join::Rounding;

    fn owners_ten_walls() -> Document {
        let path = format!(
            "{}/../heron-brush/tests/data/ten_brushes.map",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut d = Document::open(path, Default::default()).unwrap();
        d.select_all();
        d
    }

    /// The settings in the owner's screenshot.
    fn owners_settings() -> Rounding {
        Rounding {
            outer: Some((8.0, 8)),
            inner: Some((13.0, 9)),
            concentric: false,
            wedges: true,
            preserve_thickness: true,
            snap: Some(1.0),
            textures: true,
        }
    }

    fn no_overlap(brushes: &[map_model::brush::Brush]) {
        for (i, a) in brushes.iter().enumerate() {
            for b in &brushes[i + 1..] {
                assert!(
                    crate::corner_join::shared_volume(a, b) < 1e-4,
                    "{:?} {:?}",
                    a.bounds(),
                    b.bounds()
                );
            }
        }
    }

    #[test]
    fn the_owners_wedges_running_into_each_other_are_refused_with_the_cause() {
        let mut d = owners_ten_walls();
        let before = d.to_map_text();
        // As in the screenshot: unsnapped the two fans on the 32 long wall just fit;
        // snapped to a grid of 1, their thin wedges tip into each other.
        let error = d
            .preview_corner_join_options(owners_settings())
            .err()
            .unwrap();
        assert!(
            error.contains(
                "would overlap along the brush at (-112 136 64) once their new corners are snapped to the grid"
            ),
            "{error}"
        );
        assert!(error.contains("unsnapped it fits"), "{error}");
        let refusal = d.corner_join_refusal(owners_settings()).expect("marked");
        assert!(refusal.place.is_some());
        assert_eq!(refusal.message, error);
        assert!(d
            .join_corner_options(d.revision(), owners_settings())
            .is_err());
        assert_eq!(d.to_map_text(), before, "nothing applied");
        let unsnapped = Rounding {
            snap: None,
            ..owners_settings()
        };
        let draft = d.preview_corner_join_options(unsnapped).unwrap();
        assert_eq!(draft.corners, 10);
        no_overlap(&draft.after_all);
        // A radius the short wall really has no room for: refused with the largest that
        // fits, and that one joins.
        let large = Rounding {
            outer: None,
            inner: Some((20.0, 9)),
            wedges: false,
            preserve_thickness: false,
            ..unsnapped
        };
        let error = d.preview_corner_join_options(large).err().unwrap();
        assert!(error.contains("it is too short between them"), "{error}");
        let fits: f64 = error
            .rsplit("fits is ")
            .next()
            .unwrap()
            .trim_end_matches('.')
            .parse()
            .unwrap_or_else(|_| panic!("{error}"));
        assert!((8.0..20.0).contains(&fits), "{fits}");
        let smaller = Rounding {
            inner: Some((fits, 9)),
            ..large
        };
        let draft = d.preview_corner_join_options(smaller).unwrap();
        no_overlap(&draft.after_all);
        d.join_corner_options(draft.revision, smaller).unwrap();
        no_overlap(
            &node::brushes(d.tree_world())
                .into_iter()
                .cloned()
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn every_rounding_of_the_room_is_joined_without_overlap_or_refused() {
        let d = owners_ten_walls();
        let mut joined = 0;
        for segments in [1, 2, 4, 8, 9, 16] {
            for radius in [4.0, 8.0, 13.0, 24.0] {
                for (outer, inner, concentric, wedges, preserve) in [
                    (true, false, false, false, false),
                    (false, true, false, false, false),
                    (true, true, false, false, false),
                    (true, true, true, false, false),
                    (true, true, false, true, false),
                    (true, true, false, true, true),
                ] {
                    for snap in [None, Some(1.0)] {
                        let rounding = Rounding {
                            outer: outer.then_some((radius, segments)),
                            inner: inner.then_some((radius, segments)),
                            concentric,
                            wedges,
                            preserve_thickness: preserve,
                            snap,
                            textures: false,
                        };
                        if d.corner_join_fits(rounding) {
                            let draft = d.preview_corner_join_options(rounding).unwrap();
                            no_overlap(&draft.after_all);
                            joined += 1;
                        }
                    }
                }
            }
        }
        assert!(joined > 50, "{joined}");
    }

    fn boxes(list: &[([f64; 3], [f64; 3])]) -> (Document, Vec<NodeId>) {
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
        let ids = node::brushes_with_ids(d.tree_world())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        (d, ids)
    }

    /// A U whose base is 48 long: two corners 16 wide leave 16 of straight wall.
    const U: [([f64; 3], [f64; 3]); 3] = [
        ([0.0, 0.0, 0.0], [48.0, 16.0, 64.0]),
        ([0.0, 0.0, 0.0], [16.0, 128.0, 64.0]),
        ([32.0, 0.0, 0.0], [48.0, 128.0, 64.0]),
    ];

    #[test]
    fn a_short_wall_between_two_mitres_still_joins() {
        let (mut d, _) = boxes(&U);
        d.select_all();
        let draft = d.preview_corner_join().unwrap();
        assert_eq!(draft.corners, 2);
        d.join_corner(draft.revision).unwrap();
        no_overlap(
            &node::brushes(d.tree_world())
                .into_iter()
                .cloned()
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn a_rounding_into_the_wedges_of_the_other_end_is_refused() {
        let (mut d, ids) = boxes(&U);
        let wedges = Rounding {
            inner: Some((13.0, 4)),
            ..Default::default()
        };
        d.select_only(ids[0]);
        d.toggle_selected(ids[1]);
        let draft = d.preview_corner_join_options(wedges).unwrap();
        d.join_corner_options(draft.revision, wedges).unwrap();
        // The base now goes on to the other post, the first corner's wedges in place.
        d.select_only(ids[0]);
        d.toggle_selected(ids[2]);
        let before = d.to_map_text();
        let error = d.preview_corner_join_options(wedges).err().unwrap();
        assert!(error.contains("unselected brush"), "{error}");
        assert!(error.contains("fits"), "{error}");
        assert!(d
            .corner_join_refusal(wedges)
            .is_some_and(|r| r.place.is_some()));
        assert!(d.join_corner_options(d.revision(), wedges).is_err());
        assert_eq!(d.to_map_text(), before);
        // Plainly mitred it goes.
        let draft = d.preview_corner_join().unwrap();
        d.join_corner(draft.revision).unwrap();
        no_overlap(
            &node::brushes(d.tree_world())
                .into_iter()
                .cloned()
                .collect::<Vec<_>>(),
        );
    }
}

#[cfg(test)]
mod angle_tests {
    use super::*;
    use crate::corner_join::Rounding;

    /// A straight wall seen from above: running from `start` at `degrees`, `length`
    /// long and `thickness` thick on its left (`left`) or right side, drawn on `back`
    /// units behind its start so its end overshoots the corner, as a rotated box does.
    fn wall(
        start: [f64; 2],
        degrees: f64,
        length: f64,
        thickness: f64,
        left: bool,
        back: f64,
    ) -> map_model::brush::Brush {
        let world = Bbox3d::from_scalars(-4096.0, 4096.0);
        let (low, high) = if left {
            (0.0, thickness)
        } else {
            (-thickness, 0.0)
        };
        let local = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d {
                min: Vec3d::new(-back, low, 0.0),
                max: Vec3d::new(length, high, 128.0),
            },
            "stone",
        )
        .unwrap();
        let (sin, cos) = degrees.to_radians().sin_cos();
        local
            .map_vertices(|p| {
                Vec3d::new(
                    start[0] + p.x() * cos - p.y() * sin,
                    start[1] + p.x() * sin + p.y() * cos,
                    p.z(),
                )
            })
            .unwrap()
    }

    fn square(min: [f64; 2], max: [f64; 2]) -> map_model::brush::Brush {
        map_model::shapes::create_cuboid(
            &Bbox3d::from_scalars(-4096.0, 4096.0),
            &Bbox3d {
                min: Vec3d::new(min[0], min[1], 0.0),
                max: Vec3d::new(max[0], max[1], 128.0),
            },
            "stone",
        )
        .unwrap()
    }

    fn document(walls: Vec<map_model::brush::Brush>) -> Document {
        let mut d = Document::new(Default::default());
        for brush in walls {
            d.add_brushes(vec![brush]);
        }
        d.select_all();
        d
    }

    fn brushes(d: &Document) -> Vec<map_model::brush::Brush> {
        node::brushes(d.tree_world()).into_iter().cloned().collect()
    }

    fn sound(d: &Document) {
        let all = brushes(d);
        for (i, a) in all.iter().enumerate() {
            assert!(a.volume() > 1e-6);
            for b in &all[i + 1..] {
                assert!(crate::corner_join::shared_volume(a, b) < 1e-4);
            }
        }
        crate::corner_join::assert_skip_only_where_hidden(&all, &[]);
    }

    fn has_vertex(b: &map_model::brush::Brush, x: f64, y: f64) -> bool {
        b.vertex_positions()
            .iter()
            .any(|p| (p.x() - x).abs() < 1e-3 && (p.y() - y).abs() < 1e-3)
    }

    /// An L opening at `degrees`: the first wall along +x, the second turning off it,
    /// both with their outer faces through the origin.
    fn l(degrees: f64, first: f64, second: f64) -> Document {
        // Far enough back to reach across the corner: the wall's thickness over the
        // tangent of half the angle, and a little more.
        let back = |t: f64| t.max(first.max(second)) / (degrees / 2.0f64).to_radians().tan() + 8.0;
        document(vec![
            wall([0.0, 0.0], 0.0, 256.0, first, true, back(first)),
            wall([0.0, 0.0], degrees, 256.0, second, false, back(second)),
        ])
    }

    #[test]
    fn walls_at_45_60_120_and_135_degrees_are_mitred_on_the_bisector() {
        for degrees in [45.0, 60.0, 120.0, 135.0] {
            for (first, second) in [(16.0, 16.0), (16.0, 24.0)] {
                let mut d = l(degrees, first, second);
                let before = d.to_map_text();
                let draft = d
                    .preview_corner_join()
                    .unwrap_or_else(|e| panic!("{degrees}: {e}"));
                assert_eq!(draft.corners, 1);
                d.join_corner(draft.revision).unwrap();
                sound(&d);
                let all = brushes(&d);
                assert_eq!(all.len(), 2);
                // The outer corner, where the outer faces meet, and the inner corner,
                // where the inner ones do, are corners of both walls.
                let (sin, cos) = degrees.to_radians().sin_cos();
                // Inner lines: y = first, and the second wall's inner face, `second`
                // in from its outer face through the origin.
                let t = (second + first * cos) / sin;
                let inner = (t, first);
                for brush in &all {
                    assert!(has_vertex(brush, 0.0, 0.0), "{degrees}: outer corner");
                    assert!(
                        has_vertex(brush, inner.0, inner.1),
                        "{degrees}: inner corner"
                    );
                    assert_eq!(brush.faces().len(), 6, "a clean mitred end");
                }
                if first == second {
                    // On the bisector.
                    let half = (degrees / 2.0).to_radians();
                    assert!((inner.1.atan2(inner.0) - half).abs() < 1e-6);
                }
                d.undo();
                assert_eq!(d.to_map_text(), before);
            }
        }
    }

    #[test]
    fn a_hexagonal_room_closes_its_loop() {
        let side = 256.0;
        let mut walls = Vec::new();
        let mut at = [0.0, 0.0];
        for k in 0..6 {
            let degrees = 60.0 * k as f64;
            walls.push(wall(at, degrees, side, 16.0, true, 16.0));
            let (sin, cos) = degrees.to_radians().sin_cos();
            at = [at[0] + side * cos, at[1] + side * sin];
        }
        let mut d = document(walls);
        let draft = d.preview_corner_join().unwrap();
        assert_eq!(draft.corners, 6);
        assert!(draft.left.is_empty(), "{:?}", draft.left);
        d.join_corner(draft.revision).unwrap();
        sound(&d);
        for brush in brushes(&d) {
            let slanted = brush
                .faces()
                .iter()
                .filter(|f| f.material_name.as_str() == "skip")
                .count();
            assert_eq!(slanted, 2, "mitred at both ends");
        }
    }

    #[test]
    fn rounding_an_angled_corner_is_refused_and_90_degrees_is_unchanged() {
        let d = l(60.0, 16.0, 16.0);
        let error = d
            .preview_corner_join_options(Rounding {
                inner: Some((8.0, 4)),
                ..Default::default()
            })
            .err()
            .unwrap();
        assert!(
            error.contains("not supported for corners at angles"),
            "{error}"
        );
        // A right angle of square boxes still goes the axis-aligned way, rounding and
        // all.
        let d = document(vec![
            square([0.0, 0.0], [256.0, 16.0]),
            square([0.0, 0.0], [16.0, 256.0]),
        ]);
        let draft = d
            .preview_corner_join_options(Rounding {
                inner: Some((8.0, 4)),
                ..Default::default()
            })
            .unwrap();
        assert!(!draft.join.added.is_empty());
    }

    #[test]
    fn snapping_reaches_angled_mitres_and_rounding_wedges() {
        let on_grid = |v: f64| (v - v.round()).abs() < 1e-6;
        let mut d = l(60.0, 16.0, 16.0);
        let snap = Rounding {
            snap: Some(1.0),
            ..Default::default()
        };
        let draft = d.preview_corner_join_options(snap).unwrap();
        // The corners the mitre made sit on the grid.
        let old: Vec<Vec3d> = draft
            .before_all
            .iter()
            .flat_map(|b| b.vertex_positions())
            .collect();
        for brush in &draft.after_all {
            for p in brush.vertex_positions() {
                if old.iter().any(|o| vec::length(*o - p) < 1e-6) {
                    continue;
                }
                assert!(on_grid(p.x()) && on_grid(p.y()), "{p:?}");
            }
        }
        d.join_corner_options(draft.revision, snap).unwrap();
        sound(&d);
        // Wedges of a rounded right angle too.
        let d = document(vec![
            square([0.0, 0.0], [256.0, 16.0]),
            square([0.0, 0.0], [16.0, 256.0]),
        ]);
        let draft = d
            .preview_corner_join_options(Rounding {
                inner: Some((16.0, 4)),
                snap: Some(1.0),
                ..Default::default()
            })
            .unwrap();
        assert!(!draft.join.added.is_empty());
        for brush in &draft.join.added {
            for p in brush.vertex_positions() {
                assert!(on_grid(p.x()) && on_grid(p.y()), "{p:?}");
            }
        }
        // Nine thin wedges of radius 13 snapped to a grid of 1 tip into each other: the
        // window says snapping is the cause rather than offering a radius.
        let error = d
            .preview_corner_join_options(Rounding {
                inner: Some((13.0, 9)),
                snap: Some(1.0),
                ..Default::default()
            })
            .err()
            .unwrap();
        assert!(
            error.contains("Snapping the corner to the grid causes it"),
            "{error}"
        );
        assert!(d
            .preview_corner_join_options(Rounding {
                inner: Some((13.0, 9)),
                ..Default::default()
            })
            .is_ok());
    }
}
