//! The output pane under the views: what the editor has
//! said, newest at the bottom, and the status strip along the window's foot that shows
//! the latest line. Ctrl+4, or a click on the strip, shows and hides the pane.

use editor_core::console::Kind;
use eframe::egui;

use crate::theme;

/// How tall the pane is.
pub const HEIGHT: f32 = 140.0;
/// How tall the status strip is.
pub const STRIP_HEIGHT: f32 = 22.0;

fn colour(kind: Kind) -> egui::Color32 {
    match kind {
        Kind::Complaint => theme::error(),
        Kind::Said => theme::dim(),
    }
}

/// The status strip: the editor's latest line, and a chevron that opens the pane.
/// Returns whether it was clicked.
pub fn strip(ui: &mut egui::Ui, rect: egui::Rect, open: bool) -> bool {
    let response = ui.interact(rect, ui.id().with("status-strip"), egui::Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        0.0,
        if response.hovered() {
            theme::surface()
        } else {
            theme::base()
        },
    );
    painter.hline(
        rect.x_range(),
        rect.top() + 0.5,
        egui::Stroke::new(1.0, theme::border()),
    );
    let font = egui::FontId::proportional(theme::text_size() - 1.5);
    if let Some((kind, line)) = editor_core::console::lines().into_iter().last() {
        let clip = rect
            .shrink2(egui::vec2(10.0, 0.0))
            .with_max_x(rect.right() - 32.0);
        painter.with_clip_rect(clip).text(
            egui::pos2(rect.left() + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            line,
            font,
            colour(kind),
        );
    }
    let c = egui::pos2(rect.right() - 16.0, rect.center().y);
    let flip = if open { 1.0 } else { -1.0 };
    painter.add(egui::Shape::line(
        vec![
            c + egui::vec2(-4.0, -2.0 * flip),
            c + egui::vec2(0.0, 2.0 * flip),
            c + egui::vec2(4.0, -2.0 * flip),
        ],
        egui::Stroke::new(
            1.4,
            if response.hovered() {
                theme::fg()
            } else {
                theme::dim()
            },
        ),
    ));
    response.clicked()
}

/// Draws the pane into `rect`.
pub fn show(ui: &mut egui::Ui, rect: egui::Rect) {
    ui.painter().rect_filled(rect, 0.0, theme::panel_base());
    ui.painter().hline(
        rect.x_range(),
        rect.top() + 0.5,
        egui::Stroke::new(1.0, theme::border()),
    );
    let inner = rect.shrink2(egui::vec2(10.0, 6.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.spacing_mut().item_spacing.y = 1.0;
    egui::ScrollArea::vertical()
        .id_salt("output-pane")
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(&mut child, |ui| {
            for (kind, line) in editor_core::console::lines() {
                ui.label(egui::RichText::new(line).monospace().color(colour(kind)));
            }
        });
}
