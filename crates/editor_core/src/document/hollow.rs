use super::*;
impl Document {
    /// Atomic replacement which retains each brush's immediate parent and first ID.
    pub(super) fn replace_geometry_in_place(&mut self, replacements: HashMap<NodeId, Vec<Brush>>) {
        if !replacements.is_empty() && replacements.values().all(|brushes| brushes.len() == 1) {
            // Pure deformations do not need to clone a containing entity/group.
            let mut before = Vec::new();
            let mut after = Vec::new();
            for (id, brushes) in &replacements {
                if let Some(original) = self.brush(*id) {
                    before.push(brush_node(*id, original.clone()));
                    after.push(brush_node(*id, brushes[0].clone()));
                }
            }
            self.apply_and_record(Edit::Changed { before, after });
            self.selection = replacements.keys().copied().collect();
            self.face_selection.clear();
            return;
        }
        fn rewrite(
            source: &Node,
            map: &HashMap<NodeId, Vec<Brush>>,
            selected: &mut HashSet<NodeId>,
        ) -> Option<Vec<Node>> {
            if let Some(brushes) = map.get(&source.id) {
                return Some(
                    brushes
                        .iter()
                        .enumerate()
                        .map(|(i, b)| {
                            let node = Node::with_id(
                                if i == 0 { source.id } else { NodeId::next() },
                                NodeKind::Brush(b.clone()),
                            );
                            selected.insert(node.id);
                            node
                        })
                        .collect(),
                );
            }
            let children = match &source.kind {
                NodeKind::Group(g) => &g.children,
                NodeKind::Entity(e) => &e.children,
                NodeKind::Layer(l) => &l.children,
                _ => return None,
            };
            let mut changed = false;
            let children = children
                .iter()
                .flat_map(|child| match rewrite(child, map, selected) {
                    Some(nodes) => {
                        changed = true;
                        nodes
                    }
                    None => vec![child.clone()],
                })
                .collect();
            if !changed {
                return None;
            }
            let mut result = source.clone();
            match &mut result.kind {
                NodeKind::Group(g) => g.children = children,
                NodeKind::Entity(e) => e.children = children,
                NodeKind::Layer(l) => l.children = children,
                _ => unreachable!(),
            };
            Some(vec![result])
        }
        let mut edits = Vec::new();
        let mut selected = HashSet::new();
        for (layer, contents) in self.tree.world.layers.iter().enumerate() {
            for source in &contents.children {
                if let Some(mut nodes) = rewrite(source, &replacements, &mut selected) {
                    if nodes.is_empty() {
                        edits.push(Edit::Removed {
                            layer,
                            nodes: vec![source.clone()],
                        });
                    } else {
                        let first = nodes.remove(0);
                        edits.push(Edit::Changed {
                            before: vec![source.clone()],
                            after: vec![first],
                        });
                        if !nodes.is_empty() {
                            edits.push(Edit::Added { layer, nodes });
                        }
                    }
                }
            }
        }
        if !edits.is_empty() {
            self.apply_and_record(Edit::Batch(edits));
            self.selection = selected;
            self.face_selection.clear();
        }
    }

    pub fn hollow_selection_options(
        &mut self,
        thickness: f64,
        direction: map_model::hollow::Direction,
        open_faces: &[FaceRef],
        join_convex: bool,
    ) -> Result<usize, String> {
        if self.in_transaction() {
            return Err("Finish the current drag first.".into());
        }
        if open_faces.iter().any(|face| {
            self.brush(face.node)
                .is_none_or(|b| b.geometry_face_of(face.face).is_none())
        }) {
            return Err("A selected opening no longer names a visible brush face.".into());
        }
        let selected: Vec<_> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| {
                self.selection.contains(id) || open_faces.iter().any(|f| f.node == *id)
            })
            .map(|(id, b)| (id, b.clone()))
            .collect();
        if selected.is_empty() {
            return Err("Select brushes to hollow.".into());
        }
        if selected.iter().any(|(id, _)| !self.is_editable(*id)) {
            return Err("Every selected brush must be editable.".into());
        }
        let count = selected.len();
        let mut changes = HashMap::new();
        if join_convex && selected.len() > 1 {
            if !open_faces.is_empty() {
                return Err("Open faces individually, or join the solid first.".into());
            }
            // Joining is explicit and must preserve both volume and parent semantics.
            if selected.len() != 2 || self.corner_parent([selected[0].0, selected[1].0]).is_none() {
                return Err("Join-shell mode currently requires two brushes with the same immediate parent.".into());
            }
            let points: Vec<_> = selected
                .iter()
                .flat_map(|(_, b)| b.vertex_positions())
                .collect();
            let hull = Polyhedron::from_points(&points, 1e-7)
                .map_err(|_| "The selected solid cannot be merged.")?;
            let attrs: Vec<_> = selected
                .iter()
                .flat_map(|(_, b)| b.faces().iter().cloned())
                .collect();
            let merged = Brush::from_polyhedron(hull, &self.current_material)
                .with_face_attributes_from(&attrs);
            // Material transfer alone does not carry paint/PBR/visibility. Select
            // the same coplanar source style before the new shell is cut from it.
            let styled: Vec<_> = merged
                .faces()
                .iter()
                .map(|face| {
                    let source = attrs
                        .iter()
                        .filter(|f| {
                            vec::length(f.normal() - face.normal()) < 1e-7
                                && (f.plane.distance - face.plane.distance).abs() < 1e-7
                        })
                        .max_by(|a, b| {
                            let area = |f: &map_model::brush_face::BrushFace| {
                                vec::length(vec::cross(
                                    f.points[1] - f.points[0],
                                    f.points[2] - f.points[0],
                                ))
                            };
                            area(a).total_cmp(&area(b))
                        })
                        .or_else(|| {
                            attrs.iter().max_by(|a, b| {
                                vec::dot(a.normal(), face.normal())
                                    .total_cmp(&vec::dot(b.normal(), face.normal()))
                            })
                        })
                        .unwrap();
                    let mut result = face.clone();
                    result.color = source.color.clone();
                    result.surface_material = source.surface_material;
                    result.export_hidden = source.export_hidden;
                    result.line = source.line;
                    result
                })
                .collect();
            let merged = merged.with_exact_face_attributes(&styled).unwrap();
            let mut uncovered = vec![merged.clone()];
            for (_, brush) in &selected {
                uncovered = uncovered
                    .into_iter()
                    .flat_map(|fragment| csg::subtract(&fragment, brush))
                    .collect();
            }
            if uncovered.iter().map(Brush::volume).sum::<f64>() > merged.volume().max(1.0) * 1e-7 {
                return Err(
                    "The joined selection must enclose a convex solid without gaps.".into(),
                );
            }
            changes.insert(
                selected[0].0,
                map_model::hollow::shell(&merged, &self.world_bounds, thickness, direction, &[])?,
            );
            changes.insert(selected[1].0, Vec::new());
        } else {
            for (id, brush) in &selected {
                let open: Vec<_> = open_faces
                    .iter()
                    .filter(|f| f.node == *id)
                    .map(|f| f.face)
                    .collect();
                changes.insert(
                    *id,
                    map_model::hollow::shell(
                        brush,
                        &self.world_bounds,
                        thickness,
                        direction,
                        &open,
                    )?,
                );
            }
        }
        if direction != map_model::hollow::Direction::Inward && count > 1 {
            let mut walls: Vec<_> = changes
                .iter()
                .flat_map(|(id, brushes)| {
                    brushes
                        .iter()
                        .map(move |brush| (*id, brush, brush.bounds()))
                })
                .collect();
            walls.sort_by(|a, b| a.2.min[0].total_cmp(&b.2.min[0]));
            for (i, (id, a, aa)) in walls.iter().enumerate() {
                for (other, b, bb) in walls.iter().skip(i + 1) {
                    if bb.min[0] >= aa.max[0] - 1e-6 {
                        break;
                    }
                    if id == other
                        || (1..3).any(|axis| {
                            aa.max[axis].min(bb.max[axis]) - aa.min[axis].max(bb.min[axis]) <= 1e-6
                        })
                    {
                        continue;
                    }
                    if csg::intersect(a, b).is_some_and(|overlap| {
                        overlap.volume() > a.volume().min(b.volume()).max(1.0) * 1e-7
                    }) {
                        return Err("The expanded shells intersect. Use a joined convex pair, open the connection after merging, or choose inward thickness.".into());
                    }
                }
            }
        }
        self.replace_geometry_in_place(changes);
        Ok(count)
    }
}
