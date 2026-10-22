//! Heron Brush, the application: egui + eframe.
//!
//! Everything behind the window is in `editor_core`: the document, tools, history,
//! drawing and keyboard commands. This crate lays the window out and forwards egui's
//! input. The desktop executable (`main.rs`) calls [`run_desktop`]; the Android library
//! (`crates/heron-brush-android`) calls [`android::run`].

#[cfg(target_os = "android")]
pub mod android;
mod animation_gizmo;
mod animation_page;
mod app_actions;
mod array_dialog;
mod assets_page;
mod build_page;
mod chrome;
mod console;
mod context_menu;
mod detached;
mod dialog;
mod editor;
mod egui_canvas;
mod entity_graph_page;
mod entity_page;
mod export_page;
mod manual_page;
mod menu;
mod plugin_page;
mod preferences_page;
mod shape_dialogs;
mod shortcut_editor;
mod update_page;
mod welcome_page;
// The web build has one page per tab and no command line to hand over.
mod colour_picker;
mod face_tools;
mod file_dialog;
mod form;
mod geometry_pages;
mod gl_glue;
mod idle;
mod inspector;
mod layout;
mod map_page;
mod material_page;
mod paint_gizmo;
#[cfg(test)]
mod perf_tests;
mod pool;
mod profile;
mod render_page;
mod script;
mod session_page;
#[cfg(test)]
mod shell_tests;
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
mod single_instance;
mod texture_gizmo;
mod theme;
mod tool_rows;
mod touch;
mod trigger_panel;
#[cfg(test)]
mod ui_tests;
mod uv_panel;
mod view2d;
mod view3d;
#[cfg(feature = "wgpu")]
mod view3d_wgpu;
#[cfg(target_arch = "wasm32")]
pub mod web;
mod widgets;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod zip;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use editor_core::camera2d::ViewPlane;
use editor_core::commands::{Commands, Window};
use editor_core::i18n::tr;
use eframe::egui;

use chrome::Chrome;
use editor::Editor;
use inspector::{Inspector, Tab};
use view2d::Pane2d;
use view3d::{Gpu, View3d};

/// The application ID desktops group the windows by.
pub(crate) const APP_ID: &str = "io.github.mojzisekjan.HeronBrush";

/// The detached 3D window: the main 3D view moved into a window of its own, so it keeps
/// its camera, its GPU resources and its animation gizmo when it comes back.
pub(crate) struct DetachedWindow {
    /// The main window's 3D view, while it is here.
    pub view: View3d,
    /// Its right-click menu, while it is open.
    pub menu: Option<context_menu::Open>,
    /// The popup open from its header, and the button it hangs from.
    pub popup: Option<(detached::Popup, egui::Rect)>,
}

/// What the keyboard commands ask of the window, kept until the frame acts on it.
#[derive(Default)]
pub(crate) struct WindowState {
    action: Option<app_actions::Action>,
    /// The view arrangement.
    panes: layout::Panes,
    /// Each flat view's size last frame, in the order of the shared cameras.
    flat_sizes: [Option<(f64, f64)>; 3],
    perspective_size: Option<(f64, f64)>,
    /// An inspector tab a shortcut asked for.
    show_page: Option<usize>,
    /// Whether the output pane is showing.
    console: bool,
    /// Whether the inspector column is hidden.
    inspector_hidden: bool,
    /// Whether the toolbar row is hidden.
    toolbar_hidden: bool,
    /// Text a paste brought, handed to the command that asked for the clipboard.
    pasted: Option<String>,
    /// A clipboard read waiting for the paste event's text.
    waiting: Option<Box<dyn FnOnce(String)>>,
    middle_paste_pending: bool,
}

/// The egui window as the commands see it.
struct EguiWindow {
    ctx: egui::Context,
    state: Rc<RefCell<WindowState>>,
}

impl Window for EguiWindow {
    fn redraw(&self) {
        self.ctx.request_repaint();
    }
    fn refresh_inspector(&self) {
        self.ctx.request_repaint();
    }
    fn refresh_toolbar(&self) {
        self.ctx.request_repaint();
    }
    fn flat_view_size(&self, index: usize) -> Option<(f64, f64)> {
        self.state.borrow().flat_sizes.get(index).copied().flatten()
    }
    fn perspective_view_size(&self) -> Option<(f64, f64)> {
        self.state.borrow().perspective_size
    }
    fn focused_view(&self) -> Option<usize> {
        self.state.borrow().panes.focused
    }
    fn cycle_layout(&self) {
        let layout = editor_core::preferences::pref(|p| p.map_view_layout);
        self.state.borrow_mut().panes.cycle(layout);
    }
    fn toggle_maximised(&self) {
        self.state.borrow_mut().panes.toggle_maximised();
    }
    fn show_inspector_page(&self, page: usize) {
        self.state.borrow_mut().show_page = Some(page);
    }
    fn toggle_console(&self) {
        let mut state = self.state.borrow_mut();
        state.console = !state.console;
    }
    fn toggle_inspector(&self) {
        let mut state = self.state.borrow_mut();
        state.inspector_hidden = !state.inspector_hidden;
    }
    fn toggle_toolbar(&self) {
        let mut state = self.state.borrow_mut();
        state.toolbar_hidden = !state.toolbar_hidden;
    }
    fn set_clipboard(&self, text: &str) {
        self.ctx.copy_text(text.to_string());
    }
    fn with_clipboard(&self, then: Box<dyn FnOnce(String)>) {
        // egui hands the clipboard over only with a paste event; the key handling puts
        // that event's text here before it runs the command.
        let pasted = self.state.borrow_mut().pasted.take();
        match pasted {
            Some(text) => then(text),
            None => {
                self.state.borrow_mut().waiting = Some(then);
                // eframe's web backend has no RequestPaste: the page reads the clipboard
                // itself, and the text comes back a frame or two later (`frame`).
                #[cfg(target_arch = "wasm32")]
                web::read_clipboard();
                #[cfg(not(target_arch = "wasm32"))]
                self.ctx
                    .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
        }
    }
    fn go_to(&self) {
        self.state.borrow_mut().action = Some(app_actions::Action::GoTo);
    }
    fn save(&self) {
        self.state.borrow_mut().action = Some(app_actions::Action::Save);
    }
}

/// The keyboard commands every view shares, wired to `editor` and to the window state.
pub(crate) fn commands_for(
    editor: &Editor,
    ctx: &egui::Context,
    window: &Rc<RefCell<WindowState>>,
) -> Commands {
    Commands {
        grid: editor.grid.clone(),
        document: editor.document.clone(),
        tool: editor.tool.clone(),
        clip: editor.clip.clone(),
        assemble: editor.assemble.clone(),
        handles: editor.handles.clone(),
        rotate: editor.rotate.clone(),
        shape_parameters: editor.shape_parameters.clone(),
        cameras: editor.cameras.clone(),
        bounds: editor.bounds,
        paste_target: editor.paste_target.clone(),
        active_plane: editor.active_plane.clone(),
        fillet: editor.fillet.clone(),
        camera_3d: editor.camera.clone(),
        sources: Rc::new(RefCell::new(std::mem::take(
            &mut *editor.sources.borrow_mut(),
        ))),
        window: Rc::new(EguiWindow {
            ctx: ctx.clone(),
            state: Rc::clone(window),
        }),
    }
}

/// The shortcuts that belong to the window rather than to a view, which work even while
/// a text field has the keyboard.
pub(crate) fn window_shortcut(event: &egui::Event) -> Option<app_actions::Action> {
    use app_actions::Action;
    let egui::Event::Key {
        key,
        pressed: true,
        modifiers: m,
        ..
    } = event
    else {
        return None;
    };
    if !(m.command || m.ctrl) || m.alt {
        return None;
    }
    match (key, m.shift) {
        (egui::Key::Comma, false) => Some(Action::Preferences),
        (egui::Key::N, false) => Some(Action::New),
        (egui::Key::O, false) => Some(Action::Open),
        (egui::Key::S, false) => Some(Action::Save),
        (egui::Key::S, true) => Some(Action::SaveAs),
        _ => None,
    }
}

/// Hands key presses to the commands. The
/// window's own shortcuts go to `window` even while a text field has the keyboard
/// (`field_focused`); everything else is the field's then. The 3D view's fly keys come
/// first while it has the keyboard (`scene`).
// The app passes its own fields and the tests build each piece on its own, so there is
// no existing struct that holds them all.
#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_keys(
    commands: &Commands,
    window: &Rc<RefCell<WindowState>>,
    animation: &mut animation_page::AnimationPage,
    editor: &Editor,
    events: Vec<egui::Event>,
    held: egui::Modifiers,
    scene: Option<Rc<editor_core::view3d::Scene3d>>,
    field_focused: bool,
) {
    let modifiers = |m: egui::Modifiers| editor_core::input::Modifiers {
        shift: m.shift,
        control: m.command || m.ctrl,
        alt: m.alt,
    };
    let (binding_text, preset) =
        editor_core::preferences::pref(|p| (p.keyboard_bindings.clone(), p.keymap));
    let bindings = editor_core::custom_shortcuts::load(&binding_text)
        .ok()
        .filter(|b| editor_core::custom_shortcuts::validate(b, preset).is_ok())
        .unwrap_or_default();
    for event in events {
        let chord = match &event {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers: m,
                ..
            } => Some(editor_core::custom_shortcuts::Chord {
                key: view3d::key(*key),
                modifiers: modifiers(*m),
            }),
            egui::Event::Copy => Some(editor_core::custom_shortcuts::Chord {
                key: editor_core::input::Key::Letter('c'),
                modifiers: editor_core::input::Modifiers {
                    control: true,
                    ..Default::default()
                },
            }),
            egui::Event::Cut => Some(editor_core::custom_shortcuts::Chord {
                key: editor_core::input::Key::Letter('x'),
                modifiers: editor_core::input::Modifiers {
                    control: true,
                    ..Default::default()
                },
            }),
            _ => None,
        };
        if let Some(chord) = chord {
            if let Some(command) =
                editor_core::custom_shortcuts::resolve(&bindings, preset, chord, field_focused)
            {
                use app_actions::Action;
                let action = match command.id {
                    "new" => Some(Action::New),
                    "open" => Some(Action::Open),
                    "save" => Some(Action::Save),
                    "save_as" => Some(Action::SaveAs),
                    "preferences" => Some(Action::Preferences),
                    "export" => Some(Action::Export),
                    "build" => Some(Action::Build),
                    "preview" => Some(Action::PreviewAnimations),
                    "undo" => Some(Action::Undo),
                    "redo" => Some(Action::Redo),
                    "copy" => Some(Action::Copy),
                    "cut" => Some(Action::Cut),
                    "paste" => Some(Action::Paste),
                    "duplicate" => Some(Action::Duplicate),
                    "delete" => Some(Action::Delete),
                    "select_all" => Some(Action::SelectAll),
                    "repeat" => Some(Action::Repeat),
                    _ => None,
                };
                if let Some(action) = action {
                    window.borrow_mut().action = Some(action);
                } else if let Ok(canonical) = editor_core::custom_shortcuts::parse(command.default)
                {
                    commands.key_pressed_canonical(canonical.key, None, canonical.modifiers);
                }
                continue;
            }
            if editor_core::custom_shortcuts::suppressed(&bindings, preset, chord, field_focused) {
                continue;
            }
        }
        if let Some(action) = window_shortcut(&event) {
            window.borrow_mut().action = Some(action);
            continue;
        }
        if field_focused {
            continue;
        }
        if animation.key_event(&event, editor) {
            continue;
        }
        match event {
            egui::Event::Key {
                key,
                physical_key,
                pressed: true,
                modifiers: m,
                ..
            } => {
                let core_key = view3d::key(key);
                let mods = modifiers(m);
                if let Some(scene) = &scene {
                    if scene.key(core_key, mods) {
                        continue;
                    }
                }
                let hardware = physical_key.and_then(|k| match view3d::key(k) {
                    editor_core::input::Key::Digit(n) => Some(u32::from(n)),
                    _ => None,
                });
                commands.key_pressed(core_key, hardware, mods);
            }
            // egui has no key for `#`; it arrives as the text it types.
            egui::Event::Text(text) if text == "#" => {
                commands.key_pressed(editor_core::input::Key::NumberSign, None, modifiers(held));
            }
            // egui turns Ctrl+C, Ctrl+X and Ctrl+V into these before they are keys.
            egui::Event::Copy => {
                commands.key_pressed(editor_core::input::Key::Letter('c'), None, modifiers(held));
            }
            egui::Event::Cut => {
                commands.key_pressed(editor_core::input::Key::Letter('x'), None, modifiers(held));
            }
            egui::Event::Paste(text) => {
                let waiting = {
                    let mut state = window.borrow_mut();
                    let explicit_paste = state.middle_paste_pending && (held.command || held.ctrl);
                    state.middle_paste_pending = false;
                    let waiting = state.waiting.take();
                    if explicit_paste {
                        None
                    } else {
                        waiting
                    }
                };
                match waiting {
                    Some(then) => then(text),
                    None => {
                        window.borrow_mut().pasted = Some(text);
                        let mut mods = modifiers(held);
                        mods.control = true;
                        commands.key_pressed(editor_core::input::Key::Letter('v'), None, mods);
                        window.borrow_mut().pasted = None;
                    }
                }
            }
            _ => {}
        }
    }
}

/// The files the first window opens with, from the command line.
#[derive(Default)]
pub(crate) struct Startup {
    /// The map; none shows the welcome screen.
    pub map: Option<PathBuf>,
    /// Entity definitions to use instead of the game's.
    pub fgd: Option<PathBuf>,
    /// Materials to use instead of the game's.
    pub materials: Option<PathBuf>,
}

struct App {
    /// A developer's input script, from `HERON_BRUSH_SCRIPT`.
    script: Option<script::Script>,
    /// The frame-time logger, while `HERON_BRUSH_PROFILE` is set.
    profiler: Option<profile::Profiler>,
    startup: bool,
    pending_map: Option<Option<PathBuf>>,
    /// The map to open once the in-app unsaved-changes question is answered.
    leave_retry: Option<Option<PathBuf>>,
    live: Rc<editor_core::godot_live::LiveLink>,
    dialogs: app_actions::Dialogs,
    exports: export_page::ExportPage,
    builds: build_page::BuildPage,
    updates: update_page::UpdatePage,
    plugins: plugin_page::PluginPage,
    preferences: preferences_page::PreferencesPage,
    shapes: shape_dialogs::ShapeDialogs,
    arrays: array_dialog::ArrayDialog,
    animation: animation_page::AnimationPage,
    manual: manual_page::ManualPage,
    welcome: welcome_page::WelcomePage,
    session: session_page::SessionPage,
    entity_graph: entity_graph_page::EntityGraphPage,
    render_page: render_page::RenderPage,
    mcp: Option<editor_core::live_mcp::Server>,
    mcp_state: editor_core::mcp::Collab,
    autosaver: editor_core::autosave::Autosaver,
    gpu: Gpu,
    editor: Editor,
    chrome: Chrome,
    inspector: Inspector,
    /// The 3D view, while it is in the main window.
    view3d: Option<View3d>,
    /// Top (XY), front (XZ) and side (YZ): views 1, 2 and 3, in the cameras' order.
    flat: [Pane2d; 3],
    /// The window side of the keyboard commands.
    window: Rc<RefCell<WindowState>>,
    /// The keyboard commands every view shares.
    commands: Commands,
    /// The inspector column's width.
    inspector_width: f32,
    /// The 3D view, while it is in a window of its own.
    detached: Option<DetachedWindow>,
    /// The right-click menu, while it is open.
    menu: Option<context_menu::Open>,
    /// Whether the window has been sized and centred.
    placed: bool,
    /// Frames since the scale last changed.
    settled_frames: u32,
    /// The title last sent to the window, so it is sent only when it changes.
    shown_title: String,
    /// Where later launches hand their maps over, while this is the user's editor.
    instance: Option<Instance>,
    /// Tab presses taken from egui before it saw them, for the commands: egui would
    /// otherwise spend Tab moving the keyboard focus between widgets, and after the
    /// first press (whole brush -> vertices) the next one landed in the inspector's
    /// fields instead of stepping on to edges and faces.
    tabs: Vec<egui::Event>,
    /// Touches in the map views, read as gestures before egui sees them.
    touch: touch::Gestures,
    /// Whether the first frame has chosen the touch layout (inspector hidden on a phone).
    touch_layout_set: bool,
    /// The share of the height the inspector takes under the views, upright on a phone.
    inspector_share: f32,
}

/// The single-instance endpoint, kept for as long as the editor runs, and the requests
/// later launches have made through it.
pub(crate) struct Instance {
    _listening: single_instance::Listening,
    inbox: std::sync::mpsc::Receiver<single_instance::Request>,
}

impl Instance {
    /// Starts answering later launches, waking `ctx` for each.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn serve(server: single_instance::Server, ctx: &egui::Context) -> Self {
        let (tx, inbox) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        let listening = server.serve(move |request| {
            let _ = tx.send(request);
            ctx.request_repaint();
        });
        Self {
            _listening: listening,
            inbox,
        }
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, startup: Startup) -> Result<Self, String> {
        theme::install(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        editor_core::preferences::set_current(editor_core::preferences::Preferences::load(
            &editor_core::preferences::Preferences::default_path(),
        ));
        let gpu = Gpu::from_creation_context(cc)?;
        let mut app = Self::build(&cc.egui_ctx, gpu, startup)?;
        app.script = script::Script::from_env();
        if app.script.is_some() {
            cc.egui_ctx.set_embed_viewports(true);
        }
        Ok(app)
    }

    fn build(ctx: &egui::Context, gpu: Gpu, startup: Startup) -> Result<Self, String> {
        let welcome = startup.map.is_none();
        let editor = Editor::open_with(ctx, startup.map, startup.fgd, startup.materials);
        let view3d = View3d::new(&gpu, &editor, Rc::clone(&editor.camera))?;
        let chrome = Chrome {
            tool: editor.tool.clone(),
            grid: editor.grid.clone(),
            document: editor.document.clone(),
            clip: editor.clip.clone(),
            shape_parameters: editor.shape_parameters.clone(),
            fillet: editor.fillet.clone(),
            handles: editor.handles.clone(),
            detach_3d: false,
            action: None,
            filters: editor.filters.clone(),
            popup: None,
            fillet_window: false,
            sculpt_window: false,
        };
        let flat = [
            Pane2d::new(&editor, ViewPlane::Top),
            Pane2d::new(&editor, ViewPlane::Front),
            Pane2d::new(&editor, ViewPlane::Side),
        ];
        let window = Rc::new(RefCell::new(WindowState::default()));
        let commands = commands_for(&editor, ctx, &window);
        let live = editor_core::godot_live::LiveLink::new(&editor.document);
        let autosaver = editor_core::autosave::Autosaver::new(&editor.document.borrow());
        Ok(Self {
            script: None,
            profiler: profile::Profiler::from_env(),
            startup: welcome,
            pending_map: None,
            leave_retry: None,
            live,
            dialogs: app_actions::Dialogs::default(),
            exports: export_page::ExportPage::default(),
            builds: build_page::BuildPage::default(),
            updates: update_page::UpdatePage::default(),
            plugins: plugin_page::PluginPage::default(),
            preferences: preferences_page::PreferencesPage::default(),
            shapes: shape_dialogs::ShapeDialogs::default(),
            arrays: array_dialog::ArrayDialog::default(),
            animation: animation_page::AnimationPage::default(),
            manual: manual_page::ManualPage::default(),
            welcome: {
                let mut page = welcome_page::WelcomePage::default();
                page.open = welcome;
                page
            },
            session: session_page::SessionPage::default(),
            entity_graph: entity_graph_page::EntityGraphPage::default(),
            render_page: render_page::RenderPage::default(),
            mcp: None,
            mcp_state: {
                let mut state = editor_core::mcp::Collab::default();
                state.settings_path = Some(editor_core::preferences::Preferences::default_path());
                state
            },
            autosaver,
            gpu,
            editor,
            chrome,
            inspector: Inspector::default(),
            view3d: Some(view3d),
            flat,
            window,
            commands,
            inspector_width: 340.0,
            detached: None,
            menu: None,
            placed: false,
            settled_frames: 0,
            shown_title: String::new(),
            instance: None,
            tabs: Vec::new(),
            touch: touch::Gestures::default(),
            touch_layout_set: false,
            inspector_share: 0.45,
        })
    }

    fn title(&self) -> String {
        self.editor.document.borrow().window_title()
    }

    /// Hands the frame's key presses to the commands.
    fn keys(&mut self, ui: &egui::Ui) {
        self.editor.sync_face_edit_targets();
        let field_focused = ui.ctx().egui_wants_keyboard_input();
        let mut events = std::mem::take(&mut self.tabs);
        events.extend(ui.input(|i| i.events.clone()));
        let held = ui.input(|i| i.modifiers);
        let scene = if self.window.borrow().panes.focused == Some(0)
            && !self.window.borrow().panes.detached
        {
            self.view3d.as_ref().map(|v| Rc::clone(&v.scene))
        } else {
            None
        };
        self.key_events(events, held, scene, field_focused);
    }

    fn key_events(
        &mut self,
        events: Vec<egui::Event>,
        held: egui::Modifiers,
        scene: Option<Rc<editor_core::view3d::Scene3d>>,
        field_focused: bool,
    ) {
        dispatch_keys(
            &self.commands,
            &self.window,
            &mut self.animation,
            &self.editor,
            events,
            held,
            scene,
            field_focused,
        );
    }

    /// The map views, as the layout arranges them.
    fn views(&mut self, ui: &mut egui::Ui, area: egui::Rect) {
        let layout = editor_core::preferences::pref(|p| p.map_view_layout);
        let placed = {
            let mut window = self.window.borrow_mut();
            window.panes.detached = self.chrome.detach_3d;
            window.panes.arrange(ui, area, layout)
        };
        touch::set_views(ui.ctx(), &placed);

        // A press in a view gives it the keyboard.
        if let Some(press) = ui.input(|i| {
            if i.pointer.any_pressed() {
                i.pointer.press_origin()
            } else {
                None
            }
        }) {
            if let Some((view, _)) = placed.iter().find(|(_, rect)| rect.contains(press)) {
                self.window.borrow_mut().panes.focused = Some(*view);
            }
        }

        let mut sizes = [None; 3];
        let focused = self.window.borrow().panes.focused;
        for &(view, rect) in &placed {
            match view {
                0 => {
                    self.window.borrow_mut().perspective_size =
                        Some((f64::from(rect.width()), f64::from(rect.height())));
                    if let Some(view) = &mut self.view3d {
                        let started = self.profiler.is_some().then(web_time::Instant::now);
                        view.show(ui, rect);
                        if let (Some(profiler), Some(started)) = (&mut self.profiler, started) {
                            profiler.add_3d(started, view.gpu_ms());
                        }
                    }
                }
                n => {
                    let started = self.profiler.is_some().then(web_time::Instant::now);
                    self.flat[n - 1].show(ui, rect);
                    if let (Some(profiler), Some(started)) = (&mut self.profiler, started) {
                        profiler.add_flat(started);
                    }
                    sizes[n - 1] = Some((f64::from(rect.width()), f64::from(rect.height())));
                }
            }
            let pill = layout::view_label(ui, rect, view, focused == Some(view));
            // Without a keyboard's Space, the name steps to the pane's next view.
            if touch::enabled(ui.ctx())
                && ui
                    .interact(
                        pill.expand(6.0),
                        ui.id().with(("view-label", view)),
                        egui::Sense::click(),
                    )
                    .clicked()
            {
                let mut window = self.window.borrow_mut();
                window.panes.focused = Some(view);
                window.panes.cycle(layout);
            }
        }
        self.window.borrow_mut().flat_sizes = sizes;
        touch::modifier_strip(ui.ctx(), area);

        let paste = self
            .view3d
            .as_mut()
            .and_then(|view| view.material_paste.take())
            .or_else(|| {
                self.flat
                    .iter_mut()
                    .find_map(|pane| pane.material_paste.take())
            });
        if let Some(face) = paste {
            let document = self.editor.document.clone();
            let ctx = ui.ctx().clone();
            ctx.memory_mut(|memory| {
                if let Some(id) = memory.focused() {
                    memory.surrender_focus(id);
                }
            });
            self.window.borrow_mut().middle_paste_pending = true;
            EguiWindow {
                ctx: ctx.clone(),
                state: self.window.clone(),
            }
            .with_clipboard(Box::new(move |text| {
                document
                    .borrow_mut()
                    .paste_face_attributes_on(&text, &[face]);
                ctx.request_repaint();
            }));
        }

        // A right-click that stayed put in any view opens the menu for what it was over.
        let click = self
            .view3d
            .as_mut()
            .and_then(|view| view.context_click.take())
            .or_else(|| {
                self.flat
                    .iter_mut()
                    .find_map(|pane| pane.context_click.take())
            });
        if let Some((at, probe)) = click {
            let mut sections =
                editor_core::context_menu::build(&self.editor.document.borrow(), &probe);
            sections.splice(
                0..0,
                editor_core::context_menu::edge_section(self.picked_edges()),
            );
            // Nothing selected and the pointer over nothing: no menu rather than an empty box.
            self.menu = (!sections.is_empty()).then_some(context_menu::Open {
                at,
                sections,
                frame: ui.ctx().cumulative_frame_nr(),
            });
        }
    }

    /// How many edges are picked with the Edge or Fillet tool, for the menu's Fillet entry.
    pub(crate) fn picked_edges(&self) -> usize {
        let tool = *self.editor.tool.borrow();
        if matches!(
            tool,
            editor_core::tool::Tool::Edge | editor_core::tool::Tool::Fillet
        ) {
            self.editor.handles.borrow().selected_edges().len()
        } else {
            0
        }
    }

    /// The right-click menu, while it is open.
    fn context_menu(&mut self, ctx: &egui::Context) {
        let Some(open) = &self.menu else { return };
        let (picked, close) = context_menu::show(ctx, open);
        if let Some(action) = picked {
            self.run_context_action(ctx, action);
        }
        if close {
            self.menu = None;
        }
    }

    fn run_context_action(
        &mut self,
        ctx: &egui::Context,
        action: editor_core::context_menu::Action,
    ) {
        let step = self.editor.grid.borrow().actual_size();
        let outcome =
            editor_core::context_menu::run(&action, &mut self.editor.document.borrow_mut(), step);
        match outcome {
            editor_core::context_menu::Outcome::Done => {}
            editor_core::context_menu::Outcome::Reveal(name) => {
                self.editor.document.borrow_mut().set_current_material(name);
                self.inspector.tab = Tab::Face;
                self.inspector.materials.reveal = true;
            }
            editor_core::context_menu::Outcome::Copy(text) => ctx.copy_text(text),
            editor_core::context_menu::Outcome::Fillet => {
                // Straight to the tool: picking it again from the menu must not toggle it
                // off, and the edges stay picked because Edge and Fillet share them.
                *self.editor.tool.borrow_mut() = editor_core::tool::Tool::Fillet;
                self.chrome.fillet_window = true;
            }
        }
        ctx.request_repaint();
    }

    /// Sends the window title when it differs from the last one sent. Every viewport
    /// command asks egui for another frame, so sending it every frame kept the editor
    /// repainting nonstop: two cores busy on an idle window with a big map.
    fn set_title(&mut self, ctx: &egui::Context, title: String) {
        if self.shown_title != title {
            // eframe's web backend leaves the title alone; the tab shows the page's.
            #[cfg(target_arch = "wasm32")]
            {
                let _ = ctx;
                web::set_title(&title);
            }
            #[cfg(not(target_arch = "wasm32"))]
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.shown_title = title;
        }
    }

    /// What the bars need to know this frame.
    fn status(&self, title: String) -> chrome::Status {
        let document = self.editor.document.borrow();
        let mut indicators = Vec::new();
        let live = self.live.status();
        if let Some(label) = live.indicator {
            indicators.push(chrome::Indicator {
                label,
                tooltip: format!("{}\n{}", live.message, tr("Click to stop the live link")),
                action: app_actions::Action::StopLiveLink,
            });
        }
        // The header words the session's state: hosting, connected, reconnecting.
        if let Some(label) = self.session.indicator() {
            indicators.push(chrome::Indicator {
                tooltip: label.clone(),
                label,
                action: app_actions::Action::Session,
            });
        }
        if let Some(server) = &self.mcp {
            indicators.push(chrome::Indicator {
                label: tr("MCP active").to_string(),
                tooltip: format!("127.0.0.1:{}", server.port),
                action: app_actions::Action::Mcp,
            });
        }
        chrome::Status {
            title,
            can_reload_point: document.can_reload_point_file(),
            point_loaded: document.point_file_loaded(),
            can_reload_portal: document.can_reload_portal_file(),
            portal_loaded: document.portal_file_loaded(),
            can_repeat: document.can_repeat(),
            animation_open: self.animation.open,
            uv_window_open: self.inspector.uv_window_open(),
            inspector_shown: !self.window.borrow().inspector_hidden,
            indicators,
        }
    }

    /// Sizes the window once the display scale has settled and centres it on the monitor
    /// it opened on.
    fn place(&mut self, ui: &egui::Ui) {
        let rescaled = theme::fit_scale(ui.ctx());
        // The zoom takes effect on the next frame, and the viewport info (the monitor size)
        // catches up a frame after that, so the window is placed once two frames have
        // gone by unchanged.
        self.settled_frames = if rescaled { 0 } else { self.settled_frames + 1 };
        if self.settled_frames >= 2 && !self.placed && ui.ctx().native_pixels_per_point().is_some()
        {
            self.placed = true;
            let size = if self.startup {
                egui::vec2(920.0, 620.0)
            } else {
                egui::vec2(1360.0, 760.0)
            };
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            let (monitor, outer) =
                ui.input(|i| (i.viewport().monitor_size, i.viewport().outer_rect));
            if let (Some(monitor), Some(outer)) = (monitor, outer) {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::OuterPosition(centred_on_monitor(
                        outer, monitor, size,
                    )));
            }
        }
        if rescaled {
            ui.ctx().request_repaint();
        }
    }
}

/// Where a window of `size` goes to be centred on the monitor it is on. egui reports
/// the monitor's size but not where it is, so the monitor is taken to be the one of that
/// size whose tile holds the window's centre: exact for a single monitor and for a row
/// of same-sized ones, where the old `(monitor - size) / 2` always went to the first.
pub(crate) fn centred_on_monitor(
    outer: egui::Rect,
    monitor: egui::Vec2,
    size: egui::Vec2,
) -> egui::Pos2 {
    let centre = outer.center();
    let origin = egui::pos2(
        (centre.x / monitor.x).floor() * monitor.x,
        (centre.y / monitor.y).floor() * monitor.y,
    );
    origin + ((monitor - size) / 2.0).max(egui::Vec2::ZERO)
}

/// Lets an undecorated window be resized from its edges and corners, as a window
/// manager's frame would. macOS keeps its native frame (hidden behind the content), which
/// already does this; a phone's app has the whole screen, and its edges are for fingers.
fn resize_edges(ui: &egui::Ui, window: egui::Rect) {
    if cfg!(target_os = "macos")
        || chrome::button_style() == chrome::ButtonStyle::Nothing
        || ui.input(|i| {
            i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
        })
    {
        return;
    }
    use egui::ResizeDirection as D;
    let edge = 5.0;
    let corner = 12.0;
    let (l, r, t, b) = (window.left(), window.right(), window.top(), window.bottom());
    let zones = [
        (
            D::NorthWest,
            egui::Rect::from_min_max(egui::pos2(l, t), egui::pos2(l + corner, t + corner)),
            egui::CursorIcon::ResizeNorthWest,
        ),
        (
            D::NorthEast,
            egui::Rect::from_min_max(egui::pos2(r - corner, t), egui::pos2(r, t + corner)),
            egui::CursorIcon::ResizeNorthEast,
        ),
        (
            D::SouthWest,
            egui::Rect::from_min_max(egui::pos2(l, b - corner), egui::pos2(l + corner, b)),
            egui::CursorIcon::ResizeSouthWest,
        ),
        (
            D::SouthEast,
            egui::Rect::from_min_max(egui::pos2(r - corner, b - corner), egui::pos2(r, b)),
            egui::CursorIcon::ResizeSouthEast,
        ),
        (
            D::North,
            egui::Rect::from_min_max(egui::pos2(l, t), egui::pos2(r, t + edge)),
            egui::CursorIcon::ResizeNorth,
        ),
        (
            D::South,
            egui::Rect::from_min_max(egui::pos2(l, b - edge), egui::pos2(r, b)),
            egui::CursorIcon::ResizeSouth,
        ),
        (
            D::West,
            egui::Rect::from_min_max(egui::pos2(l, t), egui::pos2(l + edge, b)),
            egui::CursorIcon::ResizeWest,
        ),
        (
            D::East,
            egui::Rect::from_min_max(egui::pos2(r - edge, t), egui::pos2(r, b)),
            egui::CursorIcon::ResizeEast,
        ),
    ];
    for (index, (direction, rect, cursor)) in zones.into_iter().enumerate() {
        let response = ui.interact(
            rect,
            egui::Id::new(("window-edge", index)),
            egui::Sense::drag(),
        );
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(cursor);
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.updates.show(ui.ctx());
        // Before the welcome window too, where the language is first chosen.
        #[cfg(target_arch = "wasm32")]
        web::follow_cjk_fonts(ui.ctx());
        if self.profiler.is_none() {
            return self.frame(ui);
        }
        self.profile_begin(ui.ctx(), frame.info().cpu_usage);
        self.frame(ui);
        if let Some(profiler) = &mut self.profiler {
            profiler.end();
            if profiler.orbit_finished() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
    /// Cleared to nothing, so the window's translucent background is what shows.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if let Some(script) = &mut self.script {
            script.feed(ctx, raw_input);
        }
        self.touch.filter(ctx, raw_input);
        #[cfg(target_os = "android")]
        {
            raw_input.safe_area_insets = Some(android::covered(ctx, raw_input));
        }
        // Presses a frame did not hand on (no map open yet) do not wait for a later one.
        self.tabs.clear();
        // A text field that has the keyboard keeps Tab (it moves to the next field);
        // otherwise Tab is the editor's mode key and egui never sees it.
        if !ctx.egui_wants_keyboard_input() {
            let (tabs, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut raw_input.events)
                .into_iter()
                .partition(|event| {
                    matches!(
                        event,
                        egui::Event::Key {
                            key: egui::Key::Tab,
                            ..
                        }
                    )
                });
            raw_input.events = rest;
            self.tabs.extend(tabs);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.live.stop("Window closed");
        profile::report();
        // What this session's texture decodes learnt (alpha classes, average colours),
        // so the next opening of these folders has it before decoding anything.
        if let Ok(document) = self.editor.document.try_borrow() {
            document.save_material_info();
        }
        self.render_page.destroy();
        if let Some(view) = self.view3d.take() {
            view.destroy();
        }
        if let Some(detached) = self.detached.take() {
            detached.view.destroy();
        }
    }
}

impl App {
    /// One frame of the window: everything [`eframe::App::ui`] does, timed or not.
    fn frame(&mut self, ui: &mut egui::Ui) {
        // A paste from the menu in the browser: the clipboard text the page read.
        #[cfg(target_arch = "wasm32")]
        if let Some(text) = web::take_clipboard() {
            let waiting = {
                let mut state = self.window.borrow_mut();
                state.middle_paste_pending = false;
                state.waiting.take()
            };
            if let Some(then) = waiting {
                then(text);
            }
        }
        self.instance_requests(ui.ctx());
        self.chooser_answers(ui.ctx());
        if let Some(path) = self.pending_map.take() {
            self.replace_map(ui.ctx(), path);
        }
        let action = self
            .chrome
            .action
            .take()
            .or_else(|| self.window.borrow_mut().action.take());
        if let Some(action) = action {
            self.action(ui.ctx(), action);
        }
        self.live.tick();
        self.render_page.tick(ui.ctx(), &self.editor, &self.gpu);
        self.session.poll(&self.editor, &mut self.mcp_state);
        if let Some(path) = self.autosaver.trigger(&self.editor.document.borrow()) {
            editor_core::say!("autosaved to {}", path.display());
        }
        // Woken only for what waits on a clock, rather than every second regardless.
        let waiting = idle::Waiting {
            autosave: self.autosaver.next_due(&self.editor.document.borrow()),
            live_link: self.live.status().indicator.is_some(),
            session: self.mcp_state.frontend_active() || self.mcp.is_some(),
            connecting: self.session.busy(),
        };
        if let Some(after) = idle::next_wake(&waiting) {
            ui.ctx().request_repaint_after(after);
        }
        if script::take_preview_toggle() {
            let mut filters = self.editor.filters.borrow_mut();
            filters.preview_motion = !filters.preview_motion;
        }
        // The flat views move with the animation preview every frame.
        {
            let filters = self.editor.filters.borrow();
            if filters.preview_motion && filters.animation_frame.is_none() {
                ui.ctx().request_repaint();
            }
        }
        if let Some(server) = &self.mcp {
            if !self.editor.document.borrow().in_transaction() {
                for _ in 0..32 {
                    let Ok((line, reply)) = server.requests.try_recv() else {
                        break;
                    };
                    let response = editor_core::mcp::respond(
                        &mut self.editor.document.borrow_mut(),
                        &mut self.mcp_state,
                        &line,
                        false,
                    );
                    let _ = reply.send(response.map(|v| v.to_string()).unwrap_or_default());
                }
            }
            if let Some((frame, fps)) = self.mcp_state.animation_preview.take() {
                let mut filters = self.editor.filters.borrow_mut();
                filters.animation_frame = Some(frame);
                filters.animation_fps = fps;
            }
            if let Some(playing) = self.mcp_state.trigger_preview.take() {
                let mut filters = self.editor.filters.borrow_mut();
                filters.preview_motion = playing;
                filters.animation_frame = None;
            }
        }
        theme::sync(ui.ctx());
        touch::style(ui.ctx());
        #[cfg(target_os = "android")]
        android::soft_keyboard(ui.ctx());
        self.place(ui);
        if ui.input(|i| i.viewport().close_requested()) {
            self.render_page.finish_navigation(&self.editor);
            // The safety net under the whole session, whatever the answer turns out to be.
            if let Some(path) = self.autosaver.trigger(&self.editor.document.borrow()) {
                editor_core::say!("autosaved to {}", path.display());
            }
            if !app_actions::can_leave(&mut self.editor.document.borrow_mut()) {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        if let Some(path) = ui.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .find(|p| {
                    p.extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("map"))
                })
        }) {
            self.pending_map = Some(Some(path));
            ui.ctx().request_repaint();
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = ui.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .find(|p| {
                    matches!(
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(str::to_ascii_lowercase)
                            .as_deref(),
                        Some("obj" | "glb")
                    )
                })
        }) {
            self.import_model_file(&path);
        }
        if ui.input(|i| {
            !i.focused
                || i.key_pressed(egui::Key::Escape)
                || ((i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Z))
        }) {
            // Escape that cancels a pose drag goes no further.
            if self.editor.animation_link.borrow().dragging {
                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            }
            for pane in &mut self.flat {
                pane.animation.cancel();
            }
            if let Some(view) = &mut self.view3d {
                view.animation.cancel();
            }
        }
        menu::begin_frame(ui.ctx());
        // Inside what the system leaves uncovered: a phone's cut-out or keyboard.
        let window = ui.max_rect().intersect(ui.ctx().content_rect());
        if self.startup {
            let window = ui.ctx().content_rect();
            ui.set_clip_rect(window);
            self.startup_window(ui, window);
            self.capture_script_snapshot(ui.ctx());
            resize_edges(ui, window);
            return;
        }
        self.keys(ui);
        if let Some(page) = self.window.borrow_mut().show_page.take() {
            self.inspector.tab = match page {
                0 => Tab::Map,
                1 => Tab::Entity,
                _ => Tab::Face,
            };
        }
        let (toolbar_hidden, inspector_hidden, console_shown) = {
            let state = self.window.borrow();
            (state.toolbar_hidden, state.inspector_hidden, state.console)
        };

        ui.painter().rect_filled(ui.max_rect(), 0.0, theme::panel());
        let header =
            egui::Rect::from_min_size(window.min, egui::vec2(window.width(), theme::HEADER_HEIGHT));
        let toolbar_height = if toolbar_hidden {
            0.0
        } else {
            theme::TOOLBAR_HEIGHT
        };
        let toolbar = egui::Rect::from_min_size(
            egui::pos2(window.left(), header.bottom()),
            egui::vec2(window.width(), toolbar_height),
        );
        let mut workspace =
            egui::Rect::from_min_max(egui::pos2(window.left(), toolbar.bottom()), window.max);

        let title = self.title();
        self.set_title(ui.ctx(), title.clone());
        // Closing or reloading the tab asks first while the map has unsaved changes.
        #[cfg(target_arch = "wasm32")]
        {
            web::set_unsaved(self.editor.document.borrow().is_modified());
            web::update_playtest(ui.ctx(), &self.editor.document.borrow());
        }
        let status = self.status(title);
        self.chrome.header(ui, header, &status);
        if !toolbar_hidden {
            self.chrome.toolbar(ui, toolbar);
        }

        if *self.editor.tool.borrow() != editor_core::tool::Tool::Brush {
            self.editor.document.borrow_mut().finish_shape_preview();
        }
        let shape_update = self
            .editor
            .document
            .borrow_mut()
            .update_shape_preview(*self.editor.shape_parameters.borrow());
        if let Err(error) = shape_update {
            if let Some(valid) = self.editor.document.borrow().shape_preview_parameters() {
                *self.editor.shape_parameters.borrow_mut() = valid;
            }
            editor_core::complain!("could not update the shape: {error}");
        }

        // The status strip along the bottom, and the output pane over it when shown.
        let strip = egui::Rect::from_min_max(
            egui::pos2(workspace.left(), workspace.bottom() - console::STRIP_HEIGHT),
            workspace.max,
        );
        if console::strip(ui, strip, console_shown) {
            self.window.borrow_mut().console = !console_shown;
        }
        workspace.max.y = strip.top();
        if console_shown {
            let pane = egui::Rect::from_min_max(
                egui::pos2(workspace.left(), workspace.bottom() - console::HEIGHT),
                workspace.max,
            );
            console::show(ui, pane);
            workspace.max.y = pane.top();
        }

        // The keyframe panel docks under the views, or in the 3D window while that is apart.
        if self.animation.open && !self.animation.detached && self.detached.is_none() {
            // In a small window the views keep at least 240 points (or half the height)
            // rather than shrinking to a strip above a panel of fixed minimum height.
            let room = (workspace.height() - 240.0).max(workspace.height() * 0.5);
            let height = (workspace.height() * 0.68).clamp(260.0, 600.0).min(room);
            let rect = egui::Rect::from_min_max(
                egui::pos2(workspace.left(), workspace.bottom() - height),
                workspace.max,
            );
            let mut panel = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            panel.set_clip_rect(rect);
            self.animation.show_docked(&mut panel, &self.editor);
            workspace.max.y = rect.top();
            ui.painter().hline(
                rect.x_range(),
                rect.top() + 0.5,
                egui::Stroke::new(1.0, theme::border()),
            );
        }

        // A touch screen starts with the views alone when they are small (a phone); the
        // header's button brings the inspector.
        if !self.touch_layout_set {
            self.touch_layout_set = true;
            if touch::enabled(ui.ctx()) && (workspace.width() < 720.0 || workspace.height() < 420.0)
            {
                self.window.borrow_mut().inspector_hidden = true;
                ui.ctx().request_repaint();
            }
        }

        // The workspace: the views, a splitter, and the inspector column, which keeps its
        // width as the window resizes. Upright and narrow (a phone held upright), the
        // inspector goes under the views instead.
        if inspector_hidden {
            self.views(ui, workspace);
        } else if workspace.width() < 600.0 && workspace.height() > workspace.width() {
            let height = (workspace.height() * self.inspector_share)
                .clamp(160.0, (workspace.height() - 160.0).max(160.0));
            let divider_y = workspace.bottom() - height - 1.0;
            let views =
                egui::Rect::from_min_max(workspace.min, egui::pos2(workspace.right(), divider_y));
            let divider = egui::Rect::from_min_max(
                egui::pos2(workspace.left(), divider_y),
                egui::pos2(workspace.right(), divider_y + 1.0),
            );
            let panel = egui::Rect::from_min_max(
                egui::pos2(workspace.left(), divider.bottom()),
                workspace.max,
            );
            self.views(ui, views);
            self.inspector.show(ui, panel, &self.editor);
            let response = ui.interact(
                divider.expand2(egui::vec2(0.0, 8.0)),
                ui.id().with("split-inspector-below"),
                egui::Sense::drag(),
            );
            if let Some(pointer) = response
                .dragged()
                .then(|| response.interact_pointer_pos())
                .flatten()
            {
                self.inspector_share =
                    ((workspace.bottom() - pointer.y) / workspace.height()).clamp(0.2, 0.8);
            }
            let colour = if response.dragged() {
                theme::accent()
            } else {
                theme::border()
            };
            ui.painter().rect_filled(divider, 0.0, colour);
        } else {
            // On a small screen the column may narrow further, and never takes more than
            // half of it.
            let narrowest = if workspace.width() < 720.0 {
                240.0
            } else {
                280.0
            };
            let inspector_width = self
                .inspector_width
                .clamp(narrowest, (workspace.width() - 240.0).max(narrowest))
                .min((workspace.width() * 0.5).max(narrowest));
            let divider_x = workspace.right() - inspector_width - 1.0;
            let views =
                egui::Rect::from_min_max(workspace.min, egui::pos2(divider_x, workspace.bottom()));
            let divider = egui::Rect::from_min_max(
                egui::pos2(divider_x, workspace.top()),
                egui::pos2(divider_x + 1.0, workspace.bottom()),
            );
            let column = egui::Rect::from_min_max(
                egui::pos2(divider.right(), workspace.top()),
                workspace.max,
            );
            self.views(ui, views);
            self.inspector.show(ui, column, &self.editor);
            let response = ui.interact(
                divider.expand2(egui::vec2(4.0, 0.0)),
                ui.id().with("split-inspector"),
                egui::Sense::drag(),
            );
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if let Some(pointer) = response
                .dragged()
                .then(|| response.interact_pointer_pos())
                .flatten()
            {
                self.inspector_width = workspace.right() - pointer.x - 1.0;
            }
            let colour = if response.hovered() || response.dragged() {
                theme::accent()
            } else {
                theme::border()
            };
            ui.painter().rect_filled(divider, 0.0, colour);
        }

        self.chrome.popups(ui, &status);
        resize_edges(ui, window);

        let ctx = ui.ctx().clone();
        self.show_dialogs(&ctx);
        if self.inspector.show_uv_window(&ctx, &self.editor) {
            self.window.borrow_mut().inspector_hidden = false;
        }
        self.manual.show(&ctx);
        if let Some(request) = self.welcome.show(&ctx) {
            self.welcome_request(&ctx, request);
        }
        self.exports.show(&ctx, &self.editor, &self.live);
        self.builds.show(&ctx, &self.editor);
        self.plugins
            .show(&ctx, &self.editor, self.inspector.plugin_cards());
        self.capture_script_snapshot(&ctx);

        // The game names the tags and the face flags, and its path is where a class's
        // model is looked for, so a change to either reaches the open map at once.
        if self.preferences.show(&ctx) {
            preferences_page::apply_game(
                &mut self.editor.document.borrow_mut(),
                editor_core::games::current().as_ref(),
            );
        }
        if self.inspector.corner_requested() {
            self.shapes.corner(&self.editor);
        }
        if self.inspector.plugins_requested() {
            self.plugins.open();
        }
        self.shapes.show(&ctx, &self.editor);
        self.arrays.show(&ctx, &self.editor);
        if self.animation.detached {
            self.animation.show(&ctx, &self.editor);
        }
        self.session.show(&ctx, &self.editor, &mut self.mcp_state);
        self.entity_graph.show(&ctx, &self.editor);
        self.render_page.show(&ctx, &self.editor);
        self.context_menu(&ctx);
        self.detached_window(&ctx);
        file_dialog::show(&ctx);
    }

    /// Starts a profiled frame, and turns the orbiting camera if one was asked for.
    fn profile_begin(&mut self, ctx: &egui::Context, frame_cpu: Option<f32>) {
        let Some(profiler) = &mut self.profiler else {
            return;
        };
        profiler.begin(frame_cpu);
        if let Some(angle) = profiler.orbit_step() {
            if let Some(bounds) = self.editor.document.borrow().bounds() {
                let mut camera = self.editor.camera.borrow_mut();
                let centre = bounds.center();
                let reach = geometry_math::vec::length(bounds.size())
                    / 2.0
                    / (camera.fov_degrees.to_radians() / 2.0).tan();
                let from = geometry_math::vec::Vec3d::new(angle.cos(), angle.sin(), 0.35)
                    * reach.max(64.0);
                camera.position = centre + from;
                camera.look_along(-from);
            }
            ctx.request_repaint();
        }
    }
}

impl App {
    fn capture_script_snapshot(&self, ctx: &egui::Context) {
        if let Some(path) = script::take_snapshot() {
            let doc = self.editor.document.borrow();
            let imported_models: Vec<_> = map_model::node::point_entities_by_layer(doc.tree_world())
                .into_iter()
                .filter_map(|(_, entity, _)| {
                    let reference = entity.property("_heron_model")?;
                    let model = doc.imported_model(entity);
                    Some(serde_json::json!({
                        "reference": reference,
                        "triangles": model.as_ref().map_or(0, |model| model.triangles.len()),
                        "skin_size": model.as_ref().map(|model| [model.skin.width, model.skin.height]),
                        "skin_count": model.as_ref().map_or(0, |model| model.skins().count()),
                        "smooth_triangles": model.as_ref().map_or(0, |model| model.triangles.iter().filter(|t| t.normals.is_some()).count()),
                        "material_indices": model.as_ref().map(|model| model.triangles.iter().map(|t| t.material).collect::<Vec<_>>()),
                        "bounds": model.as_ref().map(|model| [
                            [model.bounds.min.x(), model.bounds.min.y(), model.bounds.min.z()],
                            [model.bounds.max.x(), model.bounds.max.y(), model.bounds.max.z()],
                        ]),
                    }))
                })
                .collect();
            let camera = *self.editor.camera.borrow();
            let evidence = serde_json::json!({"camera_position":[camera.position[0],camera.position[1],camera.position[2]],"map":doc.to_map_text(),"modified":doc.is_modified(),"shape_preview":doc.has_shape_preview(),"current_material":doc.current_material(),"imported_models":imported_models,"brushes":doc.brush_count(),"revision":doc.revision(),"selection":doc.selection_ids().len(),"tool":self.editor.tool.borrow().name(),"face_selection":doc.face_selection_count(),"simulation_preview":self.editor.filters.borrow().preview_motion,"pixels_per_point":ctx.pixels_per_point(),"screen_points":[ctx.content_rect().width(),ctx.content_rect().height()]});
            match std::fs::write(
                &path,
                serde_json::to_vec_pretty(&evidence).expect("snapshot JSON"),
            ) {
                Ok(()) => println!("script snapshot: {}", path.display()),
                Err(error) => eprintln!("script snapshot: {error}"),
            }
        }
    }

    /// The startup window: the welcome screen under a title bar of its own. In the
    /// browser the tab carries the title, so the page starts with the welcome screen.
    fn startup_window(&mut self, ui: &mut egui::Ui, window: egui::Rect) {
        let ctx = ui.ctx().clone();
        let title = tr("Welcome to Heron Brush");
        self.set_title(&ctx, title.into());
        ui.painter().rect_filled(window, 0.0, theme::bg());
        let bar_height = if cfg!(target_arch = "wasm32") {
            0.0
        } else {
            theme::HEADER_HEIGHT
        };
        let bar = egui::Rect::from_min_size(window.min, egui::vec2(window.width(), bar_height));
        if bar_height > 0.0 {
            ui.painter().rect_filled(bar, 0.0, theme::base());
            ui.painter().hline(
                bar.x_range(),
                bar.bottom() - 0.5,
                egui::Stroke::new(1.0, theme::border()),
            );
            let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            chrome::title_bar_drag(ui, bar, "welcome-title", maximized);
            // No logo here: the welcome screen's own column starts with a large one.
            crate::theme::text(
                ui.painter(),
                bar.center(),
                egui::Align2::CENTER_CENTER,
                title,
                theme::body(),
                theme::dim(),
            );
            chrome::window_buttons(ui, bar, maximized);
        }

        let body = egui::Rect::from_min_max(egui::pos2(window.left(), bar.bottom()), window.max);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let request = self.welcome.show_in(&mut child);
        if let Some(request) = request {
            self.welcome_request(&ctx, request);
        }
        self.manual.show(&ctx);
        if self.preferences.show(&ctx) {
            preferences_page::apply_game(
                &mut self.editor.document.borrow_mut(),
                editor_core::games::current().as_ref(),
            );
        }
        file_dialog::show(&ctx);
    }

    /// What later launches asked for: the window comes to the front, and a map they
    /// named opens as File > Open would open it, asking about unsaved changes first.
    fn instance_requests(&mut self, ctx: &egui::Context) {
        let Some(instance) = &self.instance else {
            return;
        };
        while let Ok(request) = instance.inbox.try_recv() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            if let single_instance::Request::Open(path) = request {
                self.pending_map = Some(Some(path));
            }
            ctx.request_repaint();
        }
    }

    fn welcome_request(&mut self, ctx: &egui::Context, request: welcome_page::Request) {
        match request {
            welcome_page::Request::Action(action) => self.action(ctx, action),
            welcome_page::Request::Map(path) => {
                self.pending_map = Some(Some(path));
                ctx.request_repaint();
            }
        }
    }
}

/// Asks eframe's wgpu device for the optional features `render_wgpu` can use, as far as
/// the adapter has them: timestamp queries, for the per-pass GPU times profiling prints.
/// Without them everything still works; the renderer just has no GPU timer. Later phases
/// add indirect drawing and texture compression here.
///
/// On Android the limits are the adapter's own: egui asks for wgpu's desktop defaults,
/// which some phone GPUs fall short of in a limit the renderer never comes near, and the
/// device would then not open at all.
#[cfg(feature = "wgpu")]
fn request_device(options: &mut eframe::egui_wgpu::WgpuConfiguration) {
    use eframe::egui_wgpu::{wgpu, WgpuSetup};
    if let WgpuSetup::CreateNew(setup) = &mut options.wgpu_setup {
        let base = std::sync::Arc::clone(&setup.device_descriptor);
        setup.device_descriptor = std::sync::Arc::new(move |adapter: &wgpu::Adapter| {
            let mut descriptor = base(adapter);
            descriptor.required_features |= adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
            if cfg!(target_os = "android") {
                descriptor.required_limits = adapter.limits();
            }
            descriptor
        });
    }
}

/// The editor, for eframe's app creator on Android and on the web.
#[cfg(any(target_os = "android", target_arch = "wasm32"))]
fn create_app(
    cc: &eframe::CreationContext<'_>,
    startup: Startup,
) -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Box::new(App::new(cc, startup)?))
}

/// The desktop application: the command line, the single-instance hand-over, then the
/// window.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_desktop() -> eframe::Result {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let new_instance = single_instance::strip_new_instance(&mut args);
    let command = editor_core::cli::parse(&args);
    let version = format!("Heron Brush {}", env!("CARGO_PKG_VERSION"));
    if command == editor_core::cli::Command::Mcp {
        editor_core::mcp::serve();
    }
    if let Some(code) = editor_core::cli::run(&command, "heron-brush", &version) {
        if command == editor_core::cli::Command::Help {
            println!("       heron-brush {} [map.map ...]   a window of its own, rather than handing the map to the running editor", single_instance::NEW_INSTANCE);
        }
        std::process::exit(code);
    }
    let editor_core::cli::Command::Open(files) = command else {
        return Ok(());
    };

    // Hand the map to the editor already running. Not with
    // definitions or materials named, which are the new session's own, and not under a
    // developer's input script, which needs a window of its own.
    let solo = new_instance
        || files.fgd.is_some()
        || files.materials.is_some()
        || std::env::var_os("HERON_BRUSH_SCRIPT").is_some();
    let server = if solo {
        None
    } else {
        let request = match &files.map {
            Some(map) => single_instance::Request::Open(
                std::path::absolute(map).unwrap_or_else(|_| map.clone()),
            ),
            None => single_instance::Request::Activate,
        };
        match single_instance::claim(&single_instance::Endpoint::current(), &request) {
            single_instance::Claim::Forwarded => return Ok(()),
            single_instance::Claim::Primary(server) => Some(server),
            single_instance::Claim::Alone(reason) => {
                eprintln!("heron-brush: running on its own ({reason})");
                None
            }
        }
    };
    let startup = Startup {
        map: files.map,
        fgd: files.fgd,
        materials: files.materials,
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Heron Brush")
        .with_app_id(APP_ID)
        // Small enough to fit any screen at any scale, so the window manager does not
        // maximise it on creation; the first frames set the real size once the display
        // scale is known.
        .with_inner_size([800.0, 450.0])
        .with_min_inner_size([640.0, 400.0])
        // The panels' background is translucent at the "Panel opacity" preference.
        .with_transparent(true);
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/app-icons/heron-brush.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    viewport = if cfg!(target_os = "macos") {
        // The native frame stays for its edge resizing, shadow, rounded corners and the
        // red, yellow and green buttons; its title bar goes transparent behind the
        // content, where the app draws its own header around those buttons.
        viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_titlebar_buttons_shown(chrome::button_style() == chrome::ButtonStyle::Mac)
    } else {
        viewport.with_decorations(false)
    };
    // With the `wgpu` feature the window and the 3D view run on wgpu (render_wgpu);
    // without it, on GL with the shared renderer (crates/render).
    #[cfg(feature = "wgpu")]
    let renderer = eframe::Renderer::Wgpu;
    #[cfg(not(feature = "wgpu"))]
    let renderer = eframe::Renderer::Glow;
    #[cfg_attr(not(feature = "wgpu"), allow(unused_mut))]
    let mut options = eframe::NativeOptions {
        renderer,
        viewport,
        ..Default::default()
    };
    #[cfg(feature = "wgpu")]
    request_device(&mut options.wgpu_options);
    eframe::run_native(
        "heron-brush",
        options,
        Box::new(move |cc| {
            let mut app = App::new(cc, startup)?;
            app.instance = server.map(|server| Instance::serve(server, &cc.egui_ctx));
            Ok(Box::new(app))
        }),
    )
}
