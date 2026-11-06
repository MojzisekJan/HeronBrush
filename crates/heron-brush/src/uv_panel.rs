//! The Face tab's UV editor: the canvas
//! with the material and the face's outline, the number fields, the flag ticks, the
//! buttons, and leaving a face out of exports. What each control does is
//! `editor_core::uv_view`.

use std::cell::RefCell;
use std::rc::Rc;

use editor_core::document::Document;
use editor_core::i18n::tr;
use editor_core::uv_view::{self, Field, UvCanvas};
use eframe::egui;

use crate::egui_canvas::EguiCanvas;
use crate::theme;
use crate::widgets::{self, icons};

mod edge;
mod keyboard;
mod workflow;

/// How many pixels the canvas is tall.
const CANVAS_HEIGHT: f32 = 160.0;
/// The gap between fields and between buttons on a line.
const GAP: f32 = 4.0;
const BROWSER_MIN_HEIGHT: f32 = 120.0;
const BROWSER_MAX_HEIGHT: f32 = 1200.0;
const BROWSER_SEPARATOR_HEIGHT: f32 = 8.0;

// Six quick UV actions. Fit remains among the additional alignment tools
// rather than taking a slot in this toolbar.
const QUICK_UV_ACTIONS: [(usize, &str); 6] = [
    (0, egui_phosphor::regular::ARROWS_CLOCKWISE),
    (1, egui_phosphor::regular::GLOBE),
    (3, egui_phosphor::regular::FLIP_HORIZONTAL),
    (4, egui_phosphor::regular::FLIP_VERTICAL),
    (5, egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE),
    (6, egui_phosphor::regular::ARROW_CLOCKWISE),
];

/// What a drag on the canvas is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Gesture {
    /// Moving or shearing what the press caught; the press point, in screen space.
    Edit(egui::Pos2),
    /// Panning the view, from the press point.
    Pan(egui::Pos2),
}

/// The panel's own state.
#[derive(Default)]
pub struct UvPanel {
    canvas: UvCanvas,
    selected_edge: Option<(editor_core::document::FaceRef, usize)>,
    previous_view: Option<editor_core::uv_view::UvView>,
    window_open: bool,
    number_drag: bool,
    pivot_vertex: usize,
    density_face: Option<editor_core::document::FaceRef>,
    density_revision: Option<u64>,
    canvas_size: Option<(f32, f32)>,
    gesture: Option<Gesture>,
    /// The line under the canvas when "Omit covered faces" has something to say, with the
    /// document revision it was said at: it gives way to the material line on the next
    /// change.
    said: Option<(String, u64)>,
    /// Which browser is resizing and its height at pointer-down.
    browser_resize: Option<(bool, f32, f32)>,
    keyboard: Option<keyboard::Transform>,
    keyboard_frame: Option<u64>,
    canvas_region: Option<(egui::ViewportId, egui::Rect, egui::Id, egui::LayerId)>,
    #[cfg(test)]
    browser_settings_path: Option<std::path::PathBuf>,
    #[cfg(test)]
    field_rects: Vec<(Field, egui::Rect)>,
}

impl UvPanel {
    pub fn keyboard_input(&mut self, ui: &egui::Ui, document: &Rc<RefCell<Document>>) {
        keyboard::handle(self, ui, document);
    }

    pub fn window_open(&self) -> bool {
        self.window_open
    }

    pub fn toggle_window(&mut self, document: &Rc<RefCell<Document>>) {
        self.cancel_interaction(document);
        self.window_open = !self.window_open;
    }

    fn history_key(ui: &egui::Ui) -> Option<bool> {
        // Read the modifiers on the key event itself: a quick Cmd/Control press
        // and release may both arrive in one frame, leaving i.modifiers empty.
        let shortcut = ui.input(|i| {
            i.events.iter().find_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if (modifiers.command || modifiers.ctrl)
                    && !modifiers.alt
                    && matches!(key, egui::Key::Z | egui::Key::Y) =>
                {
                    Some((*key, *modifiers, *key == egui::Key::Y || modifiers.shift))
                }
                _ => None,
            })
        });
        let (key, modifiers, redo) = shortcut?;
        ui.input_mut(|i| i.consume_key(modifiers, key))
            .then_some(redo)
    }

    fn history(&mut self, document: &Rc<RefCell<Document>>, redo: bool) {
        let editing = self.canvas.dragging() || self.number_drag || self.keyboard.is_some();
        self.cancel_interaction(document);
        if !editing {
            if redo {
                document.borrow_mut().redo();
            } else {
                document.borrow_mut().undo();
            }
        }
    }

    fn cancel_interaction(&mut self, document: &Rc<RefCell<Document>>) {
        self.gesture = None;
        self.keyboard = None;
        self.canvas.drag_cancel(&mut document.borrow_mut());
        if std::mem::take(&mut self.number_drag) {
            document.borrow_mut().cancel_transaction();
        }
    }

    /// One panel/canvas instance moves between the inspector and the window.
    pub fn window_button(ui: &mut egui::Ui, window_open: bool) -> bool {
        let label = if window_open {
            tr("Return to main window")
        } else {
            tr("UV editor in separate window")
        };
        let glyph = if window_open {
            egui_phosphor::regular::ARROW_U_UP_LEFT
        } else {
            egui_phosphor::regular::ARROW_SQUARE_OUT
        };
        let response = widgets::icon_button(
            ui,
            widgets::SECTION_HEIGHT,
            label,
            true,
            window_open,
            |p, rect, color| {
                crate::theme::icon_text(
                    p,
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    theme::icon(),
                    color,
                );
            },
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::Role::Button, true, window_open, label)
        });
        response.clicked()
    }

    pub fn inspector(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        if !self.window_open {
            let width = ui.available_width();
            self.canvas_area(ui, document, width, CANVAS_HEIGHT);
            self.quick_actions(ui, document, false);
            self.transform_fields(ui, document);
        }
    }

    fn browser_height(window: bool) -> f32 {
        let height = editor_core::preferences::pref(|p| {
            if window {
                p.material_browser_window_height
            } else {
                p.material_browser_face_height
            }
        });
        if height.is_finite() {
            height.clamp(BROWSER_MIN_HEIGHT, BROWSER_MAX_HEIGHT)
        } else if window {
            300.0
        } else {
            360.0
        }
    }

    fn browser_separator(
        &mut self,
        ui: &mut egui::Ui,
        window: bool,
        maximum: f32,
        above: bool,
    ) -> egui::Response {
        let maximum = maximum.clamp(BROWSER_MIN_HEIGHT, BROWSER_MAX_HEIGHT);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), BROWSER_SEPARATOR_HEIGHT),
            egui::Sense::hover(),
        );
        let response = ui.interact(
            rect,
            ui.make_persistent_id(("material-browser-separator", window)),
            egui::Sense::drag(),
        );
        let response = response
            .on_hover_cursor(egui::CursorIcon::ResizeVertical)
            .on_hover_text(tr("Height"));
        let color = if response.hovered() || response.dragged() {
            theme::accent()
        } else {
            theme::dim()
        };
        ui.painter().hline(
            rect.x_range(),
            rect.center().y,
            egui::Stroke::new(1.0, color),
        );
        ui.painter().hline(
            rect.center().x - 18.0..=rect.center().x + 18.0,
            rect.center().y,
            egui::Stroke::new(3.0, color),
        );
        if response.drag_started() {
            let origin = ui
                .input(|i| i.pointer.press_origin())
                .or_else(|| response.interact_pointer_pos())
                .unwrap_or(rect.center());
            self.browser_resize =
                Some((window, Self::browser_height(window).min(maximum), origin.y));
        }
        if response.dragged() {
            if let Some((target, start, origin_y)) = self
                .browser_resize
                .filter(|(target, _, _)| *target == window)
            {
                let delta = response
                    .interact_pointer_pos()
                    .map_or(0.0, |pos| pos.y - origin_y);
                let height = (start + delta * if above { -1.0 } else { 1.0 })
                    .clamp(BROWSER_MIN_HEIGHT, maximum);
                if Self::browser_height(target) != height {
                    editor_core::preferences::edit(|p| {
                        if target {
                            p.material_browser_window_height = height;
                        } else {
                            p.material_browser_face_height = height;
                        }
                    });
                    ui.ctx().request_repaint();
                }
            }
        }
        if response.drag_stopped()
            && self
                .browser_resize
                .is_some_and(|(target, _, _)| target == window)
        {
            self.browser_resize = None;
            let path = editor_core::preferences::Preferences::default_path();
            #[cfg(test)]
            let path = self.browser_settings_path.clone().unwrap_or(path);
            if let Err(error) = editor_core::preferences::current().save(&path) {
                editor_core::complain!(
                    "{}",
                    editor_core::i18n::fill(
                        "Could not save the preferences to {}: {}",
                        &[&path.display().to_string(), &error.to_string()]
                    )
                );
            }
        }
        response
    }

    fn browser_contents(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        materials: &mut crate::material_page::MaterialPage,
        height: f32,
        window: bool,
    ) -> bool {
        if materials.reveal {
            widgets::open_section(ui, "face-materials");
        }
        let mut settings = false;
        let open = widgets::section_with_action(
            ui,
            "face-materials",
            tr("Materials"),
            true,
            |ui| {
                settings = widgets::icon_button(
                    ui,
                    widgets::SECTION_HEIGHT,
                    tr("Settings"),
                    true,
                    false,
                    |p, r, color| {
                        theme::icon_text(
                            p,
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::GEAR_SIX,
                            theme::icon(),
                            color,
                        );
                    },
                )
                .clicked();
            },
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(("material-browser-height", window))
                    .auto_shrink([false, false])
                    .max_height(height)
                    .min_scrolled_height(height)
                    .show(ui, |ui| materials.show_sized(ui, document, Some(height)));
            },
        )
        .is_some();
        if settings {
            materials.settings_open = true;
            ui.ctx().request_repaint();
        }
        open
    }

    pub(crate) fn material_browser(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        materials: &mut crate::material_page::MaterialPage,
    ) {
        if self.browser_contents(ui, document, materials, Self::browser_height(false), false) {
            self.browser_separator(ui, false, BROWSER_MAX_HEIGHT, false);
        }
    }

    pub fn show_window(
        &mut self,
        ctx: &egui::Context,
        document: &Rc<RefCell<Document>>,
        materials: &mut crate::material_page::MaterialPage,
    ) {
        if !self.window_open {
            return;
        }
        let was_editing = self.gesture.is_some() || self.number_drag;
        let mut close = false;
        let visible = crate::dialog::show(
            ctx,
            "uv-editor",
            tr("UV editor"),
            [1000.0, 800.0],
            |ui, part| {
                if part == crate::dialog::Part::Actions {
                    close |= crate::dialog::button(ui, tr("Return to main window"), true).clicked();
                    close |= crate::dialog::button(ui, tr("Close"), true).clicked();
                    if crate::dialog::button(ui, tr("Redo"), true).clicked() {
                        self.history(document, true);
                    }
                    if crate::dialog::button(ui, tr("Undo"), true).clicked() {
                        self.history(document, false);
                    }
                    if crate::dialog::button(ui, tr("Reset view"), true).clicked() {
                        self.canvas.reset_view();
                    }
                    return;
                }
                // Native child windows do not run the main window's shortcut handler.
                if ui.ctx().viewport_id() != egui::ViewportId::ROOT && !ui.ctx().text_edit_focused()
                {
                    if let Some(redo) = Self::history_key(ui) {
                        self.history(document, redo);
                    }
                }
                if !was_editing && !ui.ctx().text_edit_focused() {
                    close |= ui.input(|i| i.key_pressed(egui::Key::Escape));
                }
                let height = ui.available_height().max(160.0);
                if ui.available_width() >= 680.0 {
                    let canvas_width =
                        (ui.available_width() - 320.0 - ui.spacing().item_spacing.x).max(200.0);
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(canvas_width, height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                let gap = ui.spacing().item_spacing.y;
                                let overhead = widgets::SECTION_HEIGHT
                                    + BROWSER_SEPARATOR_HEIGHT
                                    + gap * 6.0
                                    + 10.0
                                    + (widgets::ROW_HEIGHT + GAP);
                                let maximum = (height - 160.0 - overhead).max(BROWSER_MIN_HEIGHT);
                                let browser_height = Self::browser_height(true).min(maximum);
                                let canvas_height = (height - browser_height - overhead).max(160.0);
                                self.canvas_area(ui, document, canvas_width, canvas_height);
                                self.quick_actions(ui, document, true);
                                self.browser_separator(ui, true, maximum, true);
                                self.browser_contents(
                                    ui,
                                    document,
                                    materials,
                                    browser_height,
                                    true,
                                );
                            },
                        );
                        ui.allocate_ui_with_layout(
                            egui::vec2(320.0, height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("uv-window-controls")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        self.transform_fields(ui, document);
                                        ui.add_space(GAP);
                                        self.density_controls(ui, document);
                                        ui.add_space(GAP);
                                        self.controls(ui, document);
                                    });
                            },
                        );
                    });
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("uv-window-narrow")
                        .show(ui, |ui| {
                            let width = ui.available_width();
                            self.canvas_area(ui, document, width, CANVAS_HEIGHT);
                            self.quick_actions(ui, document, false);
                            self.transform_fields(ui, document);
                            let browser_height = Self::browser_height(true);
                            self.browser_contents(ui, document, materials, browser_height, true);
                            self.browser_separator(ui, true, BROWSER_MAX_HEIGHT, false);
                            self.controls(ui, document);
                        });
                }
            },
        );
        if !visible || close {
            self.cancel_interaction(document);
            self.window_open = false;
            ctx.request_repaint();
        }
    }

    /// Draws the UV controls alone for input regression tests.
    #[cfg(test)]
    pub fn show(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let width = ui.available_width();
        ui.spacing_mut().item_spacing.y = GAP;
        self.canvas_area(ui, document, width, CANVAS_HEIGHT);
        self.quick_actions(ui, document, false);
        self.transform_fields(ui, document);
        self.controls(ui, document);
    }

    fn quick_actions(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        density_in_panel: bool,
    ) {
        let enabled = document.borrow().face_to_edit().is_some();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for (slot, &(index, glyph)) in QUICK_UV_ACTIONS.iter().enumerate() {
                if slot == 2 || slot == 4 {
                    ui.add_space(GAP);
                }
                let (label, action, tip) = uv_view::BUTTONS[index];
                let title = format!("{} · {}", tr("UV"), tr(label));
                let tooltip = if tip.is_empty() {
                    title
                } else {
                    format!("{}\n{}", title, tr(tip))
                };
                let response = widgets::icon_button(
                    ui,
                    widgets::ROW_HEIGHT,
                    &tooltip,
                    enabled,
                    false,
                    |p, r, color| {
                        theme::icon_text(
                            p,
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            glyph,
                            theme::icon(),
                            color,
                        );
                    },
                );
                response.widget_info(|| {
                    egui::WidgetInfo::selected(egui::Role::Button, enabled, false, tr(label))
                });
                if response.clicked() {
                    let change = match action {
                        uv_view::Action::Rotate { counter_clockwise } => {
                            Some(editor_core::uv_workflow::Transform::Rotate(
                                if counter_clockwise { 90.0 } else { -90.0 },
                            ))
                        }
                        uv_view::Action::Flip { horizontally } => {
                            Some(editor_core::uv_workflow::Transform::Scale(
                                geometry_math::vec::Vec2f::new(
                                    if horizontally { -1.0 } else { 1.0 },
                                    if horizontally { 1.0 } else { -1.0 },
                                ),
                            ))
                        }
                        _ => None,
                    };
                    if let Some(change) = change {
                        let origin = self
                            .canvas
                            .pivot(&document.borrow())
                            .unwrap_or(geometry_math::vec::Vec2f::zero());
                        editor_core::uv_workflow::transform(
                            &mut document.borrow_mut(),
                            change,
                            self.canvas.view.options.pivot,
                            origin,
                        );
                    } else {
                        uv_view::run(
                            &mut document.borrow_mut(),
                            action,
                            crate::view2d::modifiers(ui),
                        );
                    }
                }
            }
            self.workflow_buttons(ui, document, density_in_panel);
        });
    }

    fn transform_fields(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        if self.number_drag && ui.input(|i| !i.focused) {
            self.cancel_interaction(document);
        }
        ui.spacing_mut().item_spacing.y = GAP;
        let enabled = document.borrow().face_to_edit().is_some();
        ui.add_enabled_ui(enabled, |ui| {
            for (text, fields) in uv_view::ROWS.into_iter().take(3) {
                self.number_row(ui, document, text, fields);
            }
        });
    }

    fn number_row(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        text: &'static str,
        fields: &[Field],
    ) {
        let compact = matches!(text, "Offset" | "Scale" | "Angle");
        let label_width = if compact {
            [tr("Offset"), tr("Scale"), tr("Angle")]
                .into_iter()
                .map(|label| {
                    ui.painter()
                        .layout_no_wrap(label.to_string(), theme::body(), theme::fg())
                        .size()
                        .x
                })
                .fold(0.0_f32, f32::max)
                .clamp(48.0, 96.0)
                + 8.0
        } else {
            (ui.available_width() * widgets::LABEL_SHARE).floor()
        };
        widgets::row_with_label_width(ui, tr(text), None, label_width, |ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            let n = fields.len() as f32;
            let each = ((ui.available_width() - GAP * (n - 1.0)) / n).floor();
            for (axis, &field) in fields.iter().enumerate() {
                // A pair is a vector: its components marked X and Y.
                let caption = (fields.len() > 1).then(|| (["X", "Y"][axis.min(1)], axis));
                self.number(ui, document, field, each, caption);
            }
        });
    }

    pub(crate) fn controls(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        if self.number_drag && ui.input(|i| !i.focused) {
            self.cancel_interaction(document);
        }
        ui.horizontal(|ui| {
            ui.label(tr("UV grid"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::button(ui, tr("Reset view"), 108.0, true).clicked() {
                    self.canvas.reset_view();
                }
            });
        });
        let labels_width: f32 = ["U", "V"]
            .into_iter()
            .map(|label| {
                ui.painter()
                    .layout_no_wrap(label.to_string(), theme::body(), theme::fg())
                    .size()
                    .x
            })
            .sum();
        let field_width = ((ui.available_width() - labels_width - 3.0 * GAP) / 2.0)
            .floor()
            .max(1.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for axis in 0..2 {
                ui.label(["U", "V"][axis]);
                let mut value = self.canvas.view.divisions[axis].max(1);
                if ui
                    .add(
                        egui_native_look::NumberField::new(&mut value)
                            .range(1..=64)
                            .width(field_width)
                            .step(1.0)
                            .accessible_label(format!("UV grid {}", ["U", "V"][axis])),
                    )
                    .changed()
                {
                    self.canvas.view.divisions[axis] = value;
                }
            }
        });
        ui.label(
            egui::RichText::new(tr(
                "Drag grid lines to scale; Alt-drag to shear; Ctrl/Cmd disables snapping",
            ))
            .small()
            .color(theme::dim()),
        );
        let width = ui.available_width();
        ui.spacing_mut().item_spacing.y = GAP;
        let revision = document.borrow().revision();
        let status = match &self.said {
            Some((text, at)) if *at == revision => text.clone(),
            _ => uv_view::status(&document.borrow()),
        };
        ui.label(egui::RichText::new(status).color(theme::dim()));

        let has_face = document.borrow().face_to_edit().is_some();
        ui.add_enabled_ui(has_face, |ui| {
            for (text, fields) in uv_view::ROWS.into_iter().skip(3) {
                self.number_row(ui, document, text, fields);
                let field = match text {
                    "Content flags" => Field::ContentFlags,
                    "Surface flags" => Field::SurfaceFlags,
                    _ => continue,
                };
                let named = {
                    let document = document.borrow();
                    let names = if field == Field::ContentFlags {
                        document.content_flag_names()
                    } else {
                        document.surface_flag_names()
                    };
                    !uv_view::flag_ticks(names).is_empty()
                };
                if !named {
                    continue;
                }
                // The ticks sit under the number, in the value column.
                let indent = (ui.available_width() * widgets::LABEL_SHARE).floor()
                    + ui.spacing().item_spacing.x;
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    ui.vertical(|ui| flag_ticks(ui, document, field));
                });
            }
        });

        ui.add_space(GAP);
        let run = |ui: &egui::Ui, action| {
            // The modifiers held when the button is let go: Shift steps an alignment
            // backwards, Ctrl keeps a trim sheet's subdivision.
            let modifiers = crate::view2d::modifiers(ui);
            uv_view::run(&mut document.borrow_mut(), action, modifiers);
        };
        let items = |buttons: &[(&'static str, uv_view::Action, &'static str)]| -> Vec<(&'static str, &'static str, bool)> {
            buttons.iter().map(|(label, _, tip)| (tr(label), if tip.is_empty() { "" } else { tr(tip) }, true)).collect()
        };
        if widgets::segmented(ui, "uv-fit-both", &[(tr("Fit"), "", has_face)]).is_some() {
            run(ui, uv_view::Action::Fit);
        }
        self.edge_actions(ui, document);
        // Aligning: Align, the four justify buttons, and the two fits.
        let align = &uv_view::ALIGNMENT_BUTTONS;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            let icon = widgets::ROW_HEIGHT;
            let rest = ui.available_width() - 4.0 * icon - 4.0 * GAP;
            if widgets::segmented_width(ui, "uv-align", &items(&align[0..1]), (rest / 3.0).floor())
                .is_some()
            {
                run(ui, align[0].1);
            }
            for (side, &(label, action, tip)) in align[1..5].iter().enumerate() {
                let side = justify_side(label).unwrap_or(side);
                if widgets::icon_button(ui, icon, tr(tip), true, false, |p, r, c| {
                    icons::justify(p, r, c, side)
                })
                .clicked()
                {
                    run(ui, action);
                }
            }
            let width = ui.available_width();
            if let Some(i) = widgets::segmented_width(ui, "uv-fit", &items(&align[5..7]), width) {
                run(ui, align[5 + i].1);
            }
        });

        ui.add_space(GAP);
        let mut hidden = document
            .borrow()
            .face_to_edit()
            .is_some_and(|(_, face)| face.export_hidden);
        if ui
            .add_enabled_ui(has_face, |ui| {
                widgets::toggle_row(ui, tr("Omit face from export"), None, &mut hidden)
            })
            .inner
            .changed()
        {
            uv_view::set_export_hidden(&mut document.borrow_mut(), hidden);
        }
        if theme::button(ui, tr("Omit covered faces"), width, true).clicked() {
            let said = uv_view::omit_covered(&mut document.borrow_mut());
            self.said = Some((said, document.borrow().revision()));
        }
    }

    /// The canvas and its gestures: a primary drag moves what
    /// the press caught (Alt shears), a Shift-middle drag shears, a middle or right drag
    /// pans (not with Shift), the wheel zooms about the pointer, and a right double-click
    /// frames the face again.
    fn canvas_area(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        width: f32,
        height: f32,
    ) {
        self.canvas_size = Some((width, height));
        self.canvas
            .prepare(&document.borrow(), f64::from(width), f64::from(height));
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
        self.canvas_region = Some((ui.ctx().viewport_id(), rect, response.id, ui.layer_id()));
        if response.hovered() && !ui.ctx().egui_wants_keyboard_input() {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Home)) {
                self.previous_view = Some(self.canvas.view);
                self.canvas.reset_view();
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::Home)) {
                if let Some(previous) = self.previous_view.replace(self.canvas.view) {
                    let divisions = self.canvas.view.divisions;
                    let options = self.canvas.view.options;
                    self.canvas.view = previous;
                    self.canvas.view.divisions = divisions;
                    self.canvas.view.options = options;
                }
            }
        }
        self.keyboard_input(ui, document);
        let keyboard_owned =
            self.keyboard.is_some() || self.keyboard_frame == Some(ui.ctx().cumulative_frame_nr());
        if response.has_focus() && !ui.ctx().text_edit_focused() {
            if let Some(redo) = Self::history_key(ui) {
                self.history(document, redo);
            }
        }
        let height = f64::from(rect.height());
        let local = |at: egui::Pos2| (f64::from(at.x - rect.left()), f64::from(at.y - rect.top()));
        let modifiers = crate::view2d::modifiers(ui);

        if !keyboard_owned && response.drag_started() {
            response.request_focus();
            // The drag begins where the button went down, not where the pointer is once
            // egui has decided this is a drag.
            let at = ui
                .input(|i| i.pointer.press_origin())
                .or_else(|| response.interact_pointer_pos())
                .unwrap_or(rect.center());
            let button = |b| response.dragged_by(b);
            self.gesture = classify(
                button(egui::PointerButton::Primary),
                button(egui::PointerButton::Middle),
                button(egui::PointerButton::Secondary),
                modifiers.shift,
                at,
            );
            match self.gesture {
                Some(Gesture::Edit(_)) => {
                    let (x, y) = local(at);
                    let shear = button(egui::PointerButton::Middle) || modifiers.alt;
                    if modifiers.control && !shear {
                        self.canvas
                            .rotate_drag_begin(&mut document.borrow_mut(), height, x, y);
                    } else {
                        self.canvas
                            .drag_begin(&mut document.borrow_mut(), height, x, y, shear);
                    }
                }
                Some(Gesture::Pan(_)) => self.canvas.pan_begin(),
                None => {}
            }
        }
        if !keyboard_owned && response.dragged() {
            if let Some(now) = response.interact_pointer_pos() {
                match self.gesture {
                    Some(Gesture::Edit(from)) => {
                        let delta = now - from;
                        self.canvas.drag_update(
                            &mut document.borrow_mut(),
                            height,
                            f64::from(delta.x),
                            f64::from(delta.y),
                            modifiers.control,
                        );
                    }
                    Some(Gesture::Pan(from)) => {
                        let delta = now - from;
                        self.canvas
                            .pan_update(f64::from(delta.x), f64::from(delta.y));
                    }
                    None => {}
                }
            }
        }
        if !keyboard_owned && response.drag_stopped() {
            if let Some(Gesture::Edit(_)) = self.gesture.take() {
                self.canvas.drag_end(&mut document.borrow_mut());
            }
        }
        // Escape, or the window losing focus mid-drag, cancels the edit.
        if matches!(self.gesture, Some(Gesture::Edit(_)))
            && ui.input(|i| i.key_pressed(egui::Key::Escape) || !i.focused)
        {
            self.gesture = None;
            self.canvas.drag_cancel(&mut document.borrow_mut());
        }
        if response.double_clicked_by(egui::PointerButton::Secondary) {
            self.canvas.reset_view();
        }
        // The wheel zooms about the pointer, one step a notch, and is not left for the inspector to scroll with.
        if response.contains_pointer() {
            let steps: Vec<f64> = ui.input(|i| {
                i.events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::MouseWheel { unit, delta, .. } if delta.y != 0.0 => {
                            let divisor = match unit {
                                egui::MouseWheelUnit::Point => 100.0,
                                egui::MouseWheelUnit::Line => 1.0,
                                egui::MouseWheelUnit::Page => 0.1,
                            };
                            Some(-f64::from(delta.y) / divisor)
                        }
                        _ => None,
                    })
                    .collect()
            });
            if let Some(at) = response.hover_pos() {
                let (x, y) = local(at);
                for &step in &steps {
                    self.canvas.scroll(x, y, step);
                }
            }
            // Wheel smoothing can continue on frames with no new raw wheel event.
            // Consume those deltas too, so an ancestor cannot scroll after UV zoom.
            ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
        }

        self.canvas
            .hover(&document.borrow(), height, response.hover_pos().map(local));
        self.pick_edge(ui, document, &response, rect);
        let painter = ui.painter_at(rect);
        let mut canvas = EguiCanvas::new(&painter, rect);
        self.canvas.draw(
            &document.borrow(),
            &mut canvas,
            f64::from(rect.width()),
            height,
        );
        self.paint_edge(document, &painter, rect);
        if let Some(session) = &self.keyboard {
            let text = session.label();
            let font = theme::body();
            let galley = painter.layout_no_wrap(text, font, theme::fg());
            let at = rect.left_top() + egui::vec2(8.0, 8.0);
            painter.rect_filled(
                egui::Rect::from_min_size(
                    at - egui::vec2(4.0, 3.0),
                    galley.size() + egui::vec2(8.0, 6.0),
                ),
                3.0,
                egui::Color32::from_black_alpha(220),
            );
            painter.galley(at, galley, theme::fg());
        }
        if let Some(at) = response.hover_pos() {
            let (x, y) = local(at);
            if let Some(uv_view::Grip::Scale {
                horizontally,
                vertically,
            }) = self.canvas.grid_handle(&document.borrow(), height, x, y)
            {
                ui.ctx().set_cursor_icon(match (horizontally, vertically) {
                    (true, true) => egui::CursorIcon::ResizeNwSe,
                    (true, false) => egui::CursorIcon::ResizeHorizontal,
                    _ => egui::CursorIcon::ResizeVertical,
                });
            } else {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, theme::accent()),
                egui::StrokeKind::Inside,
            );
        }
    }

    /// A number field, a spin button with the field's range and step. Scrubbing it is one step
    /// of the history, as a canvas drag is.
    fn number(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        field: Field,
        width: f32,
        caption: Option<(&str, usize)>,
    ) {
        let current = {
            let document = document.borrow();
            document
                .face_to_edit()
                .map_or(0.0, |(_, face)| field.value(&document, face))
        };
        let (low, high, step, digits) = field.range();
        let mut value = current;
        let response = ui
            .push_id(field_id(field), |ui| match caption {
                Some((text, axis)) => widgets::spin_captioned(
                    ui,
                    &mut value,
                    low..=high,
                    step,
                    digits,
                    width,
                    (text, theme::axis(axis)),
                ),
                None => widgets::spin_width(ui, &mut value, low..=high, step, digits, width),
            })
            .inner;
        #[cfg(test)]
        self.field_rects.push((field, response.rect));
        if response.drag_started() {
            self.number_drag = !document.borrow().in_transaction();
            if self.number_drag {
                document.borrow_mut().begin_transaction();
            }
        }
        if response.changed()
            && value != current
            && (self.number_drag || !document.borrow().in_transaction())
        {
            field.set(&mut document.borrow_mut(), value);
        }
        if response.drag_stopped() && std::mem::take(&mut self.number_drag) {
            document.borrow_mut().commit_transaction();
        }
    }
}

/// Which edge a justify button's glyph shows filled: left, right, top or bottom.
fn justify_side(label: &str) -> Option<usize> {
    ["◧", "◨", "⬒", "⬓"]
        .iter()
        .position(|glyph| *glyph == label)
}

fn field_id(field: Field) -> &'static str {
    match field {
        Field::OffsetX => "uv-offset-x",
        Field::OffsetY => "uv-offset-y",
        Field::ScaleX => "uv-scale-x",
        Field::ScaleY => "uv-scale-y",
        Field::Rotation => "uv-rotation",
        Field::ContentFlags => "uv-content-flags",
        Field::SurfaceFlags => "uv-surface-flags",
        Field::SurfaceValue => "uv-surface-value",
    }
}

/// What a canvas drag with these buttons and Shift does, begun at `at`: the primary button
/// edits (Alt makes it a shear), Shift-middle shears, plain middle and right pan, and
/// Shift-right does nothing: panning is refused while Shift is held.
fn classify(
    primary: bool,
    middle: bool,
    secondary: bool,
    shift: bool,
    at: egui::Pos2,
) -> Option<Gesture> {
    if primary || (middle && shift) {
        Some(Gesture::Edit(at))
    } else if (middle || secondary) && !shift {
        Some(Gesture::Pan(at))
    } else {
        None
    }
}

/// A tick per bit the game configuration names, under the flag word it sets.
fn flag_ticks(ui: &mut egui::Ui, document: &Rc<RefCell<Document>>, field: Field) {
    let (names, value) = {
        let document = document.borrow();
        let names = match field {
            Field::ContentFlags => document.content_flag_names().to_vec(),
            _ => document.surface_flag_names().to_vec(),
        };
        let value = document
            .face_to_edit()
            .map_or(0, |(_, face)| field.value(&document, face) as i32);
        (names, value)
    };
    for (mask, name) in uv_view::flag_ticks(&names) {
        let mut set = value & mask != 0;
        if crate::widgets::checkbox(ui, &mut set, name).changed() {
            let updated = if set { value | mask } else { value & !mask };
            field.set(&mut document.borrow_mut(), f64::from(updated));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> Rc<RefCell<Document>> {
        let mut d = Document::new(Default::default());
        for material in ["stone", "brick"] {
            let brush = map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &geometry_math::bbox::Bbox3d::from_scalars(-32.0, 32.0),
                material,
            )
            .unwrap();
            d.add_brushes(vec![brush]);
        }
        let mut ids: Vec<_> = map_model::node::brushes_with_ids(d.tree_world())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        ids.sort();
        d.select_face_only(editor_core::document::FaceRef {
            node: ids[0],
            face: 0,
        });
        Rc::new(RefCell::new(d))
    }
    fn window_frame(
        ctx: &egui::Context,
        panel: &mut UvPanel,
        d: &Rc<RefCell<Document>>,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 1000.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                panel.show_window(
                    ui.ctx(),
                    d,
                    &mut crate::material_page::MaterialPage::default(),
                )
            },
        );
        output.textures_delta.clear();
        output
    }
    fn text_at(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing {label}"))
    }
    fn click(ctx: &egui::Context, panel: &mut UvPanel, d: &Rc<RefCell<Document>>, at: egui::Pos2) {
        window_frame(
            ctx,
            panel,
            d,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        window_frame(
            ctx,
            panel,
            d,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    #[test]
    fn detached_uv_edits_the_selected_face_undoes_and_returns_to_inspector() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        theme::install(&ctx);
        let d = document();
        let mut panel = UvPanel::default();
        let before = d.borrow().to_map_text();
        panel.toggle_window(&d);
        window_frame(&ctx, &mut panel, &d, vec![]);
        window_frame(&ctx, &mut panel, &d, vec![]);
        let output = window_frame(&ctx, &mut panel, &d, vec![]);
        assert_eq!(before, d.borrow().to_map_text());
        assert!(
            panel.canvas_size.unwrap().1 > CANVAS_HEIGHT,
            "detached canvas should use the window height"
        );
        click(
            &ctx,
            &mut panel,
            &d,
            text_at(&output, egui_phosphor::regular::FLIP_HORIZONTAL),
        );
        let after = d.borrow().to_map_text();
        assert_ne!(after, before);
        let output = window_frame(&ctx, &mut panel, &d, vec![]);
        click(&ctx, &mut panel, &d, text_at(&output, tr("Undo")));
        assert_eq!(d.borrow().to_map_text(), before);
        let output = window_frame(&ctx, &mut panel, &d, vec![]);
        click(&ctx, &mut panel, &d, text_at(&output, tr("Redo")));
        assert_eq!(d.borrow().to_map_text(), after);
        let ids: Vec<_> = map_model::node::brushes_with_ids(d.borrow().tree_world())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let other = ids
            .into_iter()
            .find(|id| Some(*id) != d.borrow().face_to_edit().map(|(id, _)| id.node))
            .unwrap();
        d.borrow_mut()
            .select_face_only(editor_core::document::FaceRef {
                node: other,
                face: 0,
            });
        let output = window_frame(&ctx, &mut panel, &d, vec![]);
        assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().starts_with("brick ·"))),"window follows the main selection");
        let unchanged = d.borrow().to_map_text();
        click(
            &ctx,
            &mut panel,
            &d,
            text_at(&output, tr("Return to main window")),
        );
        assert!(!panel.window_open());
        assert_eq!(d.borrow().to_map_text(), unchanged);
    }
    #[test]
    fn detached_uv_targets_face_handles_without_changing_other_faces() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        theme::install(&ctx);
        let d = document();
        let target = {
            let mut d = d.borrow_mut();
            let id = d.face_to_edit().unwrap().0.node;
            d.select_only(id);
            let target = editor_core::document::FaceRef { node: id, face: 2 };
            d.set_face_edit_targets(vec![target]);
            target
        };
        let before = d.borrow().to_map_text();
        let other = editor_core::document::FaceRef {
            node: target.node,
            face: 0,
        };
        let untouched = d.borrow().face_at(other).unwrap().clone();
        let original = d.borrow().face_at(target).unwrap().clone();
        let mut panel = UvPanel::default();
        panel.toggle_window(&d);
        window_frame(&ctx, &mut panel, &d, vec![]);
        window_frame(&ctx, &mut panel, &d, vec![]);
        let output = window_frame(&ctx, &mut panel, &d, vec![]);
        click(
            &ctx,
            &mut panel,
            &d,
            text_at(&output, egui_phosphor::regular::FLIP_HORIZONTAL),
        );
        assert_ne!(*d.borrow().face_at(target).unwrap(), original);
        assert_eq!(*d.borrow().face_at(other).unwrap(), untouched);
        assert_eq!(d.borrow().selection_count(), 1);
        d.borrow_mut().undo().unwrap();
        assert_eq!(d.borrow().to_map_text(), before);
    }
    #[test]
    fn closing_detached_uv_cancels_only_its_unfinished_edits() {
        let d = document();
        let mut panel = UvPanel::default();
        panel.toggle_window(&d);
        let before = d.borrow().to_map_text();
        d.borrow_mut().begin_transaction();
        panel.number_drag = true;
        Field::OffsetX.set(&mut d.borrow_mut(), 32.0);
        assert_ne!(before, d.borrow().to_map_text());
        panel.toggle_window(&d);
        assert_eq!(before, d.borrow().to_map_text());
        assert!(!d.borrow().in_transaction());
        d.borrow_mut().begin_transaction();
        panel.toggle_window(&d);
        panel.toggle_window(&d);
        assert!(
            d.borrow().in_transaction(),
            "closing an idle UV editor must not cancel a map tool drag"
        );
        d.borrow_mut().cancel_transaction();
    }

    #[test]
    fn shift_right_drag_does_not_pan_and_shift_middle_shears() {
        let at = egui::pos2(3.0, 4.0);
        assert_eq!(
            classify(false, false, true, true, at),
            None,
            "Shift-right is denied"
        );
        assert_eq!(
            classify(false, false, true, false, at),
            Some(Gesture::Pan(at))
        );
        assert_eq!(
            classify(false, true, false, false, at),
            Some(Gesture::Pan(at))
        );
        assert_eq!(
            classify(false, true, false, true, at),
            Some(Gesture::Edit(at))
        );
        assert_eq!(
            classify(true, false, false, true, at),
            Some(Gesture::Edit(at))
        );
    }

    #[test]
    fn all_six_quick_uv_buttons_edit_the_face_and_undo_as_one_action() {
        use geometry_math::vec::Vec2f;
        use map_model::uv::UvCoordSystem;
        let ctx = egui::Context::default();
        theme::install(&ctx);
        for (slot, &(_, glyph)) in QUICK_UV_ACTIONS.iter().enumerate() {
            let d = document();
            d.borrow_mut().edit_faces(|f| {
                let (u_axis, v_axis) = f.uv_coord_system.axes(f.normal(), &f.uv_attributes);
                f.uv_coord_system = UvCoordSystem::Parallel { u_axis, v_axis };
                f.uv_attributes.scale = Vec2f::new(1.25, 0.75);
                f.rotate_uv(17.0);
                f.move_uv(Vec2f::new(9.0, -7.0));
            });
            let face = d.borrow().face_to_edit().unwrap().0;
            let original = d.borrow().face_at(face).unwrap().clone();
            let before = d.borrow().to_map_text();
            let mut panel = UvPanel::default();
            let draw = |panel: &mut UvPanel, events| {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(400.0, 1100.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| panel.show(ui, &d),
                );
                output.textures_delta.clear();
                output
            };
            draw(&mut panel, vec![]);
            let output = draw(&mut panel, vec![]);
            let at = text_at(&output, glyph);
            let offset = text_at(&output, tr("Offset"));
            assert!(
                at.y >= CANVAS_HEIGHT && at.y < offset.y,
                "quick actions must be directly below the canvas"
            );
            for &(_, icon) in &QUICK_UV_ACTIONS {
                let position = text_at(&output, icon);
                assert!((position.y - at.y).abs() < 1.0, "all six belong to one row");
                assert_eq!(
                    output
                        .shapes
                        .iter()
                        .filter(
                            |s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==icon)
                        )
                        .count(),
                    1,
                    "no duplicate UV action rows"
                );
            }
            for pressed in [true, false] {
                draw(
                    &mut panel,
                    vec![
                        egui::Event::PointerMoved(at),
                        egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            let changed = d.borrow().face_at(face).unwrap().clone();
            match slot {
                0 => {
                    assert_eq!(changed.uv_attributes, Default::default());
                    assert!(matches!(
                        changed.uv_coord_system,
                        UvCoordSystem::Parallel { .. }
                    ));
                }
                1 => {
                    assert_eq!(changed.uv_attributes, Default::default());
                    assert!(matches!(changed.uv_coord_system, UvCoordSystem::Paraxial));
                }
                2 => {
                    assert_eq!(
                        changed.uv_attributes.scale.x(),
                        -original.uv_attributes.scale.x()
                    );
                    assert_eq!(
                        changed.uv_attributes.scale.y(),
                        original.uv_attributes.scale.y()
                    );
                }
                3 => {
                    assert_eq!(
                        changed.uv_attributes.scale.x(),
                        original.uv_attributes.scale.x()
                    );
                    assert_eq!(
                        changed.uv_attributes.scale.y(),
                        -original.uv_attributes.scale.y()
                    );
                }
                4 => assert!(
                    uv_view::wrap_180(
                        changed.uv_attributes.rotation - original.uv_attributes.rotation - 90.0
                    )
                    .abs()
                        < 0.001
                ),
                5 => assert!(
                    uv_view::wrap_180(
                        changed.uv_attributes.rotation - original.uv_attributes.rotation + 90.0
                    )
                    .abs()
                        < 0.001
                ),
                _ => unreachable!(),
            }
            let after = d.borrow().to_map_text();
            assert_ne!(before, after);
            d.borrow_mut().undo();
            assert_eq!(d.borrow().to_map_text(), before);
            d.borrow_mut().redo();
            assert_eq!(d.borrow().to_map_text(), after);
            d.borrow_mut().clear_selection();
            let unchanged = d.borrow().to_map_text();
            draw(&mut panel, vec![]);
            for pressed in [true, false] {
                draw(
                    &mut panel,
                    vec![
                        egui::Event::PointerMoved(at),
                        egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            assert_eq!(
                d.borrow().to_map_text(),
                unchanged,
                "toolbar is disabled without a target face"
            );
        }
    }

    #[test]
    fn materials_settings_is_a_header_action_and_opens_its_own_window() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let d = document();
        let mut panel = UvPanel::default();
        let mut materials = crate::material_page::MaterialPage::default();
        let draw =
            |panel: &mut UvPanel, materials: &mut crate::material_page::MaterialPage, events| {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1280.0, 1100.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        ui.set_width(320.0);
                        panel.material_browser(ui, &d, materials);
                        materials.show_settings_window(ui.ctx(), &d);
                    },
                );
                output.textures_delta.clear();
                output
            };
        draw(&mut panel, &mut materials, vec![]);
        let output = draw(&mut panel, &mut materials, vec![]);
        let header = text_at(&output, tr("Materials"));
        let settings = text_at(&output, egui_phosphor::regular::GEAR_SIX);
        assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == egui_phosphor::regular::GEAR_SIX && t.galley.rows.len() == 1)));
        assert!((header.y - settings.y).abs() < 4.0);
        assert!(settings.x > header.x);
        let has_title = |output: &egui::FullOutput, title: &str| {
            output
                .shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == title))
        };
        assert!(!has_title(&output, tr("Load folder")));
        // Collapse Materials first: the action must remain available in its header.
        for at in [header, settings] {
            for pressed in [true, false] {
                draw(
                    &mut panel,
                    &mut materials,
                    vec![
                        egui::Event::PointerMoved(at),
                        egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        assert!(materials.settings_open);
        draw(&mut panel, &mut materials, vec![]);
        let output = draw(&mut panel, &mut materials, vec![]);
        assert!(has_title(&output, tr("Browser settings")));
        assert!(has_title(&output, tr("Load folder")));
        assert!(has_title(&output, tr("Texture folders")));
        let list = materials.folder_viewport.expect("folder scroll viewport");
        for _ in 0..30 {
            draw(&mut panel, &mut materials, vec![]);
        }
        let settled = materials.folder_viewport.unwrap();
        assert!(
            (settled.height() - list.height()).abs() < 1.0,
            "filling the list must not grow the window every frame: {list:?} -> {settled:?}"
        );

        assert!(
            list.width() > 400.0 && list.height() > 300.0,
            "folder list must fill the settings body: {list:?}"
        );
    }

    #[test]
    fn offset_scale_and_angle_are_compact_and_before_material_browser_in_both_views() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let d = document();
        let mut panel = UvPanel::default();
        let mut materials = crate::material_page::MaterialPage::default();
        let draw = |panel: &mut UvPanel,
                    materials: &mut crate::material_page::MaterialPage,
                    window: bool| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 1100.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    if window {
                        panel.show_window(ui.ctx(), &d, materials);
                    } else {
                        ui.set_width(320.0);
                        panel.inspector(ui, &d);
                        panel.material_browser(ui, &d, materials);
                    }
                },
            );
            output.textures_delta.clear();
            output
        };
        for window in [false, true] {
            panel.window_open = window;
            draw(&mut panel, &mut materials, window);
            panel.field_rects.clear();
            let output = draw(&mut panel, &mut materials, window);
            for (field, rect) in &panel.field_rects {
                if matches!(
                    field,
                    Field::OffsetX | Field::OffsetY | Field::ScaleX | Field::ScaleY
                ) {
                    assert!(
                        rect.width() > 105.0,
                        "UV numeric entries should use the row's width: {rect:?}"
                    );
                }
            }
            let offset = text_at(&output, tr("Offset"));
            let scale = text_at(&output, tr("Scale"));
            let angle = text_at(&output, tr("Angle"));
            let browser = text_at(&output, tr("Materials"));
            assert!(offset.y < scale.y && scale.y < angle.y);
            if !window {
                assert!(angle.y < browser.y);
            }
            assert!(
                angle.y - offset.y < 80.0,
                "three compact rows must stay together"
            );
            if !window {
                assert!(
                    offset.y < CANVAS_HEIGHT + widgets::ROW_HEIGHT + 50.0,
                    "fields must directly follow the 160px canvas"
                );
            } else {
                assert!(
                    offset.x > browser.x + 400.0,
                    "detached transforms belong in the right column"
                );
            }
        }
    }

    #[test]
    fn browser_separators_resize_independently_and_heights_survive_reload() {
        use editor_core::preferences::{self, Preferences};
        preferences::set_current(Preferences::default());
        let path =
            std::env::temp_dir().join(format!("heron-browser-height-{}.cfg", std::process::id()));
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut panel = UvPanel {
            browser_settings_path: Some(path.clone()),
            ..Default::default()
        };
        let frame = |panel: &mut UvPanel, window: bool, maximum: f32, events| {
            let mut at = egui::Pos2::ZERO;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 500.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    at = panel
                        .browser_separator(ui, window, maximum, window)
                        .rect
                        .center();
                },
            );
            output.textures_delta.clear();
            at
        };
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: egui::Modifiers::NONE,
        };
        for (window, dy, expected) in [(false, 60.0, 420.0), (true, 45.0, 255.0)] {
            let start = frame(&mut panel, window, BROWSER_MAX_HEIGHT, vec![]);
            frame(
                &mut panel,
                window,
                BROWSER_MAX_HEIGHT,
                vec![egui::Event::PointerMoved(start), button(start, true)],
            );
            for delta in [dy * 0.5, dy] {
                frame(
                    &mut panel,
                    window,
                    BROWSER_MAX_HEIGHT,
                    vec![egui::Event::PointerMoved(start + egui::vec2(0.0, delta))],
                );
            }
            frame(
                &mut panel,
                window,
                BROWSER_MAX_HEIGHT,
                vec![button(start + egui::vec2(0.0, dy), false)],
            );
            assert_eq!(
                UvPanel::browser_height(window),
                expected,
                "uses total drag distance once"
            );
        }
        let saved = Preferences::load(&path);
        assert_eq!(saved.material_browser_face_height, 420.0);
        assert_eq!(saved.material_browser_window_height, 255.0);
        preferences::set_current(saved);
        let mut reopened = UvPanel::default();
        frame(&mut reopened, true, 180.0, vec![]);
        assert_eq!(
            UvPanel::browser_height(true),
            255.0,
            "temporarily shrinking a window must not overwrite the saved height"
        );
        assert_eq!(UvPanel::browser_height(false), 420.0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn browser_heights_reject_invalid_saved_values() {
        use editor_core::preferences::{self, Preferences};
        let mut settings = Preferences::default();
        settings.material_browser_face_height = f32::NAN;
        settings.material_browser_window_height = f32::INFINITY;
        preferences::set_current(settings);
        assert_eq!(UvPanel::browser_height(false), 360.0);
        assert_eq!(UvPanel::browser_height(true), 300.0);
        preferences::edit(|p| {
            p.material_browser_face_height = -20.0;
            p.material_browser_window_height = 9000.0;
        });
        assert_eq!(UvPanel::browser_height(false), BROWSER_MIN_HEIGHT);
        assert_eq!(UvPanel::browser_height(true), BROWSER_MAX_HEIGHT);
    }

    #[test]
    fn detached_uv_contains_a_working_material_browser() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let d = document();
        let before = d.borrow().to_map_text();
        let face = d.borrow().face_to_edit().unwrap().0;
        let mut panel = UvPanel {
            window_open: true,
            ..Default::default()
        };
        let mut materials = crate::material_page::MaterialPage::default();
        let draw =
            |panel: &mut UvPanel, materials: &mut crate::material_page::MaterialPage, events| {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1280.0, 1000.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| panel.show_window(ui.ctx(), &d, materials),
                );
                output.textures_delta.clear();
                output
            };
        draw(&mut panel, &mut materials, vec![]);
        let output = draw(&mut panel, &mut materials, vec![]);
        let _ = text_at(&output, tr("Used"));
        let pos = text_at(&output, "special/clip");
        for pressed in [true, false] {
            draw(
                &mut panel,
                &mut materials,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert_eq!(d.borrow().face_at(face).unwrap().material_name, "clip");
        d.borrow_mut().undo();
        assert_eq!(d.borrow().to_map_text(), before);
    }

    #[test]
    fn wheel_over_uv_zooms_without_scrolling_the_parent_or_leaking_smoothed_motion() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let d = document();
        let mut panel = UvPanel::default();
        let mut time = 0.0;
        let mut frame = |panel: &mut UvPanel, events: Vec<egui::Event>| {
            time += 1.0 / 60.0;
            let mut offset = 0.0;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(350.0, 320.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let scroll = egui::ScrollArea::vertical()
                        .id_salt("uv-zoom-parent")
                        .max_height(300.0)
                        .show(ui, |ui| {
                            panel.canvas_area(ui, &d, 280.0, 240.0);
                            ui.add_space(700.0);
                        });
                    offset = scroll.state.clamped_offset().y;
                },
            );
            output.textures_delta.clear();
            offset
        };
        frame(&mut panel, vec![]);
        frame(
            &mut panel,
            vec![egui::Event::PointerMoved(egui::pos2(130.0, 110.0))],
        );
        let before = panel.canvas.view.zoom;
        let wheel = |delta| egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, delta),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::NONE,
        };
        assert_eq!(frame(&mut panel, vec![wheel(-1.0)]), 0.0);
        let first = panel.canvas.view.zoom;
        assert_ne!(first, before, "wheel over the UV canvas must zoom");
        for _ in 0..20 {
            assert_eq!(
                frame(&mut panel, vec![]),
                0.0,
                "wheel inertia must stay out of the parent panel"
            );
        }
        assert_eq!(frame(&mut panel, vec![wheel(1.0)]), 0.0);
        assert!(
            (panel.canvas.view.zoom - before).abs() < 0.0001,
            "opposite wheel directions must reverse zoom"
        );
        for _ in 0..20 {
            assert_eq!(frame(&mut panel, vec![]), 0.0);
        }
        assert!(
            frame(
                &mut panel,
                vec![
                    egui::Event::PointerMoved(egui::pos2(130.0, 280.0)),
                    wheel(-3.0)
                ]
            ) > 0.0,
            "wheel outside UV must still scroll the inspector"
        );
    }

    #[test]
    fn trackpad_zoom_uses_point_magnitude_and_resizing_keeps_navigation() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let d = document();
        let mut panel = UvPanel::default();
        let frame = |panel: &mut UvPanel, width: f32, events: Vec<egui::Event>| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 500.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| panel.canvas_area(ui, &d, width, 260.0),
            );
            output.textures_delta.clear();
        };
        frame(&mut panel, 500.0, vec![]);
        let before = panel.canvas.view.zoom;
        frame(
            &mut panel,
            500.0,
            vec![
                egui::Event::PointerMoved(egui::pos2(120.0, 80.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, 0.1),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!(
            (panel.canvas.view.zoom / before - 1.0).abs() < 0.001,
            "tiny trackpad movement must not cause a full zoom step"
        );
        panel.canvas.view.pan.0 += 37.0;
        let pan = panel.canvas.view.pan;
        let zoom = panel.canvas.view.zoom;
        frame(&mut panel, 520.0, vec![]);
        assert!((panel.canvas.view.pan.0 - pan.0 - 10.0).abs() < 0.001);
        assert_eq!(panel.canvas.view.zoom, zoom);
    }

    /// A primary drag on the canvas moves the material by the distance from where the
    /// button went down, not from where egui decided the drag began.
    #[test]
    fn a_canvas_drag_starts_at_the_press_point() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut document = Document::new(Default::default());
        let brush = map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &geometry_math::bbox::Bbox3d::from_scalars(-32.0, 32.0),
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .unwrap();
        document.add_brushes(vec![brush]);
        document.select_all();
        let document = Rc::new(RefCell::new(document));
        let offset = || {
            document
                .borrow()
                .face_to_edit()
                .unwrap()
                .1
                .uv_attributes
                .offset
        };
        let before = offset();
        let mut panel = UvPanel::default();
        let frame = |panel: &mut UvPanel, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 900.0),
                )),
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| panel.show(ui, &document));
            output.textures_delta.clear();
        };
        let press = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        // The canvas is the panel's first 160 pixels; press away from the origin handle.
        let start = egui::pos2(60.0, 40.0);
        frame(&mut panel, vec![]);
        frame(
            &mut panel,
            vec![egui::Event::PointerMoved(start), press(start, true)],
        );
        // Past the drag threshold in two steps; egui decides on the drag at the first.
        frame(
            &mut panel,
            vec![egui::Event::PointerMoved(start + egui::vec2(10.0, 0.0))],
        );
        frame(
            &mut panel,
            vec![egui::Event::PointerMoved(start + egui::vec2(20.0, 0.0))],
        );
        frame(
            &mut panel,
            vec![press(start + egui::vec2(20.0, 0.0), false)],
        );
        frame(&mut panel, vec![]);
        let moved = offset();
        assert_ne!(moved, before, "the drag moved the material");
        assert!(!document.borrow().in_transaction());
        // The same twenty pixels in one step: the result must match.
        document.borrow_mut().undo();
        assert_eq!(offset(), before);
        let mut again = UvPanel::default();
        frame(&mut again, vec![]);
        frame(
            &mut again,
            vec![egui::Event::PointerMoved(start), press(start, true)],
        );
        frame(
            &mut again,
            vec![egui::Event::PointerMoved(start + egui::vec2(20.0, 0.0))],
        );
        frame(
            &mut again,
            vec![press(start + egui::vec2(20.0, 0.0), false)],
        );
        assert_eq!(offset(), moved, "the drag is measured from the press point");
        let key = |shift| egui::Event::Key {
            key: egui::Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                command: true,
                shift,
                ..Default::default()
            },
        };
        frame(&mut again, vec![key(false)]);
        assert_eq!(
            offset(),
            before,
            "Cmd-Z must undo a completed UV drag while its canvas owns focus"
        );
        frame(&mut again, vec![key(true)]);
        assert_eq!(offset(), moved, "Cmd-Shift-Z must redo the same drag");
    }
    #[test]
    fn picked_uv_edge_enables_alignment_and_clears_when_face_changes() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        theme::install(&ctx);
        let document = document();
        document
            .borrow_mut()
            .edit_faces(|face| face.rotate_uv(27.0));
        let mut panel = UvPanel::default();
        panel.toggle_window(&document);
        window_frame(&ctx, &mut panel, &document, vec![]);
        window_frame(&ctx, &mut panel, &document, vec![]);
        let rect = panel.canvas_region.expect("UV canvas").1;
        let edge = panel.projected_edges(&document.borrow(), rect)[0];
        let midpoint = edge[0] + (edge[1] - edge[0]) * 0.5;
        click(&ctx, &mut panel, &document, midpoint);
        assert_eq!(panel.selected_edge.map(|(_, edge)| edge), Some(0));
        let before = document.borrow().to_map_text();
        let output = window_frame(&ctx, &mut panel, &document, vec![]);
        click(
            &ctx,
            &mut panel,
            &document,
            text_at(&output, tr("Edge horizontal")),
        );
        assert_ne!(
            document.borrow().to_map_text(),
            before,
            "alignment button must edit the selected edge"
        );
        document.borrow_mut().undo();
        assert_eq!(document.borrow().to_map_text(), before);
        let reference = document.borrow().face_to_edit().expect("face").0;
        document
            .borrow_mut()
            .select_face_only(editor_core::document::FaceRef {
                node: reference.node,
                face: (reference.face + 1) % 6,
            });
        window_frame(&ctx, &mut panel, &document, vec![]);
        assert!(
            panel.selected_edge.is_none(),
            "edge selection belongs to a particular face"
        );
    }
}
