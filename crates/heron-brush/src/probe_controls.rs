//! Scene lighting captures: a room's reflection volume and baked diffuse transport.
use crate::{form, theme, widgets};
use editor_core::{
    document::Document,
    i18n::tr,
    light_probes::{self, Probe, Status},
};
use eframe::egui;
use std::{cell::RefCell, rc::Rc};
pub fn show(
    ui: &mut egui::Ui,
    document: &Rc<RefCell<Document>>,
    camera: render::camera3d::Camera3d,
    capture_settings: &render::RenderSettings,
) {
    if !cfg!(feature = "wgpu") {
        return;
    }
    form::fold(
        ui,
        "local-light-probes",
        tr("Local light probes"),
        false,
        |ui| {
            let mut config = light_probes::settings(&document.borrow());
            let guide_id = egui::Id::unique(("probe-guides", document.borrow().generation()));
            let mut guides = ui
                .ctx()
                .data(|data| data.get_temp::<bool>(guide_id))
                .unwrap_or(true);
            form::switch_row(ui, tr("Show probe volumes"), None, &mut guides);
            ui.ctx().data_mut(|data| data.insert_temp(guide_id, guides));
            let drag_id = ui.scope_id().with("probe-number-drag");
            let mut drag_stopped = false;
            let context = ui.ctx().clone();
            let mut track = |response: egui::Response| {
                if response.drag_started() {
                    context
                        .data_mut(|data| data.insert_temp(drag_id, document.borrow().edit_mark()));
                }
                drag_stopped |= response.drag_stopped();
            };
            let before = config.clone();
            let id = ui.scope_id().with("selected-probe");
            let mut selected = ui
                .ctx()
                .data(|data| data.get_temp::<usize>(id))
                .unwrap_or(0);
            let mut capture = false;
            if theme::button(
                ui,
                tr("Add probe at 3D camera"),
                ui.available_width(),
                config.probes.len() < render::probes::Lighting::MAX,
            )
            .clicked()
            {
                let next = config.probes.iter().map(|p| p.id).max().unwrap_or(0) + 1;
                config.probes.push(Probe {
                    id: next,
                    position: [
                        camera.position.x() as f32,
                        camera.position.y() as f32,
                        camera.position.z() as f32,
                    ],
                    ..Default::default()
                });
                selected = config.probes.len() - 1;
                capture = config.lighting.is_none();
            }
            if !config.probes.is_empty() {
                selected = selected.min(config.probes.len() - 1);
                let names: Vec<String> = config
                    .probes
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("{} {}", tr("Probe"), i + 1))
                    .collect();
                let labels: Vec<&str> = names.iter().map(String::as_str).collect();
                widgets::dropdown(
                    ui,
                    "probe-list",
                    &mut selected,
                    &labels,
                    ui.available_width(),
                );
                let probe = &mut config.probes[selected];
                if theme::button(
                    ui,
                    tr("Move probe to 3D camera"),
                    ui.available_width(),
                    true,
                )
                .clicked()
                {
                    probe.position = [
                        camera.position.x() as f32,
                        camera.position.y() as f32,
                        camera.position.z() as f32,
                    ];
                }
                let bounds = document.borrow().selection_bounds();
                if theme::button(
                    ui,
                    tr("Fit probe to selection"),
                    ui.available_width(),
                    bounds.is_some(),
                )
                .clicked()
                {
                    if let Some(bounds) = bounds {
                        let center = bounds.center();
                        let size = (bounds.max - bounds.min) * 0.5;
                        probe.position = [center.x() as f32, center.y() as f32, center.z() as f32];
                        probe.half_size =
                            [size.x() as f32, size.y() as f32, size.z() as f32].map(|v| v.max(1.0));
                    }
                }
                for axis in 0..3 {
                    widgets::row(
                        ui,
                        &format!("{} {}", tr("Position"), ["X", "Y", "Z"][axis]),
                        None,
                        |ui| {
                            let mut value = f64::from(probe.position[axis]);
                            track(widgets::spin(
                                ui,
                                &mut value,
                                -1000000.0..=1000000.0,
                                16.0,
                                1,
                            ));
                            probe.position[axis] = value as f32;
                        },
                    );
                }
                for axis in 0..3 {
                    widgets::row(
                        ui,
                        &format!("{} {}", tr("Volume size"), ["X", "Y", "Z"][axis]),
                        None,
                        |ui| {
                            let mut value = f64::from(probe.half_size[axis]) * 2.0;
                            track(widgets::spin(ui, &mut value, 2.0..=200000.0, 16.0, 1));
                            probe.half_size[axis] = value as f32 * 0.5;
                        },
                    );
                }
                for (label, value, min, max, step) in [
                    (tr("Blend width"), &mut probe.blend, 0.01, 1.0, 0.05),
                    (tr("Intensity"), &mut probe.intensity, 0.0, 16.0, 0.1),
                ] {
                    widgets::row(ui, label, None, |ui| {
                        let mut number = f64::from(*value);
                        track(widgets::spin(ui, &mut number, min..=max, step, 2));
                        *value = number as f32;
                    });
                }
                form::switch_row(ui, tr("Local reflections"), None, &mut probe.reflections);
                form::switch_row(
                    ui,
                    tr("Indirect diffuse lighting"),
                    None,
                    &mut probe.diffuse,
                );
                let mut quality = match config.resolution {
                    16 => 0,
                    64 => 2,
                    _ => 1,
                };
                widgets::row(ui, tr("Capture quality"), None, |ui| {
                    if widgets::dropdown(
                        ui,
                        "probe-quality",
                        &mut quality,
                        &[tr("Draft"), tr("Balanced"), tr("High")],
                        ui.available_width(),
                    ) {
                        config.resolution = [16, 32, 64][quality];
                    }
                });
                if theme::button(ui, tr("Remove probe"), ui.available_width(), true).clicked() {
                    config.probes.remove(selected);
                    selected = selected.saturating_sub(1);
                }
                if theme::button(ui, tr("Bake scene lighting"), ui.available_width(), true)
                    .clicked()
                {
                    capture = true;
                    light_probes::request_bake();
                }
            }
            if capture {
                let settings = capture_settings;
                config.lighting = Some(editor_core::render_camera::Lighting::from_render(settings));
                config.sun_elevation = settings.sun_elevation;
                config.sun_azimuth = settings.sun_azimuth;
                config.shadows = settings.shadows;
            }
            ui.ctx().data_mut(|data| data.insert_temp(id, selected));
            if config != before {
                light_probes::save(&mut document.borrow_mut(), &config);
                ui.ctx().request_repaint();
            }
            if drag_stopped {
                if let Some(mark) = ui.ctx().data_mut(|data| data.remove_temp::<usize>(drag_id)) {
                    document.borrow_mut().fold_edits_since(mark);
                }
            }
            match light_probes::status(&document.borrow()) {
                Status::Disabled => {
                    ui.label(tr(
                        "Place a probe inside a room and fit its volume to the room",
                    ));
                }
                Status::Waiting => {
                    ui.label(tr("Waiting to bake light probes"));
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(100));
                }
                Status::Baking(done, total) => {
                    ui.label(format!("{} {done}/{total}", tr("Baking light probes")));
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(100));
                }
                Status::Ready => {
                    ui.label(
                        egui::RichText::new(tr("Probe lighting ready")).color(theme::success()),
                    );
                }
                Status::Failed(error) => {
                    ui.label(egui::RichText::new(error).color(theme::error()));
                }
            }
        },
    );
}

/// Editor-only influence boxes. Still renderers never receive these guide lines.
pub fn guides(
    ctx: &egui::Context,
    document: &Document,
) -> (u64, Vec<(geometry_math::vec::Vec3d, [f32; 4])>) {
    use geometry_math::vec::Vec3d;
    use std::hash::{Hash, Hasher};
    let id = egui::Id::unique(("probe-guides", document.generation()));
    if !ctx.data(|data| data.get_temp::<bool>(id)).unwrap_or(true) {
        return (0, Vec::new());
    }
    let config = light_probes::settings(document);
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    format!("{config:?}").hash(&mut hash);
    let mut lines = Vec::new();
    for probe in &config.probes {
        let center = Vec3d::new(
            probe.position[0] as f64,
            probe.position[1] as f64,
            probe.position[2] as f64,
        );
        let extent = Vec3d::new(
            probe.half_size[0] as f64,
            probe.half_size[1] as f64,
            probe.half_size[2] as f64,
        );
        let corners: [Vec3d; 8] = std::array::from_fn(|i| {
            center
                + Vec3d::new(
                    if i & 1 == 0 { -extent.x() } else { extent.x() },
                    if i & 2 == 0 { -extent.y() } else { extent.y() },
                    if i & 4 == 0 { -extent.z() } else { extent.z() },
                )
        });
        for i in 0..8 {
            for axis in 0..3 {
                let j = i ^ (1 << axis);
                if i < j {
                    lines.push((corners[i], [0.95, 0.65, 0.2, 1.0]));
                    lines.push((corners[j], [0.95, 0.65, 0.2, 1.0]));
                }
            }
        }
        for axis in 0..3 {
            let mut delta = Vec3d::zero();
            delta[axis] = 8.0;
            lines.push((center - delta, [1.0, 0.8, 0.3, 1.0]));
            lines.push((center + delta, [1.0, 0.8, 0.3, 1.0]));
        }
    }
    (hash.finish(), lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn probe_controls_fit_a_narrow_panel_and_guides_do_not_modify_the_document() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let document = Rc::new(RefCell::new(Document::new(Default::default())));
        let config = light_probes::Settings {
            probes: vec![Probe::default()],
            ..Default::default()
        };
        light_probes::save(&mut document.borrow_mut(), &config);
        let before = document.borrow().to_map_text();
        for width in [320.0, 440.0] {
            for _ in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 1600.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let fold = ui.scope_id().with(("fold", "local-light-probes"));
                        ui.ctx().data_mut(|data| data.insert_temp(fold, true));
                        let available = ui.available_width();
                        show(
                            ui,
                            &document,
                            render::camera3d::Camera3d::default(),
                            &editor_core::render_settings::from_preferences(),
                        );
                        assert!(
                            ui.min_rect().width() <= available + 1.0,
                            "light probe controls overflow: {:?}",
                            ui.min_rect()
                        );
                    },
                );
                output.textures_delta.clear();
            }
        }
        let (_, lines) = guides(&ctx, &document.borrow());
        assert_eq!(lines.len(), 30);
        assert_eq!(document.borrow().to_map_text(), before);
    }
}
