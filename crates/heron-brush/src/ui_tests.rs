//! Headless frames exercise frontend borrow lifetimes and ensure repainting never edits maps.
use crate::{
    editor::Editor,
    inspector::{Inspector, Tab},
};
use eframe::egui;
fn setup() -> (egui::Context, Editor) {
    editor_core::preferences::set_current(Default::default());
    let ctx = egui::Context::default();
    ctx.set_embed_viewports(true);
    crate::theme::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    let editor = Editor::open(&ctx, None);
    *editor.document.borrow_mut() = editor_core::document::Document::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map"),
        Default::default(),
    )
    .unwrap();
    (ctx, editor)
}
fn input() -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1000.0),
        )),
        ..Default::default()
    }
}
#[test]
fn inspector_frames_do_not_modify_selected_or_unselected_geometry() {
    let (ctx, editor) = setup();
    let mut inspector = Inspector::default();
    for selected in [false, true] {
        if selected {
            editor.document.borrow_mut().select_all();
        } else {
            editor.document.borrow_mut().clear_selection();
        }
        let before = editor.document.borrow().to_map_text();
        for tab in [Tab::Map, Tab::Entity, Tab::Face, Tab::Assets] {
            inspector.tab = tab;
            for _ in 0..2 {
                let mut output = ctx.run_ui(input(), |ui| {
                    inspector.show(
                        ui,
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(460.0, 1000.0)),
                        &editor,
                    )
                });
                output.textures_delta.clear();
            }
        }
        assert_eq!(editor.document.borrow().to_map_text(), before);
    }
}
#[test]
fn dialogs_can_render_together_without_editing_the_document() {
    let (ctx, editor) = setup();
    editor.document.borrow_mut().select_all();
    let mut draft = editor_core::animation_editor::Draft::read(&editor.document.borrow()).unwrap();
    draft.commit(&mut editor.document.borrow_mut()).unwrap();
    let before = editor.document.borrow().to_map_text();
    let mut animation = crate::animation_page::AnimationPage::default();
    animation.open = true;
    let mut shapes = crate::shape_dialogs::ShapeDialogs::default();
    shapes.text_open = true;
    shapes.corner(&editor);
    let mut preferences = crate::preferences_page::PreferencesPage::default();
    preferences.open = true;
    let mut plugin_cards = crate::plugin_page::Cards::default();
    let mut plugins = crate::plugin_page::PluginPage::default();
    plugins.open();
    let mut export = crate::export_page::ExportPage::default();
    export.open = true;
    let live = editor_core::godot_live::LiveLink::new(&editor.document);
    let mut render = crate::render_page::RenderPage::default();
    render.open(&editor);
    let mut session = crate::session_page::SessionPage::default();
    session.open = true;
    let mut state = editor_core::mcp::Collab::default();
    let mut manual = crate::manual_page::ManualPage::default();
    manual.open = true;
    for _ in 0..3 {
        let mut output = ctx.run_ui(input(), |ui| {
            animation.show(ui.ctx(), &editor);
            shapes.show(ui.ctx(), &editor);
            preferences.show(ui.ctx());
            manual.show(ui.ctx());
            plugins.show(ui.ctx(), &editor, &mut plugin_cards);
            export.show(ui.ctx(), &editor, &live);
            render.show(ui.ctx(), &editor);
            session.show(ui.ctx(), &editor, &mut state);
        });
        output.textures_delta.clear();
    }
    assert_eq!(editor.document.borrow().to_map_text(), before);
}

#[test]
fn docked_animation_repaints_without_changing_the_map() {
    let (ctx, editor) = setup();
    editor.document.borrow_mut().select_all();
    let before = editor.document.borrow().to_map_text();
    let mut page = crate::animation_page::AnimationPage::default();
    page.open = true;
    for _ in 0..3 {
        let mut output = ctx.run_ui(input(), |ui| page.show_docked(ui, &editor));
        output.textures_delta.clear();
    }
    assert_eq!(editor.document.borrow().to_map_text(), before);
}
/// A move drag in a flat view at a slow frame rate, cut short by the window losing focus.
/// The move lands as one undoable step, and the history keeps working afterwards.
#[test]
fn a_slow_drag_that_loses_focus_moves_once_and_undoes() {
    let (ctx, editor) = setup();
    let mut pane = crate::view2d::Pane2d::new(&editor, editor_core::camera2d::ViewPlane::Top);
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 600.0));
    let viewport = (600.0, 600.0);
    // Somewhere over a brush, found by asking the view what is under each point.
    let start = (0..60)
        .flat_map(|i| (0..60).map(move |j| egui::pos2(i as f32 * 10.0, j as f32 * 10.0)))
        .find(|p| pane.flat.pick_node(f64::from(p.x), f64::from(p.y), viewport).is_some())
        .expect("a brush under some point of the view");
    let flat = std::rc::Rc::clone(&pane.flat);
    let mut time = 0.0;
    let mut frame = |events: Vec<egui::Event>, focused: bool| {
        // Five frames a second: too few samples for egui's `is_moving`.
        time += 0.2;
        let raw = egui::RawInput { time: Some(time), focused, events, ..input() };
        let mut output = ctx.run_ui(raw, |ui| pane.show(ui, rect));
        output.textures_delta.clear();
    };
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(vec![egui::Event::PointerMoved(start)], true);
    // A click selects the brush, then a press on it starts the move.
    frame(vec![button(start, true)], true);
    frame(vec![button(start, false)], true);
    let before_move = editor.document.borrow().to_map_text();
    // Two steps of one 16-unit grid cell each, at the zoom the first frame framed the map at.
    let step = egui::vec2((16.0 * flat.state.borrow().camera.zoom) as f32, 0.0);
    frame(vec![button(start, true)], true);
    frame(vec![egui::Event::PointerMoved(start + step)], true);
    frame(vec![egui::Event::PointerMoved(start + step * 2.0)], true);
    let moved = editor.document.borrow().to_map_text();
    assert_ne!(moved, before_move, "the drag moved the brush while the button was held");
    frame(vec![], false);
    frame(vec![button(start + step * 2.5, false)], true);
    assert_eq!(editor.document.borrow().to_map_text(), moved, "focus loss keeps the move");
    assert!(!editor.document.borrow().in_transaction(), "focus loss closes the move's transaction");
    assert!(editor.document.borrow_mut().undo().is_some());
    assert_eq!(editor.document.borrow().to_map_text(), before_move);
}

/// In a flat view in pose mode, dragging the animated body keys its position without
/// Auto key; a click that stays within three pixels authors nothing; one drag undoes once.
#[test]
fn a_flat_view_drags_the_animated_body_without_auto_key() {
    let (ctx, editor) = setup();
    editor.document.borrow_mut().select_all();
    let mut draft = editor_core::animation_editor::Draft::read(&editor.document.borrow()).unwrap();
    draft.commit(&mut editor.document.borrow_mut()).unwrap();
    {
        let mut f = editor.filters.borrow_mut();
        f.animation_frame = Some(30);
        f.animation_fps = 30;
    }
    let mut pane = crate::view2d::Pane2d::new(&editor, editor_core::camera2d::ViewPlane::Top);
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 600.0));
    let pose = editor_core::animation_pose::Pose::selected(&editor.document.borrow(), 1.0).unwrap();
    let mut time = 0.0;
    let mut frame = |events: Vec<egui::Event>| {
        time += 0.05;
        let raw = egui::RawInput { time: Some(time), events, ..input() };
        let mut output = ctx.run_ui(raw, |ui| pane.show(ui, rect));
        output.textures_delta.clear();
    };
    frame(vec![]);
    let camera = pane_camera(&editor);
    let c = camera.world_to_screen(pose.centre(), (600.0, 600.0));
    let start = egui::pos2(c.x() as f32, c.y() as f32);
    let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    let before = editor.document.borrow().to_map_text();
    frame(vec![egui::Event::PointerMoved(start), button(start, true)]);
    frame(vec![egui::Event::PointerMoved(start + egui::vec2(2.0, 0.0))]);
    frame(vec![button(start + egui::vec2(2.0, 0.0), false)]);
    assert_eq!(editor.document.borrow().to_map_text(), before, "a click authors nothing");
    let end = start + egui::vec2(200.0, 0.0);
    frame(vec![egui::Event::PointerMoved(start), button(start, true)]);
    frame(vec![egui::Event::PointerMoved(start + egui::vec2(100.0, 0.0))]);
    frame(vec![egui::Event::PointerMoved(end)]);
    frame(vec![button(end, false)]);
    let moved = editor_core::animation_pose::Pose::selected(&editor.document.borrow(), 1.0).unwrap();
    assert!(moved.clip.sample(0, 1.0) > 0.0, "keyed along X");
    assert_eq!(moved.clip.sample(1, 1.0), 0.0);
    assert!(!editor.document.borrow().in_transaction());
    editor.document.borrow_mut().undo();
    assert_eq!(editor.document.borrow().to_map_text(), before);
}
fn pane_camera(editor: &Editor) -> editor_core::camera2d::Camera2d {
    editor.cameras.borrow().last().unwrap().borrow().camera
}
/// The opacity rule: the opacity box sets what the next paint goes on with and leaves a
/// solid face alone, but a face wearing a gradient follows it live, as one undoable step.
#[test]
fn opacity_repaints_gradients_live_but_not_solid_faces() {
    use map_model::brush_face::{FaceColor, Rgb};
    let (ctx, editor) = setup();
    let mut page = crate::face_tools::ColourPage::default();
    let frame = |page: &mut crate::face_tools::ColourPage| {
        let mut output = ctx.run_ui(input(), |ui| page.show(ui, &editor));
        output.textures_delta.clear();
    };
    editor.document.borrow_mut().select_all();
    editor.document.borrow_mut().set_face_color(Some(FaceColor::solid(Rgb::new(212, 52, 52))));
    frame(&mut page);
    let solid = editor.document.borrow().to_map_text();
    page.typed_opacity = Some(50.0);
    frame(&mut page);
    frame(&mut page);
    assert_eq!(editor.document.borrow().to_map_text(), solid, "a solid face keeps its opacity until the next paint");

    let gradient = crate::face_tools::GradientDraft::default().paint(255);
    editor.document.borrow_mut().set_face_color(Some(gradient));
    frame(&mut page);
    let before = editor.document.borrow().to_map_text();
    page.typed_opacity = Some(50.0);
    frame(&mut page);
    let painted = editor.document.borrow().first_selected_face_color().unwrap();
    assert!(matches!(painted, FaceColor::MultiGradient { .. }));
    assert_eq!(painted.alpha(), 128, "the gradient follows the opacity");
    frame(&mut page);
    editor.document.borrow_mut().undo();
    assert_eq!(editor.document.borrow().to_map_text(), before, "one step undoes it");
}

/// The sweep page says what the sweep would make before it is made.
#[test]
fn sweep_preview_describes_the_pending_sweep() {
    let (_ctx, editor) = setup();
    let mut d = editor.document.borrow_mut();
    let id = map_model::node::brushes_with_ids(d.tree_world())[0].0;
    d.select_face_only(editor_core::document::FaceRef { node: id, face: 0 });
    let source = d.sweep_source().expect("a face to sweep");
    let mut inputs = editor_core::sweep_tool::SweepInputs::default();
    assert_eq!(crate::face_tools::sweep_preview(&source, &inputs, "x"), "The sweep goes nowhere");
    inputs.transform.translation = geometry_math::vec::Vec3d::new(64.0, 64.0, 64.0);
    inputs.parameters.segments = 4;
    inputs.parameters.path_mode = editor_core::sweep_tool::SweepPathMode::Straight;
    let text = crate::face_tools::sweep_preview(&source, &inputs, "x");
    assert!(text.starts_with("4 brushes, ") && text.matches(" × ").count() == 2, "{text}");
}

/// The fillet panel's Round edges button names what it would round, and the result of a
/// press stays in the panel until what is picked changes.
#[test]
fn fillet_panel_reports_in_the_panel() {
    let (ctx, editor) = setup();
    let mut panel = crate::face_tools::FilletPanel::default();
    assert_eq!(crate::face_tools::FilletWork::of(&editor), crate::face_tools::FilletWork::Nothing);
    let id = map_model::node::brushes_with_ids(editor.document.borrow().tree_world())[0].0;
    editor.document.borrow_mut().select_face_only(editor_core::document::FaceRef { node: id, face: 0 });
    let work = crate::face_tools::FilletWork::of(&editor);
    assert_eq!(work, crate::face_tools::FilletWork::FaceLoops(1));
    assert_eq!(work.describe(), "The loop around 1 selected face");
    let mut output = ctx.run_ui(input(), |ui| panel.show(ui, &editor));
    output.textures_delta.clear();
}

/// Every inspector tab, with nothing, a brush and a face selected, and every dialog,
/// render in the light scheme at the full and at a small window size without panicking
/// or editing the map.
#[test]
fn every_tab_and_dialog_renders_in_the_light_scheme() {
    let (ctx, editor) = setup();
    crate::theme::use_scheme(&ctx, false);
    assert!(!crate::theme::is_dark());
    assert!(!ctx.global_style().visuals.dark_mode);
    let sized = |w: f32, h: f32| egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))), ..Default::default() };
    let mut inspector = Inspector::default();
    let first_brush = editor.document.borrow().selectable_brushes().first().map(|(id, _)| *id);
    for (w, h) in [(1360.0, 760.0), (900.0, 600.0)] {
        for selection in 0..3 {
            {
                let mut document = editor.document.borrow_mut();
                document.clear_selection();
                match (selection, first_brush) {
                    (1, Some(id)) => document.toggle_selected(id),
                    (2, Some(id)) => document.select_face_only(editor_core::document::FaceRef { node: id, face: 0 }),
                    _ => {}
                }
            }
            let before = editor.document.borrow().to_map_text();
            for tab in [Tab::Map, Tab::Entity, Tab::Face, Tab::Assets] {
                inspector.tab = tab;
                for _ in 0..2 {
                    let mut output = ctx.run_ui(sized(w, h), |ui| inspector.show(ui, egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(340.0, h)), &editor));
                    output.textures_delta.clear();
                }
            }
            assert_eq!(editor.document.borrow().to_map_text(), before, "{w}x{h}, selection {selection}");
        }
        let mut animation = crate::animation_page::AnimationPage::default();
        animation.open = true;
        let mut shapes = crate::shape_dialogs::ShapeDialogs::default();
        shapes.text_open = true;
        shapes.corner(&editor);
        let mut preferences = crate::preferences_page::PreferencesPage::default();
        preferences.open = true;
        let mut plugin_cards = crate::plugin_page::Cards::default();
    let mut plugins = crate::plugin_page::PluginPage::default();
        plugins.open();
        let mut export = crate::export_page::ExportPage::default();
        export.open = true;
        let live = editor_core::godot_live::LiveLink::new(&editor.document);
        let mut render = crate::render_page::RenderPage::default();
        render.open(&editor);
        let mut session = crate::session_page::SessionPage::default();
        session.open = true;
        let mut state = editor_core::mcp::Collab::default();
        let mut manual = crate::manual_page::ManualPage::default();
        manual.open = true;
        let before = editor.document.borrow().to_map_text();
        for _ in 0..3 {
            let mut output = ctx.run_ui(sized(w, h), |ui| {
                animation.show_docked(ui, &editor);
                shapes.show(ui.ctx(), &editor);
                preferences.show(ui.ctx());
                manual.show(ui.ctx());
                plugins.show(ui.ctx(), &editor, &mut plugin_cards);
                export.show(ui.ctx(), &editor, &live);
                render.show(ui.ctx(), &editor);
                session.show(ui.ctx(), &editor, &mut state);
            });
            output.textures_delta.clear();
        }
        assert_eq!(editor.document.borrow().to_map_text(), before);
    }
}

#[test]
fn custom_binding_bypasses_blender_translation_and_respects_text_focus() {
    let (ctx,editor)=setup();
    let window=std::rc::Rc::new(std::cell::RefCell::new(crate::WindowState::default()));
    let commands=crate::commands_for(&editor,&ctx,&window);
    let mut preferences=editor_core::preferences::current();preferences.keymap=editor_core::preferences::Keymap::Blender;
    preferences.keyboard_bindings=r#"{"tool_brush":"F5","save":"F6"}"#.into();editor_core::preferences::set_current(preferences);
    let mut animation=crate::animation_page::AnimationPage::default();
    let event=|key|egui::Event::Key{key,physical_key:Some(key),pressed:true,repeat:false,modifiers:egui::Modifiers::NONE};
    crate::dispatch_keys(&commands,&window,&mut animation,&editor,vec![event(egui::Key::F5)],egui::Modifiers::NONE,None,true);
    assert_eq!(*editor.tool.borrow(),editor_core::tool::Tool::Select);
    crate::dispatch_keys(&commands,&window,&mut animation,&editor,vec![event(egui::Key::F5)],egui::Modifiers::NONE,None,false);
    assert_eq!(*editor.tool.borrow(),editor_core::tool::Tool::Brush);
    crate::dispatch_keys(&commands,&window,&mut animation,&editor,vec![event(egui::Key::F6)],egui::Modifiers::NONE,None,true);
    assert_eq!(window.borrow().action,Some(crate::app_actions::Action::Save));
    editor_core::preferences::set_current(Default::default());
}
