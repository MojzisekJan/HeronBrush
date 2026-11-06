//! Scene environment controls shared with the render camera window.
use crate::{file_dialog::FileDialog, form, theme, widgets};
use editor_core::document::Document;
use editor_core::environment;
use editor_core::i18n::tr;
use eframe::egui;
use std::cell::RefCell;
use std::rc::Rc;

pub fn show(ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
    if !cfg!(feature = "wgpu") {
        return;
    }
    let mut settings = environment::settings(&document.borrow());
    let before = settings.clone();
    ui.add_space(6.0);
    ui.label(egui::RichText::new(tr("HDR environment")).strong());
    ui.label(tr("Shared by the 3D view and rendered images"));
    let choose = theme::button(ui, tr("Choose HDR / EXR"), ui.available_width(), true).clicked();
    if let Some(path) = FileDialog::new()
        .id("hdr-environment")
        .set_title(tr("Choose HDR / EXR"))
        .add_filter("HDR / OpenEXR", &["hdr", "exr"])
        .pick_file(choose)
    {
        environment::invalidate();
        // Validate before changing the map, retaining the previous valid environment.
        match environment::load(&path) {
            Ok(_) => {
                let base = document
                    .borrow()
                    .path
                    .as_deref()
                    .and_then(std::path::Path::parent)
                    .map(std::path::Path::to_owned);
                settings.path = base
                    .and_then(|base| path.strip_prefix(base).ok().map(std::path::Path::to_owned))
                    .unwrap_or(path);
                ui.ctx()
                    .data_mut(|data| data.remove::<String>(egui::Id::unique("environment-error")));
            }
            Err(error) => {
                ui.ctx().data_mut(|data| {
                    data.insert_temp(egui::Id::unique("environment-error"), error)
                });
            }
        }
    }
    if !settings.path.as_os_str().is_empty() {
        let name = settings
            .path
            .file_name()
            .unwrap_or(settings.path.as_os_str())
            .to_string_lossy();
        ui.add(egui::Label::new(egui::RichText::new(name).color(theme::dim())).truncate())
            .on_hover_text(settings.path.to_string_lossy());
        widgets::row(ui, tr("Rotation"), None, |ui| {
            let mut value = f64::from(settings.rotation_degrees);
            widgets::spin(ui, &mut value, 0.0..=360.0, 1.0, 1);
            settings.rotation_degrees = value as f32;
        });
        widgets::row(ui, tr("Environment intensity"), None, |ui| {
            let mut value = f64::from(settings.intensity);
            widgets::spin(ui, &mut value, 0.0..=64.0, 0.1, 2);
            settings.intensity = value as f32;
        });
        form::switch_row(
            ui,
            tr("Show environment as background"),
            None,
            &mut settings.show_background,
        );
        if theme::button(ui, tr("Clear"), ui.available_width(), true).clicked() {
            settings = Default::default();
            ui.ctx()
                .data_mut(|data| data.remove::<String>(egui::Id::unique("environment-error")));
        }
        if !settings.path.as_os_str().is_empty() {
            if let Err(error) = environment::load(&environment::resolve_path(
                &document.borrow(),
                &settings.path,
            )) {
                ui.label(egui::RichText::new(error).color(theme::error()));
            }
        }
    }
    if let Some(error) = ui
        .ctx()
        .data(|data| data.get_temp::<String>(egui::Id::unique("environment-error")))
    {
        ui.label(egui::RichText::new(error).color(theme::error()));
    }
    if settings != before {
        environment::save(&mut document.borrow_mut(), &settings);
        ui.ctx().request_repaint();
    }
}

#[cfg(all(test, feature = "wgpu"))]
mod tests {
    use super::*;

    #[test]
    fn environment_controls_fit_a_narrow_inspector_with_a_long_filename() {
        let context = egui::Context::default();
        let document = Rc::new(RefCell::new(Document::new(Default::default())));
        environment::save(&mut document.borrow_mut(), &environment::Settings {
            path: "/missing/very long folder/environment panorama with a very long descriptive filename.exr".into(),
            ..Default::default()
        });
        for width in [280.0, 420.0] {
            for _ in 0..2 {
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 900.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let available = ui.available_width();
                        show(ui, &document);
                        assert!(
                            ui.min_rect().width() <= available + 1.0,
                            "HDR controls overflow a {width}-point panel: {:?}",
                            ui.min_rect()
                        );
                    },
                );
                output.textures_delta.clear();
            }
        }
    }
}
