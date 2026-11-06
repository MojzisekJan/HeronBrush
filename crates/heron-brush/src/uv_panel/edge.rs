//! Explicit edge picking and alignment in either UV canvas.
use super::UvPanel;
use crate::widgets;
use editor_core::document::Document;
use editor_core::i18n::tr;
use editor_core::uv_edge::{self, EdgeAxis};
use eframe::egui;
use std::cell::RefCell;
use std::rc::Rc;

impl UvPanel {
    pub(super) fn projected_edges(
        &self,
        document: &Document,
        rect: egui::Rect,
    ) -> Vec<[egui::Pos2; 2]> {
        let Some((reference, face)) = document.face_to_edit() else {
            return Vec::new();
        };
        let Some(brush) = document.brush(reference.node) else {
            return Vec::new();
        };
        let points = brush
            .geometry()
            .face_positions(map_model::polyhedron::FaceId(reference.face as u32));
        let projected: Vec<_> = points
            .iter()
            .map(|&point| {
                let (x, y) = self.canvas.view.project(
                    document,
                    f64::from(rect.height()),
                    face.uv_coords(point),
                );
                rect.min + egui::vec2(x as f32, y as f32)
            })
            .collect();
        (0..projected.len())
            .map(|i| [projected[i], projected[(i + 1) % projected.len()]])
            .collect()
    }
    pub(super) fn pick_edge(
        &mut self,
        ui: &egui::Ui,
        document: &Rc<RefCell<Document>>,
        response: &egui::Response,
        rect: egui::Rect,
    ) {
        let document = document.borrow();
        let active = document.face_to_edit().map(|(reference, _)| reference);
        if self
            .selected_edge
            .is_some_and(|(reference, _)| Some(reference) != active)
        {
            self.selected_edge = None;
        }
        if !response.clicked() || ui.input(|i| !i.modifiers.is_none()) {
            return;
        }
        let Some(pointer) = response.interact_pointer_pos() else {
            return;
        };
        let nearest = self
            .projected_edges(&document, rect)
            .iter()
            .enumerate()
            .map(|(index, &[a, b])| {
                let edge = b - a;
                let t = ((pointer - a).dot(edge) / edge.length_sq().max(1e-6)).clamp(0.0, 1.0);
                (index, pointer.distance(a + edge * t))
            })
            .filter(|(_, distance)| *distance <= 8.0)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        self.selected_edge = active.zip(nearest.map(|(index, _)| index));
    }
    pub(super) fn paint_edge(
        &self,
        document: &Rc<RefCell<Document>>,
        painter: &egui::Painter,
        rect: egui::Rect,
    ) {
        let Some((_, index)) = self.selected_edge else {
            return;
        };
        if let Some(&edge) = self.projected_edges(&document.borrow(), rect).get(index) {
            painter.line_segment(edge, egui::Stroke::new(3.0, crate::theme::fg()));
        }
    }
    pub(super) fn edge_actions(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let valid = self.selected_edge.filter(|(reference, _)| {
            document.borrow().face_to_edit().map(|(face, _)| face) == Some(*reference)
        });
        let items = [
            (
                tr("Edge horizontal"),
                tr("Click an edge in the UV view, then align it horizontally"),
                valid.is_some(),
            ),
            (
                tr("Edge vertical"),
                tr("Click an edge in the UV view, then align it vertically"),
                valid.is_some(),
            ),
        ];
        if let Some(index) = widgets::segmented(ui, "uv-edge-alignment", &items) {
            if let Some((reference, edge)) = valid {
                uv_edge::align(
                    &mut document.borrow_mut(),
                    reference,
                    edge,
                    if index == 0 {
                        EdgeAxis::Horizontal
                    } else {
                        EdgeAxis::Vertical
                    },
                );
            }
        }
    }
}
