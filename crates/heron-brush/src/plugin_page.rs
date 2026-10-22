//! The plugins: the window that lists, switches on and off, installs and runs them, and
//! the docked panel on the Map tab that runs the active ones. Each plugin is a card: its name, what it does, its
//! settings and actions, and what its last run said.
//!
//! The plugins are found once and kept until something changes them (a switch, an
//! install, Reload plugins), not looked for on every frame.
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use crate::{dialog, editor::Editor, file_dialog::FileDialog, form, theme, widgets};
use editor_core::{
    document::Document,
    i18n::{fill, tr},
    plugin::{RunOutput, UiItem},
    plugin_manifest::{Setting, SettingKind, SettingValue, API_VERSION},
    plugins::Plugin,
};
use eframe::egui;

thread_local! {
    /// The plugins last found, shared by the window and the docked panel.
    static FOUND: RefCell<Option<Rc<Vec<Plugin>>>> = const { RefCell::new(None) };
    /// Why the activation file could not be read, when it could not.
    static GENERATION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static ACTIVATION_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The plugins, found on first use and kept until [`reload`].
pub fn plugins() -> Rc<Vec<Plugin>> {
    FOUND.with(|found| {
        Rc::clone(found.borrow_mut().get_or_insert_with(|| {
            let error =
                editor_core::plugin_state::load(&editor_core::plugins::activation_path()).err();
            ACTIVATION_ERROR.with(|slot| *slot.borrow_mut() = error);
            Rc::new(editor_core::plugins::discover())
        }))
    })
}

/// Finds the plugins again, so every view shows what is on disk now.
pub fn reload() {
    FOUND.with(|found| *found.borrow_mut() = None);
    GENERATION.with(|generation| generation.set(generation.get().wrapping_add(1)));
}

/// Why the activation file could not be read, as of the last time the plugins were found.
fn activation_error() -> Option<String> {
    plugins();
    ACTIVATION_ERROR.with(|slot| slot.borrow().clone())
}

/// The plugins matching `query` (by name or summary, any case), active ones first, as the
/// window groups them.
pub fn groups<'a>(plugins: &'a [Plugin], query: &str) -> [(bool, Vec<&'a Plugin>); 2] {
    let query = query.trim().to_lowercase();
    let matching = |active: bool| {
        plugins
            .iter()
            .filter(|plugin| plugin.enabled == active)
            .filter(|plugin| {
                format!("{}\n{}", plugin.name, plugin.summary)
                    .to_lowercase()
                    .contains(&query)
            })
            .collect()
    };
    [(true, matching(true)), (false, matching(false))]
}

/// What the cards remember between frames: each setting's answer and each plugin's last
/// output.
#[derive(Default)]
pub struct Cards {
    generation: u64,
    preferences_loaded: bool,
    preferences: editor_core::plugin_state::Settings,
    preference_error: Option<String>,
    store: Option<std::path::PathBuf>,
    values: BTreeMap<(String, String), SettingValue>,
    outputs: HashMap<String, RunOutput>,
    sessions: HashMap<String, (String, Result<editor_core::plugin::Session, String>)>,
    waiting: HashMap<String, (u64, u64, web_time::Instant)>,
    inspected: HashMap<String, ((u64, u64), Vec<(String, SettingValue)>)>,
}

impl Cards {
    fn sync(&mut self) {
        let generation = GENERATION.with(|generation| generation.get());
        if self.generation != generation {
            self.sessions.clear();
            self.inspected.clear();
            self.waiting.clear();
            self.outputs.clear();
            self.values.clear();
            self.preferences_loaded = false;
            self.preferences.clear();
            self.generation = generation;
        }
    }

    /// A setting's answer, its default until the user changes it.
    fn value(&mut self, plugin: &Plugin, setting: &Setting) -> &mut SettingValue {
        if !self.preferences_loaded {
            let path = self.store.clone().unwrap_or_else(|| {
                editor_core::preferences::config_dir().join("plugin-settings.json")
            });
            match editor_core::plugin_state::load_settings(&path) {
                Ok(preferences) => self.preferences = preferences,
                Err(error) => self.preference_error = Some(error),
            }
            self.preferences_loaded = true;
        }
        let saved = self
            .preferences
            .get(&plugin.id)
            .and_then(|answers| answers.get(&setting.key))
            .cloned();
        let saved = match saved {
            Some(SettingValue::Number(n))
                if setting.kind == SettingKind::Number && n.is_finite() =>
            {
                Some(SettingValue::Number(
                    n.max(setting.min.unwrap_or(-1_000_000.0))
                        .min(setting.max.unwrap_or(1_000_000.0)),
                ))
            }
            Some(SettingValue::Check(on)) if setting.kind == SettingKind::Check => {
                Some(SettingValue::Check(on))
            }
            Some(SettingValue::Text(text))
                if text.len() <= 4096
                    && (setting.kind == SettingKind::Text
                        || (setting.kind == SettingKind::Choice
                            && setting.choices.contains(&text))) =>
            {
                Some(SettingValue::Text(text))
            }
            _ => None,
        };
        self.values
            .entry((plugin.id.clone(), setting.key.clone()))
            .or_insert_with(|| {
                saved.unwrap_or_else(|| match setting.kind {
                    SettingKind::Number => SettingValue::Number(setting.default_number()),
                    SettingKind::Check => SettingValue::Check(setting.default_check()),
                    SettingKind::Choice => SettingValue::Text(
                        setting
                            .choices
                            .iter()
                            .find(|c| **c == setting.default)
                            .or(setting.choices.first())
                            .cloned()
                            .unwrap_or_default(),
                    ),
                    SettingKind::Text => SettingValue::Text(setting.default.clone()),
                })
            })
    }

    /// Runs one of `plugin`'s actions on the selection (or everything, with nothing
    /// selected) as one undo step, and keeps what it said for its card and the console.
    fn run(&mut self, plugin: &Plugin, action: &str, document: &Rc<RefCell<Document>>) {
        if !editor_core::plugins::enabled(&plugin.id) || plugin.wrong_api.is_some() {
            return;
        }
        let answers: Vec<(String, SettingValue)> = plugin
            .settings
            .iter()
            .map(|setting| (setting.key.clone(), self.value(plugin, setting).clone()))
            .collect();
        let session = self.sessions.entry(plugin.id.clone()).or_insert_with(|| {
            (
                plugin.source.clone(),
                editor_core::plugin::Session::new(&plugin.source),
            )
        });
        if session.0 != plugin.source {
            *session = (
                plugin.source.clone(),
                editor_core::plugin::Session::new(&plugin.source),
            );
        }
        let result = {
            let mut document = document.borrow_mut();
            let had_selection = document.has_selection();
            let result = document.run_as_one_edit(|document| {
                if !had_selection && plugin.panel_action.is_none() {
                    document.select_all();
                }
                match &mut session.1 {
                    Ok(session) => session.run(document, &answers, action),
                    Err(error) => Err(error.clone()),
                }
            });
            if !had_selection && plugin.panel_action.is_none() {
                document.clear_selection();
            }
            result
        };
        let output = match result {
            Ok(output) => {
                for line in &output.lines {
                    editor_core::say!("{}: {line}", plugin.name);
                }
                output
            }
            Err(error) => {
                editor_core::complain!("{}: {error}", plugin.name);
                RunOutput {
                    lines: vec![error],
                    panel: vec![],
                }
            }
        };
        self.outputs.insert(plugin.id.clone(), output);
    }
}

/// What a card asks of its caller.
enum Request {
    /// Switch the plugin on or off.
    Enable(bool),
}

/// One plugin's card. In the window (`management`) it carries the on/off switch and keeps
/// the settings and actions behind a fold, shown only while the plugin can run; docked,
/// the settings and actions are out in the open.
fn card(
    ui: &mut egui::Ui,
    plugin: &Plugin,
    cards: &mut Cards,
    document: &Rc<RefCell<Document>>,
    management: bool,
) -> Option<Request> {
    cards.sync();
    let mut request = None;
    let runnable = plugin.enabled && plugin.wrong_api.is_none();
    ui.push_id(&plugin.id, |ui| {
        widgets::card(ui, "", None, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            if management {
                ui.horizontal(|ui| {
                    let title = plugin.version.as_ref().map_or_else(
                        || plugin.name.clone(),
                        |version| format!("{} {version}", plugin.name),
                    );
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(title)
                                .font(theme::body_bold())
                                .color(theme::fg()),
                        )
                        .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut on = plugin.enabled;
                        if widgets::switch(ui, &mut on)
                            .on_hover_text(tr("Enable plugin"))
                            .changed()
                        {
                            request = Some(Request::Enable(on));
                        }
                    });
                });
            }
            if !plugin.summary.is_empty() {
                form::note(ui, &plugin.summary);
            }
            if let Some(asked) = plugin.wrong_api {
                form::text(
                    ui,
                    &fill(
                        "Written for plugin API {}; this editor offers {}.",
                        &[&asked.to_string(), &API_VERSION.to_string()],
                    ),
                );
            }
            if management {
                if runnable {
                    form::fold(ui, "details", tr("Actions and settings"), false, |ui| {
                        body(ui, plugin, cards, document)
                    });
                }
            } else {
                ui.add_space(2.0);
                ui.add_enabled_ui(runnable, |ui| body(ui, plugin, cards, document));
            }
            if let Some(output) = cards.outputs.get(&plugin.id) {
                ui.add_space(4.0);
                show_output(ui, output);
            }
        });
    });
    request
}

/// A plugin's settings and its action buttons.
fn body(ui: &mut egui::Ui, plugin: &Plugin, cards: &mut Cards, document: &Rc<RefCell<Document>>) {
    let before: Vec<_> = plugin
        .settings
        .iter()
        .map(|setting| (setting.key.clone(), cards.value(plugin, setting).clone()))
        .collect();
    for setting in &plugin.settings {
        ui.push_id(&setting.key, |ui| {
            widgets::row(ui, &setting.label, None, |ui| {
                let width = ui.available_width().min(240.0);
                match cards.value(plugin, setting) {
                    SettingValue::Number(number) => {
                        // A number with no declared range still needs one to spin within.
                        let min = setting.min.unwrap_or(-1_000_000.0);
                        let max = setting.max.unwrap_or(1_000_000.0);
                        // Whole numbers unless the default or a bound says otherwise, since
                        // most of these are counts.
                        let fractional = [Some(setting.default_number()), setting.min, setting.max]
                            .into_iter()
                            .flatten()
                            .any(|v| v.fract() != 0.0);
                        widgets::spin_width(
                            ui,
                            number,
                            min..=max,
                            1.0,
                            if fractional { 3 } else { 0 },
                            width.min(160.0),
                        );
                    }
                    SettingValue::Check(on) => {
                        widgets::switch(ui, on);
                    }
                    SettingValue::Text(text) if setting.kind == SettingKind::Choice => {
                        let names: Vec<&str> = setting.choices.iter().map(String::as_str).collect();
                        let mut index = setting
                            .choices
                            .iter()
                            .position(|choice| choice == text)
                            .unwrap_or(0);
                        if widgets::dropdown(ui, "choice", &mut index, &names, width) {
                            *text = setting.choices[index].clone();
                        }
                    }
                    SettingValue::Text(text) => {
                        if let Some(new) = widgets::field(ui, ui.id().with("text"), text, "", width)
                        {
                            *text = new;
                        }
                    }
                }
            });
        });
    }
    let after: Vec<_> = plugin
        .settings
        .iter()
        .map(|setting| (setting.key.clone(), cards.value(plugin, setting).clone()))
        .collect();
    if before != after {
        let path = cards
            .store
            .clone()
            .unwrap_or_else(|| editor_core::preferences::config_dir().join("plugin-settings.json"));
        cards.preference_error =
            editor_core::plugin_state::save_settings(&path, &plugin.id, &after).err();
    }
    if let Some(error) = &cards.preference_error {
        form::note(ui, error);
    }
    if let Some(action) = &plugin.panel_action {
        let answers: Vec<_> = plugin
            .settings
            .iter()
            .map(|setting| (setting.key.clone(), cards.value(plugin, setting).clone()))
            .collect();
        let revision = {
            let doc = document.borrow();
            (doc.generation(), doc.revision())
        };
        let stamp = (revision, answers.clone());
        if cards.inspected.get(&plugin.id) != Some(&stamp) {
            let waiting = cards.waiting.entry(plugin.id.clone()).or_insert((
                revision.0,
                revision.1,
                web_time::Instant::now(),
            ));
            if (waiting.0, waiting.1) != revision {
                *waiting = (revision.0, revision.1, web_time::Instant::now());
            }
            let delay = web_time::Duration::from_millis(150);
            if cards.inspected.contains_key(&plugin.id) && waiting.2.elapsed() < delay {
                ui.ctx().request_repaint_after(delay - waiting.2.elapsed());
            } else {
                let session = cards.sessions.entry(plugin.id.clone()).or_insert_with(|| {
                    (
                        plugin.source.clone(),
                        editor_core::plugin::Session::new(&plugin.source),
                    )
                });
                if session.0 != plugin.source {
                    *session = (
                        plugin.source.clone(),
                        editor_core::plugin::Session::new(&plugin.source),
                    );
                }
                let result = match &mut session.1 {
                    Ok(session) => session.inspect(&document.borrow(), &answers, action),
                    Err(error) => Err(error.clone()),
                };
                cards.outputs.insert(
                    plugin.id.clone(),
                    result.unwrap_or_else(|error| RunOutput {
                        lines: vec![error],
                        panel: vec![],
                    }),
                );
                cards.inspected.insert(plugin.id.clone(), stamp);
            }
        }
    }
    let actions = if plugin.actions.is_empty() {
        vec![editor_core::plugin_manifest::Action {
            id: String::new(),
            label: tr("Run").into(),
            description: String::new(),
        }]
    } else {
        plugin.actions.clone()
    };
    ui.add_space(2.0);
    let mut chosen = None;
    ui.horizontal_wrapped(|ui| {
        for action in &actions {
            let response = form::button(ui, &action.label, ui.is_enabled());
            let response = if action.description.is_empty() {
                response
            } else {
                response.on_hover_text(&action.description)
            };
            if response.clicked() {
                chosen = Some(action.id.clone());
            }
        }
    });
    if let Some(action) = chosen {
        cards.run(plugin, &action, document);
    }
}

/// What a run said: the plugin's panel items, then its lines.
fn show_output(ui: &mut egui::Ui, output: &RunOutput) {
    for item in &output.panel {
        match item {
            UiItem::Text(text) => {
                ui.add(egui::Label::new(text.as_str()).wrap().selectable(true));
            }
            UiItem::Value { label, value } => form::value_row(ui, label, value),
            UiItem::Progress { label, fraction } => {
                form::progress(ui, *fraction as f32, label);
            }
        }
    }
    for line in &output.lines {
        ui.add(egui::Label::new(line.as_str()).wrap().selectable(true));
    }
}

/// Switches a plugin on or off and finds the plugins again, so every view follows.
/// Returns why it could not, when it could not.
fn set_enabled(plugin: &Plugin, on: bool) -> Result<(), String> {
    editor_core::plugin_state::set_enabled(
        &editor_core::plugins::activation_path(),
        &plugin.id,
        on,
    )?;
    reload();
    Ok(())
}

/// The Map tab's plugin panel: Reload plugins, then each active plugin under a fold of
/// its own with its card. Returns true when the Plugins window
/// was asked for.
pub fn docked(ui: &mut egui::Ui, cards: &mut Cards, document: &Rc<RefCell<Document>>) -> bool {
    let mut manage = false;
    ui.horizontal(|ui| {
        if form::button(ui, tr("Reload plugins"), true).clicked() {
            reload();
        }
        manage = form::button(ui, tr("Plugins"), true).clicked();
    });
    ui.add_space(4.0);
    let found = plugins();
    for plugin in found
        .iter()
        .filter(|plugin| plugin.enabled && plugin.wrong_api.is_none())
    {
        ui.push_id(("docked-plugin", &plugin.id), |ui| {
            form::fold(ui, "plugin", &plugin.name, false, |ui| {
                card(ui, plugin, cards, document, false);
            });
        });
    }
    manage
}

#[derive(Default)]
pub struct PluginPage {
    pub open: bool,
    query: String,
    /// What the last install or switch said.
    status: String,
}

impl PluginPage {
    pub fn open(&mut self) {
        reload();
        self.open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, editor: &Editor, cards: &mut Cards) {
        if !self.open {
            return;
        }
        let mut close = false;
        let mut install = None;
        let embedded = ctx.embed_viewports();
        self.open = dialog::show(ctx, "plugins", tr("Plugins"), [620.0, 660.0], |ui, part| {
            if part == dialog::Part::Actions {
                close = dialog::button(ui, tr("Close"), true).clicked();
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let file = dialog::button(ui, tr("Install a plugin"), true).clicked();
                    let folder = dialog::button(ui, tr("Install a plugin folder"), true).clicked();
                    let picked = FileDialog::new()
                        .id("install-plugin")
                        .set_title(tr("Install a plugin"))
                        .add_filter("Rhai plugin", &["rhai"])
                        .pick_file(file);
                    let picked = picked.or_else(|| {
                        FileDialog::new()
                            .id("install-plugin-folder")
                            .set_title(tr("Install a plugin folder"))
                            .pick_folder(folder)
                    });
                    if picked.is_some() {
                        install = picked;
                    }
                    if dialog::button(ui, tr("Reload plugins"), true).clicked() {
                        reload();
                    }
                });
                return;
            }
            self.body(ui, editor, embedded, cards);
        }) && !close;
        if let Some(path) = install {
            self.install(&path);
        }
    }

    /// Installs what was picked and says how it went, in the window and in the console.
    fn install(&mut self, chosen: &std::path::Path) {
        match editor_core::plugins::install(chosen) {
            Ok(name) => {
                reload();
                let said = fill("{} is installed", &[&name]);
                editor_core::say!("{said}");
                self.status = said;
            }
            Err(error) => {
                editor_core::complain!("{error}");
                self.status = error;
            }
        }
    }

    fn body(&mut self, ui: &mut egui::Ui, editor: &Editor, embedded: bool, cards: &mut Cards) {
        let full = form::body_rect(ui, embedded);
        let mut ui = form::region(ui, full);
        let edit = egui::TextEdit::singleline(&mut self.query)
            .id(egui::Id::new("plugins-search"))
            .hint_text(tr("Search by name or description"))
            .desired_width(f32::INFINITY)
            .margin(crate::widgets::body_entry_margin(&ui, 26, 6))
            .min_size(egui::vec2(0.0, widgets::ROW_HEIGHT));
        let response = ui.add(edit);
        magnifier(ui.painter(), response.rect, theme::dim());
        ui.add_space(8.0);
        let status = [
            activation_error(),
            (!self.status.is_empty()).then(|| self.status.clone()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
        let reserve = if status.is_empty() {
            0.0
        } else {
            8.0 + 18.0 * status.lines().count() as f32
        };
        let found = plugins();
        let mut toggles = Vec::new();
        egui::ScrollArea::vertical()
            .id_salt("plugins-list")
            .auto_shrink([false, false])
            .max_height((ui.available_height() - reserve).max(80.0))
            .show(&mut ui, |ui| {
                ui.set_max_width(ui.available_width() - 10.0);
                for (active, plugins) in groups(&found, &self.query) {
                    let label = if active {
                        tr("Active plugins")
                    } else {
                        tr("Inactive plugins")
                    };
                    theme::heading(ui, &format!("{label} ({})", plugins.len()));
                    ui.add_space(4.0);
                    if plugins.is_empty() {
                        form::note(ui, tr("No matching plugins"));
                    }
                    for plugin in plugins {
                        if let Some(Request::Enable(on)) =
                            card(ui, plugin, cards, &editor.document, true)
                        {
                            toggles.push((plugin.clone(), on));
                        }
                        ui.add_space(8.0);
                    }
                    ui.add_space(8.0);
                }
            });
        for (plugin, on) in toggles {
            if let Err(error) = set_enabled(&plugin, on) {
                cards.outputs.insert(
                    plugin.id.clone(),
                    RunOutput {
                        lines: vec![error],
                        panel: vec![],
                    },
                );
            }
        }
        if !status.is_empty() {
            ui.add_space(8.0);
            form::note(&mut ui, &status);
        }
    }
}

/// A magnifying glass at the start of the search field.
fn magnifier(painter: &egui::Painter, field: egui::Rect, color: egui::Color32) {
    let centre = egui::pos2(field.min.x + 13.0, field.center().y - 1.0);
    painter.circle_stroke(centre, 4.5, egui::Stroke::new(1.5, color));
    painter.line_segment(
        [centre + egui::vec2(3.3, 3.3), centre + egui::vec2(6.5, 6.5)],
        egui::Stroke::new(1.5, color),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin(name: &str, summary: &str, enabled: bool) -> Plugin {
        Plugin {
            id: name.to_lowercase(),
            enabled,
            path: None,
            name: name.into(),
            summary: summary.into(),
            version: None,
            wrong_api: None,
            settings: vec![],
            actions: vec![],
            panel_action: None,
            source: String::new(),
        }
    }

    #[test]
    fn visible_panel_reuses_loaded_state_without_editing_the_map() {
        let ctx = egui::Context::default();
        let document = Rc::new(RefCell::new(Document::new(
            editor_core::world::WorldBounds::default(),
        )));
        let before = document.borrow().to_map_text();
        let mut owner = plugin("Live", "", true);
        owner.panel_action = Some("inspect".into());
        owner.source = r#"set_state("runs", (state("runs") ?? 0) + 1); ui_value("Runs", state("runs")); add_cuboid([0,0,0],[16,16,16],"stone");"#.into();
        let mut cards = Cards::default();
        for _ in 0..3 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                body(ui, &owner, &mut cards, &document)
            });
            output.textures_delta.clear();
        }
        assert_eq!(document.borrow().to_map_text(), before);
        assert!(
            matches!(&cards.outputs[&owner.id].panel[0], UiItem::Value { value, .. } if value == "1")
        );
        owner.source = "ui_value(\"Runs\", state(\"runs\") ?? 0);".into();
        cards.inspected.clear();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            body(ui, &owner, &mut cards, &document)
        });
        output.textures_delta.clear();
        assert!(
            matches!(&cards.outputs[&owner.id].panel[0], UiItem::Value { value, .. } if value == "0")
        );
    }

    #[test]
    fn saved_answers_survive_cards_restart_and_validate_current_schema() {
        let path =
            std::env::temp_dir().join(format!("heron-card-settings-{}.json", std::process::id()));
        let owner = plugin("Persisted", "", true);
        editor_core::plugin_state::save_settings(
            &path,
            &owner.id,
            &[
                ("step".into(), SettingValue::Number(200.0)),
                ("kind".into(), SettingValue::Text("removed".into())),
            ],
        )
        .unwrap();
        let number = Setting {
            key: "step".into(),
            label: "Step".into(),
            kind: SettingKind::Number,
            default: "8".into(),
            min: Some(0.0),
            max: Some(64.0),
            choices: vec![],
        };
        let choice = Setting {
            key: "kind".into(),
            label: "Kind".into(),
            kind: SettingKind::Choice,
            default: "valid".into(),
            min: None,
            max: None,
            choices: vec!["valid".into()],
        };
        for _ in 0..2 {
            let mut cards = Cards {
                store: Some(path.clone()),
                ..Cards::default()
            };
            assert!(matches!(cards.value(&owner, &number), SettingValue::Number(n) if *n == 64.0));
            assert!(matches!(cards.value(&owner, &choice), SettingValue::Text(s) if s == "valid"));
        }
        std::fs::remove_file(path).unwrap();
    }

    /// Active plugins come first, a search matches name or summary in any case, and each
    /// group counts only what matches.
    #[test]
    fn plugins_group_by_activation_and_filter_by_search() {
        let all = [
            plugin("Bevel", "Rounds edges", true),
            plugin("Stairs", "Builds a staircase", false),
            plugin("Inspector", "Says what is selected", true),
        ];
        let [(active, on), (inactive, off)] = groups(&all, "");
        assert!(active && !inactive);
        assert_eq!(
            on.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["Bevel", "Inspector"]
        );
        assert_eq!(off.len(), 1);
        let [(_, on), (_, off)] = groups(&all, "  STAIRCASE ");
        assert!(on.is_empty());
        assert_eq!(off[0].name, "Stairs");
        let [(_, on), (_, off)] = groups(&all, "no such plugin");
        assert!(on.is_empty() && off.is_empty());
    }

    /// A choice setting starts on its default, or its first choice when the default is not
    /// one of them.
    #[test]
    fn a_choice_starts_on_its_default() {
        let mut cards = Cards::default();
        let owner = plugin("Choices", "", true);
        let mut setting = Setting {
            key: "size".into(),
            label: "Size".into(),
            kind: SettingKind::Choice,
            default: "large".into(),
            min: None,
            max: None,
            choices: vec!["small".into(), "large".into()],
        };
        assert!(
            matches!(cards.value(&owner, &setting), SettingValue::Text(text) if text == "large")
        );
        setting.key = "other".into();
        setting.default = "huge".into();
        assert!(
            matches!(cards.value(&owner, &setting), SettingValue::Text(text) if text == "small")
        );
    }
}
