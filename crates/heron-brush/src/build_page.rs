//! Explicit, project-local build and game launch profiles.
use crate::{dialog, editor::Editor};
#[cfg(not(target_arch = "wasm32"))]
use editor_core::build_profiles::{Profile, Step};
use editor_core::{build_profiles::Store, i18n::tr};
use eframe::egui;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Default)]
pub struct BuildPage {
    pub open: bool,
    map: Option<PathBuf>,
    store_path: Option<PathBuf>,
    store: Store,
    selected: usize,
    error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    status: String,
    dirty: bool,
    #[cfg(not(target_arch = "wasm32"))]
    log: VecDeque<String>,
    #[cfg(not(target_arch = "wasm32"))]
    job: Option<editor_core::build_profiles::runner::Job>,
}
impl BuildPage {
    pub fn show(&mut self, ctx: &egui::Context, editor: &Editor) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(job) = &self.job {
            let mut done = None;
            for _ in 0..64 {
                let Ok(event) = job.events.try_recv() else {
                    break;
                };
                match event {
                    editor_core::build_profiles::runner::Event::Log(text) => {
                        self.log.push_back(text.chars().take(8192).collect());
                        while self.log.len() > 128 {
                            self.log.pop_front();
                        }
                    }
                    editor_core::build_profiles::runner::Event::Done(result) => done = Some(result),
                }
            }
            if let Some(result) = done {
                self.status = match result {
                    Ok(()) => tr("Build finished").into(),
                    Err(error) => error,
                };
                self.job = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
        if !self.open {
            return;
        }
        let map = editor.document.borrow().path.clone();
        if self.map != map {
            self.map = map.clone();
            self.store_path = map.as_deref().map(editor_core::build_profiles::path);
            self.store = Store::default();
            self.selected = 0;
            self.dirty = false;
            self.error = None;
            if let Some(path) = &self.store_path {
                match Store::load(path) {
                    Ok(store) => self.store = store,
                    Err(error) => self.error = Some(error),
                }
            }
        }
        let mut close = false;
        self.open = dialog::show_modal(
            ctx,
            "build-profiles",
            tr("Build and run"),
            [820.0, 640.0],
            |ui, part| {
                if matches!(part, dialog::Part::Actions) {
                    if dialog::button(ui, tr("Close"), true).clicked() {
                        close = true;
                    }
                    return;
                }
                #[cfg(target_arch = "wasm32")]
                {
                    ui.label(tr("External tools require the desktop editor"));
                    return;
                }
                #[cfg(not(target_arch = "wasm32"))]
                self.body(ui, editor);
            },
        );
        self.open &= !close;
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn body(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let running = self.job.is_some();
        if self.store_path.is_none() {
            ui.label(tr("Save the map once before using build profiles"));
            return;
        }
        if let Some(error) = &self.error {
            ui.colored_label(crate::theme::error(), error);
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !running && self.error.is_none(),
                    egui::Button::new(tr("Add profile")),
                )
                .clicked()
            {
                self.store.profiles.push(Profile {
                    name: format!("{} {}", tr("Profile"), self.store.profiles.len() + 1),
                    workdir: "${PROJECT_DIR}".into(),
                    steps: vec![],
                });
                self.selected = self.store.profiles.len() - 1;
                self.dirty = true;
            }
            let exists = !self.store.profiles.is_empty();
            if ui
                .add_enabled(!running && exists, egui::Button::new(tr("Duplicate")))
                .clicked()
            {
                let mut profile = self.store.profiles[self.selected].clone();
                profile.name.push_str(" copy");
                self.store.profiles.push(profile);
                self.selected = self.store.profiles.len() - 1;
                self.dirty = true;
            }
            if ui
                .add_enabled(!running && exists, egui::Button::new(tr("Delete")))
                .clicked()
            {
                self.store.profiles.remove(self.selected);
                self.selected = self.selected.saturating_sub(1);
                self.dirty = true;
            }
            if ui
                .add_enabled(!running, egui::Button::new(tr("Reload")))
                .clicked()
            {
                match Store::load(self.store_path.as_ref().unwrap()) {
                    Ok(store) => {
                        self.store = store;
                        self.selected = 0;
                        self.error = None;
                        self.dirty = false;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            if ui
                .add_enabled(
                    self.dirty && self.error.is_none(),
                    egui::Button::new(tr("Save profiles")),
                )
                .clicked()
            {
                match self.store.save(self.store_path.as_ref().unwrap()) {
                    Ok(()) => {
                        self.dirty = false;
                        self.status = tr("Profiles saved").into();
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        });
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("build-profile")
                .selected_text(
                    self.store
                        .profiles
                        .get(self.selected)
                        .map(|p| p.name.as_str())
                        .unwrap_or(""),
                )
                .show_ui(ui, |ui| {
                    for (index, profile) in self.store.profiles.iter().enumerate() {
                        ui.selectable_value(&mut self.selected, index, &profile.name);
                    }
                });
            if ui
                .add_enabled(
                    !running && self.error.is_none() && !self.store.profiles.is_empty(),
                    egui::Button::new(tr("Run profile")),
                )
                .clicked()
            {
                if self.dirty {
                    if let Err(error) = self.store.save(self.store_path.as_ref().unwrap()) {
                        self.error = Some(error);
                        return;
                    }
                    self.dirty = false;
                }
                match editor_core::build_profiles::runner::Snapshot::capture(
                    &editor.document.borrow(),
                ) {
                    Ok(snapshot) => {
                        self.log.clear();
                        self.status = tr("Running…").into();
                        self.job = Some(editor_core::build_profiles::runner::Job::start(
                            snapshot,
                            self.store.profiles[self.selected].clone(),
                        ));
                    }
                    Err(error) => self.status = error,
                }
            }
            if ui
                .add_enabled(running, egui::Button::new(tr("Stop")))
                .clicked()
            {
                self.job.as_ref().unwrap().cancel();
            }
            ui.label(&self.status);
        });
        ui.separator();
        if let Some(profile) = self.store.profiles.get_mut(self.selected) {
            ui.add_enabled_ui(!running, |ui| {
                self.dirty |= field(ui,tr("Name"),&mut profile.name);
                self.dirty |= field(ui,tr("Working directory"),&mut profile.workdir);
                ui.label(tr("Relative paths use the working directory. Variables are available in paths and arguments."));
                egui::CollapsingHeader::new(tr("Variables")).show(ui,|ui| {ui.label("${PROJECT_DIR}  ${MAP_PATH}  ${MAP_DIR}  ${MAP_BASE_NAME}\n${WORK_DIR_PATH}  ${GAME_DIR_PATH}  ${APP_DIR_PATH}  ${CPU_COUNT}");});
                egui::ScrollArea::vertical().id_salt("build-steps").max_height(290.0).show(ui, |ui| {
                    let count = profile.steps.len(); let mut action = None;
                    for (index,step) in profile.steps.iter_mut().enumerate() {
                        ui.push_id(index, |ui| {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.strong(format!("{} {}",tr("Step"),index+1));
                                    if ui.add_enabled(index>0,egui::Button::new("↑")).clicked(){action=Some((index,-1));}
                                    if ui.add_enabled(index+1<count,egui::Button::new("↓")).clicked(){action=Some((index,1));}
                                    if ui.button(tr("Remove")).clicked(){action=Some((index,0));}
                                });
                                self.dirty |= step_fields(ui,step);
                            });
                        });
                    }
                    if let Some((index,move_by))=action {
                        if move_by==0 {profile.steps.remove(index);} else {profile.steps.swap(index,(index as isize+move_by) as usize);}
                        self.dirty=true;
                    }
                });
                ui.horizontal(|ui| {
                    for (label,step) in [(tr("Export map"),Step::ExportMap{path:"build/${MAP_BASE_NAME}.map".into()}),
                        (tr("Export target"),Step::ExportTarget{engine:"godot".into(),directory:String::new()}),
                        (tr("Copy file"),Step::CopyFile{source:String::new(),destination:String::new()}),
                        (tr("Run tool / game"),Step::Run{program:String::new(),args:vec![],workdir:String::new()})] {
                        if ui.button(label).clicked(){profile.steps.push(step);self.dirty=true;}
                    }
                });
            });
        }
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("build-log")
            .max_height(150.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &self.log {
                    ui.add(
                        egui::Label::new(egui::RichText::new(line).monospace())
                            .wrap()
                            .selectable(true),
                    );
                }
            });
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::TextEdit::singleline(value).desired_width(480.0).margin(crate::widgets::body_entry_margin(ui, 6, 6)).min_size(egui::vec2(0.0, crate::widgets::ROW_HEIGHT)))
            .changed()
    })
    .inner
}
#[cfg(not(target_arch = "wasm32"))]
fn step_fields(ui: &mut egui::Ui, step: &mut Step) -> bool {
    let mut changed = false;
    match step {
        Step::ExportMap { path } => {
            ui.label(tr("Export map"));
            changed |= field(ui, tr("Destination"), path);
        }
        Step::ExportTarget { engine, directory } => {
            ui.label(tr("Export target"));
            changed |= field(ui, tr("Engine"), engine);
            changed |= field(ui, tr("Directory (empty uses engine profile)"), directory);
        }
        Step::CopyFile {
            source,
            destination,
        } => {
            ui.label(tr("Copy file"));
            changed |= field(ui, tr("Source"), source);
            changed |= field(ui, tr("Destination"), destination);
        }
        Step::Run {
            program,
            args,
            workdir,
        } => {
            ui.label(tr("Run tool / game"));
            changed |= field(ui, tr("Executable"), program);
            changed |= field(
                ui,
                tr("Working directory (empty inherits profile)"),
                workdir,
            );
            ui.label(tr("Arguments: one per line, without shell quoting"));
            let mut text = args.join("\n");
            if ui
                .add(
                    egui::TextEdit::multiline(&mut text)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                )
                .changed()
            {
                *args = if text.is_empty() {
                    vec![]
                } else {
                    text.split('\n').map(str::to_owned).collect()
                };
                changed = true;
            }
        }
    }
    changed
}
