//! Camera-local output color controls.
use crate::{theme, widgets};
use editor_core::{i18n::tr, preferences::ToneMapper, render_camera::Imaging};
use eframe::egui;
pub fn show(ui: &mut egui::Ui, imaging: &mut Imaging) {
    for (label, value, low, high, step, decimals) in [
        (
            tr("White balance (K)"),
            &mut imaging.temperature,
            2000.0,
            12000.0,
            100.0,
            0,
        ),
        (tr("Tint"), &mut imaging.tint, -1.0, 1.0, 0.01, 2),
        (tr("Contrast"), &mut imaging.contrast, 0.25, 2.0, 0.05, 2),
        (tr("Saturation"), &mut imaging.saturation, 0.0, 2.0, 0.05, 2),
    ] {
        widgets::row(ui, label, None, |ui| {
            let mut number = f64::from(*value);
            widgets::spin(ui, &mut number, low..=high, step, decimals);
            *value = number as f32;
        });
    }
    let mut choices = vec![tr("Editor setting")];
    choices.extend(ToneMapper::ALL.iter().map(|map| map.name()));
    let mut selected = imaging
        .tone_map
        .and_then(|map| ToneMapper::ALL.iter().position(|value| *value == map))
        .map_or(0, |index| index + 1);
    widgets::row(ui, tr("Tone mapping"), None, |ui| {
        if widgets::dropdown(
            ui,
            "camera-tone-map",
            &mut selected,
            &choices,
            ui.available_width(),
        ) {
            imaging.tone_map = selected.checked_sub(1).map(|i| ToneMapper::ALL[i]);
        }
    });
    if theme::button(ui, tr("Reset color management"), ui.available_width(), true).clicked() {
        *imaging = Imaging::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_controls_fit_narrow_and_wide_panels() {
        let context = egui::Context::default();
        let mut settings = Imaging::default();
        for width in [280.0, 420.0] {
            for _ in 0..2 {
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 800.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let available = ui.available_width();
                        show(ui, &mut settings);
                        assert!(
                            ui.min_rect().width() <= available + 1.0,
                            "color controls overflow: {:?}",
                            ui.min_rect()
                        );
                    },
                );
                output.textures_delta.clear();
            }
        }
    }
}
