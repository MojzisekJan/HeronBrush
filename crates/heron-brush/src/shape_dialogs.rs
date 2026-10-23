//! Text insertion (`text_dialog`) and reviewed corner joins (`corner_join_dialog`), both
//! modal.
use crate::{dialog, editor::Editor, theme, widgets};
use editor_core::canvas::{Canvas, FillRule};
use editor_core::i18n::{tr, translation_key};
use eframe::egui;
use geometry_math::vec::Vec3d;

/// The line under the text preview before anything has been inserted.
const TEXT_HINT: &str = translation_key("A letter is cut into as few brushes as its shape allows.");

pub struct ShapeDialogs {
    pub settings_open: bool,
    settings_status: String,
    pub text_open: bool,
    pub corner_open: bool,
    /// Whether the text dialog was open last frame, so an opening can be told apart.
    text_was_open: bool,
    text: String,
    height: f64,
    depth: f64,
    /// The text dialog's status line.
    text_status: String,
    corner: Option<editor_core::document::CornerDraft>,
    corner_rounded: bool,
    corner_radius: f64,
    corner_segments: usize,
    corner_inner: bool,
    corner_inner_radius: f64,
    corner_inner_segments: usize,
    corner_concentric: bool,
    corner_wedges: bool,
    corner_preserve_thickness: bool,
    /// Whether the corner's new vertices snap to `corner_grid`, a grid of its own.
    corner_snap: bool,
    corner_grid: f64,
    corner_steps: [[bool; 2]; 4],
    corner_zoom: f32,
    corner_pan: egui::Vec2,
    /// The corner dialog's error, if any.
    status: String,
}
impl Default for ShapeDialogs {
    fn default() -> Self {
        Self {
            settings_open: false,
            settings_status: String::new(),
            text_open: false,
            corner_open: false,
            text_was_open: false,
            text: String::new(),
            height: 64.0,
            depth: 16.0,
            text_status: tr(TEXT_HINT).to_string(),
            corner: None,
            corner_rounded: false,
            corner_radius: 8.0,
            corner_segments: 8,
            corner_inner: false,
            corner_inner_radius: 8.0,
            corner_inner_segments: 8,
            corner_concentric: false,
            corner_wedges: false,
            corner_preserve_thickness: false,
            corner_snap: false,
            corner_grid: 1.0,
            corner_steps: [[true; 2]; 4],
            corner_zoom: 1.0,
            corner_pan: egui::Vec2::ZERO,
            status: String::new(),
        }
    }
}

/// Where pasted text goes: the paste target, with any coordinate that is not a number
/// taken as zero.
fn text_origin(target: Option<Vec3d>) -> Vec3d {
    target
        .map(|at| {
            let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
            Vec3d::new(finite(at.x()), finite(at.y()), finite(at.z()))
        })
        .unwrap_or_else(Vec3d::zero)
}

impl ShapeDialogs {
    fn corner_rounding(&self) -> editor_core::corner_join::Rounding {
        editor_core::corner_join::Rounding {
            outer: self
                .corner_rounded
                .then_some((self.corner_radius, self.corner_segments)),
            inner: self
                .corner_inner
                .then_some((self.corner_inner_radius, self.corner_inner_segments)),
            concentric: self.corner_concentric
                && self.corner_rounded
                && self.corner_inner
                && !self.corner_wedges,
            wedges: self.corner_wedges,
            preserve_thickness: self.corner_preserve_thickness,
            snap: self.corner_snap.then_some(self.corner_grid),
        }
    }
    pub fn corner(&mut self, editor: &Editor) {
        self.corner_open = true;
        self.corner_zoom = 1.0;
        self.corner_pan = egui::Vec2::ZERO;
        self.status.clear();
        self.corner = match editor
            .document
            .borrow()
            .preview_corner_join_options(self.corner_rounding())
        {
            Ok(draft) => Some(draft),
            Err(error) => {
                self.status = error;
                None
            }
        };
        self.refresh_corner_steps(editor);
    }

    fn refresh_corner_steps(&mut self, editor: &Editor) {
        for field in 0..4 {
            for direction in 0..2 {
                let mut options = self.corner_rounding();
                let entry = if field < 2 {
                    &mut options.outer
                } else {
                    &mut options.inner
                };
                let valid = if let Some((radius, segments)) = entry {
                    let delta = if direction == 0 { -1.0 } else { 1.0 };
                    if field % 2 == 0 {
                        *radius = (*radius + delta).max(0.01);
                    } else {
                        *segments = (*segments as isize + delta as isize).max(0) as usize;
                    }
                    editor
                        .document
                        .borrow()
                        .preview_corner_join_options(options)
                        .is_ok()
                } else {
                    false
                };
                self.corner_steps[field][direction] = valid;
            }
        }
    }

    /// Inserts the text as brushes, grouped under its own name, and says how it went.
    fn insert_text(&mut self, editor: &Editor) {
        if self.text.trim().is_empty() {
            self.text_status = tr("Type something to insert.").into();
            return;
        }
        let origin = text_origin(*editor.paste_target.borrow());
        let mut document = editor.document.borrow_mut();
        let brushes = map_model::stylized_text::brushes(
            &self.text,
            self.height,
            self.depth,
            document.current_material(),
            origin,
            &document.world_bounds(),
        );
        if brushes.is_empty() {
            self.text_status = tr("None of those characters are in the font.").into();
            return;
        }
        let count = brushes.len();
        document.run_as_one_edit(|d| {
            d.add_brushes(brushes);
            d.group_selection(self.text.trim());
        });
        self.text_status = format!("Inserted {count} brushes.");
    }

    pub fn open_settings(&mut self) {
        self.settings_status.clear();
        self.settings_open = true;
    }

    fn shape_settings(&mut self, ctx: &egui::Context, editor: &Editor) {
        if !self.settings_open {
            return;
        }
        let before = *editor.shape_parameters.borrow();
        let mut close = false;
        let mut confirm = false;
        let mut cancel = false;
        let pending = editor.document.borrow().has_shape_preview();
        self.settings_open = dialog::show_tool(
            ctx,
            "shape-settings",
            tr("Shape settings"),
            [480.0, 500.0],
            |ui, part| {
                if part == dialog::Part::Actions {
                    let count = if pending { 3.0 } else { 1.0 };
                    let width = ((ui.available_width()
                        - ui.spacing().item_spacing.x * (count - 1.0))
                        / count)
                        .min(110.0);
                    // The strip fills from the right: the step to take last, on the right
                    // as in every dialog, then the ways out.
                    if pending {
                        confirm |= crate::form::accent_button_width(ui, tr("Apply"), width, true)
                            .clicked();
                        cancel |= theme::button(ui, tr("Cancel"), width, true).clicked();
                    }
                    close |= theme::button(ui, tr("Close"), width, true).clicked();
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("shape-settings-controls")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            crate::tool_rows::shape_controls(
                                ui,
                                &mut editor.shape_parameters.borrow_mut(),
                            );
                            if pending {
                                ui.label(
                                    egui::RichText::new(tr("Shape preview")).color(theme::dim()),
                                );
                            }
                            if !self.settings_status.is_empty() {
                                ui.label(
                                    egui::RichText::new(&self.settings_status)
                                        .color(theme::error()),
                                );
                            }
                        });
                }
            },
        ) && !close;
        let parameters = *editor.shape_parameters.borrow();
        if before != parameters {
            self.settings_status.clear();
            let update = editor
                .document
                .borrow_mut()
                .update_shape_preview(parameters);
            if let Err(error) = update {
                self.settings_status = error;
                if let Some(valid) = editor.document.borrow().shape_preview_parameters() {
                    *editor.shape_parameters.borrow_mut() = valid;
                }
            }
        }
        if confirm {
            editor.document.borrow_mut().finish_shape_preview();
        }
        if cancel {
            editor.document.borrow_mut().cancel_shape_preview();
        }
        if before != *editor.shape_parameters.borrow() || confirm || cancel || !self.settings_open {
            ctx.request_repaint_of(egui::ViewportId::ROOT);
            ctx.request_repaint();
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, editor: &Editor) {
        self.shape_settings(ctx, editor);
        let opening = self.text_open && !self.text_was_open;
        if opening {
            // Each opening starts with the hint.
            self.text_status = tr(TEXT_HINT).to_string();
        }
        if self.text_open {
            let mut close = false;
            let mut insert = false;
            let entry_id = egui::Id::unique("text-dialog-entry");
            self.text_open = dialog::show_modal(
                ctx,
                "text",
                tr("Insert 3D text"),
                [600.0, 380.0],
                |ui, part| {
                    if part == dialog::Part::Actions {
                        // Right to left: Insert is the rightmost, the default.
                        insert |= dialog::accent_button(ui, tr("Insert"), true).clicked();
                        close |= dialog::button(ui, tr("Close"), true).clicked();
                        return;
                    }
                    let (escape, enter) = dialog::keys(ui);
                    close |= escape;
                    let response = widgets::row(ui, tr("Text"), None, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.text)
                                .id(entry_id)
                                .hint_text(tr("SIGN TEXT"))
                                .desired_width(ui.available_width())
                                .margin(crate::widgets::body_entry_margin(ui, 6, 6))
                                .min_size(egui::vec2(0.0, widgets::ROW_HEIGHT))
                                .align(egui::Align2::LEFT_CENTER),
                        )
                    });
                    if opening {
                        response.request_focus();
                    }
                    // Enter in the entry, or with nothing else holding the keyboard, inserts.
                    if enter && (response.lost_focus() || ui.memory(|m| m.focused().is_none())) {
                        insert = true;
                    }
                    widgets::row(ui, tr("Height"), None, |ui| {
                        widgets::spin(ui, &mut self.height, 4.0..=512.0, 4.0, 0)
                    });
                    widgets::row(ui, tr("Depth"), None, |ui| {
                        widgets::spin(ui, &mut self.depth, 1.0..=256.0, 1.0, 0)
                    });
                    ui.add_space(4.0);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 130.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect(
                        rect,
                        egui::CornerRadius::same(6),
                        theme::well(),
                        egui::Stroke::new(1.0, theme::border()),
                        egui::StrokeKind::Inside,
                    );
                    text_preview(
                        ui,
                        rect,
                        if self.text.is_empty() {
                            "Heron Brush"
                        } else {
                            &self.text
                        },
                    );
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&self.text_status).color(theme::dim()),
                        )
                        .wrap(),
                    );
                },
            ) && !close;
            if insert {
                self.insert_text(editor);
            }
        }
        self.text_was_open = self.text_open;

        if self.corner_open {
            let mut close = false;
            let mut apply = false;
            self.corner_open = dialog::show_modal(
                ctx,
                "corner",
                tr("Join corner"),
                [800.0, 500.0],
                |ui, part| {
                    if part == dialog::Part::Actions {
                        apply |=
                            dialog::accent_button(ui, tr("Apply"), self.corner.is_some()).clicked();
                        close |= dialog::button(ui, tr("Cancel"), true).clicked();
                        return;
                    }
                    let (escape, enter) = dialog::keys(ui);
                    close |= escape;
                    apply |= enter && self.corner.is_some();
                    if self
                        .corner
                        .as_ref()
                        .is_some_and(|draft| draft.revision != editor.document.borrow().revision())
                    {
                        self.refresh_corner_steps(editor);
                    }
                    // The controls and the previews scroll together when the window is
                    // shorter than they are.
                    egui::ScrollArea::vertical()
                        .id_salt("corner-controls")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                    let old_options = self.corner_rounding();
                    let mut changed = false;
                    ui.allocate_ui(egui::vec2(ui.available_width(), 190.0), |ui| {
                    ui.columns(2, |columns| {
                        let ui = &mut columns[0];
                        changed |= ui.add_enabled_ui(!self.corner_wedges, |ui| crate::widgets::checkbox(ui, &mut self.corner_rounded, tr("Round outer corner"))).inner.changed();
                        ui.add_enabled_ui(self.corner_rounded && !self.corner_wedges && !(self.corner_concentric && self.corner_inner), |ui| {
                            ui.horizontal(|ui| {
                                ui.label(tr("Radius"));
                                changed |= corner_number(ui, &mut self.corner_radius, 0.01..=65536.0, self.corner_steps[0]);
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr("Segments"));
                                changed |= corner_number(ui, &mut self.corner_segments, 1.0..=64.0, self.corner_steps[1]);
                            });
                        });
                        let ui = &mut columns[1];
                        changed |= ui.add_enabled_ui(!self.corner_wedges, |ui| crate::widgets::checkbox(ui, &mut self.corner_inner, tr("Round inner corner"))).inner.changed();
                        ui.add_enabled_ui(self.corner_inner, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(tr("Radius"));
                                changed |= corner_number(ui, &mut self.corner_inner_radius, 0.01..=65536.0, self.corner_steps[2]);
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr("Segments"));
                                changed |= corner_number(ui, &mut self.corner_inner_segments, 1.0..=64.0, self.corner_steps[3]);
                            });
                        });
                    });
                    ui.add_enabled_ui(self.corner_rounded && self.corner_inner && !self.corner_wedges, |ui| {
                        changed |= crate::widgets::checkbox(ui, &mut self.corner_concentric, tr("Concentric rounding")).changed();
                    });
                    let wedge_changed = crate::widgets::checkbox(ui, &mut self.corner_wedges, tr("Full-thickness wedges")).changed();
                    changed |= wedge_changed;
                    if wedge_changed && self.corner_wedges {
                        self.corner_rounded = true;
                        self.corner_inner = true;
                    }
                    ui.add_enabled_ui(self.corner_wedges, |ui| {
                        changed |= crate::widgets::checkbox(ui, &mut self.corner_preserve_thickness, tr("Preserve wall thickness")).changed();
                    });
                    ui.horizontal(|ui| {
                        changed |= crate::widgets::checkbox(ui, &mut self.corner_snap, tr("Snap to grid")).changed();
                        ui.add_enabled_ui(self.corner_snap, |ui| {
                            changed |= corner_grid(ui, &mut self.corner_grid);
                        });
                    });
                    ui.label(tr(if self.corner_wedges {
                        "Inner radius and segments define the wedges. Thickness mode offsets the outer facets."
                    } else {
                        "Outer radius = inner radius + member width. Inner segments apply to both arcs."
                    }));
                });
                    if changed {
                        match editor
                            .document
                            .borrow()
                            .preview_corner_join_options(self.corner_rounding())
                        {
                            Ok(draft) => {
                                self.corner = Some(draft);
                                self.status.clear();
                            }
                            Err(error) => {
                                // Keep the last valid parameters and preview for invalid text/drag edits.
                                self.corner_rounded = old_options.outer.is_some();
                                self.corner_inner = old_options.inner.is_some();
                                if let Some((r, n)) = old_options.outer {
                                    self.corner_radius = r;
                                    self.corner_segments = n;
                                }
                                if let Some((r, n)) = old_options.inner {
                                    self.corner_inner_radius = r;
                                    self.corner_inner_segments = n;
                                }
                                self.corner_concentric = old_options.concentric;
                                self.corner_wedges = old_options.wedges;
                                self.corner_preserve_thickness = old_options.preserve_thickness;
                                self.corner_snap = old_options.snap.is_some();
                                if let Some(grid) = old_options.snap {
                                    self.corner_grid = grid;
                                }
                                self.status = error;
                            }
                        }
                    }
                    if changed {
                        self.refresh_corner_steps(editor);
                    }
                    if let Some(draft) = &self.corner {
                        ui.horizontal(|ui| {
                            if self.corner_wedges && !self.corner_preserve_thickness {
                                let [u, v] = draft.join.axes;
                                let a = draft.join.before[0].bounds();
                                let b = draft.join.before[1].bounds();
                                ui.label(format!(
                                    "{}: {:.3} × {:.3}",
                                    tr("Outer radii"),
                                    b.max[u] - b.min[u] + self.corner_inner_radius,
                                    a.max[v] - a.min[v] + self.corner_inner_radius
                                ));
                            } else if self.corner_rounding().concentric {
                                let u = draft.join.axes[0];
                                let width = draft.join.before[1].bounds().max[u]
                                    - draft.join.before[1].bounds().min[u];
                                ui.label(format!(
                                    "{}: {:.3}",
                                    tr("Outer radius"),
                                    width + self.corner_inner_radius
                                ));
                            }
                            ui.label(format!(
                                "{}: {}",
                                tr("Added brushes"),
                                draft.join.added.len()
                            ));
                            if draft.join.snapped_away > 0 {
                                ui.label(format!(
                                    "{}: {}",
                                    tr("Left out by the grid"),
                                    draft.join.snapped_away
                                ));
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(tr("Preview zoom"));
                            if ui
                                .add_enabled(self.corner_zoom > 0.25, egui::Button::new("−"))
                                .clicked()
                            {
                                self.corner_zoom = (self.corner_zoom / 1.25).max(0.25);
                            }
                            if ui
                                .add_enabled(self.corner_zoom < 32.0, egui::Button::new("+"))
                                .clicked()
                            {
                                self.corner_zoom = (self.corner_zoom * 1.25).min(32.0);
                            }
                            if ui.button(tr("Reset view")).clicked() {
                                self.corner_zoom = 1.0;
                                self.corner_pan = egui::Vec2::ZERO;
                            }
                        });
                        let height = (ui.available_height() - 40.0).max(160.0);
                        ui.columns(2, |columns| {
                            for (index, ui) in columns.iter_mut().enumerate() {
                                ui.vertical_centered(|ui| {
                                    ui.label(tr(if index == 0 { "Before" } else { "After" }))
                                });
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), height),
                                    egui::Sense::drag(),
                                );
                                if response.hovered() {
                                    let scroll = ui.input_mut(|i| {
                                        let delta = i.smooth_scroll_delta.y;
                                        i.smooth_scroll_delta = egui::Vec2::ZERO;
                                        delta
                                    });
                                    if scroll != 0.0 {
                                        let old = self.corner_zoom;
                                        self.corner_zoom =
                                            (old * (scroll * 0.005).exp()).clamp(0.25, 32.0);
                                        let cursor = response.hover_pos().unwrap_or(rect.center())
                                            - rect.center();
                                        self.corner_pan = cursor
                                            - (cursor - self.corner_pan) * (self.corner_zoom / old);
                                    }
                                }
                                if response.dragged() {
                                    self.corner_pan += ui.input(|i| i.pointer.delta());
                                }
                                if response.double_clicked() {
                                    self.corner_zoom = 1.0;
                                    self.corner_pan = egui::Vec2::ZERO;
                                }
                                draw_join(
                                    ui,
                                    rect,
                                    &draft.join,
                                    index == 1,
                                    self.corner_zoom,
                                    self.corner_pan,
                                );
                            }
                        });
                    }
                    if !self.status.is_empty() {
                        ui.add(egui::Label::new(&self.status).wrap().selectable(true));
                    }
                        });
                },
            ) && !close;
            if apply {
                if let Some(revision) = self.corner.as_ref().map(|draft| draft.revision) {
                    let result = editor
                        .document
                        .borrow_mut()
                        .join_corner_options(revision, self.corner_rounding());
                    match result {
                        Ok(()) => self.corner_open = false,
                        Err(error) => {
                            // Apply stays off once the map has moved on.
                            self.status = error;
                            self.corner = None;
                        }
                    }
                }
            }
        }
    }
}

/// The text's silhouette, filled even-odd so letters keep their holes, centred in `rect`.
fn text_preview(ui: &egui::Ui, rect: egui::Rect, text: &str) {
    let outlines = map_model::stylized_text::outlines(text);
    if outlines.is_empty() {
        return;
    }
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in outlines.iter().flatten() {
        bounds = [
            bounds[0].min(p.x()),
            bounds[1].min(p.y()),
            bounds[2].max(p.x()),
            bounds[3].max(p.y()),
        ];
    }
    let (width, height) = (f64::from(rect.width()), f64::from(rect.height()));
    let scale = ((width - 32.0) / (bounds[2] - bounds[0]).max(0.01))
        .min((height - 32.0) / (bounds[3] - bounds[1]).max(0.01));
    let painter = ui.painter_at(rect);
    let mut cr = crate::egui_canvas::EguiCanvas::new(&painter, rect);
    let c = theme::fg();
    cr.set_source_rgba(
        f64::from(c.r()) / 255.0,
        f64::from(c.g()) / 255.0,
        f64::from(c.b()) / 255.0,
        1.0,
    );
    cr.translate(
        (width - (bounds[2] - bounds[0]) * scale) / 2.0,
        (height - (bounds[3] - bounds[1]) * scale) / 2.0,
    );
    cr.scale(scale, -scale);
    cr.translate(-bounds[0], -bounds[3]);
    for ring in &outlines {
        for (index, p) in ring.iter().enumerate() {
            if index == 0 {
                cr.move_to(p.x(), p.y());
            } else {
                cr.line_to(p.x(), p.y());
            }
        }
        cr.close_path();
    }
    cr.set_fill_rule(FillRule::EvenOdd);
    cr.fill();
}

fn draw_join(
    ui: &egui::Ui,
    rect: egui::Rect,
    join: &editor_core::corner_join::Join,
    after: bool,
    zoom: f32,
    pan: egui::Vec2,
) {
    // The previews sit in a well.
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(6),
        theme::well(),
        egui::Stroke::new(1.0, theme::border()),
        egui::StrokeKind::Inside,
    );
    let [u, v] = join.axes;
    let a = join.before[0].bounds();
    let b = join.before[1].bounds();
    let min = [a.min[u].min(b.min[u]), a.min[v].min(b.min[v])];
    let max = [a.max[u].max(b.max[u]), a.max[v].max(b.max[v])];
    let scale = ((rect.width() as f64 - 32.0) / (max[0] - min[0]).max(0.01))
        .min((rect.height() as f64 - 32.0) / (max[1] - min[1]).max(0.01));
    let scale = scale * zoom as f64;
    let project = |p: geometry_math::vec::Vec3d| {
        egui::pos2(
            rect.center().x + pan.x + ((p[u] - (min[0] + max[0]) / 2.0) * scale) as f32,
            rect.center().y + pan.y - ((p[v] - (min[1] + max[1]) / 2.0) * scale) as f32,
        )
    };
    let painter = ui.painter_at(rect);
    for (index, brush) in (if after { &join.after } else { &join.before })
        .iter()
        .chain(join.added.iter().filter(|_| after))
        .enumerate()
    {
        let colour = theme::series(index);
        for (a, b) in brush.edge_positions() {
            painter.line_segment([project(a), project(b)], egui::Stroke::new(2.0, colour));
        }
    }
}

/// The corner's own grid, picked from the sizes the editor's grid offers, in the same
/// drop-down. Returns whether it changed.
fn corner_grid(ui: &mut egui::Ui, grid: &mut f64) -> bool {
    let exponents: Vec<i32> = crate::chrome::GRID_EXPONENTS.collect();
    let labels: Vec<String> = exponents
        .iter()
        .map(|exponent| crate::chrome::grid_label(2f64.powi(*exponent)))
        .collect();
    let names: Vec<&str> = labels.iter().map(String::as_str).collect();
    let mut index = exponents
        .iter()
        .position(|exponent| 2f64.powi(*exponent) >= *grid)
        .unwrap_or(exponents.len() - 1);
    let changed = crate::widgets::dropdown(ui, "corner-grid", &mut index, &names, 120.0);
    if changed {
        *grid = 2f64.powi(exponents[index]);
    }
    changed
}

fn corner_number<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut T,
    range: std::ops::RangeInclusive<f64>,
    valid: [bool; 2],
) -> bool {
    let before = *value;
    if ui
        .add_enabled(
            valid[0] && value.to_f64() > *range.start(),
            egui::Button::new("−"),
        )
        .clicked()
    {
        *value = T::from_f64((value.to_f64() - 1.0).max(*range.start()));
    }
    let changed = ui
        .add(egui::DragValue::new(value).range(range.clone()))
        .changed();
    if ui
        .add_enabled(
            valid[1] && value.to_f64() + 1.0 <= *range.end(),
            egui::Button::new("+"),
        )
        .clicked()
    {
        *value = T::from_f64(value.to_f64() + 1.0);
    }
    changed || *value != before
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_frame(
        ctx: &egui::Context,
        dialogs: &mut ShapeDialogs,
        editor: &Editor,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ui| dialogs.show(ui.ctx(), editor),
        );
        output.textures_delta.clear();
        output
    }
    fn settings_click(
        ctx: &egui::Context,
        dialogs: &mut ShapeDialogs,
        editor: &Editor,
        at: egui::Pos2,
    ) {
        settings_frame(
            ctx,
            dialogs,
            editor,
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
        settings_frame(
            ctx,
            dialogs,
            editor,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    fn settings_text(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("settings did not show {label}"))
    }

    #[test]
    fn shape_settings_edit_preview_and_close_without_committing_then_cancel() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let editor = Editor::open(&ctx, None);
        let parameters = editor_core::shape_tool::ShapeParameters {
            kind: editor_core::shape_tool::ShapeKind::Arch,
            ..Default::default()
        };
        *editor.shape_parameters.borrow_mut() = parameters;
        let before = editor.document.borrow().to_map_text();
        editor
            .document
            .borrow_mut()
            .begin_shape_preview(
                geometry_math::bbox::Bbox3d::from_scalars(-64.0, 64.0),
                parameters,
            )
            .unwrap();
        let inserted = editor.document.borrow().to_map_text();
        let mut dialogs = ShapeDialogs::default();
        dialogs.open_settings();
        settings_frame(&ctx, &mut dialogs, &editor, vec![]);
        let output = settings_frame(&ctx, &mut dialogs, &editor, vec![]);
        assert_eq!(editor.document.borrow().to_map_text(), inserted);
        // The second numeric row is the arch's thickness.
        let plus = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "+" => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("thickness step button");
        settings_click(&ctx, &mut dialogs, &editor, plus);
        assert_eq!(editor.shape_parameters.borrow().thickness, 17.0);
        assert_ne!(editor.document.borrow().to_map_text(), inserted);
        assert!(editor.document.borrow().has_shape_preview());
        let output = settings_frame(&ctx, &mut dialogs, &editor, vec![]);
        settings_click(
            &ctx,
            &mut dialogs,
            &editor,
            settings_text(&output, tr("Close")),
        );
        assert!(!dialogs.settings_open);
        assert!(editor.document.borrow().has_shape_preview());
        dialogs.open_settings();
        settings_frame(&ctx, &mut dialogs, &editor, vec![]);
        let output = settings_frame(&ctx, &mut dialogs, &editor, vec![]);
        settings_click(
            &ctx,
            &mut dialogs,
            &editor,
            settings_text(&output, tr("Cancel")),
        );
        assert!(!editor.document.borrow().has_shape_preview());
        assert_eq!(editor.document.borrow().to_map_text(), before);
    }

    #[test]
    fn shape_settings_shell_count_advances_by_four_and_undo_stays_atomic() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let editor = Editor::open(&ctx, None);
        let before = editor.document.borrow().to_map_text();
        let parameters = editor_core::shape_tool::ShapeParameters {
            kind: editor_core::shape_tool::ShapeKind::HollowSphere,
            sphere_brushes: 20,
            ..Default::default()
        };
        *editor.shape_parameters.borrow_mut() = parameters;
        editor
            .document
            .borrow_mut()
            .begin_shape_preview(
                geometry_math::bbox::Bbox3d::from_scalars(-64.0, 64.0),
                parameters,
            )
            .unwrap();
        let mut dialogs = ShapeDialogs::default();
        dialogs.open_settings();
        settings_frame(&ctx, &mut dialogs, &editor, vec![]);
        for count in [24, 28] {
            let output = settings_frame(&ctx, &mut dialogs, &editor, vec![]);
            let plus = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == "+" => {
                        Some(t.pos + t.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .max_by(|a, b| a.y.total_cmp(&b.y))
                .unwrap();
            settings_click(&ctx, &mut dialogs, &editor, plus);
            assert_eq!(editor.shape_parameters.borrow().sphere_brushes, count);
            assert_eq!(editor.document.borrow().selection_count(), count);
        }
        assert!(editor.document.borrow().has_shape_preview());
        editor.document.borrow_mut().undo().unwrap();
        assert_eq!(editor.document.borrow().to_map_text(), before);
    }

    #[test]
    fn shape_settings_reject_invalid_shell_thickness_and_keep_the_last_preview() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let editor = Editor::open(&ctx, None);
        let parameters = editor_core::shape_tool::ShapeParameters {
            kind: editor_core::shape_tool::ShapeKind::HollowSphere,
            sphere_thickness: 63.0,
            sphere_brushes: 20,
            ..Default::default()
        };
        *editor.shape_parameters.borrow_mut() = parameters;
        editor
            .document
            .borrow_mut()
            .begin_shape_preview(
                geometry_math::bbox::Bbox3d::from_scalars(-64.0, 64.0),
                parameters,
            )
            .unwrap();
        let inserted = editor.document.borrow().to_map_text();
        let mut dialogs = ShapeDialogs::default();
        dialogs.open_settings();
        settings_frame(&ctx, &mut dialogs, &editor, vec![]);
        let output = settings_frame(&ctx, &mut dialogs, &editor, vec![]);
        let plus = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "+" => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .min_by(|a, b| a.y.total_cmp(&b.y))
            .expect("thickness step button");
        settings_click(&ctx, &mut dialogs, &editor, plus);
        assert_eq!(editor.shape_parameters.borrow().sphere_thickness, 63.0);
        assert!(!dialogs.settings_status.is_empty());
        assert_eq!(editor.document.borrow().to_map_text(), inserted);
        assert!(editor.document.borrow().has_shape_preview());
    }

    #[test]
    fn shape_settings_confirm_keeps_geometry_with_one_undo() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let editor = Editor::open(&ctx, None);
        let before = editor.document.borrow().to_map_text();
        let parameters = editor_core::shape_tool::ShapeParameters {
            kind: editor_core::shape_tool::ShapeKind::Cone,
            ..Default::default()
        };
        *editor.shape_parameters.borrow_mut() = parameters;
        editor
            .document
            .borrow_mut()
            .begin_shape_preview(
                geometry_math::bbox::Bbox3d::from_scalars(-64.0, 64.0),
                parameters,
            )
            .unwrap();
        let inserted = editor.document.borrow().to_map_text();
        let mut dialogs = ShapeDialogs::default();
        dialogs.open_settings();
        settings_frame(&ctx, &mut dialogs, &editor, vec![]);
        let output = settings_frame(&ctx, &mut dialogs, &editor, vec![]);
        settings_click(
            &ctx,
            &mut dialogs,
            &editor,
            settings_text(&output, tr("Apply")),
        );
        assert!(!editor.document.borrow().has_shape_preview());
        assert_eq!(editor.document.borrow().to_map_text(), inserted);
        editor.document.borrow_mut().undo().unwrap();
        assert_eq!(editor.document.borrow().to_map_text(), before);
    }

    #[test]
    fn shape_settings_all_generators_fit_a_narrow_vertical_form() {
        use editor_core::shape_tool::{ShapeKind, ShapeParameters};
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        for width in [280.0, 456.0] {
            for kind in ShapeKind::ALL {
                let mut parameters = ShapeParameters {
                    kind,
                    hollow: true,
                    ..Default::default()
                };
                let before = parameters;
                let mut occupied = egui::Rect::NOTHING;
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 800.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        crate::tool_rows::shape_controls(ui, &mut parameters);
                        occupied = ui.min_rect();
                    },
                );
                output.textures_delta.clear();
                assert!(
                    occupied.width() <= width + 1.0,
                    "{kind:?} overflowed a {width}px form: {occupied:?}"
                );
                assert_eq!(
                    parameters, before,
                    "drawing settings must preserve every generator's parameters"
                );
            }
        }
    }

    #[test]
    fn corner_steps_disable_geometry_limits_and_segment_endpoints() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let editor = Editor::open(&ctx, None);
        {
            let mut d = editor.document.borrow_mut();
            d.select_all();
            d.delete_selection();
            let world = d.world_bounds();
            for max in [Vec3d::new(128.0, 16.0, 8.0), Vec3d::new(16.0, 96.0, 8.0)] {
                d.add_brushes(vec![map_model::shapes::create_cuboid(
                    &world,
                    &geometry_math::bbox::Bbox3d {
                        min: Vec3d::zero(),
                        max,
                    },
                    "stone",
                )
                .unwrap()]);
            }
            d.select_all();
        }
        let mut dialogs = ShapeDialogs {
            corner_rounded: true,
            corner_radius: 16.0,
            corner_segments: 1,
            ..Default::default()
        };
        dialogs.corner(&editor);
        assert!(dialogs.corner.is_some());
        assert_eq!(
            dialogs.corner_steps[0],
            [true, false],
            "radius is bounded by actual member width"
        );
        assert_eq!(dialogs.corner_steps[1], [false, true]);
        dialogs.corner_segments = 64;
        dialogs.refresh_corner_steps(&editor);
        assert_eq!(dialogs.corner_steps[1], [true, false]);
        dialogs.corner_wedges = true;
        dialogs.corner_inner = true;
        dialogs.corner_inner_radius = 79.0;
        dialogs.corner_inner_segments = 8;
        dialogs.refresh_corner_steps(&editor);
        assert_eq!(
            dialogs.corner_steps[2],
            [true, false],
            "wedges must retain positive straight tails"
        );
    }

    #[test]
    fn a_paste_target_that_is_not_a_number_lands_on_zero() {
        let at = text_origin(Some(Vec3d::new(f64::NAN, 32.0, f64::INFINITY)));
        assert_eq!((at.x(), at.y(), at.z()), (0.0, 32.0, 0.0));
        assert_eq!(text_origin(None), Vec3d::zero());
    }

    #[test]
    fn every_opening_starts_with_the_hint_and_empty_text_is_refused() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let editor = Editor::open(&ctx, None);
        let mut dialogs = ShapeDialogs::default();
        let frame = |dialogs: &mut ShapeDialogs| {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                dialogs.show(ui.ctx(), &editor)
            });
            output.textures_delta.clear();
        };
        dialogs.text_open = true;
        frame(&mut dialogs);
        assert_eq!(dialogs.text_status, tr(TEXT_HINT));
        dialogs.insert_text(&editor);
        assert_eq!(dialogs.text_status, tr("Type something to insert."));
        dialogs.text = "HI".into();
        dialogs.insert_text(&editor);
        assert!(dialogs.text_status.starts_with("Inserted"));
        assert!(editor.document.borrow().selection_count() > 0);
        dialogs.text_open = false;
        frame(&mut dialogs);
        dialogs.text_open = true;
        frame(&mut dialogs);
        assert_eq!(
            dialogs.text_status,
            tr(TEXT_HINT),
            "the status starts afresh"
        );
    }
}
