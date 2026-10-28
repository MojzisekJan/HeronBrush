use super::*;
fn add_box(d: &mut Document, min: Vec3d, max: Vec3d) {
    let brush =
        map_model::shapes::create_cuboid(&d.world_bounds(), &Bbox3d { min, max }, "stone").unwrap();
    let mut faces = brush.faces().to_vec();
    for face in &mut faces {
        face.set_paint(Some(map_model::brush_face::FaceColor::Solid {
            color: map_model::brush_face::Rgb::new(72, 158, 168),
            alpha: 255,
        }));
        face.surface_material = Some(map_model::brush_face::SurfaceMaterial {
            roughness: 45,
            metallic: 20,
            emission: 3,
        });
    }
    d.add_brushes(vec![brush.with_exact_face_attributes(&faces).unwrap()]);
}
#[test]
fn hollow_join_preserves_parent_and_one_undo_without_a_common_wall() {
    let mut d = Document::new(Default::default());
    add_box(&mut d, Vec3d::zero(), Vec3d::new(32.0, 64.0, 64.0));
    add_box(
        &mut d,
        Vec3d::new(32.0, 0.0, 0.0),
        Vec3d::new(64.0, 64.0, 64.0),
    );
    d.select_all();
    let parent = d.create_brush_entity("func_detail").unwrap();
    let before = d.to_map_text();
    assert_eq!(
        d.hollow_selection_options(8.0, map_model::hollow::Direction::Inward, &[], true)
            .unwrap(),
        2
    );
    let brushes = node::brushes(d.tree_world());
    assert_eq!(brushes.len(), 6);
    for face in brushes.iter().flat_map(|brush| brush.faces()) {
        assert_eq!(
            face.paint(),
            Some(map_model::brush_face::FaceColor::Solid {
                color: map_model::brush_face::Rgb::new(72, 158, 168),
                alpha: 255
            })
        );
        assert_eq!(
            face.surface_material,
            Some(map_model::brush_face::SurfaceMaterial {
                roughness: 45,
                metallic: 20,
                emission: 3
            })
        );
    }
    assert!(!brushes
        .iter()
        .any(|b| b.contains_point(Vec3d::new(32.0, 32.0, 32.0))));
    let NodeKind::Entity(entity) = &node::find_node(d.tree_world(), parent).unwrap().kind else {
        panic!()
    };
    assert_eq!(entity.children.len(), 6);
    let after = d.to_map_text();
    d.undo();
    assert_eq!(d.to_map_text(), before);
    d.redo();
    assert_eq!(d.to_map_text(), after);
    let reopened =
        Document::from_map_owned(read_map(&after, None).unwrap(), Default::default(), None);
    assert_eq!(node::brushes(reopened.tree_world()).len(), 6);
}
#[test]
fn hollow_and_bevel_reject_the_whole_selection_on_one_invalid_brush() {
    let mut d = Document::new(Default::default());
    add_box(&mut d, Vec3d::zero(), Vec3d::new(64.0, 64.0, 64.0));
    add_box(
        &mut d,
        Vec3d::new(100.0, 0.0, 0.0),
        Vec3d::new(108.0, 8.0, 8.0),
    );
    d.select_all();
    let before = d.to_map_text();
    let revision = d.revision();
    assert!(d
        .hollow_selection_options(8.0, map_model::hollow::Direction::Inward, &[], false)
        .is_err());
    assert_eq!(d.to_map_text(), before);
    assert_eq!(d.revision(), revision);
    assert!(d
        .hollow_selection_options(2.0, map_model::hollow::Direction::Inward, &[], true)
        .is_err());
    assert_eq!(d.to_map_text(), before);
    let faces: Vec<_> = node::brushes_with_ids(d.tree_world())
        .into_iter()
        .map(|(id, b)| FaceRef {
            node: id,
            face: b.faces().iter().position(|f| f.normal()[2] > 0.99).unwrap(),
        })
        .collect();
    assert!(d.bevel_faces_options(&faces, 8.0, 0.0, true).is_err());
    assert_eq!(d.to_map_text(), before);
    assert_eq!(d.revision(), revision);
}
#[test]
fn inner_fillet_has_preview_and_one_undo_with_unequal_wall_widths() {
    let mut d = Document::new(Default::default());
    add_box(&mut d, Vec3d::zero(), Vec3d::new(128.0, 8.0, 8.0));
    add_box(&mut d, Vec3d::zero(), Vec3d::new(24.0, 96.0, 8.0));
    d.select_all();
    let before = d.to_map_text();
    let parameters = crate::fillet_tool::FilletParameters {
        inner: true,
        radius: 32.0,
        faces: 8,
        ..Default::default()
    };
    assert!(!crate::fillet_tool::preview_edges(&d, &[], parameters).is_empty());
    assert_eq!(d.to_map_text(), before);
    let document = Rc::new(RefCell::new(d));
    let mut handles = crate::handle_tool::HandleManager::new();
    assert_eq!(
        crate::fillet_tool::apply(&document, &mut handles, parameters),
        crate::fillet_tool::Outcome::Rounded(2)
    );
    let mut d = document.borrow_mut();
    assert_eq!(node::brushes(d.tree_world()).len(), 10);
    let after = d.to_map_text();
    d.undo();
    assert_eq!(d.to_map_text(), before);
    d.redo();
    assert_eq!(d.to_map_text(), after);
}
#[test]
fn bevel_coplanar_faces_share_their_seam_across_brushes() {
    let mut d = Document::new(Default::default());
    add_box(&mut d, Vec3d::zero(), Vec3d::new(32.0, 64.0, 16.0));
    add_box(
        &mut d,
        Vec3d::new(32.0, 0.0, 0.0),
        Vec3d::new(64.0, 64.0, 16.0),
    );
    d.select_all();
    let faces: Vec<_> = node::brushes_with_ids(d.tree_world())
        .into_iter()
        .map(|(id, b)| FaceRef {
            node: id,
            face: b.faces().iter().position(|f| f.normal()[2] > 0.99).unwrap(),
        })
        .collect();
    let before = d.to_map_text();
    assert_eq!(d.bevel_faces_options(&faces, 4.0, 4.0, true).unwrap(), 2);
    for brush in node::brushes(d.tree_world()) {
        for y in [4.0, 60.0] {
            assert!(brush
                .vertex_positions()
                .iter()
                .any(|p| vec::length(*p - Vec3d::new(32.0, y, 20.0)) < 1e-6));
        }
    }
    d.undo();
    assert_eq!(d.to_map_text(), before);
}
#[test]
fn sweep_keeps_source_parent_and_is_one_undo_step() {
    let mut d = Document::new(Default::default());
    add_box(&mut d, Vec3d::zero(), Vec3d::new(16.0, 16.0, 16.0));
    let parent = d.create_brush_entity("func_detail").unwrap();
    let (id, brush) = node::brushes_with_ids(d.tree_world())
        .into_iter()
        .next()
        .unwrap();
    let face = brush
        .faces()
        .iter()
        .position(|f| f.normal()[2] > 0.99)
        .unwrap();
    d.face_selection.insert(FaceRef { node: id, face });
    let before = d.to_map_text();
    let transform = crate::sweep_tool::SweepTransform {
        translation: Vec3d::new(0.0, 0.0, 32.0),
        ..Default::default()
    };
    let parameters = crate::sweep_tool::SweepParameters {
        segments: 4,
        ..Default::default()
    };
    assert_eq!(
        d.sweep_selection_checked(&transform, &parameters).unwrap(),
        4
    );
    assert_eq!(node::brushes(d.tree_world()).len(), 5);
    assert_eq!(d.selection_ids().len(), 4);
    assert!(!d.selection_ids().contains(&id));
    let NodeKind::Entity(entity) = &node::find_node(d.tree_world(), parent).unwrap().kind else {
        panic!()
    };
    assert_eq!(entity.children.len(), 5);
    let after = d.to_map_text();
    d.undo();
    assert_eq!(d.to_map_text(), before);
    d.redo();
    assert_eq!(d.to_map_text(), after);
}
#[test]
fn sweep_and_fillet_face_selection_survive_reordered_source_planes() {
    let mut d = Document::new(Default::default());
    let original = map_model::shapes::create_cuboid(
        &d.world_bounds(),
        &Bbox3d::new(Vec3d::zero(), Vec3d::new(32.0, 32.0, 16.0)),
        "tile",
    )
    .unwrap();
    let mut faces = original.faces().to_vec();
    faces.reverse();
    d.add_brushes(vec![Brush::from_faces(&d.world_bounds(), faces).unwrap()]);
    let (id, brush) = node::brushes_with_ids(d.tree_world())
        .into_iter()
        .next()
        .unwrap();
    let face = brush
        .faces()
        .iter()
        .position(|f| f.normal()[2] > 0.99)
        .unwrap();
    d.face_selection.insert(FaceRef { node: id, face });
    let source = d.sweep_source().unwrap();
    assert!(source.normal[2] > 0.99);
    assert!(source.faces[0].iter().all(|p| (p[2] - 16.0).abs() < 1e-7));
    assert!(!crate::fillet_tool::preview_edges(&d, &[], Default::default()).is_empty());
    let before = d.to_map_text();
    assert_eq!(
        d.fillet_face_loops(&map_model::fillet::Fillet::default())
            .unwrap(),
        1
    );
    d.undo();
    assert_eq!(d.to_map_text(), before);
}
#[test]
fn outward_shells_cannot_silently_overlap_each_other() {
    let mut d = Document::new(Default::default());
    add_box(&mut d, Vec3d::zero(), Vec3d::new(32.0, 64.0, 64.0));
    add_box(
        &mut d,
        Vec3d::new(32.0, 0.0, 0.0),
        Vec3d::new(64.0, 64.0, 64.0),
    );
    d.select_all();
    let before = d.to_map_text();
    for mode in [
        map_model::hollow::Direction::Outward,
        map_model::hollow::Direction::Centered,
    ] {
        assert!(d.hollow_selection_options(4.0, mode, &[], false).is_err());
        assert_eq!(d.to_map_text(), before);
    }
    assert_eq!(
        d.hollow_selection_options(4.0, map_model::hollow::Direction::Outward, &[], true)
            .unwrap(),
        2
    );
    d.undo();
    assert_eq!(d.to_map_text(), before);
}

#[test]
fn snapped_fillet_preview_apply_save_and_one_undo_agree() {
    let mut d = Document::new(Default::default());
    add_box(&mut d, Vec3d::zero(), Vec3d::fill(64.0));
    d.select_all();
    let before = d.to_map_text();
    let edges = vec![(Vec3d::new(64.0, 0.0, 64.0), Vec3d::fill(64.0))];
    let parameters = crate::fillet_tool::FilletParameters {
        radius: 12.0,
        faces: 8,
        snap: true,
        grid: 2.0,
        ..Default::default()
    };
    let preview = crate::fillet_tool::surface_preview(&d, &edges, parameters).unwrap();
    assert_eq!(d.to_map_text(), before);
    d.fillet_edges(&edges, &parameters.shape()).unwrap();
    let brush = d.selected_brushes()[0];
    assert_eq!(
        brush.vertex_positions(),
        preview.brushes[0].1[0].vertex_positions()
    );
    assert!(brush.vertices_are_on_grid(2.0));
    assert!(brush
        .faces()
        .iter()
        .all(|f| f.color.is_some() && f.surface_material.is_some()));
    let after = d.to_map_text();
    let loaded =
        Document::from_map_owned(read_map(&after, None).unwrap(), Default::default(), None);
    for brush in node::brushes(loaded.tree_world()) {
        assert!(brush.vertices_are_on_grid(2.0));
    }
    d.undo();
    assert_eq!(d.to_map_text(), before);
    d.redo();
    assert_eq!(d.to_map_text(), after);
}

#[test]
fn refused_fillet_grid_leaves_all_brushes_and_history_unchanged() {
    let mut d = Document::new(Default::default());
    add_box(&mut d, Vec3d::zero(), Vec3d::fill(64.0));
    add_box(
        &mut d,
        Vec3d::new(256.0, 0.0, 0.0),
        Vec3d::new(320.0, 64.0, 64.0),
    );
    d.select_all();
    let before = d.to_map_text();
    let mark = d.edit_mark();
    let edges = vec![
        (Vec3d::new(64.0, 0.0, 64.0), Vec3d::fill(64.0)),
        (Vec3d::new(320.0, 0.0, 64.0), Vec3d::new(320.0, 64.0, 64.0)),
    ];
    assert!(d
        .fillet_edges(
            &edges,
            &map_model::fillet::Fillet {
                snap: Some(128.0),
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(d.to_map_text(), before);
    assert_eq!(d.edit_mark(), mark);
}

#[test]
fn inner_fillet_uses_its_own_grid() {
    let mut d = Document::new(Default::default());
    add_box(&mut d, Vec3d::zero(), Vec3d::new(128.0, 8.0, 8.0));
    add_box(&mut d, Vec3d::zero(), Vec3d::new(24.0, 96.0, 8.0));
    d.select_all();
    let before = d.to_map_text();
    let parameters = crate::fillet_tool::FilletParameters {
        inner: true,
        radius: 32.0,
        faces: 8,
        snap: true,
        grid: 2.0,
        ..Default::default()
    };
    let preview = crate::fillet_tool::surface_preview(&d, &[], parameters).unwrap();
    assert!(preview
        .brushes
        .iter()
        .flat_map(|(_, brushes)| brushes)
        .all(|b| b.vertices_are_on_grid(2.0)));
    let doc = Rc::new(RefCell::new(d));
    assert!(matches!(
        crate::fillet_tool::apply(
            &doc,
            &mut crate::handle_tool::HandleManager::new(),
            parameters
        ),
        crate::fillet_tool::Outcome::Rounded(_)
    ));
    assert!(node::brushes(doc.borrow().tree_world())
        .iter()
        .all(|b| b.vertices_are_on_grid(2.0)));
    doc.borrow_mut().undo();
    assert_eq!(doc.borrow().to_map_text(), before);
}
