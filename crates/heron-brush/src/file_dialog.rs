//! Choosing files and asking questions: the system's dialogs (rfd) on the desktop, and an
//! in-app file browser and message box where the system has none (Android, and for now
//! the web build), or anywhere
//! with `HERON_BRUSH_IN_APP_FILES=1` set, which is how the Android path is tried on a
//! desktop. A developer's input script (`HERON_BRUSH_SCRIPT`) gets the in-app ones too: a
//! system dialog would block the window, and the script with it, until someone answered.
//!
//! The system dialogs block and answer at once; the in-app ones answer frames later. So a
//! call site asks every frame, with `open` true on the frame it wants the chooser, and
//! takes the path on whichever frame it arrives:
//!
//! ```ignore
//! if let Some(path) = FileDialog::new().add_filter("Maps", &["map"]).pick_file(button.clicked()) { … }
//! ```
//!
//! The answer goes back to the call site that asked, known by its place in the source, or
//! by [`FileDialog::id`] where the asking and the answering are in different places.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use editor_core::i18n::tr;
use eframe::egui;

use crate::{dialog, theme};

/// Whether choosers and questions are drawn in the window rather than by the system.
pub fn in_app() -> bool {
    static IN_APP: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *IN_APP.get_or_init(|| {
        cfg!(any(target_os = "android", target_arch = "wasm32"))
            || std::env::var_os("HERON_BRUSH_IN_APP_FILES")
                .is_some_and(|v| !v.is_empty() && v != "0")
            || std::env::var_os("HERON_BRUSH_SCRIPT").is_some()
    })
}

/// What a chooser picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Open,
    Save,
    Folder,
}

/// A file or folder chooser, built like rfd's.
#[derive(Default)]
pub struct FileDialog {
    id: Option<String>,
    title: Option<String>,
    filters: Vec<(String, Vec<String>)>,
    directory: Option<PathBuf>,
    file_name: Option<String>,
}

impl FileDialog {
    /// A chooser with no title, filters or starting place.
    pub fn new() -> Self {
        Self::default()
    }

    /// Names the request, for an answer taken somewhere other than where it was asked.
    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    /// The window title.
    pub fn set_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Shows only files with these extensions (without the dot).
    pub fn add_filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
        self.filters.push((
            name.into(),
            extensions.iter().map(|e| e.to_string()).collect(),
        ));
        self
    }

    /// Where the chooser starts.
    pub fn set_directory(mut self, directory: impl AsRef<Path>) -> Self {
        self.directory = Some(directory.as_ref().to_path_buf());
        self
    }

    /// The name a save starts with.
    pub fn set_file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    /// An existing file. `open` shows the chooser; the path comes back once chosen.
    #[track_caller]
    pub fn pick_file(self, open: bool) -> Option<PathBuf> {
        let key = self.key(std::panic::Location::caller());
        self.ask(key, Kind::Open, open)
    }

    /// A folder. `open` shows the chooser; the path comes back once chosen.
    #[track_caller]
    pub fn pick_folder(self, open: bool) -> Option<PathBuf> {
        let key = self.key(std::panic::Location::caller());
        self.ask(key, Kind::Folder, open)
    }

    /// Where to save. `open` shows the chooser; the path comes back once chosen.
    #[track_caller]
    pub fn save_file(self, open: bool) -> Option<PathBuf> {
        let key = self.key(std::panic::Location::caller());
        self.ask(key, Kind::Save, open)
    }

    fn key(&self, caller: &std::panic::Location<'_>) -> String {
        self.id
            .clone()
            .unwrap_or_else(|| format!("{}:{}:{}", caller.file(), caller.line(), caller.column()))
    }

    fn ask(self, key: String, kind: Kind, open: bool) -> Option<PathBuf> {
        // In the browser files and folders are chosen with the browser's own choosers,
        // which answer frames later (`deliver`). A folder comes in as a copy.
        #[cfg(target_arch = "wasm32")]
        {
            if open {
                if kind == Kind::Folder {
                    crate::web::choose_folder(key.clone());
                } else {
                    let extensions = self
                        .filters
                        .iter()
                        .flat_map(|(_, extensions)| extensions.iter().cloned())
                        .collect();
                    crate::web::choose_file(
                        key.clone(),
                        kind == Kind::Save,
                        extensions,
                        self.file_name.clone(),
                    );
                }
            }
            take_answer(&key)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !in_app() {
                return if open { self.system(kind) } else { None };
            }
            if open {
                let browser = Browser::new(key.clone(), kind, self);
                STATE.with(|state| state.borrow_mut().browser = Some(browser));
            }
            take_answer(&key)
        }
    }

    /// The system's dialog, which answers before it returns.
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    fn system(self, kind: Kind) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new();
        if let Some(title) = self.title {
            dialog = dialog.set_title(title);
        }
        for (name, extensions) in &self.filters {
            dialog = dialog.add_filter(name, extensions);
        }
        if let Some(directory) = self.directory {
            dialog = dialog.set_directory(directory);
        }
        if let Some(name) = self.file_name {
            dialog = dialog.set_file_name(name);
        }
        match kind {
            Kind::Open => dialog.pick_file(),
            Kind::Save => dialog.save_file(),
            Kind::Folder => dialog.pick_folder(),
        }
    }

    #[cfg(target_os = "android")]
    fn system(self, _kind: Kind) -> Option<PathBuf> {
        None
    }
}

/// The answer to the request `key`, once there is one; another request's answer stays.
fn take_answer(key: &str) -> Option<PathBuf> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        match state.answer.take() {
            Some((asked, path)) if asked == key => path,
            other => {
                state.answer = other;
                None
            }
        }
    })
}

/// Hands in the answer to the request `key` from the browser's chooser.
#[cfg(target_arch = "wasm32")]
pub fn deliver(key: String, path: Option<PathBuf>) {
    STATE.with(|state| state.borrow_mut().answer = Some((key, path)));
}

/// Asks a question with a row of buttons; returns the index of the button chosen, or
/// `None` while it is unanswered (and when it was dismissed). `open` asks it. On the
/// desktop the system's message box answers at once.
pub fn question(
    key: &str,
    open: bool,
    title: &str,
    message: &str,
    buttons: &[&str],
) -> Option<usize> {
    if !in_app() {
        return if open {
            system_question(title, message, buttons)
        } else {
            None
        };
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if open {
            state.question = Some(Question {
                key: key.to_string(),
                title: title.to_string(),
                message: message.to_string(),
                buttons: buttons.iter().map(|b| b.to_string()).collect(),
            });
        }
        match state.answered.take() {
            Some((asked, choice)) if asked == key => choice,
            other => {
                state.answered = other;
                None
            }
        }
    })
}

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
fn system_question(title: &str, message: &str, buttons: &[&str]) -> Option<usize> {
    let choice = match buttons {
        [yes, no] => rfd::MessageButtons::OkCancelCustom(yes.to_string(), no.to_string()),
        [yes, no, cancel] => rfd::MessageButtons::YesNoCancelCustom(
            yes.to_string(),
            no.to_string(),
            cancel.to_string(),
        ),
        _ => rfd::MessageButtons::Ok,
    };
    let answer = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title(title)
        .set_description(message)
        .set_buttons(choice)
        .show();
    match answer {
        rfd::MessageDialogResult::Custom(label) => buttons.iter().position(|b| *b == label),
        rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes => Some(0),
        rfd::MessageDialogResult::No => Some(1),
        rfd::MessageDialogResult::Cancel => None,
    }
}

#[cfg(any(target_os = "android", target_arch = "wasm32"))]
fn system_question(_title: &str, _message: &str, _buttons: &[&str]) -> Option<usize> {
    None
}

/// Where the in-app browser offers to go: a name and a folder.
pub type Place = (String, PathBuf);

/// Sets the places the in-app browser lists along its top (Android sets its storage).
#[cfg(target_os = "android")]
pub fn set_places(places: Vec<Place>) {
    STATE.with(|state| state.borrow_mut().places = places);
}

/// Whether an in-app chooser or question is showing.
pub fn showing() -> bool {
    STATE.with(|state| {
        let state = state.borrow();
        state.browser.is_some() || state.question.is_some()
    })
}

#[derive(Default)]
struct State {
    browser: Option<Browser>,
    answer: Option<(String, Option<PathBuf>)>,
    question: Option<Question>,
    answered: Option<(String, Option<usize>)>,
    places: Vec<Place>,
    /// The folder the last chooser was in, where the next starts when it names none.
    last: Option<PathBuf>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

struct Question {
    key: String,
    title: String,
    message: String,
    buttons: Vec<String>,
}

/// A folder's entry.
struct Entry {
    name: String,
    path: PathBuf,
    folder: bool,
}

struct Browser {
    key: String,
    kind: Kind,
    title: String,
    /// Lower-case extensions a file must have to be listed; empty lists everything.
    extensions: Vec<String>,
    folder: PathBuf,
    entries: Vec<Entry>,
    error: Option<String>,
    /// The file picked in the list (to open) or typed (to save).
    name: String,
}

impl Browser {
    // The web build chooses with the browser's own dialogs; the in-app browser stays for
    // Android and HERON_BRUSH_IN_APP_FILES.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn new(key: String, kind: Kind, request: FileDialog) -> Self {
        let title = request.title.unwrap_or_else(|| {
            tr(match kind {
                Kind::Open => "Open",
                Kind::Save => "Save",
                Kind::Folder => "Choose",
            })
            .to_string()
        });
        let extensions = request
            .filters
            .iter()
            .flat_map(|(_, list)| list.iter().map(|e| e.to_ascii_lowercase()))
            .collect();
        let folder = request
            .directory
            .filter(|d| d.is_dir())
            .or_else(|| {
                STATE
                    .with(|s| s.borrow().last.clone())
                    .filter(|d| d.is_dir())
            })
            .or_else(|| {
                default_places()
                    .into_iter()
                    .map(|(_, p)| p)
                    .find(|p| p.is_dir())
            })
            .unwrap_or_else(|| PathBuf::from("/"));
        let mut browser = Self {
            key,
            kind,
            title,
            extensions,
            folder: folder.clone(),
            entries: Vec::new(),
            error: None,
            name: request.file_name.unwrap_or_default(),
        };
        browser.go(folder);
        browser
    }

    /// Lists `folder`: folders first, then the files the filters allow, each by name.
    fn go(&mut self, folder: PathBuf) {
        match std::fs::read_dir(&folder) {
            Ok(read) => {
                let mut entries: Vec<Entry> = read
                    .flatten()
                    .filter_map(|entry| {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if name.starts_with('.') {
                            return None;
                        }
                        let path = entry.path();
                        let folder = path.is_dir();
                        let shown = folder || (self.kind != Kind::Folder && self.allows(&path));
                        shown.then_some(Entry { name, path, folder })
                    })
                    .collect();
                entries.sort_by(|a, b| {
                    b.folder
                        .cmp(&a.folder)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
                self.entries = entries;
                self.error = None;
                if self.kind == Kind::Open {
                    self.name.clear();
                }
                self.folder = folder;
            }
            Err(error) => self.error = Some(format!("{}: {error}", folder.display())),
        }
    }

    fn allows(&self, path: &Path) -> bool {
        self.extensions.is_empty()
            || path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| self.extensions.iter().any(|x| x.eq_ignore_ascii_case(e)))
    }

    /// The path the default button would hand back.
    fn chosen(&self) -> Option<PathBuf> {
        match self.kind {
            Kind::Folder => Some(self.folder.clone()),
            Kind::Open | Kind::Save => {
                let name = self.name.trim();
                (!name.is_empty() && !name.contains('/')).then(|| self.folder.join(name))
            }
        }
    }

    /// One frame of the browser. Returns the answer once there is one: a path, or `None`
    /// inside for a cancel.
    fn show(&mut self, ctx: &egui::Context, places: &[Place]) -> Option<Option<PathBuf>> {
        let mut answer = None;
        let mut go = None;
        let title = self.title.clone();
        let row_height = if crate::touch::enabled(ctx) {
            44.0
        } else {
            30.0
        };
        let open = dialog::show_modal(
            ctx,
            "in-app-file-browser",
            &title,
            [620.0, 560.0],
            |ui, part| match part {
                dialog::Part::Body => {
                    // The places as a row of buttons, the one holding this folder in the accent;
                    // the home folder by its house and its name rather than "~".
                    ui.horizontal_wrapped(|ui| {
                        let home = std::env::var_os("HOME").map(PathBuf::from);
                        let here = places
                            .iter()
                            .filter(|(_, path)| self.folder.starts_with(path))
                            .max_by_key(|(_, path)| path.as_os_str().len())
                            .map(|(_, path)| path.clone());
                        for (name, path) in places {
                            let is_home = home.as_ref() == Some(path);
                            let label = if is_home {
                                path.file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| name.clone())
                            } else {
                                name.clone()
                            };
                            let icon = if is_home {
                                egui_phosphor::regular::HOUSE
                            } else {
                                egui_phosphor::regular::FOLDER_SIMPLE
                            };
                            if crate::widgets::icon_label_button(
                                ui,
                                icon,
                                &label,
                                here.as_ref() == Some(path),
                            )
                            .clicked()
                            {
                                go = Some(path.clone());
                            }
                        }
                    });
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        let up = self.folder.parent().map(Path::to_path_buf);
                        if crate::widgets::icon_label_button(
                            ui,
                            egui_phosphor::regular::ARROW_UP,
                            "",
                            false,
                        )
                        .clicked()
                            && up.is_some()
                        {
                            go = up;
                        }
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(self.folder.display().to_string())
                                    .color(theme::dim()),
                            )
                            .truncate(),
                        );
                    });
                    if let Some(error) = &self.error {
                        ui.colored_label(theme::error(), error);
                    }
                    let list_height = (ui.available_height()
                        - if self.kind == Kind::Save {
                            theme::CONTROL_HEIGHT + 12.0
                        } else {
                            0.0
                        })
                    .max(row_height * 2.0);
                    egui::ScrollArea::vertical()
                        .max_height(list_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for entry in &self.entries {
                                let picked = !entry.folder && entry.name == self.name;
                                let (rect, response) = crate::widgets::list_row(
                                    ui,
                                    row_height,
                                    picked,
                                    egui::Sense::click(),
                                );
                                let icon = if entry.folder {
                                    egui_phosphor::regular::FOLDER_SIMPLE
                                } else {
                                    egui_phosphor::regular::FILE
                                };
                                let colour = if entry.folder || self.kind != Kind::Folder {
                                    theme::fg()
                                } else {
                                    theme::disabled()
                                };
                                let icon_colour = if entry.folder {
                                    theme::accent_text()
                                } else {
                                    theme::dim()
                                };
                                crate::theme::icon_text(
                                    ui.painter(),
                                    rect.left_center() + egui::vec2(18.0, 0.0),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    theme::icon(),
                                    icon_colour,
                                );
                                crate::theme::text(
                                    ui.painter(),
                                    rect.left_center() + egui::vec2(34.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    &entry.name,
                                    theme::body(),
                                    colour,
                                );
                                if response.clicked() {
                                    if entry.folder {
                                        go = Some(entry.path.clone());
                                    } else if self.kind != Kind::Folder {
                                        // A tap picks; a second tap on the picked file (or a
                                        // double click) takes it.
                                        if picked || response.double_clicked() {
                                            answer = Some(Some(entry.path.clone()));
                                        }
                                        self.name = entry.name.clone();
                                    }
                                }
                            }
                        });
                    if self.kind == Kind::Save {
                        ui.add_space(6.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.name)
                                .hint_text("name.map")
                                .desired_width(f32::INFINITY)
                                .margin(crate::widgets::body_entry_margin(ui, 6, 6))
                                .min_size(egui::vec2(0.0, crate::widgets::ROW_HEIGHT)),
                        );
                    }
                }
                dialog::Part::Actions => {
                    let label = match self.kind {
                        Kind::Open => tr("Open"),
                        Kind::Save => tr("Save"),
                        Kind::Folder => tr("Choose"),
                    };
                    let chosen = self.chosen();
                    if dialog::accent_button(ui, label, chosen.is_some()).clicked() {
                        answer = Some(chosen);
                    }
                    if dialog::button(ui, tr("Cancel"), true).clicked() {
                        answer = Some(None);
                    }
                }
            },
        );
        if let Some(folder) = go {
            self.go(folder);
        }
        if !open {
            answer = Some(None);
        }
        answer
    }
}

/// The places offered when nobody set any (trying the in-app browser on a desktop): the
/// home folder, the examples and the working folder, the first and last by their paths.
fn default_places() -> Vec<Place> {
    let mut places = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        places.push(("~".to_string(), home));
    }
    places.push((
        tr("Examples").to_string(),
        editor_core::samples::directory(),
    ));
    if let Ok(here) = std::env::current_dir() {
        places.push((
            here.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| here.display().to_string()),
            here,
        ));
    }
    places
}

/// Draws the in-app chooser or question, while one is asked. Call once a frame.
pub fn show(ctx: &egui::Context) {
    if !in_app() {
        return;
    }
    let (browser, places) = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let places = if state.places.is_empty() {
            default_places()
        } else {
            state.places.clone()
        };
        (state.browser.take(), places)
    });
    if let Some(mut browser) = browser {
        let answer = browser.show(ctx, &places);
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.last = Some(browser.folder.clone());
            match answer {
                Some(path) => {
                    state.answer = Some((browser.key.clone(), path));
                    ctx.request_repaint();
                }
                None => state.browser = Some(browser),
            }
        });
    }
    let question = STATE.with(|state| state.borrow_mut().question.take());
    if let Some(question) = question {
        let mut choice = None;
        let open = dialog::show_modal(
            ctx,
            "in-app-question",
            &question.title,
            [460.0, 220.0],
            |ui, part| match part {
                dialog::Part::Body => {
                    ui.label(&question.message);
                }
                dialog::Part::Actions => {
                    // Right to left: the first button is the rightmost, in the accent.
                    for (index, label) in question.buttons.iter().enumerate() {
                        let clicked = if index == 0 {
                            dialog::accent_button(ui, label, true).clicked()
                        } else {
                            dialog::button(ui, label, true).clicked()
                        };
                        if clicked {
                            choice = Some(Some(index));
                        }
                    }
                }
            },
        );
        if !open {
            choice = Some(None);
        }
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            match choice {
                Some(choice) => {
                    state.answered = Some((question.key.clone(), choice));
                    ctx.request_repaint();
                }
                None => state.question = Some(question),
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_browser_lists_folders_first_and_only_the_files_the_filter_allows() {
        let dir = std::env::temp_dir().join(format!("heron-file-browser-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("b-folder")).unwrap();
        std::fs::write(dir.join("a.map"), "").unwrap();
        std::fs::write(dir.join("c.MAP"), "").unwrap();
        std::fs::write(dir.join("notes.txt"), "").unwrap();
        std::fs::write(dir.join(".hidden.map"), "").unwrap();
        let browser = Browser::new(
            "k".into(),
            Kind::Open,
            FileDialog::new()
                .add_filter("Maps", &["map"])
                .set_directory(&dir),
        );
        let names: Vec<&str> = browser.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["b-folder", "a.map", "c.MAP"]);
        let folders = Browser::new(
            "k".into(),
            Kind::Folder,
            FileDialog::new().set_directory(&dir),
        );
        assert_eq!(folders.entries.len(), 1);
        assert_eq!(folders.chosen(), Some(dir.clone()));
        let mut save = Browser::new(
            "k".into(),
            Kind::Save,
            FileDialog::new()
                .set_directory(&dir)
                .set_file_name("new.map"),
        );
        assert_eq!(save.chosen(), Some(dir.join("new.map")));
        save.name = "  ".into();
        assert_eq!(save.chosen(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The browser draws inside the window (as on Android, where viewports are embedded)
    /// and waits for an answer; a tap on a file picks it, a second tap takes it.
    #[test]
    fn the_browser_draws_in_the_window_and_a_second_tap_opens() {
        let dir =
            std::env::temp_dir().join(format!("heron-file-browser-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.map"), "").unwrap();
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let mut browser = Browser::new(
            "k".into(),
            Kind::Open,
            FileDialog::new()
                .add_filter("Maps", &["map"])
                .set_directory(&dir),
        );
        let places = vec![("Here".to_string(), dir.clone())];
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let frame = |browser: &mut Browser, events: Vec<egui::Event>| {
            let raw = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            let mut answer = None;
            let mut output = ctx.run_ui(raw, |ui| answer = browser.show(ui.ctx(), &places));
            output.textures_delta.clear();
            (output.shapes, answer)
        };
        frame(&mut browser, vec![]);
        let (shapes, answer) = frame(&mut browser, vec![]);
        assert_eq!(answer, None, "nothing chosen yet");
        // Where the file's row was drawn: the text shape that names it.
        let mut row = None;
        for clipped in &shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                if text.galley.text().contains("a.map") {
                    row = Some(text.pos + egui::vec2(4.0, text.galley.size().y / 2.0));
                }
            }
        }
        let at = row.expect("the file is listed");
        let click = |at| {
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ]
        };
        let (_, answer) = frame(&mut browser, click(at));
        assert_eq!(answer, None, "the first tap picks");
        assert_eq!(browser.name, "a.map");
        frame(&mut browser, vec![]);
        let (_, answer) = frame(&mut browser, click(at));
        assert_eq!(answer, Some(Some(dir.join("a.map"))), "the second opens");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_answer_goes_only_to_the_call_site_that_asked() {
        STATE.with(|state| {
            state.borrow_mut().answer = Some(("open-map".into(), Some(PathBuf::from("/x.map"))))
        });
        if in_app() {
            assert_eq!(FileDialog::new().id("other").pick_file(false), None);
            assert_eq!(
                FileDialog::new().id("open-map").pick_file(false),
                Some(PathBuf::from("/x.map"))
            );
            assert_eq!(FileDialog::new().id("open-map").pick_file(false), None);
        } else {
            // The system dialogs answer at once, so nothing is ever waiting.
            assert_eq!(FileDialog::new().id("open-map").pick_file(false), None);
        }
        STATE.with(|state| state.borrow_mut().answer = None);
    }
}
