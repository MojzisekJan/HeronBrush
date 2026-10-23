//! The inspector's Entity tab: the selected entities' properties (`inspector.rs`) and the
//! entity browser (`entity_browser.rs`) under them.

use std::cell::RefCell;
use std::rc::Rc;

use editor_core::document::{DefaultProperties, Document};
use editor_core::entity_panel::{
    choice_menu, merge_entities, parse_colour, ChoiceMenu, MULTIPLE_VALUES,
};
use editor_core::entity_placement::FALLBACK_CLASS;
use editor_core::grid::Grid;
use editor_core::i18n::{counted, fill, tr};
use eframe::egui;
use map_model::fgd::{DefinitionKind, EntityDefinition, PropertyType};
use map_model::node::NodeId;

use crate::widgets::{self, field, icons, Dragged};
use crate::{colour_picker, theme};

/// The Entity tab's own state.
#[derive(Default)]
pub struct EntityPage {
    /// The class filter.
    filter: String,
    /// The class picked in the browser.
    chosen: Option<String>,
    /// A package typed into the wad editor.
    wad: String,
}

impl EntityPage {
    /// Draws the tab.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        grid: &Rc<RefCell<Grid>>,
    ) {
        self.properties(ui, document);
        widgets::section(ui, "entity-browser", tr("Entities"), true, |ui| {
            self.browser(ui, document, grid)
        });
    }

    fn properties(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let entities: Vec<(NodeId, map_model::node::EntityNode)> = document
            .borrow()
            .selected_entities()
            .into_iter()
            .map(|(id, entity)| (id, entity.clone()))
            .collect();
        // Entities of the same class are edited together. Mixed-class selections have no
        // shared property panel.
        let all_one_class = entities.first().is_some_and(|(_, first)| {
            entities
                .iter()
                .all(|(_, e)| e.classname() == first.classname())
        });
        if !all_one_class {
            let count = document.borrow().selection_count();
            let heading = match (count, entities.len()) {
                (0, _) => tr("Nothing selected").to_string(),
                (_, n) if n >= 2 => fill("{} of different classes", &[&counted(n, "entity")]),
                (n, _) => fill("{} selected", &[&counted(n, "object")]),
            };
            widgets::section(ui, "entity-properties", &heading, true, |ui| {
                self.buttons(ui, document, false)
            });
            return;
        }

        let ids: Vec<NodeId> = entities.iter().map(|(id, _)| *id).collect();
        let entity = merge_entities(entities.iter().map(|(_, e)| e));
        let multiple = ids.len() > 1;
        let classname = entity.classname().unwrap_or("(no class)").to_string();
        let definition: Option<EntityDefinition> = document
            .borrow()
            .definitions()
            .and_then(|d| d.find(&classname))
            .cloned();
        let heading = match (&definition, multiple) {
            (_, true) => format!("{} {classname}", counted(ids.len(), "entity")),
            (Some(definition), false) if !definition.description.is_empty() => {
                format!("{classname} — {}", definition.description)
            }
            _ => classname.clone(),
        };

        // A protected-property toggle on each row is only worth showing when the entity
        // is in a linked group -- that is the only time the flag does anything.
        let in_linked_group = document.borrow().any_in_linked_group(&ids);
        widgets::section(ui, "entity-properties", &heading, true, |ui| {
            let columns = Columns::new(ui, in_linked_group);
            for (key, value) in &entity.properties {
                // The `_tb_` keys are the editor's own and are managed elsewhere.
                if map_model::entity_properties::is_editor_property(key) {
                    continue;
                }
                ui.push_id(("property", key), |ui| {
                    self.property_row(
                        ui,
                        document,
                        &ids,
                        key,
                        value,
                        definition.as_ref(),
                        in_linked_group,
                        &columns,
                    )
                });
            }
            // `showDefaultRows`: the properties the class defines that no selected entity
            // has got, dimmed and after their own. Typing a value adds the property.
            if let Some(definition) = &definition {
                for property in &definition.properties {
                    if entity.property(&property.key).is_some() {
                        continue;
                    }
                    ui.push_id(("default", &property.key), |ui| {
                        default_row(ui, document, &ids, property, &columns)
                    });
                }
            }
            ui.add_space(4.0);
            self.buttons(ui, document, true);
        });
    }

    /// Add property and the Defaults menu.
    fn buttons(&self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>, enabled: bool) {
        ui.horizontal(|ui| {
            let half = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
            // Adding a property the class does not define is how a map uses a key the FGD
            // has not caught up with, so it has to be possible.
            if theme::button(ui, tr("Add property"), half, enabled).clicked() {
                let ids: Vec<NodeId> = document
                    .borrow()
                    .selected_entities()
                    .iter()
                    .map(|(id, _)| *id)
                    .collect();
                if !ids.is_empty() {
                    // A blank key is a placeholder the user then types over.
                    document
                        .borrow_mut()
                        .set_entity_property_on(&ids, "new_key", "");
                }
            }
            let defaults = theme::button(ui, tr("Defaults"), half, true)
                .on_hover_text(tr("Write the class's own values"));
            egui::Popup::menu(&defaults).show(|ui| {
                if ui
                    .button(tr("Put existing properties back to their defaults"))
                    .clicked()
                {
                    let written = document
                        .borrow_mut()
                        .set_default_properties(DefaultProperties::Existing);
                    editor_core::say!("wrote {written} default properties");
                    ui.close();
                }
                if ui
                    .button(tr("Add the properties that are missing"))
                    .clicked()
                {
                    let written = document
                        .borrow_mut()
                        .set_default_properties(DefaultProperties::Missing);
                    editor_core::say!("wrote {written} default properties");
                    ui.close();
                }
                ui.separator();
                if ui.button(tr("Clear protected properties")).clicked() {
                    let cleared = document.borrow_mut().clear_protected_properties();
                    if cleared > 0 {
                        editor_core::say!("cleared protected properties on {cleared} entities");
                    } else {
                        editor_core::complain!("nothing selected has a protected property");
                    }
                    ui.close();
                }
            });
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn property_row(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        ids: &[NodeId],
        key: &str,
        value: &str,
        definition: Option<&EntityDefinition>,
        in_linked_group: bool,
        columns: &Columns,
    ) {
        let property = definition.and_then(|d| d.property(key));
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = columns.gap;
            match property {
                // A property the class defines has a name of its own, which is friendlier
                // than its key, and renaming it would only break the class's meaning for it.
                Some(defined) => {
                    let name = if defined.short_description.is_empty() {
                        key
                    } else {
                        &defined.short_description
                    };
                    let tip = match &defined.long_description {
                        Some(long) if !long.is_empty() => format!("{key}\n\n{long}"),
                        _ => key.to_string(),
                    };
                    key_label(
                        ui,
                        name,
                        &tip,
                        columns.key,
                        theme::dim(),
                        defined
                            .long_description
                            .as_ref()
                            .is_some_and(|l| !l.is_empty()),
                    );
                }
                // A key the class does not define is the map's own, so it can be renamed.
                None => {
                    if let Some(new) = field(ui, ui.scope_id().with("key"), key, "", columns.key) {
                        if !new.is_empty() {
                            document
                                .borrow_mut()
                                .rename_entity_property_on(ids, key, &new);
                        }
                    }
                }
            }

            let value_width = columns.value;
            ui.allocate_ui_with_layout(
                egui::vec2(value_width, widgets::ROW_HEIGHT),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(value_width);
                    // `SmartWadEditor`: worldspawn's `wad` is a list of texture packages.
                    if key == "wad" {
                        self.wad_editor(ui, document, ids[0], value, value_width);
                        return;
                    }
                    match property.map(|p| &p.kind) {
                        // A closed set of values is a menu, not free text.
                        Some(PropertyType::Choices(choices)) if !choices.is_empty() => {
                            let ChoiceMenu {
                                labels,
                                values,
                                selected,
                            } = choice_menu(choices, value);
                            let mut picked = selected;
                            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
                            if widgets::dropdown(
                                ui,
                                ("choices", key),
                                &mut picked,
                                &labels,
                                value_width,
                            ) && picked != selected
                            {
                                document.borrow_mut().set_entity_property_on(
                                    ids,
                                    key,
                                    &values[picked],
                                );
                            }
                        }
                        // `SmartFlagsEditor`: a tick per bit rather than the number.
                        Some(PropertyType::Flags(flags)) if !flags.is_empty() => {
                            let current: i64 = value.trim().parse().unwrap_or(0);
                            flag_ticks(ui, flags, current, None, |bit, on| {
                                // Each entity keeps its own value for the other bits.
                                document.borrow_mut().set_entity_flag_on(ids, key, bit, on);
                            });
                        }
                        // `SmartColorEditor`: three numbers, which a picker says better.
                        Some(PropertyType::Color) => {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = columns.gap;
                                if let Some(text) = field(
                                    ui,
                                    ui.scope_id().with("value"),
                                    value,
                                    "",
                                    value_width - 36.0 - columns.gap,
                                ) {
                                    if text != MULTIPLE_VALUES {
                                        document
                                            .borrow_mut()
                                            .set_entity_property_on(ids, key, &text);
                                    }
                                }
                                let mut rgb = parse_colour(value)
                                    .map(|c| c.map(|v| (v * 255.0).round().clamp(0.0, 255.0) as u8))
                                    .unwrap_or([255, 255, 255]);
                                if colour_picker::button(
                                    ui,
                                    ui.scope_id().with("colour"),
                                    &mut rgb,
                                    None,
                                )
                                .changed()
                                {
                                    // Three numbers from 0 to 255, which is what the games read.
                                    let text = format!("{} {} {}", rgb[0], rgb[1], rgb[2]);
                                    document
                                        .borrow_mut()
                                        .set_entity_property_on(ids, key, &text);
                                }
                            });
                        }
                        _ => {
                            let committed =
                                field(ui, ui.scope_id().with("value"), value, "", value_width);
                            // Leaving an untouched "<multiple>" field alone must not write that
                            // literal onto every selected entity.
                            if let Some(text) = committed.filter(|text| text != MULTIPLE_VALUES) {
                                document
                                    .borrow_mut()
                                    .set_entity_property_on(ids, key, &text);
                            }
                        }
                    }
                },
            );

            ui.spacing_mut().item_spacing.x = 0.0;
            // Inside a linked group, a property can be marked as this copy's own, so a sync
            // from another member leaves it be.
            if in_linked_group {
                let protected = ids
                    .iter()
                    .any(|id| document.borrow().is_entity_property_protected(*id, key));
                let icon = if protected {
                    icons::lock
                } else {
                    icons::unlock
                };
                if widgets::icon_button(
                    ui,
                    Columns::TOOL,
                    tr("Keep this property this copy's own when linked groups sync"),
                    true,
                    protected,
                    icon,
                )
                .clicked()
                {
                    document
                        .borrow_mut()
                        .set_entity_property_protected_on(ids, key, !protected);
                }
            }
            // Every property can be removed, including one the class defines: a map may
            // leave it out and take the game's own default.
            if widgets::icon_button(
                ui,
                Columns::TOOL,
                tr("Remove this property"),
                true,
                false,
                icons::minus,
            )
            .clicked()
            {
                document.borrow_mut().remove_entity_property_on(ids, key);
            }
        });
    }

    /// The editor for worldspawn's `wad` property: the packages, and a way to change them.
    fn wad_editor(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        id: NodeId,
        value: &str,
        width: f32,
    ) {
        let packages: Vec<String> = value
            .split(';')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect();
        let write = |packages: Vec<String>| {
            document
                .borrow_mut()
                .set_entity_property(id, "wad", &packages.join(";"));
        };
        if !packages.is_empty() {
            widgets::list(ui, 0.0, |ui| {
                for (at, package) in packages.iter().enumerate() {
                    let (row, response) =
                        widgets::list_row(ui, widgets::LIST_ROW, false, egui::Sense::hover());
                    let (galley, _) = widgets::truncated(
                        ui,
                        package,
                        theme::body(),
                        theme::fg(),
                        row.width() - 34.0,
                    );
                    ui.painter().galley(
                        egui::pos2(
                            row.left() + 6.0,
                            crate::theme::galley_top(ui.ctx(), &galley, row.center().y),
                        ),
                        galley,
                        theme::fg(),
                    );
                    response.on_hover_text(package);
                    let slot = egui::Rect::from_min_size(
                        egui::pos2(row.right() - 24.0, row.center().y - 11.0),
                        egui::vec2(22.0, 22.0),
                    );
                    let remove = ui
                        .scope_builder(
                            egui::UiBuilder::new().max_rect(slot).id_salt(("wad", at)),
                            |ui| {
                                widgets::icon_button(
                                    ui,
                                    22.0,
                                    tr("Stop using this package"),
                                    true,
                                    false,
                                    icons::minus,
                                )
                            },
                        )
                        .inner;
                    if remove.clicked() {
                        let mut left = packages.clone();
                        left.remove(at);
                        write(left);
                    }
                }
            });
        }
        // Adding one is typing a path: a `.wad` is named the way the game names it.
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.wad)
                .hint_text(tr("Add a package, then press Enter"))
                .desired_width(width)
                .margin(crate::widgets::body_entry_margin(ui, 6, 6))
                .min_size(egui::vec2(0.0, widgets::ROW_HEIGHT)),
        );
        if response.lost_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
            && !self.wad.trim().is_empty()
        {
            let mut all = packages.clone();
            all.push(self.wad.trim().to_string());
            self.wad.clear();
            write(all);
        }
    }

    fn browser(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        grid: &Rc<RefCell<Grid>>,
    ) {
        let from = document.borrow().definitions_path().and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        });
        let label = from.unwrap_or_else(|| tr("No definitions loaded").to_string());
        let clicked = widgets::row(
            ui,
            tr("Entity definitions"),
            Some(tr("The entity definitions the classes come from")),
            |ui| {
                let width = ui.available_width();
                theme::button(ui, &label, width, true)
                    .on_hover_text(tr("The entity definitions the classes come from"))
                    .clicked()
            },
        );
        let picked = crate::file_dialog::FileDialog::new()
            .set_title(tr("Entity definitions"))
            .add_filter(
                tr("Entity definitions"),
                &["fgd", "FGD", "def", "DEF", "ent", "ENT"],
            )
            .pick_file(clicked);
        if let Some(path) = picked {
            match document.borrow_mut().load_definitions(&path) {
                Ok(warnings) => {
                    editor_core::say!("loaded entity definitions from {}", path.display());
                    for warning in warnings {
                        editor_core::complain!("warning: {warning}");
                    }
                }
                Err(error) => editor_core::complain!("{}: {error}", path.display()),
            }
        }
        widgets::search(ui, &mut self.filter, tr("Filter classes"));

        // Selected brushes enable solid classes; otherwise the browser lists point classes.
        let making = {
            let document = document.borrow();
            document.selection_count() > 0 && document.selected_entity().is_none()
        };
        let wanted = if making {
            DefinitionKind::Solid
        } else {
            DefinitionKind::Point
        };
        let needle = self.filter.to_lowercase();
        let loaded = document.borrow().definitions().is_some();
        let names: Vec<(String, String)> = document
            .borrow()
            .definitions()
            .map(|definitions| {
                definitions
                    .placeable()
                    .filter(|d| {
                        d.kind == wanted
                            && (needle.is_empty() || d.name.to_lowercase().contains(&needle))
                    })
                    .map(|d| (d.name.clone(), d.description.clone()))
                    .collect()
            })
            .unwrap_or_default();
        widgets::list(ui, 0.0, |ui| {
            if names.is_empty() {
                widgets::empty(
                    ui,
                    if loaded {
                        tr("No matching classes")
                    } else {
                        tr("No definitions loaded")
                    },
                );
                return;
            }
            egui::ScrollArea::vertical()
                .id_salt("entity-classes")
                .max_height(220.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for (name, _) in &names {
                        let picked = self.chosen.as_deref() == Some(name.as_str());
                        let response = class_row(ui, name, picked);
                        // Dragging a class onto a view places the entity at that position.
                        response.dnd_set_drag_payload(Dragged::Class(name.clone()));
                        if response.clicked() {
                            self.chosen = Some(name.clone());
                        }
                        if response.double_clicked() {
                            let size = grid.borrow().actual_size();
                            editor_core::entity_placement::place_entity(document, name, size);
                        }
                    }
                });
        });
        let description = match &self.chosen {
            Some(name) => names
                .iter()
                .find(|(n, _)| n == name)
                .map(|(n, d)| if d.is_empty() { n.clone() } else { d.clone() })
                .unwrap_or_else(|| name.clone()),
            None if names.is_empty() => {
                format!("No definitions loaded; placing a {FALLBACK_CLASS}")
            }
            None => "Choose a class".to_string(),
        };
        widgets::status(ui, &description);
        let width = ui.available_width();
        if theme::button(
            ui,
            if making { tr("Make") } else { tr("Place") },
            width,
            true,
        )
        .clicked()
        {
            let classname = self
                .chosen
                .clone()
                .unwrap_or_else(|| FALLBACK_CLASS.to_string());
            // A solid class is not placed but made out of what is selected.
            if making {
                let made = document
                    .borrow_mut()
                    .create_brush_entity(&classname)
                    .is_some();
                if made {
                    editor_core::say!("made a {classname} out of the selection");
                } else {
                    editor_core::say!("select some brushes to make one out of");
                }
            } else {
                let size = grid.borrow().actual_size();
                editor_core::entity_placement::place_entity(document, &classname, size);
            }
        }
    }
}

/// The widths a property row's parts take, so the rows' fields line up: the key, the
/// value, and the small buttons at the end.
struct Columns {
    key: f32,
    value: f32,
    gap: f32,
}

impl Columns {
    /// A row's small buttons' size.
    const TOOL: f32 = 24.0;

    fn new(ui: &egui::Ui, in_linked_group: bool) -> Self {
        let width = ui.available_width();
        let gap = 4.0;
        let tools = if in_linked_group { 2.0 } else { 1.0 } * Self::TOOL;
        let key = (width * 0.4).floor();
        Self {
            key,
            value: (width - key - tools - 2.0 * gap).max(60.0),
            gap,
        }
    }
}

/// A property's name in the key column: ellipsized, with `tip` on hover and an info mark
/// when the class says more about it.
fn key_label(
    ui: &mut egui::Ui,
    text: &str,
    tip: &str,
    width: f32,
    colour: egui::Color32,
    info: bool,
) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, widgets::ROW_HEIGHT), egui::Sense::hover());
    let room = rect.width() - if info { 20.0 } else { 4.0 };
    let (galley, _) = widgets::truncated(ui, text, theme::body(), colour, room);
    let end = rect.left() + galley.size().x;
    ui.painter().galley(
        egui::pos2(
            rect.left(),
            crate::theme::galley_top(ui.ctx(), &galley, rect.center().y),
        ),
        galley,
        colour,
    );
    if info {
        let mark = egui::Rect::from_center_size(
            egui::pos2(end + 10.0, rect.center().y),
            egui::vec2(12.0, 12.0),
        );
        icons::info(ui.painter(), mark, theme::dim());
    }
    response.on_hover_text(tip);
}

/// A tick per bit of a flags property, under one another. `dim` greys the names, as a
/// default row's are. `set` is called with the bit and its new state.
fn flag_ticks(
    ui: &mut egui::Ui,
    flags: &[map_model::fgd::Flag],
    current: i64,
    dim: Option<egui::Color32>,
    mut set: impl FnMut(i64, bool),
) {
    ui.add_space(4.0);
    ui.spacing_mut().item_spacing.y = 2.0;
    for flag in flags {
        let mut on = current & flag.value != 0;
        let mut text = egui::RichText::new(format!("{} ({})", flag.description, flag.value));
        if let Some(colour) = dim {
            text = text.color(colour);
        }
        if crate::widgets::checkbox(ui, &mut on, text).changed() {
            set(flag.value, on);
        }
    }
    ui.add_space(4.0);
}

/// One row for a property the class defines and the entity has not got: dimmed, with a
/// field that adds it.
fn default_row(
    ui: &mut egui::Ui,
    document: &Rc<RefCell<Document>>,
    ids: &[NodeId],
    property: &map_model::fgd::PropertyDefinition,
    columns: &Columns,
) {
    let tip = property
        .long_description
        .clone()
        .unwrap_or_else(|| property.short_description.clone());
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = columns.gap;
        key_label(
            ui,
            &property.key,
            &tip,
            columns.key,
            theme::disabled(),
            false,
        );
        ui.allocate_ui_with_layout(
            egui::vec2(columns.value, widgets::ROW_HEIGHT),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(columns.value);
                // A `flags` property still shows as ticks, with the class's own defaults checked.
                if let PropertyType::Flags(flags) = &property.kind {
                    if !flags.is_empty() {
                        let default_bits: i64 = flags
                            .iter()
                            .filter(|f| f.set_by_default)
                            .map(|f| f.value)
                            .sum();
                        flag_ticks(ui, flags, default_bits, Some(theme::dim()), |bit, on| {
                            document.borrow_mut().set_entity_flag_seeded_on(
                                ids,
                                &property.key,
                                default_bits,
                                bit,
                                on,
                            );
                        });
                        return;
                    }
                }
                let hint = property.default.clone().unwrap_or_default();
                if let Some(text) = field(ui, ui.scope_id().with("value"), "", &hint, columns.value)
                {
                    if !text.trim().is_empty() {
                        document
                            .borrow_mut()
                            .set_entity_property_on(ids, &property.key, &text);
                    }
                }
            },
        );
    });
}

/// A class in the browser: its icon, when `entity_icons` draws one, then its name; picked
/// or hovered rows are highlighted. Clicked to pick, dragged onto a view to place.
fn class_row(ui: &mut egui::Ui, name: &str, picked: bool) -> egui::Response {
    const ICON: f32 = 18.0;
    let (rect, response) = widgets::list_row(
        ui,
        widgets::LIST_ROW,
        picked,
        widgets::drag_source_sense(ui.ctx()),
    );
    let colour = theme::fg();
    let mut left = rect.left() + 6.0;
    if editor_core::entity_icons::kind(name).is_some() {
        let icon = egui::Rect::from_min_size(
            egui::pos2(left, rect.center().y - ICON / 2.0),
            egui::vec2(ICON, ICON),
        );
        let painter = ui.painter_at(icon);
        let mut canvas = crate::egui_canvas::EguiCanvas::new(&painter, icon);
        let c = theme::dim();
        use editor_core::canvas::Canvas;
        canvas.set_source_rgba(
            f64::from(c.r()) / 255.0,
            f64::from(c.g()) / 255.0,
            f64::from(c.b()) / 255.0,
            f64::from(c.a()) / 255.0,
        );
        editor_core::entity_icons::draw(&mut canvas, name, 0.0, 0.0, f64::from(ICON));
    }
    // Names line up whether or not a class has an icon.
    left += ICON + 6.0;
    let (galley, _) =
        widgets::truncated(ui, name, theme::body(), colour, rect.right() - left - 4.0);
    ui.painter().galley(
        egui::pos2(
            left,
            crate::theme::galley_top(ui.ctx(), &galley, rect.center().y),
        ),
        galley,
        colour,
    );
    response
}
