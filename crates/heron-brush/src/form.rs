//! Rows and pieces the Export, Live session, Plugins and Render windows share, built on
//! [`widgets`] and [`theme`] so they read like the Preferences window: a sidebar of
//! categories, cards of inspector rows, a status line.

use eframe::egui::{self, Rect, Sense, Stroke};

use crate::{theme, widgets};

/// A switch with its label: the label takes the row's width and the switch sits at its
/// right end, so a sentence-long label is never cut short.
/// Returns true when the value changed.
pub fn switch_row(ui: &mut egui::Ui, label: &str, tooltip: Option<&str>, value: &mut bool) -> bool {
    let width = ui.available_width();
    let switch_w = 40.0;
    let galley = ui.painter().layout(
        label.to_string(),
        theme::body(),
        theme::fg(),
        (width - switch_w - 12.0).max(40.0),
    );
    let height = (galley.size().y + 10.0).max(widgets::ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let enabled = ui.is_enabled();
    let color = if enabled {
        theme::fg()
    } else {
        theme::disabled()
    };
    ui.painter().galley(
        egui::pos2(
            rect.min.x,
            crate::theme::galley_top(ui.ctx(), &galley, rect.center().y),
        ),
        galley,
        color,
    );
    let switch_rect = Rect::from_min_size(
        egui::pos2(
            rect.max.x - switch_w,
            rect.center().y - widgets::ROW_HEIGHT / 2.0,
        ),
        egui::vec2(switch_w, widgets::ROW_HEIGHT),
    );
    let mut changed = false;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(switch_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
        |ui| {
            changed = widgets::switch(ui, value).changed();
        },
    );
    // A click on the label works the switch too.
    if !changed && enabled && response.clicked() {
        *value = !*value;
        changed = true;
    }
    if let Some(tip) = tooltip {
        response.on_hover_text(tip);
    }
    changed
}

/// Secondary text that wraps: hints, notes, what a choice means.
pub fn note(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(egui::RichText::new(text).color(theme::dim())).wrap());
}

/// Ordinary text that wraps.
pub fn text(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(egui::RichText::new(text).color(theme::fg())).wrap());
}

/// The width a button needs for `label`.
pub fn button_width(ui: &egui::Ui, label: &str) -> f32 {
    ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_string(), theme::body(), theme::fg())
            .size()
            .x
    }) + 24.0
}

/// A button as wide as its label.
pub fn button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let width = button_width(ui, label).max(72.0);
    theme::button(ui, label, width, enabled)
}

/// The window's default action: the accent-filled button.
pub fn accent_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let width = (button_width(ui, label) + 8.0).max(90.0);
    accent_button_width(ui, label, width, enabled)
}

/// [`accent_button`] with an explicit width.
pub fn accent_button_width(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    enabled: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, theme::CONTROL_HEIGHT),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    theme::paint_button(ui, rect, &response, label, enabled, true);
    response
}

/// Buttons sharing a row's width equally. Returns the
/// index of the one clicked.
pub fn button_strip(ui: &mut egui::Ui, labels: &[&str], enabled: bool) -> Option<usize> {
    let gap = ui.spacing().item_spacing.x;
    let width =
        ((ui.available_width() - gap * (labels.len() as f32 - 1.0)) / labels.len() as f32).floor();
    let mut clicked = None;
    ui.horizontal(|ui| {
        for (index, label) in labels.iter().enumerate() {
            if theme::button(ui, label, width, enabled).clicked() {
                clicked = Some(index);
            }
        }
    });
    clicked
}

/// The space a window's body has. `dialog::show` keeps the button strip out of the body
/// in both of its forms, so `_embedded` no longer changes anything; it stays so callers
/// need not care how the window is drawn.
pub fn body_rect(ui: &mut egui::Ui, _embedded: bool) -> Rect {
    let height = ui.available_height().max(240.0);
    let full = Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), height));
    ui.allocate_rect(full, Sense::hover());
    full
}

/// A child Ui filling `rect`, laid out top to bottom and clipped to it.
pub fn region(ui: &mut egui::Ui, rect: Rect) -> egui::Ui {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child
}

/// The sidebar's panel: a raised, rounded block.
pub fn sidebar_frame(ui: &egui::Ui, rect: Rect) {
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(6),
        theme::surface(),
        Stroke::new(1.0, theme::border()),
        egui::StrokeKind::Inside,
    );
}

/// One entry of a sidebar: the current one carries the accent, an optional image leads
/// the title, and `height` gives room for it.
pub fn sidebar_item(
    ui: &mut egui::Ui,
    title: &str,
    selected: bool,
    image: Option<&egui::TextureHandle>,
    height: f32,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
    crate::widgets::row_focus(ui, &response);
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(theme::radius());
    if selected {
        painter.rect_filled(rect, radius, theme::accent().gamma_multiply(0.22));
        let bar = Rect::from_min_size(
            rect.min + egui::vec2(0.0, 6.0),
            egui::vec2(3.0, rect.height() - 12.0),
        );
        painter.rect_filled(bar, egui::CornerRadius::same(2), theme::accent());
    } else if response.hovered() {
        painter.rect_filled(rect, radius, theme::button_hover());
    }
    let mut x = rect.min.x + 12.0;
    if let Some(texture) = image {
        let side = (height - 12.0).min(32.0);
        let at = Rect::from_min_size(
            egui::pos2(x, rect.center().y - side / 2.0),
            egui::vec2(side, side),
        );
        painter.image(
            texture.id(),
            at,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        x += side + 10.0;
    }
    let color = if selected || response.hovered() {
        theme::fg()
    } else {
        theme::dim()
    };
    let font = if selected {
        theme::body_bold()
    } else {
        theme::body()
    };
    // A title too long for the sidebar ends in an ellipsis and shows in full on hover.
    let (galley, cut) = crate::widgets::truncated(ui, title, font, color, rect.max.x - x - 8.0);
    painter.galley(
        egui::pos2(
            x,
            crate::theme::galley_top(painter.ctx(), &galley, rect.center().y),
        ),
        galley,
        color,
    );
    if cut {
        response.on_hover_text(title)
    } else {
        response
    }
}

/// A section that folds: a header row with a chevron and a bold title, and `contents`
/// under it while it is open. Remembers whether it is open under `id`.
pub fn fold<R>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    title: &str,
    default_open: bool,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let id = ui.scope_id().with(("fold", id));
    let mut open = ui.data_mut(|d| *d.get_temp_mut_or(id, default_open));
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), widgets::ROW_HEIGHT),
        Sense::click(),
    );
    if response.clicked() {
        open = !open;
        ui.data_mut(|d| d.insert_temp(id, open));
    }
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(theme::radius()),
            theme::button_hover(),
        );
    }
    chevron(
        painter,
        egui::pos2(rect.min.x + 10.0, rect.center().y),
        open,
        if response.hovered() {
            theme::fg()
        } else {
            theme::dim()
        },
    );
    let galley = painter.layout_no_wrap(title.to_string(), theme::body_bold(), theme::fg());
    painter.galley(
        egui::pos2(
            rect.min.x + 22.0,
            crate::theme::galley_top(painter.ctx(), &galley, rect.center().y),
        ),
        galley,
        theme::fg(),
    );
    if open {
        ui.add_space(2.0);
        Some(contents(ui))
    } else {
        None
    }
}

/// A small chevron pointing right, or down when `open`.
pub fn chevron(painter: &egui::Painter, centre: egui::Pos2, open: bool, color: egui::Color32) {
    let points = if open {
        vec![
            centre + egui::vec2(-4.0, -2.0),
            centre + egui::vec2(0.0, 2.0),
            centre + egui::vec2(4.0, -2.0),
        ]
    } else {
        vec![
            centre + egui::vec2(-2.0, -4.0),
            centre + egui::vec2(2.0, 0.0),
            centre + egui::vec2(-2.0, 4.0),
        ]
    };
    painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
}

/// A label and a value in two columns, the value at the right end and selectable.
pub fn value_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(width * 0.55, 20.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add(egui::Label::new(egui::RichText::new(label).color(theme::dim())).wrap());
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(
                egui::Label::new(egui::RichText::new(value).color(theme::fg()))
                    .selectable(true)
                    .wrap(),
            );
        });
    });
}

/// A progress bar the height of a control, with its text centred on it.
pub fn progress(ui: &mut egui::Ui, fraction: f32, text: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), Sense::hover());
    let radius = egui::CornerRadius::same(theme::radius());
    let painter = ui.painter();
    // A sunken track, not an entry's fill, so it does not read as a text field.
    painter.rect(
        rect,
        radius,
        theme::well(),
        Stroke::new(1.0, theme::border()),
        egui::StrokeKind::Inside,
    );
    let fraction = fraction.clamp(0.0, 1.0);
    let mut split = rect.left();
    if fraction > 0.0 {
        let fill = Rect::from_min_size(
            rect.min,
            egui::vec2(
                (rect.width() * fraction).max(2.0 * f32::from(theme::radius())),
                rect.height(),
            ),
        );
        painter.rect_filled(fill, radius, theme::accent());
        split = fill.right();
    }
    // The text is white over the filled part and ordinary over the rest, so it reads on
    // both in either scheme.
    let galley = painter.layout_no_wrap(text.to_string(), theme::body(), theme::fg());
    let at = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        crate::theme::galley_top(painter.ctx(), &galley, rect.center().y),
    );
    let filled = Rect::from_min_max(rect.min, egui::pos2(split, rect.max.y));
    let rest = Rect::from_min_max(egui::pos2(split, rect.min.y), rect.max);
    painter
        .with_clip_rect(rest.intersect(painter.clip_rect()))
        .galley_with_override_text_color(at, galley.clone(), theme::fg());
    if fraction > 0.0 {
        painter
            .with_clip_rect(filled.intersect(painter.clip_rect()))
            .galley_with_override_text_color(at, galley, theme::on_accent());
    }
}

/// A small square button with a folder drawn on it, for choosing a folder.
pub fn folder_button(ui: &mut egui::Ui, tooltip: &str) -> egui::Response {
    let side = widgets::ROW_HEIGHT;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), Sense::click());
    theme::paint_button(ui, rect, &response, "", true, false);
    let color = if response.hovered() {
        theme::fg()
    } else {
        theme::dim()
    };
    let c = rect.center();
    let body = Rect::from_center_size(c + egui::vec2(0.0, 1.0), egui::vec2(14.0, 10.0));
    let painter = ui.painter();
    painter.rect_stroke(
        body,
        egui::CornerRadius::same(2),
        Stroke::new(1.3, color),
        egui::StrokeKind::Middle,
    );
    painter.line_segment(
        [
            body.left_top() + egui::vec2(0.0, -2.0),
            body.left_top() + egui::vec2(5.0, -2.0),
        ],
        Stroke::new(1.3, color),
    );
    response.on_hover_text(tooltip)
}
