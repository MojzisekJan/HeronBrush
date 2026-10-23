use super::*;
pub struct CornerDraft {
    pub revision: u64,
    pub ids: [NodeId; 2],
    pub join: crate::corner_join::Join,
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
        })
    }
    pub fn preview_corner_join_options(
        &self,
        rounding: crate::corner_join::Rounding,
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
        Ok(CornerDraft {
            revision: self.revision(),
            ids: [ids[0], ids[1]],
            join: crate::corner_join::preview_with_rounding(a, b, rounding)?,
        })
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
        let draft = self.preview_corner_join_options(rounding)?;
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
                .corner_parent(draft.ids)
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
                for replacement in after {
                    let child = children
                        .iter_mut()
                        .find(|node| node.id == replacement.id)
                        .ok_or("Corner brush no longer exists.")?;
                    *child = replacement;
                }
                children.extend(additions);
                Edit::Changed {
                    before: vec![original],
                    after: vec![updated],
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
