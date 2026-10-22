//! Window actions and small dialogs shared by menu entries and keyboard shortcuts.
use crate::{
    dialog, editor::Editor, file_dialog, file_dialog::FileDialog, form, theme, widgets, App,
};

/// File > Open's chooser.
fn model_chooser() -> FileDialog {
    FileDialog::new()
        .id("import-model")
        .set_title(tr("Import model"))
        .add_filter(tr("OBJ and GLB models"), &["obj", "glb"])
}

fn open_chooser() -> FileDialog {
    FileDialog::new()
        .id("open-map")
        .set_title(tr("Open"))
        .add_filter(tr("Quake maps"), &["map"])
}

/// The chooser for a point file (`point`) or a portal file.
fn point_file_chooser(point: bool) -> FileDialog {
    FileDialog::new()
        .id(if point {
            "load-point-file"
        } else {
            "load-portal-file"
        })
        .set_title(tr(if point {
            "Load point file"
        } else {
            "Load portal file"
        }))
        .add_filter(
            tr(if point { "Point files" } else { "Portal files" }),
            if point { &["pts", "lin"] } else { &["prt"] },
        )
}
use editor_core::i18n::tr;
use eframe::egui;

/// A 2D cursor carries NaN on its unknown depth axis, like paste placement.
fn model_insert_target(
    target: Option<geometry_math::vec::Vec3d>,
    reference: geometry_math::vec::Vec3d,
) -> geometry_math::vec::Vec3d {
    let mut at = target.unwrap_or_else(geometry_math::vec::Vec3d::zero);
    for axis in 0..3 {
        if at[axis].is_nan() {
            at[axis] = reference[axis];
        }
    }
    at
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    Delete,
    Duplicate,
    SelectAll,
    Welcome,
    Render,
    Session,
    /// The Entity links window (entity_graph_page.rs).
    EntityGraph,
    Animation,
    UvEditor,
    ShapeSettings,
    Text,
    ImportModel,
    Corner,
    /// The Array window (array_dialog.rs).
    Array,
    /// Cut the selected box into columns one grid step across, to sculpt.
    SplitColumns,
    Preferences,
    Plugins,
    Export,
    Build,
    Updates,
    #[cfg(target_arch = "wasm32")]
    Playtest,
    Mcp,
    New,
    Open,
    Save,
    SaveAs,
    Revert,
    GoTo,
    About,
    Manual,
    Repeat,
    ClearRepeat,
    Patches,
    Structural,
    Sync,
    PlaceCamera,
    LoadPoint,
    ReloadPoint,
    UnloadPoint,
    LoadPortal,
    ReloadPortal,
    UnloadPortal,
    /// The 3D view into a window of its own, or back.
    Detach3d,
    SnapGrid,
    ShowGrid,
    PreviewAnimations,
    ShowOmitted,
    /// The grid size, as a power of two.
    GridSize(i32),
    /// Stops the engine live link (its header indicator).
    StopLiveLink,
    /// Shows or hides the inspector column.
    ToggleInspector,
    /// The next view in the pane that has the keyboard (Space).
    NextView,
}

/// `path` with `.map` added when it has no extension, for Save As.
pub fn with_map_extension(mut path: std::path::PathBuf) -> std::path::PathBuf {
    if path.extension().is_none() {
        path.set_extension("map");
    }
    path
}

/// The chooser Save As asks with, which [`saved_as`] takes the in-app answer from.
const SAVE_AS: &str = "save-map-as";

/// Saves the map, asking where first for Save As or a map never saved. Returns whether it
/// was saved; an in-app chooser answers later, through [`saved_as`].
pub fn save(document: &mut editor_core::document::Document, save_as: bool) -> bool {
    let chosen = save_as || document.path.is_none();
    if !chosen {
        let result = document.save();
        return finish_save(document, result, false);
    }
    let mut picker = FileDialog::new()
        .id(SAVE_AS)
        .set_title(tr("Save map as"))
        .add_filter(tr("Quake maps"), &["map"]);
    match &document.path {
        Some(path) => {
            if let Some(parent) = path.parent() {
                picker = picker.set_directory(parent);
            }
            if let Some(name) = path.file_name() {
                picker = picker.set_file_name(name.to_string_lossy());
            }
        }
        None => picker = picker.set_file_name(format!("{}.map", document.level_name())),
    }
    let Some(path) = picker.save_file(true) else {
        return false;
    };
    let result = document.save_as(with_map_extension(path));
    finish_save(document, result, true)
}

/// Takes the answer of an in-app Save As chooser and saves there. Returns whether it saved.
/// Call every frame.
pub fn saved_as(document: &mut editor_core::document::Document) -> bool {
    let Some(path) = FileDialog::new().id(SAVE_AS).save_file(false) else {
        return false;
    };
    let result = document.save_as(with_map_extension(path));
    finish_save(document, result, true)
}

/// Records and reports a save, and exports on save.
fn finish_save(
    document: &mut editor_core::document::Document,
    result: Result<(), editor_core::document::SaveError>,
    chosen: bool,
) -> bool {
    match result {
        Ok(()) => {
            if let Some(path) = &document.path {
                editor_core::recent_documents::record(path);
                if chosen {
                    editor_core::say!("saved {}", path.display());
                } else {
                    editor_core::say!("saved");
                }
            }
            // The web build has no folder to write a level into on every save.
            #[cfg(not(target_arch = "wasm32"))]
            for message in editor_core::export::export_on_save(
                document,
                &editor_core::export_settings::Settings::for_map(document.path.as_deref()),
            ) {
                editor_core::say!("wrote {message}");
            }
            true
        }
        Err(error) => {
            editor_core::complain!("could not save: {error}");
            false
        }
    }
}

/// What the unsaved-changes prompt says about `document`: the map's name, then that
/// discarded changes are gone.
pub fn unsaved_message(document: &editor_core::document::Document) -> String {
    format!(
        "{} {}\n{}",
        document.name(),
        tr("has changes that are not saved."),
        tr("If you discard them they are gone.")
    )
}

/// The unsaved-changes question, by its key in [`file_dialog::question`].
const UNSAVED: &str = "unsaved-changes";

thread_local! {
    /// An in-app answer of Discard, letting the next [`can_leave`] through.
    static LEAVE_GRANTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Offers to save before replacing or closing a modified document: Save, Discard or
/// Cancel. Returns whether the document may go.
///
/// The in-app question (Android) answers frames later: this returns false at once, and
/// [`leave_answer`] says when the caller may try again.
pub fn can_leave(document: &mut editor_core::document::Document) -> bool {
    if !document.is_modified() || LEAVE_GRANTED.with(|granted| granted.replace(false)) {
        return true;
    }
    let buttons = [tr("Save"), tr("Discard"), tr("Cancel")];
    let in_app = file_dialog::in_app();
    match file_dialog::question(
        UNSAVED,
        true,
        tr("Unsaved changes"),
        &unsaved_message(document),
        &buttons,
    ) {
        _ if in_app => false,
        Some(0) => save(document, false),
        Some(1) => true,
        _ => false,
    }
}

/// The in-app answer to [`can_leave`]'s question: `Some(true)` when the document may go
/// now (saved or discarded; the next `can_leave` lets it through), `Some(false)` for
/// Cancel, or a Save that waits for a chooser. Call every frame.
pub fn leave_answer(document: &mut editor_core::document::Document) -> Option<bool> {
    let answer = file_dialog::question(UNSAVED, false, "", "", &[])?;
    Some(match answer {
        0 => save(document, false),
        1 => {
            LEAVE_GRANTED.with(|granted| granted.set(true));
            true
        }
        _ => false,
    })
}

/// `Menu/File/Revert`: reads the map again from the file it came from, throwing away
/// everything since the last save without asking. Returns where its materials and
/// definitions came from, or `None` when there was nothing to go back to.
pub fn revert(editor: &Editor) -> Option<editor_core::document_setup::Sources> {
    let path = editor.document.borrow().path.clone();
    let Some(path) = path else {
        editor_core::say!("revert: the map has never been saved");
        return None;
    };
    // Checked first: loading falls back to an empty map on a read error.
    if let Err(error) = editor_core::document::Document::open(&path, editor.bounds) {
        editor_core::complain!("{}: {error}", path.display());
        return None;
    }
    let loaded = editor_core::document_setup::load(Some(path.clone()), None, None);
    *editor.document.borrow_mut() = loaded.document;
    editor_core::say!("reverted to {}", path.display());
    Some(loaded.sources)
}

/// Runs a document-only menu command and says what came of it in the console.
pub fn run_document_command(editor: &Editor, action: Action) {
    let mut document = editor.document.borrow_mut();
    match action {
        Action::ReloadPoint => match document.reload_point_file() {
            Some(Ok(points)) => editor_core::say!("point file: {points} points"),
            Some(Err(error)) => {
                editor_core::complain!("the point file could not be read again: {error}")
            }
            None => editor_core::say!("no point file to read again"),
        },
        Action::ReloadPortal => match document.reload_portal_file() {
            Some(Ok(portals)) => editor_core::say!("portal file: {portals} portals"),
            Some(Err(why)) => {
                editor_core::complain!("the portal file could not be read again: {why}")
            }
            None => editor_core::say!("no portal file to read again"),
        },
        Action::UnloadPoint => document.unload_point_file(),
        Action::UnloadPortal => document.unload_portal_file(),
        Action::ClearRepeat => {
            document.clear_repeatable();
            editor_core::say!("forgot what Repeat would do again");
        }
        Action::Sync => {
            let synced = document.sync_linked_groups();
            editor_core::say!("synced {synced} linked groups");
        }
        Action::Patches => {
            let made =
                document.convert_selection_to_patches(map_model::face_to_patch::DEFAULT_POINTS);
            if made > 0 {
                editor_core::say!("convert faces to patches: {made} patches");
            } else {
                editor_core::say!("convert faces to patches: select some brush faces first");
            }
        }
        Action::Structural => {
            let moved = document.make_structural();
            if moved > 0 {
                editor_core::say!("moved {moved} objects out of their entity");
            } else {
                editor_core::complain!("select brushes that are inside a brush entity");
            }
        }
        _ => {}
    }
}

/// Toggles one of the window-level switches the menus and the grid menu share.
pub fn toggle(editor: &Editor, action: Action) {
    match action {
        Action::SnapGrid => {
            let mut grid = editor.grid.borrow_mut();
            let snap = grid.snap();
            grid.set_snap(!snap);
        }
        Action::ShowGrid => {
            let mut grid = editor.grid.borrow_mut();
            let visible = grid.visible();
            grid.set_visible(!visible);
        }
        Action::PreviewAnimations => {
            let mut filters = editor.filters.borrow_mut();
            filters.preview_motion = !filters.preview_motion;
        }
        Action::ShowOmitted => {
            let mut filters = editor.filters.borrow_mut();
            filters.show_export_hidden = !filters.show_export_hidden;
        }
        Action::GridSize(exponent) => {
            let resized = editor.grid.borrow().with_size(exponent);
            *editor.grid.borrow_mut() = resized;
        }
        _ => {}
    }
}

#[derive(Default)]
pub struct Dialogs {
    pub mcp: bool,
    pub go_to: bool,
    pub about: bool,
    pub licences: bool,
    at: [f64; 3],
    delta: [f64; 3],
    line: usize,
    status: String,
}

impl App {
    pub(crate) fn action(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::Undo
            | Action::Redo
            | Action::Copy
            | Action::Cut
            | Action::Paste
            | Action::Delete
            | Action::Duplicate
            | Action::SelectAll => {
                use editor_core::input::{Key, Modifiers};
                if matches!(action, Action::Undo | Action::Redo) {
                    self.cancel_animation_previews();
                }
                let animation_event = match action {
                    Action::Copy => Some(egui::Event::Copy),
                    Action::Cut => Some(egui::Event::Cut),
                    Action::Paste => Some(egui::Event::Paste(String::new())),
                    Action::Duplicate | Action::SelectAll => Some(egui::Event::Key {
                        key: if matches!(action, Action::Duplicate) {
                            egui::Key::D
                        } else {
                            egui::Key::A
                        },
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers {
                            ctrl: true,
                            ..Default::default()
                        },
                    }),
                    Action::Delete | Action::Undo | Action::Redo => Some(egui::Event::Key {
                        key: if matches!(action, Action::Delete) {
                            egui::Key::Delete
                        } else {
                            egui::Key::Z
                        },
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers {
                            ctrl: !matches!(action, Action::Delete),
                            shift: matches!(action, Action::Redo),
                            ..Default::default()
                        },
                    }),
                    _ => None,
                };
                if animation_event
                    .as_ref()
                    .is_some_and(|event| self.animation.key_event(event, &self.editor))
                {
                    return;
                }
                let key = match action {
                    Action::Undo | Action::Redo => Key::Letter('z'),
                    Action::Copy => Key::Letter('c'),
                    Action::Cut => Key::Letter('x'),
                    Action::Paste => Key::Letter('v'),
                    Action::Duplicate => Key::Letter('d'),
                    Action::SelectAll => Key::Letter('a'),
                    _ => Key::Delete,
                };
                self.commands.key_pressed(
                    key,
                    None,
                    Modifiers {
                        control: !matches!(action, Action::Delete),
                        shift: matches!(action, Action::Redo),
                        alt: false,
                    },
                );
            }

            Action::Welcome => self.welcome.open = true,
            Action::Render => self.render_page.open(&self.editor),
            Action::Session => self.session.open = true,
            Action::EntityGraph => self.entity_graph.open = true,
            // A toggle: the menu's check closes it again.
            Action::Animation => self.animation.open = !self.animation.open,
            Action::UvEditor => {
                self.inspector.toggle_uv_window(&self.editor.document);
                if !self.inspector.uv_window_open() {
                    self.window.borrow_mut().inspector_hidden = false;
                }
            }
            Action::ShapeSettings => self.shapes.open_settings(),
            Action::Text => self.shapes.text_open = true,
            Action::ImportModel => {
                if let Some(path) = model_chooser().pick_file(true) {
                    self.import_model_file(&path);
                }
            }
            Action::Corner => self.shapes.corner(&self.editor),
            Action::Array => self.arrays.open(&self.editor),
            Action::SplitColumns => {
                let cell = self.editor.grid.borrow().actual_size();
                let shape = editor_core::sculpt::parameters().columns;
                let result = self
                    .editor
                    .document
                    .borrow_mut()
                    .split_into_columns(cell, shape);
                match result {
                    Ok(count) => editor_core::say!(
                        "{}",
                        editor_core::i18n::fill("Columns: {}", &[&count.to_string()])
                    ),
                    Err(why) => editor_core::say!("{}", tr(why)),
                }
            }
            Action::Preferences => self.preferences.open = true,
            Action::Plugins => self.plugins.open(),
            Action::Export => self.exports.open = true,
            Action::Build => self.builds.open = true,
            Action::Updates => self.updates.open = true,
            #[cfg(target_arch = "wasm32")]
            Action::Playtest => crate::web::open_playtest(),
            Action::Mcp => {
                if self.mcp.is_none() {
                    match editor_core::live_mcp::Server::start() {
                        Ok(server) => self.mcp = Some(server),
                        Err(error) => {
                            editor_core::complain!("the MCP endpoint could not start: {error}")
                        }
                    }
                }
                self.dialogs.mcp = self.mcp.is_some();
            }
            Action::Save | Action::SaveAs => {
                save(
                    &mut self.editor.document.borrow_mut(),
                    matches!(action, Action::SaveAs),
                );
            }
            Action::New => self.open_map(ctx, None),
            Action::Open => {
                if let Some(path) = open_chooser().pick_file(true) {
                    self.open_map(ctx, Some(path));
                }
            }
            Action::Revert => {
                if let Some(sources) = revert(&self.editor) {
                    self.cancel_animation_previews();
                    *self.commands.sources.borrow_mut() = sources;
                    self.autosaver =
                        editor_core::autosave::Autosaver::new(&self.editor.document.borrow());
                }
            }
            Action::GoTo => {
                let p = self.editor.camera.borrow().position;
                self.dialogs.at = [p.x(), p.y(), p.z()];
                self.dialogs.line = 1;
                self.dialogs.go_to = true;
            }
            Action::About => self.dialogs.about = true,
            Action::Manual => self.manual.open = true,
            Action::Repeat => editor_core::commands::repeat_last_commands(
                &self.editor.document,
                self.commands.window.as_ref(),
            ),
            Action::ClearRepeat
            | Action::Patches
            | Action::Structural
            | Action::Sync
            | Action::ReloadPoint
            | Action::ReloadPortal
            | Action::UnloadPoint
            | Action::UnloadPortal => {
                run_document_command(&self.editor, action);
            }
            Action::PlaceCamera => {
                editor_core::render_camera::align_to_view(
                    &mut self.editor.document.borrow_mut(),
                    &self.editor.camera,
                );
            }
            Action::LoadPoint | Action::LoadPortal => {
                let point = matches!(action, Action::LoadPoint);
                if let Some(path) = point_file_chooser(point).pick_file(true) {
                    self.load_point_file(&path, point);
                }
            }
            Action::Detach3d => self.chrome.detach_3d = !self.chrome.detach_3d,
            Action::SnapGrid
            | Action::ShowGrid
            | Action::PreviewAnimations
            | Action::ShowOmitted
            | Action::GridSize(_) => toggle(&self.editor, action),
            Action::StopLiveLink => self.live.stop("Live link stopped"),
            Action::ToggleInspector => self.commands.window.toggle_inspector(),
            Action::NextView => self.commands.window.cycle_layout(),
        }
        ctx.request_repaint();
    }

    /// Inserts a model at the last cursor position, resolving unspecified 2D depth.
    pub(crate) fn import_model_file(&mut self, path: &std::path::Path) {
        let at = model_insert_target(
            *self.editor.paste_target.borrow(),
            self.editor.document.borrow().reference_bounds().center(),
        );
        let at = self.editor.grid.borrow().snap_point(at);
        match self
            .editor
            .document
            .borrow_mut()
            .import_model_copy(path, at)
        {
            Ok(_) => editor_core::say!("Imported model {}", path.display()),
            Err(error) => editor_core::complain!("{}", error),
        }
    }

    /// Applies answers from in-app choosers and questions once per frame.
    pub(crate) fn chooser_answers(&mut self, ctx: &egui::Context) {
        if !file_dialog::in_app() {
            return;
        }
        if let Some(path) = open_chooser().pick_file(false) {
            self.open_map(ctx, Some(path));
        }
        if let Some(path) = model_chooser().pick_file(false) {
            self.import_model_file(&path);
        }
        for point in [true, false] {
            if let Some(path) = point_file_chooser(point).pick_file(false) {
                self.load_point_file(&path, point);
            }
        }
        let saved = saved_as(&mut self.editor.document.borrow_mut());
        let answer = leave_answer(&mut self.editor.document.borrow_mut());
        // A map waiting on the unsaved-changes question opens once the old one is saved
        // or discarded.
        match answer {
            Some(true) => self.pending_map = self.leave_retry.take(),
            Some(false) if !file_dialog::showing() => self.leave_retry = None,
            _ if saved && self.leave_retry.is_some() => self.pending_map = self.leave_retry.take(),
            _ => {}
        }
        if self.pending_map.is_some() {
            ctx.request_repaint();
        }
    }

    /// Loads a point file (`point`) or a portal file over the map.
    fn load_point_file(&mut self, path: &std::path::Path, point: bool) {
        let mut document = self.editor.document.borrow_mut();
        if point {
            match document.load_point_file(path) {
                Ok(0) => editor_core::say!("{}: no points in it", path.display()),
                Ok(points) => editor_core::say!("point file: {points} points"),
                Err(error) => editor_core::complain!("{}: {error}", path.display()),
            }
        } else {
            match document.load_portal_file(path) {
                Ok(portals) => editor_core::say!("portal file: {portals} portals"),
                Err(why) => editor_core::complain!("{}: {why}", path.display()),
            }
        }
    }

    /// Stops the animation gizmos' previews, which an undo or a reload invalidates.
    pub(crate) fn cancel_animation_previews(&mut self) {
        for pane in &mut self.flat {
            pane.animation.cancel();
        }
        if let Some(view) = &mut self.view3d {
            view.animation.cancel();
        }
        if let Some(view) = &mut self.detached {
            view.view.animation.cancel();
        }
    }

    fn open_map(&mut self, ctx: &egui::Context, path: Option<std::path::PathBuf>) {
        self.pending_map = Some(path);
        ctx.request_repaint();
    }

    pub(crate) fn replace_map(&mut self, ctx: &egui::Context, path: Option<std::path::PathBuf>) {
        // Validate before replacing the current editor: document_setup falls back to an
        // empty map on a read error, which is suitable at startup but not for File/Open.
        if let Some(path) = &path {
            if let Err(error) = editor_core::document::Document::open(path, self.editor.bounds) {
                editor_core::complain!("could not open {}: {error}", path.display());
                return;
            }
        }
        self.render_page.finish_navigation(&self.editor);
        if !can_leave(&mut self.editor.document.borrow_mut()) {
            // An in-app question answers later; the map opens then (`chooser_answers`).
            self.leave_retry = Some(path);
            return;
        }
        self.editor.document.borrow().save_material_info();
        match Self::build(
            ctx,
            self.gpu.clone(),
            crate::Startup {
                map: path,
                ..Default::default()
            },
        ) {
            Ok(mut next) => {
                next.placed = self.placed && !self.startup;
                next.startup = false;
                next.mcp = self.mcp.take();
                next.instance = self.instance.take();
                next.welcome.open = false;
                next.script = self.script.take();
                next.inspector_width = self.inspector_width;
                next.inspector_share = self.inspector_share;
                next.touch_layout_set = self.touch_layout_set;
                next.window.borrow_mut().inspector_hidden = self.window.borrow().inspector_hidden;
                if let Some(view) = self.view3d.take() {
                    view.destroy();
                }
                if let Some(detached) = self.detached.take() {
                    detached.view.destroy();
                }
                self.live.stop("Map closed");
                self.render_page.destroy();
                *self = next;
            }
            Err(error) => editor_core::complain!("could not open editor: {error}"),
        }
    }

    pub(crate) fn show_dialogs(&mut self, ctx: &egui::Context) {
        if self.dialogs.mcp {
            let mut stop = false;
            let mut close = false;
            self.dialogs.mcp = dialog::show(ctx, "mcp", "MCP", [480.0, 230.0], |ui, part| {
                if part == dialog::Part::Actions {
                    close = dialog::button(ui, tr("Close"), true).clicked();
                    stop = dialog::button(ui, tr("Stop server"), true).clicked();
                } else if let Some(server) = &self.mcp {
                    ui.horizontal(|ui| {
                        let (dot, _) =
                            ui.allocate_exact_size(egui::vec2(10.0, 16.0), egui::Sense::hover());
                        ui.painter()
                            .circle_filled(dot.center(), 4.0, theme::success());
                        ui.label(egui::RichText::new(tr("MCP active")).font(theme::body_bold()));
                    });
                    ui.add_space(4.0);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(tr("Serve for an agent (MCP)")).color(theme::dim()),
                        )
                        .wrap(),
                    );
                    ui.add_space(8.0);
                    let address = format!("127.0.0.1:{}", server.port);
                    ui.horizontal(|ui| {
                        // Selectable and copyable; anything typed into it is put back.
                        let mut shown = address.clone();
                        ui.add(
                            egui::TextEdit::singleline(&mut shown)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(ui.available_width() - 90.0)
                                .margin(crate::widgets::entry_margin(
                                    ui,
                                    &egui::TextStyle::Monospace.resolve(ui.style()),
                                    8,
                                    8,
                                ))
                                .min_size(egui::vec2(0.0, theme::CONTROL_HEIGHT))
                                .vertical_align(egui::Align::Center),
                        );
                        if theme::button(ui, tr("Copy"), 80.0, true).clicked() {
                            ui.ctx().copy_text(address);
                        }
                    });
                }
            }) && !close;
            if stop {
                self.mcp = None;
                self.dialogs.mcp = false;
            }
        }
        if self.dialogs.go_to {
            let mut close = false;
            self.dialogs.go_to =
                dialog::show(ctx, "go-to", tr("Go to"), [460.0, 470.0], |ui, part| {
                    if part == dialog::Part::Actions {
                        close = dialog::button(ui, tr("Close"), true).clicked();
                        return;
                    }
                    widgets::card(ui, tr("A point in the world"), None, |ui| {
                        widgets::axis_spins(
                            ui,
                            "go-to-at",
                            &mut self.dialogs.at,
                            &["X", "Y", "Z"],
                            -131_072.0..=131_072.0,
                            1.0,
                            2,
                        );
                        ui.add_space(2.0);
                        if form::button(ui, tr("Move camera there"), true).clicked() {
                            let [x, y, z] = self.dialogs.at;
                            let at = geometry_math::vec::Vec3d::new(x, y, z);
                            self.editor.camera.borrow_mut().position = at;
                            for state in self.editor.cameras.borrow().iter() {
                                let mut state = state.borrow_mut();
                                let plane = state.camera.plane;
                                state.camera.set_center(plane.project(at));
                            }
                        }
                    });
                    ui.add_space(6.0);
                    widgets::card(ui, tr("Move the selection by"), None, |ui| {
                        widgets::axis_spins(
                            ui,
                            "go-to-delta",
                            &mut self.dialogs.delta,
                            &["X", "Y", "Z"],
                            -131_072.0..=131_072.0,
                            1.0,
                            2,
                        );
                        ui.add_space(2.0);
                        if form::button(ui, tr("Move it"), true).clicked() {
                            let [x, y, z] = self.dialogs.delta;
                            let moved = self
                                .editor
                                .document
                                .borrow_mut()
                                .translate_selection(geometry_math::vec::Vec3d::new(x, y, z));
                            self.dialogs.status =
                                tr(if moved { "Moved" } else { "Nothing to move" }).to_owned();
                        }
                    });
                    ui.add_space(6.0);
                    widgets::card(ui, tr("A line of the map file"), None, |ui| {
                        ui.horizontal(|ui| {
                            let mut line = self.dialogs.line as f64;
                            if widgets::spin(ui, &mut line, 1.0..=10_000_000.0, 1.0, 0).changed() {
                                self.dialogs.line = line.round() as usize;
                            }
                            if form::button(ui, tr("Select what is there"), true).clicked() {
                                let selected = self
                                    .editor
                                    .document
                                    .borrow_mut()
                                    .select_by_line(self.dialogs.line);
                                self.dialogs.status = tr(if selected {
                                    "Selected"
                                } else {
                                    "Nothing was read from that line"
                                })
                                .to_owned();
                            }
                        });
                    });
                    if !self.dialogs.status.is_empty() {
                        ui.add_space(4.0);
                        widgets::status(ui, &self.dialogs.status);
                    }
                }) && !close;
        }
        if self.dialogs.about {
            let mut close = false;
            let mut licences = false;
            self.dialogs.about =
                dialog::show(ctx, "about", tr("About"), [460.0, 360.0], |ui, part| {
                    if part == dialog::Part::Actions {
                        close = dialog::button(ui, tr("Close"), true).clicked();
                        return;
                    }
                    licences |= about(ui);
                }) && !close;
            self.dialogs.licences |= licences;
        }
        if self.dialogs.licences {
            let mut close = false;
            self.dialogs.licences = dialog::show(
                ctx,
                "licences",
                tr("Licences and third-party credits"),
                [760.0, 640.0],
                |ui, part| {
                    if part == dialog::Part::Actions {
                        close = dialog::button(ui, tr("Close"), true).clicked();
                        return;
                    }
                    licences(ui);
                },
            ) && !close;
        }
    }
}

/// The about page: the logo, the name and version, the licence, and a button for the
/// credits. Returns whether that button was pressed.
pub fn about(ui: &mut egui::Ui) -> bool {
    ui.vertical_centered(|ui| {
        ui.add_space(8.0);
        let (uri, bytes) = crate::chrome::LOGO;
        ui.add(egui::Image::from_bytes(uri, bytes).fit_to_exact_size(egui::Vec2::splat(72.0)));
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Heron Brush")
                .font(egui::FontId::new(theme::text_size() * 1.7, theme::bold())),
        );
        ui.label(
            egui::RichText::new(format!("{} {}", tr("Version"), env!("CARGO_PKG_VERSION")))
                .color(theme::dim()),
        );
        ui.add_space(12.0);
        ui.label(tr("Licensed under GPL-3.0-or-later"));
        ui.add_space(10.0);
        let pressed = ui.button(tr("Licences and third-party credits")).clicked();
        ui.add_space(10.0);
        ui.hyperlink_to(env!("CARGO_PKG_REPOSITORY"), env!("CARGO_PKG_REPOSITORY"));
        pressed
    })
    .inner
}

/// `CREDITS.md`, compiled in: the credits, the derivative-work notice and the third-party
/// licences, shown from the About dialog.
const NOTICE: &str = include_str!("../../../CREDITS.md");

/// One block of [`NOTICE`] as it is drawn.
#[derive(Debug, PartialEq)]
enum NoticeBlock {
    /// A `#` or `##` heading, with its level.
    Heading(usize, String),
    /// A paragraph or a list item, its wrapped lines joined.
    Paragraph(String),
}

/// Splits Markdown into headings and paragraphs, joining hard-wrapped lines and writing
/// a link `[text](url)` as `text (url)`.
fn notice_blocks(text: &str) -> Vec<NoticeBlock> {
    let mut blocks = Vec::new();
    let mut paragraph = String::new();
    let finish = |paragraph: &mut String, blocks: &mut Vec<NoticeBlock>| {
        if !paragraph.is_empty() {
            blocks.push(NoticeBlock::Paragraph(links_as_text(&std::mem::take(
                paragraph,
            ))));
        }
    };
    for line in text.lines().map(str::trim_end) {
        let level = line.chars().take_while(|&c| c == '#').count();
        if level > 0 && line[level..].starts_with(' ') {
            finish(&mut paragraph, &mut blocks);
            blocks.push(NoticeBlock::Heading(level, line[level..].trim().to_owned()));
        } else if line.is_empty() {
            finish(&mut paragraph, &mut blocks);
        } else {
            if let Some(item) = line.strip_prefix("- ") {
                finish(&mut paragraph, &mut blocks);
                paragraph.push_str("• ");
                paragraph.push_str(item);
                continue;
            }
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(line.trim_start());
        }
    }
    finish(&mut paragraph, &mut blocks);
    blocks
}

/// `[text](url)` as `text (url)`; anything else as it is.
fn links_as_text(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let Some((label, url, after)) = rest[open + 1..]
            .split_once("](")
            .and_then(|(label, tail)| tail.split_once(')').map(|(url, after)| (label, url, after)))
        else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push_str(label);
        out.push_str(" (");
        out.push_str(url);
        out.push(')');
        rest = after;
    }
    out.push_str(rest);
    out
}

/// The credits window's page: [`NOTICE`], scrolled.
fn licences(ui: &mut egui::Ui) {
    let size = theme::text_size();
    egui::ScrollArea::vertical()
        .id_salt("licences-body")
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                for block in notice_blocks(NOTICE) {
                    match block {
                        NoticeBlock::Heading(level, text) => {
                            ui.add_space(size * 0.6);
                            let scale = if level == 1 { 1.5 } else { 1.15 };
                            ui.label(egui::RichText::new(text).size(size * scale).strong());
                            ui.add_space(size * 0.3);
                        }
                        NoticeBlock::Paragraph(text) => {
                            ui.label(crate::manual_page::inline(ui.style(), &text, size));
                            ui.add_space(size * 0.5);
                        }
                    }
                }
            });
        });
}

#[cfg(test)]
mod notice_tests {
    use super::*;

    #[test]
    fn the_notice_reads_as_headings_and_joined_paragraphs() {
        let blocks = notice_blocks(
            "# Title\n\nOne line\nwrapped, see [the site](https://x.y).\n\n## Fonts\n- a\n- b\n",
        );
        assert_eq!(
            blocks,
            [
                NoticeBlock::Heading(1, "Title".into()),
                NoticeBlock::Paragraph("One line wrapped, see the site (https://x.y).".into()),
                NoticeBlock::Heading(2, "Fonts".into()),
                NoticeBlock::Paragraph("• a".into()),
                NoticeBlock::Paragraph("• b".into()),
            ]
        );
        // The bundled notice has content, and it keeps the licences it must.
        let notice = notice_blocks(NOTICE);
        assert!(notice.len() > 5);
        assert!(NOTICE.contains("GNU General Public License"));
    }
}

#[cfg(test)]
mod model_placement_tests {
    use super::model_insert_target;
    use geometry_math::vec::Vec3d;

    #[test]
    fn flat_view_depth_uses_reference_without_moving_cursor_axes() {
        let reference = Vec3d::new(10., 20., 30.);
        for axis in 0..3 {
            let mut target = Vec3d::new(64., 96., 128.);
            target[axis] = f64::NAN;
            let resolved = model_insert_target(Some(target), reference);
            for i in 0..3 {
                assert_eq!(
                    resolved[i],
                    if i == axis { reference[i] } else { target[i] }
                );
            }
        }
        assert_eq!(model_insert_target(None, reference), Vec3d::zero());
        assert_eq!(
            model_insert_target(Some(Vec3d::new(1., 2., 3.)), reference),
            Vec3d::new(1., 2., 3.)
        );
    }
}
