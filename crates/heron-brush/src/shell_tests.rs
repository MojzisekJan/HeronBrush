//! Headless tests of the application shell: window shortcuts, the menus, Revert, Save
//! As, the window placement and the view layout.
use crate::editor::Editor;
use eframe::egui;

fn setup() -> (egui::Context, Editor) {
    editor_core::preferences::set_current(Default::default());
    let ctx = egui::Context::default();
    let editor = Editor::open(&ctx, None);
    *editor.document.borrow_mut() = editor_core::document::Document::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map"),
        Default::default(),
    )
    .unwrap();
    (ctx, editor)
}

/// The window side of the keyboard commands for a headless editor.
fn keyboard(
    ctx: &egui::Context,
    editor: &Editor,
) -> (editor_core::commands::Commands, std::rc::Rc<std::cell::RefCell<crate::WindowState>>, crate::animation_page::AnimationPage) {
    let window = std::rc::Rc::new(std::cell::RefCell::new(crate::WindowState::default()));
    (crate::commands_for(editor, ctx, &window), window, crate::animation_page::AnimationPage::default())
}

fn press(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }
}

/// Ctrl+S, Ctrl+O, Ctrl+N, Ctrl+Shift+S and Ctrl+, reach the window while a text field
/// has the keyboard; everything else is the field's.
#[test]
fn window_shortcuts_work_while_a_field_has_the_keyboard() {
    use crate::app_actions::Action;
    let (ctx, editor) = setup();
    let (commands, window, mut animation) = keyboard(&ctx, &editor);
    let cmd = egui::Modifiers::COMMAND;
    for (event, action) in [
        (press(egui::Key::S, cmd), Action::Save),
        (press(egui::Key::S, cmd | egui::Modifiers::SHIFT), Action::SaveAs),
        (press(egui::Key::O, cmd), Action::Open),
        (press(egui::Key::N, cmd), Action::New),
        (press(egui::Key::Comma, cmd), Action::Preferences),
    ] {
        crate::dispatch_keys(&commands, &window, &mut animation, &editor, vec![event], cmd, None, true);
        assert_eq!(window.borrow_mut().action.take(), Some(action));
    }
    // A plain key while the field has the keyboard is typing, not a command.
    editor.document.borrow_mut().select_all();
    let before = editor.document.borrow().to_map_text();
    let typed = vec![press(egui::Key::Delete, egui::Modifiers::NONE), egui::Event::Text("#".into())];
    crate::dispatch_keys(&commands, &window, &mut animation, &editor, typed, egui::Modifiers::NONE, None, true);
    assert_eq!(editor.document.borrow().to_map_text(), before);
    assert!(editor.grid.borrow().visible());
    assert!(window.borrow().action.is_none());
    // Without a field it is the command.
    crate::dispatch_keys(&commands, &window, &mut animation, &editor, vec![press(egui::Key::Delete, egui::Modifiers::NONE)], egui::Modifiers::NONE, None, false);
    assert_ne!(editor.document.borrow().to_map_text(), before);
}

/// `#` shows and hides the grid, as 0 does.
#[test]
fn number_sign_toggles_the_grid() {
    let (ctx, editor) = setup();
    let (commands, window, mut animation) = keyboard(&ctx, &editor);
    let visible = editor.grid.borrow().visible();
    let hash = || vec![egui::Event::Text("#".into())];
    crate::dispatch_keys(&commands, &window, &mut animation, &editor, hash(), egui::Modifiers::SHIFT, None, false);
    assert_eq!(editor.grid.borrow().visible(), !visible);
    crate::dispatch_keys(&commands, &window, &mut animation, &editor, hash(), egui::Modifiers::SHIFT, None, false);
    assert_eq!(editor.grid.borrow().visible(), visible);
}

/// Revert reads the map again without asking, and a map never saved has nothing to go
/// back to.
#[test]
fn revert_discards_changes_without_asking() {
    let (_ctx, editor) = setup();
    let saved = editor.document.borrow().to_map_text();
    editor.document.borrow_mut().select_all();
    assert!(editor.document.borrow_mut().delete_selection() > 0);
    assert!(editor.document.borrow().is_modified());
    assert!(crate::app_actions::revert(&editor).is_some());
    assert!(!editor.document.borrow().is_modified());
    assert_eq!(editor.document.borrow().to_map_text(), saved);

    let ctx = egui::Context::default();
    let unsaved = Editor::open(&ctx, None);
    unsaved.document.borrow_mut().path = None;
    let text = unsaved.document.borrow().to_map_text();
    assert!(crate::app_actions::revert(&unsaved).is_none());
    assert_eq!(unsaved.document.borrow().to_map_text(), text);
}

/// Save As adds `.map` to a name without an extension, and leaves one with an extension
/// as it was typed.
#[test]
fn save_as_adds_the_map_extension() {
    use crate::app_actions::with_map_extension;
    use std::path::PathBuf;
    assert_eq!(with_map_extension(PathBuf::from("/maps/castle")), PathBuf::from("/maps/castle.map"));
    assert_eq!(with_map_extension(PathBuf::from("/maps/castle.map")), PathBuf::from("/maps/castle.map"));
    assert_eq!(with_map_extension(PathBuf::from("/maps/castle.MAP")), PathBuf::from("/maps/castle.MAP"));
}

/// The close prompt names the map.
#[test]
fn the_unsaved_changes_prompt_names_the_map() {
    let (_ctx, editor) = setup();
    let message = crate::app_actions::unsaved_message(&editor.document.borrow());
    assert!(message.starts_with("crate.map has changes that are not saved."), "{message}");
    assert!(message.ends_with("If you discard them they are gone."));
}

/// The menus offer Repeat and the point and portal reloads only when there is something
/// for them to do, and the keyframe item is a check that follows the panel.
#[test]
fn menu_items_are_enabled_only_when_they_apply() {
    use crate::app_actions::Action;
    use crate::menu::Entry;
    let (_ctx, editor) = setup();
    let find = |menus: &[Vec<Entry<Action>>], wanted: Action| -> (bool, Option<bool>) {
        menus
            .iter()
            .flatten()
            .find_map(|entry| match entry {
                Entry::Item { action, enabled, checked, .. } if *action == wanted => Some((*enabled, *checked)),
                _ => None,
            })
            .expect("the item is in a menu")
    };
    let grid = editor.grid.borrow();
    let filters = editor.filters.borrow();
    let mut status = crate::chrome::Status::default();
    let menus = crate::chrome::menus(&status, false, &grid, &filters);
    for action in [Action::Repeat, Action::ClearRepeat, Action::ReloadPoint, Action::UnloadPoint, Action::ReloadPortal, Action::UnloadPortal] {
        assert!(!find(&menus, action).0, "{action:?} is offered with nothing to do");
    }
    assert!(find(&menus, Action::LoadPoint).0);
    assert_eq!(find(&menus, Action::Animation).1, Some(false));
    status.can_repeat = true;
    status.point_loaded = true;
    status.animation_open = true;
    let menus = crate::chrome::menus(&status, true, &grid, &filters);
    assert!(find(&menus, Action::Repeat).0 && find(&menus, Action::UnloadPoint).0);
    assert_eq!(find(&menus, Action::Animation).1, Some(true));
    assert_eq!(find(&menus, Action::Detach3d).1, Some(true));
}

/// A window is centred on the monitor it is on, not always on the first one.
#[test]
fn the_window_is_centred_on_its_own_monitor() {
    let monitor = egui::vec2(1920.0, 1080.0);
    let size = egui::vec2(1360.0, 760.0);
    let first = egui::Rect::from_min_size(egui::pos2(100.0, 80.0), egui::vec2(800.0, 450.0));
    assert_eq!(crate::centred_on_monitor(first, monitor, size), egui::pos2(280.0, 160.0));
    let second = first.translate(egui::vec2(1920.0, 0.0));
    assert_eq!(crate::centred_on_monitor(second, monitor, size), egui::pos2(2200.0, 160.0));
    let left = first.translate(egui::vec2(-1920.0, 0.0));
    assert_eq!(crate::centred_on_monitor(left, monitor, size), egui::pos2(-1640.0, 160.0));
}

/// A new layout, or the 3D view leaving, drops a maximised view; Space does not cycle
/// the detached 3D view.
#[test]
fn a_layout_change_drops_the_maximised_view() {
    use editor_core::preferences::Layout;
    let ctx = egui::Context::default();
    let mut panes = crate::layout::Panes::default();
    let area = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    let arrange = |panes: &mut crate::layout::Panes, layout| {
        let mut out = Vec::new();
        let raw = egui::RawInput { screen_rect: Some(area), ..Default::default() };
        let mut output = ctx.run_ui(raw, |ui| out = panes.arrange(ui, area, layout));
        output.textures_delta.clear();
        out
    };
    arrange(&mut panes, Layout::Four);
    panes.focused = Some(1);
    panes.toggle_maximised();
    assert_eq!(arrange(&mut panes, Layout::Four), vec![(1, area)]);
    assert_eq!(arrange(&mut panes, Layout::Two).len(), 2);
    assert_eq!(panes.maximised, None);
    panes.focused = Some(2);
    panes.toggle_maximised();
    panes.detached = true;
    arrange(&mut panes, Layout::Two);
    assert_eq!(panes.maximised, None);
    panes.focused = Some(0);
    panes.cycle(Layout::One);
    assert_eq!(panes.focused, Some(0), "the detached 3D view has no pane to cycle");
}
