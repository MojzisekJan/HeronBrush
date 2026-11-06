//! Compact UV workflow popovers and the detached window's density controls.
use super::*;
use editor_core::uv_workflow::{self, PivotMode, SnapTarget};
use geometry_math::vec::Vec2f;

impl UvPanel {
    pub(super) fn workflow_buttons(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        density_in_panel: bool,
    ) {
        let snap = self.workflow_icon(
            ui,
            egui_phosphor::regular::MAGNET,
            tr("Snapping"),
            self.canvas.view.options.snap,
        );
        egui::Popup::menu(&snap)
            .id(ui.make_persistent_id("uv-snap-menu"))
            .width(232.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.checkbox(&mut self.canvas.view.options.snap, tr("Snapping"));
                ui.add_enabled_ui(self.canvas.view.options.snap, |ui| {
                    for (mode, label) in [
                        (SnapTarget::Grid, tr("Grid")),
                        (SnapTarget::Vertex, tr("Vertex")),
                        (SnapTarget::Pixel, tr("Pixel")),
                    ] {
                        ui.radio_value(&mut self.canvas.view.options.snap_target, mode, label);
                    }
                    ui.add_enabled_ui(
                        self.canvas.view.options.snap_target != SnapTarget::Vertex,
                        |ui| {
                            widgets::row(ui, tr("Angle"), None, |ui| {
                                ui.add(
                                    egui_native_look::NumberField::new(
                                        &mut self.canvas.view.options.rotation_step,
                                    )
                                    .range(0.1..=180.0)
                                    .step(5.0)
                                    .width(ui.available_width()),
                                );
                            });
                        },
                    );
                });
            });
        let pivot = self.workflow_icon(
            ui,
            egui_phosphor::regular::CROSSHAIR,
            tr("Pivot"),
            self.canvas.view.options.pivot != PivotMode::Center,
        );
        egui::Popup::menu(&pivot)
            .id(ui.make_persistent_id("uv-pivot-menu"))
            .width(260.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                let old = self.canvas.view.options.pivot;
                for (mode, label) in [
                    (PivotMode::Center, tr("Center")),
                    (PivotMode::Origin, tr("Origin")),
                    (PivotMode::Cursor, tr("Cursor")),
                    (PivotMode::Individual, tr("Individual")),
                ] {
                    ui.radio_value(&mut self.canvas.view.options.pivot, mode, label);
                }
                if old != self.canvas.view.options.pivot
                    && self.canvas.view.options.pivot != PivotMode::Cursor
                {
                    self.canvas.origin = None;
                }
                let enabled = document.borrow().face_to_edit().is_some();
                ui.add_enabled_ui(enabled, |ui| {
                    let mut at = self
                        .canvas
                        .pivot(&document.borrow())
                        .unwrap_or(Vec2f::zero());
                    let mut changed = false;
                    for axis in 0..2 {
                        widgets::row_with_label_width(ui, ["U", "V"][axis], None, 24.0, |ui| {
                            let mut value = f64::from(at[axis]);
                            changed |= widgets::spin_width(
                                ui,
                                &mut value,
                                -1e7..=1e7,
                                1.0,
                                3,
                                ui.available_width(),
                            )
                            .changed();
                            at.v[axis] = value as f32;
                        });
                    }
                    if changed {
                        self.canvas.origin = Some(at);
                        self.canvas.view.options.pivot = PivotMode::Cursor;
                    }
                    let count = document
                        .borrow()
                        .face_to_edit_polygon()
                        .map_or(0, |p| p.len());
                    ui.horizontal(|ui| {
                        self.pivot_vertex = self.pivot_vertex.min(count.saturating_sub(1));
                        let mut vertex = self.pivot_vertex + 1;
                        if ui
                            .add(
                                egui_native_look::NumberField::new(&mut vertex)
                                    .range(1..=count.max(1))
                                    .width(110.0),
                            )
                            .changed()
                        {
                            self.pivot_vertex = vertex.saturating_sub(1);
                        }
                        if theme::button(ui, tr("Vertex"), ui.available_width(), count > 0)
                            .clicked()
                        {
                            let d = document.borrow();
                            if let (Some((_, face)), Some(polygon)) =
                                (d.face_to_edit(), d.face_to_edit_polygon())
                            {
                                self.canvas.origin =
                                    Some(face.uv_coords(polygon[self.pivot_vertex]));
                                self.canvas.view.options.pivot = PivotMode::Cursor;
                            }
                        }
                    });
                });
            });
        if !density_in_panel {
            let density = self.workflow_icon(
                ui,
                egui_phosphor::regular::RULER,
                tr("Texel density"),
                false,
            );
            egui::Popup::menu(&density)
                .id(ui.make_persistent_id("uv-density-menu"))
                .width(296.0)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| self.density_controls(ui, document));
        }
    }
    fn workflow_icon(
        &self,
        ui: &mut egui::Ui,
        glyph: &str,
        label: &str,
        on: bool,
    ) -> egui::Response {
        let response =
            widgets::icon_button(ui, widgets::ROW_HEIGHT, label, true, on, |p, r, color| {
                theme::icon_text(
                    p,
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    theme::icon(),
                    color,
                );
            });
        response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, label));
        response
    }
    pub(super) fn density_controls(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let active = document
            .borrow()
            .face_to_edit()
            .and_then(|(id, face)| uv_workflow::metrics(face).map(|metrics| (id, metrics.density)));
        let revision = document.borrow().revision();
        if self.density_face != active.map(|(id, _)| id) || self.density_revision != Some(revision)
        {
            self.density_revision = Some(revision);
            self.density_face = active.map(|(id, _)| id);
            if let Some((_, density)) = active {
                self.canvas.view.options.density = density;
            }
        }
        let enabled = active.is_some() && !document.borrow().in_transaction();
        ui.add_enabled_ui(enabled, |ui| {
            widgets::row_with_label_width(ui, tr("Texel density"), None, 104.0, |ui| {
                let width = (ui.available_width() - 58.0).max(60.0);
                widgets::spin_width(
                    ui,
                    &mut self.canvas.view.options.density,
                    1e-4..=1e4,
                    0.25,
                    3,
                    width,
                );
                ui.label("px/unit");
            });
            let each = (ui.available_width() - GAP) / 2.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                let apply = theme::button(ui, tr("Apply"), each, true).clicked();
                let match_active = theme::button(ui, tr("Match active"), each, true).clicked();
                if match_active {
                    if let Some((_, density)) = active {
                        self.canvas.view.options.density = density;
                    }
                }
                if apply || match_active {
                    uv_workflow::set_density(
                        &mut document.borrow_mut(),
                        self.canvas.view.options.density,
                    );
                    ui.ctx().request_repaint();
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Rc<RefCell<Document>> {
        let mut d = Document::new(Default::default());
        for material in ["a", "b"] {
            let brush = map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &geometry_math::bbox::Bbox3d::from_scalars(-32.0, 32.0),
                material,
            )
            .unwrap();
            d.add_brushes(vec![brush]);
        }
        d.select_all();
        d.edit_faces(|f| {
            f.uv_attributes.scale = if f.material_name == "a" {
                Vec2f::new(0.25, 0.25)
            } else {
                Vec2f::new(2.0, 2.0)
            }
        });
        Rc::new(RefCell::new(d))
    }
    fn draw(
        ctx: &egui::Context,
        panel: &mut UvPanel,
        d: &Rc<RefCell<Document>>,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 1100.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                panel.show(ui, d);
                assert!(
                    ui.min_rect().right() <= 320.5,
                    "UV toolbar and fields must fit the inspector width"
                );
            },
        );
        output.textures_delta.clear();
        output
    }
    fn at(output: &egui::FullOutput, text: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .rev()
            .find_map(|s| {
                if let egui::Shape::Text(t) = &s.shape {
                    (t.galley.text() == text).then(|| t.pos + t.galley.size() * 0.5)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("missing {text}"))
    }
    fn click(
        ctx: &egui::Context,
        panel: &mut UvPanel,
        d: &Rc<RefCell<Document>>,
        pos: egui::Pos2,
    ) -> egui::FullOutput {
        for pressed in [true, false] {
            draw(
                ctx,
                panel,
                d,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        draw(ctx, panel, d, vec![]);
        draw(ctx, panel, d, vec![])
    }
    #[test]
    fn popovers_change_snap_and_pivot_without_editing_the_document() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let d = document();
        let before = d.borrow().to_map_text();
        let mut panel = UvPanel::default();
        draw(&ctx, &mut panel, &d, vec![]);
        let output = draw(&ctx, &mut panel, &d, vec![]);
        let output = click(
            &ctx,
            &mut panel,
            &d,
            at(&output, egui_phosphor::regular::MAGNET),
        );
        let output = click(&ctx, &mut panel, &d, at(&output, tr("Pixel")));
        assert_eq!(panel.canvas.view.options.snap_target, SnapTarget::Pixel);
        let output = click(&ctx, &mut panel, &d, at(&output, tr("Snapping")));
        assert!(!panel.canvas.view.options.snap);
        let output = click(
            &ctx,
            &mut panel,
            &d,
            at(&output, egui_phosphor::regular::CROSSHAIR),
        );
        let output = click(&ctx, &mut panel, &d, at(&output, tr("Origin")));
        assert_eq!(panel.canvas.pivot(&d.borrow()), Some(Vec2f::zero()));
        panel.pivot_vertex = 1;
        let output = click(&ctx, &mut panel, &d, at(&output, tr("Vertex")));
        let expected = {
            let d = d.borrow();
            d.face_to_edit()
                .unwrap()
                .1
                .uv_coords(d.face_to_edit_polygon().unwrap()[1])
        };
        assert_eq!(panel.canvas.view.options.pivot, PivotMode::Cursor);
        assert!(geometry_math::vec::length(panel.canvas.origin.unwrap() - expected) < 0.001);
        let _output = click(&ctx, &mut panel, &d, at(&output, tr("Center")));
        assert_eq!(panel.canvas.view.options.pivot, PivotMode::Center);
        assert!(panel.canvas.origin.is_none());
        assert_eq!(d.borrow().to_map_text(), before);
    }
    #[test]
    fn density_apply_and_match_active_edit_the_whole_selection_as_one_undo() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let d = document();
        let before = d.borrow().to_map_text();
        let mut panel = UvPanel::default();
        draw(&ctx, &mut panel, &d, vec![]);
        let output = draw(&ctx, &mut panel, &d, vec![]);
        let output = click(
            &ctx,
            &mut panel,
            &d,
            at(&output, egui_phosphor::regular::RULER),
        );
        panel.canvas.view.options.density = 2.0;
        let output = click(&ctx, &mut panel, &d, at(&output, tr("Apply")));
        for (_, brush) in map_model::node::brushes_with_ids(d.borrow().tree_world()) {
            for face in brush.faces() {
                assert!((uv_workflow::metrics(face).unwrap().density - 2.0).abs() < 1e-5);
            }
        }
        d.borrow_mut().undo();
        assert_eq!(d.borrow().to_map_text(), before);
        let active = uv_workflow::metrics(d.borrow().face_to_edit().unwrap().1)
            .unwrap()
            .density;
        click(&ctx, &mut panel, &d, at(&output, tr("Match active")));
        for (_, brush) in map_model::node::brushes_with_ids(d.borrow().tree_world()) {
            for face in brush.faces() {
                assert!((uv_workflow::metrics(face).unwrap().density - active).abs() < 1e-5);
            }
        }
        d.borrow_mut().undo();
        assert_eq!(d.borrow().to_map_text(), before);
    }
}
