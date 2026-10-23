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

/// How many pixels the canvas is tall.
const CANVAS_HEIGHT: f32 = 160.0;
/// The gap between fields and between buttons on a line.
const GAP: f32 = 4.0;

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
    window_open: bool,
    number_drag: bool,
    canvas_size: Option<(f32, f32)>,
    gesture: Option<Gesture>,
    /// The line under the canvas when "Omit covered faces" has something to say, with the
    /// document revision it was said at: it gives way to the material line on the next
    /// change.
    said: Option<(String, u64)>,
}

impl UvPanel {
    pub fn window_open(&self) -> bool {
        self.window_open
    }

    pub fn toggle_window(&mut self, document: &Rc<RefCell<Document>>) {
        self.cancel_interaction(document);
        self.window_open = !self.window_open;
    }

    fn history(&mut self, document: &Rc<RefCell<Document>>, redo: bool) {
        let editing = self.canvas.dragging() || self.number_drag;
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
        self.canvas.drag_cancel(&mut document.borrow_mut());
        if std::mem::take(&mut self.number_drag) {
            document.borrow_mut().cancel_transaction();
        }
    }

    /// One panel/canvas instance moves between the inspector and the window.
    pub fn inspector(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let label = if self.window_open {
            tr("Return to main window")
        } else {
            tr("UV editor in separate window")
        };
        let glyph = if self.window_open {
            egui_phosphor::regular::ARROW_U_UP_LEFT
        } else {
            egui_phosphor::regular::ARROW_SQUARE_OUT
        };
        let response = widgets::icon_button(
            ui,
            widgets::ROW_HEIGHT,
            label,
            true,
            self.window_open,
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
            egui::WidgetInfo::selected(egui::Role::Button, true, self.window_open, label)
        });
        if response.clicked() {
            self.toggle_window(document);
        }
        if !self.window_open {
            self.show(ui, document);
        }
    }

    pub fn show_window(&mut self, ctx: &egui::Context, document: &Rc<RefCell<Document>>) {
        if !self.window_open {
            return;
        }
        let was_editing = self.gesture.is_some() || self.number_drag;
        let mut close = false;
        let visible = crate::dialog::show(
            ctx,
            "uv-editor",
            tr("UV editor"),
            [1000.0, 760.0],
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
                    let modifiers = ui.input(|i| i.modifiers);
                    if (modifiers.command || modifiers.ctrl) && !modifiers.alt {
                        let (z, y) = ui.input_mut(|i| {
                            (
                                i.consume_key(modifiers, egui::Key::Z),
                                i.consume_key(modifiers, egui::Key::Y),
                            )
                        });
                        if z || y {
                            self.history(document, y || modifiers.shift);
                        }
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
                            |ui| self.canvas_area(ui, document, canvas_width, height),
                        );
                        ui.allocate_ui_with_layout(
                            egui::vec2(320.0, height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("uv-window-controls")
                                    .show(ui, |ui| self.controls(ui, document));
                            },
                        );
                    });
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("uv-window-narrow")
                        .show(ui, |ui| self.show(ui, document));
                }
            },
        );
        if !visible || close {
            self.cancel_interaction(document);
            self.window_open = false;
            ctx.request_repaint();
        }
    }

    /// Draws the panel.
    pub fn show(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let width = ui.available_width();
        ui.spacing_mut().item_spacing.y = GAP;
        self.canvas_area(ui, document, width, CANVAS_HEIGHT);
        self.controls(ui, document);
    }

    fn controls(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        if self.number_drag && ui.input(|i| !i.focused) {
            self.cancel_interaction(document);
        }
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
            for (text, fields) in uv_view::ROWS {
                widgets::row(ui, tr(text), None, |ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    let n = fields.len() as f32;
                    let each = ((ui.available_width() - GAP * (n - 1.0)) / n).floor();
                    for (axis, &field) in fields.iter().enumerate() {
                        // A pair is a vector: its components marked X and Y.
                        let caption = (fields.len() > 1).then(|| (["X", "Y"][axis.min(1)], axis));
                        self.number(ui, document, field, each, caption);
                    }
                });
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
        // Resetting and fitting, then flipping and turning.
        for (strip, range) in [(0, 0..3), (1, 3..7)] {
            let buttons = &uv_view::BUTTONS[range];
            if let Some(i) = widgets::segmented(ui, ("uv-buttons", strip), &items(buttons)) {
                run(ui, buttons[i].1);
            }
        }
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
        if self.gesture.is_none() && self.canvas_size != Some((width, height)) {
            self.canvas.reset_view();
            self.canvas_size = Some((width, height));
        }
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
        let height = f64::from(rect.height());
        let local = |at: egui::Pos2| (f64::from(at.x - rect.left()), f64::from(at.y - rect.top()));
        let modifiers = crate::view2d::modifiers(ui);

        if response.drag_started() {
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
                    self.canvas
                        .drag_begin(&mut document.borrow_mut(), height, x, y, shear);
                }
                Some(Gesture::Pan(_)) => self.canvas.pan_begin(),
                None => {}
            }
        }
        if response.dragged() {
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
        if response.drag_stopped() {
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
        if response.hovered() {
            let steps: Vec<f32> = ui.input(|i| {
                i.events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::MouseWheel { delta, .. } if delta.y != 0.0 => Some(delta.y),
                        _ => None,
                    })
                    .collect()
            });
            if let Some(at) = response.hover_pos() {
                let (x, y) = local(at);
                for step in &steps {
                    self.canvas.scroll(x, y, -f64::from(step.signum()));
                }
            }
            if !steps.is_empty() {
                ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
            }
        }
        let painter = ui.painter_at(rect);
        let mut canvas = EguiCanvas::new(&painter, rect);
        self.canvas.draw(
            &document.borrow(),
            &mut canvas,
            f64::from(rect.width()),
            height,
        );
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
        let current = document
            .borrow()
            .face_to_edit()
            .map_or(0.0, |(_, face)| field.value(face));
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
            .map_or(0, |(_, face)| field.value(face) as i32);
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
            |ui| panel.show_window(ui.ctx(), d),
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
        click(&ctx, &mut panel, &d, text_at(&output, tr("Flip H")));
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
        click(&ctx, &mut panel, &d, text_at(&output, tr("Flip H")));
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
    }
}
