//! Face geometry controls backed by the shared document operations.

use crate::widgets::{self, icons};
use editor_core::document::{Document, FaceRef};
use editor_core::handle_tool::HandleManager;
use editor_core::i18n::{counted, fill, tr, translation_key};
use eframe::egui;

/// Draws the selected patches' grid controls.
pub fn patch(ui: &mut egui::Ui, document: &mut Document) {
    let patches = document.selected_patches();
    let mut enabled = [false; 4];
    if let Some(first) = patches.first() {
        let mut status = fill(
            "{} × {} control points",
            &[&first.columns.to_string(), &first.rows.to_string()],
        );
        if patches.len() > 1 {
            status.push_str(&fill(
                " (+{} more selected)",
                &[&(patches.len() - 1).to_string()],
            ));
        }
        widgets::status(ui, &status);
        enabled = [
            map_model::patch_edit::can_remove_columns(first),
            map_model::patch_edit::can_add_columns(first),
            map_model::patch_edit::can_remove_rows(first),
            map_model::patch_edit::can_add_rows(first),
        ];
    } else {
        widgets::status(ui, tr("Select a patch to reshape its grid"));
    }
    drop(patches);
    type Resize = fn(&map_model::io::map_reader::Patch) -> Option<map_model::io::map_reader::Patch>;
    let operations: [Resize; 4] = [
        map_model::patch_edit::remove_columns,
        map_model::patch_edit::add_columns,
        map_model::patch_edit::remove_rows,
        map_model::patch_edit::add_rows,
    ];
    for (row, label) in [translation_key("Columns"), translation_key("Rows")]
        .into_iter()
        .enumerate()
    {
        ui.push_id(label, |ui| {
            widgets::row(ui, tr(label), None, |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                type Icon = fn(&egui::Painter, egui::Rect, egui::Color32);
                let buttons: [(Icon, &str); 2] = [
                    (icons::minus, translation_key("Drop pair")),
                    (icons::plus, translation_key("Add pair")),
                ];
                for (column, (icon, tip)) in buttons.into_iter().enumerate() {
                    let index = row * 2 + column;
                    if widgets::icon_button(
                        ui,
                        widgets::ROW_HEIGHT,
                        tr(tip),
                        enabled[index],
                        false,
                        icon,
                    )
                    .clicked()
                    {
                        document.resize_selected_patches(operations[index]);
                    }
                }
            });
        });
    }
}

/// The inset and bevel panel's values and most recent operation result.
pub struct BevelPage {
    pub corner_requested: bool,
    inset: f64,
    hollow_thickness: f64,
    hollow_direction: usize,
    hollow_open: bool,
    hollow_join: bool,
    connected_region: bool,
    depth: f64,
    result: Option<String>,
    targets: Vec<FaceRef>,
}

impl Default for BevelPage {
    fn default() -> Self {
        Self {
            corner_requested: false,
            hollow_thickness: 8.0,
            hollow_direction: 0,
            hollow_open: false,
            hollow_join: false,
            connected_region: true,
            inset: 8.0,
            depth: 0.0,
            result: None,
            targets: Vec::new(),
        }
    }
}

impl BevelPage {
    /// Draws controls for faces selected directly or through face-tool handles.
    pub fn show(&mut self, ui: &mut egui::Ui, document: &mut Document, handles: &HandleManager) {
        let mut faces = document.faces_at(&handles.selected_face_centres());
        if faces.is_empty() {
            faces = document.selected_faces();
        }
        if self.targets != faces {
            self.result = None;
            self.targets = faces.clone();
        }
        let status = self.result.clone().unwrap_or_else(|| {
            if faces.is_empty() {
                tr("Select faces to inset, bevel or panel them").to_owned()
            } else {
                fill("{} selected", &[&counted(faces.len(), "face")])
            }
        });
        widgets::status(ui, &status);
        let tip = tr("How far the face is drawn in from its own edges");
        if widgets::row(ui, tr("Inset"), Some(tip), |ui| {
            widgets::spin(ui, &mut self.inset, 0.0..=1024.0, 1.0, 0)
                .on_hover_text(tip)
                .changed()
        }) {
            self.result = None;
        }
        let tip =
            tr("How far the face is pushed out along its normal; a negative depth pulls it in");
        if widgets::row(ui, tr("Depth"), Some(tip), |ui| {
            widgets::spin(ui, &mut self.depth, -1024.0..=1024.0, 1.0, 0)
                .on_hover_text(tip)
                .changed()
        }) {
            self.result = None;
        }
        if crate::widgets::checkbox(ui, &mut self.connected_region, tr("Connected face region"))
            .changed()
        {
            self.result = None;
        }
        let label = if self.depth < 0.0 {
            translation_key("Cut a recess")
        } else {
            translation_key("Add panel")
        };
        let actions = [
            (
                tr("Bevel faces"),
                "",
                !faces.is_empty() && (self.inset != 0.0 || self.depth != 0.0),
            ),
            (tr("Join corner"), "", true),
            (tr(label), "", !faces.is_empty() && self.depth != 0.0),
        ];
        match widgets::segmented(ui, "bevel-actions", &actions) {
            Some(0) => {
                match document.bevel_faces_options(
                    &faces,
                    self.inset,
                    self.depth,
                    self.connected_region,
                ) {
                    Ok(done) => self.report(done, translation_key("No face could take that inset")),
                    Err(error) => self.result = Some(error),
                }
            }
            Some(1) => self.corner_requested = true,
            Some(_) => {
                let done = document.panel_on_faces(&faces, self.inset, self.depth);
                self.report(done, translation_key("Nothing could be stood on that face"));
            }
            None => {}
        }
        ui.separator();
        let mut hollow_changed = widgets::row(ui, tr("Wall thickness"), None, |ui| {
            widgets::number_buttons(ui, &mut self.hollow_thickness, 0.01..=1024.0)
        });
        let modes = [tr("Inward"), tr("Outward"), tr("Centered")];
        hollow_changed |= widgets::dropdown(
            ui,
            "hollow-direction",
            &mut self.hollow_direction,
            &modes,
            140.0,
        );
        if crate::widgets::checkbox(ui, &mut self.hollow_open, tr("Open selected faces")).changed()
        {
            hollow_changed = true;
            if self.hollow_open {
                self.hollow_join = false;
            }
        }
        if crate::widgets::checkbox(ui, &mut self.hollow_join, tr("Join convex pair")).changed() {
            hollow_changed = true;
            if self.hollow_join {
                self.hollow_open = false;
            }
        }
        if hollow_changed {
            self.result = None;
        }
        if ui
            .add_enabled(
                !self.hollow_open || !faces.is_empty(),
                egui::Button::new(tr("Hollow selection")),
            )
            .clicked()
        {
            let direction = [
                map_model::hollow::Direction::Inward,
                map_model::hollow::Direction::Outward,
                map_model::hollow::Direction::Centered,
            ][self.hollow_direction];
            match document.hollow_selection_options(
                self.hollow_thickness,
                direction,
                if self.hollow_open { &faces } else { &[] },
                self.hollow_join,
            ) {
                Ok(done) => self.report(done, translation_key("Nothing to hollow")),
                Err(error) => self.result = Some(error),
            }
        }
    }

    fn report(&mut self, done: usize, nothing: &'static str) {
        self.result = Some(if done == 0 {
            tr(nothing).to_owned()
        } else {
            fill("{} done", &[&counted(done, "face")])
        });
    }
}
