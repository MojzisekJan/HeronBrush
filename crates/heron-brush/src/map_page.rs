//! The inspector's Map tab: what is selected, with the ids an agent works with, the
//! layer and group panel
//! (`layer_panel`), the plugins, the issues (`issue_panel`) and what the views draw
//! (`view_filters_panel`).

use std::cell::RefCell;
use std::rc::Rc;

use editor_core::document::Document;
use editor_core::i18n::{counted, fill, tr, translation_key};
use editor_core::validation::{self, Issue, Severity};
use editor_core::view_filters::{EntityLinkMode, ViewFilters};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense};

use crate::theme;
use crate::widgets::{self, icons};

/// The Map tab's own state.
#[derive(Default)]
pub struct MapPage {
    /// The layer or group name entry.
    name: String,
    /// The issues last found, and the document state they were found in.
    issues: Vec<Issue>,
    hidden_issues: std::collections::HashSet<Issue>,
    issue_generation: u64,
    checked: Option<(u64, u64)>,
    /// The document state last seen changing and when, so the check waits until the
    /// edits stop: a drag changes the map every frame, and checking a big map every frame
    /// of one was a frame's worth of work on its own.
    changed: Option<((u64, u64), web_time::Instant)>,
    /// The issue picked in the list.
    picked: Option<usize>,
    /// Whether the issue list was the last thing clicked, so Ctrl+C copies from it.
    issues_focused: bool,
    /// Each issue row's height when last laid out, and the width it was laid out at, so
    /// rows scrolled out of sight take their space without being laid out: a big map
    /// can have thousands of issues, and laying all of them out every frame was most of
    /// the frame.
    issue_heights: (f32, Vec<f32>),
    /// What the last leak check said, and whether it leaked.
    leak: Option<(String, bool)>,
    /// The docked plugin cards' answers and output.
    pub(crate) plugin_cards: crate::plugin_page::Cards,
    pub plugins_requested: bool,
    /// The selection report, and the selection and map state it was made from: it walks
    /// the whole map, so it is made again only when either changes.
    selection: Option<(u64, serde_json::Value)>,
}

/// How many selected objects the Selected section lists before "and N more".
const SELECTION_ROWS: usize = 8;

/// How long the map has to stay unchanged before the issues are checked again.
const ISSUE_SETTLE: web_time::Duration = web_time::Duration::from_millis(250);

/// Which of a layer's three switches an icon works.
#[derive(Clone, Copy)]
enum Flag {
    Hidden,
    Locked,
    Omitted,
}

impl MapPage {
    /// Draws the tab.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        filters: &Rc<RefCell<ViewFilters>>,
    ) {
        self.selected(ui, document);
        self.layers(ui, document);
        self.groups(ui, document);
        ui.add_space(4.0);
        self.plugins(ui, document);
        ui.separator();
        self.issues(ui, document);
        view(ui, document, filters);
    }

    /// What is selected, by the ids the MCP tools take, with Copy for agent: the text to
    /// paste into a conversation with an agent driving the editor ("change these").
    /// Hidden while nothing is selected.
    fn selected(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let document = document.borrow();
        if document.selection_count() == 0 {
            return;
        }
        let key = {
            use std::hash::{Hash, Hasher};
            let mut ids: Vec<u64> = document.selection_ids().iter().map(|id| id.0).collect();
            ids.sort_unstable();
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            ids.hash(&mut hasher);
            document.modification_count().hash(&mut hasher);
            hasher.finish()
        };
        if self.selection.as_ref().map(|(made, _)| *made) != Some(key) {
            self.selection = Some((key, editor_core::selection_report::describe(&document)));
        }
        let Some((_, report)) = &self.selection else {
            return;
        };
        let list = |key: &str| report[key].as_array().cloned().unwrap_or_default();
        let mut rows: Vec<(String, String)> = Vec::new();
        let size = |v: &serde_json::Value| format!("{} × {} × {}", v[0], v[1], v[2]);
        for brush in list("brushes") {
            rows.push((
                format!("brush {}", brush["id"].as_str().unwrap_or("")),
                format!(
                    "{} · {}",
                    brush["entity"]["classname"].as_str().unwrap_or(""),
                    size(&brush["size"])
                ),
            ));
        }
        for entity in list("entities") {
            rows.push((
                format!("entity {}", entity["id"].as_str().unwrap_or("")),
                entity["classname"].as_str().unwrap_or("").to_string(),
            ));
        }
        for group in list("groups") {
            rows.push((
                format!("group {}", group["id"].as_str().unwrap_or("")),
                group["name"].as_str().unwrap_or("").to_string(),
            ));
        }
        for patch in list("patches") {
            rows.push((
                format!("patch {}", patch["id"].as_str().unwrap_or("")),
                String::new(),
            ));
        }
        let heading = fill("{} selected", &[&counted(rows.len(), "object")]);
        widgets::section(ui, "map-selection", &heading, true, |ui| {
            // The id on a line of its own, the inspector being narrow, and what it is under it.
            for (what, detail) in rows.iter().take(SELECTION_ROWS) {
                ui.label(egui::RichText::new(what).monospace().color(theme::fg()));
                if !detail.is_empty() {
                    ui.label(egui::RichText::new(detail).color(theme::dim()));
                }
            }
            if rows.len() > SELECTION_ROWS {
                ui.label(
                    egui::RichText::new(fill(
                        "… and {} more",
                        &[&(rows.len() - SELECTION_ROWS).to_string()],
                    ))
                    .color(theme::dim()),
                );
            }
            ui.add_space(4.0);
            match widgets::segmented(
                ui,
                "selection-copy",
                &[(tr("Copy for agent"), "", true), (tr("Copy ids"), "", true)],
            ) {
                Some(0) => ui
                    .ctx()
                    .copy_text(editor_core::selection_report::text(&document)),
                Some(1) => {
                    let ids: Vec<String> = rows
                        .iter()
                        .filter_map(|(what, _)| what.split(' ').nth(1).map(str::to_string))
                        .collect();
                    ui.ctx().copy_text(ids.join(" "));
                }
                _ => {}
            }
        });
    }

    fn layers(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let (layers, current) = {
            let document = document.borrow();
            (document.layers(), document.current_layer())
        };
        let note = layers.len().to_string();
        widgets::section_noted(ui, "map-layers", tr("Layers"), Some(&note), true, |ui| {
            widgets::list(ui, 2.0 * widgets::LIST_ROW + 4.0, |ui| {
                let rows: Vec<egui::Response> = layers
                    .iter()
                    .enumerate()
                    .map(|(index, layer)| {
                        self.layer_row(ui, document, index, layer, layer.index == current)
                    })
                    .collect();
                // Up and Down make the layer before or after the current one.
                if let Some(next) = widgets::list_keys(ui, &rows) {
                    document.borrow_mut().set_current_layer(layers[next].index);
                }
            });
            ui.add_space(4.0);

            let can_remove = current != 0 && layers.len() > 1;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let tools = 4.0 * widgets::ROW_HEIGHT + 3.0 * 2.0 + 9.0 + 2.0;
                // Enter in the name renames the layer that is selected, which is the one
                // place a name is wanted often enough to deserve a key rather than a button.
                let entry = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.name)
                            .hint_text(tr("Layer or group name"))
                            .desired_width(ui.available_width() - tools - 6.0)
                            .margin(crate::widgets::body_entry_margin(ui, 6, 6))
                            .min_size(egui::vec2(0.0, widgets::ROW_HEIGHT)),
                    )
                    .on_hover_text(tr(
                        "Add makes a new layer with this name; Enter renames the current one",
                    ));
                if entry.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.name.trim().is_empty()
                {
                    document
                        .borrow_mut()
                        .rename_layer(current, self.name.trim());
                    self.name.clear();
                }
                ui.add_space(4.0);
                let size = widgets::ROW_HEIGHT;
                if widgets::icon_button(ui, size, tr("Add"), true, false, icons::plus).clicked() {
                    let chosen = if self.name.trim().is_empty() {
                        format!("Layer {}", layers.len())
                    } else {
                        self.name.trim().to_string()
                    };
                    document.borrow_mut().add_layer(chosen);
                    self.name.clear();
                }
                if widgets::icon_button(ui, size, tr("Remove"), can_remove, false, icons::minus)
                    .clicked()
                    && document.borrow_mut().remove_layer(current)
                {
                    editor_core::say!("removed the layer; what was on it is on the default one");
                }
                widgets::toolbar_gap(ui);
                if widgets::icon_button(ui, size, tr("Move here"), true, false, icons::move_here)
                    .clicked()
                {
                    let moved = document.borrow_mut().move_selection_to_layer(current);
                    editor_core::say!("moved {moved} objects");
                }
                if widgets::icon_button(ui, size, tr("Select"), true, false, icons::select)
                    .clicked()
                {
                    let selected = document.borrow_mut().select_layer(current);
                    editor_core::say!("selected {selected} objects");
                }
            });
        });
    }

    /// One layer of the list: its name and object count, then its three switches. A click
    /// on the row makes the layer the current one.
    fn layer_row(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        index: usize,
        layer: &editor_core::document::LayerSummary,
        is_current: bool,
    ) -> egui::Response {
        let (row, response) = widgets::list_row(ui, widgets::LIST_ROW, is_current, Sense::click());
        widgets::row_focus(ui, &response);
        let flags = 3.0 * 24.0 + 4.0;
        // The current layer is where a new brush lands, which is worth seeing without
        // having to check the selection.
        let font = if is_current {
            theme::body_bold()
        } else {
            theme::body()
        };
        // A long name ends in an ellipsis short of the switches.
        let name = ui.painter().layout_job(ellipsized(
            layer.name.clone(),
            font,
            theme::fg(),
            row.width() - flags - 8.0 - 40.0,
        ));
        let at = Pos2::new(
            row.left() + 8.0,
            crate::theme::galley_top(ui.ctx(), &name, row.center().y),
        );
        let name_end = at.x + name.size().x;
        ui.painter().galley(at, name, theme::fg());
        crate::theme::text(
            ui.painter(),
            Pos2::new(name_end + 6.0, row.center().y),
            egui::Align2::LEFT_CENTER,
            layer.objects.to_string(),
            theme::body(),
            theme::dim(),
        );
        /// Draws a switch's icon in a rect, in a colour.
        type Icon = fn(&egui::Painter, Rect, Color32);
        let switches: [(Flag, bool, &str, Icon); 3] = [
            (
                Flag::Hidden,
                layer.hidden,
                translation_key("Hidden: not drawn and not clickable"),
                icons::eye,
            ),
            (
                Flag::Locked,
                layer.locked,
                translation_key("Locked: drawn but not editable"),
                icons::lock,
            ),
            (
                Flag::Omitted,
                layer.omit_from_export,
                translation_key("Left out of exports"),
                icons::export,
            ),
        ];
        let mut switched = false;
        for (i, (flag, on, tip, draw)) in switches.into_iter().enumerate() {
            let slot = Rect::from_min_size(
                Pos2::new(row.right() - flags + i as f32 * 24.0, row.center().y - 12.0),
                egui::vec2(24.0, 24.0),
            );
            let toggle = ui
                .scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(slot)
                        .id_salt(("layer-flag", index, i)),
                    |ui| widgets::icon_button(ui, 24.0, tr(tip), true, on, draw),
                )
                .inner;
            if toggle.clicked() {
                let mut document = document.borrow_mut();
                match flag {
                    Flag::Hidden => document.set_layer_hidden(layer.index, !on),
                    Flag::Locked => document.set_layer_locked(layer.index, !on),
                    Flag::Omitted => document.set_layer_omitted(layer.index, !on),
                };
                switched = true;
            }
        }
        if response.clicked() && !switched {
            document.borrow_mut().set_current_layer(layer.index);
        }
        response
    }

    fn groups(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let (groups, open, has_selection) = {
            let document = document.borrow();
            (
                document.groups(),
                document.open_group(),
                document.has_selection(),
            )
        };
        let status = match open {
            Some(id) => {
                let name = groups
                    .iter()
                    .find(|(group_id, _, _)| *group_id == id)
                    .map(|(_, name, _)| name.clone())
                    .unwrap_or_else(|| tr("a group").to_string());
                fill(
                    "{} · \"{}\" is open",
                    &[&counted(groups.len(), "group"), &name],
                )
            }
            None => counted(groups.len(), "group"),
        };
        widgets::section(ui, "map-groups", tr("Groups"), true, |ui| {
            widgets::status(ui, &status);
            type Run = fn(&mut Document, &str);
            let buttons: [(&str, &str, bool, Run); 11] = [
                (translation_key("Group"), "", has_selection, |d, _| {
                    let count = d.groups().len();
                    if d.group_selection(format!("Group {}", count + 1)).is_some() {
                        editor_core::say!("grouped the selection");
                    }
                }),
                (
                    translation_key("Ungroup"),
                    "",
                    has_selection && !groups.is_empty(),
                    |d, _| {
                        let dissolved = d.ungroup_selection();
                        editor_core::say!("took apart {dissolved} groups");
                    },
                ),
                (
                    translation_key("Merge"),
                    translation_key("Fold the selected groups into one"),
                    has_selection && groups.len() >= 2,
                    |d, _| {
                        let folded = d.merge_selected_groups();
                        if folded > 0 {
                            editor_core::say!("merged {folded} groups into one");
                        } else {
                            editor_core::complain!("select two or more unlinked groups to merge");
                        }
                    },
                ),
                (
                    translation_key("Rename"),
                    translation_key("Rename the selected group to the name above"),
                    true,
                    |d, name| {
                        let renamed = d.rename_selected_groups(name);
                        editor_core::say!("renamed {renamed} groups");
                    },
                ),
                (
                    translation_key("Open"),
                    "",
                    has_selection && !groups.is_empty(),
                    |d, _| {
                        d.open_selected_group();
                    },
                ),
                (translation_key("Close"), "", open.is_some(), |d, _| {
                    d.close_group();
                }),
                (
                    translation_key("Add"),
                    translation_key("Move the selection into the open group"),
                    open.is_some() && has_selection,
                    |d, _| {
                        let moved = d.add_selection_to_open_group();
                        editor_core::say!("moved {moved} objects into the group");
                    },
                ),
                (
                    translation_key("Take out"),
                    translation_key("Move the selection out of the open group"),
                    open.is_some() && has_selection,
                    |d, _| {
                        let moved = d.remove_selection_from_open_group();
                        editor_core::say!("moved {moved} objects out of the group");
                    },
                ),
                (
                    translation_key("Linked"),
                    translation_key("Select every member of the linked set"),
                    true,
                    |d, _| {
                        d.select_linked_groups();
                    },
                ),
                (
                    translation_key("Unlink"),
                    translation_key("Make the selected linked groups independent"),
                    true,
                    |d, _| {
                        let freed = d.separate_selected_linked_groups();
                        editor_core::say!("separated {freed} groups");
                    },
                ),
                (
                    translation_key("Extract"),
                    translation_key("Pull the selection into a linked set of its own"),
                    open.is_some() && has_selection,
                    |d, _| match d.extract_linked_groups() {
                        Some(groups) => {
                            editor_core::say!("extracted a linked set of {} groups", groups.len())
                        }
                        None => editor_core::complain!(
                            "open a linked group and select some but not all of it to extract"
                        ),
                    },
                ),
            ];
            // Making and taking apart, working inside an open group, and linked sets: a
            // strip each.
            for (strip, range) in [0..4, 4..8, 8..11].into_iter().enumerate() {
                let row = &buttons[range.clone()];
                let items: Vec<(&str, &str, bool)> = row
                    .iter()
                    .map(|(label, tip, enabled, _)| {
                        (
                            tr(label),
                            if tip.is_empty() { "" } else { tr(tip) },
                            *enabled,
                        )
                    })
                    .collect();
                if let Some(i) = widgets::segmented(ui, ("group-actions", strip), &items) {
                    (row[i].3)(&mut document.borrow_mut(), &self.name);
                }
            }
        });
    }

    fn plugins(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let cards = &mut self.plugin_cards;
        let requested = widgets::section(ui, "map-plugins", tr("Plugins"), false, |ui| {
            crate::plugin_page::docked(ui, cards, document)
        });
        if requested == Some(true) {
            self.plugins_requested = true;
        }
    }

    fn issues(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        // Checked again only when the map's content changed.
        let key = {
            let document = document.borrow();
            (document.generation(), document.content_revision())
        };
        // Checked at once the first time, then once the map has been left alone for a
        // moment; until then the last findings stay up.
        let settled = match self.changed {
            _ if self.checked.is_none() => true,
            Some((seen, since)) if seen == key => since.elapsed() >= ISSUE_SETTLE,
            _ => {
                self.changed = Some((key, web_time::Instant::now()));
                false
            }
        };
        if self.checked != Some(key) && !settled {
            ui.ctx().request_repaint_after(ISSUE_SETTLE);
        }
        if self.checked != Some(key) && settled {
            let found = validation::validate(&document.borrow());
            // An edit often leaves the findings as they were; the picked row stays then.
            if found != self.issues {
                self.issues = found;
                let existing: std::collections::HashSet<_> = self.issues.iter().cloned().collect();
                self.hidden_issues.retain(|issue| existing.contains(issue));
                self.picked = None;
                self.issue_heights.1.clear();
            }
            self.checked = Some(key);
        }
        if self.issue_generation != key.0 {
            self.hidden_issues.clear();
            self.issue_generation = key.0;
        }

        let note = (!self.issues.is_empty()).then(|| self.issues.len().to_string());
        widgets::section_noted(
            ui,
            "map-issues",
            tr("Issues"),
            note.as_deref(),
            true,
            |ui| self.issue_list(ui, document),
        );
    }

    fn issue_list(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        if !self.hidden_issues.is_empty()
            && ui
                .button(format!(
                    "{} ({})",
                    tr("Show hidden issues"),
                    self.hidden_issues.len()
                ))
                .clicked()
        {
            self.hidden_issues.clear();
            self.issue_heights.1.clear();
        }
        let mut corrected = None;
        let mut hide = None;
        if self.issues.is_empty() {
            widgets::list(ui, 0.0, |ui| widgets::empty(ui, tr("Nothing wrong")));
        } else {
            let errors = self
                .issues
                .iter()
                .filter(|issue| issue.severity == Severity::Error)
                .count();
            widgets::status(
                ui,
                &fill(
                    "{} found, {} of them errors",
                    &[&counted(self.issues.len(), "issue"), &errors.to_string()],
                ),
            );
            let list = widgets::list(ui, 0.0, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("issues")
                    .max_height(160.0)
                    .show_viewport(ui, |ui, viewport| {
                        ui.set_width(ui.available_width());
                        let width = ui.available_width();
                        if self.issue_heights.0 != width
                            || self.issue_heights.1.len() != self.issues.len()
                        {
                            self.issue_heights = (width, vec![0.0; self.issues.len()]);
                        }
                        let mut top = 0.0;
                        for (index, issue) in self.issues.iter().enumerate() {
                            if self.hidden_issues.contains(issue) {
                                continue;
                            }
                            let known = self.issue_heights.1[index];
                            if known > 0.0 && (top + known < viewport.min.y || top > viewport.max.y)
                            {
                                ui.allocate_space(egui::vec2(width, known));
                                top += known;
                                continue;
                            }
                            let colour = if issue.severity == Severity::Error {
                                theme::error()
                            } else {
                                theme::fg()
                            };
                            let picked = self.picked == Some(index);
                            // A wrapped label whose text can be selected and copied; a click on
                            // it still picks the issue and selects its object. The row's
                            // background goes under it once its height is known.
                            let background = ui.painter().add(egui::Shape::Noop);
                            let shown = egui::Frame::new()
                                .inner_margin(egui::Margin::symmetric(6, 4))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    let label = egui::Label::new(
                                        egui::RichText::new(issue_text(issue)).color(colour),
                                    )
                                    .wrap()
                                    .selectable(true)
                                    .sense(Sense::click());
                                    let clicked = ui.add(label).clicked();
                                    ui.horizontal(|ui| {
                                        if let Some(label) =
                                            document.borrow().issue_fix_label(issue)
                                        {
                                            if ui
                                                .small_button(tr("Fix issue"))
                                                .on_hover_text(tr(label))
                                                .clicked()
                                            {
                                                corrected = Some(issue.clone());
                                            }
                                        }
                                        if ui.small_button(tr("Hide issue")).clicked() {
                                            hide = Some(issue.clone());
                                        }
                                    });
                                    clicked
                                });
                            let rect = shown.response.rect;
                            self.issue_heights.1[index] = rect.height();
                            top += rect.height();
                            let hovered = ui.rect_contains_pointer(rect);
                            let fill = if picked {
                                theme::row_selected()
                            } else if hovered {
                                theme::row_hover()
                            } else {
                                Color32::TRANSPARENT
                            };
                            ui.painter().set(
                                background,
                                egui::Shape::rect_filled(rect, CornerRadius::same(3), fill),
                            );
                            if shown.inner {
                                self.picked = Some(index);
                                self.issues_focused = true;
                                if let Some(node) = issue.node {
                                    document.borrow_mut().select_only(node);
                                }
                            }
                        }
                    })
            });
            // The list keeps the keyboard until a press lands elsewhere; Ctrl+C there copies
            // the picked issue, unless some of its text is selected, which egui copies.
            let pressed_elsewhere = ui.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer
                        .interact_pos()
                        .is_some_and(|at| !list.inner_rect.contains(at))
            });
            if pressed_elsewhere {
                self.issues_focused = false;
            }
            let copy = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
            if copy && self.issues_focused && ui.memory(|m| m.focused().is_none()) {
                if let Some(issue) = self.picked.and_then(|i| self.issues.get(i)) {
                    ui.ctx().copy_text(issue_text(issue));
                }
            }
        }

        if let Some(issue) = hide {
            self.picked = None;
            self.hidden_issues.insert(issue);
            self.issue_heights.1.clear();
        }
        if let Some(issue) = corrected {
            match document.borrow_mut().fix_issue(&issue) {
                Ok(_) => {
                    self.checked = None;
                    self.issue_heights.1.clear();
                }
                Err(error) => editor_core::complain!("{error}"),
            }
        }
        ui.add_space(4.0);
        match widgets::segmented(
            ui,
            "issue-checks",
            &[
                (tr("Check map"), "", true),
                (tr("Check for leaks"), "", true),
            ],
        ) {
            Some(0) => self.checked = None,
            Some(_) => {
                let cell_size =
                    f64::from(editor_core::preferences::pref(|p| p.leak_check_cell_size));
                let outcome = document.borrow_mut().check_for_leaks(cell_size);
                self.leak = Some((
                    validation::describe_leak(&outcome),
                    matches!(outcome, map_model::leak::Leak::Leaked(_)),
                ));
            }
            None => {}
        }
        if let Some((text, leaked)) = &self.leak {
            let colour = if *leaked {
                theme::error()
            } else {
                theme::dim()
            };
            ui.add(
                egui::Label::new(egui::RichText::new(text).color(colour))
                    .wrap()
                    .selectable(true),
            );
        }
    }
}

/// What the views draw, `ViewEditor`: every switch changes only what is drawn, so nothing
/// here reaches the document or the history -- except the tags' actions.
fn view(ui: &mut egui::Ui, document: &Rc<RefCell<Document>>, filters: &Rc<RefCell<ViewFilters>>) {
    widgets::section(ui, "map-view", tr("View"), true, |ui| {
        let mut f = filters.borrow_mut();
        type Read = fn(&ViewFilters) -> bool;
        type Write = fn(&mut ViewFilters, bool);
        let switches: [(&str, Read, Write); 12] = [
            (
                translation_key("Brushes"),
                |f| f.show_brushes,
                |f, on| f.show_brushes = on,
            ),
            (
                translation_key("Point entities"),
                |f| f.show_point_entities,
                |f, on| f.show_point_entities = on,
            ),
            (
                translation_key("Entity models"),
                |f| f.show_point_entity_models,
                |f, on| f.show_point_entity_models = on,
            ),
            (
                translation_key("Entity classnames"),
                |f| f.show_entity_classnames,
                |f, on| f.show_entity_classnames = on,
            ),
            (
                translation_key("Group outlines"),
                |f| f.show_group_bounds,
                |f, on| f.show_group_bounds = on,
            ),
            (
                translation_key("Patches"),
                |f| f.show_patches,
                |f, on| f.show_patches = on,
            ),
            (
                translation_key("Materials"),
                |f| f.textured_faces,
                |f, on| f.textured_faces = on,
            ),
            (
                translation_key("Show omitted faces"),
                |f| f.show_export_hidden,
                |f, on| f.show_export_hidden = on,
            ),
            (
                translation_key("Preview animations"),
                |f| f.preview_motion,
                |f, on| f.preview_motion = on,
            ),
            (
                translation_key("Wireframe"),
                |f| f.wireframe,
                |f, on| f.wireframe = on,
            ),
            (
                translation_key("Edges"),
                |f| f.show_edges,
                |f, on| f.show_edges = on,
            ),
            (
                translation_key("Brush entity outlines"),
                |f| f.show_brush_entity_bounds,
                |f, on| f.show_brush_entity_bounds = on,
            ),
        ];
        for (label, read, write) in switches {
            let mut on = read(&f);
            if ui
                .push_id(label, |ui| {
                    widgets::toggle_row(ui, tr(label), None, &mut on)
                })
                .inner
                .changed()
            {
                write(&mut f, on);
            }
        }
        let names: Vec<&str> = EntityLinkMode::ALL
            .iter()
            .map(|mode| tr(mode.name()))
            .collect();
        let mut index = EntityLinkMode::ALL
            .iter()
            .position(|mode| *mode == f.entity_links)
            .unwrap_or(0);
        if widgets::dropdown(ui, "entity-links", &mut index, &names, ui.available_width()) {
            f.entity_links = EntityLinkMode::ALL[index];
        }
    });

    // The tags the game configuration gives brushes and faces, each with a switch that
    // hides everything carrying it; a game that names none has nothing here.
    let named: Vec<(editor_core::games::Tag, bool, bool, bool)> = {
        let document = document.borrow();
        let (brush_tags, face_tags) = document.tags();
        brush_tags
            .iter()
            .chain(face_tags)
            .map(|tag| {
                (
                    tag.clone(),
                    document.tag_is_hidden(&tag.name),
                    document.can_enable_tag(tag),
                    document.can_disable_tag(tag),
                )
            })
            .collect()
    };
    if named.is_empty() {
        return;
    }
    widgets::section(ui, "map-tags", tr("Tags"), true, |ui| {
        for (tag, hidden, can_enable, can_disable) in named {
            ui.push_id(&tag.name, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let size = widgets::ROW_HEIGHT;
                    let buttons =
                        f32::from(u8::from(can_enable) + u8::from(can_disable)) * (size + 2.0);
                    let mut shown = !hidden;
                    let row = ui
                        .allocate_ui(
                            egui::vec2(ui.available_width() - buttons - 4.0, size),
                            |ui| widgets::toggle_row(ui, &tag.name, None, &mut shown),
                        )
                        .inner;
                    if row.changed() {
                        document.borrow_mut().set_tag_hidden(&tag.name, !shown);
                    }
                    ui.add_space(4.0);
                    if can_enable
                        && widgets::icon_button(
                            ui,
                            size,
                            tr("Turn the selected faces into this tag"),
                            true,
                            false,
                            icons::plus,
                        )
                        .clicked()
                    {
                        let outcome = document.borrow_mut().enable_tag(&tag);
                        editor_core::view_filters::report_tag_outcome(&tag.name, Some(outcome));
                    }
                    if can_disable
                        && widgets::icon_button(
                            ui,
                            size,
                            tr("Turn this tag off the selected faces"),
                            true,
                            false,
                            icons::minus,
                        )
                        .clicked()
                    {
                        let outcome = document.borrow_mut().disable_tag(&tag);
                        editor_core::view_filters::report_tag_outcome(&tag.name, Some(outcome));
                    }
                });
            });
        }
    });
}

/// An issue's row text, which Ctrl+C copies.
fn issue_text(issue: &Issue) -> String {
    format!("{}: {}", issue.severity.name(), issue.description)
}

/// `text` on one line, cut short with an ellipsis where it would pass `width`.
fn ellipsized(
    text: String,
    font: egui::FontId,
    color: Color32,
    width: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::simple_singleline(text, font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(1.0));
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_layer_name_stops_short_of_the_switches() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let job = ellipsized(
                "A very long layer name that would run under the icons (12)".repeat(3),
                theme::body(),
                theme::fg(),
                120.0,
            );
            let galley = ui.painter().layout_job(job);
            assert!(galley.size().x <= 120.5);
            assert_eq!(galley.rows.len(), 1);
            assert_eq!(galley.rows[0].row.glyphs.last().map(|g| g.chr), Some('…'));
        });
        output.textures_delta.clear();
    }

    /// Re-checking a map whose findings did not change keeps the picked issue, and Ctrl+C
    /// copies it once the list has been clicked.
    #[test]
    fn the_picked_issue_survives_a_recheck_and_copies() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let document = Rc::new(RefCell::new(
            Document::open(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map"),
                Default::default(),
            )
            .unwrap(),
        ));
        // Two brushes off the finest grid, so there are findings to pick.
        {
            let mut document = document.borrow_mut();
            let world = document.world_bounds();
            let brushes = [100.01, 200.01].map(|at| {
                map_model::shapes::create_cuboid(
                    &world,
                    &geometry_math::bbox::Bbox3d::from_scalars(at, at + 16.0),
                    "wood",
                )
                .unwrap()
            });
            document.add_brushes(brushes.to_vec());
        }
        let mut page = MapPage::default();
        let mut copied = String::new();
        let mut frame = |page: &mut MapPage, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(460.0, 4000.0),
                )),
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| page.issues(ui, &document));
            output.textures_delta.clear();
            for command in output.platform_output.commands {
                if let egui::OutputCommand::CopyText(text) = command {
                    copied = text;
                }
            }
        };
        frame(&mut page, vec![]);
        assert!(page.issues.len() >= 2, "the map has issues to pick");
        page.picked = Some(1);
        page.issues_focused = true;
        // An edit that leaves the findings as they were.
        document.borrow_mut().select_all();
        document.borrow_mut().clear_selection();
        page.checked = None;
        frame(&mut page, vec![]);
        assert_eq!(page.picked, Some(1));
        frame(&mut page, vec![egui::Event::Copy]);
        assert_eq!(copied, issue_text(&page.issues[1]));
    }
}

#[cfg(test)]
mod idle_check_tests {
    use super::*;

    /// The issues are checked again only once the map has stopped changing for a moment,
    /// and then they are the new map's.
    #[test]
    fn issues_are_checked_again_once_the_edits_stop() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let document = Rc::new(RefCell::new(
            Document::open(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map"),
                Default::default(),
            )
            .unwrap(),
        ));
        let mut page = MapPage::default();
        let frame = |page: &mut MapPage| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(460.0, 2000.0),
                )),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| page.issues(ui, &document));
            output.textures_delta.clear();
        };
        frame(&mut page);
        let before = page.issues.len();
        {
            let mut document = document.borrow_mut();
            let world = document.world_bounds();
            let brush = map_model::shapes::create_cuboid(
                &world,
                &geometry_math::bbox::Bbox3d::from_scalars(300.01, 316.01),
                "wood",
            )
            .unwrap();
            document.add_brushes(vec![brush]);
        }
        frame(&mut page);
        assert_eq!(
            page.issues.len(),
            before,
            "not checked while the map is still changing"
        );
        std::thread::sleep(ISSUE_SETTLE + web_time::Duration::from_millis(50));
        frame(&mut page);
        assert!(page.issues.len() > before, "checked once it settled");
        assert_eq!(page.issues, validation::validate(&document.borrow()));
    }
}
