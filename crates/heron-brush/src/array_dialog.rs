//! The Array window: copies of the selection in a line, a grid, a ring or a scatter,
//! shown in the map as the values change, then kept with Apply or taken away with
//! Cancel. Closing the window keeps them, as the shape settings window does.
use crate::{dialog, editor::Editor, form, theme, widgets};
use editor_core::document::{ArrayKind, ArrayParameters};
use editor_core::i18n::{fill, tr, translation_key};
use eframe::egui;
use geometry_math::util::Axis;
use geometry_math::vec::Vec3d;

/// The layouts' names, in [`ArrayKind::ALL`]'s order: the label and its tooltip.
const KINDS: [(&str, &str); 4] = [
    (translation_key("Row"), translation_key("Copies one after another, each moved and turned a step further")),
    (translation_key("Grid"), translation_key("Copies along X, Y and Z")),
    (translation_key("Ring"), translation_key("Copies round a centre, evenly over an angle")),
    (translation_key("Scatter"), translation_key("Copies at random places within a radius")),
];

pub struct ArrayDialog {
    pub open: bool,
    parameters: ArrayParameters,
    /// What the last change did: how many copies, or why there are none.
    status: String,
    failed: bool,
}

impl Default for ArrayDialog {
    fn default() -> Self {
        Self { open: false, parameters: ArrayParameters::default(), status: String::new(), failed: false }
    }
}

impl ArrayDialog {
    /// Opens the window and previews an array of the selection with the last values.
    pub fn open(&mut self, editor: &Editor) {
        self.open = true;
        if !editor.document.borrow().has_array_preview() {
            self.begin(editor);
        }
    }

    fn begin(&mut self, editor: &Editor) {
        let result = editor.document.borrow_mut().begin_array_preview(self.parameters);
        self.report(result);
    }

    fn report(&mut self, result: Result<usize, &'static str>) {
        self.failed = result.is_err();
        self.status = match result {
            Ok(copies) => fill("Copies: {}", &[&copies.to_string()]),
            Err(error) => tr(error).to_string(),
        };
    }

    pub fn show(&mut self, ctx: &egui::Context, editor: &Editor) {
        if !self.open {
            return;
        }
        let before = self.parameters;
        let previewing = editor.document.borrow().has_array_preview();
        let linkable = editor.document.borrow().can_link_array();
        let (mut apply, mut cancel, mut make) = (false, false, false);
        let open = dialog::show_tool(ctx, "array", tr("Array"), [440.0, 520.0], |ui, part| {
            if part == dialog::Part::Actions {
                let width = ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).min(120.0);
                if previewing {
                    cancel |= theme::button(ui, tr("Cancel"), width, true).clicked();
                    apply |= form::accent_button_width(ui, tr("Apply"), width, true).clicked();
                } else {
                    make |= form::accent_button_width(ui, tr("Make array"), width * 2.0, true).clicked();
                }
                return;
            }
            egui::ScrollArea::vertical().id_salt("array-controls").auto_shrink([false, false]).show(ui, |ui| {
                self.controls(ui, linkable);
                ui.add_space(6.0);
                let colour = if self.failed { theme::error() } else { theme::dim() };
                ui.label(egui::RichText::new(&self.status).color(colour));
                if !previewing && !self.failed {
                    ui.label(egui::RichText::new(tr("Select what to copy, then press Make array.")).color(theme::dim()));
                }
            });
        });
        if self.parameters != before && previewing {
            let result = editor.document.borrow_mut().update_array_preview(self.parameters);
            self.report(result);
        }
        if make {
            self.begin(editor);
        }
        if cancel {
            editor.document.borrow_mut().cancel_array_preview();
            self.status.clear();
        }
        // Apply, the window's own close, or Escape keep the copies.
        if apply || !open {
            editor.document.borrow_mut().finish_array_preview();
            self.status.clear();
        }
        self.open = open && !apply && !cancel;
        if self.parameters != before || apply || cancel || make || !self.open {
            ctx.request_repaint_of(egui::ViewportId::ROOT);
            ctx.request_repaint();
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui, linkable: bool) {
        let p = &mut self.parameters;
        ui.horizontal(|ui| {
            let mut kind = ArrayKind::ALL.iter().position(|k| *k == p.kind).unwrap_or(0);
            let items: Vec<(&str, &str)> = KINDS.iter().map(|(name, tip)| (tr(name), tr(tip))).collect();
            if widgets::segmented_choice(ui, "array-kind", &items, &mut kind, ui.available_width()) {
                p.kind = ArrayKind::ALL[kind];
            }
        });
        ui.add_space(4.0);
        let count = |ui: &mut egui::Ui, label: &'static str, value: &mut u32| {
            widgets::row(ui, tr(label), None, |ui| widgets::number_buttons(ui, value, 1.0..=1025.0));
        };
        let number = |ui: &mut egui::Ui, label: &'static str, value: &mut f64, range: std::ops::RangeInclusive<f64>, step: f64| {
            widgets::row(ui, tr(label), None, |ui| {
                let width = ui.available_width().min(150.0);
                widgets::spin_width(ui, value, range, step, 1, width);
            });
        };
        let step = |ui: &mut egui::Ui, step: &mut Vec3d| {
            let mut v = [step.x(), step.y(), step.z()];
            for (label, value) in [translation_key("Step X"), translation_key("Step Y"), translation_key("Step Z")].into_iter().zip(&mut v) {
                number(ui, label, value, -65536.0..=65536.0, 8.0);
            }
            *step = Vec3d::new(v[0], v[1], v[2]);
        };
        let axis = |ui: &mut egui::Ui, axis: &mut Axis| {
            widgets::row(ui, tr("Axis:"), None, |ui| {
                let mut index = axis.index();
                if widgets::segmented_choice(ui, "array-axis", &[("X", ""), ("Y", ""), ("Z", "")], &mut index, ui.available_width().min(200.0)) {
                    *axis = [Axis::X, Axis::Y, Axis::Z][index];
                }
            });
        };
        match p.kind {
            ArrayKind::Line => {
                count(ui, translation_key("Count"), &mut p.count[0]);
                step(ui, &mut p.step);
                number(ui, translation_key("Turn per copy (degrees)"), &mut p.turn, -360.0..=360.0, 15.0);
                axis(ui, &mut p.axis);
            }
            ArrayKind::Grid => {
                count(ui, translation_key("Count X"), &mut p.count[0]);
                count(ui, translation_key("Count Y"), &mut p.count[1]);
                count(ui, translation_key("Count Z"), &mut p.count[2]);
                step(ui, &mut p.step);
            }
            ArrayKind::Radial => {
                count(ui, translation_key("Count"), &mut p.count[0]);
                axis(ui, &mut p.axis);
                number(ui, translation_key("Radius (units)"), &mut p.radius, 0.0..=65536.0, 16.0);
                number(ui, translation_key("Angle (degrees)"), &mut p.angle, -360.0..=360.0, 15.0);
                form::switch_row(ui, tr("Face the centre"), None, &mut p.rotate);
            }
            ArrayKind::Scatter => {
                count(ui, translation_key("Count"), &mut p.count[0]);
                axis(ui, &mut p.axis);
                number(ui, translation_key("Radius (units)"), &mut p.radius, 1.0..=65536.0, 16.0);
                form::switch_row(ui, tr("Random turn"), None, &mut p.rotate);
                widgets::row(ui, tr("Seed"), None, |ui| widgets::number_buttons(ui, &mut p.seed, 0.0..=99999.0));
            }
        }
        ui.add_space(4.0);
        ui.add_enabled_ui(linkable, |ui| {
            form::switch_row(ui, tr("Linked copies"), Some(tr("Each copy is a linked duplicate of the selected group, so an edit to one reaches them all. Select one whole group to use it.")), &mut p.linked);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Opening previews the copies, a change makes them again, Cancel takes them away and
    /// Apply keeps them.
    #[test]
    fn the_window_previews_applies_and_cancels() {
        let ctx = egui::Context::default();
        let editor = Editor::open(&ctx, None);
        let bounds = geometry_math::bbox::Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(32.0, 32.0, 32.0));
        let cube = map_model::shapes::create_cuboid(&editor.document.borrow().world_bounds(), &bounds, map_model::shapes::DEFAULT_MATERIAL).unwrap();
        editor.document.borrow_mut().add_brushes(vec![cube]);
        // The new map has brushes of its own; only the cube is selected.
        let brushes = |editor: &Editor| map_model::node::brushes_by_layer(editor.document.borrow().tree_world()).len();
        let start = brushes(&editor) - 1;
        let mut dialog = ArrayDialog::default();
        dialog.open(&editor);
        assert_eq!(brushes(&editor), start + 4);
        dialog.parameters.count[0] = 6;
        let result = editor.document.borrow_mut().update_array_preview(dialog.parameters);
        dialog.report(result);
        assert_eq!(brushes(&editor), start + 6);
        assert_eq!(dialog.status, "Copies: 5");
        assert!(editor.document.borrow_mut().cancel_array_preview());
        assert_eq!(brushes(&editor), start + 1);
        dialog.open(&editor);
        assert!(editor.document.borrow_mut().finish_array_preview());
        assert_eq!(brushes(&editor), start + 6);
    }
}
