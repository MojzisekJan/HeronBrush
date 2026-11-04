//! Ordinary editing sequences, with history and persistence checked together.
use super::*;

fn scene() -> Document {
    let mut d = Document::new(Default::default());
    for x in [0.0, 128.0] {
        let b = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &Bbox3d::new(Vec3d::new(x, 0.0, 0.0), Vec3d::new(x + 64.0, 48.0, 32.0)),
            "brick",
        )
        .unwrap();
        d.add_brushes(vec![b]);
    }
    d.select_all();
    d
}

#[test]
fn basic_workflow_noop_uv_colour_and_flags_do_not_change_history_or_revision() {
    let mut d = scene();
    let before = d.to_map_text();
    let revision = d.revision();
    let undo = d.history.undo_len();
    d.edit_faces(|_| {});
    d.edit_faces(|f| f.move_uv(geometry_math::vec::Vec2f::zero()));
    d.set_face_color(None);
    d.edit_faces(|f| f.export_hidden = false);
    assert_eq!(d.to_map_text(), before);
    assert_eq!(
        d.revision(),
        revision,
        "passive/no-op edits must not dirty the document"
    );
    assert_eq!(d.history.undo_len(), undo);
}

#[test]
fn basic_workflow_uv_changes_only_explicit_faces_even_with_brush_handles_selected() {
    let mut d = scene();
    let id = d.selectable_brushes()[0].0;
    let target = FaceRef { node: id, face: 0 };
    let other = FaceRef { node: id, face: 1 };
    d.select_face_only(target);
    d.selection.insert(id);
    let before = d.to_map_text();
    let untouched = d.face_at(other).unwrap().clone();
    let original = d.face_at(target).unwrap().uv_attributes;
    d.edit_faces(|f| f.move_uv(geometry_math::vec::Vec2f::new(8.0, -4.0)));
    assert_ne!(d.face_at(target).unwrap().uv_attributes, original);
    assert_eq!(*d.face_at(other).unwrap(), untouched);
    let after = d.to_map_text();
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.redo().unwrap();
    assert_eq!(d.to_map_text(), after);
}

#[test]
fn basic_workflow_face_handle_texturing_does_not_paint_the_selected_brush() {
    let mut d = scene();
    let id = d.selectable_brushes()[0].0;
    d.select_only(id);
    let brush = d.brush(id).unwrap();
    let face = map_model::polyhedron::FaceId(0);
    let points = brush.geometry().face_positions(face);
    let center = points.iter().copied().fold(Vec3d::zero(), |a, b| a + b) / points.len() as f64;
    let mut handles = crate::handle_tool::HandleManager::new();
    handles.collect(
        crate::handle_tool::HandleKind::Face,
        d.selected_brushes().into_iter(),
    );
    handles.select(center, false);
    let picked = d.faces_at(&handles.selected_face_centres());
    assert_eq!(picked.len(), 1);
    let before = d.to_map_text();
    let untouched = d.brush(id).unwrap().faces().to_vec();
    d.set_current_material("stone");
    assert_eq!(d.apply_current_material_on(&picked), 1);
    for index in 0..d.brush(id).unwrap().geometry().face_count() {
        let reference = FaceRef {
            node: id,
            face: index,
        };
        if index == picked[0].face {
            assert_eq!(d.face_at(reference).unwrap().material_name, "stone");
        } else {
            assert_eq!(d.face_at(reference).unwrap().material_name, "brick");
        }
    }
    assert_ne!(d.brush(id).unwrap().faces(), untouched.as_slice());
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
}

#[test]
fn basic_workflow_copy_paste_cut_preserves_attributes_and_unique_ids() {
    let mut d = scene();
    d.set_face_color(Some(map_model::brush_face::FaceColor::solid(
        map_model::brush_face::Rgb::new(10, 80, 180),
    )));
    d.edit_faces(|f| f.move_uv(geometry_math::vec::Vec2f::new(8.0, 16.0)));
    let before = d.to_map_text();
    let original = d.selection_ids().clone();
    let copied = d.copy_selection().unwrap();
    assert_eq!(d.paste_at(&copied, Some(Vec3d::new(512.0, 128.0, 64.0))), 2);
    assert_eq!(d.brush_count(), 4);
    assert_eq!(d.selection_count(), 2);
    assert!(d.selection_ids().is_disjoint(&original));
    assert!(d.selected_brushes().iter().all(|b| b
        .faces()
        .iter()
        .all(|f| f.material_name == "brick" && f.color.is_some())));
    let pasted = d.to_map_text();
    let cut = d.cut_selection().unwrap();
    assert_eq!(d.brush_count(), 2);
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), pasted);
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.redo().unwrap();
    assert_eq!(d.to_map_text(), pasted);
    assert_eq!(d.paste_at(&cut, None), 2);
    assert_eq!(d.brush_count(), 6);
    let ids: HashSet<_> = node::brushes_with_ids(d.tree_world())
        .iter()
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(ids.len(), 6);
}

#[test]
fn basic_workflow_transform_duplicate_delete_save_reopen_and_undo_redo_chain() {
    let mut d = scene();
    let mut states = vec![d.to_map_text()];
    assert_eq!(d.duplicate_selection_by(Vec3d::new(256.0, 0.0, 0.0)), 2);
    states.push(d.to_map_text());
    assert!(d.translate_selection(Vec3d::new(16.0, 32.0, 16.0)));
    states.push(d.to_map_text());
    assert!(d.rotate_selection(Vec3d::new(0.0, 0.0, 1.0), 90.0));
    states.push(d.to_map_text());
    assert!(
        d.transform_selection(&geometry_math::mat_ext::scaling_matrix(Vec3d::new(
            2.0, 1.0, 1.0
        )))
    );
    states.push(d.to_map_text());
    assert_eq!(d.delete_selection(), 2);
    states.push(d.to_map_text());
    for previous in states[..states.len() - 1].iter().rev() {
        d.undo().unwrap();
        assert_eq!(&d.to_map_text(), previous);
    }
    for next in states.iter().skip(1) {
        d.redo().unwrap();
        assert_eq!(&d.to_map_text(), next);
    }
    // Persist the transformed copies too, rather than testing an empty/unchanged map.
    d.undo().unwrap();
    let saved = d.to_map_text();
    let directory =
        std::env::temp_dir().join(format!("heron-basic-workflow-save-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("roundtrip.map");
    d.save_as(&path).unwrap();
    assert!(!d.is_modified());
    let reopened = Document::open(&path, Default::default()).unwrap();
    assert_eq!(reopened.to_map_text(), saved);
    assert_eq!(reopened.brush_count(), 4);
    d.edit_faces(|_| {});
    assert!(
        !d.is_modified(),
        "no-op controls must not dirty a saved map"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn basic_workflow_face_handle_context_targets_uv_and_clears_without_dirtying() {
    let mut d = scene();
    let id = d.selectable_brushes()[0].0;
    d.select_only(id);
    let target = FaceRef { node: id, face: 2 };
    let other = FaceRef { node: id, face: 0 };
    let revision = d.revision();
    let before = d.to_map_text();
    let original = d.face_at(other).unwrap().clone();
    d.set_face_edit_targets(vec![target]);
    assert_eq!(d.revision(), revision);
    assert_eq!(d.selection_count(), 1);
    assert_eq!(d.face_selection_count(), 0);
    assert_eq!(d.face_to_edit().unwrap().0, target);
    crate::uv_view::Field::OffsetX.set(&mut d, 32.0);
    assert_eq!(*d.face_at(other).unwrap(), original);
    assert_eq!(d.face_at(target).unwrap().uv_attributes.offset.x(), 32.0);
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    let revision = d.revision();
    d.set_face_edit_targets(Vec::new());
    assert_eq!(d.revision(), revision);
    assert_eq!(d.face_to_edit().unwrap().0, other);
    d.set_face_edit_targets(vec![target]);
    d.select_face_only(other);
    assert_eq!(
        d.face_to_edit().unwrap().0,
        other,
        "explicit face selection wins over stale handles"
    );
}

#[test]
fn basic_workflow_delete_restores_layer_parent_and_sibling_order() {
    let mut d = scene();
    let layer = d.add_layer("Details");
    d.select_all();
    d.move_selection_to_layer(layer);
    let members: Vec<_> = d.selection_ids().iter().copied().collect();
    let group = d.group_selection("Walls").unwrap();
    assert!(d.open_selected_group());
    d.select_only(members[0]);
    let before = d.to_map_text();
    assert_eq!(d.delete_selection(), 1);
    assert!(node::find_node(d.tree_world(), members[0]).is_none());
    assert!(node::find_node(d.tree_world(), group).is_some());
    let after = d.to_map_text();
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.redo().unwrap();
    assert_eq!(d.to_map_text(), after);
    d.undo().unwrap();
    d.select_only(group);
    let before = d.to_map_text();
    d.delete_selection();
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    assert!(d.tree_world().layers[layer]
        .children
        .iter()
        .any(|n| n.id == group));
}

#[test]
fn basic_workflow_delete_middle_root_roundtrips_and_wire_reorders() {
    let mut d = scene();
    d.duplicate_selection_by(Vec3d::new(256.0, 0.0, 0.0));
    let ids: Vec<_> = d.tree_world().layers[0]
        .children
        .iter()
        .map(|n| n.id)
        .collect();
    d.select_only(ids[1]);
    let before = d.to_map_text();
    d.cut_selection().unwrap();
    let after = d.to_map_text();
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.redo().unwrap();
    assert_eq!(d.to_map_text(), after);
    let edit = Edit::Reordered {
        layer: 0,
        before: ids.clone(),
        after: ids.into_iter().rev().collect(),
    };
    let wire = crate::collab::wire::WireEdit::from_edit(&edit);
    let decoded = crate::collab::wire::WireEdit::from_json(&wire.to_json())
        .unwrap()
        .into_edit(&d.world_bounds())
        .unwrap();
    assert_eq!(format!("{edit:?}"), format!("{decoded:?}"));
}

#[test]
fn texture_transfer_replaces_target_paint_without_touching_other_faces() {
    use map_model::brush_face::{FaceColor, Rgb};
    for paint in [
        FaceColor::solid(Rgb::new(20, 100, 200)),
        FaceColor::gradient(Rgb::new(20, 100, 200), Rgb::new(200, 50, 20), 45.0),
    ] {
        for mode in [
            FaceAttributeTransfer::MaterialOnly,
            FaceAttributeTransfer::Projection,
            FaceAttributeTransfer::Rotation,
        ] {
            let mut d = scene();
            let nodes = d.selectable_brushes();
            let source = FaceRef {
                node: nodes[0].0,
                face: 0,
            };
            let target = FaceRef {
                node: nodes[1].0,
                face: 1,
            };
            d.select_face_only(target);
            d.set_face_color(Some(paint));
            let untouched = d
                .face_at(FaceRef {
                    node: target.node,
                    face: 2,
                })
                .unwrap()
                .clone();
            let before = d.to_map_text();
            assert!(d.copy_face_attributes(source, target, mode, false));
            assert!(
                d.face_at(target).unwrap().color.is_none(),
                "target paint hides the transferred texture"
            );
            assert_eq!(
                *d.face_at(FaceRef {
                    node: target.node,
                    face: 2
                })
                .unwrap(),
                untouched
            );
            let after = d.to_map_text();
            d.undo().unwrap();
            assert_eq!(d.to_map_text(), before);
            d.redo().unwrap();
            assert_eq!(d.to_map_text(), after);
        }
    }
}

#[test]
fn texture_transfer_carries_the_source_paint_and_a_placed_gradient() {
    use map_model::brush_face::{ColorStop, FaceColor, GradientEnds, GradientStops, Rgb};
    let stops = GradientStops::new(&[
        ColorStop {
            position: 0.0,
            color: Rgb::new(255, 0, 0),
        },
        ColorStop {
            position: 0.4,
            color: Rgb::new(0, 255, 0),
        },
        ColorStop {
            position: 1.0,
            color: Rgb::new(0, 0, 255),
        },
    ])
    .unwrap();
    let paint = FaceColor::MultiGradient {
        stops,
        radial: false,
        angle: 0.0,
        alpha: 200,
        ends: Some(GradientEnds {
            start: [0.0, 0.0],
            end: [1.0, 1.0],
        }),
    };
    for mode in [
        FaceAttributeTransfer::MaterialOnly,
        FaceAttributeTransfer::Projection,
        FaceAttributeTransfer::Rotation,
    ] {
        let mut d = scene();
        let nodes = d.selectable_brushes();
        let source = FaceRef {
            node: nodes[0].0,
            face: 0,
        };
        let target = FaceRef {
            node: nodes[1].0,
            face: 1,
        };
        d.select_face_only(source);
        d.set_face_color(Some(paint));
        assert!(d.copy_face_attributes(source, target, mode, false));
        assert_eq!(d.face_at(target).unwrap().paint(), Some(paint));
        d.undo().unwrap();
        let clipboard = d.copy_face_attributes_text().unwrap();
        d.select_face_only(target);
        assert_eq!(d.paste_at(&clipboard, None), 1);
        assert_eq!(
            d.face_at(target).unwrap().paint(),
            Some(paint),
            "the clipboard carries the paint"
        );
    }
}

#[test]
fn texture_transfer_clipboard_matches_direct_transfer_for_every_source_normal() {
    for face in 0..6 {
        let mut d = scene();
        let nodes = d.selectable_brushes();
        let source = FaceRef {
            node: nodes[0].0,
            face,
        };
        let target = FaceRef {
            node: nodes[1].0,
            face: 2,
        };
        d.select_face_only(source);
        d.edit_faces(|f| {
            f.material_name = "stone".into();
            f.rotate_uv(37.0);
            f.move_uv(geometry_math::vec::Vec2f::new(12.0, 24.0));
        });
        let clipboard = d.copy_face_attributes_text().unwrap();
        d.copy_face_attributes(source, target, FaceAttributeTransfer::Projection, false);
        let expected = d.face_at(target).unwrap().clone();
        d.undo().unwrap();
        d.select_face_only(target);
        assert_eq!(d.paste_at(&clipboard, None), 1);
        assert_eq!(*d.face_at(target).unwrap(), expected, "source face {face}");
    }
}

#[test]
fn texture_transfer_copy_and_paste_accept_face_tool_handles() {
    let mut d = scene();
    let nodes = d.selectable_brushes();
    let source = FaceRef {
        node: nodes[0].0,
        face: 0,
    };
    let target = FaceRef {
        node: nodes[1].0,
        face: 1,
    };
    d.select_face_only(source);
    d.edit_faces(|f| f.material_name = "stone".into());
    d.select_only(source.node);
    d.set_face_edit_targets(vec![source]);
    assert_eq!(d.single_selected_face(), Some(source));
    let clipboard = d
        .copy_face_attributes_text()
        .expect("face handle should copy attributes");
    d.select_only(target.node);
    d.set_face_edit_targets(vec![target]);
    assert_eq!(d.paste_at(&clipboard, None), 1);
    assert_eq!(d.face_at(target).unwrap().material_name, "stone");
    assert_eq!(
        d.face_at(FaceRef {
            node: target.node,
            face: 2
        })
        .unwrap()
        .material_name,
        "brick"
    );
}

#[test]
fn texture_transfer_onto_source_is_not_an_edit_and_keeps_its_paint() {
    let mut d = scene();
    let id = d.selectable_brushes()[0].0;
    let source = FaceRef { node: id, face: 0 };
    d.select_face_only(source);
    d.set_face_color(Some(map_model::brush_face::FaceColor::solid(
        map_model::brush_face::Rgb::new(10, 20, 30),
    )));
    let before = d.to_map_text();
    let revision = d.revision();
    assert!(!d.copy_face_attributes(source, source, FaceAttributeTransfer::Projection, false));
    assert_eq!(
        d.copy_face_attributes_across(source, &[source], FaceAttributeTransfer::Projection),
        0
    );
    assert_eq!(d.to_map_text(), before);
    assert_eq!(d.revision(), revision);
}

#[test]
fn texture_transfer_select_all_replaces_the_previous_explicit_face_selection() {
    let mut d = scene();
    let nodes = d.selectable_brushes();
    let source = FaceRef {
        node: nodes[0].0,
        face: 0,
    };
    let target = FaceRef {
        node: nodes[1].0,
        face: 1,
    };
    d.select_face_only(source);
    d.edit_faces(|f| f.material_name = "stone".into());
    d.select_all();
    assert_eq!(d.face_selection_count(), 0);
    d.set_face_edit_targets(vec![source]);
    let clipboard = d.copy_face_attributes_text().unwrap();
    d.set_face_edit_targets(vec![target]);
    assert_eq!(d.single_selected_face(), Some(target));
    assert_eq!(d.paste_at(&clipboard, None), 1);
    assert_eq!(d.face_at(target).unwrap().material_name, "stone");
}

#[test]
fn material_stroke_updates_live_and_one_undo_restores_all_faces_and_paint() {
    let mut d = scene();
    d.set_face_color(Some(map_model::brush_face::FaceColor::solid(
        map_model::brush_face::Rgb::new(20, 100, 200),
    )));
    let ids: Vec<_> = d
        .selectable_brushes()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let a = FaceRef {
        node: ids[0],
        face: 0,
    };
    let b = FaceRef {
        node: ids[1],
        face: 1,
    };
    let untouched = d
        .face_at(FaceRef {
            node: ids[0],
            face: 2,
        })
        .unwrap()
        .clone();
    d.set_current_material("stone");
    let before = d.to_map_text();
    let mark = d.history.undo_len();
    let mut stroke = d.begin_material_stroke();
    assert_eq!(d.paint_material_stroke(&[a], &mut stroke), 1);
    assert_eq!(d.face_at(a).unwrap().material_name, "stone");
    assert!(d.face_at(a).unwrap().color.is_none());
    assert_eq!(d.paint_material_stroke(&[a, b, b], &mut stroke), 1);
    assert_eq!(
        *d.face_at(FaceRef {
            node: ids[0],
            face: 2
        })
        .unwrap(),
        untouched
    );
    assert_eq!(d.history.undo_len(), mark + 1);
    let after = d.to_map_text();
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.redo().unwrap();
    assert_eq!(d.to_map_text(), after);
}

#[test]
fn material_stroke_keeps_separate_strokes_and_does_not_merge_intervening_edits() {
    let mut d = scene();
    let id = d.selectable_brushes()[0].0;
    d.set_current_material("stone");
    let mut first = d.begin_material_stroke();
    assert_eq!(
        d.paint_material_stroke(&[FaceRef { node: id, face: 0 }], &mut first),
        1
    );
    let after_first = d.to_map_text();
    let mut second = d.begin_material_stroke();
    assert_eq!(
        d.paint_material_stroke(&[FaceRef { node: id, face: 1 }], &mut second),
        1
    );
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), after_first);
    assert_eq!(
        d.paint_material_stroke(&[FaceRef { node: id, face: 2 }], &mut second),
        0
    );
    let revision = d.revision();
    d.set_current_material("stone");
    let mut noop = d.begin_material_stroke();
    assert_eq!(
        d.paint_material_stroke(&[FaceRef { node: id, face: 0 }], &mut noop),
        0
    );
    assert_eq!(d.revision(), revision);
}

#[test]
fn material_stroke_remains_one_undo_when_history_is_full() {
    let mut d = scene();
    let id = d.selectable_brushes()[0].0;
    d.select_face_only(FaceRef { node: id, face: 5 });
    for n in 0..crate::history::CAPACITY {
        d.edit_faces(|f| f.uv_attributes.offset = Vec2f::new(n as f32 + 1.0, 0.0));
    }
    let before = d.to_map_text();
    d.set_current_material("stone");
    let mut stroke = d.begin_material_stroke();
    d.paint_material_stroke(&[FaceRef { node: id, face: 0 }], &mut stroke);
    d.paint_material_stroke(&[FaceRef { node: id, face: 1 }], &mut stroke);
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
}

#[test]
fn pointer_material_paste_targets_one_face_without_changing_selection_or_inserting_objects() {
    let mut d = scene();
    let ids: Vec<_> = d
        .selectable_brushes()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let source = FaceRef {
        node: ids[0],
        face: 0,
    };
    let target = FaceRef {
        node: ids[1],
        face: 1,
    };
    d.select_face_only(source);
    d.edit_faces(|f| f.material_name = "stone".into());
    let clipboard = d.copy_face_attributes_text().unwrap();
    let selection = d.selection_snapshot();
    let before = d.to_map_text();
    assert_eq!(d.paste_face_attributes_on(&clipboard, &[target]), 1);
    assert!(d.selection_snapshot() == selection);
    assert_eq!(d.brush_count(), 2);
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.select_all();
    let objects = d.copy_selection().unwrap();
    let before = d.to_map_text();
    let revision = d.revision();
    assert_eq!(d.paste_face_attributes_on(&objects, &[target]), 0);
    assert_eq!(d.paste_face_attributes_on("random text", &[target]), 0);
    assert_eq!(d.to_map_text(), before);
    assert_eq!(d.revision(), revision);
}

#[test]
fn clipboard_preserves_free_uv_axes_in_every_document_format() {
    use geometry_math::vec::Vec2f;
    use map_model::map_format::MapFormat;
    use map_model::uv::{parallel_axes, UvCoordSystem};
    for format in [MapFormat::Standard, MapFormat::Valve, MapFormat::Quake3] {
        let mut d = scene();
        d.format = format;
        let id = d.selectable_brushes()[0].0;
        d.select_only(id);
        d.edit_faces(|face| {
            let (u_axis, v_axis) = parallel_axes(face.normal());
            face.uv_coord_system = UvCoordSystem::Parallel { u_axis, v_axis };
            face.uv_attributes.scale = Vec2f::new(0.5, -0.75);
            face.rotate_uv(37.0);
            face.shear_uv(Vec2f::new(0.3, -0.2));
            face.move_uv(Vec2f::new(13.0, -7.0));
        });
        let original = d.brush(id).unwrap().clone();
        let clipboard = d.copy_selection().unwrap();
        assert_eq!(d.paste_at(&clipboard, None), 1);
        for face in d.selected_brushes()[0].faces() {
            let source = original
                .faces()
                .iter()
                .find(|f| vec::dot(f.normal(), face.normal()) > 0.999)
                .unwrap();
            for point in face.points {
                geometry_math::assert_approx_eq!(
                    face.uv_coords(point),
                    source.uv_coords(point),
                    1e-3
                );
            }
        }
        d.undo().unwrap();
        let source = FaceRef { node: id, face: 0 };
        let target = FaceRef {
            node: d.selectable_brushes()[1].0,
            face: 2,
        };
        d.select_face_only(source);
        let clipboard = d.copy_face_attributes_text().unwrap();
        d.copy_face_attributes(source, target, FaceAttributeTransfer::Projection, false);
        let expected = d.face_at(target).unwrap().clone();
        d.undo().unwrap();
        assert_eq!(d.paste_face_attributes_on(&clipboard, &[target]), 1);
        let pasted = d.face_at(target).unwrap();
        for point in pasted.points {
            geometry_math::assert_approx_eq!(
                pasted.uv_coords(point),
                expected.uv_coords(point),
                1e-3
            );
        }
    }
}

#[test]
fn texture_transfer_replaces_pending_uv_only_when_copying_the_mapping() {
    for mode in [
        FaceAttributeTransfer::MaterialOnly,
        FaceAttributeTransfer::Projection,
        FaceAttributeTransfer::Rotation,
    ] {
        let mut d = scene();
        let nodes = d.selectable_brushes();
        let source = FaceRef {
            node: nodes[0].0,
            face: 0,
        };
        let target = FaceRef {
            node: nodes[1].0,
            face: 2,
        };
        d.select_face_only(source);
        d.edit_faces(|f| {
            f.material_name = "stone".into();
            f.rotate_uv(37.0);
            f.move_uv(geometry_math::vec::Vec2f::new(13.0, -7.0));
        });
        d.select_face_only(target);
        d.edit_faces(|f| f.primitive_uv = Some(Box::new([[0.02, 0.01, 0.3], [0.0, 0.04, -0.2]])));
        let before = d.to_map_text();
        assert!(d.copy_face_attributes(source, target, mode, false));
        let copied = d.face_at(target).unwrap();
        assert_eq!(
            copied.primitive_uv.is_some(),
            mode == FaceAttributeTransfer::MaterialOnly
        );
        if mode != FaceAttributeTransfer::MaterialOnly {
            let mut discovered = copied.clone();
            discovered.resolve_primitive_uv(geometry_math::vec::Vec2f::new(128.0, 64.0));
            assert_eq!(discovered.uv_attributes, copied.uv_attributes);
            assert_eq!(discovered.uv_coord_system, copied.uv_coord_system);
        }
        d.undo().unwrap();
        assert_eq!(d.to_map_text(), before);
    }
}

#[test]
fn pending_texture_transfer_matches_resolving_the_source_first() {
    use geometry_math::vec::Vec2f;
    for mode in [
        FaceAttributeTransfer::Projection,
        FaceAttributeTransfer::Rotation,
    ] {
        let mut d = scene();
        let turn = geometry_math::mat_ext::rotation_matrix_about_axis(
            vec::normalize(Vec3d::new(1.0, 2.0, 3.0)),
            0.63,
        );
        assert!(d.transform_selection(&turn));
        let nodes = d.selectable_brushes();
        let source = FaceRef {
            node: nodes[0].0,
            face: 0,
        };
        let target = FaceRef {
            node: nodes[1].0,
            face: 3,
        };
        d.select_face_only(source);
        d.edit_faces(|f| {
            f.material_name = "pending_mapping".into();
            f.primitive_uv = Some(Box::new([[0.02, 0.01, 0.3], [-0.01, 0.04, -0.2]]));
        });
        let before = d.to_map_text();
        let size = Vec2f::new(128.0, 64.0);
        let mut resolved = d.face_at(source).unwrap().clone();
        resolved.resolve_primitive_uv(size);
        assert_eq!(d.set_faces_from(&resolved, &[target], mode), 1);
        let expected = d.face_at(target).unwrap().clone();
        d.undo().unwrap();
        assert!(d.copy_face_attributes(source, target, mode, false));
        let mut actual = d.face_at(target).unwrap().clone();
        assert!(actual.primitive_uv.is_some());
        actual.resolve_primitive_uv(size);
        for point in actual.points {
            geometry_math::assert_approx_eq!(
                actual.uv_coords(point),
                expected.uv_coords(point),
                1e-3
            );
        }
        d.undo().unwrap();
        if mode == FaceAttributeTransfer::Projection {
            d.select_face_only(source);
            let clipboard = d.copy_face_attributes_text().unwrap();
            assert_eq!(d.paste_face_attributes_on(&clipboard, &[target]), 1);
            let mut actual = d.face_at(target).unwrap().clone();
            assert!(actual.primitive_uv.is_some());
            actual.resolve_primitive_uv(size);
            for point in actual.points {
                geometry_math::assert_approx_eq!(
                    actual.uv_coords(point),
                    expected.uv_coords(point),
                    1e-3
                );
            }
            d.undo().unwrap();
        }
        assert_eq!(d.to_map_text(), before);
    }
}

#[test]
fn appearance_edits_keep_complex_brush_geometry_and_face_indices() {
    use geometry_math::vec::Vec2f;
    use map_model::brush_face::{FaceColor, Rgb};
    let bounds = Document::new(Default::default()).world_bounds();
    let brush = map_model::shapes::create_cylinder(
        &bounds,
        &Bbox3d::from_scalars(-128.0, 128.0),
        map_model::shapes::CircleShape::EdgeAligned { num_sides: 64 },
        Axis::Z,
        "stone",
    )
    .unwrap();
    let mut faces = brush.faces().to_vec();
    faces[0].material_name = "source".into();
    faces[0].rotate_uv(37.0);
    let brush = brush.with_exact_face_attributes(&faces).unwrap();
    for operation in 0..7 {
        let mut d = Document::new(Default::default());
        d.add_brushes(vec![brush.clone()]);
        let id = d.selectable_brushes()[0].0;
        d.select_only(id);
        let before = d.to_map_text();
        let source_face = brush.geometry_face_of(0).unwrap().0 as usize;
        let target = FaceRef {
            node: id,
            face: (source_face + 1) % brush.geometry().face_count(),
        };
        let source = FaceRef {
            node: id,
            face: source_face,
        };
        match operation {
            0 => d.edit_faces(|f| f.move_uv(Vec2f::new(8.0, 16.0))),
            1 => {
                assert_eq!(
                    d.set_face_color_on(&[target], Some(FaceColor::solid(Rgb::new(30, 80, 150)))),
                    1
                );
            }
            2 => {
                assert!(d.paint_face(target, "metal"));
            }
            3 => {
                assert!(d.copy_face_attributes(
                    source,
                    target,
                    FaceAttributeTransfer::Projection,
                    false
                ));
            }
            4 => {
                assert_eq!(
                    d.copy_face_attributes_across(
                        source,
                        &[target],
                        FaceAttributeTransfer::Projection
                    ),
                    1
                );
            }
            5 => {
                assert!(d.replace_material("stone", "metal") > 0);
            }
            _ => {
                d.set_surface_material(Some(map_model::brush_face::SurfaceMaterial::default()));
            }
        }
        let after = d.to_map_text();
        assert_ne!(after, before, "operation {operation} made no edit");
        let changed = d.brush(id).unwrap();
        assert_eq!(
            changed.vertex_positions(),
            brush.vertex_positions(),
            "operation {operation}"
        );
        for index in 0..brush.geometry().face_count() {
            assert_eq!(
                changed.source_face_index(index),
                brush.source_face_index(index)
            );
        }
        d.undo().unwrap();
        assert_eq!(d.to_map_text(), before);
        d.redo().unwrap();
        assert_eq!(d.to_map_text(), after);
    }
}

#[test]
fn face_edit_that_moves_a_plane_still_rebuilds_geometry() {
    let mut d = scene();
    let id = d.selectable_brushes()[0].0;
    d.select_only(id);
    let before = d.to_map_text();
    let old_max = d.brush(id).unwrap().bounds().max.x();
    d.edit_faces(|face| {
        if face.normal().x() > 0.9 {
            for point in &mut face.points {
                point[0] += 8.0;
            }
            face.plane.distance += 8.0;
        }
    });
    assert_eq!(d.brush(id).unwrap().bounds().max.x(), old_max + 8.0);
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
}

#[test]
fn duplicating_mixed_group_preserves_entity_properties_patch_uvs_and_one_undo() {
    use geometry_math::vec::Vec2f;
    use map_model::io::map_reader::{Patch, PatchControlPoint};
    let original = scene();
    let mut world = original.tree_world().clone();
    let brushes = std::mem::take(&mut world.layers[0].children);
    let entity = Node::new(NodeKind::Entity(map_model::node::EntityNode {
        properties: vec![
            ("classname".into(), "func_detail".into()),
            ("custom".into(), "kept".into()),
        ],
        children: brushes,
    }));
    let point = Node::new(NodeKind::Entity(map_model::node::EntityNode {
        properties: vec![
            ("classname".into(), "info_player_start".into()),
            ("origin".into(), "16 32 48".into()),
            ("custom".into(), "kept".into()),
        ],
        children: Vec::new(),
    }));
    let patch = Patch {
        material_name: "patch_material".into(),
        rows: 3,
        columns: 3,
        id: None,
        line: 0,
        control_points: (0..9)
            .map(|i| PatchControlPoint {
                position: Vec3d::new((i % 3) as f64 * 16.0, (i / 3) as f64 * 16.0, 8.0),
                uv: Vec2f::new(i as f32 * 0.1, i as f32 * 0.2),
            })
            .collect(),
    };
    let patch_id = NodeId::next();
    let inner = Node::new(NodeKind::Group(Group {
        id: "100".into(),
        name: "Inner".into(),
        linked_group_id: None,
        transformation: None,
        children: vec![entity, point, patch_node(patch_id, patch.clone())],
    }));
    let root = Node::new(NodeKind::Group(Group {
        id: "101".into(),
        name: "Outer".into(),
        linked_group_id: None,
        transformation: None,
        children: vec![inner],
    }));
    let root_id = root.id;
    world.layers[0].children = vec![root];
    let mut d = Document::from_world(
        world,
        Default::default(),
        map_model::map_format::MapFormat::Valve,
        None,
    );
    d.selection = node::contents_of(&d.tree.world, root_id)
        .into_iter()
        .collect();
    let old_ids = node::all_node_ids(&d.tree.world);
    let before = d.to_map_text();
    let delta = Vec3d::new(0.0, 128.0, 64.0);
    assert_eq!(d.duplicate_selection_by(delta), 5);
    assert!(d.selection_ids().iter().all(|id| !old_ids.contains(id)));
    let copied_patch = node::patches_by_layer(&d.tree.world)
        .into_iter()
        .find(|(id, _, _)| *id != patch_id)
        .unwrap()
        .1;
    for (actual, old) in copied_patch
        .control_points
        .iter()
        .zip(&patch.control_points)
    {
        assert_eq!(actual.position, old.position + delta);
        assert_eq!(actual.uv, old.uv);
    }
    for (id, entity) in node::entities_with_ids(&d.tree.world) {
        assert_eq!(entity.property("custom"), Some("kept"));
        if d.selection_ids().contains(&id) && entity.children.is_empty() {
            assert_eq!(entity.origin(), Some(Vec3d::new(16.0, 32.0, 48.0) + delta));
        }
    }
    assert_eq!(node::groups_with_ids(&d.tree.world).len(), 4);
    let after = d.to_map_text();
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.redo().unwrap();
    assert_eq!(d.to_map_text(), after);
}

#[test]
fn converting_selection_uses_geometry_faces_and_skips_redundant_planes() {
    let mut d = scene();
    let brush = node::brushes_with_ids(d.tree_world())[0].1;
    let mut faces = brush.faces().to_vec();
    let mut redundant = faces[0].clone();
    let offset = redundant.plane.normal * 128.0;
    redundant.plane.distance += 128.0;
    for point in &mut redundant.points {
        *point += offset;
    }
    faces.insert(0, redundant);
    let brush = Brush::from_faces(&d.world_bounds(), faces).unwrap();
    assert_eq!(brush.faces().len(), 7);
    assert_eq!(brush.geometry().face_count(), 6);
    d.add_brushes(vec![brush]);
    d.select_all();
    let expected: HashSet<_> = node::brushes_with_ids(d.tree_world())
        .into_iter()
        .flat_map(|(node, brush)| {
            (0..brush.geometry().face_count()).map(move |face| FaceRef { node, face })
        })
        .collect();
    assert_eq!(d.convert_selection_to_faces(), expected.len());
    assert!(!d.has_selection());
    let actual: HashSet<_> = d.selected_faces().into_iter().collect();
    assert_eq!(actual, expected);
    assert!(actual.iter().all(|face| d.face_at(*face).is_some()));
    let before = d.to_map_text();
    d.edit_faces(|face| face.material_name = "painted".into());
    for (id, brush) in node::brushes_with_ids(d.tree_world()) {
        for face in 0..brush.geometry().face_count() {
            assert_eq!(
                d.face_at(FaceRef { node: id, face })
                    .unwrap()
                    .material_name
                    .as_str(),
                "painted"
            );
        }
    }
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
}

#[test]
fn audit_transaction_records_export_visibility_and_heretic_lighting() {
    for lighting in [false, true] {
        let mut d = scene();
        if lighting {
            d.edit_faces(|face| {
                face.surface_attributes = Some(map_model::uv::SurfaceAttributes {
                    surface_flags: 0x400,
                    ..Default::default()
                })
            });
        }
        let before = d.to_map_text();
        d.begin_transaction();
        d.edit_faces(|face| {
            if lighting {
                face.heretic2_lighting = Some(Box::new([0.2, 0.3, 0.4, 0.5]));
            } else {
                face.export_hidden = true;
            }
        });
        assert!(d.commit_transaction());
        let after = d.to_map_text();
        assert_ne!(before, after);
        d.undo().unwrap();
        assert_eq!(d.to_map_text(), before);
        d.redo().unwrap();
        assert_eq!(d.to_map_text(), after);
        d.begin_transaction();
        d.edit_faces(|face| {
            if lighting {
                face.heretic2_lighting = None;
            } else {
                face.export_hidden = false;
            }
        });
        d.cancel_transaction();
        assert_eq!(d.to_map_text(), after);
    }
}

#[test]
fn audit_combined_document_edit_undoes_once_at_history_capacity() {
    let mut d = scene();
    for _ in 0..crate::history::CAPACITY {
        assert!(d.translate_selection(Vec3d::new(1.0, 0.0, 0.0)));
    }
    let before = d.to_map_text();
    d.run_as_one_edit(|d| {
        assert!(d.translate_selection(Vec3d::new(4.0, 0.0, 0.0)));
        assert!(d.translate_selection(Vec3d::new(0.0, 8.0, 0.0)));
    });
    let after = d.to_map_text();
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.redo().unwrap();
    assert_eq!(d.to_map_text(), after);
}

fn next_audit_patch() -> map_model::io::map_reader::Patch {
    use map_model::io::map_reader::{Patch, PatchControlPoint};
    Patch {
        material_name: "stone".into(),
        rows: 3,
        columns: 3,
        control_points: (0..9)
            .map(|i| PatchControlPoint {
                position: Vec3d::new(14.0 + (i % 3) as f64, (i / 3) as f64, 0.0),
                uv: geometry_math::vec::Vec2f::zero(),
            })
            .collect(),
        id: None,
        line: 0,
    }
}
fn next_audit_patch_document() -> Document {
    let d = Document::new(crate::world::WorldBounds::new(16.0));
    let mut world = d.tree_world().clone();
    let brush = map_model::shapes::create_cuboid(
        &d.world_bounds(),
        &Bbox3d::from_scalars(0.0, 2.0),
        "stone",
    )
    .unwrap();
    world.layers[0].children = vec![
        Node::new(NodeKind::Brush(brush)),
        Node::new(NodeKind::Patch(next_audit_patch())),
    ];
    let mut d = Document::from_world(
        world,
        crate::world::WorldBounds::new(16.0),
        map_model::map_format::MapFormat::Valve,
        None,
    );
    d.select_all();
    d
}

#[test]
fn next_audit_hidden_and_locked_layers_drop_patches_from_selection() {
    for locked in [false, true] {
        let mut d = next_audit_patch_document();
        assert_eq!(d.selection_count(), 2);
        if locked {
            assert!(d.set_layer_locked(0, true));
        } else {
            assert!(d.set_layer_hidden(0, true));
        }
        assert_eq!(d.selection_count(), 0);
        let before = d.to_map_text();
        assert_eq!(
            d.resize_selected_patches(map_model::patch_edit::add_rows),
            0
        );
        assert!(!d.translate_selection(Vec3d::new(-1.0, 0.0, 0.0)));
        assert_eq!(d.to_map_text(), before);
    }
}

#[test]
fn next_audit_invalid_patch_transforms_are_atomic_and_valid_moves_undo() {
    let mut d = next_audit_patch_document();
    let before = d.to_map_text();
    let revision = d.revision();
    let undo = d.history.undo_len();
    // The brush can move right; only the patch at the boundary would leave the world.
    assert!(!d.translate_selection(Vec3d::new(1.0, 0.0, 0.0)));
    assert!(!d.translate_selection(Vec3d::new(f64::NAN, 0.0, 0.0)));
    assert!(
        !d.transform_selection(&geometry_math::mat_ext::translation_matrix(Vec3d::new(
            1.0, 0.0, 0.0
        )))
    );
    let mut invalid = Mat4x4d::identity();
    invalid.v[0].v[0] = f64::INFINITY;
    assert!(!d.transform_selection(&invalid));
    assert_eq!(d.to_map_text(), before);
    assert_eq!(d.revision(), revision);
    assert_eq!(d.history.undo_len(), undo);
    let patch = node::patches_by_layer(d.tree_world())[0].0;
    d.select_only(patch);
    assert!(
        !d.transform_selection(&geometry_math::mat_ext::scaling_matrix(Vec3d::new(
            f64::MAX,
            1.0,
            1.0
        )))
    );
    assert_eq!(d.to_map_text(), before);
    d.select_all();
    assert!(d.translate_selection(Vec3d::new(-4.0, 0.0, 0.0)));
    let after = d.to_map_text();
    assert_ne!(after, before);
    assert!(crate::collab::wire::WireWorld::from_world(d.tree_world())
        .into_world(&d.world_bounds())
        .is_ok());
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
    d.redo().unwrap();
    assert_eq!(d.to_map_text(), after);
}

fn next_audit_clipboard_document() -> (Document, NodeId) {
    let d = Document::new(Default::default());
    let mut world = d.tree_world().clone();
    let brush = map_model::shapes::create_cuboid(
        &d.world_bounds(),
        &Bbox3d::from_scalars(0.0, 8.0),
        "stone",
    )
    .unwrap();
    let child = Node::new(NodeKind::Brush(brush.clone()));
    let child_id = child.id;
    let entity = Node::new(NodeKind::Entity(node::EntityNode {
        properties: vec![
            ("classname".into(), "func_door".into()),
            ("targetname".into(), "door_a".into()),
        ],
        children: vec![child, Node::new(NodeKind::Brush(brush.clone()))],
    }));
    let point = Node::new(NodeKind::Entity(node::EntityNode {
        properties: vec![
            ("classname".into(), "light".into()),
            ("origin".into(), "1 2 3".into()),
        ],
        children: vec![],
    }));
    world.layers[0].children = vec![Node::new(NodeKind::Group(Group {
        id: "clipboard-group".into(),
        name: "Clipboard".into(),
        linked_group_id: None,
        transformation: None,
        children: vec![
            entity,
            Node::new(NodeKind::Brush(brush)),
            Node::new(NodeKind::Patch(next_audit_patch())),
            point,
        ],
    }))];
    let mut d = Document::from_world(
        world,
        Default::default(),
        map_model::map_format::MapFormat::Valve,
        None,
    );
    d.select_only(child_id);
    (d, child_id)
}

#[test]
fn next_audit_copy_and_cut_grouped_entities_do_not_duplicate_their_contents() {
    let (mut d, _) = next_audit_clipboard_document();
    let before = d.to_map_text();
    let text = d.cut_selection().unwrap();
    assert_eq!(d.brush_count(), 0);
    let mut pasted = Document::new(Default::default());
    assert!(pasted.paste_at(&text, None) > 0);
    assert_eq!(pasted.brush_count(), 3);
    assert_eq!(node::patches_by_layer(pasted.tree_world()).len(), 1);
    let entities = node::entities_with_ids(pasted.tree_world());
    assert_eq!(entities.len(), 2);
    let door = entities
        .iter()
        .find(|(_, e)| e.classname() == Some("func_door"))
        .unwrap()
        .1;
    assert_eq!(door.property("targetname"), Some("door_a"));
    assert_eq!(door.children.len(), 2);
    d.undo().unwrap();
    assert_eq!(d.to_map_text(), before);
}

#[test]
fn next_audit_copy_partial_brush_entity_preserves_properties_and_only_selected_children() {
    let (mut d, child) = next_audit_clipboard_document();
    assert!(d.open_selected_group());
    d.select_only(child);
    assert_eq!(d.selection_count(), 1);
    let text = d.copy_selection().unwrap();
    let mut pasted = Document::new(Default::default());
    assert!(pasted.paste_at(&text, None) > 0);
    assert_eq!(pasted.brush_count(), 1);
    let entities = node::entities_with_ids(pasted.tree_world());
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].1.classname(), Some("func_door"));
    assert_eq!(entities[0].1.property("targetname"), Some("door_a"));
    assert_eq!(entities[0].1.children.len(), 1);
}
