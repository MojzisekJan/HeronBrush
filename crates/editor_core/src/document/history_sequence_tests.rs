//! Multi-action history contracts checked against full serialized document states.
use super::*;

fn scene() -> Document {
    let mut doc = Document::new(Default::default());
    for x in [0., 128., 256.] {
        let brush = map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &Bbox3d::new(Vec3d::new(x, 0., 0.), Vec3d::new(x + 64., 64., 64.)),
            "brick",
        )
        .unwrap();
        doc.add_brushes(vec![brush]);
    }
    doc.select_all();
    doc
}

fn assert_selection_exists(doc: &Document) {
    let filters = crate::view_filters::ViewFilters::default();
    let cached = doc.preview_mesh(&filters);
    *doc.surface_cache.borrow_mut() = Default::default();
    let rebuilt = doc.preview_mesh(&filters);
    assert_eq!(
        cached.triangles, rebuilt.triangles,
        "preview cache differs from a full rebuild"
    );
    fn visit(node: &Node, ids: &mut HashSet<NodeId>, groups: &mut HashSet<String>) {
        assert!(ids.insert(node.id), "duplicate runtime node identity");
        let children = match &node.kind {
            NodeKind::Group(group) => {
                assert!(
                    groups.insert(group.id.clone()),
                    "duplicate persistent group identity"
                );
                &group.children
            }
            NodeKind::Layer(layer) => &layer.children,
            NodeKind::Entity(entity) => &entity.children,
            _ => return,
        };
        for child in children {
            visit(child, ids, groups);
        }
    }
    let mut ids = HashSet::new();
    let mut groups = HashSet::new();
    for layer in &doc.tree_world().layers {
        for child in &layer.children {
            visit(child, &mut ids, &mut groups);
        }
    }
    for id in doc.selection_ids() {
        assert!(node::find_node(doc.tree_world(), *id).is_some());
    }
}

fn roundtrip(doc: &mut Document, states: &[String]) {
    for expected in states[..states.len() - 1].iter().rev() {
        assert!(doc.undo().is_some());
        assert_eq!(&doc.to_map_text(), expected);
        assert_selection_exists(doc);
    }
    for expected in &states[1..] {
        assert!(doc.redo().is_some());
        assert_eq!(&doc.to_map_text(), expected);
        assert_selection_exists(doc);
    }
}

#[test]
fn seeded_edit_sequences_restore_every_intermediate_state() {
    for seed in 1..=12u64 {
        let mut random = seed;
        let mut doc = scene();
        let mut states = vec![doc.to_map_text()];
        for step in 0..32 {
            random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
            let brushes = doc.selectable_brushes();
            let id = brushes[(random as usize >> 8) % brushes.len()].0;
            doc.select_only(id);
            let mark = doc.history.undo_len();
            match (random >> 32) % 6 {
                0 => {
                    doc.translate_selection(Vec3d::new(16., -8., 4.));
                }
                1 => {
                    doc.rotate_selection(Vec3d::new(0., 0., 1.), 90.);
                }
                2 if doc.brush_count() < 16 => {
                    doc.duplicate_selection_by(Vec3d::new(96., 0., 0.));
                }
                3 if doc.brush_count() > 1 => {
                    doc.delete_selection();
                }
                4 => {
                    doc.edit_faces(|face| face.move_uv(geometry_math::vec::Vec2f::new(3., -2.)));
                }
                _ => {
                    doc.edit_faces(|face| face.export_hidden = !face.export_hidden);
                }
            }
            if doc.history.undo_len() > mark {
                states.push(doc.to_map_text());
            }
            assert_selection_exists(&doc);
            // Exercise undo/redo during a sequence, not just at its end.
            if step % 7 == 6 && states.len() > 1 {
                assert!(doc.undo().is_some());
                assert_eq!(
                    doc.to_map_text(),
                    states[states.len() - 2],
                    "seed {seed} step {step}"
                );
                assert!(doc.redo().is_some());
                assert_eq!(doc.to_map_text(), *states.last().unwrap());
            }
        }
        roundtrip(&mut doc, &states);
    }
}

#[test]
fn nested_group_layer_delete_sequence_restores_structure_and_order() {
    let mut doc = scene();
    let mut states = vec![doc.to_map_text()];
    let layer = doc.add_layer("Details");
    states.push(doc.to_map_text());
    doc.move_selection_to_layer(layer);
    states.push(doc.to_map_text());
    doc.set_current_layer(layer);
    let inner = doc.group_selection("Inner").unwrap();
    states.push(doc.to_map_text());
    doc.select_only(inner);
    let outer = doc.group_selection("Outer").unwrap();
    doc.select_only(outer);
    states.push(doc.to_map_text());
    doc.duplicate_selection_by(Vec3d::new(512., 0., 0.));
    states.push(doc.to_map_text());
    doc.delete_selection();
    states.push(doc.to_map_text());
    doc.remove_layer(layer);
    states.push(doc.to_map_text());
    roundtrip(&mut doc, &states);
}

#[test]
fn cancelled_drag_preserves_redo_and_committed_drag_replaces_it() {
    let mut doc = scene();
    doc.translate_selection(Vec3d::new(32., 0., 0.));
    let translated = doc.to_map_text();
    doc.undo().unwrap();
    doc.select_all();
    let before = doc.to_map_text();
    doc.begin_transaction();
    doc.translate_selection(Vec3d::new(0., 48., 0.));
    doc.cancel_transaction();
    assert_eq!(doc.to_map_text(), before);
    doc.redo().unwrap();
    assert_eq!(doc.to_map_text(), translated);
    doc.undo().unwrap();
    doc.select_all();
    let mark = doc.history.undo_len();
    doc.begin_transaction();
    for amount in [8., 16., 24., 32.] {
        doc.rollback_transaction();
        doc.translate_selection(Vec3d::new(0., amount, 0.));
    }
    assert!(doc.commit_transaction());
    let after = doc.to_map_text();
    assert_eq!(doc.history.undo_len(), mark + 1);
    assert!(doc.redo().is_none());
    doc.undo().unwrap();
    assert_eq!(doc.to_map_text(), before);
    doc.redo().unwrap();
    assert_eq!(doc.to_map_text(), after);
}

#[test]
fn grouped_command_keeps_material_geometry_and_duplication_in_one_undo_step() {
    let mut doc = scene();
    let before = doc.to_map_text();
    let mark = doc.history.undo_len();
    doc.run_as_one_edit(|doc| {
        doc.duplicate_selection_by(Vec3d::new(0., 128., 0.));
        doc.translate_selection(Vec3d::new(0., 0., 32.));
        doc.edit_faces(|face| face.move_uv(geometry_math::vec::Vec2f::new(16., 4.)));
    });
    let after = doc.to_map_text();
    assert_eq!(doc.history.undo_len(), mark + 1);
    doc.undo().unwrap();
    assert_eq!(doc.to_map_text(), before);
    doc.redo().unwrap();
    assert_eq!(doc.to_map_text(), after);
}

#[test]
fn saved_checkpoint_follows_history_cursor_and_noop_preserves_redo() {
    let mut doc = scene();
    let directory =
        std::env::temp_dir().join(format!("heron-sequence-checkpoint-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("checkpoint.map");
    doc.save_as(&path).unwrap();
    let initial = doc.to_map_text();
    assert!(!doc.is_modified());
    doc.translate_selection(Vec3d::new(16., 0., 0.));
    assert!(doc.is_modified());
    doc.undo().unwrap();
    assert_eq!(doc.to_map_text(), initial);
    assert!(!doc.is_modified());
    doc.edit_faces(|_| {});
    assert!(doc.redo().is_some(), "no-op edits must not discard redo");
    doc.save_as(&path).unwrap();
    let saved = doc.to_map_text();
    assert!(!doc.is_modified());
    doc.undo().unwrap();
    assert!(doc.is_modified());
    doc.redo().unwrap();
    assert!(!doc.is_modified());
    assert_eq!(doc.to_map_text(), saved);
    let reopened = Document::open(&path, Default::default()).unwrap();
    assert_eq!(reopened.to_map_text(), saved);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn branching_and_unrecorded_layer_changes_cannot_reuse_a_saved_position() {
    let mut doc = scene();
    doc.translate_selection(Vec3d::new(16., 0., 0.));
    doc.history.mark_saved();
    doc.modified = false;
    doc.undo().unwrap();
    doc.select_all();
    doc.translate_selection(Vec3d::new(0., 16., 0.));
    assert!(
        doc.is_modified(),
        "same history depth is not the same saved content"
    );
    doc.undo().unwrap();
    assert!(doc.is_modified());
    doc.redo().unwrap();
    assert!(doc.is_modified());
    doc.history.mark_saved();
    doc.modified = false;
    doc.set_layer_hidden(0, true);
    doc.undo().unwrap();
    doc.redo().unwrap();
    assert!(
        doc.is_modified(),
        "an unrecorded persistent change must stay dirty"
    );
}

#[test]
fn remote_edit_invalidates_local_savepoint_before_undo() {
    let mut doc = scene();
    doc.history.mark_saved();
    doc.modified = false;
    let mut after = doc.tree_world().properties.clone();
    after.push(("remote-note".into(), "keep this change".into()));
    doc.apply_remote(&crate::history::Edit::WorldChanged {
        before: doc.tree_world().properties.clone(),
        after,
    });
    doc.select_all();
    doc.translate_selection(Vec3d::new(16., 0., 0.));
    doc.undo().unwrap();
    assert!(doc.is_modified());
    assert!(doc.to_map_text().contains("keep this change"));
}

#[test]
fn adopted_world_stays_unsaved_after_undoing_a_local_edit() {
    let source = scene();
    let mut doc = Document::new(Default::default());
    doc.adopt_world(source.tree_world().clone());
    doc.select_all();
    doc.translate_selection(Vec3d::new(16.,0.,0.));
    doc.undo().unwrap();
    assert!(doc.is_modified(), "a received world has never been saved locally");
}
