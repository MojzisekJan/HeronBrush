//! Compact toolbar controls and the shape settings window's vertical form.

use std::cell::RefCell;
use std::rc::Rc;

use editor_core::fillet_tool::FilletParameters;
use editor_core::i18n::{tr, translation_key};
use editor_core::shape_tool::{CircleMode, ShapeKind, ShapeParameters, StairDirection};
use eframe::egui;

use crate::{theme, widgets};

/// How the precision of a scalable circle reads in the chooser: the vertex count it gives.
const PRECISION_LABELS: [&str; 4] = ["12", "24", "48", "96"];

/// The fillet's corner modes, and what the chooser calls them.
pub const CORNERS: [(map_model::fillet::Corners, &str); 4] = [
    (
        map_model::fillet::Corners::Mitred,
        translation_key("Mitred"),
    ),
    (
        map_model::fillet::Corners::Chamfered,
        translation_key("Chamfered"),
    ),
    (
        map_model::fillet::Corners::Rounded,
        translation_key("Rounded"),
    ),
    (
        map_model::fillet::Corners::Setback,
        translation_key("Setback"),
    ),
];

/// What the corner modes do, for the chooser's tooltip.
pub fn corners_tip() -> String {
    format!(
        "{}\n{}",
        tr("A mitre lets the edges' own planes cut each other; rounding lays a ball into the corner as well"),
        tr("A chamfer cuts the corner off flat; a setback lays a larger ball into it, starting further back"),
    )
}

/// The compact toolbar keeps shape configuration in its own window.
pub fn shape_row(
    ui: &mut egui::Ui,
    parameters: &Rc<RefCell<ShapeParameters>>,
) -> Option<crate::app_actions::Action> {
    use crate::app_actions::Action;
    let mut request = None;
    let mut settings = parameters.borrow_mut();
    let names: Vec<_> = ShapeKind::ALL
        .iter()
        .map(|kind| tr(kind.name()))
        .chain([tr("Insert 3D text")])
        .collect();
    let mut index = ShapeKind::ALL
        .iter()
        .position(|kind| *kind == settings.kind)
        .unwrap_or(0);
    let compact = ui.available_width() < 250.0;
    let button_width = if compact {
        28.0
    } else {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(tr("Shape settings").to_string(), theme::body(), theme::fg())
                .size()
                .x
        }) + 20.0
    };
    let picker_width = (ui.available_width() - button_width - 8.0).clamp(60.0, 168.0);
    if widgets::dropdown(ui, "shape-kind", &mut index, &names, picker_width) {
        if let Some(kind) = ShapeKind::ALL.get(index) {
            settings.kind = *kind;
            request = Some(Action::ShapeSettings);
        } else {
            request = Some(Action::Text);
        }
    }
    if ui
        .button(if compact { "…" } else { tr("Shape settings") })
        .on_hover_text(tr("Shape settings"))
        .clicked()
    {
        request = Some(Action::ShapeSettings);
    }
    request
}

/// All generators share a vertical form, including numeric step buttons.
pub fn shape_controls(ui: &mut egui::Ui, settings: &mut ShapeParameters) {
    use geometry_math::util::Axis;
    let mut index = ShapeKind::ALL
        .iter()
        .position(|kind| *kind == settings.kind)
        .unwrap_or(0);
    let names: Vec<_> = ShapeKind::ALL.iter().map(|kind| tr(kind.name())).collect();
    widgets::row(ui, tr("Shape"), None, |ui| {
        if widgets::dropdown(
            ui,
            "shape-window-kind",
            &mut index,
            &names,
            ui.available_width(),
        ) {
            settings.kind = ShapeKind::ALL[index];
        }
    });
    let kind = settings.kind;
    if kind.has_axis() && kind != ShapeKind::Stairs {
        widgets::row(ui, tr("Axis:"), None, |ui| {
            let mut axis = settings.axis.index();
            if widgets::segmented_choice(
                ui,
                "shape-axis",
                &[("X", ""), ("Y", ""), ("Z", "")],
                &mut axis,
                ui.available_width().min(200.0),
            ) {
                settings.axis = [Axis::X, Axis::Y, Axis::Z][axis];
            }
        });
    }
    if kind.is_circular() {
        let modes: Vec<_> = CircleMode::ALL
            .iter()
            .map(|mode| match mode {
                CircleMode::EdgeAligned => (tr("Edge"), tr("Edge aligned")),
                CircleMode::VertexAligned => (tr("Vertex"), tr("Vertex aligned")),
                CircleMode::Scalable => (tr("Scalable"), tr("Scalable")),
            })
            .collect();
        // An entire row leaves room for translated mode names without horizontal scrolling.
        ui.horizontal(|ui| {
            let mut mode = CircleMode::ALL
                .iter()
                .position(|m| *m == settings.circle_mode)
                .unwrap_or(0);
            if widgets::segmented_choice(ui, "circle-mode", &modes, &mut mode, ui.available_width())
            {
                settings.circle_mode = CircleMode::ALL[mode];
            }
        });
        widgets::row(ui, tr("Sides:"), None, |ui| {
            if settings.circle_mode == CircleMode::Scalable {
                let mut precision = settings.precision.min(PRECISION_LABELS.len() - 1);
                if widgets::dropdown(
                    ui,
                    "shape-precision",
                    &mut precision,
                    &PRECISION_LABELS,
                    ui.available_width(),
                ) {
                    settings.precision = precision;
                }
            } else {
                widgets::number_buttons(ui, &mut settings.num_sides, 3.0..=96.0);
            }
        });
    }
    if kind == ShapeKind::Cylinder {
        widgets::row(ui, tr("Hollow"), None, |ui| {
            widgets::switch(ui, &mut settings.hollow)
        });
    }
    if kind == ShapeKind::Arch
        || kind == ShapeKind::HollowSphere
        || (kind == ShapeKind::Cylinder && settings.hollow)
    {
        widgets::row(ui, tr("Thickness:"), None, |ui| {
            if kind == ShapeKind::HollowSphere {
                widgets::number_buttons(ui, &mut settings.sphere_thickness, 1.0..=1024.0);
            } else {
                widgets::number_buttons(ui, &mut settings.thickness, 1.0..=1024.0);
            }
        });
    }
    if kind == ShapeKind::Arch {
        widgets::row(ui, tr("Spandrel"), None, |ui| {
            widgets::switch(ui, &mut settings.create_spandrel)
        });
    }
    if kind == ShapeKind::UvSphere && settings.circle_mode != CircleMode::Scalable {
        widgets::row(ui, tr("Rings:"), None, |ui| {
            widgets::number_buttons(ui, &mut settings.num_rings, 1.0..=256.0)
        });
    }
    if kind == ShapeKind::IcoSphere {
        widgets::row(ui, tr("Accuracy:"), None, |ui| {
            widgets::number_buttons(ui, &mut settings.accuracy, 1.0..=4.0)
        });
    }
    if kind == ShapeKind::HollowSphere {
        widgets::row(ui, tr("Brushes"), None, |ui| {
            if widgets::number_buttons_step(ui, &mut settings.sphere_brushes, 20.0..=320.0, 4.0) {
                settings.sphere_brushes =
                    ((settings.sphere_brushes.saturating_add(2) / 4) * 4).clamp(20, 320);
            }
        });
    }
    if kind == ShapeKind::Stairs {
        use map_model::shapes::StairLayout;
        widgets::row(ui, tr("Step height:"), None, |ui| {
            widgets::number_buttons(ui, &mut settings.step_height, 1.0..=1024.0)
        });
        let layouts = [
            StairLayout::Straight,
            StairLayout::Left,
            StairLayout::Right,
            StairLayout::Return,
        ];
        let names = [
            tr("Straight"),
            tr("Turn left"),
            tr("Turn right"),
            tr("Return stairs"),
        ];
        widgets::row(ui, tr("Stairs"), None, |ui| {
            let mut index = layouts
                .iter()
                .position(|m| *m == settings.stairs.layout)
                .unwrap_or(0);
            if widgets::dropdown(
                ui,
                "stairs-layout",
                &mut index,
                &names,
                ui.available_width(),
            ) {
                settings.stairs.layout = layouts[index];
            }
        });
        if settings.stairs.layout == StairLayout::Straight {
            widgets::row(ui, tr("Landing"), None, |ui| {
                widgets::number_buttons(ui, &mut settings.stairs.landing, 0.0..=4096.0)
            });
        } else {
            widgets::row(ui, tr("Flight width"), None, |ui| {
                widgets::number_buttons(ui, &mut settings.stairs.width, 1.0..=4096.0)
            });
        }
        widgets::row(ui, tr("Slab thickness"), None, |ui| {
            widgets::number_buttons(ui, &mut settings.stairs.slab, 0.0..=1024.0)
        });
        widgets::row(ui, tr("Direction:"), None, |ui| {
            let mut direction = StairDirection::ALL
                .iter()
                .position(|d| *d == settings.stair_direction)
                .unwrap_or(0);
            let names: Vec<_> = StairDirection::ALL.iter().map(|d| tr(d.name())).collect();
            if widgets::dropdown(
                ui,
                "shape-stairs",
                &mut direction,
                &names,
                ui.available_width(),
            ) {
                settings.stair_direction = StairDirection::ALL[direction];
            }
        });
    }
}

/// What the fillet's strip in the tool bar asked for this frame.
#[derive(Default)]
pub struct FilletBar {
    /// The settings window's button was clicked.
    pub settings: bool,
    /// Apply was clicked.
    pub apply: bool,
}

/// The fillet's strip in the tool bar: a button for its settings window (`open` while the
/// window shows) and Apply. The settings themselves are in [`fillet_controls`].
pub fn fillet_bar(ui: &mut egui::Ui, open: bool) -> FilletBar {
    let settings = widgets::icon_label_button(
        ui,
        egui_phosphor::regular::SLIDERS_HORIZONTAL,
        tr("Fillet"),
        open,
    )
    .clicked();
    // The other way to apply it: the tool's own press rounds the edge it lands on, and
    // this rounds whatever is picked.
    let apply = ui
        .button(tr("Apply"))
        .on_hover_text(tr("Round what is picked (Return)"))
        .clicked();
    FilletBar { settings, apply }
}

/// The fillet's settings for its settings window, as a form: the labels right-aligned in
/// a column of their own, every control the same width beside them, the check box under
/// the controls.
pub fn fillet_controls(ui: &mut egui::Ui, parameters: &Rc<RefCell<FilletParameters>>) {
    let mut settings = parameters.borrow_mut();
    let labels = [tr("Radius:"), tr("Faces:"), tr("Corners:")];
    let label_width = labels
        .iter()
        .map(|l| crate::chrome::text_width(ui, l, theme::body()))
        .fold(0.0, f32::max);
    let gap = 12.0;
    let control = (ui.available_width() - label_width - gap).max(140.0);
    let row =
        |ui: &mut egui::Ui, label: &str, tip: Option<&str>, add: &mut dyn FnMut(&mut egui::Ui)| {
            ui.horizontal(|ui| {
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(label_width, widgets::ROW_HEIGHT),
                    egui::Sense::hover(),
                );
                if let Some(tip) = tip {
                    response.on_hover_text(tip);
                }
                theme::text(
                    ui.painter(),
                    egui::pos2(rect.right(), rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    label,
                    theme::body(),
                    theme::dim(),
                );
                ui.add_space(gap - ui.spacing().item_spacing.x);
                add(ui);
            });
        };
    row(ui, labels[0], None, &mut |ui| {
        ui.add(
            egui_native_look::NumberField::new(&mut settings.radius)
                .range(0.01..=1024.0)
                .width(control),
        );
    });
    row(ui, labels[1], None, &mut |ui| {
        ui.add(
            egui_native_look::NumberField::new(&mut settings.faces)
                .range(1.0..=64.0)
                .width(control),
        );
    });
    let mut corners = CORNERS
        .iter()
        .position(|(corners, _)| *corners == settings.corners)
        .unwrap_or(0);
    let names: Vec<&str> = CORNERS.iter().map(|(_, name)| tr(name)).collect();
    let tip = corners_tip();
    row(ui, labels[2], Some(&tip), &mut |ui| {
        if widgets::dropdown(ui, "fillet-corners", &mut corners, &names, control) {
            ui.ctx().request_repaint();
        }
    });
    settings.corners = CORNERS[corners].0;
    ui.horizontal(|ui| {
        ui.add_space(label_width + gap - ui.spacing().item_spacing.x);
        crate::widgets::checkbox(ui, &mut settings.inner, tr("Inner L corner"));
    });
}

/// The sculpt brush's strip in the tool bar: what a dab does, its radius and strength,
/// and the button for the settings window. Returns whether that button was clicked.
pub fn sculpt_row(ui: &mut egui::Ui, open: bool) -> bool {
    use editor_core::sculpt::{self, SculptMode, SculptParameters};
    use egui_phosphor::regular as icon;
    let mut parameters = sculpt::parameters();
    let modes: Vec<(&str, &str)> = SculptMode::ALL
        .iter()
        .map(|mode| (tr(mode.name()), ""))
        .collect();
    let mut mode = SculptMode::ALL
        .iter()
        .position(|m| *m == parameters.mode)
        .unwrap_or(0);
    if widgets::segmented_choice(ui, "sculpt-mode", &modes, &mut mode, 240.0) {
        parameters.mode = SculptMode::ALL[mode];
    }
    ui.label(
        egui::RichText::new(icon::CIRCLE_DASHED)
            .font(theme::icon())
            .color(theme::dim()),
    )
    .on_hover_text(tr("Radius (units)"));
    widgets::spin_width(
        ui,
        &mut parameters.radius,
        SculptParameters::RADIUS,
        8.0,
        0,
        60.0,
    )
    .on_hover_text(tr("Radius (units)"));
    ui.label(
        egui::RichText::new(icon::GAUGE)
            .font(theme::icon())
            .color(theme::dim()),
    )
    .on_hover_text(tr("Strength (%)"));
    let mut percent = parameters.strength * 100.0;
    if widgets::spin_width(ui, &mut percent, 1.0..=100.0, 5.0, 0, 52.0)
        .on_hover_text(tr("Strength (%)"))
        .changed()
    {
        parameters.strength = percent / 100.0;
    }
    sculpt::set_parameters(parameters);
    widgets::icon_label_button(ui, icon::SLIDERS_HORIZONTAL, "", open)
        .on_hover_text(tr("Sculpt"))
        .clicked()
}

/// The sculpt brush's settings for its window, as a form like the fillet's: which way a
/// stroke moves the surface, and the shape of the columns, with the split and the merge
/// beside it. Returns the one of those two that was clicked.
pub fn sculpt_controls(ui: &mut egui::Ui) -> Option<crate::app_actions::Action> {
    use crate::app_actions::Action;
    use editor_core::sculpt::{self, ColumnShape, SculptDirection};
    let mut parameters = sculpt::parameters();
    let labels = [tr("Direction:"), tr("Column shape:")];
    let label_width = labels
        .iter()
        .map(|l| crate::chrome::text_width(ui, l, theme::body()))
        .fold(0.0, f32::max);
    let gap = 12.0;
    let control = (ui.available_width() - label_width - gap).max(200.0);
    let row = |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(label_width, widgets::ROW_HEIGHT),
                egui::Sense::hover(),
            );
            theme::text(
                ui.painter(),
                egui::pos2(rect.right(), rect.center().y),
                egui::Align2::RIGHT_CENTER,
                label,
                theme::body(),
                theme::dim(),
            );
            ui.add_space(gap - ui.spacing().item_spacing.x);
            add(ui);
        });
    };
    let directions: Vec<(&str, &str)> = SculptDirection::ALL
        .iter()
        .map(|direction| (tr(direction.name()), ""))
        .collect();
    let mut direction = SculptDirection::ALL
        .iter()
        .position(|d| *d == parameters.direction)
        .unwrap_or(0);
    row(ui, labels[0], &mut |ui| {
        widgets::segmented_choice(ui, "sculpt-direction", &directions, &mut direction, control);
    });
    parameters.direction = SculptDirection::ALL[direction];
    let shapes: Vec<(&str, &str)> = ColumnShape::ALL
        .iter()
        .map(|shape| (tr(shape.name()), ""))
        .collect();
    let mut shape = ColumnShape::ALL
        .iter()
        .position(|s| *s == parameters.columns)
        .unwrap_or(0);
    row(ui, labels[1], &mut |ui| {
        widgets::segmented_choice(ui, "sculpt-columns", &shapes, &mut shape, control);
    });
    parameters.columns = ColumnShape::ALL[shape];
    sculpt::set_parameters(parameters);
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.add_space(label_width + gap - ui.spacing().item_spacing.x);
        if ui.button(tr("Split into columns")).clicked() {
            chosen = Some(Action::SplitColumns);
        }
        if ui.button(tr("Merge columns")).clicked() {
            chosen = Some(Action::MergeColumns);
        }
    });
    chosen
}
