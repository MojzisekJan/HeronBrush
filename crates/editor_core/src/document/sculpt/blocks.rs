//! Sculpting square columns as blocks: a dab moves each column's whole top up or down by
//! whole grid steps and keeps its sides upright, so the terrain is built of steps, as
//! voxel games build theirs.
use super::*;

/// What a stroke in blocks remembers between dabs.
pub(super) struct Blocks {
    /// How far apart the heights a top may stop at are: the grid step.
    step: f64,
    /// Each column's height before it is rounded to a step, so dabs that each move it
    /// less than a step still add up to one.
    heights: HashMap<NodeId, f64>,
}

/// Whether `brush` is an upright box: six faces, each square to an axis. Only such a
/// column moves as a block.
fn upright_box(brush: &Brush) -> bool {
    brush.faces().len() == 6
        && brush.faces().iter().all(|face| {
            let n = face.normal();
            [n.x(), n.y(), n.z()]
                .iter()
                .filter(|c| c.abs() > 1.0 - 1e-9)
                .count()
                == 1
        })
}

impl Document {
    /// Starts a stroke that moves square columns as blocks, by whole steps of `step`
    /// units: what Raise, Lower, Smooth and Flatten do is worked out as for any stroke,
    /// then each top is rounded to a step, and the sides stay upright. Columns of any
    /// other shape are left alone. Every dab until the next begins is one undo step.
    pub fn begin_block_stroke(&self, step: f64) -> SculptStroke {
        SculptStroke {
            blocks: Some(Blocks {
                step: step.max(MIN_CELL),
                heights: HashMap::new(),
            }),
            ..self.begin_sculpt_stroke()
        }
    }

    /// One dab of a stroke in blocks at `centre`. See [`Self::begin_block_stroke`].
    pub(super) fn block_dab(
        &mut self,
        centre: Vec3d,
        parameters: &SculptParameters,
        stroke: &mut SculptStroke,
    ) -> bool {
        let radius = parameters.radius;
        let strength = parameters.strength.clamp(0.0, 1.0);
        let level = *stroke.level.get_or_insert(centre.z());
        let Some(blocks) = stroke.blocks.as_mut() else {
            return false;
        };
        let step = blocks.step;
        let middle = |bounds: &Bbox3d| {
            Vec3d::new(
                (bounds.min.x() + bounds.max.x()) / 2.0,
                (bounds.min.y() + bounds.max.y()) / 2.0,
                0.0,
            )
        };
        let across = |a: Vec3d, b: Vec3d| (a.x() - b.x()).hypot(a.y() - b.y());
        let everything = self.selection.is_empty();
        let world = node::brushes_by_layer(&self.tree.world);
        let open = self.open_group_contents();
        let editable = |id: NodeId, layer| self.is_editable_on(id, layer, &open);

        // The columns the dab reaches, by how much: their middles within the radius.
        let columns: Vec<(NodeId, &Brush, f64)> = world
            .iter()
            .filter(|(id, brush, layer)| {
                (everything || self.selection.contains(id))
                    && upright_box(brush)
                    && editable(*id, *layer)
            })
            .filter_map(|(id, brush, _)| {
                let weight = falloff(across(middle(&brush.bounds()), centre), radius);
                (weight > 0.0).then_some((*id, *brush, weight))
            })
            .collect();
        let height = |id: NodeId, brush: &Brush| {
            blocks
                .heights
                .get(&id)
                .copied()
                .unwrap_or(brush.bounds().max.z())
        };
        let neighbours = radius * 0.5;
        let mut heights = Vec::with_capacity(columns.len());
        for (id, brush, weight) in &columns {
            let h = height(*id, brush);
            let target = match parameters.mode {
                SculptMode::Raise => h + radius * 0.1 * strength,
                SculptMode::Lower => h - radius * 0.1 * strength,
                SculptMode::Flatten => h + (level - h) * strength,
                SculptMode::Smooth => {
                    let at = middle(&brush.bounds());
                    let (sum, total) = columns
                        .iter()
                        .map(|(other, other_brush, _)| {
                            let w = falloff(across(middle(&other_brush.bounds()), at), neighbours);
                            (height(*other, other_brush), w)
                        })
                        .filter(|(_, w)| *w > 0.0)
                        .fold((0.0, 0.0), |(sum, total), (h, w)| (sum + h * w, total + w));
                    let average = if total > 0.0 { sum / total } else { h };
                    h + (average - h) * strength
                }
            };
            let lowest = brush.bounds().min.z() + step;
            heights.push((h + (target - h) * weight).max(lowest));
        }

        // Each top rounded to a step, never thinner than one.
        let mut changed: HashMap<NodeId, Brush> = HashMap::new();
        for ((id, brush, _), h) in columns.iter().zip(heights) {
            blocks.heights.insert(*id, h);
            let bounds = brush.bounds();
            let top = ((h / step).round() * step).max(bounds.min.z() + step);
            if (top - bounds.max.z()).abs() < 1e-9 {
                continue;
            }
            let old = bounds.max.z();
            let Ok(moved) = brush.map_vertices(|p| {
                if (p.z() - old).abs() < 1e-6 {
                    Vec3d::new(p.x(), p.y(), top)
                } else {
                    p
                }
            }) else {
                continue;
            };
            changed.insert(*id, moved);
        }
        if changed.is_empty() {
            return false;
        }

        // A side is left out of exports while a neighbour covers it, so a column that
        // rose shows the side it lifted out, and one beside a column that sank does too.
        let grown = |bounds: &Bbox3d| bounds.expand(step.max(1.0));
        let reach: Vec<Bbox3d> = changed
            .values()
            .map(|brush| grown(&brush.bounds()))
            .chain(
                changed
                    .keys()
                    .filter_map(|id| self.brush(*id))
                    .map(|b| grown(&b.bounds())),
            )
            .collect();
        let near = |bounds: &Bbox3d| reach.iter().any(|r| r.intersects(bounds));
        let current = |id: &NodeId, brush: &'_ Brush| -> Brush {
            changed.get(id).cloned().unwrap_or_else(|| brush.clone())
        };
        let nearby: Vec<(NodeId, Brush, bool)> = world
            .iter()
            .filter(|(_, brush, _)| near(&brush.bounds()))
            .map(|(id, brush, layer)| (*id, current(id, brush), editable(*id, *layer)))
            .collect();
        let around = Around::new(nearby.iter().map(|(_, brush, _)| brush), step.max(8.0));
        let mut marked: Vec<(NodeId, Brush)> = Vec::new();
        for (id, brush, can_edit) in &nearby {
            if !can_edit || !upright_box(brush) {
                continue;
            }
            let shown = with_hidden_faces(brush, |face| {
                if face.normal().z().abs() > 0.5 {
                    face.export_hidden
                } else {
                    covered(&face.plane, &corners_in(brush, &face.plane), &around)
                }
            });
            if let Some(shown) = shown {
                marked.push((*id, shown));
            }
        }
        drop(around);
        for (id, brush) in marked {
            changed.insert(id, brush);
        }

        let before = changed
            .keys()
            .filter_map(|id| Some(brush_node(*id, self.brush(*id)?.clone())))
            .collect();
        drop(world);
        let after = changed
            .into_iter()
            .map(|(id, brush)| brush_node(id, brush))
            .collect();
        self.record_dab(stroke, before, after);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{square_terrain, terrain};
    use super::*;

    /// Every column of `d` with its middle and its top.
    fn tops(d: &Document) -> Vec<(f64, f64, f64)> {
        node::brushes_by_layer(d.tree_world())
            .iter()
            .map(|(_, brush, _)| {
                let b = brush.bounds();
                (
                    (b.min.x() + b.max.x()) / 2.0,
                    (b.min.y() + b.max.y()) / 2.0,
                    b.max.z(),
                )
            })
            .collect()
    }

    fn raise(d: &mut Document, mode: SculptMode, at: Vec3d, dabs: usize) -> SculptStroke {
        let params = SculptParameters {
            mode,
            radius: 40.0,
            strength: 1.0,
            ..Default::default()
        };
        let mut stroke = d.begin_block_stroke(16.0);
        for _ in 0..dabs {
            d.sculpt_dab(at, &params, &mut stroke);
        }
        stroke
    }

    #[test]
    fn blocks_rise_by_whole_steps_and_stay_upright_boxes() {
        let mut d = square_terrain();
        d.clear_selection();
        raise(&mut d, SculptMode::Raise, Vec3d::new(64.0, 64.0, 32.0), 6);
        let tops = tops(&d);
        assert!(
            tops.iter().all(|(_, _, z)| (z / 16.0).fract().abs() < 1e-9),
            "{tops:?}"
        );
        assert!(tops.iter().any(|(_, _, z)| *z > 32.0));
        assert!(tops.iter().any(|(_, _, z)| *z == 32.0), "the edge stays");
        assert!(node::brushes_by_layer(d.tree_world())
            .iter()
            .all(|(_, brush, _)| upright_box(brush) && brush.vertex_positions().len() == 8));
    }

    #[test]
    fn a_small_dab_adds_up_to_a_step() {
        let mut d = square_terrain();
        d.clear_selection();
        let params = SculptParameters {
            radius: 40.0,
            strength: 1.0,
            ..Default::default()
        };
        let mut stroke = d.begin_block_stroke(16.0);
        let at = Vec3d::new(72.0, 72.0, 32.0);
        // Four units a dab at the middle: 36 rounds back to 32, 40 goes up to 48.
        assert!(!d.sculpt_dab(at, &params, &mut stroke));
        assert!(d.sculpt_dab(at, &params, &mut stroke));
        assert!(tops(&d)
            .iter()
            .any(|(x, y, z)| (*x, *y) == (72.0, 72.0) && *z == 48.0));
    }

    #[test]
    fn lower_keeps_a_block_one_step_thick() {
        let mut d = square_terrain();
        d.clear_selection();
        raise(&mut d, SculptMode::Lower, Vec3d::new(64.0, 64.0, 32.0), 20);
        assert!(tops(&d).iter().all(|(_, _, z)| *z >= 16.0));
        assert!(tops(&d).iter().any(|(_, _, z)| *z == 16.0));
    }

    #[test]
    fn a_side_lifted_out_of_its_neighbours_is_exported() {
        let mut d = square_terrain();
        d.clear_selection();
        raise(&mut d, SculptMode::Raise, Vec3d::new(72.0, 72.0, 32.0), 6);
        // The column under the dab stands highest, so all four of its sides show.
        let peak = node::brushes_by_layer(d.tree_world())
            .into_iter()
            .map(|(_, brush, _)| brush.clone())
            .max_by(|a, b| a.bounds().max.z().total_cmp(&b.bounds().max.z()))
            .unwrap();
        let sides: Vec<_> = peak
            .faces()
            .iter()
            .filter(|f| f.normal().z().abs() < 0.5)
            .collect();
        assert_eq!(sides.len(), 4);
        let highest_neighbour = tops(&d)
            .iter()
            .map(|(_, _, z)| *z)
            .filter(|z| *z < peak.bounds().max.z())
            .fold(0.0, f64::max);
        assert!(highest_neighbour < peak.bounds().max.z());
        assert!(sides.iter().all(|f| !f.export_hidden), "{sides:?}");
    }

    #[test]
    fn columns_that_are_not_boxes_are_left_alone() {
        let mut d = terrain();
        d.clear_selection();
        let before = tops(&d);
        raise(&mut d, SculptMode::Raise, Vec3d::new(64.0, 64.0, 32.0), 6);
        assert_eq!(tops(&d), before);
    }
}
