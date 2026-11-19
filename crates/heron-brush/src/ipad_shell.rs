//! iPad document navigation and a compact, touch-sized editing shelf.
use crate::{
    app_actions::Action,
    chrome::{Chrome, Popup, Status},
    theme,
};
use editor_core::{i18n::tr, tool::Tool};
use eframe::egui::{self, Rect, Ui};
use egui_phosphor::regular as icon;

pub const HEADER: f32 = 60.0;
pub const SHELF: f32 = 56.0;

/// Only reserve a second shelf for tools that actually have inline controls.
pub fn has_tool_options(tool: Tool) -> bool {
    matches!(tool, Tool::Brush | Tool::Fillet | Tool::Sculpt)
}

/// Keep the canvas usable in portrait and in narrow Stage Manager windows.
pub fn inspector_below(size: egui::Vec2) -> bool {
    size.x < 960.0 && size.y > size.x * 0.75
}

fn button(
    ui: &mut Ui,
    rect: Rect,
    text: impl Into<egui::WidgetText>,
    selected: bool,
) -> egui::Response {
    ui.put(
        rect,
        egui::Button::new(text)
            .selected(selected)
            .corner_radius(10)
            .frame(selected)
            .truncate(),
    )
}

fn dropdown(ui: &mut Ui, rect: Rect, text: impl Into<egui::WidgetText>) -> egui::Response {
    let response = button(ui, rect, text, false);
    crate::chrome::chevron(
        ui.painter(),
        egui::pos2(rect.right() - 12.0, rect.center().y),
        theme::dim(),
    );
    response
}

fn slot(rect: Rect, x: f32, width: f32) -> Rect {
    Rect::from_min_size(
        egui::pos2(x, rect.center().y - 22.0),
        egui::vec2(width, 44.0),
    )
}

fn popup(chrome: &mut Chrome, which: Popup, rect: Rect) {
    chrome.popup = if chrome.popup.is_some_and(|(open, _)| open == which) {
        None
    } else {
        Some((which, rect))
    };
}

pub fn header(ui: &mut Ui, rect: Rect, chrome: &mut Chrome, status: &Status) {
    ui.painter().rect_filled(rect, 0., theme::panel_base());
    let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    bar.set_clip_rect(rect);
    let mut x = rect.left() + 12.;
    let documents = slot(rect, x, 44.);
    if button(
        &mut bar,
        documents,
        egui::RichText::new(icon::FOLDER_OPEN)
            .font(theme::icon())
            .size(23.)
            .color(theme::accent_text()),
        false,
    )
    .on_hover_text(tr("Open map"))
    .clicked()
    {
        chrome.action = Some(Action::Open);
    }
    x += 52.;
    for (glyph, label, action) in [
        (icon::ARROW_U_UP_LEFT, "Undo", Action::Undo),
        (icon::ARROW_U_UP_RIGHT, "Redo", Action::Redo),
    ] {
        if button(
            &mut bar,
            slot(rect, x, 44.),
            egui::RichText::new(glyph)
                .font(theme::icon())
                .size(23.)
                .color(theme::accent_text()),
            false,
        )
        .on_hover_text(tr(label))
        .clicked()
        {
            chrome.action = Some(action);
        }
        x += 48.;
    }
    let right = rect.right() - 12.;
    for (offset, glyph, label) in [
        (44., icon::SIDEBAR_SIMPLE, "Inspector"),
        (96., icon::DOTS_THREE, "More"),
        (148., icon::MAGNIFYING_GLASS, "Search"),
    ] {
        let r = slot(rect, right - offset, 44.);
        if button(
            &mut bar,
            r,
            egui::RichText::new(glyph)
                .font(theme::icon())
                .size(23.)
                .color(if label == "Inspector" && status.inspector_shown {
                    theme::on_accent()
                } else {
                    theme::accent_text()
                }),
            label == "Inspector" && status.inspector_shown,
        )
        .on_hover_text(tr(label))
        .clicked()
        {
            match label {
                "Inspector" => chrome.action = Some(Action::ToggleInspector),
                "Search" => {
                    chrome.palette.open();
                    chrome.popup = None;
                }
                _ => popup(chrome, Popup::AllMenus, r),
            }
        }
    }
    let title_rect = slot(rect, x + 8., (right - 160. - x - 8.).max(0.));
    if title_rect.width() >= 64. {
        let title = status
            .title
            .strip_suffix(" — Heron Brush")
            .unwrap_or(&status.title);
        let label = egui::RichText::new(title).size(18.).strong();
        if dropdown(&mut bar, title_rect, label).clicked() {
            popup(chrome, Popup::Menu(0), title_rect);
        }
    }
}

pub fn shelf(ui: &mut Ui, rect: Rect, chrome: &mut Chrome, view: &mut Option<usize>) {
    ui.painter().rect_filled(rect, 0., theme::panel_layer());
    let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(12., 0.))));
    bar.set_clip_rect(rect);
    egui::ScrollArea::horizontal()
        .id_salt("ipad-editing-shelf")
        .show(&mut bar, |ui| {
            ui.set_min_size(egui::vec2(788., SHELF));
            let row = ui.max_rect();
            let mut x = row.left();
            for (label, number, width) in [
                ("3D", Some(0), 52.),
                ("Top", Some(1), 60.),
                ("Front", Some(2), 72.),
                ("Side", Some(3), 64.),
                ("All", None, 60.),
            ] {
                if button(ui, slot(row, x, width), tr(label), *view == number).clicked() {
                    *view = number;
                }
                x += width + 4.;
            }
            x += 12.;
            let tool = *chrome.tool.borrow();
            let tools = slot(row, x, 132.);
            if dropdown(ui, tools, tr(tool.name())).clicked() {
                popup(chrome, Popup::TabletTools, tools);
            }
            x += 144.;
            let grid = slot(row, x, 104.);
            let grid_label = crate::chrome::grid_label(chrome.grid.borrow().actual_size());
            if button(ui, grid, grid_label, false).clicked() {
                popup(chrome, Popup::GridSize, grid);
            }
            x += 116.;
            if button(
                ui,
                slot(row, x, 44.),
                egui::RichText::new(icon::CROSSHAIR)
                    .font(theme::icon())
                    .size(23.)
                    .color(theme::accent_text()),
                false,
            )
            .on_hover_text(tr("Frame selection"))
            .clicked()
            {
                chrome.action = Some(Action::FrameSelection);
            }
            x += 52.;
            if button(
                ui,
                slot(row, x, 44.),
                egui::RichText::new(icon::FLOPPY_DISK)
                    .font(theme::icon())
                    .size(23.)
                    .color(theme::accent_text()),
                false,
            )
            .on_hover_text(tr("Save"))
            .clicked()
            {
                chrome.action = Some(Action::Save);
            }
        });
}

pub fn tools() -> Vec<crate::menu::Entry<Action>> {
    Tool::ALL
        .into_iter()
        .map(|tool| crate::menu::Entry::item(tr(tool.name()), Action::ActivateTool(tool)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inspector_layout_follows_available_space() {
        assert!(!inspector_below(egui::vec2(1210., 700.)));
        assert!(inspector_below(egui::vec2(834., 1060.)));
        assert!(inspector_below(egui::vec2(600., 900.)));
        assert!(!inspector_below(egui::vec2(834., 350.)));
    }
}
